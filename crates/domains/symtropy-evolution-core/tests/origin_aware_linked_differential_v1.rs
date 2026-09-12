use std::collections::BTreeMap;

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
    OperatorProfileId, ParentRole, PhasedAncestryState, PhasedChromosomeState,
    PhasedHereditaryState, PopulationGeneration, PopulationId, PopulationProcessModel,
    PopulationProcessProfile, PopulationProcessProfileId, PopulationTrajectoryPoint,
    PopulationTransitionId, RecombinationMode, RecombinationProfile, ReproductionEventId,
    PROBABILITY_SCALE_PPM,
};

const EXPLICIT_REPLICATES: usize = 1_024;
const AGGREGATE_REPLICATES: usize = 4_096;
const MATCHED_TV_MAX: f64 = 0.08;
const BINOMIAL_ABS_MAX: f64 = 0.06;
const STRUCTURED_TV_MIN: f64 = 0.35;

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn locus() -> LocusId {
    LocusId::new("focal").unwrap()
}

fn chromosome() -> ChromosomeId {
    ChromosomeId::new("chr-popgen-05e3").unwrap()
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

fn homozygous_source(schema: &HereditarySchema, map: &ChromosomeMap) -> PhasedHereditaryState {
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
        profile_id: OperatorProfileId::new("popgen-05e3-mutation").unwrap(),
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
        profile_id: PopulationProcessProfileId::new("popgen-05e3-neutral").unwrap(),
        version: "1".into(),
        model: PopulationProcessModel::NeutralIndependentLocusWrightFisher,
    }
}

struct SubjectState {
    state: PhasedHereditaryState,
    ancestry: PhasedAncestryState,
    lineage: MutationLineageState,
    graph: ModeledAncestryGraph,
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
    let ancestry_a = root_ancestry(&schema, &map, &state_a, "e3-a-0", "e3-a-1");
    let ancestry_b = root_ancestry(&schema, &map, &state_b, "e3-b-0", "e3-b-1");
    let lineage_a = initialize_root_mutation_lineage(&schema, &map, &state_a, &ancestry_a).unwrap();
    let lineage_b = initialize_root_mutation_lineage(&schema, &map, &state_b, &ancestry_b).unwrap();
    let profile_a = recombination_profile(&schema, &map, "e3-profile-a");
    let profile_b = recombination_profile(&schema, &map, "e3-profile-b");
    let roots = ModeledAncestryGraph::new_roots(
        &schema,
        &map,
        [
            AncestryGraphNode::root(ancestry("e3-a-0"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("e3-a-1"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("e3-b-0"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("e3-b-1"), chromosome(), AncestryGeneration::new(0)),
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

#[allow(clippy::too_many_arguments)]
fn breed(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    source_a: &PhasedHereditaryState,
    ancestry_a: &PhasedAncestryState,
    lineage_a: &MutationLineageState,
    profile_a: &ChromosomeRecombinationProfile,
    source_b: &PhasedHereditaryState,
    ancestry_b: &PhasedAncestryState,
    lineage_b: &MutationLineageState,
    profile_b: &ChromosomeRecombinationProfile,
    source_graph: &ModeledAncestryGraph,
    generation: u64,
    event_name: &str,
    mutation_rate_ppm: u32,
) -> SubjectState {
    let event = ReproductionEventId::new(event_name).unwrap();
    let gamete_a = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            schema,
            map,
            source_a,
            profile_a,
            &event,
            ParentRole::ParentA,
        )
        .unwrap(),
    );
    let gamete_b = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            schema,
            map,
            source_b,
            profile_b,
            &event,
            ParentRole::ParentB,
        )
        .unwrap(),
    );
    let gamete_ancestry_a = derive_modeled_gamete_ancestry(
        schema,
        map,
        source_a,
        ancestry_a,
        profile_a,
        &gamete_a,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();
    let gamete_ancestry_b = derive_modeled_gamete_ancestry(
        schema,
        map,
        source_b,
        ancestry_b,
        profile_b,
        &gamete_b,
        &event,
        ParentRole::ParentB,
    )
    .unwrap();
    let offspring = assemble_diploid_linked_offspring_from_evidence(
        schema,
        map,
        source_a,
        profile_a,
        &gamete_a,
        source_b,
        profile_b,
        &gamete_b,
        &event,
    )
    .unwrap();
    let descendant = derive_descendant_ancestry(
        schema,
        map,
        source_a,
        ancestry_a,
        profile_a,
        &gamete_a,
        &gamete_ancestry_a,
        source_b,
        ancestry_b,
        profile_b,
        &gamete_b,
        &gamete_ancestry_b,
        &offspring,
        &event,
    )
    .unwrap();
    let graph = append_descendant_ancestry_to_graph(
        source_graph,
        schema,
        map,
        source_a,
        ancestry_a,
        profile_a,
        &gamete_a,
        &gamete_ancestry_a,
        source_b,
        ancestry_b,
        profile_b,
        &gamete_b,
        &gamete_ancestry_b,
        &descendant,
        AncestryGeneration::new(generation),
    )
    .unwrap()
    .graph;
    let operators = mutation_operators(mutation_rate_ppm);
    let execution = execute_linked_mutations(
        schema,
        map,
        &operators,
        source_a,
        ancestry_a,
        profile_a,
        &gamete_a,
        &gamete_ancestry_a,
        source_b,
        ancestry_b,
        profile_b,
        &gamete_b,
        &gamete_ancestry_b,
        &descendant,
        &graph,
    )
    .unwrap();
    let lineage = derive_descendant_mutation_lineage(
        schema,
        map,
        &operators,
        source_a,
        ancestry_a,
        lineage_a,
        profile_a,
        &gamete_a,
        &gamete_ancestry_a,
        source_b,
        ancestry_b,
        lineage_b,
        profile_b,
        &gamete_b,
        &gamete_ancestry_b,
        &descendant,
        &graph,
        &execution,
    )
    .unwrap()
    .state;
    SubjectState {
        state: execution.mutated_child,
        ancestry: execution.mutated_child_ancestry,
        lineage,
        graph,
    }
}

fn recurrent_parent(f: &RootFixture) -> SubjectState {
    breed(
        &f.schema,
        &f.map,
        &f.state_a,
        &f.ancestry_a,
        &f.lineage_a,
        &f.profile_a,
        &f.state_b,
        &f.ancestry_b,
        &f.lineage_b,
        &f.profile_b,
        &f.roots,
        1,
        "popgen-05e3-recurrent-parent",
        PROBABILITY_SCALE_PPM,
    )
}

fn active_origins(lineage: &MutationLineageState) -> Vec<MutationOriginDigest> {
    let mut origins: Vec<_> = lineage
        .entries
        .iter()
        .filter_map(|entry| entry.active_origin)
        .collect();
    origins.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    origins.dedup_by(|a, b| a.as_bytes() == b.as_bytes());
    origins
}

fn projection_origin_count(
    projection: &symtropy_evolution_core::LinkedCensusProjection,
    origin: MutationOriginDigest,
) -> u64 {
    projection
        .mutation_fate
        .loci
        .iter()
        .find(|entry| entry.locus_id == locus())
        .unwrap()
        .origin_counts
        .iter()
        .find(|entry| entry.origin_digest == origin)
        .map(|entry| entry.count)
        .unwrap_or(0)
}

fn aggregate_origin_count(
    state: &symtropy_evolution_core::OriginAwarePopulationState,
    origin: MutationOriginDigest,
) -> u64 {
    state
        .allele_provenance(&locus(), &allele("a1"))
        .unwrap()
        .active_origin_counts
        .iter()
        .find(|entry| entry.origin_digest == origin)
        .map(|entry| entry.count)
        .unwrap_or(0)
}

fn normalized(histogram: &[u64; 3]) -> [f64; 3] {
    let total: u64 = histogram.iter().sum();
    histogram.map(|count| count as f64 / total as f64)
}

fn total_variation(left: &[f64; 3], right: &[f64; 3]) -> f64 {
    0.5 * left
        .iter()
        .zip(right.iter())
        .map(|(a, b)| (a - b).abs())
        .sum::<f64>()
}

fn assert_binomial_50_50(observed: &[f64; 3], tolerance: f64) {
    for (value, expected) in observed.iter().zip([0.25, 0.50, 0.25]) {
        assert!(
            (value - expected).abs() <= tolerance,
            "observed={observed:?}, expected=[0.25, 0.50, 0.25], tolerance={tolerance}"
        );
    }
}

fn fixed_for_origin(subject: &SubjectState, origin: MutationOriginDigest) -> bool {
    subject.lineage.entries.len() == 2
        && subject
            .lineage
            .entries
            .iter()
            .all(|entry| entry.active_origin == Some(origin))
}

#[test]
fn explicit_selfing_matches_origin_aware_wright_fisher_provenance_law() {
    let f = fixture();
    let parent = recurrent_parent(&f);
    let origins = active_origins(&parent.lineage);
    assert_eq!(origins.len(), 2);
    assert!(parent
        .lineage
        .entries
        .iter()
        .all(|entry| entry.allele == allele("a1")));

    let parent_subject = MutationFateSubject::new(
        &parent.state,
        &parent.ancestry,
        &parent.lineage,
        1,
    )
    .unwrap();
    let parent_projection = project_declared_linked_census(
        PopulationId::new("popgen-05e3-selfing").unwrap(),
        &f.schema,
        &f.map,
        &[parent_subject],
    )
    .unwrap();
    let aggregate_source = initialize_origin_aware_population_from_projection(
        &f.schema,
        &f.map,
        &parent_projection,
        &[parent_subject],
    )
    .unwrap();
    let source_point = PopulationTrajectoryPoint::declare_reference_start(
        &f.schema,
        &aggregate_source.population,
        EvolutionExperimentId::new("popgen-05e3-selfing-aggregate").unwrap(),
        PopulationGeneration(0),
    )
    .unwrap();
    let profile = population_profile();

    let mut explicit_histogram = [0_u64; 3];
    for replicate in 0..EXPLICIT_REPLICATES {
        let child = breed(
            &f.schema,
            &f.map,
            &parent.state,
            &parent.ancestry,
            &parent.lineage,
            &f.profile_a,
            &parent.state,
            &parent.ancestry,
            &parent.lineage,
            &f.profile_b,
            &parent.graph,
            2,
            &format!("popgen-05e3-selfing-explicit-{replicate}"),
            0,
        );
        let subject = MutationFateSubject::new(&child.state, &child.ancestry, &child.lineage, 1)
            .unwrap();
        let projection = project_declared_linked_census(
            PopulationId::new(format!("popgen-05e3-explicit-{replicate}")).unwrap(),
            &f.schema,
            &f.map,
            &[subject],
        )
        .unwrap();
        let count = projection_origin_count(&projection, origins[0]);
        assert!(count <= 2);
        explicit_histogram[count as usize] += 1;
    }

    let mut aggregate_histogram = [0_u64; 3];
    for replicate in 0..AGGREGATE_REPLICATES {
        let result = neutral_origin_aware_population_step(
            &f.schema,
            &aggregate_source,
            &source_point,
            &PopulationTransitionId::new(format!("popgen-05e3-aggregate-{replicate}")).unwrap(),
            &profile,
        )
        .unwrap();
        let count = aggregate_origin_count(&result.destination, origins[0]);
        assert!(count <= 2);
        aggregate_histogram[count as usize] += 1;
    }

    let explicit = normalized(&explicit_histogram);
    let aggregate = normalized(&aggregate_histogram);
    assert_binomial_50_50(&explicit, BINOMIAL_ABS_MAX);
    assert_binomial_50_50(&aggregate, BINOMIAL_ABS_MAX);
    assert!(
        total_variation(&explicit, &aggregate) <= MATCHED_TV_MAX,
        "explicit={explicit:?}, aggregate={aggregate:?}"
    );
}

#[test]
fn fixed_parent_roles_are_detected_as_outside_gene_copy_wright_fisher_closure() {
    let f = fixture();
    let recurrent = recurrent_parent(&f);
    let origins = active_origins(&recurrent.lineage);
    assert_eq!(origins.len(), 2);

    let mut first_fixed = None;
    for candidate in 0..512 {
        let child = breed(
            &f.schema,
            &f.map,
            &recurrent.state,
            &recurrent.ancestry,
            &recurrent.lineage,
            &f.profile_a,
            &recurrent.state,
            &recurrent.ancestry,
            &recurrent.lineage,
            &f.profile_b,
            &recurrent.graph,
            2,
            &format!("popgen-05e3-find-origin-0-{candidate}"),
            0,
        );
        if fixed_for_origin(&child, origins[0]) {
            first_fixed = Some(child);
            break;
        }
    }
    let first_fixed = first_fixed.expect("frozen search window must find origin-0 homozygote");

    let mut second_fixed = None;
    for candidate in 0..512 {
        let child = breed(
            &f.schema,
            &f.map,
            &recurrent.state,
            &recurrent.ancestry,
            &recurrent.lineage,
            &f.profile_a,
            &recurrent.state,
            &recurrent.ancestry,
            &recurrent.lineage,
            &f.profile_b,
            &first_fixed.graph,
            2,
            &format!("popgen-05e3-find-origin-1-{candidate}"),
            0,
        );
        if fixed_for_origin(&child, origins[1]) {
            second_fixed = Some(child);
            break;
        }
    }
    let second_fixed = second_fixed.expect("frozen search window must find origin-1 homozygote");

    let mut structured_histogram = [0_u64; 3];
    for replicate in 0..128 {
        let child = breed(
            &f.schema,
            &f.map,
            &first_fixed.state,
            &first_fixed.ancestry,
            &first_fixed.lineage,
            &f.profile_a,
            &second_fixed.state,
            &second_fixed.ancestry,
            &second_fixed.lineage,
            &f.profile_b,
            &second_fixed.graph,
            3,
            &format!("popgen-05e3-structured-{replicate}"),
            0,
        );
        let subject = MutationFateSubject::new(&child.state, &child.ancestry, &child.lineage, 1)
            .unwrap();
        let projection = project_declared_linked_census(
            PopulationId::new(format!("popgen-05e3-structured-{replicate}")).unwrap(),
            &f.schema,
            &f.map,
            &[subject],
        )
        .unwrap();
        let first = projection_origin_count(&projection, origins[0]);
        let second = projection_origin_count(&projection, origins[1]);
        assert_eq!((first, second), (1, 1));
        structured_histogram[first as usize] += 1;
    }

    let structured = normalized(&structured_histogram);
    let wright_fisher_null = [0.25, 0.50, 0.25];
    assert_eq!(structured, [0.0, 1.0, 0.0]);
    assert!(
        total_variation(&structured, &wright_fisher_null) >= STRUCTURED_TV_MIN,
        "family-structured reproduction unexpectedly collapsed to Wright-Fisher: {structured:?}"
    );
}
