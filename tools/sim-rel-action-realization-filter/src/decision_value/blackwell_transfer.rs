//! Constructive exact decision-rule transfer for Blackwell-dominated channels.
//!
//! A verified garbling witness is more than a pairwise channel-order label: it
//! lets us compile any (possibly randomized) decision rule for the less
//! informative experiment into a rule for the more informative experiment.
//! The lifted rule first samples the garbling output conditional on the richer
//! observation, then applies the original rule. Its exact expected loss must be
//! identical. Optimizing over rules after the richer observation can therefore
//! only improve Bayes risk.
//!
//! All probabilities and losses in this module are exact rationals. The
//! certificate is conditional on the supplied finite channels, prior, loss
//! table and exact garbling witness; it says nothing about a shifted true
//! observation channel.

use super::experiment_ordering::{
    BlackwellDominanceReceipt, BlackwellGarblingWitness, ChannelDecisionLossTable,
    FiniteObservationChannel, verify_blackwell_dominance,
};
use crate::Rational;

/// A randomized terminal policy indexed by public observation and decision.
///
/// Each observation row distributes unit mass over the concrete decisions and
/// explicit abstention. Keeping abstention as a separate column avoids
/// smuggling escalation into a synthetic decision with an arbitrary loss.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RandomizedTerminalDecisionRule {
    pub rule_id: String,
    pub channel_id: String,
    pub observation_ids: Vec<String>,
    pub decision_ids: Vec<String>,
    /// Conditional decision probabilities indexed [observation][decision].
    pub decision_probabilities: Vec<Vec<Rational>>,
    /// Conditional abstention probability indexed [observation].
    pub abstention_probabilities: Vec<Rational>,
}

impl RandomizedTerminalDecisionRule {
    pub fn validate(
        &self,
        channel: &FiniteObservationChannel,
        losses: &ChannelDecisionLossTable,
    ) -> Result<(), String> {
        channel.validate()?;
        losses.validate(channel.state_ids.len())?;
        if self.rule_id.trim().is_empty() {
            return Err("decision-rule ID must be non-empty".into());
        }
        if self.channel_id != channel.channel_id {
            return Err("decision rule is not bound to the supplied channel".into());
        }
        if self.observation_ids != channel.observation_ids {
            return Err("decision-rule observation vocabulary/order mismatch".into());
        }
        if self.decision_ids != losses.decision_ids {
            return Err("decision-rule decision vocabulary/order mismatch".into());
        }
        if self.decision_probabilities.len() != channel.observation_ids.len()
            || self.abstention_probabilities.len() != channel.observation_ids.len()
        {
            return Err("decision rule must contain one row per observation".into());
        }

        for (observation, row) in self.decision_probabilities.iter().enumerate() {
            if row.len() != losses.decision_ids.len() {
                return Err(format!(
                    "decision-rule row {observation} has the wrong decision dimension"
                ));
            }
            let abstention = self.abstention_probabilities[observation];
            if !abstention.is_probability() {
                return Err(format!(
                    "decision-rule row {observation} has an invalid abstention probability"
                ));
            }
            let mut row_mass = abstention;
            for probability in row {
                if !probability.is_probability() {
                    return Err(format!(
                        "decision-rule row {observation} contains an invalid decision probability"
                    ));
                }
                row_mass = row_mass.checked_add(*probability)?;
            }
            if row_mass != Rational::one() {
                return Err(format!(
                    "decision-rule row {observation} must sum exactly to one including abstention"
                ));
            }
        }
        Ok(())
    }
}

/// Per-observation risk accounting for one fixed randomized terminal rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationRuleRisk {
    pub observation_id: String,
    /// Marginal observation probability under the supplied channel and prior.
    pub observation_probability: Rational,
    /// Conditional expected loss; absent only for a zero-probability symbol.
    pub conditional_loss: Option<Rational>,
    /// Unconditional contribution to expected terminal loss.
    pub risk_contribution: Rational,
}

/// Exact expected-loss receipt for a randomized terminal rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RandomizedRuleRiskReceipt {
    pub channel_id: String,
    pub rule_id: String,
    pub expected_loss: Rational,
    pub observations: Vec<ObservationRuleRisk>,
}

/// The exact Bayes-optimal deterministic rule for one finite channel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactBayesRiskReceipt {
    pub rule: RandomizedTerminalDecisionRule,
    pub evaluation: RandomizedRuleRiskReceipt,
}

/// A constructive Blackwell certificate.
///
/// The source rule on the less-informative channel and its lifted rule on the
/// more-informative channel have exactly equal expected loss. The two Bayes
/// risks are also computed independently, and the richer channel's risk must
/// be no larger.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlackwellDecisionTransferCertificate {
    pub dominance: BlackwellDominanceReceipt,
    pub source_rule: RandomizedTerminalDecisionRule,
    pub transferred_rule: RandomizedTerminalDecisionRule,
    pub source_rule_risk: RandomizedRuleRiskReceipt,
    pub transferred_rule_risk: RandomizedRuleRiskReceipt,
    pub less_informative_bayes: ExactBayesRiskReceipt,
    pub more_informative_bayes: ExactBayesRiskReceipt,
    /// Exact difference: Bayes risk of the less informative channel minus
    /// Bayes risk of the more informative channel.
    pub bayes_risk_advantage: Rational,
    pub transfer_preserves_expected_loss: bool,
    pub bayes_risk_monotonicity_verified: bool,
}

fn validate_prior(prior: &[Rational], state_count: usize) -> Result<(), String> {
    if prior.is_empty() || prior.len() != state_count {
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

fn checked_sub_nonnegative(left: Rational, right: Rational) -> Result<Rational, String> {
    if left < right {
        return Err("exact non-negative subtraction would underflow".into());
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
    let numerator = left_numerator
        .checked_sub(right_numerator)
        .ok_or_else(|| "rational subtraction would underflow".to_string())?;
    Rational::new(numerator, denominator)
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

/// Evaluate a fixed randomized rule using unnormalized joint path masses.
///
/// This formulation sums P(state) P(observation | state) P(decision |
/// observation) loss directly. It does not normalize a posterior and therefore
/// remains well-defined for zero-probability observation symbols.
pub fn evaluate_randomized_terminal_rule(
    channel: &FiniteObservationChannel,
    prior: &[Rational],
    losses: &ChannelDecisionLossTable,
    rule: &RandomizedTerminalDecisionRule,
) -> Result<RandomizedRuleRiskReceipt, String> {
    channel.validate()?;
    losses.validate(channel.state_ids.len())?;
    validate_prior(prior, channel.state_ids.len())?;
    rule.validate(channel, losses)?;

    let mut expected_loss = Rational::zero();
    let mut observations = Vec::with_capacity(channel.observation_ids.len());

    for (observation, observation_id) in channel.observation_ids.iter().enumerate() {
        let mut observation_probability = Rational::zero();
        let mut risk_contribution = Rational::zero();

        for (state, prior_mass) in prior.iter().copied().enumerate() {
            let state_observation_mass =
                prior_mass.checked_mul(channel.probabilities[state][observation])?;
            observation_probability =
                observation_probability.checked_add(state_observation_mass)?;

            let mut conditional_action_loss = Rational::zero();
            for (decision, probability) in rule.decision_probabilities[observation]
                .iter()
                .copied()
                .enumerate()
            {
                conditional_action_loss = conditional_action_loss
                    .checked_add(probability.checked_mul(losses.losses[decision][state])?)?;
            }
            conditional_action_loss = conditional_action_loss.checked_add(
                rule.abstention_probabilities[observation].checked_mul(losses.abstention_loss)?,
            )?;
            risk_contribution = risk_contribution
                .checked_add(state_observation_mass.checked_mul(conditional_action_loss)?)?;
        }

        let conditional_loss = if observation_probability.is_zero() {
            if !risk_contribution.is_zero() {
                return Err("zero-probability observation has non-zero risk contribution".into());
            }
            None
        } else {
            Some(risk_contribution.checked_div(observation_probability)?)
        };
        expected_loss = expected_loss.checked_add(risk_contribution)?;
        observations.push(ObservationRuleRisk {
            observation_id: observation_id.clone(),
            observation_probability,
            conditional_loss,
            risk_contribution,
        });
    }

    Ok(RandomizedRuleRiskReceipt {
        channel_id: channel.channel_id.clone(),
        rule_id: rule.rule_id.clone(),
        expected_loss,
        observations,
    })
}

/// Construct the exact deterministic Bayes rule for a supplied channel.
///
/// Ties are resolved in favour of explicit abstention, then by the declared
/// order of concrete decisions. Zero-probability observations abstain.
pub fn exact_bayes_risk(
    channel: &FiniteObservationChannel,
    prior: &[Rational],
    losses: &ChannelDecisionLossTable,
) -> Result<ExactBayesRiskReceipt, String> {
    channel.validate()?;
    losses.validate(channel.state_ids.len())?;
    validate_prior(prior, channel.state_ids.len())?;

    let mut decision_probabilities = Vec::with_capacity(channel.observation_ids.len());
    let mut abstention_probabilities = Vec::with_capacity(channel.observation_ids.len());

    for (observation, _) in channel.observation_ids.iter().enumerate() {
        let mut observation_probability = Rational::zero();
        let mut decision_loss_numerators = vec![Rational::zero(); losses.decision_ids.len()];

        for (state, prior_mass) in prior.iter().copied().enumerate() {
            let joint_mass = prior_mass.checked_mul(channel.probabilities[state][observation])?;
            observation_probability = observation_probability.checked_add(joint_mass)?;
            for (loss_numerator, decision_loss) in
                decision_loss_numerators.iter_mut().zip(&losses.losses)
            {
                *loss_numerator =
                    loss_numerator.checked_add(joint_mass.checked_mul(decision_loss[state])?)?;
            }
        }

        if observation_probability.is_zero() {
            decision_probabilities.push(vec![Rational::zero(); losses.decision_ids.len()]);
            abstention_probabilities.push(Rational::one());
            continue;
        }

        let mut best_numerator = observation_probability.checked_mul(losses.abstention_loss)?;
        let mut selected_decision = None;
        for (decision, numerator) in decision_loss_numerators.iter().copied().enumerate() {
            if numerator < best_numerator {
                best_numerator = numerator;
                selected_decision = Some(decision);
            }
        }

        let mut row = vec![Rational::zero(); losses.decision_ids.len()];
        if let Some(decision) = selected_decision {
            row[decision] = Rational::one();
            abstention_probabilities.push(Rational::zero());
        } else {
            abstention_probabilities.push(Rational::one());
        }
        decision_probabilities.push(row);
    }

    let rule = RandomizedTerminalDecisionRule {
        rule_id: format!("exact-bayes:{}", channel.channel_id),
        channel_id: channel.channel_id.clone(),
        observation_ids: channel.observation_ids.clone(),
        decision_ids: losses.decision_ids.clone(),
        decision_probabilities,
        abstention_probabilities,
    };
    let evaluation = evaluate_randomized_terminal_rule(channel, prior, losses, &rule)?;
    Ok(ExactBayesRiskReceipt { rule, evaluation })
}

/// Lift any randomized rule for the less-informative channel through a
/// verified garbling witness, then verify exact risk preservation and Bayes
/// risk monotonicity.
///
/// If G(z,y)=P(Y=y | Z=z) is the garbling matrix, the lifted rule is
/// P(d | z)=sum_y G(z,y) P(d | y), with the same composition for abstention.
/// The equality of expected risks is an algebraic consequence of the witness
/// identity, not a statistical estimate.
pub fn certify_blackwell_decision_transfer(
    more_informative: &FiniteObservationChannel,
    less_informative: &FiniteObservationChannel,
    witness: &BlackwellGarblingWitness,
    prior: &[Rational],
    losses: &ChannelDecisionLossTable,
    less_rule: &RandomizedTerminalDecisionRule,
) -> Result<BlackwellDecisionTransferCertificate, String> {
    let dominance = verify_blackwell_dominance(more_informative, less_informative, witness)?;
    validate_prior(prior, more_informative.state_ids.len())?;
    losses.validate(more_informative.state_ids.len())?;
    less_rule.validate(less_informative, losses)?;

    let mut decision_probabilities = Vec::with_capacity(more_informative.observation_ids.len());
    let mut abstention_probabilities = Vec::with_capacity(more_informative.observation_ids.len());

    for garbling_row in &witness.probabilities {
        let mut row = vec![Rational::zero(); losses.decision_ids.len()];
        let mut abstention = Rational::zero();
        for (target_observation, garbling_probability) in garbling_row.iter().copied().enumerate() {
            for (decision, target_probability) in less_rule.decision_probabilities
                [target_observation]
                .iter()
                .copied()
                .enumerate()
            {
                row[decision] = row[decision]
                    .checked_add(garbling_probability.checked_mul(target_probability)?)?;
            }
            abstention = abstention.checked_add(
                garbling_probability
                    .checked_mul(less_rule.abstention_probabilities[target_observation])?,
            )?;
        }
        decision_probabilities.push(row);
        abstention_probabilities.push(abstention);
    }

    let transferred_rule = RandomizedTerminalDecisionRule {
        rule_id: format!(
            "blackwell-transfer:{}:{}",
            less_rule.rule_id, more_informative.channel_id
        ),
        channel_id: more_informative.channel_id.clone(),
        observation_ids: more_informative.observation_ids.clone(),
        decision_ids: losses.decision_ids.clone(),
        decision_probabilities,
        abstention_probabilities,
    };
    transferred_rule.validate(more_informative, losses)?;

    let source_rule_risk =
        evaluate_randomized_terminal_rule(less_informative, prior, losses, less_rule)?;
    let transferred_rule_risk =
        evaluate_randomized_terminal_rule(more_informative, prior, losses, &transferred_rule)?;
    if source_rule_risk.expected_loss != transferred_rule_risk.expected_loss {
        return Err("Blackwell-transferred rule failed exact expected-loss preservation".into());
    }

    let less_informative_bayes = exact_bayes_risk(less_informative, prior, losses)?;
    let more_informative_bayes = exact_bayes_risk(more_informative, prior, losses)?;
    if more_informative_bayes.evaluation.expected_loss
        > less_informative_bayes.evaluation.expected_loss
    {
        return Err("Blackwell Bayes-risk monotonicity failed under exact arithmetic".into());
    }
    let bayes_risk_advantage = checked_sub_nonnegative(
        less_informative_bayes.evaluation.expected_loss,
        more_informative_bayes.evaluation.expected_loss,
    )?;

    Ok(BlackwellDecisionTransferCertificate {
        dominance,
        source_rule: less_rule.clone(),
        transferred_rule,
        source_rule_risk,
        transferred_rule_risk,
        less_informative_bayes,
        more_informative_bayes,
        bayes_risk_advantage,
        transfer_preserves_expected_loss: true,
        bayes_risk_monotonicity_verified: true,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        RandomizedTerminalDecisionRule, certify_blackwell_decision_transfer,
        evaluate_randomized_terminal_rule, exact_bayes_risk,
    };
    use crate::Rational;
    use crate::decision_value::experiment_ordering::{
        BlackwellGarblingWitness, ChannelDecisionLossTable, FiniteObservationChannel,
    };

    fn q(numerator: u128, denominator: u128) -> Rational {
        Rational::new(numerator, denominator).expect("valid rational")
    }

    fn channels() -> (
        FiniteObservationChannel,
        FiniteObservationChannel,
        BlackwellGarblingWitness,
    ) {
        let more = FiniteObservationChannel {
            channel_id: "perfect".into(),
            state_ids: vec!["s0".into(), "s1".into()],
            observation_ids: vec!["z0".into(), "z1".into()],
            probabilities: vec![vec![q(1, 1), q(0, 1)], vec![q(0, 1), q(1, 1)]],
        };
        let less = FiniteObservationChannel {
            channel_id: "noisy-80".into(),
            state_ids: vec!["s0".into(), "s1".into()],
            observation_ids: vec!["y0".into(), "y1".into()],
            probabilities: vec![vec![q(4, 5), q(1, 5)], vec![q(1, 5), q(4, 5)]],
        };
        let witness = BlackwellGarblingWitness {
            probabilities: vec![vec![q(4, 5), q(1, 5)], vec![q(1, 5), q(4, 5)]],
        };
        (more, less, witness)
    }

    fn losses() -> ChannelDecisionLossTable {
        ChannelDecisionLossTable {
            decision_ids: vec!["choose-s0".into(), "choose-s1".into()],
            losses: vec![vec![q(0, 1), q(1, 1)], vec![q(1, 1), q(0, 1)]],
            abstention_loss: q(1, 2),
        }
    }

    fn randomized_less_rule(channel: &FiniteObservationChannel) -> RandomizedTerminalDecisionRule {
        RandomizedTerminalDecisionRule {
            rule_id: "randomized-with-abstention".into(),
            channel_id: channel.channel_id.clone(),
            observation_ids: channel.observation_ids.clone(),
            decision_ids: vec!["choose-s0".into(), "choose-s1".into()],
            decision_probabilities: vec![vec![q(1, 3), q(1, 6)], vec![q(1, 4), q(1, 2)]],
            abstention_probabilities: vec![q(1, 2), q(1, 4)],
        }
    }

    #[test]
    fn transferred_randomized_rule_preserves_exact_risk_with_abstention() {
        let (more, less, witness) = channels();
        let table = losses();
        let source_rule = randomized_less_rule(&less);
        let source_risk =
            evaluate_randomized_terminal_rule(&less, &[q(1, 2), q(1, 2)], &table, &source_rule)
                .expect("source risk evaluates");

        let certificate = certify_blackwell_decision_transfer(
            &more,
            &less,
            &witness,
            &[q(1, 2), q(1, 2)],
            &table,
            &source_rule,
        )
        .expect("valid Blackwell transfer certificate");

        assert_eq!(source_risk.expected_loss, q(7, 16));
        assert_eq!(
            certificate.source_rule_risk.expected_loss,
            certificate.transferred_rule_risk.expected_loss
        );
        assert_eq!(certificate.transferred_rule_risk.expected_loss, q(7, 16));
        assert!(certificate.transfer_preserves_expected_loss);
        assert!(certificate.bayes_risk_monotonicity_verified);
        assert_eq!(
            certificate.transferred_rule.observation_ids,
            more.observation_ids
        );
    }

    #[test]
    fn exact_bayes_risk_is_monotone_and_reports_decision_value_of_refinement() {
        let (more, less, witness) = channels();
        let table = losses();
        let prior = [q(1, 2), q(1, 2)];
        let more_bayes = exact_bayes_risk(&more, &prior, &table).expect("perfect-channel risk");
        let less_bayes = exact_bayes_risk(&less, &prior, &table).expect("noisy-channel risk");
        assert_eq!(more_bayes.evaluation.expected_loss, q(0, 1));
        assert_eq!(less_bayes.evaluation.expected_loss, q(1, 5));

        let source_rule = randomized_less_rule(&less);
        let certificate = certify_blackwell_decision_transfer(
            &more,
            &less,
            &witness,
            &prior,
            &table,
            &source_rule,
        )
        .expect("valid Blackwell transfer certificate");
        assert_eq!(certificate.bayes_risk_advantage, q(1, 5));
    }

    #[test]
    fn identity_garbling_has_zero_decision_value_of_refinement() {
        let (_, less, _) = channels();
        let identity = BlackwellGarblingWitness {
            probabilities: vec![vec![q(1, 1), q(0, 1)], vec![q(0, 1), q(1, 1)]],
        };
        let table = losses();
        let source_rule = randomized_less_rule(&less);
        let certificate = certify_blackwell_decision_transfer(
            &less,
            &less,
            &identity,
            &[q(1, 2), q(1, 2)],
            &table,
            &source_rule,
        )
        .expect("identity transfer certificate");
        assert_eq!(certificate.bayes_risk_advantage, q(0, 1));
        assert_eq!(
            certificate.source_rule_risk.expected_loss,
            certificate.transferred_rule_risk.expected_loss
        );
    }

    #[test]
    fn zero_probability_observation_is_accounted_without_posterior_normalization() {
        let (more, less, witness) = channels();
        let mut with_unused_symbol = more.clone();
        with_unused_symbol.observation_ids.push("never".into());
        for row in &mut with_unused_symbol.probabilities {
            row.push(q(0, 1));
        }
        let witness_with_unused_symbol = BlackwellGarblingWitness {
            probabilities: vec![
                vec![q(4, 5), q(1, 5)],
                vec![q(1, 5), q(4, 5)],
                vec![q(1, 2), q(1, 2)],
            ],
        };
        let table = losses();
        let source_rule = randomized_less_rule(&less);
        let certificate = certify_blackwell_decision_transfer(
            &with_unused_symbol,
            &less,
            &witness_with_unused_symbol,
            &[q(1, 2), q(1, 2)],
            &table,
            &source_rule,
        )
        .expect("zero-probability source symbol remains harmless");
        assert_eq!(certificate.transferred_rule_risk.expected_loss, q(7, 16));
        assert_eq!(
            certificate.transferred_rule_risk.observations[2].observation_probability,
            q(0, 1)
        );
        assert_eq!(
            certificate.transferred_rule_risk.observations[2].conditional_loss,
            None
        );
    }

    #[test]
    fn malformed_rules_and_non_dominance_fail_closed() {
        let (more, less, witness) = channels();
        let table = losses();
        let mut invalid_rule = randomized_less_rule(&less);
        invalid_rule.abstention_probabilities[0] = q(1, 3);
        assert!(
            evaluate_randomized_terminal_rule(&less, &[q(1, 2), q(1, 2)], &table, &invalid_rule)
                .is_err()
        );

        let not_dominating = BlackwellGarblingWitness {
            probabilities: vec![vec![q(1, 2), q(1, 2)], vec![q(1, 2), q(1, 2)]],
        };
        assert!(
            certify_blackwell_decision_transfer(
                &more,
                &less,
                &not_dominating,
                &[q(1, 2), q(1, 2)],
                &table,
                &randomized_less_rule(&less)
            )
            .is_err()
        );

        let mut wrong_channel_rule = randomized_less_rule(&less);
        wrong_channel_rule.channel_id = "different-channel".into();
        assert!(
            certify_blackwell_decision_transfer(
                &more,
                &less,
                &witness,
                &[q(1, 2), q(1, 2)],
                &table,
                &wrong_channel_rule
            )
            .is_err()
        );
    }
}
