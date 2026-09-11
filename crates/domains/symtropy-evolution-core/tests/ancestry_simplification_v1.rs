use std::collections::BTreeMap;
use symtropy_evolution_core::{
    simplify_modeled_ancestry, AlleleId, AncestryCopyId, AncestryGeneration,
    AncestryGraphEdge, AncestryGraphNode, AncestryRetentionSet,
    AncestrySimplificationProfile, ChromosomeDefinition, ChromosomeId, ChromosomeLocus,
    ChromosomeMap, ChromosomeMapId, GeneticMapPositionMicromorgans, HereditarySchema,
    HereditarySchemaId, LocusDefinition, LocusId, ModeledAncestryGraph,
    ReproductionEventId, SimplifiedAncestryEdge, MODELED_ANCESTRY_GRAPH_VERSION,
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
fn copy(id: &str) -> AncestryCopyId {
    AncestryCopyId::new(id).unwrap()
}
fn event(id: &str) -> ReproductionEventId {
    ReproductionEventId::new(id).unwrap()
}
fn pos(value: u64) -> GeneticMapPositionMicromorgans {
    GeneticMapPositionMicromorgans::new(value)
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("phylo-04d2b1-schema").unwrap(),
        2,
        [
            LocusDefinition::new(locus("a"), [allele("a0"), allele("a1")]).unwrap(),
            LocusDefinition::new(locus("b"), [allele("b0"), allele("b1")]).unwrap(),
        ],
    )
    .unwrap()
}

fn map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("phylo-04d2b1-map").unwrap(),
        schema,
        [ChromosomeDefinition::new(
            chromosome("chr-a"),
            vec![
                ChromosomeLocus::new(locus("a"), pos(0)),
                ChromosomeLocus::new(locus("b"), pos(1_000_000)),
            ],
        )
        .unwrap()],
    )
    .unwrap()
}

fn root(id: &str) -> AncestryGraphNode {
    AncestryGraphNode::root(copy(id), chromosome("chr-a"), AncestryGeneration::new(0))
}

fn descendant(id: &str, generation: u64) -> AncestryGraphNode {
    AncestryGraphNode {
        copy_id: copy(id),
        chromosome_id: chromosome("chr-a"),
        generation: AncestryGeneration::new(generation),
        birth_event: Some(event(&format!("birth-{id}"))),
    }
}

fn edge(source: &str, child: &str, locus_id: &str) -> AncestryGraphEdge {
    AncestryGraphEdge {
        chromosome_id: chromosome("chr-a"),
        locus_id: locus(locus_id),
        source_copy_id: copy(source),
        child_copy_id: copy(child),
    }
}

/// Graph shape:
///
/// locus a: root-a -> x -> y
///                    \-> z
/// locus b: root-b ------> y
///                    \--> z
///          root-b -> x       (D2A whole-copy closure only; not active for y/z at locus b)
///
/// unreachable: root-dead -> dead at both loci.
fn graph(schema: &HereditarySchema, map: &ChromosomeMap) -> ModeledAncestryGraph {
    let nodes = [
        root("root-a"),
        root("root-b"),
        root("root-dead"),
        descendant("x", 1),
        descendant("dead", 1),
        descendant("y", 2),
        descendant("z", 2),
    ]
    .into_iter()
    .map(|node| (node.copy_id.clone(), node))
    .collect::<BTreeMap<_, _>>();

    let mut edges = vec![
        edge("root-a", "x", "a"),
        edge("root-b", "x", "b"),
        edge("x", "y", "a"),
        edge("root-b", "y", "b"),
        edge("x", "z", "a"),
        edge("root-b", "z", "b"),
        edge("root-dead", "dead", "a"),
        edge("root-dead", "dead", "b"),
    ];
    edges.sort();

    let graph = ModeledAncestryGraph {
        graph_version: MODELED_ANCESTRY_GRAPH_VERSION,
        schema_digest: schema.canonical_digest().unwrap(),
        chromosome_map_digest: map.canonical_digest(schema).unwrap(),
        nodes,
        edges,
    };
    graph.validate_current(schema, map).unwrap();
    graph
}

fn simplified_edge(source: &str, child: &str, locus_id: &str) -> SimplifiedAncestryEdge {
    SimplifiedAncestryEdge {
        chromosome_id: chromosome("chr-a"),
        locus_id: locus(locus_id),
        source_copy_id: copy(source),
        child_copy_id: copy(child),
    }
}

#[test]
fn pure_unary_path_contracts_and_unreachable_lineage_disappears() {
    let schema = schema();
    let map = map(&schema);
    let source = graph(&schema, &map);
    let retention = AncestryRetentionSet::new(vec![copy("y")], vec![]).unwrap();

    let result = simplify_modeled_ancestry(
        &source,
        &retention,
        AncestrySimplificationProfile::FocalProtectedRootsAndBranchesV1,
        &schema,
        &map,
    )
    .unwrap();

    assert_eq!(
        result.graph.nodes.keys().cloned().collect::<Vec<_>>(),
        vec![copy("root-a"), copy("root-b"), copy("y")]
    );
    assert_eq!(
        result.graph.edges,
        vec![
            simplified_edge("root-a", "y", "a"),
            simplified_edge("root-b", "y", "b"),
        ]
    );
    assert!(!result.graph.nodes.contains_key(&copy("x")));
    assert!(!result.graph.nodes.contains_key(&copy("root-dead")));
    assert!(!result.graph.nodes.contains_key(&copy("dead")));
    result.graph.validate_current(&schema, &map).unwrap();
}

#[test]
fn protected_unary_copy_is_retained_and_divides_the_path() {
    let schema = schema();
    let map = map(&schema);
    let source = graph(&schema, &map);
    let retention = AncestryRetentionSet::new(vec![copy("y")], vec![copy("x")]).unwrap();

    let result = simplify_modeled_ancestry(
        &source,
        &retention,
        AncestrySimplificationProfile::FocalProtectedRootsAndBranchesV1,
        &schema,
        &map,
    )
    .unwrap();

    assert!(result.graph.nodes.contains_key(&copy("x")));
    assert!(result
        .graph
        .edges
        .contains(&simplified_edge("root-a", "x", "a")));
    assert!(result
        .graph
        .edges
        .contains(&simplified_edge("x", "y", "a")));
    assert!(result
        .graph
        .edges
        .contains(&simplified_edge("root-b", "x", "b")));
    assert!(result
        .graph
        .edges
        .contains(&simplified_edge("root-b", "y", "b")));
}

#[test]
fn shared_branch_is_retained_only_on_the_locus_where_it_branches() {
    let schema = schema();
    let map = map(&schema);
    let source = graph(&schema, &map);
    let retention =
        AncestryRetentionSet::new(vec![copy("z"), copy("y")], vec![]).unwrap();

    let result = simplify_modeled_ancestry(
        &source,
        &retention,
        AncestrySimplificationProfile::FocalProtectedRootsAndBranchesV1,
        &schema,
        &map,
    )
    .unwrap();

    assert!(result.graph.nodes.contains_key(&copy("x")));
    assert!(result
        .graph
        .edges
        .contains(&simplified_edge("root-a", "x", "a")));
    assert!(result
        .graph
        .edges
        .contains(&simplified_edge("x", "y", "a")));
    assert!(result
        .graph
        .edges
        .contains(&simplified_edge("x", "z", "a")));

    // X remains globally because it is a real branch at locus a, but it is not
    // on either focal path at locus b and therefore has no locus-b edge.
    assert!(!result.graph.edges.iter().any(|edge| {
        edge.locus_id == locus("b")
            && (edge.source_copy_id == copy("x") || edge.child_copy_id == copy("x"))
    }));
    assert!(result
        .graph
        .edges
        .contains(&simplified_edge("root-b", "y", "b")));
    assert!(result
        .graph
        .edges
        .contains(&simplified_edge("root-b", "z", "b")));
    result.graph.validate_current(&schema, &map).unwrap();
}

#[test]
fn end_only_simplification_is_deterministic_and_restore_requires_replay() {
    let schema = schema();
    let map = map(&schema);
    let source = graph(&schema, &map);
    let retention =
        AncestryRetentionSet::new(vec![copy("y"), copy("z")], vec![copy("x")]).unwrap();
    let profile = AncestrySimplificationProfile::FocalProtectedRootsAndBranchesV1;

    let first = simplify_modeled_ancestry(&source, &retention, profile, &schema, &map).unwrap();
    let second = simplify_modeled_ancestry(&source, &retention, profile, &schema, &map).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        first.graph.canonical_digest(&schema, &map).unwrap(),
        second.graph.canonical_digest(&schema, &map).unwrap()
    );

    let encoded = serde_json::to_string(&first).unwrap();
    let restored: symtropy_evolution_core::AncestrySimplificationResult =
        serde_json::from_str(&encoded).unwrap();
    restored
        .provenance
        .validate_current(
            &source,
            &retention,
            profile,
            &schema,
            &map,
            &restored.graph,
        )
        .unwrap();

    let wrong_retention = AncestryRetentionSet::new(vec![copy("y")], vec![]).unwrap();
    assert!(restored
        .provenance
        .validate_current(
            &source,
            &wrong_retention,
            profile,
            &schema,
            &map,
            &restored.graph,
        )
        .is_err());
}

#[test]
fn restored_branch_cannot_be_severed_from_its_own_locus_ancestor() {
    let schema = schema();
    let map = map(&schema);
    let source = graph(&schema, &map);
    let retention =
        AncestryRetentionSet::new(vec![copy("y"), copy("z")], vec![]).unwrap();
    let mut result = simplify_modeled_ancestry(
        &source,
        &retention,
        AncestrySimplificationProfile::FocalProtectedRootsAndBranchesV1,
        &schema,
        &map,
    )
    .unwrap();

    result.graph.edges.retain(|edge| {
        !(edge.locus_id == locus("a")
            && edge.source_copy_id == copy("root-a")
            && edge.child_copy_id == copy("x"))
    });

    assert!(result.graph.validate_current(&schema, &map).is_err());
}
