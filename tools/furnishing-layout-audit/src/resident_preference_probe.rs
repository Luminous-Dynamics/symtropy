//! Deterministic, finite-model preference-query selection for feasible furnishing alternatives.
//!
//! This module ranks *questions*, not residents. It assumes candidates already
//! passed an independent hard-constraint verifier, uses only the explicitly
//! supplied finite preference-model set, and assigns no probabilities to models.
//! All preference scores and weights are integer basis points so ties and regret
//! comparisons are deterministic. A caller remains responsible for consent,
//! source qualification, and deciding whether the model set reflects real needs.

use crate::alternative_frontier::VerificationStatus;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const PREFERENCE_SCALE_BPS: u16 = 10_000;
const MAX_OBJECTIVES: usize = 32;
const MAX_ALTERNATIVES: usize = 64;
const MAX_SCENARIOS: usize = 256;
const MAX_QUESTIONS: usize = 2_048;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreferenceAlternative {
    pub candidate_id: String,
    /// SHA-256-shaped identifier of the exact candidate revision. Syntax only;
    /// this module does not recompute the digest.
    pub revision_sha256: String,
    /// SHA-256-shaped identifier of the independent hard-feasibility receipt.
    pub verifier_receipt_sha256: String,
    /// Candidate revision that the receipt claims to cover; must match exactly.
    pub verified_revision_sha256: String,
    /// Caller-reported result from the independent geometry/mandatory-requirement verifier.
    pub hard_feasibility: VerificationStatus,
    /// Upstream-normalized criterion scores: 0..=10,000 and higher is preferred.
    /// Hard constraints must not be encoded as compensable preference criteria.
    pub criterion_scores_bps: Vec<u16>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreferenceScenario {
    pub scenario_id: String,
    /// Explicitly plausible preference weights, not probabilities. Must sum to
    /// exactly 10,000; no hidden weight normalization is performed.
    pub weights_bps: Vec<u16>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairwisePreferenceQuestion {
    pub question_id: String,
    pub left_candidate_id: String,
    pub right_candidate_id: String,
    /// Separate, caller-declared query effort/cost measure.
    pub query_cost_units: u32,
    /// Separate, caller-declared time measure.
    pub query_duration_units: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentPreferenceProbeRequest {
    /// Stable, unique criterion identifiers in score-vector order.
    pub criterion_ids: Vec<String>,
    /// Unit identifier for query_cost_units, shared by all questions.
    pub query_cost_unit: String,
    /// Unit identifier for query_duration_units, shared by all questions.
    pub query_duration_unit: String,
    pub alternatives: Vec<PreferenceAlternative>,
    pub scenarios: Vec<PreferenceScenario>,
    pub questions: Vec<PairwisePreferenceQuestion>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PreferenceAnswerCounts {
    /// Scenarios are partitioned by the declared utility model only. Real answers
    /// may be noisy, context-dependent or inconsistent with all listed models.
    ExactModelPartition {
        left_preferred: usize,
        right_preferred: usize,
        indifferent: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreferenceQuestionResult {
    pub question_id: String,
    pub left_candidate_id: String,
    pub right_candidate_id: String,
    pub query_cost_units: u32,
    pub query_duration_units: u32,
    /// Maximum regret before asking, in exact weighted-score numerator units
    /// (criterion basis points × weight basis points).
    pub baseline_minimax_regret_units: u64,
    /// Worst minimax regret remaining after whichever answer partition occurs.
    pub post_answer_worst_case_regret_units: u64,
    /// Gross reduction, never negative; not expected/average value of information.
    pub worst_case_regret_reduction_units: u64,
    pub answer_counts: PreferenceAnswerCounts,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentPreferenceProbeReport {
    pub baseline_minimax_regret_units: u64,
    pub questions: Vec<PreferenceQuestionResult>,
    /// Exact Pareto frontier over maximize regret reduction, minimize query cost,
    /// and minimize duration. Ties are preserved; the function chooses no winner.
    pub frontier_question_ids: Vec<String>,
    pub criterion_ids: Vec<String>,
    pub query_cost_unit: String,
    pub query_duration_unit: String,
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PreferenceProbeError {
    EmptyCriteria,
    EmptyAlternatives,
    EmptyScenarios,
    EmptyQuestions,
    InputLimitExceeded { field: &'static str, maximum: usize },
    InvalidIdentifier { field: &'static str, index: usize },
    DuplicateIdentifier { field: &'static str, id: String },
    DuplicateQuestionPair { left: String, right: String },
    CriterionVectorLength { candidate_id: String, actual: usize, expected: usize },
    ScoreOutOfRange { candidate_id: String, criterion_id: String, score_bps: u16 },
    AlternativeNotVerified { candidate_id: String, status: VerificationStatus },
    InvalidDigest { candidate_id: String, field: &'static str },
    StaleReceiptBinding { candidate_id: String },
    ScenarioWeightLength { scenario_id: String, actual: usize, expected: usize },
    ScenarioWeightOutOfRange { scenario_id: String, weight_bps: u16 },
    ScenarioWeightSum { scenario_id: String, actual: u32 },
    UnknownQuestionCandidate { question_id: String, candidate_id: String },
    SelfComparison { question_id: String },
    RegretInvariantViolation { question_id: String },
}

impl fmt::Display for PreferenceProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCriteria => write!(f, "preference probe has no criteria"),
            Self::EmptyAlternatives => write!(f, "preference probe has fewer than two alternatives"),
            Self::EmptyScenarios => write!(f, "preference probe has no preference scenarios"),
            Self::EmptyQuestions => write!(f, "preference probe has no pairwise questions"),
            Self::InputLimitExceeded { field, maximum } => {
                write!(f, "{field} exceeds the supported maximum of {maximum}")
            }
            Self::InvalidIdentifier { field, index } => {
                write!(f, "{field} at index {index} is blank or has surrounding whitespace")
            }
            Self::DuplicateIdentifier { field, id } => {
                write!(f, "duplicate {field} identifier {id:?}")
            }
            Self::DuplicateQuestionPair { left, right } => {
                write!(f, "pairwise question for {left:?} and {right:?} is duplicated")
            }
            Self::CriterionVectorLength { candidate_id, actual, expected } => write!(
                f,
                "candidate {candidate_id:?} has {actual} criterion scores, expected {expected}"
            ),
            Self::ScoreOutOfRange { candidate_id, criterion_id, score_bps } => write!(
                f,
                "candidate {candidate_id:?} has out-of-range score {score_bps} for {criterion_id:?}"
            ),
            Self::AlternativeNotVerified { candidate_id, status } => {
                write!(f, "candidate {candidate_id:?} is not hard-verified: {status:?}")
            }
            Self::InvalidDigest { candidate_id, field } => {
                write!(f, "candidate {candidate_id:?} has a malformed {field}")
            }
            Self::StaleReceiptBinding { candidate_id } => {
                write!(f, "candidate {candidate_id:?} receipt binds a different revision")
            }
            Self::ScenarioWeightLength { scenario_id, actual, expected } => write!(
                f,
                "scenario {scenario_id:?} has {actual} weights, expected {expected}"
            ),
            Self::ScenarioWeightOutOfRange { scenario_id, weight_bps } => write!(
                f,
                "scenario {scenario_id:?} has out-of-range weight {weight_bps}"
            ),
            Self::ScenarioWeightSum { scenario_id, actual } => write!(
                f,
                "scenario {scenario_id:?} weights sum to {actual}, expected 10000"
            ),
            Self::UnknownQuestionCandidate { question_id, candidate_id } => write!(
                f,
                "question {question_id:?} references unknown candidate {candidate_id:?}"
            ),
            Self::SelfComparison { question_id } => {
                write!(f, "question {question_id:?} compares a candidate with itself")
            }
            Self::RegretInvariantViolation { question_id } => write!(
                f,
                "question {question_id:?} increased minimax regret after partitioning scenarios"
            ),
        }
    }
}

impl std::error::Error for PreferenceProbeError {}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value == value.trim()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn utility_numerator(scores_bps: &[u16], weights_bps: &[u16]) -> u64 {
    scores_bps
        .iter()
        .zip(weights_bps)
        .map(|(score, weight)| u64::from(*score) * u64::from(*weight))
        .sum()
}

fn minimax_regret(
    scenario_indices: &[usize],
    utilities: &[Vec<u64>],
    scenario_best: &[u64],
    alternative_count: usize,
) -> u64 {
    debug_assert!(!scenario_indices.is_empty());
    (0..alternative_count)
        .map(|alternative_index| {
            scenario_indices
                .iter()
                .map(|scenario_index| {
                    scenario_best[*scenario_index] - utilities[*scenario_index][alternative_index]
                })
                .max()
                .unwrap_or(0)
        })
        .min()
        .expect("validated request has alternatives")
}

fn question_dominates(left: &PreferenceQuestionResult, right: &PreferenceQuestionResult) -> bool {
    let no_worse = left.worst_case_regret_reduction_units
        >= right.worst_case_regret_reduction_units
        && left.query_cost_units <= right.query_cost_units
        && left.query_duration_units <= right.query_duration_units;
    let strictly_better = left.worst_case_regret_reduction_units
        > right.worst_case_regret_reduction_units
        || left.query_cost_units < right.query_cost_units
        || left.query_duration_units < right.query_duration_units;
    no_worse && strictly_better
}

/// Rank proposed pairwise questions by how much they can reduce worst-case
/// decision regret over the *explicit finite scenario set*.
///
/// Each scenario is a plausible preference-weight vector, not a probabilistic
/// sample. A question partitions scenarios into left-preferred, right-preferred
/// and indifferent outcomes using exact integer utility numerators. The post-query
/// regret is the maximum of minimax regret in each non-empty partition. Cost and
/// duration remain separate Pareto dimensions. No question or alternative is
/// declared universally best.
///
/// All alternatives must already be hard-verified with a receipt bound to the
/// exact candidate revision. SHA-256 values are syntax-checked only: receipt
/// authentication and score-source qualification remain the caller's job.
pub fn compute_resident_preference_probe(
    request: &ResidentPreferenceProbeRequest,
) -> Result<ResidentPreferenceProbeReport, PreferenceProbeError> {
    let criterion_count = request.criterion_ids.len();
    if criterion_count == 0 {
        return Err(PreferenceProbeError::EmptyCriteria);
    }
    if criterion_count > MAX_OBJECTIVES {
        return Err(PreferenceProbeError::InputLimitExceeded {
            field: "criteria",
            maximum: MAX_OBJECTIVES,
        });
    }
    if request.alternatives.len() < 2 {
        return Err(PreferenceProbeError::EmptyAlternatives);
    }
    if request.alternatives.len() > MAX_ALTERNATIVES {
        return Err(PreferenceProbeError::InputLimitExceeded {
            field: "alternatives",
            maximum: MAX_ALTERNATIVES,
        });
    }
    if request.scenarios.is_empty() {
        return Err(PreferenceProbeError::EmptyScenarios);
    }
    if request.scenarios.len() > MAX_SCENARIOS {
        return Err(PreferenceProbeError::InputLimitExceeded {
            field: "scenarios",
            maximum: MAX_SCENARIOS,
        });
    }
    if request.questions.is_empty() {
        return Err(PreferenceProbeError::EmptyQuestions);
    }
    if request.questions.len() > MAX_QUESTIONS {
        return Err(PreferenceProbeError::InputLimitExceeded {
            field: "questions",
            maximum: MAX_QUESTIONS,
        });
    }
    if !valid_id(&request.query_cost_unit) || !valid_id(&request.query_duration_unit) {
        return Err(PreferenceProbeError::InvalidIdentifier {
            field: "query unit",
            index: 0,
        });
    }

    let mut criterion_ids = BTreeSet::new();
    for (index, criterion_id) in request.criterion_ids.iter().enumerate() {
        if !valid_id(criterion_id) {
            return Err(PreferenceProbeError::InvalidIdentifier {
                field: "criterion",
                index,
            });
        }
        if !criterion_ids.insert(criterion_id.as_str()) {
            return Err(PreferenceProbeError::DuplicateIdentifier {
                field: "criterion",
                id: criterion_id.clone(),
            });
        }
    }

    let mut alternative_ids = BTreeSet::new();
    for alternative in &request.alternatives {
        if !valid_id(&alternative.candidate_id) {
            return Err(PreferenceProbeError::InvalidIdentifier {
                field: "candidate",
                index: 0,
            });
        }
        if !alternative_ids.insert(alternative.candidate_id.as_str()) {
            return Err(PreferenceProbeError::DuplicateIdentifier {
                field: "candidate",
                id: alternative.candidate_id.clone(),
            });
        }
        if alternative.hard_feasibility != VerificationStatus::Verified {
            return Err(PreferenceProbeError::AlternativeNotVerified {
                candidate_id: alternative.candidate_id.clone(),
                status: alternative.hard_feasibility,
            });
        }
        for (value, field) in [
            (alternative.revision_sha256.as_str(), "revision SHA-256"),
            (alternative.verifier_receipt_sha256.as_str(), "receipt SHA-256"),
            (
                alternative.verified_revision_sha256.as_str(),
                "receipt-bound revision SHA-256",
            ),
        ] {
            if !valid_sha256(value) {
                return Err(PreferenceProbeError::InvalidDigest {
                    candidate_id: alternative.candidate_id.clone(),
                    field,
                });
            }
        }
        if alternative.revision_sha256 != alternative.verified_revision_sha256 {
            return Err(PreferenceProbeError::StaleReceiptBinding {
                candidate_id: alternative.candidate_id.clone(),
            });
        }
        if alternative.criterion_scores_bps.len() != criterion_count {
            return Err(PreferenceProbeError::CriterionVectorLength {
                candidate_id: alternative.candidate_id.clone(),
                actual: alternative.criterion_scores_bps.len(),
                expected: criterion_count,
            });
        }
        for (index, score) in alternative.criterion_scores_bps.iter().enumerate() {
            if *score > PREFERENCE_SCALE_BPS {
                return Err(PreferenceProbeError::ScoreOutOfRange {
                    candidate_id: alternative.candidate_id.clone(),
                    criterion_id: request.criterion_ids[index].clone(),
                    score_bps: *score,
                });
            }
        }
    }

    let mut scenario_ids = BTreeSet::new();
    for scenario in &request.scenarios {
        if !valid_id(&scenario.scenario_id) {
            return Err(PreferenceProbeError::InvalidIdentifier {
                field: "scenario",
                index: 0,
            });
        }
        if !scenario_ids.insert(scenario.scenario_id.as_str()) {
            return Err(PreferenceProbeError::DuplicateIdentifier {
                field: "scenario",
                id: scenario.scenario_id.clone(),
            });
        }
        if scenario.weights_bps.len() != criterion_count {
            return Err(PreferenceProbeError::ScenarioWeightLength {
                scenario_id: scenario.scenario_id.clone(),
                actual: scenario.weights_bps.len(),
                expected: criterion_count,
            });
        }
        let mut sum = 0_u32;
        for weight in &scenario.weights_bps {
            if *weight > PREFERENCE_SCALE_BPS {
                return Err(PreferenceProbeError::ScenarioWeightOutOfRange {
                    scenario_id: scenario.scenario_id.clone(),
                    weight_bps: *weight,
                });
            }
            sum += u32::from(*weight);
        }
        if sum != u32::from(PREFERENCE_SCALE_BPS) {
            return Err(PreferenceProbeError::ScenarioWeightSum {
                scenario_id: scenario.scenario_id.clone(),
                actual: sum,
            });
        }
    }

    let mut alternatives = request.alternatives.iter().collect::<Vec<_>>();
    alternatives.sort_by_key(|alternative| alternative.candidate_id.clone());
    let alternative_index = alternatives
        .iter()
        .enumerate()
        .map(|(index, alternative)| (alternative.candidate_id.as_str(), index))
        .collect::<BTreeMap<_, _>>();

    let mut scenarios = request.scenarios.iter().collect::<Vec<_>>();
    scenarios.sort_by_key(|scenario| scenario.scenario_id.clone());
    let utilities = scenarios
        .iter()
        .map(|scenario| {
            alternatives
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
    let scenario_best = utilities
        .iter()
        .map(|scenario_utilities| {
            *scenario_utilities
                .iter()
                .max()
                .expect("validated request has at least two alternatives")
        })
        .collect::<Vec<_>>();
    let all_scenarios = (0..scenarios.len()).collect::<Vec<_>>();
    let baseline_regret = minimax_regret(
        &all_scenarios,
        &utilities,
        &scenario_best,
        alternatives.len(),
    );

    let mut questions = request.questions.iter().collect::<Vec<_>>();
    questions.sort_by_key(|question| question.question_id.clone());
    let mut seen_question_ids = BTreeSet::new();
    let mut seen_pairs = BTreeSet::new();
    for (index, question) in questions.iter().enumerate() {
        if !valid_id(&question.question_id) {
            return Err(PreferenceProbeError::InvalidIdentifier {
                field: "question",
                index,
            });
        }
        if !seen_question_ids.insert(question.question_id.as_str()) {
            return Err(PreferenceProbeError::DuplicateIdentifier {
                field: "question",
                id: question.question_id.clone(),
            });
        }
        if question.left_candidate_id == question.right_candidate_id {
            return Err(PreferenceProbeError::SelfComparison {
                question_id: question.question_id.clone(),
            });
        }
        for candidate_id in [
            question.left_candidate_id.as_str(),
            question.right_candidate_id.as_str(),
        ] {
            if !alternative_index.contains_key(candidate_id) {
                return Err(PreferenceProbeError::UnknownQuestionCandidate {
                    question_id: question.question_id.clone(),
                    candidate_id: candidate_id.to_owned(),
                });
            }
        }
        let pair = if question.left_candidate_id < question.right_candidate_id {
            (
                question.left_candidate_id.as_str(),
                question.right_candidate_id.as_str(),
            )
        } else {
            (
                question.right_candidate_id.as_str(),
                question.left_candidate_id.as_str(),
            )
        };
        if !seen_pairs.insert(pair) {
            return Err(PreferenceProbeError::DuplicateQuestionPair {
                left: pair.0.to_owned(),
                right: pair.1.to_owned(),
            });
        }
    }

    let mut results = Vec::with_capacity(questions.len());
    for question in questions {
        let left_index = alternative_index[question.left_candidate_id.as_str()];
        let right_index = alternative_index[question.right_candidate_id.as_str()];
        let mut partitions = [Vec::new(), Vec::new(), Vec::new()];
        for (scenario_index, scenario_utilities) in utilities.iter().enumerate() {
            let outcome_index = match scenario_utilities[left_index].cmp(&scenario_utilities[right_index]) {
                std::cmp::Ordering::Greater => 0,
                std::cmp::Ordering::Less => 1,
                std::cmp::Ordering::Equal => 2,
            };
            partitions[outcome_index].push(scenario_index);
        }

        let mut post_query_regret = 0_u64;
        for partition in &partitions {
            if !partition.is_empty() {
                post_query_regret = post_query_regret.max(minimax_regret(
                    partition,
                    &utilities,
                    &scenario_best,
                    alternatives.len(),
                ));
            }
        }
        if post_query_regret > baseline_regret {
            return Err(PreferenceProbeError::RegretInvariantViolation {
                question_id: question.question_id.clone(),
            });
        }

        results.push(PreferenceQuestionResult {
            question_id: question.question_id.clone(),
            left_candidate_id: question.left_candidate_id.clone(),
            right_candidate_id: question.right_candidate_id.clone(),
            query_cost_units: question.query_cost_units,
            query_duration_units: question.query_duration_units,
            baseline_minimax_regret_units: baseline_regret,
            post_answer_worst_case_regret_units: post_query_regret,
            worst_case_regret_reduction_units: baseline_regret - post_query_regret,
            answer_counts: PreferenceAnswerCounts::ExactModelPartition {
                left_preferred: partitions[0].len(),
                right_preferred: partitions[1].len(),
                indifferent: partitions[2].len(),
            },
        });
    }

    let mut frontier_question_ids = Vec::new();
    for question in &results {
        let dominated = results
            .iter()
            .any(|other| other.question_id != question.question_id && question_dominates(other, question));
        if !dominated {
            frontier_question_ids.push(question.question_id.clone());
        }
    }
    frontier_question_ids.sort();

    Ok(ResidentPreferenceProbeReport {
        baseline_minimax_regret_units: baseline_regret,
        questions: results,
        frontier_question_ids,
        criterion_ids: request.criterion_ids.clone(),
        query_cost_unit: request.query_cost_unit.clone(),
        query_duration_unit: request.query_duration_unit.clone(),
        claim_ceiling: "An exact question ranking conditional on the caller-supplied, finite, deterministic preference-model set, upstream-normalized criterion scores and perfectly scenario-consistent answers. It does not establish a resident's true preferences, model-set adequacy, accessibility, real-room safety, or a universally best alternative. Hard-feasibility receipts are syntax/binding checked but not authenticated here.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alternative(id: &str, scores: &[u16]) -> PreferenceAlternative {
        let revision = format!("{:064x}", id.bytes().fold(1_u64, |value, byte| value + u64::from(byte)));
        PreferenceAlternative {
            candidate_id: id.to_owned(),
            revision_sha256: revision.clone(),
            verifier_receipt_sha256: "b".repeat(64),
            verified_revision_sha256: revision,
            hard_feasibility: VerificationStatus::Verified,
            criterion_scores_bps: scores.to_vec(),
        }
    }

    fn question(id: &str, left: &str, right: &str, cost: u32, duration: u32) -> PairwisePreferenceQuestion {
        PairwisePreferenceQuestion {
            question_id: id.to_owned(),
            left_candidate_id: left.to_owned(),
            right_candidate_id: right.to_owned(),
            query_cost_units: cost,
            query_duration_units: duration,
        }
    }

    fn request() -> ResidentPreferenceProbeRequest {
        ResidentPreferenceProbeRequest {
            criterion_ids: vec!["affordability".to_owned(), "task_support".to_owned()],
            query_cost_unit: "effort_points".to_owned(),
            query_duration_unit: "seconds".to_owned(),
            alternatives: vec![
                alternative("low-cost", &[10_000, 0]),
                alternative("balanced", &[6_000, 6_000]),
                alternative("high-support", &[0, 10_000]),
            ],
            scenarios: vec![
                PreferenceScenario { scenario_id: "cost-first".to_owned(), weights_bps: vec![10_000, 0] },
                PreferenceScenario { scenario_id: "support-first".to_owned(), weights_bps: vec![0, 10_000] },
            ],
            questions: vec![
                question("ask-extremes", "low-cost", "high-support", 2, 8),
                question("ask-balanced-cost", "low-cost", "balanced", 1, 6),
                question("ask-balanced-support", "balanced", "high-support", 1, 6),
            ],
        }
    }

    fn direct_utility(
        alternative: &PreferenceAlternative,
        scenario: &PreferenceScenario,
    ) -> u64 {
        alternative
            .criterion_scores_bps
            .iter()
            .zip(&scenario.weights_bps)
            .map(|(score, weight)| u64::from(*score) * u64::from(*weight))
            .sum()
    }

    /// Small-instance reference: deliberately recomputes utilities for every
    /// alternative/scenario pair and does not call the optimized implementation.
    fn direct_minimax_regret(
        alternatives: &[PreferenceAlternative],
        scenarios: &[PreferenceScenario],
    ) -> u64 {
        assert!(!alternatives.is_empty() && !scenarios.is_empty());
        let best_by_scenario = scenarios
            .iter()
            .map(|scenario| {
                alternatives
                    .iter()
                    .map(|alternative| direct_utility(alternative, scenario))
                    .max()
                    .expect("reference fixture contains an alternative")
            })
            .collect::<Vec<_>>();
        alternatives
            .iter()
            .map(|alternative| {
                scenarios
                    .iter()
                    .zip(&best_by_scenario)
                    .map(|(scenario, best)| best - direct_utility(alternative, scenario))
                    .max()
                    .expect("reference fixture contains scenarios")
            })
            .min()
            .expect("reference fixture contains alternatives")
    }

    fn direct_question_partition(
        input: &ResidentPreferenceProbeRequest,
        question: &PairwisePreferenceQuestion,
    ) -> ([usize; 3], u64) {
        let left = input
            .alternatives
            .iter()
            .find(|alternative| alternative.candidate_id == question.left_candidate_id)
            .expect("question left candidate exists");
        let right = input
            .alternatives
            .iter()
            .find(|alternative| alternative.candidate_id == question.right_candidate_id)
            .expect("question right candidate exists");
        let mut partitions: [Vec<PreferenceScenario>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        for scenario in &input.scenarios {
            let outcome = match direct_utility(left, scenario).cmp(&direct_utility(right, scenario)) {
                std::cmp::Ordering::Greater => 0,
                std::cmp::Ordering::Less => 1,
                std::cmp::Ordering::Equal => 2,
            };
            partitions[outcome].push(scenario.clone());
        }
        let counts = [
            partitions[0].len(),
            partitions[1].len(),
            partitions[2].len(),
        ];
        let post_regret = partitions
            .iter()
            .filter(|partition| !partition.is_empty())
            .map(|partition| direct_minimax_regret(&input.alternatives, partition))
            .max()
            .expect("all scenarios belong to an answer partition");
        (counts, post_regret)
    }

    #[test]
    fn extreme_pair_question_reduces_known_minimax_regret_to_zero() {
        let report = compute_resident_preference_probe(&request()).expect("valid probe");
        assert_eq!(report.baseline_minimax_regret_units, 40_000_000);
        let question = report.questions.iter().find(|q| q.question_id == "ask-extremes").unwrap();
        assert_eq!(question.post_answer_worst_case_regret_units, 0);
        assert_eq!(question.worst_case_regret_reduction_units, 40_000_000);
        assert_eq!(
            question.answer_counts,
            PreferenceAnswerCounts::ExactModelPartition { left_preferred: 1, right_preferred: 1, indifferent: 0 }
        );
    }

    #[test]
    fn scenario_partition_never_increases_worst_case_regret() {
        let report = compute_resident_preference_probe(&request()).expect("valid probe");
        for question in &report.questions {
            assert!(question.post_answer_worst_case_regret_units <= report.baseline_minimax_regret_units);
            assert_eq!(
                question.worst_case_regret_reduction_units,
                report.baseline_minimax_regret_units - question.post_answer_worst_case_regret_units
            );
        }
    }

    #[test]
    fn one_scenario_has_no_information_value_under_the_model() {
        let mut input = request();
        input.scenarios.truncate(1);
        let report = compute_resident_preference_probe(&input).expect("valid single model");
        assert_eq!(report.baseline_minimax_regret_units, 0);
        assert!(report.questions.iter().all(|q| q.worst_case_regret_reduction_units == 0));
    }

    #[test]
    fn request_order_does_not_change_report() {
        let input = request();
        let first = compute_resident_preference_probe(&input).expect("valid probe");
        let mut permuted = input;
        permuted.alternatives.reverse();
        permuted.scenarios.reverse();
        permuted.questions.reverse();
        let second = compute_resident_preference_probe(&permuted).expect("permuted probe");
        assert_eq!(first, second);
    }

    #[test]
    fn generated_question_frontiers_match_an_independent_pairwise_oracle() {
        for seed in 0..40_u32 {
            let mut input = request();
            input.alternatives = (0..6)
                .map(|index| {
                    let first = ((seed * 97 + index * 1_337 + 71) % 10_001) as u16;
                    let second = ((seed * 211 + index * 811 + 19) % 10_001) as u16;
                    alternative(&format!("alternative-{index}"), &[first, second])
                })
                .collect();
            input.scenarios = vec![
                PreferenceScenario {
                    scenario_id: "affordability".to_owned(),
                    weights_bps: vec![10_000, 0],
                },
                PreferenceScenario {
                    scenario_id: "task-support".to_owned(),
                    weights_bps: vec![0, 10_000],
                },
                PreferenceScenario {
                    scenario_id: "balanced".to_owned(),
                    weights_bps: vec![5_000, 5_000],
                },
            ];
            input.questions.clear();
            for left in 0..input.alternatives.len() {
                for right in (left + 1)..input.alternatives.len() {
                    input.questions.push(question(
                        &format!("q-{left}-{right}"),
                        &input.alternatives[left].candidate_id,
                        &input.alternatives[right].candidate_id,
                        (seed + (left * 7 + right * 3) as u32) % 9,
                        (seed * 3 + (left * 5 + right * 11) as u32) % 13,
                    ));
                }
            }
            let report = compute_resident_preference_probe(&input).expect("valid generated request");
            let expected_frontier = report
                .questions
                .iter()
                .filter(|candidate| {
                    !report.questions.iter().any(|other| {
                        let no_worse = other.worst_case_regret_reduction_units
                            >= candidate.worst_case_regret_reduction_units
                            && other.query_cost_units <= candidate.query_cost_units
                            && other.query_duration_units <= candidate.query_duration_units;
                        let strictly_better = other.worst_case_regret_reduction_units
                            > candidate.worst_case_regret_reduction_units
                            || other.query_cost_units < candidate.query_cost_units
                            || other.query_duration_units < candidate.query_duration_units;
                        other.question_id != candidate.question_id && no_worse && strictly_better
                    })
                })
                .map(|question| question.question_id.clone())
                .collect::<Vec<_>>();
            assert_eq!(report.frontier_question_ids, expected_frontier);
        }
    }

    #[test]
    fn expanding_the_admissible_model_set_cannot_lower_baseline_minimax_regret() {
        let base = ResidentPreferenceProbeRequest {
            criterion_ids: vec![
                "affordability".to_owned(),
                "task_support".to_owned(),
                "privacy".to_owned(),
            ],
            query_cost_unit: "effort_points".to_owned(),
            query_duration_unit: "seconds".to_owned(),
            alternatives: vec![
                alternative("a-cost", &[10_000, 0, 0]),
                alternative("b-support", &[0, 10_000, 0]),
                alternative("c-privacy", &[0, 0, 10_000]),
                alternative("d-balanced", &[5_000, 5_000, 0]),
            ],
            scenarios: vec![
                PreferenceScenario {
                    scenario_id: "cost".to_owned(),
                    weights_bps: vec![10_000, 0, 0],
                },
                PreferenceScenario {
                    scenario_id: "support".to_owned(),
                    weights_bps: vec![0, 10_000, 0],
                },
            ],
            questions: vec![question("ask-a-vs-b", "a-cost", "b-support", 1, 1)],
        };
        let base_report = compute_resident_preference_probe(&base).expect("valid base set");

        let mut expanded = base.clone();
        expanded.scenarios.push(PreferenceScenario {
            scenario_id: "privacy".to_owned(),
            weights_bps: vec![0, 0, 10_000],
        });
        let expanded_report =
            compute_resident_preference_probe(&expanded).expect("valid expanded model set");

        assert_eq!(base_report.baseline_minimax_regret_units, 50_000_000);
        assert_eq!(expanded_report.baseline_minimax_regret_units, 100_000_000);
        assert!(
            expanded_report.baseline_minimax_regret_units
                >= base_report.baseline_minimax_regret_units,
            "adding an admissible preference model cannot reduce worst-case regret"
        );
    }

    #[test]
    fn blocked_indeterminate_and_stale_receipts_fail_closed() {
        let mut blocked = request();
        blocked.alternatives[0].hard_feasibility = VerificationStatus::Blocked;
        assert!(matches!(
            compute_resident_preference_probe(&blocked),
            Err(PreferenceProbeError::AlternativeNotVerified { status: VerificationStatus::Blocked, .. })
        ));

        let mut unknown = request();
        unknown.alternatives[0].hard_feasibility = VerificationStatus::Indeterminate;
        assert!(matches!(
            compute_resident_preference_probe(&unknown),
            Err(PreferenceProbeError::AlternativeNotVerified { status: VerificationStatus::Indeterminate, .. })
        ));

        let mut stale = request();
        stale.alternatives[0].verified_revision_sha256 = "c".repeat(64);
        assert!(matches!(
            compute_resident_preference_probe(&stale),
            Err(PreferenceProbeError::StaleReceiptBinding { .. })
        ));
    }

    #[test]
    fn invalid_weights_dimensions_scores_and_pairs_are_rejected() {
        let mut weights = request();
        weights.scenarios[0].weights_bps[0] = 9_999;
        assert!(matches!(
            compute_resident_preference_probe(&weights),
            Err(PreferenceProbeError::ScenarioWeightSum { .. })
        ));

        let mut dimension = request();
        dimension.alternatives[0].criterion_scores_bps.pop();
        assert!(matches!(
            compute_resident_preference_probe(&dimension),
            Err(PreferenceProbeError::CriterionVectorLength { .. })
        ));

        let mut score = request();
        score.alternatives[0].criterion_scores_bps[0] = 10_001;
        assert!(matches!(
            compute_resident_preference_probe(&score),
            Err(PreferenceProbeError::ScoreOutOfRange { .. })
        ));

        let mut duplicate_pair = request();
        duplicate_pair.questions.push(question("reverse-same-pair", "high-support", "low-cost", 2, 8));
        assert!(matches!(
            compute_resident_preference_probe(&duplicate_pair),
            Err(PreferenceProbeError::DuplicateQuestionPair { .. })
        ));
    }

    #[test]
    fn question_frontier_preserves_tradeoffs_without_weighted_scalarization() {
        let report = compute_resident_preference_probe(&request()).expect("valid probe");
        // The two lower-effort questions split the two models just as effectively
        // as the extremes question and should dominate that more costly question.
        assert!(!report.frontier_question_ids.contains(&"ask-extremes".to_owned()));
        assert!(report.frontier_question_ids.contains(&"ask-balanced-cost".to_owned()));
        assert!(report.frontier_question_ids.contains(&"ask-balanced-support".to_owned()));
        let extreme = report.questions.iter().find(|q| q.question_id == "ask-extremes").unwrap();
        assert_eq!(extreme.worst_case_regret_reduction_units, 40_000_000);
        assert_eq!(extreme.query_cost_units, 2);
        assert_eq!(extreme.query_duration_units, 8);
    }

    #[test]
    fn generated_small_instances_match_input_permutation_and_regret_invariant() {
        for seed in 0..80_u16 {
            let mut input = request();
            input.alternatives = (0..5)
                .map(|candidate_index| {
                    let scores = (0..2)
                        .map(|criterion_index| {
                            (u32::from(seed) * 97
                                + u32::from(candidate_index) * 1_339
                                + u32::from(criterion_index) * 2_717)
                                % 10_001
                        })
                        .map(|value| value as u16)
                        .collect::<Vec<_>>();
                    alternative(&format!("c-{candidate_index}"), &scores)
                })
                .collect();
            input.scenarios = (0..6)
                .map(|scenario_index| {
                    let first = (u32::from(seed) * 113 + u32::from(scenario_index) * 1_907) % 10_001;
                    PreferenceScenario {
                        scenario_id: format!("s-{scenario_index}"),
                        weights_bps: vec![first as u16, 10_000 - first as u16],
                    }
                })
                .collect();
            input.questions = (0..5)
                .map(|index| {
                    let left = index;
                    let right = (index + 1) % 5;
                    question(
                        &format!("q-{index}"),
                        &format!("c-{left}"),
                        &format!("c-{right}"),
                        (index % 3) as u32,
                        (index % 4 + 1) as u32,
                    )
                })
                .collect();

            let report = compute_resident_preference_probe(&input).expect("generated model valid");
            let direct_baseline = direct_minimax_regret(&input.alternatives, &input.scenarios);
            assert_eq!(report.baseline_minimax_regret_units, direct_baseline);
            for question in &input.questions {
                let (counts, direct_post) = direct_question_partition(&input, question);
                let actual = report
                    .questions
                    .iter()
                    .find(|result| result.question_id == question.question_id)
                    .expect("question result exists");
                assert_eq!(actual.baseline_minimax_regret_units, direct_baseline);
                assert_eq!(actual.post_answer_worst_case_regret_units, direct_post);
                assert_eq!(
                    actual.worst_case_regret_reduction_units,
                    direct_baseline - direct_post
                );
                assert_eq!(
                    actual.answer_counts,
                    PreferenceAnswerCounts::ExactModelPartition {
                        left_preferred: counts[0],
                        right_preferred: counts[1],
                        indifferent: counts[2],
                    }
                );
                assert!(direct_post <= direct_baseline);
            }
            input.alternatives.reverse();
            input.scenarios.reverse();
            input.questions.reverse();
            assert_eq!(
                report,
                compute_resident_preference_probe(&input).expect("permuted model valid")
            );
        }
    }
}
