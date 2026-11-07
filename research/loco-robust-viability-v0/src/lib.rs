//! Exact finite-state robust viability and recoverability oracle.
//!
//! This package proves properties only of the finite nondeterministic
//! transition system supplied by its caller. Physical safety additionally
//! requires a sound abstraction that includes every physically possible
//! successor.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateSpec {
    pub id: String,
    pub instantaneously_safe: bool,
    pub recovery_target: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionSpec {
    pub id: String,
    pub source_state: String,
    /// All declared outcomes, interpreted adversarially in V0.
    pub successors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TransitionSystem {
    pub states: Vec<StateSpec>,
    pub actions: Vec<ActionSpec>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceLimits {
    pub maximum_states: usize,
    pub maximum_actions: usize,
    pub maximum_successor_edges: usize,
    pub maximum_horizon: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            maximum_states: 100_000,
            maximum_actions: 500_000,
            maximum_successor_edges: 2_000_000,
            maximum_horizon: 100_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StateClassification {
    InstantaneouslyUnsafe,
    RobustlyViable,
    SafeButNotViable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionAssessment {
    pub state_id: String,
    pub action_id: String,
    pub admissible: bool,
    /// Concrete declared outcomes outside the target set.
    pub violating_successors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViabilityResult {
    /// None denotes the maximal robust controlled-invariant fixed point.
    pub finite_horizon: Option<usize>,
    pub viable_states: BTreeSet<String>,
    /// Every action whose complete successor set lies in the result's
    /// continuation set. For finite horizon h > 0, that is V_(h-1).
    pub admissible_actions: BTreeMap<String, Vec<String>>,
    /// Includes a counterexample successor for every rejected action.
    pub action_assessments: Vec<ActionAssessment>,
    pub classifications: BTreeMap<String, StateClassification>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryResult {
    pub target_states: BTreeSet<String>,
    pub recoverable_states: BTreeSet<String>,
    /// Minimum worst-case number of transitions to reach a target.
    pub worst_case_horizons: BTreeMap<String, usize>,
    /// Actions that guarantee a strict decrease in worst-case recovery rank.
    pub recovery_actions: BTreeMap<String, Vec<String>>,
    pub unrecoverable_safe_states: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViabilityError {
    EmptyStateSet,
    EmptyStateId,
    UntrimmedStateId(String),
    DuplicateStateId(String),
    EmptyActionId,
    UntrimmedActionId(String),
    DuplicateActionId(String),
    EmptySourceState(String),
    UnknownSourceState { action_id: String, state_id: String },
    EmptySuccessorSet(String),
    DuplicateSuccessor { action_id: String, state_id: String },
    UnknownSuccessor { action_id: String, state_id: String },
    UnsafeRecoveryTarget(String),
    StateLimitExceeded { maximum: usize },
    ActionLimitExceeded { maximum: usize },
    SuccessorEdgeLimitExceeded { maximum: usize },
    HorizonLimitExceeded { requested: usize, maximum: usize },
}

#[derive(Debug, Clone)]
struct ValidatedSystem {
    states: BTreeMap<String, StateSpec>,
    actions_by_state: BTreeMap<String, Vec<ActionSpec>>,
}

impl TransitionSystem {
    pub fn validate(&self, limits: ResourceLimits) -> Result<(), ViabilityError> {
        validate_system(self, limits).map(|_| ())
    }
}

pub fn compute_finite_horizon_viability(
    system: &TransitionSystem,
    horizon: usize,
    limits: ResourceLimits,
) -> Result<ViabilityResult, ViabilityError> {
    if horizon > limits.maximum_horizon {
        return Err(ViabilityError::HorizonLimitExceeded {
            requested: horizon,
            maximum: limits.maximum_horizon,
        });
    }
    let validated = validate_system(system, limits)?;
    let mut continuation = safe_state_ids(&validated);

    if horizon == 0 {
        return Ok(ViabilityResult {
            finite_horizon: Some(0),
            classifications: classify_states(&validated, &continuation),
            viable_states: continuation,
            admissible_actions: BTreeMap::new(),
            action_assessments: Vec::new(),
        });
    }

    let mut final_assessments = Vec::new();
    for _ in 0..horizon {
        let (predecessor, assessments) = robust_predecessor(&validated, &continuation);
        continuation = predecessor;
        final_assessments = assessments;
    }

    let admissible_actions = admissible_action_map(&final_assessments);
    Ok(ViabilityResult {
        finite_horizon: Some(horizon),
        classifications: classify_states(&validated, &continuation),
        viable_states: continuation,
        admissible_actions,
        action_assessments: final_assessments,
    })
}

pub fn compute_maximal_robust_invariant_set(
    system: &TransitionSystem,
    limits: ResourceLimits,
) -> Result<ViabilityResult, ViabilityError> {
    let validated = validate_system(system, limits)?;
    let mut current = safe_state_ids(&validated);
    let final_assessments = loop {
        let (next, assessments) = robust_predecessor(&validated, &current);
        if next == current {
            break assessments;
        }
        current = next;
    };

    let admissible_actions = admissible_action_map(&final_assessments);
    Ok(ViabilityResult {
        finite_horizon: None,
        classifications: classify_states(&validated, &current),
        viable_states: current,
        admissible_actions,
        action_assessments: final_assessments,
    })
}

pub fn compute_robust_recovery(
    system: &TransitionSystem,
    limits: ResourceLimits,
) -> Result<RecoveryResult, ViabilityError> {
    let validated = validate_system(system, limits)?;
    let targets = validated
        .states
        .values()
        .filter(|state| state.instantaneously_safe && state.recovery_target)
        .map(|state| state.id.clone())
        .collect::<BTreeSet<_>>();

    let safe_states = safe_state_ids(&validated);
    let mut reachable = targets.clone();
    let mut horizons = targets
        .iter()
        .map(|id| (id.clone(), 0_usize))
        .collect::<BTreeMap<_, _>>();
    let mut recovery_actions = BTreeMap::new();
    let mut horizon = 0_usize;

    loop {
        let (predecessor, assessments) = robust_predecessor(&validated, &reachable);
        let newly_recoverable = predecessor
            .difference(&reachable)
            .cloned()
            .collect::<BTreeSet<_>>();
        if newly_recoverable.is_empty() {
            break;
        }
        if horizon >= limits.maximum_horizon {
            return Err(ViabilityError::HorizonLimitExceeded {
                requested: horizon + 1,
                maximum: limits.maximum_horizon,
            });
        }
        horizon += 1;
        let actions = admissible_action_map(&assessments);
        for state_id in &newly_recoverable {
            horizons.insert(state_id.clone(), horizon);
            if let Some(action_ids) = actions.get(state_id) {
                recovery_actions.insert(state_id.clone(), action_ids.clone());
            }
        }
        reachable.extend(newly_recoverable);
    }

    let unrecoverable_safe_states = safe_states
        .difference(&reachable)
        .cloned()
        .collect::<BTreeSet<_>>();

    Ok(RecoveryResult {
        target_states: targets,
        recoverable_states: reachable,
        worst_case_horizons: horizons,
        recovery_actions,
        unrecoverable_safe_states,
    })
}

fn validate_system(
    system: &TransitionSystem,
    limits: ResourceLimits,
) -> Result<ValidatedSystem, ViabilityError> {
    if system.states.is_empty() {
        return Err(ViabilityError::EmptyStateSet);
    }
    if system.states.len() > limits.maximum_states {
        return Err(ViabilityError::StateLimitExceeded {
            maximum: limits.maximum_states,
        });
    }
    if system.actions.len() > limits.maximum_actions {
        return Err(ViabilityError::ActionLimitExceeded {
            maximum: limits.maximum_actions,
        });
    }

    let mut states = BTreeMap::new();
    for state in &system.states {
        if state.id.trim().is_empty() {
            return Err(ViabilityError::EmptyStateId);
        }
        if state.id.trim() != state.id {
            return Err(ViabilityError::UntrimmedStateId(state.id.clone()));
        }
        if state.recovery_target && !state.instantaneously_safe {
            return Err(ViabilityError::UnsafeRecoveryTarget(state.id.clone()));
        }
        if states.insert(state.id.clone(), state.clone()).is_some() {
            return Err(ViabilityError::DuplicateStateId(state.id.clone()));
        }
    }

    let mut actions_by_state: BTreeMap<String, Vec<ActionSpec>> = BTreeMap::new();
    let mut seen_action_ids = BTreeSet::new();
    let mut successor_edge_count = 0_usize;

    for action in &system.actions {
        if action.id.trim().is_empty() {
            return Err(ViabilityError::EmptyActionId);
        }
        if action.id.trim() != action.id {
            return Err(ViabilityError::UntrimmedActionId(action.id.clone()));
        }
        if !seen_action_ids.insert(action.id.clone()) {
            return Err(ViabilityError::DuplicateActionId(action.id.clone()));
        }
        if action.source_state.trim().is_empty() {
            return Err(ViabilityError::EmptySourceState(action.id.clone()));
        }
        if !states.contains_key(&action.source_state) {
            return Err(ViabilityError::UnknownSourceState {
                action_id: action.id.clone(),
                state_id: action.source_state.clone(),
            });
        }
        if action.successors.is_empty() {
            return Err(ViabilityError::EmptySuccessorSet(action.id.clone()));
        }

        let mut seen_successors = BTreeSet::new();
        for successor in &action.successors {
            successor_edge_count = successor_edge_count.saturating_add(1);
            if successor_edge_count > limits.maximum_successor_edges {
                return Err(ViabilityError::SuccessorEdgeLimitExceeded {
                    maximum: limits.maximum_successor_edges,
                });
            }
            if !seen_successors.insert(successor.clone()) {
                return Err(ViabilityError::DuplicateSuccessor {
                    action_id: action.id.clone(),
                    state_id: successor.clone(),
                });
            }
            if !states.contains_key(successor) {
                return Err(ViabilityError::UnknownSuccessor {
                    action_id: action.id.clone(),
                    state_id: successor.clone(),
                });
            }
        }
        actions_by_state
            .entry(action.source_state.clone())
            .or_default()
            .push(action.clone());
    }

    for actions in actions_by_state.values_mut() {
        actions.sort_unstable_by(|left, right| left.id.as_bytes().cmp(right.id.as_bytes()));
    }

    Ok(ValidatedSystem {
        states,
        actions_by_state,
    })
}

fn safe_state_ids(system: &ValidatedSystem) -> BTreeSet<String> {
    system
        .states
        .values()
        .filter(|state| state.instantaneously_safe)
        .map(|state| state.id.clone())
        .collect()
}

fn robust_predecessor(
    system: &ValidatedSystem,
    continuation: &BTreeSet<String>,
) -> (BTreeSet<String>, Vec<ActionAssessment>) {
    let mut predecessor = BTreeSet::new();
    let mut assessments = Vec::new();

    for state in system
        .states
        .values()
        .filter(|state| state.instantaneously_safe)
    {
        let actions = system
            .actions_by_state
            .get(&state.id)
            .map_or(&[], Vec::as_slice);

        for action in actions {
            let violating_successors = action
                .successors
                .iter()
                .filter(|successor| !continuation.contains(*successor))
                .cloned()
                .collect::<Vec<_>>();
            let admissible = violating_successors.is_empty();
            if admissible {
                predecessor.insert(state.id.clone());
            }
            assessments.push(ActionAssessment {
                state_id: state.id.clone(),
                action_id: action.id.clone(),
                admissible,
                violating_successors,
            });
        }
    }
    (predecessor, assessments)
}

fn admissible_action_map(assessments: &[ActionAssessment]) -> BTreeMap<String, Vec<String>> {
    let mut output: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for assessment in assessments.iter().filter(|item| item.admissible) {
        output
            .entry(assessment.state_id.clone())
            .or_default()
            .push(assessment.action_id.clone());
    }
    for action_ids in output.values_mut() {
        action_ids.sort();
        action_ids.dedup();
    }
    output
}

fn classify_states(
    system: &ValidatedSystem,
    viable_states: &BTreeSet<String>,
) -> BTreeMap<String, StateClassification> {
    system
        .states
        .values()
        .map(|state| {
            let classification = if !state.instantaneously_safe {
                StateClassification::InstantaneouslyUnsafe
            } else if viable_states.contains(&state.id) {
                StateClassification::RobustlyViable
            } else {
                StateClassification::SafeButNotViable
            };
            (state.id.clone(), classification)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> ResourceLimits {
        ResourceLimits::default()
    }

    fn state(id: &str, safe: bool, target: bool) -> StateSpec {
        StateSpec {
            id: id.to_owned(),
            instantaneously_safe: safe,
            recovery_target: target,
        }
    }

    fn action(id: &str, source: &str, successors: &[&str]) -> ActionSpec {
        ActionSpec {
            id: id.to_owned(),
            source_state: source.to_owned(),
            successors: successors.iter().map(|value| (*value).to_owned()).collect(),
        }
    }

    #[test]
    fn nominally_safe_action_with_one_unsafe_contact_outcome_is_rejected() {
        let system = TransitionSystem {
            states: vec![
                state("stance", true, false),
                state("next-stance", true, false),
                state("fall", false, false),
            ],
            actions: vec![action("step", "stance", &["next-stance", "fall"])],
        };
        let result = compute_finite_horizon_viability(&system, 1, limits()).unwrap();
        assert!(!result.viable_states.contains("stance"));
        let witness = result
            .action_assessments
            .iter()
            .find(|item| item.action_id == "step")
            .unwrap();
        assert!(!witness.admissible);
        assert_eq!(witness.violating_successors, vec!["fall"]);
    }

    #[test]
    fn safe_but_dead_end_state_is_not_robustly_viable() {
        let system = TransitionSystem {
            states: vec![
                state("current", true, false),
                state("dead-end", true, false),
            ],
            actions: vec![action("one-step", "current", &["dead-end"])],
        };
        let result = compute_maximal_robust_invariant_set(&system, limits()).unwrap();
        assert!(result.viable_states.is_empty());
        assert_eq!(
            result.classifications["current"],
            StateClassification::SafeButNotViable
        );
        assert_eq!(
            result.classifications["dead-end"],
            StateClassification::SafeButNotViable
        );
    }

    #[test]
    fn robust_safe_alternative_preserves_state_and_rejects_risky_action() {
        let system = TransitionSystem {
            states: vec![
                state("s", true, false),
                state("a", true, false),
                state("b", true, false),
                state("fall", false, false),
            ],
            actions: vec![
                action("safe-detour", "s", &["a", "b"]),
                action("risky-shortcut", "s", &["a", "fall"]),
                action("a-loop", "a", &["a"]),
                action("b-loop", "b", &["b"]),
            ],
        };
        let result = compute_maximal_robust_invariant_set(&system, limits()).unwrap();
        assert!(result.viable_states.contains("s"));
        assert_eq!(
            result.admissible_actions["s"],
            vec!["safe-detour".to_owned()]
        );
        let rejected = result
            .action_assessments
            .iter()
            .find(|item| item.action_id == "risky-shortcut")
            .unwrap();
        assert_eq!(rejected.violating_successors, vec!["fall"]);
    }

    #[test]
    fn safe_cycle_is_retained_in_maximal_invariant_set() {
        let system = TransitionSystem {
            states: vec![
                state("a", true, false),
                state("b", true, false),
                state("c", true, false),
                state("fall", false, false),
            ],
            actions: vec![
                action("a-to-b", "a", &["b"]),
                action("b-to-c", "b", &["c"]),
                action("c-to-a", "c", &["a"]),
                action("c-or-fall", "c", &["a", "fall"]),
            ],
        };
        let result = compute_maximal_robust_invariant_set(&system, limits()).unwrap();
        assert_eq!(
            result.viable_states,
            BTreeSet::from(["a".to_owned(), "b".to_owned(), "c".to_owned()])
        );
        assert_eq!(result.admissible_actions["c"], vec!["c-to-a"]);
    }

    #[test]
    fn finite_horizon_viability_shrinks_with_horizon() {
        let system = TransitionSystem {
            states: vec![
                state("s0", true, false),
                state("s1", true, false),
                state("s2", true, false),
                state("fall", false, false),
            ],
            actions: vec![
                action("0", "s0", &["s1"]),
                action("1", "s1", &["s2"]),
                action("2", "s2", &["fall"]),
            ],
        };
        let v0 = compute_finite_horizon_viability(&system, 0, limits()).unwrap();
        let v1 = compute_finite_horizon_viability(&system, 1, limits()).unwrap();
        let v2 = compute_finite_horizon_viability(&system, 2, limits()).unwrap();
        assert_eq!(v0.viable_states.len(), 3);
        assert_eq!(v1.viable_states, BTreeSet::from(["s0".into(), "s1".into()]));
        assert_eq!(v2.viable_states, BTreeSet::from(["s0".into()]));
    }

    #[test]
    fn robust_recovery_reports_worst_case_horizon_and_strict_rank_decrease() {
        let system = TransitionSystem {
            states: vec![
                state("unstable", true, false),
                state("recovering", true, false),
                state("recovering-alt", true, false),
                state("safe", true, true),
                state("dead-end", true, false),
                state("fall", false, false),
            ],
            actions: vec![
                action("recover-1", "unstable", &["recovering", "recovering-alt"]),
                action("recover-2", "recovering", &["safe"]),
                action("recover-alt", "recovering-alt", &["safe"]),
                action("target-loop", "safe", &["safe"]),
            ],
        };
        let result = compute_robust_recovery(&system, limits()).unwrap();
        assert_eq!(result.worst_case_horizons["safe"], 0);
        assert_eq!(result.worst_case_horizons["recovering"], 1);
        assert_eq!(result.worst_case_horizons["unstable"], 2);
        assert_eq!(
            result.recovery_actions["unstable"],
            vec!["recover-1".to_owned()]
        );
        assert_eq!(
            result.recovery_actions["recovering"],
            vec!["recover-2".to_owned()]
        );
        assert_eq!(
            result.unrecoverable_safe_states,
            BTreeSet::from(["dead-end".to_owned()])
        );
    }

    #[test]
    fn adversarial_branch_can_only_shrink_the_viable_set() {
        let base = TransitionSystem {
            states: vec![
                state("s", true, false),
                state("a", true, false),
                state("b", true, false),
                state("fall", false, false),
            ],
            actions: vec![
                action("s-step", "s", &["a"]),
                action("a-loop", "a", &["a"]),
                action("b-loop", "b", &["b"]),
            ],
        };
        let mut perturbed = base.clone();
        perturbed.actions[0].successors.push("fall".to_owned());
        let base_result = compute_maximal_robust_invariant_set(&base, limits()).unwrap();
        let perturbed_result = compute_maximal_robust_invariant_set(&perturbed, limits()).unwrap();
        assert!(
            perturbed_result
                .viable_states
                .is_subset(&base_result.viable_states)
        );
        assert!(base_result.viable_states.contains("s"));
        assert!(!perturbed_result.viable_states.contains("s"));
    }

    #[test]
    fn input_permutations_preserve_canonical_results() {
        let system = TransitionSystem {
            states: vec![
                state("a", true, false),
                state("b", true, true),
                state("c", false, false),
            ],
            actions: vec![
                action("go", "a", &["b"]),
                action("loop", "b", &["b"]),
                action("bad", "a", &["c"]),
            ],
        };
        let mut reordered = system.clone();
        reordered.states.reverse();
        reordered.actions.reverse();
        for item in &mut reordered.actions {
            item.successors.reverse();
        }
        let left = compute_maximal_robust_invariant_set(&system, limits()).unwrap();
        let right = compute_maximal_robust_invariant_set(&reordered, limits()).unwrap();
        assert_eq!(left, right);
    }

    #[test]
    fn synthetic_tripod_phase_step_preserves_a_staggered_recovery_option() {
        let system = TransitionSystem {
            states: vec![
                state("tripod-a", true, false),
                state("tripod-b", true, false),
                state("staggered-recovery", true, false),
                state("one-leg-slip-dead-end", true, false),
                state("fall", false, false),
            ],
            actions: vec![
                action(
                    "phase-step-a-b",
                    "tripod-a",
                    &["tripod-b", "one-leg-slip-dead-end"],
                ),
                action("fallback-a", "tripod-a", &["staggered-recovery"]),
                action(
                    "phase-step-b-a",
                    "tripod-b",
                    &["tripod-a", "one-leg-slip-dead-end"],
                ),
                action("fallback-b", "tripod-b", &["staggered-recovery"]),
                action(
                    "re-enter-from-staggered",
                    "staggered-recovery",
                    &["tripod-a", "tripod-b"],
                ),
            ],
        };
        let result = compute_maximal_robust_invariant_set(&system, limits()).unwrap();
        assert_eq!(
            result.viable_states,
            BTreeSet::from([
                "tripod-a".to_owned(),
                "tripod-b".to_owned(),
                "staggered-recovery".to_owned(),
            ])
        );
        assert_eq!(
            result.admissible_actions["tripod-a"],
            vec!["fallback-a".to_owned()]
        );
        assert_eq!(
            result.admissible_actions["tripod-b"],
            vec!["fallback-b".to_owned()]
        );
        let nominal = result
            .action_assessments
            .iter()
            .find(|item| item.action_id == "phase-step-a-b")
            .unwrap();
        assert_eq!(
            nominal.violating_successors,
            vec!["one-leg-slip-dead-end".to_owned()]
        );
    }

    #[test]
    fn empty_state_system_is_rejected_instead_of_vacuously_certified() {
        assert_eq!(
            compute_maximal_robust_invariant_set(&TransitionSystem::default(), limits()),
            Err(ViabilityError::EmptyStateSet)
        );
    }

    #[test]
    fn removing_an_adversarial_branch_changes_the_model_and_can_expand_viability() {
        let uncertain = TransitionSystem {
            states: vec![
                state("s", true, false),
                state("a", true, false),
                state("dead", true, false),
            ],
            actions: vec![
                action("probe", "s", &["a", "dead"]),
                action("a-loop", "a", &["a"]),
            ],
        };
        let restricted = TransitionSystem {
            states: uncertain.states.clone(),
            actions: vec![action("probe", "s", &["a"]), action("a-loop", "a", &["a"])],
        };
        let before = compute_maximal_robust_invariant_set(&uncertain, limits()).unwrap();
        let after = compute_maximal_robust_invariant_set(&restricted, limits()).unwrap();
        assert!(!before.viable_states.contains("s"));
        assert!(after.viable_states.contains("s"));
    }

    #[test]
    fn empty_successor_set_is_rejected() {
        let system = TransitionSystem {
            states: vec![state("s", true, false)],
            actions: vec![action("no-outcomes", "s", &[])],
        };
        assert_eq!(
            compute_maximal_robust_invariant_set(&system, limits()),
            Err(ViabilityError::EmptySuccessorSet("no-outcomes".into()))
        );
    }

    #[test]
    fn incomplete_transition_graphs_and_duplicate_ids_fail_closed() {
        let unknown = TransitionSystem {
            states: vec![state("s", true, false)],
            actions: vec![action("a", "s", &["missing"])],
        };
        assert_eq!(
            compute_maximal_robust_invariant_set(&unknown, limits()),
            Err(ViabilityError::UnknownSuccessor {
                action_id: "a".into(),
                state_id: "missing".into()
            })
        );

        let duplicate = TransitionSystem {
            states: vec![state("s", true, false), state("s", true, false)],
            actions: vec![],
        };
        assert_eq!(
            compute_maximal_robust_invariant_set(&duplicate, limits()),
            Err(ViabilityError::DuplicateStateId("s".into()))
        );
    }

    #[test]
    fn resource_exhaustion_returns_unknown_error_not_a_partial_certificate() {
        let system = TransitionSystem {
            states: vec![state("a", true, false), state("b", true, false)],
            actions: vec![action("a-loop", "a", &["a"]), action("b-loop", "b", &["b"])],
        };
        let bounded = ResourceLimits {
            maximum_states: 1,
            ..limits()
        };
        assert_eq!(
            compute_maximal_robust_invariant_set(&system, bounded),
            Err(ViabilityError::StateLimitExceeded { maximum: 1 })
        );
    }

    #[test]
    fn exhaustive_three_state_transition_graphs_match_subset_oracle() {
        // Exhaust all 8 safe-state masks and all 7^3 non-empty successor
        // choices for one adversarial action per state.
        for safe_mask in 0_u8..8 {
            for a_mask in 1_u8..8 {
                for b_mask in 1_u8..8 {
                    for c_mask in 1_u8..8 {
                        let successor_masks = [a_mask, b_mask, c_mask];
                        let states = (0..3)
                            .map(|index| {
                                state(&format!("s{index}"), safe_mask & (1 << *index) != 0, false)
                            })
                            .collect::<Vec<_>>();
                        let actions = (0..3)
                            .map(|index| ActionSpec {
                                id: format!("a{index}"),
                                source_state: format!("s{index}"),
                                successors: (0..3)
                                    .filter(|successor| {
                                        successor_masks[index] & (1 << *successor) != 0
                                    })
                                    .map(|successor| format!("s{successor}"))
                                    .collect(),
                            })
                            .collect::<Vec<_>>();
                        let system = TransitionSystem { states, actions };
                        let actual =
                            compute_maximal_robust_invariant_set(&system, limits()).unwrap();

                        let safe_ids = (0..3)
                            .filter(|index| safe_mask & (1 << *index) != 0)
                            .map(|index| format!("s{index}"))
                            .collect::<Vec<_>>();
                        let mut expected = BTreeSet::new();
                        for subset_mask in 0_u8..(1 << safe_ids.len()) {
                            let candidate = safe_ids
                                .iter()
                                .enumerate()
                                .filter(|(index, _)| subset_mask & (1 << *index) != 0)
                                .map(|(_, id)| id.clone())
                                .collect::<BTreeSet<_>>();
                            let invariant = candidate.iter().all(|source| {
                                system
                                    .actions
                                    .iter()
                                    .find(|item| item.source_state == *source)
                                    .is_some_and(|item| {
                                        item.successors
                                            .iter()
                                            .all(|successor| candidate.contains(successor))
                                    })
                            });
                            if invariant {
                                expected.extend(candidate);
                            }
                        }
                        assert_eq!(actual.viable_states, expected);
                    }
                }
            }
        }
    }

    #[test]
    fn exact_fixed_point_matches_independent_brute_force_oracle() {
        let system = TransitionSystem {
            states: vec![
                state("a", true, false),
                state("b", true, false),
                state("c", true, false),
                state("d", true, false),
                state("fall", false, false),
            ],
            actions: vec![
                action("a1", "a", &["b"]),
                action("a2", "a", &["c", "fall"]),
                action("b1", "b", &["c", "d"]),
                action("c1", "c", &["c"]),
                action("d1", "d", &["a", "fall"]),
            ],
        };
        let result = compute_maximal_robust_invariant_set(&system, limits()).unwrap();
        let safe = system
            .states
            .iter()
            .filter(|item| item.instantaneously_safe)
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        let mut union_of_all_invariants = BTreeSet::new();

        for mask in 0_u64..(1_u64 << safe.len()) {
            let candidate = safe
                .iter()
                .enumerate()
                .filter(|(index, _)| (mask & (1_u64 << *index)) != 0)
                .map(|(_, id)| id.clone())
                .collect::<BTreeSet<_>>();
            let is_invariant = candidate.iter().all(|state_id| {
                system
                    .actions
                    .iter()
                    .filter(|item| item.source_state == *state_id)
                    .any(|item| {
                        item.successors
                            .iter()
                            .all(|successor| candidate.contains(successor))
                    })
            });
            if is_invariant {
                union_of_all_invariants.extend(candidate);
            }
        }
        assert_eq!(result.viable_states, union_of_all_invariants);
    }
}
