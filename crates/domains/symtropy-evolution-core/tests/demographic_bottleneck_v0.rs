use std::collections::BTreeMap;
use symtropy_evolution_core::{
    execute_census_resize_bottleneck, AlleleId, DemographicEventDeclaration, DemographicEventId,
    DemographicEventKind, DemographicInterventionCursor, DemographicStructureTransition,
    EvolutionError, EvolutionExperimentId, HereditarySchema, HereditarySchemaId, LocusDefinition,
    LocusId, MetapopulationSnapshot, PopulationGeneration, PopulationGeneticState, PopulationId,
    PopulationStructureModel, PopulationStructureProfile, PopulationStructureProfileId,
    PopulationTrajectoryPoint,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn pop(id: &str) -> PopulationId {
    PopulationId::new(id).unwrap()
}

fn schema(extra_locus: bool) -> HereditarySchema {
    let mut loci = vec![
        LocusDefinition::new(
            LocusId::new("focal").unwrap(),
            [allele("a"), allele("b")],
        )
        .unwrap(),
    ];
    if extra_locus {
        loci.push(
            LocusDefinition::new(
                LocusId::new("unrelated").unwrap(),
                [allele("x"), allele("y")],
            )
            .unwrap(),
        );
    }
    HereditarySchema::new(
        HereditarySchemaId::new(if extra_locus {
            "bottleneck-two-locus"
        } else {
            "bottleneck-one-locus"
        })
        .unwrap(),
        2,
        loci,
    )
    .unwrap()
}

fn state(schema: &HereditarySchema, census: u64, focal_a: u64) -> PopulationGeneticState {
    let id = pop("island");
    let copies = census * u64::from(schema.ploidy);
    let mut loci = BTreeMap::new();
    let mut focal = BTreeMap::new();
    if focal_a > 0 {
        focal.insert(allele("a"), focal_a);
    }
    if focal_a < copies {
        focal.insert(allele("b"), copies - focal_a);
    }
    loci.insert(LocusId::new("focal").unwrap(), focal);
    if schema.loci.contains_key(&LocusId::new("unrelated").unwrap()) {
        loci.insert(
            LocusId::new("unrelated").unwrap(),
            BTreeMap::from([(allele("x"), copies / 2), (allele("y"), copies - copies / 2)]),
        );
    }
    PopulationGeneticState::from_counts(id, schema, census, loci).unwrap()
}

struct Fixture {
    schema: HereditarySchema,
    structure: PopulationStructureProfile,
    populations: BTreeMap<PopulationId, PopulationGeneticState>,
    points: BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    snapshot: MetapopulationSnapshot,
}

fn fixture(census: u64, focal_a: u64, extra_locus: bool) -> Fixture {
    let schema = schema(extra_locus);
    let id = pop("island");
    let structure = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("single-island").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![id.clone()],
        vec![],
    )
    .unwrap();
    let population = state(&schema, census, focal_a);
    let populations = BTreeMap::from([(id.clone(), population)]);
    let point = PopulationTrajectoryPoint::declare_reference_start(
        &schema,
        populations.get(&id).unwrap(),
        EvolutionExperimentId::new("bottleneck-experiment").unwrap(),
        PopulationGeneration(17),
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

fn execute(f: &Fixture, event_id: &str, target_census: u64) -> Result<symtropy_evolution_core::DemographicEventExecutionResult, EvolutionError> {
    let event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new(event_id).unwrap(),
        "v1",
        DemographicEventKind::CensusResize {
            population: pop("island"),
            target_census,
        },
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    )?;
    let structure_transition = DemographicStructureTransition::declare_current(
        &event,
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &f.structure,
    )?;
    let cursor = DemographicInterventionCursor::declare_reference_root(
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    )?;
    execute_census_resize_bottleneck(
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &cursor,
        &event,
        &structure_transition,
    )
}

#[test]
fn noop_preserves_biology_but_advances_demographic_history() {
    let f = fixture(10, 10, false);
    let result = execute(&f, "noop", 10).unwrap();
    assert_eq!(result.populations, f.populations);
    assert_eq!(result.points, f.points);
    assert_eq!(result.snapshot, f.snapshot);
    assert_eq!(result.history_cursor.intervention_ordinal(), 1);
    let root = DemographicInterventionCursor::declare_reference_root(
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    )
    .unwrap();
    assert_ne!(
        result.history_cursor.canonical_digest().unwrap(),
        root.canonical_digest().unwrap()
    );
}

#[test]
fn expansion_is_not_treated_as_inverse_bottleneck() {
    let f = fixture(10, 10, false);
    assert_eq!(
        execute(&f, "expand", 11).unwrap_err(),
        EvolutionError::DemographicExpansionUnsupported {
            current_census: 10,
            target_census: 11,
        }
    );
}

#[test]
fn downsample_has_exact_copy_total_and_fixed_alleles_stay_fixed() {
    let f = fixture(12, 24, false);
    let result = execute(&f, "fixed-bottleneck", 3).unwrap();
    let state = &result.populations[&pop("island")];
    assert_eq!(state.census_individuals, 3);
    assert_eq!(
        state.allele_copy_counts[&LocusId::new("focal").unwrap()],
        BTreeMap::from([(allele("a"), 6)])
    );
    assert_eq!(result.history_cursor.generation(), PopulationGeneration(17));
    assert_eq!(result.snapshot.generation(), PopulationGeneration(17));
}

#[test]
fn harsher_common_random_bottleneck_is_nested_inside_milder_one() {
    let f = fixture(20, 17, false);
    let mild = execute(&f, "nested", 12).unwrap();
    let harsh = execute(&f, "nested", 5).unwrap();
    let locus = LocusId::new("focal").unwrap();
    let mild_counts = &mild.populations[&pop("island")].allele_copy_counts[&locus];
    let harsh_counts = &harsh.populations[&pop("island")].allele_copy_counts[&locus];
    for (allele_id, harsh_count) in harsh_counts {
        assert!(*harsh_count <= mild_counts.get(allele_id).copied().unwrap_or(0));
    }
}

#[test]
fn unrelated_locus_does_not_reroll_focal_survivor_field() {
    let one = fixture(20, 17, false);
    let two = fixture(20, 17, true);
    let one_result = execute(&one, "locus-invariance", 7).unwrap();
    let two_result = execute(&two, "locus-invariance", 7).unwrap();
    let locus = LocusId::new("focal").unwrap();
    assert_eq!(
        one_result.populations[&pop("island")].allele_copy_counts[&locus],
        two_result.populations[&pop("island")].allele_copy_counts[&locus]
    );
}

#[test]
fn restored_result_provenance_requires_exact_current_revalidation() {
    let f = fixture(20, 17, false);
    let event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("restore").unwrap(),
        "v1",
        DemographicEventKind::CensusResize {
            population: pop("island"),
            target_census: 7,
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

    let encoded = serde_json::to_string(&result).unwrap();
    let restored: symtropy_evolution_core::DemographicEventExecutionResult =
        serde_json::from_str(&encoded).unwrap();
    restored
        .provenance
        .validate_current(
            &f.schema,
            &f.structure,
            &f.populations,
            &f.points,
            &f.snapshot,
            &root,
            &event,
            &transition,
            &restored,
        )
        .unwrap();

    let stale_event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("restore").unwrap(),
        "v1",
        DemographicEventKind::CensusResize {
            population: pop("island"),
            target_census: 6,
        },
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    )
    .unwrap();
    assert!(restored
        .provenance
        .validate_current(
            &f.schema,
            &f.structure,
            &f.populations,
            &f.points,
            &f.snapshot,
            &root,
            &stale_event,
            &transition,
            &restored,
        )
        .is_err());
}
