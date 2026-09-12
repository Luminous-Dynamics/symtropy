use symtropy_evolution_core::{
    append_descendant_ancestry_to_graph, assemble_diploid_linked_offspring_from_evidence,
    continue_origin_aware_trajectory, derive_descendant_ancestry,
    derive_descendant_mutation_lineage, derive_modeled_gamete_ancestry,
    derive_zero_crossover_linked_gamete, execute_linked_mutations,
    initialize_origin_aware_population_from_projection, initialize_root_mutation_lineage,
    project_declared_linked_census, AlleleId, AncestryCopyId, AncestryGeneration,
    AncestryGraphNode, ChromosomeAncestryState, ChromosomeDefinition, ChromosomeHaplotype,
    ChromosomeId, ChromosomeLocus, ChromosomeMap, ChromosomeMapId,
    ChromosomeRecombinationDomain, ChromosomeRecombinationModel,
    ChromosomeRecombinationProfile, ChromosomeRecombinationProfileId,
    EvolutionExperimentId, EvolutionOperatorProfile, GeneticMapIntervalMicromorgans,
    GeneticMapPositionMicromorgans, HaplotypeAncestryClass, HereditarySchema,
    HereditarySchemaId, LinkedGameteDerivationEvidence, LocusDefinition, LocusId,
    ModeledAncestryGraph, MutationFateSubject, MutationLineageState, MutationProfile,
    OperatorProfileId, OriginAwarePopulationState, OriginAwarePopulationTrajectoryPoint,
    OriginAwareTrajectoryTransition, ParentRole, PhasedAncestryState,
    PhasedChromosomeState, PhasedHereditaryState, PopulationGeneration, PopulationId,
    PopulationProcessModel, PopulationProcessProfile, PopulationProcessProfileId,
    PopulationTransitionId, RecombinationMode, RecombinationProfile, ReproductionEventId,
    PROBABILITY_SCALE_PPM,
};

fn allele(id: &str) -> AlleleId { AlleleId::new(id).unwrap() }
fn locus() -> LocusId { LocusId::new("focal").unwrap() }
fn chromosome() -> ChromosomeId { ChromosomeId::new("chr-origin-trajectory").unwrap() }
fn ancestry(id: &str) -> AncestryCopyId { AncestryCopyId::new(id).unwrap() }
fn pos(value: u64) -> GeneticMapPositionMicromorgans { GeneticMapPositionMicromorgans::new(value) }

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("popgen-05d-schema").unwrap(),
        2,
        [LocusDefinition::new(locus(), [allele("a0"), allele("a1")]).unwrap()],
    ).unwrap()
}

fn map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("popgen-05d-map").unwrap(),
        schema,
        [ChromosomeDefinition::new(
            chromosome(),
            vec![ChromosomeLocus::new(locus(), pos(1))],
        ).unwrap()],
    ).unwrap()
}

fn source(schema: &HereditarySchema, map: &ChromosomeMap) -> PhasedHereditaryState {
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
    ).unwrap()
}

fn source_ancestry(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    source: &PhasedHereditaryState,
    first: &str,
    second: &str,
) -> PhasedAncestryState {
    PhasedAncestryState::new(
        schema,
        map,
        source,
        [ChromosomeAncestryState::new(
            chromosome(),
            vec![HaplotypeAncestryClass::new(
                0,
                vec![ancestry(first), ancestry(second)],
            ).unwrap()],
        ).unwrap()],
    ).unwrap()
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
        ).unwrap()],
    ).unwrap()
}

fn mutation_operators() -> EvolutionOperatorProfile {
    EvolutionOperatorProfile {
        profile_id: OperatorProfileId::new("popgen-05d-mutation").unwrap(),
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
        profile_id: PopulationProcessProfileId::new("popgen-05d-neutral").unwrap(),
        version: "1".into(),
        model: PopulationProcessModel::NeutralIndependentLocusWrightFisher,
    }
}

struct Fixture {
    schema: HereditarySchema,
    map: ChromosomeMap,
    source_a: PhasedHereditaryState,
    source_b: PhasedHereditaryState,
    ancestry_a: PhasedAncestryState,
    ancestry_b: PhasedAncestryState,
    lineage_a: MutationLineageState,
    lineage_b: MutationLineageState,
    profile_a: ChromosomeRecombinationProfile,
    profile_b: ChromosomeRecombinationProfile,
    roots: ModeledAncestryGraph,
}

fn fixture() -> Fixture {
    let schema = schema();
    let map = map(&schema);
    let source_a = source(&schema, &map);
    let source_b = source(&schema, &map);
    let ancestry_a = source_ancestry(&schema, &map, &source_a, "traj-a-0", "traj-a-1");
    let ancestry_b = source_ancestry(&schema, &map, &source_b, "traj-b-0", "traj-b-1");
    let lineage_a = initialize_root_mutation_lineage(&schema, &map, &source_a, &ancestry_a).unwrap();
    let lineage_b = initialize_root_mutation_lineage(&schema, &map, &source_b, &ancestry_b).unwrap();
    let profile_a = recombination_profile(&schema, &map, "traj-profile-a");
    let profile_b = recombination_profile(&schema, &map, "traj-profile-b");
    let roots = ModeledAncestryGraph::new_roots(
        &schema,
        &map,
        [
            AncestryGraphNode::root(ancestry("traj-a-0"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("traj-a-1"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("traj-b-0"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("traj-b-1"), chromosome(), AncestryGeneration::new(0)),
        ],
    ).unwrap();
    Fixture {
        schema, map, source_a, source_b, ancestry_a, ancestry_b,
        lineage_a, lineage_b, profile_a, profile_b, roots,
    }
}

struct Child {
    state: PhasedHereditaryState,
    ancestry: PhasedAncestryState,
    lineage: MutationLineageState,
}

fn recurrent_child(f: &Fixture, event_name: &str) -> Child {
    let event = ReproductionEventId::new(event_name).unwrap();
    let gamete_a = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &f.schema, &f.map, &f.source_a, &f.profile_a, &event, ParentRole::ParentA,
        ).unwrap(),
    );
    let gamete_b = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &f.schema, &f.map, &f.source_b, &f.profile_b, &event, ParentRole::ParentB,
        ).unwrap(),
    );
    let gamete_ancestry_a = derive_modeled_gamete_ancestry(
        &f.schema, &f.map, &f.source_a, &f.ancestry_a, &f.profile_a,
        &gamete_a, &event, ParentRole::ParentA,
    ).unwrap();
    let gamete_ancestry_b = derive_modeled_gamete_ancestry(
        &f.schema, &f.map, &f.source_b, &f.ancestry_b, &f.profile_b,
        &gamete_b, &event, ParentRole::ParentB,
    ).unwrap();
    let offspring = assemble_diploid_linked_offspring_from_evidence(
        &f.schema, &f.map,
        &f.source_a, &f.profile_a, &gamete_a,
        &f.source_b, &f.profile_b, &gamete_b,
        &event,
    ).unwrap();
    let descendant = derive_descendant_ancestry(
        &f.schema, &f.map,
        &f.source_a, &f.ancestry_a, &f.profile_a, &gamete_a, &gamete_ancestry_a,
        &f.source_b, &f.ancestry_b, &f.profile_b, &gamete_b, &gamete_ancestry_b,
        &offspring, &event,
    ).unwrap();
    let graph = append_descendant_ancestry_to_graph(
        &f.roots, &f.schema, &f.map,
        &f.source_a, &f.ancestry_a, &f.profile_a, &gamete_a, &gamete_ancestry_a,
        &f.source_b, &f.ancestry_b, &f.profile_b, &gamete_b, &gamete_ancestry_b,
        &descendant, AncestryGeneration::new(1),
    ).unwrap().graph;
    let operators = mutation_operators();
    let execution = execute_linked_mutations(
        &f.schema, &f.map, &operators,
        &f.source_a, &f.ancestry_a, &f.profile_a, &gamete_a, &gamete_ancestry_a,
        &f.source_b, &f.ancestry_b, &f.profile_b, &gamete_b, &gamete_ancestry_b,
        &descendant, &graph,
    ).unwrap();
    let lineage = derive_descendant_mutation_lineage(
        &f.schema, &f.map, &operators,
        &f.source_a, &f.ancestry_a, &f.lineage_a, &f.profile_a, &gamete_a, &gamete_ancestry_a,
        &f.source_b, &f.ancestry_b, &f.lineage_b, &f.profile_b, &gamete_b, &gamete_ancestry_b,
        &descendant, &graph, &execution,
    ).unwrap().state;
    Child {
        state: execution.mutated_child,
        ancestry: execution.mutated_child_ancestry,
        lineage,
    }
}

fn origin_state_for_child(
    f: &Fixture,
    event_name: &str,
    population_id: &str,
) -> OriginAwarePopulationState {
    let child = recurrent_child(f, event_name);
    let subject = MutationFateSubject::new(&child.state, &child.ancestry, &child.lineage, 1).unwrap();
    let projection = project_declared_linked_census(
        PopulationId::new(population_id).unwrap(),
        &f.schema,
        &f.map,
        &[subject],
    ).unwrap();
    initialize_origin_aware_population_from_projection(
        &f.schema,
        &f.map,
        &projection,
        &[subject],
    ).unwrap()
}

#[test]
fn origin_aware_point_distinguishes_equal_allele_states_with_different_histories() {
    let f = fixture();
    let first = origin_state_for_child(&f, "traj-history-a", "traj-equal-pop");
    let second = origin_state_for_child(&f, "traj-history-b", "traj-equal-pop");
    assert_eq!(first.population, second.population);
    assert_ne!(first.canonical_digest(&f.schema).unwrap(), second.canonical_digest(&f.schema).unwrap());

    let point_a = OriginAwarePopulationTrajectoryPoint::declare_reference_start(
        &f.schema,
        &first,
        EvolutionExperimentId::new("traj-equal-experiment").unwrap(),
        PopulationGeneration(4),
    ).unwrap();
    let point_b = OriginAwarePopulationTrajectoryPoint::declare_reference_start(
        &f.schema,
        &second,
        EvolutionExperimentId::new("traj-equal-experiment").unwrap(),
        PopulationGeneration(4),
    ).unwrap();
    assert_eq!(point_a.ordinary_point, point_b.ordinary_point);
    assert_ne!(point_a.canonical_digest(), point_b.canonical_digest());

    let encoded = serde_json::to_vec(&point_a).unwrap();
    let restored: OriginAwarePopulationTrajectoryPoint = serde_json::from_slice(&encoded).unwrap();
    restored.validate_current(&f.schema, &first).unwrap();
    assert!(restored.validate_current(&f.schema, &second).is_err());
}

#[test]
fn fate_delta_reports_origin_loss_while_fixed_allele_survives() {
    let f = fixture();
    let source = origin_state_for_child(&f, "traj-loss-source", "traj-loss-pop");
    let source_partition = source.allele_provenance(&locus(), &allele("a1")).unwrap();
    assert_eq!(source_partition.active_origin_counts.len(), 2);

    let point = OriginAwarePopulationTrajectoryPoint::declare_reference_start(
        &f.schema,
        &source,
        EvolutionExperimentId::new("traj-loss-experiment").unwrap(),
        PopulationGeneration(0),
    ).unwrap();
    let profile = population_profile();

    let result = (0..256).find_map(|index| {
        let transition_id = PopulationTransitionId::new(format!("traj-loss-{index}")).unwrap();
        let candidate = continue_origin_aware_trajectory(
            &f.schema,
            &source,
            &point,
            &transition_id,
            &profile,
        ).unwrap();
        let lost = candidate.fate_delta.origins.iter().filter(|entry| entry.lost()).count();
        let fixed = candidate
            .fate_delta
            .origins
            .iter()
            .filter(|entry| entry.fixed_at_locus())
            .count();
        if lost == 1 && fixed == 1 {
            Some((transition_id, candidate))
        } else {
            None
        }
    }).expect("frozen transition corpus must contain one-origin extinction under fixed allele");

    let (transition_id, transition) = result;
    assert_eq!(transition.destination_point.generation(), PopulationGeneration(1));
    assert_eq!(transition.fate_delta.origins.len(), 2);
    assert_eq!(transition.fate_delta.origins.iter().filter(|entry| entry.lost()).count(), 1);
    assert_eq!(
        transition.fate_delta.origins.iter().filter(|entry| entry.fixed_within_allele()).count(),
        1
    );
    assert_eq!(
        transition.fate_delta.origins.iter().filter(|entry| entry.fixed_at_locus()).count(),
        1
    );
    assert!(transition
        .fate_delta
        .origins
        .iter()
        .all(|entry| entry.destination_allele_count == 2));

    let encoded = serde_json::to_vec(&transition).unwrap();
    let restored: OriginAwareTrajectoryTransition = serde_json::from_slice(&encoded).unwrap();
    restored
        .validate_current(&f.schema, &source, &point, &transition_id, &profile)
        .unwrap();
}
