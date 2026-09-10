use std::collections::BTreeMap;
use symtropy_evolution_core::{
    execute_census_preserving_pulse_admixture, AlleleId, DemographicEventDeclaration,
    DemographicEventId, DemographicEventKind, DemographicInterventionCursor,
    DemographicStructureTransition, EvolutionExperimentId, HereditarySchema, HereditarySchemaId,
    LocusDefinition, LocusId, MetapopulationSnapshot, PopulationGeneration,
    PopulationGeneticState, PopulationId, PopulationStructureModel, PopulationStructureProfile,
    PopulationStructureProfileId, PopulationTrajectoryPoint,
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

fn fixture(source_census: u64, destination_census: u64, source_a: u64, destination_a: u64) -> Fixture {
    let schema = HereditarySchema::new(
        HereditarySchemaId::new("pulse-admixture-v0").unwrap(),
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
        PopulationStructureProfileId::new("pulse-structure").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![source.clone(), destination.clone(), witness.clone()],
        vec![],
    )
    .unwrap();
    let populations = BTreeMap::from([
        (
            source.clone(),
            state(&schema, source.clone(), source_census, source_a),
        ),
        (
            destination.clone(),
            state(
                &schema,
                destination.clone(),
                destination_census,
                destination_a,
            ),
        ),
        (witness.clone(), state(&schema, witness.clone(), 3, 3)),
    ]);
    let experiment = EvolutionExperimentId::new("pulse-exp").unwrap();
    let generation = PopulationGeneration(17);
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

fn event(f: &Fixture, event_id: &str, fraction_ppm: u32) -> DemographicEventDeclaration {
    DemographicEventDeclaration::declare_current(
        DemographicEventId::new(event_id).unwrap(),
        "v1",
        DemographicEventKind::PulseAdmixture {
            destination: pop("destination"),
            source: pop("source"),
            source_fraction_ppm: fraction_ppm,
        },
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    )
    .unwrap()
}

fn execute(
    f: &Fixture,
    event: &DemographicEventDeclaration,
) -> Result<
    symtropy_evolution_core::DemographicAdmixtureExecutionResult,
    symtropy_evolution_core::EvolutionError,
> {
    let transition = DemographicStructureTransition::declare_current(
        event,
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &f.structure,
    )?;
    let root = DemographicInterventionCursor::declare_reference_root(
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    )?;
    execute_census_preserving_pulse_admixture(
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &root,
        event,
        &transition,
    )
}

#[test]
fn pulse_preserves_destination_census_source_and_unrelated_state() {
    let f = fixture(10, 10, 20, 0);
    let event = event(&f, "pulse-basic", 250_000);
    let result = execute(&f, &event).unwrap();

    assert_eq!(
        result.populations[&pop("destination")].census_individuals,
        f.populations[&pop("destination")].census_individuals
    );
    assert_eq!(result.populations[&pop("source")], f.populations[&pop("source")]);
    assert_eq!(result.points[&pop("source")], f.points[&pop("source")]);
    assert_eq!(result.populations[&pop("witness")], f.populations[&pop("witness")]);
    assert_eq!(result.points[&pop("witness")], f.points[&pop("witness")]);
    assert_eq!(result.snapshot.generation(), PopulationGeneration(17));
    assert_eq!(result.history_cursor.intervention_ordinal(), 1);
    assert_eq!(result.provenance.realized_replacement_copies(), 5);
}

#[test]
fn full_pulse_replaces_destination_marginals_when_capacities_match() {
    let f = fixture(10, 10, 20, 0);
    let event = event(&f, "pulse-full", 1_000_000);
    let result = execute(&f, &event).unwrap();
    let locus = LocusId::new("focal").unwrap();

    assert_eq!(result.provenance.realized_replacement_copies(), 20);
    assert_eq!(
        result.populations[&pop("destination")].allele_copy_counts[&locus],
        BTreeMap::from([(allele("a"), 20)])
    );
    assert_eq!(result.populations[&pop("source")], f.populations[&pop("source")]);
}

#[test]
fn tiny_fraction_can_quantize_to_biological_noop_but_history_advances() {
    let f = fixture(2, 2, 4, 1);
    let event = event(&f, "pulse-quantized-noop", 100_000);
    let result = execute(&f, &event).unwrap();

    assert_eq!(result.provenance.realized_replacement_copies(), 0);
    assert_eq!(result.populations, f.populations);
    assert_eq!(result.points, f.points);
    assert_eq!(result.snapshot, f.snapshot);
    assert_eq!(result.history_cursor.intervention_ordinal(), 1);
}

#[test]
fn realized_copy_count_uses_declared_nearest_integer_rounding() {
    let f = fixture(10, 10, 20, 0); // destination has 20 marginal copies

    let low = execute(&f, &event(&f, "round-low", 124_999)).unwrap();
    let tie_up = execute(&f, &event(&f, "round-up", 125_000)).unwrap();

    // 20 * 0.124999 = 2.49998 -> 2; 20 * 0.125 = 2.5 -> 3.
    assert_eq!(low.provenance.realized_replacement_copies(), 2);
    assert_eq!(tie_up.provenance.realized_replacement_copies(), 3);
}

#[test]
fn source_capacity_violation_fails_closed() {
    let f = fixture(1, 10, 2, 0); // donor has only two marginal copies
    let event = event(&f, "pulse-too-large", 500_000); // K = 10
    assert!(execute(&f, &event).is_err());
}

#[test]
fn increasing_fraction_with_fixed_opposite_alleles_changes_exactly_realized_k_copies() {
    let f = fixture(10, 10, 20, 0);
    let locus = LocusId::new("focal").unwrap();

    for (fraction, expected_k) in [(100_000, 2_u64), (250_000, 5), (500_000, 10)] {
        let result = execute(&f, &event(&f, &format!("pulse-{fraction}"), fraction)).unwrap();
        let destination = &result.populations[&pop("destination")];
        let a_count = destination.allele_copy_counts[&locus]
            .get(&allele("a"))
            .copied()
            .unwrap_or(0);
        assert_eq!(result.provenance.realized_replacement_copies(), expected_k);
        assert_eq!(a_count, expected_k);
    }
}

#[test]
fn restored_receipt_requires_exact_current_revalidation() {
    let f = fixture(10, 10, 14, 6);
    let event = event(&f, "pulse-restore", 300_000);
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
    let result = execute_census_preserving_pulse_admixture(
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
    let restored: symtropy_evolution_core::DemographicAdmixtureExecutionResult =
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
}
