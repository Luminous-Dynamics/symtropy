//! Scope-aware structural correspondence for ErgoStream v0.2.
//!
//! The v0.1 witness validates a total correspondence over two complete finite
//! structures. Real cross-domain transfer is often scoped: a source mechanism
//! may embed into a region of a larger target without implying whole-system
//! equivalence.
//!
//! This module supports three explicitly qualified finite map kinds:
//! - Isomorphism: complete structures, total one-to-one map.
//! - InducedEmbedding: selected source and target induced substructures map
//!   bijectively; every relation wholly inside each selected node set must be
//!   included in its declared relation scope.
//! - SubgraphEmbedding: a declared source subgraph maps bijectively to a
//!   declared target subgraph; additional target relations may be excluded,
//!   but must be listed outside the target scope.
//!
//! Homomorphism, quotient, simulation, bisimulation and approximate/continuous
//! maps remain separate future validators. They must not be treated as aliases
//! for these finite embedding checks.

use std::collections::BTreeSet;

use super::transfer::{
    StructuralCorrespondenceV0, StructuralTransferWitnessV0, StructureV0, TransferErrorV0,
    validate_structural_correspondence,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralMapKindV0 {
    Isomorphism,
    InducedEmbedding,
    SubgraphEmbedding,
}

/// A complete partition of one structure into a selected scope and its
/// explicitly excluded context. The selected relation IDs must be supported
/// entirely by selected nodes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StructureScopeV0 {
    pub node_ids: Vec<u32>,
    pub relation_ids: Vec<u32>,
    pub excluded_node_ids: Vec<u32>,
    pub excluded_relation_ids: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedStructuralCandidateV0 {
    pub map_kind: StructuralMapKindV0,
    pub source_scope: StructureScopeV0,
    pub target_scope: StructureScopeV0,
    pub correspondence: StructuralCorrespondenceV0,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedStructuralWitnessV0 {
    pub map_kind: StructuralMapKindV0,
    pub structural: StructuralTransferWitnessV0,
    pub excluded_source_nodes: usize,
    pub excluded_source_relations: usize,
    pub excluded_target_nodes: usize,
    pub excluded_target_relations: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopedTransferErrorV0 {
    DuplicateScopeId,
    ScopeOverlap,
    UnknownNodeId,
    UnknownRelationId,
    ScopePartitionIncomplete,
    RelationParticipantOutsideScope,
    IsomorphismExcludesEntities,
    SourceInducedScopeOmitsRelation,
    TargetInducedScopeOmitsRelation,
    StructuralWitnessRejected(TransferErrorV0),
}

impl std::fmt::Display for ScopedTransferErrorV0 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::DuplicateScopeId => "scope declarations contain duplicate IDs",
            Self::ScopeOverlap => "selected and excluded scopes overlap",
            Self::UnknownNodeId => "scope references an unknown node ID",
            Self::UnknownRelationId => "scope references an unknown relation ID",
            Self::ScopePartitionIncomplete => {
                "selected and excluded IDs must partition the complete structure"
            }
            Self::RelationParticipantOutsideScope => {
                "a selected relation references a node outside the selected scope"
            }
            Self::IsomorphismExcludesEntities => {
                "an isomorphism must cover complete source and target structures"
            }
            Self::SourceInducedScopeOmitsRelation => {
                "source induced scope omits a relation wholly inside its selected nodes"
            }
            Self::TargetInducedScopeOmitsRelation => {
                "target induced scope omits a relation wholly inside its selected nodes"
            }
            Self::StructuralWitnessRejected(error) => {
                return write!(formatter, "scoped structural witness rejected: {error}");
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ScopedTransferErrorV0 {}

/// Validate a declared structural map only within its explicit source and
/// target scopes. All entities outside those scopes must be accounted for and
/// are reported in the returned witness; they are never silently discarded.
///
/// The correspondence must be total and one-to-one over the selected
/// substructures. This function does not yet implement many-to-one
/// homomorphisms, quotients, state-transition simulation or approximate maps.
pub fn validate_scoped_structural_correspondence(
    source: &StructureV0,
    target: &StructureV0,
    candidate: &ScopedStructuralCandidateV0,
) -> Result<ScopedStructuralWitnessV0, ScopedTransferErrorV0> {
    validate_scope_partition(source, &candidate.source_scope)?;
    validate_scope_partition(target, &candidate.target_scope)?;

    validate_selected_relation_participants(source, &candidate.source_scope)?;
    validate_selected_relation_participants(target, &candidate.target_scope)?;

    match candidate.map_kind {
        StructuralMapKindV0::Isomorphism => {
            if !candidate.source_scope.excluded_node_ids.is_empty()
                || !candidate.source_scope.excluded_relation_ids.is_empty()
                || !candidate.target_scope.excluded_node_ids.is_empty()
                || !candidate.target_scope.excluded_relation_ids.is_empty()
            {
                return Err(ScopedTransferErrorV0::IsomorphismExcludesEntities);
            }
        }
        StructuralMapKindV0::InducedEmbedding => {
            validate_induced_scope(
                source,
                &candidate.source_scope,
                ScopedTransferErrorV0::SourceInducedScopeOmitsRelation,
            )?;
            validate_induced_scope(
                target,
                &candidate.target_scope,
                ScopedTransferErrorV0::TargetInducedScopeOmitsRelation,
            )?;
        }
        StructuralMapKindV0::SubgraphEmbedding => {}
    }

    let source_substructure = scoped_structure(source, &candidate.source_scope);
    let target_substructure = scoped_structure(target, &candidate.target_scope);

    let structural = validate_structural_correspondence(
        &source_substructure,
        &target_substructure,
        &candidate.correspondence,
    )
    .map_err(ScopedTransferErrorV0::StructuralWitnessRejected)?;

    Ok(ScopedStructuralWitnessV0 {
        map_kind: candidate.map_kind,
        structural,
        excluded_source_nodes: candidate.source_scope.excluded_node_ids.len(),
        excluded_source_relations: candidate.source_scope.excluded_relation_ids.len(),
        excluded_target_nodes: candidate.target_scope.excluded_node_ids.len(),
        excluded_target_relations: candidate.target_scope.excluded_relation_ids.len(),
    })
}

fn validate_scope_partition(
    structure: &StructureV0,
    scope: &StructureScopeV0,
) -> Result<(), ScopedTransferErrorV0> {
    if !ids_unique(&scope.node_ids)
        || !ids_unique(&scope.relation_ids)
        || !ids_unique(&scope.excluded_node_ids)
        || !ids_unique(&scope.excluded_relation_ids)
    {
        return Err(ScopedTransferErrorV0::DuplicateScopeId);
    }

    let selected_nodes = scope.node_ids.iter().copied().collect::<BTreeSet<_>>();
    let excluded_nodes = scope
        .excluded_node_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let selected_relations = scope.relation_ids.iter().copied().collect::<BTreeSet<_>>();
    let excluded_relations = scope
        .excluded_relation_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();

    if !selected_nodes.is_disjoint(&excluded_nodes)
        || !selected_relations.is_disjoint(&excluded_relations)
    {
        return Err(ScopedTransferErrorV0::ScopeOverlap);
    }

    let known_nodes = structure
        .nodes
        .iter()
        .map(|node| node.id)
        .collect::<BTreeSet<_>>();
    let known_relations = structure
        .relations
        .iter()
        .map(|relation| relation.id)
        .collect::<BTreeSet<_>>();

    if !selected_nodes.is_subset(&known_nodes) || !excluded_nodes.is_subset(&known_nodes) {
        return Err(ScopedTransferErrorV0::UnknownNodeId);
    }
    if !selected_relations.is_subset(&known_relations)
        || !excluded_relations.is_subset(&known_relations)
    {
        return Err(ScopedTransferErrorV0::UnknownRelationId);
    }

    if selected_nodes.union(&excluded_nodes).count() != structure.nodes.len()
        || selected_relations.union(&excluded_relations).count() != structure.relations.len()
    {
        return Err(ScopedTransferErrorV0::ScopePartitionIncomplete);
    }

    Ok(())
}

fn validate_selected_relation_participants(
    structure: &StructureV0,
    scope: &StructureScopeV0,
) -> Result<(), ScopedTransferErrorV0> {
    let selected_nodes = scope.node_ids.iter().copied().collect::<BTreeSet<_>>();
    for relation in structure
        .relations
        .iter()
        .filter(|relation| scope.relation_ids.contains(&relation.id))
    {
        if relation.participants.is_empty()
            || relation
                .participants
                .iter()
                .any(|participant| !selected_nodes.contains(participant))
        {
            return Err(ScopedTransferErrorV0::RelationParticipantOutsideScope);
        }
    }
    Ok(())
}

fn validate_induced_scope(
    structure: &StructureV0,
    scope: &StructureScopeV0,
    error: ScopedTransferErrorV0,
) -> Result<(), ScopedTransferErrorV0> {
    let selected_nodes = scope.node_ids.iter().copied().collect::<BTreeSet<_>>();
    for relation in &structure.relations {
        let fully_inside = !relation.participants.is_empty()
            && relation
                .participants
                .iter()
                .all(|participant| selected_nodes.contains(participant));
        if fully_inside && !scope.relation_ids.contains(&relation.id) {
            return Err(error);
        }
    }
    Ok(())
}

fn scoped_structure(structure: &StructureV0, scope: &StructureScopeV0) -> StructureV0 {
    StructureV0 {
        nodes: structure
            .nodes
            .iter()
            .filter(|node| scope.node_ids.contains(&node.id))
            .cloned()
            .collect(),
        relations: structure
            .relations
            .iter()
            .filter(|relation| scope.relation_ids.contains(&relation.id))
            .cloned()
            .collect(),
    }
}

fn ids_unique(ids: &[u32]) -> bool {
    ids.iter()
        .enumerate()
        .all(|(index, id)| !ids[index + 1..].contains(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transfer::{
        NodeCorrespondenceV0, NodeV0, ParticipantTransformV0, RelationCorrespondenceV0,
        RelationKindCorrespondenceV0, RelationOrientationV0, RelationV0,
    };

    fn source_and_target() -> (StructureV0, StructureV0) {
        let source = StructureV0 {
            nodes: vec![NodeV0 { id: 1 }, NodeV0 { id: 2 }, NodeV0 { id: 3 }],
            relations: vec![
                RelationV0 {
                    id: 10,
                    kind: 1,
                    orientation: RelationOrientationV0::Undirected,
                    participants: vec![1, 2],
                },
                RelationV0 {
                    id: 11,
                    kind: 1,
                    orientation: RelationOrientationV0::Undirected,
                    participants: vec![2, 3],
                },
            ],
        };
        let target = StructureV0 {
            nodes: vec![NodeV0 { id: 11 }, NodeV0 { id: 12 }, NodeV0 { id: 13 }],
            relations: vec![
                RelationV0 {
                    id: 110,
                    kind: 100,
                    orientation: RelationOrientationV0::Undirected,
                    participants: vec![11, 12],
                },
                RelationV0 {
                    id: 111,
                    kind: 100,
                    orientation: RelationOrientationV0::Undirected,
                    participants: vec![12, 13],
                },
            ],
        };
        (source, target)
    }

    fn scoped_candidate(kind: StructuralMapKindV0) -> ScopedStructuralCandidateV0 {
        ScopedStructuralCandidateV0 {
            map_kind: kind,
            source_scope: StructureScopeV0 {
                node_ids: vec![1, 2],
                relation_ids: vec![10],
                excluded_node_ids: vec![3],
                excluded_relation_ids: vec![11],
            },
            target_scope: StructureScopeV0 {
                node_ids: vec![11, 12],
                relation_ids: vec![110],
                excluded_node_ids: vec![13],
                excluded_relation_ids: vec![111],
            },
            correspondence: StructuralCorrespondenceV0 {
                nodes: vec![
                    NodeCorrespondenceV0 {
                        source_node_id: 1,
                        target_node_id: 11,
                    },
                    NodeCorrespondenceV0 {
                        source_node_id: 2,
                        target_node_id: 12,
                    },
                ],
                relation_kinds: vec![RelationKindCorrespondenceV0 {
                    source_kind: 1,
                    target_kind: 100,
                    participant_transform: ParticipantTransformV0::Unordered,
                }],
                relations: vec![RelationCorrespondenceV0 {
                    source_relation_id: 10,
                    target_relation_id: 110,
                }],
            },
        }
    }

    #[test]
    fn induced_embedding_transfers_an_isolated_selected_substructure() {
        let (source, target) = source_and_target();
        let candidate = scoped_candidate(StructuralMapKindV0::InducedEmbedding);

        let witness = validate_scoped_structural_correspondence(&source, &target, &candidate)
            .expect("selected induced substructures correspond");

        assert_eq!(witness.map_kind, StructuralMapKindV0::InducedEmbedding);
        assert_eq!(witness.structural.mapped_nodes, 2);
        assert_eq!(witness.structural.mapped_relations, 1);
        assert_eq!(witness.excluded_source_nodes, 1);
        assert_eq!(witness.excluded_target_nodes, 1);
        assert_eq!(witness.excluded_source_relations, 1);
        assert_eq!(witness.excluded_target_relations, 1);
    }

    #[test]
    fn isomorphism_cannot_hide_out_of_scope_entities() {
        let (source, target) = source_and_target();
        let candidate = scoped_candidate(StructuralMapKindV0::Isomorphism);

        assert_eq!(
            validate_scoped_structural_correspondence(&source, &target, &candidate),
            Err(ScopedTransferErrorV0::IsomorphismExcludesEntities)
        );
    }

    #[test]
    fn induced_embedding_rejects_an_omitted_internal_target_relation() {
        let (source, mut target) = source_and_target();
        target.relations.push(RelationV0 {
            id: 112,
            kind: 100,
            orientation: RelationOrientationV0::Undirected,
            participants: vec![11, 12],
        });
        let mut candidate = scoped_candidate(StructuralMapKindV0::InducedEmbedding);
        candidate.target_scope.excluded_relation_ids.push(112);

        assert_eq!(
            validate_scoped_structural_correspondence(&source, &target, &candidate),
            Err(ScopedTransferErrorV0::TargetInducedScopeOmitsRelation)
        );
    }

    #[test]
    fn subgraph_embedding_can_exclude_an_additional_target_relation_explicitly() {
        let (source, mut target) = source_and_target();
        target.relations.push(RelationV0 {
            id: 112,
            kind: 100,
            orientation: RelationOrientationV0::Undirected,
            participants: vec![11, 12],
        });
        let mut candidate = scoped_candidate(StructuralMapKindV0::SubgraphEmbedding);
        candidate.target_scope.excluded_relation_ids.push(112);

        let witness = validate_scoped_structural_correspondence(&source, &target, &candidate)
            .expect("declared target subgraph preserves the selected relation");
        assert_eq!(witness.map_kind, StructuralMapKindV0::SubgraphEmbedding);
        assert_eq!(witness.excluded_target_relations, 2);
    }

    #[test]
    fn scope_partition_must_account_for_every_entity() {
        let (source, target) = source_and_target();
        let mut candidate = scoped_candidate(StructuralMapKindV0::SubgraphEmbedding);
        candidate.source_scope.excluded_node_ids.clear();

        assert_eq!(
            validate_scoped_structural_correspondence(&source, &target, &candidate),
            Err(ScopedTransferErrorV0::ScopePartitionIncomplete)
        );
    }

    #[test]
    fn selected_relations_cannot_escape_selected_node_scope() {
        let (source, target) = source_and_target();
        let mut candidate = scoped_candidate(StructuralMapKindV0::SubgraphEmbedding);
        candidate.source_scope.node_ids.pop();

        assert_eq!(
            validate_scoped_structural_correspondence(&source, &target, &candidate),
            Err(ScopedTransferErrorV0::RelationParticipantOutsideScope)
        );
    }

    #[test]
    fn map_candidate_must_cover_exact_selected_substructures() {
        let (source, target) = source_and_target();
        let mut candidate = scoped_candidate(StructuralMapKindV0::SubgraphEmbedding);
        candidate.correspondence.nodes.pop();

        assert_eq!(
            validate_scoped_structural_correspondence(&source, &target, &candidate),
            Err(ScopedTransferErrorV0::StructuralWitnessRejected(
                TransferErrorV0::NodeMapIncomplete
            ))
        );
    }
}
