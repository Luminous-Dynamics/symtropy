use symtropy_evolution_core::{
    initialize_root_mutation_lineage, AlleleId, AncestryCopyId, ChromosomeAncestryState,
    ChromosomeDefinition, ChromosomeHaplotype, ChromosomeId, ChromosomeLocus, ChromosomeMap,
    ChromosomeMapId, ConsequenceError, ConsequenceObservationId, ConsequenceWindowId,
    DescendantProductionConsequence, DescendantRecruitmentConsequence, EvolutionIndividualId,
    EvolutionaryContextContentDigest, EvolutionaryContextId, EvolutionaryContextRef,
    ExplicitConsequenceLedger, ExplicitLinkedPopulationCensus, GeneticMapPositionMicromorgans,
    HaplotypeAncestryClass, HereditarySchema, HereditarySchemaId,
    IndividualConsequenceObservation, IndividualConsequences, LinkedIndividualManifest,
    LinkedIndividualSubject, LocusDefinition, LocusId, MutationLineageState, PhasedAncestryState,
    PhasedChromosomeState, PhasedHereditaryState, PopulationId, ReproductiveEventConsequence,
    ViabilityConsequence,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn locus() -> LocusId {
    LocusId::new("focal").unwrap()
}

fn chromosome() -> ChromosomeId {
    ChromosomeId::new("chr-consequence").unwrap()
}

fn ancestry(id: &str) -> AncestryCopyId {
    AncestryCopyId::new(id).unwrap()
}

fn individual_id(id: &str) -> EvolutionIndividualId {
    EvolutionIndividualId::new(id).unwrap()
}

fn population() -> PopulationId {
    PopulationId::new("consequence-pop").unwrap()
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("consequence-schema").unwrap(),
        2,
        [LocusDefinition::new(locus(), [allele("a0"), allele("a1")]).unwrap()],
    )
    .unwrap()
}

fn chromosome_map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("consequence-map").unwrap(),
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

fn context(id: &str, revision: u64, byte: u8, window: &str) -> EvolutionaryContextRef {
    EvolutionaryContextRef::new(
        EvolutionaryContextId::new(id).unwrap(),
        revision,
        EvolutionaryContextContentDigest::new([byte; 32]),
        ConsequenceWindowId::new(window).unwrap(),
    )
}

fn survival_only(value: ViabilityConsequence) -> IndividualConsequences {
    IndividualConsequences {
        viability: Some(value),
        reproductive_events: None,
        descendant_production: None,
        descendant_recruitment: None,
    }
}

fn full_consequences(
    viability: ViabilityConsequence,
    opportunities: u64,
    realized: u64,
    produced: u64,
    recruited: u64,
) -> IndividualConsequences {
    IndividualConsequences {
        viability: Some(viability),
        reproductive_events: Some(ReproductiveEventConsequence {
            opportunities_observed: opportunities,
            realized_events: realized,
        }),
        descendant_production: Some(DescendantProductionConsequence { produced }),
        descendant_recruitment: Some(DescendantRecruitmentConsequence { recruited }),
    }
}

struct Fixture {
    schema: HereditarySchema,
    map: ChromosomeMap,
    first: Individual,
    second: Individual,
}

fn fixture() -> Fixture {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let second = individual(&schema, &map, "individual-b", "b-copy-0", "b-copy-1");
    Fixture {
        schema,
        map,
        first,
        second,
    }
}

#[test]
fn identical_genomes_can_have_different_observed_consequences_without_relabeling_genetics() {
    let f = fixture();
    assert_eq!(f.first.state, f.second.state);
    let subjects = [
        subject(&f.schema, &f.map, &f.first),
        subject(&f.schema, &f.map, &f.second),
    ];
    let population = population();
    let census = ExplicitLinkedPopulationCensus::capture(
        population.clone(),
        &f.schema,
        &f.map,
        &subjects,
    )
    .unwrap();
    let context = context("dry-season", 1, 7, "window-1");
    let first = IndividualConsequenceObservation::declare(
        ConsequenceObservationId::new("obs-a").unwrap(),
        individual_id("individual-a"),
        &population,
        &f.schema,
        &f.map,
        &census,
        &subjects,
        &context,
        full_consequences(ViabilityConsequence::SurvivedWindow, 3, 2, 4, 3),
    )
    .unwrap();
    let second = IndividualConsequenceObservation::declare(
        ConsequenceObservationId::new("obs-b").unwrap(),
        individual_id("individual-b"),
        &population,
        &f.schema,
        &f.map,
        &census,
        &subjects,
        &context,
        full_consequences(ViabilityConsequence::DiedDuringWindow, 1, 0, 0, 0),
    )
    .unwrap();

    assert_ne!(first.canonical_digest().unwrap(), second.canonical_digest().unwrap());
    assert_eq!(f.first.state, f.second.state);
}

#[test]
fn missing_channel_is_not_observed_zero() {
    let f = fixture();
    let subjects = [subject(&f.schema, &f.map, &f.first)];
    let population = population();
    let census = ExplicitLinkedPopulationCensus::capture(
        population.clone(),
        &f.schema,
        &f.map,
        &subjects,
    )
    .unwrap();
    let context = context("context", 1, 1, "window");

    let missing = IndividualConsequenceObservation::declare(
        ConsequenceObservationId::new("missing").unwrap(),
        individual_id("individual-a"),
        &population,
        &f.schema,
        &f.map,
        &census,
        &subjects,
        &context,
        survival_only(ViabilityConsequence::SurvivedWindow),
    )
    .unwrap();
    let observed_zero = IndividualConsequenceObservation::declare(
        ConsequenceObservationId::new("zero").unwrap(),
        individual_id("individual-a"),
        &population,
        &f.schema,
        &f.map,
        &census,
        &subjects,
        &context,
        IndividualConsequences {
            viability: Some(ViabilityConsequence::SurvivedWindow),
            reproductive_events: Some(ReproductiveEventConsequence {
                opportunities_observed: 0,
                realized_events: 0,
            }),
            descendant_production: None,
            descendant_recruitment: None,
        },
    )
    .unwrap();

    assert!(missing.consequences.reproductive_events.is_none());
    assert!(observed_zero.consequences.reproductive_events.is_some());
    assert_ne!(missing.canonical_digest().unwrap(), observed_zero.canonical_digest().unwrap());
}

#[test]
fn malformed_consequence_partitions_fail_closed() {
    assert!(matches!(
        IndividualConsequences {
            viability: None,
            reproductive_events: None,
            descendant_production: None,
            descendant_recruitment: None,
        }
        .validate(),
        Err(ConsequenceError::NoObservedChannels)
    ));
    assert!(matches!(
        IndividualConsequences {
            viability: None,
            reproductive_events: Some(ReproductiveEventConsequence {
                opportunities_observed: 1,
                realized_events: 2,
            }),
            descendant_production: None,
            descendant_recruitment: None,
        }
        .validate(),
        Err(ConsequenceError::ReproductiveEventsExceedOpportunities)
    ));
    assert!(matches!(
        IndividualConsequences {
            viability: None,
            reproductive_events: None,
            descendant_production: Some(DescendantProductionConsequence { produced: 1 }),
            descendant_recruitment: Some(DescendantRecruitmentConsequence { recruited: 2 }),
        }
        .validate(),
        Err(ConsequenceError::RecruitmentExceedsProduction)
    ));
}

#[test]
fn observation_replay_binds_exact_context_window_population_and_manifest() {
    let f = fixture();
    let subjects = [subject(&f.schema, &f.map, &f.first)];
    let population = population();
    let census = ExplicitLinkedPopulationCensus::capture(
        population.clone(),
        &f.schema,
        &f.map,
        &subjects,
    )
    .unwrap();
    let context_a = context("context", 1, 3, "window-a");
    let observation = IndividualConsequenceObservation::declare(
        ConsequenceObservationId::new("obs").unwrap(),
        individual_id("individual-a"),
        &population,
        &f.schema,
        &f.map,
        &census,
        &subjects,
        &context_a,
        survival_only(ViabilityConsequence::SurvivedWindow),
    )
    .unwrap();

    observation
        .validate_current(
            &population,
            &f.schema,
            &f.map,
            &census,
            &subjects,
            &context_a,
        )
        .unwrap();

    for changed in [
        context("context", 2, 3, "window-a"),
        context("context", 1, 4, "window-a"),
        context("context", 1, 3, "window-b"),
    ] {
        assert!(observation
            .validate_current(
                &population,
                &f.schema,
                &f.map,
                &census,
                &subjects,
                &changed,
            )
            .is_err());
    }
    assert!(observation
        .validate_current(
            &PopulationId::new("other-pop").unwrap(),
            &f.schema,
            &f.map,
            &census,
            &subjects,
            &context_a,
        )
        .is_err());

    let mut raw = serde_json::to_value(&observation).unwrap();
    raw["individual_manifest_digest"] = serde_json::json!([0; 32]);
    let restored: IndividualConsequenceObservation = serde_json::from_value(raw).unwrap();
    assert!(matches!(
        restored.validate_current(
            &population,
            &f.schema,
            &f.map,
            &census,
            &subjects,
            &context_a,
        ),
        Err(ConsequenceError::ObservationReplayMismatch)
    ));
}

fn pair_observations(
    f: &Fixture,
    subjects: &[LinkedIndividualSubject<'_>],
    census: &ExplicitLinkedPopulationCensus,
    context: &EvolutionaryContextRef,
) -> [IndividualConsequenceObservation; 2] {
    let population = population();
    [
        IndividualConsequenceObservation::declare(
            ConsequenceObservationId::new("obs-a").unwrap(),
            individual_id("individual-a"),
            &population,
            &f.schema,
            &f.map,
            census,
            subjects,
            context,
            survival_only(ViabilityConsequence::SurvivedWindow),
        )
        .unwrap(),
        IndividualConsequenceObservation::declare(
            ConsequenceObservationId::new("obs-b").unwrap(),
            individual_id("individual-b"),
            &population,
            &f.schema,
            &f.map,
            census,
            subjects,
            context,
            survival_only(ViabilityConsequence::DiedDuringWindow),
        )
        .unwrap(),
    ]
}

#[test]
fn complete_ledger_is_order_invariant_and_restorable() {
    let f = fixture();
    let subjects = [
        subject(&f.schema, &f.map, &f.first),
        subject(&f.schema, &f.map, &f.second),
    ];
    let population = population();
    let census = ExplicitLinkedPopulationCensus::capture(
        population.clone(),
        &f.schema,
        &f.map,
        &subjects,
    )
    .unwrap();
    let context = context("context", 1, 8, "window");
    let observations = pair_observations(&f, &subjects, &census, &context);

    let forward = ExplicitConsequenceLedger::capture(
        &population,
        &f.schema,
        &f.map,
        &census,
        &subjects,
        &context,
        observations.clone(),
    )
    .unwrap();
    let reverse = ExplicitConsequenceLedger::capture(
        &population,
        &f.schema,
        &f.map,
        &census,
        &subjects,
        &context,
        observations.into_iter().rev(),
    )
    .unwrap();
    assert_eq!(forward, reverse);
    assert_eq!(forward.canonical_digest().unwrap(), reverse.canonical_digest().unwrap());

    let encoded = serde_json::to_vec(&forward).unwrap();
    let restored: ExplicitConsequenceLedger = serde_json::from_slice(&encoded).unwrap();
    restored
        .validate_current(
            &population,
            &f.schema,
            &f.map,
            &census,
            &subjects,
            &context,
        )
        .unwrap();
}

#[test]
fn ledger_rejects_duplicate_or_incomplete_census_observations() {
    let f = fixture();
    let subjects = [
        subject(&f.schema, &f.map, &f.first),
        subject(&f.schema, &f.map, &f.second),
    ];
    let population = population();
    let census = ExplicitLinkedPopulationCensus::capture(
        population.clone(),
        &f.schema,
        &f.map,
        &subjects,
    )
    .unwrap();
    let context = context("context", 1, 9, "window");
    let observations = pair_observations(&f, &subjects, &census, &context);

    assert!(matches!(
        ExplicitConsequenceLedger::capture(
            &population,
            &f.schema,
            &f.map,
            &census,
            &subjects,
            &context,
            [observations[0].clone()],
        ),
        Err(ConsequenceError::IncompleteCensusCoverage)
    ));

    let duplicate_individual = IndividualConsequenceObservation::declare(
        ConsequenceObservationId::new("obs-a-second").unwrap(),
        individual_id("individual-a"),
        &population,
        &f.schema,
        &f.map,
        &census,
        &subjects,
        &context,
        survival_only(ViabilityConsequence::SurvivedWindow),
    )
    .unwrap();
    assert!(matches!(
        ExplicitConsequenceLedger::capture(
            &population,
            &f.schema,
            &f.map,
            &census,
            &subjects,
            &context,
            [observations[0].clone(), duplicate_individual],
        ),
        Err(ConsequenceError::DuplicateIndividualObservation)
    ));

    let duplicate_id_for_second = IndividualConsequenceObservation::declare(
        ConsequenceObservationId::new("obs-a").unwrap(),
        individual_id("individual-b"),
        &population,
        &f.schema,
        &f.map,
        &census,
        &subjects,
        &context,
        survival_only(ViabilityConsequence::DiedDuringWindow),
    )
    .unwrap();
    assert!(matches!(
        ExplicitConsequenceLedger::capture(
            &population,
            &f.schema,
            &f.map,
            &census,
            &subjects,
            &context,
            [observations[0].clone(), duplicate_id_for_second],
        ),
        Err(ConsequenceError::DuplicateObservationId(_))
    ));
}

#[test]
fn same_numeric_consequences_under_different_contexts_have_distinct_identity() {
    let f = fixture();
    let subjects = [subject(&f.schema, &f.map, &f.first)];
    let population = population();
    let census = ExplicitLinkedPopulationCensus::capture(
        population.clone(),
        &f.schema,
        &f.map,
        &subjects,
    )
    .unwrap();
    let consequences = full_consequences(ViabilityConsequence::SurvivedWindow, 2, 1, 2, 1);
    let first = IndividualConsequenceObservation::declare(
        ConsequenceObservationId::new("obs").unwrap(),
        individual_id("individual-a"),
        &population,
        &f.schema,
        &f.map,
        &census,
        &subjects,
        &context("context-a", 1, 1, "window"),
        consequences.clone(),
    )
    .unwrap();
    let second = IndividualConsequenceObservation::declare(
        ConsequenceObservationId::new("obs").unwrap(),
        individual_id("individual-a"),
        &population,
        &f.schema,
        &f.map,
        &census,
        &subjects,
        &context("context-b", 1, 2, "window"),
        consequences,
    )
    .unwrap();
    assert_ne!(first.canonical_digest().unwrap(), second.canonical_digest().unwrap());
}

#[test]
fn consequence_wire_shape_contains_no_genetic_or_fitness_claim_fields() {
    let consequences = full_consequences(ViabilityConsequence::SurvivedWindow, 2, 1, 2, 1);
    let value = serde_json::to_value(consequences).unwrap();
    let object = value.as_object().unwrap();
    for forbidden in [
        "fitness",
        "selection_coefficient",
        "beneficial",
        "deleterious",
        "allele",
        "genotype",
        "phenotype",
    ] {
        assert!(!object.contains_key(forbidden));
    }
}
