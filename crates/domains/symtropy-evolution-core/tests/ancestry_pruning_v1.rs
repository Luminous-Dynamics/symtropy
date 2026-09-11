use symtropy_evolution_core::{
    prune_ancestry_reachability, AlleleId, AncestryCopyId, AncestryGeneration,
    AncestryGraphEdge, AncestryGraphNode, AncestryRetentionSet, ChromosomeDefinition,
    ChromosomeId, ChromosomeLocus, ChromosomeMap, ChromosomeMapId,
    GeneticMapPositionMicromorgans, HereditarySchema, HereditarySchemaId, LocusDefinition,
    LocusId, ModeledAncestryGraph, ReproductionEventId,
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
        HereditarySchemaId::new("pruning-v1-schema").unwrap(),
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
        ChromosomeMapId::new("pruning-v1-map").unwrap(),
        schema,
        [ChromosomeDefinition::new(
            chromosome("chr-a"),
            vec![
                ChromosomeLocus::new(locus("a"), pos(1)),
                ChromosomeLocus::new(locus("b"), pos(2)),
            ],
        )
        .unwrap()],
    )
    .unwrap()
}

fn edge(source: &str, child: &str, locus_id: &str) -> AncestryGraphEdge {
    AncestryGraphEdge {
        chromosome_id: chromosome("chr-a"),
        locus_id: locus(locus_id),
        source_copy_id: ancestry(source),
        child_copy_id: ancestry(child),
    }
}

fn graph(schema: &HereditarySchema, map: &ChromosomeMap) -> ModeledAncestryGraph {
    let mut graph = ModeledAncestryGraph::new_roots(
        schema,
        map,
        [
            AncestryGraphNode::root(
                ancestry("root-c"),
                chromosome("chr-a"),
                AncestryGeneration::new(0),
            ),
            AncestryGraphNode::root(
                ancestry("root-a"),
                chromosome("chr-a"),
                AncestryGeneration::new(0),
            ),
            AncestryGraphNode::root(
                ancestry("root-b"),
                chromosome("chr-a"),
                AncestryGeneration::new(0),
            ),
        ],
    )
    .unwrap();

    for (id, generation, event) in [
        ("copy-x", 1, "birth-x"),
        ("copy-y", 2, "birth-y"),
        ("copy-z", 1, "birth-z"),
    ] {
        graph.nodes.insert(
            ancestry(id),
            AncestryGraphNode {
                copy_id: ancestry(id),
                chromosome_id: chromosome("chr-a"),
                generation: AncestryGeneration::new(generation),
                birth_event: Some(ReproductionEventId::new(event).unwrap()),
            },
        );
    }

    // copy-x is mosaic at modeled loci: a <- root-a, b <- root-b.
    // focal copy-y inherits a <- copy-x, b <- root-a.
    // Thus retaining copy-y also retains root-b solely because copy-x must keep
    // complete D1 parentage at its non-focal b locus.
    // copy-z is a completely unreachable lineage from root-c.
    graph.edges = vec![
        edge("root-a", "copy-x", "a"),
        edge("root-b", "copy-x", "b"),
        edge("copy-x", "copy-y", "a"),
        edge("root-a", "copy-y", "b"),
        edge("root-c", "copy-z", "a"),
        edge("root-c", "copy-z", "b"),
    ];
    graph.edges.sort();
    graph.validate_current(schema, map).unwrap();
    graph
}

#[test]
fn focal_closure_prunes_unreachable_lineage_but_preserves_complete_parentage() {
    let schema = schema();
    let map = map(&schema);
    let source = graph(&schema, &map);
    let original = source.clone();
    let retention = AncestryRetentionSet::new(vec![ancestry("copy-y")], vec![]).unwrap();

    let pruned = prune_ancestry_reachability(&source, &retention, &schema, &map).unwrap();
    pruned.graph.validate_current(&schema, &map).unwrap();
    assert_eq!(source, original);

    for id in ["copy-y", "copy-x", "root-a", "root-b"] {
        assert!(pruned.graph.nodes.contains_key(&ancestry(id)));
    }
    for id in ["copy-z", "root-c"] {
        assert!(!pruned.graph.nodes.contains_key(&ancestry(id)));
    }

    // root-b is retained even though copy-y's direct b-locus source is root-a:
    // copy-x is retained for y's a locus and therefore must keep all of its own
    // incoming D1 parentage, including b <- root-b.
    assert!(pruned.graph.edges.contains(&edge("root-b", "copy-x", "b")));
    assert_eq!(pruned.graph.edges.len(), 4);
    assert!(pruned
        .graph
        .edges
        .iter()
        .all(|retained| source.edges.contains(retained)));
}

#[test]
fn protected_copy_keeps_its_complete_non_focal_ancestry() {
    let schema = schema();
    let map = map(&schema);
    let source = graph(&schema, &map);
    let retention = AncestryRetentionSet::new(
        vec![ancestry("copy-y")],
        vec![ancestry("copy-z")],
    )
    .unwrap();

    let pruned = prune_ancestry_reachability(&source, &retention, &schema, &map).unwrap();
    assert_eq!(pruned.graph, source);
}

#[test]
fn retention_construction_is_order_canonical_and_restore_is_fail_closed() {
    let schema = schema();
    let map = map(&schema);
    let source = graph(&schema, &map);
    let a = AncestryRetentionSet::new(
        vec![ancestry("copy-y"), ancestry("copy-x")],
        vec![ancestry("copy-z")],
    )
    .unwrap();
    let b = AncestryRetentionSet::new(
        vec![ancestry("copy-x"), ancestry("copy-y")],
        vec![ancestry("copy-z")],
    )
    .unwrap();
    assert_eq!(a, b);
    assert_eq!(
        a.canonical_digest(&source, &schema, &map).unwrap(),
        b.canonical_digest(&source, &schema, &map).unwrap()
    );

    let encoded = serde_json::to_string(&a).unwrap();
    let mut restored: AncestryRetentionSet = serde_json::from_str(&encoded).unwrap();
    restored.focal_copy_ids.reverse();
    assert!(restored.validate_current(&source, &schema, &map).is_err());

    let missing =
        AncestryRetentionSet::new(vec![ancestry("not-in-graph")], Vec::new()).unwrap();
    assert!(missing.validate_current(&source, &schema, &map).is_err());
}

#[test]
fn pruning_is_idempotent_and_receipt_replays_after_restore() {
    let schema = schema();
    let map = map(&schema);
    let source = graph(&schema, &map);
    let retention = AncestryRetentionSet::new(vec![ancestry("copy-y")], vec![]).unwrap();
    let first = prune_ancestry_reachability(&source, &retention, &schema, &map).unwrap();
    let second =
        prune_ancestry_reachability(&first.graph, &retention, &schema, &map).unwrap();
    assert_eq!(first.graph, second.graph);
    assert_eq!(
        first.graph.canonical_digest(&schema, &map).unwrap(),
        second.graph.canonical_digest(&schema, &map).unwrap()
    );

    let encoded = serde_json::to_string(&first).unwrap();
    let restored: symtropy_evolution_core::AncestryReachabilityPruningResult =
        serde_json::from_str(&encoded).unwrap();
    restored
        .provenance
        .validate_current(&source, &retention, &schema, &map, &restored.graph)
        .unwrap();
    assert_eq!(restored, first);
}

#[test]
fn changing_focal_set_changes_pruned_graph_and_authority() {
    let schema = schema();
    let map = map(&schema);
    let source = graph(&schema, &map);
    let focal_y = AncestryRetentionSet::new(vec![ancestry("copy-y")], vec![]).unwrap();
    let focal_z = AncestryRetentionSet::new(vec![ancestry("copy-z")], vec![]).unwrap();
    let y = prune_ancestry_reachability(&source, &focal_y, &schema, &map).unwrap();
    let z = prune_ancestry_reachability(&source, &focal_z, &schema, &map).unwrap();

    assert_ne!(y.graph, z.graph);
    assert_ne!(
        y.graph.canonical_digest(&schema, &map).unwrap(),
        z.graph.canonical_digest(&schema, &map).unwrap()
    );
    assert_ne!(
        y.provenance.canonical_digest(),
        z.provenance.canonical_digest()
    );
}
