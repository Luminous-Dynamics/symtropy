//! Finite robust decision value for experiment designs under source ambiguity.
//!
//! Unlike EIG-only ranking, this module evaluates whether an experiment improves
//! a downstream action when the dependence scenario is not observable at decision
//! time. It searches one deterministic outcome-to-action policy shared by every
//! admitted scenario (the non-anticipativity constraint).
//!
//! The reference is exact over the supplied finite scenario set and finite
//! deterministic policy class, up to ordinary f64 arithmetic. It is not a
//! randomized-policy LP, continuous ambiguity-set solver, formal certificate,
//! or receipt authenticator.

use crate::ModelError;
use std::collections::BTreeSet;

/// Finite decision problem shared by every candidate experiment.
#[derive(Debug, Clone, PartialEq)]
pub struct RobustDecisionProblem {
    pub state_ids: Vec<String>,
    pub action_ids: Vec<String>,
    /// Prior probability for each state.
    pub state_probabilities: Vec<f64>,
    /// Action-major utility matrix: [action][state].
    pub action_utilities: Vec<Vec<f64>>,
}

/// One admissible source/dependence scenario and its state-conditional
/// observation model. Likelihood rows are state-major: [state][outcome].
#[derive(Debug, Clone, PartialEq)]
pub struct RobustDecisionLikelihoodScenario {
    pub scenario_id: String,
    pub outcome_likelihoods: Vec<Vec<f64>>,
}

/// One candidate experiment with a common scenario set.
#[derive(Debug, Clone, PartialEq)]
pub struct RobustDecisionDesignCandidate {
    pub candidate_id: String,
    pub outcome_ids: Vec<String>,
    /// Resource cost in the caller's declared resource unit.
    pub resource_cost: f64,
    /// External reference to evidence qualifying this complete dependence set.
    /// The identifier is not authenticated by this numerical kernel.
    pub dependence_scope_receipt_id: String,
    pub scenarios: Vec<RobustDecisionLikelihoodScenario>,
}

/// Hard budget and exact-enumeration policy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RobustDecisionDesignPolicy {
    pub remaining_resource_budget: f64,
    /// Utility units charged per resource-cost unit.
    pub cost_to_utility_rate: f64,
    /// Hard cap across all candidate policies in one ranking call.
    pub max_policy_evaluations: usize,
}

/// Utility and information metrics for one candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct RobustDecisionDesignScore {
    pub candidate_id: String,
    pub outcome_ids: Vec<String>,
    /// Action chosen for each outcome by one policy shared across scenarios.
    pub robust_policy_action_ids_by_outcome: Vec<String>,
    pub scenario_ids: Vec<String>,
    pub robust_policy_expected_utilities_by_scenario: Vec<f64>,
    /// Best expected utility if the hidden scenario were revealed before acting.
    pub clairvoyant_expected_utilities_by_scenario: Vec<f64>,
    pub prior_best_action_id: String,
    pub prior_best_expected_utility: f64,
    pub robust_post_information_utility: f64,
    pub robust_expected_value_of_sample_information: f64,
    pub clairvoyant_post_information_utility_upper_bound: f64,
    pub non_anticipativity_gap: f64,
    pub expected_information_gain_nats_by_scenario: Vec<f64>,
    pub worst_case_expected_information_gain_nats: f64,
    pub best_case_expected_information_gain_nats: f64,
    pub resource_cost: f64,
    pub cost_in_utility_units: f64,
    pub budget_feasible: bool,
    /// None when the candidate exceeds the remaining hard resource budget.
    pub net_robust_decision_value: Option<f64>,
    pub deterministic_policy_count: usize,
    pub policy_evaluations: usize,
    pub finite_scenario_exact: bool,
    /// Always false: this is not a formal interval or SDP certificate.
    pub formal_certificate: bool,
}

/// Ranking under one finite dependence ambiguity set.
#[derive(Debug, Clone, PartialEq)]
pub struct RobustDecisionDesignRanking {
    pub dependence_scope_receipt_id: String,
    pub prior_best_action_id: String,
    pub prior_best_expected_utility: f64,
    pub recommended_candidate_id: Option<String>,
    pub total_policy_evaluations: usize,
    pub max_policy_evaluations: usize,
    pub scores: Vec<RobustDecisionDesignScore>,
}

/// Rank candidate experiments by worst-case decision utility using a single
/// non-anticipative deterministic policy for every admitted dependence scenario.
///
/// For each design d, the optimized quantity is:
///
/// max_{pi: outcomes -> actions} min_s E[U(pi(Y), state) | d, s].
///
/// This is intentionally not min_s max_pi: the latter lets the action policy
/// observe the hidden dependence scenario. That clairvoyant quantity is reported
/// separately as an upper bound and as a non-anticipativity diagnostic.
///
/// Robust EVSI is robust post-information utility less the best prior action
/// utility. Net robust value additionally subtracts resource cost converted to
/// utility units. Candidates outside the hard resource budget are scored but
/// cannot be recommended.
///
/// The max_policy_evaluations field is a hard cumulative cap. The function
/// returns an error rather than a partial ranking if the exact deterministic
/// policy search would exceed it.
pub fn rank_designs_by_robust_decision_value(
    problem: &RobustDecisionProblem,
    candidates: &[RobustDecisionDesignCandidate],
    policy: RobustDecisionDesignPolicy,
) -> Result<RobustDecisionDesignRanking, ModelError> {
    validate_decision_problem(problem)?;
    if candidates.is_empty() {
        return Err(ModelError::MismatchedDataLengths);
    }
    validate_policy(policy)?;

    let prior_best = prior_best_action(problem)?;
    let expected_scenario_ids = validate_candidate_family(candidates)?;
    let common_receipt = candidates[0].dependence_scope_receipt_id.clone();

    let mut total_policy_evaluations = 0usize;
    let mut scores = Vec::with_capacity(candidates.len());

    for candidate in candidates {
        validate_candidate(problem, candidate, &expected_scenario_ids)?;

        let action_count = problem.action_ids.len();
        let outcome_count = candidate.outcome_ids.len();
        let policy_count = deterministic_policy_count(action_count, outcome_count)?;
        total_policy_evaluations = total_policy_evaluations
            .checked_add(policy_count)
            .ok_or(ModelError::InvalidParameter(
                "robust_decision_policy_evaluation_count_overflow",
            ))?;
        if total_policy_evaluations > policy.max_policy_evaluations {
            return Err(ModelError::InvalidParameter(
                "robust_decision_policy_evaluation_cap_exceeded",
            ));
        }

        let scenario_count = candidate.scenarios.len();
        let mut best_robust_utility = f64::NEG_INFINITY;
        let mut best_policy = vec![0usize; outcome_count];
        let mut clairvoyant_by_scenario = vec![f64::NEG_INFINITY; scenario_count];
        let mut policy_indices = vec![0usize; outcome_count];

        for _ in 0..policy_count {
            let scenario_utilities =
                scenario_policy_utilities(problem, candidate, &policy_indices)?;
            let worst_case_utility = scenario_utilities
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min);

            if worst_case_utility > best_robust_utility {
                best_robust_utility = worst_case_utility;
                best_policy.clone_from(&policy_indices);
            }

            for (scenario_index, utility) in scenario_utilities.iter().enumerate() {
                clairvoyant_by_scenario[scenario_index] =
                    clairvoyant_by_scenario[scenario_index].max(*utility);
            }

            increment_policy(&mut policy_indices, action_count);
        }

        let robust_policy_utilities =
            scenario_policy_utilities(problem, candidate, &best_policy)?;
        let clairvoyant_post_information_utility_upper_bound = clairvoyant_by_scenario
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let robust_evsi = best_robust_utility - prior_best.expected_utility;
        let non_anticipativity_gap =
            (clairvoyant_post_information_utility_upper_bound - best_robust_utility).max(0.0);
        if !robust_evsi.is_finite()
            || !non_anticipativity_gap.is_finite()
            || !clairvoyant_post_information_utility_upper_bound.is_finite()
        {
            return Err(ModelError::NonFiniteValue(
                "robust_decision_value_or_non_anticipativity_gap",
            ));
        }

        let eig_by_scenario = candidate
            .scenarios
            .iter()
            .map(|scenario| {
                finite_state_expected_information_gain(
                    &problem.state_probabilities,
                    &scenario.outcome_likelihoods,
                    outcome_count,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let worst_case_eig = eig_by_scenario
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let best_case_eig = eig_by_scenario
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);

        let budget_feasible = candidate.resource_cost <= policy.remaining_resource_budget;
        let cost_in_utility_units = candidate.resource_cost * policy.cost_to_utility_rate;
        if !cost_in_utility_units.is_finite() {
            return Err(ModelError::NonFiniteValue(
                "robust_decision_cost_in_utility_units",
            ));
        }
        let net_robust_decision_value = if budget_feasible {
            let net_value = robust_evsi - cost_in_utility_units;
            if !net_value.is_finite() {
                return Err(ModelError::NonFiniteValue(
                    "robust_decision_net_value",
                ));
            }
            Some(net_value)
        } else {
            None
        };

        scores.push(RobustDecisionDesignScore {
            candidate_id: candidate.candidate_id.clone(),
            outcome_ids: candidate.outcome_ids.clone(),
            robust_policy_action_ids_by_outcome: best_policy
                .iter()
                .map(|action_index| problem.action_ids[*action_index].clone())
                .collect(),
            scenario_ids: expected_scenario_ids.clone(),
            robust_policy_expected_utilities_by_scenario: robust_policy_utilities,
            clairvoyant_expected_utilities_by_scenario: clairvoyant_by_scenario,
            prior_best_action_id: prior_best.action_id.clone(),
            prior_best_expected_utility: prior_best.expected_utility,
            robust_post_information_utility: best_robust_utility,
            robust_expected_value_of_sample_information: robust_evsi,
            clairvoyant_post_information_utility_upper_bound,
            non_anticipativity_gap,
            expected_information_gain_nats_by_scenario: eig_by_scenario,
            worst_case_expected_information_gain_nats: worst_case_eig,
            best_case_expected_information_gain_nats: best_case_eig,
            resource_cost: candidate.resource_cost,
            cost_in_utility_units,
            budget_feasible,
            net_robust_decision_value,
            deterministic_policy_count: policy_count,
            policy_evaluations: policy_count,
            finite_scenario_exact: true,
            formal_certificate: false,
        });
    }

    scores.sort_by(|left, right| {
        let left_value = left
            .net_robust_decision_value
            .unwrap_or(f64::NEG_INFINITY);
        let right_value = right
            .net_robust_decision_value
            .unwrap_or(f64::NEG_INFINITY);
        right_value
            .total_cmp(&left_value)
            .then_with(|| left.candidate_id.cmp(&right.candidate_id))
    });

    let recommended_candidate_id = scores
        .iter()
        .find(|score| {
            score
                .net_robust_decision_value
                .is_some_and(|value| value > 0.0)
        })
        .map(|score| score.candidate_id.clone());

    Ok(RobustDecisionDesignRanking {
        dependence_scope_receipt_id: common_receipt,
        prior_best_action_id: prior_best.action_id,
        prior_best_expected_utility: prior_best.expected_utility,
        recommended_candidate_id,
        total_policy_evaluations,
        max_policy_evaluations: policy.max_policy_evaluations,
        scores,
    })
}

#[derive(Debug, Clone, PartialEq)]
struct PriorBestAction {
    action_id: String,
    expected_utility: f64,
}

fn prior_best_action(problem: &RobustDecisionProblem) -> Result<PriorBestAction, ModelError> {
    let mut best_index = 0usize;
    let mut best_utility = f64::NEG_INFINITY;
    for (action_index, utilities) in problem.action_utilities.iter().enumerate() {
        let expected_utility = utilities
            .iter()
            .zip(problem.state_probabilities.iter())
            .map(|(utility, probability)| utility * probability)
            .sum::<f64>();
        if !expected_utility.is_finite() {
            return Err(ModelError::NonFiniteValue(
                "robust_decision_prior_expected_utility",
            ));
        }
        if expected_utility > best_utility {
            best_index = action_index;
            best_utility = expected_utility;
        }
    }
    Ok(PriorBestAction {
        action_id: problem.action_ids[best_index].clone(),
        expected_utility: best_utility,
    })
}

fn scenario_policy_utilities(
    problem: &RobustDecisionProblem,
    candidate: &RobustDecisionDesignCandidate,
    policy_by_outcome: &[usize],
) -> Result<Vec<f64>, ModelError> {
    candidate
        .scenarios
        .iter()
        .map(|scenario| {
            let mut expected_utility = 0.0;
            for (state_index, state_probability) in
                problem.state_probabilities.iter().enumerate()
            {
                for (outcome_index, likelihood) in
                    scenario.outcome_likelihoods[state_index].iter().enumerate()
                {
                    let action_index = policy_by_outcome[outcome_index];
                    expected_utility += state_probability
                        * likelihood
                        * problem.action_utilities[action_index][state_index];
                }
            }
            if !expected_utility.is_finite() {
                return Err(ModelError::NonFiniteValue(
                    "robust_decision_scenario_expected_utility",
                ));
            }
            Ok(expected_utility)
        })
        .collect()
}

fn finite_state_expected_information_gain(
    state_probabilities: &[f64],
    outcome_likelihoods: &[Vec<f64>],
    outcome_count: usize,
) -> Result<f64, ModelError> {
    let mut outcome_probabilities = vec![0.0; outcome_count];
    for (state_index, state_probability) in state_probabilities.iter().enumerate() {
        for (outcome_index, likelihood) in
            outcome_likelihoods[state_index].iter().enumerate()
        {
            outcome_probabilities[outcome_index] += state_probability * likelihood;
        }
    }

    let mut information_gain = 0.0;
    for (state_index, state_probability) in state_probabilities.iter().enumerate() {
        if *state_probability == 0.0 {
            continue;
        }
        for (outcome_index, likelihood) in
            outcome_likelihoods[state_index].iter().enumerate()
        {
            let joint_probability = state_probability * likelihood;
            let marginal_probability = outcome_probabilities[outcome_index];
            if joint_probability > 0.0 && marginal_probability > 0.0 {
                information_gain +=
                    joint_probability * (likelihood.ln() - marginal_probability.ln());
            }
        }
    }
    if !information_gain.is_finite() {
        return Err(ModelError::NonFiniteValue(
            "robust_decision_expected_information_gain",
        ));
    }
    Ok(information_gain.max(0.0))
}

fn deterministic_policy_count(
    action_count: usize,
    outcome_count: usize,
) -> Result<usize, ModelError> {
    let mut count = 1usize;
    for _ in 0..outcome_count {
        count = count.checked_mul(action_count).ok_or(ModelError::InvalidParameter(
            "robust_decision_policy_count_overflow",
        ))?;
    }
    Ok(count)
}

fn increment_policy(policy: &mut [usize], action_count: usize) {
    for action_index in policy.iter_mut() {
        *action_index += 1;
        if *action_index < action_count {
            break;
        }
        *action_index = 0;
    }
}

fn validate_decision_problem(problem: &RobustDecisionProblem) -> Result<(), ModelError> {
    let state_count = problem.state_ids.len();
    let action_count = problem.action_ids.len();
    if state_count == 0
        || action_count == 0
        || problem.state_probabilities.len() != state_count
        || problem.action_utilities.len() != action_count
        || problem
            .action_utilities
            .iter()
            .any(|row| row.len() != state_count)
    {
        return Err(ModelError::MismatchedDataLengths);
    }

    validate_unique_nonempty_ids(&problem.state_ids, "robust_decision_state_ids")?;
    validate_unique_nonempty_ids(&problem.action_ids, "robust_decision_action_ids")?;

    if problem
        .state_probabilities
        .iter()
        .any(|probability| !probability.is_finite())
    {
        return Err(ModelError::NonFiniteValue(
            "robust_decision_state_probability",
        ));
    }
    if problem
        .state_probabilities
        .iter()
        .any(|probability| *probability < 0.0)
    {
        return Err(ModelError::InvalidParameter(
            "robust_decision_state_probabilities_must_be_nonnegative",
        ));
    }
    let probability_sum = problem.state_probabilities.iter().sum::<f64>();
    if !probability_sum.is_finite() || (probability_sum - 1.0).abs() > 1.0e-10 {
        return Err(ModelError::InvalidParameter(
            "robust_decision_state_probabilities_must_sum_to_one",
        ));
    }
    if problem
        .action_utilities
        .iter()
        .flatten()
        .any(|utility| !utility.is_finite())
    {
        return Err(ModelError::NonFiniteValue("robust_decision_action_utility"));
    }
    Ok(())
}

fn validate_policy(policy: RobustDecisionDesignPolicy) -> Result<(), ModelError> {
    if !policy.remaining_resource_budget.is_finite()
        || !policy.cost_to_utility_rate.is_finite()
    {
        return Err(ModelError::NonFiniteValue("robust_decision_policy_budget_or_rate"));
    }
    if policy.remaining_resource_budget < 0.0 || policy.cost_to_utility_rate < 0.0 {
        return Err(ModelError::InvalidParameter(
            "robust_decision_policy_budget_and_rate_must_be_nonnegative",
        ));
    }
    if policy.max_policy_evaluations == 0 {
        return Err(ModelError::InvalidParameter(
            "robust_decision_policy_evaluation_cap_must_be_positive",
        ));
    }
    Ok(())
}

fn validate_candidate_family(
    candidates: &[RobustDecisionDesignCandidate],
) -> Result<Vec<String>, ModelError> {
    let mut candidate_ids = BTreeSet::new();
    let first = &candidates[0];
    if first.dependence_scope_receipt_id.trim().is_empty() {
        return Err(ModelError::InvalidParameter(
            "robust_decision_dependence_receipt_must_be_nonempty",
        ));
    }
    let expected_scenario_ids = first
        .scenarios
        .iter()
        .map(|scenario| scenario.scenario_id.clone())
        .collect::<Vec<_>>();
    if expected_scenario_ids.is_empty() {
        return Err(ModelError::MismatchedDataLengths);
    }
    validate_unique_nonempty_ids(&expected_scenario_ids, "robust_decision_scenario_ids")?;

    for candidate in candidates {
        validate_unique_nonempty_ids(
            std::slice::from_ref(&candidate.candidate_id),
            "robust_decision_candidate_id",
        )?;
        if !candidate_ids.insert(candidate.candidate_id.as_str()) {
            return Err(ModelError::InvalidParameter(
                "robust_decision_candidate_ids_must_be_unique",
            ));
        }
        if candidate.dependence_scope_receipt_id != first.dependence_scope_receipt_id {
            return Err(ModelError::InvalidParameter(
                "robust_decision_candidates_must_share_dependence_receipt",
            ));
        }
        let scenario_ids = candidate
            .scenarios
            .iter()
            .map(|scenario| scenario.scenario_id.clone())
            .collect::<Vec<_>>();
        if scenario_ids != expected_scenario_ids {
            return Err(ModelError::InvalidParameter(
                "robust_decision_candidates_must_share_ordered_scenarios",
            ));
        }
    }
    Ok(expected_scenario_ids)
}

fn validate_candidate(
    problem: &RobustDecisionProblem,
    candidate: &RobustDecisionDesignCandidate,
    expected_scenario_ids: &[String],
) -> Result<(), ModelError> {
    if candidate.outcome_ids.is_empty() {
        return Err(ModelError::MismatchedDataLengths);
    }
    validate_unique_nonempty_ids(&candidate.outcome_ids, "robust_decision_outcome_ids")?;
    if !candidate.resource_cost.is_finite() {
        return Err(ModelError::NonFiniteValue("robust_decision_resource_cost"));
    }
    if candidate.resource_cost < 0.0 {
        return Err(ModelError::InvalidParameter(
            "robust_decision_resource_cost_must_be_nonnegative",
        ));
    }
    if candidate.scenarios.len() != expected_scenario_ids.len() {
        return Err(ModelError::MismatchedDataLengths);
    }

    for scenario in &candidate.scenarios {
        if scenario.outcome_likelihoods.len() != problem.state_ids.len()
            || scenario
                .outcome_likelihoods
                .iter()
                .any(|row| row.len() != candidate.outcome_ids.len())
        {
            return Err(ModelError::MismatchedDataLengths);
        }
        for row in &scenario.outcome_likelihoods {
            if row.iter().any(|value| !value.is_finite()) {
                return Err(ModelError::NonFiniteValue(
                    "robust_decision_outcome_likelihood",
                ));
            }
            if row.iter().any(|value| *value < 0.0) {
                return Err(ModelError::InvalidParameter(
                    "robust_decision_outcome_likelihood_must_be_nonnegative",
                ));
            }
            let probability_sum = row.iter().sum::<f64>();
            if !probability_sum.is_finite() || (probability_sum - 1.0).abs() > 1.0e-10 {
                return Err(ModelError::InvalidParameter(
                    "robust_decision_outcome_likelihoods_must_sum_to_one",
                ));
            }
        }
    }
    Ok(())
}

fn validate_unique_nonempty_ids(
    ids: &[String],
    error_name: &'static str,
) -> Result<(), ModelError> {
    if ids.iter().any(|id| id.trim().is_empty()) {
        return Err(ModelError::InvalidParameter(error_name));
    }
    let mut unique = BTreeSet::new();
    if ids.iter().any(|id| !unique.insert(id.as_str())) {
        return Err(ModelError::InvalidParameter(error_name));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECEIPT: &str = "receipt:shared-source-ambiguity-fixture-v1";

    fn problem() -> RobustDecisionProblem {
        RobustDecisionProblem {
            state_ids: vec!["state-0".to_owned(), "state-1".to_owned()],
            action_ids: vec!["choose-0".to_owned(), "choose-1".to_owned()],
            state_probabilities: vec![0.5, 0.5],
            action_utilities: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        }
    }

    fn ambiguous_perfect_design() -> RobustDecisionDesignCandidate {
        RobustDecisionDesignCandidate {
            candidate_id: "ambiguous-perfect-sensor".to_owned(),
            outcome_ids: vec!["reading-0".to_owned(), "reading-1".to_owned()],
            resource_cost: 0.05,
            dependence_scope_receipt_id: RECEIPT.to_owned(),
            scenarios: vec![
                RobustDecisionLikelihoodScenario {
                    scenario_id: "calibration-forward".to_owned(),
                    outcome_likelihoods: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
                },
                RobustDecisionLikelihoodScenario {
                    scenario_id: "calibration-reversed".to_owned(),
                    outcome_likelihoods: vec![vec![0.0, 1.0], vec![1.0, 0.0]],
                },
            ],
        }
    }

    fn stable_noisy_design() -> RobustDecisionDesignCandidate {
        RobustDecisionDesignCandidate {
            candidate_id: "stable-80-percent-sensor".to_owned(),
            outcome_ids: vec!["reading-0".to_owned(), "reading-1".to_owned()],
            resource_cost: 0.05,
            dependence_scope_receipt_id: RECEIPT.to_owned(),
            scenarios: vec![
                RobustDecisionLikelihoodScenario {
                    scenario_id: "calibration-forward".to_owned(),
                    outcome_likelihoods: vec![vec![0.8, 0.2], vec![0.2, 0.8]],
                },
                RobustDecisionLikelihoodScenario {
                    scenario_id: "calibration-reversed".to_owned(),
                    outcome_likelihoods: vec![vec![0.8, 0.2], vec![0.2, 0.8]],
                },
            ],
        }
    }

    fn policy() -> RobustDecisionDesignPolicy {
        RobustDecisionDesignPolicy {
            remaining_resource_budget: 1.0,
            cost_to_utility_rate: 1.0,
            max_policy_evaluations: 64,
        }
    }

    #[test]
    fn robust_decision_value_reverses_the_eig_only_ranking() {
        let candidates = vec![ambiguous_perfect_design(), stable_noisy_design()];
        let result = rank_designs_by_robust_decision_value(&problem(), &candidates, policy())
            .expect("finite robust decision ranking");

        assert_eq!(
            result.recommended_candidate_id.as_deref(),
            Some("stable-80-percent-sensor")
        );
        assert_eq!(result.scores[0].candidate_id, "stable-80-percent-sensor");
        assert_eq!(result.total_policy_evaluations, 8);

        let ambiguous = result
            .scores
            .iter()
            .find(|score| score.candidate_id == "ambiguous-perfect-sensor")
            .expect("ambiguous candidate");
        let stable = result
            .scores
            .iter()
            .find(|score| score.candidate_id == "stable-80-percent-sensor")
            .expect("stable candidate");

        assert!(
            (ambiguous.worst_case_expected_information_gain_nats - 2.0_f64.ln()).abs()
                < 1.0e-12
        );
        assert!(
            (stable.worst_case_expected_information_gain_nats - 0.1927447570217574).abs()
                < 1.0e-12
        );
        assert!((ambiguous.robust_post_information_utility - 0.5).abs() < 1.0e-12);
        assert!(
            ambiguous
                .robust_expected_value_of_sample_information
                .abs()
                < 1.0e-12
        );
        assert!(
            (ambiguous.clairvoyant_post_information_utility_upper_bound - 1.0).abs()
                < 1.0e-12
        );
        assert!((ambiguous.non_anticipativity_gap - 0.5).abs() < 1.0e-12);
        assert!((ambiguous.net_robust_decision_value.unwrap() + 0.05).abs() < 1.0e-12);

        assert!((stable.robust_post_information_utility - 0.8).abs() < 1.0e-12);
        assert!((stable.robust_expected_value_of_sample_information - 0.3).abs() < 1.0e-12);
        assert!(stable.non_anticipativity_gap.abs() < 1.0e-12);
        assert!((stable.net_robust_decision_value.unwrap() - 0.25).abs() < 1.0e-12);
        assert!(
            stable.worst_case_expected_information_gain_nats
                < ambiguous.worst_case_expected_information_gain_nats
        );
    }

    #[test]
    fn nuisance_information_has_no_decision_value() {
        let nuisance_problem = RobustDecisionProblem {
            state_ids: vec![
                "decision-0-nuisance-0".to_owned(),
                "decision-0-nuisance-1".to_owned(),
                "decision-1-nuisance-0".to_owned(),
                "decision-1-nuisance-1".to_owned(),
            ],
            action_ids: vec!["choose-0".to_owned(), "choose-1".to_owned()],
            state_probabilities: vec![0.25, 0.25, 0.25, 0.25],
            action_utilities: vec![
                vec![1.0, 1.0, 0.0, 0.0],
                vec![0.0, 0.0, 1.0, 1.0],
            ],
        };
        let nuisance_likelihoods = vec![
            vec![1.0, 0.0],
            vec![0.0, 1.0],
            vec![1.0, 0.0],
            vec![0.0, 1.0],
        ];
        let candidate = RobustDecisionDesignCandidate {
            candidate_id: "nuisance-only-calibration".to_owned(),
            outcome_ids: vec!["nuisance-0".to_owned(), "nuisance-1".to_owned()],
            resource_cost: 0.05,
            dependence_scope_receipt_id: RECEIPT.to_owned(),
            scenarios: vec![
                RobustDecisionLikelihoodScenario {
                    scenario_id: "calibration-forward".to_owned(),
                    outcome_likelihoods: nuisance_likelihoods.clone(),
                },
                RobustDecisionLikelihoodScenario {
                    scenario_id: "calibration-reversed".to_owned(),
                    outcome_likelihoods: nuisance_likelihoods,
                },
            ],
        };
        let result = rank_designs_by_robust_decision_value(
            &nuisance_problem,
            &[candidate],
            policy(),
        )
        .expect("nuisance-only signal");
        let score = &result.scores[0];

        assert!((score.worst_case_expected_information_gain_nats - 2.0_f64.ln()).abs() < 1.0e-12);
        assert!((score.robust_post_information_utility - 0.5).abs() < 1.0e-12);
        assert!(score.robust_expected_value_of_sample_information.abs() < 1.0e-12);
        assert!(score.net_robust_decision_value.unwrap() < 0.0);
        assert_eq!(result.recommended_candidate_id, None);
    }

    #[test]
    fn rejects_policy_search_that_exceeds_hard_cap() {
        let mut limited = policy();
        limited.max_policy_evaluations = 7;
        assert_eq!(
            rank_designs_by_robust_decision_value(
                &problem(),
                &[ambiguous_perfect_design(), stable_noisy_design()],
                limited,
            ),
            Err(ModelError::InvalidParameter(
                "robust_decision_policy_evaluation_cap_exceeded"
            ))
        );
    }

    #[test]
    fn rejects_candidates_with_different_receipt_scope() {
        let mut candidate = stable_noisy_design();
        candidate.dependence_scope_receipt_id = "receipt:wrong-scope".to_owned();
        assert_eq!(
            rank_designs_by_robust_decision_value(
                &problem(),
                &[ambiguous_perfect_design(), candidate],
                policy(),
            ),
            Err(ModelError::InvalidParameter(
                "robust_decision_candidates_must_share_dependence_receipt"
            ))
        );
    }

    #[test]
    fn rejects_candidates_with_different_scenario_order() {
        let mut candidate = stable_noisy_design();
        candidate.scenarios.swap(0, 1);
        assert_eq!(
            rank_designs_by_robust_decision_value(
                &problem(),
                &[ambiguous_perfect_design(), candidate],
                policy(),
            ),
            Err(ModelError::InvalidParameter(
                "robust_decision_candidates_must_share_ordered_scenarios"
            ))
        );
    }

    #[test]
    fn a_candidate_outside_the_budget_is_never_recommended() {
        let mut candidate = stable_noisy_design();
        candidate.resource_cost = 2.0;
        let result = rank_designs_by_robust_decision_value(&problem(), &[candidate], policy())
            .expect("budget-infeasible candidate is still scored");
        assert_eq!(result.recommended_candidate_id, None);
        assert!(!result.scores[0].budget_feasible);
        assert_eq!(result.scores[0].net_robust_decision_value, None);
    }

    #[test]
    fn rejects_non_stochastic_likelihood_rows() {
        let mut candidate = stable_noisy_design();
        candidate.scenarios[0].outcome_likelihoods[0] = vec![0.7, 0.2];
        assert_eq!(
            rank_designs_by_robust_decision_value(&problem(), &[candidate], policy()),
            Err(ModelError::InvalidParameter(
                "robust_decision_outcome_likelihoods_must_sum_to_one"
            ))
        );
    }
}
