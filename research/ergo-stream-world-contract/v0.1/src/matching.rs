//! Conservative degree-signature graph correspondence baseline.
//!
//! This is a deliberately limited baseline for small finite structures. It
//! maps nodes only when their relation-incidence signatures are unique, then
//! matches relation instances under preserve/reverse/unordered incidence.
//! Ambiguous signatures cause abstention instead of arbitrary tie-breaking.
//!
//! This is not a complete graph-isomorphism solver. It is a transparent
//! baseline for paired-world transfer experiments.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::transfer::{
    NodeCorrespondenceV0, ParticipantTransformV0, RelationCorrespondenceV0,
    RelationKindCorrespondenceV0, RelationOrientationV0, StructuralCorrespondenceV0,
    StructuralTransferWitnessV0, StructureV0, TransferErrorV0, validate_structural_correspondence,
};

type NodeSignatureV0 = Vec<(u8, usize, usize)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphMatchResultV0 {
    pub correspondence: StructuralCorrespondenceV0,
    pub witness: StructuralTransferWitnessV0,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphMatchErrorV0 {
    NoMatchingNodeSignature,
    AmbiguousNodeSignature,
    NoMatchingRelation,
    AmbiguousRelation,
    InconsistentRelationKindTransform,
    NonInjectiveRelationKindMap,
    InvalidCandidate(TransferErrorV0),
}

impl fmt::Display for GraphMatchErrorV0 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NoMatchingNodeSignature => "source and target node signatures do not match",
            Self::AmbiguousNodeSignature => "node signatures are not unique; baseline abstains",
            Self::NoMatchingRelation => "no target relation matches a mapped source relation",
            Self::AmbiguousRelation => "multiple target relations match a source relation",
            Self::InconsistentRelationKindTransform => {
                "one source relation kind requires inconsistent target kinds or transforms"
            }
            Self::NonInjectiveRelationKindMap => {
                "distinct source relation kinds map to the same target relation kind"
            }
            Self::InvalidCandidate(error) => {
                return write!(
                    formatter,
                    "candidate correspondence failed validation: {error}"
                );
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for GraphMatchErrorV0 {}

/// Conservative graph-correspondence baseline.
///
/// The matcher uses only the two supplied finite structures. It does not see
/// generator seeds, hidden evaluator labels, downstream probes or intervention
/// outcomes. If a node signature is shared by multiple nodes, it returns an
/// ambiguity error rather than selecting an arbitrary permutation.
#[derive(Debug, Clone, Copy, Default)]
pub struct UniqueIncidenceSignatureBaselineV0;

impl UniqueIncidenceSignatureBaselineV0 {
    pub const NAME: &'static str = "unique-incidence-signature-v0";

    pub fn match_structures(
        &self,
        source: &StructureV0,
        target: &StructureV0,
    ) -> Result<GraphMatchResultV0, GraphMatchErrorV0> {
        let source_signatures = grouped_signatures(source);
        let target_signatures = grouped_signatures(target);

        if source_signatures.len() != target_signatures.len() {
            return Err(GraphMatchErrorV0::NoMatchingNodeSignature);
        }

        let mut node_correspondence = Vec::with_capacity(source.nodes.len());
        let mut node_map = BTreeMap::new();

        for (signature, source_ids) in &source_signatures {
            let target_ids = target_signatures
                .get(signature)
                .ok_or(GraphMatchErrorV0::NoMatchingNodeSignature)?;

            if source_ids.len() != 1 || target_ids.len() != 1 {
                return Err(GraphMatchErrorV0::AmbiguousNodeSignature);
            }

            let source_node_id = source_ids[0];
            let target_node_id = target_ids[0];
            node_correspondence.push(NodeCorrespondenceV0 {
                source_node_id,
                target_node_id,
            });
            node_map.insert(source_node_id, target_node_id);
        }

        if node_correspondence.len() != source.nodes.len()
            || node_correspondence.len() != target.nodes.len()
        {
            return Err(GraphMatchErrorV0::NoMatchingNodeSignature);
        }

        let mut used_target_relations = BTreeSet::new();
        let mut relation_correspondence = Vec::with_capacity(source.relations.len());
        let mut kind_map = BTreeMap::<u16, (u16, ParticipantTransformV0)>::new();
        let mut reverse_kind_map = BTreeMap::<u16, u16>::new();

        for source_relation in &source.relations {
            let mapped_participants = source_relation
                .participants
                .iter()
                .map(|source_id| {
                    node_map
                        .get(source_id)
                        .copied()
                        .ok_or(GraphMatchErrorV0::NoMatchingNodeSignature)
                })
                .collect::<Result<Vec<_>, _>>()?;

            let mut candidates = target.relations.iter().filter_map(|target_relation| {
                if used_target_relations.contains(&target_relation.id)
                    || target_relation.orientation != source_relation.orientation
                    || target_relation.participants.len() != mapped_participants.len()
                {
                    return None;
                }

                let transform = participant_transform(
                    source_relation.orientation,
                    &mapped_participants,
                    &target_relation.participants,
                )?;

                Some((target_relation, transform))
            });

            let (target_relation, transform) = candidates
                .next()
                .ok_or(GraphMatchErrorV0::NoMatchingRelation)?;

            if candidates.next().is_some() {
                return Err(GraphMatchErrorV0::AmbiguousRelation);
            }

            used_target_relations.insert(target_relation.id);
            relation_correspondence.push(RelationCorrespondenceV0 {
                source_relation_id: source_relation.id,
                target_relation_id: target_relation.id,
            });

            match kind_map.get(&source_relation.kind) {
                Some((existing_target_kind, existing_transform))
                    if *existing_target_kind != target_relation.kind
                        || *existing_transform != transform =>
                {
                    return Err(GraphMatchErrorV0::InconsistentRelationKindTransform);
                }
                Some(_) => {}
                None => {
                    if reverse_kind_map
                        .get(&target_relation.kind)
                        .is_some_and(|source_kind| *source_kind != source_relation.kind)
                    {
                        return Err(GraphMatchErrorV0::NonInjectiveRelationKindMap);
                    }
                    kind_map.insert(source_relation.kind, (target_relation.kind, transform));
                    reverse_kind_map.insert(target_relation.kind, source_relation.kind);
                }
            }
        }

        if used_target_relations.len() != target.relations.len() {
            return Err(GraphMatchErrorV0::NoMatchingRelation);
        }

        let relation_kinds = kind_map
            .into_iter()
            .map(|(source_kind, (target_kind, participant_transform))| {
                RelationKindCorrespondenceV0 {
                    source_kind,
                    target_kind,
                    participant_transform,
                }
            })
            .collect();

        let correspondence = StructuralCorrespondenceV0 {
            nodes: node_correspondence,
            relation_kinds,
            relations: relation_correspondence,
        };
        let witness = validate_structural_correspondence(source, target, &correspondence)
            .map_err(GraphMatchErrorV0::InvalidCandidate)?;

        Ok(GraphMatchResultV0 {
            correspondence,
            witness,
        })
    }
}

fn grouped_signatures(structure: &StructureV0) -> BTreeMap<NodeSignatureV0, Vec<u32>> {
    let mut grouped = BTreeMap::<NodeSignatureV0, Vec<u32>>::new();

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

        let signature = counts
            .into_iter()
            .map(|((orientation, arity), count)| (orientation, arity, count))
            .collect();
        grouped.entry(signature).or_default().push(node.id);
    }

    grouped
}

fn participant_transform(
    orientation: RelationOrientationV0,
    mapped_source: &[u32],
    target: &[u32],
) -> Option<ParticipantTransformV0> {
    match orientation {
        RelationOrientationV0::Directed => {
            if mapped_source == target {
                Some(ParticipantTransformV0::PreserveOrder)
            } else if mapped_source
                .iter()
                .rev()
                .copied()
                .eq(target.iter().copied())
            {
                Some(ParticipantTransformV0::ReverseOrder)
            } else {
                None
            }
        }
        RelationOrientationV0::Undirected => {
            let mut mapped = mapped_source.to_vec();
            let mut expected = target.to_vec();
            mapped.sort_unstable();
            expected.sort_unstable();
            (mapped == expected).then_some(ParticipantTransformV0::Unordered)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transfer::{NodeV0, RelationV0, StructuralCorrespondenceV0};

    fn asymmetric_structures() -> (StructureV0, StructureV0) {
        let source = StructureV0 {
            nodes: vec![NodeV0 { id: 1 }, NodeV0 { id: 2 }, NodeV0 { id: 3 }],
            relations: vec![
                RelationV0 {
                    id: 10,
                    kind: 1,
                    orientation: RelationOrientationV0::Directed,
                    participants: vec![1, 2],
                },
                RelationV0 {
                    id: 20,
                    kind: 2,
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
                    orientation: RelationOrientationV0::Directed,
                    participants: vec![12, 11],
                },
                RelationV0 {
                    id: 120,
                    kind: 200,
                    orientation: RelationOrientationV0::Undirected,
                    participants: vec![13, 12],
                },
            ],
        };
        (source, target)
    }

    #[test]
    fn discovers_unique_incidence_correspondence_and_declared_transforms() {
        let (source, target) = asymmetric_structures();
        let result = UniqueIncidenceSignatureBaselineV0
            .match_structures(&source, &target)
            .unwrap();

        assert_eq!(result.witness.mapped_nodes, 3);
        assert_eq!(result.witness.mapped_relation_kinds, 2);
        assert_eq!(result.witness.mapped_relations, 2);
        assert_eq!(
            result
                .correspondence
                .relation_kinds
                .iter()
                .find(|mapping| mapping.source_kind == 1)
                .unwrap()
                .participant_transform,
            ParticipantTransformV0::ReverseOrder
        );
    }

    #[test]
    fn abstains_on_ambiguous_symmetric_leaves() {
        let source = StructureV0 {
            nodes: vec![NodeV0 { id: 1 }, NodeV0 { id: 2 }, NodeV0 { id: 3 }],
            relations: vec![RelationV0 {
                id: 10,
                kind: 1,
                orientation: RelationOrientationV0::Undirected,
                participants: vec![1, 2, 3],
            }],
        };
        let target = StructureV0 {
            nodes: vec![NodeV0 { id: 11 }, NodeV0 { id: 12 }, NodeV0 { id: 13 }],
            relations: vec![RelationV0 {
                id: 110,
                kind: 100,
                orientation: RelationOrientationV0::Undirected,
                participants: vec![11, 12, 13],
            }],
        };

        assert_eq!(
            UniqueIncidenceSignatureBaselineV0.match_structures(&source, &target),
            Err(GraphMatchErrorV0::AmbiguousNodeSignature)
        );
    }

    #[test]
    fn rejects_inconsistent_mapping_for_one_source_relation_kind() {
        let (mut source, mut target) = asymmetric_structures();
        source.relations.push(RelationV0 {
            id: 30,
            kind: 1,
            orientation: RelationOrientationV0::Directed,
            participants: vec![1, 2],
        });
        target.relations.push(RelationV0 {
            id: 130,
            kind: 101,
            orientation: RelationOrientationV0::Directed,
            participants: vec![12, 11],
        });

        assert_eq!(
            UniqueIncidenceSignatureBaselineV0.match_structures(&source, &target),
            Err(GraphMatchErrorV0::InconsistentRelationKindTransform)
        );
    }

    #[test]
    fn validator_is_not_replaced_by_candidate_generation() {
        let (source, target) = asymmetric_structures();
        let candidate = StructuralCorrespondenceV0::default();

        assert_eq!(
            validate_structural_correspondence(&source, &target, &candidate),
            Err(TransferErrorV0::NodeMapIncomplete)
        );
    }
}
