//! Exact finite-channel experiment ordering under model misspecification.
//!
//! This module separates the channel used for inference from the channel that
//! generates observations. It validates nominal Blackwell dominance using an
//! explicit stochastic garbling witness, then scores the chosen terminal
//! decisions under the true channel. Mutual information is provided only as a
//! floating-point diagnostic; posterior, decision and risk receipts use exact
//! rational arithmetic.

use crate::Rational;
use std::collections::BTreeSet;

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
                return Err(format!("channel state row {state} has the wrong observation dimension"));
            }
            let mut mass = Rational::zero();
            for probability in row {
                if !probability.is_probability() {
                    return Err(format!("channel state row {state} has an invalid probability"));
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
                return Err(format!("garbling row {source_observation} contains an invalid probability"));
            }
            row_mass = row_mass.checked_add(*probability)?;
        }
        if row_mass != Rational::one() {
            return Err(format!("garbling row {source_observation} must sum exactly to one"));
        }
    }

    for (state, more_row) in more_informative.probabilities.iter().enumerate() {
        let less_row = &less_informative.probabilities[state];
        for (less_observation, target_probability) in less_row.iter().copied().enumerate() {
            let mut reconstructed = Rational::zero();
            for (more_observation, more_probability) in more_row.iter().copied().enumerate() {
                reconstructed = reconstructed.checked_add(
                    more_probability.checked_mul(
                        witness.probabilities[more_observation][less_observation],
                    )?,
                )?;
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

#[derive(Clone, Debug, Eq, PartialEq)]
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
            return Err("inference rule state/observation vocabularies do not match the assumed channel".into());
        }
        if self.posterior_by_observation.len() != assumed.observation_ids.len() {
            return Err("inference rule must provide one posterior for every observation".into());
        }
        for (observation, posterior) in self.posterior_by_observation.iter().enumerate() {
            if posterior.len() != assumed.state_ids.len() {
                return Err(format!("inference posterior {observation} has the wrong state dimension"));
            }
            let mut total = Rational::zero();
            for probability in posterior {
                if !probability.is_probability() {
                    return Err(format!("inference posterior {observation} contains an invalid probability"));
                }
                total = total.checked_add(*probability)?;
            }
            if total != Rational::one() {
                return Err(format!("inference posterior {observation} must sum exactly to one"));
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
    /// Total-variation distance between the supplied inference posterior and
    /// exact Bayes posterior under the working channel; None means unsupported.
    pub inference_posterior_tv_distance: Option<Rational>,
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
    /// True-observation-frequency-weighted posterior approximation error.
    pub expected_inference_posterior_tv_distance: Rational,
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
        return Err("assumed and true channels must share the same ordered state vocabulary".into());
    }
    if assumed.observation_ids != truth.observation_ids {
        return Err("assumed and true channels must share the same observable symbol vocabulary".into());
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
    let mut observations = Vec::new();

    for (observation, observation_id) in assumed.observation_ids.iter().enumerate() {
        let mut true_probability = Rational::zero();
        let mut assumed_probability = Rational::zero();
        let mut true_joint = vec![Rational::zero(); prior.len()];
        let mut assumed_joint = vec![Rational::zero(); prior.len()];

        for (state, prior_mass) in prior.iter().copied().enumerate() {
            let true_mass = prior_mass.checked_mul(truth.probabilities[state][observation])?;
            let assumed_mass =
                prior_mass.checked_mul(assumed.probabilities[state][observation])?;
            true_joint[state] = true_mass;
            assumed_joint[state] = assumed_mass;
            true_probability = true_probability.checked_add(true_mass)?;
            assumed_probability = assumed_probability.checked_add(assumed_mass)?;
        }

        let (inference_decision, inference_conditional_loss) =
            if assumed_probability.is_zero() {
                (None, table.abstention_loss)
            } else {
                bayes_decision(
                    table,
                    &inference_rule.posterior_by_observation[observation],
                )?
            };
        nominal_model_expected_loss = nominal_model_expected_loss.checked_add(
            assumed_probability.checked_mul(inference_conditional_loss)?,
        )?;

        if true_probability.is_zero() {
            continue;
        }

        let true_posterior: Vec<Rational> = true_joint
            .iter()
            .map(|mass| mass.checked_div(true_probability))
            .collect::<Result<_, _>>()?;
        let (true_bayes_decision, true_bayes_loss) = bayes_decision(table, &true_posterior)?;

        let (assumed_model_bayes_decision, inference_posterior_tv_distance) =
            if assumed_probability.is_zero() {
                unsupported_observation_probability =
                    unsupported_observation_probability.checked_add(true_probability)?;
                (None, None)
            } else {
                let assumed_posterior: Vec<Rational> = assumed_joint
                    .iter()
                    .map(|mass| mass.checked_div(assumed_probability))
                    .collect::<Result<_, _>>()?;
                let (model_decision, _) = bayes_decision(table, &assumed_posterior)?;
                if model_decision != inference_decision {
                    inference_rule_disagreement_probability =
                        inference_rule_disagreement_probability.checked_add(true_probability)?;
                }
                let mut l1_distance = Rational::zero();
                for (inference_probability, model_probability) in inference_rule
                    .posterior_by_observation[observation]
                    .iter()
                    .zip(assumed_posterior.iter())
                {
                    let difference = if inference_probability >= model_probability {
                        subtract_nonnegative(*inference_probability, *model_probability)?
                    } else {
                        subtract_nonnegative(*model_probability, *inference_probability)?
                    };
                    l1_distance = l1_distance.checked_add(difference)?;
                }
                let posterior_tv = l1_distance.checked_div(Rational::from_u64(2))?;
                expected_inference_posterior_tv_distance =
                    expected_inference_posterior_tv_distance
                        .checked_add(true_probability.checked_mul(posterior_tv)?)?;
                (Some(model_decision), Some(posterior_tv))
            };
        let realized_conditional_loss =
            expected_loss_for_posterior(table, &true_posterior, inference_decision)?;
        if inference_decision != true_bayes_decision {
            decision_disagreement_probability =
                decision_disagreement_probability.checked_add(true_probability)?;
        }

        expected_assumed_loss = expected_assumed_loss.checked_add(
            true_probability.checked_mul(inference_conditional_loss)?,
        )?;
        realized_policy_loss = realized_policy_loss.checked_add(
            true_probability.checked_mul(realized_conditional_loss)?,
        )?;
        true_bayes_oracle_loss =
            true_bayes_oracle_loss.checked_add(true_probability.checked_mul(true_bayes_loss)?)?;

        observations.push(ObservationDecisionReceipt {
            observation_id: observation_id.clone(),
            true_observation_probability: true_probability,
            assumed_observation_probability: assumed_probability,
            inference_decision,
            assumed_model_bayes_decision,
            inference_posterior_tv_distance,
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
    evaluate_channel_decisions_with_inference_rule(
        assumed,
        truth,
        prior,
        table,
        &inference_rule,
    )
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
            Ok(Self::from_parts(
                false,
                subtract_nonnegative(left, right)?,
            ))
        } else {
            Ok(Self::from_parts(
                true,
                subtract_nonnegative(right, left)?,
            ))
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
        return Err("binary Blackwell solver requires exactly two states and two observations per channel".into());
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
        let b0 = a.checked_mul(x).unwrap()
            .checked_add(one_minus(a).checked_mul(y).unwrap())
            .unwrap();
        let b1 = c.checked_mul(x).unwrap()
            .checked_add(one_minus(c).checked_mul(y).unwrap())
            .unwrap();
        let more = FiniteObservationChannel {
            channel_id: "more-asymmetric".into(),
            state_ids: vec!["s0".into(), "s1".into()],
            observation_ids: vec!["z0".into(), "z1".into()],
            probabilities: vec![
                vec![a, one_minus(a)],
                vec![c, one_minus(c)],
            ],
        };
        let less = FiniteObservationChannel {
            channel_id: "less-asymmetric".into(),
            state_ids: vec!["s0".into(), "s1".into()],
            observation_ids: vec!["y0".into(), "y1".into()],
            probabilities: vec![
                vec![b0, one_minus(b0)],
                vec![b1, one_minus(b1)],
            ],
        };
        let receipt = solve_binary_blackwell_garbling(&more, &less)
            .unwrap()
            .unwrap();
        assert_eq!(
            receipt.witness.probabilities,
            vec![
                vec![x, one_minus(x)],
                vec![y, one_minus(y)],
            ]
        );
    }

    #[test]
    fn binary_blackwell_solver_rejects_non_dominance_and_inconsistent_singular_case() {
        let noisy = noisy_channel("noisy");
        let perfect = perfect_channel("perfect");
        assert!(solve_binary_blackwell_garbling(&noisy, &perfect)
            .unwrap()
            .is_none());

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
        assert!(solve_binary_blackwell_garbling(&uninformative, &inconsistent_target)
            .unwrap()
            .is_none());
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
        let receipt = solve_binary_blackwell_garbling(
            &uninformative,
            &same_uninformative_target,
        )
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

    #[test]
    fn more_nominal_information_can_produce_greater_realized_decision_loss() {
        let prior = vec![rational(1, 2), rational(1, 2)];
        let loss = binary_decisions();
        let assumed_perfect = perfect_channel("A-perfect-assumed");
        let true_inverted = inverted_channel("A-inverted-truth");
        let assumed_noisy = noisy_channel("B-noisy-assumed");
        let true_noisy = noisy_channel("B-noisy-truth");

        let eig_a = mutual_information_bits(&assumed_perfect, &prior).unwrap();
        let eig_b = mutual_information_bits(&assumed_noisy, &prior).unwrap();
        assert!(eig_a > eig_b);

        let evaluation_a = evaluate_channel_decisions(
            &assumed_perfect,
            &true_inverted,
            &prior,
            &loss,
        )
        .unwrap();
        let evaluation_b = evaluate_channel_decisions(
            &assumed_noisy,
            &true_noisy,
            &prior,
            &loss,
        )
        .unwrap();

        assert_eq!(evaluation_a.nominal_model_expected_loss, Rational::zero());
        assert_eq!(evaluation_a.expected_assumed_loss_under_true_observations, Rational::zero());
        assert_eq!(evaluation_a.realized_policy_loss, Rational::one());
        assert_eq!(evaluation_a.decision_regret, Rational::one());
        assert_eq!(evaluation_a.decision_disagreement_probability, Rational::one());

        assert_eq!(evaluation_b.nominal_model_expected_loss, rational(1, 5));
        assert_eq!(evaluation_b.realized_policy_loss, rational(1, 5));
        assert_eq!(evaluation_b.true_bayes_oracle_loss, rational(1, 5));
        assert_eq!(evaluation_b.decision_regret, Rational::zero());
        assert!(evaluation_a.realized_policy_loss > evaluation_b.realized_policy_loss);
    }

    #[test]
    fn inference_mismatch_and_model_mismatch_are_separately_identifiable() {
        let prior = vec![rational(1, 2), rational(1, 2)];
        let loss = binary_decisions();
        let perfect = perfect_channel("perfect");
        let inverted = inverted_channel("inverted");

        // Correct data model, wrong inference model.
        let inference_only =
            evaluate_channel_decisions(&inverted, &perfect, &prior, &loss).unwrap();
        assert_eq!(inference_only.nominal_model_expected_loss, Rational::zero());
        assert_eq!(inference_only.realized_policy_loss, Rational::one());

        // Exact inference, wrong data-generating model.
        let model_only =
            evaluate_channel_decisions(&perfect, &inverted, &prior, &loss).unwrap();
        assert_eq!(model_only.nominal_model_expected_loss, Rational::zero());
        assert_eq!(model_only.realized_policy_loss, Rational::one());

        // Both inference and the data-generating channel differ from the nominal perfect model.
        let approximate_inference = noisy_channel("approximate-inference");
        let interaction =
            evaluate_channel_decisions(&approximate_inference, &inverted, &prior, &loss).unwrap();
        assert_eq!(interaction.nominal_model_expected_loss, rational(1, 5));
        assert_eq!(interaction.expected_assumed_loss_under_true_observations, rational(1, 5));
        assert_eq!(interaction.realized_policy_loss, Rational::one());
        assert_eq!(interaction.decision_regret, Rational::one());

        // Correct negative control.
        let matched = evaluate_channel_decisions(&perfect, &perfect, &prior, &loss).unwrap();
        assert_eq!(matched.realized_policy_loss, Rational::zero());
        assert_eq!(matched.decision_regret, Rational::zero());
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
        let unsupported = evaluate_channel_decisions(
            &partial_assumption,
            &truth,
            &prior,
            &binary_decisions(),
        )
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
