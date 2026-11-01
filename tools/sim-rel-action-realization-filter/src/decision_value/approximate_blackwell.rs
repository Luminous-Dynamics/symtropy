//! Approximate Blackwell simulation with decision-specific exact loss bounds.
//!
//! Given two finite channels A and B and a stochastic post-processing kernel G,
//! this module measures how closely A followed by G reproduces B. A row-wise
//! total-variation residual epsilon_s yields a decision-specific bound:
//!
//! |R_A(delta_A) - R_B(delta_B)| <= sum_s prior(s) epsilon_s rho_s,
//!
//! where rho_s is the span of the terminal losses (including abstention) in
//! hidden state s. This is an exact finite-channel guarantee, not a physical
//! model-coverage claim.

use super::blackwell_transfer::{
    evaluate_randomized_terminal_rule, exact_bayes_risk, ExactBayesRiskReceipt,
    RandomizedRuleRiskReceipt, RandomizedTerminalDecisionRule,
};
use super::experiment_ordering::{
    BlackwellGarblingWitness, ChannelDecisionLossTable, FiniteObservationChannel,
};
use crate::Rational;

/// Exact approximation error for one hidden-state row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateChannelResidual {
    pub state_id: String,
    pub total_variation: Rational,
}

/// Audit result for a declared post-processing kernel G.
///
/// The within_declared_radius field is explicit. An approximate-dominance certificate
/// refuses to proceed when any statewise TV residual exceeds the caller's
/// declared bound.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApproximateBlackwellDominanceReceipt {
    pub more_informative_channel_id: String,
    pub target_channel_id: String,
    pub witness: BlackwellGarblingWitness,
    pub state_residuals: Vec<StateChannelResidual>,
    pub maximum_state_residual: Rational,
    pub declared_maximum_residual: Rational,
    pub within_declared_radius: bool,
}

/// Loss span and its contribution to the exact decision-specific bound.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateDecisionLossSpan {
    pub state_id: String,
    pub loss_span: Rational,
    pub prior_weighted_bound: Rational,
}

/// Complete decision-transfer certificate for approximate Blackwell
/// simulation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApproximateBlackwellDecisionTransferCertificate {
    pub dominance: ApproximateBlackwellDominanceReceipt,
    pub source_rule: RandomizedTerminalDecisionRule,
    pub transferred_rule: RandomizedTerminalDecisionRule,
    pub source_rule_risk: RandomizedRuleRiskReceipt,
    pub transferred_rule_risk: RandomizedRuleRiskReceipt,
    pub arbitrary_rule_absolute_risk_gap: Rational,
    pub state_loss_spans: Vec<StateDecisionLossSpan>,
    pub decision_specific_transfer_bound: Rational,
    pub target_bayes: ExactBayesRiskReceipt,
    pub transferred_target_bayes_rule: RandomizedTerminalDecisionRule,
    pub transferred_target_bayes_risk: RandomizedRuleRiskReceipt,
    pub candidate_bayes: ExactBayesRiskReceipt,
    pub bayes_risk_upper_bound: Rational,
    pub arbitrary_rule_bound_verified: bool,
    pub bayes_risk_bound_verified: bool,
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

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

fn subtract_nonnegative(left: Rational, right: Rational) -> Result<Rational, String> {
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

fn absolute_difference(left: Rational, right: Rational) -> Result<Rational, String> {
    if left >= right {
        subtract_nonnegative(left, right)
    } else {
        subtract_nonnegative(right, left)
    }
}

fn validate_postprocessing_kernel(
    more_informative: &FiniteObservationChannel,
    target: &FiniteObservationChannel,
    witness: &BlackwellGarblingWitness,
) -> Result<(), String> {
    more_informative.validate()?;
    target.validate()?;
    if more_informative.state_ids != target.state_ids {
        return Err("channels must share the same ordered hidden-state vocabulary".into());
    }
    if witness.probabilities.len() != more_informative.observation_ids.len() {
        return Err("post-processing row count must equal the richer channel output count".into());
    }

    for (source_observation, row) in witness.probabilities.iter().enumerate() {
        if row.len() != target.observation_ids.len() {
            return Err(format!(
                "post-processing row {source_observation} has the wrong target-output dimension"
            ));
        }
        let mut row_mass = Rational::zero();
        for probability in row {
            if !probability.is_probability() {
                return Err(format!(
                    "post-processing row {source_observation} contains an invalid probability"
                ));
            }
            row_mass = row_mass.checked_add(*probability)?;
        }
        if row_mass != Rational::one() {
            return Err(format!(
                "post-processing row {source_observation} must sum exactly to one"
            ));
        }
    }
    Ok(())
}

/// Audit an approximate Blackwell witness without treating a failed radius
/// check as a mathematical non-dominance result.
///
/// Each residual is computed exactly as one half of the L1 distance between
/// the target row and the richer channel followed by the supplied stochastic
/// post-processing kernel.
pub fn verify_approximate_blackwell_dominance(
    more_informative: &FiniteObservationChannel,
    target: &FiniteObservationChannel,
    witness: &BlackwellGarblingWitness,
    declared_maximum_residual: Rational,
) -> Result<ApproximateBlackwellDominanceReceipt, String> {
    if !declared_maximum_residual.is_probability() {
        return Err("declared maximum TV residual must be in [0, 1]".into());
    }
    validate_postprocessing_kernel(more_informative, target, witness)?;

    let mut state_residuals = Vec::with_capacity(more_informative.state_ids.len());
    let mut maximum_state_residual = Rational::zero();

    for (state, state_id) in more_informative.state_ids.iter().enumerate() {
        let mut l1_residual = Rational::zero();
        for (target_observation, target_probability) in
            target.probabilities[state].iter().copied().enumerate()
        {
            let mut reconstructed = Rational::zero();
            for (source_observation, source_probability) in
                more_informative.probabilities[state].iter().copied().enumerate()
            {
                reconstructed = reconstructed.checked_add(
                    source_probability
                        .checked_mul(witness.probabilities[source_observation][target_observation])?,
                )?;
            }
            l1_residual = l1_residual
                .checked_add(absolute_difference(target_probability, reconstructed)?)?;
        }
        let total_variation = l1_residual.checked_div(Rational::from_u64(2))?;
        if total_variation > maximum_state_residual {
            maximum_state_residual = total_variation;
        }
        state_residuals.push(StateChannelResidual {
            state_id: state_id.clone(),
            total_variation,
        });
    }

    Ok(ApproximateBlackwellDominanceReceipt {
        more_informative_channel_id: more_informative.channel_id.clone(),
        target_channel_id: target.channel_id.clone(),
        witness: witness.clone(),
        state_residuals,
        maximum_state_residual,
        declared_maximum_residual,
        within_declared_radius: maximum_state_residual <= declared_maximum_residual,
    })
}

fn lift_randomized_rule(
    more_informative: &FiniteObservationChannel,
    target: &FiniteObservationChannel,
    witness: &BlackwellGarblingWitness,
    losses: &ChannelDecisionLossTable,
    target_rule: &RandomizedTerminalDecisionRule,
) -> Result<RandomizedTerminalDecisionRule, String> {
    target_rule.validate(target, losses)?;
    let mut decision_probabilities =
        Vec::with_capacity(more_informative.observation_ids.len());
    let mut abstention_probabilities =
        Vec::with_capacity(more_informative.observation_ids.len());

    for garbling_row in &witness.probabilities {
        let mut decision_row = vec![Rational::zero(); losses.decision_ids.len()];
        let mut abstention_probability = Rational::zero();
        for (target_observation, garbling_probability) in
            garbling_row.iter().copied().enumerate()
        {
            for (decision, target_probability) in target_rule.decision_probabilities
                [target_observation]
                .iter()
                .copied()
                .enumerate()
            {
                decision_row[decision] = decision_row[decision]
                    .checked_add(garbling_probability.checked_mul(target_probability)?)?;
            }
            abstention_probability = abstention_probability.checked_add(
                garbling_probability
                    .checked_mul(target_rule.abstention_probabilities[target_observation])?,
            )?;
        }
        decision_probabilities.push(decision_row);
        abstention_probabilities.push(abstention_probability);
    }

    let lifted = RandomizedTerminalDecisionRule {
        rule_id: format!(
            "approximate-blackwell-transfer:{}:{}",
            target_rule.rule_id, more_informative.channel_id
        ),
        channel_id: more_informative.channel_id.clone(),
        observation_ids: more_informative.observation_ids.clone(),
        decision_ids: losses.decision_ids.clone(),
        decision_probabilities,
        abstention_probabilities,
    };
    lifted.validate(more_informative, losses)?;
    Ok(lifted)
}

fn state_loss_spans(
    state_ids: &[String],
    losses: &ChannelDecisionLossTable,
) -> Result<Vec<Rational>, String> {
    let mut spans = Vec::with_capacity(state_ids.len());
    for state in 0..state_ids.len() {
        let mut minimum = losses.abstention_loss;
        let mut maximum = losses.abstention_loss;
        for decision_losses in &losses.losses {
            let loss = *decision_losses
                .get(state)
                .ok_or_else(|| "loss table has the wrong hidden-state dimension".to_string())?;
            if loss < minimum {
                minimum = loss;
            }
            if loss > maximum {
                maximum = loss;
            }
        }
        spans.push(subtract_nonnegative(maximum, minimum)?);
    }
    Ok(spans)
}

/// Build an exact additive decision-loss certificate for an approximate
/// Blackwell simulation.
///
/// A failed radius check returns an error. The separate
/// verify_approximate_blackwell_dominance function can still be used to inspect
/// the exact residual vector before choosing an admissible radius.
pub fn certify_approximate_blackwell_decision_transfer(
    more_informative: &FiniteObservationChannel,
    target: &FiniteObservationChannel,
    witness: &BlackwellGarblingWitness,
    prior: &[Rational],
    losses: &ChannelDecisionLossTable,
    target_rule: &RandomizedTerminalDecisionRule,
    declared_maximum_residual: Rational,
) -> Result<ApproximateBlackwellDecisionTransferCertificate, String> {
    let dominance = verify_approximate_blackwell_dominance(
        more_informative,
        target,
        witness,
        declared_maximum_residual,
    )?;
    if !dominance.within_declared_radius {
        return Err(format!(
            "approximate Blackwell residual {} exceeds declared radius {}",
            dominance.maximum_state_residual.numerator(),
            dominance.declared_maximum_residual.numerator()
        ));
    }
    validate_prior(prior, more_informative.state_ids.len())?;
    losses.validate(more_informative.state_ids.len())?;
    target_rule.validate(target, losses)?;

    let spans = state_loss_spans(&more_informative.state_ids, losses)?;
    let mut state_loss_spans = Vec::with_capacity(spans.len());
    let mut decision_specific_transfer_bound = Rational::zero();
    for (state, span) in spans.iter().copied().enumerate() {
        let contribution = prior[state]
            .checked_mul(dominance.state_residuals[state].total_variation)?
            .checked_mul(span)?;
        decision_specific_transfer_bound =
            decision_specific_transfer_bound.checked_add(contribution)?;
        state_loss_spans.push(StateDecisionLossSpan {
            state_id: more_informative.state_ids[state].clone(),
            loss_span: span,
            prior_weighted_bound: contribution,
        });
    }

    let transferred_rule = lift_randomized_rule(
        more_informative,
        target,
        witness,
        losses,
        target_rule,
    )?;
    let source_rule_risk =
        evaluate_randomized_terminal_rule(target, prior, losses, target_rule)?;
    let transferred_rule_risk =
        evaluate_randomized_terminal_rule(more_informative, prior, losses, &transferred_rule)?;
    let arbitrary_rule_absolute_risk_gap = absolute_difference(
        source_rule_risk.expected_loss,
        transferred_rule_risk.expected_loss,
    )?;
    if arbitrary_rule_absolute_risk_gap > decision_specific_transfer_bound {
        return Err("arbitrary-policy risk gap exceeds the exact decision-specific bound".into());
    }

    let target_bayes = exact_bayes_risk(target, prior, losses)?;
    let transferred_target_bayes_rule = lift_randomized_rule(
        more_informative,
        target,
        witness,
        losses,
        &target_bayes.rule,
    )?;
    let transferred_target_bayes_risk = evaluate_randomized_terminal_rule(
        more_informative,
        prior,
        losses,
        &transferred_target_bayes_rule,
    )?;
    let candidate_bayes = exact_bayes_risk(more_informative, prior, losses)?;
    let bayes_risk_upper_bound = target_bayes
        .evaluation
        .expected_loss
        .checked_add(decision_specific_transfer_bound)?;
    if transferred_target_bayes_risk.expected_loss
        > bayes_risk_upper_bound
        || candidate_bayes.evaluation.expected_loss > bayes_risk_upper_bound
    {
        return Err("Bayes-risk transfer exceeds the exact additive bound".into());
    }

    Ok(ApproximateBlackwellDecisionTransferCertificate {
        dominance,
        source_rule: target_rule.clone(),
        transferred_rule,
        source_rule_risk,
        transferred_rule_risk,
        arbitrary_rule_absolute_risk_gap,
        state_loss_spans,
        decision_specific_transfer_bound,
        target_bayes,
        transferred_target_bayes_rule,
        transferred_target_bayes_risk,
        candidate_bayes,
        bayes_risk_upper_bound,
        arbitrary_rule_bound_verified: true,
        bayes_risk_bound_verified: true,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        certify_approximate_blackwell_decision_transfer,
        verify_approximate_blackwell_dominance,
    };
    use crate::decision_value::blackwell_transfer::RandomizedTerminalDecisionRule;
    use crate::decision_value::experiment_ordering::{
        BlackwellGarblingWitness, ChannelDecisionLossTable, FiniteObservationChannel,
    };
    use crate::Rational;

    fn q(numerator: u128, denominator: u128) -> Rational {
        Rational::new(numerator, denominator).expect("valid rational")
    }

    // Independent exhaustive raw path-mass sum. This deliberately does not
    // call the production evaluator or normalize any posterior.
    fn independent_raw_mass_risk(
        channel: &FiniteObservationChannel,
        prior: &[Rational],
        losses: &ChannelDecisionLossTable,
        rule: &RandomizedTerminalDecisionRule,
    ) -> Rational {
        let mut total = q(0, 1);
        for state in 0..channel.state_ids.len() {
            for observation in 0..channel.observation_ids.len() {
                let state_observation_mass =
                    prior[state].checked_mul(channel.probabilities[state][observation])
                        .expect("joint state-observation mass");
                for decision in 0..losses.decision_ids.len() {
                    let action_mass = state_observation_mass
                        .checked_mul(rule.decision_probabilities[observation][decision])
                        .expect("decision path mass");
                    total = total
                        .checked_add(action_mass.checked_mul(losses.losses[decision][state])
                            .expect("decision path loss"))
                        .expect("total decision risk");
                }
                let abstention_mass = state_observation_mass
                    .checked_mul(rule.abstention_probabilities[observation])
                    .expect("abstention path mass");
                total = total
                    .checked_add(abstention_mass.checked_mul(losses.abstention_loss)
                        .expect("abstention path loss"))
                    .expect("total terminal risk");
            }
        }
        total
    }

    fn more_channel() -> FiniteObservationChannel {
        FiniteObservationChannel {
            channel_id: "perfect".into(),
            state_ids: vec!["s0".into(), "s1".into()],
            observation_ids: vec!["z0".into(), "z1".into()],
            probabilities: vec![vec![q(1, 1), q(0, 1)], vec![q(0, 1), q(1, 1)]],
        }
    }

    fn target_channel(correct: u128) -> FiniteObservationChannel {
        let incorrect = 100 - correct;
        FiniteObservationChannel {
            channel_id: format!("symmetric-{correct}-percent"),
            state_ids: vec!["s0".into(), "s1".into()],
            observation_ids: vec!["y0".into(), "y1".into()],
            probabilities: vec![
                vec![q(correct, 100), q(incorrect, 100)],
                vec![q(incorrect, 100), q(correct, 100)],
            ],
        }
    }

    fn garbling() -> BlackwellGarblingWitness {
        BlackwellGarblingWitness {
            probabilities: vec![
                vec![q(4, 5), q(1, 5)],
                vec![q(1, 5), q(4, 5)],
            ],
        }
    }

    fn losses(scale: Rational) -> ChannelDecisionLossTable {
        ChannelDecisionLossTable {
            decision_ids: vec!["choose-s0".into(), "choose-s1".into()],
            losses: vec![
                vec![q(0, 1), scale],
                vec![scale, q(0, 1)],
            ],
            abstention_loss: scale.checked_div(q(2, 1)).expect("half scale"),
        }
    }

    fn target_rule(channel: &FiniteObservationChannel) -> RandomizedTerminalDecisionRule {
        RandomizedTerminalDecisionRule {
            rule_id: "randomized-with-abstention".into(),
            channel_id: channel.channel_id.clone(),
            observation_ids: channel.observation_ids.clone(),
            decision_ids: vec!["choose-s0".into(), "choose-s1".into()],
            decision_probabilities: vec![
                vec![q(1, 3), q(1, 6)],
                vec![q(1, 4), q(1, 2)],
            ],
            abstention_probabilities: vec![q(1, 2), q(1, 4)],
        }
    }

    fn deterministic_guess_rule(
        channel: &FiniteObservationChannel,
    ) -> RandomizedTerminalDecisionRule {
        RandomizedTerminalDecisionRule {
            rule_id: "guess-observed-label".into(),
            channel_id: channel.channel_id.clone(),
            observation_ids: channel.observation_ids.clone(),
            decision_ids: vec!["choose-s0".into(), "choose-s1".into()],
            decision_probabilities: vec![
                vec![q(1, 1), q(0, 1)],
                vec![q(0, 1), q(1, 1)],
            ],
            abstention_probabilities: vec![q(0, 1), q(0, 1)],
        }
    }

    #[test]
    fn exact_garbling_reduces_to_zero_residual_and_zero_transfer_bound() {
        let more = more_channel();
        let target = target_channel(80);
        let witness = garbling();
        let losses = losses(q(1, 1));
        let rule = target_rule(&target);
        let certificate = certify_approximate_blackwell_decision_transfer(
            &more,
            &target,
            &witness,
            &[q(1, 2), q(1, 2)],
            &losses,
            &rule,
            q(0, 1),
        )
        .expect("exact garbling is accepted at radius zero");

        assert_eq!(certificate.dominance.maximum_state_residual, q(0, 1));
        assert_eq!(certificate.decision_specific_transfer_bound, q(0, 1));
        assert_eq!(certificate.arbitrary_rule_absolute_risk_gap, q(0, 1));
        assert_eq!(
            certificate.source_rule_risk.expected_loss,
            certificate.transferred_rule_risk.expected_loss
        );
        assert!(certificate.arbitrary_rule_bound_verified);
        assert!(certificate.bayes_risk_bound_verified);
    }

    #[test]
    fn approximate_garbling_has_exact_statewise_tv_and_decision_bound() {
        let more = more_channel();
        let target = target_channel(79);
        let witness = garbling();
        let losses = losses(q(1, 1));
        let rule = deterministic_guess_rule(&target);
        let certificate = certify_approximate_blackwell_decision_transfer(
            &more,
            &target,
            &witness,
            &[q(1, 2), q(1, 2)],
            &losses,
            &rule,
            q(1, 100),
        )
        .expect("one-percent approximate garbling is accepted");

        assert_eq!(
            certificate
                .dominance
                .state_residuals
                .iter()
                .map(|entry| entry.total_variation)
                .collect::<Vec<_>>(),
            vec![q(1, 100), q(1, 100)]
        );
        assert_eq!(certificate.dominance.maximum_state_residual, q(1, 100));
        assert_eq!(certificate.decision_specific_transfer_bound, q(1, 100));
        assert_eq!(certificate.arbitrary_rule_absolute_risk_gap, q(1, 100));
        assert_eq!(certificate.source_rule_risk.expected_loss, q(21, 100));
        assert_eq!(certificate.transferred_rule_risk.expected_loss, q(1, 5));
        assert_eq!(
            certificate.source_rule_risk.expected_loss,
            independent_raw_mass_risk(&target, &[q(1, 2), q(1, 2)], &losses, &rule)
        );
        assert_eq!(
            certificate.transferred_rule_risk.expected_loss,
            independent_raw_mass_risk(&more, &[q(1, 2), q(1, 2)], &losses, &certificate.transferred_rule)
        );
        assert_eq!(
            certificate.candidate_bayes.evaluation.expected_loss,
            q(0, 1)
        );
        assert_eq!(
            certificate.target_bayes.evaluation.expected_loss,
            q(21, 100)
        );
        assert_eq!(certificate.bayes_risk_upper_bound, q(22, 100));
    }

    #[test]
    fn decision_specific_bound_scales_with_terminal_loss_span() {
        let more = more_channel();
        let target = target_channel(79);
        let witness = garbling();
        let rule = deterministic_guess_rule(&target);

        let unit = certify_approximate_blackwell_decision_transfer(
            &more,
            &target,
            &witness,
            &[q(1, 2), q(1, 2)],
            &losses(q(1, 1)),
            &rule,
            q(1, 100),
        )
        .expect("unit-span certificate");
        let tenth = certify_approximate_blackwell_decision_transfer(
            &more,
            &target,
            &witness,
            &[q(1, 2), q(1, 2)],
            &losses(q(1, 10)),
            &rule,
            q(1, 100),
        )
        .expect("tenth-span certificate");

        assert_eq!(unit.decision_specific_transfer_bound, q(1, 100));
        assert_eq!(tenth.decision_specific_transfer_bound, q(1, 1_000));
        assert_eq!(
            unit.dominance.maximum_state_residual,
            tenth.dominance.maximum_state_residual
        );
    }

    #[test]
    fn declared_radius_is_never_silently_exceeded() {
        let more = more_channel();
        let target = target_channel(79);
        let witness = garbling();
        let residual = verify_approximate_blackwell_dominance(
            &more,
            &target,
            &witness,
            q(1, 200),
        )
        .expect("residual audit");
        assert_eq!(residual.maximum_state_residual, q(1, 100));
        assert!(!residual.within_declared_radius);

        let error = certify_approximate_blackwell_decision_transfer(
            &more,
            &target,
            &witness,
            &[q(1, 2), q(1, 2)],
            &losses(q(1, 1)),
            &deterministic_guess_rule(&target),
            q(1, 200),
        )
        .expect_err("out-of-radius witness must be rejected");
        assert!(error.contains("exceeds declared radius"));
    }

    #[test]
    fn randomized_policy_and_abstention_obey_exact_bound() {
        let more = more_channel();
        let target = target_channel(79);
        let witness = garbling();
        let losses = losses(q(1, 1));
        let rule = target_rule(&target);
        let certificate = certify_approximate_blackwell_decision_transfer(
            &more,
            &target,
            &witness,
            &[q(1, 2), q(1, 2)],
            &losses,
            &rule,
            q(1, 100),
        )
        .expect("randomized policy certificate");

        assert!(certificate.arbitrary_rule_absolute_risk_gap
            <= certificate.decision_specific_transfer_bound);
        assert_eq!(
            certificate.source_rule_risk.expected_loss,
            certificate.transferred_rule_risk.expected_loss
        );
    }

    #[test]
    fn invalid_postprocessing_kernel_and_radius_fail_closed() {
        let more = more_channel();
        let target = target_channel(79);
        let invalid = BlackwellGarblingWitness {
            probabilities: vec![
                vec![q(1, 2), q(1, 2)],
                vec![q(1, 2), q(1, 3)],
            ],
        };
        assert!(verify_approximate_blackwell_dominance(
            &more,
            &target,
            &invalid,
            q(1, 1)
        )
        .is_err());
        assert!(verify_approximate_blackwell_dominance(
            &more,
            &target,
            &garbling(),
            q(3, 2)
        )
        .is_err());
    }


    #[test]
    fn rare_critical_state_uses_a_tighter_prior_weighted_loss_bound() {
        let more = more_channel();
        let target = target_channel(79);
        let witness = garbling();
        let rare_sensitive_losses = ChannelDecisionLossTable {
            decision_ids: vec!["d0".into(), "d1".into()],
            losses: vec![vec![q(0, 1), q(0, 1)], vec![q(1, 1), q(100, 1)]],
            abstention_loss: q(0, 1),
        };
        let prior = [q(99, 100), q(1, 100)];
        let rule = deterministic_guess_rule(&target);
        let certificate = certify_approximate_blackwell_decision_transfer(
            &more,
            &target,
            &witness,
            &prior,
            &rare_sensitive_losses,
            &rule,
            q(1, 100),
        )
        .expect("rare-state risk bound");

        assert_eq!(
            certificate.state_loss_spans.iter().map(|s| s.loss_span).collect::<Vec<_>>(),
            vec![q(1, 1), q(100, 1)]
        );
        assert_eq!(certificate.decision_specific_transfer_bound, q(199, 10_000));
        let global_max_tv_times_max_span = q(1, 100).checked_mul(q(100, 1)).unwrap();
        assert!(certificate.decision_specific_transfer_bound < global_max_tv_times_max_span);
    }

    #[test]
    fn zero_probability_source_symbol_is_retained_without_normalization() {
        let mut more = more_channel();
        more.observation_ids.push("never".into());
        for row in &mut more.probabilities {
            row.push(q(0, 1));
        }
        let mut witness = garbling();
        witness
            .probabilities
            .push(vec![q(1, 2), q(1, 2)]);
        let target = target_channel(80);
        let certificate = certify_approximate_blackwell_decision_transfer(
            &more,
            &target,
            &witness,
            &[q(1, 2), q(1, 2)],
            &losses(q(1, 1)),
            &target_rule(&target),
            q(0, 1),
        )
        .expect("zero-probability source symbol has no effect");

        assert_eq!(certificate.dominance.maximum_state_residual, q(0, 1));
        assert_eq!(
            certificate.transferred_rule_risk.observations[2].observation_probability,
            q(0, 1)
        );
        assert_eq!(
            certificate.transferred_rule_risk.observations[2].conditional_loss,
            None
        );
    }
}
