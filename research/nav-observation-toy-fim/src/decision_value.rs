//! Exact finite Bayesian value-of-observation benchmark for a binary task decision.
//!
//! This module deliberately separates mutual information from downstream
//! decision value. A candidate is evaluated by marginalizing every declared
//! observation outcome and the possibility that acquisition fails. Resource
//! limits are hard feasibility constraints, not terms hidden in a scalar score.
//!
//! The model is intentionally small: one binary latent state, a finite outcome
//! alphabet, a one-step action choice, and symmetric zero-one decision loss.
//! It is an exact synthetic oracle, not a navigation estimator or safety case.

use std::collections::BTreeSet;
use std::fmt;

const PROBABILITY_TOLERANCE: f64 = 1.0e-9;

/// Asymmetric cost of the two wrong navigation-service decisions.
///
/// StateZero is the unsafe/out-of-envelope state; StateOne is the
/// safe/in-envelope state. Claiming StateOne when StateZero is true incurs
/// false_safe_loss. Claiming StateZero when StateOne is true incurs
/// false_degraded_loss.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecisionLoss {
    pub false_safe_loss: f64,
    pub false_degraded_loss: f64,
}

impl DecisionLoss {
    pub const fn zero_one() -> Self {
        Self {
            false_safe_loss: 1.0,
            false_degraded_loss: 1.0,
        }
    }

    pub fn new(
        false_safe_loss: f64,
        false_degraded_loss: f64,
    ) -> Result<Self, DecisionValueError> {
        let loss = Self {
            false_safe_loss,
            false_degraded_loss,
        };
        loss.validate()?;
        Ok(loss)
    }

    pub fn validate(self) -> Result<(), DecisionValueError> {
        if !self.false_safe_loss.is_finite()
            || self.false_safe_loss < 0.0
            || !self.false_degraded_loss.is_finite()
            || self.false_degraded_loss < 0.0
            || (self.false_safe_loss == 0.0 && self.false_degraded_loss == 0.0)
        {
            return Err(DecisionValueError::InvalidDecisionLoss);
        }
        Ok(())
    }
}

/// Prior probability that the latent state is StateOne.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BinaryBelief {
    pub probability_state_one: f64,
}

impl BinaryBelief {
    pub fn new(probability_state_one: f64) -> Result<Self, DecisionValueError> {
        if !probability_state_one.is_finite() || !(0.0..=1.0).contains(&probability_state_one) {
            return Err(DecisionValueError::InvalidPriorProbability);
        }
        Ok(Self {
            probability_state_one,
        })
    }

    pub fn probability_state_zero(self) -> f64 {
        1.0 - self.probability_state_one
    }

    /// Minimum expected zero-one classification loss before another observation.
    pub fn bayes_risk(self) -> f64 {
        self.probability_state_zero()
            .min(self.probability_state_one)
    }

    /// Minimum expected loss under an explicit asymmetric decision cost.
    pub fn bayes_risk_with_loss(
        self,
        loss: DecisionLoss,
    ) -> Result<f64, DecisionValueError> {
        BinaryBelief::new(self.probability_state_one)?;
        loss.validate()?;
        Ok((self.probability_state_zero() * loss.false_safe_loss)
            .min(self.probability_state_one * loss.false_degraded_loss))
    }
}

/// Resource use for one observation request. Each dimension is checked
/// independently against its budget; the dimensions are never summed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActionCost {
    pub energy_units: u32,
    pub latency_ticks: u32,
    pub bandwidth_units: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResourceBudget {
    pub energy_units: u32,
    pub latency_ticks: u32,
    pub bandwidth_units: u32,
}

impl ActionCost {
    pub fn fits(self, budget: ResourceBudget) -> bool {
        self.energy_units <= budget.energy_units
            && self.latency_ticks <= budget.latency_ticks
            && self.bandwidth_units <= budget.bandwidth_units
    }
}

/// One possible acquired observation.
#[derive(Clone, Debug, PartialEq)]
pub struct ObservationOutcome {
    pub id: String,
    /// P(outcome | StateZero), e.g. P(alarm | outside service envelope).
    pub likelihood_state_zero: f64,
    /// P(outcome | StateOne), e.g. P(alarm | inside service envelope).
    pub likelihood_state_one: f64,
}

/// Candidate sensing action with a finite conditional observation model.
///
/// Acquisition success is assumed independent of the latent state in this v0
/// model. A failed or unavailable acquisition carries no information.
#[derive(Clone, Debug, PartialEq)]
pub struct DiagnosticAction {
    pub id: String,
    pub cost: ActionCost,
    pub acquisition_success_probability: f64,
    pub outcomes: Vec<ObservationOutcome>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DecisionValueError {
    InvalidPriorProbability,
    InvalidDecisionLoss,
    EmptyActionId,
    DuplicateActionId(String),
    InvalidAcquisitionProbability(String),
    EmptyOutcomeSet(String),
    EmptyOutcomeId(String),
    DuplicateOutcomeId { action_id: String, outcome_id: String },
    InvalidOutcomeLikelihood { action_id: String, outcome_id: String },
    OutcomeMassMismatch { action_id: String, state: &'static str, sum: f64 },
    NonFiniteMetric(String),
}

impl fmt::Display for DecisionValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPriorProbability => {
                write!(f, "prior probability must be finite and within [0, 1]")
            }
            Self::InvalidDecisionLoss => {
                write!(f, "decision losses must be finite, nonnegative, and not both zero")
            }
            Self::EmptyActionId => write!(f, "action ID must not be empty"),
            Self::DuplicateActionId(id) => write!(f, "duplicate action ID: {id}"),
            Self::InvalidAcquisitionProbability(id) => {
                write!(f, "invalid acquisition-success probability for action: {id}")
            }
            Self::EmptyOutcomeSet(id) => write!(f, "action has no observation outcomes: {id}"),
            Self::EmptyOutcomeId(id) => write!(f, "outcome ID must not be empty in action: {id}"),
            Self::DuplicateOutcomeId { action_id, outcome_id } => {
                write!(f, "duplicate outcome {outcome_id} in action {action_id}")
            }
            Self::InvalidOutcomeLikelihood { action_id, outcome_id } => {
                write!(f, "invalid likelihood for outcome {outcome_id} in action {action_id}")
            }
            Self::OutcomeMassMismatch { action_id, state, sum } => write!(
                f,
                "outcome likelihoods for {state} in action {action_id} sum to {sum}, not 1"
            ),
            Self::NonFiniteMetric(id) => {
                write!(f, "decision-value calculation became non-finite for action: {id}")
            }
        }
    }
}
impl std::error::Error for DecisionValueError {}

/// Separate comparator objectives. Neither is treated as a substitute for the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionCriterion {
    ExpectedDecisionValue,
    ExpectedMutualInformation,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActionAssessment {
    pub action_id: String,
    pub cost: ActionCost,
    pub feasible: bool,
    pub baseline_bayes_risk: f64,
    /// Expected Bayes risk conditional on acquisition succeeding.
    pub bayes_risk_after_success: f64,
    /// Includes the possibility that acquisition fails.
    pub expected_bayes_risk: f64,
    /// Baseline risk minus acquisition-failure-adjusted expected risk.
    pub expected_decision_value: f64,
    /// Mutual information between the latent state and observation, conditional on success.
    pub mutual_information_bits_on_success: f64,
    /// Acquisition-success-adjusted mutual information.
    pub expected_mutual_information_bits: f64,
}

fn validate_action(action: &DiagnosticAction) -> Result<(), DecisionValueError> {
    if action.id.trim().is_empty() {
        return Err(DecisionValueError::EmptyActionId);
    }
    if !action.acquisition_success_probability.is_finite()
        || !(0.0..=1.0).contains(&action.acquisition_success_probability)
    {
        return Err(DecisionValueError::InvalidAcquisitionProbability(
            action.id.clone(),
        ));
    }
    if action.outcomes.is_empty() {
        return Err(DecisionValueError::EmptyOutcomeSet(action.id.clone()));
    }

    let mut outcome_ids = BTreeSet::new();
    let mut state_zero_mass = 0.0;
    let mut state_one_mass = 0.0;

    for outcome in &action.outcomes {
        if outcome.id.trim().is_empty() {
            return Err(DecisionValueError::EmptyOutcomeId(action.id.clone()));
        }
        if !outcome_ids.insert(outcome.id.clone()) {
            return Err(DecisionValueError::DuplicateOutcomeId {
                action_id: action.id.clone(),
                outcome_id: outcome.id.clone(),
            });
        }
        if !outcome.likelihood_state_zero.is_finite()
            || !(0.0..=1.0).contains(&outcome.likelihood_state_zero)
            || !outcome.likelihood_state_one.is_finite()
            || !(0.0..=1.0).contains(&outcome.likelihood_state_one)
        {
            return Err(DecisionValueError::InvalidOutcomeLikelihood {
                action_id: action.id.clone(),
                outcome_id: outcome.id.clone(),
            });
        }
        state_zero_mass += outcome.likelihood_state_zero;
        state_one_mass += outcome.likelihood_state_one;
    }

    if (state_zero_mass - 1.0).abs() > PROBABILITY_TOLERANCE {
        return Err(DecisionValueError::OutcomeMassMismatch {
            action_id: action.id.clone(),
            state: "StateZero",
            sum: state_zero_mass,
        });
    }
    if (state_one_mass - 1.0).abs() > PROBABILITY_TOLERANCE {
        return Err(DecisionValueError::OutcomeMassMismatch {
            action_id: action.id.clone(),
            state: "StateOne",
            sum: state_one_mass,
        });
    }
    Ok(())
}

/// Evaluate one candidate by exact finite marginalization over outcomes.
pub fn evaluate_action(
    action: &DiagnosticAction,
    belief: BinaryBelief,
    budget: ResourceBudget,
) -> Result<ActionAssessment, DecisionValueError> {
    evaluate_action_with_loss(action, belief, budget, DecisionLoss::zero_one())
}

/// Evaluate one candidate with explicit false-safe and false-degraded losses.
pub fn evaluate_action_with_loss(
    action: &DiagnosticAction,
    belief: BinaryBelief,
    budget: ResourceBudget,
    loss: DecisionLoss,
) -> Result<ActionAssessment, DecisionValueError> {
    // Revalidate even if the caller constructed the public structs directly.
    BinaryBelief::new(belief.probability_state_one)?;
    loss.validate()?;
    validate_action(action)?;

    let p0 = belief.probability_state_zero();
    let p1 = belief.probability_state_one;
    let baseline_bayes_risk = belief.bayes_risk_with_loss(loss)?;
    let mut bayes_risk_after_success = 0.0;
    let mut mutual_information_bits_on_success = 0.0;

    for outcome in &action.outcomes {
        let joint_state_zero = p0 * outcome.likelihood_state_zero;
        let joint_state_one = p1 * outcome.likelihood_state_one;
        let probability_outcome = joint_state_zero + joint_state_one;

        // Bayes classification loss after this outcome is the smaller joint
        // mass. Summing it over outcomes gives expected posterior Bayes risk.
        bayes_risk_after_success +=
            (joint_state_zero * loss.false_safe_loss)
                .min(joint_state_one * loss.false_degraded_loss);

        // I(State; Outcome) = sum_s,o P(s,o) log2(P(o|s) / P(o)).
        // Zero-probability terms contribute exactly zero.
        if probability_outcome > 0.0 {
            if joint_state_zero > 0.0 {
                mutual_information_bits_on_success += joint_state_zero
                    * (outcome.likelihood_state_zero / probability_outcome).log2();
            }
            if joint_state_one > 0.0 {
                mutual_information_bits_on_success += joint_state_one
                    * (outcome.likelihood_state_one / probability_outcome).log2();
            }
        }
    }

    let success = action.acquisition_success_probability;
    let expected_bayes_risk =
        success * bayes_risk_after_success + (1.0 - success) * baseline_bayes_risk;
    let expected_decision_value =
        (baseline_bayes_risk - expected_bayes_risk).max(0.0);
    let mutual_information_bits_on_success = mutual_information_bits_on_success.max(0.0);
    let expected_mutual_information_bits = success * mutual_information_bits_on_success;

    if !bayes_risk_after_success.is_finite()
        || !expected_bayes_risk.is_finite()
        || !expected_decision_value.is_finite()
        || !mutual_information_bits_on_success.is_finite()
        || !expected_mutual_information_bits.is_finite()
    {
        return Err(DecisionValueError::NonFiniteMetric(action.id.clone()));
    }

    Ok(ActionAssessment {
        action_id: action.id.clone(),
        cost: action.cost,
        feasible: action.cost.fits(budget),
        baseline_bayes_risk,
        bayes_risk_after_success,
        expected_bayes_risk,
        expected_decision_value,
        mutual_information_bits_on_success,
        expected_mutual_information_bits,
    })
}

/// Evaluate all candidates in a stable order, rejecting duplicate action IDs.
pub fn evaluate_actions(
    actions: &[DiagnosticAction],
    belief: BinaryBelief,
    budget: ResourceBudget,
) -> Result<Vec<ActionAssessment>, DecisionValueError> {
    evaluate_actions_with_loss(actions, belief, budget, DecisionLoss::zero_one())
}

/// Evaluate every candidate under one shared decision-loss model.
pub fn evaluate_actions_with_loss(
    actions: &[DiagnosticAction],
    belief: BinaryBelief,
    budget: ResourceBudget,
    loss: DecisionLoss,
) -> Result<Vec<ActionAssessment>, DecisionValueError> {
    loss.validate()?;
    let mut ids = BTreeSet::new();
    let mut assessments = Vec::with_capacity(actions.len());
    for action in actions {
        if !ids.insert(action.id.clone()) {
            return Err(DecisionValueError::DuplicateActionId(action.id.clone()));
        }
        assessments.push(evaluate_action_with_loss(action, belief, budget, loss)?);
    }
    assessments.sort_by(|a, b| a.action_id.cmp(&b.action_id));
    Ok(assessments)
}

fn compare_assessments(
    left: &ActionAssessment,
    right: &ActionAssessment,
    criterion: SelectionCriterion,
) -> std::cmp::Ordering {
    let primary = match criterion {
        SelectionCriterion::ExpectedDecisionValue => left
            .expected_decision_value
            .total_cmp(&right.expected_decision_value),
        SelectionCriterion::ExpectedMutualInformation => left
            .expected_mutual_information_bits
            .total_cmp(&right.expected_mutual_information_bits),
    };
    if primary != std::cmp::Ordering::Equal {
        return primary;
    }

    // Deterministic tie-break: lower energy, then latency, then bandwidth,
    // then lexicographically smaller action ID.
    right
        .cost
        .energy_units
        .cmp(&left.cost.energy_units)
        .then_with(|| right.cost.latency_ticks.cmp(&left.cost.latency_ticks))
        .then_with(|| right.cost.bandwidth_units.cmp(&left.cost.bandwidth_units))
        .then_with(|| right.action_id.cmp(&left.action_id))
}

/// Exact one-step oracle over the supplied finite candidate set.
///
/// Infeasible actions and candidates with zero value under the selected
/// criterion are not returned. The function does not optimize action sequences.
pub fn select_action(
    actions: &[DiagnosticAction],
    belief: BinaryBelief,
    budget: ResourceBudget,
    criterion: SelectionCriterion,
) -> Result<Option<ActionAssessment>, DecisionValueError> {
    select_action_with_loss(
        actions,
        belief,
        budget,
        criterion,
        DecisionLoss::zero_one(),
    )
}

/// Select with a cost-sensitive navigation decision objective.
pub fn select_action_with_loss(
    actions: &[DiagnosticAction],
    belief: BinaryBelief,
    budget: ResourceBudget,
    criterion: SelectionCriterion,
    loss: DecisionLoss,
) -> Result<Option<ActionAssessment>, DecisionValueError> {
    let assessments = evaluate_actions_with_loss(actions, belief, budget, loss)?;
    Ok(assessments
        .into_iter()
        .filter(|assessment| {
            assessment.feasible
                && match criterion {
                    SelectionCriterion::ExpectedDecisionValue => {
                        assessment.expected_decision_value > 0.0
                    }
                    SelectionCriterion::ExpectedMutualInformation => {
                        assessment.expected_mutual_information_bits > 0.0
                    }
                }
        })
        .max_by(|left, right| compare_assessments(left, right, criterion)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_actions() -> Vec<DiagnosticAction> {
        vec![
            // More mutual information, but neither outcome changes the
            // MAP decision under the 90% StateOne prior.
            DiagnosticAction {
                id: "balanced_entropy".to_owned(),
                cost: ActionCost {
                    energy_units: 1,
                    latency_ticks: 1,
                    bandwidth_units: 1,
                },
                acquisition_success_probability: 1.0,
                outcomes: vec![
                    ObservationOutcome {
                        id: "supports_state_one".to_owned(),
                        likelihood_state_zero: 0.11,
                        likelihood_state_one: 0.89,
                    },
                    ObservationOutcome {
                        id: "supports_state_zero".to_owned(),
                        likelihood_state_zero: 0.89,
                        likelihood_state_one: 0.11,
                    },
                ],
            },
            // Lower mutual information, but a sufficiently discriminative
            // adverse-state alert changes the downstream MAP decision.
            DiagnosticAction {
                id: "decision_relevant_alert".to_owned(),
                cost: ActionCost {
                    energy_units: 2,
                    latency_ticks: 2,
                    bandwidth_units: 1,
                },
                acquisition_success_probability: 0.8,
                outcomes: vec![
                    ObservationOutcome {
                        id: "adverse_alert".to_owned(),
                        likelihood_state_zero: 0.70,
                        likelihood_state_one: 0.04,
                    },
                    ObservationOutcome {
                        id: "no_alert".to_owned(),
                        likelihood_state_zero: 0.30,
                        likelihood_state_one: 0.96,
                    },
                ],
            },
        ]
    }

    fn budget() -> ResourceBudget {
        ResourceBudget {
            energy_units: 2,
            latency_ticks: 2,
            bandwidth_units: 1,
        }
    }

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-10,
            "actual {actual}, expected {expected}"
        );
    }

    #[test]
    fn mutual_information_and_decision_value_can_rank_actions_differently() {
        let belief = BinaryBelief::new(0.9).unwrap();
        let actions = demo_actions();

        let information = select_action(
            &actions,
            belief,
            budget(),
            SelectionCriterion::ExpectedMutualInformation,
        )
        .unwrap()
        .unwrap();
        let decision = select_action(
            &actions,
            belief,
            budget(),
            SelectionCriterion::ExpectedDecisionValue,
        )
        .unwrap()
        .unwrap();

        assert_eq!(information.action_id, "balanced_entropy");
        assert_eq!(decision.action_id, "decision_relevant_alert");
        close(information.expected_decision_value, 0.0);
        close(decision.expected_decision_value, 0.0272);
        assert!(
            information.expected_mutual_information_bits
                > decision.expected_mutual_information_bits
        );
    }

    #[test]
    fn acquisition_failure_scales_value_and_zero_success_has_no_value() {
        let belief = BinaryBelief::new(0.9).unwrap();
        let mut action = demo_actions().remove(1);
        let full = evaluate_action(&action, belief, budget()).unwrap();
        close(full.expected_decision_value, 0.0272);

        action.acquisition_success_probability = 0.4;
        let half = evaluate_action(&action, belief, budget()).unwrap();
        close(half.expected_decision_value, 0.0136);

        action.acquisition_success_probability = 0.0;
        let failed = evaluate_action(&action, belief, budget()).unwrap();
        close(failed.expected_decision_value, 0.0);
        close(failed.expected_mutual_information_bits, 0.0);
        assert!(
            select_action(
                &[action],
                belief,
                budget(),
                SelectionCriterion::ExpectedDecisionValue
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn infeasible_high_value_action_is_never_selected() {
        let belief = BinaryBelief::new(0.9).unwrap();
        let mut action = demo_actions().remove(1);
        action.cost.energy_units = 3;
        let result = evaluate_action(&action, belief, budget()).unwrap();
        assert!(!result.feasible);
        assert!(
            select_action(
                &[action],
                belief,
                budget(),
                SelectionCriterion::ExpectedDecisionValue
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn non_normalized_outcome_likelihoods_fail_closed() {
        let belief = BinaryBelief::new(0.9).unwrap();
        let mut action = demo_actions().remove(1);
        action.outcomes[0].likelihood_state_zero = 0.60;
        assert!(matches!(
            evaluate_action(&action, belief, budget()),
            Err(DecisionValueError::OutcomeMassMismatch {
                state: "StateZero",
                ..
            })
        ));
    }

    #[test]
    fn candidate_order_does_not_change_the_selected_action() {
        let belief = BinaryBelief::new(0.9).unwrap();
        let mut actions = demo_actions();
        let original = select_action(
            &actions,
            belief,
            budget(),
            SelectionCriterion::ExpectedDecisionValue,
        )
        .unwrap()
        .unwrap();
        actions.reverse();
        let reversed = select_action(
            &actions,
            belief,
            budget(),
            SelectionCriterion::ExpectedDecisionValue,
        )
        .unwrap()
        .unwrap();
        assert_eq!(original, reversed);
    }

    #[test]
    fn outcome_order_does_not_change_metrics() {
        let belief = BinaryBelief::new(0.9).unwrap();
        let mut action = demo_actions().remove(1);
        let original = evaluate_action(&action, belief, budget()).unwrap();
        action.outcomes.reverse();
        let reversed = evaluate_action(&action, belief, budget()).unwrap();
        close(original.expected_decision_value, reversed.expected_decision_value);
        close(
            original.expected_mutual_information_bits,
            reversed.expected_mutual_information_bits,
        );
    }

    #[test]
    fn perfect_observation_reduces_zero_one_bayes_risk_to_zero() {
        let action = DiagnosticAction {
            id: "perfect".to_owned(),
            cost: ActionCost::default(),
            acquisition_success_probability: 1.0,
            outcomes: vec![
                ObservationOutcome {
                    id: "state_zero".to_owned(),
                    likelihood_state_zero: 1.0,
                    likelihood_state_one: 0.0,
                },
                ObservationOutcome {
                    id: "state_one".to_owned(),
                    likelihood_state_zero: 0.0,
                    likelihood_state_one: 1.0,
                },
            ],
        };
        let result = evaluate_action(
            &action,
            BinaryBelief::new(0.9).unwrap(),
            ResourceBudget::default(),
        )
        .unwrap();
        close(result.bayes_risk_after_success, 0.0);
        close(result.expected_decision_value, 0.1);
        close(result.expected_mutual_information_bits, 0.4689955935892812);
    }

    #[test]
    fn asymmetric_false_safe_loss_changes_expected_decision_value() {
        let belief = BinaryBelief::new(0.9).unwrap();
        let action = demo_actions().remove(1);
        let symmetric = evaluate_action(&action, belief, budget()).unwrap();
        let asymmetric_loss = DecisionLoss::new(10.0, 1.0).unwrap();
        let asymmetric =
            evaluate_action_with_loss(&action, belief, budget(), asymmetric_loss).unwrap();

        close(symmetric.expected_decision_value, 0.0272);
        close(asymmetric.baseline_bayes_risk, 0.9);
        close(asymmetric.bayes_risk_after_success, 0.336);
        close(asymmetric.expected_bayes_risk, 0.4488);
        close(asymmetric.expected_decision_value, 0.4512);
        assert!(asymmetric.expected_decision_value > symmetric.expected_decision_value);
    }

    #[test]
    fn invalid_asymmetric_decision_losses_are_rejected() {
        assert!(DecisionLoss::new(-1.0, 1.0).is_err());
        assert!(DecisionLoss::new(f64::NAN, 1.0).is_err());
        assert!(DecisionLoss::new(0.0, 0.0).is_err());

        let action = demo_actions().remove(1);
        assert!(matches!(
            evaluate_action_with_loss(
                &action,
                BinaryBelief::new(0.9).unwrap(),
                budget(),
                DecisionLoss {
                    false_safe_loss: 1.0,
                    false_degraded_loss: f64::INFINITY,
                },
            ),
            Err(DecisionValueError::InvalidDecisionLoss)
        ));
    }

    #[test]
    fn duplicate_action_ids_are_rejected() {
        let belief = BinaryBelief::new(0.9).unwrap();
        let action = demo_actions().remove(0);
        assert!(matches!(
            evaluate_actions(&[action.clone(), action], belief, budget()),
            Err(DecisionValueError::DuplicateActionId(_))
        ));
    }

    #[test]
    fn invalid_prior_and_acquisition_probabilities_are_rejected() {
        assert!(BinaryBelief::new(f64::NAN).is_err());
        assert!(BinaryBelief::new(1.01).is_err());
        let mut action = demo_actions().remove(0);
        action.acquisition_success_probability = f64::INFINITY;
        assert!(matches!(
            evaluate_action(&action, BinaryBelief::new(0.9).unwrap(), budget()),
            Err(DecisionValueError::InvalidAcquisitionProbability(_))
        ));
    }
}
