//! Finite action-conditioned dynamics correspondence for ErgoStream v0.1.
//!
//! This module checks whether a proposed state/action mapping commutes with
//! deterministic finite-state evolution:
//!
//!     phi(delta_source(state, action)) == delta_target(phi(state), psi(action))
//!
//! It is a finite transition-system witness, not a continuous-time model,
//! causal identification procedure, isolated evaluator or scientific
//! qualification authority. The correspondence must be proposed and frozen
//! before any hidden target transitions are disclosed to the learner.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateV0 {
    pub id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionV0 {
    pub id: u32,
    pub kind: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionV0 {
    pub from_state_id: u32,
    pub action_id: u32,
    pub to_state_id: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FiniteTransitionSystemV0 {
    pub states: Vec<StateV0>,
    pub actions: Vec<ActionV0>,
    pub transitions: Vec<TransitionV0>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateCorrespondenceV0 {
    pub source_state_id: u32,
    pub target_state_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicsActionCorrespondenceV0 {
    pub source_action_id: u32,
    pub target_action_id: u32,
    pub source_action_kind: u16,
    pub target_action_kind: u16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DynamicsCorrespondenceV0 {
    pub states: Vec<StateCorrespondenceV0>,
    pub actions: Vec<DynamicsActionCorrespondenceV0>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicsTransferWitnessV0 {
    pub mapped_states: usize,
    pub mapped_actions: usize,
    pub mapped_action_kinds: usize,
    pub checked_transitions: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicsTransferErrorV0 {
    DuplicateSourceStateId,
    DuplicateTargetStateId,
    DuplicateSourceActionId,
    DuplicateTargetActionId,
    UnknownSourceTransitionState,
    UnknownTargetTransitionState,
    UnknownSourceTransitionAction,
    UnknownTargetTransitionAction,
    NonDeterministicSourceSystem,
    NonDeterministicTargetSystem,
    StateMapIncomplete,
    StateMapNotInjective,
    UnknownSourceStateMapping,
    UnknownTargetStateMapping,
    ActionMapIncomplete,
    ActionMapNotInjective,
    UnknownSourceActionMapping,
    UnknownTargetActionMapping,
    ActionKindMapNotInjective,
    InconsistentActionKindMapping,
    TransitionCountMismatch,
    TransitionMismatch,
}

impl fmt::Display for DynamicsTransferErrorV0 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::DuplicateSourceStateId => "source state IDs must be unique",
            Self::DuplicateTargetStateId => "target state IDs must be unique",
            Self::DuplicateSourceActionId => "source action IDs must be unique",
            Self::DuplicateTargetActionId => "target action IDs must be unique",
            Self::UnknownSourceTransitionState => "source transition references an unknown state",
            Self::UnknownTargetTransitionState => "target transition references an unknown state",
            Self::UnknownSourceTransitionAction => "source transition references an unknown action",
            Self::UnknownTargetTransitionAction => "target transition references an unknown action",
            Self::NonDeterministicSourceSystem => "source system has multiple outcomes for one state/action pair",
            Self::NonDeterministicTargetSystem => "target system has multiple outcomes for one state/action pair",
            Self::StateMapIncomplete => "state correspondence must cover both systems",
            Self::StateMapNotInjective => "state correspondence must be one-to-one",
            Self::UnknownSourceStateMapping => "state correspondence references an unknown source state",
            Self::UnknownTargetStateMapping => "state correspondence references an unknown target state",
            Self::ActionMapIncomplete => "action correspondence must cover both systems",
            Self::ActionMapNotInjective => "action correspondence must be one-to-one",
            Self::UnknownSourceActionMapping => "action correspondence references an unknown source action",
            Self::UnknownTargetActionMapping => "action correspondence references an unknown target action",
            Self::ActionKindMapNotInjective => "distinct source action kinds must map to distinct target action kinds",
            Self::InconsistentActionKindMapping => "one source action kind maps to inconsistent target kinds",
            Self::TransitionCountMismatch => "source and target systems have different transition counts",
            Self::TransitionMismatch => "mapped state/action transition is absent or has a different outcome",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for DynamicsTransferErrorV0 {}

/// Validate a total, one-to-one correspondence between two deterministic
/// finite-state systems. All in-scope transitions must commute under the
/// supplied state and action maps. Unmapped or additional transitions fail
/// closed; partial-domain simulation is reserved for a later scoped contract.
pub fn validate_dynamics_correspondence(
    source: &FiniteTransitionSystemV0,
    target: &FiniteTransitionSystemV0,
    correspondence: &DynamicsCorrespondenceV0,
) -> Result<DynamicsTransferWitnessV0, DynamicsTransferErrorV0> {
    validate_system(source, true)?;
    validate_system(target, false)?;

    if correspondence.states.len() != source.states.len()
        || correspondence.states.len() != target.states.len()
    {
        return Err(DynamicsTransferErrorV0::StateMapIncomplete);
    }

    if !is_unique(&correspondence.states, |mapping| mapping.source_state_id)
        || !is_unique(&correspondence.states, |mapping| mapping.target_state_id)
    {
        return Err(DynamicsTransferErrorV0::StateMapNotInjective);
    }

    for mapping in &correspondence.states {
        if !source
            .states
            .iter()
            .any(|state| state.id == mapping.source_state_id)
        {
            return Err(DynamicsTransferErrorV0::UnknownSourceStateMapping);
        }
        if !target
            .states
            .iter()
            .any(|state| state.id == mapping.target_state_id)
        {
            return Err(DynamicsTransferErrorV0::UnknownTargetStateMapping);
        }
    }

    if correspondence.actions.len() != source.actions.len()
        || correspondence.actions.len() != target.actions.len()
    {
        return Err(DynamicsTransferErrorV0::ActionMapIncomplete);
    }

    if !is_unique(&correspondence.actions, |mapping| mapping.source_action_id)
        || !is_unique(&correspondence.actions, |mapping| mapping.target_action_id)
    {
        return Err(DynamicsTransferErrorV0::ActionMapNotInjective);
    }

    let mut action_kind_map = Vec::<(u16, u16)>::new();
    for mapping in &correspondence.actions {
        let source_action = source
            .actions
            .iter()
            .find(|action| action.id == mapping.source_action_id)
            .ok_or(DynamicsTransferErrorV0::UnknownSourceActionMapping)?;
        let target_action = target
            .actions
            .iter()
            .find(|action| action.id == mapping.target_action_id)
            .ok_or(DynamicsTransferErrorV0::UnknownTargetActionMapping)?;

        if source_action.kind != mapping.source_action_kind
            || target_action.kind != mapping.target_action_kind
        {
            return Err(DynamicsTransferErrorV0::InconsistentActionKindMapping);
        }

        match action_kind_map
            .iter()
            .find(|(source_kind, _)| *source_kind == mapping.source_action_kind)
        {
            Some((_, target_kind)) if *target_kind != mapping.target_action_kind => {
                return Err(DynamicsTransferErrorV0::InconsistentActionKindMapping);
            }
            Some(_) => {}
            None => {
                if action_kind_map
                    .iter()
                    .any(|(_, target_kind)| *target_kind == mapping.target_action_kind)
                {
                    return Err(DynamicsTransferErrorV0::ActionKindMapNotInjective);
                }
                action_kind_map.push((
                    mapping.source_action_kind,
                    mapping.target_action_kind,
                ));
            }
        }
    }

    if source.transitions.len() != target.transitions.len() {
        return Err(DynamicsTransferErrorV0::TransitionCountMismatch);
    }

    for transition in &source.transitions {
        let mapped_from = correspondence
            .states
            .iter()
            .find(|mapping| mapping.source_state_id == transition.from_state_id)
            .map(|mapping| mapping.target_state_id)
            .ok_or(DynamicsTransferErrorV0::UnknownSourceStateMapping)?;
        let mapped_action = correspondence
            .actions
            .iter()
            .find(|mapping| mapping.source_action_id == transition.action_id)
            .map(|mapping| mapping.target_action_id)
            .ok_or(DynamicsTransferErrorV0::UnknownSourceActionMapping)?;
        let mapped_to = correspondence
            .states
            .iter()
            .find(|mapping| mapping.source_state_id == transition.to_state_id)
            .map(|mapping| mapping.target_state_id)
            .ok_or(DynamicsTransferErrorV0::UnknownSourceStateMapping)?;

        if !target.transitions.iter().any(|candidate| {
            candidate.from_state_id == mapped_from
                && candidate.action_id == mapped_action
                && candidate.to_state_id == mapped_to
        }) {
            return Err(DynamicsTransferErrorV0::TransitionMismatch);
        }
    }

    Ok(DynamicsTransferWitnessV0 {
        mapped_states: correspondence.states.len(),
        mapped_actions: correspondence.actions.len(),
        mapped_action_kinds: action_kind_map.len(),
        checked_transitions: source.transitions.len(),
    })
}

fn validate_system(
    system: &FiniteTransitionSystemV0,
    is_source: bool,
) -> Result<(), DynamicsTransferErrorV0> {
    if !is_unique(&system.states, |state| state.id) {
        return Err(if is_source {
            DynamicsTransferErrorV0::DuplicateSourceStateId
        } else {
            DynamicsTransferErrorV0::DuplicateTargetStateId
        });
    }

    if !is_unique(&system.actions, |action| action.id) {
        return Err(if is_source {
            DynamicsTransferErrorV0::DuplicateSourceActionId
        } else {
            DynamicsTransferErrorV0::DuplicateTargetActionId
        });
    }

    for transition in &system.transitions {
        if !system
            .states
            .iter()
            .any(|state| state.id == transition.from_state_id)
            || !system
                .states
                .iter()
                .any(|state| state.id == transition.to_state_id)
        {
            return Err(if is_source {
                DynamicsTransferErrorV0::UnknownSourceTransitionState
            } else {
                DynamicsTransferErrorV0::UnknownTargetTransitionState
            });
        }

        if !system
            .actions
            .iter()
            .any(|action| action.id == transition.action_id)
        {
            return Err(if is_source {
                DynamicsTransferErrorV0::UnknownSourceTransitionAction
            } else {
                DynamicsTransferErrorV0::UnknownTargetTransitionAction
            });
        }
    }

    for (index, transition) in system.transitions.iter().enumerate() {
        if system.transitions[index + 1..].iter().any(|later| {
            later.from_state_id == transition.from_state_id
                && later.action_id == transition.action_id
        }) {
            return Err(if is_source {
                DynamicsTransferErrorV0::NonDeterministicSourceSystem
            } else {
                DynamicsTransferErrorV0::NonDeterministicTargetSystem
            });
        }
    }

    Ok(())
}

fn is_unique<T>(values: &[T], id_of: impl Fn(&T) -> u32) -> bool {
    values.iter().enumerate().all(|(index, value)| {
        !values[index + 1..]
            .iter()
            .any(|later| id_of(value) == id_of(later))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (
        FiniteTransitionSystemV0,
        FiniteTransitionSystemV0,
        DynamicsCorrespondenceV0,
    ) {
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
                    to_state_id: 2,
                },
                TransitionV0 {
                    from_state_id: 2,
                    action_id: 10,
                    to_state_id: 3,
                },
                TransitionV0 {
                    from_state_id: 1,
                    action_id: 20,
                    to_state_id: 3,
                },
                TransitionV0 {
                    from_state_id: 3,
                    action_id: 20,
                    to_state_id: 1,
                },
            ],
        };

        let target = FiniteTransitionSystemV0 {
            states: vec![
                StateV0 { id: 11 },
                StateV0 { id: 12 },
                StateV0 { id: 13 },
            ],
            actions: vec![
                ActionV0 { id: 110, kind: 100 },
                ActionV0 { id: 120, kind: 200 },
            ],
            transitions: vec![
                TransitionV0 {
                    from_state_id: 11,
                    action_id: 110,
                    to_state_id: 12,
                },
                TransitionV0 {
                    from_state_id: 12,
                    action_id: 110,
                    to_state_id: 13,
                },
                TransitionV0 {
                    from_state_id: 11,
                    action_id: 120,
                    to_state_id: 13,
                },
                TransitionV0 {
                    from_state_id: 13,
                    action_id: 120,
                    to_state_id: 11,
                },
            ],
        };

        let correspondence = DynamicsCorrespondenceV0 {
            states: vec![
                StateCorrespondenceV0 {
                    source_state_id: 1,
                    target_state_id: 11,
                },
                StateCorrespondenceV0 {
                    source_state_id: 2,
                    target_state_id: 12,
                },
                StateCorrespondenceV0 {
                    source_state_id: 3,
                    target_state_id: 13,
                },
            ],
            actions: vec![
                DynamicsActionCorrespondenceV0 {
                    source_action_id: 10,
                    target_action_id: 110,
                    source_action_kind: 1,
                    target_action_kind: 100,
                },
                DynamicsActionCorrespondenceV0 {
                    source_action_id: 20,
                    target_action_id: 120,
                    source_action_kind: 2,
                    target_action_kind: 200,
                },
            ],
        };

        (source, target, correspondence)
    }

    #[test]
    fn accepts_relabelled_dynamics_that_commute_under_state_action_mapping() {
        let (source, target, correspondence) = fixture();
        let witness = validate_dynamics_correspondence(&source, &target, &correspondence)
            .expect("fixture has a valid deterministic dynamics correspondence");

        assert_eq!(witness.mapped_states, 3);
        assert_eq!(witness.mapped_actions, 2);
        assert_eq!(witness.mapped_action_kinds, 2);
        assert_eq!(witness.checked_transitions, 4);
    }

    #[test]
    fn rejects_a_single_transition_counterexample() {
        let (source, mut target, correspondence) = fixture();
        target.transitions[3].to_state_id = 12;

        assert_eq!(
            validate_dynamics_correspondence(&source, &target, &correspondence),
            Err(DynamicsTransferErrorV0::TransitionMismatch)
        );
    }

    #[test]
    fn rejects_non_injective_state_mapping() {
        let (source, target, mut correspondence) = fixture();
        correspondence.states[2].target_state_id = 12;

        assert_eq!(
            validate_dynamics_correspondence(&source, &target, &correspondence),
            Err(DynamicsTransferErrorV0::StateMapNotInjective)
        );
    }

    #[test]
    fn rejects_action_kind_mapping_collision() {
        let (source, mut target, mut correspondence) = fixture();
        target.actions[1].kind = 100;
        correspondence.actions[1].target_action_kind = 100;

        assert_eq!(
            validate_dynamics_correspondence(&source, &target, &correspondence),
            Err(DynamicsTransferErrorV0::ActionKindMapNotInjective)
        );
    }

    #[test]
    fn rejects_a_nondeterministic_source_system() {
        let (mut source, target, correspondence) = fixture();
        source.transitions.push(TransitionV0 {
            from_state_id: 1,
            action_id: 10,
            to_state_id: 3,
        });

        assert_eq!(
            validate_dynamics_correspondence(&source, &target, &correspondence),
            Err(DynamicsTransferErrorV0::NonDeterministicSourceSystem)
        );
    }

    #[test]
    fn rejects_incomplete_state_mapping() {
        let (source, target, mut correspondence) = fixture();
        correspondence.states.pop();

        assert_eq!(
            validate_dynamics_correspondence(&source, &target, &correspondence),
            Err(DynamicsTransferErrorV0::StateMapIncomplete)
        );
    }

    #[test]
    fn rejects_incomplete_action_mapping() {
        let (source, target, mut correspondence) = fixture();
        correspondence.actions.pop();

        assert_eq!(
            validate_dynamics_correspondence(&source, &target, &correspondence),
            Err(DynamicsTransferErrorV0::ActionMapIncomplete)
        );
    }

    #[test]
    fn rejects_a_missing_target_transition() {
        let (source, mut target, correspondence) = fixture();
        target.transitions.pop();

        assert_eq!(
            validate_dynamics_correspondence(&source, &target, &correspondence),
            Err(DynamicsTransferErrorV0::TransitionCountMismatch)
        );
    }
}
