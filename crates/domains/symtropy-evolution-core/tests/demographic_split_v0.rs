use std::collections::BTreeMap;
use symtropy_evolution_core::{
    execute_conservative_population_split, AlleleId, DaughterPopulation,
    DemographicEventDeclaration, DemographicEventId, DemographicEventKind,
    DemographicInterventionCursor, DemographicStructureTransition, EvolutionExperimentId,
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
    source_structure: PopulationStructureProfile,
    successor_structure: PopulationStructureProfile,
    populations: BTreeMap<PopulationId, PopulationGeneticState>,
    points: BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    snapshot: MetapopulationSnapshot,
}

fn state(
    schema: &HereditarySchema,
    id: PopulationId,
    census: u64,
    a_copies: u64,
) -> PopulationGeneticState {
    let total = census * u64::from(schema.ploidy);
    let mut counts = BTreeMap::new();
    if a_copies > 0 {
        counts.insert(allele("a"), a_copies);
    }
    if a_copies < total {
        counts.insert(allele("b"), total - a_copies);
    }
    PopulationGeneticState::from_counts(
        id,
        schema,
        census,
        BTreeMap::from([(LocusId::new("focal").unwrap(), counts)]),
    )
    .unwrap()
}

fn fixture(source_a_copies: u64) -> Fixture {
    let schema = HereditarySchema::new(
        HereditarySchemaId::new("split-v0").unwrap(),
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
    let east = pop("east");
    let west = pop("west");
    let source_structure = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("before-split").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![ancestral.clone(), witness.clone()],
        vec![],
    )
    .unwrap();
    let successor_structure = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("after-split").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![east, west, witness.clone()],
        vec![],
    )
    .unwrap();
    let populations = BTreeMap::from([
        (
            ancestral.clone(),
            state(&schema, ancestral.clone(), 10, source_a_copies),
        ),
        (witness.clone(), state(&schema, witness.clone(), 7, 8)),
    ]);
    let experiment = EvolutionExperimentId::new("split-exp").unwrap();
    let generation = PopulationGeneration(23);
    let points = populations
        .iter()
        .map(|(id, population)| {
            (
                id.clone(),
                PopulationTrajectoryPoint::declare_reference_start(
                    &schema,
                    population,
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

fn event(f: &Fixture, event_id: &str, daughters: Vec<(&str, u64)>) -> DemographicEventDeclaration {
    DemographicEventDeclaration::declare_current(
        DemographicEventId::new(event_id).unwrap(),
        "v1",
        DemographicEventKind::PopulationSplit {
            source: pop("ancestral"),
            daughters: daughters
                .into_iter()
                .map(|(id, census)| DaughterPopulation::new(pop(id), census).unwrap())
                .collect(),
        },
        &f.schema,
        &f.source_structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    )
    .unwrap()
}

fn execute(
    f: &Fixture,
    event: &DemographicEventDeclaration,
) -> Result<symtropy_evolution_core::DemographicSplitExecutionResult, symtropy_evolution_core::EvolutionError> {
    let transition = DemographicStructureTransition::declare_current(
        event,
        &f.schema,
        &f.source_structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &f.successor_structure,
    )?;
    let root = DemographicInterventionCursor::declare_reference_root(
        &f.schema,
        &f.source_structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    )?;
    execute_conservative_population_split(
        &f.schema,
        &f.source_structure,
        &f.successor_structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &root,
        event,
        &transition,
    )
}

#[test]
fn conservative_split_removes_source_and_preserves_unrelated_population() {
    let f = fixture(9);
    let event = event(&f, "split", vec![("west", 6), ("east", 4)]);
    let result = execute(&f, &event).unwrap();

    assert!(!result.populations.contains_key(&pop("ancestral")));
    assert!(!result.points.contains_key(&pop("ancestral")));
    assert_eq!(result.populations[&pop("east")].census_individuals, 4);
    assert_eq!(result.populations[&pop("west")].census_individuals, 6);
    assert_eq!(result.populations[&pop("witness")], f.populations[&pop("witness")]);
    assert_eq!(result.points[&pop("witness")], f.points[&pop("witness")]);
    assert_eq!(result.snapshot.generation(), PopulationGeneration(23));
    assert_eq!(result.history_cursor.intervention_ordinal(), 1);
}

#[test]
fn daughter_union_exactly_reconstructs_source_allele_counts() {
    let f = fixture(9);
    let event = event(&f, "conservation", vec![("east", 4), ("west", 6)]);
    let result = execute(&f, &event).unwrap();
    let locus = LocusId::new("focal").unwrap();
    let mut union: BTreeMap<AlleleId, u64> = BTreeMap::new();
    for daughter in [pop("east"), pop("west")] {
        for (allele_id, count) in &result.populations[&daughter].allele_copy_counts[&locus] {
            *union.entry(allele_id.clone()).or_insert(0) += *count;
        }
    }
    assert_eq!(union, f.populations[&pop("ancestral")].allele_copy_counts[&locus]);
}

#[test]
fn fixed_source_allele_is_fixed_in_every_daughter() {
    let f = fixture(20);
    let event = event(&f, "fixed", vec![("east", 4), ("west", 6)]);
    let result = execute(&f, &event).unwrap();
    let locus = LocusId::new("focal").unwrap();
    assert_eq!(
        result.populations[&pop("east")].allele_copy_counts[&locus],
        BTreeMap::from([(allele("a"), 8)])
    );
    assert_eq!(
        result.populations[&pop("west")].allele_copy_counts[&locus],
        BTreeMap::from([(allele("a"), 12)])
    );
}

#[test]
fn split_cannot_hide_population_growth_or_loss() {
    let f = fixture(9);
    let too_many = event(&f, "growth-hidden", vec![("east", 4), ("west", 7)]);
    let too_few = event(&f, "loss-hidden", vec![("east", 4), ("west", 5)]);
    assert!(execute(&f, &too_many).is_err());
    assert!(execute(&f, &too_few).is_err());
}

#[test]
fn canonical_daughter_declaration_order_does_not_change_split() {
    let f = fixture(9);
    let a = event(&f, "ordered", vec![("east", 4), ("west", 6)]);
    let b = event(&f, "ordered", vec![("west", 6), ("east", 4)]);
    assert_eq!(a.canonical_digest().unwrap(), b.canonical_digest().unwrap());
    assert_eq!(execute(&f, &a).unwrap(), execute(&f, &b).unwrap());
}

#[test]
fn restored_split_receipt_requires_exact_current_revalidation() {
    let f = fixture(9);
    let event = event(&f, "restore", vec![("east", 4), ("west", 6)]);
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
    let encoded = serde_json::to_string(&result).unwrap();
    let restored: symtropy_evolution_core::DemographicSplitExecutionResult =
        serde_json::from_str(&encoded).unwrap();
    restored
        .provenance
        .validate_current(
            &f.schema,
            &f.source_structure,
            &f.successor_structure,
            &f.populations,
            &f.points,
            &f.snapshot,
            &root,
            &event,
            &transition,
            &restored,
        )
        .unwrap();
}
