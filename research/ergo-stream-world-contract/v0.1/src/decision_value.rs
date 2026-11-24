//! Exact finite decision-value oracle for experiment selection.
//!
//! This module separates information gain from expected value of sample
//! information (EVSI). It is an evaluator/reference baseline, not an online
//! scheduler and not a general posterior engine.
//!
//! The finite task has:
//! - a prior over hypotheses;
//! - a finite action set and non-negative loss for each hypothesis/action;
//! - an experiment with integer likelihood weights for each hypothesis/outcome;
//! - a cost expressed in the same declared loss units.
//!
//! Expected decision loss and EVSI are computed with exact integer numerators
//! over a common denominator. Shannon mutual information is reported only as
//! a floating-point diagnostic; it does not authorize or rank actions.

use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecisionHypothesisV0 {
    pub id: u32,
    pub prior_weight: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecisionActionV0 {
    pub id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecisionLossV0 {
    pub hypothesis_id: u32,
    pub action_id: u32,
    pub loss_units: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DecisionProblemV0 {
    pub hypotheses: Vec<DecisionHypothesisV0>,
    pub actions: Vec<DecisionActionV0>,
    pub losses: Vec<DecisionLossV0>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExperimentLikelihoodV0 {
    pub hypothesis_id: u32,
    pub outcome_id: u32,
    pub likelihood_weight: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionExperimentV0 {
    pub id: u32,
    pub outcome_ids: Vec<u32>,
    /// Each hypothesis row must sum to this integer likelihood scale.
    pub likelihood_scale: u16,
    pub likelihoods: Vec<ExperimentLikelihoodV0>,
    /// Must be expressed in the same loss units as DecisionLossV0.
    pub cost_units: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutcomeDecisionV0 {
    pub outcome_id: u32,
    /// Unnormalised prior-predictive outcome mass.
    pub outcome_mass: u128,
    /// None only when the outcome is impossible under every live hypothesis.
    pub best_action_id: Option<u32>,
    /// Minimum unnormalised posterior-weighted loss for this outcome.
    pub minimum_weighted_loss: u128,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecisionValueReportV0 {
    pub experiment_id: u32,
    pub baseline_action_id: u32,
    pub outcome_decisions: Vec<OutcomeDecisionV0>,
    pub prior_total_weight: u128,
    pub likelihood_scale: u16,
    /// Numerators share denominator = prior_total_weight * likelihood_scale.
    pub expected_loss_before_scaled_numerator: u128,
    pub expected_loss_after_numerator: u128,
    pub common_denominator: u128,
    /// Exact expected loss reduction in numerator units.
    pub evsi_numerator: u128,
    pub cost_units: u32,
    /// Exact net EVSI numerator over common_denominator; may be negative.
    pub net_evsi_numerator: i128,
    /// Descriptive mutual information in bits; floating point and not used
    /// in the exact decision-value calculation.
    pub mutual_information_bits: f64,
    pub possible_outcomes_changing_baseline_action: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionValueErrorV0 {
    EmptyHypotheses,
    EmptyActions,
    EmptyOutcomes,
    ZeroPriorWeight,
    ZeroLikelihoodScale,
    DuplicateHypothesisId,
    DuplicateActionId,
    DuplicateOutcomeId,
    UnknownHypothesisId,
    UnknownActionId,
    UnknownOutcomeId,
    DuplicateLossEntry,
    IncompleteLossTable,
    DuplicateLikelihoodEntry,
    IncompleteLikelihoodTable,
    LikelihoodExceedsScale,
    LikelihoodRowDoesNotNormalize,
    ArithmeticOverflow,
    NegativeEvsiInvariant,
    NonFiniteInformationGain,
}

impl fmt::Display for DecisionValueErrorV0 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyHypotheses => "decision problem must contain at least one hypothesis",
            Self::EmptyActions => "decision problem must contain at least one action",
            Self::EmptyOutcomes => "experiment must contain at least one outcome",
            Self::ZeroPriorWeight => "every live hypothesis must have positive prior weight",
            Self::ZeroLikelihoodScale => "experiment likelihood scale must be positive",
            Self::DuplicateHypothesisId => "hypothesis IDs must be unique",
            Self::DuplicateActionId => "action IDs must be unique",
            Self::DuplicateOutcomeId => "experiment outcome IDs must be unique",
            Self::UnknownHypothesisId => "loss or likelihood references an unknown hypothesis",
            Self::UnknownActionId => "loss references an unknown action",
            Self::UnknownOutcomeId => "likelihood references an unknown outcome",
            Self::DuplicateLossEntry => "loss table contains a duplicate hypothesis/action pair",
            Self::IncompleteLossTable => "loss table must cover every hypothesis/action pair",
            Self::DuplicateLikelihoodEntry => {
                "likelihood table contains a duplicate hypothesis/outcome pair"
            }
            Self::IncompleteLikelihoodTable => {
                "likelihood table must cover every hypothesis/outcome pair"
            }
            Self::LikelihoodExceedsScale => "likelihood weight exceeds the declared scale",
            Self::LikelihoodRowDoesNotNormalize => {
                "likelihood weights for each hypothesis must sum to the declared scale"
            }
            Self::ArithmeticOverflow => "exact decision-value arithmetic overflowed its bounded integer representation",
            Self::NegativeEvsiInvariant => {
                "expected loss after an available experiment exceeded pre-experiment Bayes risk"
            }
            Self::NonFiniteInformationGain => "mutual-information diagnostic was not finite",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for DecisionValueErrorV0 {}

/// Compute the exact finite Bayes risk before and after an experiment.
///
/// The evaluator owns the prior, loss table and action-conditioned likelihood
/// table. The candidate may propose an experiment, but cannot redefine these
/// evaluator contracts. The result distinguishes:
/// - information gain about hypotheses;
/// - expected reduction in the minimum available decision loss;
/// - net decision value after the declared experiment cost.
///
/// Integer weights/losses make the EVSI numerator exact. Mutual information
/// is a separate floating-point diagnostic and must never be substituted for
/// decision value.
pub fn evaluate_decision_value(
    problem: &DecisionProblemV0,
    experiment: &DecisionExperimentV0,
) -> Result<DecisionValueReportV0, DecisionValueErrorV0> {
    validate_problem(problem)?;
    validate_experiment(problem, experiment)?;

    let prior_total_weight = problem
        .hypotheses
        .iter()
        .try_fold(0_u128, |total, hypothesis| {
            total.checked_add(u128::from(hypothesis.prior_weight))
        })
        .ok_or(DecisionValueErrorV0::ArithmeticOverflow)?;

    let mut baseline_action_id = problem.actions[0].id;
    let mut baseline_loss_numerator = u128::MAX;

    for action in &problem.actions {
        let weighted_loss = weighted_action_loss(problem, |hypothesis| {
            u128::from(hypothesis.prior_weight)
        }, action.id)?;
        if weighted_loss < baseline_loss_numerator {
            baseline_loss_numerator = weighted_loss;
            baseline_action_id = action.id;
        }
    }

    let mut outcome_decisions = Vec::with_capacity(experiment.outcome_ids.len());
    let mut expected_loss_after_numerator = 0_u128;
    let mut possible_outcomes_changing_baseline_action = 0_usize;

    for &outcome_id in &experiment.outcome_ids {
        let outcome_mass = problem
            .hypotheses
            .iter()
            .try_fold(0_u128, |total, hypothesis| {
                let likelihood = likelihood_weight(experiment, hypothesis.id, outcome_id)?;
                let weighted = u128::from(hypothesis.prior_weight)
                    .checked_mul(u128::from(likelihood))
                    .ok_or(DecisionValueErrorV0::ArithmeticOverflow)?;
                total
                    .checked_add(weighted)
                    .ok_or(DecisionValueErrorV0::ArithmeticOverflow)
            })?;

        if outcome_mass == 0 {
            outcome_decisions.push(OutcomeDecisionV0 {
                outcome_id,
                outcome_mass,
                best_action_id: None,
                minimum_weighted_loss: 0,
            });
            continue;
        }

        let mut best_action_id = problem.actions[0].id;
        let mut minimum_weighted_loss = u128::MAX;

        for action in &problem.actions {
            let weighted_loss = weighted_action_loss(
                problem,
                |hypothesis| {
                    likelihood_weight(experiment, hypothesis.id, outcome_id)
                        .map(|likelihood| {
                            u128::from(hypothesis.prior_weight) * u128::from(likelihood)
                        })
                },
                action.id,
            )?;

            if weighted_loss < minimum_weighted_loss {
                minimum_weighted_loss = weighted_loss;
                best_action_id = action.id;
            }
        }

        expected_loss_after_numerator = expected_loss_after_numerator
            .checked_add(minimum_weighted_loss)
            .ok_or(DecisionValueErrorV0::ArithmeticOverflow)?;

        if best_action_id != baseline_action_id {
            possible_outcomes_changing_baseline_action += 1;
        }

        outcome_decisions.push(OutcomeDecisionV0 {
            outcome_id,
            outcome_mass,
            best_action_id: Some(best_action_id),
            minimum_weighted_loss,
        });
    }

    let likelihood_scale = u128::from(experiment.likelihood_scale);
    let expected_loss_before_scaled_numerator = baseline_loss_numerator
        .checked_mul(likelihood_scale)
        .ok_or(DecisionValueErrorV0::ArithmeticOverflow)?;

    if expected_loss_after_numerator > expected_loss_before_scaled_numerator {
        return Err(DecisionValueErrorV0::NegativeEvsiInvariant);
    }

    let evsi_numerator =
        expected_loss_before_scaled_numerator - expected_loss_after_numerator;
    let common_denominator = prior_total_weight
        .checked_mul(likelihood_scale)
        .ok_or(DecisionValueErrorV0::ArithmeticOverflow)?;

    let information_gain_bits = mutual_information_bits(problem, experiment, prior_total_weight)?;
    let evsi_i128 = i128::try_from(evsi_numerator)
        .map_err(|_| DecisionValueErrorV0::ArithmeticOverflow)?;
    let cost_numerator = i128::from(experiment.cost_units)
        .checked_mul(
            i128::try_from(common_denominator)
                .map_err(|_| DecisionValueErrorV0::ArithmeticOverflow)?,
        )
        .ok_or(DecisionValueErrorV0::ArithmeticOverflow)?;
    let net_evsi_numerator = evsi_i128
        .checked_sub(cost_numerator)
        .ok_or(DecisionValueErrorV0::ArithmeticOverflow)?;

    Ok(DecisionValueReportV0 {
        experiment_id: experiment.id,
        baseline_action_id,
        outcome_decisions,
        prior_total_weight,
        likelihood_scale: experiment.likelihood_scale,
        expected_loss_before_scaled_numerator,
        expected_loss_after_numerator,
        common_denominator,
        evsi_numerator,
        cost_units: experiment.cost_units,
        net_evsi_numerator,
        mutual_information_bits: information_gain_bits,
        possible_outcomes_changing_baseline_action,
    })
}

fn validate_problem(problem: &DecisionProblemV0) -> Result<(), DecisionValueErrorV0> {
    if problem.hypotheses.is_empty() {
        return Err(DecisionValueErrorV0::EmptyHypotheses);
    }
    if problem.actions.is_empty() {
        return Err(DecisionValueErrorV0::EmptyActions);
    }
    if problem
        .hypotheses
        .iter()
        .any(|hypothesis| hypothesis.prior_weight == 0)
    {
        return Err(DecisionValueErrorV0::ZeroPriorWeight);
    }
    if !unique_by(&problem.hypotheses, |hypothesis| hypothesis.id)
        || !unique_by(&problem.actions, |action| action.id)
    {
        return Err(DecisionValueErrorV0::DuplicateHypothesisId);
    }

    let expected_loss_entries = problem
        .hypotheses
        .len()
        .checked_mul(problem.actions.len())
        .ok_or(DecisionValueErrorV0::ArithmeticOverflow)?;
    if problem.losses.len() != expected_loss_entries {
        return Err(DecisionValueErrorV0::IncompleteLossTable);
    }

    let mut seen_losses = BTreeSet::new();
    for loss in &problem.losses {
        if !problem
            .hypotheses
            .iter()
            .any(|hypothesis| hypothesis.id == loss.hypothesis_id)
        {
            return Err(DecisionValueErrorV0::UnknownHypothesisId);
        }
        if !problem
            .actions
            .iter()
            .any(|action| action.id == loss.action_id)
        {
            return Err(DecisionValueErrorV0::UnknownActionId);
        }
        if !seen_losses.insert((loss.hypothesis_id, loss.action_id)) {
            return Err(DecisionValueErrorV0::DuplicateLossEntry);
        }
    }

    Ok(())
}

fn validate_experiment(
    problem: &DecisionProblemV0,
    experiment: &DecisionExperimentV0,
) -> Result<(), DecisionValueErrorV0> {
    if experiment.outcome_ids.is_empty() {
        return Err(DecisionValueErrorV0::EmptyOutcomes);
    }
    if experiment.likelihood_scale == 0 {
        return Err(DecisionValueErrorV0::ZeroLikelihoodScale);
    }
    if !unique_by(&experiment.outcome_ids, |id| *id) {
        return Err(DecisionValueErrorV0::DuplicateOutcomeId);
    }

    let expected_likelihood_entries = problem
        .hypotheses
        .len()
        .checked_mul(experiment.outcome_ids.len())
        .ok_or(DecisionValueErrorV0::ArithmeticOverflow)?;
    if experiment.likelihoods.len() != expected_likelihood_entries {
        return Err(DecisionValueErrorV0::IncompleteLikelihoodTable);
    }

    let mut seen_likelihoods = BTreeSet::new();
    for hypothesis in &problem.hypotheses {
        let mut row_total = 0_u32;
        for outcome_id in &experiment.outcome_ids {
            let entry = experiment
                .likelihoods
                .iter()
                .find(|entry| {
                    entry.hypothesis_id == hypothesis.id && entry.outcome_id == *outcome_id
                })
                .ok_or(DecisionValueErrorV0::IncompleteLikelihoodTable)?;
            if entry.likelihood_weight > experiment.likelihood_scale {
                return Err(DecisionValueErrorV0::LikelihoodExceedsScale);
            }
            row_total += u32::from(entry.likelihood_weight);
            if !seen_likelihoods.insert((entry.hypothesis_id, entry.outcome_id)) {
                return Err(DecisionValueErrorV0::DuplicateLikelihoodEntry);
            }
        }
        if row_total != u32::from(experiment.likelihood_scale) {
            return Err(DecisionValueErrorV0::LikelihoodRowDoesNotNormalize);
        }
    }

    if experiment.likelihoods.iter().any(|entry| {
        !problem
            .hypotheses
            .iter()
            .any(|hypothesis| hypothesis.id == entry.hypothesis_id)
    }) {
        return Err(DecisionValueErrorV0::UnknownHypothesisId);
    }
    if experiment
        .likelihoods
        .iter()
        .any(|entry| !experiment.outcome_ids.contains(&entry.outcome_id))
    {
        return Err(DecisionValueErrorV0::UnknownOutcomeId);
    }

    Ok(())
}

fn likelihood_weight(
    experiment: &DecisionExperimentV0,
    hypothesis_id: u32,
    outcome_id: u32,
) -> Result<u16, DecisionValueErrorV0> {
    experiment
        .likelihoods
        .iter()
        .find(|entry| entry.hypothesis_id == hypothesis_id && entry.outcome_id == outcome_id)
        .map(|entry| entry.likelihood_weight)
        .ok_or(DecisionValueErrorV0::IncompleteLikelihoodTable)
}

fn weighted_action_loss(
    problem: &DecisionProblemV0,
    weight_of: impl Fn(&DecisionHypothesisV0) -> Result<u128, DecisionValueErrorV0>,
    action_id: u32,
) -> Result<u128, DecisionValueErrorV0> {
    problem
        .hypotheses
        .iter()
        .try_fold(0_u128, |total, hypothesis| {
            let weight = weight_of(hypothesis)?;
            let loss = problem
                .losses
                .iter()
                .find(|loss| {
                    loss.hypothesis_id == hypothesis.id && loss.action_id == action_id
                })
                .ok_or(DecisionValueErrorV0::IncompleteLossTable)?;
            let weighted_loss = weight
                .checked_mul(u128::from(loss.loss_units))
                .ok_or(DecisionValueErrorV0::ArithmeticOverflow)?;
            total
                .checked_add(weighted_loss)
                .ok_or(DecisionValueErrorV0::ArithmeticOverflow)
        })
}

fn mutual_information_bits(
    problem: &DecisionProblemV0,
    experiment: &DecisionExperimentV0,
    prior_total_weight: u128,
) -> Result<f64, DecisionValueErrorV0> {
    let scale = f64::from(experiment.likelihood_scale);
    let mut information = 0.0;

    for hypothesis in &problem.hypotheses {
        let prior_probability = f64::from(hypothesis.prior_weight)
            / prior_total_weight as f64;
        for outcome_id in &experiment.outcome_ids {
            let likelihood_probability =
                f64::from(likelihood_weight(experiment, hypothesis.id, *outcome_id)?) / scale;
            if likelihood_probability == 0.0 {
                continue;
            }
            let mut outcome_probability = 0.0;
            for other_hypothesis in &problem.hypotheses {
                let other_prior = f64::from(other_hypothesis.prior_weight)
                    / prior_total_weight as f64;
                let other_likelihood = f64::from(likelihood_weight(
                    experiment,
                    other_hypothesis.id,
                    *outcome_id,
                )?) / scale;
                outcome_probability += other_prior * other_likelihood;
            }
            let joint_probability = prior_probability * likelihood_probability;
            information += joint_probability * (likelihood_probability / outcome_probability).log2();
        }
    }

    information.is_finite()
        .then_some(information)
        .ok_or(DecisionValueErrorV0::NonFiniteInformationGain)
}

fn unique_by<T>(values: &[T], id_of: impl Fn(&T) -> u32) -> bool {
    values.iter().enumerate().all(|(index, value)| {
        !values[index + 1..]
            .iter()
            .any(|later| id_of(value) == id_of(later))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decision_problem() -> DecisionProblemV0 {
        DecisionProblemV0 {
            hypotheses: vec![
                DecisionHypothesisV0 { id: 0, prior_weight: 4 },
                DecisionHypothesisV0 { id: 1, prior_weight: 4 },
                DecisionHypothesisV0 { id: 2, prior_weight: 2 },
            ],
            actions: vec![
                DecisionActionV0 { id: 10 },
                DecisionActionV0 { id: 20 },
            ],
            losses: vec![
                DecisionLossV0 { hypothesis_id: 0, action_id: 10, loss_units: 0 },
                DecisionLossV0 { hypothesis_id: 0, action_id: 20, loss_units: 10 },
                DecisionLossV0 { hypothesis_id: 1, action_id: 10, loss_units: 0 },
                DecisionLossV0 { hypothesis_id: 1, action_id: 20, loss_units: 10 },
                DecisionLossV0 { hypothesis_id: 2, action_id: 10, loss_units: 10 },
                DecisionLossV0 { hypothesis_id: 2, action_id: 20, loss_units: 0 },
            ],
        }
    }

    fn information_only_experiment(cost_units: u32) -> DecisionExperimentV0 {
        DecisionExperimentV0 {
            id: 100,
            outcome_ids: vec![0, 1],
            likelihood_scale: 2,
            likelihoods: vec![
                ExperimentLikelihoodV0 { hypothesis_id: 0, outcome_id: 0, likelihood_weight: 2 },
                ExperimentLikelihoodV0 { hypothesis_id: 0, outcome_id: 1, likelihood_weight: 0 },
                ExperimentLikelihoodV0 { hypothesis_id: 1, outcome_id: 0, likelihood_weight: 0 },
                ExperimentLikelihoodV0 { hypothesis_id: 1, outcome_id: 1, likelihood_weight: 2 },
                ExperimentLikelihoodV0 { hypothesis_id: 2, outcome_id: 0, likelihood_weight: 1 },
                ExperimentLikelihoodV0 { hypothesis_id: 2, outcome_id: 1, likelihood_weight: 1 },
            ],
            cost_units,
        }
    }

    fn decision_relevant_experiment(cost_units: u32) -> DecisionExperimentV0 {
        DecisionExperimentV0 {
            id: 101,
            outcome_ids: vec![0, 1],
            likelihood_scale: 2,
            likelihoods: vec![
                ExperimentLikelihoodV0 { hypothesis_id: 0, outcome_id: 0, likelihood_weight: 2 },
                ExperimentLikelihoodV0 { hypothesis_id: 0, outcome_id: 1, likelihood_weight: 0 },
                ExperimentLikelihoodV0 { hypothesis_id: 1, outcome_id: 0, likelihood_weight: 2 },
                ExperimentLikelihoodV0 { hypothesis_id: 1, outcome_id: 1, likelihood_weight: 0 },
                ExperimentLikelihoodV0 { hypothesis_id: 2, outcome_id: 0, likelihood_weight: 0 },
                ExperimentLikelihoodV0 { hypothesis_id: 2, outcome_id: 1, likelihood_weight: 2 },
            ],
            cost_units,
        }
    }

    #[test]
    fn higher_information_gain_can_have_zero_decision_value() {
        let problem = decision_problem();
        let information_only =
            evaluate_decision_value(&problem, &information_only_experiment(1)).unwrap();
        let decision_relevant =
            evaluate_decision_value(&problem, &decision_relevant_experiment(1)).unwrap();

        assert!((information_only.mutual_information_bits - 0.8).abs() < 1e-12);
        assert!(information_only.evsi_numerator == 0);
        assert_eq!(information_only.net_evsi_numerator, -20);
        assert_eq!(information_only.baseline_action_id, 10);
        assert!(information_only.possible_outcomes_changing_baseline_action == 0);

        assert!((decision_relevant.mutual_information_bits - 0.7219280948873623).abs() < 1e-12);
        assert_eq!(decision_relevant.evsi_numerator, 40);
        assert_eq!(decision_relevant.net_evsi_numerator, 20);
        assert_eq!(decision_relevant.baseline_action_id, 10);
        assert!(decision_relevant.possible_outcomes_changing_baseline_action == 1);
        assert_eq!(
            decision_relevant.outcome_decisions[1].best_action_id,
            Some(20)
        );
    }

    #[test]
    fn exact_evsi_is_not_negative_for_a_valid_likelihood_model() {
        let problem = decision_problem();
        let experiment = decision_relevant_experiment(0);
        let report = evaluate_decision_value(&problem, &experiment).unwrap();

        assert_eq!(report.expected_loss_before_scaled_numerator, 40);
        assert_eq!(report.expected_loss_after_numerator, 0);
        assert_eq!(report.common_denominator, 20);
        assert_eq!(report.evsi_numerator, 40);
    }

    #[test]
    fn experiment_cost_can_make_positive_evsi_net_negative() {
        let problem = decision_problem();
        let report =
            evaluate_decision_value(&problem, &decision_relevant_experiment(3)).unwrap();

        assert_eq!(report.evsi_numerator, 40);
        assert_eq!(report.net_evsi_numerator, -20);
    }

    #[test]
    fn rejects_likelihood_rows_that_do_not_normalize() {
        let problem = decision_problem();
        let mut experiment = decision_relevant_experiment(0);
        experiment.likelihoods[0].likelihood_weight = 1;

        assert_eq!(
            evaluate_decision_value(&problem, &experiment),
            Err(DecisionValueErrorV0::LikelihoodRowDoesNotNormalize)
        );
    }

    #[test]
    fn rejects_an_incomplete_loss_table() {
        let mut problem = decision_problem();
        problem.losses.pop();

        assert_eq!(
            evaluate_decision_value(&problem, &decision_relevant_experiment(0)),
            Err(DecisionValueErrorV0::IncompleteLossTable)
        );
    }

    #[test]
    fn impossible_outcomes_have_no_selected_action() {
        let problem = decision_problem();
        let mut experiment = decision_relevant_experiment(0);
        experiment.outcome_ids.push(2);
        experiment.likelihoods.extend([
            ExperimentLikelihoodV0 { hypothesis_id: 0, outcome_id: 2, likelihood_weight: 0 },
            ExperimentLikelihoodV0 { hypothesis_id: 1, outcome_id: 2, likelihood_weight: 0 },
            ExperimentLikelihoodV0 { hypothesis_id: 2, outcome_id: 2, likelihood_weight: 0 },
        ]);
        experiment.likelihood_scale = 3;
        for likelihood in &mut experiment.likelihoods {
            if likelihood.outcome_id == 0 || likelihood.outcome_id == 1 {
                likelihood.likelihood_weight = 1;
            }
        }

        let report = evaluate_decision_value(&problem, &experiment).unwrap();
        let impossible = report
            .outcome_decisions
            .iter()
            .find(|outcome| outcome.outcome_id == 2)
            .unwrap();

        assert_eq!(impossible.outcome_mass, 0);
        assert_eq!(impossible.best_action_id, None);
    }
}
