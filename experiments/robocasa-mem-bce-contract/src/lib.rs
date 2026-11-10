//! Synthetic behavior-conditioned evaluation (BCE) contract.
//!
//! This crate rehearses one narrow property described by RoboCasa-Mem:
//! a later evaluator constraint can depend on the agent's own earlier
//! interaction, where multiple paths are valid. The evaluator derives the
//! target from an observed prior interaction rather than a fixed target.
//!
//! This is a synthetic contract fixture, not a RoboCasa-Mem implementation,
//! task evaluator, simulator adapter, sensor witness, or physical evidence.
//! Policy-visible input and evaluator-private derived constraints are separate
//! Rust types so the latter cannot accidentally be passed through this API.

#![forbid(unsafe_code)]

pub mod process_memory;
pub mod result_envelope;

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionKind {
    /// Example: the agent actually dosed / modified a selected cup.
    PriorChoice,
    /// Example: the agent later selects that same cup for a follow-up action.
    FollowUpChoice,
    /// Irrelevant interaction used to test history invariance.
    Unrelated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionEvent {
    pub episode_id: String,
    pub interaction_id: String,
    /// Evaluator-observed event sequence. Must be strictly increasing.
    pub sequence: u64,
    pub kind: InteractionKind,
    pub entity_id: String,
    /// Synthetic evaluator receipt reference, not a sensor or physical witness.
    pub observed_event_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyInput {
    /// Only policy-permitted information belongs here.
    pub episode_id: String,
    pub instruction: String,
    pub current_observation_ref: String,
    pub robot_state_ref: String,
}

/// The rule is part of the evaluator-side benchmark profile, not the policy input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsistencyPredicate {
    SameEntityAsPriorInteraction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BceRule {
    pub source_kind: InteractionKind,
    pub target_kind: InteractionKind,
    pub predicate: ConsistencyPredicate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluatorDerivedConstraint {
    pub episode_id: String,
    pub source_interaction_id: String,
    pub source_sequence: u64,
    pub source_event_ref: String,
    /// Evaluator-private expected entity, derived from the observed prior event.
    pub expected_entity_id: String,
    pub target_kind: InteractionKind,
    pub predicate: ConsistencyPredicate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BceError {
    EmptyEpisodeId,
    EmptyInteractionId,
    EmptyEntityId,
    MissingObservedEventRef,
    DuplicateInteractionId(String),
    DuplicateSequence(u64),
    NonMonotonicSequence {
        previous: u64,
        received: u64,
    },
    MixedEpisode {
        expected: String,
        received: String,
    },
    NoPriorInteraction(InteractionKind),
    TargetEpisodeMismatch {
        expected: String,
        received: String,
    },
    TargetKindMismatch {
        expected: InteractionKind,
        received: InteractionKind,
    },
    TargetNotLater {
        source_sequence: u64,
        target_sequence: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BceOutcome {
    Consistent {
        episode_id: String,
        source_interaction_id: String,
        target_interaction_id: String,
        entity_id: String,
    },
    Inconsistent {
        episode_id: String,
        source_interaction_id: String,
        target_interaction_id: String,
        expected_entity_id: String,
        actual_entity_id: String,
    },
}

/// Validate the chronological evaluator event stream. This function does not
/// sort, deduplicate or repair input: replayed, reordered or mixed-episode
/// events remain visible as explicit errors.
pub fn validate_interaction_stream(events: &[InteractionEvent]) -> Result<(), BceError> {
    let Some(first) = events.first() else {
        return Err(BceError::NoPriorInteraction(InteractionKind::PriorChoice));
    };

    if first.episode_id.trim().is_empty() {
        return Err(BceError::EmptyEpisodeId);
    }

    let mut ids = BTreeSet::new();
    let mut sequences = BTreeSet::new();
    let mut previous_sequence = None;

    for event in events {
        if event.episode_id.trim().is_empty() {
            return Err(BceError::EmptyEpisodeId);
        }
        if event.interaction_id.trim().is_empty() {
            return Err(BceError::EmptyInteractionId);
        }
        if event.entity_id.trim().is_empty() {
            return Err(BceError::EmptyEntityId);
        }
        if event.observed_event_ref.trim().is_empty() {
            return Err(BceError::MissingObservedEventRef);
        }
        if event.episode_id != first.episode_id {
            return Err(BceError::MixedEpisode {
                expected: first.episode_id.clone(),
                received: event.episode_id.clone(),
            });
        }
        if !ids.insert(event.interaction_id.as_str()) {
            return Err(BceError::DuplicateInteractionId(
                event.interaction_id.clone(),
            ));
        }
        if !sequences.insert(event.sequence) {
            return Err(BceError::DuplicateSequence(event.sequence));
        }
        if let Some(previous) = previous_sequence {
            if event.sequence <= previous {
                return Err(BceError::NonMonotonicSequence {
                    previous,
                    received: event.sequence,
                });
            }
        }
        previous_sequence = Some(event.sequence);
    }

    Ok(())
}

/// Derive a later target from the latest evaluator-observed interaction of the
/// declared source kind. No fixed target is accepted as an argument. The
/// returned constraint is evaluator-private and must never be added to
/// PolicyInput or exposed to the acting policy.
pub fn derive_constraint(
    events: &[InteractionEvent],
    rule: &BceRule,
) -> Result<EvaluatorDerivedConstraint, BceError> {
    validate_interaction_stream(events)?;

    let source = events
        .iter()
        .rev()
        .find(|event| event.kind == rule.source_kind)
        .ok_or(BceError::NoPriorInteraction(rule.source_kind))?;

    Ok(EvaluatorDerivedConstraint {
        episode_id: source.episode_id.clone(),
        source_interaction_id: source.interaction_id.clone(),
        source_sequence: source.sequence,
        source_event_ref: source.observed_event_ref.clone(),
        expected_entity_id: source.entity_id.clone(),
        target_kind: rule.target_kind,
        predicate: rule.predicate,
    })
}

/// Evaluate a later evaluator-observed interaction against the derived
/// constraint. This does not claim that a physical operation was successful;
/// it checks only the synthetic event semantics supplied to this fixture.
pub fn evaluate_follow_up(
    constraint: &EvaluatorDerivedConstraint,
    target: &InteractionEvent,
) -> Result<BceOutcome, BceError> {
    if target.interaction_id.trim().is_empty() {
        return Err(BceError::EmptyInteractionId);
    }
    if target.entity_id.trim().is_empty() {
        return Err(BceError::EmptyEntityId);
    }
    if target.observed_event_ref.trim().is_empty() {
        return Err(BceError::MissingObservedEventRef);
    }
    if target.interaction_id == constraint.source_interaction_id {
        return Err(BceError::DuplicateInteractionId(
            target.interaction_id.clone(),
        ));
    }
    if target.episode_id.trim().is_empty() {
        return Err(BceError::EmptyEpisodeId);
    }
    if target.episode_id != constraint.episode_id {
        return Err(BceError::TargetEpisodeMismatch {
            expected: constraint.episode_id.clone(),
            received: target.episode_id.clone(),
        });
    }
    if target.kind != constraint.target_kind {
        return Err(BceError::TargetKindMismatch {
            expected: constraint.target_kind,
            received: target.kind,
        });
    }
    if target.sequence <= constraint.source_sequence {
        return Err(BceError::TargetNotLater {
            source_sequence: constraint.source_sequence,
            target_sequence: target.sequence,
        });
    }

    match constraint.predicate {
        ConsistencyPredicate::SameEntityAsPriorInteraction
            if target.entity_id == constraint.expected_entity_id =>
        {
            Ok(BceOutcome::Consistent {
                episode_id: constraint.episode_id.clone(),
                source_interaction_id: constraint.source_interaction_id.clone(),
                target_interaction_id: target.interaction_id.clone(),
                entity_id: target.entity_id.clone(),
            })
        }
        ConsistencyPredicate::SameEntityAsPriorInteraction => Ok(BceOutcome::Inconsistent {
            episode_id: constraint.episode_id.clone(),
            source_interaction_id: constraint.source_interaction_id.clone(),
            target_interaction_id: target.interaction_id.clone(),
            expected_entity_id: constraint.expected_entity_id.clone(),
            actual_entity_id: target.entity_id.clone(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy_input() -> PolicyInput {
        PolicyInput {
            episode_id: "episode-1".into(),
            instruction: "Prepare the selected cup and later retrieve it".into(),
            current_observation_ref: "observation-current-9".into(),
            robot_state_ref: "robot-state-9".into(),
        }
    }

    fn event(
        interaction_id: &str,
        sequence: u64,
        kind: InteractionKind,
        entity_id: &str,
    ) -> InteractionEvent {
        InteractionEvent {
            episode_id: "episode-1".into(),
            interaction_id: interaction_id.into(),
            sequence,
            kind,
            entity_id: entity_id.into(),
            observed_event_ref: format!("synthetic-receipt-{interaction_id}"),
        }
    }

    fn rule() -> BceRule {
        BceRule {
            source_kind: InteractionKind::PriorChoice,
            target_kind: InteractionKind::FollowUpChoice,
            predicate: ConsistencyPredicate::SameEntityAsPriorInteraction,
        }
    }

    #[test]
    fn derives_constraint_from_actual_prior_choice_not_a_fixed_target() {
        let history_left = vec![event(
            "dose-left",
            1,
            InteractionKind::PriorChoice,
            "cup-left",
        )];
        let history_right = vec![event(
            "dose-right",
            1,
            InteractionKind::PriorChoice,
            "cup-right",
        )];

        let left = derive_constraint(&history_left, &rule()).unwrap();
        let right = derive_constraint(&history_right, &rule()).unwrap();

        assert_eq!(left.expected_entity_id, "cup-left");
        assert_eq!(right.expected_entity_id, "cup-right");
        assert_ne!(left.expected_entity_id, right.expected_entity_id);
    }

    #[test]
    fn same_policy_input_can_have_different_evaluator_constraints() {
        let input = policy_input();
        let left = derive_constraint(
            &[event("prior-left", 1, InteractionKind::PriorChoice, "left")],
            &rule(),
        )
        .unwrap();
        let right = derive_constraint(
            &[event(
                "prior-right",
                1,
                InteractionKind::PriorChoice,
                "right",
            )],
            &rule(),
        )
        .unwrap();

        // The policy input is unchanged and has no evaluator-target field.
        assert_eq!(input, policy_input());
        assert_eq!(left.episode_id, input.episode_id);
        assert_eq!(right.episode_id, input.episode_id);
        assert_ne!(left.expected_entity_id, right.expected_entity_id);
    }

    #[test]
    fn accepts_a_follow_up_consistent_with_the_agent_own_prior_choice() {
        let constraint = derive_constraint(
            &[event("prior", 2, InteractionKind::PriorChoice, "cup-a")],
            &rule(),
        )
        .unwrap();
        let follow_up = event("follow-up", 8, InteractionKind::FollowUpChoice, "cup-a");

        assert!(matches!(
            evaluate_follow_up(&constraint, &follow_up),
            Ok(BceOutcome::Consistent { .. })
        ));
    }

    #[test]
    fn rejects_a_follow_up_that_uses_a_different_entity() {
        let constraint = derive_constraint(
            &[event("prior", 2, InteractionKind::PriorChoice, "cup-a")],
            &rule(),
        )
        .unwrap();
        let follow_up = event("follow-up", 8, InteractionKind::FollowUpChoice, "cup-b");

        assert_eq!(
            evaluate_follow_up(&constraint, &follow_up),
            Ok(BceOutcome::Inconsistent {
                episode_id: "episode-1".into(),
                source_interaction_id: "prior".into(),
                target_interaction_id: "follow-up".into(),
                expected_entity_id: "cup-a".into(),
                actual_entity_id: "cup-b".into(),
            })
        );
    }

    #[test]
    fn irrelevant_history_does_not_change_the_derived_choice() {
        let original = vec![event("prior", 2, InteractionKind::PriorChoice, "cup-a")];
        let with_unrelated = vec![
            event("prior", 2, InteractionKind::PriorChoice, "cup-a"),
            event("unrelated", 5, InteractionKind::Unrelated, "drawer-3"),
        ];

        assert_eq!(
            derive_constraint(&original, &rule()).unwrap(),
            derive_constraint(&with_unrelated, &rule()).unwrap()
        );
    }

    #[test]
    fn latest_qualifying_prior_interaction_is_used() {
        let history = vec![
            event("prior-1", 1, InteractionKind::PriorChoice, "cup-a"),
            event("noise", 2, InteractionKind::Unrelated, "drawer-1"),
            event("prior-2", 3, InteractionKind::PriorChoice, "cup-b"),
        ];

        let constraint = derive_constraint(&history, &rule()).unwrap();
        assert_eq!(constraint.source_interaction_id, "prior-2");
        assert_eq!(constraint.expected_entity_id, "cup-b");
    }

    #[test]
    fn rejects_missing_prior_choice_instead_of_falling_back_to_fixed_target() {
        let history = vec![event("noise", 1, InteractionKind::Unrelated, "drawer-1")];

        assert_eq!(
            derive_constraint(&history, &rule()),
            Err(BceError::NoPriorInteraction(InteractionKind::PriorChoice))
        );
    }

    #[test]
    fn rejects_duplicate_or_replayed_interaction_identity() {
        let history = vec![
            event("prior", 1, InteractionKind::PriorChoice, "cup-a"),
            event("prior", 2, InteractionKind::PriorChoice, "cup-a"),
        ];

        assert_eq!(
            validate_interaction_stream(&history),
            Err(BceError::DuplicateInteractionId("prior".into()))
        );
    }

    #[test]
    fn rejects_reordered_or_duplicated_sequence_numbers() {
        let reordered = vec![
            event("later", 3, InteractionKind::PriorChoice, "cup-a"),
            event("earlier", 2, InteractionKind::Unrelated, "drawer-1"),
        ];
        assert_eq!(
            validate_interaction_stream(&reordered),
            Err(BceError::NonMonotonicSequence {
                previous: 3,
                received: 2,
            })
        );

        let duplicated = vec![
            event("prior", 3, InteractionKind::PriorChoice, "cup-a"),
            event("noise", 3, InteractionKind::Unrelated, "drawer-1"),
        ];
        assert_eq!(
            validate_interaction_stream(&duplicated),
            Err(BceError::DuplicateSequence(3))
        );
    }

    #[test]
    fn rejects_mixed_episodes_and_missing_evidence_references() {
        let mut mixed = event("prior", 1, InteractionKind::PriorChoice, "cup-a");
        let mut other = event("other", 2, InteractionKind::Unrelated, "drawer-1");
        other.episode_id = "episode-2".into();
        assert_eq!(
            validate_interaction_stream(&[mixed.clone(), other]),
            Err(BceError::MixedEpisode {
                expected: "episode-1".into(),
                received: "episode-2".into(),
            })
        );

        mixed.observed_event_ref.clear();
        assert_eq!(
            validate_interaction_stream(&[mixed]),
            Err(BceError::MissingObservedEventRef)
        );
    }

    #[test]
    fn follow_up_requires_unique_identity_and_observed_event_reference() {
        let constraint = derive_constraint(
            &[event("prior", 2, InteractionKind::PriorChoice, "cup-a")],
            &rule(),
        )
        .unwrap();

        let mut missing_id = event("follow-up", 8, InteractionKind::FollowUpChoice, "cup-a");
        missing_id.interaction_id.clear();
        assert_eq!(
            evaluate_follow_up(&constraint, &missing_id),
            Err(BceError::EmptyInteractionId)
        );

        let mut missing_entity = event("follow-up", 8, InteractionKind::FollowUpChoice, "cup-a");
        missing_entity.entity_id.clear();
        assert_eq!(
            evaluate_follow_up(&constraint, &missing_entity),
            Err(BceError::EmptyEntityId)
        );

        let mut missing_receipt = event("follow-up", 8, InteractionKind::FollowUpChoice, "cup-a");
        missing_receipt.observed_event_ref.clear();
        assert_eq!(
            evaluate_follow_up(&constraint, &missing_receipt),
            Err(BceError::MissingObservedEventRef)
        );

        let reused_source = event("prior", 8, InteractionKind::FollowUpChoice, "cup-a");
        assert_eq!(
            evaluate_follow_up(&constraint, &reused_source),
            Err(BceError::DuplicateInteractionId("prior".into()))
        );
    }

    #[test]
    fn rejects_wrong_episode_kind_and_non_later_follow_up() {
        let constraint = derive_constraint(
            &[event("prior", 2, InteractionKind::PriorChoice, "cup-a")],
            &rule(),
        )
        .unwrap();

        let mut wrong_episode = event("follow-up", 8, InteractionKind::FollowUpChoice, "cup-a");
        wrong_episode.episode_id = "episode-2".into();
        assert!(matches!(
            evaluate_follow_up(&constraint, &wrong_episode),
            Err(BceError::TargetEpisodeMismatch { .. })
        ));

        let wrong_kind = event("follow-up", 8, InteractionKind::Unrelated, "cup-a");
        assert!(matches!(
            evaluate_follow_up(&constraint, &wrong_kind),
            Err(BceError::TargetKindMismatch { .. })
        ));

        let earlier = event("follow-up", 1, InteractionKind::FollowUpChoice, "cup-a");
        assert_eq!(
            evaluate_follow_up(&constraint, &earlier),
            Err(BceError::TargetNotLater {
                source_sequence: 2,
                target_sequence: 1,
            })
        );
    }
}
