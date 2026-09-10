use std::collections::BTreeMap;
use symtropy_evolution_core::{
    execute_founder_or_recolonization_sample, AlleleId, DemographicEventDeclaration,
    DemographicEventId, DemographicEventKind, DemographicInterventionCursor,
    DemographicStructureTransition, EvolutionError, EvolutionExperimentId, HereditarySchema,
    HereditarySchemaId, LocusDefinition, LocusId, MetapopulationSnapshot, ParentalSourceEdge,
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
    source_structure: PopulationStructureProfile,
    populations: BTreeMap<PopulationId, PopulationGeneticState>,
    points: BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    snapshot: MetapopulationSnapshot,
}

fn fixture(fixed: bool) -> Fixture {
    let schema = HereditarySchema::new(
        HereditarySchemaId::new("founder-v0").unwrap(),
        2,
        vec![LocusDefinition::new(
            LocusId::new("focal").unwrap(),
            [allele("a"), allele("b")],
        )
        .unwrap()],
    )
    .unwrap();
    let source = pop("source");
    let source_structure = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("source-only").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![source.clone()],
        vec![],
    )
    .unwrap();
    let counts = if fixed {
        BTreeMap::from([(allele("a"), 40)])
    } else {
        BTreeMap::from([(allele("a"), 17), (allele("b"), 23)])
    };
    let state = PopulationGeneticState::from_counts(
        source.clone(),
        &schema,
        20,
        BTreeMap::from([(LocusId::new("focal").unwrap(), counts)]),
    )
    .unwrap();
    let populations = BTreeMap::from([(source.clone(), state)]);
    let point = PopulationTrajectoryPoint::declare_reference_start(
        &schema,
        populations.get(&source).unwrap(),
        EvolutionExperimentId::new("founder-exp").unwrap(),
        PopulationGeneration(11),
    )
    .unwrap();
    let points = BTreeMap::from([(source, point)]);
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
        populations,
        points,
        snapshot,
    }
}

fn successor(destination: &str, edge_ppm: Option<u32>) -> PopulationStructureProfile {
    let source = pop("source");
    let dest = pop(destination);
    let edges = edge_ppm
        .map(|ppm| vec![ParentalSourceEdge::new(dest.clone(), source.clone(), ppm).unwrap()])
        .unwrap_or_default();
    PopulationStructureProfile::new(
        PopulationStructureProfileId::new(format!("with-{destination}")).unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![source, dest],
        edges,
    )
    .unwrap()
}

fn execute(
    f: &Fixture,
    event_id: &str,
    destination: &str,
    founder_census: u64,
    recolonization: bool,
    successor_structure: &PopulationStructureProfile,
) -> Result<symtropy_evolution_core::DemographicFounderExecutionResult, EvolutionError> {
    let kind = if recolonization {
        DemographicEventKind::Recolonization {
            source: pop("source"),
            recolonized_population: pop(destination),
            founder_census,
        }
    } else {
        DemographicEventKind::FounderEvent {
            source: pop("source"),
            founded_population: pop(destination),
            founder_census,
        }
    };
    let event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new(event_id).unwrap(),
        "v1",
        kind,
        &f.schema,
        &f.source_structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    )?;
    let transition = DemographicStructureTransition::declare_current(
        &event,
        &f.schema,
        &f.source_structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        successor_structure,
    )?;
    let root = DemographicInterventionCursor::declare_reference_root(
        &f.schema,
        &f.source_structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    )?;
    execute_founder_or_recolonization_sample(
        &f.schema,
        &f.source_structure,
        successor_structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &root,
        &event,
        &transition,
    )
}

#[test]
fn founder_adds_one_population_without_depleting_source() {
    let f = fixture(false);
    let successor = successor("colony", None);
    let result = execute(&f, "founding", "colony", 5, false, &successor).unwrap();

    assert_eq!(result.populations[&pop("source")], f.populations[&pop("source")]);
    assert_eq!(result.points[&pop("source")], f.points[&pop("source")]);
    assert_eq!(result.populations[&pop("colony")].census_individuals, 5);
    assert_eq!(result.snapshot.generation(), PopulationGeneration(11));
    assert_eq!(result.history_cursor.intervention_ordinal(), 1);
}

#[test]
fn fixed_source_allele_remains_fixed_in_founder_population() {
    let f = fixture(true);
    let successor = successor("colony", None);
    let result = execute(&f, "fixed-founder", "colony", 3, false, &successor).unwrap();
    assert_eq!(
        result.populations[&pop("colony")].allele_copy_counts[&LocusId::new("focal").unwrap()],
        BTreeMap::from([(allele("a"), 6)])
    );
}

#[test]
fn founder_census_cannot_exceed_source_census_in_reference_model() {
    let f = fixture(false);
    let successor = successor("colony", None);
    assert_eq!(
        execute(&f, "too-large", "colony", 21, false, &successor).unwrap_err(),
        EvolutionError::DemographicSampleExceedsSourceCensus {
            source_census: 20,
            target_census: 21,
        }
    );
}

#[test]
fn harsher_founder_sample_is_nested_inside_milder_sample() {
    let f = fixture(false);
    let successor = successor("colony", None);
    let mild = execute(&f, "nested-founder", "colony", 12, false, &successor).unwrap();
    let harsh = execute(&f, "nested-founder", "colony", 4, false, &successor).unwrap();
    let locus = LocusId::new("focal").unwrap();
    let mild_counts = &mild.populations[&pop("colony")].allele_copy_counts[&locus];
    let harsh_counts = &harsh.populations[&pop("colony")].allele_copy_counts[&locus];
    for (allele_id, harsh_count) in harsh_counts {
        assert!(*harsh_count <= mild_counts.get(allele_id).copied().unwrap_or(0));
    }
}

#[test]
fn recolonization_uses_same_non_depleting_reference_semantics() {
    let f = fixture(false);
    let successor = successor("recolonized", None);
    let result = execute(&f, "recolonize", "recolonized", 6, true, &successor).unwrap();
    assert_eq!(result.populations[&pop("source")], f.populations[&pop("source")]);
    assert_eq!(result.populations[&pop("recolonized")].census_individuals, 6);
}

#[test]
fn changed_successor_structure_stales_founder_execution_authority() {
    let f = fixture(false);
    let successor_a = successor("colony", None);
    let event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("stale-successor").unwrap(),
        "v1",
        DemographicEventKind::FounderEvent {
            source: pop("source"),
            founded_population: pop("colony"),
            founder_census: 5,
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
        &successor_a,
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
    let result = execute_founder_or_recolonization_sample(
        &f.schema,
        &f.source_structure,
        &successor_a,
        &f.populations,
        &f.points,
        &f.snapshot,
        &root,
        &event,
        &transition,
    )
    .unwrap();
    let successor_b = successor("colony", Some(100_000));

    assert!(result
        .provenance
        .validate_current(
            &f.schema,
            &f.source_structure,
            &successor_b,
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
