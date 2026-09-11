use std::collections::BTreeSet;
use symtropy_evolution_core::{
    append_descendant_ancestry_to_simplified_graph,
    assemble_diploid_linked_offspring_from_evidence, derive_descendant_ancestry,
    derive_modeled_gamete_ancestry, derive_zero_crossover_linked_gamete,
    resimplify_simplified_ancestry, simplify_modeled_ancestry, AlleleId, AncestryCopyId,
    AncestryGeneration, AncestryGraphNode, AncestryRetentionSet,
    AncestrySimplificationProfile, ChromosomeAncestryState, ChromosomeDefinition,
    ChromosomeHaplotype, ChromosomeId, ChromosomeLocus, ChromosomeMap, ChromosomeMapId,
    ChromosomeRecombinationDomain, ChromosomeRecombinationModel,
    ChromosomeRecombinationProfile, ChromosomeRecombinationProfileId,
    GeneticMapIntervalMicromorgans, GeneticMapPositionMicromorgans,
    HaplotypeAncestryClass, HereditarySchema, HereditarySchemaId,
    LinkedGameteDerivationEvidence, LocusDefinition, LocusId, ModeledAncestryGraph,
    ParentRole, PhasedAncestryState, PhasedChromosomeState, PhasedHereditaryState,
    ReproductionEventId,
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

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("forward-schema").unwrap(),
        2,
        [LocusDefinition::new(locus("a"), [allele("a0"), allele("a1")]).unwrap()],
    )
    .unwrap()
}

fn map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("forward-map").unwrap(),
        schema,
        [ChromosomeDefinition::new(
            chromosome("chr-a"),
            vec![ChromosomeLocus::new(locus("a"), pos(0))],
        )
        .unwrap()],
    )
    .unwrap()
}

fn interval() -> GeneticMapIntervalMicromorgans {
    GeneticMapIntervalMicromorgans::new(pos(0), pos(1)).unwrap()
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
        [ChromosomeRecombinationDomain::new(chromosome("chr-a"), interval()).unwrap()],
    )
    .unwrap()
}

fn hap(id: &str) -> ChromosomeHaplotype {
    ChromosomeHaplotype::new(vec![allele(id)])
}

fn source(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
) -> PhasedHereditaryState {
    PhasedHereditaryState::new(
        schema,
        map,
        [PhasedChromosomeState::new(
            chromosome("chr-a"),
            vec![hap("a0"), hap("a1")],
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
            chromosome("chr-a"),
            vec![
                HaplotypeAncestryClass::new(0, vec![ancestry(first)]).unwrap(),
                HaplotypeAncestryClass::new(1, vec![ancestry(second)]).unwrap(),
            ],
        )
        .unwrap()],
    )
    .unwrap()
}

struct Fixture {
    schema: HereditarySchema,
    map: ChromosomeMap,
    parent_a_source: PhasedHereditaryState,
    parent_b_source: PhasedHereditaryState,
    parent_a_ancestry: PhasedAncestryState,
    parent_b_ancestry: PhasedAncestryState,
    parent_a_profile: ChromosomeRecombinationProfile,
    parent_b_profile: ChromosomeRecombinationProfile,
    parent_a_gamete: LinkedGameteDerivationEvidence,
    parent_b_gamete: LinkedGameteDerivationEvidence,
    parent_a_gamete_ancestry: symtropy_evolution_core::GameteAncestryDerivation,
    parent_b_gamete_ancestry: symtropy_evolution_core::GameteAncestryDerivation,
    descendant: symtropy_evolution_core::DescendantAncestryDerivation,
    source_graph: symtropy_evolution_core::SimplifiedAncestryGraph,
}

fn fixture() -> Fixture {
    let schema = schema();
    let map = map(&schema);
    let parent_a_source = source(&schema, &map);
    let parent_b_source = source(&schema, &map);
    let parent_a_ancestry = ancestry_state(
        &schema,
        &map,
        &parent_a_source,
        "parent-a-0",
        "parent-a-1",
    );
    let parent_b_ancestry = ancestry_state(
        &schema,
        &map,
        &parent_b_source,
        "parent-b-0",
        "parent-b-1",
    );
    let parent_a_profile = profile(&schema, &map, "parent-a-zero");
    let parent_b_profile = profile(&schema, &map, "parent-b-zero");
    let event = ReproductionEventId::new("forward-child-1").unwrap();

    let parent_a_gamete = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &schema,
            &map,
            &parent_a_source,
            &parent_a_profile,
            &event,
            ParentRole::ParentA,
        )
        .unwrap(),
    );
    let parent_b_gamete = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &schema,
            &map,
            &parent_b_source,
            &parent_b_profile,
            &event,
            ParentRole::ParentB,
        )
        .unwrap(),
    );
    let parent_a_gamete_ancestry = derive_modeled_gamete_ancestry(
        &schema,
        &map,
        &parent_a_source,
        &parent_a_ancestry,
        &parent_a_profile,
        &parent_a_gamete,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();
    let parent_b_gamete_ancestry = derive_modeled_gamete_ancestry(
        &schema,
        &map,
        &parent_b_source,
        &parent_b_ancestry,
        &parent_b_profile,
        &parent_b_gamete,
        &event,
        ParentRole::ParentB,
    )
    .unwrap();
    let offspring = assemble_diploid_linked_offspring_from_evidence(
        &schema,
        &map,
        &parent_a_source,
        &parent_a_profile,
        &parent_a_gamete,
        &parent_b_source,
        &parent_b_profile,
        &parent_b_gamete,
        &event,
    )
    .unwrap();
    let descendant = derive_descendant_ancestry(
        &schema,
        &map,
        &parent_a_source,
        &parent_a_ancestry,
        &parent_a_profile,
        &parent_a_gamete,
        &parent_a_gamete_ancestry,
        &parent_b_source,
        &parent_b_ancestry,
        &parent_b_profile,
        &parent_b_gamete,
        &parent_b_gamete_ancestry,
        &offspring,
        &event,
    )
    .unwrap();

    let root_ids = [
        ancestry("parent-a-0"),
        ancestry("parent-a-1"),
        ancestry("parent-b-0"),
        ancestry("parent-b-1"),
    ];
    let d1 = ModeledAncestryGraph::new_roots(
        &schema,
        &map,
        root_ids.iter().cloned().map(|id| {
            AncestryGraphNode::root(id, chromosome("chr-a"), AncestryGeneration::new(0))
        }),
    )
    .unwrap();
    let root_retention = AncestryRetentionSet::new(root_ids.to_vec(), vec![]).unwrap();
    let source_graph = simplify_modeled_ancestry(
        &d1,
        &root_retention,
        AncestrySimplificationProfile::FocalProtectedRootsAndBranchesV1,
        &schema,
        &map,
    )
    .unwrap()
    .graph;

    Fixture {
        schema,
        map,
        parent_a_source,
        parent_b_source,
        parent_a_ancestry,
        parent_b_ancestry,
        parent_a_profile,
        parent_b_profile,
        parent_a_gamete,
        parent_b_gamete,
        parent_a_gamete_ancestry,
        parent_b_gamete_ancestry,
        descendant,
        source_graph,
    }
}

fn next_retention(f: &Fixture) -> (AncestryRetentionSet, AncestryCopyId) {
    let child_ids: Vec<_> = f
        .descendant
        .materialization
        .descendant_copies
        .iter()
        .map(|copy| copy.child_copy_id.clone())
        .collect();
    let used: BTreeSet<_> = f
        .descendant
        .materialization
        .edges
        .iter()
        .map(|edge| edge.source_copy_id.clone())
        .collect();
    let protected = f
        .source_graph
        .nodes
        .keys()
        .find(|id| !used.contains(*id))
        .unwrap()
        .clone();
    (
        AncestryRetentionSet::new(child_ids, vec![protected.clone()]).unwrap(),
        protected,
    )
}

#[test]
fn exact_descendant_appends_then_resimplifies_under_new_focal_authority() {
    let f = fixture();
    let source_before = f.source_graph.clone();
    let (next_retention, protected) = next_retention(&f);

    let appended = append_descendant_ancestry_to_simplified_graph(
        &f.source_graph,
        &f.schema,
        &f.map,
        &f.parent_a_source,
        &f.parent_a_ancestry,
        &f.parent_a_profile,
        &f.parent_a_gamete,
        &f.parent_a_gamete_ancestry,
        &f.parent_b_source,
        &f.parent_b_ancestry,
        &f.parent_b_profile,
        &f.parent_b_gamete,
        &f.parent_b_gamete_ancestry,
        &f.descendant,
        AncestryGeneration::new(1),
        &next_retention,
    )
    .unwrap();

    assert_eq!(f.source_graph, source_before);
    assert_eq!(appended.graph.retention, next_retention);
    assert_eq!(appended.provenance.appended_node_ids().len(), 2);
    assert_eq!(appended.provenance.appended_edges().len(), 2);
    appended.graph.validate_current(&f.schema, &f.map).unwrap();

    let used: BTreeSet<_> = f
        .descendant
        .materialization
        .edges
        .iter()
        .map(|edge| edge.source_copy_id.clone())
        .collect();
    let resimplified = resimplify_simplified_ancestry(
        &appended.graph,
        &next_retention,
        &f.schema,
        &f.map,
    )
    .unwrap();

    for child in &next_retention.focal_copy_ids {
        assert!(resimplified.graph.nodes.contains_key(child));
    }
    for source in &used {
        assert!(resimplified.graph.nodes.contains_key(source));
    }
    assert!(resimplified.graph.nodes.contains_key(&protected));

    let expected_node_count = next_retention.focal_copy_ids.len() + used.len() + 1;
    assert_eq!(resimplified.graph.nodes.len(), expected_node_count);

    let again = resimplify_simplified_ancestry(
        &resimplified.graph,
        &next_retention,
        &f.schema,
        &f.map,
    )
    .unwrap();
    assert_eq!(again.graph, resimplified.graph);
    assert_eq!(
        again.graph.canonical_digest(&f.schema, &f.map).unwrap(),
        resimplified
            .graph
            .canonical_digest(&f.schema, &f.map)
            .unwrap()
    );
}

#[test]
fn append_and_resimplification_receipts_restore_only_via_exact_replay() {
    let f = fixture();
    let (next_retention, _) = next_retention(&f);
    let appended = append_descendant_ancestry_to_simplified_graph(
        &f.source_graph,
        &f.schema,
        &f.map,
        &f.parent_a_source,
        &f.parent_a_ancestry,
        &f.parent_a_profile,
        &f.parent_a_gamete,
        &f.parent_a_gamete_ancestry,
        &f.parent_b_source,
        &f.parent_b_ancestry,
        &f.parent_b_profile,
        &f.parent_b_gamete,
        &f.parent_b_gamete_ancestry,
        &f.descendant,
        AncestryGeneration::new(1),
        &next_retention,
    )
    .unwrap();

    let encoded = serde_json::to_string(&appended).unwrap();
    let restored: symtropy_evolution_core::SimplifiedAncestryAppendResult =
        serde_json::from_str(&encoded).unwrap();
    restored
        .provenance
        .validate_current(
            &f.source_graph,
            &f.schema,
            &f.map,
            &f.parent_a_source,
            &f.parent_a_ancestry,
            &f.parent_a_profile,
            &f.parent_a_gamete,
            &f.parent_a_gamete_ancestry,
            &f.parent_b_source,
            &f.parent_b_ancestry,
            &f.parent_b_profile,
            &f.parent_b_gamete,
            &f.parent_b_gamete_ancestry,
            &f.descendant,
            AncestryGeneration::new(1),
            &next_retention,
            &restored.graph,
        )
        .unwrap();

    let compacted = resimplify_simplified_ancestry(
        &restored.graph,
        &next_retention,
        &f.schema,
        &f.map,
    )
    .unwrap();
    let encoded = serde_json::to_string(&compacted).unwrap();
    let restored_compacted: symtropy_evolution_core::SimplifiedAncestryResimplificationResult =
        serde_json::from_str(&encoded).unwrap();
    restored_compacted
        .provenance
        .validate_current(
            &restored.graph,
            &next_retention,
            &f.schema,
            &f.map,
            &restored_compacted.graph,
        )
        .unwrap();

    let wrong = AncestryRetentionSet::new(vec![next_retention.focal_copy_ids[0].clone()], vec![])
        .unwrap();
    assert!(restored_compacted
        .provenance
        .validate_current(
            &restored.graph,
            &wrong,
            &f.schema,
            &f.map,
            &restored_compacted.graph,
        )
        .is_err());
}

#[test]
fn append_rejects_generation_reuse_duplicate_child_and_missing_source() {
    let f = fixture();
    let (next_retention, _) = next_retention(&f);

    assert!(append_descendant_ancestry_to_simplified_graph(
        &f.source_graph,
        &f.schema,
        &f.map,
        &f.parent_a_source,
        &f.parent_a_ancestry,
        &f.parent_a_profile,
        &f.parent_a_gamete,
        &f.parent_a_gamete_ancestry,
        &f.parent_b_source,
        &f.parent_b_ancestry,
        &f.parent_b_profile,
        &f.parent_b_gamete,
        &f.parent_b_gamete_ancestry,
        &f.descendant,
        AncestryGeneration::new(0),
        &next_retention,
    )
    .is_err());

    let appended = append_descendant_ancestry_to_simplified_graph(
        &f.source_graph,
        &f.schema,
        &f.map,
        &f.parent_a_source,
        &f.parent_a_ancestry,
        &f.parent_a_profile,
        &f.parent_a_gamete,
        &f.parent_a_gamete_ancestry,
        &f.parent_b_source,
        &f.parent_b_ancestry,
        &f.parent_b_profile,
        &f.parent_b_gamete,
        &f.parent_b_gamete_ancestry,
        &f.descendant,
        AncestryGeneration::new(1),
        &next_retention,
    )
    .unwrap();
    assert!(append_descendant_ancestry_to_simplified_graph(
        &appended.graph,
        &f.schema,
        &f.map,
        &f.parent_a_source,
        &f.parent_a_ancestry,
        &f.parent_a_profile,
        &f.parent_a_gamete,
        &f.parent_a_gamete_ancestry,
        &f.parent_b_source,
        &f.parent_b_ancestry,
        &f.parent_b_profile,
        &f.parent_b_gamete,
        &f.parent_b_gamete_ancestry,
        &f.descendant,
        AncestryGeneration::new(2),
        &next_retention,
    )
    .is_err());

    let missing_source = f.descendant.materialization.edges[0].source_copy_id.clone();
    let mut bad_source = f.source_graph.clone();
    bad_source.nodes.remove(&missing_source);
    let fallback = bad_source.nodes.keys().next().unwrap().clone();
    bad_source.retention = AncestryRetentionSet::new(vec![fallback], vec![]).unwrap();
    bad_source.validate_current(&f.schema, &f.map).unwrap();
    assert!(append_descendant_ancestry_to_simplified_graph(
        &bad_source,
        &f.schema,
        &f.map,
        &f.parent_a_source,
        &f.parent_a_ancestry,
        &f.parent_a_profile,
        &f.parent_a_gamete,
        &f.parent_a_gamete_ancestry,
        &f.parent_b_source,
        &f.parent_b_ancestry,
        &f.parent_b_profile,
        &f.parent_b_gamete,
        &f.parent_b_gamete_ancestry,
        &f.descendant,
        AncestryGeneration::new(1),
        &next_retention,
    )
    .is_err());
}
