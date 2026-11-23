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
        total_policy_evaluations = total_policy_evaluations.checked_add(policy_count).ok_or(
            ModelError::InvalidParameter("robust_decision_policy_evaluation_count_overflow"),
        )?;
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

        let robust_policy_utilities = scenario_policy_utilities(problem, candidate, &best_policy)?;
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
                return Err(ModelError::NonFiniteValue("robust_decision_net_value"));
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
        let left_value = left.net_robust_decision_value.unwrap_or(f64::NEG_INFINITY);
        let right_value = right.net_robust_decision_value.unwrap_or(f64::NEG_INFINITY);
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

/// One component of a randomized policy, represented as a distribution over
/// deterministic outcome-to-action maps. The mixture is fixed before the
/// hidden dependence scenario is known.
#[derive(Debug, Clone, PartialEq)]
pub struct RandomizedDecisionPolicyComponent {
    pub action_ids_by_outcome: Vec<String>,
    pub probability: f64,
}

/// One component of the least-favourable mixture over dependence scenarios.
/// This is the dual strategy of the finite zero-sum game, not a claim that the
/// physical scenario itself is random.
#[derive(Debug, Clone, PartialEq)]
pub struct LeastFavourableScenarioWeight {
    pub scenario_id: String,
    pub probability: f64,
}

/// Resource and numerical limits for the finite randomized-policy game.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RandomizedRobustDecisionPolicy {
    pub remaining_resource_budget: f64,
    pub cost_to_utility_rate: f64,
    pub max_policy_evaluations: usize,
    pub max_basis_evaluations: usize,
    pub feasibility_tolerance: f64,
    pub duality_gap_tolerance: f64,
}

/// Randomized robust decision value and independently checked game dual.
#[derive(Debug, Clone, PartialEq)]
pub struct RandomizedRobustDecisionDesignScore {
    pub candidate_id: String,
    pub outcome_ids: Vec<String>,
    pub scenario_ids: Vec<String>,
    pub randomized_policy_mixture: Vec<RandomizedDecisionPolicyComponent>,
    pub least_favourable_scenario_mixture: Vec<LeastFavourableScenarioWeight>,
    pub randomized_policy_expected_utilities_by_scenario: Vec<f64>,
    pub clairvoyant_expected_utilities_by_scenario: Vec<f64>,
    pub prior_best_action_id: String,
    pub prior_best_expected_utility: f64,
    pub deterministic_robust_post_information_utility: f64,
    pub randomized_robust_post_information_utility: f64,
    pub deterministic_to_randomized_value_gain: f64,
    pub robust_expected_value_of_sample_information: f64,
    pub clairvoyant_post_information_utility_upper_bound: f64,
    pub non_anticipativity_gap: f64,
    pub expected_information_gain_nats_by_scenario: Vec<f64>,
    pub worst_case_expected_information_gain_nats: f64,
    pub best_case_expected_information_gain_nats: f64,
    pub primal_game_value: f64,
    pub dual_game_value: f64,
    pub primal_dual_gap: f64,
    pub maximum_primal_feasibility_residual: f64,
    pub maximum_dual_feasibility_residual: f64,
    pub maximum_complementarity_residual: f64,
    pub resource_cost: f64,
    pub cost_in_utility_units: f64,
    pub budget_feasible: bool,
    pub net_robust_decision_value: Option<f64>,
    pub deterministic_policy_count: usize,
    pub policy_evaluations: usize,
    pub basis_evaluations: usize,
    pub finite_game_exact: bool,
    pub primal_dual_verified_within_tolerance: bool,
    pub formal_certificate: bool,
}

/// Ranking over randomized non-anticipative policies for one finite ambiguity set.
#[derive(Debug, Clone, PartialEq)]
pub struct RandomizedRobustDecisionDesignRanking {
    pub dependence_scope_receipt_id: String,
    pub prior_best_action_id: String,
    pub prior_best_expected_utility: f64,
    pub recommended_candidate_id: Option<String>,
    pub total_policy_evaluations: usize,
    pub total_basis_evaluations: usize,
    pub max_policy_evaluations: usize,
    pub max_basis_evaluations: usize,
    pub scores: Vec<RandomizedRobustDecisionDesignScore>,
}

#[derive(Debug, Clone, PartialEq)]
struct FiniteGameStrategy {
    support: Vec<usize>,
    probabilities: Vec<f64>,
    value: f64,
    feasibility_residual: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct FiniteGameSolution {
    maximizing_policy_mixture: FiniteGameStrategy,
    minimizing_scenario_mixture: FiniteGameStrategy,
    primal_dual_gap: f64,
    maximum_complementarity_residual: f64,
    basis_evaluations: usize,
}

/// Rank designs by a finite zero-sum game over randomized policies and
/// dependence scenarios.
///
/// The primal chooses a distribution over deterministic maps Y -> A before
/// the unknown dependence scenario is known:
///
/// max_q min_s sum_pi q[pi] E[U(pi(Y), theta) | d, s].
///
/// The dual independently chooses a distribution over admitted scenarios:
///
/// min_w max_pi sum_s w[s] E[U(pi(Y), theta) | d, s].
///
/// Both strategies are computed by enumerating basic supports of the finite
/// primal and dual LPs. Results are returned only when both strategies are
/// feasible and their objectives agree within the caller's tolerance.
/// This is a finite-game numerical solution using f64, not an LP proof,
/// continuous ambiguity solver, or formal certificate.
pub fn rank_designs_by_randomized_robust_decision_value(
    problem: &RobustDecisionProblem,
    candidates: &[RobustDecisionDesignCandidate],
    policy: RandomizedRobustDecisionPolicy,
) -> Result<RandomizedRobustDecisionDesignRanking, ModelError> {
    validate_decision_problem(problem)?;
    if candidates.is_empty() {
        return Err(ModelError::MismatchedDataLengths);
    }
    validate_randomized_policy(policy)?;

    let prior_best = prior_best_action(problem)?;
    let expected_scenario_ids = validate_candidate_family(candidates)?;
    let common_receipt = candidates[0].dependence_scope_receipt_id.clone();

    let mut total_policy_evaluations = 0usize;
    let mut total_basis_evaluations = 0usize;
    let mut scores = Vec::with_capacity(candidates.len());

    for candidate in candidates {
        validate_candidate(problem, candidate, &expected_scenario_ids)?;
        let action_count = problem.action_ids.len();
        let outcome_count = candidate.outcome_ids.len();
        let scenario_count = candidate.scenarios.len();
        let policy_count = deterministic_policy_count(action_count, outcome_count)?;

        total_policy_evaluations = total_policy_evaluations.checked_add(policy_count).ok_or(
            ModelError::InvalidParameter("randomized_robust_policy_count_overflow"),
        )?;
        if total_policy_evaluations > policy.max_policy_evaluations {
            return Err(ModelError::InvalidParameter(
                "randomized_robust_policy_evaluation_cap_exceeded",
            ));
        }

        let payoff_evaluation_count =
            policy_count
                .checked_mul(scenario_count)
                .ok_or(ModelError::InvalidParameter(
                    "randomized_robust_payoff_evaluation_count_overflow",
                ))?;
        if payoff_evaluation_count > policy.max_policy_evaluations {
            return Err(ModelError::InvalidParameter(
                "randomized_robust_payoff_evaluation_cap_exceeded",
            ));
        }

        let mut payoff_matrix = Vec::with_capacity(policy_count);
        let mut action_maps = Vec::with_capacity(policy_count);
        let mut policy_indices = vec![0usize; outcome_count];
        let mut deterministic_robust_value = f64::NEG_INFINITY;
        let mut clairvoyant_by_scenario = vec![f64::NEG_INFINITY; scenario_count];

        for _ in 0..policy_count {
            let scenario_utilities =
                scenario_policy_utilities(problem, candidate, &policy_indices)?;
            let deterministic_worst_case = scenario_utilities
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min);
            deterministic_robust_value = deterministic_robust_value.max(deterministic_worst_case);

            for (scenario_index, utility) in scenario_utilities.iter().enumerate() {
                clairvoyant_by_scenario[scenario_index] =
                    clairvoyant_by_scenario[scenario_index].max(*utility);
            }

            payoff_matrix.push(scenario_utilities);
            action_maps.push(policy_indices.clone());
            increment_policy(&mut policy_indices, action_count);
        }

        let game = solve_finite_zero_sum_game(
            &payoff_matrix,
            policy.feasibility_tolerance,
            policy.duality_gap_tolerance,
            policy
                .max_basis_evaluations
                .saturating_sub(total_basis_evaluations),
        )?;
        total_basis_evaluations = total_basis_evaluations
            .checked_add(game.basis_evaluations)
            .ok_or(ModelError::InvalidParameter(
                "randomized_robust_basis_evaluation_count_overflow",
            ))?;
        if total_basis_evaluations > policy.max_basis_evaluations {
            return Err(ModelError::InvalidParameter(
                "randomized_robust_basis_evaluation_cap_exceeded",
            ));
        }

        let mut randomized_policy_utilities = vec![0.0; scenario_count];
        let mut randomized_policy_mixture = Vec::new();
        for (policy_index, probability) in game
            .maximizing_policy_mixture
            .support
            .iter()
            .zip(game.maximizing_policy_mixture.probabilities.iter())
        {
            if *probability <= 0.0 {
                continue;
            }
            for (scenario_index, utility) in payoff_matrix[*policy_index].iter().enumerate() {
                randomized_policy_utilities[scenario_index] += probability * utility;
            }
            randomized_policy_mixture.push(RandomizedDecisionPolicyComponent {
                action_ids_by_outcome: action_maps[*policy_index]
                    .iter()
                    .map(|index| problem.action_ids[*index].clone())
                    .collect(),
                probability: *probability,
            });
        }

        let least_favourable_scenario_mixture = game
            .minimizing_scenario_mixture
            .support
            .iter()
            .zip(game.minimizing_scenario_mixture.probabilities.iter())
            .filter(|(_, probability)| **probability > 0.0)
            .map(
                |(scenario_index, probability)| LeastFavourableScenarioWeight {
                    scenario_id: expected_scenario_ids[*scenario_index].clone(),
                    probability: *probability,
                },
            )
            .collect::<Vec<_>>();

        let randomized_robust_value = randomized_policy_utilities
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let deterministic_to_randomized_value_gain =
            randomized_robust_value - deterministic_robust_value;
        if deterministic_to_randomized_value_gain < -policy.feasibility_tolerance {
            return Err(ModelError::InvalidParameter(
                "randomized_robust_value_below_deterministic_baseline",
            ));
        }
        let robust_evsi = randomized_robust_value - prior_best.expected_utility;
        let clairvoyant_upper = clairvoyant_by_scenario
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let non_anticipativity_gap = (clairvoyant_upper - randomized_robust_value).max(0.0);

        if !randomized_robust_value.is_finite()
            || !robust_evsi.is_finite()
            || !clairvoyant_upper.is_finite()
            || !non_anticipativity_gap.is_finite()
        {
            return Err(ModelError::NonFiniteValue(
                "randomized_robust_decision_value",
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
                "randomized_robust_decision_cost",
            ));
        }
        let net_robust_decision_value = if budget_feasible {
            let net_value = robust_evsi - cost_in_utility_units;
            if !net_value.is_finite() {
                return Err(ModelError::NonFiniteValue(
                    "randomized_robust_net_decision_value",
                ));
            }
            Some(net_value)
        } else {
            None
        };

        scores.push(RandomizedRobustDecisionDesignScore {
            candidate_id: candidate.candidate_id.clone(),
            outcome_ids: candidate.outcome_ids.clone(),
            scenario_ids: expected_scenario_ids.clone(),
            randomized_policy_mixture,
            least_favourable_scenario_mixture,
            randomized_policy_expected_utilities_by_scenario: randomized_policy_utilities,
            clairvoyant_expected_utilities_by_scenario: clairvoyant_by_scenario,
            prior_best_action_id: prior_best.action_id.clone(),
            prior_best_expected_utility: prior_best.expected_utility,
            deterministic_robust_post_information_utility: deterministic_robust_value,
            randomized_robust_post_information_utility: randomized_robust_value,
            deterministic_to_randomized_value_gain,
            robust_expected_value_of_sample_information: robust_evsi,
            clairvoyant_post_information_utility_upper_bound: clairvoyant_upper,
            non_anticipativity_gap,
            expected_information_gain_nats_by_scenario: eig_by_scenario,
            worst_case_expected_information_gain_nats: worst_case_eig,
            best_case_expected_information_gain_nats: best_case_eig,
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
            resource_cost: candidate.resource_cost,
            cost_in_utility_units,
            budget_feasible,
            net_robust_decision_value,
            deterministic_policy_count: policy_count,
            policy_evaluations: payoff_evaluation_count,
            basis_evaluations: game.basis_evaluations,
            finite_game_exact: true,
            primal_dual_verified_within_tolerance: true,
            formal_certificate: false,
        });
    }

    scores.sort_by(|left, right| {
        let left_value = left.net_robust_decision_value.unwrap_or(f64::NEG_INFINITY);
        let right_value = right.net_robust_decision_value.unwrap_or(f64::NEG_INFINITY);
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

    Ok(RandomizedRobustDecisionDesignRanking {
        dependence_scope_receipt_id: common_receipt,
        prior_best_action_id: prior_best.action_id,
        prior_best_expected_utility: prior_best.expected_utility,
        recommended_candidate_id,
        total_policy_evaluations,
        total_basis_evaluations,
        max_policy_evaluations: policy.max_policy_evaluations,
        max_basis_evaluations: policy.max_basis_evaluations,
        scores,
    })
}

fn validate_randomized_policy(policy: RandomizedRobustDecisionPolicy) -> Result<(), ModelError> {
    if !policy.remaining_resource_budget.is_finite()
        || !policy.cost_to_utility_rate.is_finite()
        || !policy.feasibility_tolerance.is_finite()
        || !policy.duality_gap_tolerance.is_finite()
    {
        return Err(ModelError::NonFiniteValue(
            "randomized_robust_policy_limits",
        ));
    }
    if policy.remaining_resource_budget < 0.0
        || policy.cost_to_utility_rate < 0.0
        || policy.feasibility_tolerance <= 0.0
        || policy.duality_gap_tolerance <= 0.0
    {
        return Err(ModelError::InvalidParameter(
            "randomized_robust_policy_limits_invalid",
        ));
    }
    if policy.max_policy_evaluations == 0 || policy.max_basis_evaluations == 0 {
        return Err(ModelError::InvalidParameter(
            "randomized_robust_policy_caps_must_be_positive",
        ));
    }
    Ok(())
}

fn solve_finite_zero_sum_game(
    payoffs: &[Vec<f64>],
    feasibility_tolerance: f64,
    duality_gap_tolerance: f64,
    max_basis_evaluations: usize,
) -> Result<FiniteGameSolution, ModelError> {
    if payoffs.is_empty()
        || payoffs[0].is_empty()
        || payoffs.iter().any(|row| row.len() != payoffs[0].len())
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    if payoffs.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ModelError::NonFiniteValue("randomized_robust_game_payoff"));
    }

    let policy_count = payoffs.len();
    let scenario_count = payoffs[0].len();
    let estimated_bases = total_support_basis_count(policy_count, scenario_count)?;
    if estimated_bases > max_basis_evaluations {
        return Err(ModelError::InvalidParameter(
            "randomized_robust_basis_evaluation_cap_exceeded",
        ));
    }

    let payoff_scale = payoffs
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max)
        .max(f64::MIN_POSITIVE);
    let normalized_payoffs = payoffs
        .iter()
        .map(|row| {
            row.iter()
                .map(|value| value / payoff_scale)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let (mut maximizing_policy_mixture, primal_bases) =
        enumerate_primal_game_strategy(&normalized_payoffs, feasibility_tolerance)?;
    let (mut minimizing_scenario_mixture, dual_bases) =
        enumerate_dual_game_strategy(&normalized_payoffs, feasibility_tolerance)?;
    maximizing_policy_mixture.value = robust_policy_value(
        payoffs,
        &maximizing_policy_mixture.support,
        &maximizing_policy_mixture.probabilities,
    );
    minimizing_scenario_mixture.value = dual_scenario_value(
        payoffs,
        &minimizing_scenario_mixture.support,
        &minimizing_scenario_mixture.probabilities,
    );
    let basis_evaluations =
        primal_bases
            .checked_add(dual_bases)
            .ok_or(ModelError::InvalidParameter(
                "randomized_robust_basis_evaluation_count_overflow",
            ))?;
    if basis_evaluations > max_basis_evaluations {
        return Err(ModelError::InvalidParameter(
            "randomized_robust_basis_evaluation_cap_exceeded",
        ));
    }

    let primal_value = maximizing_policy_mixture.value;
    let dual_value = minimizing_scenario_mixture.value;
    let gap = dual_value - primal_value;
    let scale = primal_value.abs().max(dual_value.abs()).max(1.0);
    if !gap.is_finite() || gap < -duality_gap_tolerance * scale {
        return Err(ModelError::InvalidParameter(
            "randomized_robust_primal_dual_ordering_invalid",
        ));
    }
    if gap > duality_gap_tolerance * scale {
        return Err(ModelError::InvalidParameter(
            "randomized_robust_primal_dual_gap_exceeds_tolerance",
        ));
    }

    let mut maximum_complementarity_residual = 0.0_f64;
    for (policy_index, probability) in maximizing_policy_mixture
        .support
        .iter()
        .zip(maximizing_policy_mixture.probabilities.iter())
    {
        if *probability > feasibility_tolerance {
            let expected_against_dual = minimizing_scenario_mixture
                .support
                .iter()
                .zip(minimizing_scenario_mixture.probabilities.iter())
                .map(|(scenario_index, scenario_probability)| {
                    payoffs[*policy_index][*scenario_index] * scenario_probability
                })
                .sum::<f64>();
            maximum_complementarity_residual =
                maximum_complementarity_residual.max((expected_against_dual - dual_value).abs());
        }
    }
    for (scenario_index, probability) in minimizing_scenario_mixture
        .support
        .iter()
        .zip(minimizing_scenario_mixture.probabilities.iter())
    {
        if *probability > feasibility_tolerance {
            let primal_payoff = maximizing_policy_mixture
                .support
                .iter()
                .zip(maximizing_policy_mixture.probabilities.iter())
                .map(|(policy_index, policy_probability)| {
                    payoffs[*policy_index][*scenario_index] * policy_probability
                })
                .sum::<f64>();
            maximum_complementarity_residual =
                maximum_complementarity_residual.max((primal_payoff - primal_value).abs());
        }
    }
    if maximum_complementarity_residual > feasibility_tolerance * scale * 10.0 {
        return Err(ModelError::InvalidParameter(
            "randomized_robust_complementarity_residual_exceeds_tolerance",
        ));
    }

    Ok(FiniteGameSolution {
        maximizing_policy_mixture,
        minimizing_scenario_mixture,
        primal_dual_gap: gap.abs(),
        maximum_complementarity_residual,
        basis_evaluations,
    })
}

fn enumerate_primal_game_strategy(
    payoffs: &[Vec<f64>],
    tolerance: f64,
) -> Result<(FiniteGameStrategy, usize), ModelError> {
    let policy_count = payoffs.len();
    let scenario_count = payoffs[0].len();
    let mut best: Option<FiniteGameStrategy> = None;
    let mut basis_evaluations = 0usize;

    for support_size in 1..=policy_count.min(scenario_count) {
        let policy_supports = index_combinations(policy_count, support_size);
        let scenario_supports = index_combinations(scenario_count, support_size);
        for policy_support in &policy_supports {
            for scenario_support in &scenario_supports {
                basis_evaluations += 1;
                let mut matrix = vec![vec![0.0; support_size + 1]; support_size + 1];
                let mut rhs = vec![0.0; support_size + 1];
                for (row, scenario_index) in scenario_support.iter().enumerate() {
                    let equation = &mut matrix[row];
                    for (column, policy_index) in policy_support.iter().enumerate() {
                        equation[column] = payoffs[*policy_index][*scenario_index];
                    }
                    equation[support_size] = -1.0;
                }
                matrix[support_size][..support_size].fill(1.0);
                rhs[support_size] = 1.0;

                let Some(solution) = solve_small_linear_system(matrix, rhs, tolerance) else {
                    continue;
                };
                let Some(probabilities) =
                    normalize_feasible_probabilities(&solution[..support_size], tolerance)
                else {
                    continue;
                };
                let value = robust_policy_value(payoffs, policy_support, &probabilities);
                if !value.is_finite() {
                    continue;
                }
                let residual = (probabilities.iter().sum::<f64>() - 1.0).abs()
                    + probabilities
                        .iter()
                        .copied()
                        .map(|probability| (-probability).max(0.0))
                        .fold(0.0, f64::max);
                let candidate = FiniteGameStrategy {
                    support: policy_support.clone(),
                    probabilities,
                    value,
                    feasibility_residual: residual,
                };
                if best
                    .as_ref()
                    .is_none_or(|current| candidate.value > current.value + tolerance)
                {
                    best = Some(candidate);
                }
            }
        }
    }

    best.map(|strategy| (strategy, basis_evaluations))
        .ok_or(ModelError::InvalidParameter(
            "randomized_robust_primal_game_basis_not_found",
        ))
}

fn enumerate_dual_game_strategy(
    payoffs: &[Vec<f64>],
    tolerance: f64,
) -> Result<(FiniteGameStrategy, usize), ModelError> {
    let policy_count = payoffs.len();
    let scenario_count = payoffs[0].len();
    let mut best: Option<FiniteGameStrategy> = None;
    let mut basis_evaluations = 0usize;

    for support_size in 1..=scenario_count.min(policy_count) {
        let scenario_supports = index_combinations(scenario_count, support_size);
        let policy_supports = index_combinations(policy_count, support_size);
        for scenario_support in &scenario_supports {
            for policy_support in &policy_supports {
                basis_evaluations += 1;
                let mut matrix = vec![vec![0.0; support_size + 1]; support_size + 1];
                let mut rhs = vec![0.0; support_size + 1];
                for (row, policy_index) in policy_support.iter().enumerate() {
                    let equation = &mut matrix[row];
                    for (column, scenario_index) in scenario_support.iter().enumerate() {
                        equation[column] = payoffs[*policy_index][*scenario_index];
                    }
                    equation[support_size] = -1.0;
                }
                matrix[support_size][..support_size].fill(1.0);
                rhs[support_size] = 1.0;

                let Some(solution) = solve_small_linear_system(matrix, rhs, tolerance) else {
                    continue;
                };
                let Some(probabilities) =
                    normalize_feasible_probabilities(&solution[..support_size], tolerance)
                else {
                    continue;
                };
                let value = dual_scenario_value(payoffs, scenario_support, &probabilities);
                if !value.is_finite() {
                    continue;
                }
                let residual = (probabilities.iter().sum::<f64>() - 1.0).abs()
                    + probabilities
                        .iter()
                        .copied()
                        .map(|probability| (-probability).max(0.0))
                        .fold(0.0, f64::max);
                let candidate = FiniteGameStrategy {
                    support: scenario_support.clone(),
                    probabilities,
                    value,
                    feasibility_residual: residual,
                };
                if best
                    .as_ref()
                    .is_none_or(|current| candidate.value < current.value - tolerance)
                {
                    best = Some(candidate);
                }
            }
        }
    }

    best.map(|strategy| (strategy, basis_evaluations))
        .ok_or(ModelError::InvalidParameter(
            "randomized_robust_dual_game_basis_not_found",
        ))
}

fn robust_policy_value(
    payoffs: &[Vec<f64>],
    policy_support: &[usize],
    probabilities: &[f64],
) -> f64 {
    payoffs[0]
        .iter()
        .enumerate()
        .map(|(scenario_index, _)| {
            policy_support
                .iter()
                .zip(probabilities.iter())
                .map(|(policy_index, probability)| {
                    payoffs[*policy_index][scenario_index] * probability
                })
                .sum::<f64>()
        })
        .fold(f64::INFINITY, f64::min)
}

fn dual_scenario_value(
    payoffs: &[Vec<f64>],
    scenario_support: &[usize],
    probabilities: &[f64],
) -> f64 {
    payoffs
        .iter()
        .map(|policy_payoffs| {
            scenario_support
                .iter()
                .zip(probabilities.iter())
                .map(|(scenario_index, probability)| policy_payoffs[*scenario_index] * probability)
                .sum::<f64>()
        })
        .fold(f64::NEG_INFINITY, f64::max)
}

fn normalize_feasible_probabilities(values: &[f64], tolerance: f64) -> Option<Vec<f64>> {
    if values
        .iter()
        .any(|value| !value.is_finite() || *value < -tolerance)
    {
        return None;
    }
    let mut probabilities = values
        .iter()
        .map(|value| value.max(0.0))
        .collect::<Vec<_>>();
    let sum = probabilities.iter().sum::<f64>();
    if !sum.is_finite() || sum <= tolerance {
        return None;
    }
    for probability in &mut probabilities {
        *probability /= sum;
    }
    Some(probabilities)
}

// Matrix row/column indices are required for pivoted Gaussian elimination.
#[allow(clippy::needless_range_loop)]
fn solve_small_linear_system(
    mut matrix: Vec<Vec<f64>>,
    mut rhs: Vec<f64>,
    tolerance: f64,
) -> Option<Vec<f64>> {
    let dimension = rhs.len();
    if dimension == 0
        || matrix.len() != dimension
        || matrix.iter().any(|row| row.len() != dimension)
    {
        return None;
    }
    let scale = matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(1.0_f64, f64::max);
    for column in 0..dimension {
        let pivot_row = (column..dimension).max_by(|left, right| {
            matrix[*left][column]
                .abs()
                .total_cmp(&matrix[*right][column].abs())
        })?;
        if matrix[pivot_row][column].abs() <= tolerance * scale {
            return None;
        }
        matrix.swap(column, pivot_row);
        rhs.swap(column, pivot_row);
        let pivot = matrix[column][column];
        for entry in column..dimension {
            matrix[column][entry] /= pivot;
        }
        rhs[column] /= pivot;

        for row in 0..dimension {
            if row == column {
                continue;
            }
            let factor = matrix[row][column];
            for entry in column..dimension {
                matrix[row][entry] -= factor * matrix[column][entry];
            }
            rhs[row] -= factor * rhs[column];
        }
    }
    if rhs.iter().all(|value| value.is_finite()) {
        Some(rhs)
    } else {
        None
    }
}

fn index_combinations(count: usize, choose: usize) -> Vec<Vec<usize>> {
    fn visit(
        count: usize,
        remaining: usize,
        next: usize,
        current: &mut Vec<usize>,
        output: &mut Vec<Vec<usize>>,
    ) {
        if remaining == 0 {
            output.push(current.clone());
            return;
        }
        if count.saturating_sub(next) < remaining {
            return;
        }
        for index in next..=count - remaining {
            current.push(index);
            visit(count, remaining - 1, index + 1, current, output);
            current.pop();
        }
    }
    let mut output = Vec::new();
    visit(count, choose, 0, &mut Vec::new(), &mut output);
    output
}

fn total_support_basis_count(
    policy_count: usize,
    scenario_count: usize,
) -> Result<usize, ModelError> {
    let mut one_side_count = 0usize;
    for support_size in 1..=policy_count.min(scenario_count) {
        let policy_combinations = binomial_count(policy_count, support_size)?;
        let scenario_combinations = binomial_count(scenario_count, support_size)?;
        let pair_count = policy_combinations
            .checked_mul(scenario_combinations)
            .ok_or(ModelError::InvalidParameter(
                "randomized_robust_basis_count_overflow",
            ))?;
        one_side_count =
            one_side_count
                .checked_add(pair_count)
                .ok_or(ModelError::InvalidParameter(
                    "randomized_robust_basis_count_overflow",
                ))?;
    }
    one_side_count
        .checked_mul(2)
        .ok_or(ModelError::InvalidParameter(
            "randomized_robust_basis_count_overflow",
        ))
}

fn binomial_count(n: usize, k: usize) -> Result<usize, ModelError> {
    let k = k.min(n.saturating_sub(k));
    let mut value = 1usize;
    for i in 0..k {
        value = value
            .checked_mul(n - i)
            .ok_or(ModelError::InvalidParameter(
                "randomized_robust_binomial_overflow",
            ))?
            / (i + 1);
    }
    Ok(value)
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
            for (state_index, state_probability) in problem.state_probabilities.iter().enumerate() {
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
        for (outcome_index, likelihood) in outcome_likelihoods[state_index].iter().enumerate() {
            outcome_probabilities[outcome_index] += state_probability * likelihood;
        }
    }

    let mut information_gain = 0.0;
    for (state_index, state_probability) in state_probabilities.iter().enumerate() {
        if *state_probability == 0.0 {
            continue;
        }
        for (outcome_index, likelihood) in outcome_likelihoods[state_index].iter().enumerate() {
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
        count = count
            .checked_mul(action_count)
            .ok_or(ModelError::InvalidParameter(
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
    if !policy.remaining_resource_budget.is_finite() || !policy.cost_to_utility_rate.is_finite() {
        return Err(ModelError::NonFiniteValue(
            "robust_decision_policy_budget_or_rate",
        ));
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
            (ambiguous.worst_case_expected_information_gain_nats - 2.0_f64.ln()).abs() < 1.0e-12
        );
        assert!(
            (stable.worst_case_expected_information_gain_nats - 0.1927447570217574).abs() < 1.0e-12
        );
        assert!((ambiguous.robust_post_information_utility - 0.5).abs() < 1.0e-12);
        assert!(ambiguous.robust_expected_value_of_sample_information.abs() < 1.0e-12);
        assert!((ambiguous.clairvoyant_post_information_utility_upper_bound - 1.0).abs() < 1.0e-12);
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
            action_utilities: vec![vec![1.0, 1.0, 0.0, 0.0], vec![0.0, 0.0, 1.0, 1.0]],
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
        let result =
            rank_designs_by_robust_decision_value(&nuisance_problem, &[candidate], policy())
                .expect("nuisance-only signal");
        let score = &result.scores[0];

        assert!((score.worst_case_expected_information_gain_nats - 2.0_f64.ln()).abs() < 1.0e-12);
        assert!((score.robust_post_information_utility - 0.5).abs() < 1.0e-12);
        assert!(score.robust_expected_value_of_sample_information.abs() < 1.0e-12);
        assert!(score.net_robust_decision_value.unwrap() < 0.0);
        assert_eq!(result.recommended_candidate_id, None);
    }

    fn randomized_fixture() -> (RobustDecisionProblem, RobustDecisionDesignCandidate) {
        (
            RobustDecisionProblem {
                state_ids: vec![
                    "state-0".to_owned(),
                    "state-1".to_owned(),
                    "state-2".to_owned(),
                ],
                action_ids: vec![
                    "action-0".to_owned(),
                    "action-1".to_owned(),
                    "action-2".to_owned(),
                ],
                state_probabilities: vec![0.4, 0.4, 0.2],
                action_utilities: vec![
                    vec![0.0, 1.0, 0.0],
                    vec![0.0, 0.0, 1.0],
                    vec![1.0, 0.0, 0.0],
                ],
            },
            RobustDecisionDesignCandidate {
                candidate_id: "strict-randomization-improvement".to_owned(),
                outcome_ids: vec![
                    "outcome-0".to_owned(),
                    "outcome-1".to_owned(),
                    "outcome-2".to_owned(),
                ],
                resource_cost: 0.01,
                dependence_scope_receipt_id: "receipt:randomized-policy-game-v1".to_owned(),
                scenarios: vec![
                    RobustDecisionLikelihoodScenario {
                        scenario_id: "dependence-a".to_owned(),
                        outcome_likelihoods: vec![
                            vec![0.6, 0.2, 0.2],
                            vec![0.2, 0.6, 0.2],
                            vec![0.4, 0.2, 0.4],
                        ],
                    },
                    RobustDecisionLikelihoodScenario {
                        scenario_id: "dependence-b".to_owned(),
                        outcome_likelihoods: vec![
                            vec![0.2, 0.4, 0.4],
                            vec![0.4, 0.4, 0.2],
                            vec![0.4, 0.2, 0.4],
                        ],
                    },
                ],
            },
        )
    }

    fn randomized_policy() -> RandomizedRobustDecisionPolicy {
        RandomizedRobustDecisionPolicy {
            remaining_resource_budget: 1.0,
            cost_to_utility_rate: 1.0,
            max_policy_evaluations: 100,
            max_basis_evaluations: 5_000,
            feasibility_tolerance: 1.0e-9,
            duality_gap_tolerance: 1.0e-8,
        }
    }

    #[test]
    fn randomized_policy_strictly_improves_the_finite_robust_game_value() {
        let (problem, candidate) = randomized_fixture();
        let result = rank_designs_by_randomized_robust_decision_value(
            &problem,
            &[candidate],
            randomized_policy(),
        )
        .expect("finite randomized robust decision game");
        let score = &result.scores[0];

        assert!((score.deterministic_robust_post_information_utility - 0.4).abs() < 1.0e-10);
        assert!((score.randomized_robust_post_information_utility - 34.0 / 75.0).abs() < 1.0e-9);
        assert!((score.deterministic_to_randomized_value_gain - 4.0 / 75.0).abs() < 1.0e-9);
        assert!((score.robust_expected_value_of_sample_information - 4.0 / 75.0).abs() < 1.0e-9);
        assert!((score.net_robust_decision_value.unwrap() - 13.0 / 300.0).abs() < 1.0e-9);
        assert!((score.primal_game_value - score.dual_game_value).abs() < 1.0e-9);
        assert!(score.primal_dual_gap < 1.0e-9);
        assert!(score.maximum_complementarity_residual < 1.0e-8);
        assert!(
            (score.randomized_policy_expected_utilities_by_scenario[0] - 34.0 / 75.0).abs()
                < 1.0e-9
        );
        assert!(
            (score.randomized_policy_expected_utilities_by_scenario[1] - 34.0 / 75.0).abs()
                < 1.0e-9
        );
        assert_eq!(score.randomized_policy_mixture.len(), 2);
        assert_eq!(score.least_favourable_scenario_mixture.len(), 2);
        let scenario_a_weight = score
            .least_favourable_scenario_mixture
            .iter()
            .find(|weight| weight.scenario_id == "dependence-a")
            .expect("first dual scenario weight")
            .probability;
        let scenario_b_weight = score
            .least_favourable_scenario_mixture
            .iter()
            .find(|weight| weight.scenario_id == "dependence-b")
            .expect("second dual scenario weight")
            .probability;
        assert!((scenario_a_weight - 1.0 / 3.0).abs() < 1.0e-9);
        assert!((scenario_b_weight - 2.0 / 3.0).abs() < 1.0e-9);

        // Independently replay the reported primal policy mixture and verify
        // the dual scenario mixture bounds every one of the 3^3 pure policies.
        let policy_a = score
            .randomized_policy_mixture
            .iter()
            .find(|component| {
                component.action_ids_by_outcome
                    == vec![
                        "action-0".to_owned(),
                        "action-0".to_owned(),
                        "action-2".to_owned(),
                    ]
            })
            .expect("first primal policy");
        let policy_b = score
            .randomized_policy_mixture
            .iter()
            .find(|component| {
                component.action_ids_by_outcome
                    == vec![
                        "action-2".to_owned(),
                        "action-0".to_owned(),
                        "action-2".to_owned(),
                    ]
            })
            .expect("second primal policy");
        assert!((policy_a.probability - 2.0 / 3.0).abs() < 1.0e-9);
        assert!((policy_b.probability - 1.0 / 3.0).abs() < 1.0e-9);

        let mut independently_replayed_primal = vec![0.0; candidate.scenarios.len()];
        for component in &score.randomized_policy_mixture {
            for (scenario_index, scenario) in candidate.scenarios.iter().enumerate() {
                let mut expected_utility = 0.0;
                for (state_index, state_probability) in
                    problem.state_probabilities.iter().enumerate()
                {
                    for (outcome_index, likelihood) in
                        scenario.outcome_likelihoods[state_index].iter().enumerate()
                    {
                        let action_index = problem
                            .action_ids
                            .iter()
                            .position(|action_id| {
                                action_id == &component.action_ids_by_outcome[outcome_index]
                            })
                            .expect("primal action ID belongs to the action set");
                        expected_utility += state_probability
                            * likelihood
                            * problem.action_utilities[action_index][state_index];
                    }
                }
                independently_replayed_primal[scenario_index] +=
                    component.probability * expected_utility;
            }
        }
        for utility in independently_replayed_primal {
            assert!((utility - 34.0 / 75.0).abs() < 1.0e-9);
        }

        for policy_code in 0..27 {
            let mut remaining_code = policy_code;
            let mut action_indices = vec![0usize; candidate.outcome_ids.len()];
            for action_index in &mut action_indices {
                *action_index = remaining_code % problem.action_ids.len();
                remaining_code /= problem.action_ids.len();
            }
            let mut scenario_utilities = vec![0.0; candidate.scenarios.len()];
            for (scenario_index, scenario) in candidate.scenarios.iter().enumerate() {
                for (state_index, state_probability) in
                    problem.state_probabilities.iter().enumerate()
                {
                    for (outcome_index, likelihood) in
                        scenario.outcome_likelihoods[state_index].iter().enumerate()
                    {
                        scenario_utilities[scenario_index] += state_probability
                            * likelihood
                            * problem.action_utilities[action_indices[outcome_index]][state_index];
                    }
                }
            }
            let dual_expected_utility =
                scenario_a_weight * scenario_utilities[0]
                    + scenario_b_weight * scenario_utilities[1];
            assert!(dual_expected_utility <= score.dual_game_value + 1.0e-9);
        }
        assert!(!score.formal_certificate);
        assert!(score.primal_dual_verified_within_tolerance);
    }

    #[test]
    fn randomized_game_fails_closed_when_basis_budget_is_exceeded() {
        let (problem, candidate) = randomized_fixture();
        let mut limits = randomized_policy();
        limits.max_basis_evaluations = 10;
        assert_eq!(
            rank_designs_by_randomized_robust_decision_value(&problem, &[candidate], limits,),
            Err(ModelError::InvalidParameter(
                "randomized_robust_basis_evaluation_cap_exceeded"
            ))
        );
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
