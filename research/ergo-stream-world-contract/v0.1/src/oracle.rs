//! Independent exhaustive finite graph-isomorphism oracle for ErgoStream.
//!
//! This module is intentionally separate from the candidate witness validator
//! and from the heuristic unique-incidence matcher. It enumerates node
//! bijections (with a safe degree-signature pruning step), then independently
//! checks relation-kind and relation-instance multisets. It does not call
//! \`validate_structural_correspondence\`.
//!
//! The oracle is an exact reference only for small, finite, whole-structure
//! isomorphism. It is not a subgraph matcher, graph-edit solver, causal oracle,
//! or scalable production algorithm.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::transfer::{
    NodeCorrespondenceV0, ParticipantTransformV0, RelationCorrespondenceV0,
    RelationKindCorrespondenceV0, RelationOrientationV0, StructuralCorrespondenceV0,
    StructureV0,
};

/// Hard limits keep factorial enumeration explicit and bounded.
pub const MAX_ORACLE_NODES_V0: usize = 8;
pub const MAX_ORACLE_RELATIONS_V0: usize = 64;
pub const MAX_ORACLE_RELATION_KINDS_V0: usize = 8;
/// Maximum number of semantic full-witness classes counted by the v0 oracle.
/// Exceeding this bound yields `BoundedUnknown`, never false uniqueness.
pub const MAX_ORACLE_WITNESS_CLASSES_V0: usize = 65_536;

type NodeSignatureV0 = Vec<(u8, usize, usize)>;
type RelationSignatureV0 = (u8, usize, Vec<u32>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsomorphismOracleErrorV0 {
    DuplicateNodeId,
    DuplicateRelationId,
    EmptyRelationParticipants,
    UnknownParticipant,
    NoRelations,
    TooManyNodes,
    TooManyRelations,
    TooManyRelationKinds,
}

impl fmt::Display for IsomorphismOracleErrorV0 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::DuplicateNodeId => "node IDs must be unique",
            Self::DuplicateRelationId => "relation IDs must be unique",
            Self::EmptyRelationParticipants => "relations must have at least one participant",
            Self::UnknownParticipant => "relation references an unknown node",
            Self::NoRelations => "both structures must contain at least one relation",
            Self::TooManyNodes => "node count exceeds the exhaustive oracle bound",
            Self::TooManyRelations => "relation count exceeds the exhaustive oracle bound",
            Self::TooManyRelationKinds => "relation-kind count exceeds the exhaustive oracle bound",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for IsomorphismOracleErrorV0 {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralWitnessAmbiguityV0 {
    NoCorrespondence,
    UniqueUnderDeclaredEquivalence,
    Ambiguous,
    BoundedUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsomorphismOracleReportV0 {
    pub isomorphic: bool,
    pub correspondence: Option<StructuralCorrespondenceV0>,
    /// Number of complete node bijections examined after signature pruning.
    pub explored_node_bijections: usize,
    /// Number of distinct total node bijections admitting at least one full witness.
    /// This is a lower bound when `witness_count_complete` is false.
    pub valid_node_mapping_count: usize,
    /// Full witness classes: a node map plus an injective relation-kind map and
    /// one valid participant transform per kind. Relation-instance ID permutations
    /// among duplicate incidence records are quotiented out.
    pub valid_witness_class_count: usize,
    /// False means enumeration stopped at the explicit witness-class budget.
    pub witness_count_complete: bool,
    pub ambiguity_status: StructuralWitnessAmbiguityV0,
}

/// Decide whole-structure isomorphism by exhaustive search.
///
/// Relation IDs, node IDs and relation-kind IDs are treated as domain-local
/// names. Directed relations permit one consistent preserve/reverse transform
/// per relation kind; undirected relations permit unordered incidence. A
/// relation kind containing mixed directed and undirected instances cannot
/// receive one valid transform under the v0 witness contract.
pub fn exhaustive_isomorphism_oracle(
    source: &StructureV0,
    target: &StructureV0,
) -> Result<IsomorphismOracleReportV0, IsomorphismOracleErrorV0> {
    validate_oracle_input(source)?;
    validate_oracle_input(target)?;

    if source.relations.is_empty() || target.relations.is_empty() {
        return Err(IsomorphismOracleErrorV0::NoRelations);
    }

    if source.nodes.len() > MAX_ORACLE_NODES_V0
        || target.nodes.len() > MAX_ORACLE_NODES_V0
    {
        return Err(IsomorphismOracleErrorV0::TooManyNodes);
    }
    if source.relations.len() > MAX_ORACLE_RELATIONS_V0
        || target.relations.len() > MAX_ORACLE_RELATIONS_V0
    {
        return Err(IsomorphismOracleErrorV0::TooManyRelations);
    }

    let source_kinds = relation_kinds(source);
    let target_kinds = relation_kinds(target);
    if source_kinds.len() > MAX_ORACLE_RELATION_KINDS_V0
        || target_kinds.len() > MAX_ORACLE_RELATION_KINDS_V0
    {
        return Err(IsomorphismOracleErrorV0::TooManyRelationKinds);
    }

    if source.nodes.len() != target.nodes.len()
        || source.relations.len() != target.relations.len()
        || source_kinds.len() != target_kinds.len()
    {
        return Ok(no_isomorphism(0));
    }

    let source_signatures = node_signatures(source);
    let target_signatures = node_signatures(target);
    let mut target_groups = BTreeMap::<NodeSignatureV0, Vec<u32>>::new();
    for (node_id, signature) in &target_signatures {
        target_groups
            .entry(signature.clone())
            .or_default()
            .push(*node_id);
    }

    let mut source_node_ids = source.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
    source_node_ids.sort_unstable();

    // A node's orientation/arity incidence multiset is invariant under every
    // permitted node, relation and relation-kind relabeling. It is safe to
    // prune mappings whose signatures differ, but not to accept a map from
    // signatures alone.
    if source_node_ids.iter().any(|node_id| {
        let signature = &source_signatures[node_id];
        !target_groups.contains_key(signature)
    }) {
        return Ok(no_isomorphism(0));
    }

    let mut report = IsomorphismOracleReportV0 {
        isomorphic: false,
        correspondence: None,
        explored_node_bijections: 0,
        valid_node_mapping_count: 0,
        valid_witness_class_count: 0,
        witness_count_complete: true,
        ambiguity_status: StructuralWitnessAmbiguityV0::NoCorrespondence,
    };
    let mut current_nodes = Vec::with_capacity(source.nodes.len());
    let mut used_targets = BTreeSet::new();

    let enumeration_complete = enumerate_node_bijections(
        0,
        &source_node_ids,
        &source_signatures,
        &target_groups,
        &mut current_nodes,
        &mut used_targets,
        source,
        target,
        &mut report,
    );

    report.witness_count_complete = enumeration_complete;
    report.isomorphic = report.valid_witness_class_count > 0;
    report.ambiguity_status = if !report.witness_count_complete {
        StructuralWitnessAmbiguityV0::BoundedUnknown
    } else if report.valid_witness_class_count == 0 {
        StructuralWitnessAmbiguityV0::NoCorrespondence
    } else if report.valid_witness_class_count == 1 {
        StructuralWitnessAmbiguityV0::UniqueUnderDeclaredEquivalence
    } else {
        StructuralWitnessAmbiguityV0::Ambiguous
    };
    Ok(report)
}

fn no_isomorphism(explored_node_bijections: usize) -> IsomorphismOracleReportV0 {
    IsomorphismOracleReportV0 {
        isomorphic: false,
        correspondence: None,
        explored_node_bijections,
        valid_node_mapping_count: 0,
        valid_witness_class_count: 0,
        witness_count_complete: true,
        ambiguity_status: StructuralWitnessAmbiguityV0::NoCorrespondence,
    }
}

fn validate_oracle_input(
    structure: &StructureV0,
) -> Result<(), IsomorphismOracleErrorV0> {
    let mut node_ids = BTreeSet::new();
    for node in &structure.nodes {
        if !node_ids.insert(node.id) {
            return Err(IsomorphismOracleErrorV0::DuplicateNodeId);
        }
    }

    let mut relation_ids = BTreeSet::new();
    for relation in &structure.relations {
        if !relation_ids.insert(relation.id) {
            return Err(IsomorphismOracleErrorV0::DuplicateRelationId);
        }
        if relation.participants.is_empty() {
            return Err(IsomorphismOracleErrorV0::EmptyRelationParticipants);
        }
        if relation
            .participants
            .iter()
            .any(|participant| !node_ids.contains(participant))
        {
            return Err(IsomorphismOracleErrorV0::UnknownParticipant);
        }
    }
    Ok(())
}

fn relation_kinds(structure: &StructureV0) -> Vec<u16> {
    let mut kinds = structure
        .relations
        .iter()
        .map(|relation| relation.kind)
        .collect::<Vec<_>>();
    kinds.sort_unstable();
    kinds.dedup();
    kinds
}

fn node_signatures(structure: &StructureV0) -> BTreeMap<u32, NodeSignatureV0> {
    let mut result = BTreeMap::new();
    for node in &structure.nodes {
        let mut counts = BTreeMap::<(u8, usize), usize>::new();
        for relation in &structure.relations {
            if relation.participants.contains(&node.id) {
                let orientation = match relation.orientation {
                    RelationOrientationV0::Directed => 0,
                    RelationOrientationV0::Undirected => 1,
                };
                *counts
                    .entry((orientation, relation.participants.len()))
                    .or_default() += 1;
            }
        }
        result.insert(
            node.id,
            counts
                .into_iter()
                .map(|((orientation, arity), count)| (orientation, arity, count))
                .collect(),
        );
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn enumerate_node_bijections(
    index: usize,
    source_node_ids: &[u32],
    source_signatures: &BTreeMap<u32, NodeSignatureV0>,
    target_groups: &BTreeMap<NodeSignatureV0, Vec<u32>>,
    current_nodes: &mut Vec<NodeCorrespondenceV0>,
    used_targets: &mut BTreeSet<u32>,
    source: &StructureV0,
    target: &StructureV0,
    report: &mut IsomorphismOracleReportV0,
) -> bool {
    if index == source_node_ids.len() {
        report.explored_node_bijections += 1;
        let node_map = current_nodes
            .iter()
            .map(|mapping| (mapping.source_node_id, mapping.target_node_id))
            .collect::<BTreeMap<_, _>>();
        let witnesses_before = report.valid_witness_class_count;
        let complete = enumerate_kind_mapping_classes(
            source,
            target,
            &node_map,
            current_nodes,
            report,
        );
        if report.valid_witness_class_count > witnesses_before {
            report.valid_node_mapping_count += 1;
        }
        return complete;
    }

    let source_id = source_node_ids[index];
    let signature = &source_signatures[&source_id];
    let Some(candidates) = target_groups.get(signature) else {
        return true;
    };

    for target_id in candidates {
        if !used_targets.insert(*target_id) {
            continue;
        }
        current_nodes.push(NodeCorrespondenceV0 {
            source_node_id: source_id,
            target_node_id: *target_id,
        });
        let complete = enumerate_node_bijections(
            index + 1,
            source_node_ids,
            source_signatures,
            target_groups,
            current_nodes,
            used_targets,
            source,
            target,
            report,
        );
        current_nodes.pop();
        used_targets.remove(target_id);
        if !complete {
            return false;
        }
    }
    true
}

fn enumerate_kind_mapping_classes(
    source: &StructureV0,
    target: &StructureV0,
    node_map: &BTreeMap<u32, u32>,
    node_correspondence: &[NodeCorrespondenceV0],
    report: &mut IsomorphismOracleReportV0,
) -> bool {
    let source_kinds = relation_kinds(source);
    let target_kinds = relation_kinds(target);
    let mut compatibility = Vec::with_capacity(source_kinds.len());

    for source_kind in &source_kinds {
        let mut candidates = Vec::new();
        for target_kind in &target_kinds {
            for transform in compatible_kind_transforms(
                source,
                target,
                *source_kind,
                *target_kind,
                node_map,
            ) {
                candidates.push((*target_kind, transform));
            }
        }
        compatibility.push(candidates);
    }

    choose_kind_mapping_classes(
        0,
        &source_kinds,
        &compatibility,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        source,
        target,
        node_map,
        node_correspondence,
        report,
    )
}
fn find_kind_and_relation_mapping(
    source: &StructureV0,
    target: &StructureV0,
    node_map: &BTreeMap<u32, u32>,
    node_correspondence: &[NodeCorrespondenceV0],
) -> Option<StructuralCorrespondenceV0> {
    let source_kinds = relation_kinds(source);
    let target_kinds = relation_kinds(target);
    let mut compatibility = Vec::with_capacity(source_kinds.len());

    for source_kind in &source_kinds {
        let mut candidates = Vec::new();
        for target_kind in &target_kinds {
            if let Some(transform) =
                compatible_kind_transform(source, target, *source_kind, *target_kind, node_map)
            {
                candidates.push((*target_kind, transform));
            }
        }
        compatibility.push(candidates);
    }

    choose_kind_mapping(
        0,
        &source_kinds,
        &compatibility,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        source,
        target,
        node_map,
        node_correspondence,
    )
}

fn compatible_kind_transform(
    source: &StructureV0,
    target: &StructureV0,
    source_kind: u16,
    target_kind: u16,
    node_map: &BTreeMap<u32, u32>,
) -> Option<ParticipantTransformV0> {
    let source_relations = source
        .relations
        .iter()
        .filter(|relation| relation.kind == source_kind)
        .collect::<Vec<_>>();
    let target_relations = target
        .relations
        .iter()
        .filter(|relation| relation.kind == target_kind)
        .collect::<Vec<_>>();
    if source_relations.len() != target_relations.len() || source_relations.is_empty() {
        return None;
    }

    let source_directed = source_relations
        .iter()
        .any(|relation| relation.orientation == RelationOrientationV0::Directed);
    let source_undirected = source_relations
        .iter()
        .any(|relation| relation.orientation == RelationOrientationV0::Undirected);
    let target_directed = target_relations
        .iter()
        .any(|relation| relation.orientation == RelationOrientationV0::Directed);
    let target_undirected = target_relations
        .iter()
        .any(|relation| relation.orientation == RelationOrientationV0::Undirected);

    let transforms: &[ParticipantTransformV0] = match (
        source_directed,
        source_undirected,
        target_directed,
        target_undirected,
    ) {
        (true, false, true, false) => &[
            ParticipantTransformV0::PreserveOrder,
            ParticipantTransformV0::ReverseOrder,
        ],
        (false, true, false, true) => &[ParticipantTransformV0::Unordered],
        _ => return None,
    };

    transforms.iter().copied().find(|transform| {
        let mut source_signatures = source_relations
            .iter()
            .map(|relation| {
                source_relation_signature(relation, node_map, *transform)
            })
            .collect::<Option<Vec<_>>>();
        let Some(source_signatures) = source_signatures.as_mut() else {
            return false;
        };
        let mut target_signatures = target_relations
            .iter()
            .map(|relation| target_relation_signature(relation, *transform))
            .collect::<Vec<_>>();
        source_signatures.sort();
        target_signatures.sort();
        *source_signatures == target_signatures
    })
}

fn source_relation_signature(
    relation: &super::transfer::RelationV0,
    node_map: &BTreeMap<u32, u32>,
    transform: ParticipantTransformV0,
) -> Option<RelationSignatureV0> {
    let mut participants = relation
        .participants
        .iter()
        .map(|source_id| node_map.get(source_id).copied())
        .collect::<Option<Vec<_>>>()?;
    match transform {
        ParticipantTransformV0::PreserveOrder => {
            if relation.orientation != RelationOrientationV0::Directed {
                return None;
            }
        }
        ParticipantTransformV0::ReverseOrder => {
            if relation.orientation != RelationOrientationV0::Directed {
                return None;
            }
            participants.reverse();
        }
        ParticipantTransformV0::Unordered => {
            if relation.orientation != RelationOrientationV0::Undirected {
                return None;
            }
            participants.sort_unstable();
        }
    }
    Some((
        orientation_code(relation.orientation),
        participants.len(),
        participants,
    ))
}

fn target_relation_signature(
    relation: &super::transfer::RelationV0,
    transform: ParticipantTransformV0,
) -> RelationSignatureV0 {
    let mut participants = relation.participants.clone();
    if transform == ParticipantTransformV0::Unordered {
        participants.sort_unstable();
    }
    (
        orientation_code(relation.orientation),
        participants.len(),
        participants,
    )
}

fn orientation_code(orientation: RelationOrientationV0) -> u8 {
    match orientation {
        RelationOrientationV0::Directed => 0,
        RelationOrientationV0::Undirected => 1,
    }
}

#[allow(clippy::too_many_arguments)]
fn choose_kind_mapping(
    index: usize,
    source_kinds: &[u16],
    compatibility: &[Vec<(u16, ParticipantTransformV0)>],
    chosen: &mut Vec<RelationKindCorrespondenceV0>,
    used_target_kinds: &mut BTreeSet<u16>,
    source: &StructureV0,
    target: &StructureV0,
    node_map: &BTreeMap<u32, u32>,
    node_correspondence: &[NodeCorrespondenceV0],
) -> Option<StructuralCorrespondenceV0> {
    if index == source_kinds.len() {
        return build_correspondence(
            source,
            target,
            node_map,
            node_correspondence,
            chosen,
        );
    }

    for (target_kind, transform) in &compatibility[index] {
        if !used_target_kinds.insert(*target_kind) {
            continue;
        }
        chosen.push(RelationKindCorrespondenceV0 {
            source_kind: source_kinds[index],
            target_kind: *target_kind,
            participant_transform: *transform,
        });
        let found = choose_kind_mapping(
            index + 1,
            source_kinds,
            compatibility,
            chosen,
            used_target_kinds,
            source,
            target,
            node_map,
            node_correspondence,
        );
        chosen.pop();
        used_target_kinds.remove(target_kind);
        if found.is_some() {
            return found;
        }
    }
    None
}

fn build_correspondence(
    source: &StructureV0,
    target: &StructureV0,
    node_map: &BTreeMap<u32, u32>,
    node_correspondence: &[NodeCorrespondenceV0],
    kind_correspondence: &[RelationKindCorrespondenceV0],
) -> Option<StructuralCorrespondenceV0> {
    let mut target_relations = target.relations.iter().collect::<Vec<_>>();
    target_relations.sort_by_key(|relation| relation.id);
    let mut used_target_relations = BTreeSet::new();
    let mut relation_correspondence = Vec::with_capacity(source.relations.len());

    let mut source_relations = source.relations.iter().collect::<Vec<_>>();
    source_relations.sort_by_key(|relation| relation.id);

    for source_relation in source_relations {
        let kind_mapping = kind_correspondence
            .iter()
            .find(|mapping| mapping.source_kind == source_relation.kind)?;
        let signature = source_relation_signature(
            source_relation,
            node_map,
            kind_mapping.participant_transform,
        )?;

        let target_relation = target_relations.iter().find(|candidate| {
            !used_target_relations.contains(&candidate.id)
                && candidate.kind == kind_mapping.target_kind
                && target_relation_signature(candidate, kind_mapping.participant_transform)
                    == signature
        })?;

        used_target_relations.insert(target_relation.id);
        relation_correspondence.push(RelationCorrespondenceV0 {
            source_relation_id: source_relation.id,
            target_relation_id: target_relation.id,
        });
    }

    if used_target_relations.len() != target.relations.len() {
        return None;
    }

    Some(StructuralCorrespondenceV0 {
        nodes: node_correspondence.to_vec(),
        relation_kinds: kind_correspondence.to_vec(),
        relations: relation_correspondence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::{TransferAtlasFamilyV0, TransferAtlasWorldV0};
    use crate::transfer::{RelationV0, validate_structural_correspondence};

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
    fn independently_finds_relabelled_and_reversed_structure() {
        let source = StructureV0 {
            nodes: vec![NodeV0 { id: 1 }, NodeV0 { id: 2 }, NodeV0 { id: 3 }],
            relations: vec![
                edge(10, 1, 1, 2, RelationOrientationV0::Directed),
                edge(20, 2, 2, 3, RelationOrientationV0::Undirected),
            ],
        };
        let target = StructureV0 {
            nodes: vec![NodeV0 { id: 11 }, NodeV0 { id: 12 }, NodeV0 { id: 13 }],
            relations: vec![
                edge(110, 101, 12, 11, RelationOrientationV0::Directed),
                edge(120, 102, 13, 12, RelationOrientationV0::Undirected),
            ],
        };

        let report = exhaustive_isomorphism_oracle(&source, &target).unwrap();
        assert!(report.isomorphic);
        assert_eq!(report.explored_node_bijections, 1);
        assert_eq!(report.valid_node_mapping_count, 1);
        let correspondence = report.correspondence.unwrap();
        let witness =
            validate_structural_correspondence(&source, &target, &correspondence).unwrap();
        assert_eq!(witness.mapped_nodes, 3);
        assert_eq!(witness.mapped_relations, 2);
        assert!(correspondence
            .relation_kinds
            .iter()
            .any(|mapping| mapping.participant_transform == ParticipantTransformV0::ReverseOrder));
    }

    fn k33() -> StructureV0 {
        let nodes = (0..6).map(|id| NodeV0 { id }).collect::<Vec<_>>();
        let mut relations = Vec::new();
        let mut id = 0;
        for left in 0..3 {
            for right in 3..6 {
                relations.push(edge(
                    id,
                    1,
                    left,
                    right,
                    RelationOrientationV0::Undirected,
                ));
                id += 1;
            }
        }
        StructureV0 { nodes, relations }
    }

    fn triangular_prism() -> StructureV0 {
        let nodes = (0..6).map(|id| NodeV0 { id }).collect::<Vec<_>>();
        let edges = [
            (0, 1),
            (1, 2),
            (2, 0),
            (3, 4),
            (4, 5),
            (5, 3),
            (0, 3),
            (1, 4),
            (2, 5),
        ];
        let relations = edges
            .iter()
            .enumerate()
            .map(|(id, (left, right))| {
                edge(
                    id as u32,
                    1,
                    *left,
                    *right,
                    RelationOrientationV0::Undirected,
                )
            })
            .collect();
        StructureV0 { nodes, relations }
    }

    #[test]
    fn rejects_degree_matched_non_isomorphic_k33_and_prism() {
        let report = exhaustive_isomorphism_oracle(&k33(), &triangular_prism()).unwrap();
        assert!(!report.isomorphic);
        assert!(report.correspondence.is_none());
        assert_eq!(report.explored_node_bijections, 720);
        assert_eq!(report.valid_node_mapping_count, 0);
    }

    fn cycle_six() -> StructureV0 {
        let nodes = (0..6).map(|id| NodeV0 { id }).collect::<Vec<_>>();
        let relations = (0..6)
            .map(|left| {
                edge(
                    left,
                    1,
                    left,
                    (left + 1) % 6,
                    RelationOrientationV0::Undirected,
                )
            })
            .collect();
        StructureV0 { nodes, relations }
    }

    #[test]
    fn counts_symmetric_node_mappings_instead_of_forcing_uniqueness() {
        let cycle = cycle_six();
        let report = exhaustive_isomorphism_oracle(&cycle, &cycle).unwrap();
        assert!(report.isomorphic);
        assert_eq!(report.explored_node_bijections, 720);
        // The automorphism group of an undirected six-cycle is the dihedral
        // group D6: six rotations and six reflections.
        assert_eq!(report.valid_node_mapping_count, 12);
        assert!(report.correspondence.is_some());
    }

    #[test]
    fn rejects_duplicate_ids_and_unknown_participants_independently() {
        let duplicate_nodes = StructureV0 {
            nodes: vec![NodeV0 { id: 1 }, NodeV0 { id: 1 }],
            relations: vec![edge(1, 1, 1, 1, RelationOrientationV0::Undirected)],
        };
        assert_eq!(
            exhaustive_isomorphism_oracle(&duplicate_nodes, &duplicate_nodes),
            Err(IsomorphismOracleErrorV0::DuplicateNodeId)
        );

        let unknown_participant = StructureV0 {
            nodes: vec![NodeV0 { id: 1 }],
            relations: vec![edge(1, 1, 1, 2, RelationOrientationV0::Undirected)],
        };
        assert_eq!(
            exhaustive_isomorphism_oracle(&unknown_participant, &unknown_participant),
            Err(IsomorphismOracleErrorV0::UnknownParticipant)
        );
    }

    #[test]
    fn rejects_unsupported_search_size_before_enumeration() {
        let structure = StructureV0 {
            nodes: (0..9).map(|id| NodeV0 { id }).collect(),
            relations: vec![edge(1, 1, 0, 1, RelationOrientationV0::Undirected)],
        };
        assert_eq!(
            exhaustive_isomorphism_oracle(&structure, &structure),
            Err(IsomorphismOracleErrorV0::TooManyNodes)
        );
    }


    #[test]
    fn independently_checks_transfer_atlas_structural_families() {
        let cases = [
            (TransferAtlasFamilyV0::RelabelledReverse, true),
            (TransferAtlasFamilyV0::DegreeMatchedNonIsomorphic, false),
            (
                TransferAtlasFamilyV0::IsomorphicObservationDifferentMechanism,
                true,
            ),
            (
                TransferAtlasFamilyV0::StructurallySimilarInterventionDivergent,
                true,
            ),
        ];

        for (family, expected_structural_isomorphism) in cases {
            let world = TransferAtlasWorldV0::generate(family);
            let view = world.policy_view();
            let report =
                exhaustive_isomorphism_oracle(&view.source, &view.target).unwrap();
            assert_eq!(
                report.isomorphic,
                expected_structural_isomorphism,
                "independent structural oracle disagreed for {family:?}"
            );
        }
    }

    const FIVE_NODE_EDGE_COUNT: usize = 10;

    fn permute_five(
        values: &mut [u8; 5],
        index: usize,
        output: &mut Vec<[u8; 5]>,
    ) {
        if index == values.len() {
            output.push(*values);
            return;
        }
        for swap_index in index..values.len() {
            values.swap(index, swap_index);
            permute_five(values, index + 1, output);
            values.swap(index, swap_index);
        }
    }

    fn all_five_node_permutations() -> Vec<[u8; 5]> {
        let mut values = [0, 1, 2, 3, 4];
        let mut output = Vec::with_capacity(120);
        permute_five(&mut values, 0, &mut output);
        output
    }

    fn edge_bit_index(left: usize, right: usize) -> usize {
        debug_assert!(left < right);
        left * (9 - left) / 2 + (right - left - 1)
    }

    /// Independent graph canonicalization by trying every vertex permutation.
    /// This does not call the exhaustive witness oracle or candidate validator.
    fn canonical_five_node_graph(mask: u16, permutations: &[[u8; 5]]) -> u16 {
        let mut best = u16::MAX;
        for permutation in permutations {
            let mut relabelled = 0_u16;
            let mut source_bit = 0_usize;
            for left in 0..5 {
                for right in (left + 1)..5 {
                    if mask & (1_u16 << source_bit) != 0 {
                        let mapped_left = usize::from(permutation[left]);
                        let mapped_right = usize::from(permutation[right]);
                        let (first, second) = if mapped_left < mapped_right {
                            (mapped_left, mapped_right)
                        } else {
                            (mapped_right, mapped_left)
                        };
                        let target_bit = edge_bit_index(first, second);
                        relabelled |= 1_u16 << target_bit;
                    }
                    source_bit += 1;
                }
            }
            best = best.min(relabelled);
        }
        best
    }

    fn structure_from_five_node_mask(mask: u16) -> StructureV0 {
        let nodes = (0..5).map(|id| NodeV0 { id }).collect::<Vec<_>>();
        let mut relations = Vec::new();
        let mut bit = 0_usize;
        let mut relation_id = 0_u32;
        for left in 0..5 {
            for right in (left + 1)..5 {
                if mask & (1_u16 << bit) != 0 {
                    relations.push(edge(
                        relation_id,
                        1,
                        left as u32,
                        right as u32,
                        RelationOrientationV0::Undirected,
                    ));
                    relation_id += 1;
                }
                bit += 1;
            }
        }
        StructureV0 { nodes, relations }
    }

    #[test]
    fn differentially_checks_every_nonempty_five_node_simple_graph_class() {
        let permutations = all_five_node_permutations();
        assert_eq!(permutations.len(), 120);

        // There are 34 unlabeled simple graphs on five vertices. The existing
        // v0 transfer contract rejects relation-free structures, so exclude
        // the empty graph and check the remaining 33 nonempty classes.
        let mut representatives = BTreeMap::<u16, StructureV0>::new();
        for mask in 1_u16..(1_u16 << FIVE_NODE_EDGE_COUNT) {
            let canonical = canonical_five_node_graph(mask, &permutations);
            representatives
                .entry(canonical)
                .or_insert_with(|| structure_from_five_node_mask(mask));
        }
        assert_eq!(representatives.len(), 33);

        let classes = representatives.into_iter().collect::<Vec<_>>();
        let mut comparisons = 0_usize;
        for (source_code, source) in &classes {
            for (target_code, target) in &classes {
                let report = exhaustive_isomorphism_oracle(source, target).unwrap();
                assert_eq!(
                    report.isomorphic,
                    source_code == target_code,
                    "oracle mismatch: {source_code:#05x} vs {target_code:#05x}"
                );
                comparisons += 1;
            }
        }
        assert_eq!(comparisons, 33 * 33);
    }

    #[test]
    fn returns_non_isomorphic_for_cardinality_mismatch_without_search() {
        let source = k33();
        let target = StructureV0 {
            nodes: vec![NodeV0 { id: 1 }],
            relations: vec![edge(1, 1, 1, 1, RelationOrientationV0::Undirected)],
        };
        let report = exhaustive_isomorphism_oracle(&source, &target).unwrap();
        assert!(!report.isomorphic);
        assert_eq!(report.explored_node_bijections, 0);
        assert_eq!(report.valid_node_mapping_count, 0);
    }
}
