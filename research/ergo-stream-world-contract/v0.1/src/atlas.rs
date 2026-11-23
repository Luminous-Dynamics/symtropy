//! Paired-world Transfer Atlas reference cases for ErgoStream v0.1.
//!
//! The first cases are intentionally small and finite:
//! - a true correspondence under node/relation relabeling and directed reversal;
//! - a false analogy pair with identical undirected degree signatures but
//!   different graph structure (K3,3 versus the triangular prism).
//!
//! This module exposes a policy view and an evaluator-side disposition API.
//! It is a local research reference, not a process-isolation boundary. Do not
//! link evaluator methods into an untrusted learner process.

use super::matching::{GraphMatchErrorV0, UniqueIncidenceSignatureBaselineV0};
use super::transfer::{
    ActionCorrespondenceV0, EffectSignV0, InterventionObservationV0, InterventionTransferWitnessV0,
    StructuralCorrespondenceV0, StructuralTransferWitnessV0, StructureV0, TransferErrorV0,
    validate_intervention_correspondence, validate_structural_correspondence,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferAtlasFamilyV0 {
    RelabelledReverse,
    DegreeMatchedNonIsomorphic,
    IsomorphicObservationDifferentMechanism,
    StructurallySimilarInterventionDivergent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionDescriptorV0 {
    pub id: u32,
    pub kind: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferAtlasPolicyViewV0 {
    pub source: StructureV0,
    pub target: StructureV0,
    pub source_actions: Vec<ActionDescriptorV0>,
    pub target_actions: Vec<ActionDescriptorV0>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferAtlasDispositionV0 {
    TransferAccepted,
    CorrectAbstention,
    MissedTransfer,
    RejectedFalseProposal,
    FalseUnification,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferAtlasReportV0 {
    pub disposition: TransferAtlasDispositionV0,
    pub witness: Option<StructuralTransferWitnessV0>,
    pub validation_error: Option<TransferErrorV0>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterventionAtlasDispositionV0 {
    NoEvidence,
    AllDeclaredEffectsMatch,
    Counterexample,
    InvalidCandidate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterventionAtlasReportV0 {
    pub disposition: InterventionAtlasDispositionV0,
    pub witness: Option<InterventionTransferWitnessV0>,
    pub validation_error: Option<TransferErrorV0>,
}

/// Owner-side paired-world fixture and evaluator.
///
/// The expected-transfer flag is intentionally private and absent from the
/// learner-facing policy view. The evaluate method is evaluator-only and must
/// not be exposed across an untrusted process boundary.
pub struct TransferAtlasWorldV0 {
    policy_view: TransferAtlasPolicyViewV0,
    transfer_expected: bool,
    intervention_observations: Option<Vec<InterventionObservationV0>>,
}

impl TransferAtlasWorldV0 {
    pub fn generate(family: TransferAtlasFamilyV0) -> Self {
        match family {
            TransferAtlasFamilyV0::RelabelledReverse => Self {
                policy_view: relabelled_reverse_pair(),
                transfer_expected: true,
                intervention_observations: Some(vec![InterventionObservationV0 {
                    source_action_id: 1,
                    target_action_id: 11,
                    source_effect: EffectSignV0::Positive,
                    target_effect: EffectSignV0::Positive,
                }]),
            },
            TransferAtlasFamilyV0::DegreeMatchedNonIsomorphic => Self {
                policy_view: degree_matched_non_isomorphic_pair(),
                transfer_expected: false,
                intervention_observations: None,
            },
            TransferAtlasFamilyV0::IsomorphicObservationDifferentMechanism => Self {
                policy_view: relabelled_reverse_pair(),
                transfer_expected: false,
                intervention_observations: None,
            },
            TransferAtlasFamilyV0::StructurallySimilarInterventionDivergent => Self {
                policy_view: intervention_divergent_pair(),
                transfer_expected: true,
                intervention_observations: Some(vec![
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
                ]),
            },
        }
    }

    /// Learner-visible source and target structures, without the expected
    /// transfer disposition. A caller must still ensure the learner cannot
    /// access this owner's evaluator process or hidden fixture custody.
    pub fn policy_view(&self) -> &TransferAtlasPolicyViewV0 {
        &self.policy_view
    }

    /// Evaluate an optional frozen learner-proposed mapping.
    ///
    /// None is an explicit abstention. An invalid mapping is a missed transfer
    /// for positive pairs and a rejected false proposal for negative pairs.
    /// A structurally valid mapping on a negative pair is a false unification.
    pub fn evaluate(
        &self,
        candidate: Option<&StructuralCorrespondenceV0>,
    ) -> TransferAtlasReportV0 {
        let Some(candidate) = candidate else {
            return TransferAtlasReportV0 {
                disposition: if self.transfer_expected {
                    TransferAtlasDispositionV0::MissedTransfer
                } else {
                    TransferAtlasDispositionV0::CorrectAbstention
                },
                witness: None,
                validation_error: None,
            };
        };

        match validate_structural_correspondence(
            &self.policy_view.source,
            &self.policy_view.target,
            candidate,
        ) {
            Ok(witness) => TransferAtlasReportV0 {
                disposition: if self.transfer_expected {
                    TransferAtlasDispositionV0::TransferAccepted
                } else {
                    TransferAtlasDispositionV0::FalseUnification
                },
                witness: Some(witness),
                validation_error: None,
            },
            Err(error) => TransferAtlasReportV0 {
                disposition: if self.transfer_expected {
                    TransferAtlasDispositionV0::MissedTransfer
                } else {
                    TransferAtlasDispositionV0::RejectedFalseProposal
                },
                witness: None,
                validation_error: Some(error),
            },
        }
    }

    /// Compare a frozen action-kind/effect mapping with evaluator-owned
    /// intervention observations. The outcome is only a qualitative sign
    /// check, not causal qualification or intervention provenance.
    pub fn evaluate_interventions(
        &self,
        candidate: &[ActionCorrespondenceV0],
    ) -> InterventionAtlasReportV0 {
        let Some(observations) = self.intervention_observations.as_deref() else {
            return InterventionAtlasReportV0 {
                disposition: InterventionAtlasDispositionV0::NoEvidence,
                witness: None,
                validation_error: None,
            };
        };

        match validate_intervention_correspondence(candidate, observations) {
            Ok(witness) => InterventionAtlasReportV0 {
                disposition: InterventionAtlasDispositionV0::AllDeclaredEffectsMatch,
                witness: Some(witness),
                validation_error: None,
            },
            Err(TransferErrorV0::InterventionEffectMismatch) => InterventionAtlasReportV0 {
                disposition: InterventionAtlasDispositionV0::Counterexample,
                witness: None,
                validation_error: Some(TransferErrorV0::InterventionEffectMismatch),
            },
            Err(error) => InterventionAtlasReportV0 {
                disposition: InterventionAtlasDispositionV0::InvalidCandidate,
                witness: None,
                validation_error: Some(error),
            },
        }
    }

    /// Run the transparent unique-incidence baseline on the policy view, then
    /// score its frozen proposal. The baseline's error is preserved so
    /// ambiguity-driven abstention is observable.
    pub fn run_unique_incidence_baseline(
        &self,
    ) -> Result<TransferAtlasReportV0, GraphMatchErrorV0> {
        let matcher = UniqueIncidenceSignatureBaselineV0;
        let matched =
            matcher.match_structures(&self.policy_view.source, &self.policy_view.target)?;
        Ok(self.evaluate(Some(&matched.correspondence)))
    }
}

fn relabelled_reverse_pair() -> TransferAtlasPolicyViewV0 {
    use super::transfer::{NodeV0, RelationOrientationV0, RelationV0};

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

    TransferAtlasPolicyViewV0 {
        source,
        target,
        source_actions: vec![ActionDescriptorV0 { id: 1, kind: 1 }],
        target_actions: vec![ActionDescriptorV0 { id: 11, kind: 10 }],
    }
}

fn degree_matched_non_isomorphic_pair() -> TransferAtlasPolicyViewV0 {
    use super::transfer::{NodeV0, RelationOrientationV0, RelationV0};

    let source_nodes = (1..=6).map(|id| NodeV0 { id }).collect();
    let mut source_relations = Vec::with_capacity(9);
    let mut relation_id = 10;
    // K3,3: every node has degree three and the graph is bipartite.
    for left in 1..=3 {
        for right in 4..=6 {
            source_relations.push(RelationV0 {
                id: relation_id,
                kind: 1,
                orientation: RelationOrientationV0::Undirected,
                participants: vec![left, right],
            });
            relation_id += 1;
        }
    }

    let target_nodes = (11..=16).map(|id| NodeV0 { id }).collect();
    let target_edges = [
        (11, 12),
        (12, 13),
        (13, 11),
        (14, 15),
        (15, 16),
        (16, 14),
        (11, 14),
        (12, 15),
        (13, 16),
    ];
    let target_relations = target_edges
        .into_iter()
        .enumerate()
        .map(|(index, (left, right))| RelationV0 {
            id: 110 + u32::try_from(index).expect("nine fixture edges fit u32"),
            kind: 100,
            orientation: RelationOrientationV0::Undirected,
            participants: vec![left, right],
        })
        .collect();

    TransferAtlasPolicyViewV0 {
        source: StructureV0 {
            nodes: source_nodes,
            relations: source_relations,
        },
        target: StructureV0 {
            nodes: target_nodes,
            relations: target_relations,
        },
        source_actions: Vec::new(),
        target_actions: Vec::new(),
    }
}

fn intervention_divergent_pair() -> TransferAtlasPolicyViewV0 {
    let mut pair = relabelled_reverse_pair();
    pair.source_actions = vec![
        ActionDescriptorV0 { id: 1, kind: 5 },
        ActionDescriptorV0 { id: 2, kind: 5 },
    ];
    pair.target_actions = vec![
        ActionDescriptorV0 { id: 11, kind: 50 },
        ActionDescriptorV0 { id: 22, kind: 50 },
    ];
    pair
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transfer::{
        EffectTransformV0, NodeCorrespondenceV0, ParticipantTransformV0,
        RelationCorrespondenceV0, RelationKindCorrespondenceV0,
    };

    #[test]
    fn unique_incidence_baseline_transfers_relabelled_reverse_pair() {
        let world = TransferAtlasWorldV0::generate(TransferAtlasFamilyV0::RelabelledReverse);
        let report = world.run_unique_incidence_baseline().unwrap();

        assert_eq!(
            report.disposition,
            TransferAtlasDispositionV0::TransferAccepted
        );
        assert_eq!(report.witness.unwrap().mapped_nodes, 3);
    }

    #[test]
    fn unique_incidence_baseline_abstains_on_degree_matched_false_analogy() {
        let world =
            TransferAtlasWorldV0::generate(TransferAtlasFamilyV0::DegreeMatchedNonIsomorphic);

        let result = world.run_unique_incidence_baseline();
        assert_eq!(result, Err(GraphMatchErrorV0::AmbiguousNodeSignature));

        let report = world.evaluate(None);
        assert_eq!(
            report.disposition,
            TransferAtlasDispositionV0::CorrectAbstention
        );
    }

    fn degree_sequence(structure: &StructureV0) -> Vec<usize> {
        let mut degrees = structure
            .nodes
            .iter()
            .map(|node| {
                structure
                    .relations
                    .iter()
                    .filter(|relation| relation.participants.contains(&node.id))
                    .count()
            })
            .collect::<Vec<_>>();
        degrees.sort_unstable();
        degrees
    }

    fn triangle_count(structure: &StructureV0) -> usize {
        let mut triangles = 0;
        for left in &structure.nodes {
            for middle in &structure.nodes {
                for right in &structure.nodes {
                    if left.id < middle.id
                        && middle.id < right.id
                        && has_undirected_edge(structure, left.id, middle.id)
                        && has_undirected_edge(structure, middle.id, right.id)
                        && has_undirected_edge(structure, right.id, left.id)
                    {
                        triangles += 1;
                    }
                }
            }
        }
        triangles
    }

    fn has_undirected_edge(structure: &StructureV0, left: u32, right: u32) -> bool {
        structure.relations.iter().any(|relation| {
            relation.orientation == super::super::transfer::RelationOrientationV0::Undirected
                && relation.participants.len() == 2
                && ((relation.participants[0] == left && relation.participants[1] == right)
                    || (relation.participants[0] == right && relation.participants[1] == left))
        })
    }

    #[test]
    fn degree_matched_false_pair_has_distinct_triangle_invariant() {
        let world =
            TransferAtlasWorldV0::generate(TransferAtlasFamilyV0::DegreeMatchedNonIsomorphic);
        let view = world.policy_view();

        assert_eq!(degree_sequence(&view.source), vec![3, 3, 3, 3, 3, 3]);
        assert_eq!(degree_sequence(&view.target), vec![3, 3, 3, 3, 3, 3]);
        assert_eq!(triangle_count(&view.source), 0);
        assert_eq!(triangle_count(&view.target), 2);
    }

    #[test]
    fn unique_incidence_baseline_false_unifies_isomorphic_observation_with_different_mechanism() {
        let world = TransferAtlasWorldV0::generate(
            TransferAtlasFamilyV0::IsomorphicObservationDifferentMechanism,
        );
        let report = world.run_unique_incidence_baseline().unwrap();

        assert_eq!(
            report.disposition,
            TransferAtlasDispositionV0::FalseUnification
        );
        assert!(report.witness.is_some());
    }

    #[test]
    fn structural_match_can_fail_under_intervention_divergence() {
        let world = TransferAtlasWorldV0::generate(
            TransferAtlasFamilyV0::StructurallySimilarInterventionDivergent,
        );
        let structural = world.run_unique_incidence_baseline().unwrap();
        assert_eq!(
            structural.disposition,
            TransferAtlasDispositionV0::TransferAccepted
        );

        let candidate = vec![
            ActionCorrespondenceV0 {
                source_action_id: 1,
                target_action_id: 11,
                source_action_kind: 5,
                target_action_kind: 50,
                effect_transform: EffectTransformV0::Preserve,
            },
            ActionCorrespondenceV0 {
                source_action_id: 2,
                target_action_id: 22,
                source_action_kind: 5,
                target_action_kind: 50,
                effect_transform: EffectTransformV0::Preserve,
            },
        ];
        let report = world.evaluate_interventions(&candidate);
        assert_eq!(
            report.disposition,
            InterventionAtlasDispositionV0::Counterexample
        );
    }

    #[test]
    fn intervention_mapping_cannot_change_transform_per_example() {
        let world = TransferAtlasWorldV0::generate(
            TransferAtlasFamilyV0::StructurallySimilarInterventionDivergent,
        );
        let candidate = vec![
            ActionCorrespondenceV0 {
                source_action_id: 1,
                target_action_id: 11,
                source_action_kind: 5,
                target_action_kind: 50,
                effect_transform: EffectTransformV0::Preserve,
            },
            ActionCorrespondenceV0 {
                source_action_id: 2,
                target_action_id: 22,
                source_action_kind: 5,
                target_action_kind: 50,
                effect_transform: EffectTransformV0::Negate,
            },
        ];
        let report = world.evaluate_interventions(&candidate);
        assert_eq!(
            report.disposition,
            InterventionAtlasDispositionV0::InvalidCandidate
        );
        assert_eq!(
            report.validation_error,
            Some(TransferErrorV0::InconsistentActionKindTransform)
        );
    }

    #[test]
    fn matching_qualitative_signs_are_not_reported_as_causal_qualification() {
        let world = TransferAtlasWorldV0::generate(TransferAtlasFamilyV0::RelabelledReverse);
        let candidate = vec![ActionCorrespondenceV0 {
            source_action_id: 1,
            target_action_id: 11,
            source_action_kind: 1,
            target_action_kind: 10,
            effect_transform: EffectTransformV0::Preserve,
        }];

        let report = world.evaluate_interventions(&candidate);
        assert_eq!(
            report.disposition,
            InterventionAtlasDispositionV0::AllDeclaredEffectsMatch
        );
        assert!(report.witness.unwrap().all_declared_effects_match);
    }

    #[test]
    fn complete_degree_signature_map_is_rejected_on_false_analogy() {
        let world =
            TransferAtlasWorldV0::generate(TransferAtlasFamilyV0::DegreeMatchedNonIsomorphic);
        let candidate = StructuralCorrespondenceV0 {
            nodes: (1_u32..=6)
                .map(|source_node_id| NodeCorrespondenceV0 {
                    source_node_id,
                    target_node_id: source_node_id + 10,
                })
                .collect(),
            relation_kinds: vec![RelationKindCorrespondenceV0 {
                source_kind: 1,
                target_kind: 100,
                participant_transform: ParticipantTransformV0::Unordered,
            }],
            relations: (0_u32..9)
                .map(|index| RelationCorrespondenceV0 {
                    source_relation_id: 10 + index,
                    target_relation_id: 110 + index,
                })
                .collect(),
        };

        let report = world.evaluate(Some(&candidate));
        assert_eq!(
            report.disposition,
            TransferAtlasDispositionV0::RejectedFalseProposal
        );
        assert_eq!(
            report.validation_error,
            Some(TransferErrorV0::IncidenceMismatch)
        );
    }

    #[test]
    fn incomplete_candidate_is_not_accepted_on_non_isomorphic_pair() {
        let world =
            TransferAtlasWorldV0::generate(TransferAtlasFamilyV0::DegreeMatchedNonIsomorphic);
        let view = world.policy_view();
        let incomplete = StructuralCorrespondenceV0::default();

        let report = world.evaluate(Some(&incomplete));
        assert_eq!(
            report.disposition,
            TransferAtlasDispositionV0::RejectedFalseProposal
        );
        assert_eq!(
            report.validation_error,
            Some(TransferErrorV0::NodeMapIncomplete)
        );
        assert_eq!(view.source.nodes.len(), 6);
        assert_eq!(view.target.nodes.len(), 6);
    }
}
