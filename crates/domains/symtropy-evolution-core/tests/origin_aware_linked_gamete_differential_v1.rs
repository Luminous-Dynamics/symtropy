use symtropy_evolution_core::{
    append_descendant_ancestry_to_graph, assemble_diploid_linked_offspring_from_evidence,
    derive_descendant_ancestry, derive_descendant_mutation_lineage,
    derive_modeled_gamete_ancestry, derive_zero_crossover_linked_gamete,
    execute_linked_mutations, initialize_origin_aware_population_from_projection,
    initialize_root_mutation_lineage, neutral_origin_aware_population_step,
    project_declared_linked_census, AlleleId, AncestryCopyId, AncestryGeneration,
    AncestryGraphNode, ChromosomeAncestryState, ChromosomeDefinition, ChromosomeHaplotype,
    ChromosomeId, ChromosomeLocus, ChromosomeMap, ChromosomeMapId,
    ChromosomeRecombinationDomain, ChromosomeRecombinationModel,
    ChromosomeRecombinationProfile, ChromosomeRecombinationProfileId, EvolutionExperimentId,
    EvolutionOperatorProfile, GeneticMapIntervalMicromorgans, GeneticMapPositionMicromorgans,
    HaplotypeAncestryClass, HereditarySchema, HereditarySchemaId,
    LinkedGameteDerivationEvidence, LocusDefinition, LocusId, ModeledAncestryGraph,
    MutationFateSubject, MutationLineageState, MutationOriginDigest, MutationProfile,
    OperatorProfileId, OriginAwarePopulationState, ParentRole, PhasedAncestryState,
    PhasedChromosomeState, PhasedHereditaryState, PopulationGeneration, PopulationId,
    PopulationProcessModel, PopulationProcessProfile, PopulationProcessProfileId,
    PopulationTrajectoryPoint, PopulationTransitionId, RecombinationMode, RecombinationProfile,
    ReproductionEventId, PROBABILITY_SCALE_PPM,
};

const REPLICATES: usize = 4_096;
const ANALYTICAL_TOLERANCE: f64 = 0.04;
const PRODUCTION_EXPLICIT_TV_TOLERANCE: f64 = 0.06;

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn locus() -> LocusId {
    LocusId::new("focal").unwrap()
}

fn chromosome() -> ChromosomeId {
    ChromosomeId::new("chr-explicit-origin").unwrap()
}

fn ancestry(id: &str) -> AncestryCopyId {
    AncestryCopyId::new(id).unwrap()
}

fn pos(value: u64) -> GeneticMapPositionMicromorgans {
    GeneticMapPositionMicromorgans::new(value)
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("popgen-05e3-schema").unwrap(),
        2,
        [LocusDefinition::new(locus(), [allele("a0"), allele("a1")]).unwrap()],
    )
    .unwrap()
}

fn map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("popgen-05e3-map").unwrap(),
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

fn mutation_operators() -> EvolutionOperatorProfile {
    EvolutionOperatorProfile {
        profile_id: OperatorProfileId::new("popgen-05e3-mutation").unwrap(),
        version: "1".into(),
        mutation: MutationProfile {
            model_id: "modeled-locus-substitution".into(),
            version: "1".into(),
            per_copy_rate_ppm: PROBABILITY_SCALE_PPM,
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
        profile_id: PopulationProcessProfileId::new("popgen-05e3-neutral").unwrap(),
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
        "e3-a-0",
        "e3-a-1",
    );
    let ancestry_b = identical_homolog_ancestry(
        &schema,
        &map,
        &state_b,
        "e3-b-0",
        "e3-b-1",
    );
    let lineage_a = initialize_root_mutation_lineage(&schema, &map, &state_a, &ancestry_a).unwrap();
    let lineage_b = initialize_root_mutation_lineage(&schema, &map, &state_b, &ancestry_b).unwrap();
    let profile_a = recombination_profile(&schema, &map, "e3-profile-a");
    let profile_b = recombination_profile(&schema, &map, "e3-profile-b");
    let roots = ModeledAncestryGraph::new_roots(
        &schema,
        &map,
        [
            AncestryGraphNode::root(
                ancestry("e3-a-0"),
                chromosome(),
                AncestryGeneration::new(0),
            ),
            AncestryGraphNode::root(
                ancestry("e3-a-1"),
                chromosome(),
                AncestryGeneration::new(0),
            ),
            AncestryGraphNode::root(
                ancestry("e3-b-0"),
                chromosome(),
                AncestryGeneration::new(0),
            ),
            AncestryGraphNode::root(
                ancestry("e3-b-1"),
                chromosome(),
                AncestryGeneration::new(0),
            ),
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

struct MutatedParent {
    state: PhasedHereditaryState,
    ancestry: PhasedAncestryState,
    lineage: MutationLineageState,
}

fn recurrent_mutated_parent(f: &RootFixture) -> MutatedParent {
    let event = ReproductionEventId::new("popgen-05e3-parent-birth").unwrap();
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
    let operators = mutation_operators();
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

    MutatedParent {
        state: execution.mutated_child,
        ancestry: execution.mutated_child_ancestry,
        lineage,
    }
}

fn active_origins(parent: &MutatedParent) -> [MutationOriginDigest; 2] {
    let mut origins: Vec<_> = parent
        .lineage
        .entries
        .iter()
        .filter(|entry| entry.locus_id == locus())
        .map(|entry| entry.active_origin.expect("both child copies must be mutated"))
        .collect();
    origins.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    origins.dedup();
    assert_eq!(origins.len(), 2);
    [origins[0], origins[1]]
}

fn transmitted_origin(
    f: &RootFixture,
    parent: &MutatedParent,
    profile: &ChromosomeRecombinationProfile,
    event_name: &str,
) -> MutationOriginDigest {
    let event = ReproductionEventId::new(event_name).unwrap();
    let gamete = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &f.schema,
            &f.map,
            &parent.state,
            profile,
            &event,
            ParentRole::ParentA,
        )
        .unwrap(),
    );
    let ancestry = derive_modeled_gamete_ancestry(
        &f.schema,
        &f.map,
        &parent.state,
        &parent.ancestry,
        profile,
        &gamete,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();
    let source_copy = &ancestry.ancestry.chromosomes[&chromosome()].loci[0].source_copy_id;
    parent
        .lineage
        .entry(source_copy, &locus())
        .and_then(|entry| entry.active_origin)
        .expect("explicit linked gamete must resolve to one active origin")
}

fn start_point(
    schema: &HereditarySchema,
    state: &OriginAwarePopulationState,
) -> PopulationTrajectoryPoint {
    PopulationTrajectoryPoint::declare_reference_start(
        schema,
        &state.population,
        EvolutionExperimentId::new("popgen-05e3-aggregate").unwrap(),
        PopulationGeneration(0),
    )
    .unwrap()
}

fn total_variation(left: &[u64; 3], right: &[u64; 3]) -> f64 {
    let n = REPLICATES as f64;
    0.5 * left
        .iter()
        .zip(right)
        .map(|(left, right)| (*left as f64 / n - *right as f64 / n).abs())
        .sum::<f64>()
}

#[test]
fn explicit_linked_gamete_pairs_match_origin_aware_two_copy_neutral_null() {
    let f = fixture();
    let parent = recurrent_mutated_parent(&f);
    parent
        .lineage
        .validate_current(&f.schema, &f.map, &parent.state, &parent.ancestry)
        .unwrap();
    let origins = active_origins(&parent);

    let subject = MutationFateSubject::new(&parent.state, &parent.ancestry, &parent.lineage, 1)
        .unwrap();
    let projection = project_declared_linked_census(
        PopulationId::new("popgen-05e3-pop").unwrap(),
        &f.schema,
        &f.map,
        &[subject],
    )
    .unwrap();
    let aggregate = initialize_origin_aware_population_from_projection(
        &f.schema,
        &f.map,
        &projection,
        &[subject],
    )
    .unwrap();
    let aggregate_provenance = aggregate
        .allele_provenance(&locus(), &allele("a1"))
        .unwrap();
    assert_eq!(aggregate_provenance.modeled_baseline_count, 0);
    assert_eq!(aggregate_provenance.active_origin_counts.len(), 2);
    assert!(aggregate_provenance
        .active_origin_counts
        .iter()
        .all(|entry| entry.count == 1));

    let point = start_point(&f.schema, &aggregate);
    let child_profile = recombination_profile(&f.schema, &f.map, "e3-child-profile");
    let process = population_profile();
    let mut explicit_histogram = [0_u64; 3];
    let mut aggregate_histogram = [0_u64; 3];

    for replicate in 0..REPLICATES {
        let first = transmitted_origin(
            &f,
            &parent,
            &child_profile,
            &format!("e3-explicit-{replicate}-a"),
        );
        let second = transmitted_origin(
            &f,
            &parent,
            &child_profile,
            &format!("e3-explicit-{replicate}-b"),
        );
        assert!(origins.contains(&first));
        assert!(origins.contains(&second));
        let explicit_first_count = if first == origins[0] { 1_u64 } else { 0_u64 }
            + if second == origins[0] { 1_u64 } else { 0_u64 };
        explicit_histogram[explicit_first_count as usize] += 1;

        let result = neutral_origin_aware_population_step(
            &f.schema,
            &aggregate,
            &point,
            &PopulationTransitionId::new(format!("e3-aggregate-{replicate}")).unwrap(),
            &process,
        )
        .unwrap();
        let aggregate_first_count = result
            .destination
            .allele_provenance(&locus(), &allele("a1"))
            .unwrap()
            .active_origin_counts
            .iter()
            .find(|entry| entry.origin_digest == origins[0])
            .map(|entry| entry.count)
            .unwrap_or(0);
        aggregate_histogram[aggregate_first_count as usize] += 1;
    }

    let n = REPLICATES as f64;
    for (index, expected) in [(0, 0.25), (1, 0.50), (2, 0.25)] {
        let explicit = explicit_histogram[index] as f64 / n;
        let aggregate = aggregate_histogram[index] as f64 / n;
        assert!(
            (explicit - expected).abs() <= ANALYTICAL_TOLERANCE,
            "explicit linked-gamete P(N={index})={explicit} diverged from {expected}"
        );
        assert!(
            (aggregate - expected).abs() <= ANALYTICAL_TOLERANCE,
            "aggregate P(N={index})={aggregate} diverged from {expected}"
        );
    }

    let distance = total_variation(&explicit_histogram, &aggregate_histogram);
    assert!(
        distance <= PRODUCTION_EXPLICIT_TV_TOLERANCE,
        "production-vs-explicit total variation {distance} exceeded frozen tolerance {PRODUCTION_EXPLICIT_TV_TOLERANCE}"
    );
}
