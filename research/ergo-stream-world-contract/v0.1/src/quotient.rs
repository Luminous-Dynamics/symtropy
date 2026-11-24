//! Congruence-checked quotient / coarse-graining for finite deterministic systems.
//!
//! A state partition is a valid deterministic quotient only when each
//! equivalence class is a congruence for every declared action: members must
//! agree on whether an action is enabled, and enabled members must transition
//! into the same quotient class.
//!
//! This is a finite behavioural abstraction of one system. It is not yet a
//! source-to-target transfer map, approximate quotient, probabilistic
//! abstraction, simulation or bisimulation.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::dynamics::{FiniteTransitionSystemV0, TransitionV0};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateClassMappingV0 {
    pub state_id: u32,
    pub class_id: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatePartitionV0 {
    pub mappings: Vec<StateClassMappingV0>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuotientTransitionV0 {
    pub from_class_id: u32,
    pub action_id: u32,
    pub to_class_id: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuotientSystemV0 {
    pub class_ids: Vec<u32>,
    pub action_ids: Vec<u32>,
    pub transitions: Vec<QuotientTransitionV0>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotientWitnessV0 {
    pub source_state_count: usize,
    pub quotient_class_count: usize,
    pub source_transition_count: usize,
    pub quotient_transition_count: usize,
    pub quotient: QuotientSystemV0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotientErrorV0 {
    EmptyStateSet,
    EmptyActionSet,
    DuplicateSourceStateId,
    DuplicateActionId,
    DuplicateStateMapping,
    UnknownStateMapping,
    PartitionIncomplete,
    UnknownTransitionState,
    UnknownTransitionAction,
    DuplicateTransition,
    NondeterministicTransition,
    ActionEnablementMismatch,
    SuccessorClassMismatch,
}

impl fmt::Display for QuotientErrorV0 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyStateSet => "finite transition system must contain at least one state",
            Self::EmptyActionSet => "finite transition system must contain at least one action",
            Self::DuplicateSourceStateId => "source state IDs must be unique",
            Self::DuplicateActionId => "action IDs must be unique",
            Self::DuplicateStateMapping => "state partition contains duplicate state mappings",
            Self::UnknownStateMapping => "state partition references an unknown state",
            Self::PartitionIncomplete => "state partition must cover every source state exactly once",
            Self::UnknownTransitionState => "transition references an unknown source or target state",
            Self::UnknownTransitionAction => "transition references an unknown action",
            Self::DuplicateTransition => "transition table contains a duplicate state/action row",
            Self::NondeterministicTransition => "source transition system is not deterministic",
            Self::ActionEnablementMismatch => {
                "states merged into one class disagree on whether an action is enabled"
            }
            Self::SuccessorClassMismatch => {
                "states merged into one class transition to different quotient classes"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for QuotientErrorV0 {}

/// Construct a quotient after validating that the supplied partition is a
/// congruence for every action. A missing transition is treated as an action
/// being disabled; enabledness must therefore agree across every class.
///
/// Source transitions are deterministic. Nondeterministic quotient semantics
/// require a separate contract and are not silently approximated here.
pub fn validate_and_build_quotient(
    source: &FiniteTransitionSystemV0,
    partition: &StatePartitionV0,
) -> Result<QuotientWitnessV0, QuotientErrorV0> {
    if source.states.is_empty() {
        return Err(QuotientErrorV0::EmptyStateSet);
    }
    if source.actions.is_empty() {
        return Err(QuotientErrorV0::EmptyActionSet);
    }
    if !unique_by(&source.states, |state| state.id) {
        return Err(QuotientErrorV0::DuplicateSourceStateId);
    }
    if !unique_by(&source.actions, |action| action.id) {
        return Err(QuotientErrorV0::DuplicateActionId);
    }

    if !unique_by(&partition.mappings, |mapping| mapping.state_id) {
        return Err(QuotientErrorV0::DuplicateStateMapping);
    }
    if partition.mappings.len() != source.states.len() {
        return Err(QuotientErrorV0::PartitionIncomplete);
    }

    let known_states = source
        .states
        .iter()
        .map(|state| state.id)
        .collect::<BTreeSet<_>>();
    for mapping in &partition.mappings {
        if !known_states.contains(&mapping.state_id) {
            return Err(QuotientErrorV0::UnknownStateMapping);
        }
    }

    let mut state_to_class = BTreeMap::new();
    let mut class_members = BTreeMap::<u32, Vec<u32>>::new();
    for mapping in &partition.mappings {
        state_to_class.insert(mapping.state_id, mapping.class_id);
        class_members
            .entry(mapping.class_id)
            .or_default()
            .push(mapping.state_id);
    }

    validate_source_transitions(source)?;

    let mut quotient_transitions = Vec::new();
    for (&class_id, member_states) in &class_members {
        for action in &source.actions {
            let mut expected_successor_class = None;
            let mut first_member = true;

            for &state_id in member_states {
                let successor = source
                    .transitions
                    .iter()
                    .find(|transition| {
                        transition.from_state_id == state_id
                            && transition.action_id == action.id
                    })
                    .map(|transition| {
                        state_to_class
                            .get(&transition.to_state_id)
                            .copied()
                            .ok_or(QuotientErrorV0::UnknownTransitionState)
                    })
                    .transpose()?;

                if first_member {
                    expected_successor_class = successor;
                    first_member = false;
                    continue;
                }

                match (expected_successor_class, successor) {
                    (None, None) => {}
                    (Some(_), None) | (None, Some(_)) => {
                        return Err(QuotientErrorV0::ActionEnablementMismatch);
                    }
                    (Some(expected), Some(actual)) if expected != actual => {
                        return Err(QuotientErrorV0::SuccessorClassMismatch);
                    }
                    (Some(_), Some(_)) => {}
                }
            }

            if let Some(to_class_id) = expected_successor_class {
                quotient_transitions.push(QuotientTransitionV0 {
                    from_class_id: class_id,
                    action_id: action.id,
                    to_class_id,
                });
            }
        }
    }

    quotient_transitions.sort_by_key(|transition| {
        (transition.from_class_id, transition.action_id, transition.to_class_id)
    });
    let class_ids = class_members.keys().copied().collect::<Vec<_>>();
    let action_ids = source
        .actions
        .iter()
        .map(|action| action.id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    Ok(QuotientWitnessV0 {
        source_state_count: source.states.len(),
        quotient_class_count: class_ids.len(),
        source_transition_count: source.transitions.len(),
        quotient_transition_count: quotient_transitions.len(),
        quotient: QuotientSystemV0 {
            class_ids,
            action_ids,
            transitions: quotient_transitions,
        },
    })
}

fn validate_source_transitions(
    source: &FiniteTransitionSystemV0,
) -> Result<(), QuotientErrorV0> {
    let mut seen = BTreeSet::new();
    for transition in &source.transitions {
        if !source
            .states
            .iter()
            .any(|state| state.id == transition.from_state_id)
            || !source
                .states
                .iter()
                .any(|state| state.id == transition.to_state_id)
        {
            return Err(QuotientErrorV0::UnknownTransitionState);
        }
        if !source
            .actions
            .iter()
            .any(|action| action.id == transition.action_id)
        {
            return Err(QuotientErrorV0::UnknownTransitionAction);
        }
        if !seen.insert((transition.from_state_id, transition.action_id)) {
            let duplicate_outcome = source.transitions.iter().filter(|candidate| {
                candidate.from_state_id == transition.from_state_id
                    && candidate.action_id == transition.action_id
            }).count() == 1;
            return Err(if duplicate_outcome {
                QuotientErrorV0::DuplicateTransition
            } else {
                QuotientErrorV0::NondeterministicTransition
            });
        }
    }
    Ok(())
}

fn unique_by<T>(values: &[T], id_of: impl Fn(&T) -> u32) -> bool {
    values.iter().enumerate().all(|(index, value)| {
        !values[index + 1..]
            .iter()
            .any(|later| id_of(value) == id_of(later))
    })
}

fn fixture() -> (FiniteTransitionSystemV0, StatePartitionV0) {
    use super::dynamics::{ActionV0, StateV0};

    let source = FiniteTransitionSystemV0 {
        states: vec![
            StateV0 { id: 1 },
            StateV0 { id: 2 },
            StateV0 { id: 3 },
        ],
        actions: vec![
            ActionV0 { id: 10, kind: 1 },
            ActionV0 { id: 20, kind: 2 },
        ],
        transitions: vec![
            TransitionV0 {
                from_state_id: 1,
                action_id: 10,
                to_state_id: 3,
            },
            TransitionV0 {
                from_state_id: 2,
                action_id: 10,
                to_state_id: 3,
            },
            TransitionV0 {
                from_state_id: 3,
                action_id: 10,
                to_state_id: 3,
            },
            TransitionV0 {
                from_state_id: 1,
                action_id: 20,
                to_state_id: 2,
            },
            TransitionV0 {
                from_state_id: 2,
                action_id: 20,
                to_state_id: 1,
            },
            TransitionV0 {
                from_state_id: 3,
                action_id: 20,
                to_state_id: 3,
            },
        ],
    };
    let partition = StatePartitionV0 {
        mappings: vec![
            StateClassMappingV0 {
                state_id: 1,
                class_id: 100,
            },
            StateClassMappingV0 {
                state_id: 2,
                class_id: 100,
            },
            StateClassMappingV0 {
                state_id: 3,
                class_id: 200,
            },
        ],
    };
    (source, partition)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_valid_nontrivial_finite_quotient() {
        let (source, partition) = fixture();
        let witness = validate_and_build_quotient(&source, &partition).unwrap();

        assert_eq!(witness.source_state_count, 3);
        assert_eq!(witness.quotient_class_count, 2);
        assert_eq!(witness.source_transition_count, 6);
        assert_eq!(witness.quotient_transition_count, 4);
        assert_eq!(
            witness.quotient.transitions,
            vec![
                QuotientTransitionV0 {
                    from_class_id: 100,
                    action_id: 10,
                    to_class_id: 200,
                },
                QuotientTransitionV0 {
                    from_class_id: 100,
                    action_id: 20,
                    to_class_id: 100,
                },
                QuotientTransitionV0 {
                    from_class_id: 200,
                    action_id: 10,
                    to_class_id: 200,
                },
                QuotientTransitionV0 {
                    from_class_id: 200,
                    action_id: 20,
                    to_class_id: 200,
                },
            ]
        );
    }

    #[test]
    fn rejects_partition_members_with_different_successor_classes() {
        let (mut source, partition) = fixture();
        source.transitions[1].to_state_id = 2;

        assert_eq!(
            validate_and_build_quotient(&source, &partition),
            Err(QuotientErrorV0::SuccessorClassMismatch)
        );
    }

    #[test]
    fn rejects_partition_members_with_different_action_enablement() {
        let (mut source, partition) = fixture();
        source.transitions.retain(|transition| {
            !(transition.from_state_id == 2 && transition.action_id == 10)
        });

        assert_eq!(
            validate_and_build_quotient(&source, &partition),
            Err(QuotientErrorV0::ActionEnablementMismatch)
        );
    }

    #[test]
    fn rejects_an_incomplete_state_partition() {
        let (source, mut partition) = fixture();
        partition.mappings.pop();

        assert_eq!(
            validate_and_build_quotient(&source, &partition),
            Err(QuotientErrorV0::PartitionIncomplete)
        );
    }

    #[test]
    fn rejects_nondeterministic_source_transitions() {
        let (mut source, partition) = fixture();
        source.transitions.push(TransitionV0 {
            from_state_id: 1,
            action_id: 10,
            to_state_id: 1,
        });

        assert_eq!(
            validate_and_build_quotient(&source, &partition),
            Err(QuotientErrorV0::NondeterministicTransition)
        );
    }

    #[test]
    fn rejects_unknown_state_in_partition() {
        let (source, mut partition) = fixture();
        partition.mappings[0].state_id = 99;

        assert_eq!(
            validate_and_build_quotient(&source, &partition),
            Err(QuotientErrorV0::UnknownStateMapping)
        );
    }

    #[test]
    fn every_abstract_transition_is_well_defined() {
        let (source, partition) = fixture();
        let witness = validate_and_build_quotient(&source, &partition).unwrap();

        for transition in &witness.quotient.transitions {
            assert!(witness
                .quotient
                .class_ids
                .contains(&transition.from_class_id));
            assert!(witness
                .quotient
                .class_ids
                .contains(&transition.to_class_id));
            assert!(witness.quotient.action_ids.contains(&transition.action_id));
        }
    }
}
