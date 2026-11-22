//! Exact finite scenario-tree planner for short scientific evidence campaigns.
//!
//! Each acquisition option is used at most once. Its outcome likelihood is
//! conditional on an expanded scientific state that must include relevant
//! common-cause latent variables. A single campaign-scoped receipt is required
//! across all qualified options; it must cover the joint likelihood/factorization
//! model, not merely assert that source IDs differ.
//!
//! The planner enumerates short scenario trees exactly, enforcing hard pathwise
//! resource budgets and charging acquisition costs in declared utility units.
//! The one-step finite EVSI policy is the myopic baseline. This module is a
//! small-instance oracle, not a scalable POMDP solver or physical likelihood
//! generator. If sequential likelihoods are not conditionally independent given
//! the expanded state (or otherwise represented by that receipt's joint model),
//! this factorized likelihood interface is not applicable.

use crate::decision_value::{
    InformationAcquisitionKind, InformationAcquisitionOption, InformationSourceQualification,
};
use crate::ModelError;

/// Frozen resource and search policy for finite campaign planning.
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceCampaignConfig {
    /// Maximum number of acquisition stages on any root-to-leaf path.
    pub max_steps: usize,
    /// Hard scenario-tree node expansion limit; exceeding it returns an error.
    pub max_nodes: usize,
    /// Hard pathwise resource budget in the option cost unit.
    pub remaining_budget: f64,
    /// Utility units charged per unit acquisition cost.
    pub cost_to_utility_rate: f64,
    /// Minimum improvement over stopping utility required to acquire information.
    pub minimum_net_improvement: f64,
    /// One receipt covering the campaign's complete joint likelihood model.
    pub campaign_model_receipt_id: String,
}

/// One possible outcome branch after an information acquisition.
#[derive(Debug, Clone, PartialEq)]
pub struct CampaignOutcomeBranch {
    pub outcome_index: usize,
    /// Probability of this outcome under the belief at its parent node.
    pub outcome_probability: f64,
    /// Optimal continuation plan conditional on this outcome.
    pub continuation: Box<CampaignPlanNode>,
}

/// One node in an exact finite evidence-campaign plan.
#[derive(Debug, Clone, PartialEq)]
pub enum CampaignPlanNode {
    Stop {
        selected_action_id: Option<String>,
        expected_utility: f64,
        abstained: bool,
    },
    Acquire {
        option_id: String,
        kind: InformationAcquisitionKind,
        acquisition_cost: f64,
        /// Expected value of the optimal continuation before this node's cost.
        expected_value_after_information: f64,
        /// Continuation value minus this node's acquisition cost.
        net_expected_utility: f64,
        /// Best utility from stopping immediately at this node.
        utility_if_stopped_now: f64,
        outcomes: Vec<CampaignOutcomeBranch>,
    },
}

/// Result of exact finite scenario-tree campaign planning.
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceCampaignPlan {
    pub root: CampaignPlanNode,
    /// Expected utility at the root, net of all pathwise acquisition costs.
    pub expected_net_utility: f64,
    pub nodes_expanded: usize,
    pub skipped_unqualified_options: usize,
    pub campaign_model_receipt_id: String,
}

/// Plan a short evidence campaign by exact scenario-tree enumeration.
///
/// State probabilities are normalized internally. Utilities are action-major:
/// action_utilities[action][state]. Information option likelihoods are
/// state-major: outcome_likelihoods[state][outcome].
///
/// Every qualified option must carry the same receipt ID as
/// campaign_model_receipt_id. This receipt must qualify the joint conditional
/// likelihood/factorization model across all options in the campaign. The
/// expanded state must include common-cause latent variables needed for
/// conditional factorization. Unknown or unqualified options are skipped.
///
/// Each option is one-shot. The planner may conditionally choose a later option
/// after observing an earlier outcome, but may not repeat an already-used ID.
/// The node limit is fail-closed: if exact enumeration exceeds max_nodes, no
/// partial plan is returned or represented as optimal.
pub fn plan_finite_evidence_campaign(
    action_ids: &[String],
    state_probabilities: &[f64],
    action_utilities: &[Vec<f64>],
    abstention_utility: f64,
    information_options: &[InformationAcquisitionOption],
    config: &EvidenceCampaignConfig,
) -> Result<EvidenceCampaignPlan, ModelError> {
    validate_decision_inputs(
        action_ids,
        state_probabilities,
        action_utilities,
        abstention_utility,
    )?;
    if config.max_nodes == 0 {
        return Err(ModelError::InvalidParameter(
            "campaign_node_budget_must_be_positive",
        ));
    }
    if !config.remaining_budget.is_finite()
        || !config.cost_to_utility_rate.is_finite()
        || !config.minimum_net_improvement.is_finite()
    {
        return Err(ModelError::NonFiniteValue("campaign_budget_or_cost_policy"));
    }
    if config.remaining_budget < 0.0
        || config.cost_to_utility_rate < 0.0
        || config.minimum_net_improvement < 0.0
    {
        return Err(ModelError::InvalidParameter(
            "campaign_budget_cost_rate_and_threshold_must_be_nonnegative",
        ));
    }
    if config.campaign_model_receipt_id.trim().is_empty() {
        return Err(ModelError::InvalidParameter(
            "campaign_model_receipt_id_must_be_nonempty",
        ));
    }

    let normalized_probabilities = normalize_probabilities(state_probabilities)?;
    let mut seen_options = std::collections::BTreeSet::new();
    let mut qualified_option_indices = Vec::new();
    let mut skipped_unqualified_options = 0usize;
    for (option_index, option) in information_options.iter().enumerate() {
        if option.option_id.trim().is_empty() || !seen_options.insert(option.option_id.as_str()) {
            return Err(ModelError::InvalidParameter(
                "campaign_option_ids_must_be_nonempty_and_unique",
            ));
        }
        if !option.cost.is_finite() {
            return Err(ModelError::NonFiniteValue("campaign_option_cost"));
        }
        if option.cost < 0.0 {
            return Err(ModelError::InvalidParameter(
                "campaign_option_cost_must_be_nonnegative",
            ));
        }
        match &option.source_qualification {
            InformationSourceQualification::Qualified { receipt_id } => {
                if receipt_id.trim().is_empty() {
                    return Err(ModelError::InvalidParameter(
                        "campaign_option_receipt_must_be_nonempty",
                    ));
                }
                if receipt_id != &config.campaign_model_receipt_id {
                    return Err(ModelError::InvalidParameter(
                        "campaign_options_must_share_joint_model_receipt",
                    ));
                }
                validate_option_likelihoods(option, state_probabilities.len())?;
                qualified_option_indices.push(option_index);
            }
            InformationSourceQualification::Unqualified => {
                skipped_unqualified_options += 1;
            }
        }
    }

    let mut solver = CampaignSolver {
        action_ids,
        action_utilities,
        abstention_utility,
        information_options,
        config,
        nodes_expanded: 0,
    };
    let max_steps = config.max_steps.min(qualified_option_indices.len());
    let (root, expected_net_utility) = solver.solve(
        &normalized_probabilities,
        &qualified_option_indices,
        config.remaining_budget,
        max_steps,
    )?;

    Ok(EvidenceCampaignPlan {
        root,
        expected_net_utility,
        nodes_expanded: solver.nodes_expanded,
        skipped_unqualified_options,
        campaign_model_receipt_id: config.campaign_model_receipt_id.clone(),
    })
}

struct CampaignSolver<'a> {
    action_ids: &'a [String],
    action_utilities: &'a [Vec<f64>],
    abstention_utility: f64,
    information_options: &'a [InformationAcquisitionOption],
    config: &'a EvidenceCampaignConfig,
    nodes_expanded: usize,
}

impl CampaignSolver<'_> {
    fn solve(
        &mut self,
        state_probabilities: &[f64],
        remaining_option_indices: &[usize],
        remaining_budget: f64,
        remaining_steps: usize,
    ) -> Result<(CampaignPlanNode, f64), ModelError> {
        if self.nodes_expanded >= self.config.max_nodes {
            return Err(ModelError::InvalidParameter(
                "campaign_planner_node_budget_exceeded",
            ));
        }
        self.nodes_expanded += 1;

        let (best_action_index, best_action_utility) =
            best_current_action(state_probabilities, self.action_utilities);
        let stop_utility = best_action_utility.max(self.abstention_utility);
        let stop_action_id = (best_action_utility > self.abstention_utility)
            .then(|| self.action_ids[best_action_index].clone());
        let mut best_node = CampaignPlanNode::Stop {
            selected_action_id: stop_action_id,
            expected_utility: stop_utility,
            abstained: best_action_utility <= self.abstention_utility,
        };
        let mut best_value = stop_utility;

        if remaining_steps == 0 || remaining_option_indices.is_empty() {
            return Ok((best_node, best_value));
        }

        for option_index in remaining_option_indices {
            let option = &self.information_options[*option_index];
            if option.cost > remaining_budget {
                continue;
            }
            let mut next_option_indices = Vec::with_capacity(remaining_option_indices.len() - 1);
            next_option_indices.extend(
                remaining_option_indices
                    .iter()
                    .copied()
                    .filter(|candidate| candidate != option_index),
            );

            let outcome_count = option.outcome_likelihoods[0].len();
            let mut expected_value_after_information = 0.0;
            let mut outcomes = Vec::with_capacity(outcome_count);
            for outcome_index in 0..outcome_count {
                let outcome_probability = state_probabilities
                    .iter()
                    .enumerate()
                    .map(|(state_index, probability)| {
                        probability * option.outcome_likelihoods[state_index][outcome_index]
                    })
                    .sum::<f64>();
                if outcome_probability <= 0.0 {
                    continue;
                }
                if !outcome_probability.is_finite() || outcome_probability > 1.0 + 1.0e-10 {
                    return Err(ModelError::InvalidParameter(
                        "campaign_outcome_probability_invalid",
                    ));
                }

                let mut posterior_probabilities: Vec<f64> = state_probabilities
                    .iter()
                    .enumerate()
                    .map(|(state_index, probability)| {
                        probability * option.outcome_likelihoods[state_index][outcome_index]
                            / outcome_probability
                    })
                    .collect();
                let posterior_sum = posterior_probabilities.iter().sum::<f64>();
                if !posterior_sum.is_finite() || posterior_sum <= 0.0 {
                    return Err(ModelError::InvalidParameter(
                        "campaign_posterior_probability_sum_invalid",
                    ));
                }
                for probability in &mut posterior_probabilities {
                    *probability /= posterior_sum;
                }

                let (continuation, continuation_value) = self.solve(
                    &posterior_probabilities,
                    &next_option_indices,
                    remaining_budget - option.cost,
                    remaining_steps - 1,
                )?;
                expected_value_after_information += outcome_probability * continuation_value;
                outcomes.push(CampaignOutcomeBranch {
                    outcome_index,
                    outcome_probability,
                    continuation: Box::new(continuation),
                });
            }

            let cost_in_utility_units = option.cost * self.config.cost_to_utility_rate;
            if !cost_in_utility_units.is_finite() {
                return Err(ModelError::InvalidParameter(
                    "campaign_option_cost_conversion_nonfinite",
                ));
            }
            let net_expected_utility = expected_value_after_information - cost_in_utility_units;
            if !net_expected_utility.is_finite() || !expected_value_after_information.is_finite() {
                return Err(ModelError::InvalidParameter(
                    "campaign_expected_utility_nonfinite",
                ));
            }
            if net_expected_utility > best_value + self.config.minimum_net_improvement {
                best_value = net_expected_utility;
                best_node = CampaignPlanNode::Acquire {
                    option_id: option.option_id.clone(),
                    kind: option.kind,
                    acquisition_cost: option.cost,
                    expected_value_after_information,
                    net_expected_utility,
                    utility_if_stopped_now: stop_utility,
                    outcomes,
                };
            }
        }

        Ok((best_node, best_value))
    }
}

fn validate_decision_inputs(
    action_ids: &[String],
    state_probabilities: &[f64],
    action_utilities: &[Vec<f64>],
    abstention_utility: f64,
) -> Result<(), ModelError> {
    let state_count = state_probabilities.len();
    if state_count == 0
        || action_ids.is_empty()
        || action_utilities.len() != action_ids.len()
        || action_utilities.iter().any(|row| row.len() != state_count)
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    if action_ids
        .iter()
        .any(|action_id| action_id.trim().is_empty())
    {
        return Err(ModelError::InvalidParameter(
            "campaign_action_ids_must_be_nonempty",
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    if action_ids
        .iter()
        .any(|action_id| !seen.insert(action_id.as_str()))
    {
        return Err(ModelError::InvalidParameter(
            "campaign_action_ids_must_be_unique",
        ));
    }
    if !abstention_utility.is_finite()
        || action_utilities
            .iter()
            .flatten()
            .any(|utility| !utility.is_finite())
    {
        return Err(ModelError::NonFiniteValue("campaign_action_utility"));
    }
    Ok(())
}

fn normalize_probabilities(probabilities: &[f64]) -> Result<Vec<f64>, ModelError> {
    if probabilities
        .iter()
        .any(|probability| !probability.is_finite())
    {
        return Err(ModelError::NonFiniteValue("campaign_state_probability"));
    }
    if probabilities.iter().any(|probability| *probability < 0.0) {
        return Err(ModelError::InvalidParameter(
            "campaign_state_probabilities_must_be_nonnegative",
        ));
    }
    let total = probabilities.iter().sum::<f64>();
    if !total.is_finite() {
        return Err(ModelError::InvalidParameter(
            "campaign_state_probability_sum_nonfinite",
        ));
    }
    if total <= 0.0 {
        return Err(ModelError::NonPositiveValue(
            "campaign_state_probability_sum",
        ));
    }
    Ok(probabilities
        .iter()
        .map(|probability| probability / total)
        .collect())
}

fn best_current_action(state_probabilities: &[f64], action_utilities: &[Vec<f64>]) -> (usize, f64) {
    let mut best_index = 0usize;
    let mut best_utility = f64::NEG_INFINITY;
    for (action_index, utilities) in action_utilities.iter().enumerate() {
        let expected_utility = state_probabilities
            .iter()
            .zip(utilities.iter())
            .map(|(probability, utility)| probability * utility)
            .sum::<f64>();
        if expected_utility > best_utility {
            best_index = action_index;
            best_utility = expected_utility;
        }
    }
    (best_index, best_utility)
}

fn validate_option_likelihoods(
    option: &InformationAcquisitionOption,
    state_count: usize,
) -> Result<(), ModelError> {
    if option.outcome_likelihoods.len() != state_count || option.outcome_likelihoods.is_empty() {
        return Err(ModelError::MismatchedDataLengths);
    }
    let outcome_count = option.outcome_likelihoods[0].len();
    if outcome_count == 0
        || option
            .outcome_likelihoods
            .iter()
            .any(|row| row.len() != outcome_count)
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    for row in &option.outcome_likelihoods {
        if row.iter().any(|likelihood| !likelihood.is_finite()) {
            return Err(ModelError::NonFiniteValue("campaign_outcome_likelihood"));
        }
        if row
            .iter()
            .any(|likelihood| *likelihood < 0.0 || *likelihood > 1.0)
        {
            return Err(ModelError::InvalidParameter(
                "campaign_outcome_likelihood_must_be_in_[0,1]",
            ));
        }
        let row_sum = row.iter().sum::<f64>();
        if (row_sum - 1.0).abs() > 1.0e-10 {
            return Err(ModelError::InvalidParameter(
                "campaign_likelihood_row_must_sum_to_one",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECEIPT: &str = "receipt:joint-campaign-likelihood-v1";

    fn actions() -> Vec<String> {
        vec!["choose-a".to_owned(), "choose-b".to_owned()]
    }

    fn option(id: &str, cost: f64, accuracy: f64) -> InformationAcquisitionOption {
        InformationAcquisitionOption {
            option_id: id.to_owned(),
            kind: InformationAcquisitionKind::LowFidelitySimulation,
            cost,
            outcome_likelihoods: vec![
                vec![accuracy, 1.0 - accuracy],
                vec![1.0 - accuracy, accuracy],
            ],
            source_qualification: InformationSourceQualification::Qualified {
                receipt_id: RECEIPT.to_owned(),
            },
        }
    }

    fn config(max_steps: usize) -> EvidenceCampaignConfig {
        EvidenceCampaignConfig {
            max_steps,
            max_nodes: 10_000,
            remaining_budget: 0.15,
            cost_to_utility_rate: 1.0,
            minimum_net_improvement: 0.0,
            campaign_model_receipt_id: RECEIPT.to_owned(),
        }
    }

    fn count_acquisitions_on_longest_path(plan: &CampaignPlanNode) -> usize {
        match plan {
            CampaignPlanNode::Stop { .. } => 0,
            CampaignPlanNode::Acquire { outcomes, .. } => {
                1 + outcomes
                    .iter()
                    .map(|branch| count_acquisitions_on_longest_path(&branch.continuation))
                    .max()
                    .unwrap_or(0)
            }
        }
    }

    fn expected_acquisition_count(plan: &CampaignPlanNode) -> f64 {
        match plan {
            CampaignPlanNode::Stop { .. } => 0.0,
            CampaignPlanNode::Acquire { outcomes, .. } => {
                1.0 + outcomes
                    .iter()
                    .map(|branch| {
                        branch.outcome_probability
                            * expected_acquisition_count(&branch.continuation)
                    })
                    .sum::<f64>()
            }
        }
    }

    fn probability_of_acquisition_at_depth(plan: &CampaignPlanNode, depth: usize) -> f64 {
        match plan {
            CampaignPlanNode::Stop { .. } => 0.0,
            CampaignPlanNode::Acquire { .. } if depth == 1 => 1.0,
            CampaignPlanNode::Acquire { outcomes, .. } => outcomes
                .iter()
                .map(|branch| {
                    branch.outcome_probability
                        * probability_of_acquisition_at_depth(&branch.continuation, depth - 1)
                })
                .sum(),
        }
    }

    #[test]
    fn exact_tree_lookahead_finds_conditional_information_value_missed_by_one_step() {
        let options = vec![
            option("test-1", 0.05, 0.8),
            option("test-2", 0.05, 0.8),
            option("test-3", 0.05, 0.8),
        ];
        let one_step = plan_finite_evidence_campaign(
            &actions(),
            &[0.5, 0.5],
            &[vec![1.0, -1.0], vec![-1.0, 1.0]],
            0.0,
            &options,
            &config(1),
        )
        .expect("one-step plan");
        let three_step = plan_finite_evidence_campaign(
            &actions(),
            &[0.5, 0.5],
            &[vec![1.0, -1.0], vec![-1.0, 1.0]],
            0.0,
            &options,
            &config(3),
        )
        .expect("three-step lookahead plan");

        assert!(
            three_step.expected_net_utility > one_step.expected_net_utility + 0.1,
            "one-step={}, three-step={}",
            one_step.expected_net_utility,
            three_step.expected_net_utility
        );
        assert!((one_step.expected_net_utility - 0.55).abs() < 1.0e-12);
        assert!((three_step.expected_net_utility - 0.676).abs() < 1.0e-12);

        let myopic_evsi = crate::decision_value::recommend_finite_decision_step(
            &actions(),
            &[0.5, 0.5],
            &[vec![1.0, -1.0], vec![-1.0, 1.0]],
            0.0,
            &[options[0].clone()],
            crate::decision_value::InformationAcquisitionPolicy {
                remaining_budget: 0.15,
                cost_to_utility_rate: 1.0,
                minimum_net_benefit: 0.0,
            },
        )
        .expect("one-step finite EVSI baseline");
        match myopic_evsi {
            crate::decision_value::FiniteDecisionRecommendation::AcquireInformation {
                expected_net_benefit_of_sampling,
                ..
            } => assert!(
                (one_step.expected_net_utility - expected_net_benefit_of_sampling).abs() < 1.0e-12
            ),
            other => panic!("expected myopic information acquisition, got {other:?}"),
        }
        assert_eq!(count_acquisitions_on_longest_path(&one_step.root), 1);
        assert_eq!(count_acquisitions_on_longest_path(&three_step.root), 3);
        assert!(three_step.nodes_expanded > one_step.nodes_expanded);
    }

    #[test]
    fn expanded_common_cause_state_prevents_false_independent_remeasurement() {
        // State order: physical A/calibration-good, physical B/calibration-good,
        // physical A/calibration-bad, physical B/calibration-bad.
        // Repeated observations are independent only after conditioning on
        // this expanded state, because both share the same latent calibration.
        let expanded_actions = vec!["choose-a".to_owned(), "choose-b".to_owned()];
        let expanded_utilities = vec![vec![1.0, -1.0, 1.0, -1.0], vec![-1.0, 1.0, -1.0, 1.0]];
        let shared_likelihoods = vec![
            vec![0.99, 0.01],
            vec![0.01, 0.99],
            vec![0.01, 0.99],
            vec![0.99, 0.01],
        ];
        let options: Vec<InformationAcquisitionOption> = (1..=3)
            .map(|index| InformationAcquisitionOption {
                option_id: format!("same-calibration-test-{index}"),
                kind: InformationAcquisitionKind::LowFidelitySimulation,
                cost: 0.002,
                outcome_likelihoods: shared_likelihoods.clone(),
                source_qualification: InformationSourceQualification::Qualified {
                    receipt_id: RECEIPT.to_owned(),
                },
            })
            .collect();
        let mut policy = config(3);
        policy.remaining_budget = 0.006;
        let plan = plan_finite_evidence_campaign(
            &expanded_actions,
            &[0.35, 0.35, 0.15, 0.15],
            &expanded_utilities,
            0.0,
            &options,
            &policy,
        )
        .expect("shared-calibration campaign");

        assert!((plan.expected_net_utility - 0.395_722).abs() < 1.0e-9);
        assert_eq!(count_acquisitions_on_longest_path(&plan.root), 3);
        assert!((expected_acquisition_count(&plan.root) - 2.0198).abs() < 1.0e-9);
        assert!((probability_of_acquisition_at_depth(&plan.root, 3) - 0.0198).abs() < 1.0e-9);
    }

    #[test]
    fn one_step_plan_matches_closed_form_evsi() {
        let option = option("single-test", 0.05, 0.8);
        let plan = plan_finite_evidence_campaign(
            &actions(),
            &[0.5, 0.5],
            &[vec![1.0, -1.0], vec![-1.0, 1.0]],
            0.0,
            &[option],
            &config(1),
        )
        .expect("one-step exact EVSI");
        assert!((plan.expected_net_utility - 0.55).abs() < 1.0e-12);
    }

    #[test]
    fn hard_budget_prevents_over_budget_campaign_sequences() {
        let mut policy = config(3);
        policy.remaining_budget = 0.09;
        let options = vec![option("test-1", 0.05, 0.8), option("test-2", 0.05, 0.8)];
        let plan = plan_finite_evidence_campaign(
            &actions(),
            &[0.5, 0.5],
            &[vec![1.0, -1.0], vec![-1.0, 1.0]],
            0.0,
            &options,
            &policy,
        )
        .expect("budget-limited campaign");
        assert_eq!(count_acquisitions_on_longest_path(&plan.root), 1);
    }

    #[test]
    fn unqualified_options_are_skipped_but_receipt_scope_mismatch_fails_closed() {
        let mut unknown = option("unknown", 0.05, 0.99);
        unknown.source_qualification = InformationSourceQualification::Unqualified;
        let plan = plan_finite_evidence_campaign(
            &actions(),
            &[0.5, 0.5],
            &[vec![1.0, -1.0], vec![-1.0, 1.0]],
            0.0,
            &[unknown],
            &config(2),
        )
        .expect("unknown option skipped");
        assert_eq!(plan.skipped_unqualified_options, 1);
        assert!(matches!(plan.root, CampaignPlanNode::Stop { .. }));

        let mut mismatched = option("mismatched", 0.05, 0.8);
        mismatched.source_qualification = InformationSourceQualification::Qualified {
            receipt_id: "receipt:other-scope".to_owned(),
        };
        assert_eq!(
            plan_finite_evidence_campaign(
                &actions(),
                &[0.5, 0.5],
                &[vec![1.0, -1.0], vec![-1.0, 1.0]],
                0.0,
                &[mismatched],
                &config(2),
            ),
            Err(ModelError::InvalidParameter(
                "campaign_options_must_share_joint_model_receipt"
            ))
        );
    }

    #[test]
    fn planner_fails_closed_when_node_budget_is_exceeded() {
        let mut policy = config(2);
        policy.max_nodes = 1;
        assert_eq!(
            plan_finite_evidence_campaign(
                &actions(),
                &[0.5, 0.5],
                &[vec![1.0, -1.0], vec![-1.0, 1.0]],
                0.0,
                &[option("test", 0.05, 0.8)],
                &policy,
            ),
            Err(ModelError::InvalidParameter(
                "campaign_planner_node_budget_exceeded"
            ))
        );
    }
}
