//! Policy-level evaluation on held-out preference models.
//!
//! This simulator enumerates every answer permitted by an explicit set-valued
//! response channel. There are no response probabilities: leaf summaries are
//! over enumerated reachable histories, not a distribution or expectation.

use super::{
    BoundedPreferenceResponseChannel, ObservedPreferenceAnswer, PreferenceOutcome,
    ResponseRobustnessError, compute_response_robustness,
};
use crate::resident_preference_probe::{
    PairwisePreferenceQuestion, PreferenceProbeError, PreferenceScenario,
    ResidentPreferenceProbeRequest, utility_numerator,
};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const MAX_POLICY_ALTERNATIVES: usize = 8;
const MAX_TRAINING_MODELS: usize = 16;
const MAX_PAIRWISE_QUESTIONS: usize = 28;
const MAX_QUESTION_BUDGET: usize = 6;
const MAX_EXPANSION_NODES: usize = 100_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum QuestionPolicy {
    /// Recompute bounded-response minimax-regret reduction after every answer.
    AdaptiveMinimaxRegret,
    /// Ask the remaining questions by ascending stable question identifier.
    FixedIdentifierOrder,
    /// Ask one deterministic pseudo-random ordering, keyed by the declared seed.
    SeededIdentifierOrder,
    /// Continue through all pairs in identifier order until the budget is exhausted.
    /// This is an exhaustive-pairwise baseline only when the supplied questions
    /// contain all pairs and the budget reaches that question count.
    ExhaustivePairwiseOrder,
}

impl QuestionPolicy {
    pub const ALL: [Self; 4] = [
        Self::AdaptiveMinimaxRegret,
        Self::FixedIdentifierOrder,
        Self::SeededIdentifierOrder,
        Self::ExhaustivePairwiseOrder,
    ];
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PolicyQuestionAnswer {
    pub question_id: String,
    pub observed_answer: ObservedPreferenceAnswer,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyEvaluationLeaf {
    pub history: Vec<PolicyQuestionAnswer>,
    pub final_training_support_ids: Vec<String>,
    pub chosen_candidate_id: Option<String>,
    /// Regret measured against the hidden held-out model. None means unresolved.
    pub realized_regret_units: Option<u64>,
    /// Minimax regret of the selected alternative over the final training support.
    pub final_training_minimax_regret_units: Option<u64>,
    pub unresolved_out_of_channel: bool,
    pub questions_used: usize,
    pub total_query_cost_units: u64,
    pub total_query_duration_units: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyEvaluationReport {
    pub policy: QuestionPolicy,
    pub question_budget: usize,
    pub seed: u64,
    pub held_out_model_id: String,
    pub leaves: Vec<PolicyEvaluationLeaf>,
    pub resolved_leaf_count: usize,
    pub unresolved_leaf_count: usize,
    /// Maximum regret among resolved leaves; unresolved leaves are reported separately.
    pub worst_resolved_leaf_regret_units: Option<u64>,
    /// Lower median among resolved leaves; leaves are not probability-weighted.
    pub lower_median_resolved_leaf_regret_units: Option<u64>,
    pub max_questions_used: usize,
    pub expansion_nodes_visited: usize,
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyEvaluationError {
    InvalidBaseRequest(PreferenceProbeError),
    InvalidChannel(ResponseRobustnessError),
    InvalidHeldOutModel(&'static str),
    HeldOutModelLeakage { model_id: String },
    AlternativeLimitExceeded { actual: usize, maximum: usize },
    TrainingModelLimitExceeded { actual: usize, maximum: usize },
    QuestionLimitExceeded { actual: usize, maximum: usize },
    QuestionBudgetExceeded { actual: usize, maximum: usize },
    ExpansionLimitExceeded { maximum: usize },
    NoReachableAnswer { question_id: String },
}

impl fmt::Display for PolicyEvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBaseRequest(error) => write!(f, "invalid base request: {error}"),
            Self::InvalidChannel(error) => write!(f, "invalid response channel: {error}"),
            Self::InvalidHeldOutModel(reason) => write!(f, "invalid held-out model: {reason}"),
            Self::HeldOutModelLeakage { model_id } => write!(
                f,
                "held-out model {model_id:?} matches a training model and would leak evaluation truth"
            ),
            Self::AlternativeLimitExceeded { actual, maximum } => {
                write!(f, "{actual} alternatives exceed policy simulator limit {maximum}")
            }
            Self::TrainingModelLimitExceeded { actual, maximum } => {
                write!(f, "{actual} training models exceed policy simulator limit {maximum}")
            }
            Self::QuestionLimitExceeded { actual, maximum } => {
                write!(f, "{actual} questions exceed policy simulator limit {maximum}")
            }
            Self::QuestionBudgetExceeded { actual, maximum } => {
                write!(f, "question budget {actual} exceeds policy simulator limit {maximum}")
            }
            Self::ExpansionLimitExceeded { maximum } => {
                write!(f, "policy enumeration exceeded its {maximum}-node limit")
            }
            Self::NoReachableAnswer { question_id } => {
                write!(f, "question {question_id:?} has no reachable answer")
            }
        }
    }
}

impl std::error::Error for PolicyEvaluationError {}

struct EvaluationContext<'a> {
    request: &'a ResidentPreferenceProbeRequest,
    channel: &'a BoundedPreferenceResponseChannel,
    policy: QuestionPolicy,
    budget: usize,
    seed: u64,
    alternative_index: BTreeMap<String, usize>,
    training_utilities: Vec<Vec<u64>>,
    training_best: Vec<u64>,
    held_out_utilities: Vec<u64>,
}

fn valid_identifier(value: &str) -> bool {
    !value.trim().is_empty() && value == value.trim()
}

fn outcome_for(left: u64, right: u64, indifference_margin: u64) -> PreferenceOutcome {
    if left.abs_diff(right) <= indifference_margin {
        PreferenceOutcome::Indifferent
    } else if left > right {
        PreferenceOutcome::LeftPreferred
    } else {
        PreferenceOutcome::RightPreferred
    }
}

fn answer_mask(
    channel: &BoundedPreferenceResponseChannel,
    outcome: PreferenceOutcome,
) -> u8 {
    channel.allowed_observed_mask_by_true_outcome[outcome.index()]
}

fn minimax_choice(
    context: &EvaluationContext<'_>,
    compatible_scenarios: &[usize],
) -> (usize, u64) {
    let choice = (0..context.request.alternatives.len())
        .map(|alternative_index| {
            let regret = compatible_scenarios
                .iter()
                .map(|scenario_index| {
                    context.training_best[*scenario_index]
                        - context.training_utilities[*scenario_index][alternative_index]
                })
                .max()
                .unwrap_or(0);
            (regret, context.request.alternatives[alternative_index].candidate_id.as_str(), alternative_index)
        })
        .min_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(right.1)))
        .expect("the base request is required to contain alternatives");
    (choice.2, choice.0)
}

fn stable_order_hash(seed: u64, id: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64 ^ seed;
    for byte in id.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn choose_next_question(
    context: &EvaluationContext<'_>,
    compatible_scenarios: &[usize],
    available_questions: &[&PairwisePreferenceQuestion],
) -> Result<PairwisePreferenceQuestion, PolicyEvaluationError> {
    match context.policy {
        QuestionPolicy::FixedIdentifierOrder | QuestionPolicy::ExhaustivePairwiseOrder => {
            available_questions
                .iter()
                .min_by_key(|question| question.question_id.as_str())
                .map(|question| (**question).clone())
                .ok_or_else(|| PolicyEvaluationError::NoReachableAnswer {
                    question_id: "<none>".to_owned(),
                })
        }
        QuestionPolicy::SeededIdentifierOrder => available_questions
            .iter()
            .min_by_key(|question| {
                (
                    stable_order_hash(context.seed, &question.question_id),
                    question.question_id.as_str(),
                )
            })
            .map(|question| (**question).clone())
            .ok_or_else(|| PolicyEvaluationError::NoReachableAnswer {
                question_id: "<none>".to_owned(),
            }),
        QuestionPolicy::AdaptiveMinimaxRegret => {
            let compatible_ids = compatible_scenarios
                .iter()
                .map(|index| context.request.scenarios[*index].scenario_id.as_str())
                .collect::<BTreeSet<_>>();
            let mut current = context.request.clone();
            current.scenarios = context
                .request
                .scenarios
                .iter()
                .filter(|scenario| compatible_ids.contains(scenario.scenario_id.as_str()))
                .cloned()
                .collect();
            current.questions = available_questions
                .iter()
                .map(|question| (**question).clone())
                .collect();
            let report = compute_response_robustness(&current, context.channel)
                .map_err(PolicyEvaluationError::InvalidChannel)?;
            report
                .questions
                .iter()
                .min_by_key(|question| {
                    (
                        Reverse(question.worst_case_regret_reduction_units),
                        question.query_cost_units,
                        question.query_duration_units,
                        question.question_id.as_str(),
                    )
                })
                .and_then(|result| {
                    available_questions
                        .iter()
                        .find(|question| question.question_id == result.question_id)
                })
                .map(|question| (**question).clone())
                .ok_or_else(|| PolicyEvaluationError::NoReachableAnswer {
                    question_id: "<adaptive>".to_owned(),
                })
        }
    }
}

fn push_terminal_leaf(
    context: &EvaluationContext<'_>,
    compatible_scenarios: &[usize],
    history: Vec<PolicyQuestionAnswer>,
    total_query_cost_units: u64,
    total_query_duration_units: u64,
    unresolved_out_of_channel: bool,
    leaves: &mut Vec<PolicyEvaluationLeaf>,
) {
    if unresolved_out_of_channel || compatible_scenarios.is_empty() {
        let questions_used = history.len();
        leaves.push(PolicyEvaluationLeaf {
            history,
            final_training_support_ids: Vec::new(),
            chosen_candidate_id: None,
            realized_regret_units: None,
            final_training_minimax_regret_units: None,
            unresolved_out_of_channel: true,
            questions_used,
            total_query_cost_units,
            total_query_duration_units,
        });
        return;
    }

    let (chosen_index, minimax_regret_units) = minimax_choice(context, compatible_scenarios);
    let hidden_best = *context
        .held_out_utilities
        .iter()
        .max()
        .expect("base request contains at least two alternatives");
    let realized_regret_units =
        hidden_best - context.held_out_utilities[chosen_index];
    let mut support_ids = compatible_scenarios
        .iter()
        .map(|index| context.request.scenarios[*index].scenario_id.clone())
        .collect::<Vec<_>>();
    support_ids.sort();

    let questions_used = history.len();
    leaves.push(PolicyEvaluationLeaf {
        history,
        final_training_support_ids: support_ids,
        chosen_candidate_id: Some(
            context.request.alternatives[chosen_index].candidate_id.clone(),
        ),
        realized_regret_units: Some(realized_regret_units),
        final_training_minimax_regret_units: Some(minimax_regret_units),
        unresolved_out_of_channel: false,
        questions_used,
        total_query_cost_units,
        total_query_duration_units,
    });
}

#[allow(clippy::too_many_arguments)]
fn enumerate_policy_tree(
    context: &EvaluationContext<'_>,
    compatible_scenarios: Vec<usize>,
    asked_question_ids: BTreeSet<String>,
    history: Vec<PolicyQuestionAnswer>,
    total_query_cost_units: u64,
    total_query_duration_units: u64,
    leaves: &mut Vec<PolicyEvaluationLeaf>,
    nodes_visited: &mut usize,
) -> Result<(), PolicyEvaluationError> {
    *nodes_visited += 1;
    if *nodes_visited > MAX_EXPANSION_NODES {
        return Err(PolicyEvaluationError::ExpansionLimitExceeded {
            maximum: MAX_EXPANSION_NODES,
        });
    }

    if compatible_scenarios.is_empty() {
        push_terminal_leaf(
            context,
            &compatible_scenarios,
            history,
            total_query_cost_units,
            total_query_duration_units,
            true,
            leaves,
        );
        return Ok(());
    }

    let available_questions = context
        .request
        .questions
        .iter()
        .filter(|question| !asked_question_ids.contains(&question.question_id))
        .collect::<Vec<_>>();
    if history.len() >= context.budget || available_questions.is_empty() {
        push_terminal_leaf(
            context,
            &compatible_scenarios,
            history,
            total_query_cost_units,
            total_query_duration_units,
            false,
            leaves,
        );
        return Ok(());
    }

    let (_, current_minimax_regret) = minimax_choice(context, &compatible_scenarios);
    if current_minimax_regret == 0 && context.policy != QuestionPolicy::ExhaustivePairwiseOrder {
        push_terminal_leaf(
            context,
            &compatible_scenarios,
            history,
            total_query_cost_units,
            total_query_duration_units,
            false,
            leaves,
        );
        return Ok(());
    }

    let question =
        choose_next_question(context, &compatible_scenarios, &available_questions)?;
    let left_index = context.alternative_index[&question.left_candidate_id];
    let right_index = context.alternative_index[&question.right_candidate_id];
    let hidden_outcome = outcome_for(
        context.held_out_utilities[left_index],
        context.held_out_utilities[right_index],
        context.request.indifference_margin_units,
    );
    let possible_observed_mask = answer_mask(context.channel, hidden_outcome);
    if possible_observed_mask == 0 {
        return Err(PolicyEvaluationError::NoReachableAnswer {
            question_id: question.question_id,
        });
    }

    for answer in ObservedPreferenceAnswer::ALL {
        if possible_observed_mask & (1_u8 << answer.index()) == 0 {
            continue;
        }
        let next_compatible = compatible_scenarios
            .iter()
            .copied()
            .filter(|scenario_index| {
                let left = context.training_utilities[*scenario_index][left_index];
                let right = context.training_utilities[*scenario_index][right_index];
                let training_outcome = outcome_for(
                    left,
                    right,
                    context.request.indifference_margin_units,
                );
                answer_mask(context.channel, training_outcome) & (1_u8 << answer.index()) != 0
            })
            .collect::<Vec<_>>();

        let mut next_history = history.clone();
        next_history.push(PolicyQuestionAnswer {
            question_id: question.question_id.clone(),
            observed_answer: answer,
        });
        let mut next_asked = asked_question_ids.clone();
        next_asked.insert(question.question_id.clone());
        enumerate_policy_tree(
            context,
            next_compatible,
            next_asked,
            next_history,
            total_query_cost_units + u64::from(question.query_cost_units),
            total_query_duration_units + u64::from(question.query_duration_units),
            leaves,
            nodes_visited,
        )?;
    }

    Ok(())
}

/// Enumerate a question-selection policy against one held-out model.
///
/// The hidden model is used only to generate reachable observations and score
/// terminal decisions; the next-question selector only receives the current
/// support from the supplied training scenarios. Model probabilities are never
/// assigned. A held-out vector identical to any training vector is rejected.
pub fn evaluate_question_policy(
    request: &ResidentPreferenceProbeRequest,
    channel: &BoundedPreferenceResponseChannel,
    held_out_model: &PreferenceScenario,
    policy: QuestionPolicy,
    question_budget: usize,
    seed: u64,
) -> Result<PolicyEvaluationReport, PolicyEvaluationError> {
    if request.alternatives.len() > MAX_POLICY_ALTERNATIVES {
        return Err(PolicyEvaluationError::AlternativeLimitExceeded {
            actual: request.alternatives.len(),
            maximum: MAX_POLICY_ALTERNATIVES,
        });
    }
    if request.scenarios.len() > MAX_TRAINING_MODELS {
        return Err(PolicyEvaluationError::TrainingModelLimitExceeded {
            actual: request.scenarios.len(),
            maximum: MAX_TRAINING_MODELS,
        });
    }
    if request.questions.len() > MAX_PAIRWISE_QUESTIONS {
        return Err(PolicyEvaluationError::QuestionLimitExceeded {
            actual: request.questions.len(),
            maximum: MAX_PAIRWISE_QUESTIONS,
        });
    }
    if question_budget > MAX_QUESTION_BUDGET {
        return Err(PolicyEvaluationError::QuestionBudgetExceeded {
            actual: question_budget,
            maximum: MAX_QUESTION_BUDGET,
        });
    }

    match compute_response_robustness(request, channel) {
        Ok(_) => {}
        Err(ResponseRobustnessError::InvalidProbe(error)) => {
            return Err(PolicyEvaluationError::InvalidBaseRequest(error));
        }
        Err(error) => return Err(PolicyEvaluationError::InvalidChannel(error)),
    }

    if !valid_identifier(&held_out_model.scenario_id) {
        return Err(PolicyEvaluationError::InvalidHeldOutModel(
            "scenario identifier is blank or has surrounding whitespace",
        ));
    }
    if request
        .scenarios
        .iter()
        .any(|scenario| scenario.scenario_id == held_out_model.scenario_id)
    {
        return Err(PolicyEvaluationError::HeldOutModelLeakage {
            model_id: held_out_model.scenario_id.clone(),
        });
    }
    if held_out_model.weights_bps.len() != request.criterion_ids.len() {
        return Err(PolicyEvaluationError::InvalidHeldOutModel(
            "weight vector length does not match criterion count",
        ));
    }
    let mut held_out_weight_sum = 0_u32;
    for weight in &held_out_model.weights_bps {
        if *weight > 10_000 {
            return Err(PolicyEvaluationError::InvalidHeldOutModel(
                "weight is outside basis-point range",
            ));
        }
        held_out_weight_sum += u32::from(*weight);
    }
    if held_out_weight_sum != 10_000 {
        return Err(PolicyEvaluationError::InvalidHeldOutModel(
            "weights must sum exactly to 10,000 basis points",
        ));
    }
    if request
        .scenarios
        .iter()
        .any(|scenario| scenario.weights_bps == held_out_model.weights_bps)
    {
        return Err(PolicyEvaluationError::HeldOutModelLeakage {
            model_id: held_out_model.scenario_id.clone(),
        });
    }

    let alternative_index = request
        .alternatives
        .iter()
        .enumerate()
        .map(|(index, alternative)| (alternative.candidate_id.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let training_utilities = request
        .scenarios
        .iter()
        .map(|scenario| {
            request
                .alternatives
                .iter()
                .map(|alternative| {
                    utility_numerator(
                        &alternative.criterion_scores_bps,
                        &scenario.weights_bps,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let training_best = training_utilities
        .iter()
        .map(|utilities| *utilities.iter().max().expect("base request has alternatives"))
        .collect::<Vec<_>>();
    let held_out_utilities = request
        .alternatives
        .iter()
        .map(|alternative| {
            utility_numerator(
                &alternative.criterion_scores_bps,
                &held_out_model.weights_bps,
            )
        })
        .collect::<Vec<_>>();

    let context = EvaluationContext {
        request,
        channel,
        policy,
        budget: question_budget,
        seed,
        alternative_index,
        training_utilities,
        training_best,
        held_out_utilities,
    };
    let mut leaves = Vec::new();
    let mut nodes_visited = 0;
    enumerate_policy_tree(
        &context,
        (0..request.scenarios.len()).collect(),
        BTreeSet::new(),
        Vec::new(),
        0,
        0,
        &mut leaves,
        &mut nodes_visited,
    )?;

    leaves.sort_by(|left, right| {
        left.history
            .cmp(&right.history)
            .then_with(|| left.chosen_candidate_id.cmp(&right.chosen_candidate_id))
    });
    let mut resolved_regrets = leaves
        .iter()
        .filter_map(|leaf| leaf.realized_regret_units)
        .collect::<Vec<_>>();
    resolved_regrets.sort_unstable();
    let worst_resolved_leaf_regret_units = resolved_regrets.iter().copied().max();
    let lower_median_resolved_leaf_regret_units = resolved_regrets
        .get(resolved_regrets.len().saturating_sub(1) / 2)
        .copied();
    let resolved_leaf_count = resolved_regrets.len();
    let unresolved_leaf_count = leaves.len() - resolved_leaf_count;
    let max_questions_used = leaves
        .iter()
        .map(|leaf| leaf.questions_used)
        .max()
        .unwrap_or(0);

    Ok(PolicyEvaluationReport {
        policy,
        question_budget,
        seed,
        held_out_model_id: held_out_model.scenario_id.clone(),
        leaves,
        resolved_leaf_count,
        unresolved_leaf_count,
        worst_resolved_leaf_regret_units,
        lower_median_resolved_leaf_regret_units,
        max_questions_used,
        expansion_nodes_visited: nodes_visited,
        claim_ceiling: "Enumerated decision-policy outcomes conditional on the supplied feasible alternatives, training models, held-out utility vector, explicit indifference margin, and set-valued response channel. Per-leaf summaries are not probabilities or expectations. No human preference validity, safety, accessibility, or SOTA superiority is established.",
    })
}

#[cfg(test)]
mod tests {
    use super::{
        PolicyEvaluationError, QuestionPolicy, evaluate_question_policy,
    };
    use crate::alternative_frontier::VerificationStatus;
    use crate::resident_preference_probe::{
        PairwisePreferenceQuestion, PreferenceAlternative, PreferenceScenario,
        ResidentPreferenceProbeRequest,
    };
    use crate::resident_preference_probe::response_robustness::{
        BoundedPreferenceResponseChannel, ObservedPreferenceAnswer,
    };

    fn alternative(id: &str, scores: &[u16]) -> PreferenceAlternative {
        let revision = format!("{:064x}", id.bytes().fold(1_u64, |v, b| v + u64::from(b)));
        PreferenceAlternative {
            candidate_id: id.to_owned(),
            revision_sha256: revision.clone(),
            verifier_receipt_sha256: format!("{:064x}", revision.bytes().fold(1_u64, |v, b| v + u64::from(b))),
            verified_revision_sha256: revision,
            hard_feasibility: VerificationStatus::Verified,
            criterion_scores_bps: scores.to_vec(),
        }
    }

    fn question(id: &str, left: &str, right: &str) -> PairwisePreferenceQuestion {
        PairwisePreferenceQuestion {
            question_id: id.to_owned(),
            left_candidate_id: left.to_owned(),
            right_candidate_id: right.to_owned(),
            query_cost_units: 1,
            query_duration_units: 1,
        }
    }

    fn request() -> ResidentPreferenceProbeRequest {
        ResidentPreferenceProbeRequest {
            criterion_ids: vec!["cost".to_owned(), "support".to_owned()],
            query_cost_unit: "effort_points".to_owned(),
            query_duration_unit: "seconds".to_owned(),
            indifference_margin_units: 0,
            alternatives: vec![
                alternative("low-cost", &[10_000, 0]),
                alternative("balanced", &[5_000, 5_000]),
                alternative("high-support", &[0, 10_000]),
            ],
            scenarios: vec![
                PreferenceScenario {
                    scenario_id: "training-cost".to_owned(),
                    weights_bps: vec![10_000, 0],
                },
                PreferenceScenario {
                    scenario_id: "training-support".to_owned(),
                    weights_bps: vec![0, 10_000],
                },
            ],
            questions: vec![
                question("pair-extremes", "high-support", "low-cost"),
                question("pair-balanced", "balanced", "low-cost"),
                question("pair-middle", "high-support", "balanced"),
            ],
        }
    }

    fn hidden_cost_biased() -> PreferenceScenario {
        PreferenceScenario {
            scenario_id: "heldout-cost-biased".to_owned(),
            weights_bps: vec![8_000, 2_000],
        }
    }

    #[test]
    fn budget_one_is_consistent_with_direct_held_out_evaluation() {
        let report = evaluate_question_policy(
            &request(),
            &BoundedPreferenceResponseChannel::exact(),
            &hidden_cost_biased(),
            QuestionPolicy::AdaptiveMinimaxRegret,
            1,
            0,
        )
        .expect("valid held-out policy evaluation");

        assert_eq!(report.leaves.len(), 1);
        assert_eq!(report.resolved_leaf_count, 1);
        assert_eq!(report.unresolved_leaf_count, 0);
        assert_eq!(report.worst_resolved_leaf_regret_units, Some(0));
        assert_eq!(report.leaves[0].realized_regret_units, Some(0));
        assert_ne!(
            report.leaves[0].history[0].observed_answer,
            ObservedPreferenceAnswer::Unresolved
        );
    }

    #[test]
    fn hidden_model_answer_outside_training_channel_is_unresolved() {
        let hidden_tie = PreferenceScenario {
            scenario_id: "heldout-balanced".to_owned(),
            weights_bps: vec![5_000, 5_000],
        };
        let report = evaluate_question_policy(
            &request(),
            &BoundedPreferenceResponseChannel::exact(),
            &hidden_tie,
            QuestionPolicy::AdaptiveMinimaxRegret,
            1,
            0,
        )
        .expect("valid held-out tie model");

        assert_eq!(report.unresolved_leaf_count, 1);
        assert_eq!(report.leaves[0].chosen_candidate_id, None);
        assert!(report.leaves[0].unresolved_out_of_channel);
    }

    #[test]
    fn full_ambiguity_enumerates_every_reachable_answer_leaf() {
        let report = evaluate_question_policy(
            &request(),
            &BoundedPreferenceResponseChannel::fully_ambiguous(),
            &hidden_cost_biased(),
            QuestionPolicy::FixedIdentifierOrder,
            1,
            0,
        )
        .expect("valid ambiguous channel");
        assert_eq!(report.leaves.len(), 4);
        assert!(report.leaves.iter().all(|leaf| leaf.questions_used == 1));
    }

    #[test]
    fn selector_does_not_leak_hidden_model_into_first_question() {
        let input = request();
        let channel = BoundedPreferenceResponseChannel {
            allowed_observed_mask_by_true_outcome: [0b0011, 0b0110, 0b1100],
        };
        let a = evaluate_question_policy(
            &input,
            &channel,
            &hidden_cost_biased(),
            QuestionPolicy::AdaptiveMinimaxRegret,
            1,
            19,
        )
        .expect("first hidden model");
        let hidden_support = PreferenceScenario {
            scenario_id: "heldout-support-biased".to_owned(),
            weights_bps: vec![2_000, 8_000],
        };
        let b = evaluate_question_policy(
            &input,
            &channel,
            &hidden_support,
            QuestionPolicy::AdaptiveMinimaxRegret,
            1,
            19,
        )
        .expect("second hidden model");
        let first_question_a = &a.leaves[0].history[0].question_id;
        let first_question_b = &b.leaves[0].history[0].question_id;
        assert_eq!(first_question_a, first_question_b);
    }

    #[test]
    fn hidden_training_vector_is_rejected_to_prevent_evaluation_leakage() {
        let hidden = PreferenceScenario {
            scenario_id: "heldout-renamed-training".to_owned(),
            weights_bps: vec![10_000, 0],
        };
        assert!(matches!(
            evaluate_question_policy(
                &request(),
                &BoundedPreferenceResponseChannel::exact(),
                &hidden,
                QuestionPolicy::AdaptiveMinimaxRegret,
                1,
                0
            ),
            Err(PolicyEvaluationError::HeldOutModelLeakage { .. })
        ));
    }

    #[test]
    fn seed_is_replayable_and_changes_only_declared_randomized_order() {
        let input = request();
        let channel = BoundedPreferenceResponseChannel::exact();
        let first = evaluate_question_policy(
            &input,
            &channel,
            &hidden_cost_biased(),
            QuestionPolicy::SeededIdentifierOrder,
            2,
            81,
        )
        .expect("valid seeded policy");
        let replay = evaluate_question_policy(
            &input,
            &channel,
            &hidden_cost_biased(),
            QuestionPolicy::SeededIdentifierOrder,
            2,
            81,
        )
        .expect("deterministic replay");
        assert_eq!(first, replay);
    }

    #[test]
    fn ambiguous_budget_two_enumerates_every_reachable_answer_history() {
        let report = evaluate_question_policy(
            &request(),
            &BoundedPreferenceResponseChannel::fully_ambiguous(),
            &hidden_cost_biased(),
            QuestionPolicy::FixedIdentifierOrder,
            2,
            0,
        )
        .expect("valid fully ambiguous policy tree");

        assert_eq!(report.leaves.len(), 16);
        assert!(report.leaves.iter().all(|leaf| leaf.questions_used == 2));
        assert!(report.leaves.iter().all(|leaf| leaf.history.len() == 2));
        assert_eq!(report.expansion_nodes_visited, 21);
    }

    #[test]
    fn exhaustive_baseline_continues_after_the_training_decision_is_identified() {
        let input = request();
        let channel = BoundedPreferenceResponseChannel::exact();
        let adaptive = evaluate_question_policy(
            &input,
            &channel,
            &hidden_cost_biased(),
            QuestionPolicy::AdaptiveMinimaxRegret,
            2,
            0,
        )
        .expect("adaptive policy");
        let exhaustive = evaluate_question_policy(
            &input,
            &channel,
            &hidden_cost_biased(),
            QuestionPolicy::ExhaustivePairwiseOrder,
            2,
            0,
        )
        .expect("exhaustive policy");

        assert_eq!(adaptive.leaves.len(), 1);
        assert_eq!(adaptive.leaves[0].questions_used, 1);
        assert_eq!(exhaustive.leaves.len(), 1);
        assert_eq!(exhaustive.leaves[0].questions_used, 2);
    }
}
