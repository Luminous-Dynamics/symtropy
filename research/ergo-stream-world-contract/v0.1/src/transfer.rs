//! Finite structural-transfer witnesses for ErgoStream v0.1.
//!
//! This module validates a learner-proposed correspondence between two
//! explicitly scoped finite structures. It checks total one-to-one mappings,
//! relation-kind consistency, and incidence preservation under a declared
//! transformation. A separate function checks precommitted qualitative
//! intervention-effect transformations.
//!
//! These checks validate a supplied witness; they do not discover the mapping,
//! establish the trustworthiness of observations, create an OS isolation
//! boundary, or independently qualify a scientific claim.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeV0 {
    pub id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationOrientationV0 {
    Directed,
    Undirected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationV0 {
    pub id: u32,
    pub kind: u16,
    pub orientation: RelationOrientationV0,
    pub participants: Vec<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StructureV0 {
    pub nodes: Vec<NodeV0>,
    pub relations: Vec<RelationV0>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeCorrespondenceV0 {
    pub source_node_id: u32,
    pub target_node_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticipantTransformV0 {
    PreserveOrder,
    ReverseOrder,
    Unordered,
}

/// Relation kinds may have different domain-local IDs, but their mapping must
/// be consistent across every occurrence in the declared finite structures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelationKindCorrespondenceV0 {
    pub source_kind: u16,
    pub target_kind: u16,
    pub participant_transform: ParticipantTransformV0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelationCorrespondenceV0 {
    pub source_relation_id: u32,
    pub target_relation_id: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StructuralCorrespondenceV0 {
    pub nodes: Vec<NodeCorrespondenceV0>,
    pub relation_kinds: Vec<RelationKindCorrespondenceV0>,
    pub relations: Vec<RelationCorrespondenceV0>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructuralTransferWitnessV0 {
    pub mapped_nodes: usize,
    pub mapped_relation_kinds: usize,
    pub mapped_relations: usize,
    pub checked_incidences: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectSignV0 {
    Negative,
    NoChange,
    Positive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectTransformV0 {
    Preserve,
    Negate,
}

/// This mapping is part of the candidate submitted before the held-out
/// intervention observations are revealed to the learner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionCorrespondenceV0 {
    pub source_action_id: u32,
    pub target_action_id: u32,
    pub source_action_kind: u16,
    pub target_action_kind: u16,
    pub effect_transform: EffectTransformV0,
}

/// Evaluator-supplied qualitative result for a matched intervention pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterventionObservationV0 {
    pub source_action_id: u32,
    pub target_action_id: u32,
    pub source_effect: EffectSignV0,
    pub target_effect: EffectSignV0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterventionTransferWitnessV0 {
    pub compared_actions: usize,
    /// False when no intervention mapping/evidence was supplied. Structural
    /// correspondence alone must not be presented as causal qualification.
    pub all_declared_effects_match: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferErrorV0 {
    NoRelations,
    DuplicateSourceNodeId,
    DuplicateTargetNodeId,
    DuplicateSourceRelationId,
    DuplicateTargetRelationId,
    EmptyRelationParticipants,
    UnknownSourceParticipant,
    UnknownTargetParticipant,
    NodeMapIncomplete,
    NodeMapNotInjective,
    UnknownSourceNodeMapping,
    UnknownTargetNodeMapping,
    RelationMapIncomplete,
    RelationMapNotInjective,
    UnknownSourceRelationMapping,
    UnknownTargetRelationMapping,
    RelationKindMapIncomplete,
    RelationKindMapNotInjective,
    UnknownSourceRelationKind,
    UnknownTargetRelationKind,
    RelationArityMismatch,
    RelationOrientationMismatch,
    InvalidParticipantTransform,
    IncidenceMismatch,
    ActionMapNotInjective,
    ActionKindMapNotInjective,
    InconsistentActionKindTransform,
    MissingInterventionEvidence,
    DuplicateInterventionEvidence,
    UnmappedInterventionEvidence,
    InterventionEffectMismatch,
}

impl fmt::Display for TransferErrorV0 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NoRelations => "both structures must contain at least one relation",
            Self::DuplicateSourceNodeId => "source node IDs must be unique",
            Self::DuplicateTargetNodeId => "target node IDs must be unique",
            Self::DuplicateSourceRelationId => "source relation IDs must be unique",
            Self::DuplicateTargetRelationId => "target relation IDs must be unique",
            Self::EmptyRelationParticipants => "relations must have at least one participant",
            Self::UnknownSourceParticipant => "source relation references an unknown node",
            Self::UnknownTargetParticipant => "target relation references an unknown node",
            Self::NodeMapIncomplete => "node correspondence must cover both structures",
            Self::NodeMapNotInjective => "node correspondence must be one-to-one",
            Self::UnknownSourceNodeMapping => {
                "node correspondence references an unknown source node"
            }
            Self::UnknownTargetNodeMapping => {
                "node correspondence references an unknown target node"
            }
            Self::RelationMapIncomplete => "relation correspondence must cover both structures",
            Self::RelationMapNotInjective => "relation correspondence must be one-to-one",
            Self::UnknownSourceRelationMapping => {
                "relation correspondence references an unknown source relation"
            }
            Self::UnknownTargetRelationMapping => {
                "relation correspondence references an unknown target relation"
            }
            Self::RelationKindMapIncomplete => {
                "relation-kind correspondence must cover both structures"
            }
            Self::RelationKindMapNotInjective => "relation-kind correspondence must be one-to-one",
            Self::UnknownSourceRelationKind => {
                "relation-kind correspondence references an unknown source kind"
            }
            Self::UnknownTargetRelationKind => {
                "relation-kind correspondence references an unknown target kind"
            }
            Self::RelationArityMismatch => "mapped relations have different participant counts",
            Self::RelationOrientationMismatch => "mapped relations have different directedness",
            Self::InvalidParticipantTransform => {
                "participant transform is incompatible with relation orientation"
            }
            Self::IncidenceMismatch => {
                "mapped relation participants do not preserve the declared structure"
            }
            Self::ActionMapNotInjective => "action correspondence must be one-to-one",
            Self::ActionKindMapNotInjective => "distinct source action kinds must map to distinct target kinds",
            Self::InconsistentActionKindTransform => "one source action kind has inconsistent target kinds or effect transforms",
            Self::MissingInterventionEvidence => "a mapped action has no intervention evidence",
            Self::DuplicateInterventionEvidence => {
                "intervention evidence contains a duplicate action pair"
            }
            Self::UnmappedInterventionEvidence => {
                "intervention evidence has no declared action correspondence"
            }
            Self::InterventionEffectMismatch => {
                "observed intervention effect contradicts the precommitted transform"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for TransferErrorV0 {}

/// Validate a total, one-to-one structural correspondence over the complete
/// finite structures supplied to this function.
///
/// Directed relation participants must preserve or reverse their order exactly.
/// Undirected participants must match as a set. Relation-kind mappings must be
/// one-to-one and use one consistent participant transform for every instance.
pub fn validate_structural_correspondence(
    source: &StructureV0,
    target: &StructureV0,
    correspondence: &StructuralCorrespondenceV0,
) -> Result<StructuralTransferWitnessV0, TransferErrorV0> {
    validate_structure(source, true)?;
    validate_structure(target, false)?;

    if source.relations.is_empty() || target.relations.is_empty() {
        return Err(TransferErrorV0::NoRelations);
    }

    if correspondence.nodes.len() != source.nodes.len()
        || correspondence.nodes.len() != target.nodes.len()
    {
        return Err(TransferErrorV0::NodeMapIncomplete);
    }

    if !ids_are_unique(&correspondence.nodes, |pair| pair.source_node_id)
        || !ids_are_unique(&correspondence.nodes, |pair| pair.target_node_id)
    {
        return Err(TransferErrorV0::NodeMapNotInjective);
    }

    for pair in &correspondence.nodes {
        if !source
            .nodes
            .iter()
            .any(|node| node.id == pair.source_node_id)
        {
            return Err(TransferErrorV0::UnknownSourceNodeMapping);
        }
        if !target
            .nodes
            .iter()
            .any(|node| node.id == pair.target_node_id)
        {
            return Err(TransferErrorV0::UnknownTargetNodeMapping);
        }
    }

    let source_kinds = unique_relation_kinds(source);
    let target_kinds = unique_relation_kinds(target);
    if correspondence.relation_kinds.len() != source_kinds.len()
        || correspondence.relation_kinds.len() != target_kinds.len()
    {
        return Err(TransferErrorV0::RelationKindMapIncomplete);
    }

    if !ids_are_unique(&correspondence.relation_kinds, |pair| {
        u32::from(pair.source_kind)
    }) || !ids_are_unique(&correspondence.relation_kinds, |pair| {
        u32::from(pair.target_kind)
    }) {
        return Err(TransferErrorV0::RelationKindMapNotInjective);
    }

    for pair in &correspondence.relation_kinds {
        if !source_kinds.contains(&pair.source_kind) {
            return Err(TransferErrorV0::UnknownSourceRelationKind);
        }
        if !target_kinds.contains(&pair.target_kind) {
            return Err(TransferErrorV0::UnknownTargetRelationKind);
        }
    }

    if correspondence.relations.len() != source.relations.len()
        || correspondence.relations.len() != target.relations.len()
    {
        return Err(TransferErrorV0::RelationMapIncomplete);
    }

    if !ids_are_unique(&correspondence.relations, |pair| pair.source_relation_id)
        || !ids_are_unique(&correspondence.relations, |pair| pair.target_relation_id)
    {
        return Err(TransferErrorV0::RelationMapNotInjective);
    }

    let mut checked_incidences = 0;
    for pair in &correspondence.relations {
        let source_relation = source
            .relations
            .iter()
            .find(|relation| relation.id == pair.source_relation_id)
            .ok_or(TransferErrorV0::UnknownSourceRelationMapping)?;
        let target_relation = target
            .relations
            .iter()
            .find(|relation| relation.id == pair.target_relation_id)
            .ok_or(TransferErrorV0::UnknownTargetRelationMapping)?;

        if source_relation.participants.len() != target_relation.participants.len() {
            return Err(TransferErrorV0::RelationArityMismatch);
        }

        if source_relation.orientation != target_relation.orientation {
            return Err(TransferErrorV0::RelationOrientationMismatch);
        }

        let kind_pair = correspondence
            .relation_kinds
            .iter()
            .find(|kind| kind.source_kind == source_relation.kind)
            .ok_or(TransferErrorV0::UnknownSourceRelationKind)?;

        if kind_pair.target_kind != target_relation.kind {
            return Err(TransferErrorV0::IncidenceMismatch);
        }

        let mut mapped_participants = source_relation
            .participants
            .iter()
            .map(|source_id| {
                correspondence
                    .nodes
                    .iter()
                    .find(|node| node.source_node_id == *source_id)
                    .map(|node| node.target_node_id)
                    .ok_or(TransferErrorV0::UnknownSourceNodeMapping)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let incidence_matches = match (source_relation.orientation, kind_pair.participant_transform)
        {
            (RelationOrientationV0::Directed, ParticipantTransformV0::PreserveOrder) => {
                mapped_participants == target_relation.participants
            }
            (RelationOrientationV0::Directed, ParticipantTransformV0::ReverseOrder) => {
                mapped_participants.reverse();
                mapped_participants == target_relation.participants
            }
            (RelationOrientationV0::Undirected, ParticipantTransformV0::Unordered) => {
                mapped_participants.sort_unstable();
                let mut target_participants = target_relation.participants.clone();
                target_participants.sort_unstable();
                mapped_participants == target_participants
            }
            _ => return Err(TransferErrorV0::InvalidParticipantTransform),
        };

        if !incidence_matches {
            return Err(TransferErrorV0::IncidenceMismatch);
        }

        checked_incidences += source_relation.participants.len();
    }

    Ok(StructuralTransferWitnessV0 {
        mapped_nodes: correspondence.nodes.len(),
        mapped_relation_kinds: correspondence.relation_kinds.len(),
        mapped_relations: correspondence.relations.len(),
        checked_incidences,
    })
}

/// Check the held-out qualitative effects against an action/effect mapping that
/// the learner committed before these observations were revealed.
///
/// An empty mapping returns an explicitly unqualified result. This function
/// validates declared observations; their provenance and intervention
/// execution must be independently established by the enclosing evaluator.
pub fn validate_intervention_correspondence(
    action_map: &[ActionCorrespondenceV0],
    observations: &[InterventionObservationV0],
) -> Result<InterventionTransferWitnessV0, TransferErrorV0> {
    if !ids_are_unique(action_map, |pair| pair.source_action_id)
        || !ids_are_unique(action_map, |pair| pair.target_action_id)
    {
        return Err(TransferErrorV0::ActionMapNotInjective);
    }

    let mut action_kind_map = Vec::<(u16, u16, EffectTransformV0)>::new();
    for mapping in action_map {
        match action_kind_map
            .iter()
            .find(|(source_kind, _, _)| *source_kind == mapping.source_action_kind)
        {
            Some((_, target_kind, transform))
                if *target_kind != mapping.target_action_kind
                    || *transform != mapping.effect_transform =>
            {
                return Err(TransferErrorV0::InconsistentActionKindTransform);
            }
            Some(_) => {}
            None => {
                if action_kind_map
                    .iter()
                    .any(|(_, target_kind, _)| *target_kind == mapping.target_action_kind)
                {
                    return Err(TransferErrorV0::ActionKindMapNotInjective);
                }
                action_kind_map.push((
                    mapping.source_action_kind,
                    mapping.target_action_kind,
                    mapping.effect_transform,
                ));
            }
        }
    }

    if observations.len() < action_map.len() {
        return Err(TransferErrorV0::MissingInterventionEvidence);
    }
    if observations.len() > action_map.len() {
        return Err(TransferErrorV0::UnmappedInterventionEvidence);
    }

    if !ids_are_unique(observations, |observation| observation.source_action_id)
        || !ids_are_unique(observations, |observation| observation.target_action_id)
    {
        return Err(TransferErrorV0::DuplicateInterventionEvidence);
    }

    for mapping in action_map {
        let observation = observations
            .iter()
            .find(|observation| {
                observation.source_action_id == mapping.source_action_id
                    && observation.target_action_id == mapping.target_action_id
            })
            .ok_or(TransferErrorV0::MissingInterventionEvidence)?;

        if transform_effect(observation.source_effect, mapping.effect_transform)
            != observation.target_effect
        {
            return Err(TransferErrorV0::InterventionEffectMismatch);
        }
    }

    Ok(InterventionTransferWitnessV0 {
        compared_actions: action_map.len(),
        all_declared_effects_match: !action_map.is_empty(),
    })
}

fn transform_effect(effect: EffectSignV0, transform: EffectTransformV0) -> EffectSignV0 {
    match (effect, transform) {
        (EffectSignV0::Positive, EffectTransformV0::Negate) => EffectSignV0::Negative,
        (EffectSignV0::Negative, EffectTransformV0::Negate) => EffectSignV0::Positive,
        (unchanged, EffectTransformV0::Preserve | EffectTransformV0::Negate) => unchanged,
    }
}

fn validate_structure(structure: &StructureV0, is_source: bool) -> Result<(), TransferErrorV0> {
    if !ids_are_unique(&structure.nodes, |node| node.id) {
        return Err(if is_source {
            TransferErrorV0::DuplicateSourceNodeId
        } else {
            TransferErrorV0::DuplicateTargetNodeId
        });
    }

    if !ids_are_unique(&structure.relations, |relation| relation.id) {
        return Err(if is_source {
            TransferErrorV0::DuplicateSourceRelationId
        } else {
            TransferErrorV0::DuplicateTargetRelationId
        });
    }

    for relation in &structure.relations {
        if relation.participants.is_empty() {
            return Err(TransferErrorV0::EmptyRelationParticipants);
        }
        for participant in &relation.participants {
            let exists = structure.nodes.iter().any(|node| node.id == *participant);
            if !exists {
                return Err(if is_source {
                    TransferErrorV0::UnknownSourceParticipant
                } else {
                    TransferErrorV0::UnknownTargetParticipant
                });
            }
        }
    }

    Ok(())
}

fn unique_relation_kinds(structure: &StructureV0) -> Vec<u16> {
    let mut kinds = Vec::new();
    for relation in &structure.relations {
        if !kinds.contains(&relation.kind) {
            kinds.push(relation.kind);
        }
    }
    kinds
}

fn ids_are_unique<T>(values: &[T], id_of: impl Fn(&T) -> u32) -> bool {
    values.iter().enumerate().all(|(index, value)| {
        !values[index + 1..]
            .iter()
            .any(|later| id_of(value) == id_of(later))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (StructureV0, StructureV0, StructuralCorrespondenceV0) {
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

        let correspondence = StructuralCorrespondenceV0 {
            nodes: vec![
                NodeCorrespondenceV0 {
                    source_node_id: 1,
                    target_node_id: 11,
                },
                NodeCorrespondenceV0 {
                    source_node_id: 2,
                    target_node_id: 12,
                },
                NodeCorrespondenceV0 {
                    source_node_id: 3,
                    target_node_id: 13,
                },
            ],
            relation_kinds: vec![
                RelationKindCorrespondenceV0 {
                    source_kind: 1,
                    target_kind: 100,
                    participant_transform: ParticipantTransformV0::ReverseOrder,
                },
                RelationKindCorrespondenceV0 {
                    source_kind: 2,
                    target_kind: 200,
                    participant_transform: ParticipantTransformV0::Unordered,
                },
            ],
            relations: vec![
                RelationCorrespondenceV0 {
                    source_relation_id: 10,
                    target_relation_id: 110,
                },
                RelationCorrespondenceV0 {
                    source_relation_id: 20,
                    target_relation_id: 120,
                },
            ],
        };

        (source, target, correspondence)
    }

    #[test]
    fn accepts_relabelled_structure_with_explicit_relation_transforms() {
        let (source, target, correspondence) = fixture();
        let witness =
            validate_structural_correspondence(&source, &target, &correspondence).unwrap();

        assert_eq!(witness.mapped_nodes, 3);
        assert_eq!(witness.mapped_relation_kinds, 2);
        assert_eq!(witness.mapped_relations, 2);
        assert_eq!(witness.checked_incidences, 4);
    }

    #[test]
    fn rejects_false_analogy_with_incidence_counterexample() {
        let (source, mut target, correspondence) = fixture();
        target.relations[1].participants = vec![11, 13];

        assert_eq!(
            validate_structural_correspondence(&source, &target, &correspondence),
            Err(TransferErrorV0::IncidenceMismatch)
        );
    }

    #[test]
    fn rejects_relation_kind_false_unification() {
        let (source, target, mut correspondence) = fixture();
        correspondence.relation_kinds[1].target_kind = 100;

        assert_eq!(
            validate_structural_correspondence(&source, &target, &correspondence),
            Err(TransferErrorV0::RelationKindMapNotInjective)
        );
    }

    #[test]
    fn rejects_non_injective_node_mapping() {
        let (source, target, mut correspondence) = fixture();
        correspondence.nodes[2].target_node_id = 12;

        assert_eq!(
            validate_structural_correspondence(&source, &target, &correspondence),
            Err(TransferErrorV0::NodeMapNotInjective)
        );
    }

    #[test]
    fn rejects_incomplete_relation_mapping() {
        let (source, target, mut correspondence) = fixture();
        correspondence.relations.pop();

        assert_eq!(
            validate_structural_correspondence(&source, &target, &correspondence),
            Err(TransferErrorV0::RelationMapIncomplete)
        );
    }

    #[test]
    fn rejects_directed_relation_with_unordered_transform() {
        let (source, target, mut correspondence) = fixture();
        correspondence.relation_kinds[0].participant_transform = ParticipantTransformV0::Unordered;

        assert_eq!(
            validate_structural_correspondence(&source, &target, &correspondence),
            Err(TransferErrorV0::InvalidParticipantTransform)
        );
    }

    fn intervention_fixture() -> (Vec<ActionCorrespondenceV0>, Vec<InterventionObservationV0>) {
        let action_map = vec![
            ActionCorrespondenceV0 {
                source_action_id: 1,
                target_action_id: 11,
                source_action_kind: 1,
                target_action_kind: 10,
                effect_transform: EffectTransformV0::Preserve,
            },
            ActionCorrespondenceV0 {
                source_action_id: 2,
                target_action_id: 22,
                source_action_kind: 2,
                target_action_kind: 20,
                effect_transform: EffectTransformV0::Negate,
            },
        ];

        let observations = vec![
            InterventionObservationV0 {
                source_action_id: 1,
                target_action_id: 11,
                source_effect: EffectSignV0::Positive,
                target_effect: EffectSignV0::Positive,
            },
            InterventionObservationV0 {
                source_action_id: 2,
                target_action_id: 22,
                source_effect: EffectSignV0::Negative,
                target_effect: EffectSignV0::Positive,
            },
        ];

        (action_map, observations)
    }

    #[test]
    fn accepts_precommitted_intervention_effect_transforms() {
        let (action_map, observations) = intervention_fixture();
        let witness = validate_intervention_correspondence(&action_map, &observations).unwrap();

        assert_eq!(witness.compared_actions, 2);
        assert!(witness.all_declared_effects_match);
    }

    #[test]
    fn rejects_inconsistent_transform_for_same_action_kind() {
        let mut action_map = vec![
            ActionCorrespondenceV0 {
                source_action_id: 1,
                target_action_id: 11,
                source_action_kind: 1,
                target_action_kind: 10,
                effect_transform: EffectTransformV0::Preserve,
            },
            ActionCorrespondenceV0 {
                source_action_id: 2,
                target_action_id: 22,
                source_action_kind: 1,
                target_action_kind: 10,
                effect_transform: EffectTransformV0::Negate,
            },
        ];
        let observations = vec![
            InterventionObservationV0 {
                source_action_id: 1,
                target_action_id: 11,
                source_effect: EffectSignV0::Positive,
                target_effect: EffectSignV0::Positive,
            },
            InterventionObservationV0 {
                source_action_id: 2,
                target_action_id: 22,
                source_effect: EffectSignV0::Negative,
                target_effect: EffectSignV0::Positive,
            },
        ];

        action_map[1].effect_transform = EffectTransformV0::Negate;
        assert_eq!(
            validate_intervention_correspondence(&action_map, &observations),
            Err(TransferErrorV0::InconsistentActionKindTransform)
        );
    }

    #[test]
    fn rejects_intervention_counterexample() {
        let (action_map, mut observations) = intervention_fixture();
        observations[1].target_effect = EffectSignV0::Negative;

        assert_eq!(
            validate_intervention_correspondence(&action_map, &observations),
            Err(TransferErrorV0::InterventionEffectMismatch)
        );
    }

    #[test]
    fn missing_intervention_evidence_is_not_qualified() {
        let (action_map, mut observations) = intervention_fixture();
        observations.pop();

        assert_eq!(
            validate_intervention_correspondence(&action_map, &observations),
            Err(TransferErrorV0::MissingInterventionEvidence)
        );
    }

    #[test]
    fn no_intervention_evidence_produces_an_unqualified_result() {
        let witness = validate_intervention_correspondence(&[], &[]).unwrap();

        assert_eq!(witness.compared_actions, 0);
        assert!(!witness.all_declared_effects_match);
    }
}
