//! Exact contingent horizon-two active-sensing oracle.
//!
//! The first measurement branches the posterior. Each branch may select a
//! different second action, subject to the remaining hard resource budget.
//! Observation outcomes are conditionally independent given the binary latent
//! state in this v0 model. That assumption is explicit and must not be used for
//! correlated sensor errors or shared environmental disturbances.

use std::collections::BTreeSet;
use std::fmt;

use crate::decision_value::{
    evaluate_action_with_models, AcquisitionSuccessModel, ActionCost, BinaryBelief,
    DecisionLoss, DecisionValueError, DiagnosticAction, ResourceBudget,
};

const PROBABILITY_TOLERANCE: f64 = 1.0e-9;

/// One candidate sensing action and its explicit acquisition model.
#[derive(Clone, Debug, PartialEq)]
pub struct SequentialCandidate {
    pub action: DiagnosticAction,
    pub acquisition: AcquisitionSuccessModel,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObservationBranch {
    pub branch_id: String,
    pub probability: f64,
    pub posterior: BinaryBelief,
    pub acquisition_succeeded: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContingentBranchPlan {
    pub branch_id: String,
    pub branch_probability: f64,
    pub posterior_state_one_probability: f64,
    pub second_action_id: Option<String>,
    pub expected_terminal_bayes_risk: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HorizonTwoPlan {
    pub first_action_id: String,
    pub branches: Vec<ContingentBranchPlan>,
    pub baseline_bayes_risk: f64,
    pub expected_terminal_bayes_risk: f64,
    pub expected_decision_value: f64,
    pub expected_energy_units: f64,
    pub expected_latency_ticks: f64,
    pub expected_bandwidth_units: f64,
    /// Maximum per-branch total resource use (first plus selected second action).
    pub maximum_path_cost: ActionCost,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SequentialValueError {
    EmptyActionId,
    DuplicateActionId(String),
    InvalidBranchMass { action_id: String, total: f64 },
    InvalidProbability(String),
    Evaluation(DecisionValueError),
}

impl fmt::Display for SequentialValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyActionId => write!(f, "candidate action ID must not be empty"),
            Self::DuplicateActionId(id) => write!(f, "duplicate candidate action ID: {id}"),
            Self::InvalidBranchMass { action_id, total } => {
                write!(f, "outcome branch mass for {action_id} sums to {total}, not 1")
            }
            Self::InvalidProbability(id) => {
                write!(f, "invalid probability encountered in action: {id}")
            }
            Self::Evaluation(error) => write!(f, "action evaluation failed: {error}"),
        }
    }
}
impl std::error::Error for SequentialValueError {}
impl From<DecisionValueError> for SequentialValueError {
    fn from(value: DecisionValueError) -> Self {
        Self::Evaluation(value)
    }
}

fn validate_candidates(
    candidates: &[SequentialCandidate],
    belief: BinaryBelief,
    loss: DecisionLoss,
) -> Result<(), SequentialValueError> {
    let mut ids = BTreeSet::new();
    for candidate in candidates {
        let id = candidate.action.id.trim();
        if id.is_empty() {
            return Err(SequentialValueError::EmptyActionId);
        }
        if !ids.insert(candidate.action.id.clone()) {
            return Err(SequentialValueError::DuplicateActionId(
                candidate.action.id.clone(),
            ));
        }
        // Reuse the one-step evaluator's fail-closed validation of the
        // likelihood distribution, acquisition model, prior and loss.
        evaluate_action_with_models(
            &candidate.action,
            belief,
            ResourceBudget {
                energy_units: u32::MAX,
                latency_ticks: u32::MAX,
                bandwidth_units: u32::MAX,
            },
            loss,
            candidate.acquisition,
        )?;
    }
    Ok(())
}

/// Enumerate the complete event distribution for a first action.
///
/// Events include every positive-probability measurement outcome plus a
/// distinct no-observation event whenever acquisition failure is possible.
/// The posterior is conditioned on the full event, so state-dependent
/// missingness can update belief only when q0 and q1 explicitly differ.
pub fn enumerate_observation_branches(
    candidate: &SequentialCandidate,
    belief: BinaryBelief,
) -> Result<Vec<ObservationBranch>, SequentialValueError> {
    BinaryBelief::new(belief.probability_state_one)?;
    // Validate the public model structs even when called directly.
    evaluate_action_with_models(
        &candidate.action,
        belief,
        ResourceBudget {
            energy_units: u32::MAX,
            latency_ticks: u32::MAX,
            bandwidth_units: u32::MAX,
        },
        DecisionLoss::zero_one(),
        candidate.acquisition,
    )?;

    let p0 = belief.probability_state_zero();
    let p1 = belief.probability_state_one;
    let q0 = candidate.acquisition.probability_success_state_zero;
    let q1 = candidate.acquisition.probability_success_state_one;
    let mut branches = Vec::new();

    let mut outcomes = candidate.action.outcomes.clone();
    outcomes.sort_by(|a, b| a.id.cmp(&b.id));
    for outcome in outcomes {
        let joint_zero = p0 * q0 * outcome.likelihood_state_zero;
        let joint_one = p1 * q1 * outcome.likelihood_state_one;
        let probability = joint_zero + joint_one;
        if !probability.is_finite() || probability < 0.0 {
            return Err(SequentialValueError::InvalidProbability(
                candidate.action.id.clone(),
            ));
        }
        if probability <= 0.0 {
            continue;
        }
        let posterior_probability_state_one = joint_one / probability;
        let posterior = BinaryBelief::new(posterior_probability_state_one)?;
        branches.push(ObservationBranch {
            branch_id: format!("measurement:{}", outcome.id),
            probability,
            posterior,
            acquisition_succeeded: true,
        });
    }

    let failed_zero = p0 * (1.0 - q0);
    let failed_one = p1 * (1.0 - q1);
    let failure_probability = failed_zero + failed_one;
    if !failure_probability.is_finite() || failure_probability < 0.0 {
        return Err(SequentialValueError::InvalidProbability(
            candidate.action.id.clone(),
        ));
    }
    if failure_probability > 0.0 {
        let posterior = BinaryBelief::new(failed_one / failure_probability)?;
        branches.push(ObservationBranch {
            branch_id: "no_observation".to_owned(),
            probability: failure_probability,
            posterior,
            acquisition_succeeded: false,
        });
    }

    branches.sort_by(|a, b| a.branch_id.cmp(&b.branch_id));
    let total = branches.iter().map(|branch| branch.probability).sum::<f64>();
    if (total - 1.0).abs() > PROBABILITY_TOLERANCE {
        return Err(SequentialValueError::InvalidBranchMass {
            action_id: candidate.action.id.clone(),
            total,
        });
    }
    Ok(branches)
}

fn remaining_budget(budget: ResourceBudget, cost: ActionCost) -> ResourceBudget {
    ResourceBudget {
        energy_units: budget.energy_units - cost.energy_units,
        latency_ticks: budget.latency_ticks - cost.latency_ticks,
        bandwidth_units: budget.bandwidth_units - cost.bandwidth_units,
    }
}

fn combined_cost(first: ActionCost, second: ActionCost) -> ActionCost {
    ActionCost {
        energy_units: first.energy_units.saturating_add(second.energy_units),
        latency_ticks: first.latency_ticks.saturating_add(second.latency_ticks),
        bandwidth_units: first.bandwidth_units.saturating_add(second.bandwidth_units),
    }
}

fn resource_components(cost: ActionCost) -> (f64, f64, f64) {
    (
        cost.energy_units as f64,
        cost.latency_ticks as f64,
        cost.bandwidth_units as f64,
    )
}

fn prefer_second(
    risk: f64,
    cost: ActionCost,
    id: &str,
    best_risk: f64,
    best_cost: ActionCost,
    best_id: Option<&str>,
) -> bool {
    match risk.total_cmp(&best_risk) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => {
            (cost.energy_units, cost.latency_ticks, cost.bandwidth_units, id)
                < (
                    best_cost.energy_units,
                    best_cost.latency_ticks,
                    best_cost.bandwidth_units,
                    best_id.unwrap_or("~"),
                )
        }
    }
}

fn plan_for_first_action(
    first: &SequentialCandidate,
    candidates: &[SequentialCandidate],
    belief: BinaryBelief,
    budget: ResourceBudget,
    loss: DecisionLoss,
) -> Result<HorizonTwoPlan, SequentialValueError> {
    let first_cost = first.action.cost;
    let remaining = remaining_budget(budget, first_cost);
    let baseline_bayes_risk = belief.bayes_risk_with_loss(loss)?;
    let branches = enumerate_observation_branches(first, belief)?;

    let mut branch_plans = Vec::with_capacity(branches.len());
    let mut expected_terminal_bayes_risk = 0.0;
    let mut expected_energy_units = first_cost.energy_units as f64;
    let mut expected_latency_ticks = first_cost.latency_ticks as f64;
    let mut expected_bandwidth_units = first_cost.bandwidth_units as f64;
    let mut maximum_path_cost = first_cost;

    for branch in branches {
        let branch_bayes_risk = branch.posterior.bayes_risk_with_loss(loss)?;
        let mut best_risk = branch_bayes_risk;
        let mut best_action: Option<&SequentialCandidate> = None;
        let mut best_cost = ActionCost::default();

        for candidate in candidates {
            if candidate.action.id == first.action.id
                || !candidate.action.cost.fits(remaining)
            {
                continue;
            }
            let assessment = evaluate_action_with_models(
                &candidate.action,
                branch.posterior,
                remaining,
                loss,
                candidate.acquisition,
            )?;
            if !assessment.feasible {
                continue;
            }
            if assessment.expected_bayes_risk + 1.0e-12 < best_risk
                || ((assessment.expected_bayes_risk - best_risk).abs() <= 1.0e-12
                    && best_action.is_some_and(|best| {
                        prefer_second(
                            assessment.expected_bayes_risk,
                            candidate.action.cost,
                            &candidate.action.id,
                            best_risk,
                            best_cost,
                            best_action.map(|item| item.action.id.as_str()),
                        )
                    }))
            {
                // Do not spend the remaining budget on a zero-value action.
                if assessment.expected_bayes_risk + 1.0e-12 < branch_bayes_risk {
                    best_risk = assessment.expected_bayes_risk;
                    best_cost = candidate.action.cost;
                    best_action = Some(candidate);
                }
            }
        }

        let second_action_id = best_action.map(|candidate| candidate.action.id.clone());
        if let Some(second) = best_action {
            let path_cost = combined_cost(first_cost, second.action.cost);
            maximum_path_cost.energy_units =
                maximum_path_cost.energy_units.max(path_cost.energy_units);
            maximum_path_cost.latency_ticks =
                maximum_path_cost.latency_ticks.max(path_cost.latency_ticks);
            maximum_path_cost.bandwidth_units =
                maximum_path_cost.bandwidth_units.max(path_cost.bandwidth_units);

            let (energy, latency, bandwidth) = resource_components(second.action.cost);
            expected_energy_units += branch.probability * energy;
            expected_latency_ticks += branch.probability * latency;
            expected_bandwidth_units += branch.probability * bandwidth;
        }

        expected_terminal_bayes_risk += branch.probability * best_risk;
        branch_plans.push(ContingentBranchPlan {
            branch_id: branch.branch_id,
            branch_probability: branch.probability,
            posterior_state_one_probability: branch.posterior.probability_state_one,
            second_action_id,
            expected_terminal_bayes_risk: best_risk,
        });
    }

    branch_plans.sort_by(|a, b| a.branch_id.cmp(&b.branch_id));
    Ok(HorizonTwoPlan {
        first_action_id: first.action.id.clone(),
        branches: branch_plans,
        baseline_bayes_risk,
        expected_terminal_bayes_risk,
        expected_decision_value: (baseline_bayes_risk - expected_terminal_bayes_risk).max(0.0),
        expected_energy_units,
        expected_latency_ticks,
        expected_bandwidth_units,
        maximum_path_cost,
    })
}

fn prefer_plan(candidate: &HorizonTwoPlan, current: &HorizonTwoPlan) -> bool {
    match candidate
        .expected_terminal_bayes_risk
        .total_cmp(&current.expected_terminal_bayes_risk)
    {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => (
            candidate.expected_energy_units,
            candidate.expected_latency_ticks,
            candidate.expected_bandwidth_units,
            candidate.first_action_id.as_str(),
        ) < (
            current.expected_energy_units,
            current.expected_latency_ticks,
            current.expected_bandwidth_units,
            current.first_action_id.as_str(),
        ),
    }
}

/// Globally best contingent horizon-two plan over the supplied finite action
/// set. The second action may differ across first-observation branches.
pub fn exact_horizon_two_plan(
    candidates: &[SequentialCandidate],
    belief: BinaryBelief,
    budget: ResourceBudget,
    loss: DecisionLoss,
) -> Result<Option<HorizonTwoPlan>, SequentialValueError> {
    validate_candidates(candidates, belief, loss)?;
    let mut ordered: Vec<&SequentialCandidate> = candidates.iter().collect();
    ordered.sort_by(|a, b| a.action.id.cmp(&b.action.id));

    let mut best: Option<HorizonTwoPlan> = None;
    for first in ordered {
        if !first.action.cost.fits(budget) {
            continue;
        }
        let plan = plan_for_first_action(first, candidates, belief, budget, loss)?;
        if best
            .as_ref()
            .is_none_or(|current| prefer_plan(&plan, current))
        {
            best = Some(plan);
        }
    }
    Ok(best.filter(|plan| plan.expected_decision_value > 0.0))
}

/// One-step greedy comparator: pick the first action with the best one-step
/// expected Bayes risk reduction, then optimize the second action per branch.
/// The continuation is contingent, but the first action is myopic.
pub fn greedy_first_horizon_two_plan(
    candidates: &[SequentialCandidate],
    belief: BinaryBelief,
    budget: ResourceBudget,
    loss: DecisionLoss,
) -> Result<Option<HorizonTwoPlan>, SequentialValueError> {
    validate_candidates(candidates, belief, loss)?;
    let mut ordered: Vec<&SequentialCandidate> = candidates.iter().collect();
    ordered.sort_by(|a, b| a.action.id.cmp(&b.action.id));

    let mut first_choice: Option<(&SequentialCandidate, f64)> = None;
    for candidate in &ordered {
        if !candidate.action.cost.fits(budget) {
            continue;
        }
        let assessment = evaluate_action_with_models(
            &candidate.action,
            belief,
            budget,
            loss,
            candidate.acquisition,
        )?;
        if !assessment.feasible {
            continue;
        }
        let risk = assessment.expected_bayes_risk;
        let replace = first_choice.is_none_or(|(best, best_risk)| {
            risk.total_cmp(&best_risk).is_lt()
                || (risk.total_cmp(&best_risk).is_eq()
                    && (
                        candidate.action.cost.energy_units,
                        candidate.action.cost.latency_ticks,
                        candidate.action.cost.bandwidth_units,
                        candidate.action.id.as_str(),
                    ) < (
                        best.action.cost.energy_units,
                        best.action.cost.latency_ticks,
                        best.action.cost.bandwidth_units,
                        best.action.id.as_str(),
                    ))
        });
        if replace {
            first_choice = Some((candidate, risk));
        }
    }

    let Some((first, _)) = first_choice else {
        return Ok(None);
    };
    let plan = plan_for_first_action(first, candidates, belief, budget, loss)?;
    Ok(plan.expected_decision_value.gt(&0.0).then_some(plan))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        id: &str,
        cost: ActionCost,
        outcomes: Vec<crate::decision_value::ObservationOutcome>,
        acquisition: AcquisitionSuccessModel,
    ) -> SequentialCandidate {
        SequentialCandidate {
            action: DiagnosticAction {
                id: id.to_owned(),
                cost,
                acquisition_success_probability: acquisition.probability_success_state_one,
                outcomes,
            },
            acquisition,
        }
    }

    fn asymmetric_set() -> Vec<SequentialCandidate> {
        let unit = ActionCost {
            energy_units: 1,
            latency_ticks: 1,
            bandwidth_units: 1,
        };
        let two = ActionCost {
            energy_units: 2,
            latency_ticks: 2,
            bandwidth_units: 1,
        };
        let always = AcquisitionSuccessModel::state_independent(1.0);
        vec![
            candidate(
                "a-balanced-first",
                unit,
                vec![
                    crate::decision_value::ObservationOutcome {
                        id: "supports-state-one".to_owned(),
                        likelihood_state_zero: 0.11,
                        likelihood_state_one: 0.89,
                    },
                    crate::decision_value::ObservationOutcome {
                        id: "supports-state-zero".to_owned(),
                        likelihood_state_zero: 0.89,
                        likelihood_state_one: 0.11,
                    },
                ],
                always,
            ),
            candidate(
                "b-local-follow-up",
                unit,
                vec![
                    crate::decision_value::ObservationOutcome {
                        id: "supports-state-one".to_owned(),
                        likelihood_state_zero: 0.20,
                        likelihood_state_one: 0.80,
                    },
                    crate::decision_value::ObservationOutcome {
                        id: "supports-state-zero".to_owned(),
                        likelihood_state_zero: 0.80,
                        likelihood_state_one: 0.20,
                    },
                ],
                always,
            ),
            candidate(
                "c-high-one-step-value",
                two,
                vec![
                    crate::decision_value::ObservationOutcome {
                        id: "alert".to_owned(),
                        likelihood_state_zero: 0.70,
                        likelihood_state_one: 0.04,
                    },
                    crate::decision_value::ObservationOutcome {
                        id: "no-alert".to_owned(),
                        likelihood_state_zero: 0.30,
                        likelihood_state_one: 0.96,
                    },
                ],
                AcquisitionSuccessModel::state_independent(0.8),
            ),
        ]
    }

    fn budget() -> ResourceBudget {
        ResourceBudget {
            energy_units: 2,
            latency_ticks: 2,
            bandwidth_units: 1,
        }
    }

    fn loss() -> DecisionLoss {
        DecisionLoss::zero_one()
    }

    #[test]
    fn branch_probabilities_sum_to_one_and_include_no_observation() {
        let candidates = asymmetric_set();
        let first = candidates.iter().find(|item| item.action.id == "c-high-one-step-value").unwrap();
        let branches = enumerate_observation_branches(
            first,
            BinaryBelief::new(0.9).unwrap(),
        ).unwrap();
        let sum = branches.iter().map(|branch| branch.probability).sum::<f64>();
        assert!((sum - 1.0).abs() < 1.0e-10);
        assert!(branches.iter().any(|branch| branch.branch_id == "no_observation"));
    }

    #[test]
    fn state_dependent_failure_updates_the_posterior() {
        let candidate = candidate(
            "dropout",
            ActionCost::default(),
            vec![
                crate::decision_value::ObservationOutcome {
                    id: "measurement".to_owned(),
                    likelihood_state_zero: 1.0,
                    likelihood_state_one: 1.0,
                },
            ],
            AcquisitionSuccessModel::new(0.2, 0.8).unwrap(),
        );
        let branches = enumerate_observation_branches(
            &candidate,
            BinaryBelief::new(0.6).unwrap(),
        ).unwrap();
        let failed = branches.iter().find(|branch| branch.branch_id == "no_observation").unwrap();
        assert!(!failed.acquisition_succeeded);
        assert!(failed.posterior.probability_state_one < 0.6);
    }

    #[test]
    fn horizon_two_oracle_finds_better_contingent_plan_than_myopic_first_action() {
        let candidates = asymmetric_set();
        let belief = BinaryBelief::new(0.9).unwrap();
        let exact = exact_horizon_two_plan(&candidates, belief, budget(), loss()).unwrap().unwrap();
        let greedy = greedy_first_horizon_two_plan(&candidates, belief, budget(), loss()).unwrap().unwrap();

        assert_eq!(greedy.first_action_id, "c-high-one-step-value");
        assert_eq!(
            exact.first_action_id,
            "a-balanced-first",
            "exact plan={exact:#?}; greedy plan={greedy:#?}"
        );
        assert!(exact.expected_decision_value > greedy.expected_decision_value);
        let branch_after_state_zero = exact.branches.iter()
            .find(|branch| branch.branch_id == "measurement:supports-state-zero")
            .unwrap();
        assert_eq!(branch_after_state_zero.second_action_id.as_deref(), Some("b-local-follow-up"));
        let branch_after_state_one = exact.branches.iter()
            .find(|branch| branch.branch_id == "measurement:supports-state-one")
            .unwrap();
        assert_eq!(branch_after_state_one.second_action_id, None);
    }

    #[test]
    fn no_action_is_reused_in_a_contingent_plan() {
        let candidates = asymmetric_set();
        let plan = exact_horizon_two_plan(
            &candidates,
            BinaryBelief::new(0.9).unwrap(),
            budget(),
            loss(),
        ).unwrap().unwrap();
        for branch in &plan.branches {
            assert_ne!(branch.second_action_id.as_deref(), Some(plan.first_action_id.as_str()));
        }
    }

    #[test]
    fn all_paths_respect_hard_resource_budgets() {
        let candidates = asymmetric_set();
        let plan = exact_horizon_two_plan(
            &candidates,
            BinaryBelief::new(0.9).unwrap(),
            budget(),
            loss(),
        ).unwrap().unwrap();
        assert!(plan.maximum_path_cost.energy_units <= budget().energy_units);
        assert!(plan.maximum_path_cost.latency_ticks <= budget().latency_ticks);
        assert!(plan.maximum_path_cost.bandwidth_units <= budget().bandwidth_units);
    }

    #[test]
    fn candidate_input_order_does_not_change_exact_plan() {
        let mut forward = asymmetric_set();
        let mut reverse = asymmetric_set();
        reverse.reverse();
        let a = exact_horizon_two_plan(
            &forward,
            BinaryBelief::new(0.9).unwrap(),
            budget(),
            loss(),
        ).unwrap();
        let b = exact_horizon_two_plan(
            &reverse,
            BinaryBelief::new(0.9).unwrap(),
            budget(),
            loss(),
        ).unwrap();
        assert_eq!(a, b);
        forward.reverse();
    }

    #[test]
    fn duplicate_candidate_ids_fail_closed() {
        let mut candidates = asymmetric_set();
        candidates.push(candidates[0].clone());
        assert!(matches!(
            exact_horizon_two_plan(
                &candidates,
                BinaryBelief::new(0.9).unwrap(),
                budget(),
                loss(),
            ),
            Err(SequentialValueError::DuplicateActionId(_))
        ));
    }

    #[test]
    fn exact_oracle_is_no_worse_than_myopic_greedy() {
        let candidates = asymmetric_set();
        let belief = BinaryBelief::new(0.9).unwrap();
        let exact = exact_horizon_two_plan(&candidates, belief, budget(), loss()).unwrap().unwrap();
        let greedy = greedy_first_horizon_two_plan(&candidates, belief, budget(), loss()).unwrap().unwrap();
        assert!(exact.expected_terminal_bayes_risk <= greedy.expected_terminal_bayes_risk + 1.0e-12);
    }
}
