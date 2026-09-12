use symtropy_evolution_core::{
    initialize_root_mutation_lineage, observe_mutation_fates, project_declared_linked_census,
    AlleleId, AncestryCopyId, ChromosomeAncestryState, ChromosomeDefinition,
    ChromosomeHaplotype, ChromosomeId, ChromosomeLocus, ChromosomeMap, ChromosomeMapId,
    EvolutionIndividualId, ExplicitLinkedPopulationCensus, GeneticMapPositionMicromorgans,
    HaplotypeAncestryClass, HereditarySchema, HereditarySchemaId, LinkedIndividualError,
    LinkedIndividualManifest, LinkedIndividualSubject, LocusDefinition, LocusId,
    MutationLineageState, PhasedAncestryState, PhasedChromosomeState, PhasedHereditaryState,
    PopulationId,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn locus() -> LocusId {
    LocusId::new("focal").unwrap()
}

fn chromosome() -> ChromosomeId {
    ChromosomeId::new("chr-individual").unwrap()
}

fn ancestry(id: &str) -> AncestryCopyId {
    AncestryCopyId::new(id).unwrap()
}

fn individual_id(id: &str) -> EvolutionIndividualId {
    EvolutionIndividualId::new(id).unwrap()
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("individual-schema-v1").unwrap(),
        2,
        [LocusDefinition::new(locus(), [allele("a0"), allele("a1")]).unwrap()],
    )
    .unwrap()
}

fn chromosome_map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("individual-map-v1").unwrap(),
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

fn homozygous_state(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
) -> PhasedHereditaryState {
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

fn root_ancestry(
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

struct RootIndividual {
    state: PhasedHereditaryState,
    ancestry: PhasedAncestryState,
    lineage: MutationLineageState,
    manifest: LinkedIndividualManifest,
}

fn root_individual(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    id: &str,
    copy_a: &str,
    copy_b: &str,
) -> RootIndividual {
    let state = homozygous_state(schema, map);
    let ancestry = root_ancestry(schema, map, &state, copy_a, copy_b);
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
    RootIndividual {
        state,
        ancestry,
        lineage,
        manifest,
    }
}

fn subject<'a>(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    individual: &'a RootIndividual,
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

#[test]
fn identical_genomes_do_not_collapse_distinct_individual_identity() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = root_individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let second = root_individual(&schema, &map, "individual-b", "b-copy-0", "b-copy-1");

    assert_eq!(first.state, second.state);
    assert_ne!(first.ancestry, second.ancestry);
    assert_ne!(first.manifest.individual_id, second.manifest.individual_id);
    assert_ne!(
        first.manifest.canonical_digest(),
        second.manifest.canonical_digest()
    );
}

#[test]
fn same_individual_id_cannot_silently_rebind_changed_current_authority() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = root_individual(&schema, &map, "stable-individual", "a-copy-0", "a-copy-1");
    let changed = root_individual(&schema, &map, "other-id", "b-copy-0", "b-copy-1");

    assert!(matches!(
        first.manifest.validate_current(
            &schema,
            &map,
            &changed.state,
            &changed.ancestry,
            &changed.lineage,
        ),
        Err(LinkedIndividualError::ManifestAuthorityMismatch)
    ));
}

#[test]
fn malformed_restored_individual_id_fails_before_manifest_authority() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let individual = root_individual(&schema, &map, "wire-safe", "copy-0", "copy-1");

    let mut raw = serde_json::to_value(&individual.manifest).unwrap();
    raw["individual_id"] = serde_json::Value::String("   ".into());
    assert!(serde_json::from_value::<LinkedIndividualManifest>(raw).is_err());
}

#[test]
fn census_is_order_invariant_and_revalidatable_after_serde_restore() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = root_individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let second = root_individual(&schema, &map, "individual-b", "b-copy-0", "b-copy-1");
    let first_subject = subject(&schema, &map, &first);
    let second_subject = subject(&schema, &map, &second);

    let forward = ExplicitLinkedPopulationCensus::capture(
        PopulationId::new("population-a").unwrap(),
        &schema,
        &map,
        &[first_subject, second_subject],
    )
    .unwrap();
    let reverse = ExplicitLinkedPopulationCensus::capture(
        PopulationId::new("population-a").unwrap(),
        &schema,
        &map,
        &[second_subject, first_subject],
    )
    .unwrap();

    assert_eq!(forward, reverse);
    assert_eq!(forward.census_size(), 2);
    assert_eq!(forward.canonical_digest().unwrap(), reverse.canonical_digest().unwrap());

    let encoded = serde_json::to_vec(&forward).unwrap();
    let restored: ExplicitLinkedPopulationCensus = serde_json::from_slice(&encoded).unwrap();
    restored
        .validate_current(&schema, &map, &[second_subject, first_subject])
        .unwrap();
}

#[test]
fn duplicate_individual_identity_fails_even_when_current_states_are_distinct() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = root_individual(&schema, &map, "duplicate-id", "a-copy-0", "a-copy-1");
    let mut second = root_individual(&schema, &map, "temporary-id", "b-copy-0", "b-copy-1");
    second.manifest = LinkedIndividualManifest::new(
        individual_id("duplicate-id"),
        &schema,
        &map,
        &second.state,
        &second.ancestry,
        &second.lineage,
    )
    .unwrap();

    let first_subject = subject(&schema, &map, &first);
    let second_subject = subject(&schema, &map, &second);
    assert!(matches!(
        ExplicitLinkedPopulationCensus::capture(
            PopulationId::new("population-a").unwrap(),
            &schema,
            &map,
            &[first_subject, second_subject],
        ),
        Err(LinkedIndividualError::DuplicateIndividualId(_))
    ));
}

#[test]
fn distinct_individuals_cannot_double_own_one_persistent_ancestry_copy() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = root_individual(&schema, &map, "individual-a", "shared-0", "shared-1");
    let mut second = root_individual(&schema, &map, "individual-b", "other-0", "other-1");
    second.state = first.state.clone();
    second.ancestry = first.ancestry.clone();
    second.lineage = first.lineage.clone();
    second.manifest = LinkedIndividualManifest::new(
        individual_id("individual-b"),
        &schema,
        &map,
        &second.state,
        &second.ancestry,
        &second.lineage,
    )
    .unwrap();

    let first_subject = subject(&schema, &map, &first);
    let second_subject = subject(&schema, &map, &second);
    assert!(matches!(
        ExplicitLinkedPopulationCensus::capture(
            PopulationId::new("population-a").unwrap(),
            &schema,
            &map,
            &[first_subject, second_subject],
        ),
        Err(LinkedIndividualError::DuplicateAncestryCopyOwnership { .. })
    ));
}

#[test]
fn changed_member_set_stales_a_restored_census() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = root_individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let second = root_individual(&schema, &map, "individual-b", "b-copy-0", "b-copy-1");
    let replacement = root_individual(&schema, &map, "individual-c", "c-copy-0", "c-copy-1");
    let first_subject = subject(&schema, &map, &first);
    let second_subject = subject(&schema, &map, &second);
    let replacement_subject = subject(&schema, &map, &replacement);

    let census = ExplicitLinkedPopulationCensus::capture(
        PopulationId::new("population-a").unwrap(),
        &schema,
        &map,
        &[first_subject, second_subject],
    )
    .unwrap();
    assert!(matches!(
        census.validate_current(&schema, &map, &[first_subject, replacement_subject]),
        Err(LinkedIndividualError::CensusMemberMismatch)
    ));
}

#[test]
fn population_identity_changes_census_identity_without_changing_members() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let individual = root_individual(&schema, &map, "individual-a", "copy-0", "copy-1");
    let individual_subject = subject(&schema, &map, &individual);

    let first = ExplicitLinkedPopulationCensus::capture(
        PopulationId::new("population-a").unwrap(),
        &schema,
        &map,
        &[individual_subject],
    )
    .unwrap();
    let second = ExplicitLinkedPopulationCensus::capture(
        PopulationId::new("population-b").unwrap(),
        &schema,
        &map,
        &[individual_subject],
    )
    .unwrap();

    assert_ne!(first.canonical_digest().unwrap(), second.canonical_digest().unwrap());
}

#[test]
fn mutation_fate_conversion_preserves_exact_explicit_copy_counts() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = root_individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let second = root_individual(&schema, &map, "individual-b", "b-copy-0", "b-copy-1");
    let subjects = [subject(&schema, &map, &first), subject(&schema, &map, &second)];
    let census = ExplicitLinkedPopulationCensus::capture(
        PopulationId::new("population-a").unwrap(),
        &schema,
        &map,
        &subjects,
    )
    .unwrap();
    let fate_subjects = census
        .mutation_fate_subjects(&schema, &map, &subjects)
        .unwrap();
    let observation = observe_mutation_fates(&schema, &map, &fate_subjects).unwrap();

    assert_eq!(observation.loci.len(), 1);
    let observed = &observation.loci[0];
    assert_eq!(observed.total_copy_count, 4);
    assert_eq!(observed.modeled_baseline_count, 4);
    assert!(observed.origin_counts.is_empty());
    assert_eq!(observed.allele_counts.len(), 1);
    assert_eq!(observed.allele_counts[0].allele_id, allele("a0"));
    assert_eq!(observed.allele_counts[0].count, 4);
}

#[test]
fn aggregate_population_cannot_reconstruct_explicit_individual_membership() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = root_individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let second = root_individual(&schema, &map, "individual-b", "b-copy-0", "b-copy-1");
    let first_subjects = [subject(&schema, &map, &first)];
    let second_subjects = [subject(&schema, &map, &second)];

    let first_census = ExplicitLinkedPopulationCensus::capture(
        PopulationId::new("population-a").unwrap(),
        &schema,
        &map,
        &first_subjects,
    )
    .unwrap();
    let second_census = ExplicitLinkedPopulationCensus::capture(
        PopulationId::new("population-a").unwrap(),
        &schema,
        &map,
        &second_subjects,
    )
    .unwrap();

    let first_fate = first_census
        .mutation_fate_subjects(&schema, &map, &first_subjects)
        .unwrap();
    let second_fate = second_census
        .mutation_fate_subjects(&schema, &map, &second_subjects)
        .unwrap();
    let first_projection = project_declared_linked_census(
        PopulationId::new("population-a").unwrap(),
        &schema,
        &map,
        &first_fate,
    )
    .unwrap();
    let second_projection = project_declared_linked_census(
        PopulationId::new("population-a").unwrap(),
        &schema,
        &map,
        &second_fate,
    )
    .unwrap();

    assert_eq!(
        first_projection.aggregate_population,
        second_projection.aggregate_population
    );
    assert_ne!(
        first_census.canonical_digest().unwrap(),
        second_census.canonical_digest().unwrap()
    );
}
