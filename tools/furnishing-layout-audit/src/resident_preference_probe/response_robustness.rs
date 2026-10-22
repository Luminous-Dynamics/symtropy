//! Robustness analysis for preference questions under explicit bounded answer ambiguity.
//!
//! Each latent preference scenario maps to a set of admissible observed answers.
//! The channel is set-valued, not probabilistic: no response frequency or Bayesian
//! posterior is implied. Ambiguous answers keep every scenario that could produce
//! them, and unsupported observations return an explicit outside-channel state.

use super::{
    PreferenceProbeError, ResidentPreferenceProbeRequest, compute_resident_preference_probe,
    minimax_regret, utility_numerator,
};
pub mod policy_evaluation;

use std::fmt;

const OBSERVED_ANSWER_MASK: u8 = 0b1111;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum PreferenceOutcome {
    LeftPreferred,
    RightPreferred,
    Indifferent,
}

impl PreferenceOutcome {
    const ALL: [Self; 3] = [Self::LeftPreferred, Self::RightPreferred, Self::Indifferent];

    fn index(self) -> usize {
        match self {
            Self::LeftPreferred => 0,
            Self::RightPreferred => 1,
            Self::Indifferent => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ObservedPreferenceAnswer {
    LeftPreferred,
    RightPreferred,
    Indifferent,
    Unresolved,
}

impl ObservedPreferenceAnswer {
    const ALL: [Self; 4] = [
        Self::LeftPreferred,
        Self::RightPreferred,
        Self::Indifferent,
        Self::Unresolved,
    ];

    fn index(self) -> usize {
        match self {
            Self::LeftPreferred => 0,
            Self::RightPreferred => 1,
            Self::Indifferent => 2,
            Self::Unresolved => 3,
        }
    }
}

/// Possible observed answers for each latent, model-predicted outcome.
///
/// Bit 0 = left preferred, bit 1 = right preferred, bit 2 = indifferent,
/// bit 3 = unresolved / insufficient information. Each mask must be nonzero
/// and contain no bits beyond bit 3. Masks encode possibility, never probability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedPreferenceResponseChannel {
    pub allowed_observed_mask_by_true_outcome: [u8; 3],
}

impl BoundedPreferenceResponseChannel {
    pub const fn exact() -> Self {
        Self {
            allowed_observed_mask_by_true_outcome: [0b0001, 0b0010, 0b0100],
        }
    }

    pub const fn fully_ambiguous() -> Self {
        Self {
            allowed_observed_mask_by_true_outcome: [0b1111, 0b1111, 0b1111],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservedAnswerSupport {
    pub answer: ObservedPreferenceAnswer,
    /// All scenarios whose declared response channel allows this recorded answer.
    /// This list may overlap the support of other answers.
    pub compatible_scenario_ids: Vec<String>,
    /// None when this answer cannot be emitted under the declared channel.
    pub post_answer_minimax_regret_units: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RobustPreferenceQuestionResult {
    pub question_id: String,
    pub baseline_minimax_regret_units: u64,
    /// Worst regret among every answer that the channel says is possible.
    pub worst_case_post_answer_regret_units: u64,
    pub worst_case_regret_reduction_units: u64,
    pub answer_supports: Vec<ObservedAnswerSupport>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreferenceResponseRobustnessReport {
    pub baseline_minimax_regret_units: u64,
    pub channel: BoundedPreferenceResponseChannel,
    pub questions: Vec<RobustPreferenceQuestionResult>,
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResponseRobustnessError {
    InvalidProbe(PreferenceProbeError),
    InvalidResponseMask {
        true_outcome: PreferenceOutcome,
        mask: u8,
    },
    NoPossibleObservedAnswer {
        question_id: String,
    },
    RegretInvariantViolation {
        question_id: String,
    },
}

impl fmt::Display for ResponseRobustnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProbe(error) => write!(f, "invalid preference probe: {error}"),
            Self::InvalidResponseMask { true_outcome, mask } => write!(
                f,
                "response mask {mask:#06b} for {true_outcome:?} must be nonzero and use only four answer bits"
            ),
            Self::NoPossibleObservedAnswer { question_id } => write!(
                f,
                "question {question_id:?} has no possible observed answer under the channel"
            ),
            Self::RegretInvariantViolation { question_id } => write!(
                f,
                "question {question_id:?} has greater regret after observing an answer than before asking"
            ),
        }
    }
}

impl std::error::Error for ResponseRobustnessError {}

/// Evaluate each query against an explicit set-valued answer channel.
///
/// A latent model predicts left-preferred, right-preferred, or indifferent. The
/// caller supplies the possible recorded responses for each latent outcome.
/// Unlike the exact probe, observed-answer supports may overlap: an ambiguous
/// answer cannot silently discard models still compatible with it. No
/// probabilities are attached to models or answers.
pub fn compute_response_robustness(
    request: &ResidentPreferenceProbeRequest,
    channel: &BoundedPreferenceResponseChannel,
) -> Result<PreferenceResponseRobustnessReport, ResponseRobustnessError> {
    let exact_report = compute_resident_preference_probe(request)
        .map_err(ResponseRobustnessError::InvalidProbe)?;

    for true_outcome in PreferenceOutcome::ALL {
        let mask = channel.allowed_observed_mask_by_true_outcome[true_outcome.index()];
        if mask == 0 || mask & !OBSERVED_ANSWER_MASK != 0 {
            return Err(ResponseRobustnessError::InvalidResponseMask { true_outcome, mask });
        }
    }

    let mut alternatives = request.alternatives.iter().collect::<Vec<_>>();
    alternatives.sort_by_key(|alternative| alternative.candidate_id.clone());
    let alternative_index = alternatives
        .iter()
        .enumerate()
        .map(|(index, alternative)| (alternative.candidate_id.as_str(), index))
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut scenarios = request.scenarios.iter().collect::<Vec<_>>();
    scenarios.sort_by_key(|scenario| scenario.scenario_id.clone());
    let utilities = scenarios
        .iter()
        .map(|scenario| {
            alternatives
                .iter()
                .map(|alternative| {
                    utility_numerator(&alternative.criterion_scores_bps, &scenario.weights_bps)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let scenario_best = utilities
        .iter()
        .map(|scenario_utilities| {
            *scenario_utilities
                .iter()
                .max()
                .expect("the exact probe rejects empty alternatives")
        })
        .collect::<Vec<_>>();

    let mut questions = request.questions.iter().collect::<Vec<_>>();
    questions.sort_by_key(|question| question.question_id.clone());
    let mut question_results = Vec::with_capacity(questions.len());

    for question in questions {
        let left_index = alternative_index[question.left_candidate_id.as_str()];
        let right_index = alternative_index[question.right_candidate_id.as_str()];
        let mut scenario_supports = vec![Vec::<usize>::new(); ObservedPreferenceAnswer::ALL.len()];

        for (scenario_index, scenario_utilities) in utilities.iter().enumerate() {
            let left_utility = scenario_utilities[left_index];
            let right_utility = scenario_utilities[right_index];
            let true_outcome =
                if left_utility.abs_diff(right_utility) <= request.indifference_margin_units {
                    PreferenceOutcome::Indifferent
                } else if left_utility > right_utility {
                    PreferenceOutcome::LeftPreferred
                } else {
                    PreferenceOutcome::RightPreferred
                };
            let mask = channel.allowed_observed_mask_by_true_outcome[true_outcome.index()];
            for answer in ObservedPreferenceAnswer::ALL {
                if mask & (1_u8 << answer.index()) != 0 {
                    scenario_supports[answer.index()].push(scenario_index);
                }
            }
        }

        let answer_supports = ObservedPreferenceAnswer::ALL
            .into_iter()
            .map(|answer| {
                let support = &scenario_supports[answer.index()];
                let regret = (!support.is_empty()).then(|| {
                    minimax_regret(support, &utilities, &scenario_best, alternatives.len())
                });
                ObservedAnswerSupport {
                    answer,
                    compatible_scenario_ids: support
                        .iter()
                        .map(|index| scenarios[*index].scenario_id.clone())
                        .collect(),
                    post_answer_minimax_regret_units: regret,
                }
            })
            .collect::<Vec<_>>();

        let Some(worst_case_post_answer_regret_units) = answer_supports
            .iter()
            .filter_map(|support| support.post_answer_minimax_regret_units)
            .max()
        else {
            return Err(ResponseRobustnessError::NoPossibleObservedAnswer {
                question_id: question.question_id.clone(),
            });
        };
        if worst_case_post_answer_regret_units > exact_report.baseline_minimax_regret_units {
            return Err(ResponseRobustnessError::RegretInvariantViolation {
                question_id: question.question_id.clone(),
            });
        }

        question_results.push(RobustPreferenceQuestionResult {
            question_id: question.question_id.clone(),
            baseline_minimax_regret_units: exact_report.baseline_minimax_regret_units,
            worst_case_post_answer_regret_units,
            worst_case_regret_reduction_units: exact_report.baseline_minimax_regret_units
                - worst_case_post_answer_regret_units,
            answer_supports,
        });
    }

    Ok(PreferenceResponseRobustnessReport {
        baseline_minimax_regret_units: exact_report.baseline_minimax_regret_units,
        channel: channel.clone(),
        questions: question_results,
        claim_ceiling: "Exact minimax-regret robustness conditional on caller-supplied finite models, hard-verified alternatives, explicit integer indifference margin, and a declared set-valued answer channel. No probabilities, human validity, physical feasibility, or resident preference fit is established.",
    })
}

/// Resolve an observed answer without nearest-model forcing.
///
/// An answer with no compatible scenario is surfaced as outside the declared
/// channel. It must not be assigned to the nearest preference model implicitly.
pub fn resolve_observed_answer(
    report: &PreferenceResponseRobustnessReport,
    question_id: &str,
    answer: ObservedPreferenceAnswer,
) -> Result<ObservedAnswerResolution, AnswerResolutionError> {
    let question = report
        .questions
        .iter()
        .find(|question| question.question_id == question_id)
        .ok_or_else(|| AnswerResolutionError::UnknownQuestion {
            question_id: question_id.to_owned(),
        })?;
    let support = question
        .answer_supports
        .iter()
        .find(|support| support.answer == answer)
        .expect("reports include every observed-answer category");

    if support.compatible_scenario_ids.is_empty() {
        Ok(ObservedAnswerResolution::OutsideDeclaredChannel {
            question_id: question_id.to_owned(),
            answer,
        })
    } else {
        Ok(ObservedAnswerResolution::SupportedByModels {
            question_id: question_id.to_owned(),
            answer,
            compatible_scenario_ids: support.compatible_scenario_ids.clone(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservedAnswerResolution {
    SupportedByModels {
        question_id: String,
        answer: ObservedPreferenceAnswer,
        compatible_scenario_ids: Vec<String>,
    },
    OutsideDeclaredChannel {
        question_id: String,
        answer: ObservedPreferenceAnswer,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnswerResolutionError {
    UnknownQuestion { question_id: String },
}

impl fmt::Display for AnswerResolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownQuestion { question_id } => {
                write!(f, "unknown question identifier {question_id:?}")
            }
        }
    }
}

impl std::error::Error for AnswerResolutionError {}

#[cfg(test)]
mod tests {
    use super::{
        AnswerResolutionError, BoundedPreferenceResponseChannel, ObservedAnswerResolution,
        ObservedPreferenceAnswer, PreferenceOutcome, ResponseRobustnessError,
        compute_response_robustness, resolve_observed_answer,
    };
    use crate::alternative_frontier::VerificationStatus;
    use crate::resident_preference_probe::{
        PairwisePreferenceQuestion, PreferenceAlternative, PreferenceScenario,
        ResidentPreferenceProbeRequest, compute_resident_preference_probe,
    };

    fn alternative(id: &str, scores: &[u16]) -> PreferenceAlternative {
        let revision = format!(
            "{:064x}",
            id.bytes()
                .fold(1_u64, |value, byte| value + u64::from(byte))
        );
        PreferenceAlternative {
            candidate_id: id.to_owned(),
            revision_sha256: revision.clone(),
            verifier_receipt_sha256: format!(
                "{:064x}",
                revision
                    .bytes()
                    .fold(1_u64, |value, byte| value + u64::from(byte))
            ),
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
            criterion_ids: vec!["affordability".to_owned(), "task_support".to_owned()],
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
                    scenario_id: "cost-first".to_owned(),
                    weights_bps: vec![10_000, 0],
                },
                PreferenceScenario {
                    scenario_id: "support-first".to_owned(),
                    weights_bps: vec![0, 10_000],
                },
            ],
            questions: vec![
                question("ask-extremes", "low-cost", "high-support"),
                question("ask-balanced", "low-cost", "balanced"),
            ],
        }
    }

    #[test]
    fn exact_channel_matches_the_exact_partition_regret() {
        let input = request();
        let exact = compute_resident_preference_probe(&input).expect("valid exact probe");
        let robust =
            compute_response_robustness(&input, &BoundedPreferenceResponseChannel::exact())
                .expect("valid exact channel");

        assert_eq!(
            robust.baseline_minimax_regret_units,
            exact.baseline_minimax_regret_units
        );
        assert_eq!(robust.baseline_minimax_regret_units, 50_000_000);
        for question in &exact.questions {
            let robust_question = robust
                .questions
                .iter()
                .find(|row| row.question_id == question.question_id)
                .expect("robust row exists");
            assert_eq!(
                robust_question.worst_case_post_answer_regret_units,
                question.post_answer_worst_case_regret_units
            );
            assert_eq!(
                robust_question.worst_case_regret_reduction_units,
                question.worst_case_regret_reduction_units
            );
            let counts = match &question.answer_counts {
                super::super::PreferenceAnswerCounts::ExactModelPartition {
                    left_preferred,
                    right_preferred,
                    indifferent,
                } => (*left_preferred, *right_preferred, *indifferent),
            };
            assert_eq!(
                robust_question
                    .answer_supports
                    .iter()
                    .map(|support| support.compatible_scenario_ids.len())
                    .collect::<Vec<_>>(),
                vec![counts.0, counts.1, counts.2, 0]
            );
        }
    }

    #[test]
    fn widening_answer_ambiguity_cannot_improve_the_worst_case_bound() {
        let input = request();
        let exact = compute_response_robustness(&input, &BoundedPreferenceResponseChannel::exact())
            .expect("exact channel");
        let noisy = compute_response_robustness(
            &input,
            &BoundedPreferenceResponseChannel {
                allowed_observed_mask_by_true_outcome: [0b0011, 0b0110, 0b1100],
            },
        )
        .expect("noisy set-valued channel");

        for exact_question in &exact.questions {
            let noisy_question = noisy
                .questions
                .iter()
                .find(|row| row.question_id == exact_question.question_id)
                .expect("matching question");
            assert!(
                noisy_question.worst_case_post_answer_regret_units
                    >= exact_question.worst_case_post_answer_regret_units
            );
            assert!(
                noisy_question.worst_case_regret_reduction_units
                    <= exact_question.worst_case_regret_reduction_units
            );
        }
    }

    #[test]
    fn fully_ambiguous_responses_have_zero_worst_case_information_value() {
        let input = request();
        let report = compute_response_robustness(
            &input,
            &BoundedPreferenceResponseChannel::fully_ambiguous(),
        )
        .expect("fully ambiguous channel");

        for question in report.questions {
            assert_eq!(
                question.worst_case_post_answer_regret_units,
                report.baseline_minimax_regret_units
            );
            assert_eq!(question.worst_case_regret_reduction_units, 0);
            assert!(
                question
                    .answer_supports
                    .iter()
                    .all(|support| support.compatible_scenario_ids.len() == 2)
            );
        }
    }

    #[test]
    fn unsupported_answer_is_typed_instead_of_nearest_model_forcing() {
        let input = request();
        let report =
            compute_response_robustness(&input, &BoundedPreferenceResponseChannel::exact())
                .expect("exact channel");
        let resolution = resolve_observed_answer(
            &report,
            "ask-extremes",
            ObservedPreferenceAnswer::Unresolved,
        )
        .expect("known question");
        assert_eq!(
            resolution,
            ObservedAnswerResolution::OutsideDeclaredChannel {
                question_id: "ask-extremes".to_owned(),
                answer: ObservedPreferenceAnswer::Unresolved,
            }
        );

        assert!(matches!(
            resolve_observed_answer(
                &report,
                "missing-question",
                ObservedPreferenceAnswer::Unresolved
            ),
            Err(AnswerResolutionError::UnknownQuestion { .. })
        ));
    }

    #[test]
    fn explicit_unresolved_answers_keep_every_compatible_model() {
        let report = compute_response_robustness(
            &request(),
            &BoundedPreferenceResponseChannel {
                allowed_observed_mask_by_true_outcome: [0b1001, 0b1010, 0b1100],
            },
        )
        .expect("channel includes unresolved");
        let resolution = resolve_observed_answer(
            &report,
            "ask-extremes",
            ObservedPreferenceAnswer::Unresolved,
        )
        .expect("known question");
        assert_eq!(
            resolution,
            ObservedAnswerResolution::SupportedByModels {
                question_id: "ask-extremes".to_owned(),
                answer: ObservedPreferenceAnswer::Unresolved,
                compatible_scenario_ids: vec!["cost-first".to_owned(), "support-first".to_owned()],
            }
        );
    }

    #[test]
    fn empty_or_out_of_range_response_masks_fail_closed() {
        let cases = [
            (0_u8, PreferenceOutcome::LeftPreferred),
            (0b1_0000_u8, PreferenceOutcome::RightPreferred),
        ];
        for (mask, expected) in cases {
            let mut channel = BoundedPreferenceResponseChannel::exact();
            channel.allowed_observed_mask_by_true_outcome[expected.index()] = mask;
            assert!(matches!(
                compute_response_robustness(&request(), &channel),
                Err(ResponseRobustnessError::InvalidResponseMask {
                    true_outcome,
                    mask: actual
                }) if true_outcome == expected && actual == mask
            ));
        }
    }

    #[test]
    fn report_is_invariant_to_input_order() {
        let mut input = request();
        let channel = BoundedPreferenceResponseChannel {
            allowed_observed_mask_by_true_outcome: [0b0011, 0b0110, 0b1100],
        };
        let first = compute_response_robustness(&input, &channel).expect("valid input");
        input.alternatives.reverse();
        input.scenarios.reverse();
        input.questions.reverse();
        let second = compute_response_robustness(&input, &channel).expect("permuted input");
        assert_eq!(first, second);
    }

    #[test]
    fn held_out_balanced_model_is_not_force_fitted_to_extreme_answers() {
        let input = request();
        let report =
            compute_response_robustness(&input, &BoundedPreferenceResponseChannel::exact())
                .expect("exact channel");
        let held_out_weights_bps = [5_000_u16, 5_000_u16];
        let left = input
            .alternatives
            .iter()
            .find(|alternative| alternative.candidate_id == "low-cost")
            .expect("left candidate");
        let right = input
            .alternatives
            .iter()
            .find(|alternative| alternative.candidate_id == "high-support")
            .expect("right candidate");
        let utility = |scores: &[u16]| {
            scores
                .iter()
                .zip(held_out_weights_bps)
                .map(|(score, weight)| u64::from(*score) * u64::from(weight))
                .sum::<u64>()
        };
        assert_eq!(
            utility(&left.criterion_scores_bps),
            utility(&right.criterion_scores_bps)
        );

        // Neither supplied training scenario predicts indifference for this pair.
        let resolution = resolve_observed_answer(
            &report,
            "ask-extremes",
            ObservedPreferenceAnswer::Indifferent,
        )
        .expect("known question");
        assert_eq!(
            resolution,
            ObservedAnswerResolution::OutsideDeclaredChannel {
                question_id: "ask-extremes".to_owned(),
                answer: ObservedPreferenceAnswer::Indifferent,
            }
        );
    }
}
