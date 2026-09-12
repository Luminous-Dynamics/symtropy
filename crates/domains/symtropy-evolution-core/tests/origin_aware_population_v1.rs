use symtropy_evolution_core::{
    append_descendant_ancestry_to_graph, assemble_diploid_linked_offspring_from_evidence,
    derive_descendant_ancestry, derive_descendant_mutation_lineage,
    derive_modeled_gamete_ancestry, derive_zero_crossover_linked_gamete,
    execute_linked_mutations, initialize_origin_aware_population_from_projection,
    initialize_root_mutation_lineage, neutral_origin_aware_population_step,
    neutral_wright_fisher_step, project_declared_linked_census, AlleleId, AncestryCopyId,
    AncestryGeneration, AncestryGraphNode, ChromosomeAncestryState, ChromosomeDefinition,
    ChromosomeHaplotype, ChromosomeId, ChromosomeLocus, ChromosomeMap, ChromosomeMapId,
    ChromosomeRecombinationDomain, ChromosomeRecombinationModel,
    ChromosomeRecombinationProfile, ChromosomeRecombinationProfileId,
    EvolutionExperimentId, EvolutionOperatorProfile, GeneticMapIntervalMicromorgans,
    GeneticMapPositionMicromorgans, HaplotypeAncestryClass, HereditarySchema,
    HereditarySchemaId, LinkedGameteDerivationEvidence, LocusDefinition, LocusId,
    ModeledAncestryGraph, MutationFateSubject, MutationLineageState, MutationProfile,
    OperatorProfileId, OriginAwarePopulationState, OriginAwarePopulationTransitionResult,
    ParentRole, PhasedAncestryState, PhasedChromosomeState, PhasedHereditaryState,
    PopulationGeneration, PopulationId, PopulationProcessModel, PopulationProcessProfile,
    PopulationProcessProfileId, PopulationTrajectoryPoint, PopulationTransitionId,
    RecombinationMode, RecombinationProfile, ReproductionEventId, PROBABILITY_SCALE_PPM,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn locus() -> LocusId {
    LocusId::new("focal").unwrap()
}

fn chromosome() -> ChromosomeId {
    ChromosomeId::new("chr-origin-aware").unwrap()
}

fn ancestry(id: &str) -> AncestryCopyId {
    AncestryCopyId::new(id).unwrap()
}

fn pos(value: u64) -> GeneticMapPositionMicromorgans {
    GeneticMapPositionMicromorgans::new(value)
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("popgen-05c-schema").unwrap(),
        2,
        [LocusDefinition::new(locus(), [allele("a0"), allele("a1")]).unwrap()],
    )
    .unwrap()
}

fn map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("popgen-05c-map").unwrap(),
        schema,
        [ChromosomeDefinition::new(
            chromosome(),
            vec![ChromosomeLocus::new(locus(), pos(1))],
        )
        .unwrap()],
    )
    .unwrap()
}

fn homozygous_source(
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

fn identical_homolog_ancestry(
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

fn recombination_profile(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    id: &str,
) -> ChromosomeRecombinationProfile {
    ChromosomeRecombinationProfile::new(
        ChromosomeRecombinationProfileId::new(id).unwrap(),
        schema,
        map,
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
        [ChromosomeRecombinationDomain::new(
            chromosome(),
            GeneticMapIntervalMicromorgans::new(pos(0), pos(2)).unwrap(),
        )
        .unwrap()],
    )
    .unwrap()
}

fn mutation_operators(rate_ppm: u32) -> EvolutionOperatorProfile {
    EvolutionOperatorProfile {
        profile_id: OperatorProfileId::new("popgen-05c-mutation").unwrap(),
        version: "1".into(),
        mutation: MutationProfile {
            model_id: "modeled-locus-substitution".into(),
            version: "1".into(),
            per_copy_rate_ppm: rate_ppm,
        },
        recombination: RecombinationProfile {
            model_id: "linked-context".into(),
            version: "1".into(),
            mode: RecombinationMode::IndependentLoci,
        },
    }
}

fn population_profile() -> PopulationProcessProfile {
    PopulationProcessProfile {
        profile_id: PopulationProcessProfileId::new("popgen-05c-neutral").unwrap(),
        version: "1".into(),
        model: PopulationProcessModel::NeutralIndependentLocusWrightFisher,
    }
}

struct RootFixture {
    schema: HereditarySchema,
    map: ChromosomeMap,
    state_a: PhasedHereditaryState,
    state_b: PhasedHereditaryState,
    ancestry_a: PhasedAncestryState,
    ancestry_b: PhasedAncestryState,
    lineage_a: MutationLineageState,
    lineage_b: MutationLineageState,
    profile_a: ChromosomeRecombinationProfile,
    profile_b: ChromosomeRecombinationProfile,
    roots: ModeledAncestryGraph,
}

fn fixture() -> RootFixture {
    let schema = schema();
    let map = map(&schema);
    let state_a = homozygous_source(&schema, &map);
    let state_b = homozygous_source(&schema, &map);
    let ancestry_a = identical_homolog_ancestry(
        &schema,
        &map,
        &state_a,
        "oa-a-0",
        "oa-a-1",
    );
    let ancestry_b = identical_homolog_ancestry(
        &schema,
        &map,
        &state_b,
        "oa-b-0",
        "oa-b-1",
    );
    let lineage_a = initialize_root_mutation_lineage(&schema, &map, &state_a, &ancestry_a).unwrap();
    let lineage_b = initialize_root_mutation_lineage(&schema, &map, &state_b, &ancestry_b).unwrap();
    let profile_a = recombination_profile(&schema, &map, "oa-profile-a");
    let profile_b = recombination_profile(&schema, &map, "oa-profile-b");
    let roots = ModeledAncestryGraph::new_roots(
        &schema,
        &map,
        [
            AncestryGraphNode::root(ancestry("oa-a-0"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("oa-a-1"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("oa-b-0"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("oa-b-1"), chromosome(), AncestryGeneration::new(0)),
        ],
    )
    .unwrap();
    RootFixture {
        schema,
        map,
        state_a,
        state_b,
        ancestry_a,
        ancestry_b,
        lineage_a,
        lineage_b,
        profile_a,
        profile_b,
        roots,
    }
}

struct MutatedChild {
    state: PhasedHereditaryState,
    ancestry: PhasedAncestryState,
    lineage: MutationLineageState,
}

fn recurrent_child(f: &RootFixture, event_name: &str) -> MutatedChild {
    let event = ReproductionEventId::new(event_name).unwrap();
    let gamete_a = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &f.schema,
            &f.map,
            &f.state_a,
            &f.profile_a,
            &event,
            ParentRole::ParentA,
        )
        .unwrap(),
    );
    let gamete_b = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &f.schema,
            &f.map,
            &f.state_b,
            &f.profile_b,
            &event,
            ParentRole::ParentB,
        )
        .unwrap(),
    );
    let gamete_ancestry_a = derive_modeled_gamete_ancestry(
        &f.schema,
        &f.map,
        &f.state_a,
        &f.ancestry_a,
        &f.profile_a,
        &gamete_a,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();
    let gamete_ancestry_b = derive_modeled_gamete_ancestry(
        &f.schema,
        &f.map,
        &f.state_b,
        &f.ancestry_b,
        &f.profile_b,
        &gamete_b,
        &event,
        ParentRole::ParentB,
    )
    .unwrap();
    let offspring = assemble_diploid_linked_offspring_from_evidence(
        &f.schema,
        &f.map,
        &f.state_a,
        &f.profile_a,
        &gamete_a,
        &f.state_b,
        &f.profile_b,
        &gamete_b,
        &event,
    )
    .unwrap();
    let descendant = derive_descendant_ancestry(
        &f.schema,
        &f.map,
        &f.state_a,
        &f.ancestry_a,
        &f.profile_a,
        &gamete_a,
        &gamete_ancestry_a,
        &f.state_b,
        &f.ancestry_b,
        &f.profile_b,
        &gamete_b,
        &gamete_ancestry_b,
        &offspring,
        &event,
    )
    .unwrap();
    let graph = append_descendant_ancestry_to_graph(
        &f.roots,
        &f.schema,
        &f.map,
        &f.state_a,
        &f.ancestry_a,
        &f.profile_a,
        &gamete_a,
        &gamete_ancestry_a,
        &f.state_b,
        &f.ancestry_b,
        &f.profile_b,
        &gamete_b,
        &gamete_ancestry_b,
        &descendant,
        AncestryGeneration::new(1),
    )
    .unwrap()
    .graph;
    let operators = mutation_operators(PROBABILITY_SCALE_PPM);
    let execution = execute_linked_mutations(
        &f.schema,
        &f.map,
        &operators,
        &f.state_a,
        &f.ancestry_a,
        &f.profile_a,
        &gamete_a,
        &gamete_ancestry_a,
        &f.state_b,
        &f.ancestry_b,
        &f.profile_b,
        &gamete_b,
        &gamete_ancestry_b,
        &descendant,
        &graph,
    )
    .unwrap();
    let lineage = derive_descendant_mutation_lineage(
        &f.schema,
        &f.map,
        &operators,
        &f.state_a,
        &f.ancestry_a,
        &f.lineage_a,
        &f.profile_a,
        &gamete_a,
        &gamete_ancestry_a,
        &f.state_b,
        &f.ancestry_b,
        &f.lineage_b,
        &f.profile_b,
        &gamete_b,
        &gamete_ancestry_b,
        &descendant,
        &graph,
        &execution,
    )
    .unwrap()
    .state;
    MutatedChild {
        state: execution.mutated_child,
        ancestry: execution.mutated_child_ancestry,
        lineage,
    }
}

fn start_point(
    schema: &HereditarySchema,
    state: &OriginAwarePopulationState,
    experiment: &str,
) -> PopulationTrajectoryPoint {
    PopulationTrajectoryPoint::declare_reference_start(
        schema,
        &state.population,
        EvolutionExperimentId::new(experiment).unwrap(),
        PopulationGeneration(0),
    )
    .unwrap()
}

#[test]
fn baseline_projection_matches_direct_wright_fisher_and_recurses() {
    let f = fixture();
    let subject = MutationFateSubject::new(&f.state_a, &f.ancestry_a, &f.lineage_a, 8).unwrap();
    let projection = project_declared_linked_census(
        PopulationId::new("oa-baseline-pop").unwrap(),
        &f.schema,
        &f.map,
        &[subject],
    )
    .unwrap();
    let mut state = initialize_origin_aware_population_from_projection(
        &f.schema,
        &f.map,
        &projection,
        &[subject],
    )
    .unwrap();
    state.validate_current(&f.schema).unwrap();

    let provenance = state.allele_provenance(&locus(), &allele("a0")).unwrap();
    assert_eq!(provenance.modeled_baseline_count, 16);
    assert!(provenance.active_origin_counts.is_empty());

    let profile = population_profile();
    let mut point = start_point(&f.schema, &state, "oa-baseline-experiment");
    for generation in 0..8 {
        let transition_id = PopulationTransitionId::new(format!("oa-baseline-{generation}")).unwrap();
        let direct = neutral_wright_fisher_step(
            &f.schema,
            &state.population,
            &point,
            &transition_id,
            &profile,
        )
        .unwrap();
        let result = neutral_origin_aware_population_step(
            &f.schema,
            &state,
            &point,
            &transition_id,
            &profile,
        )
        .unwrap();
        assert_eq!(result.ordinary_transition, direct);
        assert_eq!(result.destination.population, direct.destination);
        let destination_provenance = result
            .destination
            .allele_provenance(&locus(), &allele("a0"))
            .unwrap();
        assert_eq!(
            destination_provenance.modeled_baseline_count,
            direct.destination.allele_copy_counts[&locus()][&allele("a0")]
        );
        assert!(destination_provenance.active_origin_counts.is_empty());

        let encoded = serde_json::to_vec(&result).unwrap();
        let restored: OriginAwarePopulationTransitionResult =
            serde_json::from_slice(&encoded).unwrap();
        restored
            .validate_current(&f.schema, &state, &point, &transition_id, &profile)
            .unwrap();

        point = result.ordinary_transition.destination_point.clone();
        state = result.destination;
    }
}

#[test]
fn recurrent_origins_are_distinct_and_can_go_extinct_without_changing_allele_fate() {
    let f = fixture();
    let child = recurrent_child(&f, "oa-recurrent-child");
    let subject = MutationFateSubject::new(&child.state, &child.ancestry, &child.lineage, 1).unwrap();
    let projection = project_declared_linked_census(
        PopulationId::new("oa-recurrent-pop").unwrap(),
        &f.schema,
        &f.map,
        &[subject],
    )
    .unwrap();
    let source = initialize_origin_aware_population_from_projection(
        &f.schema,
        &f.map,
        &projection,
        &[subject],
    )
    .unwrap();
    let source_partition = source.allele_provenance(&locus(), &allele("a1")).unwrap();
    assert_eq!(source_partition.modeled_baseline_count, 0);
    assert_eq!(source_partition.active_origin_counts.len(), 2);
    assert!(source_partition
        .active_origin_counts
        .iter()
        .all(|origin| origin.count == 1));

    let profile = population_profile();
    let point = start_point(&f.schema, &source, "oa-recurrent-experiment");
    let mut saw_both = false;
    let mut saw_origin_extinction = false;

    for index in 0..256 {
        let transition_id = PopulationTransitionId::new(format!("oa-recurrent-{index}")).unwrap();
        let direct = neutral_wright_fisher_step(
            &f.schema,
            &source.population,
            &point,
            &transition_id,
            &profile,
        )
        .unwrap();
        let result = neutral_origin_aware_population_step(
            &f.schema,
            &source,
            &point,
            &transition_id,
            &profile,
        )
        .unwrap();
        assert_eq!(result.destination.population, direct.destination);
        assert_eq!(direct.destination.allele_copy_counts[&locus()][&allele("a1")], 2);

        let partition = result
            .destination
            .allele_provenance(&locus(), &allele("a1"))
            .unwrap();
        let origin_total: u64 = partition.active_origin_counts.iter().map(|entry| entry.count).sum();
        assert_eq!(partition.modeled_baseline_count + origin_total, 2);
        if partition.active_origin_counts.len() == 2
            && partition.active_origin_counts.iter().all(|entry| entry.count == 1)
        {
            saw_both = true;
        }
        if partition.active_origin_counts.len() == 1
            && partition.active_origin_counts[0].count == 2
        {
            saw_origin_extinction = true;
        }
        if saw_both && saw_origin_extinction {
            break;
        }
    }

    assert!(saw_both, "frozen transition corpus should preserve both recurrent origins at least once");
    assert!(
        saw_origin_extinction,
        "frozen transition corpus should lose one origin while fixed allele a1 survives"
    );
}
