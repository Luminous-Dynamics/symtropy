use symtropy_evolution_core::{
    append_descendant_ancestry_to_graph, assemble_diploid_linked_offspring_from_evidence,
    derive_descendant_ancestry, derive_descendant_mutation_lineage,
    derive_modeled_gamete_ancestry, derive_zero_crossover_linked_gamete,
    execute_linked_mutations, initialize_root_mutation_lineage, AlleleId, AncestryCopyId,
    AncestryGeneration, AncestryGraphNode, ChromosomeAncestryState, ChromosomeDefinition,
    ChromosomeHaplotype, ChromosomeId, ChromosomeLocus, ChromosomeMap, ChromosomeMapId,
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

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn locus(id: &str) -> LocusId {
    LocusId::new(id).unwrap()
}

fn chromosome() -> ChromosomeId {
    ChromosomeId::new("chr-lineage").unwrap()
}

fn ancestry(id: &str) -> AncestryCopyId {
    AncestryCopyId::new(id).unwrap()
}

fn pos(value: u64) -> GeneticMapPositionMicromorgans {
    GeneticMapPositionMicromorgans::new(value)
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("mut-05c-schema").unwrap(),
        2,
        [
            LocusDefinition::new(
                locus("a"),
                [allele("a0"), allele("a1"), allele("a2")],
            )
            .unwrap(),
            LocusDefinition::new(locus("b"), [allele("b0"), allele("b1")]).unwrap(),
        ],
    )
    .unwrap()
}

fn map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("mut-05c-map").unwrap(),
        schema,
        [ChromosomeDefinition::new(
            chromosome(),
            vec![
                ChromosomeLocus::new(locus("a"), pos(1)),
                ChromosomeLocus::new(locus("b"), pos(8_000_001)),
            ],
        )
        .unwrap()],
    )
    .unwrap()
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
    )
    .unwrap()
}

fn ancestry_state(
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
            vec![
                HaplotypeAncestryClass::new(0, vec![ancestry(first)]).unwrap(),
                HaplotypeAncestryClass::new(1, vec![ancestry(second)]).unwrap(),
            ],
        )
        .unwrap()],
    )
    .unwrap()
}

fn profile(
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
            GeneticMapIntervalMicromorgans::new(pos(0), pos(8_000_002)).unwrap(),
        )
        .unwrap()],
    )
    .unwrap()
}

fn operators(rate_ppm: u32) -> EvolutionOperatorProfile {
    EvolutionOperatorProfile {
        profile_id: OperatorProfileId::new("mut-05c-operators").unwrap(),
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
    profile_a: ChromosomeRecombinationProfile,
    profile_b: ChromosomeRecombinationProfile,
    roots: ModeledAncestryGraph,
}

fn root_fixture() -> RootFixture {
    let schema = schema();
    let map = map(&schema);
    let source_a = source(&schema, &map, &["a0", "b0"], &["a1", "b1"]);
    let source_b = source(&schema, &map, &["a1", "b0"], &["a0", "b1"]);
    let ancestry_a = ancestry_state(&schema, &map, &source_a, "a-root-0", "a-root-1");
    let ancestry_b = ancestry_state(&schema, &map, &source_b, "b-root-0", "b-root-1");
    let lineage_a = initialize_root_mutation_lineage(&schema, &map, &source_a, &ancestry_a).unwrap();
    let lineage_b = initialize_root_mutation_lineage(&schema, &map, &source_b, &ancestry_b).unwrap();
    let profile_a = profile(&schema, &map, "mut-05c-parent-a");
    let profile_b = profile(&schema, &map, "mut-05c-parent-b");
    let roots = ModeledAncestryGraph::new_roots(
        &schema,
        &map,
        [
            AncestryGraphNode::root(ancestry("a-root-0"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("a-root-1"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("b-root-0"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("b-root-1"), chromosome(), AncestryGeneration::new(0)),
        ],
    )
    .unwrap();
    RootFixture {
        schema,
        map,
        source_a,
        source_b,
        ancestry_a,
        ancestry_b,
        lineage_a,
        lineage_b,
        profile_a,
        profile_b,
        roots,
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
        AncestryGeneration::new(generation_number),
    )
    .unwrap()
    .graph;
    let authority = operators(mutation_rate_ppm);
    let execution = execute_linked_mutations(
        schema,
        map,
        &authority,
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
        &authority,
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
    .unwrap();
    Generation {
        gamete_a,
        gamete_b,
        gamete_ancestry_a,
        gamete_ancestry_b,
        descendant,
        graph,
        execution,
        lineage,
    }
}

#[test]
fn roots_are_complete_replayable_and_have_no_modeled_mutation_history() {
    let f = root_fixture();
    assert_eq!(f.lineage_a.entries.len(), 4);
    assert!(f.lineage_a.history.is_empty());
    assert!(f
        .lineage_a
        .entries
        .iter()
        .all(|entry| entry.active_origin.is_none()));
    f.lineage_a
        .validate_root_current(&f.schema, &f.map, &f.source_a, &f.ancestry_a)
        .unwrap();

    let encoded = serde_json::to_vec(&f.lineage_a).unwrap();
    let restored: MutationLineageState = serde_json::from_slice(&encoded).unwrap();
    restored
        .validate_root_current(&f.schema, &f.map, &f.source_a, &f.ancestry_a)
        .unwrap();
    assert_eq!(restored, f.lineage_a);
}

#[test]
fn realized_mutations_become_active_origins_and_zero_mutation_transmits_them() {
    let f = root_fixture();
    let first = generation(
        &f.schema,
        &f.map,
        &f.source_a,
        &f.ancestry_a,
        &f.lineage_a,
        &f.profile_a,
        &f.source_b,
        &f.ancestry_b,
        &f.lineage_b,
        &f.profile_b,
        &f.roots,
        1,
        "mut-05c-generation-1",
        PROBABILITY_SCALE_PPM,
    );

    let substitutions = first
        .execution
        .opportunities
        .iter()
        .filter(|opportunity| matches!(&opportunity.outcome, LinkedMutationOutcome::Substitution { .. }))
        .count();
    assert_eq!(substitutions, 4);
    assert_eq!(first.lineage.state.history.len(), 4);
    assert_eq!(
        first
            .lineage
            .state
            .entries
            .iter()
            .filter(|entry| entry.active_origin.is_some())
            .count(),
        4
    );

    for opportunity in &first.execution.opportunities {
        let entry = first
            .lineage
            .state
            .entry(&opportunity.ancestry_copy_id, &opportunity.locus_id)
            .unwrap();
        match &opportunity.outcome {
            LinkedMutationOutcome::Substitution { origin, .. } => {
                assert_eq!(entry.active_origin, Some(origin.canonical_digest()));
                assert_eq!(&entry.allele, origin.derived_allele());
            }
            LinkedMutationOutcome::NoMutation { .. } => panic!("maximum-rate polymorphic fixture must mutate"),
        }
    }

    let first_encoded = serde_json::to_vec(&first.lineage).unwrap();
    let first_restored: MutationLineageTransitionResult =
        serde_json::from_slice(&first_encoded).unwrap();
    first_restored
        .validate_current(
            &f.schema,
            &f.map,
            &operators(PROBABILITY_SCALE_PPM),
            &f.source_a,
            &f.ancestry_a,
            &f.lineage_a,
            &f.profile_a,
            &first.gamete_a,
            &first.gamete_ancestry_a,
            &f.source_b,
            &f.ancestry_b,
            &f.lineage_b,
            &f.profile_b,
            &first.gamete_b,
            &first.gamete_ancestry_b,
            &first.descendant,
            &first.graph,
            &first.execution,
        )
        .unwrap();

    let second = generation(
        &f.schema,
        &f.map,
        &first.execution.mutated_child,
        &first.execution.mutated_child_ancestry,
        &first.lineage.state,
        &f.profile_a,
        &f.source_b,
        &f.ancestry_b,
        &f.lineage_b,
        &f.profile_b,
        &first.graph,
        2,
        "mut-05c-generation-2",
        0,
    );

    assert!(second.execution.opportunities.iter().all(|opportunity| matches!(
        &opportunity.outcome,
        LinkedMutationOutcome::NoMutation { .. }
    )));

    let mut inherited_active = 0_usize;
    for opportunity in &second.execution.opportunities {
        let edge = second
            .descendant
            .materialization
            .edges
            .iter()
            .find(|edge| {
                edge.child_copy_id == opportunity.ancestry_copy_id
                    && edge.locus_id == opportunity.locus_id
            })
            .unwrap();
        let parent_lineage = match edge.parent_role {
            ParentRole::ParentA => &first.lineage.state,
            ParentRole::ParentB => &f.lineage_b,
            ParentRole::ClonalParent => unreachable!(),
        };
        let parent_entry = parent_lineage
            .entry(&edge.source_copy_id, &edge.locus_id)
            .unwrap();
        let child_entry = second
            .lineage
            .state
            .entry(&edge.child_copy_id, &edge.locus_id)
            .unwrap();
        assert_eq!(child_entry.active_origin, parent_entry.active_origin);
        assert_eq!(child_entry.allele, parent_entry.allele);
        if child_entry.active_origin.is_some() {
            inherited_active += 1;
        }
    }
    assert_eq!(inherited_active, 2);
    assert_eq!(second.lineage.state.history.len(), 2);
    assert!(second.lineage.state.history.len() < first.lineage.state.history.len());

    second
        .lineage
        .validate_current(
            &f.schema,
            &f.map,
            &operators(0),
            &first.execution.mutated_child,
            &first.execution.mutated_child_ancestry,
            &first.lineage.state,
            &f.profile_a,
            &second.gamete_a,
            &second.gamete_ancestry_a,
            &f.source_b,
            &f.ancestry_b,
            &f.lineage_b,
            &f.profile_b,
            &second.gamete_b,
            &second.gamete_ancestry_b,
            &second.descendant,
            &second.graph,
            &second.execution,
        )
        .unwrap();
}
