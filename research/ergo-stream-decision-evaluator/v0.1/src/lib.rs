use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use ergo_stream_microcosm::decision_disambiguation::{
    DecisionDisambiguationProblemV0, DecisionDisambiguationResultV0,
    select_decision_sufficient_probe_v0,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroundTruthHypothesisV0 {
    pub witness_key: u64,
    pub decision_class_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CandidateWitnessBindingV0 {
    pub candidate_hypothesis_id: u32,
    pub witness_key: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GroundTruthProbeOutcomeSupportV0 {
    pub probe_id: u32,
    pub witness_key: u64,
    pub possible_outcome_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DecisionDisambiguationGroundTruthV0 {
    pub admissible_hypotheses: Vec<GroundTruthHypothesisV0>,
    pub probe_outcome_supports: Vec<GroundTruthProbeOutcomeSupportV0>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionDisambiguationEvaluationV0 {
    pub candidate_hypothesis_count: usize,
    pub admissible_hypothesis_count: usize,
    pub candidate_policy_result_matches_problem: bool,
    pub correctly_bound_hypotheses: usize,
    pub false_excluded_hypotheses: usize,
    pub false_included_hypotheses: usize,
    pub decision_pairs_compared: u128,
    pub false_decision_equivalence_pairs: u128,
    pub false_decision_split_pairs: u128,
    pub missed_probe_outcome_supports: usize,
    pub spurious_probe_outcome_supports: usize,
    pub actual_baseline_conflicting_pairs: u128,
    pub actual_worst_case_conflicting_pairs: Option<u128>,
    pub claimed_decision_sufficient: bool,
    pub decision_sufficiency_receipt_matches_truth: Option<bool>,
    pub decision_sufficiency_is_valid: bool,
    pub false_decision_sufficiency_claim: bool,
    pub claimed_probe_id: Option<u32>,
    pub claimed_probe_is_decision_reducing: Option<bool>,
    pub claimed_probe_overstates_improvement: bool,
    pub unnecessary_abstention: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionEvaluatorErrorV0 {
    InvalidCandidateProblem,
    EmptyGroundTruthHypotheses,
    DuplicateGroundTruthWitness,
    DuplicateCandidateBinding,
    MissingCandidateBinding,
    BindingForUnknownCandidateHypothesis,
    BindingToUnknownWitness,
    DuplicateTruthProbeOutcome,
    MissingTruthProbeOutcomeSupport,
    ArithmeticOverflow,
}

impl fmt::Display for DecisionEvaluatorErrorV0 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidCandidateProblem => "candidate problem is invalid",
            Self::EmptyGroundTruthHypotheses => "ground truth has no admissible witnesses",
            Self::DuplicateGroundTruthWitness => "ground-truth witness IDs are duplicated",
            Self::DuplicateCandidateBinding => "candidate witness bindings are duplicated",
            Self::MissingCandidateBinding => "a candidate hypothesis has no evaluator binding",
            Self::BindingForUnknownCandidateHypothesis => {
                "binding references an unknown candidate hypothesis"
            }
            Self::BindingToUnknownWitness => "binding references a witness outside ground truth",
            Self::DuplicateTruthProbeOutcome => "ground-truth probe outcome row is duplicated",
            Self::MissingTruthProbeOutcomeSupport => {
                "ground truth lacks support for a declared probe/witness pair"
            }
            Self::ArithmeticOverflow => "evaluation pair accounting overflowed",
        };
        formatter.write_str(message)
    }
}
impl std::error::Error for DecisionEvaluatorErrorV0 {}

#[derive(Debug, Clone)]
struct ValidatedTruthV0 {
    decision_by_witness: BTreeMap<u64, u32>,
    outcomes_by_probe_witness: BTreeMap<(u32, u64), BTreeSet<u32>>,
}

fn validate_truth(
    problem: &DecisionDisambiguationProblemV0,
    bindings: &[CandidateWitnessBindingV0],
    truth: &DecisionDisambiguationGroundTruthV0,
) -> Result<(BTreeMap<u32, Option<u64>>, ValidatedTruthV0), DecisionEvaluatorErrorV0> {
    if truth.admissible_hypotheses.is_empty() {
        return Err(DecisionEvaluatorErrorV0::EmptyGroundTruthHypotheses);
    }

    let candidate_ids = problem
        .hypotheses
        .iter()
        .map(|hypothesis| hypothesis.hypothesis_id)
        .collect::<BTreeSet<_>>();
    let mut binding_by_candidate = BTreeMap::new();
    for binding in bindings {
        if !candidate_ids.contains(&binding.candidate_hypothesis_id) {
            return Err(DecisionEvaluatorErrorV0::BindingForUnknownCandidateHypothesis);
        }
        if binding_by_candidate
            .insert(binding.candidate_hypothesis_id, binding.witness_key)
            .is_some()
        {
            return Err(DecisionEvaluatorErrorV0::DuplicateCandidateBinding);
        }
    }
    if binding_by_candidate.len() != candidate_ids.len() {
        return Err(DecisionEvaluatorErrorV0::MissingCandidateBinding);
    }

    let mut decision_by_witness = BTreeMap::new();
    for hypothesis in &truth.admissible_hypotheses {
        if decision_by_witness
            .insert(hypothesis.witness_key, hypothesis.decision_class_id)
            .is_some()
        {
            return Err(DecisionEvaluatorErrorV0::DuplicateGroundTruthWitness);
        }
    }

    let mut bound_witnesses = BTreeSet::new();
    for witness_key in binding_by_candidate.values().flatten() {
        if !decision_by_witness.contains_key(witness_key) {
            return Err(DecisionEvaluatorErrorV0::BindingToUnknownWitness);
        }
        if !bound_witnesses.insert(*witness_key) {
            return Err(DecisionEvaluatorErrorV0::DuplicateCandidateBinding);
        }
    }

    let mut outcomes_by_probe_witness = BTreeMap::<(u32, u64), BTreeSet<u32>>::new();
    let mut seen_rows = BTreeSet::new();
    for row in &truth.probe_outcome_supports {
        if !decision_by_witness.contains_key(&row.witness_key) {
            return Err(DecisionEvaluatorErrorV0::BindingToUnknownWitness);
        }
        if !seen_rows.insert(*row) {
            return Err(DecisionEvaluatorErrorV0::DuplicateTruthProbeOutcome);
        }
        outcomes_by_probe_witness
            .entry((row.probe_id, row.witness_key))
            .or_default()
            .insert(row.possible_outcome_id);
    }
    for option in &problem.probes {
        for witness_key in decision_by_witness.keys() {
            if !outcomes_by_probe_witness
                .get(&(option.probe_id, *witness_key))
                .is_some_and(|set| !set.is_empty())
            {
                return Err(DecisionEvaluatorErrorV0::MissingTruthProbeOutcomeSupport);
            }
        }
    }

    Ok((
        binding_by_candidate,
        ValidatedTruthV0 {
            decision_by_witness,
            outcomes_by_probe_witness,
        },
    ))
}

fn conflicting_pairs(
    classes: impl IntoIterator<Item = u32>,
) -> Result<u128, DecisionEvaluatorErrorV0> {
    let mut counts = BTreeMap::<u32, u128>::new();
    let mut total = 0_u128;
    for class_id in classes {
        let count = counts.entry(class_id).or_default();
        *count = count
            .checked_add(1)
            .ok_or(DecisionEvaluatorErrorV0::ArithmeticOverflow)?;
        total = total
            .checked_add(1)
            .ok_or(DecisionEvaluatorErrorV0::ArithmeticOverflow)?;
    }
    let all_pairs = total
        .checked_mul(total.saturating_sub(1))
        .ok_or(DecisionEvaluatorErrorV0::ArithmeticOverflow)?
        / 2;
    let same_pairs = counts.values().try_fold(0_u128, |sum, count| {
        let pairs = count
            .checked_mul(count.saturating_sub(1))
            .ok_or(DecisionEvaluatorErrorV0::ArithmeticOverflow)?
            / 2;
        sum.checked_add(pairs)
            .ok_or(DecisionEvaluatorErrorV0::ArithmeticOverflow)
    })?;
    all_pairs
        .checked_sub(same_pairs)
        .ok_or(DecisionEvaluatorErrorV0::ArithmeticOverflow)
}

fn actual_worst_case_conflicts(
    probe_id: u32,
    truth: &ValidatedTruthV0,
) -> Result<u128, DecisionEvaluatorErrorV0> {
    let mut by_outcome = BTreeMap::<u32, Vec<u32>>::new();
    for (witness_key, decision_class) in &truth.decision_by_witness {
        let outcomes = truth
            .outcomes_by_probe_witness
            .get(&(probe_id, *witness_key))
            .ok_or(DecisionEvaluatorErrorV0::MissingTruthProbeOutcomeSupport)?;
        for outcome in outcomes {
            by_outcome
                .entry(*outcome)
                .or_default()
                .push(*decision_class);
        }
    }
    by_outcome
        .values()
        .map(|classes| conflicting_pairs(classes.iter().copied()))
        .try_fold(0_u128, |worst, value| Ok(worst.max(value?)))
}

fn unique_truth_decision_class(truth: &ValidatedTruthV0) -> Option<u32> {
    let mut classes = truth.decision_by_witness.values().copied();
    let first = classes.next()?;
    classes.all(|class_id| class_id == first).then_some(first)
}

/// Score a frozen candidate result against evaluator-held admissible witnesses
/// and true probe-outcome supports. Bindings are created by the independent
/// structural validator and never included in candidate-visible data.
pub fn evaluate_decision_disambiguation_claim_v0(
    problem: &DecisionDisambiguationProblemV0,
    claimed_result: &DecisionDisambiguationResultV0,
    bindings: &[CandidateWitnessBindingV0],
    ground_truth: &DecisionDisambiguationGroundTruthV0,
) -> Result<DecisionDisambiguationEvaluationV0, DecisionEvaluatorErrorV0> {
    let expected_candidate_result = select_decision_sufficient_probe_v0(problem)
        .map_err(|_| DecisionEvaluatorErrorV0::InvalidCandidateProblem)?;
    let candidate_policy_result_matches_problem = expected_candidate_result == *claimed_result;
    let (binding_by_candidate, truth) = validate_truth(problem, bindings, ground_truth)?;

    let truth_keys = truth
        .decision_by_witness
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();
    let candidate_keys = binding_by_candidate
        .values()
        .flatten()
        .copied()
        .collect::<BTreeSet<_>>();
    let correctly_bound_hypotheses = candidate_keys.intersection(&truth_keys).count();
    let false_excluded_hypotheses = truth_keys.difference(&candidate_keys).count();
    let false_included_hypotheses = binding_by_candidate
        .values()
        .filter(|key| key.is_none_or(|key| !truth_keys.contains(&key)))
        .count();

    let candidate_decisions = problem
        .hypotheses
        .iter()
        .map(|hypothesis| (hypothesis.hypothesis_id, hypothesis.decision_class_id))
        .collect::<BTreeMap<_, _>>();
    let valid_bindings = binding_by_candidate
        .iter()
        .filter_map(|(candidate_id, witness_key)| witness_key.map(|key| (*candidate_id, key)))
        .filter(|(_, witness_key)| truth_keys.contains(witness_key))
        .collect::<Vec<_>>();

    let mut decision_pairs_compared = 0_u128;
    let mut false_decision_equivalence_pairs = 0_u128;
    let mut false_decision_split_pairs = 0_u128;
    for (index, (left_id, left_key)) in valid_bindings.iter().enumerate() {
        for (right_id, right_key) in valid_bindings.iter().skip(index + 1) {
            let candidate_same = candidate_decisions[left_id] == candidate_decisions[right_id];
            let truth_same =
                truth.decision_by_witness[left_key] == truth.decision_by_witness[right_key];
            decision_pairs_compared = decision_pairs_compared
                .checked_add(1)
                .ok_or(DecisionEvaluatorErrorV0::ArithmeticOverflow)?;
            match (candidate_same, truth_same) {
                (true, false) => {
                    false_decision_equivalence_pairs = false_decision_equivalence_pairs
                        .checked_add(1)
                        .ok_or(DecisionEvaluatorErrorV0::ArithmeticOverflow)?
                }
                (false, true) => {
                    false_decision_split_pairs = false_decision_split_pairs
                        .checked_add(1)
                        .ok_or(DecisionEvaluatorErrorV0::ArithmeticOverflow)?
                }
                _ => {}
            }
        }
    }

    let mut missed_probe_outcome_supports = 0_usize;
    let mut spurious_probe_outcome_supports = 0_usize;
    for (candidate_id, witness_key) in &valid_bindings {
        for option in &problem.probes {
            let predicted = problem
                .outcome_supports
                .iter()
                .filter(|row| row.probe_id == option.probe_id && row.hypothesis_id == *candidate_id)
                .map(|row| row.possible_outcome_id)
                .collect::<BTreeSet<_>>();
            let actual = truth
                .outcomes_by_probe_witness
                .get(&(option.probe_id, *witness_key))
                .ok_or(DecisionEvaluatorErrorV0::MissingTruthProbeOutcomeSupport)?;
            missed_probe_outcome_supports += actual.difference(&predicted).count();
            spurious_probe_outcome_supports += predicted.difference(actual).count();
        }
    }

    let actual_baseline_conflicting_pairs =
        conflicting_pairs(truth.decision_by_witness.values().copied())?;
    let claimed_decision_sufficient = matches!(
        claimed_result,
        DecisionDisambiguationResultV0::DecisionSufficient { .. }
    );
    let complete_witness_coverage =
        false_excluded_hypotheses == 0 && false_included_hypotheses == 0;
    let decision_sufficiency_receipt_matches_truth = match claimed_result {
        DecisionDisambiguationResultV0::DecisionSufficient {
            decision_class_id,
            live_hypotheses,
        } => Some(
            unique_truth_decision_class(&truth) == Some(*decision_class_id)
                && *live_hypotheses == problem.hypotheses.len(),
        ),
        _ => None,
    };
    let decision_sufficiency_is_valid = claimed_decision_sufficient
        && candidate_policy_result_matches_problem
        && complete_witness_coverage
        && decision_sufficiency_receipt_matches_truth == Some(true)
        && false_decision_equivalence_pairs == 0
        && false_decision_split_pairs == 0;
    let false_decision_sufficiency_claim =
        claimed_decision_sufficient && !decision_sufficiency_is_valid;

    let mut claimed_probe_id = None;
    let mut claimed_probe_is_decision_reducing = None;
    let mut claimed_probe_overstates_improvement = false;
    let mut actual_worst_case_conflicting_pairs = None;
    let mut unnecessary_abstention = false;

    match claimed_result {
        DecisionDisambiguationResultV0::ProbeSelected {
            probe_id,
            worst_case_conflicting_pairs: predicted_worst,
            ..
        } => {
            claimed_probe_id = Some(*probe_id);
            let option = problem
                .probes
                .iter()
                .find(|option| option.probe_id == *probe_id)
                .ok_or(DecisionEvaluatorErrorV0::InvalidCandidateProblem)?;
            if option.cost_units > problem.remaining_cost_units
                || option.latency_ticks > problem.remaining_latency_ticks
            {
                claimed_probe_is_decision_reducing = Some(false);
            } else {
                let actual_worst = actual_worst_case_conflicts(*probe_id, &truth)?;
                actual_worst_case_conflicting_pairs = Some(actual_worst);
                claimed_probe_is_decision_reducing =
                    Some(actual_worst < actual_baseline_conflicting_pairs);
                claimed_probe_overstates_improvement = u128::from(*predicted_worst) < actual_worst;
            }
        }
        DecisionDisambiguationResultV0::Unresolved { .. } => {
            for option in &problem.probes {
                if option.cost_units > problem.remaining_cost_units
                    || option.latency_ticks > problem.remaining_latency_ticks
                {
                    continue;
                }
                let actual_worst = actual_worst_case_conflicts(option.probe_id, &truth)?;
                if actual_worst < actual_baseline_conflicting_pairs {
                    unnecessary_abstention = true;
                    actual_worst_case_conflicting_pairs = Some(actual_worst);
                    break;
                }
            }
        }
        DecisionDisambiguationResultV0::DecisionSufficient { .. } => {}
    }

    Ok(DecisionDisambiguationEvaluationV0 {
        candidate_hypothesis_count: problem.hypotheses.len(),
        admissible_hypothesis_count: truth.decision_by_witness.len(),
        candidate_policy_result_matches_problem,
        correctly_bound_hypotheses,
        false_excluded_hypotheses,
        false_included_hypotheses,
        decision_pairs_compared,
        false_decision_equivalence_pairs,
        false_decision_split_pairs,
        missed_probe_outcome_supports,
        spurious_probe_outcome_supports,
        actual_baseline_conflicting_pairs,
        actual_worst_case_conflicting_pairs,
        claimed_decision_sufficient,
        decision_sufficiency_receipt_matches_truth,
        decision_sufficiency_is_valid,
        false_decision_sufficiency_claim,
        claimed_probe_id,
        claimed_probe_is_decision_reducing,
        claimed_probe_overstates_improvement,
        unnecessary_abstention,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ergo_stream_microcosm::decision_disambiguation::{
        DecisionHypothesisProjectionV0, DecisionProbeOptionV0, DecisionProbeOutcomeSupportV0,
        select_decision_sufficient_probe_v0,
    };

    fn problem() -> DecisionDisambiguationProblemV0 {
        DecisionDisambiguationProblemV0 {
            hypotheses: vec![
                DecisionHypothesisProjectionV0 {
                    hypothesis_id: 1,
                    decision_class_id: 10,
                },
                DecisionHypothesisProjectionV0 {
                    hypothesis_id: 2,
                    decision_class_id: 10,
                },
                DecisionHypothesisProjectionV0 {
                    hypothesis_id: 3,
                    decision_class_id: 20,
                },
                DecisionHypothesisProjectionV0 {
                    hypothesis_id: 4,
                    decision_class_id: 20,
                },
            ],
            probes: vec![
                DecisionProbeOptionV0 {
                    probe_id: 1,
                    cost_units: 1,
                    latency_ticks: 1,
                },
                DecisionProbeOptionV0 {
                    probe_id: 2,
                    cost_units: 5,
                    latency_ticks: 2,
                },
            ],
            outcome_supports: vec![
                DecisionProbeOutcomeSupportV0 {
                    probe_id: 1,
                    hypothesis_id: 1,
                    possible_outcome_id: 100,
                },
                DecisionProbeOutcomeSupportV0 {
                    probe_id: 1,
                    hypothesis_id: 2,
                    possible_outcome_id: 200,
                },
                DecisionProbeOutcomeSupportV0 {
                    probe_id: 1,
                    hypothesis_id: 3,
                    possible_outcome_id: 100,
                },
                DecisionProbeOutcomeSupportV0 {
                    probe_id: 1,
                    hypothesis_id: 4,
                    possible_outcome_id: 200,
                },
                DecisionProbeOutcomeSupportV0 {
                    probe_id: 2,
                    hypothesis_id: 1,
                    possible_outcome_id: 300,
                },
                DecisionProbeOutcomeSupportV0 {
                    probe_id: 2,
                    hypothesis_id: 2,
                    possible_outcome_id: 300,
                },
                DecisionProbeOutcomeSupportV0 {
                    probe_id: 2,
                    hypothesis_id: 3,
                    possible_outcome_id: 400,
                },
                DecisionProbeOutcomeSupportV0 {
                    probe_id: 2,
                    hypothesis_id: 4,
                    possible_outcome_id: 400,
                },
            ],
            remaining_cost_units: 10,
            remaining_latency_ticks: 10,
        }
    }

    fn bindings() -> Vec<CandidateWitnessBindingV0> {
        vec![
            CandidateWitnessBindingV0 {
                candidate_hypothesis_id: 1,
                witness_key: Some(101),
            },
            CandidateWitnessBindingV0 {
                candidate_hypothesis_id: 2,
                witness_key: Some(102),
            },
            CandidateWitnessBindingV0 {
                candidate_hypothesis_id: 3,
                witness_key: Some(103),
            },
            CandidateWitnessBindingV0 {
                candidate_hypothesis_id: 4,
                witness_key: Some(104),
            },
        ]
    }

    fn truth() -> DecisionDisambiguationGroundTruthV0 {
        DecisionDisambiguationGroundTruthV0 {
            admissible_hypotheses: vec![
                GroundTruthHypothesisV0 {
                    witness_key: 101,
                    decision_class_id: 7,
                },
                GroundTruthHypothesisV0 {
                    witness_key: 102,
                    decision_class_id: 7,
                },
                GroundTruthHypothesisV0 {
                    witness_key: 103,
                    decision_class_id: 9,
                },
                GroundTruthHypothesisV0 {
                    witness_key: 104,
                    decision_class_id: 9,
                },
            ],
            probe_outcome_supports: vec![
                GroundTruthProbeOutcomeSupportV0 {
                    probe_id: 1,
                    witness_key: 101,
                    possible_outcome_id: 100,
                },
                GroundTruthProbeOutcomeSupportV0 {
                    probe_id: 1,
                    witness_key: 102,
                    possible_outcome_id: 200,
                },
                GroundTruthProbeOutcomeSupportV0 {
                    probe_id: 1,
                    witness_key: 103,
                    possible_outcome_id: 100,
                },
                GroundTruthProbeOutcomeSupportV0 {
                    probe_id: 1,
                    witness_key: 104,
                    possible_outcome_id: 200,
                },
                GroundTruthProbeOutcomeSupportV0 {
                    probe_id: 2,
                    witness_key: 101,
                    possible_outcome_id: 300,
                },
                GroundTruthProbeOutcomeSupportV0 {
                    probe_id: 2,
                    witness_key: 102,
                    possible_outcome_id: 300,
                },
                GroundTruthProbeOutcomeSupportV0 {
                    probe_id: 2,
                    witness_key: 103,
                    possible_outcome_id: 400,
                },
                GroundTruthProbeOutcomeSupportV0 {
                    probe_id: 2,
                    witness_key: 104,
                    possible_outcome_id: 400,
                },
            ],
        }
    }

    #[test]
    fn decision_sufficiency_receipt_must_match_truth_class_and_live_count() {
        let candidate_problem = DecisionDisambiguationProblemV0 {
            hypotheses: vec![
                DecisionHypothesisProjectionV0 {
                    hypothesis_id: 1,
                    decision_class_id: 10,
                },
                DecisionHypothesisProjectionV0 {
                    hypothesis_id: 2,
                    decision_class_id: 10,
                },
            ],
            probes: Vec::new(),
            outcome_supports: Vec::new(),
            remaining_cost_units: 0,
            remaining_latency_ticks: 0,
        };
        let bindings = vec![
            CandidateWitnessBindingV0 {
                candidate_hypothesis_id: 1,
                witness_key: Some(101),
            },
            CandidateWitnessBindingV0 {
                candidate_hypothesis_id: 2,
                witness_key: Some(102),
            },
        ];
        let ground_truth = DecisionDisambiguationGroundTruthV0 {
            admissible_hypotheses: vec![
                GroundTruthHypothesisV0 {
                    witness_key: 101,
                    decision_class_id: 7,
                },
                GroundTruthHypothesisV0 {
                    witness_key: 102,
                    decision_class_id: 7,
                },
            ],
            probe_outcome_supports: Vec::new(),
        };

        let wrong_class = DecisionDisambiguationResultV0::DecisionSufficient {
            decision_class_id: 99,
            live_hypotheses: 2,
        };
        let report = evaluate_decision_disambiguation_claim_v0(
            &candidate_problem,
            &wrong_class,
            &bindings,
            &ground_truth,
        )
        .unwrap();
        assert_eq!(
            report.decision_sufficiency_receipt_matches_truth,
            Some(false)
        );
        assert!(report.false_decision_sufficiency_claim);

        let wrong_count = DecisionDisambiguationResultV0::DecisionSufficient {
            decision_class_id: 7,
            live_hypotheses: 3,
        };
        let report = evaluate_decision_disambiguation_claim_v0(
            &candidate_problem,
            &wrong_count,
            &bindings,
            &ground_truth,
        )
        .unwrap();
        assert_eq!(
            report.decision_sufficiency_receipt_matches_truth,
            Some(false)
        );
        assert!(report.false_decision_sufficiency_claim);

        let correct = DecisionDisambiguationResultV0::DecisionSufficient {
            decision_class_id: 7,
            live_hypotheses: 2,
        };
        let report = evaluate_decision_disambiguation_claim_v0(
            &candidate_problem,
            &correct,
            &bindings,
            &ground_truth,
        )
        .unwrap();
        assert_eq!(
            report.decision_sufficiency_receipt_matches_truth,
            Some(true)
        );
        assert!(report.decision_sufficiency_is_valid);
    }

    #[test]
    fn evaluates_complete_witness_set_and_real_probe_partition() {
        let candidate_problem = problem();
        let selected = select_decision_sufficient_probe_v0(&candidate_problem).unwrap();
        let report = evaluate_decision_disambiguation_claim_v0(
            &candidate_problem,
            &selected,
            &bindings(),
            &truth(),
        )
        .unwrap();

        assert_eq!(report.correctly_bound_hypotheses, 4);
        assert_eq!(report.false_excluded_hypotheses, 0);
        assert_eq!(report.false_included_hypotheses, 0);
        assert_eq!(report.false_decision_equivalence_pairs, 0);
        assert_eq!(report.false_decision_split_pairs, 0);
        assert_eq!(report.actual_baseline_conflicting_pairs, 4);
        assert_eq!(report.claimed_probe_id, Some(2));
        assert_eq!(report.claimed_probe_is_decision_reducing, Some(true));
        assert!(!report.claimed_probe_overstates_improvement);
    }

    #[test]
    fn rejects_sufficiency_claim_that_omits_a_valid_alternative() {
        let candidate_problem = DecisionDisambiguationProblemV0 {
            hypotheses: vec![
                DecisionHypothesisProjectionV0 {
                    hypothesis_id: 1,
                    decision_class_id: 10,
                },
                DecisionHypothesisProjectionV0 {
                    hypothesis_id: 2,
                    decision_class_id: 10,
                },
            ],
            probes: Vec::new(),
            outcome_supports: Vec::new(),
            remaining_cost_units: 0,
            remaining_latency_ticks: 0,
        };
        let selected = select_decision_sufficient_probe_v0(&candidate_problem).unwrap();
        let bindings = vec![
            CandidateWitnessBindingV0 {
                candidate_hypothesis_id: 1,
                witness_key: Some(101),
            },
            CandidateWitnessBindingV0 {
                candidate_hypothesis_id: 2,
                witness_key: Some(102),
            },
        ];
        let ground_truth = DecisionDisambiguationGroundTruthV0 {
            admissible_hypotheses: vec![
                GroundTruthHypothesisV0 {
                    witness_key: 101,
                    decision_class_id: 7,
                },
                GroundTruthHypothesisV0 {
                    witness_key: 102,
                    decision_class_id: 7,
                },
                GroundTruthHypothesisV0 {
                    witness_key: 103,
                    decision_class_id: 9,
                },
            ],
            probe_outcome_supports: Vec::new(),
        };
        let report = evaluate_decision_disambiguation_claim_v0(
            &candidate_problem,
            &selected,
            &bindings,
            &ground_truth,
        )
        .unwrap();

        assert_eq!(report.false_excluded_hypotheses, 1);
        assert!(report.false_decision_sufficiency_claim);
        assert!(!report.decision_sufficiency_is_valid);
    }

    #[test]
    fn scores_false_merges_and_false_splits_separately() {
        let mut candidate_problem = problem();
        candidate_problem.hypotheses[2].decision_class_id = 10;
        let selected = select_decision_sufficient_probe_v0(&candidate_problem).unwrap();
        let report = evaluate_decision_disambiguation_claim_v0(
            &candidate_problem,
            &selected,
            &bindings(),
            &truth(),
        )
        .unwrap();
        assert!(report.false_decision_equivalence_pairs > 0);

        candidate_problem.hypotheses[1].decision_class_id = 99;
        let selected = select_decision_sufficient_probe_v0(&candidate_problem).unwrap();
        let report = evaluate_decision_disambiguation_claim_v0(
            &candidate_problem,
            &selected,
            &bindings(),
            &truth(),
        )
        .unwrap();
        assert!(report.false_decision_split_pairs > 0);
    }

    #[test]
    fn rejects_probe_that_candidate_predicts_as_useful_but_truth_does_not() {
        let candidate_problem = DecisionDisambiguationProblemV0 {
            hypotheses: vec![
                DecisionHypothesisProjectionV0 {
                    hypothesis_id: 1,
                    decision_class_id: 10,
                },
                DecisionHypothesisProjectionV0 {
                    hypothesis_id: 2,
                    decision_class_id: 20,
                },
            ],
            probes: vec![DecisionProbeOptionV0 {
                probe_id: 8,
                cost_units: 1,
                latency_ticks: 1,
            }],
            outcome_supports: vec![
                DecisionProbeOutcomeSupportV0 {
                    probe_id: 8,
                    hypothesis_id: 1,
                    possible_outcome_id: 80,
                },
                DecisionProbeOutcomeSupportV0 {
                    probe_id: 8,
                    hypothesis_id: 2,
                    possible_outcome_id: 81,
                },
            ],
            remaining_cost_units: 2,
            remaining_latency_ticks: 2,
        };
        let selected = select_decision_sufficient_probe_v0(&candidate_problem).unwrap();
        let bindings = vec![
            CandidateWitnessBindingV0 {
                candidate_hypothesis_id: 1,
                witness_key: Some(201),
            },
            CandidateWitnessBindingV0 {
                candidate_hypothesis_id: 2,
                witness_key: Some(202),
            },
        ];
        let ground_truth = DecisionDisambiguationGroundTruthV0 {
            admissible_hypotheses: vec![
                GroundTruthHypothesisV0 {
                    witness_key: 201,
                    decision_class_id: 1,
                },
                GroundTruthHypothesisV0 {
                    witness_key: 202,
                    decision_class_id: 2,
                },
            ],
            probe_outcome_supports: vec![
                GroundTruthProbeOutcomeSupportV0 {
                    probe_id: 8,
                    witness_key: 201,
                    possible_outcome_id: 800,
                },
                GroundTruthProbeOutcomeSupportV0 {
                    probe_id: 8,
                    witness_key: 202,
                    possible_outcome_id: 800,
                },
            ],
        };
        let report = evaluate_decision_disambiguation_claim_v0(
            &candidate_problem,
            &selected,
            &bindings,
            &ground_truth,
        )
        .unwrap();

        assert_eq!(report.actual_baseline_conflicting_pairs, 1);
        assert_eq!(report.actual_worst_case_conflicting_pairs, Some(1));
        assert_eq!(report.claimed_probe_is_decision_reducing, Some(false));
    }

    #[test]
    fn detects_unnecessary_abstention_when_truth_contains_a_useful_probe() {
        let candidate_problem = problem();
        let unresolved = DecisionDisambiguationResultV0::Unresolved {
            reason: ergo_stream_microcosm::decision_disambiguation::DecisionDisambiguationUnresolvedReasonV0::NoDecisionDistinguishingProbe,
            baseline_conflicting_pairs: 4,
            declared_probe_count: 2,
            budget_feasible_probe_count: 2,
        };
        let report = evaluate_decision_disambiguation_claim_v0(
            &candidate_problem,
            &unresolved,
            &bindings(),
            &truth(),
        )
        .unwrap();
        assert!(report.unnecessary_abstention);
    }

    #[test]
    fn reports_missing_and_spurious_outcome_supports_independently() {
        let mut candidate_problem = problem();
        candidate_problem
            .outcome_supports
            .push(DecisionProbeOutcomeSupportV0 {
                probe_id: 2,
                hypothesis_id: 1,
                possible_outcome_id: 999,
            });
        let selected = select_decision_sufficient_probe_v0(&candidate_problem).unwrap();
        let report = evaluate_decision_disambiguation_claim_v0(
            &candidate_problem,
            &selected,
            &bindings(),
            &truth(),
        )
        .unwrap();
        assert!(report.spurious_probe_outcome_supports > 0);
        assert_eq!(report.missed_probe_outcome_supports, 0);
    }
}
