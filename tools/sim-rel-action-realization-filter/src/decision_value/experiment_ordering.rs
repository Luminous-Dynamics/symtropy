//! Exact finite-channel experiment ordering under model misspecification.
//!
//! This module separates the channel used for inference from the channel that
//! generates observations. It validates nominal Blackwell dominance using an
//! explicit stochastic garbling witness, then scores the chosen terminal
//! decisions under the true channel. Mutual information is provided only as a
//! floating-point diagnostic; posterior, decision and risk receipts use exact
//! rational arithmetic.

use crate::Rational;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteObservationChannel {
    pub channel_id: String,
    pub state_ids: Vec<String>,
    pub observation_ids: Vec<String>,
    /// Conditional probabilities indexed [state][observation].
    pub probabilities: Vec<Vec<Rational>>,
}

impl FiniteObservationChannel {
    pub fn validate(&self) -> Result<(), String> {
        if self.channel_id.trim().is_empty() {
            return Err("channel ID must be non-empty".into());
        }
        validate_unique_ids(&self.state_ids, "state")?;
        validate_unique_ids(&self.observation_ids, "observation")?;
        if self.probabilities.len() != self.state_ids.len() {
            return Err("channel row count must equal the state vocabulary size".into());
        }
        for (state, row) in self.probabilities.iter().enumerate() {
            if row.len() != self.observation_ids.len() {
                return Err(format!(
                    "channel state row {state} has the wrong observation dimension"
                ));
            }
            let mut mass = Rational::zero();
            for probability in row {
                if !probability.is_probability() {
                    return Err(format!(
                        "channel state row {state} has an invalid probability"
                    ));
                }
                mass = mass.checked_add(*probability)?;
            }
            if mass != Rational::one() {
                return Err(format!("channel state row {state} must sum exactly to one"));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlackwellGarblingWitness {
    /// Rows correspond to observations from the more-informative channel.
    /// Columns correspond to observations from the less-informative channel.
    pub probabilities: Vec<Vec<Rational>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlackwellDominanceReceipt {
    pub more_informative_channel_id: String,
    pub less_informative_channel_id: String,
    pub witness: BlackwellGarblingWitness,
    pub exactly_verified: bool,
}

/// Verify the exact Blackwell relation:
/// less(s, y) = sum_z more(s, z) * garbling(z, y) for every state and output.
///
/// This validator consumes an explicit stochastic garbling witness. It does
/// not search for one; a supplied witness is not a solved feasibility problem.
pub fn verify_blackwell_dominance(
    more_informative: &FiniteObservationChannel,
    less_informative: &FiniteObservationChannel,
    witness: &BlackwellGarblingWitness,
) -> Result<BlackwellDominanceReceipt, String> {
    more_informative.validate()?;
    less_informative.validate()?;
    if more_informative.state_ids != less_informative.state_ids {
        return Err("Blackwell channels must share the same ordered state vocabulary".into());
    }
    if witness.probabilities.len() != more_informative.observation_ids.len() {
        return Err("garbling row count must equal the more-informative observation count".into());
    }

    for (source_observation, row) in witness.probabilities.iter().enumerate() {
        if row.len() != less_informative.observation_ids.len() {
            return Err(format!(
                "garbling row {source_observation} has the wrong output dimension"
            ));
        }
        let mut row_mass = Rational::zero();
        for probability in row {
            if !probability.is_probability() {
                return Err(format!(
                    "garbling row {source_observation} contains an invalid probability"
                ));
            }
            row_mass = row_mass.checked_add(*probability)?;
        }
        if row_mass != Rational::one() {
            return Err(format!(
                "garbling row {source_observation} must sum exactly to one"
            ));
        }
    }

    for (state, more_row) in more_informative.probabilities.iter().enumerate() {
        let less_row = &less_informative.probabilities[state];
        for (less_observation, target_probability) in less_row.iter().copied().enumerate() {
            let mut reconstructed = Rational::zero();
            for (more_observation, more_probability) in more_row.iter().copied().enumerate() {
                reconstructed = reconstructed
                    .checked_add(more_probability.checked_mul(
                        witness.probabilities[more_observation][less_observation],
                    )?)?;
            }
            if reconstructed != target_probability {
                return Err(format!(
                    "garbling witness does not reconstruct state {state}, observation {less_observation}"
                ));
            }
        }
    }

    Ok(BlackwellDominanceReceipt {
        more_informative_channel_id: more_informative.channel_id.clone(),
        less_informative_channel_id: less_informative.channel_id.clone(),
        witness: witness.clone(),
        exactly_verified: true,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChannelDecisionLossTable {
    pub decision_ids: Vec<String>,
    /// Loss indexed [decision][state].
    pub losses: Vec<Vec<Rational>>,
    pub abstention_loss: Rational,
}

impl ChannelDecisionLossTable {
    pub fn validate(&self, state_count: usize) -> Result<(), String> {
        validate_unique_ids(&self.decision_ids, "decision")?;
        if self.losses.len() != self.decision_ids.len() || self.decision_ids.is_empty() {
            return Err("one non-empty loss row is required for every decision".into());
        }
        for (decision, losses) in self.losses.iter().enumerate() {
            if losses.len() != state_count {
                return Err(format!("decision {decision} has the wrong state dimension"));
            }
        }
        Ok(())
    }
}

/// Construct a finite-rational generalized-Bayes/Gibbs posterior rule.
///
/// The supplied factors are indexed [observation][state] and represent
/// nonnegative weights proportional to exp(-eta * loss(state, observation)).
/// They are normalized against the prior separately for each observation.
/// Keeping the factors explicit allows exact rational oracle tests; callers
/// must document how the factors/temperature were calibrated. This is the
/// Gibbs posterior update primitive, not the full Gibbs-EIG design optimizer.
pub fn generalized_bayes_posterior_rule_from_weights(
    rule_id: &str,
    assumed: &FiniteObservationChannel,
    prior: &[Rational],
    gibbs_factors: &[Vec<Rational>],
) -> Result<InferencePosteriorRule, String> {
    if rule_id.trim().is_empty() {
        return Err("generalized-Bayes rule ID must be non-empty".into());
    }
    assumed.validate()?;
    validate_prior(prior, assumed.state_ids.len())?;
    if gibbs_factors.len() != assumed.observation_ids.len() {
        return Err("Gibbs factor row count must equal the observation vocabulary size".into());
    }

    let mut posterior_by_observation = Vec::with_capacity(gibbs_factors.len());
    for (observation, factors) in gibbs_factors.iter().enumerate() {
        if factors.len() != assumed.state_ids.len() {
            return Err(format!(
                "Gibbs factor row {observation} has the wrong state dimension"
            ));
        }
        let mut unnormalized = Vec::with_capacity(prior.len());
        let mut normalizer = Rational::zero();
        for (prior_mass, factor) in prior.iter().copied().zip(factors.iter().copied()) {
            let mass = prior_mass.checked_mul(factor)?;
            normalizer = normalizer.checked_add(mass)?;
            unnormalized.push(mass);
        }
        if normalizer.is_zero() {
            return Err(format!(
                "Gibbs factor row {observation} has zero prior-weighted normalizer"
            ));
        }
        let posterior = unnormalized
            .iter()
            .map(|mass| mass.checked_div(normalizer))
            .collect::<Result<Vec<_>, _>>()?;
        posterior_by_observation.push(posterior);
    }

    let rule = InferencePosteriorRule {
        rule_id: rule_id.to_string(),
        assumed_channel_id: assumed.channel_id.clone(),
        state_ids: assumed.state_ids.clone(),
        observation_ids: assumed.observation_ids.clone(),
        posterior_by_observation,
    };
    rule.validate(assumed)?;
    Ok(rule)
}

/// A posterior-producing inference procedure over a declared observation
/// alphabet. It is separate from the working likelihood channel so model
/// misspecification and inferential approximation can be varied independently.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InferencePosteriorRule {
    pub rule_id: String,
    pub assumed_channel_id: String,
    pub state_ids: Vec<String>,
    pub observation_ids: Vec<String>,
    /// Approximate/working posterior indexed [observation][state].
    pub posterior_by_observation: Vec<Vec<Rational>>,
}

impl InferencePosteriorRule {
    pub fn validate(&self, assumed: &FiniteObservationChannel) -> Result<(), String> {
        if self.rule_id.trim().is_empty() {
            return Err("inference rule ID must be non-empty".into());
        }
        if self.assumed_channel_id != assumed.channel_id {
            return Err("inference rule is not bound to the supplied assumed channel".into());
        }
        if self.state_ids != assumed.state_ids || self.observation_ids != assumed.observation_ids {
            return Err(
                "inference rule state/observation vocabularies do not match the assumed channel"
                    .into(),
            );
        }
        if self.posterior_by_observation.len() != assumed.observation_ids.len() {
            return Err("inference rule must provide one posterior for every observation".into());
        }
        for (observation, posterior) in self.posterior_by_observation.iter().enumerate() {
            if posterior.len() != assumed.state_ids.len() {
                return Err(format!(
                    "inference posterior {observation} has the wrong state dimension"
                ));
            }
            let mut total = Rational::zero();
            for probability in posterior {
                if !probability.is_probability() {
                    return Err(format!(
                        "inference posterior {observation} contains an invalid probability"
                    ));
                }
                total = total.checked_add(*probability)?;
            }
            if total != Rational::one() {
                return Err(format!(
                    "inference posterior {observation} must sum exactly to one"
                ));
            }
        }
        Ok(())
    }
}

/// Construct the exact Bayesian posterior rule induced by a working channel
/// and prior. Rows for observations impossible under that working model carry
/// the prior as a harmless placeholder; the evaluator will abstain on those
/// observations if they occur under the true channel.
pub fn exact_bayes_posterior_rule(
    assumed: &FiniteObservationChannel,
    prior: &[Rational],
) -> Result<InferencePosteriorRule, String> {
    assumed.validate()?;
    validate_prior(prior, assumed.state_ids.len())?;
    let mut posterior_by_observation = Vec::with_capacity(assumed.observation_ids.len());
    for observation in 0..assumed.observation_ids.len() {
        let mut joint = Vec::with_capacity(assumed.state_ids.len());
        let mut marginal = Rational::zero();
        for (state, prior_mass) in prior.iter().copied().enumerate() {
            let mass = prior_mass.checked_mul(assumed.probabilities[state][observation])?;
            joint.push(mass);
            marginal = marginal.checked_add(mass)?;
        }
        let posterior = if marginal.is_zero() {
            prior.to_vec()
        } else {
            joint
                .iter()
                .map(|mass| mass.checked_div(marginal))
                .collect::<Result<Vec<_>, _>>()?
        };
        posterior_by_observation.push(posterior);
    }
    let rule = InferencePosteriorRule {
        rule_id: format!("exact-bayes:{}", assumed.channel_id),
        assumed_channel_id: assumed.channel_id.clone(),
        state_ids: assumed.state_ids.clone(),
        observation_ids: assumed.observation_ids.clone(),
        posterior_by_observation,
    };
    rule.validate(assumed)?;
    Ok(rule)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationDecisionReceipt {
    pub observation_id: String,
    pub true_observation_probability: Rational,
    pub assumed_observation_probability: Rational,
    pub inference_decision: Option<usize>,
    pub assumed_model_bayes_decision: Option<usize>,
    /// The posterior actually supplied to the terminal decision rule.
    pub inference_posterior: Option<Vec<Rational>>,
    /// Exact posterior under the working channel.
    pub assumed_model_posterior: Option<Vec<Rational>>,
    /// Exact posterior under the true generating channel.
    pub true_posterior: Vec<Rational>,
    /// TV between approximate inference and exact working-model posterior.
    pub inference_posterior_tv_distance: Option<Rational>,
    /// TV between working-model and true posteriors (model misspecification).
    pub model_posterior_tv_distance: Option<Rational>,
    /// TV between actual inference posterior and true posterior.
    pub total_posterior_tv_distance: Option<Rational>,
    /// Squared-error score between actual inference posterior and true posterior.
    pub total_posterior_brier_score: Option<Rational>,
    pub true_bayes_decision: Option<usize>,
    pub inference_conditional_loss: Rational,
    pub realized_conditional_loss: Rational,
    pub true_bayes_conditional_loss: Rational,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChannelDecisionEvaluation {
    pub assumed_channel_id: String,
    pub true_channel_id: String,
    pub inference_rule_id: String,
    /// Bayes risk predicted under the working channel and inference rule's own observation law.
    pub nominal_model_expected_loss: Rational,
    /// Inference-rule conditional losses reweighted by true observation frequencies.
    pub expected_assumed_loss_under_true_observations: Rational,
    pub realized_policy_loss: Rational,
    pub true_bayes_oracle_loss: Rational,
    pub decision_regret: Rational,
    pub unsupported_observation_probability: Rational,
    /// Probability the inference rule's decision differs from the true Bayes decision.
    pub decision_disagreement_probability: Rational,
    /// Probability the inference rule differs from exact Bayes under the working channel.
    pub inference_rule_disagreement_probability: Rational,
    /// True-observation-frequency-weighted inference approximation error.
    pub expected_inference_posterior_tv_distance: Rational,
    /// True-observation-frequency-weighted posterior shift due to model error.
    pub expected_model_posterior_tv_distance: Rational,
    /// Total posterior error (inference + model) against the true posterior.
    pub expected_total_posterior_tv_distance: Rational,
    /// Proper squared-error/Brier diagnostic, separately weighted by true observations.
    pub expected_total_posterior_brier_score: Rational,
    pub observations: Vec<ObservationDecisionReceipt>,
}

fn validate_prior(prior: &[Rational], state_count: usize) -> Result<(), String> {
    if prior.len() != state_count || prior.is_empty() {
        return Err("prior dimension must equal the non-empty state vocabulary".into());
    }
    let mut total = Rational::zero();
    for probability in prior {
        if !probability.is_probability() {
            return Err("prior contains an invalid probability".into());
        }
        total = total.checked_add(*probability)?;
    }
    if total != Rational::one() {
        return Err("prior probability mass must sum exactly to one".into());
    }
    Ok(())
}

fn validate_channel_pair(
    assumed: &FiniteObservationChannel,
    truth: &FiniteObservationChannel,
    prior: &[Rational],
    table: &ChannelDecisionLossTable,
) -> Result<(), String> {
    assumed.validate()?;
    truth.validate()?;
    if assumed.state_ids != truth.state_ids {
        return Err(
            "assumed and true channels must share the same ordered state vocabulary".into(),
        );
    }
    if assumed.observation_ids != truth.observation_ids {
        return Err(
            "assumed and true channels must share the same observable symbol vocabulary".into(),
        );
    }
    validate_prior(prior, assumed.state_ids.len())?;
    table.validate(assumed.state_ids.len())
}

fn expected_loss_for_posterior(
    table: &ChannelDecisionLossTable,
    posterior: &[Rational],
    decision: Option<usize>,
) -> Result<Rational, String> {
    match decision {
        Some(index) if index < table.decision_ids.len() => {
            let mut expected = Rational::zero();
            for (probability, loss) in posterior.iter().zip(table.losses[index].iter()) {
                expected = expected.checked_add(probability.checked_mul(*loss)?)?;
            }
            Ok(expected)
        }
        Some(_) => Err("decision index is outside the loss table".into()),
        None => Ok(table.abstention_loss),
    }
}

fn posterior_tv_distance(left: &[Rational], right: &[Rational]) -> Result<Rational, String> {
    if left.len() != right.len() {
        return Err("posterior vectors must have the same dimension".into());
    }
    let mut l1_distance = Rational::zero();
    for (left_probability, right_probability) in left.iter().zip(right.iter()) {
        let difference = if left_probability >= right_probability {
            subtract_nonnegative(*left_probability, *right_probability)?
        } else {
            subtract_nonnegative(*right_probability, *left_probability)?
        };
        l1_distance = l1_distance.checked_add(difference)?;
    }
    l1_distance.checked_div(Rational::from_u64(2))
}

/// Expected multiclass Brier score for a predictive posterior under a
/// categorical truth distribution: E_{S~truth}[sum_j (prediction_j - 1[S=j])^2].
fn posterior_expected_brier_score(
    prediction: &[Rational],
    truth: &[Rational],
) -> Result<Rational, String> {
    if prediction.len() != truth.len() {
        return Err("posterior vectors must have the same dimension".into());
    }
    let mut prediction_squared_norm = Rational::zero();
    let mut probability_dot_product = Rational::zero();
    for (predicted_probability, true_probability) in prediction.iter().zip(truth.iter()) {
        prediction_squared_norm = prediction_squared_norm
            .checked_add(predicted_probability.checked_mul(*predicted_probability)?)?;
        probability_dot_product = probability_dot_product
            .checked_add(predicted_probability.checked_mul(*true_probability)?)?;
    }
    let twice_dot_product = probability_dot_product.checked_mul(Rational::from_u64(2))?;
    subtract_nonnegative(
        prediction_squared_norm.checked_add(Rational::one())?,
        twice_dot_product,
    )
}

fn bayes_decision(
    table: &ChannelDecisionLossTable,
    posterior: &[Rational],
) -> Result<(Option<usize>, Rational), String> {
    let mut selected_decision = None;
    let mut best_loss = table.abstention_loss;
    for (decision, _) in table.decision_ids.iter().enumerate() {
        let loss = expected_loss_for_posterior(table, posterior, Some(decision))?;
        if loss < best_loss {
            best_loss = loss;
            selected_decision = Some(decision);
        }
    }
    Ok((selected_decision, best_loss))
}

/// Evaluate an explicit inference rule while the assumed working channel
/// and the true observation-generating channel are held independently fixed.
///
/// This is the key two-axis interface:
/// - change `truth` while keeping `assumed` and `inference_rule` fixed to
///   isolate model misspecification;
/// - change only `inference_rule` while keeping `assumed == truth` to
///   isolate inferential misspecification;
/// - change both to measure their interaction.
///
/// Unsupported true observations absent from the working model cause explicit
/// abstention. All posterior and decision-risk quantities are exact rationals.
pub fn evaluate_channel_decisions_with_inference_rule(
    assumed: &FiniteObservationChannel,
    truth: &FiniteObservationChannel,
    prior: &[Rational],
    table: &ChannelDecisionLossTable,
    inference_rule: &InferencePosteriorRule,
) -> Result<ChannelDecisionEvaluation, String> {
    validate_channel_pair(assumed, truth, prior, table)?;
    inference_rule.validate(assumed)?;

    let mut nominal_model_expected_loss = Rational::zero();
    let mut expected_assumed_loss = Rational::zero();
    let mut realized_policy_loss = Rational::zero();
    let mut true_bayes_oracle_loss = Rational::zero();
    let mut unsupported_observation_probability = Rational::zero();
    let mut decision_disagreement_probability = Rational::zero();
    let mut inference_rule_disagreement_probability = Rational::zero();
    let mut expected_inference_posterior_tv_distance = Rational::zero();
    let mut expected_model_posterior_tv_distance = Rational::zero();
    let mut expected_total_posterior_tv_distance = Rational::zero();
    let mut expected_total_posterior_brier_score = Rational::zero();
    let mut observations = Vec::new();

    for (observation, observation_id) in assumed.observation_ids.iter().enumerate() {
        let mut true_probability = Rational::zero();
        let mut assumed_probability = Rational::zero();
        let mut true_joint = vec![Rational::zero(); prior.len()];
        let mut assumed_joint = vec![Rational::zero(); prior.len()];

        for (state, prior_mass) in prior.iter().copied().enumerate() {
            let true_mass = prior_mass.checked_mul(truth.probabilities[state][observation])?;
            let assumed_mass = prior_mass.checked_mul(assumed.probabilities[state][observation])?;
            true_joint[state] = true_mass;
            assumed_joint[state] = assumed_mass;
            true_probability = true_probability.checked_add(true_mass)?;
            assumed_probability = assumed_probability.checked_add(assumed_mass)?;
        }

        let (inference_decision, inference_conditional_loss) = if assumed_probability.is_zero() {
            (None, table.abstention_loss)
        } else {
            bayes_decision(table, &inference_rule.posterior_by_observation[observation])?
        };
        nominal_model_expected_loss = nominal_model_expected_loss
            .checked_add(assumed_probability.checked_mul(inference_conditional_loss)?)?;

        if true_probability.is_zero() {
            continue;
        }

        let true_posterior: Vec<Rational> = true_joint
            .iter()
            .map(|mass| mass.checked_div(true_probability))
            .collect::<Result<_, _>>()?;
        let (true_bayes_decision, true_bayes_loss) = bayes_decision(table, &true_posterior)?;

        let (
            assumed_model_bayes_decision,
            inference_posterior,
            assumed_model_posterior,
            inference_posterior_tv_distance,
            model_posterior_tv_distance,
            total_posterior_tv_distance,
            total_posterior_brier_score,
        ) = if assumed_probability.is_zero() {
            unsupported_observation_probability =
                unsupported_observation_probability.checked_add(true_probability)?;
            (None, None, None, None, None, None, None)
        } else {
            let assumed_posterior: Vec<Rational> = assumed_joint
                .iter()
                .map(|mass| mass.checked_div(assumed_probability))
                .collect::<Result<_, _>>()?;
            let actual_inference_posterior =
                inference_rule.posterior_by_observation[observation].clone();
            let (model_decision, _) = bayes_decision(table, &assumed_posterior)?;
            if model_decision != inference_decision {
                inference_rule_disagreement_probability =
                    inference_rule_disagreement_probability.checked_add(true_probability)?;
            }
            let inference_tv =
                posterior_tv_distance(&actual_inference_posterior, &assumed_posterior)?;
            let model_tv = posterior_tv_distance(&assumed_posterior, &true_posterior)?;
            let total_tv = posterior_tv_distance(&actual_inference_posterior, &true_posterior)?;
            let total_brier =
                posterior_expected_brier_score(&actual_inference_posterior, &true_posterior)?;

            expected_inference_posterior_tv_distance = expected_inference_posterior_tv_distance
                .checked_add(true_probability.checked_mul(inference_tv)?)?;
            expected_model_posterior_tv_distance = expected_model_posterior_tv_distance
                .checked_add(true_probability.checked_mul(model_tv)?)?;
            expected_total_posterior_tv_distance = expected_total_posterior_tv_distance
                .checked_add(true_probability.checked_mul(total_tv)?)?;
            expected_total_posterior_brier_score = expected_total_posterior_brier_score
                .checked_add(true_probability.checked_mul(total_brier)?)?;

            (
                model_decision,
                Some(actual_inference_posterior),
                Some(assumed_posterior),
                Some(inference_tv),
                Some(model_tv),
                Some(total_tv),
                Some(total_brier),
            )
        };
        let realized_conditional_loss =
            expected_loss_for_posterior(table, &true_posterior, inference_decision)?;
        if inference_decision != true_bayes_decision {
            decision_disagreement_probability =
                decision_disagreement_probability.checked_add(true_probability)?;
        }

        expected_assumed_loss = expected_assumed_loss
            .checked_add(true_probability.checked_mul(inference_conditional_loss)?)?;
        realized_policy_loss = realized_policy_loss
            .checked_add(true_probability.checked_mul(realized_conditional_loss)?)?;
        true_bayes_oracle_loss =
            true_bayes_oracle_loss.checked_add(true_probability.checked_mul(true_bayes_loss)?)?;

        observations.push(ObservationDecisionReceipt {
            observation_id: observation_id.clone(),
            true_observation_probability: true_probability,
            assumed_observation_probability: assumed_probability,
            inference_decision,
            assumed_model_bayes_decision,
            inference_posterior,
            assumed_model_posterior,
            true_posterior: true_posterior.clone(),
            inference_posterior_tv_distance,
            model_posterior_tv_distance,
            total_posterior_tv_distance,
            total_posterior_brier_score,
            true_bayes_decision,
            inference_conditional_loss,
            realized_conditional_loss,
            true_bayes_conditional_loss: true_bayes_loss,
        });
    }

    let decision_regret = if realized_policy_loss >= true_bayes_oracle_loss {
        subtract_nonnegative(realized_policy_loss, true_bayes_oracle_loss)?
    } else {
        return Err("realized policy loss fell below the exact true-channel Bayes oracle".into());
    };

    Ok(ChannelDecisionEvaluation {
        assumed_channel_id: assumed.channel_id.clone(),
        true_channel_id: truth.channel_id.clone(),
        inference_rule_id: inference_rule.rule_id.clone(),
        nominal_model_expected_loss,
        expected_assumed_loss_under_true_observations: expected_assumed_loss,
        realized_policy_loss,
        true_bayes_oracle_loss,
        decision_regret,
        unsupported_observation_probability,
        decision_disagreement_probability,
        inference_rule_disagreement_probability,
        expected_inference_posterior_tv_distance,
        expected_model_posterior_tv_distance,
        expected_total_posterior_tv_distance,
        expected_total_posterior_brier_score,
        observations,
    })
}

/// Evaluate exact Bayesian inference under the assumed channel. This wrapper
/// is equivalent to constructing `exact_bayes_posterior_rule` and passing it
/// to `evaluate_channel_decisions_with_inference_rule`.
pub fn evaluate_channel_decisions(
    assumed: &FiniteObservationChannel,
    truth: &FiniteObservationChannel,
    prior: &[Rational],
    table: &ChannelDecisionLossTable,
) -> Result<ChannelDecisionEvaluation, String> {
    let inference_rule = exact_bayes_posterior_rule(assumed, prior)?;
    evaluate_channel_decisions_with_inference_rule(assumed, truth, prior, table, &inference_rule)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExperimentPreference {
    A,
    B,
    Tie,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExperimentParetoRelation {
    ADominatesB,
    BDominatesA,
    Tradeoff,
    Equivalent,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExperimentPairComparison {
    pub experiment_a: ChannelDecisionEvaluation,
    pub experiment_b: ChannelDecisionEvaluation,
    pub experiment_cost_a: Rational,
    pub experiment_cost_b: Rational,
    /// Multiplier converting one cost unit into terminal-decision-loss units.
    pub cost_to_loss_exchange_rate: Rational,
    /// Componentwise comparison of (cost, nominal decision risk), without scalarization.
    pub nominal_pareto_relation: ExperimentParetoRelation,
    /// Componentwise comparison of (cost, realized decision risk), without scalarization.
    pub realized_pareto_relation: ExperimentParetoRelation,
    pub nominal_cost_to_loss_crossover_rate: Option<Rational>,
    pub realized_cost_to_loss_crossover_rate: Option<Rational>,
    pub mutual_information_a_bits: f64,
    pub mutual_information_b_bits: f64,
    pub information_preference: ExperimentPreference,
    /// Exact nominal Blackwell certificate search from A to B. An inner None
    /// means no dominance witness was found; Err denotes the bounded solver's
    /// unsupported-size/arithmetic boundary.
    pub blackwell_a_dominates_b: Result<Option<BlackwellDominanceReceipt>, String>,
    /// Reverse nominal Blackwell certificate search from B to A.
    pub blackwell_b_dominates_a: Result<Option<BlackwellDominanceReceipt>, String>,
    pub nominal_total_loss_a: Rational,
    pub nominal_total_loss_b: Rational,
    pub nominal_total_loss_preference: ExperimentPreference,
    pub realized_total_loss_a: Rational,
    pub realized_total_loss_b: Rational,
    pub realized_total_loss_preference: ExperimentPreference,
    /// True when the nominal information ranking and realized cost-plus-loss
    /// ranking strictly prefer opposite experiments.
    pub information_loss_ranking_reversal: bool,
}

fn cost_to_loss_crossover_rate(
    cost_a: Rational,
    loss_a: Rational,
    cost_b: Rational,
    loss_b: Rational,
) -> Result<Option<Rational>, String> {
    if cost_a > cost_b && loss_a < loss_b {
        let loss_advantage_a = subtract_nonnegative(loss_b, loss_a)?;
        let extra_cost_a = subtract_nonnegative(cost_a, cost_b)?;
        return Ok(Some(loss_advantage_a.checked_div(extra_cost_a)?));
    }
    if cost_b > cost_a && loss_b < loss_a {
        let loss_advantage_b = subtract_nonnegative(loss_a, loss_b)?;
        let extra_cost_b = subtract_nonnegative(cost_b, cost_a)?;
        return Ok(Some(loss_advantage_b.checked_div(extra_cost_b)?));
    }
    Ok(None)
}

fn pareto_relation(
    cost_a: Rational,
    decision_loss_a: Rational,
    cost_b: Rational,
    decision_loss_b: Rational,
) -> ExperimentParetoRelation {
    let a_no_worse = cost_a <= cost_b && decision_loss_a <= decision_loss_b;
    let b_no_worse = cost_b <= cost_a && decision_loss_b <= decision_loss_a;
    let a_strict = cost_a < cost_b || decision_loss_a < decision_loss_b;
    let b_strict = cost_b < cost_a || decision_loss_b < decision_loss_a;

    match (
        a_no_worse && a_strict,
        b_no_worse && b_strict,
        a_no_worse,
        b_no_worse,
    ) {
        (true, false, _, _) => ExperimentParetoRelation::ADominatesB,
        (false, true, _, _) => ExperimentParetoRelation::BDominatesA,
        (_, _, true, true) => ExperimentParetoRelation::Equivalent,
        _ => ExperimentParetoRelation::Tradeoff,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExperimentParetoCandidate {
    pub experiment_id: String,
    pub experiment_cost: Rational,
    pub nominal_decision_loss: Rational,
    pub realized_decision_loss: Rational,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExperimentParetoFrontier {
    /// Stable-ID-sorted nondominated candidates using (cost, nominal loss).
    pub nominal_frontier_ids: Vec<String>,
    pub nominal_dominated_ids: Vec<String>,
    /// Stable-ID-sorted nondominated candidates using (cost, realized loss).
    pub realized_frontier_ids: Vec<String>,
    pub realized_dominated_ids: Vec<String>,
}

const MAX_PARETO_EXPERIMENTS: usize = 4096;

fn candidate_is_dominated(
    candidate: &ExperimentParetoCandidate,
    candidates: &[ExperimentParetoCandidate],
    realized: bool,
) -> bool {
    let candidate_loss = if realized {
        candidate.realized_decision_loss
    } else {
        candidate.nominal_decision_loss
    };
    candidates.iter().any(|other| {
        if other.experiment_id == candidate.experiment_id {
            return false;
        }
        let other_loss = if realized {
            other.realized_decision_loss
        } else {
            other.nominal_decision_loss
        };
        other.experiment_cost <= candidate.experiment_cost
            && other_loss <= candidate_loss
            && (other.experiment_cost < candidate.experiment_cost || other_loss < candidate_loss)
    })
}

/// Extract nominal and realized two-objective Pareto frontiers without
/// scalarizing experiment cost and decision loss.
///
/// Equivalent designs are all retained on the frontier; a design is removed
/// only when another is no worse on both objectives and strictly better on at
/// least one. Inputs and outputs are stable-ID sorted. This is quadratic in the
/// number of candidates and therefore bounded by MAX_PARETO_EXPERIMENTS.
pub fn compute_experiment_pareto_frontier(
    candidates: &[ExperimentParetoCandidate],
) -> Result<ExperimentParetoFrontier, String> {
    if candidates.is_empty() || candidates.len() > MAX_PARETO_EXPERIMENTS {
        return Err(format!(
            "Pareto candidate count must be in 1..={MAX_PARETO_EXPERIMENTS}"
        ));
    }

    let mut canonical = candidates.to_vec();
    canonical.sort_by(|left, right| left.experiment_id.cmp(&right.experiment_id));
    let mut seen = BTreeSet::new();
    for candidate in &canonical {
        if candidate.experiment_id.trim().is_empty()
            || !seen.insert(candidate.experiment_id.as_str())
        {
            return Err("experiment IDs must be non-empty and unique".into());
        }
    }

    let mut nominal_frontier_ids = Vec::new();
    let mut nominal_dominated_ids = Vec::new();
    let mut realized_frontier_ids = Vec::new();
    let mut realized_dominated_ids = Vec::new();

    for candidate in &canonical {
        if candidate_is_dominated(candidate, &canonical, false) {
            nominal_dominated_ids.push(candidate.experiment_id.clone());
        } else {
            nominal_frontier_ids.push(candidate.experiment_id.clone());
        }
        if candidate_is_dominated(candidate, &canonical, true) {
            realized_dominated_ids.push(candidate.experiment_id.clone());
        } else {
            realized_frontier_ids.push(candidate.experiment_id.clone());
        }
    }

    Ok(ExperimentParetoFrontier {
        nominal_frontier_ids,
        nominal_dominated_ids,
        realized_frontier_ids,
        realized_dominated_ids,
    })
}

fn lower_loss_preference(loss_a: Rational, loss_b: Rational) -> ExperimentPreference {
    if loss_a < loss_b {
        ExperimentPreference::A
    } else if loss_b < loss_a {
        ExperimentPreference::B
    } else {
        ExperimentPreference::Tie
    }
}

/// Compare two experiments under an explicit conversion from cost units to
/// terminal-decision-loss units. Cost and decision loss are also retained as
/// separate Pareto objectives, so the caller can inspect trade-offs without
/// relying on any scalarization coefficient.
#[allow(clippy::too_many_arguments)]
pub fn compare_experiment_pair_with_inference_rules_and_cost_rate(
    assumed_a: &FiniteObservationChannel,
    truth_a: &FiniteObservationChannel,
    inference_rule_a: &InferencePosteriorRule,
    cost_a: Rational,
    assumed_b: &FiniteObservationChannel,
    truth_b: &FiniteObservationChannel,
    inference_rule_b: &InferencePosteriorRule,
    cost_b: Rational,
    cost_to_loss_exchange_rate: Rational,
    prior: &[Rational],
    table: &ChannelDecisionLossTable,
) -> Result<ExperimentPairComparison, String> {
    let evaluation_a = evaluate_channel_decisions_with_inference_rule(
        assumed_a,
        truth_a,
        prior,
        table,
        inference_rule_a,
    )?;
    let evaluation_b = evaluate_channel_decisions_with_inference_rule(
        assumed_b,
        truth_b,
        prior,
        table,
        inference_rule_b,
    )?;
    let information_a = mutual_information_bits(assumed_a, prior)?;
    let information_b = mutual_information_bits(assumed_b, prior)?;
    let blackwell_a_dominates_b = solve_small_blackwell_garbling(assumed_a, assumed_b);
    let blackwell_b_dominates_a = solve_small_blackwell_garbling(assumed_b, assumed_a);

    let information_preference = if information_a > information_b {
        ExperimentPreference::A
    } else if information_b > information_a {
        ExperimentPreference::B
    } else {
        ExperimentPreference::Tie
    };
    let weighted_cost_a = cost_a.checked_mul(cost_to_loss_exchange_rate)?;
    let weighted_cost_b = cost_b.checked_mul(cost_to_loss_exchange_rate)?;
    let nominal_total_loss_a =
        weighted_cost_a.checked_add(evaluation_a.nominal_model_expected_loss)?;
    let nominal_total_loss_b =
        weighted_cost_b.checked_add(evaluation_b.nominal_model_expected_loss)?;
    let realized_total_loss_a = weighted_cost_a.checked_add(evaluation_a.realized_policy_loss)?;
    let realized_total_loss_b = weighted_cost_b.checked_add(evaluation_b.realized_policy_loss)?;
    let nominal_pareto_relation = pareto_relation(
        cost_a,
        evaluation_a.nominal_model_expected_loss,
        cost_b,
        evaluation_b.nominal_model_expected_loss,
    );
    let realized_pareto_relation = pareto_relation(
        cost_a,
        evaluation_a.realized_policy_loss,
        cost_b,
        evaluation_b.realized_policy_loss,
    );
    let nominal_cost_to_loss_crossover_rate = cost_to_loss_crossover_rate(
        cost_a,
        evaluation_a.nominal_model_expected_loss,
        cost_b,
        evaluation_b.nominal_model_expected_loss,
    )?;
    let realized_cost_to_loss_crossover_rate = cost_to_loss_crossover_rate(
        cost_a,
        evaluation_a.realized_policy_loss,
        cost_b,
        evaluation_b.realized_policy_loss,
    )?;
    let nominal_total_loss_preference =
        lower_loss_preference(nominal_total_loss_a, nominal_total_loss_b);
    let realized_total_loss_preference =
        lower_loss_preference(realized_total_loss_a, realized_total_loss_b);
    let information_loss_ranking_reversal = matches!(
        (information_preference, realized_total_loss_preference),
        (ExperimentPreference::A, ExperimentPreference::B)
            | (ExperimentPreference::B, ExperimentPreference::A)
    );

    Ok(ExperimentPairComparison {
        experiment_a: evaluation_a,
        experiment_b: evaluation_b,
        experiment_cost_a: cost_a,
        experiment_cost_b: cost_b,
        cost_to_loss_exchange_rate,
        nominal_pareto_relation,
        realized_pareto_relation,
        nominal_cost_to_loss_crossover_rate,
        realized_cost_to_loss_crossover_rate,
        mutual_information_a_bits: information_a,
        mutual_information_b_bits: information_b,
        information_preference,
        blackwell_a_dominates_b,
        blackwell_b_dominates_a,
        nominal_total_loss_a,
        nominal_total_loss_b,
        nominal_total_loss_preference,
        realized_total_loss_a,
        realized_total_loss_b,
        realized_total_loss_preference,
        information_loss_ranking_reversal,
    })
}

/// Compare two experiments with explicit inference rules and the default
/// one-to-one cost-to-loss exchange rate. Use
/// `compare_experiment_pair_with_inference_rules_and_cost_rate` when the
/// currency/utility conversion differs from one.
#[allow(clippy::too_many_arguments)]
pub fn compare_experiment_pair_with_inference_rules(
    assumed_a: &FiniteObservationChannel,
    truth_a: &FiniteObservationChannel,
    inference_rule_a: &InferencePosteriorRule,
    cost_a: Rational,
    assumed_b: &FiniteObservationChannel,
    truth_b: &FiniteObservationChannel,
    inference_rule_b: &InferencePosteriorRule,
    cost_b: Rational,
    prior: &[Rational],
    table: &ChannelDecisionLossTable,
) -> Result<ExperimentPairComparison, String> {
    compare_experiment_pair_with_inference_rules_and_cost_rate(
        assumed_a,
        truth_a,
        inference_rule_a,
        cost_a,
        assumed_b,
        truth_b,
        inference_rule_b,
        cost_b,
        Rational::one(),
        prior,
        table,
    )
}

/// Compare two experiments using exact Bayesian inference under each
/// experiment's own working channel. Use
/// `compare_experiment_pair_with_inference_rules` to supply explicit
/// approximate/alternative posterior procedures.
#[allow(clippy::too_many_arguments)]
pub fn compare_experiment_pair(
    assumed_a: &FiniteObservationChannel,
    truth_a: &FiniteObservationChannel,
    cost_a: Rational,
    assumed_b: &FiniteObservationChannel,
    truth_b: &FiniteObservationChannel,
    cost_b: Rational,
    prior: &[Rational],
    table: &ChannelDecisionLossTable,
) -> Result<ExperimentPairComparison, String> {
    let inference_rule_a = exact_bayes_posterior_rule(assumed_a, prior)?;
    let inference_rule_b = exact_bayes_posterior_rule(assumed_b, prior)?;
    compare_experiment_pair_with_inference_rules(
        assumed_a,
        truth_a,
        &inference_rule_a,
        cost_a,
        assumed_b,
        truth_b,
        &inference_rule_b,
        cost_b,
        prior,
        table,
    )
}

#[derive(Clone, Debug, PartialEq)]
pub struct GibbsEigExactReceipt {
    pub rule_id: String,
    /// C(xi) = sum_y sum_theta prior(theta) * exp(-eta * loss(theta,xi,y)).
    pub pseudo_predictive_normalizer: Rational,
    /// Unnormalized pseudo-predictive mass for each observation.
    pub pseudo_predictive_mass_by_observation: Vec<Rational>,
    /// Normalized pseudo-predictive distribution over observations.
    pub normalized_pseudo_predictive: Vec<Rational>,
    pub posterior_rule: InferencePosteriorRule,
    /// KL(Gibbs posterior || prior), by observation, in nats.
    pub conditional_kl_nats: Vec<f64>,
    /// Pseudo-predictive-weighted KL contribution, by observation, in nats.
    pub contribution_nats: Vec<f64>,
    /// Exact finite-discrete Gibbs EIG oracle, reported in nats.
    pub value_nats: f64,
}

/// Compute the exact finite-discrete Gibbs EIG by exhaustive summation.
///
/// GEIG(xi) = sum_y [m(y)/C] KL(q_Gibbs(theta|y) || prior(theta)),
/// where m(y) = sum_theta prior(theta) * factor(theta,y) and
/// C = sum_y m(y). The finite rational masses and posterior are exact; only
/// logarithms and the reported information value are floating-point.
///
/// This is an exact oracle for small finite problems, not a Monte Carlo
/// approximation. The corresponding importance-sampling estimator is below.
pub fn compute_gibbs_eig_exact_finite(
    rule_id: &str,
    assumed: &FiniteObservationChannel,
    prior: &[Rational],
    gibbs_factors: &[Vec<Rational>],
) -> Result<GibbsEigExactReceipt, String> {
    let posterior_rule =
        generalized_bayes_posterior_rule_from_weights(rule_id, assumed, prior, gibbs_factors)?;

    let mut pseudo_predictive_mass_by_observation =
        Vec::with_capacity(assumed.observation_ids.len());
    let mut normalizer = Rational::zero();

    for (observation, factors) in gibbs_factors.iter().enumerate() {
        let mut marginal_mass = Rational::zero();
        for (prior_mass, factor) in prior.iter().copied().zip(factors.iter().copied()) {
            marginal_mass = marginal_mass.checked_add(prior_mass.checked_mul(factor)?)?;
        }
        if marginal_mass.is_zero() {
            return Err(format!(
                "Gibbs EIG observation {observation} has zero pseudo-predictive mass"
            ));
        }
        normalizer = normalizer.checked_add(marginal_mass)?;
        pseudo_predictive_mass_by_observation.push(marginal_mass);
    }
    if normalizer.is_zero() {
        return Err("Gibbs EIG has a zero pseudo-predictive normalizer".into());
    }

    let normalized_pseudo_predictive = pseudo_predictive_mass_by_observation
        .iter()
        .map(|mass| mass.checked_div(normalizer))
        .collect::<Result<Vec<_>, _>>()?;

    let mut conditional_kl_nats = Vec::with_capacity(assumed.observation_ids.len());
    let mut contribution_nats = Vec::with_capacity(assumed.observation_ids.len());
    let mut total = 0.0_f64;
    let mut compensation = 0.0_f64;

    for (observation, posterior) in posterior_rule.posterior_by_observation.iter().enumerate() {
        let mut kl = 0.0_f64;
        let mut kl_compensation = 0.0_f64;
        for (gibbs_probability, prior_probability) in
            posterior.iter().copied().zip(prior.iter().copied())
        {
            if gibbs_probability.is_zero() {
                continue;
            }
            if prior_probability.is_zero() {
                return Err("Gibbs posterior assigned positive mass outside prior support".into());
            }
            let ratio_log =
                rational_to_f64(gibbs_probability).ln() - rational_to_f64(prior_probability).ln();
            let term = rational_to_f64(gibbs_probability) * ratio_log;
            kahan_add(&mut kl, &mut kl_compensation, term);
        }
        if kl < -1.0e-12 {
            return Err(format!(
                "computed Gibbs posterior KL was materially negative for observation {observation}"
            ));
        }
        let kl = kl.max(0.0);
        let predictive_probability = rational_to_f64(normalized_pseudo_predictive[observation]);
        let contribution = predictive_probability * kl;
        conditional_kl_nats.push(kl);
        contribution_nats.push(contribution);
        kahan_add(&mut total, &mut compensation, contribution);
    }

    if !total.is_finite() || total < -1.0e-12 {
        return Err("finite Gibbs EIG computation was non-finite or negative".into());
    }

    Ok(GibbsEigExactReceipt {
        rule_id: rule_id.to_string(),
        pseudo_predictive_normalizer: normalizer,
        pseudo_predictive_mass_by_observation,
        normalized_pseudo_predictive,
        posterior_rule,
        conditional_kl_nats,
        contribution_nats,
        value_nats: total.max(0.0),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GibbsImportanceSample {
    pub state_index: usize,
    pub observation_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GibbsEigImportanceReceipt {
    pub proposal_id: String,
    pub sample_count: usize,
    /// Exact importance weights factor(theta,y) / q(y|theta) for every sample.
    pub unnormalized_importance_weights: Vec<Rational>,
    /// Exact normalized self-normalized importance weights.
    pub normalized_importance_weights: Vec<Rational>,
    /// Exact Kish ESS = (sum w)^2 / sum(w^2).
    pub effective_sample_size: Rational,
    pub effective_sample_size_fraction: f64,
    pub maximum_normalized_weight: Rational,
    pub estimated_gibbs_eig_nats: f64,
    pub exact_oracle_gibbs_eig_nats: f64,
    pub absolute_error_nats: f64,
}

/// Estimate finite-discrete Gibbs EIG using user-supplied draws from
/// theta~prior and y~q(.|theta), with exact finite-model pseudo-predictive
/// marginals and a self-normalized importance correction.
///
/// The per-sample importance weight is factor(theta,y)/q(y|theta). The
/// diagnostic returns exact weights and Kish ESS, plus the Monte Carlo
/// estimate and its discrepancy from the exact finite-support oracle. It
/// validates proposal support, but it cannot verify that the caller actually
/// generated samples from the declared prior/proposal distribution. Sample
/// seeds and sampler version must therefore be recorded by the caller.
///
/// This separates proposal/ESS error from the nested Monte Carlo error in
/// approximating m(y); here m(y) is evaluated exactly because the model is
/// finite. It is a controlled discrete diagnostic, not a substitute for the
/// paper's continuous-model nested Monte Carlo implementation.
pub fn estimate_gibbs_eig_importance_from_samples(
    proposal_id: &str,
    assumed: &FiniteObservationChannel,
    prior: &[Rational],
    gibbs_factors: &[Vec<Rational>],
    proposal_probabilities: &[Vec<Rational>],
    samples: &[GibbsImportanceSample],
) -> Result<GibbsEigImportanceReceipt, String> {
    if proposal_id.trim().is_empty() {
        return Err("importance proposal ID must be non-empty".into());
    }
    assumed.validate()?;
    validate_prior(prior, assumed.state_ids.len())?;
    if samples.is_empty() {
        return Err("Gibbs EIG importance estimate requires at least one sample".into());
    }
    if gibbs_factors.len() != assumed.observation_ids.len()
        || proposal_probabilities.len() != assumed.state_ids.len()
    {
        return Err("Gibbs factors/proposal dimensions do not match the channel".into());
    }
    for (observation, factors) in gibbs_factors.iter().enumerate() {
        if factors.len() != assumed.state_ids.len() {
            return Err(format!(
                "Gibbs factor row {observation} has the wrong state dimension"
            ));
        }
    }
    for (state, proposal_row) in proposal_probabilities.iter().enumerate() {
        if proposal_row.len() != assumed.observation_ids.len() {
            return Err(format!(
                "proposal row {state} has the wrong observation dimension"
            ));
        }
        let mut row_mass = Rational::zero();
        for probability in proposal_row {
            if !probability.is_probability() {
                return Err(format!("proposal row {state} has an invalid probability"));
            }
            row_mass = row_mass.checked_add(*probability)?;
        }
        if row_mass != Rational::one() {
            return Err(format!("proposal row {state} must sum exactly to one"));
        }
    }

    for (state, prior_mass) in prior.iter().copied().enumerate() {
        if prior_mass.is_zero() {
            continue;
        }
        for observation in 0..assumed.observation_ids.len() {
            if !gibbs_factors[observation][state].is_zero()
                && proposal_probabilities[state][observation].is_zero()
            {
                return Err(format!(
                    "proposal has zero support for positive Gibbs mass at state {state}, observation {observation}"
                ));
            }
        }
    }

    let exact = compute_gibbs_eig_exact_finite(
        &format!("importance-oracle:{proposal_id}"),
        assumed,
        prior,
        gibbs_factors,
    )?;

    let mut importance_weights = Vec::with_capacity(samples.len());
    let mut weight_sum = Rational::zero();
    let mut squared_weight_sum = Rational::zero();
    let mut weighted_utility = 0.0_f64;
    let mut utility_compensation = 0.0_f64;

    for sample in samples {
        if sample.state_index >= assumed.state_ids.len()
            || sample.observation_index >= assumed.observation_ids.len()
        {
            return Err("importance sample index is outside the channel vocabulary".into());
        }
        if prior[sample.state_index].is_zero() {
            return Err("importance sample lies outside prior support".into());
        }
        let proposal_probability =
            proposal_probabilities[sample.state_index][sample.observation_index];
        if proposal_probability.is_zero() {
            return Err("importance sample lies outside proposal support".into());
        }
        let gibbs_factor = gibbs_factors[sample.observation_index][sample.state_index];
        let weight = gibbs_factor.checked_div(proposal_probability)?;
        weight_sum = weight_sum.checked_add(weight)?;
        squared_weight_sum = squared_weight_sum.checked_add(weight.checked_mul(weight)?)?;
        importance_weights.push(weight);

        if weight.is_zero() {
            continue;
        }
        let marginal_mass = exact.pseudo_predictive_mass_by_observation[sample.observation_index];
        let log_ratio = rational_to_f64(gibbs_factor).ln() - rational_to_f64(marginal_mass).ln();
        let weighted_term = rational_to_f64(weight) * log_ratio;
        kahan_add(
            &mut weighted_utility,
            &mut utility_compensation,
            weighted_term,
        );
    }

    if weight_sum.is_zero() || squared_weight_sum.is_zero() {
        return Err("all sampled Gibbs importance weights are zero".into());
    }
    let effective_sample_size = weight_sum
        .checked_mul(weight_sum)?
        .checked_div(squared_weight_sum)?;
    let sample_count = Rational::from_u64(samples.len() as u64);
    let effective_sample_size_fraction =
        rational_to_f64(effective_sample_size) / rational_to_f64(sample_count);
    let normalized_importance_weights = importance_weights
        .iter()
        .map(|weight| weight.checked_div(weight_sum))
        .collect::<Result<Vec<_>, _>>()?;
    let maximum_normalized_weight = normalized_importance_weights
        .iter()
        .copied()
        .max()
        .unwrap_or_else(Rational::zero);
    let estimated = weighted_utility / rational_to_f64(weight_sum);
    if !estimated.is_finite() {
        return Err("importance-sampled Gibbs EIG was non-finite".into());
    }

    Ok(GibbsEigImportanceReceipt {
        proposal_id: proposal_id.to_string(),
        sample_count: samples.len(),
        unnormalized_importance_weights: importance_weights,
        normalized_importance_weights,
        effective_sample_size,
        effective_sample_size_fraction,
        maximum_normalized_weight,
        estimated_gibbs_eig_nats: estimated,
        exact_oracle_gibbs_eig_nats: exact.value_nats,
        absolute_error_nats: (estimated - exact.value_nats).abs(),
    })
}

fn rational_to_f64(value: Rational) -> f64 {
    value.numerator() as f64 / value.denominator() as f64
}

fn kahan_add(sum: &mut f64, compensation: &mut f64, value: f64) {
    let adjusted = value - *compensation;
    let next = *sum + adjusted;
    *compensation = (next - *sum) - adjusted;
    *sum = next;
}

/// Mutual information in bits under the supplied nominal prior/channel.
///
/// This uses floating-point logarithms and is a diagnostic comparator only;
/// all decision and risk quantities above remain exact.
pub fn mutual_information_bits(
    channel: &FiniteObservationChannel,
    prior: &[Rational],
) -> Result<f64, String> {
    channel.validate()?;
    validate_prior(prior, channel.state_ids.len())?;

    let mut observation_marginal = vec![0.0_f64; channel.observation_ids.len()];
    for (state, prior_probability) in prior.iter().copied().enumerate() {
        let prior_mass =
            prior_probability.numerator() as f64 / prior_probability.denominator() as f64;
        for (observation, conditional_probability) in
            channel.probabilities[state].iter().copied().enumerate()
        {
            let channel_probability = conditional_probability.numerator() as f64
                / conditional_probability.denominator() as f64;
            observation_marginal[observation] += prior_mass * channel_probability;
        }
    }

    let mut information = 0.0_f64;
    for (state, prior_probability) in prior.iter().copied().enumerate() {
        let prior_mass =
            prior_probability.numerator() as f64 / prior_probability.denominator() as f64;
        if prior_mass == 0.0 {
            continue;
        }
        for (observation, conditional_probability) in
            channel.probabilities[state].iter().copied().enumerate()
        {
            let conditional = conditional_probability.numerator() as f64
                / conditional_probability.denominator() as f64;
            let marginal = observation_marginal[observation];
            if conditional > 0.0 && marginal > 0.0 {
                information += prior_mass * conditional * (conditional / marginal).log2();
            }
        }
    }
    if information.is_finite() {
        Ok(information)
    } else {
        Err("mutual-information diagnostic was non-finite".into())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SignedRational {
    negative: bool,
    magnitude: Rational,
}

impl SignedRational {
    fn from_parts(negative: bool, magnitude: Rational) -> Self {
        Self {
            negative: negative && !magnitude.is_zero(),
            magnitude,
        }
    }

    fn from_rational(value: Rational) -> Self {
        Self::from_parts(false, value)
    }

    fn from_difference(left: Rational, right: Rational) -> Result<Self, String> {
        if left >= right {
            Ok(Self::from_parts(false, subtract_nonnegative(left, right)?))
        } else {
            Ok(Self::from_parts(true, subtract_nonnegative(right, left)?))
        }
    }

    fn negated(self) -> Self {
        if self.magnitude.is_zero() {
            self
        } else {
            Self {
                negative: !self.negative,
                magnitude: self.magnitude,
            }
        }
    }

    fn checked_add(self, other: Self) -> Result<Self, String> {
        if self.negative == other.negative {
            Ok(Self::from_parts(
                self.negative,
                self.magnitude.checked_add(other.magnitude)?,
            ))
        } else if self.magnitude >= other.magnitude {
            Ok(Self::from_parts(
                self.negative,
                subtract_nonnegative(self.magnitude, other.magnitude)?,
            ))
        } else {
            Ok(Self::from_parts(
                other.negative,
                subtract_nonnegative(other.magnitude, self.magnitude)?,
            ))
        }
    }

    fn checked_mul(self, other: Self) -> Result<Self, String> {
        Ok(Self::from_parts(
            self.negative ^ other.negative,
            self.magnitude.checked_mul(other.magnitude)?,
        ))
    }

    fn checked_div(self, other: Self) -> Result<Self, String> {
        if other.magnitude.is_zero() {
            return Err("signed rational division by zero".into());
        }
        Ok(Self::from_parts(
            self.negative ^ other.negative,
            self.magnitude.checked_div(other.magnitude)?,
        ))
    }

    fn as_probability(self) -> Option<Rational> {
        if self.negative || !self.magnitude.is_probability() {
            None
        } else {
            Some(self.magnitude)
        }
    }
}

/// Solve Blackwell garbling feasibility exactly for two-state, two-observation
/// channels. The general verifier remains authoritative and checks the
/// constructed witness before it is returned.
///
/// Let A's probability of observation 0 in the two states be a and c, and
/// let B's corresponding probabilities be b and d. A binary garbling is
/// determined by x=P(output0 | A-observation0) and
/// y=P(output0 | A-observation1). This routine solves the resulting two
/// linear equations with signed exact rationals and returns None when the
/// unique solution is outside [0,1] or the singular equations are inconsistent.
pub fn solve_binary_blackwell_garbling(
    more_informative: &FiniteObservationChannel,
    less_informative: &FiniteObservationChannel,
) -> Result<Option<BlackwellDominanceReceipt>, String> {
    more_informative.validate()?;
    less_informative.validate()?;
    if more_informative.state_ids != less_informative.state_ids {
        return Err("Blackwell channels must share the same ordered state vocabulary".into());
    }
    if more_informative.state_ids.len() != 2
        || more_informative.observation_ids.len() != 2
        || less_informative.observation_ids.len() != 2
    {
        return Err(
            "binary Blackwell solver requires exactly two states and two observations per channel"
                .into(),
        );
    }

    let a = more_informative.probabilities[0][0];
    let c = more_informative.probabilities[1][0];
    let b = less_informative.probabilities[0][0];
    let d = less_informative.probabilities[1][0];

    let (x, y) = if a == c {
        if b != d {
            return Ok(None);
        }
        (b, b)
    } else {
        let target_difference = SignedRational::from_difference(b, d)?;
        let source_difference = SignedRational::from_difference(a, c)?;
        let delta = target_difference.checked_div(source_difference)?;
        let a_delta = SignedRational::from_rational(a).checked_mul(delta)?;
        let y_signed = SignedRational::from_rational(b).checked_add(a_delta.negated())?;
        let x_signed = y_signed.checked_add(delta)?;
        let (Some(x), Some(y)) = (x_signed.as_probability(), y_signed.as_probability()) else {
            return Ok(None);
        };
        (x, y)
    };

    let witness = BlackwellGarblingWitness {
        probabilities: vec![
            vec![x, subtract_nonnegative(Rational::one(), x)?],
            vec![y, subtract_nonnegative(Rational::one(), y)?],
        ],
    };
    let receipt = verify_blackwell_dominance(more_informative, less_informative, &witness)?;
    Ok(Some(receipt))
}

impl SignedRational {
    fn is_zero(self) -> bool {
        self.magnitude.is_zero()
    }

    fn is_negative(self) -> bool {
        self.negative && !self.magnitude.is_zero()
    }

    fn checked_sub(self, other: Self) -> Result<Self, String> {
        self.checked_add(other.negated())
    }

    fn absolute_magnitude(self) -> Rational {
        self.magnitude
    }
}

impl Ord for SignedRational {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => self.magnitude.cmp(&other.magnitude),
            (true, true) => other.magnitude.cmp(&self.magnitude),
        }
    }
}

impl PartialOrd for SignedRational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LinearInequality {
    coefficients: Vec<SignedRational>,
    upper_bound: SignedRational,
}

const MAX_SMALL_BLACKWELL_VARIABLES: usize = 12;
const MAX_SMALL_BLACKWELL_CONSTRAINTS: usize = 25_000;
const MAX_SMALL_BLACKWELL_PAIR_PRODUCTS: usize = 500_000;

/// Solve the Blackwell garbling feasibility problem for bounded small channel
/// dimensions using exact-rational Fourier-Motzkin elimination.
///
/// A garbling has one stochastic row for every observation of the more
/// informative channel. The final output coordinate is eliminated using the
/// row-sum constraint, leaving m*(n-1) variables in [0,1] subject to exact
/// linear equalities and simplex inequalities. Feasibility is decided by
/// elimination; a witness is reconstructed by exact reverse substitution and
/// then checked again with verify_blackwell_dominance.
///
/// The solver deliberately has hard variable, constraint and pair-product
/// caps. Exceeding a cap is an explicit unsupported-size error, never a
/// negative dominance result. It is intended for exact small-domain oracle
/// tests and bounded benchmark tasks, not arbitrary large channels.
pub fn solve_small_blackwell_garbling(
    more_informative: &FiniteObservationChannel,
    less_informative: &FiniteObservationChannel,
) -> Result<Option<BlackwellDominanceReceipt>, String> {
    more_informative.validate()?;
    less_informative.validate()?;
    if more_informative.state_ids != less_informative.state_ids {
        return Err("Blackwell channels must share the same ordered state vocabulary".into());
    }

    let source_observations = more_informative.observation_ids.len();
    let target_observations = less_informative.observation_ids.len();
    if target_observations == 1 {
        let witness = BlackwellGarblingWitness {
            probabilities: vec![vec![Rational::one()]; source_observations],
        };
        return verify_blackwell_dominance(more_informative, less_informative, &witness).map(Some);
    }

    let variables = source_observations
        .checked_mul(target_observations - 1)
        .ok_or_else(|| "Blackwell variable-count overflow".to_string())?;
    if variables == 0 || variables > MAX_SMALL_BLACKWELL_VARIABLES {
        return Err(format!(
            "small Blackwell solver supports 1..={MAX_SMALL_BLACKWELL_VARIABLES} free variables"
        ));
    }

    let mut constraints = Vec::new();
    let zero_vector = || vec![SignedRational::from_rational(Rational::zero()); variables];

    // For every state and all but the final target observation:
    // sum_z K_more(state,z) * G(z,y) = K_less(state,y).
    for (state, source_row) in more_informative.probabilities.iter().enumerate() {
        for (target_observation, target_probability) in less_informative.probabilities[state]
            .iter()
            .copied()
            .take(target_observations - 1)
            .enumerate()
        {
            let mut coefficients = zero_vector();
            for (source_observation, probability) in source_row.iter().copied().enumerate() {
                let variable = source_observation * (target_observations - 1) + target_observation;
                coefficients[variable] = SignedRational::from_rational(probability);
            }
            let target_probability = SignedRational::from_rational(target_probability);
            constraints.push(LinearInequality {
                coefficients: coefficients.clone(),
                upper_bound: target_probability,
            });
            constraints.push(LinearInequality {
                coefficients: coefficients
                    .iter()
                    .copied()
                    .map(SignedRational::negated)
                    .collect(),
                upper_bound: target_probability.negated(),
            });
        }
    }

    // Each garbling probability is nonnegative.
    for variable in 0..variables {
        let coefficients = (0..variables)
            .map(|index| {
                if index == variable {
                    SignedRational::from_rational(Rational::one()).negated()
                } else {
                    SignedRational::from_rational(Rational::zero())
                }
            })
            .collect();
        constraints.push(LinearInequality {
            coefficients,
            upper_bound: SignedRational::from_rational(Rational::zero()),
        });
    }

    // Each source-observation row lies in the simplex: sum_y G(z,y) <= 1.
    for source_observation in 0..source_observations {
        let mut coefficients = zero_vector();
        let start = source_observation * (target_observations - 1);
        let end = start + target_observations - 1;
        for coefficient in &mut coefficients[start..end] {
            *coefficient = SignedRational::from_rational(Rational::one());
        }
        constraints.push(LinearInequality {
            coefficients,
            upper_bound: SignedRational::from_rational(Rational::one()),
        });
    }

    let Some(solution) = exact_boxed_linear_feasibility(variables, constraints)? else {
        return Ok(None);
    };

    let mut garbling_rows = Vec::with_capacity(source_observations);
    for solution_row in solution.chunks_exact(target_observations - 1) {
        let mut row = Vec::with_capacity(target_observations);
        let mut assigned_mass = Rational::zero();
        for signed_probability in solution_row.iter().copied() {
            let Some(probability) = signed_probability.as_probability() else {
                return Err(
                    "Blackwell witness reconstruction produced a negative probability".into(),
                );
            };
            assigned_mass = assigned_mass.checked_add(probability)?;
            row.push(probability);
        }
        let final_probability =
            subtract_nonnegative(Rational::one(), assigned_mass).map_err(|_| {
                "Blackwell witness reconstruction exceeded a row's probability mass".to_string()
            })?;
        row.push(final_probability);
        garbling_rows.push(row);
    }

    let witness = BlackwellGarblingWitness {
        probabilities: garbling_rows,
    };
    verify_blackwell_dominance(more_informative, less_informative, &witness).map(Some)
}

fn normalize_inequality(mut inequality: LinearInequality) -> Result<LinearInequality, String> {
    let scale = inequality
        .coefficients
        .iter()
        .copied()
        .find(|coefficient| !coefficient.is_zero())
        .map(SignedRational::absolute_magnitude)
        .or_else(|| {
            (!inequality.upper_bound.is_zero())
                .then_some(inequality.upper_bound.absolute_magnitude())
        })
        .unwrap_or_else(Rational::one);
    let positive_scale = SignedRational::from_rational(scale);
    for coefficient in &mut inequality.coefficients {
        *coefficient = coefficient.checked_div(positive_scale)?;
    }
    inequality.upper_bound = inequality.upper_bound.checked_div(positive_scale)?;
    Ok(inequality)
}

/// Normalize positive scalar multiples and retain only the strongest bound
/// for each coefficient vector.
fn deduplicate_inequalities(
    inequalities: Vec<LinearInequality>,
) -> Result<Option<Vec<LinearInequality>>, String> {
    let mut strongest = BTreeMap::<Vec<SignedRational>, SignedRational>::new();
    for inequality in inequalities {
        let inequality = normalize_inequality(inequality)?;
        if inequality
            .coefficients
            .iter()
            .all(|coefficient| coefficient.is_zero())
        {
            if inequality.upper_bound.is_negative() {
                return Ok(None);
            }
            continue;
        }
        strongest
            .entry(inequality.coefficients)
            .and_modify(|upper| {
                if inequality.upper_bound < *upper {
                    *upper = inequality.upper_bound;
                }
            })
            .or_insert(inequality.upper_bound);
        if strongest.len() > MAX_SMALL_BLACKWELL_CONSTRAINTS {
            return Err("small Blackwell elimination exceeded the constraint cap".into());
        }
    }
    Ok(Some(
        strongest
            .into_iter()
            .map(|(coefficients, upper_bound)| LinearInequality {
                coefficients,
                upper_bound,
            })
            .collect(),
    ))
}

fn eliminate_first_variable(
    inequalities: Vec<LinearInequality>,
    pair_products: &mut usize,
) -> Result<Option<Vec<LinearInequality>>, String> {
    let mut positive = Vec::new();
    let mut negative = Vec::new();
    let mut zero = Vec::new();
    for inequality in inequalities {
        let coefficient = inequality.coefficients[0];
        if coefficient.is_negative() {
            negative.push(inequality);
        } else if coefficient.is_zero() {
            let mut reduced = inequality;
            reduced.coefficients.remove(0);
            zero.push(reduced);
        } else {
            positive.push(inequality);
        }
    }

    let pair_count = positive
        .len()
        .checked_mul(negative.len())
        .ok_or_else(|| "Blackwell elimination pair-count overflow".to_string())?;
    *pair_products = pair_products
        .checked_add(pair_count)
        .ok_or_else(|| "Blackwell elimination pair-product counter overflow".to_string())?;
    if *pair_products > MAX_SMALL_BLACKWELL_PAIR_PRODUCTS {
        return Err("small Blackwell elimination exceeded the pair-product cap".into());
    }

    // If one sign is absent, the eliminated variable can move without bound
    // in the satisfying direction, so all nonzero-coefficient rows disappear.
    if positive.is_empty() || negative.is_empty() {
        return deduplicate_inequalities(zero);
    }

    let mut generated = zero;
    for upper in &positive {
        for lower in &negative {
            let positive_factor =
                SignedRational::from_rational(lower.coefficients[0].absolute_magnitude());
            let negative_factor =
                SignedRational::from_rational(upper.coefficients[0].absolute_magnitude());
            let mut coefficients = Vec::with_capacity(upper.coefficients.len() - 1);
            for index in 1..upper.coefficients.len() {
                let upper_term = upper.coefficients[index].checked_mul(positive_factor)?;
                let lower_term = lower.coefficients[index].checked_mul(negative_factor)?;
                coefficients.push(upper_term.checked_add(lower_term)?);
            }
            let upper_bound = upper
                .upper_bound
                .checked_mul(positive_factor)?
                .checked_add(lower.upper_bound.checked_mul(negative_factor)?)?;
            generated.push(LinearInequality {
                coefficients,
                upper_bound,
            });
            if generated.len() > MAX_SMALL_BLACKWELL_CONSTRAINTS * 2 {
                return Err("small Blackwell elimination exceeded its intermediate cap".into());
            }
        }
    }
    deduplicate_inequalities(generated)
}

fn exact_boxed_linear_feasibility(
    variables: usize,
    mut inequalities: Vec<LinearInequality>,
) -> Result<Option<Vec<SignedRational>>, String> {
    let mut stages = Vec::with_capacity(variables);
    let mut pair_products = 0usize;
    for _ in 0..variables {
        if inequalities.len() > MAX_SMALL_BLACKWELL_CONSTRAINTS {
            return Err("small Blackwell elimination exceeded the constraint cap".into());
        }
        stages.push(inequalities.clone());
        let Some(reduced) = eliminate_first_variable(inequalities, &mut pair_products)? else {
            return Ok(None);
        };
        inequalities = reduced;
    }
    if inequalities
        .iter()
        .any(|inequality| inequality.upper_bound.is_negative())
    {
        return Ok(None);
    }

    // Reconstruct a feasible point from the saved elimination stages.
    let zero = SignedRational::from_rational(Rational::zero());
    let mut solution = vec![zero; variables];
    for variable in (0..variables).rev() {
        let mut lower_bound: Option<SignedRational> = None;
        let mut upper_bound: Option<SignedRational> = None;
        for inequality in &stages[variable] {
            let coefficient = inequality.coefficients[0];
            let mut residual = inequality.upper_bound;
            for (remaining_coefficient, assigned) in inequality
                .coefficients
                .iter()
                .skip(1)
                .zip(solution.iter().skip(variable + 1))
            {
                residual = residual.checked_sub(remaining_coefficient.checked_mul(*assigned)?)?;
            }
            if coefficient.is_negative() {
                let bound = residual.checked_div(coefficient)?;
                if lower_bound.is_none_or(|current| bound > current) {
                    lower_bound = Some(bound);
                }
            } else if coefficient.is_zero() {
                if residual.is_negative() {
                    return Ok(None);
                }
            } else {
                let bound = residual.checked_div(coefficient)?;
                if upper_bound.is_none_or(|current| bound < current) {
                    upper_bound = Some(bound);
                }
            }
        }
        if lower_bound
            .zip(upper_bound)
            .is_some_and(|(lower, upper)| lower > upper)
        {
            return Ok(None);
        }
        solution[variable] = match (lower_bound, upper_bound) {
            (Some(lower), _) if lower > zero => lower,
            (_, Some(upper)) if upper < zero => upper,
            _ => zero,
        };
    }
    Ok(Some(solution))
}


/// Maximum inequalities accepted by the independent three-variable vertex oracle.
/// Six box constraints plus two residual constraints per state fit at most 13 states.
const MAX_BINARY_LECAM_VERTEX_CONSTRAINTS: usize = 32;

/// Exact directed Le Cam deficiency for two-output finite channels.
///
/// For A and B with two observations each, write
/// x = G(0,0), y = G(1,0). The remaining garbling entries are 1-x and 1-y.
/// For each state s, the binary-output total variation residual is
/// |B(s,0) - A(s,0)x - (1-A(s,0))y|. We minimize its maximum exactly.
///
/// This is deliberately a small-domain two-output oracle, not a generalized
/// Le Cam deficiency solver. It provides two independent optimality checks:
/// Fourier-Motzkin projection and exhaustive exact active-set vertex enumeration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinaryLeCamDeficiencyReceipt {
    pub dominance: super::approximate_blackwell::ApproximateBlackwellDominanceReceipt,
    pub optimal_deficiency: Rational,
    pub fme_pair_products: usize,
    pub active_sets_checked: usize,
    pub feasible_vertices: usize,
    pub independent_vertex_lower_bound: Rational,
    pub optimality_verified: bool,
}

fn binary_lecam_constraints(
    more_informative: &FiniteObservationChannel,
    target: &FiniteObservationChannel,
) -> Result<Vec<LinearInequality>, String> {
    more_informative.validate()?;
    target.validate()?;
    if more_informative.state_ids != target.state_ids {
        return Err("Le Cam channels must share the same ordered state vocabulary".into());
    }
    if more_informative.observation_ids.len() != 2 || target.observation_ids.len() != 2 {
        return Err(
            "binary Le Cam solver requires exactly two observations in each channel".into(),
        );
    }
    if more_informative.state_ids.len() > 13 {
        return Err("binary Le Cam vertex oracle supports at most 13 hidden states".into());
    }

    let mut constraints = Vec::with_capacity(6 + 2 * more_informative.state_ids.len());
    let signed = SignedRational::from_rational;

    // 0 <= x,y,epsilon <= 1.
    for variable in 0..3 {
        let mut lower = vec![signed(Rational::zero()); 3];
        lower[variable] = signed(Rational::one()).negated();
        constraints.push(LinearInequality {
            coefficients: lower,
            upper_bound: signed(Rational::zero()),
        });
        let mut upper = vec![signed(Rational::zero()); 3];
        upper[variable] = signed(Rational::one());
        constraints.push(LinearInequality {
            coefficients: upper,
            upper_bound: signed(Rational::one()),
        });
    }

    // For each state, constrain both signs of the binary-output residual.
    for (state, source_row) in more_informative.probabilities.iter().enumerate() {
        let a = source_row[0];
        let not_a = subtract_nonnegative(Rational::one(), a)?;
        let b = target.probabilities[state][0];

        // a*x + (1-a)*y - epsilon <= b.
        constraints.push(LinearInequality {
            coefficients: vec![
                signed(a),
                signed(not_a),
                signed(Rational::one()).negated(),
            ],
            upper_bound: signed(b),
        });

        // -a*x - (1-a)*y - epsilon <= -b.
        constraints.push(LinearInequality {
            coefficients: vec![
                signed(a).negated(),
                signed(not_a).negated(),
                signed(Rational::one()).negated(),
            ],
            upper_bound: signed(b).negated(),
        });
    }

    if constraints.len() > MAX_BINARY_LECAM_VERTEX_CONSTRAINTS {
        return Err("binary Le Cam vertex oracle exceeded its inequality cap".into());
    }
    Ok(constraints)
}

/// Return the exact minimum epsilon after eliminating x and y.
///
/// The projected system has one variable. Its negative-coefficient inequalities
/// give lower bounds, its positive-coefficient inequalities give upper bounds.
fn project_binary_lecam_minimum(
    constraints: &[LinearInequality],
) -> Result<(Rational, usize), String> {
    let mut projected = constraints.to_vec();
    let mut pair_products = 0usize;
    for _ in 0..2 {
        let Some(next) = eliminate_first_variable(projected, &mut pair_products)? else {
            return Err("binary Le Cam projection found an empty feasible set".into());
        };
        projected = next;
    }

    let zero = SignedRational::from_rational(Rational::zero());
    let one = SignedRational::from_rational(Rational::one());
    let mut lower = zero;
    let mut upper = one;

    for inequality in &projected {
        if inequality.coefficients.len() != 1 {
            return Err("binary Le Cam projection did not reduce to one variable".into());
        }
        let coefficient = inequality.coefficients[0];
        if coefficient.is_zero() {
            if inequality.upper_bound.is_negative() {
                return Err("binary Le Cam projected constraints are infeasible".into());
            }
        } else {
            let bound = inequality.upper_bound.checked_div(coefficient)?;
            if coefficient.is_negative() {
                if bound > lower {
                    lower = bound;
                }
            } else if bound < upper {
                upper = bound;
            }
        }
    }

    if lower > upper {
        return Err("binary Le Cam projected interval is empty".into());
    }
    let Some(minimum) = lower.as_probability() else {
        return Err("binary Le Cam projection produced an invalid minimum".into());
    };
    Ok((minimum, pair_products))
}

fn solve_three_active_constraints(
    first: &LinearInequality,
    second: &LinearInequality,
    third: &LinearInequality,
) -> Result<Option<Vec<SignedRational>>, String> {
    let selected = [first, second, third];
    let mut matrix = Vec::with_capacity(3);
    for constraint in selected {
        if constraint.coefficients.len() != 3 {
            return Err("binary Le Cam active constraint has the wrong dimension".into());
        }
        let mut row = constraint.coefficients.clone();
        row.push(constraint.upper_bound);
        matrix.push(row);
    }

    for column in 0..3 {
        let Some(pivot_row) =
            (column..3).find(|row| !matrix[*row][column].is_zero())
        else {
            return Ok(None);
        };
        matrix.swap(column, pivot_row);

        let pivot = matrix[column][column];
        for entry in column..=3 {
            matrix[column][entry] = matrix[column][entry].checked_div(pivot)?;
        }

        for row in 0..3 {
            if row == column {
                continue;
            }
            let factor = matrix[row][column];
            if factor.is_zero() {
                continue;
            }
            for entry in column..=3 {
                let product = factor.checked_mul(matrix[column][entry])?;
                matrix[row][entry] = matrix[row][entry].checked_sub(product)?;
            }
        }
    }

    Ok(Some(
        matrix
            .into_iter()
            .map(|row| row[3])
            .collect::<Vec<_>>(),
    ))
}

fn satisfies_linear_constraints(
    candidate: &[SignedRational],
    constraints: &[LinearInequality],
) -> Result<bool, String> {
    if candidate.len() != 3 {
        return Err("binary Le Cam candidate must contain x, y and epsilon".into());
    }
    for constraint in constraints {
        let mut left = SignedRational::from_rational(Rational::zero());
        for (coefficient, value) in constraint.coefficients.iter().zip(candidate) {
            left = left.checked_add(coefficient.checked_mul(*value)?)?;
        }
        if left > constraint.upper_bound {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Independently minimize epsilon by enumerating every three-constraint active
/// set. The feasible polytope is bounded in [0,1]^3, so an optimum is attained
/// at a vertex with three linearly independent active constraints.
fn binary_lecam_vertex_lower_bound(
    constraints: &[LinearInequality],
) -> Result<(Rational, usize, usize), String> {
    if constraints.len() > MAX_BINARY_LECAM_VERTEX_CONSTRAINTS {
        return Err("binary Le Cam vertex oracle exceeded its inequality cap".into());
    }
    let mut active_sets_checked = 0usize;
    let mut feasible_vertices = 0usize;
    let mut best: Option<Rational> = None;

    for first in 0..constraints.len() {
        for second in (first + 1)..constraints.len() {
            for third in (second + 1)..constraints.len() {
                active_sets_checked = active_sets_checked
                    .checked_add(1)
                    .ok_or_else(|| "binary Le Cam active-set counter overflow".to_string())?;
                let Some(candidate) = solve_three_active_constraints(
                    &constraints[first],
                    &constraints[second],
                    &constraints[third],
                )? else {
                    continue;
                };
                if !satisfies_linear_constraints(&candidate, constraints)? {
                    continue;
                }
                let Some(epsilon) = candidate[2].as_probability() else {
                    continue;
                };
                feasible_vertices = feasible_vertices
                    .checked_add(1)
                    .ok_or_else(|| "binary Le Cam feasible-vertex counter overflow".to_string())?;
                if best.is_none_or(|current| epsilon < current) {
                    best = Some(epsilon);
                }
            }
        }
    }

    let lower_bound = best.ok_or_else(|| {
        "binary Le Cam vertex oracle found no feasible vertex in a non-empty bounded polytope"
            .to_string()
    })?;
    Ok((lower_bound, active_sets_checked, feasible_vertices))
}

/// Solve the exact directed Le Cam deficiency for finite channels with
/// exactly two observations each and at most 13 hidden states.
///
/// The minimum is established by agreement between exact Fourier-Motzkin
/// projection and an independent exact active-set vertex oracle. The returned
/// channel witness is separately replayed by the SIM-REL-031 approximate
/// Blackwell verifier, and its maximum state residual must equal the optimum.
pub fn solve_binary_le_cam_deficiency(
    more_informative: &FiniteObservationChannel,
    target: &FiniteObservationChannel,
) -> Result<BinaryLeCamDeficiencyReceipt, String> {
    let constraints = binary_lecam_constraints(more_informative, target)?;
    let (fme_minimum, fme_pair_products) =
        project_binary_lecam_minimum(&constraints)?;
    let (
        independent_vertex_lower_bound,
        active_sets_checked,
        feasible_vertices,
    ) = binary_lecam_vertex_lower_bound(&constraints)?;

    if fme_minimum != independent_vertex_lower_bound {
        return Err(
            "binary Le Cam Fourier-Motzkin and independent vertex optima disagree".into(),
        );
    }

    // Substitute the exact optimum epsilon and solve the remaining boxed
    // two-variable feasibility problem to reconstruct x and y.
    let mut fixed_epsilon_constraints = Vec::with_capacity(constraints.len());
    let epsilon_signed = SignedRational::from_rational(fme_minimum);
    for constraint in &constraints {
        let epsilon_coefficient = constraint.coefficients[2];
        let adjusted_bound = constraint
            .upper_bound
            .checked_sub(epsilon_coefficient.checked_mul(epsilon_signed)?)?;
        fixed_epsilon_constraints.push(LinearInequality {
            coefficients: constraint.coefficients[..2].to_vec(),
            upper_bound: adjusted_bound,
        });
    }
    let solution = exact_boxed_linear_feasibility(2, fixed_epsilon_constraints)?
        .ok_or_else(|| "binary Le Cam optimum could not reconstruct a feasible garbling".to_string())?;
    if solution.len() != 2 {
        return Err("binary Le Cam reconstruction returned the wrong variable count".into());
    }
    let Some(x) = solution[0].as_probability() else {
        return Err("binary Le Cam reconstruction produced invalid x".into());
    };
    let Some(y) = solution[1].as_probability() else {
        return Err("binary Le Cam reconstruction produced invalid y".into());
    };

    let witness = BlackwellGarblingWitness {
        probabilities: vec![
            vec![x, subtract_nonnegative(Rational::one(), x)?],
            vec![y, subtract_nonnegative(Rational::one(), y)?],
        ],
    };
    let dominance = super::approximate_blackwell::verify_approximate_blackwell_dominance(
        more_informative,
        target,
        &witness,
        fme_minimum,
    )?;
    if dominance.maximum_state_residual != fme_minimum {
        return Err(
            "binary Le Cam reconstructed witness residual differs from the exact optimum".into(),
        );
    }

    Ok(BinaryLeCamDeficiencyReceipt {
        dominance,
        optimal_deficiency: fme_minimum,
        fme_pair_products,
        active_sets_checked,
        feasible_vertices,
        independent_vertex_lower_bound,
        optimality_verified: true,
    })
}

fn validate_unique_ids(ids: &[String], label: &str) -> Result<(), String> {
    if ids.is_empty() {
        return Err(format!("{label} vocabulary must be non-empty"));
    }
    let mut unique = BTreeSet::new();
    if ids
        .iter()
        .any(|id| id.trim().is_empty() || !unique.insert(id.as_str()))
    {
        return Err(format!("{label} IDs must be non-empty and unique"));
    }
    Ok(())
}

fn subtract_nonnegative(left: Rational, right: Rational) -> Result<Rational, String> {
    if left < right {
        return Err("rational subtraction would be negative".into());
    }
    let common = gcd(left.denominator(), right.denominator());
    let left_scale = right.denominator() / common;
    let right_scale = left.denominator() / common;
    let left_numerator = left
        .numerator()
        .checked_mul(left_scale)
        .ok_or_else(|| "rational subtraction numerator overflow".to_string())?;
    let right_numerator = right
        .numerator()
        .checked_mul(right_scale)
        .ok_or_else(|| "rational subtraction numerator overflow".to_string())?;
    let denominator = left
        .denominator()
        .checked_mul(left_scale)
        .ok_or_else(|| "rational subtraction denominator overflow".to_string())?;
    Rational::new(left_numerator - right_numerator, denominator)
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rational(n: u128, d: u128) -> Rational {
        Rational::new(n, d).unwrap()
    }

    fn binary_decisions() -> ChannelDecisionLossTable {
        ChannelDecisionLossTable {
            decision_ids: vec!["state-zero".into(), "state-one".into()],
            losses: vec![
                vec![Rational::zero(), Rational::one()],
                vec![Rational::one(), Rational::zero()],
            ],
            abstention_loss: rational(1, 2),
        }
    }

    fn perfect_channel(id: &str) -> FiniteObservationChannel {
        FiniteObservationChannel {
            channel_id: id.into(),
            state_ids: vec!["s0".into(), "s1".into()],
            observation_ids: vec!["o0".into(), "o1".into()],
            probabilities: vec![
                vec![Rational::one(), Rational::zero()],
                vec![Rational::zero(), Rational::one()],
            ],
        }
    }

    fn noisy_channel(id: &str) -> FiniteObservationChannel {
        FiniteObservationChannel {
            channel_id: id.into(),
            state_ids: vec!["s0".into(), "s1".into()],
            observation_ids: vec!["o0".into(), "o1".into()],
            probabilities: vec![
                vec![rational(4, 5), rational(1, 5)],
                vec![rational(1, 5), rational(4, 5)],
            ],
        }
    }

    fn inverted_channel(id: &str) -> FiniteObservationChannel {
        FiniteObservationChannel {
            channel_id: id.into(),
            state_ids: vec!["s0".into(), "s1".into()],
            observation_ids: vec!["o0".into(), "o1".into()],
            probabilities: vec![
                vec![Rational::zero(), Rational::one()],
                vec![Rational::one(), Rational::zero()],
            ],
        }
    }

    fn posterior_rule(
        assumed: &FiniteObservationChannel,
        rule_id: &str,
        posterior_by_observation: Vec<Vec<Rational>>,
    ) -> InferencePosteriorRule {
        InferencePosteriorRule {
            rule_id: rule_id.into(),
            assumed_channel_id: assumed.channel_id.clone(),
            state_ids: assumed.state_ids.clone(),
            observation_ids: assumed.observation_ids.clone(),
            posterior_by_observation,
        }
    }

    #[test]
    fn exact_garbling_witness_proves_nominal_blackwell_dominance() {
        let perfect = perfect_channel("perfect");
        let noisy = noisy_channel("noisy");
        let witness = BlackwellGarblingWitness {
            probabilities: vec![
                vec![rational(4, 5), rational(1, 5)],
                vec![rational(1, 5), rational(4, 5)],
            ],
        };
        let receipt = verify_blackwell_dominance(&perfect, &noisy, &witness).unwrap();
        assert!(receipt.exactly_verified);
        assert_eq!(receipt.more_informative_channel_id, "perfect");
        assert_eq!(receipt.less_informative_channel_id, "noisy");
    }

    #[test]
    fn binary_blackwell_solver_constructs_and_verifies_exact_garbling() {
        let perfect = perfect_channel("perfect");
        let noisy = noisy_channel("noisy");
        let receipt = solve_binary_blackwell_garbling(&perfect, &noisy)
            .unwrap()
            .unwrap();
        assert!(receipt.exactly_verified);
        assert_eq!(
            receipt.witness.probabilities,
            vec![
                vec![rational(4, 5), rational(1, 5)],
                vec![rational(1, 5), rational(4, 5)],
            ]
        );
    }

    #[test]
    fn binary_blackwell_solver_recovers_a_non_symmetric_rational_garbling() {
        let a = rational(2, 3);
        let c = rational(1, 4);
        let x = rational(1, 5);
        let y = rational(4, 5);
        let one_minus = |value: Rational| subtract_nonnegative(Rational::one(), value).unwrap();
        let b0 = a
            .checked_mul(x)
            .unwrap()
            .checked_add(one_minus(a).checked_mul(y).unwrap())
            .unwrap();
        let b1 = c
            .checked_mul(x)
            .unwrap()
            .checked_add(one_minus(c).checked_mul(y).unwrap())
            .unwrap();
        let more = FiniteObservationChannel {
            channel_id: "more-asymmetric".into(),
            state_ids: vec!["s0".into(), "s1".into()],
            observation_ids: vec!["z0".into(), "z1".into()],
            probabilities: vec![vec![a, one_minus(a)], vec![c, one_minus(c)]],
        };
        let less = FiniteObservationChannel {
            channel_id: "less-asymmetric".into(),
            state_ids: vec!["s0".into(), "s1".into()],
            observation_ids: vec!["y0".into(), "y1".into()],
            probabilities: vec![vec![b0, one_minus(b0)], vec![b1, one_minus(b1)]],
        };
        let receipt = solve_binary_blackwell_garbling(&more, &less)
            .unwrap()
            .unwrap();
        assert_eq!(
            receipt.witness.probabilities,
            vec![vec![x, one_minus(x)], vec![y, one_minus(y)],]
        );
    }

    #[test]
    fn binary_blackwell_solver_rejects_non_dominance_and_inconsistent_singular_case() {
        let noisy = noisy_channel("noisy");
        let perfect = perfect_channel("perfect");
        assert!(
            solve_binary_blackwell_garbling(&noisy, &perfect)
                .unwrap()
                .is_none()
        );

        let uninformative = FiniteObservationChannel {
            channel_id: "uninformative".into(),
            state_ids: vec!["s0".into(), "s1".into()],
            observation_ids: vec!["z0".into(), "z1".into()],
            probabilities: vec![
                vec![rational(1, 2), rational(1, 2)],
                vec![rational(1, 2), rational(1, 2)],
            ],
        };
        let inconsistent_target = noisy_channel("not-a-garbling");
        assert!(
            solve_binary_blackwell_garbling(&uninformative, &inconsistent_target)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn binary_blackwell_solver_handles_deterministic_boundary_garblings() {
        let more = perfect_channel("perfect");
        let identity_target = perfect_channel("identity-target");
        let identity = solve_binary_blackwell_garbling(&more, &identity_target)
            .unwrap()
            .unwrap();
        assert_eq!(
            identity.witness.probabilities,
            vec![
                vec![Rational::one(), Rational::zero()],
                vec![Rational::zero(), Rational::one()],
            ]
        );

        let reversed_target = inverted_channel("reversed-target");
        let reversed = solve_binary_blackwell_garbling(&more, &reversed_target)
            .unwrap()
            .unwrap();
        assert_eq!(
            reversed.witness.probabilities,
            vec![
                vec![Rational::zero(), Rational::one()],
                vec![Rational::one(), Rational::zero()],
            ]
        );
    }

    #[test]
    fn binary_blackwell_solver_handles_singular_but_consistent_channels() {
        let uninformative = FiniteObservationChannel {
            channel_id: "uninformative".into(),
            state_ids: vec!["s0".into(), "s1".into()],
            observation_ids: vec!["z0".into(), "z1".into()],
            probabilities: vec![
                vec![rational(1, 2), rational(1, 2)],
                vec![rational(1, 2), rational(1, 2)],
            ],
        };
        let same_uninformative_target = FiniteObservationChannel {
            channel_id: "same-uninformative".into(),
            state_ids: vec!["s0".into(), "s1".into()],
            observation_ids: vec!["y0".into(), "y1".into()],
            probabilities: vec![
                vec![rational(3, 10), rational(7, 10)],
                vec![rational(3, 10), rational(7, 10)],
            ],
        };
        let receipt = solve_binary_blackwell_garbling(&uninformative, &same_uninformative_target)
            .unwrap()
            .unwrap();
        assert_eq!(
            receipt.witness.probabilities,
            vec![
                vec![rational(3, 10), rational(7, 10)],
                vec![rational(3, 10), rational(7, 10)],
            ]
        );
    }

    #[test]
    fn binary_blackwell_solver_rejects_unsupported_dimensions() {
        let binary = perfect_channel("binary");
        let multi = FiniteObservationChannel {
            channel_id: "multi".into(),
            state_ids: vec!["s0".into(), "s1".into()],
            observation_ids: vec!["o0".into(), "o1".into(), "o2".into()],
            probabilities: vec![
                vec![rational(1, 3), rational(1, 3), rational(1, 3)],
                vec![rational(1, 3), rational(1, 3), rational(1, 3)],
            ],
        };
        assert!(solve_binary_blackwell_garbling(&binary, &multi).is_err());
    }

    fn asymmetric_three_by_three_channels() -> (
        FiniteObservationChannel,
        FiniteObservationChannel,
        Vec<Vec<Rational>>,
    ) {
        let half = rational(1, 2);
        let third = rational(1, 3);
        let two_thirds = rational(2, 3);
        let quarter = rational(1, 4);
        let garbling = vec![
            vec![two_thirds, third, Rational::zero()],
            vec![Rational::zero(), half, half],
            vec![third, Rational::zero(), two_thirds],
        ];
        let more = FiniteObservationChannel {
            channel_id: "three-state-source".into(),
            state_ids: vec!["s0".into(), "s1".into(), "s2".into()],
            observation_ids: vec!["z0".into(), "z1".into(), "z2".into()],
            probabilities: vec![
                vec![half, half, Rational::zero()],
                vec![Rational::zero(), half, half],
                vec![half, Rational::zero(), half],
            ],
        };
        let less = FiniteObservationChannel {
            channel_id: "three-state-garbled".into(),
            state_ids: vec!["s0".into(), "s1".into(), "s2".into()],
            observation_ids: vec!["y0".into(), "y1".into(), "y2".into()],
            probabilities: vec![
                vec![third, rational(5, 12), quarter],
                vec![rational(1, 6), quarter, rational(7, 12)],
                vec![half, rational(1, 6), third],
            ],
        };
        (more, less, garbling)
    }

    #[test]
    fn general_blackwell_solver_recovers_non_symmetric_three_by_three_witness() {
        let (more, less, expected_witness) = asymmetric_three_by_three_channels();
        let receipt = solve_small_blackwell_garbling(&more, &less)
            .unwrap()
            .unwrap();
        assert!(receipt.exactly_verified);
        assert_eq!(receipt.witness.probabilities, expected_witness);
        verify_blackwell_dominance(&more, &less, &receipt.witness).unwrap();
    }

    #[test]
    fn general_blackwell_solver_matches_independent_binary_oracle_on_quarter_grid() {
        let probability = |numerator: u128| Rational::new(numerator, 4).unwrap();
        let make_channel =
            |id: String, first: Rational, second: Rational| FiniteObservationChannel {
                channel_id: id,
                state_ids: vec!["s0".into(), "s1".into()],
                observation_ids: vec!["o0".into(), "o1".into()],
                probabilities: vec![
                    vec![first, subtract_nonnegative(Rational::one(), first).unwrap()],
                    vec![
                        second,
                        subtract_nonnegative(Rational::one(), second).unwrap(),
                    ],
                ],
            };

        let mut comparisons = 0usize;
        for source_state_zero in 0..=4 {
            for source_state_one in 0..=4 {
                let more = make_channel(
                    format!("source-{source_state_zero}-{source_state_one}"),
                    probability(source_state_zero),
                    probability(source_state_one),
                );
                for target_state_zero in 0..=4 {
                    for target_state_one in 0..=4 {
                        let less = make_channel(
                            format!("target-{target_state_zero}-{target_state_one}"),
                            probability(target_state_zero),
                            probability(target_state_one),
                        );
                        let binary = solve_binary_blackwell_garbling(&more, &less)
                            .expect("binary closed-form feasibility must be well-defined");
                        let general = solve_small_blackwell_garbling(&more, &less)
                            .expect("bounded rational feasibility must be well-defined");
                        assert_eq!(
                            binary.is_some(),
                            general.is_some(),
                            "solver disagreement for source=({source_state_zero}/4,{source_state_one}/4), target=({target_state_zero}/4,{target_state_one}/4)"
                        );
                        if let Some(receipt) = general {
                            verify_blackwell_dominance(&more, &less, &receipt.witness).unwrap();
                        }
                        comparisons += 1;
                    }
                }
            }
        }
        assert_eq!(comparisons, 625);
    }

    #[test]
    fn general_blackwell_solver_rejects_an_impossible_three_by_three_channel() {
        let (more, _, _) = asymmetric_three_by_three_channels();
        let perfect = FiniteObservationChannel {
            channel_id: "impossibly-perfect-target".into(),
            state_ids: more.state_ids.clone(),
            observation_ids: vec!["y0".into(), "y1".into(), "y2".into()],
            probabilities: vec![
                vec![Rational::one(), Rational::zero(), Rational::zero()],
                vec![Rational::zero(), Rational::one(), Rational::zero()],
                vec![Rational::zero(), Rational::zero(), Rational::one()],
            ],
        };
        assert!(
            solve_small_blackwell_garbling(&more, &perfect)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn general_blackwell_solver_handles_singular_channels_and_detects_inconsistency() {
        let more = FiniteObservationChannel {
            channel_id: "singular-source".into(),
            state_ids: vec!["s0".into(), "s1".into(), "s2".into()],
            observation_ids: vec!["z0".into(), "z1".into(), "z2".into()],
            probabilities: vec![
                vec![rational(1, 3), rational(1, 3), rational(1, 3)],
                vec![rational(1, 3), rational(1, 3), rational(1, 3)],
                vec![rational(1, 3), rational(1, 3), rational(1, 3)],
            ],
        };
        let consistent = FiniteObservationChannel {
            channel_id: "singular-consistent-target".into(),
            state_ids: more.state_ids.clone(),
            observation_ids: vec!["y0".into(), "y1".into(), "y2".into()],
            probabilities: vec![
                vec![rational(1, 2), rational(1, 3), rational(1, 6)],
                vec![rational(1, 2), rational(1, 3), rational(1, 6)],
                vec![rational(1, 2), rational(1, 3), rational(1, 6)],
            ],
        };
        let receipt = solve_small_blackwell_garbling(&more, &consistent)
            .unwrap()
            .unwrap();
        verify_blackwell_dominance(&more, &consistent, &receipt.witness).unwrap();

        let inconsistent = FiniteObservationChannel {
            channel_id: "singular-inconsistent-target".into(),
            probabilities: vec![
                vec![Rational::one(), Rational::zero(), Rational::zero()],
                vec![Rational::zero(), Rational::one(), Rational::zero()],
                vec![Rational::zero(), Rational::zero(), Rational::one()],
            ],
            ..consistent
        };
        assert!(
            solve_small_blackwell_garbling(&more, &inconsistent)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn general_blackwell_solver_fails_closed_when_variable_cap_is_exceeded() {
        let more = FiniteObservationChannel {
            channel_id: "too-wide-source".into(),
            state_ids: vec!["s0".into()],
            observation_ids: (0..7).map(|index| format!("z{index}")).collect(),
            probabilities: vec![vec![rational(1, 7); 7]],
        };
        let less = FiniteObservationChannel {
            channel_id: "too-wide-target".into(),
            state_ids: more.state_ids.clone(),
            observation_ids: vec!["y0".into(), "y1".into(), "y2".into()],
            probabilities: vec![vec![rational(1, 3); 3]],
        };
        assert!(solve_small_blackwell_garbling(&more, &less).is_err());
    }

    #[test]
    fn general_blackwell_solver_handles_a_single_output_target() {
        let more = perfect_channel("binary-source");
        let less = FiniteObservationChannel {
            channel_id: "single-output-target".into(),
            state_ids: more.state_ids.clone(),
            observation_ids: vec!["always".into()],
            probabilities: vec![vec![Rational::one()], vec![Rational::one()]],
        };
        let receipt = solve_small_blackwell_garbling(&more, &less)
            .unwrap()
            .unwrap();
        assert_eq!(
            receipt.witness.probabilities,
            vec![vec![Rational::one()], vec![Rational::one()]]
        );
    }

    #[test]
    fn more_nominal_information_can_produce_greater_realized_decision_loss() {
        let prior = vec![rational(1, 2), rational(1, 2)];
        let loss = binary_decisions();
        let assumed_perfect = perfect_channel("A-perfect-assumed");
        let true_inverted = inverted_channel("A-inverted-truth");
        let assumed_noisy = noisy_channel("B-noisy-assumed");
        let true_noisy = noisy_channel("B-noisy-truth");

        let comparison = compare_experiment_pair(
            &assumed_perfect,
            &true_inverted,
            rational(1, 10),
            &assumed_noisy,
            &true_noisy,
            rational(1, 10),
            &prior,
            &loss,
        )
        .unwrap();
        let evaluation_a = &comparison.experiment_a;
        let evaluation_b = &comparison.experiment_b;

        assert!(comparison.mutual_information_a_bits > comparison.mutual_information_b_bits);
        assert_eq!(comparison.information_preference, ExperimentPreference::A);
        assert_eq!(comparison.cost_to_loss_exchange_rate, Rational::one());
        assert_eq!(
            comparison.nominal_pareto_relation,
            ExperimentParetoRelation::ADominatesB
        );
        assert_eq!(
            comparison.realized_pareto_relation,
            ExperimentParetoRelation::BDominatesA
        );
        assert_eq!(
            comparison.nominal_total_loss_preference,
            ExperimentPreference::A
        );
        assert_eq!(comparison.nominal_total_loss_a, rational(1, 10));
        assert_eq!(comparison.nominal_total_loss_b, rational(3, 10));
        assert_eq!(
            comparison.realized_total_loss_preference,
            ExperimentPreference::B
        );
        assert_eq!(comparison.realized_total_loss_a, rational(11, 10));
        assert_eq!(comparison.realized_total_loss_b, rational(3, 10));
        assert!(comparison.information_loss_ranking_reversal);
        assert!(
            comparison
                .blackwell_a_dominates_b
                .as_ref()
                .unwrap()
                .is_some()
        );
        assert!(
            comparison
                .blackwell_b_dominates_a
                .as_ref()
                .unwrap()
                .is_none()
        );

        assert_eq!(evaluation_a.nominal_model_expected_loss, Rational::zero());
        assert_eq!(
            evaluation_a.expected_assumed_loss_under_true_observations,
            Rational::zero()
        );
        assert_eq!(evaluation_a.realized_policy_loss, Rational::one());
        assert_eq!(evaluation_a.decision_regret, Rational::one());
        assert_eq!(
            evaluation_a.decision_disagreement_probability,
            Rational::one()
        );

        assert_eq!(evaluation_b.nominal_model_expected_loss, rational(1, 5));
        assert_eq!(evaluation_b.realized_policy_loss, rational(1, 5));
        assert_eq!(evaluation_b.true_bayes_oracle_loss, rational(1, 5));
        assert_eq!(evaluation_b.decision_regret, Rational::zero());
    }

    #[test]
    fn cost_to_loss_exchange_rate_exposes_scalarization_sensitivity_and_pareto_tradeoff() {
        let prior = vec![rational(1, 2), rational(1, 2)];
        let loss = binary_decisions();
        let perfect = perfect_channel("expensive-perfect");
        let noisy = noisy_channel("cheap-noisy");

        let low_cost_weight = compare_experiment_pair(
            &perfect,
            &perfect,
            rational(2, 5),
            &noisy,
            &noisy,
            rational(1, 10),
            &prior,
            &loss,
        )
        .unwrap();
        assert_eq!(
            low_cost_weight.nominal_pareto_relation,
            ExperimentParetoRelation::Tradeoff
        );
        assert_eq!(
            low_cost_weight.realized_pareto_relation,
            ExperimentParetoRelation::Tradeoff
        );
        assert_eq!(
            low_cost_weight.realized_cost_to_loss_crossover_rate,
            Some(rational(2, 3))
        );
        assert_eq!(
            low_cost_weight.nominal_cost_to_loss_crossover_rate,
            Some(rational(2, 3))
        );
        // The default exchange rate is one: 0.4 vs 0.1 + 0.2, so B wins.
        assert_eq!(
            low_cost_weight.realized_total_loss_preference,
            ExperimentPreference::B
        );

        let cheap_cost_conversion = compare_experiment_pair_with_inference_rules_and_cost_rate(
            &perfect,
            &perfect,
            &exact_bayes_posterior_rule(&perfect, &prior).unwrap(),
            rational(2, 5),
            &noisy,
            &noisy,
            &exact_bayes_posterior_rule(&noisy, &prior).unwrap(),
            rational(1, 10),
            rational(1, 10),
            &prior,
            &loss,
        )
        .unwrap();
        assert_eq!(cheap_cost_conversion.realized_total_loss_a, rational(1, 25));
        assert_eq!(
            cheap_cost_conversion.realized_total_loss_b,
            rational(21, 100)
        );
        assert_eq!(
            cheap_cost_conversion.realized_total_loss_preference,
            ExperimentPreference::A
        );
        assert_eq!(
            cheap_cost_conversion.realized_pareto_relation,
            ExperimentParetoRelation::Tradeoff
        );
    }

    #[test]
    fn pareto_frontier_preserves_tradeoffs_and_changes_under_realized_shift() {
        let candidates = vec![
            ExperimentParetoCandidate {
                experiment_id: "A".into(),
                experiment_cost: rational(2, 5),
                nominal_decision_loss: Rational::zero(),
                realized_decision_loss: Rational::one(),
            },
            ExperimentParetoCandidate {
                experiment_id: "B".into(),
                experiment_cost: rational(1, 10),
                nominal_decision_loss: rational(1, 5),
                realized_decision_loss: rational(1, 5),
            },
            ExperimentParetoCandidate {
                experiment_id: "C".into(),
                experiment_cost: rational(1, 2),
                nominal_decision_loss: rational(3, 10),
                realized_decision_loss: rational(4, 5),
            },
            ExperimentParetoCandidate {
                experiment_id: "D".into(),
                experiment_cost: rational(1, 10),
                nominal_decision_loss: rational(1, 4),
                realized_decision_loss: rational(1, 10),
            },
            ExperimentParetoCandidate {
                experiment_id: "E".into(),
                experiment_cost: rational(1, 10),
                nominal_decision_loss: rational(1, 4),
                realized_decision_loss: rational(1, 10),
            },
        ];

        let frontier = compute_experiment_pareto_frontier(&candidates).unwrap();
        assert_eq!(
            frontier.nominal_frontier_ids,
            vec!["A".to_string(), "B".to_string()]
        );
        assert_eq!(
            frontier.nominal_dominated_ids,
            vec!["C".to_string(), "D".to_string(), "E".to_string()]
        );
        // D and E are equivalent and both remain; they jointly dominate every
        // alternative under the realized channel.
        assert_eq!(
            frontier.realized_frontier_ids,
            vec!["D".to_string(), "E".to_string()]
        );
        assert_eq!(
            frontier.realized_dominated_ids,
            vec!["A".to_string(), "B".to_string(), "C".to_string()]
        );
    }

    #[test]
    fn pareto_frontier_rejects_empty_duplicate_and_oversized_inputs() {
        assert!(compute_experiment_pareto_frontier(&[]).is_err());
        let duplicate = vec![
            ExperimentParetoCandidate {
                experiment_id: "same".into(),
                experiment_cost: Rational::zero(),
                nominal_decision_loss: Rational::zero(),
                realized_decision_loss: Rational::zero(),
            },
            ExperimentParetoCandidate {
                experiment_id: "same".into(),
                experiment_cost: Rational::one(),
                nominal_decision_loss: Rational::one(),
                realized_decision_loss: Rational::one(),
            },
        ];
        assert!(compute_experiment_pareto_frontier(&duplicate).is_err());

        let oversized = (0..=MAX_PARETO_EXPERIMENTS)
            .map(|index| ExperimentParetoCandidate {
                experiment_id: format!("candidate-{index:04}"),
                experiment_cost: Rational::from_u64(index as u64),
                nominal_decision_loss: Rational::from_u64(index as u64),
                realized_decision_loss: Rational::from_u64(index as u64),
            })
            .collect::<Vec<_>>();
        assert!(compute_experiment_pareto_frontier(&oversized).is_err());
    }

    #[test]
    fn inference_model_and_interaction_misspecification_are_separately_identifiable() {
        let prior = vec![rational(1, 2), rational(1, 2)];
        let loss = binary_decisions();
        let perfect = perfect_channel("perfect");
        let inverted = inverted_channel("inverted");

        // Inference-only: the working and true data models are identical,
        // but the approximate posterior rule reverses both observations.
        let inverted_rule = posterior_rule(
            &perfect,
            "posterior-inverted",
            vec![
                vec![Rational::zero(), Rational::one()],
                vec![Rational::one(), Rational::zero()],
            ],
        );
        let inference_only = evaluate_channel_decisions_with_inference_rule(
            &perfect,
            &perfect,
            &prior,
            &loss,
            &inverted_rule,
        )
        .unwrap();
        assert_eq!(inference_only.realized_policy_loss, Rational::one());
        assert_eq!(
            inference_only.inference_rule_disagreement_probability,
            Rational::one()
        );
        assert_eq!(
            inference_only.expected_inference_posterior_tv_distance,
            Rational::one()
        );
        assert_eq!(
            inference_only.expected_model_posterior_tv_distance,
            Rational::zero()
        );
        assert_eq!(
            inference_only.expected_total_posterior_tv_distance,
            Rational::one()
        );
        assert_eq!(
            inference_only.expected_total_posterior_brier_score,
            Rational::from_u64(2)
        );
        assert_eq!(
            inference_only.observations[0].inference_posterior,
            Some(vec![Rational::zero(), Rational::one()])
        );
        assert_eq!(
            inference_only.observations[0].assumed_model_posterior,
            Some(vec![Rational::one(), Rational::zero()])
        );
        assert_eq!(
            inference_only.observations[0].true_posterior,
            vec![Rational::one(), Rational::zero()]
        );

        // Model-only: the inference rule is exact Bayes for the assumed
        // channel, but the true data-generating channel is inverted.
        let model_only = evaluate_channel_decisions(&perfect, &inverted, &prior, &loss).unwrap();
        assert_eq!(model_only.realized_policy_loss, Rational::one());
        assert_eq!(
            model_only.inference_rule_disagreement_probability,
            Rational::zero()
        );
        assert_eq!(
            model_only.expected_inference_posterior_tv_distance,
            Rational::zero()
        );
        assert_eq!(
            model_only.expected_model_posterior_tv_distance,
            Rational::one()
        );
        assert_eq!(
            model_only.expected_total_posterior_tv_distance,
            Rational::one()
        );
        assert_eq!(
            model_only.expected_total_posterior_brier_score,
            Rational::from_u64(2)
        );

        // Interaction: model channel is misspecified and the inference rule
        // is additionally overconfident relative to exact Bayes under it.
        let assumed_noisy = noisy_channel("assumed-noisy");
        let overconfident_rule = posterior_rule(
            &assumed_noisy,
            "overconfident-posterior",
            vec![
                vec![rational(9, 10), rational(1, 10)],
                vec![rational(1, 10), rational(9, 10)],
            ],
        );
        let interaction = evaluate_channel_decisions_with_inference_rule(
            &assumed_noisy,
            &inverted,
            &prior,
            &loss,
            &overconfident_rule,
        )
        .unwrap();
        assert_eq!(interaction.nominal_model_expected_loss, rational(1, 10));
        assert_eq!(interaction.realized_policy_loss, Rational::one());
        assert_eq!(interaction.decision_regret, Rational::one());
        assert_eq!(
            interaction.expected_inference_posterior_tv_distance,
            rational(1, 10)
        );
        assert_eq!(
            interaction.expected_model_posterior_tv_distance,
            rational(4, 5)
        );
        assert_eq!(
            interaction.expected_total_posterior_tv_distance,
            rational(9, 10)
        );
        assert_eq!(
            interaction.expected_total_posterior_brier_score,
            rational(81, 50)
        );
        assert_eq!(
            interaction.inference_rule_disagreement_probability,
            Rational::zero()
        );

        // Correctly specified negative control with exact Bayesian inference.
        let matched = evaluate_channel_decisions(&perfect, &perfect, &prior, &loss).unwrap();
        assert_eq!(matched.realized_policy_loss, Rational::zero());
        assert_eq!(matched.decision_regret, Rational::zero());
        assert_eq!(
            matched.expected_inference_posterior_tv_distance,
            Rational::zero()
        );
        assert_eq!(
            matched.expected_model_posterior_tv_distance,
            Rational::zero()
        );
        assert_eq!(
            matched.expected_total_posterior_tv_distance,
            Rational::zero()
        );
        assert_eq!(
            matched.expected_total_posterior_brier_score,
            Rational::zero()
        );

        let exact_rule = exact_bayes_posterior_rule(&perfect, &prior).unwrap();
        let pair = compare_experiment_pair_with_inference_rules(
            &perfect,
            &perfect,
            &inverted_rule,
            rational(1, 10),
            &perfect,
            &perfect,
            &exact_rule,
            rational(1, 10),
            &prior,
            &loss,
        )
        .unwrap();
        assert_eq!(pair.information_preference, ExperimentPreference::Tie);
        assert_eq!(pair.realized_total_loss_preference, ExperimentPreference::B);
        assert_eq!(pair.experiment_a.inference_rule_id, "posterior-inverted");
        assert_eq!(pair.experiment_b.inference_rule_id, exact_rule.rule_id);
    }

    fn likelihood_proposal_samples() -> Vec<GibbsImportanceSample> {
        vec![
            GibbsImportanceSample {
                state_index: 0,
                observation_index: 0,
            },
            GibbsImportanceSample {
                state_index: 0,
                observation_index: 0,
            },
            GibbsImportanceSample {
                state_index: 0,
                observation_index: 0,
            },
            GibbsImportanceSample {
                state_index: 0,
                observation_index: 0,
            },
            GibbsImportanceSample {
                state_index: 0,
                observation_index: 1,
            },
            GibbsImportanceSample {
                state_index: 1,
                observation_index: 0,
            },
            GibbsImportanceSample {
                state_index: 1,
                observation_index: 1,
            },
            GibbsImportanceSample {
                state_index: 1,
                observation_index: 1,
            },
            GibbsImportanceSample {
                state_index: 1,
                observation_index: 1,
            },
            GibbsImportanceSample {
                state_index: 1,
                observation_index: 1,
            },
        ]
    }

    #[test]
    fn exact_gibbs_eig_reduces_to_mutual_information_at_unit_log_likelihood() {
        let prior = vec![rational(1, 2), rational(1, 2)];
        let channel = noisy_channel("gibbs-unit-likelihood");
        let factors = vec![
            vec![channel.probabilities[0][0], channel.probabilities[1][0]],
            vec![channel.probabilities[0][1], channel.probabilities[1][1]],
        ];
        let exact =
            compute_gibbs_eig_exact_finite("ordinary-bayes", &channel, &prior, &factors).unwrap();
        let mutual_information_nats =
            mutual_information_bits(&channel, &prior).unwrap() * std::f64::consts::LN_2;
        assert!((exact.value_nats - mutual_information_nats).abs() < 1.0e-12);
        assert_eq!(exact.pseudo_predictive_normalizer, Rational::one());
    }

    #[test]
    fn exact_gibbs_eig_is_zero_at_zero_learning_rate() {
        let prior = vec![rational(1, 2), rational(1, 2)];
        let channel = noisy_channel("gibbs-zero-rate");
        let factors = vec![
            vec![Rational::one(), Rational::one()],
            vec![Rational::one(), Rational::one()],
        ];
        let exact =
            compute_gibbs_eig_exact_finite("zero-learning-rate", &channel, &prior, &factors)
                .unwrap();
        assert_eq!(exact.value_nats, 0.0);
        assert_eq!(
            exact.normalized_pseudo_predictive,
            vec![rational(1, 2), rational(1, 2)]
        );
    }

    #[test]
    fn importance_sampler_has_unit_ess_when_proposal_matches_gibbs_likelihood() {
        let prior = vec![rational(1, 2), rational(1, 2)];
        let channel = noisy_channel("importance-matched");
        let factors = vec![
            vec![channel.probabilities[0][0], channel.probabilities[1][0]],
            vec![channel.probabilities[0][1], channel.probabilities[1][1]],
        ];
        let samples = likelihood_proposal_samples();
        let receipt = estimate_gibbs_eig_importance_from_samples(
            "working-model-proposal",
            &channel,
            &prior,
            &factors,
            &channel.probabilities,
            &samples,
        )
        .unwrap();
        assert_eq!(receipt.effective_sample_size, Rational::from_u64(10));
        assert_eq!(receipt.effective_sample_size_fraction, 1.0);
        assert!(
            (receipt.estimated_gibbs_eig_nats - receipt.exact_oracle_gibbs_eig_nats).abs()
                < 1.0e-12
        );
        assert_eq!(receipt.maximum_normalized_weight, rational(1, 10));
    }

    #[test]
    fn low_learning_rate_can_reduce_model_proposal_ess_without_changing_zero_geig() {
        let prior = vec![rational(1, 2), rational(1, 2)];
        let channel = noisy_channel("importance-low-rate");
        let constant_factors = vec![
            vec![Rational::one(), Rational::one()],
            vec![Rational::one(), Rational::one()],
        ];
        let samples = likelihood_proposal_samples();
        let exact =
            compute_gibbs_eig_exact_finite("omega-zero", &channel, &prior, &constant_factors)
                .unwrap();
        assert_eq!(exact.value_nats, 0.0);

        let receipt = estimate_gibbs_eig_importance_from_samples(
            "working-model-proposal",
            &channel,
            &prior,
            &constant_factors,
            &channel.probabilities,
            &samples,
        )
        .unwrap();
        assert_eq!(receipt.effective_sample_size, rational(32, 5));
        assert_eq!(receipt.effective_sample_size_fraction, 0.64);
        assert_eq!(receipt.estimated_gibbs_eig_nats, 0.0);
        assert_eq!(receipt.exact_oracle_gibbs_eig_nats, 0.0);
    }

    #[test]
    fn importance_sampler_reports_exact_weight_degeneracy_and_oracle_error() {
        let prior = vec![rational(1, 2), rational(1, 2)];
        let channel = noisy_channel("importance-weighted");
        let factors = vec![
            vec![Rational::from_u64(3), Rational::one()],
            vec![Rational::one(), Rational::from_u64(3)],
        ];
        let samples = likelihood_proposal_samples();
        let receipt = estimate_gibbs_eig_importance_from_samples(
            "working-model-proposal",
            &channel,
            &prior,
            &factors,
            &channel.probabilities,
            &samples,
        )
        .unwrap();
        assert_eq!(receipt.effective_sample_size, rational(128, 13));
        assert!(receipt.effective_sample_size_fraction < 1.0);
        assert!(receipt.absolute_error_nats < 1.0e-12);
    }

    #[test]
    fn gibbs_eig_importance_rejects_proposal_support_gaps_and_empty_samples() {
        let prior = vec![rational(1, 2), rational(1, 2)];
        let channel = noisy_channel("importance-support-gap");
        let factors = vec![
            vec![Rational::one(), Rational::one()],
            vec![Rational::one(), Rational::one()],
        ];
        assert!(
            estimate_gibbs_eig_importance_from_samples(
                "empty",
                &channel,
                &prior,
                &factors,
                &channel.probabilities,
                &[],
            )
            .is_err()
        );

        let proposal = vec![
            vec![Rational::one(), Rational::zero()],
            vec![Rational::one(), Rational::zero()],
        ];
        assert!(
            estimate_gibbs_eig_importance_from_samples(
                "support-gap",
                &channel,
                &prior,
                &factors,
                &proposal,
                &[GibbsImportanceSample {
                    state_index: 0,
                    observation_index: 0
                }],
            )
            .is_err()
        );
    }

    #[test]
    fn generalized_bayes_weights_reduce_to_exact_bayes_when_equal_to_likelihoods() {
        let prior = vec![rational(1, 2), rational(1, 2)];
        let assumed = noisy_channel("gibbs-equivalent");
        let likelihood_weights = vec![
            vec![assumed.probabilities[0][0], assumed.probabilities[1][0]],
            vec![assumed.probabilities[0][1], assumed.probabilities[1][1]],
        ];
        let gibbs = generalized_bayes_posterior_rule_from_weights(
            "gibbs-equals-bayes",
            &assumed,
            &prior,
            &likelihood_weights,
        )
        .unwrap();
        let bayes = exact_bayes_posterior_rule(&assumed, &prior).unwrap();
        assert_eq!(
            gibbs.posterior_by_observation,
            bayes.posterior_by_observation
        );
    }

    #[test]
    fn generalized_bayes_weights_produce_exact_loss_based_posteriors() {
        let prior = vec![rational(1, 2), rational(1, 2)];
        let assumed = noisy_channel("gibbs-loss-based");
        let factors = vec![
            vec![Rational::from_u64(3), Rational::one()],
            vec![Rational::one(), Rational::from_u64(3)],
        ];
        let rule = generalized_bayes_posterior_rule_from_weights(
            "gibbs-tempered",
            &assumed,
            &prior,
            &factors,
        )
        .unwrap();
        assert_eq!(
            rule.posterior_by_observation,
            vec![
                vec![rational(3, 4), rational(1, 4)],
                vec![rational(1, 4), rational(3, 4)],
            ]
        );

        let evaluation = evaluate_channel_decisions_with_inference_rule(
            &assumed,
            &assumed,
            &prior,
            &binary_decisions(),
            &rule,
        )
        .unwrap();
        assert_eq!(
            evaluation.expected_inference_posterior_tv_distance,
            rational(1, 20)
        );
        assert_eq!(evaluation.realized_policy_loss, rational(1, 5));
    }

    #[test]
    fn generalized_bayes_weights_reject_malformed_and_zero_normalizers() {
        let prior = vec![rational(1, 2), rational(1, 2)];
        let assumed = noisy_channel("gibbs-invalid");
        assert!(
            generalized_bayes_posterior_rule_from_weights(
                "bad-dimension",
                &assumed,
                &prior,
                &[vec![Rational::one()]],
            )
            .is_err()
        );
        assert!(
            generalized_bayes_posterior_rule_from_weights(
                "zero-normalizer",
                &assumed,
                &prior,
                &[
                    vec![Rational::zero(), Rational::zero()],
                    vec![Rational::one(), Rational::one()],
                ],
            )
            .is_err()
        );
    }

    #[test]
    fn inference_rule_contract_rejects_wrong_binding_and_unnormalized_posteriors() {
        let assumed = perfect_channel("perfect");
        let mut wrong_binding = posterior_rule(
            &assumed,
            "wrong-binding",
            vec![
                vec![Rational::one(), Rational::zero()],
                vec![Rational::zero(), Rational::one()],
            ],
        );
        wrong_binding.assumed_channel_id = "other-channel".into();
        assert!(wrong_binding.validate(&assumed).is_err());

        let unnormalized = posterior_rule(
            &assumed,
            "unnormalized",
            vec![
                vec![rational(3, 4), rational(1, 4)],
                vec![rational(1, 2), rational(1, 4)],
            ],
        );
        assert!(unnormalized.validate(&assumed).is_err());
    }

    #[test]
    fn unsupported_observations_fail_closed_to_explicit_abstention() {
        let prior = vec![rational(1, 2), rational(1, 2)];
        let assumed = noisy_channel("assumed");
        let truth = inverted_channel("truth");
        let evaluation =
            evaluate_channel_decisions(&assumed, &truth, &prior, &binary_decisions()).unwrap();
        assert_eq!(
            evaluation.unsupported_observation_probability,
            Rational::zero()
        );

        let partial_assumption = FiniteObservationChannel {
            channel_id: "partial".into(),
            state_ids: vec!["s0".into(), "s1".into()],
            observation_ids: vec!["o0".into(), "o1".into()],
            probabilities: vec![
                vec![Rational::one(), Rational::zero()],
                vec![Rational::one(), Rational::zero()],
            ],
        };
        let unsupported =
            evaluate_channel_decisions(&partial_assumption, &truth, &prior, &binary_decisions())
                .unwrap();
        assert_eq!(
            unsupported.unsupported_observation_probability,
            rational(1, 2)
        );
        assert_eq!(unsupported.realized_policy_loss, rational(1, 2));
    }

    #[test]
    fn invalid_garbling_and_probability_mass_fail_closed() {
        let perfect = perfect_channel("perfect");
        let noisy = noisy_channel("noisy");
        let invalid = BlackwellGarblingWitness {
            probabilities: vec![
                vec![Rational::one(), Rational::zero()],
                vec![Rational::one(), Rational::zero()],
            ],
        };
        assert!(verify_blackwell_dominance(&perfect, &noisy, &invalid).is_err());

        let mut invalid_channel = noisy_channel("bad-channel");
        invalid_channel.probabilities[0][0] = Rational::new(2, 1).unwrap();
        assert!(invalid_channel.validate().is_err());
    }
}
