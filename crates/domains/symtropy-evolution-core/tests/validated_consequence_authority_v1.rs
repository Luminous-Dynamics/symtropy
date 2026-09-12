use symtropy_evolution_core::{
    initialize_root_mutation_lineage, AlleleId, AncestryCopyId, ChromosomeAncestryState,
    ChromosomeDefinition, ChromosomeHaplotype, ChromosomeId, ChromosomeLocus, ChromosomeMap,
    ChromosomeMapId, ConsequenceError, ConsequenceObservationId, ConsequenceWindowId,
    EvolutionIndividualId, EvolutionaryContextContentDigest, EvolutionaryContextId,
    EvolutionaryContextRef, ExplicitConsequenceLedger, ExplicitLinkedPopulationCensus,
    GeneticMapPositionMicromorgans, HaplotypeAncestryClass, HereditarySchema,
    HereditarySchemaId, IndividualConsequenceObservation, IndividualConsequences,
    LinkedIndividualManifest, LinkedIndividualSubject, LocusDefinition, LocusId,
    MutationLineageState, PhasedAncestryState, PhasedChromosomeState, PhasedHereditaryState,
    PopulationId, ValidatedConsequenceLedger, ViabilityConsequence,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn locus() -> LocusId {
    LocusId::new("focal").unwrap()
}

fn chromosome() -> ChromosomeId {
    ChromosomeId::new("chr-validated-consequence").unwrap()
}

fn ancestry(id: &str) -> AncestryCopyId {
    AncestryCopyId::new(id).unwrap()
}

fn individual_id(id: &str) -> EvolutionIndividualId {
    EvolutionIndividualId::new(id).unwrap()
}

fn population() -> PopulationId {
    PopulationId::new("validated-consequence-pop").unwrap()
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("validated-consequence-schema").unwrap(),
        2,
        [LocusDefinition::new(locus(), [allele("a0"), allele("a1")]).unwrap()],
    )
    .unwrap()
}

fn chromosome_map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("validated-consequence-map").unwrap(),
        schema,
        [ChromosomeDefinition::new(
            chromosome(),
            vec![ChromosomeLocus::new(
                locus(),
                GeneticMapPositionMicromorgans::new(1),
            )],
        )
        .unwrap()],
    )
    .unwrap()
}

fn state(schema: &HereditarySchema, map: &ChromosomeMap) -> PhasedHereditaryState {
    PhasedHereditaryState::new(
        schema,
        map,
        [PhasedChromosomeState::new(
            chromosome(),
            vec![
                ChromosomeHaplotype::new(vec![allele("a0")]),
                ChromosomeHaplotype::new(vec![allele("a0")]),
            ],
        )],
    )
    .unwrap()
}

fn ancestry_state(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    state: &PhasedHereditaryState,
    first: &str,
    second: &str,
) -> PhasedAncestryState {
    PhasedAncestryState::new(
        schema,
        map,
        state,
        [ChromosomeAncestryState::new(
            chromosome(),
            vec![HaplotypeAncestryClass::new(
                0,
                vec![ancestry(first), ancestry(second)],
            )
            .unwrap()],
        )
        .unwrap()],
    )
    .unwrap()
}

struct Individual {
    state: PhasedHereditaryState,
    ancestry: PhasedAncestryState,
    lineage: MutationLineageState,
    manifest: LinkedIndividualManifest,
}

fn individual(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    id: &str,
    first_copy: &str,
    second_copy: &str,
) -> Individual {
    let state = state(schema, map);
    let ancestry = ancestry_state(schema, map, &state, first_copy, second_copy);
    let lineage = initialize_root_mutation_lineage(schema, map, &state, &ancestry).unwrap();
    let manifest = LinkedIndividualManifest::new(
        individual_id(id),
        schema,
        map,
        &state,
        &ancestry,
        &lineage,
    )
    .unwrap();
    Individual {
        state,
        ancestry,
        lineage,
        manifest,
    }
}

fn subject<'a>(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    individual: &'a Individual,
) -> LinkedIndividualSubject<'a> {
    LinkedIndividualSubject::new(
        &individual.manifest,
        schema,
        map,
        &individual.state,
        &individual.ancestry,
        &individual.lineage,
    )
    .unwrap()
}

fn context(byte: u8, window: &str) -> EvolutionaryContextRef {
    EvolutionaryContextRef::new(
        EvolutionaryContextId::new("validated-context").unwrap(),
        1,
        EvolutionaryContextContentDigest::new([byte; 32]),
        ConsequenceWindowId::new(window).unwrap(),
    )
}

fn viability(value: ViabilityConsequence) -> IndividualConsequences {
    IndividualConsequences {
        viability: Some(value),
        reproductive_events: None,
        descendant_production: None,
        descendant_recruitment: None,
    }
}

#[test]
fn only_complete_current_ledger_can_mint_validated_capability() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let second = individual(&schema, &map, "individual-b", "b-copy-0", "b-copy-1");
    let subjects = [
        subject(&schema, &map, &first),
        subject(&schema, &map, &second),
    ];
    let population = population();
    let census = ExplicitLinkedPopulationCensus::capture(
        population.clone(),
        &schema,
        &map,
        &subjects,
    )
    .unwrap();
    let context = context(7, "window-a");

    let observations = [
        IndividualConsequenceObservation::declare(
            ConsequenceObservationId::new("obs-a").unwrap(),
            individual_id("individual-a"),
            &population,
            &schema,
            &map,
            &census,
            &subjects,
            &context,
            viability(ViabilityConsequence::SurvivedWindow),
        )
        .unwrap(),
        IndividualConsequenceObservation::declare(
            ConsequenceObservationId::new("obs-b").unwrap(),
            individual_id("individual-b"),
            &population,
            &schema,
            &map,
            &census,
            &subjects,
            &context,
            viability(ViabilityConsequence::DiedDuringWindow),
        )
        .unwrap(),
    ];

    let ledger = ExplicitConsequenceLedger::capture(
        &population,
        &schema,
        &map,
        &census,
        &subjects,
        &context,
        observations,
    )
    .unwrap();

    let validated = ValidatedConsequenceLedger::validate_current(
        &ledger,
        &population,
        &schema,
        &map,
        &census,
        &subjects,
        &context,
    )
    .unwrap();

    assert_eq!(validated.ledger_digest(), ledger.canonical_digest().unwrap());
    assert_eq!(validated.population_id(), &population);
    assert_eq!(validated.census_digest(), census.canonical_digest().unwrap());
    assert_eq!(validated.context_digest(), context.canonical_digest().unwrap());

    let mut raw = serde_json::to_value(&ledger).unwrap();
    raw["observations"].as_array_mut().unwrap().pop();
    let partial: ExplicitConsequenceLedger = serde_json::from_value(raw).unwrap();

    assert!(partial.canonical_digest().is_ok());
    assert!(matches!(
        ValidatedConsequenceLedger::validate_current(
            &partial,
            &population,
            &schema,
            &map,
            &census,
            &subjects,
            &context,
        ),
        Err(ConsequenceError::IncompleteCensusCoverage)
    ));
}

#[test]
fn validated_capability_stales_under_context_or_population_drift() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let subjects = [subject(&schema, &map, &first)];
    let population = population();
    let census = ExplicitLinkedPopulationCensus::capture(
        population.clone(),
        &schema,
        &map,
        &subjects,
    )
    .unwrap();
    let context = context(7, "window-a");
    let observation = IndividualConsequenceObservation::declare(
        ConsequenceObservationId::new("obs-a").unwrap(),
        individual_id("individual-a"),
        &population,
        &schema,
        &map,
        &census,
        &subjects,
        &context,
        viability(ViabilityConsequence::SurvivedWindow),
    )
    .unwrap();
    let ledger = ExplicitConsequenceLedger::capture(
        &population,
        &schema,
        &map,
        &census,
        &subjects,
        &context,
        [observation],
    )
    .unwrap();

    assert!(ValidatedConsequenceLedger::validate_current(
        &ledger,
        &population,
        &schema,
        &map,
        &census,
        &subjects,
        &context(8, "window-a"),
    )
    .is_err());

    assert!(ValidatedConsequenceLedger::validate_current(
        &ledger,
        &PopulationId::new("other-pop").unwrap(),
        &schema,
        &map,
        &census,
        &subjects,
        &context,
    )
    .is_err());
}
