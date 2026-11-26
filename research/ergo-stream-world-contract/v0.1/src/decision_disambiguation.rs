//! Decision-sufficient active disambiguation for finite hypothesis sets.
//!
//! This bounded reference policy distinguishes explanations only when doing so
//! can change a declared downstream decision. It uses candidate-owned,
//! set-valued outcome predictions and worst-case decision disagreement; it
//! does not require uncalibrated probabilities or treat entropy as utility.
//!
//! The evaluator must independently score the candidate's hypothesis set and
//! predicted outcome supports against hidden world truth. This module is a
//! candidate-side policy primitive, not an evaluator oracle or an action
//! execution service.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const MAX_DECISION_PROBE_HYPOTHESES_V0: usize = 4_096;
pub const MAX_DECISION_PROBE_OPTIONS_V0: usize = 256;
pub const MAX_DECISION_PROBE_SUPPORTS_V0: usize = 1_048_576;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecisionHypothesisProjectionV0 {
    pub hypothesis_id: u32,
    /// Candidate-predicted decision-equivalence class under the frozen
    /// domain-specific utility/tolerance contract.
    pub decision_class_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecisionProbeOptionV0 {
    pub probe_id: u32,
    pub cost_units: u64,
    pub latency_ticks: u64,
}

/// A candidate-predicted possible outcome for a probe under one hypothesis.
/// Multiple rows for a (probe, hypothesis) pair express set-valued outcome
/// uncertainty. These are not evaluator labels or calibrated probabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DecisionProbeOutcomeSupportV0 {
    pub probe_id: u32,
    pub hypothesis_id: u32,
    pub possible_outcome_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DecisionDisambiguationProblemV0 {
    pub hypotheses: Vec<DecisionHypothesisProjectionV0>,
    pub probes: Vec<DecisionProbeOptionV0>,
    pub outcome_supports: Vec<DecisionProbeOutcomeSupportV0>,
    pub remaining_cost_units: u64,
    pub remaining_latency_ticks: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionDisambiguationUnresolvedReasonV0 {
    NoDecisionDistinguishingProbe,
    NoProbeWithinBudget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionDisambiguationResultV0 {
    /// Multiple structural explanations may remain, but all candidate
    /// hypotheses imply the same declared downstream decision.
    DecisionSufficient {
        decision_class_id: u32,
        live_hypotheses: usize,
    },
    /// A legal probe is predicted to reduce worst-case cross-decision
    /// disagreement. Execution and response accounting belong to the owner.
    ProbeSelected {
        probe_id: u32,
        baseline_conflicting_pairs: u128,
        worst_case_conflicting_pairs: u128,
        cost_units: u64,
        latency_ticks: u64,
    },
    /// No permitted, affordable probe is predicted to reduce decision
    /// disagreement. The caller must preserve uncertainty or abstain.
    Unresolved {
        reason: DecisionDisambiguationUnresolvedReasonV0,
        baseline_conflicting_pairs: u128,
        declared_probe_count: usize,
        budget_feasible_probe_count: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionDisambiguationErrorV0 {
    EmptyHypothesisSet,
    TooManyHypotheses,
    TooManyProbeOptions,
    TooManyOutcomeSupports,
    DuplicateHypothesisId,
    DuplicateProbeId,
    UnknownHypothesisId,
    UnknownProbeId,
    DuplicateOutcomeSupport,
    IncompleteOutcomeSupport,
    UnknownObservedProbe,
    UnexpectedObservedOutcome,
    ArithmeticOverflow,
}

impl fmt::Display for DecisionDisambiguationErrorV0 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyHypothesisSet => "at least one live hypothesis is required",
            Self::TooManyHypotheses => "hypothesis count exceeds the v0 resource bound",
            Self::TooManyProbeOptions => "probe count exceeds the v0 resource bound",
            Self::TooManyOutcomeSupports => "outcome-support count exceeds the v0 resource bound",
            Self::DuplicateHypothesisId => "hypothesis IDs must be unique",
            Self::DuplicateProbeId => "probe IDs must be unique",
            Self::UnknownHypothesisId => "outcome support references an unknown hypothesis",
            Self::UnknownProbeId => "outcome support references an unknown probe",
            Self::DuplicateOutcomeSupport => "outcome support contains a duplicate row",
            Self::IncompleteOutcomeSupport => {
                "every probe must declare at least one possible outcome for every hypothesis"
            }
            Self::UnknownObservedProbe => "observed probe ID is not declared",
            Self::UnexpectedObservedOutcome => {
                "observed outcome is outside every live hypothesis prediction"
            }
            Self::ArithmeticOverflow => "decision-disagreement accounting overflowed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for DecisionDisambiguationErrorV0 {}

#[derive(Debug, Clone)]
struct ValidatedProblemV0 {
    decision_by_hypothesis: BTreeMap<u32, u32>,
    option_by_probe: BTreeMap<u32, DecisionProbeOptionV0>,
    outcomes_by_probe_hypothesis: BTreeMap<(u32, u32), BTreeSet<u32>>,
}

fn validate_problem(
    problem: &DecisionDisambiguationProblemV0,
) -> Result<ValidatedProblemV0, DecisionDisambiguationErrorV0> {
    if problem.hypotheses.is_empty() {
        return Err(DecisionDisambiguationErrorV0::EmptyHypothesisSet);
    }
    if problem.hypotheses.len() > MAX_DECISION_PROBE_HYPOTHESES_V0 {
        return Err(DecisionDisambiguationErrorV0::TooManyHypotheses);
    }
    if problem.probes.len() > MAX_DECISION_PROBE_OPTIONS_V0 {
        return Err(DecisionDisambiguationErrorV0::TooManyProbeOptions);
    }
    if problem.outcome_supports.len() > MAX_DECISION_PROBE_SUPPORTS_V0 {
        return Err(DecisionDisambiguationErrorV0::TooManyOutcomeSupports);
    }

    let mut decision_by_hypothesis = BTreeMap::new();
    for hypothesis in &problem.hypotheses {
        if decision_by_hypothesis
            .insert(hypothesis.hypothesis_id, hypothesis.decision_class_id)
            .is_some()
        {
            return Err(DecisionDisambiguationErrorV0::DuplicateHypothesisId);
        }
    }

    let mut option_by_probe = BTreeMap::new();
    for option in &problem.probes {
        if option_by_probe.insert(option.probe_id, *option).is_some() {
            return Err(DecisionDisambiguationErrorV0::DuplicateProbeId);
        }
    }

    let mut outcomes_by_probe_hypothesis =
        BTreeMap::<(u32, u32), BTreeSet<u32>>::new();
    for support in &problem.outcome_supports {
        if !option_by_probe.contains_key(&support.probe_id) {
            return Err(DecisionDisambiguationErrorV0::UnknownProbeId);
        }
        if !decision_by_hypothesis.contains_key(&support.hypothesis_id) {
            return Err(DecisionDisambiguationErrorV0::UnknownHypothesisId);
        }
        if !outcomes_by_probe_hypothesis
            .entry((support.probe_id, support.hypothesis_id))
            .or_default()
            .insert(support.possible_outcome_id)
        {
            return Err(DecisionDisambiguationErrorV0::DuplicateOutcomeSupport);
        }
    }

    for probe_id in option_by_probe.keys() {
        for hypothesis_id in decision_by_hypothesis.keys() {
            if !outcomes_by_probe_hypothesis
                .contains_key(&(*probe_id, *hypothesis_id))
            {
                return Err(DecisionDisambiguationErrorV0::IncompleteOutcomeSupport);
            }
        }
    }

    Ok(ValidatedProblemV0 {
        decision_by_hypothesis,
        option_by_probe,
        outcomes_by_probe_hypothesis,
    })
}

fn conflicting_decision_pairs(
    decision_classes: impl IntoIterator<Item = u32>,
) -> Result<u128, DecisionDisambiguationErrorV0> {
    let mut counts = BTreeMap::<u32, u128>::new();
    let mut total = 0_u128;

    for decision_class_id in decision_classes {
        let count = counts.entry(decision_class_id).or_default();
        *count = count
            .checked_add(1)
            .ok_or(DecisionDisambiguationErrorV0::ArithmeticOverflow)?;
        total = total
            .checked_add(1)
            .ok_or(DecisionDisambiguationErrorV0::ArithmeticOverflow)?;
    }

    let all_pairs = total
        .checked_mul(total.saturating_sub(1))
        .ok_or(DecisionDisambiguationErrorV0::ArithmeticOverflow)?
        / 2;
    let same_decision_pairs = counts.values().try_fold(0_u128, |sum, count| {
        let pairs = count
            .checked_mul(count.saturating_sub(1))
            .ok_or(DecisionDisambiguationErrorV0::ArithmeticOverflow)?
            / 2;
        sum.checked_add(pairs)
            .ok_or(DecisionDisambiguationErrorV0::ArithmeticOverflow)
    })?;

    all_pairs
        .checked_sub(same_decision_pairs)
        .ok_or(DecisionDisambiguationErrorV0::ArithmeticOverflow)
}

/// Choose the next probe by lexicographically minimizing worst-case residual
/// decision disagreement, then cost, latency and stable probe identity.
///
/// The policy uses set-valued candidate predictions, not probabilities. If
/// every live hypothesis implies one decision class, it returns a
/// DecisionSufficient disposition without acquiring more observations even
/// if the structural hypotheses are not unique.
pub fn select_decision_sufficient_probe_v0(
    problem: &DecisionDisambiguationProblemV0,
) -> Result<DecisionDisambiguationResultV0, DecisionDisambiguationErrorV0> {
    let validated = validate_problem(problem)?;
    let mut decision_classes = validated.decision_by_hypothesis.values().copied();
    let first_decision = decision_classes
        .next()
        .ok_or(DecisionDisambiguationErrorV0::EmptyHypothesisSet)?;

    if decision_classes.all(|decision_class_id| decision_class_id == first_decision) {
        return Ok(DecisionDisambiguationResultV0::DecisionSufficient {
            decision_class_id: first_decision,
            live_hypotheses: validated.decision_by_hypothesis.len(),
        });
    }

    let baseline_conflicts =
        conflicting_decision_pairs(validated.decision_by_hypothesis.values().copied())?;

    if validated.option_by_probe.is_empty() {
        return Ok(DecisionDisambiguationResultV0::Unresolved {
            reason: DecisionDisambiguationUnresolvedReasonV0::NoDecisionDistinguishingProbe,
            baseline_conflicting_pairs: baseline_conflicts,
            declared_probe_count: 0,
            budget_feasible_probe_count: 0,
        });
    }

    let mut budget_feasible_probe_count = 0_usize;
    let mut best: Option<(u128, u64, u64, u32)> = None;

    for (probe_id, option) in &validated.option_by_probe {
        if option.cost_units > problem.remaining_cost_units
            || option.latency_ticks > problem.remaining_latency_ticks
        {
            continue;
        }
        budget_feasible_probe_count += 1;

        let mut decisions_by_outcome = BTreeMap::<u32, Vec<u32>>::new();
        for hypothesis_id in validated.decision_by_hypothesis.keys() {
            let outcome_ids =
                &validated.outcomes_by_probe_hypothesis[&(*probe_id, *hypothesis_id)];
            let decision_class_id = validated.decision_by_hypothesis[hypothesis_id];

            for outcome_id in outcome_ids {
                decisions_by_outcome
                    .entry(*outcome_id)
                    .or_default()
                    .push(decision_class_id);
            }
        }

        let mut worst_case_conflicts = 0_u128;
        for decisions in decisions_by_outcome.values() {
            let conflicts = conflicting_decision_pairs(decisions.iter().copied())?;
            worst_case_conflicts = worst_case_conflicts.max(conflicts);
        }

        // A probe is useful only if every candidate-predicted outcome leaves
        // strictly less decision disagreement than the current hypothesis set.
        if worst_case_conflicts >= baseline_conflicts {
            continue;
        }

        let score = (
            worst_case_conflicts,
            option.cost_units,
            option.latency_ticks,
            *probe_id,
        );
        if best.is_none_or(|current| score < current) {
            best = Some(score);
        }
    }

    if let Some((worst_case_conflicts, cost_units, latency_ticks, probe_id)) = best {
        return Ok(DecisionDisambiguationResultV0::ProbeSelected {
            probe_id,
            baseline_conflicting_pairs: baseline_conflicts,
            worst_case_conflicting_pairs: worst_case_conflicts,
            cost_units,
            latency_ticks,
        });
    }

    let reason = if budget_feasible_probe_count == 0 {
        DecisionDisambiguationUnresolvedReasonV0::NoProbeWithinBudget
    } else {
        DecisionDisambiguationUnresolvedReasonV0::NoDecisionDistinguishingProbe
    };

    Ok(DecisionDisambiguationResultV0::Unresolved {
        reason,
        baseline_conflicting_pairs: baseline_conflicts,
        declared_probe_count: validated.option_by_probe.len(),
        budget_feasible_probe_count,
    })
}

/// Filter the caller's current candidate hypotheses after a returned outcome.
/// An outcome outside every candidate prediction is an explicit model
/// invalidation, not an invitation to select the nearest hypothesis.
pub fn surviving_hypotheses_after_outcome_v0(
    problem: &DecisionDisambiguationProblemV0,
    probe_id: u32,
    observed_outcome_id: u32,
) -> Result<Vec<u32>, DecisionDisambiguationErrorV0> {
    let validated = validate_problem(problem)?;
    if !validated.option_by_probe.contains_key(&probe_id) {
        return Err(DecisionDisambiguationErrorV0::UnknownObservedProbe);
    }

    let surviving = validated
        .decision_by_hypothesis
        .keys()
        .filter(|hypothesis_id| {
            validated
                .outcomes_by_probe_hypothesis
                .get(&(probe_id, **hypothesis_id))
                .is_some_and(|outcomes| outcomes.contains(&observed_outcome_id))
        })
        .copied()
        .collect::<Vec<_>>();

    if surviving.is_empty() {
        return Err(DecisionDisambiguationErrorV0::UnexpectedObservedOutcome);
    }

    Ok(surviving)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_problem() -> DecisionDisambiguationProblemV0 {
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
                // Probe 1 splits each outcome across both decisions.
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
                // Probe 2 separates the two decision classes.
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

    #[test]
    fn decision_equivalence_stops_acquisition_even_with_multiple_hypotheses() {
        let mut problem = base_problem();
        for hypothesis in &mut problem.hypotheses {
            hypothesis.decision_class_id = 7;
        }

        assert_eq!(
            select_decision_sufficient_probe_v0(&problem).unwrap(),
            DecisionDisambiguationResultV0::DecisionSufficient {
                decision_class_id: 7,
                live_hypotheses: 4,
            }
        );
    }

    #[test]
    fn minimax_probe_prioritizes_decision_separation_before_cost() {
        let problem = base_problem();

        assert_eq!(
            select_decision_sufficient_probe_v0(&problem).unwrap(),
            DecisionDisambiguationResultV0::ProbeSelected {
                probe_id: 2,
                baseline_conflicting_pairs: 4,
                worst_case_conflicting_pairs: 0,
                cost_units: 5,
                latency_ticks: 2,
            }
        );
    }

    #[test]
    fn no_probe_that_reduces_worst_case_disagreement_means_unresolved() {
        let mut problem = base_problem();
        for support in &mut problem.outcome_supports {
            support.possible_outcome_id = 0;
        }

        assert_eq!(
            select_decision_sufficient_probe_v0(&problem).unwrap(),
            DecisionDisambiguationResultV0::Unresolved {
                reason: DecisionDisambiguationUnresolvedReasonV0::NoDecisionDistinguishingProbe,
                baseline_conflicting_pairs: 4,
                declared_probe_count: 2,
                budget_feasible_probe_count: 2,
            }
        );
    }

    #[test]
    fn unaffordable_probes_preserve_the_unresolved_decision_disagreement() {
        let mut problem = base_problem();
        problem.remaining_cost_units = 0;

        assert_eq!(
            select_decision_sufficient_probe_v0(&problem).unwrap(),
            DecisionDisambiguationResultV0::Unresolved {
                reason: DecisionDisambiguationUnresolvedReasonV0::NoProbeWithinBudget,
                baseline_conflicting_pairs: 4,
                declared_probe_count: 2,
                budget_feasible_probe_count: 0,
            }
        );
    }

    #[test]
    fn returned_outcome_filters_only_compatible_candidate_hypotheses() {
        let problem = base_problem();

        assert_eq!(
            surviving_hypotheses_after_outcome_v0(&problem, 2, 400).unwrap(),
            vec![3, 4]
        );
    }

    #[test]
    fn unpredicted_outcome_is_explicit_model_invalidation() {
        let problem = base_problem();

        assert_eq!(
            surviving_hypotheses_after_outcome_v0(&problem, 2, 999).unwrap_err(),
            DecisionDisambiguationErrorV0::UnexpectedObservedOutcome
        );
    }

    #[test]
    fn stable_probe_identity_breaks_equal_quality_cost_and_latency_ties() {
        let mut problem = base_problem();
        problem.probes.push(DecisionProbeOptionV0 {
            probe_id: 0,
            cost_units: 5,
            latency_ticks: 2,
        });
        for (hypothesis_id, possible_outcome_id) in [(1, 500), (2, 500), (3, 600), (4, 600)] {
            problem
                .outcome_supports
                .push(DecisionProbeOutcomeSupportV0 {
                    probe_id: 0,
                    hypothesis_id,
                    possible_outcome_id,
                });
        }

        let result = select_decision_sufficient_probe_v0(&problem).unwrap();
        assert!(matches!(
            result,
            DecisionDisambiguationResultV0::ProbeSelected { probe_id: 0, .. }
        ));
    }

    #[test]
    fn incomplete_probe_prediction_support_is_rejected() {
        let mut problem = base_problem();
        problem.outcome_supports.pop();

        assert_eq!(
            select_decision_sufficient_probe_v0(&problem).unwrap_err(),
            DecisionDisambiguationErrorV0::IncompleteOutcomeSupport
        );
    }

    #[test]
    fn input_order_does_not_change_the_selected_probe() {
        let mut problem = base_problem();
        let expected = select_decision_sufficient_probe_v0(&problem).unwrap();
        problem.hypotheses.reverse();
        problem.probes.reverse();
        problem.outcome_supports.reverse();

        assert_eq!(
            select_decision_sufficient_probe_v0(&problem).unwrap(),
            expected
        );
    }
}
