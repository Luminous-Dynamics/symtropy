use std::collections::BTreeMap;
use symtropy_evolution_core::{
    resimplify_simplified_ancestry, AlleleId, AncestryCopyId, AncestryGeneration,
    AncestryGraphNode, AncestryRetentionSet, AncestrySimplificationProfile,
    ChromosomeDefinition, ChromosomeId, ChromosomeLocus, ChromosomeMap, ChromosomeMapId,
    GeneticMapPositionMicromorgans, HereditarySchema, HereditarySchemaId, LocusDefinition,
    LocusId, ReproductionEventId, SimplifiedAncestryEdge, SimplifiedAncestryGraph,
    SimplifiedAncestryGraphDigest, SIMPLIFIED_ANCESTRY_GRAPH_VERSION,
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
fn copy(id: impl Into<String>) -> AncestryCopyId {
    AncestryCopyId::new(id).unwrap()
}
fn event(id: impl Into<String>) -> ReproductionEventId {
    ReproductionEventId::new(id).unwrap()
}
fn pos(value: u64) -> GeneticMapPositionMicromorgans {
    GeneticMapPositionMicromorgans::new(value)
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("schedule-invariance-schema").unwrap(),
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
        ChromosomeMapId::new("schedule-invariance-map").unwrap(),
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

fn initial_graph(schema: &HereditarySchema, map: &ChromosomeMap) -> SimplifiedAncestryGraph {
    let roots = ["root-a", "root-b", "root-dead", "root-protected"];
    let nodes = roots
        .into_iter()
        .map(root)
        .map(|node| (node.copy_id.clone(), node))
        .collect::<BTreeMap<_, _>>();
    let graph = SimplifiedAncestryGraph {
        graph_version: SIMPLIFIED_ANCESTRY_GRAPH_VERSION,
        schema_digest: schema.canonical_digest().unwrap(),
        chromosome_map_digest: map.canonical_digest(schema).unwrap(),
        profile: AncestrySimplificationProfile::FocalProtectedRootsAndBranchesV1,
        retention: AncestryRetentionSet::new(
            vec![copy("root-a"), copy("root-b"), copy("root-dead")],
            vec![copy("root-protected")],
        )
        .unwrap(),
        nodes,
        edges: Vec::new(),
    };
    graph.validate_current(schema, map).unwrap();
    graph
}

#[derive(Clone)]
struct Birth {
    id: AncestryCopyId,
    source_a: AncestryCopyId,
    source_b: AncestryCopyId,
}

impl Birth {
    fn unary(id: AncestryCopyId, source: AncestryCopyId) -> Self {
        Self {
            id,
            source_a: source.clone(),
            source_b: source,
        }
    }

    fn mosaic(id: AncestryCopyId, source_a: AncestryCopyId, source_b: AncestryCopyId) -> Self {
        Self {
            id,
            source_a,
            source_b,
        }
    }
}

fn synthetic_append_generation(
    source: &SimplifiedAncestryGraph,
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    generation: u64,
    births: &[Birth],
    focal: Vec<AncestryCopyId>,
) -> SimplifiedAncestryGraph {
    source.validate_current(schema, map).unwrap();
    let mut graph = source.clone();

    for birth in births {
        assert!(!graph.nodes.contains_key(&birth.id));
        for source_id in [&birth.source_a, &birth.source_b] {
            let source_node = source.nodes.get(source_id).unwrap();
            assert!(source_node.generation.get() < generation);
        }
        graph.nodes.insert(
            birth.id.clone(),
            AncestryGraphNode {
                copy_id: birth.id.clone(),
                chromosome_id: chromosome("chr-a"),
                generation: AncestryGeneration::new(generation),
                birth_event: Some(event(format!("birth-{}", birth.id.as_str()))),
            },
        );
        graph.edges.push(SimplifiedAncestryEdge {
            chromosome_id: chromosome("chr-a"),
            locus_id: locus("a"),
            source_copy_id: birth.source_a.clone(),
            child_copy_id: birth.id.clone(),
        });
        graph.edges.push(SimplifiedAncestryEdge {
            chromosome_id: chromosome("chr-a"),
            locus_id: locus("b"),
            source_copy_id: birth.source_b.clone(),
            child_copy_id: birth.id.clone(),
        });
    }

    graph.edges.sort();
    graph.retention = AncestryRetentionSet::new(focal, vec![copy("root-protected")]).unwrap();
    graph.validate_current(schema, map).unwrap();
    graph
}

fn generation_step(generation: u64, perturb_mosaic: bool) -> (Vec<Birth>, Vec<AncestryCopyId>) {
    match generation {
        1..=20 => {
            let prev_a = if generation == 1 {
                copy("root-a")
            } else {
                copy(format!("a-{}", generation - 1))
            };
            let prev_b = if generation == 1 {
                copy("root-b")
            } else {
                copy(format!("b-{}", generation - 1))
            };
            let prev_t = if generation == 1 {
                copy("root-dead")
            } else {
                copy(format!("t-{}", generation - 1))
            };
            let a = copy(format!("a-{generation}"));
            let b = copy(format!("b-{generation}"));
            let t = copy(format!("t-{generation}"));
            (
                vec![
                    Birth::unary(a.clone(), prev_a),
                    Birth::unary(b.clone(), prev_b),
                    Birth::unary(t.clone(), prev_t),
                ],
                vec![a, b, t],
            )
        }
        21 => {
            let mosaic = copy("m-21");
            let temp = copy("t-21");
            let source_b = if perturb_mosaic {
                copy("a-20")
            } else {
                copy("b-20")
            };
            (
                vec![
                    Birth::mosaic(mosaic.clone(), copy("a-20"), source_b),
                    Birth::unary(temp.clone(), copy("t-20")),
                ],
                vec![mosaic, temp],
            )
        }
        22..=29 => {
            let mosaic = copy(format!("m-{generation}"));
            let temp = copy(format!("t-{generation}"));
            (
                vec![
                    Birth::unary(mosaic.clone(), copy(format!("m-{}", generation - 1))),
                    Birth::unary(temp.clone(), copy(format!("t-{}", generation - 1))),
                ],
                vec![mosaic, temp],
            )
        }
        30..=50 => {
            let mosaic = copy(format!("m-{generation}"));
            (
                vec![Birth::unary(
                    mosaic.clone(),
                    copy(format!("m-{}", generation - 1)),
                )],
                vec![mosaic],
            )
        }
        51 => {
            let left = copy("left-51");
            let right = copy("right-51");
            (
                vec![
                    Birth::unary(left.clone(), copy("m-50")),
                    Birth::unary(right.clone(), copy("m-50")),
                ],
                vec![left, right],
            )
        }
        52..=130 => {
            let left = copy(format!("left-{generation}"));
            let right = copy(format!("right-{generation}"));
            (
                vec![
                    Birth::unary(
                        left.clone(),
                        copy(format!("left-{}", generation - 1)),
                    ),
                    Birth::unary(
                        right.clone(),
                        copy(format!("right-{}", generation - 1)),
                    ),
                ],
                vec![left, right],
            )
        }
        _ => panic!("generation outside V1 history"),
    }
}

#[derive(Debug)]
struct RunMetrics {
    graph: SimplifiedAncestryGraph,
    digest: SimplifiedAncestryGraphDigest,
    peak_nodes: usize,
    peak_edges: usize,
    appends: usize,
    simplifications: usize,
}

fn run_schedule(interval: Option<u64>, perturb_mosaic: bool) -> RunMetrics {
    let schema = schema();
    let map = map(&schema);
    let mut graph = initial_graph(&schema, &map);
    let mut peak_nodes = graph.nodes.len();
    let mut peak_edges = graph.edges.len();
    let mut appends = 0_usize;
    let mut simplifications = 0_usize;

    // Deliberately nonuniform chunk boundaries. They are unrelated to most
    // simplification schedules and therefore cannot become a hidden clock.
    for (start, end) in [(1_u64, 17_u64), (18, 43), (44, 78), (79, 130)] {
        for generation in start..=end {
            let (births, focal) = generation_step(generation, perturb_mosaic);
            graph = synthetic_append_generation(
                &graph,
                &schema,
                &map,
                generation,
                &births,
                focal,
            );
            appends += births.len();
            peak_nodes = peak_nodes.max(graph.nodes.len());
            peak_edges = peak_edges.max(graph.edges.len());

            if generation == 47 {
                let encoded = serde_json::to_string(&graph).unwrap();
                graph = serde_json::from_str(&encoded).unwrap();
                graph.validate_current(&schema, &map).unwrap();
            }

            if interval.is_some_and(|step| generation % step == 0) {
                let retention = graph.retention.clone();
                graph = resimplify_simplified_ancestry(&graph, &retention, &schema, &map)
                    .unwrap()
                    .graph;
                simplifications += 1;
            }
        }
    }

    let final_retention = AncestryRetentionSet::new(
        vec![copy("left-130"), copy("right-130")],
        vec![copy("root-protected")],
    )
    .unwrap();
    graph = resimplify_simplified_ancestry(&graph, &final_retention, &schema, &map)
        .unwrap()
        .graph;
    simplifications += 1;
    let digest = graph.canonical_digest(&schema, &map).unwrap();

    RunMetrics {
        graph,
        digest,
        peak_nodes,
        peak_edges,
        appends,
        simplifications,
    }
}

#[test]
fn simplification_schedule_does_not_change_final_modeled_locus_ancestry() {
    let end_only = run_schedule(None, false);
    let every_1 = run_schedule(Some(1), false);
    let every_7 = run_schedule(Some(7), false);
    let every_31 = run_schedule(Some(31), false);
    let every_101 = run_schedule(Some(101), false);

    for lane in [&every_1, &every_7, &every_31, &every_101] {
        assert_eq!(lane.graph, end_only.graph);
        assert_eq!(lane.digest, end_only.digest);
        assert_eq!(lane.appends, end_only.appends);
        assert!(lane.simplifications > end_only.simplifications);
        assert!(lane.peak_nodes < end_only.peak_nodes);
        assert!(lane.peak_edges < end_only.peak_edges);
    }

    // Frequent compaction should substantially bound the deliberately long
    // unary portions of this history, not merely shave one or two records.
    assert!(every_1.peak_nodes * 4 < end_only.peak_nodes);
    assert!(every_1.peak_edges * 4 < end_only.peak_edges);
    assert!(every_7.peak_nodes * 2 < end_only.peak_nodes);
    assert!(every_7.peak_edges * 2 < end_only.peak_edges);

    let final_ids: Vec<_> = end_only.graph.nodes.keys().cloned().collect();
    assert!(final_ids.contains(&copy("root-a")));
    assert!(final_ids.contains(&copy("root-b")));
    assert!(final_ids.contains(&copy("root-protected")));
    assert!(final_ids.contains(&copy("m-50")));
    assert!(final_ids.contains(&copy("left-130")));
    assert!(final_ids.contains(&copy("right-130")));
    assert!(!final_ids.contains(&copy("root-dead")));
    assert!(!final_ids.contains(&copy("t-29")));
}

#[test]
fn biological_or_retention_changes_are_not_laundered_as_schedule_equivalence() {
    let canonical = run_schedule(Some(7), false);
    let perturbed = run_schedule(Some(7), true);
    assert_ne!(canonical.digest, perturbed.digest);
    assert_ne!(canonical.graph, perturbed.graph);

    let schema = schema();
    let map = map(&schema);
    let changed_focal = AncestryRetentionSet::new(
        vec![copy("left-130")],
        vec![copy("root-protected")],
    )
    .unwrap();
    let focal_result = resimplify_simplified_ancestry(
        &canonical.graph,
        &changed_focal,
        &schema,
        &map,
    )
    .unwrap();
    assert_ne!(
        focal_result.graph.canonical_digest(&schema, &map).unwrap(),
        canonical.digest
    );

    let no_anchor = AncestryRetentionSet::new(
        vec![copy("left-130"), copy("right-130")],
        vec![],
    )
    .unwrap();
    let no_anchor_result = resimplify_simplified_ancestry(
        &canonical.graph,
        &no_anchor,
        &schema,
        &map,
    )
    .unwrap();
    assert_ne!(
        no_anchor_result
            .graph
            .canonical_digest(&schema, &map)
            .unwrap(),
        canonical.digest
    );
    assert!(!no_anchor_result
        .graph
        .nodes
        .contains_key(&copy("root-protected")));
}
