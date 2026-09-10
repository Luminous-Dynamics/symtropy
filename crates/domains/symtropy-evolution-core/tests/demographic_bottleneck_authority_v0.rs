use std::collections::BTreeMap;
use symtropy_evolution_core::{
    execute_census_resize_bottleneck, AlleleId, DemographicEventDeclaration, DemographicEventId,
    DemographicEventKind, DemographicInterventionCursor, DemographicStructureTransition,
    EvolutionExperimentId, HereditarySchema, HereditarySchemaId, LocusDefinition, LocusId,
    MetapopulationSnapshot, PopulationGeneration, PopulationGeneticState, PopulationId,
    PopulationStructureModel, PopulationStructureProfile, PopulationStructureProfileId,
    PopulationTrajectoryPoint,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

#[test]
fn non_root_cursor_cannot_be_reused_as_weak_execution_authority() {
    let schema = HereditarySchema::new(
        HereditarySchemaId::new("non-root-rejection").unwrap(),
        2,
        vec![LocusDefinition::new(
            LocusId::new("focal").unwrap(),
            [allele("a"), allele("b")],
        )
        .unwrap()],
    )
    .unwrap();
    let population_id = PopulationId::new("island").unwrap();
    let structure = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("single-island").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![population_id.clone()],
        vec![],
    )
    .unwrap();
    let state = PopulationGeneticState::from_counts(
        population_id.clone(),
        &schema,
        8,
        BTreeMap::from([(
            LocusId::new("focal").unwrap(),
            BTreeMap::from([(allele("a"), 8), (allele("b"), 8)]),
        )]),
    )
    .unwrap();
    let populations = BTreeMap::from([(population_id.clone(), state)]);
    let point = PopulationTrajectoryPoint::declare_reference_start(
        &schema,
        populations.get(&population_id).unwrap(),
        EvolutionExperimentId::new("exp").unwrap(),
        PopulationGeneration(3),
    )
    .unwrap();
    let points = BTreeMap::from([(population_id.clone(), point)]);
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

    let event1 = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("first").unwrap(),
        "v1",
        DemographicEventKind::CensusResize {
            population: population_id.clone(),
            target_census: 8,
        },
        &schema,
        &structure,
        &populations,
        &points,
        &snapshot,
    )
    .unwrap();
    let transition1 = DemographicStructureTransition::declare_current(
        &event1,
        &schema,
        &structure,
        &populations,
        &points,
        &snapshot,
        &structure,
    )
    .unwrap();
    let first = execute_census_resize_bottleneck(
        &schema,
        &structure,
        &populations,
        &points,
        &snapshot,
        &root,
        &event1,
        &transition1,
    )
    .unwrap();

    let event2 = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("second").unwrap(),
        "v1",
        DemographicEventKind::CensusResize {
            population: population_id,
            target_census: 8,
        },
        &schema,
        &structure,
        &first.populations,
        &first.points,
        &first.snapshot,
    )
    .unwrap();
    let transition2 = DemographicStructureTransition::declare_current(
        &event2,
        &schema,
        &structure,
        &first.populations,
        &first.points,
        &first.snapshot,
        &structure,
    )
    .unwrap();

    assert!(execute_census_resize_bottleneck(
        &schema,
        &structure,
        &first.populations,
        &first.points,
        &first.snapshot,
        &first.history_cursor,
        &event2,
        &transition2,
    )
    .is_err());
}
