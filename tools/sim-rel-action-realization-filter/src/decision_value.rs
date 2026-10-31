//! Exact one-step downstream decision-value evaluation.
//!
//! This module separates mechanism information from the downstream decision
//! that the evidence supports. Losses are defined over the mechanism and next
//! physical state; sensor and apparatus uncertainty still affect the posterior.

use super::{
    ActionRealizationKernel, BeliefMass, FilterError, ObservationField, PosteriorMass,
    PublicObservation, Rational, filter_one_step,
};
use std::collections::BTreeSet;
use std::fmt;

/// Finite terminal-decision losses indexed by (decision, mechanism, physical state).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionLossTable {
    pub decision_ids: Vec<String>,
    pub losses: Vec<Vec<Vec<Rational>>>,
    pub abstention_loss: Rational,
}

impl DecisionLossTable {
    pub fn validate(&self, kernel: &ActionRealizationKernel) -> Result<(), String> {
        if self.decision_ids.is_empty() {
            return Err("decision vocabulary must be non-empty".into());
        }
        let mut ids = BTreeSet::new();
        if self
            .decision_ids
            .iter()
            .any(|id| id.trim().is_empty() || !ids.insert(id))
        {
            return Err("decision IDs must be non-empty and unique".into());
        }
        if self.losses.len() != self.decision_ids.len() {
            return Err("loss table must have one matrix per decision".into());
        }
        for (decision, matrix) in self.losses.iter().enumerate() {
            if matrix.len() != kernel.hypothesis_ids.len() {
                return Err(format!(
                    "decision {decision} has the wrong mechanism dimension"
                ));
            }
            for (hypothesis, row) in matrix.iter().enumerate() {
                if row.len() != kernel.physical_state_count {
                    return Err(format!(
                        "decision {decision}, mechanism {hypothesis} has the wrong physical-state dimension"
                    ));
                }
            }
        }
        Ok(())
    }
}

/// A Bayes-optimal terminal choice under the supplied finite loss table.
///
/// `decision=None` means explicit abstention / escalation. Exact ties are
/// resolved in favor of abstention; ties among concrete decisions preserve
/// their declared vocabulary order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BayesDecision {
    pub decision: Option<usize>,
    pub expected_loss: Rational,
}

/// Declares which public outcome channels are available to the policy.
/// Missingness is informative only when the channel's presence/absence itself
/// was observed; a disabled channel is marginalized using ObservationField::Any.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObservationProtocol {
    pub observe_readback: bool,
    pub observe_measurement: bool,
}

impl ObservationProtocol {
    pub const fn fully_observed() -> Self {
        Self {
            observe_readback: true,
            observe_measurement: true,
        }
    }

    pub const fn no_telemetry() -> Self {
        Self {
            observe_readback: false,
            observe_measurement: false,
        }
    }
}

/// Exact comparison of one commanded experiment against stopping immediately.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OneStepDecisionValue {
    pub baseline: BayesDecision,
    /// Expected terminal decision/hazard loss, before charging command cost.
    pub expected_terminal_loss: Rational,
    /// Command cost is charged once even when the command hazards.
    pub command_cost: Rational,
    pub expected_total_loss: Rational,
    pub hazard_probability: Rational,
    pub public_observation_branches: usize,
    pub experiment_is_preferred: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecisionValueError {
    InvalidKernel(String),
    InvalidBelief(String),
    InvalidLossTable(String),
    InvalidCommand,
    Filter(FilterError),
    Arithmetic(String),
}

impl fmt::Display for DecisionValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKernel(message) => write!(f, "invalid kernel: {message}"),
            Self::InvalidBelief(message) => write!(f, "invalid belief: {message}"),
            Self::InvalidLossTable(message) => write!(f, "invalid loss table: {message}"),
            Self::InvalidCommand => write!(f, "command ID is out of range"),
            Self::Filter(error) => write!(f, "posterior update failed: {error}"),
            Self::Arithmetic(message) => write!(f, "exact arithmetic failed: {message}"),
        }
    }
}

impl std::error::Error for DecisionValueError {}

fn select_minimum(expected_losses: &[Rational], abstention_loss: Rational) -> BayesDecision {
    let mut selected = None;
    let mut best = abstention_loss;
    for (decision, loss) in expected_losses.iter().copied().enumerate() {
        if loss < best {
            selected = Some(decision);
            best = loss;
        }
    }
    BayesDecision {
        decision: selected,
        expected_loss: best,
    }
}

fn expected_losses_from_belief(
    table: &DecisionLossTable,
    belief: &[BeliefMass],
) -> Result<Vec<Rational>, DecisionValueError> {
    let mut losses = Vec::with_capacity(table.decision_ids.len());
    for decision in 0..table.decision_ids.len() {
        let mut expected = Rational::zero();
        for point in belief {
            let loss = table.losses[decision][point.state.hypothesis]
                [point.state.physical_state];
            let weighted = point
                .mass
                .checked_mul(loss)
                .map_err(DecisionValueError::Arithmetic)?;
            expected = expected
                .checked_add(weighted)
                .map_err(DecisionValueError::Arithmetic)?;
        }
        losses.push(expected);
    }
    Ok(losses)
}

fn expected_losses_from_posterior(
    table: &DecisionLossTable,
    posterior: &[PosteriorMass],
) -> Result<Vec<Rational>, DecisionValueError> {
    let mut losses = vec![Rational::zero(); table.decision_ids.len()];
    for (decision, expected) in losses.iter_mut().enumerate() {
        for point in posterior {
            let loss = table.losses[decision][point.next_state.hypothesis]
                [point.next_state.physical_state];
            let weighted = point
                .mass
                .checked_mul(loss)
                .map_err(DecisionValueError::Arithmetic)?;
            *expected = expected
                .checked_add(weighted)
                .map_err(DecisionValueError::Arithmetic)?;
        }
    }
    Ok(losses)
}

/// Choose the minimum expected-loss terminal decision from a normalized joint belief.
pub fn bayes_decision_from_belief(
    kernel: &ActionRealizationKernel,
    belief: &[BeliefMass],
    table: &DecisionLossTable,
) -> Result<BayesDecision, DecisionValueError> {
    kernel
        .validate()
        .map_err(DecisionValueError::InvalidKernel)?;
    super::validate_belief(kernel, belief).map_err(DecisionValueError::InvalidBelief)?;
    table
        .validate(kernel)
        .map_err(DecisionValueError::InvalidLossTable)?;
    let losses = expected_losses_from_belief(table, belief)?;
    Ok(select_minimum(&losses, table.abstention_loss))
}

/// Choose the minimum expected-loss terminal decision from an exact posterior.
pub fn bayes_decision_from_posterior(
    kernel: &ActionRealizationKernel,
    table: &DecisionLossTable,
    posterior: &[PosteriorMass],
) -> Result<BayesDecision, DecisionValueError> {
    kernel
        .validate()
        .map_err(DecisionValueError::InvalidKernel)?;
    if posterior.is_empty() {
        return Err(DecisionValueError::InvalidBelief(
            "posterior is empty".into(),
        ));
    }
    table
        .validate(kernel)
        .map_err(DecisionValueError::InvalidLossTable)?;
    let mut total = Rational::zero();
    for point in posterior {
        if point.next_state.hypothesis >= kernel.hypothesis_ids.len()
            || point.next_state.physical_state >= kernel.physical_state_count
            || point.next_state.sensor_state >= kernel.sensor_state_count
            || point.next_state.apparatus_state >= kernel.apparatus_state_count
            || point.realized_action >= kernel.realized_action_ids.len()
            || point.mass.is_zero()
        {
            return Err(DecisionValueError::InvalidBelief(
                "posterior contains an invalid state, action or zero mass".into(),
            ));
        }
        total = total
            .checked_add(point.mass)
            .map_err(DecisionValueError::Arithmetic)?;
    }
    if total != Rational::one() {
        return Err(DecisionValueError::InvalidBelief(
            "posterior mass must sum exactly to one".into(),
        ));
    }
    let losses = expected_losses_from_posterior(table, posterior)?;
    Ok(select_minimum(&losses, table.abstention_loss))
}

/// Compare one experiment against immediate stopping under a finite downstream
/// loss table. Every command outcome is treated as public: a missing readback is
/// distinct from an unobserved field, and hazards receive the caller's explicit
/// terminal hazard loss. The command cost is paid once on every path.
pub fn evaluate_one_step_decision_value(
    kernel: &ActionRealizationKernel,
    belief: &[BeliefMass],
    command: usize,
    table: &DecisionLossTable,
    protocol: ObservationProtocol,
    command_cost: Rational,
    hazard_loss: Rational,
) -> Result<OneStepDecisionValue, DecisionValueError> {
    kernel
        .validate()
        .map_err(DecisionValueError::InvalidKernel)?;
    super::validate_belief(kernel, belief).map_err(DecisionValueError::InvalidBelief)?;
    table
        .validate(kernel)
        .map_err(DecisionValueError::InvalidLossTable)?;
    if command >= kernel.command_ids.len() {
        return Err(DecisionValueError::InvalidCommand);
    }

    let baseline = bayes_decision_from_belief(kernel, belief, table)?;
    let mut observations = BTreeSet::<(bool, ObservationField, ObservationField)>::new();
    for point in belief {
        let row = &kernel.rows[kernel.row_index(point.state, command)];
        for outcome in &row.outcomes {
            observations.insert((
                outcome.hazard,
                field_from_option(outcome.readback, protocol.observe_readback),
                field_from_option(outcome.measurement, protocol.observe_measurement),
            ));
        }
    }

    let mut expected_terminal_loss = Rational::zero();
    let mut hazard_probability = Rational::zero();
    for (hazard, readback, measurement) in &observations {
        let public = PublicObservation {
            hazard: *hazard,
            readback: *readback,
            measurement: *measurement,
        };
        let filtered = super::filter_one_step(kernel, belief, command, public)
            .map_err(DecisionValueError::Filter)?;
        let branch_probability = filtered.observation_likelihood;
        let branch_loss = if filtered.terminal_hazard {
            hazard_probability = hazard_probability
                .checked_add(branch_probability)
                .map_err(DecisionValueError::Arithmetic)?;
            hazard_loss
        } else {
            bayes_decision_from_posterior(kernel, table, &filtered.joint_posterior)?.expected_loss
        };
        let contribution = branch_probability
            .checked_mul(branch_loss)
            .map_err(DecisionValueError::Arithmetic)?;
        expected_terminal_loss = expected_terminal_loss
            .checked_add(contribution)
            .map_err(DecisionValueError::Arithmetic)?;
    }

    let expected_total_loss = expected_terminal_loss
        .checked_add(command_cost)
        .map_err(DecisionValueError::Arithmetic)?;

    Ok(OneStepDecisionValue {
        baseline,
        expected_terminal_loss,
        command_cost,
        expected_total_loss,
        hazard_probability,
        public_observation_branches: observations.len(),
        experiment_is_preferred: expected_total_loss < baseline.expected_loss,
    })
}

fn field_from_option(value: Option<usize>, observed: bool) -> ObservationField {
    if !observed {
        return ObservationField::Any;
    }
    match value {
        Some(id) => ObservationField::Value(id),
        None => ObservationField::Missing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::LatentState;

    fn decision_screen_kernel() -> ActionRealizationKernel {
        let mut rows = Vec::new();
        for hypothesis in 0..3 {
            let state = LatentState {
                hypothesis,
                physical_state: 0,
                sensor_state: 0,
                apparatus_state: 0,
            };
            for command in 0..2 {
                let outcomes = if command == 0 {
                    match hypothesis {
                        0 => vec![outcome(Rational::one(), 0, hypothesis)],
                        1 => vec![outcome(Rational::one(), 1, hypothesis)],
                        _ => vec![
                            outcome(Rational::new(1, 2).unwrap(), 0, hypothesis),
                            outcome(Rational::new(1, 2).unwrap(), 1, hypothesis),
                        ],
                    }
                } else if hypothesis == 2 {
                    vec![outcome(Rational::one(), 1, hypothesis)]
                } else {
                    vec![outcome(Rational::one(), 0, hypothesis)]
                };
                rows.push(super::super::KernelRow {
                    state,
                    command,
                    outcomes,
                });
            }
        }
        let initial_belief = vec![
            BeliefMass {
                state: LatentState {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 0,
                    apparatus_state: 0,
                },
                mass: Rational::new(80, 100).unwrap(),
            },
            BeliefMass {
                state: LatentState {
                    hypothesis: 1,
                    physical_state: 0,
                    sensor_state: 0,
                    apparatus_state: 0,
                },
                mass: Rational::new(19, 100).unwrap(),
            },
            BeliefMass {
                state: LatentState {
                    hypothesis: 2,
                    physical_state: 0,
                    sensor_state: 0,
                    apparatus_state: 0,
                },
                mass: Rational::new(1, 100).unwrap(),
            },
        ];
        ActionRealizationKernel {
            hypothesis_ids: vec!["ordinary-a".into(), "ordinary-b".into(), "rare-critical".into()],
            physical_state_count: 1,
            sensor_state_count: 1,
            apparatus_state_count: 1,
            command_ids: vec!["detail-probe".into(), "critical-regime-screen".into()],
            realized_action_ids: vec!["executed".into()],
            readback_ids: vec!["ok".into()],
            measurement_ids: vec!["negative".into(), "positive".into()],
            initial_belief,
            rows,
        }
    }

    fn outcome(
        probability: Rational,
        measurement: usize,
        hypothesis: usize,
    ) -> super::super::TransitionOutcome {
        super::super::TransitionOutcome {
            probability,
            next_state: LatentState {
                hypothesis,
                physical_state: 0,
                sensor_state: 0,
                apparatus_state: 0,
            },
            realized_action: 0,
            readback: Some(0),
            measurement: Some(measurement),
            hazard: false,
        }
    }

    fn decision_losses() -> DecisionLossTable {
        DecisionLossTable {
            decision_ids: vec!["release".into(), "hold".into()],
            losses: vec![
                vec![
                    vec![Rational::zero()],
                    vec![Rational::zero()],
                    vec![Rational::from_u64(1_000)],
                ],
                vec![
                    vec![Rational::one()],
                    vec![Rational::one()],
                    vec![Rational::zero()],
                ],
            ],
            abstention_loss: Rational::new(2, 1).unwrap(),
        }
    }

    #[test]
    fn low_eig_decision_screen_can_beat_higher_eig_detail_probe() {
        let kernel = decision_screen_kernel();
        let table = decision_losses();
        let baseline =
            bayes_decision_from_belief(&kernel, &kernel.initial_belief, &table).unwrap();
        assert_eq!(baseline.decision, Some(1));
        assert_eq!(baseline.expected_loss, Rational::new(99, 100).unwrap());

        let detail = evaluate_one_step_decision_value(
            &kernel,
            &kernel.initial_belief,
            0,
            &table,
            ObservationProtocol::fully_observed(),
            Rational::new(1, 100).unwrap(),
            Rational::zero(),
        )
        .unwrap();
        let critical = evaluate_one_step_decision_value(
            &kernel,
            &kernel.initial_belief,
            1,
            &table,
            ObservationProtocol::fully_observed(),
            Rational::new(1, 100).unwrap(),
            Rational::zero(),
        )
        .unwrap();

        // The detail probe separates two mechanisms that share one decision.
        // It leaves the downstream Bayes risk unchanged despite providing
        // substantially more mechanism information than the rare-regime screen.
        assert_eq!(detail.expected_terminal_loss, Rational::new(99, 100).unwrap());
        assert!(!detail.experiment_is_preferred);

        // The rarer regime-screen has lower mechanism information but resolves
        // the high-consequence decision, so its expected terminal loss is zero.
        assert_eq!(critical.expected_terminal_loss, Rational::zero());
        assert_eq!(critical.expected_total_loss, Rational::new(1, 100).unwrap());
        assert!(critical.experiment_is_preferred);
    }

    #[test]
    fn abstention_is_selected_when_its_declared_loss_is_lower() {
        let kernel = decision_screen_kernel();
        let mut table = decision_losses();
        table.abstention_loss = Rational::zero();
        let decision =
            bayes_decision_from_belief(&kernel, &kernel.initial_belief, &table).unwrap();
        assert_eq!(decision.decision, None);
        assert_eq!(decision.expected_loss, Rational::zero());
    }
}
