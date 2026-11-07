//! Exact finite-horizon scenario-tree planning for discrete experiment design.
//!
//! This transparent solver enumerates positive-probability outcomes and
//! optimizes expected discounted information gain under budget and transition
//! constraints. It is exact only for the supplied finite categorical model.
//! Each action reuses a fixed P(outcome | model, action) table at every tree
//! node, so trials are conditionally independent given the model and action.
//! History-dependent / serially correlated trajectories need a different
//! conditional-forecast adapter and are not represented by this module.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    DecisionConfig, DecisionState, EngineError, InterventionCandidate, ModelAdequacy, Posterior,
    expected_information_gain, reject_candidate, validate_candidate, validate_posterior,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransitionConstraints {
    /// When present, only these candidates are allowed at the root.
    pub initial_candidate_ids: Option<BTreeSet<String>>,
    /// If a previous candidate has an entry, only listed candidates may follow.
    /// An absent entry means no additional transition restriction.
    pub allowed_after: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioTreeConfig {
    pub horizon: u32,
    /// Exceeding this limit fails closed; no truncated plan is called exact.
    pub maximum_tree_nodes: u64,
    pub discount_factor: f64,
    pub transitions: TransitionConstraints,
}

impl Default for ScenarioTreeConfig {
    fn default() -> Self {
        Self {
            horizon: 3,
            maximum_tree_nodes: 100_000,
            discount_factor: 1.0,
            transitions: TransitionConstraints::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioTreePlan {
    pub first_candidate_id: String,
    pub expected_discounted_information_gain_nats: f64,
    pub first_step_information_gain_nats: f64,
    pub horizon: u32,
    pub nodes_expanded: u64,
    pub initial_cost_budget: f64,
    pub budget_after_first_action: f64,
    pub cost_unit_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScenarioTreeError {
    InvalidHorizon,
    InvalidNodeBudget,
    InvalidDiscountFactor,
    InvalidCostBudget,
    CostUnitMismatch,
    DuplicateCandidate(String),
    UnknownTransitionCandidate(String),
    NodeBudgetExceeded { maximum: u64 },
    NoFeasibleFirstAction,
    Candidate(EngineError),
}

impl From<EngineError> for ScenarioTreeError {
    fn from(value: EngineError) -> Self {
        Self::Candidate(value)
    }
}

struct NodeValue {
    value_nats: f64,
    selected_candidate_id: Option<String>,
    first_step_eig_nats: f64,
}

fn transition_allows(
    previous: Option<&str>,
    candidate_id: &str,
    constraints: &TransitionConstraints,
) -> bool {
    match previous {
        None => constraints
            .initial_candidate_ids
            .as_ref()
            .is_none_or(|allowed| allowed.contains(candidate_id)),
        Some(previous_id) => constraints
            .allowed_after
            .get(previous_id)
            .is_none_or(|allowed| allowed.contains(candidate_id)),
    }
}

fn validate_config(
    candidates: &[InterventionCandidate],
    config: &ScenarioTreeConfig,
) -> Result<(), ScenarioTreeError> {
    if config.horizon == 0 {
        return Err(ScenarioTreeError::InvalidHorizon);
    }
    if config.maximum_tree_nodes == 0 {
        return Err(ScenarioTreeError::InvalidNodeBudget);
    }
    if !config.discount_factor.is_finite() || !(0.0..=1.0).contains(&config.discount_factor) {
        return Err(ScenarioTreeError::InvalidDiscountFactor);
    }

    let ids: BTreeSet<String> = candidates
        .iter()
        .map(|candidate| candidate.candidate_id.clone())
        .collect();
    if let Some(initial_ids) = &config.transitions.initial_candidate_ids {
        for id in initial_ids {
            if !ids.contains(id) {
                return Err(ScenarioTreeError::UnknownTransitionCandidate(id.clone()));
            }
        }
    }
    for (from_id, to_ids) in &config.transitions.allowed_after {
        if !ids.contains(from_id) {
            return Err(ScenarioTreeError::UnknownTransitionCandidate(
                from_id.clone(),
            ));
        }
        for to_id in to_ids {
            if !ids.contains(to_id) {
                return Err(ScenarioTreeError::UnknownTransitionCandidate(to_id.clone()));
            }
        }
    }
    Ok(())
}

fn posterior_after_outcome(
    posterior: &Posterior,
    candidate: &InterventionCandidate,
    outcome_id: &str,
) -> Option<(f64, Posterior)> {
    let outcome = candidate
        .outcomes
        .iter()
        .find(|outcome| outcome.outcome_id == outcome_id)?;
    let marginal: f64 = posterior
        .iter()
        .map(|(model_id, prior)| prior * outcome.model_probabilities[model_id])
        .sum();
    if marginal <= 0.0 {
        return Some((0.0, posterior.clone()));
    }
    let updated = posterior
        .iter()
        .map(|(model_id, prior)| {
            (
                model_id.clone(),
                prior * outcome.model_probabilities[model_id] / marginal,
            )
        })
        .collect();
    Some((marginal, updated))
}

struct ScenarioSearchContext<'a> {
    candidates: &'a [InterventionCandidate],
    decision_config: &'a DecisionConfig,
    planning_config: &'a ScenarioTreeConfig,
    expanded_nodes: u64,
}

fn search_node(
    posterior: &Posterior,
    remaining_budget: f64,
    previous_candidate_id: Option<&str>,
    remaining_horizon: u32,
    context: &mut ScenarioSearchContext<'_>,
) -> Result<NodeValue, ScenarioTreeError> {
    if remaining_horizon == 0 {
        return Ok(NodeValue {
            value_nats: 0.0,
            selected_candidate_id: None,
            first_step_eig_nats: 0.0,
        });
    }

    context.expanded_nodes =
        context
            .expanded_nodes
            .checked_add(1)
            .ok_or(ScenarioTreeError::NodeBudgetExceeded {
                maximum: context.planning_config.maximum_tree_nodes,
            })?;
    if context.expanded_nodes > context.planning_config.maximum_tree_nodes {
        return Err(ScenarioTreeError::NodeBudgetExceeded {
            maximum: context.planning_config.maximum_tree_nodes,
        });
    }

    let state = DecisionState {
        posterior: posterior.clone(),
        ambiguity_posteriors: Vec::new(),
        remaining_cost_budget: remaining_budget,
        completed_experiments: context.planning_config.horizon - remaining_horizon,
        model_adequacy: ModelAdequacy::Adequate,
        history: Vec::new(),
    };

    let mut ordered_candidates: Vec<&InterventionCandidate> = context
        .candidates
        .iter()
        .filter(|candidate| {
            transition_allows(
                previous_candidate_id,
                &candidate.candidate_id,
                &context.planning_config.transitions,
            ) && reject_candidate(candidate, context.decision_config, &state).is_none()
        })
        .collect();
    ordered_candidates.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));

    let mut best: Option<NodeValue> = None;
    for candidate in ordered_candidates {
        let immediate_eig = expected_information_gain(posterior, candidate)?;
        let mut expected_future = 0.0;

        if remaining_horizon > 1 {
            let mut ordered_outcomes: Vec<&super::OutcomeLikelihood> =
                candidate.outcomes.iter().collect();
            ordered_outcomes.sort_by(|left, right| left.outcome_id.cmp(&right.outcome_id));

            for outcome in ordered_outcomes {
                let Some((marginal, updated_posterior)) =
                    posterior_after_outcome(posterior, candidate, &outcome.outcome_id)
                else {
                    continue;
                };
                if marginal == 0.0 {
                    continue;
                }
                let child = search_node(
                    &updated_posterior,
                    remaining_budget - candidate.cost,
                    Some(&candidate.candidate_id),
                    remaining_horizon - 1,
                    context,
                )?;
                expected_future += marginal * child.value_nats;
            }
        }

        let value = immediate_eig + context.planning_config.discount_factor * expected_future;
        let replace = best.as_ref().is_none_or(|current| {
            value > current.value_nats
                || (value == current.value_nats
                    && candidate.candidate_id.as_str()
                        < current.selected_candidate_id.as_deref().unwrap_or(""))
        });
        if replace {
            best = Some(NodeValue {
                value_nats: value,
                selected_candidate_id: Some(candidate.candidate_id.clone()),
                first_step_eig_nats: immediate_eig,
            });
        }
    }

    Ok(best.unwrap_or(NodeValue {
        value_nats: 0.0,
        selected_candidate_id: None,
        first_step_eig_nats: 0.0,
    }))
}

/// Solve the finite-horizon discrete design problem exactly for the supplied
/// prior, likelihood tables, budget, action-transition graph and horizon.
///
/// The objective is expected discounted cumulative information gain in nats.
/// Every positive-probability categorical outcome is expanded. If the exact
/// tree exceeds the node budget, this returns an error rather than silently
/// pruning branches or returning a partial plan.
pub fn plan_first_action(
    posterior: &Posterior,
    candidates: &[InterventionCandidate],
    cost_budget: f64,
    decision_config: &DecisionConfig,
    planning_config: &ScenarioTreeConfig,
) -> Result<ScenarioTreePlan, ScenarioTreeError> {
    validate_posterior(posterior)?;
    if !cost_budget.is_finite() || cost_budget < 0.0 {
        return Err(ScenarioTreeError::InvalidCostBudget);
    }
    if candidates.is_empty() {
        return Err(ScenarioTreeError::NoFeasibleFirstAction);
    }

    let model_ids: BTreeSet<String> = posterior.keys().cloned().collect();
    let mut candidate_ids = BTreeSet::new();
    let mut cost_units = BTreeSet::new();
    for candidate in candidates {
        validate_candidate(candidate, &model_ids)?;
        cost_units.insert(candidate.cost_unit_id.clone());
        if !candidate_ids.insert(candidate.candidate_id.clone()) {
            return Err(ScenarioTreeError::DuplicateCandidate(
                candidate.candidate_id.clone(),
            ));
        }
    }
    if cost_units.len() > 1 {
        return Err(ScenarioTreeError::CostUnitMismatch);
    }
    validate_config(candidates, planning_config)?;

    let mut context = ScenarioSearchContext {
        candidates,
        decision_config,
        planning_config,
        expanded_nodes: 0,
    };
    let root = search_node(
        posterior,
        cost_budget,
        None,
        planning_config.horizon,
        &mut context,
    )?;
    let nodes_expanded = context.expanded_nodes;
    let first_candidate_id = root
        .selected_candidate_id
        .ok_or(ScenarioTreeError::NoFeasibleFirstAction)?;
    let first_candidate = candidates
        .iter()
        .find(|candidate| candidate.candidate_id == first_candidate_id)
        .ok_or(ScenarioTreeError::NoFeasibleFirstAction)?;

    Ok(ScenarioTreePlan {
        first_candidate_id,
        expected_discounted_information_gain_nats: root.value_nats,
        first_step_information_gain_nats: root.first_step_eig_nats,
        horizon: planning_config.horizon,
        nodes_expanded,
        initial_cost_budget: cost_budget,
        budget_after_first_action: (cost_budget - first_candidate.cost).max(0.0),
        cost_unit_id: first_candidate.cost_unit_id.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn posterior() -> Posterior {
        BTreeMap::from([("model-a".to_owned(), 0.5), ("model-b".to_owned(), 0.5)])
    }

    fn candidate(id: &str, p_a_yes: f64, p_b_yes: f64, cost: f64) -> InterventionCandidate {
        InterventionCandidate {
            candidate_id: id.to_owned(),
            protocol_id: format!("protocol-{id}"),
            cost_unit_id: "experiment-credit".to_owned(),
            cost,
            feasibility: super::super::Feasibility::Permitted,
            causal_validity: super::super::CausalValidity {
                assignment_design_valid: true,
                delivery_fidelity: 1.0,
                delivery_identity_verified: true,
                interference_controlled: true,
                carryover_cleared: true,
            },
            outcomes: vec![
                super::super::OutcomeLikelihood {
                    outcome_id: "yes".to_owned(),
                    model_probabilities: BTreeMap::from([
                        ("model-a".to_owned(), p_a_yes),
                        ("model-b".to_owned(), p_b_yes),
                    ]),
                },
                super::super::OutcomeLikelihood {
                    outcome_id: "no".to_owned(),
                    model_probabilities: BTreeMap::from([
                        ("model-a".to_owned(), 1.0 - p_a_yes),
                        ("model-b".to_owned(), 1.0 - p_b_yes),
                    ]),
                },
            ],
        }
    }

    #[test]
    fn lookahead_can_choose_a_lower_immediate_eig_action() {
        let prior = posterior();
        let high_first = candidate("high-first", 0.90, 0.10, 1.0);
        let low_first = candidate("low-first", 0.55, 0.45, 1.0);
        let followup = candidate("followup", 0.99, 0.01, 1.0);

        let high_eig = expected_information_gain(&prior, &high_first).unwrap();
        let low_eig = expected_information_gain(&prior, &low_first).unwrap();
        assert!(high_eig > low_eig);

        let transitions = TransitionConstraints {
            initial_candidate_ids: Some(BTreeSet::from([
                "high-first".to_owned(),
                "low-first".to_owned(),
            ])),
            allowed_after: BTreeMap::from([
                ("high-first".to_owned(), BTreeSet::new()),
                (
                    "low-first".to_owned(),
                    BTreeSet::from(["followup".to_owned()]),
                ),
                ("followup".to_owned(), BTreeSet::new()),
            ]),
        };
        let plan = plan_first_action(
            &prior,
            &[high_first, low_first, followup],
            2.0,
            &DecisionConfig::default(),
            &ScenarioTreeConfig {
                horizon: 2,
                transitions,
                ..ScenarioTreeConfig::default()
            },
        )
        .unwrap();
        assert_eq!(plan.first_candidate_id, "low-first");
        assert!(plan.expected_discounted_information_gain_nats > high_eig);
        assert!(plan.nodes_expanded > 1);
    }

    #[test]
    fn node_budget_exhaustion_never_returns_a_partial_exact_plan() {
        let items = vec![candidate("a", 0.9, 0.1, 1.0)];
        assert_eq!(
            plan_first_action(
                &posterior(),
                &items,
                2.0,
                &DecisionConfig::default(),
                &ScenarioTreeConfig {
                    horizon: 2,
                    maximum_tree_nodes: 1,
                    ..ScenarioTreeConfig::default()
                }
            ),
            Err(ScenarioTreeError::NodeBudgetExceeded { maximum: 1 })
        );
    }

    #[test]
    fn transition_constraints_are_enforced_after_every_observation() {
        let items = vec![candidate("a", 0.8, 0.2, 1.0), candidate("b", 0.7, 0.3, 1.0)];
        let transitions = TransitionConstraints {
            initial_candidate_ids: Some(BTreeSet::from(["a".to_owned()])),
            allowed_after: BTreeMap::from([("a".to_owned(), BTreeSet::new())]),
        };
        let plan = plan_first_action(
            &posterior(),
            &items,
            3.0,
            &DecisionConfig::default(),
            &ScenarioTreeConfig {
                horizon: 3,
                transitions,
                ..ScenarioTreeConfig::default()
            },
        )
        .unwrap();
        assert_eq!(plan.first_candidate_id, "a");
        assert_eq!(
            plan.expected_discounted_information_gain_nats,
            plan.first_step_information_gain_nats
        );
    }

    #[test]
    fn no_feasible_initial_action_returns_no_plan() {
        let items = vec![candidate("a", 0.9, 0.1, 1.0)];
        let transitions = TransitionConstraints {
            initial_candidate_ids: Some(BTreeSet::new()),
            allowed_after: BTreeMap::new(),
        };
        assert_eq!(
            plan_first_action(
                &posterior(),
                &items,
                3.0,
                &DecisionConfig::default(),
                &ScenarioTreeConfig {
                    transitions,
                    ..ScenarioTreeConfig::default()
                }
            ),
            Err(ScenarioTreeError::NoFeasibleFirstAction)
        );
    }

    #[test]
    fn incompatible_cost_units_are_rejected() {
        let mut other = candidate("b", 0.8, 0.2, 1.0);
        other.cost_unit_id = "seconds".to_owned();
        assert_eq!(
            plan_first_action(
                &posterior(),
                &[candidate("a", 0.9, 0.1, 1.0), other],
                3.0,
                &DecisionConfig::default(),
                &ScenarioTreeConfig::default()
            ),
            Err(ScenarioTreeError::CostUnitMismatch)
        );
    }

    #[test]
    fn candidate_and_outcome_order_do_not_change_the_plan() {
        let mut a = candidate("a", 0.9, 0.1, 1.0);
        let b = candidate("b", 0.7, 0.3, 1.0);
        let first = plan_first_action(
            &posterior(),
            &[a.clone(), b.clone()],
            3.0,
            &DecisionConfig::default(),
            &ScenarioTreeConfig::default(),
        )
        .unwrap();
        a.outcomes.reverse();
        let second = plan_first_action(
            &posterior(),
            &[b, a],
            3.0,
            &DecisionConfig::default(),
            &ScenarioTreeConfig::default(),
        )
        .unwrap();
        assert_eq!(first.first_candidate_id, second.first_candidate_id);
        assert!(
            (first.expected_discounted_information_gain_nats
                - second.expected_discounted_information_gain_nats)
                .abs()
                < 1.0e-12
        );
    }
}
