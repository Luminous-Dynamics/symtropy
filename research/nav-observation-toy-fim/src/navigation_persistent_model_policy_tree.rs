//! Exact horizon-two active sensing with a mission-persistent observation model.
//!
//! The hidden state is (navigation service condition, model identity). The
//! model identity is selected once for the mission and its likelihood row is
//! retained across all branches. Observations can update both the navigation
//! and model beliefs, including the no-observation outcome. This is an exact
//! finite Bayesian model-mixture reference, not a minimax guarantee or a
//! physical calibration result.

use core::fmt;

pub const PROBABILITY_SCALE: u64 = 1_000_000_000;
pub const MAX_ACTIONS: usize = 16;
pub const MAX_TERMINAL_LOSS: u64 = 1_000_000_000;

const SCALE: u128 = PROBABILITY_SCALE as u128;
const RISK_DENOMINATOR: u128 = SCALE * SCALE * SCALE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelState {
    ModelZeroInside,
    ModelZeroOutside,
    ModelOneInside,
    ModelOneOutside,
}

const MODEL_STATES: [ModelState; 4] = [
    ModelState::ModelZeroInside,
    ModelState::ModelZeroOutside,
    ModelState::ModelOneInside,
    ModelState::ModelOneOutside,
];

impl ModelState {
    fn index(self) -> usize {
        match self {
            Self::ModelZeroInside => 0,
            Self::ModelZeroOutside => 1,
            Self::ModelOneInside => 2,
            Self::ModelOneOutside => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionOutcome {
    NoObservation,
    ObservedZero,
    ObservedOne,
}

const OUTCOME_ORDER: [ActionOutcome; 3] = [
    ActionOutcome::NoObservation,
    ActionOutcome::ObservedZero,
    ActionOutcome::ObservedOne,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutcomeLikelihoods {
    pub no_observation_parts_per_billion: u64,
    pub observed_zero_parts_per_billion: u64,
    pub observed_one_parts_per_billion: u64,
}

impl OutcomeLikelihoods {
    fn probability(self, outcome: ActionOutcome) -> u64 {
        match outcome {
            ActionOutcome::NoObservation => self.no_observation_parts_per_billion,
            ActionOutcome::ObservedZero => self.observed_zero_parts_per_billion,
            ActionOutcome::ObservedOne => self.observed_one_parts_per_billion,
        }
    }

    fn total(self) -> Option<u64> {
        self.no_observation_parts_per_billion
            .checked_add(self.observed_zero_parts_per_billion)?
            .checked_add(self.observed_one_parts_per_billion)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResourceVector {
    pub latency_ns: u64,
    pub energy_uj: u64,
    pub bandwidth_bytes: u64,
}

impl ResourceVector {
    fn fits_within(self, budget: Self) -> bool {
        self.latency_ns <= budget.latency_ns
            && self.energy_uj <= budget.energy_uj
            && self.bandwidth_bytes <= budget.bandwidth_bytes
    }

    fn checked_add(self, other: Self) -> Result<Self, PersistentModelPolicyTreeError> {
        Ok(Self {
            latency_ns: self.latency_ns.checked_add(other.latency_ns)
                .ok_or(PersistentModelPolicyTreeError::ArithmeticOverflow)?,
            energy_uj: self.energy_uj.checked_add(other.energy_uj)
                .ok_or(PersistentModelPolicyTreeError::ArithmeticOverflow)?,
            bandwidth_bytes: self.bandwidth_bytes.checked_add(other.bandwidth_bytes)
                .ok_or(PersistentModelPolicyTreeError::ArithmeticOverflow)?,
        })
    }

    fn remaining_after(self, spent: Self) -> Option<Self> {
        Some(Self {
            latency_ns: self.latency_ns.checked_sub(spent.latency_ns)?,
            energy_uj: self.energy_uj.checked_sub(spent.energy_uj)?,
            bandwidth_bytes: self.bandwidth_bytes.checked_sub(spent.bandwidth_bytes)?,
        })
    }

    fn component_max(self, other: Self) -> Self {
        Self {
            latency_ns: self.latency_ns.max(other.latency_ns),
            energy_uj: self.energy_uj.max(other.energy_uj),
            bandwidth_bytes: self.bandwidth_bytes.max(other.bandwidth_bytes),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentModelAction {
    pub action_id: u64,
    pub cost: ResourceVector,
    /// Likelihoods for [model 0/inside, model 0/outside,
    /// model 1/inside, model 1/outside]. The model-specific rows persist.
    pub likelihoods_by_model_state: [OutcomeLikelihoods; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersistentModelDecisionProblem {
    /// Joint prior PPB masses for [model 0/inside, model 0/outside,
    /// model 1/inside, model 1/outside]. This encodes both model prior and
    /// model-conditional navigation prior and must sum exactly to 1e9.
    pub joint_prior_parts_per_billion: [u64; 4],
    pub false_safe_loss: u64,
    pub false_degraded_loss: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactProbability {
    pub numerator: u128,
    pub denominator: u128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalDecision {
    DeclareSafe,
    DeclareDegraded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlannerKind {
    ExactHorizonTwo,
    OneStepGreedy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecondStageOutcome {
    pub outcome: ActionOutcome,
    pub posterior_outside_probability: ExactProbability,
    pub posterior_model_one_probability: Option<ExactProbability>,
    pub terminal_decision: TerminalDecision,
    pub terminal_loss_numerator: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FirstActionBranch {
    pub outcome: ActionOutcome,
    pub posterior_outside_probability: ExactProbability,
    pub posterior_model_one_probability: Option<ExactProbability>,
    pub second_action_id: Option<u64>,
    pub terminal_decision: Option<TerminalDecision>,
    pub second_stage_outcomes: Vec<SecondStageOutcome>,
    pub resource_use: ResourceVector,
    pub subtree_loss_numerator: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyTreePlan {
    pub planner: PlannerKind,
    pub first_action_id: Option<u64>,
    pub expected_terminal_loss_numerator: u128,
    pub expected_terminal_loss_denominator: u128,
    pub worst_case_resource_use: ResourceVector,
    pub branches: Vec<FirstActionBranch>,
    pub terminal_decision_if_no_first_action: Option<TerminalDecision>,
    /// This offline plan cannot authorize a navigation or actuation transition.
    pub authority_transition_not_authorized_by_plan: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PersistentModelPolicyTreeError {
    InvalidJointPrior,
    InvalidDecisionModel,
    TooManyActions,
    InvalidActionId,
    DuplicateActionId(u64),
    InvalidLikelihoods {
        action_id: u64,
        hidden_state: ModelState,
    },
    ActionNotFound(u64),
    ArithmeticOverflow,
}

impl fmt::Display for PersistentModelPolicyTreeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJointPrior => write!(formatter, "joint model/navigation prior must sum exactly to one billion"),
            Self::InvalidDecisionModel => write!(formatter, "terminal decision losses are outside their valid range"),
            Self::TooManyActions => write!(formatter, "exact horizon-two oracle action count exceeds its fixed limit"),
            Self::InvalidActionId => write!(formatter, "action identity must be nonzero"),
            Self::DuplicateActionId(id) => write!(formatter, "action identity is duplicated: {id}"),
            Self::InvalidLikelihoods { action_id, hidden_state } => write!(
                formatter,
                "action {action_id} likelihoods for {hidden_state:?} must sum exactly to one billion"
            ),
            Self::ActionNotFound(id) => write!(formatter, "first action does not exist: {id}"),
            Self::ArithmeticOverflow => write!(formatter, "checked model-state probability or loss arithmetic overflowed"),
        }
    }
}

impl std::error::Error for PersistentModelPolicyTreeError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ModelStateMass {
    parts: [u128; 4],
}

impl ModelStateMass {
    fn total(self) -> Result<u128, PersistentModelPolicyTreeError> {
        self.parts.iter().try_fold(0_u128, |sum, mass| {
            sum.checked_add(*mass).ok_or(PersistentModelPolicyTreeError::ArithmeticOverflow)
        })
    }

    fn outside_mass(self) -> Result<u128, PersistentModelPolicyTreeError> {
        self.parts[1].checked_add(self.parts[3])
            .ok_or(PersistentModelPolicyTreeError::ArithmeticOverflow)
    }

    fn inside_mass(self) -> Result<u128, PersistentModelPolicyTreeError> {
        self.parts[0].checked_add(self.parts[2])
            .ok_or(PersistentModelPolicyTreeError::ArithmeticOverflow)
    }

    fn model_one_mass(self) -> Result<u128, PersistentModelPolicyTreeError> {
        self.parts[2].checked_add(self.parts[3])
            .ok_or(PersistentModelPolicyTreeError::ArithmeticOverflow)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OutcomeMass {
    outcome: ActionOutcome,
    mass: ModelStateMass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Continuation {
    action_id: Option<u64>,
    terminal_decision: Option<TerminalDecision>,
    second_stage_outcomes: Vec<SecondStageOutcome>,
    resource_use: ResourceVector,
    loss_numerator: u128,
}

fn validate_inputs<'a>(
    problem: &PersistentModelDecisionProblem,
    actions: &'a [PersistentModelAction],
) -> Result<Vec<&'a PersistentModelAction>, PersistentModelPolicyTreeError> {
    let prior_total = problem.joint_prior_parts_per_billion.iter()
        .try_fold(0_u64, |sum, mass| sum.checked_add(*mass));
    if prior_total != Some(PROBABILITY_SCALE) {
        return Err(PersistentModelPolicyTreeError::InvalidJointPrior);
    }
    if problem.false_safe_loss == 0
        || problem.false_degraded_loss == 0
        || problem.false_safe_loss > MAX_TERMINAL_LOSS
        || problem.false_degraded_loss > MAX_TERMINAL_LOSS
    {
        return Err(PersistentModelPolicyTreeError::InvalidDecisionModel);
    }
    if actions.len() > MAX_ACTIONS {
        return Err(PersistentModelPolicyTreeError::TooManyActions);
    }

    let mut ordered: Vec<&PersistentModelAction> = actions.iter().collect();
    ordered.sort_by_key(|action| action.action_id);
    for (index, action) in ordered.iter().enumerate() {
        if action.action_id == 0 {
            return Err(PersistentModelPolicyTreeError::InvalidActionId);
        }
        if index > 0 && ordered[index - 1].action_id == action.action_id {
            return Err(PersistentModelPolicyTreeError::DuplicateActionId(action.action_id));
        }
        for hidden_state in MODEL_STATES {
            let likelihoods = action.likelihoods_by_model_state[hidden_state.index()];
            if likelihoods.total() != Some(PROBABILITY_SCALE)
                || [
                    likelihoods.no_observation_parts_per_billion,
                    likelihoods.observed_zero_parts_per_billion,
                    likelihoods.observed_one_parts_per_billion,
                ].iter().any(|probability| *probability > PROBABILITY_SCALE)
            {
                return Err(PersistentModelPolicyTreeError::InvalidLikelihoods {
                    action_id: action.action_id,
                    hidden_state,
                });
            }
        }
    }
    Ok(ordered)
}

fn initial_mass(problem: &PersistentModelDecisionProblem) -> ModelStateMass {
    ModelStateMass {
        parts: problem.joint_prior_parts_per_billion.map(u128::from),
    }
}

fn exact_probability(
    numerator: u128,
    denominator: u128,
) -> Result<ExactProbability, PersistentModelPolicyTreeError> {
    if denominator == 0 {
        return Err(PersistentModelPolicyTreeError::ArithmeticOverflow);
    }
    Ok(ExactProbability { numerator, denominator })
}

fn posterior_outside_probability(
    mass: ModelStateMass,
) -> Result<ExactProbability, PersistentModelPolicyTreeError> {
    exact_probability(mass.outside_mass()?, mass.total()?)
}

fn posterior_model_one_probability(
    mass: ModelStateMass,
) -> Result<Option<ExactProbability>, PersistentModelPolicyTreeError> {
    let model_one_mass = mass.model_one_mass()?;
    if model_one_mass == 0 {
        return Ok(None);
    }
    exact_probability(model_one_mass, mass.total()?).map(Some)
}

fn terminal_assessment(
    mass: ModelStateMass,
    problem: &PersistentModelDecisionProblem,
) -> Result<(u128, TerminalDecision), PersistentModelPolicyTreeError> {
    // A single terminal decision is selected using the joint model mixture.
    // The controller does not observe the model identity.
    let false_safe_loss = mass.outside_mass()?
        .checked_mul(problem.false_safe_loss as u128)
        .ok_or(PersistentModelPolicyTreeError::ArithmeticOverflow)?;
    let false_degraded_loss = mass.inside_mass()?
        .checked_mul(problem.false_degraded_loss as u128)
        .ok_or(PersistentModelPolicyTreeError::ArithmeticOverflow)?;
    if false_safe_loss < false_degraded_loss {
        Ok((false_safe_loss, TerminalDecision::DeclareSafe))
    } else {
        Ok((false_degraded_loss, TerminalDecision::DeclareDegraded))
    }
}

fn expand_action(
    mass: ModelStateMass,
    action: &PersistentModelAction,
) -> Result<Vec<OutcomeMass>, PersistentModelPolicyTreeError> {
    let mut branches = Vec::with_capacity(OUTCOME_ORDER.len());
    for outcome in OUTCOME_ORDER {
        let mut next = [0_u128; 4];
        for hidden_state in MODEL_STATES {
            let index = hidden_state.index();
            next[index] = mass.parts[index]
                .checked_mul(action.likelihoods_by_model_state[index].probability(outcome) as u128)
                .ok_or(PersistentModelPolicyTreeError::ArithmeticOverflow)?;
        }
        if next.iter().any(|value| *value != 0) {
            branches.push(OutcomeMass { outcome, mass: ModelStateMass { parts: next } });
        }
    }
    Ok(branches)
}

fn scale_power(exponent: u8) -> Result<u128, PersistentModelPolicyTreeError> {
    let mut value = 1_u128;
    for _ in 0..exponent {
        value = value.checked_mul(SCALE).ok_or(PersistentModelPolicyTreeError::ArithmeticOverflow)?;
    }
    Ok(value)
}

fn stopped_plan(
    problem: &PersistentModelDecisionProblem,
    planner: PlannerKind,
) -> Result<PolicyTreePlan, PersistentModelPolicyTreeError> {
    let (base_loss, terminal_decision) = terminal_assessment(initial_mass(problem), problem)?;
    let numerator = base_loss.checked_mul(scale_power(2)?)
        .ok_or(PersistentModelPolicyTreeError::ArithmeticOverflow)?;
    Ok(PolicyTreePlan {
        planner,
        first_action_id: None,
        expected_terminal_loss_numerator: numerator,
        expected_terminal_loss_denominator: RISK_DENOMINATOR,
        worst_case_resource_use: ResourceVector::default(),
        branches: Vec::new(),
        terminal_decision_if_no_first_action: Some(terminal_decision),
        authority_transition_not_authorized_by_plan: true,
    })
}

fn best_continuation(
    mass: ModelStateMass,
    remaining_actions: &[&PersistentModelAction],
    remaining_budget: ResourceVector,
    problem: &PersistentModelDecisionProblem,
) -> Result<Continuation, PersistentModelPolicyTreeError> {
    let (stop_loss, stop_decision) = terminal_assessment(mass, problem)?;
    let mut best = Continuation {
        action_id: None,
        terminal_decision: Some(stop_decision),
        second_stage_outcomes: Vec::new(),
        resource_use: ResourceVector::default(),
        loss_numerator: stop_loss.checked_mul(SCALE)
            .ok_or(PersistentModelPolicyTreeError::ArithmeticOverflow)?,
    };

    for action in remaining_actions {
        if !action.cost.fits_within(remaining_budget) {
            continue;
        }
        let mut action_loss = 0_u128;
        let mut second_stage_outcomes = Vec::new();
        for branch in expand_action(mass, action)? {
            let (loss, terminal_decision) = terminal_assessment(branch.mass, problem)?;
            action_loss = action_loss.checked_add(loss)
                .ok_or(PersistentModelPolicyTreeError::ArithmeticOverflow)?;
            second_stage_outcomes.push(SecondStageOutcome {
                outcome: branch.outcome,
                posterior_outside_probability: posterior_outside_probability(branch.mass)?,
                posterior_model_one_probability: posterior_model_one_probability(branch.mass)?,
                terminal_decision,
                terminal_loss_numerator: loss,
            });
        }
        if action_loss < best.loss_numerator {
            best = Continuation {
                action_id: Some(action.action_id),
                terminal_decision: None,
                second_stage_outcomes,
                resource_use: action.cost,
                loss_numerator: action_loss,
            };
        }
    }
    Ok(best)
}

fn build_plan_for_first_action(
    problem: &PersistentModelDecisionProblem,
    actions: &[&PersistentModelAction],
    budget: ResourceVector,
    first_action_id: u64,
    planner: PlannerKind,
) -> Result<PolicyTreePlan, PersistentModelPolicyTreeError> {
    let first_action = actions.iter().copied()
        .find(|action| action.action_id == first_action_id)
        .ok_or(PersistentModelPolicyTreeError::ActionNotFound(first_action_id))?;
    let remaining_budget = budget.remaining_after(first_action.cost)
        .ok_or(PersistentModelPolicyTreeError::ArithmeticOverflow)?;
    let remaining_actions: Vec<&PersistentModelAction> = actions.iter().copied()
        .filter(|action| action.action_id != first_action_id).collect();

    let mut branches = Vec::new();
    let mut total_loss = 0_u128;
    let mut worst_case_resource_use = first_action.cost;
    for branch in expand_action(initial_mass(problem), first_action)? {
        let continuation = best_continuation(branch.mass, &remaining_actions, remaining_budget, problem)?;
        let resource_use = first_action.cost.checked_add(continuation.resource_use)?;
        total_loss = total_loss.checked_add(continuation.loss_numerator)
            .ok_or(PersistentModelPolicyTreeError::ArithmeticOverflow)?;
        worst_case_resource_use = worst_case_resource_use.component_max(resource_use);
        branches.push(FirstActionBranch {
            outcome: branch.outcome,
            posterior_outside_probability: posterior_outside_probability(branch.mass)?,
            posterior_model_one_probability: posterior_model_one_probability(branch.mass)?,
            second_action_id: continuation.action_id,
            terminal_decision: continuation.terminal_decision,
            second_stage_outcomes: continuation.second_stage_outcomes,
            resource_use,
            subtree_loss_numerator: continuation.loss_numerator,
        });
    }

    Ok(PolicyTreePlan {
        planner,
        first_action_id: Some(first_action_id),
        expected_terminal_loss_numerator: total_loss,
        expected_terminal_loss_denominator: RISK_DENOMINATOR,
        worst_case_resource_use,
        branches,
        terminal_decision_if_no_first_action: None,
        authority_transition_not_authorized_by_plan: true,
    })
}

/// Exact horizon-two Bayesian model-mixture policy tree. The model identity
/// is a persistent hidden variable. The same branch action is used regardless
/// of which model generated the observations; model posterior updates are
/// diagnostic and are never exposed as privileged model truth.
pub fn plan_exact_horizon_two(
    problem: &PersistentModelDecisionProblem,
    actions: &[PersistentModelAction],
    budget: ResourceVector,
) -> Result<PolicyTreePlan, PersistentModelPolicyTreeError> {
    let ordered = validate_inputs(problem, actions)?;
    let mut best = stopped_plan(problem, PlannerKind::ExactHorizonTwo)?;
    for action in &ordered {
        if !action.cost.fits_within(budget) {
            continue;
        }
        let candidate = build_plan_for_first_action(
            problem,
            &ordered,
            budget,
            action.action_id,
            PlannerKind::ExactHorizonTwo,
        )?;
        if candidate.expected_terminal_loss_numerator < best.expected_terminal_loss_numerator {
            best = candidate;
        }
    }
    Ok(best)
}

/// Matched greedy-first comparator. It receives the same branch-specific
/// continuation opportunity as the exact horizon-two oracle.
pub fn plan_one_step_greedy(
    problem: &PersistentModelDecisionProblem,
    actions: &[PersistentModelAction],
    budget: ResourceVector,
) -> Result<PolicyTreePlan, PersistentModelPolicyTreeError> {
    let ordered = validate_inputs(problem, actions)?;
    let (stop_loss, _) = terminal_assessment(initial_mass(problem), problem)?;
    let mut best_immediate_loss = stop_loss.checked_mul(SCALE)
        .ok_or(PersistentModelPolicyTreeError::ArithmeticOverflow)?;
    let mut best_first_action_id = None;
    for action in &ordered {
        if !action.cost.fits_within(budget) {
            continue;
        }
        let mut action_loss = 0_u128;
        for branch in expand_action(initial_mass(problem), action)? {
            let (loss, _) = terminal_assessment(branch.mass, problem)?;
            action_loss = action_loss.checked_add(loss)
                .ok_or(PersistentModelPolicyTreeError::ArithmeticOverflow)?;
        }
        if action_loss < best_immediate_loss {
            best_immediate_loss = action_loss;
            best_first_action_id = Some(action.action_id);
        }
    }
    match best_first_action_id {
        Some(action_id) => build_plan_for_first_action(problem, &ordered, budget, action_id, PlannerKind::OneStepGreedy),
        None => stopped_plan(problem, PlannerKind::OneStepGreedy),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn likelihoods(no_observation: u64, observed_zero: u64, observed_one: u64) -> OutcomeLikelihoods {
        OutcomeLikelihoods {
            no_observation_parts_per_billion: no_observation,
            observed_zero_parts_per_billion: observed_zero,
            observed_one_parts_per_billion: observed_one,
        }
    }

    fn problem(prior: [u64; 4], false_safe: u64, false_degraded: u64) -> PersistentModelDecisionProblem {
        PersistentModelDecisionProblem {
            joint_prior_parts_per_billion: prior,
            false_safe_loss: false_safe,
            false_degraded_loss: false_degraded,
        }
    }

    fn action(action_id: u64, rows: [OutcomeLikelihoods; 4], cost: ResourceVector) -> PersistentModelAction {
        PersistentModelAction { action_id, cost, likelihoods_by_model_state: rows }
    }

    fn unit_cost() -> ResourceVector {
        ResourceVector { latency_ns: 1, energy_uj: 1, bandwidth_bytes: 1 }
    }

    fn model_probe(action_id: u64, model_zero_one: u64, model_one_one: u64) -> PersistentModelAction {
        let model_zero = likelihoods(0, PROBABILITY_SCALE - model_zero_one, model_zero_one);
        let model_one = likelihoods(0, PROBABILITY_SCALE - model_one_one, model_one_one);
        action(action_id, [model_zero, model_zero, model_one, model_one], unit_cost())
    }

    fn navigation_probe(action_id: u64) -> PersistentModelAction {
        action(
            action_id,
            [
                likelihoods(50_000_000, 850_000_000, 100_000_000),
                likelihoods(50_000_000, 100_000_000, 850_000_000),
                likelihoods(400_000_000, 300_000_000, 300_000_000),
                likelihoods(400_000_000, 300_000_000, 300_000_000),
            ],
            unit_cost(),
        )
    }

    #[test]
    fn an_observation_updates_the_persistent_model_posterior() {
        let prior = ModelStateMass { parts: [250_000_000_u64, 250_000_000, 250_000_000, 250_000_000].map(u128::from) };
        let probe = model_probe(1, 100_000_000, 900_000_000);
        let observed_one = expand_action(prior, &probe).unwrap().into_iter()
            .find(|branch| branch.outcome == ActionOutcome::ObservedOne).unwrap();
        let posterior = posterior_model_one_probability(observed_one.mass).unwrap().unwrap();
        assert_eq!(posterior.numerator * 10, posterior.denominator * 9);
    }

    #[test]
    fn no_observation_can_update_the_persistent_model_posterior() {
        let prior = ModelStateMass { parts: [250_000_000_u64, 250_000_000, 250_000_000, 250_000_000].map(u128::from) };
        let probe = action(
            1,
            [
                likelihoods(900_000_000, 100_000_000, 0),
                likelihoods(900_000_000, 100_000_000, 0),
                likelihoods(100_000_000, 900_000_000, 0),
                likelihoods(100_000_000, 900_000_000, 0),
            ],
            unit_cost(),
        );
        let no_observation = expand_action(prior, &probe).unwrap().into_iter()
            .find(|branch| branch.outcome == ActionOutcome::NoObservation).unwrap();
        let posterior = posterior_model_one_probability(no_observation.mass).unwrap().unwrap();
        assert_eq!(posterior.numerator * 10, posterior.denominator);
    }

    #[test]
    fn persistent_model_identity_induces_marginal_sensor_dependence() {
        let prior = [250_000_000_u64, 250_000_000, 250_000_000, 250_000_000];
        let first = model_probe(1, 100_000_000, 900_000_000);
        let second = model_probe(2, 100_000_000, 900_000_000);
        let mut joint_both_one = 0_u128;
        let mut marginal_one = 0_u128;
        for hidden_state in MODEL_STATES {
            let index = hidden_state.index();
            let state_prior = prior[index] as u128;
            let p_first = first.likelihoods_by_model_state[index].observed_one_parts_per_billion as u128;
            let p_second = second.likelihoods_by_model_state[index].observed_one_parts_per_billion as u128;
            joint_both_one += state_prior * p_first * p_second;
            marginal_one += state_prior * p_first;
        }
        assert_eq!(joint_both_one, 410_000_000_000_000_000_000_000_000);
        assert_eq!(marginal_one, 500_000_000_000_000_000);
        assert!(joint_both_one * SCALE > marginal_one * marginal_one);
    }

    #[test]
    fn exact_policy_is_never_worse_than_greedy_first() {
        let problem = problem([300_000_000, 200_000_000, 300_000_000, 200_000_000], 100, 150);
        let actions = vec![
            navigation_probe(1),
            model_probe(2, 200_000_000, 800_000_000),
            action(
                3,
                [
                    likelihoods(100_000_000, 800_000_000, 100_000_000),
                    likelihoods(100_000_000, 100_000_000, 800_000_000),
                    likelihoods(200_000_000, 400_000_000, 400_000_000),
                    likelihoods(200_000_000, 400_000_000, 400_000_000),
                ],
                unit_cost(),
            ),
        ];
        let budget = ResourceVector { latency_ns: 2, energy_uj: 2, bandwidth_bytes: 2 };
        let exact = plan_exact_horizon_two(&problem, &actions, budget).unwrap();
        let greedy = plan_one_step_greedy(&problem, &actions, budget).unwrap();
        assert!(exact.expected_terminal_loss_numerator <= greedy.expected_terminal_loss_numerator);
        assert!(exact.authority_transition_not_authorized_by_plan);
        assert!(greedy.authority_transition_not_authorized_by_plan);
    }

    #[test]
    fn candidate_order_does_not_change_the_policy_tree() {
        let problem = problem([300_000_000, 200_000_000, 300_000_000, 200_000_000], 100, 150);
        let mut actions = vec![navigation_probe(1), model_probe(2, 200_000_000, 800_000_000), navigation_probe(3)];
        let budget = ResourceVector { latency_ns: 2, energy_uj: 2, bandwidth_bytes: 2 };
        let forward = plan_exact_horizon_two(&problem, &actions, budget).unwrap();
        actions.reverse();
        let reversed = plan_exact_horizon_two(&problem, &actions, budget).unwrap();
        assert_eq!(forward, reversed);
    }

    #[test]
    fn every_path_respects_resources_and_never_reuses_an_action() {
        let problem = problem([300_000_000, 200_000_000, 300_000_000, 200_000_000], 100, 150);
        let actions = vec![navigation_probe(1), model_probe(2, 200_000_000, 800_000_000), navigation_probe(3)];
        let budget = ResourceVector { latency_ns: 2, energy_uj: 2, bandwidth_bytes: 2 };
        let plan = plan_exact_horizon_two(&problem, &actions, budget).unwrap();
        for branch in &plan.branches {
            assert!(branch.resource_use.fits_within(budget));
            if let Some(second_action_id) = branch.second_action_id {
                assert_ne!(Some(second_action_id), plan.first_action_id);
            }
        }
        assert!(plan.worst_case_resource_use.fits_within(budget));
    }

    #[test]
    fn invalid_joint_prior_likelihoods_and_duplicate_ids_reject() {
        let bad_prior = problem([500_000_000, 0, 500_000_000, 1], 100, 100);
        assert_eq!(
            plan_exact_horizon_two(&bad_prior, &[], ResourceVector::default()),
            Err(PersistentModelPolicyTreeError::InvalidJointPrior)
        );
        let valid = problem([250_000_000; 4], 100, 100);
        let invalid = action(
            1,
            [
                likelihoods(0, 0, PROBABILITY_SCALE - 1),
                likelihoods(0, 0, PROBABILITY_SCALE),
                likelihoods(0, 0, PROBABILITY_SCALE),
                likelihoods(0, 0, PROBABILITY_SCALE),
            ],
            unit_cost(),
        );
        assert!(matches!(
            plan_exact_horizon_two(&valid, &[invalid], unit_cost()),
            Err(PersistentModelPolicyTreeError::InvalidLikelihoods {
                action_id: 1,
                hidden_state: ModelState::ModelZeroInside
            })
        ));
        let duplicate = vec![navigation_probe(9), navigation_probe(9)];
        assert_eq!(
            plan_exact_horizon_two(&valid, &duplicate, unit_cost()),
            Err(PersistentModelPolicyTreeError::DuplicateActionId(9))
        );
    }

    #[test]
    fn asymmetric_false_safe_and_false_degraded_losses_are_respected() {
        let prior = [800_000_000, 100_000_000, 100_000_000, 0];
        let safe = problem(prior, 1, 1_000);
        let degraded = problem(prior, 1_000, 1);
        let safe_plan = plan_exact_horizon_two(&safe, &[], ResourceVector::default()).unwrap();
        let degraded_plan = plan_exact_horizon_two(&degraded, &[], ResourceVector::default()).unwrap();
        assert_eq!(safe_plan.terminal_decision_if_no_first_action, Some(TerminalDecision::DeclareSafe));
        assert_eq!(degraded_plan.terminal_decision_if_no_first_action, Some(TerminalDecision::DeclareDegraded));
    }

    #[test]
    fn no_value_or_infeasible_action_produces_explicit_abstention() {
        let problem = problem([250_000_000; 4], 10, 10);
        let neutral = action(1, [likelihoods(100_000_000, 450_000_000, 450_000_000); 4], unit_cost());
        let plan = plan_exact_horizon_two(&problem, &[neutral], ResourceVector::default()).unwrap();
        assert_eq!(plan.first_action_id, None);
        assert!(plan.branches.is_empty());
        assert!(plan.authority_transition_not_authorized_by_plan);
    }

    #[test]
    fn model_identity_is_not_exposed_as_controller_truth() {
        // Both model states remain in the single mixture belief; a branch has
        // one continuation action, not one separate action per hidden model.
        let problem = problem([250_000_000; 4], 100, 100);
        let actions = vec![navigation_probe(1), model_probe(2, 100_000_000, 900_000_000), navigation_probe(3)];
        let plan = plan_exact_horizon_two(
            &problem,
            &actions,
            ResourceVector { latency_ns: 2, energy_uj: 2, bandwidth_bytes: 2 },
        ).unwrap();
        for branch in &plan.branches {
            if let Some(second) = branch.second_action_id {
                assert_ne!(Some(second), plan.first_action_id);
            }
        }
    }
}
