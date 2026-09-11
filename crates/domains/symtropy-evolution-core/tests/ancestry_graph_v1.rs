use symtropy_evolution_core::{
    append_descendant_ancestry_to_graph, assemble_diploid_linked_offspring_from_evidence,
    derive_descendant_ancestry, derive_marker_marginal_poisson_linked_gamete,
    derive_modeled_gamete_ancestry, derive_zero_crossover_linked_gamete, AlleleId,
    AncestryCopyId, AncestryGeneration, AncestryGraphNode, ChromosomeAncestryState,
    ChromosomeDefinition, ChromosomeHaplotype, ChromosomeId, ChromosomeLocus, ChromosomeMap,
    ChromosomeMapId, ChromosomeRecombinationDomain, ChromosomeRecombinationModel,
    ChromosomeRecombinationProfile, ChromosomeRecombinationProfileId,
    DescendantAncestryDerivation, GeneticMapIntervalMicromorgans,
    GeneticMapPositionMicromorgans, HaplotypeAncestryClass, HereditarySchema,
    HereditarySchemaId, LinkedGameteDerivationEvidence, LocusDefinition, LocusId,
    ModeledAncestryGraph, ParentRole, PhasedAncestryState, PhasedChromosomeState,
    PhasedHereditaryState, ReproductionEventId,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}
fn locus(id: &str) -> LocusId {
    LocusId::new(id).unwrap()
}
fn chromosome(id: &str) -> ChromosomeId {
    ChromosomeId::new(id).unwrap()
}
fn ancestry(id: &str) -> AncestryCopyId {
    AncestryCopyId::new(id).unwrap()
}
fn pos(value: u64) -> GeneticMapPositionMicromorgans {
    GeneticMapPositionMicromorgans::new(value)
}
fn interval(start: u64, end: u64) -> GeneticMapIntervalMicromorgans {
    GeneticMapIntervalMicromorgans::new(pos(start), pos(end)).unwrap()
}
fn hap(ids: &[&str]) -> ChromosomeHaplotype {
    ChromosomeHaplotype::new(ids.iter().map(|id| allele(id)).collect())
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("phylo-04d1-schema").unwrap(),
        2,
        [
            LocusDefinition::new(locus("a"), [allele("a0"), allele("a1")]).unwrap(),
            LocusDefinition::new(locus("b"), [allele("b0"), allele("b1")]).unwrap(),
            LocusDefinition::new(locus("c"), [allele("c0"), allele("c1")]).unwrap(),
        ],
    )
    .unwrap()
}

fn map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("phylo-04d1-map").unwrap(),
        schema,
        [ChromosomeDefinition::new(
            chromosome("chr-a"),
            vec![
                ChromosomeLocus::new(locus("a"), pos(1)),
                ChromosomeLocus::new(locus("b"), pos(7_000_001)),
                ChromosomeLocus::new(locus("c"), pos(14_000_001)),
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
    model: ChromosomeRecombinationModel,
) -> ChromosomeRecombinationProfile {
    ChromosomeRecombinationProfile::new(
        ChromosomeRecombinationProfileId::new(id).unwrap(),
        schema,
        map,
        model,
        [ChromosomeRecombinationDomain::new(
            chromosome("chr-a"),
            interval(0, 14_000_002),
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
            chromosome("chr-a"),
            vec![hap(first), hap(second)],
        )],
    )
    .unwrap()
}

fn ancestry_class(slot: u8, ids: &[&str]) -> HaplotypeAncestryClass {
    HaplotypeAncestryClass::new(slot, ids.iter().map(|id| ancestry(id)).collect()).unwrap()
}

fn distinct_ancestry(
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
            chromosome("chr-a"),
            vec![
                ancestry_class(0, &[first]),
                ancestry_class(1, &[second]),
            ],
        )
        .unwrap()],
    )
    .unwrap()
}

struct Fixture {
    schema: HereditarySchema,
    map: ChromosomeMap,
    source_a: PhasedHereditaryState,
    source_b: PhasedHereditaryState,
    ancestry_a: PhasedAncestryState,
    ancestry_b: PhasedAncestryState,
    profile_a: ChromosomeRecombinationProfile,
    profile_b: ChromosomeRecombinationProfile,
    gamete_a: LinkedGameteDerivationEvidence,
    gamete_b: LinkedGameteDerivationEvidence,
    gamete_ancestry_a: symtropy_evolution_core::GameteAncestryDerivation,
    gamete_ancestry_b: symtropy_evolution_core::GameteAncestryDerivation,
    descendant: DescendantAncestryDerivation,
}

fn fixture(event_name: &str, recombinant_parent_a: bool) -> Fixture {
    let schema = schema();
    let map = map(&schema);
    let event = ReproductionEventId::new(event_name).unwrap();
    let source_a = source(
        &schema,
        &map,
        &["a0", "b0", "c0"],
        &["a1", "b1", "c1"],
    );
    let source_b = source(
        &schema,
        &map,
        &["a0", "b1", "c0"],
        &["a1", "b0", "c1"],
    );
    let ancestry_a = distinct_ancestry(&schema, &map, &source_a, "a-root-0", "a-root-1");
    let ancestry_b = distinct_ancestry(&schema, &map, &source_b, "b-root-0", "b-root-1");
    let profile_a = profile(
        &schema,
        &map,
        if recombinant_parent_a { "a-poisson" } else { "a-zero" },
        if recombinant_parent_a {
            ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1
        } else {
            ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1
        },
    );
    let profile_b = profile(
        &schema,
        &map,
        "b-zero",
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
    );
    let gamete_a = if recombinant_parent_a {
        LinkedGameteDerivationEvidence::MarkerMarginalPoisson(
            derive_marker_marginal_poisson_linked_gamete(
                &schema,
                &map,
                &source_a,
                &profile_a,
                &event,
                ParentRole::ParentA,
            )
            .unwrap(),
        )
    } else {
        LinkedGameteDerivationEvidence::ZeroCrossover(
            derive_zero_crossover_linked_gamete(
                &schema,
                &map,
                &source_a,
                &profile_a,
                &event,
                ParentRole::ParentA,
            )
            .unwrap(),
        )
    };
    let gamete_b = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &schema,
            &map,
            &source_b,
            &profile_b,
            &event,
            ParentRole::ParentB,
        )
        .unwrap(),
    );
    let gamete_ancestry_a = derive_modeled_gamete_ancestry(
        &schema,
        &map,
        &source_a,
        &ancestry_a,
        &profile_a,
        &gamete_a,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();
    let gamete_ancestry_b = derive_modeled_gamete_ancestry(
        &schema,
        &map,
        &source_b,
        &ancestry_b,
        &profile_b,
        &gamete_b,
        &event,
        ParentRole::ParentB,
    )
    .unwrap();
    let offspring = assemble_diploid_linked_offspring_from_evidence(
        &schema,
        &map,
        &source_a,
        &profile_a,
        &gamete_a,
        &source_b,
        &profile_b,
        &gamete_b,
        &event,
    )
    .unwrap();
    let descendant = derive_descendant_ancestry(
        &schema,
        &map,
        &source_a,
        &ancestry_a,
        &profile_a,
        &gamete_a,
        &gamete_ancestry_a,
        &source_b,
        &ancestry_b,
        &profile_b,
        &gamete_b,
        &gamete_ancestry_b,
        &offspring,
        &event,
    )
    .unwrap();
    Fixture {
        schema,
        map,
        source_a,
        source_b,
        ancestry_a,
        ancestry_b,
        profile_a,
        profile_b,
        gamete_a,
        gamete_b,
        gamete_ancestry_a,
        gamete_ancestry_b,
        descendant,
    }
}

fn root_graph(fixture: &Fixture, generation: u64) -> ModeledAncestryGraph {
    ModeledAncestryGraph::new_roots(
        &fixture.schema,
        &fixture.map,
        [
            AncestryGraphNode::root(
                ancestry("b-root-1"),
                chromosome("chr-a"),
                AncestryGeneration::new(generation),
            ),
            AncestryGraphNode::root(
                ancestry("a-root-0"),
                chromosome("chr-a"),
                AncestryGeneration::new(generation),
            ),
            AncestryGraphNode::root(
                ancestry("b-root-0"),
                chromosome("chr-a"),
                AncestryGeneration::new(generation),
            ),
            AncestryGraphNode::root(
                ancestry("a-root-1"),
                chromosome("chr-a"),
                AncestryGeneration::new(generation),
            ),
        ],
    )
    .unwrap()
}

fn append(
    graph: &ModeledAncestryGraph,
    fixture: &Fixture,
    generation: u64,
) -> symtropy_evolution_core::AncestryGraphAppendResult {
    append_descendant_ancestry_to_graph(
        graph,
        &fixture.schema,
        &fixture.map,
        &fixture.source_a,
        &fixture.ancestry_a,
        &fixture.profile_a,
        &fixture.gamete_a,
        &fixture.gamete_ancestry_a,
        &fixture.source_b,
        &fixture.ancestry_b,
        &fixture.profile_b,
        &fixture.gamete_b,
        &fixture.gamete_ancestry_b,
        &fixture.descendant,
        AncestryGeneration::new(generation),
    )
    .unwrap()
}

#[test]
fn root_input_order_is_canonical_and_recombinant_edges_persist_exactly() {
    let fixture = fixture("parity-8", true);
    let graph_a = root_graph(&fixture, 0);
    let graph_b = ModeledAncestryGraph::new_roots(
        &fixture.schema,
        &fixture.map,
        [
            AncestryGraphNode::root(
                ancestry("a-root-1"),
                chromosome("chr-a"),
                AncestryGeneration::new(0),
            ),
            AncestryGraphNode::root(
                ancestry("b-root-0"),
                chromosome("chr-a"),
                AncestryGeneration::new(0),
            ),
            AncestryGraphNode::root(
                ancestry("a-root-0"),
                chromosome("chr-a"),
                AncestryGeneration::new(0),
            ),
            AncestryGraphNode::root(
                ancestry("b-root-1"),
                chromosome("chr-a"),
                AncestryGeneration::new(0),
            ),
        ],
    )
    .unwrap();
    assert_eq!(graph_a, graph_b);
    assert_eq!(
        graph_a.canonical_digest(&fixture.schema, &fixture.map).unwrap(),
        graph_b.canonical_digest(&fixture.schema, &fixture.map).unwrap()
    );

    let appended = append(&graph_a, &fixture, 1);
    assert_eq!(appended.graph.nodes.len(), 6);
    assert_eq!(appended.graph.edges.len(), 6);

    let child_a = &fixture.descendant.materialization.descendant_copies[0].child_copy_id;
    let edges_a: Vec<_> = appended
        .graph
        .edges
        .iter()
        .filter(|edge| &edge.child_copy_id == child_a)
        .collect();
    assert_eq!(edges_a.len(), 3);
    assert_eq!(edges_a[0].source_copy_id, edges_a[2].source_copy_id);
    assert_ne!(edges_a[0].source_copy_id, edges_a[1].source_copy_id);
    assert!(edges_a.iter().all(|edge| {
        appended.graph.nodes[&edge.source_copy_id].generation == AncestryGeneration::new(0)
            && appended.graph.nodes[&edge.child_copy_id].generation
                == AncestryGeneration::new(1)
    }));
}

#[test]
fn missing_sources_generation_inversion_and_duplicate_append_fail_closed() {
    let fixture = fixture("parity-8", true);
    let graph = root_graph(&fixture, 0);

    let too_early = append_descendant_ancestry_to_graph(
        &graph,
        &fixture.schema,
        &fixture.map,
        &fixture.source_a,
        &fixture.ancestry_a,
        &fixture.profile_a,
        &fixture.gamete_a,
        &fixture.gamete_ancestry_a,
        &fixture.source_b,
        &fixture.ancestry_b,
        &fixture.profile_b,
        &fixture.gamete_b,
        &fixture.gamete_ancestry_b,
        &fixture.descendant,
        AncestryGeneration::new(0),
    );
    assert!(too_early.is_err());

    let partial_graph = ModeledAncestryGraph::new_roots(
        &fixture.schema,
        &fixture.map,
        [
            AncestryGraphNode::root(
                ancestry("a-root-0"),
                chromosome("chr-a"),
                AncestryGeneration::new(0),
            ),
            AncestryGraphNode::root(
                ancestry("b-root-0"),
                chromosome("chr-a"),
                AncestryGeneration::new(0),
            ),
        ],
    )
    .unwrap();
    let missing_source = append_descendant_ancestry_to_graph(
        &partial_graph,
        &fixture.schema,
        &fixture.map,
        &fixture.source_a,
        &fixture.ancestry_a,
        &fixture.profile_a,
        &fixture.gamete_a,
        &fixture.gamete_ancestry_a,
        &fixture.source_b,
        &fixture.ancestry_b,
        &fixture.profile_b,
        &fixture.gamete_b,
        &fixture.gamete_ancestry_b,
        &fixture.descendant,
        AncestryGeneration::new(1),
    );
    assert!(missing_source.is_err());

    let first = append(&graph, &fixture, 1);
    let duplicate = append_descendant_ancestry_to_graph(
        &first.graph,
        &fixture.schema,
        &fixture.map,
        &fixture.source_a,
        &fixture.ancestry_a,
        &fixture.profile_a,
        &fixture.gamete_a,
        &fixture.gamete_ancestry_a,
        &fixture.source_b,
        &fixture.ancestry_b,
        &fixture.profile_b,
        &fixture.gamete_b,
        &fixture.gamete_ancestry_b,
        &fixture.descendant,
        AncestryGeneration::new(2),
    );
    assert!(duplicate.is_err());
}

#[test]
fn append_receipt_restores_and_replays_exactly() {
    let fixture = fixture("parity-8", true);
    let graph = root_graph(&fixture, 0);
    let appended = append(&graph, &fixture, 1);

    let encoded = serde_json::to_string(&appended).unwrap();
    let restored: symtropy_evolution_core::AncestryGraphAppendResult =
        serde_json::from_str(&encoded).unwrap();
    restored
        .provenance
        .validate_current(
            &graph,
            &fixture.schema,
            &fixture.map,
            &fixture.source_a,
            &fixture.ancestry_a,
            &fixture.profile_a,
            &fixture.gamete_a,
            &fixture.gamete_ancestry_a,
            &fixture.source_b,
            &fixture.ancestry_b,
            &fixture.profile_b,
            &fixture.gamete_b,
            &fixture.gamete_ancestry_b,
            &fixture.descendant,
            AncestryGeneration::new(1),
            &restored.graph,
        )
        .unwrap();
    assert_eq!(restored, appended);
}

#[test]
fn independent_same_generation_births_converge_to_one_graph_identity() {
    let first_fixture = fixture("independent-event-a", false);
    let second_fixture = fixture("independent-event-b", false);
    let root = root_graph(&first_fixture, 0);

    let first_then = append(&root, &first_fixture, 1);
    let first_then_second = append(&first_then.graph, &second_fixture, 1);

    let second_then = append(&root, &second_fixture, 1);
    let second_then_first = append(&second_then.graph, &first_fixture, 1);

    assert_eq!(first_then_second.graph, second_then_first.graph);
    assert_eq!(
        first_then_second
            .graph
            .canonical_digest(&first_fixture.schema, &first_fixture.map)
            .unwrap(),
        second_then_first
            .graph
            .canonical_digest(&first_fixture.schema, &first_fixture.map)
            .unwrap()
    );
    assert_ne!(
        first_then_second.provenance.canonical_digest(),
        second_then_first.provenance.canonical_digest()
    );
}
