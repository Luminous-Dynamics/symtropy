use ergo_stream_microcosm::oracle::{
    exhaustive_isomorphism_oracle, StructuralWitnessAmbiguityV0,
    MAX_ORACLE_WITNESS_CLASSES_V0,
};
use ergo_stream_microcosm::transfer::{
    NodeV0, RelationOrientationV0, RelationV0, StructureV0,
};

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
fn mixed_orientation_within_one_relation_kind_has_no_v0_transform() {
    let structure = StructureV0 {
        nodes: vec![NodeV0 { id: 1 }, NodeV0 { id: 2 }, NodeV0 { id: 3 }],
        relations: vec![
            edge(10, 1, 1, 2, RelationOrientationV0::Directed),
            edge(11, 1, 2, 3, RelationOrientationV0::Undirected),
        ],
    };
    let report = exhaustive_isomorphism_oracle(&structure, &structure).unwrap();
    assert!(!report.isomorphic);
    assert_eq!(report.valid_witness_class_count, 0);
    assert_eq!(
        report.ambiguity_status,
        StructuralWitnessAmbiguityV0::NoCorrespondence
    );
}

#[test]
fn witness_count_budget_returns_bounded_unknown_not_false_uniqueness() {
    let nodes = (0..8).map(|id| NodeV0 { id }).collect::<Vec<_>>();
    let participants = (0..8).collect::<Vec<_>>();
    let relations = (0..8)
        .map(|kind| RelationV0 {
            id: kind,
            kind: u16::try_from(kind).unwrap_or(u16::MAX),
            orientation: RelationOrientationV0::Undirected,
            participants: participants.clone(),
        })
        .collect::<Vec<_>>();
    let structure = StructureV0 { nodes, relations };
    let report = exhaustive_isomorphism_oracle(&structure, &structure).unwrap();
    assert!(report.isomorphic);
    assert!(!report.witness_count_complete);
    assert_eq!(
        report.valid_witness_class_count,
        MAX_ORACLE_WITNESS_CLASSES_V0
    );
    assert_eq!(
        report.ambiguity_status,
        StructuralWitnessAmbiguityV0::BoundedUnknown
    );
}

#[test]
fn preserves_duplicate_hyperedge_participant_multiplicity() {
    let source = StructureV0 {
        nodes: vec![NodeV0 { id: 1 }, NodeV0 { id: 2 }],
        relations: vec![RelationV0 {
            id: 1,
            kind: 1,
            orientation: RelationOrientationV0::Directed,
            participants: vec![1, 1, 2],
        }],
    };
    let target = StructureV0 {
        nodes: vec![NodeV0 { id: 11 }, NodeV0 { id: 12 }],
        relations: vec![RelationV0 {
            id: 11,
            kind: 10,
            orientation: RelationOrientationV0::Directed,
            participants: vec![11, 11, 12],
        }],
    };
    let report = exhaustive_isomorphism_oracle(&source, &target).unwrap();
    assert!(report.isomorphic);
    assert_eq!(report.valid_witness_class_count, 1);
    assert_eq!(
        report.ambiguity_status,
        StructuralWitnessAmbiguityV0::UniqueUnderDeclaredEquivalence
    );
}
