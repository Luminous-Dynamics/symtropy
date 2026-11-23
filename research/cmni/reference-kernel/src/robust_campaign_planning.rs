//! Exact finite two-stage campaign planning under hidden source dependence.
//!
//! Each complete campaign tree is a deterministic policy from observable
//! acquisition outcomes to later acquisitions and terminal decisions. The same
//! tree is replayed under every admitted dependence scenario; it cannot inspect
//! the hidden scenario identifier. Resource feasibility is enforced on every
//! root-to-leaf path.
//!
//! v0 is exact over a finite scenario set and finite tree space using f64.
//! Sequential likelihoods are factorized conditional on the expanded scientific
//! state and dependence scenario. The external campaign receipt must justify
//! that factorization and include all common-cause latent variables. This module
//! does not authenticate that receipt or provide a formal numerical certificate.

use crate::{InformationAcquisitionKind, ModelError};
use std::collections::BTreeSet;

/// State-conditional likelihood model for one admitted hidden source scenario.
#[derive(Debug, Clone, PartialEq)]
pub struct RobustCampaignScenarioLikelihood {
    pub scenario_id: String,
    /// State-major matrix: [scientific state][observable outcome].
    pub outcome_likelihoods: Vec<Vec<f64>>,
}

/// One acquisition, calibration, simulation or evidence-retrieval option.
#[derive(Debug, Clone, PartialEq)]
pub struct RobustCampaignAcquisition {
    pub option_id: String,
    pub kind: InformationAcquisitionKind,
    pub cost: f64,
    pub outcome_ids: Vec<String>,
    /// External receipt reference for the complete campaign likelihood model.
    /// The numerical kernel checks scope consistency but does not authenticate it.
    pub dependence_scope_receipt_id: String,
    /// Same ordered scenario identifiers must appear for every acquisition.
    pub scenarios: Vec<RobustCampaignScenarioLikelihood>,
}

/// Search limits and hard pathwise resource budget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RobustCampaignPlanningPolicy {
    /// 1 = one acquisition; 2 = one root acquisition and one conditional follow-up.
    pub max_steps: usize,
    pub remaining_resource_budget: f64,
    /// Utility units charged per resource-cost unit.
    pub cost_to_utility_rate: f64,
    /// Hard cap on the number of complete campaign trees evaluated.
    pub max_campaign_policy_trees: usize,
}

/// A stop decision at a campaign history. None means explicit abstention.
#[derive(Debug, Clone, PartialEq)]
pub struct RobustCampaignStop {
    pub terminal_action_id: Option<String>,
}

/// Continuation after one observable root outcome.
#[derive(Debug, Clone, PartialEq)]
pub enum RobustCampaignContinuation {
    Stop(RobustCampaignStop),
    AcquireSecond {
        option_id: String,
        second_outcome_ids: Vec<String>,
        /// One terminal action/abstention for each second-stage outcome.
        terminal_action_ids_by_outcome: Vec<Option<String>>,
    },
}

/// One observable outcome branch of the root acquisition.
#[derive(Debug, Clone, PartialEq)]
pub struct RobustCampaignOutcomeBranch {
    pub outcome_id: String,
    pub continuation: RobustCampaignContinuation,
}

/// Complete deterministic campaign tree. The tree is fixed before the hidden
/// dependence scenario is known and branches only on observed outcome IDs.
#[derive(Debug, Clone, PartialEq)]
pub enum RobustCampaignPlanNode {
    Stop(RobustCampaignStop),
    Acquire {
        option_id: String,
        outcome_branches: Vec<RobustCampaignOutcomeBranch>,
    },
}

/// Gross utility, expected resource cost and net utility under one scenario.
#[derive(Debug, Clone, PartialEq)]
pub struct RobustCampaignScenarioScore {
    pub scenario_id: String,
    pub expected_terminal_utility: f64,
    pub expected_resource_cost: f64,
    pub cost_in_utility_units: f64,
    pub net_expected_utility: f64,
}

/// Exact finite-scenario result for the selected common campaign tree.
#[derive(Debug, Clone, PartialEq)]
pub struct RobustCampaignPlanningResult {
    pub dependence_scope_receipt_id: String,
    pub scenario_ids: Vec<String>,
    pub selected_plan: RobustCampaignPlanNode,
    pub scenario_scores: Vec<RobustCampaignScenarioScore>,
    pub prior_best_terminal_action_id: Option<String>,
    pub prior_best_terminal_utility: f64,
    pub worst_case_net_utility: f64,
    /// Maximum acquisition cost over any reachable root-to-leaf path.
    pub maximum_path_resource_cost: f64,
    /// Declared hard limit used to reject over-budget campaign trees.
    pub pathwise_resource_budget: f64,
    /// True only when the selected tree respects the hard limit on every path.
    pub pathwise_budget_satisfied: bool,
    pub robust_expected_value_of_campaign: f64,
    /// Best scenario-specific tree value, minimized over scenarios. Clairvoyant
    /// because a different tree may be selected after revealing the scenario.
    pub clairvoyant_campaign_upper_bound: f64,
    pub non_anticipativity_gap: f64,
    pub complete_policy_trees_evaluated: usize,
    pub scenario_policy_evaluations: usize,
    pub max_campaign_policy_trees: usize,
    pub max_steps: usize,
    pub finite_scenario_exact: bool,
    /// Always false: arithmetic and likelihood validation use f64 tolerances.
    pub formal_certificate: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum BranchChoice {
    Stop {
        terminal_action_index: usize,
    },
    AcquireSecond {
        option_index: usize,
        terminal_action_indices: Vec<usize>,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct ScenarioEvaluation {
    expected_terminal_utility: f64,
    expected_resource_cost: f64,
    net_expected_utility: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct BestCampaign {
    plan: RobustCampaignPlanNode,
    scenario_evaluations: Vec<ScenarioEvaluation>,
    robust_net_utility: f64,
}

/// Plan a finite two-stage campaign against a finite hidden dependence set.
///
/// A campaign policy is chosen once and evaluated unchanged in every scenario:
///
/// max_{tree} min_s E_s[U(terminal action, state) - acquisition costs].
///
/// The root option and every outcome-contingent follow-up are selected without
/// observing the hidden dependence scenario. The tree may stop or abstain at
/// any root outcome. Each acquisition option is used at most once on a path.
///
/// max_steps is restricted to 1 or 2 in v0. A second acquisition may be
/// selected independently on each root-outcome branch, but no third acquisition
/// is allowed. Exceeding the complete-tree cap returns an error instead of a
/// partial result represented as optimal.
pub fn plan_non_anticipative_robust_campaign(
    state_ids: &[String],
    state_probabilities: &[f64],
    action_ids: &[String],
    action_utilities: &[Vec<f64>],
    abstention_utility: f64,
    acquisitions: &[RobustCampaignAcquisition],
    policy: RobustCampaignPlanningPolicy,
) -> Result<RobustCampaignPlanningResult, ModelError> {
    validate_problem(
        state_ids,
        state_probabilities,
        action_ids,
        action_utilities,
        abstention_utility,
    )?;
    validate_policy(policy)?;
    let (scenario_ids, dependence_scope_receipt_id) =
        validate_acquisition_family(acquisitions, state_ids.len())?;

    let action_count = action_ids.len();
    let terminal_choice_count = action_count
        .checked_add(1)
        .ok_or(ModelError::InvalidParameter(
            "robust_campaign_terminal_choice_count_overflow",
        ))?;
    let prior_best_index =
        best_prior_action(state_probabilities, action_utilities, abstention_utility);
    let prior_best_utility = expected_terminal_utility(
        prior_best_index,
        state_probabilities,
        action_utilities,
        abstention_utility,
    );
    if !prior_best_utility.is_finite() {
        return Err(ModelError::NonFiniteValue(
            "robust_campaign_prior_terminal_utility",
        ));
    }

    let mut total_tree_count = terminal_choice_count;
    if total_tree_count > policy.max_campaign_policy_trees {
        return Err(ModelError::InvalidParameter(
            "robust_campaign_policy_tree_cap_exceeded",
        ));
    }
    let mut branch_choices_by_root = Vec::with_capacity(acquisitions.len());
    for (root_index, root) in acquisitions.iter().enumerate() {
        if root.cost > policy.remaining_resource_budget {
            branch_choices_by_root.push(Vec::new());
            continue;
        }
        let branch_choices = if policy.max_steps == 1 {
            build_stop_only_branch_choices(terminal_choice_count)
        } else {
            build_branch_choices(root_index, acquisitions, terminal_choice_count, policy)?
        };
        let branch_count = checked_power(branch_choices.len(), root.outcome_ids.len())?;
        total_tree_count =
            total_tree_count
                .checked_add(branch_count)
                .ok_or(ModelError::InvalidParameter(
                    "robust_campaign_policy_tree_count_overflow",
                ))?;
        if total_tree_count > policy.max_campaign_policy_trees {
            return Err(ModelError::InvalidParameter(
                "robust_campaign_policy_tree_cap_exceeded",
            ));
        }
        branch_choices_by_root.push(branch_choices);
    }

    if total_tree_count > policy.max_campaign_policy_trees {
        return Err(ModelError::InvalidParameter(
            "robust_campaign_policy_tree_cap_exceeded",
        ));
    }

    let mut best: Option<BestCampaign> = None;
    let mut clairvoyant_best_by_scenario = vec![f64::NEG_INFINITY; scenario_ids.len()];
    let mut complete_policy_trees_evaluated = 0usize;
    let mut scenario_policy_evaluations = 0usize;

    // Stopping is part of the same policy family as acquisition trees.
    for terminal_action_index in 0..terminal_choice_count {
        let plan = RobustCampaignPlanNode::Stop(RobustCampaignStop {
            terminal_action_id: terminal_action_id(action_ids, terminal_action_index),
        });
        let evaluations = evaluate_stop_policy(
            state_probabilities,
            action_utilities,
            abstention_utility,
            terminal_action_index,
            scenario_ids.len(),
        );
        score_candidate_tree(
            plan,
            evaluations,
            &mut best,
            &mut clairvoyant_best_by_scenario,
        )?;
        complete_policy_trees_evaluated =
            complete_policy_trees_evaluated
                .checked_add(1)
                .ok_or(ModelError::InvalidParameter(
                    "robust_campaign_policy_tree_count_overflow",
                ))?;
        scenario_policy_evaluations = scenario_policy_evaluations
            .checked_add(scenario_ids.len())
            .ok_or(ModelError::InvalidParameter(
                "robust_campaign_scenario_evaluation_count_overflow",
            ))?;
    }

    for (root_index, root) in acquisitions.iter().enumerate() {
        let branch_choices = &branch_choices_by_root[root_index];
        if branch_choices.is_empty() {
            continue;
        }
        let combinations = branch_choice_combinations(branch_choices, root.outcome_ids.len());
        for combination in combinations {
            let plan = materialize_campaign_plan(root, &combination, acquisitions, action_ids);
            let evaluations = evaluate_acquisition_tree(
                state_probabilities,
                action_utilities,
                abstention_utility,
                acquisitions,
                root_index,
                &combination,
                policy.cost_to_utility_rate,
            )?;
            score_candidate_tree(
                plan,
                evaluations,
                &mut best,
                &mut clairvoyant_best_by_scenario,
            )?;
            complete_policy_trees_evaluated =
                complete_policy_trees_evaluated.checked_add(1).ok_or(
                    ModelError::InvalidParameter("robust_campaign_policy_tree_count_overflow"),
                )?;
            scenario_policy_evaluations = scenario_policy_evaluations
                .checked_add(scenario_ids.len())
                .ok_or(ModelError::InvalidParameter(
                    "robust_campaign_scenario_evaluation_count_overflow",
                ))?;
        }
    }

    if complete_policy_trees_evaluated != total_tree_count {
        return Err(ModelError::InvalidParameter(
            "robust_campaign_policy_tree_count_mismatch",
        ));
    }
    if complete_policy_trees_evaluated > policy.max_campaign_policy_trees {
        return Err(ModelError::InvalidParameter(
            "robust_campaign_policy_tree_cap_exceeded",
        ));
    }

    let best = best.ok_or(ModelError::MismatchedDataLengths)?;
    let clairvoyant_campaign_upper_bound = clairvoyant_best_by_scenario
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let non_anticipativity_gap =
        (clairvoyant_campaign_upper_bound - best.robust_net_utility).max(0.0);
    let robust_expected_value_of_campaign = best.robust_net_utility - prior_best_utility;
    if !clairvoyant_campaign_upper_bound.is_finite()
        || !non_anticipativity_gap.is_finite()
        || !robust_expected_value_of_campaign.is_finite()
    {
        return Err(ModelError::NonFiniteValue(
            "robust_campaign_qualification_metrics",
        ));
    }

    let maximum_path_resource_cost = maximum_path_resource_cost(&best.plan, acquisitions)?;
    let pathwise_budget_satisfied = maximum_path_resource_cost <= policy.remaining_resource_budget;
    if !pathwise_budget_satisfied {
        return Err(ModelError::InvalidParameter(
            "robust_campaign_selected_tree_violates_pathwise_budget",
        ));
    }

    let prior_best_terminal_action_id = terminal_action_id(action_ids, prior_best_index);
    let scenario_scores = scenario_ids
        .iter()
        .zip(best.scenario_evaluations.iter())
        .map(|(scenario_id, evaluation)| RobustCampaignScenarioScore {
            scenario_id: scenario_id.clone(),
            expected_terminal_utility: evaluation.expected_terminal_utility,
            expected_resource_cost: evaluation.expected_resource_cost,
            cost_in_utility_units: evaluation.expected_resource_cost * policy.cost_to_utility_rate,
            net_expected_utility: evaluation.net_expected_utility,
        })
        .collect();

    Ok(RobustCampaignPlanningResult {
        dependence_scope_receipt_id,
        scenario_ids,
        selected_plan: best.plan,
        scenario_scores,
        prior_best_terminal_action_id,
        prior_best_terminal_utility: prior_best_utility,
        worst_case_net_utility: best.robust_net_utility,
        maximum_path_resource_cost,
        pathwise_resource_budget: policy.remaining_resource_budget,
        pathwise_budget_satisfied,
        robust_expected_value_of_campaign,
        clairvoyant_campaign_upper_bound,
        non_anticipativity_gap,
        complete_policy_trees_evaluated,
        scenario_policy_evaluations,
        max_campaign_policy_trees: policy.max_campaign_policy_trees,
        max_steps: policy.max_steps,
        finite_scenario_exact: true,
        formal_certificate: false,
    })
}

fn validate_problem(
    state_ids: &[String],
    state_probabilities: &[f64],
    action_ids: &[String],
    action_utilities: &[Vec<f64>],
    abstention_utility: f64,
) -> Result<(), ModelError> {
    let state_count = state_ids.len();
    if state_count == 0
        || action_ids.is_empty()
        || state_probabilities.len() != state_count
        || action_utilities.len() != action_ids.len()
        || action_utilities.iter().any(|row| row.len() != state_count)
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    validate_unique_ids(state_ids, "robust_campaign_state_ids")?;
    validate_unique_ids(action_ids, "robust_campaign_action_ids")?;
    if state_probabilities
        .iter()
        .any(|probability| !probability.is_finite())
    {
        return Err(ModelError::NonFiniteValue(
            "robust_campaign_state_probability",
        ));
    }
    if state_probabilities
        .iter()
        .any(|probability| *probability < 0.0)
    {
        return Err(ModelError::InvalidParameter(
            "robust_campaign_state_probabilities_must_be_nonnegative",
        ));
    }
    let probability_sum = state_probabilities.iter().sum::<f64>();
    if (probability_sum - 1.0).abs() > 1.0e-10 || !probability_sum.is_finite() {
        return Err(ModelError::InvalidParameter(
            "robust_campaign_state_probabilities_must_sum_to_one",
        ));
    }
    if !abstention_utility.is_finite()
        || action_utilities
            .iter()
            .flatten()
            .any(|utility| !utility.is_finite())
    {
        return Err(ModelError::NonFiniteValue(
            "robust_campaign_terminal_utility",
        ));
    }
    Ok(())
}

fn validate_policy(policy: RobustCampaignPlanningPolicy) -> Result<(), ModelError> {
    if !policy.remaining_resource_budget.is_finite() || !policy.cost_to_utility_rate.is_finite() {
        return Err(ModelError::NonFiniteValue(
            "robust_campaign_budget_or_cost_rate",
        ));
    }
    if policy.remaining_resource_budget < 0.0 || policy.cost_to_utility_rate < 0.0 {
        return Err(ModelError::InvalidParameter(
            "robust_campaign_budget_and_cost_rate_must_be_nonnegative",
        ));
    }
    if !(1..=2).contains(&policy.max_steps) {
        return Err(ModelError::InvalidParameter(
            "robust_campaign_max_steps_must_be_one_or_two",
        ));
    }
    if policy.max_campaign_policy_trees == 0 {
        return Err(ModelError::InvalidParameter(
            "robust_campaign_tree_cap_must_be_positive",
        ));
    }
    Ok(())
}

fn validate_acquisition_family(
    acquisitions: &[RobustCampaignAcquisition],
    state_count: usize,
) -> Result<(Vec<String>, String), ModelError> {
    if acquisitions.is_empty() {
        return Err(ModelError::MismatchedDataLengths);
    }
    let first = &acquisitions[0];
    if first.dependence_scope_receipt_id.trim().is_empty() {
        return Err(ModelError::InvalidParameter(
            "robust_campaign_dependence_receipt_must_be_nonempty",
        ));
    }
    let scenario_ids = first
        .scenarios
        .iter()
        .map(|scenario| scenario.scenario_id.clone())
        .collect::<Vec<_>>();
    if scenario_ids.is_empty() {
        return Err(ModelError::MismatchedDataLengths);
    }
    validate_unique_ids(&scenario_ids, "robust_campaign_scenario_ids")?;
    let mut option_ids = BTreeSet::new();
    for acquisition in acquisitions {
        if acquisition.dependence_scope_receipt_id != first.dependence_scope_receipt_id {
            return Err(ModelError::InvalidParameter(
                "robust_campaign_acquisitions_must_share_dependence_receipt",
            ));
        }
        if acquisition.option_id.trim().is_empty()
            || !option_ids.insert(acquisition.option_id.as_str())
        {
            return Err(ModelError::InvalidParameter(
                "robust_campaign_option_ids_must_be_nonempty_and_unique",
            ));
        }
        if acquisition.scenarios.len() != scenario_ids.len() {
            return Err(ModelError::MismatchedDataLengths);
        }
        let actual_ids = acquisition
            .scenarios
            .iter()
            .map(|scenario| scenario.scenario_id.clone())
            .collect::<Vec<_>>();
        if actual_ids != scenario_ids {
            return Err(ModelError::InvalidParameter(
                "robust_campaign_acquisitions_must_share_ordered_scenarios",
            ));
        }
        if acquisition.outcome_ids.is_empty() {
            return Err(ModelError::MismatchedDataLengths);
        }
        validate_unique_ids(&acquisition.outcome_ids, "robust_campaign_outcome_ids")?;
        if !acquisition.cost.is_finite() {
            return Err(ModelError::NonFiniteValue(
                "robust_campaign_acquisition_cost",
            ));
        }
        if acquisition.cost < 0.0 {
            return Err(ModelError::InvalidParameter(
                "robust_campaign_acquisition_cost_must_be_nonnegative",
            ));
        }
        validate_acquisition(acquisition, state_count, &scenario_ids)?;
    }
    Ok((scenario_ids, first.dependence_scope_receipt_id.clone()))
}

fn validate_acquisition(
    acquisition: &RobustCampaignAcquisition,
    state_count: usize,
    scenario_ids: &[String],
) -> Result<(), ModelError> {
    if acquisition.scenarios.len() != scenario_ids.len() {
        return Err(ModelError::MismatchedDataLengths);
    }
    for (scenario, expected_id) in acquisition.scenarios.iter().zip(scenario_ids.iter()) {
        if &scenario.scenario_id != expected_id
            || scenario.outcome_likelihoods.len() != state_count
            || scenario
                .outcome_likelihoods
                .iter()
                .any(|row| row.len() != acquisition.outcome_ids.len())
        {
            return Err(ModelError::MismatchedDataLengths);
        }
        for row in &scenario.outcome_likelihoods {
            if row.iter().any(|value| !value.is_finite()) {
                return Err(ModelError::NonFiniteValue(
                    "robust_campaign_outcome_likelihood",
                ));
            }
            if row.iter().any(|value| *value < 0.0 || *value > 1.0) {
                return Err(ModelError::InvalidParameter(
                    "robust_campaign_outcome_likelihood_must_be_in_[0,1]",
                ));
            }
            if (row.iter().sum::<f64>() - 1.0).abs() > 1.0e-10 {
                return Err(ModelError::InvalidParameter(
                    "robust_campaign_likelihood_row_must_sum_to_one",
                ));
            }
        }
    }
    Ok(())
}

fn build_branch_choices(
    root_index: usize,
    acquisitions: &[RobustCampaignAcquisition],
    terminal_choice_count: usize,
    policy: RobustCampaignPlanningPolicy,
) -> Result<Vec<BranchChoice>, ModelError> {
    let mut choices = build_stop_only_branch_choices(terminal_choice_count);
    let root_cost = acquisitions[root_index].cost;
    for (second_index, second) in acquisitions.iter().enumerate() {
        if second_index == root_index {
            continue;
        }
        let total_path_cost = root_cost + second.cost;
        if !total_path_cost.is_finite() {
            return Err(ModelError::NonFiniteValue(
                "robust_campaign_path_resource_cost",
            ));
        }
        if total_path_cost > policy.remaining_resource_budget {
            continue;
        }
        let terminal_maps = checked_power(terminal_choice_count, second.outcome_ids.len())?;
        if terminal_maps
            > policy
                .max_campaign_policy_trees
                .saturating_sub(choices.len())
        {
            return Err(ModelError::InvalidParameter(
                "robust_campaign_policy_tree_cap_exceeded",
            ));
        }
        for map_index in 0..terminal_maps {
            choices.push(BranchChoice::AcquireSecond {
                option_index: second_index,
                terminal_action_indices: decode_base_indices(
                    map_index,
                    terminal_choice_count,
                    second.outcome_ids.len(),
                ),
            });
        }
    }
    Ok(choices)
}

fn build_stop_only_branch_choices(terminal_choice_count: usize) -> Vec<BranchChoice> {
    (0..terminal_choice_count)
        .map(|terminal_action_index| BranchChoice::Stop {
            terminal_action_index,
        })
        .collect()
}

fn branch_choice_combinations(
    choices: &[BranchChoice],
    branch_count: usize,
) -> Vec<Vec<BranchChoice>> {
    fn visit(
        choices: &[BranchChoice],
        remaining: usize,
        current: &mut Vec<BranchChoice>,
        output: &mut Vec<Vec<BranchChoice>>,
    ) {
        if remaining == 0 {
            output.push(current.clone());
            return;
        }
        for choice in choices {
            current.push(choice.clone());
            visit(choices, remaining - 1, current, output);
            current.pop();
        }
    }
    let mut output = Vec::new();
    visit(
        choices,
        branch_count,
        &mut Vec::with_capacity(branch_count),
        &mut output,
    );
    output
}

fn materialize_campaign_plan(
    root: &RobustCampaignAcquisition,
    branch_choices: &[BranchChoice],
    acquisitions: &[RobustCampaignAcquisition],
    action_ids: &[String],
) -> RobustCampaignPlanNode {
    let outcome_branches = root
        .outcome_ids
        .iter()
        .zip(branch_choices.iter())
        .map(|(outcome_id, choice)| {
            let continuation = match choice {
                BranchChoice::Stop {
                    terminal_action_index,
                } => RobustCampaignContinuation::Stop(RobustCampaignStop {
                    terminal_action_id: terminal_action_id(action_ids, *terminal_action_index),
                }),
                BranchChoice::AcquireSecond {
                    option_index,
                    terminal_action_indices,
                } => {
                    let second = &acquisitions[*option_index];
                    RobustCampaignContinuation::AcquireSecond {
                        option_id: second.option_id.clone(),
                        second_outcome_ids: second.outcome_ids.clone(),
                        terminal_action_ids_by_outcome: terminal_action_indices
                            .iter()
                            .map(|index| terminal_action_id(action_ids, *index))
                            .collect(),
                    }
                }
            };
            RobustCampaignOutcomeBranch {
                outcome_id: outcome_id.clone(),
                continuation,
            }
        })
        .collect();
    RobustCampaignPlanNode::Acquire {
        option_id: root.option_id.clone(),
        outcome_branches,
    }
}

fn maximum_path_resource_cost(
    plan: &RobustCampaignPlanNode,
    acquisitions: &[RobustCampaignAcquisition],
) -> Result<f64, ModelError> {
    match plan {
        RobustCampaignPlanNode::Stop(_) => Ok(0.0),
        RobustCampaignPlanNode::Acquire {
            option_id,
            outcome_branches,
        } => {
            let root = acquisitions
                .iter()
                .find(|acquisition| &acquisition.option_id == option_id)
                .ok_or(ModelError::InvalidParameter(
                    "robust_campaign_selected_root_option_missing",
                ))?;
            let mut maximum_continuation_cost = 0.0_f64;
            for branch in outcome_branches {
                let continuation_cost = match &branch.continuation {
                    RobustCampaignContinuation::Stop(_) => 0.0,
                    RobustCampaignContinuation::AcquireSecond { option_id, .. } => acquisitions
                        .iter()
                        .find(|acquisition| &acquisition.option_id == option_id)
                        .map(|acquisition| acquisition.cost)
                        .ok_or(ModelError::InvalidParameter(
                            "robust_campaign_selected_second_option_missing",
                        ))?,
                };
                maximum_continuation_cost = maximum_continuation_cost.max(continuation_cost);
            }
            let maximum_path_cost = root.cost + maximum_continuation_cost;
            if !maximum_path_cost.is_finite() {
                return Err(ModelError::NonFiniteValue(
                    "robust_campaign_maximum_path_cost",
                ));
            }
            Ok(maximum_path_cost)
        }
    }
}

fn evaluate_stop_policy(
    state_probabilities: &[f64],
    action_utilities: &[Vec<f64>],
    abstention_utility: f64,
    terminal_action_index: usize,
    scenario_count: usize,
) -> Vec<ScenarioEvaluation> {
    let utility = expected_terminal_utility(
        terminal_action_index,
        state_probabilities,
        action_utilities,
        abstention_utility,
    );
    vec![
        ScenarioEvaluation {
            expected_terminal_utility: utility,
            expected_resource_cost: 0.0,
            net_expected_utility: utility,
        };
        scenario_count
    ]
}

fn expected_terminal_utility(
    terminal_action_index: usize,
    state_probabilities: &[f64],
    action_utilities: &[Vec<f64>],
    abstention_utility: f64,
) -> f64 {
    state_probabilities
        .iter()
        .enumerate()
        .map(|(state_index, probability)| {
            *probability
                * terminal_utility(
                    terminal_action_index,
                    state_index,
                    action_utilities,
                    abstention_utility,
                )
        })
        .sum()
}

fn evaluate_acquisition_tree(
    state_probabilities: &[f64],
    action_utilities: &[Vec<f64>],
    abstention_utility: f64,
    acquisitions: &[RobustCampaignAcquisition],
    root_index: usize,
    branch_choices: &[BranchChoice],
    cost_to_utility_rate: f64,
) -> Result<Vec<ScenarioEvaluation>, ModelError> {
    let root = &acquisitions[root_index];
    let mut evaluations = Vec::with_capacity(root.scenarios.len());
    for (scenario_index, root_scenario) in root.scenarios.iter().enumerate() {
        let mut expected_terminal_utility = 0.0;
        let mut expected_resource_cost = root.cost;
        for (state_index, state_probability) in state_probabilities.iter().enumerate() {
            for (outcome_index, branch_choice) in branch_choices.iter().enumerate() {
                let root_likelihood = root_scenario.outcome_likelihoods[state_index][outcome_index];
                let branch_probability = state_probability * root_likelihood;
                match branch_choice {
                    BranchChoice::Stop {
                        terminal_action_index,
                    } => {
                        expected_terminal_utility += branch_probability
                            * terminal_utility(
                                *terminal_action_index,
                                state_index,
                                action_utilities,
                                abstention_utility,
                            );
                    }
                    BranchChoice::AcquireSecond {
                        option_index,
                        terminal_action_indices,
                    } => {
                        let second = &acquisitions[*option_index];
                        let second_scenario = &second.scenarios[scenario_index];
                        expected_resource_cost += branch_probability * second.cost;
                        for (second_outcome_index, terminal_action_index) in
                            terminal_action_indices.iter().enumerate()
                        {
                            let likelihood = second_scenario.outcome_likelihoods[state_index]
                                [second_outcome_index];
                            expected_terminal_utility += branch_probability
                                * likelihood
                                * terminal_utility(
                                    *terminal_action_index,
                                    state_index,
                                    action_utilities,
                                    abstention_utility,
                                );
                        }
                    }
                }
            }
        }
        let cost_in_utility_units = expected_resource_cost * cost_to_utility_rate;
        let net_expected_utility = expected_terminal_utility - cost_in_utility_units;
        if !expected_terminal_utility.is_finite()
            || !expected_resource_cost.is_finite()
            || !net_expected_utility.is_finite()
        {
            return Err(ModelError::NonFiniteValue(
                "robust_campaign_scenario_utility",
            ));
        }
        evaluations.push(ScenarioEvaluation {
            expected_terminal_utility,
            expected_resource_cost,
            net_expected_utility,
        });
    }
    Ok(evaluations)
}

fn score_candidate_tree(
    plan: RobustCampaignPlanNode,
    scenario_evaluations: Vec<ScenarioEvaluation>,
    best: &mut Option<BestCampaign>,
    clairvoyant_best_by_scenario: &mut [f64],
) -> Result<(), ModelError> {
    if scenario_evaluations.len() != clairvoyant_best_by_scenario.len() {
        return Err(ModelError::MismatchedDataLengths);
    }
    let robust_net_utility = scenario_evaluations
        .iter()
        .map(|evaluation| evaluation.net_expected_utility)
        .fold(f64::INFINITY, f64::min);
    if !robust_net_utility.is_finite() {
        return Err(ModelError::NonFiniteValue(
            "robust_campaign_worst_case_utility",
        ));
    }
    for (index, evaluation) in scenario_evaluations.iter().enumerate() {
        clairvoyant_best_by_scenario[index] =
            clairvoyant_best_by_scenario[index].max(evaluation.net_expected_utility);
    }
    if best
        .as_ref()
        .is_none_or(|current| robust_net_utility > current.robust_net_utility + 1.0e-12)
    {
        *best = Some(BestCampaign {
            plan,
            scenario_evaluations,
            robust_net_utility,
        });
    }
    Ok(())
}

fn best_prior_action(
    state_probabilities: &[f64],
    action_utilities: &[Vec<f64>],
    abstention_utility: f64,
) -> usize {
    (0..=action_utilities.len())
        .max_by(|left, right| {
            expected_terminal_utility(
                *left,
                state_probabilities,
                action_utilities,
                abstention_utility,
            )
            .total_cmp(&expected_terminal_utility(
                *right,
                state_probabilities,
                action_utilities,
                abstention_utility,
            ))
        })
        .unwrap_or(action_utilities.len())
}

fn terminal_utility(
    action_index: usize,
    state_index: usize,
    action_utilities: &[Vec<f64>],
    abstention_utility: f64,
) -> f64 {
    if action_index == action_utilities.len() {
        abstention_utility
    } else {
        action_utilities[action_index][state_index]
    }
}

fn terminal_action_id(action_ids: &[String], action_index: usize) -> Option<String> {
    if action_index == action_ids.len() {
        None
    } else {
        Some(action_ids[action_index].clone())
    }
}

fn decode_base_indices(mut value: usize, base: usize, length: usize) -> Vec<usize> {
    let mut indices = Vec::with_capacity(length);
    for _ in 0..length {
        indices.push(value % base);
        value /= base;
    }
    indices
}

fn checked_power(base: usize, exponent: usize) -> Result<usize, ModelError> {
    let mut result = 1usize;
    for _ in 0..exponent {
        result = result
            .checked_mul(base)
            .ok_or(ModelError::InvalidParameter(
                "robust_campaign_policy_count_overflow",
            ))?;
    }
    Ok(result)
}

fn validate_unique_ids(ids: &[String], error_name: &'static str) -> Result<(), ModelError> {
    if ids.iter().any(|id| id.trim().is_empty()) {
        return Err(ModelError::InvalidParameter(error_name));
    }
    let mut seen = BTreeSet::new();
    if ids.iter().any(|id| !seen.insert(id.as_str())) {
        return Err(ModelError::InvalidParameter(error_name));
    }
    Ok(())
}

/// Limits for randomization over complete non-anticipative campaign trees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RandomizedRobustCampaignPolicy {
    pub campaign: RobustCampaignPlanningPolicy,
    pub max_basis_evaluations: usize,
    pub feasibility_tolerance: f64,
    pub duality_gap_tolerance: f64,
}

/// One complete contingent campaign tree in the ex-ante randomized policy.
#[derive(Debug, Clone, PartialEq)]
pub struct RandomizedRobustCampaignTreeComponent {
    pub plan: RobustCampaignPlanNode,
    pub probability: f64,
    pub maximum_path_resource_cost: f64,
    pub scenario_scores: Vec<RobustCampaignScenarioScore>,
}

/// Randomized non-anticipative campaign-game solution and dual scenario witness.
#[derive(Debug, Clone, PartialEq)]
pub struct RandomizedRobustCampaignPlanningResult {
    pub dependence_scope_receipt_id: String,
    pub scenario_ids: Vec<String>,
    /// Mixture is selected before the hidden dependence scenario is revealed.
    pub campaign_policy_mixture: Vec<RandomizedRobustCampaignTreeComponent>,
    pub least_favourable_scenario_mixture: Vec<crate::LeastFavourableScenarioWeight>,
    pub randomized_net_utility_by_scenario: Vec<f64>,
    pub deterministic_maximin_net_utility: f64,
    pub randomized_worst_case_net_utility: f64,
    pub deterministic_to_randomized_value_gain: f64,
    pub prior_best_terminal_action_id: Option<String>,
    pub prior_best_terminal_utility: f64,
    pub robust_expected_value_of_campaign: f64,
    pub clairvoyant_campaign_upper_bound: f64,
    pub non_anticipativity_gap: f64,
    pub primal_game_value: f64,
    pub dual_game_value: f64,
    pub primal_dual_gap: f64,
    pub maximum_primal_feasibility_residual: f64,
    pub maximum_dual_feasibility_residual: f64,
    pub maximum_complementarity_residual: f64,
    pub complete_policy_trees_evaluated: usize,
    pub nondominated_policy_trees: usize,
    pub scenario_policy_evaluations: usize,
    pub basis_evaluations: usize,
    pub maximum_path_resource_cost_in_mixture: f64,
    pub pathwise_resource_budget: f64,
    pub all_mixture_components_pathwise_feasible: bool,
    pub max_campaign_policy_trees: usize,
    pub max_basis_evaluations: usize,
    pub finite_game_exact: bool,
    /// Floating-point primal/dual verification is not a formal certificate.
    pub formal_certificate: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct CampaignGameCandidate {
    plan: RobustCampaignPlanNode,
    scenario_evaluations: Vec<ScenarioEvaluation>,
    maximum_path_resource_cost: f64,
}

/// Randomize over complete contingent campaign trees, never over a policy that
/// can see the hidden dependence scenario.
///
/// Primal:
/// max_q min_s sum_tree q[tree] V(tree,s)
///
/// Dual:
/// min_w max_tree sum_s w[s] V(tree,s)
///
/// The tree set is enumerated exactly under the supplied finite scenario set,
/// two-stage tree semantics and hard pathwise resource limit. Strictly
/// dominated payoff vectors and exact duplicate vectors are removed before
/// solving the finite game; this is safe because a policy dominated in every
/// admitted scenario cannot improve a maximin mixture. The shared primal/dual
/// solver checks simplex feasibility, the game gap and complementary slackness.
///
/// The result remains a finite f64 reference. It is not a continuous ambiguity
/// solver, a receipt authenticator or a formal numerical certificate.
pub fn plan_randomized_non_anticipative_robust_campaign(
    state_ids: &[String],
    state_probabilities: &[f64],
    action_ids: &[String],
    action_utilities: &[Vec<f64>],
    abstention_utility: f64,
    acquisitions: &[RobustCampaignAcquisition],
    policy: RandomizedRobustCampaignPolicy,
) -> Result<RandomizedRobustCampaignPlanningResult, ModelError> {
    validate_problem(
        state_ids,
        state_probabilities,
        action_ids,
        action_utilities,
        abstention_utility,
    )?;
    validate_policy(policy.campaign)?;
    validate_randomized_campaign_policy(policy)?;

    let (scenario_ids, dependence_scope_receipt_id) =
        validate_acquisition_family(acquisitions, state_ids.len())?;
    let action_count = action_ids.len();
    let terminal_choice_count = action_count
        .checked_add(1)
        .ok_or(ModelError::InvalidParameter(
            "randomized_campaign_terminal_choice_count_overflow",
        ))?;
    let prior_best_index =
        best_prior_action(state_probabilities, action_utilities, abstention_utility);
    let prior_best_utility = expected_terminal_utility(
        prior_best_index,
        state_probabilities,
        action_utilities,
        abstention_utility,
    );
    if !prior_best_utility.is_finite() {
        return Err(ModelError::NonFiniteValue(
            "randomized_campaign_prior_terminal_utility",
        ));
    }

    // Preflight the complete tree count before constructing policy combinations.
    let mut total_tree_count = terminal_choice_count;
    if total_tree_count > policy.campaign.max_campaign_policy_trees {
        return Err(ModelError::InvalidParameter(
            "randomized_campaign_tree_cap_exceeded",
        ));
    }
    let mut branch_choices_by_root = Vec::with_capacity(acquisitions.len());
    for (root_index, root) in acquisitions.iter().enumerate() {
        if root.cost > policy.campaign.remaining_resource_budget {
            branch_choices_by_root.push(Vec::new());
            continue;
        }
        let branch_choices = if policy.campaign.max_steps == 1 {
            build_stop_only_branch_choices(terminal_choice_count)
        } else {
            build_branch_choices(
                root_index,
                acquisitions,
                terminal_choice_count,
                policy.campaign,
            )?
        };
        let branch_count = checked_power(branch_choices.len(), root.outcome_ids.len())?;
        total_tree_count = total_tree_count.checked_add(branch_count).ok_or(
            ModelError::InvalidParameter("randomized_campaign_tree_count_overflow"),
        )?;
        if total_tree_count > policy.campaign.max_campaign_policy_trees {
            return Err(ModelError::InvalidParameter(
                "randomized_campaign_tree_cap_exceeded",
            ));
        }
        branch_choices_by_root.push(branch_choices);
    }

    let mut all_candidates = Vec::with_capacity(total_tree_count);
    let mut clairvoyant_best_by_scenario = vec![f64::NEG_INFINITY; scenario_ids.len()];
    let mut complete_policy_trees_evaluated = 0usize;
    let mut scenario_policy_evaluations = 0usize;
    let mut deterministic_maximin_net_utility = f64::NEG_INFINITY;

    // Stopping trees are part of the same policy family as acquisition trees.
    for terminal_action_index in 0..terminal_choice_count {
        let plan = RobustCampaignPlanNode::Stop(RobustCampaignStop {
            terminal_action_id: terminal_action_id(action_ids, terminal_action_index),
        });
        let evaluations = evaluate_stop_policy(
            state_probabilities,
            action_utilities,
            abstention_utility,
            terminal_action_index,
            scenario_ids.len(),
        );
        add_campaign_game_candidate(
            plan,
            evaluations,
            acquisitions,
            &mut all_candidates,
            &mut clairvoyant_best_by_scenario,
            &mut deterministic_maximin_net_utility,
        )?;
        complete_policy_trees_evaluated = complete_policy_trees_evaluated
            .checked_add(1)
            .ok_or(ModelError::InvalidParameter(
                "randomized_campaign_tree_count_overflow",
            ))?;
        scenario_policy_evaluations = scenario_policy_evaluations
            .checked_add(scenario_ids.len())
            .ok_or(ModelError::InvalidParameter(
                "randomized_campaign_scenario_evaluation_count_overflow",
            ))?;
    }

    for (root_index, root) in acquisitions.iter().enumerate() {
        let branch_choices = &branch_choices_by_root[root_index];
        if branch_choices.is_empty() {
            continue;
        }
        let combinations =
            branch_choice_combinations(branch_choices, root.outcome_ids.len());
        for combination in combinations {
            let plan = materialize_campaign_plan(root, &combination, acquisitions, action_ids);
            let evaluations = evaluate_acquisition_tree(
                state_probabilities,
                action_utilities,
                abstention_utility,
                acquisitions,
                root_index,
                &combination,
                policy.campaign.cost_to_utility_rate,
            )?;
            add_campaign_game_candidate(
                plan,
                evaluations,
                acquisitions,
                &mut all_candidates,
                &mut clairvoyant_best_by_scenario,
                &mut deterministic_maximin_net_utility,
            )?;
            complete_policy_trees_evaluated = complete_policy_trees_evaluated
                .checked_add(1)
                .ok_or(ModelError::InvalidParameter(
                    "randomized_campaign_tree_count_overflow",
                ))?;
            scenario_policy_evaluations = scenario_policy_evaluations
                .checked_add(scenario_ids.len())
                .ok_or(ModelError::InvalidParameter(
                    "randomized_campaign_scenario_evaluation_count_overflow",
                ))?;
        }
    }

    if complete_policy_trees_evaluated != total_tree_count
        || all_candidates.len() != total_tree_count
    {
        return Err(ModelError::InvalidParameter(
            "randomized_campaign_tree_count_mismatch",
        ));
    }

    let frontier = campaign_game_pareto_frontier(all_candidates);
    if frontier.is_empty() {
        return Err(ModelError::MismatchedDataLengths);
    }
    let payoff_matrix = frontier
        .iter()
        .map(|candidate| {
            candidate
                .scenario_evaluations
                .iter()
                .map(|evaluation| evaluation.net_expected_utility)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let game = crate::solve_finite_zero_sum_game(
        &payoff_matrix,
        policy.feasibility_tolerance,
        policy.duality_gap_tolerance,
        policy.max_basis_evaluations,
    )?;

    let scenario_count = scenario_ids.len();
    let mut randomized_net_utility_by_scenario = vec![0.0; scenario_count];
    let mut campaign_policy_mixture = Vec::new();
    let mut mixture_probability_sum = 0.0;
    let mut maximum_path_resource_cost_in_mixture = 0.0_f64;

    for (frontier_index, probability) in game
        .maximizing_policy_mixture
        .support
        .iter()
        .zip(game.maximizing_policy_mixture.probabilities.iter())
    {
        if *probability <= 0.0 {
            continue;
        }
        if !probability.is_finite() {
            return Err(ModelError::NonFiniteValue(
                "randomized_campaign_policy_probability",
            ));
        }
        let candidate = &frontier[*frontier_index];
        for (scenario_index, evaluation) in
            candidate.scenario_evaluations.iter().enumerate()
        {
            randomized_net_utility_by_scenario[scenario_index] +=
                probability * evaluation.net_expected_utility;
        }
        let scenario_scores = scenario_ids
            .iter()
            .zip(candidate.scenario_evaluations.iter())
            .map(|(scenario_id, evaluation)| RobustCampaignScenarioScore {
                scenario_id: scenario_id.clone(),
                expected_terminal_utility: evaluation.expected_terminal_utility,
                expected_resource_cost: evaluation.expected_resource_cost,
                cost_in_utility_units: evaluation.expected_resource_cost
                    * policy.campaign.cost_to_utility_rate,
                net_expected_utility: evaluation.net_expected_utility,
            })
            .collect::<Vec<_>>();
        maximum_path_resource_cost_in_mixture =
            maximum_path_resource_cost_in_mixture.max(candidate.maximum_path_resource_cost);
        mixture_probability_sum += probability;
        campaign_policy_mixture.push(RandomizedRobustCampaignTreeComponent {
            plan: candidate.plan.clone(),
            probability: *probability,
            maximum_path_resource_cost: candidate.maximum_path_resource_cost,
            scenario_scores,
        });
    }

    if (mixture_probability_sum - 1.0).abs() > policy.feasibility_tolerance * 10.0 {
        return Err(ModelError::InvalidParameter(
            "randomized_campaign_policy_mixture_not_normalized",
        ));
    }
    let all_mixture_components_pathwise_feasible = campaign_policy_mixture.iter().all(
        |component| component.maximum_path_resource_cost
            <= policy.campaign.remaining_resource_budget,
    );
    if !all_mixture_components_pathwise_feasible {
        return Err(ModelError::InvalidParameter(
            "randomized_campaign_mixture_contains_pathwise_infeasible_tree",
        ));
    }

    let least_favourable_scenario_mixture = game
        .minimizing_scenario_mixture
        .support
        .iter()
        .zip(game.minimizing_scenario_mixture.probabilities.iter())
        .filter(|(_, probability)| **probability > 0.0)
        .map(|(scenario_index, probability)| crate::LeastFavourableScenarioWeight {
            scenario_id: scenario_ids[*scenario_index].clone(),
            probability: *probability,
        })
        .collect::<Vec<_>>();

    let randomized_worst_case_net_utility = randomized_net_utility_by_scenario
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let clairvoyant_campaign_upper_bound = clairvoyant_best_by_scenario
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let non_anticipativity_gap =
        (clairvoyant_campaign_upper_bound - randomized_worst_case_net_utility).max(0.0);
    let robust_expected_value_of_campaign =
        randomized_worst_case_net_utility - prior_best_utility;
    let deterministic_to_randomized_value_gain =
        randomized_worst_case_net_utility - deterministic_maximin_net_utility;
    let scale = randomized_worst_case_net_utility
        .abs()
        .max(deterministic_maximin_net_utility.abs())
        .max(1.0);
    if deterministic_to_randomized_value_gain < -policy.feasibility_tolerance * scale {
        return Err(ModelError::InvalidParameter(
            "randomized_campaign_value_below_deterministic_baseline",
        ));
    }
    if !randomized_worst_case_net_utility.is_finite()
        || !clairvoyant_campaign_upper_bound.is_finite()
        || !non_anticipativity_gap.is_finite()
        || !robust_expected_value_of_campaign.is_finite()
        || !deterministic_to_randomized_value_gain.is_finite()
        || !maximum_path_resource_cost_in_mixture.is_finite()
    {
        return Err(ModelError::NonFiniteValue(
            "randomized_campaign_qualification_metrics",
        ));
    }

    Ok(RandomizedRobustCampaignPlanningResult {
        dependence_scope_receipt_id,
        scenario_ids,
        campaign_policy_mixture,
        least_favourable_scenario_mixture,
        randomized_net_utility_by_scenario,
        deterministic_maximin_net_utility,
        randomized_worst_case_net_utility,
        deterministic_to_randomized_value_gain,
        prior_best_terminal_action_id: terminal_action_id(action_ids, prior_best_index),
        prior_best_terminal_utility: prior_best_utility,
        robust_expected_value_of_campaign,
        clairvoyant_campaign_upper_bound,
        non_anticipativity_gap,
        primal_game_value: game.maximizing_policy_mixture.value,
        dual_game_value: game.minimizing_scenario_mixture.value,
        primal_dual_gap: game.primal_dual_gap,
        maximum_primal_feasibility_residual: game
            .maximizing_policy_mixture
            .feasibility_residual,
        maximum_dual_feasibility_residual: game
            .minimizing_scenario_mixture
            .feasibility_residual,
        maximum_complementarity_residual: game.maximum_complementarity_residual,
        complete_policy_trees_evaluated,
        nondominated_policy_trees: frontier.len(),
        scenario_policy_evaluations,
        basis_evaluations: game.basis_evaluations,
        maximum_path_resource_cost_in_mixture,
        pathwise_resource_budget: policy.campaign.remaining_resource_budget,
        all_mixture_components_pathwise_feasible,
        max_campaign_policy_trees: policy.campaign.max_campaign_policy_trees,
        max_basis_evaluations: policy.max_basis_evaluations,
        finite_game_exact: true,
        formal_certificate: false,
    })
}

fn validate_randomized_campaign_policy(
    policy: RandomizedRobustCampaignPolicy,
) -> Result<(), ModelError> {
    if !policy.feasibility_tolerance.is_finite() || !policy.duality_gap_tolerance.is_finite() {
        return Err(ModelError::NonFiniteValue(
            "randomized_campaign_solver_tolerance",
        ));
    }
    if policy.feasibility_tolerance <= 0.0 || policy.duality_gap_tolerance <= 0.0 {
        return Err(ModelError::InvalidParameter(
            "randomized_campaign_solver_tolerance_must_be_positive",
        ));
    }
    if policy.max_basis_evaluations == 0 {
        return Err(ModelError::InvalidParameter(
            "randomized_campaign_basis_cap_must_be_positive",
        ));
    }
    Ok(())
}

fn add_campaign_game_candidate(
    plan: RobustCampaignPlanNode,
    scenario_evaluations: Vec<ScenarioEvaluation>,
    acquisitions: &[RobustCampaignAcquisition],
    candidates: &mut Vec<CampaignGameCandidate>,
    clairvoyant_best_by_scenario: &mut [f64],
    deterministic_maximin_net_utility: &mut f64,
) -> Result<(), ModelError> {
    if scenario_evaluations.len() != clairvoyant_best_by_scenario.len() {
        return Err(ModelError::MismatchedDataLengths);
    }
    let maximum_path_resource_cost = maximum_path_resource_cost(&plan, acquisitions)?;
    let mut worst_case_value = f64::INFINITY;
    for (scenario_index, evaluation) in scenario_evaluations.iter().enumerate() {
        if !evaluation.net_expected_utility.is_finite() {
            return Err(ModelError::NonFiniteValue(
                "randomized_campaign_scenario_payoff",
            ));
        }
        worst_case_value = worst_case_value.min(evaluation.net_expected_utility);
        clairvoyant_best_by_scenario[scenario_index] = clairvoyant_best_by_scenario
            [scenario_index]
            .max(evaluation.net_expected_utility);
    }
    if !worst_case_value.is_finite() {
        return Err(ModelError::NonFiniteValue(
            "randomized_campaign_worst_case_payoff",
        ));
    }
    *deterministic_maximin_net_utility =
        deterministic_maximin_net_utility.max(worst_case_value);
    candidates.push(CampaignGameCandidate {
        plan,
        scenario_evaluations,
        maximum_path_resource_cost,
    });
    Ok(())
}

fn campaign_game_pareto_frontier(
    candidates: Vec<CampaignGameCandidate>,
) -> Vec<CampaignGameCandidate> {
    let mut frontier: Vec<CampaignGameCandidate> = Vec::new();
    'candidate: for candidate in candidates {
        for existing in &frontier {
            if campaign_game_candidate_dominates(existing, &candidate) {
                continue 'candidate;
            }
        }
        frontier.retain(|existing| {
            !campaign_game_candidate_dominates(&candidate, existing)
        });
        frontier.push(candidate);
    }
    frontier
}

fn campaign_game_candidate_dominates(
    left: &CampaignGameCandidate,
    right: &CampaignGameCandidate,
) -> bool {
    if left.scenario_evaluations.len() != right.scenario_evaluations.len() {
        return false;
    }
    let all_at_least = left
        .scenario_evaluations
        .iter()
        .zip(right.scenario_evaluations.iter())
        .all(|(left, right)| left.net_expected_utility >= right.net_expected_utility);
    if !all_at_least {
        return false;
    }
    let strictly_better = left
        .scenario_evaluations
        .iter()
        .zip(right.scenario_evaluations.iter())
        .any(|(left, right)| left.net_expected_utility > right.net_expected_utility);
    strictly_better
        || left.maximum_path_resource_cost <= right.maximum_path_resource_cost
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECEIPT: &str = "receipt:robust-two-stage-campaign-v1";

    type AdaptiveFixture = (
        Vec<String>,
        Vec<f64>,
        Vec<String>,
        Vec<Vec<f64>>,
        Vec<RobustCampaignAcquisition>,
    );

    fn actions() -> Vec<String> {
        vec![
            "state-0".to_owned(),
            "state-1".to_owned(),
            "state-2".to_owned(),
            "state-3".to_owned(),
        ]
    }

    fn scenario(scenario_id: &str, likelihoods: Vec<Vec<f64>>) -> RobustCampaignScenarioLikelihood {
        RobustCampaignScenarioLikelihood {
            scenario_id: scenario_id.to_owned(),
            outcome_likelihoods: likelihoods,
        }
    }

    fn acquisition(option_id: &str, likelihoods: Vec<Vec<f64>>) -> RobustCampaignAcquisition {
        RobustCampaignAcquisition {
            option_id: option_id.to_owned(),
            kind: InformationAcquisitionKind::IndependentCalibration,
            cost: 0.05,
            outcome_ids: vec!["outcome-0".to_owned(), "outcome-1".to_owned()],
            dependence_scope_receipt_id: RECEIPT.to_owned(),
            scenarios: vec![
                scenario("dependence-a", likelihoods.clone()),
                scenario("dependence-b", likelihoods),
            ],
        }
    }

    fn adaptive_fixture() -> AdaptiveFixture {
        let ids = vec![
            "state-0".to_owned(),
            "state-1".to_owned(),
            "state-2".to_owned(),
            "state-3".to_owned(),
        ];
        let probabilities = vec![0.25; 4];
        let actions = actions();
        let utilities = vec![
            vec![1.0, 0.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0, 0.0],
            vec![0.0, 0.0, 1.0, 0.0],
            vec![0.0, 0.0, 0.0, 1.0],
        ];
        let options = vec![
            acquisition(
                "coarse",
                vec![
                    vec![1.0, 0.0],
                    vec![1.0, 0.0],
                    vec![0.0, 1.0],
                    vec![0.0, 1.0],
                ],
            ),
            acquisition(
                "detail-a",
                vec![
                    vec![1.0, 0.0],
                    vec![0.0, 1.0],
                    vec![1.0, 0.0],
                    vec![1.0, 0.0],
                ],
            ),
            acquisition(
                "detail-b",
                vec![
                    vec![1.0, 0.0],
                    vec![1.0, 0.0],
                    vec![1.0, 0.0],
                    vec![0.0, 1.0],
                ],
            ),
        ];
        (ids, probabilities, actions, utilities, options)
    }

    fn policy(max_steps: usize) -> RobustCampaignPlanningPolicy {
        RobustCampaignPlanningPolicy {
            max_steps,
            remaining_resource_budget: 0.10,
            cost_to_utility_rate: 1.0,
            max_campaign_policy_trees: 10_000,
        }
    }

    #[test]
    fn adaptive_tree_beats_every_one_stage_campaign_on_the_frozen_control() {
        let (states, probabilities, actions, utilities, options) = adaptive_fixture();
        let one_stage = plan_non_anticipative_robust_campaign(
            &states,
            &probabilities,
            &actions,
            &utilities,
            0.0,
            &options,
            policy(1),
        )
        .expect("one-stage robust campaign");
        let two_stage = plan_non_anticipative_robust_campaign(
            &states,
            &probabilities,
            &actions,
            &utilities,
            0.0,
            &options,
            policy(2),
        )
        .expect("two-stage robust campaign");

        assert!((one_stage.worst_case_net_utility - 0.45).abs() < 1.0e-12);
        assert!((two_stage.worst_case_net_utility - 0.90).abs() < 1.0e-12);
        assert!((two_stage.maximum_path_resource_cost - 0.10).abs() < 1.0e-12);
        assert!((two_stage.pathwise_resource_budget - 0.10).abs() < 1.0e-12);
        assert!(two_stage.pathwise_budget_satisfied);
        assert!((two_stage.robust_expected_value_of_campaign - 0.65).abs() < 1.0e-12);
        assert!(two_stage.non_anticipativity_gap.abs() < 1.0e-12);
        assert_eq!(two_stage.complete_policy_trees_evaluated, 9_080);
        match &two_stage.selected_plan {
            RobustCampaignPlanNode::Acquire {
                option_id,
                outcome_branches,
            } => {
                assert_eq!(option_id, "coarse");
                assert_eq!(outcome_branches.len(), 2);
                assert!(matches!(
                    &outcome_branches[0].continuation,
                    RobustCampaignContinuation::AcquireSecond { option_id, .. }
                        if option_id == "detail-a"
                ));
                assert!(matches!(
                    &outcome_branches[1].continuation,
                    RobustCampaignContinuation::AcquireSecond { option_id, .. }
                        if option_id == "detail-b"
                ));
            }
            other => panic!("expected adaptive acquisition tree, got {other:?}"),
        }
    }

    #[test]
    fn one_stage_campaign_matches_the_deterministic_robust_decision_oracle() {
        let states = vec!["state-0".to_owned(), "state-1".to_owned()];
        let probabilities = vec![0.5, 0.5];
        let actions = vec!["choose-0".to_owned(), "choose-1".to_owned()];
        let utilities = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let option = RobustCampaignAcquisition {
            option_id: "sensor".to_owned(),
            kind: InformationAcquisitionKind::IndependentCalibration,
            cost: 0.05,
            outcome_ids: vec!["reading-0".to_owned(), "reading-1".to_owned()],
            dependence_scope_receipt_id: RECEIPT.to_owned(),
            scenarios: vec![
                scenario("dependence-a", vec![vec![0.8, 0.2], vec![0.2, 0.8]]),
                scenario("dependence-b", vec![vec![0.8, 0.2], vec![0.2, 0.8]]),
            ],
        };
        let campaign = plan_non_anticipative_robust_campaign(
            &states,
            &probabilities,
            &actions,
            &utilities,
            0.0,
            std::slice::from_ref(&option),
            RobustCampaignPlanningPolicy {
                max_steps: 1,
                remaining_resource_budget: 0.05,
                cost_to_utility_rate: 1.0,
                max_campaign_policy_trees: 32,
            },
        )
        .expect("one-stage campaign oracle");
        let candidate = crate::RobustDecisionDesignCandidate {
            candidate_id: "sensor".to_owned(),
            outcome_ids: option.outcome_ids.clone(),
            resource_cost: option.cost,
            dependence_scope_receipt_id: RECEIPT.to_owned(),
            scenarios: option
                .scenarios
                .iter()
                .map(|scenario| crate::RobustDecisionLikelihoodScenario {
                    scenario_id: scenario.scenario_id.clone(),
                    outcome_likelihoods: scenario.outcome_likelihoods.clone(),
                })
                .collect(),
        };
        let decision = crate::rank_designs_by_robust_decision_value(
            &crate::RobustDecisionProblem {
                state_ids: states,
                action_ids: actions,
                state_probabilities: probabilities,
                action_utilities: utilities,
            },
            &[candidate],
            crate::RobustDecisionDesignPolicy {
                remaining_resource_budget: 0.05,
                cost_to_utility_rate: 1.0,
                max_policy_evaluations: 16,
            },
        )
        .expect("one-stage robust decision oracle");
        let decision_utility = decision.scores[0].robust_post_information_utility;
        assert!((campaign.worst_case_net_utility + 0.05 - decision_utility).abs() < 1.0e-12);
    }

    #[test]
    fn hidden_calibration_reversal_is_not_mistaken_for_observable_information() {
        let states = vec!["state-0".to_owned(), "state-1".to_owned()];
        let probabilities = vec![0.5, 0.5];
        let actions = vec!["choose-0".to_owned(), "choose-1".to_owned()];
        let utilities = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let option = RobustCampaignAcquisition {
            option_id: "ambiguous-calibration".to_owned(),
            kind: InformationAcquisitionKind::IndependentCalibration,
            cost: 0.05,
            outcome_ids: vec!["reading-0".to_owned(), "reading-1".to_owned()],
            dependence_scope_receipt_id: RECEIPT.to_owned(),
            scenarios: vec![
                scenario("forward", vec![vec![1.0, 0.0], vec![0.0, 1.0]]),
                scenario("reversed", vec![vec![0.0, 1.0], vec![1.0, 0.0]]),
            ],
        };
        let result = plan_non_anticipative_robust_campaign(
            &states,
            &probabilities,
            &actions,
            &utilities,
            0.0,
            &[option],
            RobustCampaignPlanningPolicy {
                max_steps: 1,
                remaining_resource_budget: 0.1,
                cost_to_utility_rate: 1.0,
                max_campaign_policy_trees: 32,
            },
        )
        .expect("hidden-scenario robust campaign");
        assert!((result.worst_case_net_utility - 0.5).abs() < 1.0e-12);
        assert!(matches!(
            result.selected_plan,
            RobustCampaignPlanNode::Stop(_)
        ));
        assert!((result.non_anticipativity_gap - 0.45).abs() < 1.0e-12);
    }

    #[test]
    fn second_stage_is_excluded_when_any_path_would_exceed_budget() {
        let (states, probabilities, actions, utilities, options) = adaptive_fixture();
        let mut options = options;
        for option in &mut options {
            option.cost = 0.06;
        }
        let mut limited = policy(2);
        limited.remaining_resource_budget = 0.10;
        let result = plan_non_anticipative_robust_campaign(
            &states,
            &probabilities,
            &actions,
            &utilities,
            0.0,
            &options,
            limited,
        )
        .expect("pathwise budget-limited campaign");
        assert!(result.pathwise_budget_satisfied);
        assert!(result.maximum_path_resource_cost <= limited.remaining_resource_budget);
        if let RobustCampaignPlanNode::Acquire {
            outcome_branches, ..
        } = result.selected_plan
        {
            assert!(outcome_branches
                .iter()
                .all(|branch| matches!(branch.continuation, RobustCampaignContinuation::Stop(_))));
        }
    }

    #[test]
    fn stopping_policy_reports_zero_maximum_path_cost() {
        let states = vec!["state-0".to_owned(), "state-1".to_owned()];
        let probabilities = vec![0.5, 0.5];
        let actions = vec!["choose-0".to_owned(), "choose-1".to_owned()];
        let utilities = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let option = RobustCampaignAcquisition {
            option_id: "optional-sensor".to_owned(),
            kind: InformationAcquisitionKind::IndependentCalibration,
            cost: 1.0,
            outcome_ids: vec!["reading-0".to_owned(), "reading-1".to_owned()],
            dependence_scope_receipt_id: RECEIPT.to_owned(),
            scenarios: vec![
                scenario("dependence-a", vec![vec![1.0, 0.0], vec![0.0, 1.0]]),
                scenario("dependence-b", vec![vec![1.0, 0.0], vec![0.0, 1.0]]),
            ],
        };
        let result = plan_non_anticipative_robust_campaign(
            &states,
            &probabilities,
            &actions,
            &utilities,
            0.0,
            &[option],
            RobustCampaignPlanningPolicy {
                max_steps: 2,
                remaining_resource_budget: 0.1,
                cost_to_utility_rate: 1.0,
                max_campaign_policy_trees: 8,
            },
        )
        .expect("stop policy should remain feasible");
        assert!(matches!(
            result.selected_plan,
            RobustCampaignPlanNode::Stop(_)
        ));
        assert_eq!(result.maximum_path_resource_cost, 0.0);
        assert_eq!(result.pathwise_resource_budget, 0.1);
        assert!(result.pathwise_budget_satisfied);
    }

    #[test]
    fn mismatched_dependence_scope_fails_closed() {
        let (states, probabilities, actions, utilities, options) = adaptive_fixture();
        let mut options = options;
        options[1].dependence_scope_receipt_id = "receipt:wrong-scope".to_owned();
        assert_eq!(
            plan_non_anticipative_robust_campaign(
                &states,
                &probabilities,
                &actions,
                &utilities,
                0.0,
                &options,
                policy(2),
            ),
            Err(ModelError::InvalidParameter(
                "robust_campaign_acquisitions_must_share_dependence_receipt"
            ))
        );
    }

    #[test]
    fn mismatched_scenario_order_fails_closed() {
        let (states, probabilities, actions, utilities, options) = adaptive_fixture();
        let mut options = options;
        options[1].scenarios.swap(0, 1);
        assert_eq!(
            plan_non_anticipative_robust_campaign(
                &states,
                &probabilities,
                &actions,
                &utilities,
                0.0,
                &options,
                policy(2),
            ),
            Err(ModelError::InvalidParameter(
                "robust_campaign_acquisitions_must_share_ordered_scenarios"
            ))
        );
    }

    #[test]
    fn complete_tree_cap_fails_closed() {
        let (states, probabilities, actions, utilities, options) = adaptive_fixture();
        let mut limited = policy(2);
        limited.max_campaign_policy_trees = 100;
        assert_eq!(
            plan_non_anticipative_robust_campaign(
                &states,
                &probabilities,
                &actions,
                &utilities,
                0.0,
                &options,
                limited,
            ),
            Err(ModelError::InvalidParameter(
                "robust_campaign_policy_tree_cap_exceeded"
            ))
        );
    }
}
