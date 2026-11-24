//! Exact horizon-two active sensing over joint navigation, common-cause,
//! and persistent observation-model latents.
//!
//! Hidden-state order: model 0/1 x nominal/shared-fault x inside/outside.
//! Both model identity and common-cause regime persist through the mission.
//! Observation likelihoods factorize only conditional on this full joint
//! state. This is an offline finite Bayesian-mixture reference, not a minimax
//! guarantee, physical calibration, integrity monitor, or authorization API.

use core::fmt;

pub const PROBABILITY_SCALE: u64 = 1_000_000_000;
pub const MAX_ACTIONS: usize = 16;
pub const MAX_TERMINAL_LOSS: u64 = 1_000_000_000;
const STATE_COUNT: usize = 8;
const SCALE: u128 = PROBABILITY_SCALE as u128;
const RISK_DENOMINATOR: u128 = SCALE * SCALE * SCALE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionOutcome {
    NoObservation,
    ObservedZero,
    ObservedOne,
}

const OUTCOMES: [ActionOutcome; 3] = [
    ActionOutcome::NoObservation,
    ActionOutcome::ObservedZero,
    ActionOutcome::ObservedOne,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutcomeLikelihoods {
    pub no_observation_ppb: u64,
    pub observed_zero_ppb: u64,
    pub observed_one_ppb: u64,
}

impl OutcomeLikelihoods {
    fn probability(self, outcome: ActionOutcome) -> u64 {
        match outcome {
            ActionOutcome::NoObservation => self.no_observation_ppb,
            ActionOutcome::ObservedZero => self.observed_zero_ppb,
            ActionOutcome::ObservedOne => self.observed_one_ppb,
        }
    }

    fn total(self) -> Option<u64> {
        self.no_observation_ppb
            .checked_add(self.observed_zero_ppb)?
            .checked_add(self.observed_one_ppb)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResourceVector {
    pub latency_ns: u64,
    pub energy_uj: u64,
    pub bandwidth_bytes: u64,
}

impl ResourceVector {
    fn fits(self, budget: Self) -> bool {
        self.latency_ns <= budget.latency_ns
            && self.energy_uj <= budget.energy_uj
            && self.bandwidth_bytes <= budget.bandwidth_bytes
    }

    fn add(self, other: Self) -> Result<Self, JointLatentPolicyError> {
        Ok(Self {
            latency_ns: self.latency_ns.checked_add(other.latency_ns)
                .ok_or(JointLatentPolicyError::ArithmeticOverflow)?,
            energy_uj: self.energy_uj.checked_add(other.energy_uj)
                .ok_or(JointLatentPolicyError::ArithmeticOverflow)?,
            bandwidth_bytes: self.bandwidth_bytes.checked_add(other.bandwidth_bytes)
                .ok_or(JointLatentPolicyError::ArithmeticOverflow)?,
        })
    }

    fn remaining(self, spent: Self) -> Option<Self> {
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
pub struct JointLatentAction {
    pub action_id: u64,
    pub cost: ResourceVector,
    /// State order:
    /// [M0/Nominal/Inside, M0/Nominal/Outside,
    ///  M0/Fault/Inside, M0/Fault/Outside,
    ///  M1/Nominal/Inside, M1/Nominal/Outside,
    ///  M1/Fault/Inside, M1/Fault/Outside].
    pub likelihoods: [OutcomeLikelihoods; STATE_COUNT],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JointLatentProblem {
    /// PPB prior in the exact state order documented on JointLatentAction.
    /// It encodes the model, common-cause, and navigation-state priors.
    pub prior_ppb: [u64; STATE_COUNT],
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
    pub posterior_outside: ExactProbability,
    pub posterior_model_one: ExactProbability,
    pub posterior_shared_fault: ExactProbability,
    pub terminal_decision: TerminalDecision,
    pub terminal_loss_numerator: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FirstActionBranch {
    pub outcome: ActionOutcome,
    pub posterior_outside: ExactProbability,
    pub posterior_model_one: ExactProbability,
    pub posterior_shared_fault: ExactProbability,
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
    pub authority_transition_not_authorized_by_plan: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JointLatentPolicyError {
    InvalidPrior,
    InvalidDecisionModel,
    TooManyActions,
    InvalidActionId,
    DuplicateActionId(u64),
    InvalidLikelihoods { action_id: u64, state_index: usize },
    ArithmeticOverflow,
}

impl fmt::Display for JointLatentPolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPrior => write!(f, "joint model/fault/navigation prior must sum exactly to one billion"),
            Self::InvalidDecisionModel => write!(f, "terminal decision loss is outside the valid range"),
            Self::TooManyActions => write!(f, "action count exceeds the finite oracle limit"),
            Self::InvalidActionId => write!(f, "action identity must be nonzero"),
            Self::DuplicateActionId(id) => write!(f, "action identity is duplicated: {id}"),
            Self::InvalidLikelihoods { action_id, state_index } => write!(f, "action {action_id} likelihoods for hidden state {state_index} must sum to one billion"),
            Self::ArithmeticOverflow => write!(f, "checked joint-state arithmetic overflowed"),
        }
    }
}

impl std::error::Error for JointLatentPolicyError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StateMass {
    parts: [u128; STATE_COUNT],
}

impl StateMass {
    fn total(self) -> Result<u128, JointLatentPolicyError> {
        self.parts.iter().try_fold(0_u128, |sum, x| {
            sum.checked_add(*x).ok_or(JointLatentPolicyError::ArithmeticOverflow)
        })
    }

    fn sum_indices(self, indices: &[usize]) -> Result<u128, JointLatentPolicyError> {
        indices.iter().try_fold(0_u128, |sum, index| {
            sum.checked_add(self.parts[*index]).ok_or(JointLatentPolicyError::ArithmeticOverflow)
        })
    }

    fn outside(self) -> Result<u128, JointLatentPolicyError> {
        self.sum_indices(&[1, 3, 5, 7])
    }

    fn inside(self) -> Result<u128, JointLatentPolicyError> {
        self.sum_indices(&[0, 2, 4, 6])
    }

    fn model_one(self) -> Result<u128, JointLatentPolicyError> {
        self.sum_indices(&[4, 5, 6, 7])
    }

    fn shared_fault(self) -> Result<u128, JointLatentPolicyError> {
        self.sum_indices(&[2, 3, 6, 7])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OutcomeMass {
    outcome: ActionOutcome,
    mass: StateMass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Continuation {
    action_id: Option<u64>,
    terminal_decision: Option<TerminalDecision>,
    outcomes: Vec<SecondStageOutcome>,
    resource_use: ResourceVector,
    loss_numerator: u128,
}

fn exact_probability(numerator: u128, denominator: u128) -> Result<ExactProbability, JointLatentPolicyError> {
    if denominator == 0 {
        return Err(JointLatentPolicyError::ArithmeticOverflow);
    }
    Ok(ExactProbability { numerator, denominator })
}

fn posterior_triplet(mass: StateMass) -> Result<(ExactProbability, ExactProbability, ExactProbability), JointLatentPolicyError> {
    let total = mass.total()?;
    Ok((
        exact_probability(mass.outside()?, total)?,
        exact_probability(mass.model_one()?, total)?,
        exact_probability(mass.shared_fault()?, total)?,
    ))
}

fn validate<'a>(
    problem: &JointLatentProblem,
    actions: &'a [JointLatentAction],
) -> Result<Vec<&'a JointLatentAction>, JointLatentPolicyError> {
    let prior_total = problem.prior_ppb.iter().try_fold(0_u64, |sum, x| sum.checked_add(*x));
    if prior_total != Some(PROBABILITY_SCALE) {
        return Err(JointLatentPolicyError::InvalidPrior);
    }
    if problem.false_safe_loss == 0
        || problem.false_degraded_loss == 0
        || problem.false_safe_loss > MAX_TERMINAL_LOSS
        || problem.false_degraded_loss > MAX_TERMINAL_LOSS
    {
        return Err(JointLatentPolicyError::InvalidDecisionModel);
    }
    if actions.len() > MAX_ACTIONS {
        return Err(JointLatentPolicyError::TooManyActions);
    }

    let mut ordered: Vec<&JointLatentAction> = actions.iter().collect();
    ordered.sort_by_key(|action| action.action_id);
    for (i, action) in ordered.iter().enumerate() {
        if action.action_id == 0 {
            return Err(JointLatentPolicyError::InvalidActionId);
        }
        if i > 0 && ordered[i - 1].action_id == action.action_id {
            return Err(JointLatentPolicyError::DuplicateActionId(action.action_id));
        }
        for (state_index, likelihoods) in action.likelihoods.iter().enumerate() {
            if likelihoods.total() != Some(PROBABILITY_SCALE)
                || [
                    likelihoods.no_observation_ppb,
                    likelihoods.observed_zero_ppb,
                    likelihoods.observed_one_ppb,
                ].iter().any(|p| *p > PROBABILITY_SCALE)
            {
                return Err(JointLatentPolicyError::InvalidLikelihoods {
                    action_id: action.action_id,
                    state_index,
                });
            }
        }
    }
    Ok(ordered)
}

fn initial_mass(problem: &JointLatentProblem) -> StateMass {
    StateMass { parts: problem.prior_ppb.map(u128::from) }
}

fn terminal_assessment(
    mass: StateMass,
    problem: &JointLatentProblem,
) -> Result<(u128, TerminalDecision), JointLatentPolicyError> {
    let safe_loss = mass.outside()?.checked_mul(problem.false_safe_loss as u128)
        .ok_or(JointLatentPolicyError::ArithmeticOverflow)?;
    let degraded_loss = mass.inside()?.checked_mul(problem.false_degraded_loss as u128)
        .ok_or(JointLatentPolicyError::ArithmeticOverflow)?;
    if safe_loss < degraded_loss {
        Ok((safe_loss, TerminalDecision::DeclareSafe))
    } else {
        Ok((degraded_loss, TerminalDecision::DeclareDegraded))
    }
}

fn expand(mass: StateMass, action: &JointLatentAction) -> Result<Vec<OutcomeMass>, JointLatentPolicyError> {
    let mut branches = Vec::with_capacity(OUTCOMES.len());
    for outcome in OUTCOMES {
        let mut next = [0_u128; STATE_COUNT];
        for (i, state_mass) in mass.parts.iter().enumerate() {
            next[i] = state_mass.checked_mul(action.likelihoods[i].probability(outcome) as u128)
                .ok_or(JointLatentPolicyError::ArithmeticOverflow)?;
        }
        if next.iter().any(|x| *x != 0) {
            branches.push(OutcomeMass { outcome, mass: StateMass { parts: next } });
        }
    }
    Ok(branches)
}

fn scale_pow(n: u8) -> Result<u128, JointLatentPolicyError> {
    let mut value = 1_u128;
    for _ in 0..n {
        value = value.checked_mul(SCALE).ok_or(JointLatentPolicyError::ArithmeticOverflow)?;
    }
    Ok(value)
}

fn stop_plan(problem: &JointLatentProblem, planner: PlannerKind) -> Result<PolicyTreePlan, JointLatentPolicyError> {
    let (loss, decision) = terminal_assessment(initial_mass(problem), problem)?;
    Ok(PolicyTreePlan {
        planner,
        first_action_id: None,
        expected_terminal_loss_numerator: loss.checked_mul(scale_pow(2)?)
            .ok_or(JointLatentPolicyError::ArithmeticOverflow)?,
        expected_terminal_loss_denominator: RISK_DENOMINATOR,
        worst_case_resource_use: ResourceVector::default(),
        branches: Vec::new(),
        terminal_decision_if_no_first_action: Some(decision),
        authority_transition_not_authorized_by_plan: true,
    })
}

fn best_continuation(
    mass: StateMass,
    remaining: &[&JointLatentAction],
    budget: ResourceVector,
    problem: &JointLatentProblem,
) -> Result<Continuation, JointLatentPolicyError> {
    let (stop_loss, stop_decision) = terminal_assessment(mass, problem)?;
    let mut best = Continuation {
        action_id: None,
        terminal_decision: Some(stop_decision),
        outcomes: Vec::new(),
        resource_use: ResourceVector::default(),
        loss_numerator: stop_loss.checked_mul(SCALE)
            .ok_or(JointLatentPolicyError::ArithmeticOverflow)?,
    };

    for action in remaining {
        if !action.cost.fits(budget) {
            continue;
        }
        let mut action_loss = 0_u128;
        let mut outcomes = Vec::new();
        for branch in expand(mass, action)? {
            let (loss, decision) = terminal_assessment(branch.mass, problem)?;
            let (outside, model_one, shared_fault) = posterior_triplet(branch.mass)?;
            action_loss = action_loss.checked_add(loss)
                .ok_or(JointLatentPolicyError::ArithmeticOverflow)?;
            outcomes.push(SecondStageOutcome {
                outcome: branch.outcome,
                posterior_outside: outside,
                posterior_model_one: model_one,
                posterior_shared_fault: shared_fault,
                terminal_decision: decision,
                terminal_loss_numerator: loss,
            });
        }
        if action_loss < best.loss_numerator {
            best = Continuation {
                action_id: Some(action.action_id),
                terminal_decision: None,
                outcomes,
                resource_use: action.cost,
                loss_numerator: action_loss,
            };
        }
    }
    Ok(best)
}

fn build_for_first(
    problem: &JointLatentProblem,
    actions: &[&JointLatentAction],
    budget: ResourceVector,
    first_id: u64,
    planner: PlannerKind,
) -> Result<PolicyTreePlan, JointLatentPolicyError> {
    let first = actions.iter().copied().find(|a| a.action_id == first_id)
        .ok_or(JointLatentPolicyError::ArithmeticOverflow)?;
    let remaining_budget = budget.remaining(first.cost).ok_or(JointLatentPolicyError::ArithmeticOverflow)?;
    let remaining: Vec<&JointLatentAction> = actions.iter().copied()
        .filter(|a| a.action_id != first_id).collect();

    let mut branches = Vec::new();
    let mut total_loss = 0_u128;
    let mut worst_resources = first.cost;
    for branch in expand(initial_mass(problem), first)? {
        let continuation = best_continuation(branch.mass, &remaining, remaining_budget, problem)?;
        let resources = first.cost.add(continuation.resource_use)?;
        let (outside, model_one, shared_fault) = posterior_triplet(branch.mass)?;
        total_loss = total_loss.checked_add(continuation.loss_numerator)
            .ok_or(JointLatentPolicyError::ArithmeticOverflow)?;
        worst_resources = worst_resources.component_max(resources);
        branches.push(FirstActionBranch {
            outcome: branch.outcome,
            posterior_outside: outside,
            posterior_model_one: model_one,
            posterior_shared_fault: shared_fault,
            second_action_id: continuation.action_id,
            terminal_decision: continuation.terminal_decision,
            second_stage_outcomes: continuation.outcomes,
            resource_use: resources,
            subtree_loss_numerator: continuation.loss_numerator,
        });
    }

    Ok(PolicyTreePlan {
        planner,
        first_action_id: Some(first_id),
        expected_terminal_loss_numerator: total_loss,
        expected_terminal_loss_denominator: RISK_DENOMINATOR,
        worst_case_resource_use: worst_resources,
        branches,
        terminal_decision_if_no_first_action: None,
        authority_transition_not_authorized_by_plan: true,
    })
}

pub fn plan_exact_horizon_two(
    problem: &JointLatentProblem,
    actions: &[JointLatentAction],
    budget: ResourceVector,
) -> Result<PolicyTreePlan, JointLatentPolicyError> {
    let ordered = validate(problem, actions)?;
    let mut best = stop_plan(problem, PlannerKind::ExactHorizonTwo)?;
    for action in &ordered {
        if action.cost.fits(budget) {
            let candidate = build_for_first(problem, &ordered, budget, action.action_id, PlannerKind::ExactHorizonTwo)?;
            if candidate.expected_terminal_loss_numerator < best.expected_terminal_loss_numerator {
                best = candidate;
            }
        }
    }
    Ok(best)
}

pub fn plan_one_step_greedy(
    problem: &JointLatentProblem,
    actions: &[JointLatentAction],
    budget: ResourceVector,
) -> Result<PolicyTreePlan, JointLatentPolicyError> {
    let ordered = validate(problem, actions)?;
    let (stop_loss, _) = terminal_assessment(initial_mass(problem), problem)?;
    let mut best_loss = stop_loss.checked_mul(SCALE).ok_or(JointLatentPolicyError::ArithmeticOverflow)?;
    let mut first_id = None;
    for action in &ordered {
        if !action.cost.fits(budget) {
            continue;
        }
        let mut action_loss = 0_u128;
        for branch in expand(initial_mass(problem), action)? {
            let (loss, _) = terminal_assessment(branch.mass, problem)?;
            action_loss = action_loss.checked_add(loss).ok_or(JointLatentPolicyError::ArithmeticOverflow)?;
        }
        if action_loss < best_loss {
            best_loss = action_loss;
            first_id = Some(action.action_id);
        }
    }
    match first_id {
        Some(id) => build_for_first(problem, &ordered, budget, id, PlannerKind::OneStepGreedy),
        None => stop_plan(problem, PlannerKind::OneStepGreedy),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(no: u64, zero: u64, one: u64) -> OutcomeLikelihoods {
        OutcomeLikelihoods {
            no_observation_ppb: no,
            observed_zero_ppb: zero,
            observed_one_ppb: one,
        }
    }

    fn problem(prior: [u64; 8], false_safe: u64, false_degraded: u64) -> JointLatentProblem {
        JointLatentProblem {
            prior_ppb: prior,
            false_safe_loss: false_safe,
            false_degraded_loss: false_degraded,
        }
    }

    fn action(id: u64, rows: [OutcomeLikelihoods; 8]) -> JointLatentAction {
        JointLatentAction {
            action_id: id,
            cost: ResourceVector { latency_ns: 1, energy_uj: 1, bandwidth_bytes: 1 },
            likelihoods: rows,
        }
    }

    fn model_probe(id: u64) -> JointLatentAction {
        let m0 = l(0, 900_000_000, 100_000_000);
        let m1 = l(0, 100_000_000, 900_000_000);
        action(id, [m0, m0, m0, m0, m1, m1, m1, m1])
    }

    fn fault_probe(id: u64) -> JointLatentAction {
        let nominal = l(0, 900_000_000, 100_000_000);
        let fault = l(0, 100_000_000, 900_000_000);
        action(id, [nominal, nominal, fault, fault, nominal, nominal, fault, fault])
    }

    fn navigation_probe(id: u64) -> JointLatentAction {
        action(id, [
            l(50_000_000, 850_000_000, 100_000_000),
            l(50_000_000, 100_000_000, 850_000_000),
            l(400_000_000, 300_000_000, 300_000_000),
            l(400_000_000, 300_000_000, 300_000_000),
            l(50_000_000, 850_000_000, 100_000_000),
            l(50_000_000, 100_000_000, 850_000_000),
            l(400_000_000, 300_000_000, 300_000_000),
            l(400_000_000, 300_000_000, 300_000_000),
        ])
    }

    #[test]
    fn observations_update_model_and_common_cause_posteriors() {
        let prior = [125_000_000_u64, 0, 125_000_000, 0, 125_000_000, 0, 125_000_000, 0];
        let mass = StateMass { parts: prior.map(u128::from) };

        let observed_model = expand(mass, &model_probe(1)).unwrap().into_iter()
            .find(|x| x.outcome == ActionOutcome::ObservedOne).unwrap();
        let model_one = observed_model.mass.model_one().unwrap();
        assert_eq!(model_one * 10, observed_model.mass.total().unwrap() * 9);

        let observed_fault = expand(mass, &fault_probe(2)).unwrap().into_iter()
            .find(|x| x.outcome == ActionOutcome::ObservedOne).unwrap();
        let fault = observed_fault.mass.shared_fault().unwrap();
        assert_eq!(fault * 10, observed_fault.mass.total().unwrap() * 9);
    }

    #[test]
    fn no_observation_updates_both_persistent_latents() {
        let prior = [125_000_000_u64, 0, 125_000_000, 0, 125_000_000, 0, 125_000_000, 0];
        let dropout = action(1, [
            l(900_000_000, 100_000_000, 0), l(900_000_000, 100_000_000, 0),
            l(500_000_000, 500_000_000, 0), l(500_000_000, 500_000_000, 0),
            l(200_000_000, 800_000_000, 0), l(200_000_000, 800_000_000, 0),
            l(100_000_000, 900_000_000, 0), l(100_000_000, 900_000_000, 0),
        ]);
        let no_obs = expand(StateMass { parts: prior.map(u128::from) }, &dropout).unwrap()
            .into_iter().find(|x| x.outcome == ActionOutcome::NoObservation).unwrap();
        let total = no_obs.mass.total().unwrap();
        let model_one = no_obs.mass.model_one().unwrap();
        let fault = no_obs.mass.shared_fault().unwrap();
        assert!(model_one * 2 < total);
        assert!(fault * 2 < total);
    }

    #[test]
    fn persistent_model_marginalizes_to_dependent_sensor_outcomes() {
        let prior = [500_000_000_u64, 0, 0, 0, 500_000_000, 0, 0, 0];
        let first = model_probe(1);
        let second = model_probe(2);
        let mut both_one = 0_u128;
        let mut one = 0_u128;
        for i in 0..STATE_COUNT {
            let p = prior[i] as u128;
            let p1 = first.likelihoods[i].observed_one_ppb as u128;
            let p2 = second.likelihoods[i].observed_one_ppb as u128;
            both_one += p * p1 * p2;
            one += p * p1;
        }
        assert_eq!(both_one, 410_000_000_000_000_000_000_000_000);
        assert_eq!(one, 500_000_000_000_000_000);
        assert!(both_one * SCALE > one * one);
    }

    #[test]
    fn exact_policy_is_no_worse_than_greedy_first() {
        let problem = problem([125_000_000_u64; 8], 100, 150);
        let actions = vec![navigation_probe(1), fault_probe(2), model_probe(3)];
        let budget = ResourceVector { latency_ns: 2, energy_uj: 2, bandwidth_bytes: 2 };
        let exact = plan_exact_horizon_two(&problem, &actions, budget).unwrap();
        let greedy = plan_one_step_greedy(&problem, &actions, budget).unwrap();
        assert!(exact.expected_terminal_loss_numerator <= greedy.expected_terminal_loss_numerator);
        assert!(exact.authority_transition_not_authorized_by_plan);
    }

    #[test]
    fn candidate_order_does_not_change_the_policy() {
        let problem = problem([125_000_000_u64; 8], 100, 150);
        let mut actions = vec![navigation_probe(1), fault_probe(2), model_probe(3)];
        let budget = ResourceVector { latency_ns: 2, energy_uj: 2, bandwidth_bytes: 2 };
        let a = plan_exact_horizon_two(&problem, &actions, budget).unwrap();
        actions.reverse();
        let b = plan_exact_horizon_two(&problem, &actions, budget).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn all_paths_respect_resource_limits_and_do_not_reuse_actions() {
        let problem = problem([125_000_000_u64; 8], 100, 150);
        let actions = vec![navigation_probe(1), fault_probe(2), model_probe(3)];
        let budget = ResourceVector { latency_ns: 2, energy_uj: 2, bandwidth_bytes: 2 };
        let plan = plan_exact_horizon_two(&problem, &actions, budget).unwrap();
        for branch in &plan.branches {
            assert!(branch.resource_use.fits(budget));
            if let Some(second) = branch.second_action_id {
                assert_ne!(Some(second), plan.first_action_id);
            }
        }
        assert!(plan.worst_case_resource_use.fits(budget));
    }

    #[test]
    fn malformed_prior_likelihoods_and_duplicate_ids_reject() {
        let bad = problem([125_000_001_u64, 125_000_000, 125_000_000, 125_000_000, 125_000_000, 125_000_000, 125_000_000, 125_000_000], 100, 100);
        assert_eq!(plan_exact_horizon_two(&bad, &[], ResourceVector::default()), Err(JointLatentPolicyError::InvalidPrior));

        let valid = problem([125_000_000_u64; 8], 100, 100);
        let mut rows = [l(0, 0, PROBABILITY_SCALE); 8];
        rows[0] = l(0, 0, PROBABILITY_SCALE - 1);
        assert!(matches!(plan_exact_horizon_two(&valid, &[action(1, rows)], ResourceVector { latency_ns: 1, energy_uj: 1, bandwidth_bytes: 1 }),
            Err(JointLatentPolicyError::InvalidLikelihoods { action_id: 1, state_index: 0 })));
        assert_eq!(plan_exact_horizon_two(&valid, &[navigation_probe(9), navigation_probe(9)], ResourceVector::default()),
            Err(JointLatentPolicyError::DuplicateActionId(9)));
    }

    #[test]
    fn asymmetric_false_safe_and_false_degraded_losses_are_respected() {
        let prior = [400_000_000_u64, 100_000_000, 0, 0, 400_000_000, 100_000_000, 0, 0];
        let safe = problem(prior, 1, 1_000);
        let degraded = problem(prior, 1_000, 1);
        assert_eq!(plan_exact_horizon_two(&safe, &[], ResourceVector::default()).unwrap().terminal_decision_if_no_first_action, Some(TerminalDecision::DeclareSafe));
        assert_eq!(plan_exact_horizon_two(&degraded, &[], ResourceVector::default()).unwrap().terminal_decision_if_no_first_action, Some(TerminalDecision::DeclareDegraded));
    }

    #[test]
    fn no_feasible_or_positive_value_action_abstains() {
        let problem = problem([125_000_000_u64; 8], 10, 10);
        let neutral = action(1, [l(100_000_000, 450_000_000, 450_000_000); 8]);
        let plan = plan_exact_horizon_two(&problem, &[neutral], ResourceVector::default()).unwrap();
        assert_eq!(plan.first_action_id, None);
        assert!(plan.branches.is_empty());
        assert!(plan.authority_transition_not_authorized_by_plan);
    }

    #[test]
    fn one_observable_history_has_one_policy_action_not_one_per_hidden_state() {
        let problem = problem([125_000_000_u64; 8], 100, 100);
        let actions = vec![navigation_probe(1), fault_probe(2), model_probe(3)];
        let plan = plan_exact_horizon_two(&problem, &actions, ResourceVector { latency_ns: 2, energy_uj: 2, bandwidth_bytes: 2 }).unwrap();
        for branch in &plan.branches {
            if let Some(second) = branch.second_action_id {
                assert_ne!(Some(second), plan.first_action_id);
            }
        }
    }
}
