//! Finite simulation and bisimulation witnesses over labelled transition systems.
//!
//! Unlike the deterministic one-to-one dynamics correspondence, this module
//! supports nondeterministic transitions and many-to-many state relations.
//! Simulation is one-way: every scoped source transition must be matchable in
//! the target, while additional target behaviours are allowed. Bisimulation
//! additionally requires every scoped target transition to be matched back.
//!
//! This module validates a proposed relation; it does not discover it from raw
//! observations or establish causal-mechanism identity.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LtsStateV0 {
    pub id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LtsActionV0 {
    pub id: u32,
    pub kind: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LtsTransitionV0 {
    pub from_state_id: u32,
    pub action_id: u32,
    pub to_state_id: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LabelledTransitionSystemV0 {
    pub states: Vec<LtsStateV0>,
    pub actions: Vec<LtsActionV0>,
    pub transitions: Vec<LtsTransitionV0>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BehaviouralStatePairV0 {
    pub source_state_id: u32,
    pub target_state_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BehaviouralActionMappingV0 {
    pub source_action_id: u32,
    pub target_action_id: u32,
    pub source_action_kind: u16,
    pub target_action_kind: u16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BehaviouralScopeV0 {
    pub source_state_ids: Vec<u32>,
    pub target_state_ids: Vec<u32>,
    pub source_action_ids: Vec<u32>,
    pub target_action_ids: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviouralMapKindV0 {
    Simulation,
    Bisimulation,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BehaviouralCorrespondenceV0 {
    pub scope: BehaviouralScopeV0,
    /// A relation, not necessarily a function: one state may relate to many
    /// states on the other side, and vice versa.
    pub state_relation: Vec<BehaviouralStatePairV0>,
    pub action_mappings: Vec<BehaviouralActionMappingV0>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviouralTransferWitnessV0 {
    pub map_kind: BehaviouralMapKindV0,
    pub related_state_pairs: usize,
    pub mapped_actions: usize,
    pub mapped_action_kinds: usize,
    pub checked_source_transitions: usize,
    pub checked_target_transitions: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviouralTransferErrorV0 {
    EmptySourceStates,
    EmptyTargetStates,
    EmptySourceActions,
    EmptyTargetActions,
    DuplicateSourceStateId,
    DuplicateTargetStateId,
    DuplicateSourceActionId,
    DuplicateTargetActionId,
    UnknownSourceTransitionState,
    UnknownTargetTransitionState,
    UnknownSourceTransitionAction,
    UnknownTargetTransitionAction,
    DuplicateTransition,
    DuplicateScopeId,
    UnknownScopeId,
    TransitionLeavesScope,
    EmptyStateRelation,
    DuplicateStatePair,
    UnknownSourceRelationState,
    UnknownTargetRelationState,
    SourceStateNotRelated,
    TargetStateNotRelated,
    IncompleteActionMap,
    DuplicateSourceActionMapping,
    DuplicateTargetActionMapping,
    UnknownSourceActionMapping,
    UnknownTargetActionMapping,
    ActionKindMismatch,
    ActionKindMapNotInjective,
    InconsistentActionKindMapping,
    BisimulationActionMapNotSurjective,
    MissingTargetResponse,
    MissingSourceResponse,
}

impl fmt::Display for BehaviouralTransferErrorV0 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptySourceStates => "source transition system has no states",
            Self::EmptyTargetStates => "target transition system has no states",
            Self::EmptySourceActions => "source transition system has no actions",
            Self::EmptyTargetActions => "target transition system has no actions",
            Self::DuplicateSourceStateId => "source state IDs must be unique",
            Self::DuplicateTargetStateId => "target state IDs must be unique",
            Self::DuplicateSourceActionId => "source action IDs must be unique",
            Self::DuplicateTargetActionId => "target action IDs must be unique",
            Self::UnknownSourceTransitionState => "source transition references an unknown state",
            Self::UnknownTargetTransitionState => "target transition references an unknown state",
            Self::UnknownSourceTransitionAction => "source transition references an unknown action",
            Self::UnknownTargetTransitionAction => "target transition references an unknown action",
            Self::DuplicateTransition => "transition table contains a duplicate labelled edge",
            Self::DuplicateScopeId => "scope contains duplicate state or action IDs",
            Self::UnknownScopeId => "scope references an unknown state or action",
            Self::TransitionLeavesScope => {
                "an in-scope transition leaves the declared state/action scope"
            }
            Self::EmptyStateRelation => "behavioural state relation must not be empty",
            Self::DuplicateStatePair => "behavioural state relation contains a duplicate pair",
            Self::UnknownSourceRelationState => "state relation references an unknown source state",
            Self::UnknownTargetRelationState => "state relation references an unknown target state",
            Self::SourceStateNotRelated => {
                "every source scope state must appear in the state relation"
            }
            Self::TargetStateNotRelated => {
                "every target scope state must appear in the state relation"
            }
            Self::IncompleteActionMap => "every source scope action must have an action mapping",
            Self::DuplicateSourceActionMapping => "source action mapping must be one-to-one",
            Self::DuplicateTargetActionMapping => "target action mapping must be one-to-one",
            Self::UnknownSourceActionMapping => {
                "action mapping references an unknown source action"
            }
            Self::UnknownTargetActionMapping => {
                "action mapping references an unknown target action"
            }
            Self::ActionKindMismatch => {
                "declared action kinds do not match their source/target actions"
            }
            Self::ActionKindMapNotInjective => {
                "distinct source action kinds map to one target kind"
            }
            Self::InconsistentActionKindMapping => {
                "one source action kind maps to inconsistent target kinds"
            }
            Self::BisimulationActionMapNotSurjective => {
                "bisimulation action mapping must cover the complete target action scope"
            }
            Self::MissingTargetResponse => {
                "a source transition has no related target transition response"
            }
            Self::MissingSourceResponse => {
                "a target transition has no related source transition response"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for BehaviouralTransferErrorV0 {}

/// Validate a finite simulation or bisimulation over declared scopes.
///
/// Every state in each declared scope must occur in at least one pair of the
/// many-to-many state relation. Every source action must map injectively to a
/// target action. Bisimulation additionally requires the action map to cover
/// the whole target action scope and checks transitions in both directions.
pub fn validate_behavioural_correspondence(
    source: &LabelledTransitionSystemV0,
    target: &LabelledTransitionSystemV0,
    map_kind: BehaviouralMapKindV0,
    candidate: &BehaviouralCorrespondenceV0,
) -> Result<BehaviouralTransferWitnessV0, BehaviouralTransferErrorV0> {
    validate_system(source, true)?;
    validate_system(target, false)?;
    validate_scope(source, target, &candidate.scope)?;
    validate_scope_transitions(
        source,
        &candidate.scope.source_state_ids,
        &candidate.scope.source_action_ids,
    )?;
    validate_scope_transitions(
        target,
        &candidate.scope.target_state_ids,
        &candidate.scope.target_action_ids,
    )?;
    validate_state_relation(source, target, &candidate.scope, &candidate.state_relation)?;
    let action_maps = validate_action_mappings(
        source,
        target,
        map_kind,
        &candidate.scope,
        &candidate.action_mappings,
    )?;

    let mut checked_source_transitions = 0;
    let mut checked_target_transitions = 0;

    for pair in &candidate.state_relation {
        let source_transitions = source.transitions.iter().filter(|transition| {
            transition.from_state_id == pair.source_state_id
                && candidate
                    .scope
                    .source_action_ids
                    .contains(&transition.action_id)
        });
        for source_transition in source_transitions {
            checked_source_transitions += 1;
            let target_action = action_maps
                .get(&source_transition.action_id)
                .copied()
                .ok_or(BehaviouralTransferErrorV0::IncompleteActionMap)?;

            let matched = target.transitions.iter().any(|target_transition| {
                target_transition.from_state_id == pair.target_state_id
                    && target_transition.action_id == target_action
                    && candidate.state_relation.iter().any(|next_pair| {
                        next_pair.source_state_id == source_transition.to_state_id
                            && next_pair.target_state_id == target_transition.to_state_id
                    })
            });
            if !matched {
                return Err(BehaviouralTransferErrorV0::MissingTargetResponse);
            }
        }
    }

    if map_kind == BehaviouralMapKindV0::Bisimulation {
        let reverse_action_maps = action_maps
            .iter()
            .map(|(source_action, target_action)| (*target_action, *source_action))
            .collect::<BTreeMap<_, _>>();

        for pair in &candidate.state_relation {
            let target_transitions = target.transitions.iter().filter(|transition| {
                transition.from_state_id == pair.target_state_id
                    && candidate
                        .scope
                        .target_action_ids
                        .contains(&transition.action_id)
            });
            for target_transition in target_transitions {
                checked_target_transitions += 1;
                let source_action = reverse_action_maps
                    .get(&target_transition.action_id)
                    .copied()
                    .ok_or(BehaviouralTransferErrorV0::BisimulationActionMapNotSurjective)?;

                let matched = source.transitions.iter().any(|source_transition| {
                    source_transition.from_state_id == pair.source_state_id
                        && source_transition.action_id == source_action
                        && candidate.state_relation.iter().any(|next_pair| {
                            next_pair.source_state_id == source_transition.to_state_id
                                && next_pair.target_state_id == target_transition.to_state_id
                        })
                });
                if !matched {
                    return Err(BehaviouralTransferErrorV0::MissingSourceResponse);
                }
            }
        }
    }

    let action_kinds = candidate
        .action_mappings
        .iter()
        .map(|mapping| (mapping.source_action_kind, mapping.target_action_kind))
        .collect::<BTreeSet<_>>();

    Ok(BehaviouralTransferWitnessV0 {
        map_kind,
        related_state_pairs: candidate.state_relation.len(),
        mapped_actions: candidate.action_mappings.len(),
        mapped_action_kinds: action_kinds.len(),
        checked_source_transitions,
        checked_target_transitions,
    })
}

fn validate_system(
    system: &LabelledTransitionSystemV0,
    is_source: bool,
) -> Result<(), BehaviouralTransferErrorV0> {
    if system.states.is_empty() {
        return Err(if is_source {
            BehaviouralTransferErrorV0::EmptySourceStates
        } else {
            BehaviouralTransferErrorV0::EmptyTargetStates
        });
    }
    if system.actions.is_empty() {
        return Err(if is_source {
            BehaviouralTransferErrorV0::EmptySourceActions
        } else {
            BehaviouralTransferErrorV0::EmptyTargetActions
        });
    }

    if !unique_by(&system.states, |state| state.id) {
        return Err(if is_source {
            BehaviouralTransferErrorV0::DuplicateSourceStateId
        } else {
            BehaviouralTransferErrorV0::DuplicateTargetStateId
        });
    }
    if !unique_by(&system.actions, |action| action.id) {
        return Err(if is_source {
            BehaviouralTransferErrorV0::DuplicateSourceActionId
        } else {
            BehaviouralTransferErrorV0::DuplicateTargetActionId
        });
    }

    let mut seen_edges = BTreeSet::new();
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
                BehaviouralTransferErrorV0::UnknownSourceTransitionState
            } else {
                BehaviouralTransferErrorV0::UnknownTargetTransitionState
            });
        }
        if !system
            .actions
            .iter()
            .any(|action| action.id == transition.action_id)
        {
            return Err(if is_source {
                BehaviouralTransferErrorV0::UnknownSourceTransitionAction
            } else {
                BehaviouralTransferErrorV0::UnknownTargetTransitionAction
            });
        }
        if !seen_edges.insert((
            transition.from_state_id,
            transition.action_id,
            transition.to_state_id,
        )) {
            return Err(BehaviouralTransferErrorV0::DuplicateTransition);
        }
    }

    Ok(())
}

fn validate_scope(
    source: &LabelledTransitionSystemV0,
    target: &LabelledTransitionSystemV0,
    scope: &BehaviouralScopeV0,
) -> Result<(), BehaviouralTransferErrorV0> {
    if !unique_ids(&scope.source_state_ids)
        || !unique_ids(&scope.target_state_ids)
        || !unique_ids(&scope.source_action_ids)
        || !unique_ids(&scope.target_action_ids)
    {
        return Err(BehaviouralTransferErrorV0::DuplicateScopeId);
    }

    if scope.source_state_ids.is_empty()
        || scope.target_state_ids.is_empty()
        || scope.source_action_ids.is_empty()
        || scope.target_action_ids.is_empty()
    {
        return Err(BehaviouralTransferErrorV0::UnknownScopeId);
    }

    if scope
        .source_state_ids
        .iter()
        .any(|id| !source.states.iter().any(|state| state.id == *id))
        || scope
            .target_state_ids
            .iter()
            .any(|id| !target.states.iter().any(|state| state.id == *id))
        || scope
            .source_action_ids
            .iter()
            .any(|id| !source.actions.iter().any(|action| action.id == *id))
        || scope
            .target_action_ids
            .iter()
            .any(|id| !target.actions.iter().any(|action| action.id == *id))
    {
        return Err(BehaviouralTransferErrorV0::UnknownScopeId);
    }

    Ok(())
}

fn validate_scope_transitions(
    system: &LabelledTransitionSystemV0,
    state_scope: &[u32],
    action_scope: &[u32],
) -> Result<(), BehaviouralTransferErrorV0> {
    if system.transitions.iter().any(|transition| {
        state_scope.contains(&transition.from_state_id)
            && action_scope.contains(&transition.action_id)
            && !state_scope.contains(&transition.to_state_id)
    }) {
        return Err(BehaviouralTransferErrorV0::TransitionLeavesScope);
    }
    Ok(())
}

fn validate_state_relation(
    source: &LabelledTransitionSystemV0,
    target: &LabelledTransitionSystemV0,
    scope: &BehaviouralScopeV0,
    relation: &[BehaviouralStatePairV0],
) -> Result<(), BehaviouralTransferErrorV0> {
    if relation.is_empty() {
        return Err(BehaviouralTransferErrorV0::EmptyStateRelation);
    }

    let mut pairs = BTreeSet::new();
    let mut related_source = BTreeSet::new();
    let mut related_target = BTreeSet::new();
    for pair in relation {
        if !pairs.insert((pair.source_state_id, pair.target_state_id)) {
            return Err(BehaviouralTransferErrorV0::DuplicateStatePair);
        }
        if !scope.source_state_ids.contains(&pair.source_state_id)
            || !source
                .states
                .iter()
                .any(|state| state.id == pair.source_state_id)
        {
            return Err(BehaviouralTransferErrorV0::UnknownSourceRelationState);
        }
        if !scope.target_state_ids.contains(&pair.target_state_id)
            || !target
                .states
                .iter()
                .any(|state| state.id == pair.target_state_id)
        {
            return Err(BehaviouralTransferErrorV0::UnknownTargetRelationState);
        }
        related_source.insert(pair.source_state_id);
        related_target.insert(pair.target_state_id);
    }

    if scope
        .source_state_ids
        .iter()
        .any(|state_id| !related_source.contains(state_id))
    {
        return Err(BehaviouralTransferErrorV0::SourceStateNotRelated);
    }
    if scope
        .target_state_ids
        .iter()
        .any(|state_id| !related_target.contains(state_id))
    {
        return Err(BehaviouralTransferErrorV0::TargetStateNotRelated);
    }

    Ok(())
}

fn validate_action_mappings(
    source: &LabelledTransitionSystemV0,
    target: &LabelledTransitionSystemV0,
    map_kind: BehaviouralMapKindV0,
    scope: &BehaviouralScopeV0,
    mappings: &[BehaviouralActionMappingV0],
) -> Result<BTreeMap<u32, u32>, BehaviouralTransferErrorV0> {
    if mappings.len() != scope.source_action_ids.len() {
        return Err(BehaviouralTransferErrorV0::IncompleteActionMap);
    }
    if !unique_by(mappings, |mapping| mapping.source_action_id) {
        return Err(BehaviouralTransferErrorV0::DuplicateSourceActionMapping);
    }
    if !unique_by(mappings, |mapping| mapping.target_action_id) {
        return Err(BehaviouralTransferErrorV0::DuplicateTargetActionMapping);
    }

    let mut kind_map = BTreeMap::<u16, u16>::new();
    let mut reverse_kind_map = BTreeMap::<u16, u16>::new();
    let mut action_map = BTreeMap::new();
    for mapping in mappings {
        if !scope.source_action_ids.contains(&mapping.source_action_id) {
            return Err(BehaviouralTransferErrorV0::UnknownSourceActionMapping);
        }
        if !scope.target_action_ids.contains(&mapping.target_action_id) {
            return Err(BehaviouralTransferErrorV0::UnknownTargetActionMapping);
        }
        let source_action = source
            .actions
            .iter()
            .find(|action| action.id == mapping.source_action_id)
            .ok_or(BehaviouralTransferErrorV0::UnknownSourceActionMapping)?;
        let target_action = target
            .actions
            .iter()
            .find(|action| action.id == mapping.target_action_id)
            .ok_or(BehaviouralTransferErrorV0::UnknownTargetActionMapping)?;
        if source_action.kind != mapping.source_action_kind
            || target_action.kind != mapping.target_action_kind
        {
            return Err(BehaviouralTransferErrorV0::ActionKindMismatch);
        }

        match kind_map.get(&mapping.source_action_kind) {
            Some(target_kind) if *target_kind != mapping.target_action_kind => {
                return Err(BehaviouralTransferErrorV0::InconsistentActionKindMapping);
            }
            Some(_) => {}
            None => {
                if reverse_kind_map
                    .get(&mapping.target_action_kind)
                    .is_some_and(|source_kind| *source_kind != mapping.source_action_kind)
                {
                    return Err(BehaviouralTransferErrorV0::ActionKindMapNotInjective);
                }
                kind_map.insert(mapping.source_action_kind, mapping.target_action_kind);
                reverse_kind_map.insert(mapping.target_action_kind, mapping.source_action_kind);
            }
        }

        action_map.insert(mapping.source_action_id, mapping.target_action_id);
    }

    if scope
        .source_action_ids
        .iter()
        .any(|action_id| !action_map.contains_key(action_id))
    {
        return Err(BehaviouralTransferErrorV0::IncompleteActionMap);
    }

    if map_kind == BehaviouralMapKindV0::Bisimulation
        && action_map.len() != scope.target_action_ids.len()
    {
        return Err(BehaviouralTransferErrorV0::BisimulationActionMapNotSurjective);
    }

    Ok(action_map)
}

fn unique_by<T>(values: &[T], id_of: impl Fn(&T) -> u32) -> bool {
    values.iter().enumerate().all(|(index, value)| {
        !values[index + 1..]
            .iter()
            .any(|later| id_of(value) == id_of(later))
    })
}

fn unique_ids(ids: &[u32]) -> bool {
    ids.iter()
        .enumerate()
        .all(|(index, id)| !ids[index + 1..].contains(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> LabelledTransitionSystemV0 {
        LabelledTransitionSystemV0 {
            states: vec![LtsStateV0 { id: 1 }, LtsStateV0 { id: 2 }],
            actions: vec![LtsActionV0 { id: 10, kind: 1 }],
            transitions: vec![LtsTransitionV0 {
                from_state_id: 1,
                action_id: 10,
                to_state_id: 2,
            }],
        }
    }

    fn target_with_extra_behaviour() -> LabelledTransitionSystemV0 {
        LabelledTransitionSystemV0 {
            states: vec![
                LtsStateV0 { id: 11 },
                LtsStateV0 { id: 12 },
                LtsStateV0 { id: 13 },
            ],
            actions: vec![LtsActionV0 { id: 110, kind: 100 }],
            transitions: vec![
                LtsTransitionV0 {
                    from_state_id: 11,
                    action_id: 110,
                    to_state_id: 12,
                },
                LtsTransitionV0 {
                    from_state_id: 12,
                    action_id: 110,
                    to_state_id: 13,
                },
            ],
        }
    }

    fn many_to_many_candidate() -> BehaviouralCorrespondenceV0 {
        BehaviouralCorrespondenceV0 {
            scope: BehaviouralScopeV0 {
                source_state_ids: vec![1, 2],
                target_state_ids: vec![11, 12, 13],
                source_action_ids: vec![10],
                target_action_ids: vec![110],
            },
            state_relation: vec![
                BehaviouralStatePairV0 {
                    source_state_id: 1,
                    target_state_id: 11,
                },
                BehaviouralStatePairV0 {
                    source_state_id: 2,
                    target_state_id: 12,
                },
                BehaviouralStatePairV0 {
                    source_state_id: 2,
                    target_state_id: 13,
                },
            ],
            action_mappings: vec![BehaviouralActionMappingV0 {
                source_action_id: 10,
                target_action_id: 110,
                source_action_kind: 1,
                target_action_kind: 100,
            }],
        }
    }

    #[test]
    fn simulation_allows_additional_target_behaviour() {
        let source = source();
        let target = target_with_extra_behaviour();
        let candidate = many_to_many_candidate();

        let witness = validate_behavioural_correspondence(
            &source,
            &target,
            BehaviouralMapKindV0::Simulation,
            &candidate,
        )
        .unwrap();

        assert_eq!(witness.related_state_pairs, 3);
        assert_eq!(witness.checked_source_transitions, 1);
        assert_eq!(witness.checked_target_transitions, 0);
    }

    #[test]
    fn bisimulation_rejects_unmatched_extra_target_transition() {
        let source = source();
        let target = target_with_extra_behaviour();
        let candidate = many_to_many_candidate();

        assert_eq!(
            validate_behavioural_correspondence(
                &source,
                &target,
                BehaviouralMapKindV0::Bisimulation,
                &candidate,
            ),
            Err(BehaviouralTransferErrorV0::MissingSourceResponse)
        );
    }

    #[test]
    fn bisimulation_passes_when_extra_target_behaviour_is_removed() {
        let source = source();
        let mut target = target_with_extra_behaviour();
        target.transitions.pop();
        let mut candidate = many_to_many_candidate();
        candidate.scope.target_state_ids = vec![11, 12];
        candidate
            .state_relation
            .retain(|pair| pair.target_state_id != 13);

        let witness = validate_behavioural_correspondence(
            &source,
            &target,
            BehaviouralMapKindV0::Bisimulation,
            &candidate,
        )
        .unwrap();

        assert_eq!(witness.checked_source_transitions, 1);
        assert_eq!(witness.checked_target_transitions, 1);
    }

    #[test]
    fn simulation_rejects_a_source_transition_without_target_response() {
        let source = source();
        let mut target = target_with_extra_behaviour();
        target.transitions.clear();
        let candidate = many_to_many_candidate();

        assert_eq!(
            validate_behavioural_correspondence(
                &source,
                &target,
                BehaviouralMapKindV0::Simulation,
                &candidate,
            ),
            Err(BehaviouralTransferErrorV0::MissingTargetResponse)
        );
    }

    #[test]
    fn relation_is_many_to_many_not_forced_to_be_a_function() {
        let source = source();
        let target = target_with_extra_behaviour();
        let candidate = many_to_many_candidate();

        assert!(
            candidate
                .state_relation
                .iter()
                .filter(|pair| pair.source_state_id == 2)
                .count()
                == 2
        );
        assert!(
            validate_behavioural_correspondence(
                &source,
                &target,
                BehaviouralMapKindV0::Simulation,
                &candidate
            )
            .is_ok()
        );
    }

    #[test]
    fn rejects_duplicate_transition_edges() {
        let mut source = source();
        source.transitions.push(source.transitions[0]);
        let target = target_with_extra_behaviour();

        assert_eq!(
            validate_behavioural_correspondence(
                &source,
                &target,
                BehaviouralMapKindV0::Simulation,
                &many_to_many_candidate(),
            ),
            Err(BehaviouralTransferErrorV0::DuplicateTransition)
        );
    }

    #[test]
    fn rejects_missing_state_relation_coverage() {
        let source = source();
        let target = target_with_extra_behaviour();
        let mut candidate = many_to_many_candidate();
        candidate.state_relation.pop();

        assert_eq!(
            validate_behavioural_correspondence(
                &source,
                &target,
                BehaviouralMapKindV0::Simulation,
                &candidate,
            ),
            Err(BehaviouralTransferErrorV0::TargetStateNotRelated)
        );
    }
}
