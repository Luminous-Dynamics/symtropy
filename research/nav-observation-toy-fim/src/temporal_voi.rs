//! Time-aware active sensing with service-state evolution during observation latency.
//!
//! Each source outcome is generated at sample time. Its posterior is then
//! propagated through the action's latency ticks before the next decision.
//! Failure-domain bits remain static over this short horizon.

use std::collections::BTreeSet;
use std::fmt;

use crate::decision_value::{ActionCost, DecisionLoss, ResourceBudget};
use crate::dependency_graph::{
    enumerate_domain_branches, DependencyGraphError, DomainAction, FailureDomainModel,
};

const EPS: f64 = 1.0e-9;

#[derive(Clone, Debug, PartialEq)]
pub struct TemporalTransitionModel {
    /// Per cause-pattern probabilities of StateZero -> StateOne each tick.
    pub probability_zero_to_one_per_tick: Vec<f64>,
    /// Per cause-pattern probabilities of StateOne -> StateZero each tick.
    pub probability_one_to_zero_per_tick: Vec<f64>,
}

impl TemporalTransitionModel {
    pub fn validate(&self, model: &FailureDomainModel) -> Result<(), TemporalVoIError> {
        let expected = model.cause_pattern_count();
        if self.probability_zero_to_one_per_tick.len() != expected
            || self.probability_one_to_zero_per_tick.len() != expected
        {
            return Err(TemporalVoIError::TransitionLengthMismatch {
                expected,
                zero_to_one: self.probability_zero_to_one_per_tick.len(),
                one_to_zero: self.probability_one_to_zero_per_tick.len(),
            });
        }
        for pattern in 0..expected {
            let p01 = self.probability_zero_to_one_per_tick[pattern];
            let p10 = self.probability_one_to_zero_per_tick[pattern];
            if !p01.is_finite()
                || !(0.0..=1.0).contains(&p01)
                || !p10.is_finite()
                || !(0.0..=1.0).contains(&p10)
            {
                return Err(TemporalVoIError::InvalidTransitionProbability {
                    cause_pattern: pattern,
                    p01,
                    p10,
                });
            }
        }
        Ok(())
    }

    /// Propagate a joint belief through ticks of service-state evolution.
    /// Failure-domain bits are held constant; only the service-state component
    /// transitions, using the rate for each cause pattern.
    pub fn propagate(
        &self,
        model: &FailureDomainModel,
        belief: &[f64],
        ticks: u32,
    ) -> Result<Vec<f64>, TemporalVoIError> {
        self.validate(model)?;
        model.validate_belief(belief)?;
        if ticks == 0 {
            return Ok(belief.to_vec());
        }

        let patterns = model.cause_pattern_count();
        let mut current = belief.to_vec();
        for _ in 0..ticks {
            let mut next = vec![0.0; model.joint_state_count()];
            for pattern in 0..patterns {
                let i0 = pattern;
                let i1 = patterns + pattern;
                let p0 = current[i0];
                let p1 = current[i1];
                let p01 = self.probability_zero_to_one_per_tick[pattern];
                let p10 = self.probability_one_to_zero_per_tick[pattern];
                next[i0] = p0 * (1.0 - p01) + p1 * p10;
                next[i1] = p0 * p01 + p1 * (1.0 - p10);
            }
            model.validate_belief(&next)?;
            current = next;
        }
        Ok(current)
    }
}

/// One observation/no-observation event with both sample-time and
/// decision-time posterior, making measurement age explicit.
#[derive(Clone, Debug, PartialEq)]
pub struct TemporalBranch {
    pub branch_id: String,
    pub probability: f64,
    pub posterior_at_sample: Vec<f64>,
    pub posterior_at_decision: Vec<f64>,
    pub acquisition_succeeded: bool,
    pub elapsed_ticks: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TemporalPlanBranch {
    pub branch_id: String,
    pub probability: f64,
    pub posterior_service_one_at_decision: f64,
    pub posterior_failure_domains: Vec<(String, f64)>,
    pub elapsed_ticks: u32,
    pub second_action: Option<String>,
    pub expected_terminal_bayes_risk: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TemporalPlan {
    pub first_action: String,
    pub branches: Vec<TemporalPlanBranch>,
    pub baseline_bayes_risk: f64,
    pub expected_terminal_bayes_risk: f64,
    pub expected_decision_value: f64,
    pub maximum_path_cost: ActionCost,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TemporalVoIError {
    Dependency(DependencyGraphError),
    TransitionLengthMismatch { expected: usize, zero_to_one: usize, one_to_zero: usize },
    InvalidTransitionProbability { cause_pattern: usize, p01: f64, p10: f64 },
    InvalidDecisionLoss,
    DuplicateActionId(String),
    BranchMassMismatch { action: String, sum: f64 },
}
impl fmt::Display for TemporalVoIError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dependency(error) => write!(f, "dependency model error: {error}"),
            Self::TransitionLengthMismatch { expected, zero_to_one, one_to_zero } =>
                write!(f, "transition vector lengths ({zero_to_one}, {one_to_zero}); expected {expected}"),
            Self::InvalidTransitionProbability { cause_pattern, p01, p10 } =>
                write!(f, "invalid transition probabilities for cause pattern {cause_pattern}: p01={p01}, p10={p10}"),
            Self::InvalidDecisionLoss => write!(f, "invalid terminal decision loss"),
            Self::DuplicateActionId(id) => write!(f, "duplicate action ID: {id}"),
            Self::BranchMassMismatch { action, sum } =>
                write!(f, "temporal branch mass for {action} sums to {sum}, not 1"),
        }
    }
}
impl std::error::Error for TemporalVoIError {}
impl From<DependencyGraphError> for TemporalVoIError {
    fn from(value: DependencyGraphError) -> Self { Self::Dependency(value) }
}

/// Sample the declared action at the current time, then propagate every
/// successful and failed-acquisition posterior through the action latency.
pub fn enumerate_temporal_branches(
    action: &DomainAction,
    model: &FailureDomainModel,
    dynamics: &TemporalTransitionModel,
    prior_at_sample: &[f64],
) -> Result<Vec<TemporalBranch>, TemporalVoIError> {
    dynamics.validate(model)?;
    let sample_branches = enumerate_domain_branches(action, model, prior_at_sample)?;
    let mut branches = Vec::with_capacity(sample_branches.len());
    for branch in sample_branches {
        let at_decision = dynamics.propagate(model, &branch.posterior, action.cost.latency_ticks)?;
        branches.push(TemporalBranch {
            branch_id: branch.branch_id,
            probability: branch.probability,
            posterior_at_sample: branch.posterior,
            posterior_at_decision: at_decision,
            acquisition_succeeded: branch.acquired,
            elapsed_ticks: action.cost.latency_ticks,
        });
    }
    branches.sort_by(|a, b| a.branch_id.cmp(&b.branch_id));
    let total: f64 = branches.iter().map(|b| b.probability).sum();
    if (total - 1.0).abs() > EPS {
        return Err(TemporalVoIError::BranchMassMismatch { action: action.id.clone(), sum: total });
    }
    Ok(branches)
}

fn p_service_zero(model: &FailureDomainModel, belief: &[f64]) -> f64 {
    belief[..model.cause_pattern_count()].iter().sum()
}
fn p_service_one(model: &FailureDomainModel, belief: &[f64]) -> f64 {
    belief[model.cause_pattern_count()..].iter().sum()
}
fn bayes_risk(model: &FailureDomainModel, belief: &[f64], loss: DecisionLoss) -> Result<f64, TemporalVoIError> {
    model.validate_belief(belief)?;
    loss.validate().map_err(|_| TemporalVoIError::InvalidDecisionLoss)?;
    Ok((p_service_zero(model, belief) * loss.false_safe_loss)
        .min(p_service_one(model, belief) * loss.false_degraded_loss))
}
fn expected_risk_after_action(
    action: &DomainAction,
    model: &FailureDomainModel,
    dynamics: &TemporalTransitionModel,
    prior: &[f64],
    loss: DecisionLoss,
) -> Result<f64, TemporalVoIError> {
    Ok(enumerate_temporal_branches(action, model, dynamics, prior)?
        .iter()
        .map(|b| bayes_risk(model, &b.posterior_at_decision, loss).map(|r| b.probability * r))
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .sum())
}
fn remaining_budget(b: ResourceBudget, c: ActionCost) -> ResourceBudget {
    ResourceBudget {
        energy_units: b.energy_units - c.energy_units,
        latency_ticks: b.latency_ticks - c.latency_ticks,
        bandwidth_units: b.bandwidth_units - c.bandwidth_units,
    }
}
fn combine(a: ActionCost, b: ActionCost) -> ActionCost {
    ActionCost {
        energy_units: a.energy_units.saturating_add(b.energy_units),
        latency_ticks: a.latency_ticks.saturating_add(b.latency_ticks),
        bandwidth_units: a.bandwidth_units.saturating_add(b.bandwidth_units),
    }
}
fn prefer_action(candidate: &DomainAction, current: Option<&DomainAction>) -> bool {
    current.is_some_and(|v| (
        candidate.cost.energy_units, candidate.cost.latency_ticks,
        candidate.cost.bandwidth_units, candidate.id.as_str()
    ) < (
        v.cost.energy_units, v.cost.latency_ticks,
        v.cost.bandwidth_units, v.id.as_str()
    ))
}
fn validate_universe(
    actions: &[DomainAction], model: &FailureDomainModel,
    dynamics: &TemporalTransitionModel, prior: &[f64], loss: DecisionLoss,
) -> Result<(), TemporalVoIError> {
    model.validate_belief(prior)?;
    dynamics.validate(model)?;
    loss.validate().map_err(|_| TemporalVoIError::InvalidDecisionLoss)?;
    let mut ids = BTreeSet::new();
    for action in actions {
        action.validate(model)?;
        if !ids.insert(action.id.clone()) {
            return Err(TemporalVoIError::DuplicateActionId(action.id.clone()));
        }
    }
    Ok(())
}

fn plan_for_first(
    first: &DomainAction, actions: &[DomainAction], model: &FailureDomainModel,
    dynamics: &TemporalTransitionModel, prior: &[f64], budget: ResourceBudget,
    loss: DecisionLoss,
) -> Result<TemporalPlan, TemporalVoIError> {
    let remaining = remaining_budget(budget, first.cost);
    let baseline = bayes_risk(model, prior, loss)?;
    let branches = enumerate_temporal_branches(first, model, dynamics, prior)?;
    let mut details = Vec::with_capacity(branches.len());
    let mut expected_terminal = 0.0;
    let mut maximum_path = first.cost;

    for branch in branches {
        let stop_risk = bayes_risk(model, &branch.posterior_at_decision, loss)?;
        let mut best_risk = stop_risk;
        let mut best_action: Option<&DomainAction> = None;
        for candidate in actions {
            if candidate.id == first.id || !candidate.cost.fits(remaining) { continue; }
            let risk = expected_risk_after_action(
                candidate, model, dynamics, &branch.posterior_at_decision, loss
            )?;
            if risk + 1.0e-12 < stop_risk
                && (best_action.is_none() || risk.total_cmp(&best_risk).is_lt()
                    || (risk.total_cmp(&best_risk).is_eq() && prefer_action(candidate, best_action)))
            {
                best_action = Some(candidate);
                best_risk = risk;
            }
        }
        if let Some(second) = best_action {
            let path = combine(first.cost, second.cost);
            maximum_path.energy_units = maximum_path.energy_units.max(path.energy_units);
            maximum_path.latency_ticks = maximum_path.latency_ticks.max(path.latency_ticks);
            maximum_path.bandwidth_units = maximum_path.bandwidth_units.max(path.bandwidth_units);
        }
        let mut causes = Vec::new();
        for id in &model.cause_ids {
            causes.push((id.clone(), model.p_failure_domain(&branch.posterior_at_decision, id)?));
        }
        expected_terminal += branch.probability * best_risk;
        details.push(TemporalPlanBranch {
            branch_id: branch.branch_id,
            probability: branch.probability,
            posterior_service_one_at_decision: p_service_one(model, &branch.posterior_at_decision),
            posterior_failure_domains: causes,
            elapsed_ticks: branch.elapsed_ticks,
            second_action: best_action.map(|a| a.id.clone()),
            expected_terminal_bayes_risk: best_risk,
        });
    }
    details.sort_by(|a, b| a.branch_id.cmp(&b.branch_id));
    Ok(TemporalPlan {
        first_action: first.id.clone(),
        branches: details,
        baseline_bayes_risk: baseline,
        expected_terminal_bayes_risk: expected_terminal,
        expected_decision_value: baseline - expected_terminal,
        maximum_path_cost: maximum_path,
    })
}
fn plan_better(a: &TemporalPlan, b: &TemporalPlan) -> bool {
    match a.expected_terminal_bayes_risk.total_cmp(&b.expected_terminal_bayes_risk) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => (
            a.maximum_path_cost.energy_units, a.maximum_path_cost.latency_ticks,
            a.maximum_path_cost.bandwidth_units, a.first_action.as_str()
        ) < (
            b.maximum_path_cost.energy_units, b.maximum_path_cost.latency_ticks,
            b.maximum_path_cost.bandwidth_units, b.first_action.as_str()
        ),
    }
}

pub fn exact_temporal_horizon_two(
    actions: &[DomainAction], model: &FailureDomainModel,
    dynamics: &TemporalTransitionModel, prior: &[f64],
    budget: ResourceBudget, loss: DecisionLoss,
) -> Result<Option<TemporalPlan>, TemporalVoIError> {
    validate_universe(actions, model, dynamics, prior, loss)?;
    let mut ordered: Vec<&DomainAction> = actions.iter().collect();
    ordered.sort_by(|a, b| a.id.cmp(&b.id));
    let mut best: Option<TemporalPlan> = None;
    for first in ordered {
        if !first.cost.fits(budget) { continue; }
        let plan = plan_for_first(first, actions, model, dynamics, prior, budget, loss)?;
        if best.as_ref().is_none_or(|current| plan_better(&plan, current)) {
            best = Some(plan);
        }
    }
    Ok(best.filter(|p| p.expected_decision_value > 1.0e-12))
}

pub fn greedy_first_temporal_horizon_two(
    actions: &[DomainAction], model: &FailureDomainModel,
    dynamics: &TemporalTransitionModel, prior: &[f64],
    budget: ResourceBudget, loss: DecisionLoss,
) -> Result<Option<TemporalPlan>, TemporalVoIError> {
    validate_universe(actions, model, dynamics, prior, loss)?;
    let mut ordered: Vec<&DomainAction> = actions.iter().collect();
    ordered.sort_by(|a, b| a.id.cmp(&b.id));
    let mut choice: Option<(&DomainAction, f64)> = None;
    for candidate in ordered {
        if !candidate.cost.fits(budget) { continue; }
        let risk = expected_risk_after_action(candidate, model, dynamics, prior, loss)?;
        let replace = choice.is_none_or(|(current, current_risk)| {
            risk.total_cmp(&current_risk).is_lt()
                || (risk.total_cmp(&current_risk).is_eq() && prefer_action(candidate, Some(current)))
        });
        if replace { choice = Some((candidate, risk)); }
    }
    let Some((first, _)) = choice else { return Ok(None); };
    let plan = plan_for_first(first, actions, model, dynamics, prior, budget, loss)?;
    Ok((plan.expected_decision_value > 1.0e-12).then_some(plan))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependency_graph::{exact_domain_horizon_two, DomainOutcome};

    fn model_and_prior() -> (FailureDomainModel, Vec<f64>) {
        let model = FailureDomainModel::new(vec!["clock".to_owned()], vec![0.25; 4]).unwrap();
        (model.clone(), model.prior.clone())
    }
    fn dynamics(model: &FailureDomainModel) -> TemporalTransitionModel {
        TemporalTransitionModel {
            probability_zero_to_one_per_tick: vec![0.02; model.cause_pattern_count()],
            probability_one_to_zero_per_tick: vec![0.15; model.cause_pattern_count()],
        }
    }
    fn action(id: &str, accuracy: f64, latency: u32, energy: u32, success: f64) -> DomainAction {
        DomainAction {
            id: id.to_owned(),
            cost: ActionCost { energy_units: energy, latency_ticks: latency, bandwidth_units: 1 },
            failure_domains: vec!["clock".to_owned()],
            acquisition_success: vec![success; 4],
            outcomes: vec![
                DomainOutcome {
                    id: "safe-reading".to_owned(),
                    likelihood: vec![1.0 - accuracy, 1.0 - accuracy, accuracy, accuracy],
                },
                DomainOutcome {
                    id: "unsafe-reading".to_owned(),
                    likelihood: vec![accuracy, accuracy, 1.0 - accuracy, 1.0 - accuracy],
                },
            ],
        }
    }
    fn budget(energy: u32, latency: u32) -> ResourceBudget {
        ResourceBudget { energy_units: energy, latency_ticks: latency, bandwidth_units: 4 }
    }

    #[test]
    fn zero_latency_is_identity_and_multi_tick_matches_repeated_steps() {
        let (model, prior) = model_and_prior();
        let dynamics = dynamics(&model);
        assert_eq!(dynamics.propagate(&model, &prior, 0).unwrap(), prior);
        let one = dynamics.propagate(&model, &prior, 1).unwrap();
        let two = dynamics.propagate(&model, &one, 1).unwrap();
        let direct = dynamics.propagate(&model, &prior, 2).unwrap();
        for (a, b) in two.iter().zip(direct.iter()) { assert!((a - b).abs() < 1.0e-12); }
    }

    #[test]
    fn invalid_transition_probabilities_and_dimensions_fail_closed() {
        let (model, prior) = model_and_prior();
        let bad_len = TemporalTransitionModel {
            probability_zero_to_one_per_tick: vec![0.1],
            probability_one_to_zero_per_tick: vec![0.2, f64::NAN],
        };
        assert!(matches!(bad_len.validate(&model), Err(TemporalVoIError::TransitionLengthMismatch { .. })));
        let bad_value = TemporalTransitionModel {
            probability_zero_to_one_per_tick: vec![0.1, 1.1],
            probability_one_to_zero_per_tick: vec![0.2, 0.2],
        };
        assert!(matches!(bad_value.validate(&model), Err(TemporalVoIError::InvalidTransitionProbability { .. })));
        assert!(bad_value.propagate(&model, &prior, 1).is_err());
    }

    #[test]
    fn sample_time_posterior_is_propagated_to_decision_time() {
        let (model, prior) = model_and_prior();
        let dynamics = dynamics(&model);
        let sensor = action("sensor", 0.8, 3, 1, 1.0);
        let branches = enumerate_temporal_branches(&sensor, &model, &dynamics, &prior).unwrap();
        let safe = branches.iter().find(|b| b.branch_id == "measurement:safe-reading").unwrap();
        assert_eq!(safe.elapsed_ticks, 3);
        assert_eq!(
            safe.posterior_at_decision,
            dynamics.propagate(&model, &safe.posterior_at_sample, 3).unwrap()
        );
        assert_ne!(safe.posterior_at_sample, safe.posterior_at_decision);
    }

    #[test]
    fn no_observation_branch_is_also_propagated() {
        let (model, prior) = model_and_prior();
        let dynamics = dynamics(&model);
        let sensor = action("dropout", 0.8, 2, 1, 0.5);
        let branches = enumerate_temporal_branches(&sensor, &model, &dynamics, &prior).unwrap();
        let missing = branches.iter().find(|b| b.branch_id == "no_observation").unwrap();
        assert!(!missing.acquisition_succeeded);
        assert!((missing.probability - 0.5).abs() < EPS);
        assert_eq!(
            missing.posterior_at_decision,
            dynamics.propagate(&model, &missing.posterior_at_sample, 2).unwrap()
        );
    }

    #[test]
    fn service_drift_reverses_fast_vs_high_accuracy_sensor_choice() {
        let (model, prior) = model_and_prior();
        let dynamics = dynamics(&model);
        let actions = vec![
            action("fast-lower-accuracy", 0.8, 1, 1, 1.0),
            action("slow-higher-accuracy", 0.95, 4, 1, 1.0),
        ];
        let constraints = budget(1, 4);
        let loss = DecisionLoss::zero_one();

        let static_plan = exact_domain_horizon_two(
            &actions, &model, &prior, constraints, loss
        ).unwrap().unwrap();
        let temporal_plan = exact_temporal_horizon_two(
            &actions, &model, &dynamics, &prior, constraints, loss
        ).unwrap().unwrap();

        assert_eq!(static_plan.first_action, "slow-higher-accuracy");
        assert_eq!(temporal_plan.first_action, "fast-lower-accuracy");
        assert!(temporal_plan.expected_terminal_bayes_risk < 0.27);
    }

    #[test]
    fn temporal_exact_plan_is_no_worse_than_greedy_and_respects_budgets() {
        let (model, prior) = model_and_prior();
        let dynamics = dynamics(&model);
        let actions = vec![
            action("fast", 0.8, 1, 1, 0.95),
            action("slow", 0.95, 2, 1, 0.9),
            action("motion", 0.75, 1, 1, 0.9),
        ];
        let budget = budget(2, 3);
        let exact = exact_temporal_horizon_two(&actions, &model, &dynamics, &prior, budget, DecisionLoss::zero_one()).unwrap().unwrap();
        let greedy = greedy_first_temporal_horizon_two(&actions, &model, &dynamics, &prior, budget, DecisionLoss::zero_one()).unwrap().unwrap();
        assert!(exact.expected_terminal_bayes_risk <= greedy.expected_terminal_bayes_risk + EPS);
        assert!(exact.maximum_path_cost.energy_units <= budget.energy_units);
        assert!(exact.maximum_path_cost.latency_ticks <= budget.latency_ticks);
        assert!(exact.maximum_path_cost.bandwidth_units <= budget.bandwidth_units);
    }

    #[test]
    fn action_order_does_not_change_temporal_plan() {
        let (model, prior) = model_and_prior();
        let dynamics = dynamics(&model);
        let forward = vec![action("fast", 0.8, 1, 1, 0.9), action("slow", 0.95, 2, 1, 0.9)];
        let mut reverse = forward.clone();
        reverse.reverse();
        assert_eq!(
            exact_temporal_horizon_two(&forward, &model, &dynamics, &prior, budget(2, 3), DecisionLoss::zero_one()).unwrap(),
            exact_temporal_horizon_two(&reverse, &model, &dynamics, &prior, budget(2, 3), DecisionLoss::zero_one()).unwrap()
        );
    }

    #[test]
    fn failure_domain_posterior_is_static_during_service_state_drift() {
        let (model, prior) = model_and_prior();
        let dynamics = dynamics(&model);
        let sensor = action("sensor", 0.8, 2, 1, 1.0);
        for branch in enumerate_temporal_branches(&sensor, &model, &dynamics, &prior).unwrap() {
            let before = model.p_failure_domain(&branch.posterior_at_sample, "clock").unwrap();
            let after = model.p_failure_domain(&branch.posterior_at_decision, "clock").unwrap();
            assert!((before - after).abs() < EPS);
        }
    }
}
