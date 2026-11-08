//! Exact finite-hypothesis value-of-information reference for locomotion.
//!
//! This crate evaluates decision value only over actions whose independent
//! admission receipts are supplied as current and admitted. It does not
//! generate, authenticate, or prove those physical-admission receipts.

use std::collections::BTreeSet;

pub const PROBABILITY_SCALE_PPB: u64 = 1_000_000_000;
const SCALE: u128 = PROBABILITY_SCALE_PPB as u128;
const SCALE_SQUARED: u128 = SCALE * SCALE;
const SCALE_CUBED: u128 = SCALE_SQUARED * SCALE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionStatus {
    Admitted,
    Rejected,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionReceipt {
    pub receipt_id: String,
    pub subject_action_id: String,
    pub state_digest: String,
    pub domain_digest: String,
    pub model_digest: String,
    pub valid_from_ns: u64,
    pub valid_until_ns: u64,
    /// Status is an upstream verifier result, not authenticated by this crate.
    pub status: AdmissionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    pub id: String,
    pub admission: AdmissionReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hypothesis {
    pub id: String,
    pub prior_ppb: u64,
    /// One likelihood per Outcome in the input vector, in that vector's order.
    pub likelihood_ppb_by_outcome: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutcomeKind {
    Known,
    /// The named fallback is selected for this outcome; no posterior
    /// renormalisation or forced known-hypothesis interpretation is performed.
    Unknown {
        fallback_decision_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub id: String,
    pub kind: OutcomeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub id: String,
    /// Losses align with the input Hypotheses vector.
    pub loss_ppb_by_hypothesis: Vec<u64>,
    pub fallback_only: bool,
    pub admission: AdmissionReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionProblem {
    pub now_ns: u64,
    pub probe: Probe,
    pub hypotheses: Vec<Hypothesis>,
    pub outcomes: Vec<Outcome>,
    pub decisions: Vec<Decision>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutcomeEvaluation {
    pub outcome_id: String,
    /// Numerator over PROBABILITY_SCALE_PPB^2.
    pub probability_numerator_ppb_squared: u128,
    /// All tied minimisers, sorted by stable decision ID.
    pub optimal_decision_ids: Vec<String>,
    /// Unnormalised expected-loss contribution, over SCALE^3.
    pub risk_numerator_ppb_cubed: u128,
    pub is_unknown: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecisionValueEvaluation {
    pub probe_id: String,
    /// Prior risk numerator over SCALE^2.
    pub prior_risk_numerator_ppb_squared: u128,
    /// Expected posterior risk numerator over SCALE^3.
    pub expected_posterior_risk_numerator_ppb_cubed: u128,
    /// Signed VOI numerator over SCALE^3. Negative values are possible when
    /// the declared unknown-outcome fallback is more costly than prior choice.
    pub voi_numerator_ppb_cubed: i128,
    pub prior_optimal_decision_ids: Vec<String>,
    pub outcome_evaluations: Vec<OutcomeEvaluation>,
    /// Defined only when there is exactly one prior-optimal decision. Numerator
    /// is over SCALE^2 and counts outcomes whose optimal set excludes it.
    pub action_switch_probability_numerator_ppb_squared: Option<u128>,
    /// Comparator only. It is never used in the exact VOI calculation.
    pub information_gain_nats: f64,
    pub excluded_decision_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionValueError {
    EmptyIdentifier,
    InvalidValidityInterval,
    StaleOrNotYetValidReceipt,
    AdmissionContextMismatch,
    ProbeNotAdmitted,
    NoHypotheses,
    NoOutcomes,
    NoDecisions,
    DuplicateHypothesisId,
    DuplicateOutcomeId,
    DuplicateDecisionId,
    InvalidPriorProbability,
    PriorNotNormalized,
    InvalidLikelihood,
    LikelihoodNotNormalized,
    InvalidLoss,
    LossCardinalityMismatch,
    LikelihoodCardinalityMismatch,
    NoAdmittedDecision,
    UnknownFallbackMissing,
    UnknownFallbackNotMarked,
    UnknownFallbackNotAdmitted,
    ArithmeticOverflow,
    ValueOutOfSignedRange,
    OutcomeMassNotNormalized,
}

fn is_nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_receipt(
    receipt: &AdmissionReceipt,
    reference: &AdmissionReceipt,
    now_ns: u64,
) -> Result<(), DecisionValueError> {
    if !is_nonempty(&receipt.receipt_id)
        || !is_nonempty(&receipt.subject_action_id)
        || !is_nonempty(&receipt.state_digest)
        || !is_nonempty(&receipt.domain_digest)
        || !is_nonempty(&receipt.model_digest)
    {
        return Err(DecisionValueError::EmptyIdentifier);
    }
    if receipt.valid_from_ns > receipt.valid_until_ns {
        return Err(DecisionValueError::InvalidValidityInterval);
    }
    if now_ns < receipt.valid_from_ns || now_ns > receipt.valid_until_ns {
        return Err(DecisionValueError::StaleOrNotYetValidReceipt);
    }
    if receipt.state_digest != reference.state_digest
        || receipt.domain_digest != reference.domain_digest
        || receipt.model_digest != reference.model_digest
    {
        return Err(DecisionValueError::AdmissionContextMismatch);
    }
    Ok(())
}

fn checked_add(left: u128, right: u128) -> Result<u128, DecisionValueError> {
    left.checked_add(right)
        .ok_or(DecisionValueError::ArithmeticOverflow)
}

fn checked_mul(left: u128, right: u128) -> Result<u128, DecisionValueError> {
    left.checked_mul(right)
        .ok_or(DecisionValueError::ArithmeticOverflow)
}

fn signed_difference(left: u128, right: u128) -> Result<i128, DecisionValueError> {
    let left = i128::try_from(left).map_err(|_| DecisionValueError::ValueOutOfSignedRange)?;
    let right = i128::try_from(right).map_err(|_| DecisionValueError::ValueOutOfSignedRange)?;
    left.checked_sub(right)
        .ok_or(DecisionValueError::ArithmeticOverflow)
}

fn decision_cost(
    decision: &Decision,
    hypotheses: &[Hypothesis],
    hypothesis_order: &[usize],
    outcome_index: Option<usize>,
) -> Result<u128, DecisionValueError> {
    let mut total = 0_u128;
    for &hypothesis_index in hypothesis_order {
        let hypothesis = &hypotheses[hypothesis_index];
        let mut mass = u128::from(hypothesis.prior_ppb);
        if let Some(index) = outcome_index {
            mass = checked_mul(
                mass,
                u128::from(hypothesis.likelihood_ppb_by_outcome[index]),
            )?;
        }
        let loss = u128::from(decision.loss_ppb_by_hypothesis[hypothesis_index]);
        total = checked_add(total, checked_mul(mass, loss)?)?;
    }
    Ok(total)
}

fn minimum_cost(
    decisions: &[&Decision],
    hypotheses: &[Hypothesis],
    hypothesis_order: &[usize],
    outcome_index: Option<usize>,
) -> Result<(u128, Vec<String>), DecisionValueError> {
    let mut minimum: Option<u128> = None;
    let mut minimisers = Vec::new();

    // Decisions are sorted by ID before this function is called. Keeping every
    // tie makes the result independent of input vector ordering.
    for decision in decisions {
        let cost = decision_cost(decision, hypotheses, hypothesis_order, outcome_index)?;
        match minimum {
            None => {
                minimum = Some(cost);
                minimisers.push(decision.id.clone());
            }
            Some(current) if cost < current => {
                minimum = Some(cost);
                minimisers.clear();
                minimisers.push(decision.id.clone());
            }
            Some(current) if cost == current => minimisers.push(decision.id.clone()),
            Some(_) => {}
        }
    }

    minimum
        .map(|cost| (cost, minimisers))
        .ok_or(DecisionValueError::NoAdmittedDecision)
}

fn expected_information_gain(
    hypotheses: &[Hypothesis],
    outcome_order: &[usize],
) -> Result<f64, DecisionValueError> {
    let mut eig_nats = 0.0_f64;

    for &outcome_index in outcome_order {
        let mut outcome_mass = 0_u128;
        for hypothesis in hypotheses {
            outcome_mass = checked_add(
                outcome_mass,
                checked_mul(
                    u128::from(hypothesis.prior_ppb),
                    u128::from(hypothesis.likelihood_ppb_by_outcome[outcome_index]),
                )?,
            )?;
        }

        if outcome_mass == 0 {
            continue;
        }

        for hypothesis in hypotheses {
            let joint_mass = checked_mul(
                u128::from(hypothesis.prior_ppb),
                u128::from(hypothesis.likelihood_ppb_by_outcome[outcome_index]),
            )?;
            if joint_mass == 0 {
                continue;
            }

            let likelihood_scaled = checked_mul(
                SCALE,
                u128::from(hypothesis.likelihood_ppb_by_outcome[outcome_index]),
            )?;
            let ratio = likelihood_scaled as f64 / outcome_mass as f64;
            let joint_probability = joint_mass as f64 / SCALE_SQUARED as f64;
            eig_nats += joint_probability * ratio.ln();
        }
    }

    if !eig_nats.is_finite() {
        return Err(DecisionValueError::ArithmeticOverflow);
    }
    Ok(eig_nats)
}

/// Evaluate the expected decision value of one contact probe.
///
/// The function treats admission statuses as upstream evidence inputs. It
/// validates receipt identity, context and freshness, but does not authenticate
/// the receipts or prove robust physical viability.
pub fn evaluate_decision_value(
    problem: &DecisionProblem,
) -> Result<DecisionValueEvaluation, DecisionValueError> {
    if !is_nonempty(&problem.probe.id) {
        return Err(DecisionValueError::EmptyIdentifier);
    }
    if problem.hypotheses.is_empty() {
        return Err(DecisionValueError::NoHypotheses);
    }
    if problem.outcomes.is_empty() {
        return Err(DecisionValueError::NoOutcomes);
    }
    if problem.decisions.is_empty() {
        return Err(DecisionValueError::NoDecisions);
    }

    let probe_receipt = &problem.probe.admission;
    validate_receipt(probe_receipt, probe_receipt, problem.now_ns)?;
    if probe_receipt.subject_action_id != problem.probe.id {
        return Err(DecisionValueError::AdmissionContextMismatch);
    }
    if probe_receipt.status != AdmissionStatus::Admitted {
        return Err(DecisionValueError::ProbeNotAdmitted);
    }

    let mut hypothesis_ids = BTreeSet::new();
    let mut prior_total = 0_u128;
    for hypothesis in &problem.hypotheses {
        if !is_nonempty(&hypothesis.id) {
            return Err(DecisionValueError::EmptyIdentifier);
        }
        if !hypothesis_ids.insert(hypothesis.id.as_str()) {
            return Err(DecisionValueError::DuplicateHypothesisId);
        }
        if hypothesis.prior_ppb > PROBABILITY_SCALE_PPB {
            return Err(DecisionValueError::InvalidPriorProbability);
        }
        if hypothesis.likelihood_ppb_by_outcome.len() != problem.outcomes.len() {
            return Err(DecisionValueError::LikelihoodCardinalityMismatch);
        }
        prior_total = checked_add(prior_total, u128::from(hypothesis.prior_ppb))?;

        let mut likelihood_total = 0_u128;
        for &likelihood in &hypothesis.likelihood_ppb_by_outcome {
            if likelihood > PROBABILITY_SCALE_PPB {
                return Err(DecisionValueError::InvalidLikelihood);
            }
            likelihood_total = checked_add(likelihood_total, u128::from(likelihood))?;
        }
        if likelihood_total != SCALE {
            return Err(DecisionValueError::LikelihoodNotNormalized);
        }
    }
    if prior_total != SCALE {
        return Err(DecisionValueError::PriorNotNormalized);
    }

    let mut outcome_ids = BTreeSet::new();
    for outcome in &problem.outcomes {
        if !is_nonempty(&outcome.id) {
            return Err(DecisionValueError::EmptyIdentifier);
        }
        if !outcome_ids.insert(outcome.id.as_str()) {
            return Err(DecisionValueError::DuplicateOutcomeId);
        }
        if let OutcomeKind::Unknown {
            fallback_decision_id,
        } = &outcome.kind
        {
            if !is_nonempty(fallback_decision_id) {
                return Err(DecisionValueError::EmptyIdentifier);
            }
        }
    }

    let mut decision_ids = BTreeSet::new();
    for decision in &problem.decisions {
        if !is_nonempty(&decision.id) {
            return Err(DecisionValueError::EmptyIdentifier);
        }
        if !decision_ids.insert(decision.id.as_str()) {
            return Err(DecisionValueError::DuplicateDecisionId);
        }
        if decision.loss_ppb_by_hypothesis.len() != problem.hypotheses.len() {
            return Err(DecisionValueError::LossCardinalityMismatch);
        }
        if decision
            .loss_ppb_by_hypothesis
            .iter()
            .any(|loss| *loss > PROBABILITY_SCALE_PPB)
        {
            return Err(DecisionValueError::InvalidLoss);
        }
        validate_receipt(&decision.admission, probe_receipt, problem.now_ns)?;
        if decision.admission.subject_action_id != decision.id {
            return Err(DecisionValueError::AdmissionContextMismatch);
        }
    }

    let mut hypothesis_order: Vec<usize> = (0..problem.hypotheses.len()).collect();
    hypothesis_order.sort_by(|left, right| {
        problem.hypotheses[*left]
            .id
            .cmp(&problem.hypotheses[*right].id)
    });

    let mut outcome_order: Vec<usize> = (0..problem.outcomes.len()).collect();
    outcome_order.sort_by(|left, right| {
        problem.outcomes[*left]
            .id
            .cmp(&problem.outcomes[*right].id)
    });

    let mut admitted_decisions = Vec::new();
    let mut admitted_fallbacks = Vec::new();
    let mut excluded_decision_ids = Vec::new();
    for decision in &problem.decisions {
        if decision.admission.status == AdmissionStatus::Admitted {
            if decision.fallback_only {
                admitted_fallbacks.push(decision);
            } else {
                admitted_decisions.push(decision);
            }
        } else {
            excluded_decision_ids.push(decision.id.clone());
        }
    }
    admitted_decisions.sort_by(|left, right| left.id.cmp(&right.id));
    admitted_fallbacks.sort_by(|left, right| left.id.cmp(&right.id));
    excluded_decision_ids.sort();

    if admitted_decisions.is_empty() {
        return Err(DecisionValueError::NoAdmittedDecision);
    }

    // Every unknown outcome must retain its mass and route to a specifically
    // admitted fallback. There is no implicit renormalisation.
    for outcome_index in &outcome_order {
        if let OutcomeKind::Unknown {
            fallback_decision_id,
        } = &problem.outcomes[*outcome_index].kind
        {
            let fallback = problem
                .decisions
                .iter()
                .find(|decision| decision.id == *fallback_decision_id)
                .ok_or(DecisionValueError::UnknownFallbackMissing)?;
            if !fallback.fallback_only {
                return Err(DecisionValueError::UnknownFallbackNotMarked);
            }
            if fallback.admission.status != AdmissionStatus::Admitted {
                return Err(DecisionValueError::UnknownFallbackNotAdmitted);
            }
            // It was already context/freshness checked above.
            if !admitted_fallbacks
                .iter()
                .any(|decision| decision.id == *fallback_decision_id)
            {
                return Err(DecisionValueError::UnknownFallbackNotAdmitted);
            }
        }
    }

    let (prior_risk_numerator, prior_optimal_decision_ids) = minimum_cost(
        &admitted_decisions,
        &problem.hypotheses,
        &hypothesis_order,
        None,
    )?;

    let mut expected_posterior_risk_numerator = 0_u128;
    let mut total_outcome_mass = 0_u128;
    let mut outcome_evaluations = Vec::with_capacity(problem.outcomes.len());

    for &outcome_index in &outcome_order {
        let outcome = &problem.outcomes[outcome_index];
        let mut outcome_mass = 0_u128;
        for &hypothesis_index in &hypothesis_order {
            let hypothesis = &problem.hypotheses[hypothesis_index];
            outcome_mass = checked_add(
                outcome_mass,
                checked_mul(
                    u128::from(hypothesis.prior_ppb),
                    u128::from(hypothesis.likelihood_ppb_by_outcome[outcome_index]),
                )?,
            )?;
        }
        total_outcome_mass = checked_add(total_outcome_mass, outcome_mass)?;

        let (risk_numerator, optimal_decision_ids, is_unknown) = match &outcome.kind {
            OutcomeKind::Known => {
                let (risk, minimisers) = minimum_cost(
                    &admitted_decisions,
                    &problem.hypotheses,
                    &hypothesis_order,
                    Some(outcome_index),
                )?;
                (risk, minimisers, false)
            }
            OutcomeKind::Unknown {
                fallback_decision_id,
            } => {
                let fallback = admitted_fallbacks
                    .iter()
                    .find(|decision| decision.id == *fallback_decision_id)
                    .ok_or(DecisionValueError::UnknownFallbackNotAdmitted)?;
                let cost = decision_cost(
                    fallback,
                    &problem.hypotheses,
                    &hypothesis_order,
                    Some(outcome_index),
                )?;
                (cost, vec![fallback.id.clone()], true)
            }
        };

        expected_posterior_risk_numerator = checked_add(
            expected_posterior_risk_numerator,
            risk_numerator,
        )?;

        outcome_evaluations.push(OutcomeEvaluation {
            outcome_id: outcome.id.clone(),
            probability_numerator_ppb_squared: outcome_mass,
            optimal_decision_ids,
            risk_numerator_ppb_cubed: risk_numerator,
            is_unknown,
        });
    }

    if total_outcome_mass != SCALE_SQUARED {
        return Err(DecisionValueError::OutcomeMassNotNormalized);
    }

    let scaled_prior_risk = checked_mul(SCALE, prior_risk_numerator)?;
    let voi_numerator = signed_difference(
        scaled_prior_risk,
        expected_posterior_risk_numerator,
    )?;

    let action_switch_probability_numerator =
        if prior_optimal_decision_ids.len() == 1 {
            let prior_best = &prior_optimal_decision_ids[0];
            let mut switched_mass = 0_u128;
            for outcome in &outcome_evaluations {
                if !outcome.optimal_decision_ids.contains(prior_best) {
                    switched_mass = checked_add(
                        switched_mass,
                        outcome.probability_numerator_ppb_squared,
                    )?;
                }
            }
            Some(switched_mass)
        } else {
            None
        };

    let information_gain_nats =
        expected_information_gain(&problem.hypotheses, &outcome_order)?;

    Ok(DecisionValueEvaluation {
        probe_id: problem.probe.id.clone(),
        prior_risk_numerator_ppb_squared: prior_risk_numerator,
        expected_posterior_risk_numerator_ppb_cubed: expected_posterior_risk_numerator,
        voi_numerator_ppb_cubed: voi_numerator,
        prior_optimal_decision_ids,
        outcome_evaluations,
        action_switch_probability_numerator_ppb_squared: action_switch_probability_numerator,
        information_gain_nats,
        excluded_decision_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt(id: &str, subject_action_id: &str, status: AdmissionStatus) -> AdmissionReceipt {
        AdmissionReceipt {
            receipt_id: id.to_owned(),
            subject_action_id: subject_action_id.to_owned(),
            state_digest: "state-001".to_owned(),
            domain_digest: "terrain-domain-001".to_owned(),
            model_digest: "model-001".to_owned(),
            valid_from_ns: 100,
            valid_until_ns: 1_000,
            status,
        }
    }

    fn decision(
        id: &str,
        losses: &[u64],
        fallback_only: bool,
    ) -> Decision {
        Decision {
            id: id.to_owned(),
            loss_ppb_by_hypothesis: losses.to_vec(),
            fallback_only,
            admission: receipt(&format!("receipt-{id}"), id, AdmissionStatus::Admitted),
        }
    }

    fn probe() -> Probe {
        Probe {
            id: "probe-001".to_owned(),
            admission: receipt("receipt-probe", "probe-001", AdmissionStatus::Admitted),
        }
    }

    fn evaluate(
        hypotheses: Vec<Hypothesis>,
        outcomes: Vec<Outcome>,
        decisions: Vec<Decision>,
    ) -> DecisionValueEvaluation {
        evaluate_decision_value(&DecisionProblem {
            now_ns: 500,
            probe: probe(),
            hypotheses,
            outcomes,
            decisions,
        })
        .expect("known-answer decision-value fixture should evaluate")
    }

    #[test]
    fn perfect_information_can_have_zero_decision_value() {
        let report = evaluate(
            vec![
                Hypothesis {
                    id: "h1".to_owned(),
                    prior_ppb: 500_000_000,
                    likelihood_ppb_by_outcome: vec![1_000_000_000, 0],
                },
                Hypothesis {
                    id: "h2".to_owned(),
                    prior_ppb: 500_000_000,
                    likelihood_ppb_by_outcome: vec![0, 1_000_000_000],
                },
            ],
            vec![
                Outcome {
                    id: "observe-h1".to_owned(),
                    kind: OutcomeKind::Known,
                },
                Outcome {
                    id: "observe-h2".to_owned(),
                    kind: OutcomeKind::Known,
                },
            ],
            vec![
                decision("hold", &[100_000_000, 100_000_000], false),
                decision("alternate", &[500_000_000, 500_000_000], false),
            ],
        );

        assert_eq!(report.voi_numerator_ppb_cubed, 0);
        assert_eq!(report.prior_optimal_decision_ids, vec!["hold"]);
        assert!((report.information_gain_nats - std::f64::consts::LN_2).abs() < 1e-12);
        assert_eq!(
            report.action_switch_probability_numerator_ppb_squared,
            Some(0)
        );
    }

    #[test]
    fn low_eig_probe_can_have_positive_decision_value() {
        let report = evaluate(
            vec![
                Hypothesis {
                    id: "h1".to_owned(),
                    prior_ppb: 510_000_000,
                    likelihood_ppb_by_outcome: vec![600_000_000, 400_000_000],
                },
                Hypothesis {
                    id: "h2".to_owned(),
                    prior_ppb: 490_000_000,
                    likelihood_ppb_by_outcome: vec![400_000_000, 600_000_000],
                },
            ],
            vec![
                Outcome {
                    id: "y-plus".to_owned(),
                    kind: OutcomeKind::Known,
                },
                Outcome {
                    id: "y-minus".to_owned(),
                    kind: OutcomeKind::Known,
                },
            ],
            vec![
                decision("action-A", &[0, 1_000_000_000], false),
                decision("action-B", &[1_000_000_000, 0], false),
            ],
        );

        assert_eq!(report.prior_risk_numerator_ppb_squared, 490_000_000_000_000_000);
        assert_eq!(
            report.expected_posterior_risk_numerator_ppb_cubed,
            400_000_000_000_000_000_000_000_000
        );
        assert_eq!(report.voi_numerator_ppb_cubed, 90_000_000_000_000_000_000_000_000_i128);
        assert_eq!(report.prior_optimal_decision_ids, vec!["action-A"]);
        assert_eq!(
            report.action_switch_probability_numerator_ppb_squared,
            Some(498_000_000_000_000_000)
        );
        assert!((report.information_gain_nats - 0.020127513529355406).abs() < 1e-12);
        assert_eq!(
            report
                .outcome_evaluations
                .iter()
                .map(|item| item.probability_numerator_ppb_squared)
                .collect::<Vec<_>>(),
            vec![498_000_000_000_000_000, 502_000_000_000_000_000]
        );
    }

    #[test]
    fn uninformative_observation_has_zero_eig_and_zero_voi() {
        let report = evaluate(
            vec![
                Hypothesis {
                    id: "h1".to_owned(),
                    prior_ppb: 510_000_000,
                    likelihood_ppb_by_outcome: vec![500_000_000, 500_000_000],
                },
                Hypothesis {
                    id: "h2".to_owned(),
                    prior_ppb: 490_000_000,
                    likelihood_ppb_by_outcome: vec![500_000_000, 500_000_000],
                },
            ],
            vec![
                Outcome {
                    id: "y-left".to_owned(),
                    kind: OutcomeKind::Known,
                },
                Outcome {
                    id: "y-right".to_owned(),
                    kind: OutcomeKind::Known,
                },
            ],
            vec![
                decision("action-A", &[0, 1_000_000_000], false),
                decision("action-B", &[1_000_000_000, 0], false),
            ],
        );

        assert_eq!(report.voi_numerator_ppb_cubed, 0);
        assert_eq!(report.information_gain_nats, 0.0);
        assert_eq!(report.prior_optimal_decision_ids, vec!["action-A"]);
        assert_eq!(
            report.action_switch_probability_numerator_ppb_squared,
            Some(0)
        );
    }

    #[test]
    fn unknown_outcome_keeps_mass_and_uses_admitted_fallback() {
        let report = evaluate(
            vec![
                Hypothesis {
                    id: "h1".to_owned(),
                    prior_ppb: 510_000_000,
                    likelihood_ppb_by_outcome: vec![600_000_000, 300_000_000, 100_000_000],
                },
                Hypothesis {
                    id: "h2".to_owned(),
                    prior_ppb: 490_000_000,
                    likelihood_ppb_by_outcome: vec![400_000_000, 400_000_000, 200_000_000],
                },
            ],
            vec![
                Outcome {
                    id: "y-plus".to_owned(),
                    kind: OutcomeKind::Known,
                },
                Outcome {
                    id: "y-minus".to_owned(),
                    kind: OutcomeKind::Known,
                },
                Outcome {
                    id: "out-of-model".to_owned(),
                    kind: OutcomeKind::Unknown {
                        fallback_decision_id: "abstain-hold".to_owned(),
                    },
                },
            ],
            vec![
                decision("action-A", &[0, 1_000_000_000], false),
                decision("action-B", &[1_000_000_000, 0], false),
                decision("abstain-hold", &[600_000_000, 600_000_000], true),
            ],
        );

        assert_eq!(
            report.expected_posterior_risk_numerator_ppb_cubed,
            438_400_000_000_000_000_000_000_000
        );
        assert_eq!(report.voi_numerator_ppb_cubed, 51_600_000_000_000_000_000_000_000_i128);
        assert_eq!(
            report.action_switch_probability_numerator_ppb_squared,
            Some(ppb(0.498))
        );
        let unknown = report
            .outcome_evaluations
            .iter()
            .find(|item| item.is_unknown)
            .expect("unknown outcome retained");
        assert_eq!(
            unknown.probability_numerator_ppb_squared,
            149_000_000_000_000_000
        );
        assert_eq!(unknown.optimal_decision_ids, vec!["abstain-hold"]);
    }

    #[test]
    fn rejected_probe_never_reaches_information_ranking() {
        let mut problem = DecisionProblem {
            now_ns: 500,
            probe: probe(),
            hypotheses: vec![Hypothesis {
                id: "h1".to_owned(),
                prior_ppb: PROBABILITY_SCALE_PPB,
                likelihood_ppb_by_outcome: vec![PROBABILITY_SCALE_PPB],
            }],
            outcomes: vec![Outcome {
                id: "only-outcome".to_owned(),
                kind: OutcomeKind::Known,
            }],
            decisions: vec![decision("action-A", &[0], false)],
        };
        problem.probe.admission.status = AdmissionStatus::Rejected;
        assert_eq!(
            evaluate_decision_value(&problem),
            Err(DecisionValueError::ProbeNotAdmitted)
        );
    }

    #[test]
    fn unknown_outcome_cannot_silently_drop_fallback() {
        let problem = DecisionProblem {
            now_ns: 500,
            probe: probe(),
            hypotheses: vec![Hypothesis {
                id: "h1".to_owned(),
                prior_ppb: PROBABILITY_SCALE_PPB,
                likelihood_ppb_by_outcome: vec![900_000_000, 100_000_000],
            }],
            outcomes: vec![
                Outcome {
                    id: "known".to_owned(),
                    kind: OutcomeKind::Known,
                },
                Outcome {
                    id: "unknown".to_owned(),
                    kind: OutcomeKind::Unknown {
                        fallback_decision_id: "missing-fallback".to_owned(),
                    },
                },
            ],
            decisions: vec![decision("action-A", &[0], false)],
        };
        assert_eq!(
            evaluate_decision_value(&problem),
            Err(DecisionValueError::UnknownFallbackMissing)
        );

    }

    #[test]
    fn unadmitted_decision_is_excluded_not_used() {
        let mut excluded = decision("action-B", &[0, 1_000_000_000], false);
        excluded.admission.status = AdmissionStatus::Unknown;
        let report = evaluate(
            vec![
                Hypothesis {
                    id: "h1".to_owned(),
                    prior_ppb: 500_000_000,
                    likelihood_ppb_by_outcome: vec![1_000_000_000],
                },
                Hypothesis {
                    id: "h2".to_owned(),
                    prior_ppb: 500_000_000,
                    likelihood_ppb_by_outcome: vec![1_000_000_000],
                },
            ],
            vec![Outcome {
                id: "single".to_owned(),
                kind: OutcomeKind::Known,
            }],
            vec![
                decision("action-A", &[100_000_000, 100_000_000], false),
                excluded,
            ],
        );
        assert_eq!(report.excluded_decision_ids, vec!["action-B"]);
        assert_eq!(report.prior_optimal_decision_ids, vec!["action-A"]);
    }

    #[test]
    fn hypothesis_and_action_permutations_preserve_exact_results() {
        let base = DecisionProblem {
            now_ns: 500,
            probe: probe(),
            hypotheses: vec![
                Hypothesis {
                    id: "h1".to_owned(),
                    prior_ppb: 510_000_000,
                    likelihood_ppb_by_outcome: vec![600_000_000, 400_000_000],
                },
                Hypothesis {
                    id: "h2".to_owned(),
                    prior_ppb: 490_000_000,
                    likelihood_ppb_by_outcome: vec![400_000_000, 600_000_000],
                },
            ],
            outcomes: vec![
                Outcome {
                    id: "y-plus".to_owned(),
                    kind: OutcomeKind::Known,
                },
                Outcome {
                    id: "y-minus".to_owned(),
                    kind: OutcomeKind::Known,
                },
            ],
            decisions: vec![
                decision("action-A", &[0, 1_000_000_000], false),
                decision("action-B", &[1_000_000_000, 0], false),
            ],
        };
        let original = evaluate_decision_value(&base).expect("base result");

        let permuted = DecisionProblem {
            now_ns: base.now_ns,
            probe: base.probe.clone(),
            hypotheses: vec![
                Hypothesis {
                    id: "h2".to_owned(),
                    prior_ppb: 490_000_000,
                    likelihood_ppb_by_outcome: vec![600_000_000, 400_000_000],
                },
                Hypothesis {
                    id: "h1".to_owned(),
                    prior_ppb: 510_000_000,
                    likelihood_ppb_by_outcome: vec![400_000_000, 600_000_000],
                },
            ],
            outcomes: vec![
                Outcome {
                    id: "y-minus".to_owned(),
                    kind: OutcomeKind::Known,
                },
                Outcome {
                    id: "y-plus".to_owned(),
                    kind: OutcomeKind::Known,
                },
            ],
            decisions: vec![
                decision("action-B", &[0, 1_000_000_000], false),
                decision("action-A", &[1_000_000_000, 0], false),
            ],
        };
        let permuted_report = evaluate_decision_value(&permuted).expect("permuted result");

        assert_eq!(
            original.prior_risk_numerator_ppb_squared,
            permuted_report.prior_risk_numerator_ppb_squared
        );
        assert_eq!(
            original.expected_posterior_risk_numerator_ppb_cubed,
            permuted_report.expected_posterior_risk_numerator_ppb_cubed
        );
        assert_eq!(
            original.voi_numerator_ppb_cubed,
            permuted_report.voi_numerator_ppb_cubed
        );
        assert_eq!(
            original.action_switch_probability_numerator_ppb_squared,
            permuted_report.action_switch_probability_numerator_ppb_squared
        );
        assert_eq!(
            original.outcome_evaluations,
            permuted_report.outcome_evaluations
        );
    }

    #[test]
    fn prior_and_likelihood_rows_must_be_exactly_normalized() {
        let bad_prior = DecisionProblem {
            now_ns: 500,
            probe: probe(),
            hypotheses: vec![Hypothesis {
                id: "h1".to_owned(),
                prior_ppb: 999_999_999,
                likelihood_ppb_by_outcome: vec![PROBABILITY_SCALE_PPB],
            }],
            outcomes: vec![Outcome {
                id: "only".to_owned(),
                kind: OutcomeKind::Known,
            }],
            decisions: vec![decision("a", &[0], false)],
        };
        assert_eq!(
            evaluate_decision_value(&bad_prior),
            Err(DecisionValueError::PriorNotNormalized)
        );

        let bad_likelihood = DecisionProblem {
            hypotheses: vec![Hypothesis {
                id: "h1".to_owned(),
                prior_ppb: PROBABILITY_SCALE_PPB,
                likelihood_ppb_by_outcome: vec![999_999_999],
            }],
            ..bad_prior
        };
        assert_eq!(
            evaluate_decision_value(&bad_likelihood),
            Err(DecisionValueError::LikelihoodNotNormalized)
        );
    }

    #[test]
    fn admission_receipt_must_match_state_domain_and_model() {
        let mut action = decision("action-A", &[0], false);
        action.admission.domain_digest = "different-domain".to_owned();
        let problem = DecisionProblem {
            now_ns: 500,
            probe: probe(),
            hypotheses: vec![Hypothesis {
                id: "h1".to_owned(),
                prior_ppb: PROBABILITY_SCALE_PPB,
                likelihood_ppb_by_outcome: vec![PROBABILITY_SCALE_PPB],
            }],
            outcomes: vec![Outcome {
                id: "only".to_owned(),
                kind: OutcomeKind::Known,
            }],
            decisions: vec![action],
        };
        assert_eq!(
            evaluate_decision_value(&problem),
            Err(DecisionValueError::AdmissionContextMismatch)
        );
    }

    #[test]
    fn ties_are_preserved_and_switch_probability_is_undefined() {
        let report = evaluate(
            vec![
                Hypothesis {
                    id: "h1".to_owned(),
                    prior_ppb: 500_000_000,
                    likelihood_ppb_by_outcome: vec![PROBABILITY_SCALE_PPB],
                },
                Hypothesis {
                    id: "h2".to_owned(),
                    prior_ppb: 500_000_000,
                    likelihood_ppb_by_outcome: vec![PROBABILITY_SCALE_PPB],
                },
            ],
            vec![Outcome {
                id: "only".to_owned(),
                kind: OutcomeKind::Known,
            }],
            vec![
                decision("action-A", &[0, 1_000_000_000], false),
                decision("action-B", &[1_000_000_000, 0], false),
            ],
        );
        assert_eq!(
            report.prior_optimal_decision_ids,
            vec!["action-A", "action-B"]
        );
        assert_eq!(report.action_switch_probability_numerator_ppb_squared, None);
    }
}
