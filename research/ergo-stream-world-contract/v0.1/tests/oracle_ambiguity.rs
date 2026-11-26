use ergo_stream_microcosm::oracle::{StructuralWitnessAmbiguityV0, exhaustive_isomorphism_oracle};
use ergo_stream_microcosm::transfer::{NodeV0, RelationOrientationV0, RelationV0, StructureV0};

fn edge(
    id: u32,
    kind: u16,
    left: u32,
    right: u32,
    orientation: RelationOrientationV0,
) -> RelationV0 {
    RelationV0 {
        id,
        kind,
        orientation,
        participants: vec![left, right],
    }
}

#[test]
fn counts_relation_kind_mapping_ambiguity_with_unique_node_map() {
    let source = StructureV0 {
        nodes: vec![NodeV0 { id: 1 }, NodeV0 { id: 2 }, NodeV0 { id: 3 }],
        relations: vec![
            edge(10, 1, 1, 2, RelationOrientationV0::Undirected),
            edge(20, 2, 1, 2, RelationOrientationV0::Undirected),
            edge(30, 3, 1, 3, RelationOrientationV0::Undirected),
        ],
    };
    let target = StructureV0 {
        nodes: vec![NodeV0 { id: 11 }, NodeV0 { id: 12 }, NodeV0 { id: 13 }],
        relations: vec![
            edge(110, 101, 11, 12, RelationOrientationV0::Undirected),
            edge(120, 102, 11, 12, RelationOrientationV0::Undirected),
            edge(130, 103, 11, 13, RelationOrientationV0::Undirected),
        ],
    };
    let report = exhaustive_isomorphism_oracle(&source, &target).unwrap();
    assert_eq!(report.valid_node_mapping_count, 1);
    assert_eq!(report.valid_witness_class_count, 2);
    assert_eq!(
        report.ambiguity_status,
        StructuralWitnessAmbiguityV0::Ambiguous
    );
}

#[test]
fn counts_preserve_reverse_transform_ambiguity() {
    let source = StructureV0 {
        nodes: vec![NodeV0 { id: 1 }, NodeV0 { id: 2 }, NodeV0 { id: 3 }],
        relations: vec![
            edge(10, 1, 1, 2, RelationOrientationV0::Directed),
            edge(11, 1, 2, 1, RelationOrientationV0::Directed),
            edge(20, 2, 1, 3, RelationOrientationV0::Directed),
        ],
    };
    let target = StructureV0 {
        nodes: vec![NodeV0 { id: 11 }, NodeV0 { id: 12 }, NodeV0 { id: 13 }],
        relations: vec![
            edge(110, 10, 11, 12, RelationOrientationV0::Directed),
            edge(111, 10, 12, 11, RelationOrientationV0::Directed),
            edge(120, 20, 11, 13, RelationOrientationV0::Directed),
        ],
    };
    let report = exhaustive_isomorphism_oracle(&source, &target).unwrap();
    assert_eq!(report.valid_node_mapping_count, 1);
    assert_eq!(report.valid_witness_class_count, 2);
    assert_eq!(
        report.ambiguity_status,
        StructuralWitnessAmbiguityV0::Ambiguous
    );
}

#[test]
fn duplicate_relation_instance_id_permutations_do_not_inflate_semantic_count() {
    let source = StructureV0 {
        nodes: vec![NodeV0 { id: 1 }, NodeV0 { id: 2 }, NodeV0 { id: 3 }],
        relations: vec![
            edge(10, 1, 1, 2, RelationOrientationV0::Undirected),
            edge(20, 1, 1, 2, RelationOrientationV0::Undirected),
            edge(30, 2, 1, 3, RelationOrientationV0::Undirected),
        ],
    };
    let target = StructureV0 {
        nodes: vec![NodeV0 { id: 11 }, NodeV0 { id: 12 }, NodeV0 { id: 13 }],
        relations: vec![
            edge(110, 101, 11, 12, RelationOrientationV0::Undirected),
            edge(120, 101, 11, 12, RelationOrientationV0::Undirected),
            edge(130, 102, 11, 13, RelationOrientationV0::Undirected),
        ],
    };
    let report = exhaustive_isomorphism_oracle(&source, &target).unwrap();
    assert_eq!(report.valid_node_mapping_count, 1);
    assert_eq!(report.valid_witness_class_count, 1);
    assert_eq!(
        report.ambiguity_status,
        StructuralWitnessAmbiguityV0::UniqueUnderDeclaredEquivalence
    );
}
