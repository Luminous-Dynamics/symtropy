use symtropy_evolution_core::{
    append_descendant_ancestry_to_graph, assemble_diploid_linked_offspring_from_evidence,
    derive_descendant_ancestry, derive_descendant_mutation_lineage,
    derive_marker_marginal_poisson_linked_gamete, derive_modeled_gamete_ancestry,
    derive_zero_crossover_linked_gamete, execute_linked_mutations,
    initialize_root_mutation_lineage, AlleleId, AncestryCopyId, AncestryGeneration,
    AncestryGraphNode, ChromosomeAncestryState, ChromosomeDefinition, ChromosomeHaplotype,
    ChromosomeId, ChromosomeLocus, ChromosomeMap, ChromosomeMapId,
    ChromosomeRecombinationDomain, ChromosomeRecombinationModel,
    ChromosomeRecombinationProfile, ChromosomeRecombinationProfileId,
    DescendantAncestryDerivation, EvolutionOperatorProfile, GameteAncestryDerivation,
    GeneticMapIntervalMicromorgans, GeneticMapPositionMicromorgans,
    HaplotypeAncestryClass, HereditarySchema, HereditarySchemaId,
    LinkedGameteDerivationEvidence, LinkedMutationExecution, LinkedMutationOutcome,
    LocusDefinition, LocusId, ModeledAncestryGraph, MutationLineageState,
    MutationLineageTransitionResult, MutationProfile, OperatorProfileId, ParentRole,
    PhasedAncestryState, PhasedChromosomeState, PhasedHereditaryState,
    RecombinationMode, RecombinationProfile, ReproductionEventId, PROBABILITY_SCALE_PPM,
};

fn allele(id: &str) -> AlleleId { AlleleId::new(id).unwrap() }
fn locus(id: &str) -> LocusId { LocusId::new(id).unwrap() }
fn chromosome() -> ChromosomeId { ChromosomeId::new("chr-lineage-adv").unwrap() }
fn ancestry(id: &str) -> AncestryCopyId { AncestryCopyId::new(id).unwrap() }
fn pos(value: u64) -> GeneticMapPositionMicromorgans { GeneticMapPositionMicromorgans::new(value) }

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("mut-05c-adv-schema").unwrap(),
        2,
        [
            LocusDefinition::new(locus("a"), [allele("a0"), allele("a1"), allele("a2")]).unwrap(),
            LocusDefinition::new(locus("b"), [allele("b0"), allele("b1")]).unwrap(),
        ],
    ).unwrap()
}

fn map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("mut-05c-adv-map").unwrap(),
        schema,
        [ChromosomeDefinition::new(
            chromosome(),
            vec![
                ChromosomeLocus::new(locus("a"), pos(1)),
                ChromosomeLocus::new(locus("b"), pos(8_000_001)),
            ],
        ).unwrap()],
    ).unwrap()
}

fn source(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    first: &[&str],
    second: &[&str],
) -> PhasedHereditaryState {
    PhasedHereditaryState::new(
        schema,
        map,
        [PhasedChromosomeState::new(
            chromosome(),
            vec![
                ChromosomeHaplotype::new(first.iter().map(|id| allele(id)).collect()),
                ChromosomeHaplotype::new(second.iter().map(|id| allele(id)).collect()),
            ],
        )],
    ).unwrap()
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
            vec![
                HaplotypeAncestryClass::new(0, vec![ancestry(first)]).unwrap(),
                HaplotypeAncestryClass::new(1, vec![ancestry(second)]).unwrap(),
            ],
        ).unwrap()],
    ).unwrap()
}

fn profile(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    id: &str,
    model: ChromosomeRecombinationModel,
) -> ChromosomeRecombinationProfile {
    ChromosomeRecombinationProfile::new(
        ChromosomeRecombinationProfileId::new(id).unwrap(),
        schema,
        map,
        model,
        [ChromosomeRecombinationDomain::new(
            chromosome(),
            GeneticMapIntervalMicromorgans::new(pos(0), pos(8_000_002)).unwrap(),
        ).unwrap()],
    ).unwrap()
}

fn operators(rate_ppm: u32) -> EvolutionOperatorProfile {
    EvolutionOperatorProfile {
        profile_id: OperatorProfileId::new("mut-05c-adv-operators").unwrap(),
        version: "1".into(),
        mutation: MutationProfile {
            model_id: "modeled-locus-substitution".into(),
            version: "1".into(),
            per_copy_rate_ppm: rate_ppm,
        },
        recombination: RecombinationProfile {
            model_id: "linked-chromosome-context".into(),
            version: "1".into(),
            mode: RecombinationMode::IndependentLoci,
        },
    }
}

struct RootFixture {
    schema: HereditarySchema,
    map: ChromosomeMap,
    source_a: PhasedHereditaryState,
    source_b: PhasedHereditaryState,
    ancestry_a: PhasedAncestryState,
    ancestry_b: PhasedAncestryState,
    lineage_a: MutationLineageState,
    lineage_b: MutationLineageState,
    zero_a: ChromosomeRecombinationProfile,
    zero_b: ChromosomeRecombinationProfile,
    poisson_a: ChromosomeRecombinationProfile,
    roots: ModeledAncestryGraph,
}

fn root_fixture() -> RootFixture {
    let schema = schema();
    let map = map(&schema);
    let source_a = source(&schema, &map, &["a0", "b0"], &["a1", "b1"]);
    let source_b = source(&schema, &map, &["a1", "b0"], &["a0", "b1"]);
    let ancestry_a = ancestry_state(&schema, &map, &source_a, "adv-a-root-0", "adv-a-root-1");
    let ancestry_b = ancestry_state(&schema, &map, &source_b, "adv-b-root-0", "adv-b-root-1");
    let lineage_a = initialize_root_mutation_lineage(&schema, &map, &source_a, &ancestry_a).unwrap();
    let lineage_b = initialize_root_mutation_lineage(&schema, &map, &source_b, &ancestry_b).unwrap();
    let zero_a = profile(
        &schema,
        &map,
        "mut-05c-adv-zero-a",
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
    );
    let zero_b = profile(
        &schema,
        &map,
        "mut-05c-adv-zero-b",
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
    );
    let poisson_a = profile(
        &schema,
        &map,
        "mut-05c-adv-poisson-a",
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
    );
    let roots = ModeledAncestryGraph::new_roots(
        &schema,
        &map,
        [
            AncestryGraphNode::root(ancestry("adv-a-root-0"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("adv-a-root-1"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("adv-b-root-0"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("adv-b-root-1"), chromosome(), AncestryGeneration::new(0)),
        ],
    ).unwrap();
    RootFixture {
        schema, map, source_a, source_b, ancestry_a, ancestry_b,
        lineage_a, lineage_b, zero_a, zero_b, poisson_a, roots,
    }
}

struct Generation {
    gamete_a: LinkedGameteDerivationEvidence,
    gamete_b: LinkedGameteDerivationEvidence,
    gamete_ancestry_a: GameteAncestryDerivation,
    gamete_ancestry_b: GameteAncestryDerivation,
    descendant: DescendantAncestryDerivation,
    graph: ModeledAncestryGraph,
    execution: LinkedMutationExecution,
    lineage: MutationLineageTransitionResult,
}

#[allow(clippy::too_many_arguments)]
fn generation(
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
    generation_number: u64,
    event_name: &str,
    mutation_rate_ppm: u32,
) -> Generation {
    let event = ReproductionEventId::new(event_name).unwrap();
    let gamete_a = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            schema, map, source_a, profile_a, &event, ParentRole::ParentA,
        ).unwrap(),
    );
    let gamete_b = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            schema, map, source_b, profile_b, &event, ParentRole::ParentB,
        ).unwrap(),
    );
    finish_generation(
        schema, map,
        source_a, ancestry_a, lineage_a, profile_a, gamete_a,
        source_b, ancestry_b, lineage_b, profile_b, gamete_b,
        source_graph, generation_number, event, mutation_rate_ppm,
    )
}

#[allow(clippy::too_many_arguments)]
fn finish_generation(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    source_a: &PhasedHereditaryState,
    ancestry_a: &PhasedAncestryState,
    lineage_a: &MutationLineageState,
    profile_a: &ChromosomeRecombinationProfile,
    gamete_a: LinkedGameteDerivationEvidence,
    source_b: &PhasedHereditaryState,
    ancestry_b: &PhasedAncestryState,
    lineage_b: &MutationLineageState,
    profile_b: &ChromosomeRecombinationProfile,
    gamete_b: LinkedGameteDerivationEvidence,
    source_graph: &ModeledAncestryGraph,
    generation_number: u64,
    event: ReproductionEventId,
    mutation_rate_ppm: u32,
) -> Generation {
    let gamete_ancestry_a = derive_modeled_gamete_ancestry(
        schema, map, source_a, ancestry_a, profile_a, &gamete_a, &event, ParentRole::ParentA,
    ).unwrap();
    let gamete_ancestry_b = derive_modeled_gamete_ancestry(
        schema, map, source_b, ancestry_b, profile_b, &gamete_b, &event, ParentRole::ParentB,
    ).unwrap();
    let offspring = assemble_diploid_linked_offspring_from_evidence(
        schema, map,
        source_a, profile_a, &gamete_a,
        source_b, profile_b, &gamete_b,
        &event,
    ).unwrap();
    let descendant = derive_descendant_ancestry(
        schema, map,
        source_a, ancestry_a, profile_a, &gamete_a, &gamete_ancestry_a,
        source_b, ancestry_b, profile_b, &gamete_b, &gamete_ancestry_b,
        &offspring, &event,
    ).unwrap();
    let graph = append_descendant_ancestry_to_graph(
        source_graph, schema, map,
        source_a, ancestry_a, profile_a, &gamete_a, &gamete_ancestry_a,
        source_b, ancestry_b, profile_b, &gamete_b, &gamete_ancestry_b,
        &descendant, AncestryGeneration::new(generation_number),
    ).unwrap().graph;
    let authority = operators(mutation_rate_ppm);
    let execution = execute_linked_mutations(
        schema, map, &authority,
        source_a, ancestry_a, profile_a, &gamete_a, &gamete_ancestry_a,
        source_b, ancestry_b, profile_b, &gamete_b, &gamete_ancestry_b,
        &descendant, &graph,
    ).unwrap();
    let lineage = derive_descendant_mutation_lineage(
        schema, map, &authority,
        source_a, ancestry_a, lineage_a, profile_a, &gamete_a, &gamete_ancestry_a,
        source_b, ancestry_b, lineage_b, profile_b, &gamete_b, &gamete_ancestry_b,
        &descendant, &graph, &execution,
    ).unwrap();
    Generation {
        gamete_a, gamete_b, gamete_ancestry_a, gamete_ancestry_b,
        descendant, graph, execution, lineage,
    }
}

#[test]
fn back_mutation_gets_a_new_origin_linked_to_the_previous_origin() {
    let f = root_fixture();
    let first = generation(
        &f.schema, &f.map,
        &f.source_a, &f.ancestry_a, &f.lineage_a, &f.zero_a,
        &f.source_b, &f.ancestry_b, &f.lineage_b, &f.zero_b,
        &f.roots, 1, "mut-05c-back-1", PROBABILITY_SCALE_PPM,
    );
    let second = generation(
        &f.schema, &f.map,
        &first.execution.mutated_child,
        &first.execution.mutated_child_ancestry,
        &first.lineage.state,
        &f.zero_a,
        &f.source_b, &f.ancestry_b, &f.lineage_b, &f.zero_b,
        &first.graph, 2, "mut-05c-back-2", PROBABILITY_SCALE_PPM,
    );

    let opportunity = second.execution.opportunities.iter().find(|opportunity| {
        opportunity.parent_role == ParentRole::ParentA
            && opportunity.locus_id == locus("b")
            && matches!(&opportunity.outcome, LinkedMutationOutcome::Substitution { .. })
    }).unwrap();
    let edge = second.descendant.materialization.edges.iter().find(|edge| {
        edge.child_copy_id == opportunity.ancestry_copy_id && edge.locus_id == locus("b")
    }).unwrap();
    let previous_entry = first.lineage.state.entry(&edge.source_copy_id, &locus("b")).unwrap();
    let previous_origin = previous_entry.active_origin.expect("generation 1 must mutate binary locus b");
    let previous_event = first.lineage.state.history_event(previous_origin).unwrap();

    let child_entry = second.lineage.state.entry(&edge.child_copy_id, &locus("b")).unwrap();
    let new_origin = child_entry.active_origin.expect("generation 2 must mutate binary locus b");
    assert_ne!(new_origin, previous_origin);
    let new_event = second.lineage.state.history_event(new_origin).unwrap();
    assert_eq!(new_event.previous_active_origin, Some(previous_origin));
    assert_eq!(new_event.origin.derived_allele(), previous_event.origin.ancestral_allele());
    assert_ne!(new_event.origin.canonical_digest(), previous_event.origin.canonical_digest());
}

#[test]
fn equal_allele_values_can_have_distinct_recurrent_mutation_histories() {
    let f = root_fixture();
    let chosen = (0..128).find_map(|index| {
        let event = format!("mut-05c-recurrent-{index}");
        let generation = generation(
            &f.schema, &f.map,
            &f.source_a, &f.ancestry_a, &f.lineage_a, &f.zero_a,
            &f.source_b, &f.ancestry_b, &f.lineage_b, &f.zero_b,
            &f.roots, 1, &event, PROBABILITY_SCALE_PPM,
        );
        let mut b = generation.execution.opportunities.iter().filter(|opportunity| {
            opportunity.locus_id == locus("b")
        });
        let left = b.next()?;
        let right = b.next()?;
        let (left_origin, right_origin) = match (&left.outcome, &right.outcome) {
            (
                LinkedMutationOutcome::Substitution { origin: left_origin, .. },
                LinkedMutationOutcome::Substitution { origin: right_origin, .. },
            ) if left_origin.derived_allele() == right_origin.derived_allele() => {
                (left_origin.canonical_digest(), right_origin.canonical_digest())
            }
            _ => return None,
        };
        Some((generation, left_origin, right_origin))
    }).expect("frozen 128-event corpus must contain convergent binary substitutions");

    let (generation, left_origin, right_origin) = chosen;
    assert_ne!(left_origin, right_origin);
    let left = generation.lineage.state.history_event(left_origin).unwrap();
    let right = generation.lineage.state.history_event(right_origin).unwrap();
    assert_eq!(left.origin.derived_allele(), right.origin.derived_allele());
    assert_ne!(left.origin.ancestry_copy_id(), right.origin.ancestry_copy_id());
}

#[test]
fn marker_recombination_can_mix_active_mutation_histories_by_locus() {
    let f = root_fixture();
    let first = generation(
        &f.schema, &f.map,
        &f.source_a, &f.ancestry_a, &f.lineage_a, &f.zero_a,
        &f.source_b, &f.ancestry_b, &f.lineage_b, &f.zero_b,
        &f.roots, 1, "mut-05c-recomb-source", PROBABILITY_SCALE_PPM,
    );

    let (event, marker) = (0..128).find_map(|index| {
        let event = ReproductionEventId::new(format!("mut-05c-recomb-{index}")).unwrap();
        let marker = derive_marker_marginal_poisson_linked_gamete(
            &f.schema,
            &f.map,
            &first.execution.mutated_child,
            &f.poisson_a,
            &event,
            ParentRole::ParentA,
        ).ok()?;
        let chromosome = marker.provenance.chromosomes().get(&chromosome())?;
        if chromosome.adjacent_intervals.first()?.parity == symtropy_evolution_core::CrossoverParity::Odd {
            Some((event, marker))
        } else {
            None
        }
    }).expect("frozen 128-event corpus must contain odd marker parity");

    let gamete_a = LinkedGameteDerivationEvidence::MarkerMarginalPoisson(marker);
    let gamete_b = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &f.schema, &f.map, &f.source_b, &f.zero_b, &event, ParentRole::ParentB,
        ).unwrap(),
    );
    let second = finish_generation(
        &f.schema, &f.map,
        &first.execution.mutated_child,
        &first.execution.mutated_child_ancestry,
        &first.lineage.state,
        &f.poisson_a,
        gamete_a,
        &f.source_b, &f.ancestry_b, &f.lineage_b, &f.zero_b,
        gamete_b,
        &first.graph, 2, event, 0,
    );

    let parent_a_child = second.descendant.materialization.descendant_copies.iter()
        .find(|copy| copy.parent_role == ParentRole::ParentA)
        .unwrap();
    let edge_a = second.descendant.materialization.edges.iter().find(|edge| {
        edge.child_copy_id == parent_a_child.child_copy_id && edge.locus_id == locus("a")
    }).unwrap();
    let edge_b = second.descendant.materialization.edges.iter().find(|edge| {
        edge.child_copy_id == parent_a_child.child_copy_id && edge.locus_id == locus("b")
    }).unwrap();
    assert_ne!(edge_a.source_copy_id, edge_b.source_copy_id);

    let parent_a_origin = first.lineage.state.entry(&edge_a.source_copy_id, &locus("a"))
        .unwrap().active_origin.unwrap();
    let parent_b_origin = first.lineage.state.entry(&edge_b.source_copy_id, &locus("b"))
        .unwrap().active_origin.unwrap();
    assert_ne!(parent_a_origin, parent_b_origin);

    let child_a = second.lineage.state.entry(&parent_a_child.child_copy_id, &locus("a")).unwrap();
    let child_b = second.lineage.state.entry(&parent_a_child.child_copy_id, &locus("b")).unwrap();
    assert_eq!(child_a.active_origin, Some(parent_a_origin));
    assert_eq!(child_b.active_origin, Some(parent_b_origin));
}

#[test]
fn malformed_lineage_history_fails_closed() {
    let f = root_fixture();
    let first = generation(
        &f.schema, &f.map,
        &f.source_a, &f.ancestry_a, &f.lineage_a, &f.zero_a,
        &f.source_b, &f.ancestry_b, &f.lineage_b, &f.zero_b,
        &f.roots, 1, "mut-05c-malformed", PROBABILITY_SCALE_PPM,
    );

    let mut missing = first.lineage.state.clone();
    let active = missing.entries.iter().find_map(|entry| entry.active_origin).unwrap();
    missing.history.retain(|event| event.origin_digest() != active);
    assert!(missing.validate_current(
        &f.schema, &f.map,
        &first.execution.mutated_child,
        &first.execution.mutated_child_ancestry,
    ).is_err());

    let mut orphan = first.lineage.state.clone();
    let active_index = orphan.entries.iter().position(|entry| entry.active_origin.is_some()).unwrap();
    orphan.entries[active_index].active_origin = None;
    assert!(orphan.validate_current(
        &f.schema, &f.map,
        &first.execution.mutated_child,
        &first.execution.mutated_child_ancestry,
    ).is_err());

    let mut cycle = first.lineage.state.clone();
    let self_digest = cycle.history[0].origin_digest();
    cycle.history[0].previous_active_origin = Some(self_digest);
    assert!(cycle.validate_current(
        &f.schema, &f.map,
        &first.execution.mutated_child,
        &first.execution.mutated_child_ancestry,
    ).is_err());
}
