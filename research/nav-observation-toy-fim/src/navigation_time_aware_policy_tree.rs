//! Exact horizon-two active sensing with state evolution between observation
//! capture and evidence use.
//!
//! Each action updates the historical capture-epoch belief using its outcome
//! likelihood, then propagates that belief through an action-specific Markov
//! transition kernel to the epoch at which the result is usable. This makes
//! observation age decision-relevant. It is an offline finite reference model;
//! transition kernels are supplied inputs, not physically calibrated facts.

use core::fmt;

pub const PROBABILITY_SCALE: u64 = 1_000_000;
pub const MAX_ACTIONS: usize = 16;
pub const MAX_TERMINAL_LOSS: u64 = 1_000_000;

const SCALE: u128 = PROBABILITY_SCALE as u128;
const RISK_DENOMINATOR: u128 = SCALE * SCALE * SCALE * SCALE * SCALE;

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
    pub no_observation_ppm: u64,
    pub observed_zero_ppm: u64,
    pub observed_one_ppm: u64,
}

impl OutcomeLikelihoods {
    fn probability(self, outcome: ActionOutcome) -> u64 {
        match outcome {
            ActionOutcome::NoObservation => self.no_observation_ppm,
            ActionOutcome::ObservedZero => self.observed_zero_ppm,
            ActionOutcome::ObservedOne => self.observed_one_ppm,
        }
    }

    fn total(self) -> Option<u64> {
        self.no_observation_ppm
            .checked_add(self.observed_zero_ppm)?
            .checked_add(self.observed_one_ppm)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateTransition {
    /// P(outside at use epoch | inside at capture epoch), in PPM.
    pub inside_to_outside_ppm: u64,
    /// P(inside at use epoch | outside at capture epoch), in PPM.
    pub outside_to_inside_ppm: u64,
}

impl StateTransition {
    fn is_valid(self) -> bool {
        self.inside_to_outside_ppm <= PROBABILITY_SCALE
            && self.outside_to_inside_ppm <= PROBABILITY_SCALE
    }

    fn is_identity(self) -> bool {
        self.inside_to_outside_ppm == 0 && self.outside_to_inside_ppm == 0
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

    fn checked_add(self, other: Self) -> Result<Self, TimeAwarePolicyError> {
        Ok(Self {
            latency_ns: self.latency_ns.checked_add(other.latency_ns)
                .ok_or(TimeAwarePolicyError::ArithmeticOverflow)?,
            energy_uj: self.energy_uj.checked_add(other.energy_uj)
                .ok_or(TimeAwarePolicyError::ArithmeticOverflow)?,
            bandwidth_bytes: self.bandwidth_bytes.checked_add(other.bandwidth_bytes)
                .ok_or(TimeAwarePolicyError::ArithmeticOverflow)?,
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
pub struct TimedSensorAction {
    pub action_id: u64,
    pub cost: ResourceVector,
    /// Age from physical measurement capture to the epoch when the result is
    /// usable by the controller. It cannot exceed the action latency budget.
    pub evidence_age_ns: u64,
    /// Calibrated/supplied state evolution over exactly evidence_age_ns.
    pub transition: StateTransition,
    /// Model and transition digests bind the supplied kernels to their source.
    /// The toy kernel checks presence, not cryptographic authenticity.
    pub outcome_model_sha256: [u8; 32],
    pub transition_model_sha256: [u8; 32],
    /// Likelihoods at the physical capture epoch.
    pub inside_likelihoods: OutcomeLikelihoods,
    pub outside_likelihoods: OutcomeLikelihoods,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeAwareProblem {
    pub prior_outside_ppm: u64,
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
    pub posterior_outside_at_capture: ExactProbability,
    pub posterior_outside_at_use: ExactProbability,
    pub terminal_decision: TerminalDecision,
    pub terminal_loss_numerator: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FirstActionBranch {
    pub outcome: ActionOutcome,
    pub posterior_outside_at_capture: ExactProbability,
    pub posterior_outside_at_use: ExactProbability,
    pub second_action_id: Option<u64>,
    pub terminal_decision: Option<TerminalDecision>,
    pub second_stage_outcomes: Vec<SecondStageOutcome>,
    pub resource_use: ResourceVector,
    pub subtree_loss_numerator: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeAwarePolicyPlan {
    pub planner: PlannerKind,
    pub first_action_id: Option<u64>,
    pub expected_terminal_loss_numerator: u128,
    pub expected_terminal_loss_denominator: u128,
    pub worst_case_resource_use: ResourceVector,
    pub branches: Vec<FirstActionBranch>,
    pub terminal_decision_if_no_action: Option<TerminalDecision>,
    pub authority_transition_not_authorized_by_plan: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimeAwarePolicyError {
    InvalidProblem,
    TooManyActions,
    InvalidActionId,
    DuplicateActionId(u64),
    InvalidLikelihoods { action_id: u64, state: &'static str },
    InvalidTransition(u64),
    InvalidEvidenceAge(u64),
    ZeroAgeNonIdentityTransition(u64),
    MissingModelCommitment(u64),
    ArithmeticOverflow,
}

impl fmt::Display for TimeAwarePolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProblem => write!(f, "prior or terminal loss is outside the valid range"),
            Self::TooManyActions => write!(f, "time-aware finite oracle supports at most sixteen actions"),
            Self::InvalidActionId => write!(f, "action identity must be nonzero"),
            Self::DuplicateActionId(id) => write!(f, "action identity is duplicated: {id}"),
            Self::InvalidLikelihoods { action_id, state } => write!(f, "action {action_id} likelihoods for {state} must sum exactly to one million"),
            Self::InvalidTransition(id) => write!(f, "action {id} state-transition probabilities are invalid"),
            Self::InvalidEvidenceAge(id) => write!(f, "action {id} evidence age exceeds its declared latency"),
            Self::ZeroAgeNonIdentityTransition(id) => write!(f, "action {id} has zero evidence age but a non-identity transition"),
            Self::MissingModelCommitment(id) => write!(f, "action {id} requires nonzero outcome and transition model commitments"),
            Self::ArithmeticOverflow => write!(f, "checked time-aware policy probability or loss arithmetic overflowed"),
        }
    }
}

impl std::error::Error for TimeAwarePolicyError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StateMass {
    inside: u128,
    outside: u128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OutcomeMass {
    outcome: ActionOutcome,
    capture_mass: StateMass,
    use_mass: StateMass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Continuation {
    action_id: Option<u64>,
    terminal_decision: Option<TerminalDecision>,
    outcomes: Vec<SecondStageOutcome>,
    resource_use: ResourceVector,
    loss_numerator: u128,
}

fn exact_probability(numerator: u128, denominator: u128) -> Result<ExactProbability, TimeAwarePolicyError> {
    if denominator == 0 {
        return Err(TimeAwarePolicyError::ArithmeticOverflow);
    }
    Ok(ExactProbability { numerator, denominator })
}

fn total_mass(mass: StateMass) -> Result<u128, TimeAwarePolicyError> {
    mass.inside.checked_add(mass.outside).ok_or(TimeAwarePolicyError::ArithmeticOverflow)
}

fn posterior_outside(mass: StateMass) -> Result<ExactProbability, TimeAwarePolicyError> {
    exact_probability(mass.outside, total_mass(mass)?)
}

fn terminal_assessment(
    mass: StateMass,
    problem: &TimeAwareProblem,
) -> Result<(u128, TerminalDecision), TimeAwarePolicyError> {
    let safe_loss = mass.outside.checked_mul(problem.false_safe_loss as u128)
        .ok_or(TimeAwarePolicyError::ArithmeticOverflow)?;
    let degraded_loss = mass.inside.checked_mul(problem.false_degraded_loss as u128)
        .ok_or(TimeAwarePolicyError::ArithmeticOverflow)?;
    if safe_loss < degraded_loss {
        Ok((safe_loss, TerminalDecision::DeclareSafe))
    } else {
        Ok((degraded_loss, TerminalDecision::DeclareDegraded))
    }
}

fn validate<'a>(
    problem: &TimeAwareProblem,
    actions: &'a [TimedSensorAction],
) -> Result<Vec<&'a TimedSensorAction>, TimeAwarePolicyError> {
    if problem.prior_outside_ppm > PROBABILITY_SCALE
        || problem.false_safe_loss == 0
        || problem.false_degraded_loss == 0
        || problem.false_safe_loss > MAX_TERMINAL_LOSS
        || problem.false_degraded_loss > MAX_TERMINAL_LOSS
    {
        return Err(TimeAwarePolicyError::InvalidProblem);
    }
    if actions.len() > MAX_ACTIONS {
        return Err(TimeAwarePolicyError::TooManyActions);
    }

    let mut ordered: Vec<&TimedSensorAction> = actions.iter().collect();
    ordered.sort_by_key(|action| action.action_id);
    for (index, action) in ordered.iter().enumerate() {
        if action.action_id == 0 {
            return Err(TimeAwarePolicyError::InvalidActionId);
        }
        if index > 0 && ordered[index - 1].action_id == action.action_id {
            return Err(TimeAwarePolicyError::DuplicateActionId(action.action_id));
        }
        if action.inside_likelihoods.total() != Some(PROBABILITY_SCALE) {
            return Err(TimeAwarePolicyError::InvalidLikelihoods {
                action_id: action.action_id,
                state: "inside",
            });
        }
        if action.outside_likelihoods.total() != Some(PROBABILITY_SCALE) {
            return Err(TimeAwarePolicyError::InvalidLikelihoods {
                action_id: action.action_id,
                state: "outside",
            });
        }
        if [
            action.inside_likelihoods.no_observation_ppm,
            action.inside_likelihoods.observed_zero_ppm,
            action.inside_likelihoods.observed_one_ppm,
            action.outside_likelihoods.no_observation_ppm,
            action.outside_likelihoods.observed_zero_ppm,
            action.outside_likelihoods.observed_one_ppm,
        ].iter().any(|value| *value > PROBABILITY_SCALE) {
            return Err(TimeAwarePolicyError::InvalidLikelihoods {
                action_id: action.action_id,
                state: "all",
            });
        }
        if !action.transition.is_valid() {
            return Err(TimeAwarePolicyError::InvalidTransition(action.action_id));
        }
        if action.evidence_age_ns > action.cost.latency_ns {
            return Err(TimeAwarePolicyError::InvalidEvidenceAge(action.action_id));
        }
        if action.evidence_age_ns == 0 && !action.transition.is_identity() {
            return Err(TimeAwarePolicyError::ZeroAgeNonIdentityTransition(action.action_id));
        }
        if action.outcome_model_sha256.iter().all(|byte| *byte == 0)
            || action.transition_model_sha256.iter().all(|byte| *byte == 0)
        {
            return Err(TimeAwarePolicyError::MissingModelCommitment(action.action_id));
        }
    }
    Ok(ordered)
}

fn apply_action(
    mass: StateMass,
    action: &TimedSensorAction,
) -> Result<Vec<OutcomeMass>, TimeAwarePolicyError> {
    let mut branches = Vec::with_capacity(OUTCOME_ORDER.len());
    for outcome in OUTCOME_ORDER {
        // First update the historical capture-epoch belief.
        let capture_inside = mass.inside.checked_mul(
            action.inside_likelihoods.probability(outcome) as u128,
        ).ok_or(TimeAwarePolicyError::ArithmeticOverflow)?;
        let capture_outside = mass.outside.checked_mul(
            action.outside_likelihoods.probability(outcome) as u128,
        ).ok_or(TimeAwarePolicyError::ArithmeticOverflow)?;

        // Then propagate that posterior to the epoch when the evidence is
        // usable. The outcome does not magically observe the future state.
        let stay_inside = (PROBABILITY_SCALE - action.transition.inside_to_outside_ppm) as u128;
        let stay_outside = (PROBABILITY_SCALE - action.transition.outside_to_inside_ppm) as u128;
        let inside_to_outside = action.transition.inside_to_outside_ppm as u128;
        let outside_to_inside = action.transition.outside_to_inside_ppm as u128;

        let use_inside = capture_inside.checked_mul(stay_inside)
            .and_then(|value| value.checked_add(capture_outside.checked_mul(outside_to_inside)?))
            .ok_or(TimeAwarePolicyError::ArithmeticOverflow)?;
        let use_outside = capture_inside.checked_mul(inside_to_outside)
            .and_then(|value| value.checked_add(capture_outside.checked_mul(stay_outside)?))
            .ok_or(TimeAwarePolicyError::ArithmeticOverflow)?;

        if capture_inside != 0 || capture_outside != 0 {
            branches.push(OutcomeMass {
                outcome,
                capture_mass: StateMass {
                    inside: capture_inside,
                    outside: capture_outside,
                },
                use_mass: StateMass {
                    inside: use_inside,
                    outside: use_outside,
                },
            });
        }
    }
    Ok(branches)
}

fn scale_power(exponent: u8) -> Result<u128, TimeAwarePolicyError> {
    let mut value = 1_u128;
    for _ in 0..exponent {
        value = value.checked_mul(SCALE).ok_or(TimeAwarePolicyError::ArithmeticOverflow)?;
    }
    Ok(value)
}

fn initial_mass(problem: &TimeAwareProblem) -> StateMass {
    StateMass {
        inside: (PROBABILITY_SCALE - problem.prior_outside_ppm) as u128,
        outside: problem.prior_outside_ppm as u128,
    }
}

fn stopped_plan(
    problem: &TimeAwareProblem,
    planner: PlannerKind,
) -> Result<TimeAwarePolicyPlan, TimeAwarePolicyError> {
    let (loss, decision) = terminal_assessment(initial_mass(problem), problem)?;
    let numerator = loss.checked_mul(scale_power(4)?)
        .ok_or(TimeAwarePolicyError::ArithmeticOverflow)?;
    Ok(TimeAwarePolicyPlan {
        planner,
        first_action_id: None,
        expected_terminal_loss_numerator: numerator,
        expected_terminal_loss_denominator: RISK_DENOMINATOR,
        worst_case_resource_use: ResourceVector::default(),
        branches: Vec::new(),
        terminal_decision_if_no_action: Some(decision),
        authority_transition_not_authorized_by_plan: true,
    })
}

fn action_by_id<'a>(actions: &[&'a TimedSensorAction], id: u64) -> Option<&'a TimedSensorAction> {
    actions.iter().copied().find(|action| action.action_id == id)
}

fn best_continuation(
    use_mass: StateMass,
    remaining: &[&TimedSensorAction],
    remaining_budget: ResourceVector,
    problem: &TimeAwareProblem,
) -> Result<Continuation, TimeAwarePolicyError> {
    let (stop_loss, stop_decision) = terminal_assessment(use_mass, problem)?;
    let mut best = Continuation {
        action_id: None,
        terminal_decision: Some(stop_decision),
        outcomes: Vec::new(),
        resource_use: ResourceVector::default(),
        loss_numerator: stop_loss.checked_mul(scale_power(2)?)
            .ok_or(TimeAwarePolicyError::ArithmeticOverflow)?,
    };

    for action in remaining {
        if !action.cost.fits(remaining_budget) {
            continue;
        }
        let mut total_loss = 0_u128;
        let mut outcomes = Vec::new();
        for branch in apply_action(use_mass, action)? {
            let (loss, decision) = terminal_assessment(branch.use_mass, problem)?;
            total_loss = total_loss.checked_add(loss)
                .ok_or(TimeAwarePolicyError::ArithmeticOverflow)?;
            outcomes.push(SecondStageOutcome {
                outcome: branch.outcome,
                posterior_outside_at_capture: posterior_outside(branch.capture_mass)?,
                posterior_outside_at_use: posterior_outside(branch.use_mass)?,
                terminal_decision: decision,
                terminal_loss_numerator: loss,
            });
        }
        if total_loss < best.loss_numerator {
            best = Continuation {
                action_id: Some(action.action_id),
                terminal_decision: None,
                outcomes,
                resource_use: action.cost,
                loss_numerator: total_loss,
            };
        }
    }
    Ok(best)
}

fn build_plan_for_first_action(
    problem: &TimeAwareProblem,
    actions: &[&TimedSensorAction],
    budget: ResourceVector,
    first_action_id: u64,
    planner: PlannerKind,
) -> Result<TimeAwarePolicyPlan, TimeAwarePolicyError> {
    let first = action_by_id(actions, first_action_id)
        .ok_or(TimeAwarePolicyError::ArithmeticOverflow)?;
    let remaining_budget = budget.remaining(first.cost)
        .ok_or(TimeAwarePolicyError::ArithmeticOverflow)?;
    let remaining: Vec<&TimedSensorAction> = actions.iter().copied()
        .filter(|action| action.action_id != first_action_id)
        .collect();

    let mut branches = Vec::new();
    let mut total_loss = 0_u128;
    let mut worst_resources = first.cost;
    for branch in apply_action(initial_mass(problem), first)? {
        let continuation = best_continuation(
            branch.use_mass,
            &remaining,
            remaining_budget,
            problem,
        )?;
        let resource_use = first.cost.checked_add(continuation.resource_use)?;
        total_loss = total_loss.checked_add(continuation.loss_numerator)
            .ok_or(TimeAwarePolicyError::ArithmeticOverflow)?;
        worst_resources = worst_resources.component_max(resource_use);
        branches.push(FirstActionBranch {
            outcome: branch.outcome,
            posterior_outside_at_capture: posterior_outside(branch.capture_mass)?,
            posterior_outside_at_use: posterior_outside(branch.use_mass)?,
            second_action_id: continuation.action_id,
            terminal_decision: continuation.terminal_decision,
            second_stage_outcomes: continuation.outcomes,
            resource_use,
            subtree_loss_numerator: continuation.loss_numerator,
        });
    }

    Ok(TimeAwarePolicyPlan {
        planner,
        first_action_id: Some(first_action_id),
        expected_terminal_loss_numerator: total_loss,
        expected_terminal_loss_denominator: RISK_DENOMINATOR,
        worst_case_resource_use: worst_resources,
        branches,
        terminal_decision_if_no_action: None,
        authority_transition_not_authorized_by_plan: true,
    })
}

pub fn plan_time_aware_horizon_two(
    problem: &TimeAwareProblem,
    actions: &[TimedSensorAction],
    budget: ResourceVector,
) -> Result<TimeAwarePolicyPlan, TimeAwarePolicyError> {
    let ordered = validate(problem, actions)?;
    let mut best = stopped_plan(problem, PlannerKind::ExactHorizonTwo)?;
    for action in &ordered {
        if !action.cost.fits(budget) {
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

/// Static-state comparator: replace every transition with identity, retain
/// the same observation likelihoods, action budgets, and losses.
pub fn plan_static_horizon_two(
    problem: &TimeAwareProblem,
    actions: &[TimedSensorAction],
    budget: ResourceVector,
) -> Result<TimeAwarePolicyPlan, TimeAwarePolicyError> {
    let mut static_actions = actions.to_vec();
    for action in &mut static_actions {
        action.transition = StateTransition {
            inside_to_outside_ppm: 0,
            outside_to_inside_ppm: 0,
        };
        action.transition_model_sha256 = [0xA5; 32];
    }
    plan_time_aware_horizon_two(problem, &static_actions, budget)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn likelihoods(no: u64, zero: u64, one: u64) -> OutcomeLikelihoods {
        OutcomeLikelihoods {
            no_observation_ppm: no,
            observed_zero_ppm: zero,
            observed_one_ppm: one,
        }
    }

    fn problem(prior_outside: u64) -> TimeAwareProblem {
        TimeAwareProblem {
            prior_outside_ppm: prior_outside,
            false_safe_loss: 1,
            false_degraded_loss: 1,
        }
    }

    fn action(
        id: u64,
        cost: ResourceVector,
        age: u64,
        transition: StateTransition,
        inside: OutcomeLikelihoods,
        outside: OutcomeLikelihoods,
    ) -> TimedSensorAction {
        TimedSensorAction {
            action_id: id,
            cost,
            evidence_age_ns: age,
            transition,
            outcome_model_sha256: [0x31; 32],
            transition_model_sha256: [0x42; 32],
            inside_likelihoods: inside,
            outside_likelihoods: outside,
        }
    }

    fn short_budget() -> ResourceVector {
        ResourceVector {
            latency_ns: 100,
            energy_uj: 10,
            bandwidth_bytes: 10,
        }
    }

    fn identity() -> StateTransition {
        StateTransition {
            inside_to_outside_ppm: 0,
            outside_to_inside_ppm: 0,
        }
    }

    fn transition_sensitive_fixture() -> Vec<TimedSensorAction> {
        let perfect_inside = likelihoods(0, PROBABILITY_SCALE, 0);
        let perfect_outside = likelihoods(0, 0, PROBABILITY_SCALE);
        let noisy_inside = likelihoods(0, 900_000, 100_000);
        let noisy_outside = likelihoods(0, 100_000, 900_000);
        vec![
            action(
                1,
                short_budget(),
                100,
                StateTransition {
                    inside_to_outside_ppm: 500_000,
                    outside_to_inside_ppm: 0,
                },
                perfect_inside,
                perfect_outside,
            ),
            action(
                2,
                short_budget(),
                0,
                identity(),
                noisy_inside,
                noisy_outside,
            ),
        ]
    }

    #[test]
    fn action_time_state_evolution_can_reverse_the_static_sensor_ranking() {
        let problem = problem(500_000);
        let budget = short_budget();
        let actions = transition_sensitive_fixture();
        let dynamic = plan_time_aware_horizon_two(&problem, &actions, budget).unwrap();
        let static_plan = plan_static_horizon_two(&problem, &actions, budget).unwrap();

        assert_eq!(static_plan.first_action_id, Some(1), "static-state planner prefers perfect capture");
        assert_eq!(dynamic.first_action_id, Some(2), "time-aware planner prefers fresher, noisier evidence");
    }

    #[test]
    fn accurate_capture_can_become_uncertain_at_the_use_epoch() {
        let problem = problem(500_000);
        let actions = transition_sensitive_fixture();
        let plan = plan_time_aware_horizon_two(&problem, &actions, short_budget()).unwrap();
        let stale = plan.branches.iter()
            .find(|branch| branch.outcome == ActionOutcome::ObservedZero)
            .unwrap();

        assert_eq!(plan.first_action_id, Some(2));
        let stale_action = actions.iter().find(|action| action.action_id == 1).unwrap();
        let mut single = stale_action.clone();
        single.action_id = 9;
        let stale_plan = plan_time_aware_horizon_two(
            &problem,
            &[single],
            short_budget(),
        ).unwrap();
        let branch = stale_plan.branches.iter()
            .find(|item| item.outcome == ActionOutcome::ObservedZero)
            .unwrap();
        assert_eq!(branch.posterior_outside_at_capture.numerator, 0);
        assert_eq!(branch.posterior_outside_at_use.numerator * 2, branch.posterior_outside_at_use.denominator);
        assert!(stale.posterior_outside_at_use.denominator > 0);
    }

    #[test]
    fn no_observation_is_updated_at_capture_then_propagated_to_use() {
        let problem = problem(500_000);
        let no_obs_inside = likelihoods(900_000, 100_000, 0);
        let no_obs_outside = likelihoods(100_000, 900_000, 0);
        let action = action(
            1,
            short_budget(),
            100,
            StateTransition {
                inside_to_outside_ppm: 200_000,
                outside_to_inside_ppm: 100_000,
            },
            no_obs_inside,
            no_obs_outside,
        );
        let plan = plan_time_aware_horizon_two(&problem, &[action], short_budget()).unwrap();
        let no_obs = plan.branches.iter()
            .find(|branch| branch.outcome == ActionOutcome::NoObservation)
            .unwrap();
        assert_ne!(
            no_obs.posterior_outside_at_capture.numerator,
            no_obs.posterior_outside_at_use.numerator
                .checked_mul(no_obs.posterior_outside_at_capture.denominator)
                .unwrap_or(0) / no_obs.posterior_outside_at_use.denominator.max(1)
        );
    }

    #[test]
    fn zero_age_requires_identity_transition() {
        let problem = problem(500_000);
        let invalid = action(
            1,
            short_budget(),
            0,
            StateTransition {
                inside_to_outside_ppm: 1,
                outside_to_inside_ppm: 0,
            },
            likelihoods(0, 500_000, 500_000),
            likelihoods(0, 500_000, 500_000),
        );
        assert_eq!(
            plan_time_aware_horizon_two(&problem, &[invalid], short_budget()),
            Err(TimeAwarePolicyError::ZeroAgeNonIdentityTransition(1))
        );
    }

    #[test]
    fn evidence_age_cannot_exceed_action_latency() {
        let invalid = action(
            1,
            ResourceVector { latency_ns: 10, energy_uj: 10, bandwidth_bytes: 10 },
            11,
            identity(),
            likelihoods(0, 500_000, 500_000),
            likelihoods(0, 500_000, 500_000),
        );
        assert_eq!(
            plan_time_aware_horizon_two(&problem(500_000), &[invalid], short_budget()),
            Err(TimeAwarePolicyError::InvalidEvidenceAge(1))
        );
    }

    #[test]
    fn malformed_probability_rows_transitions_and_model_commits_reject() {
        let mut bad = action(
            1,
            short_budget(),
            100,
            identity(),
            likelihoods(0, 500_000, 499_999),
            likelihoods(0, 500_000, 500_000),
        );
        assert_eq!(
            plan_time_aware_horizon_two(&problem(500_000), &[bad.clone()], short_budget()),
            Err(TimeAwarePolicyError::InvalidLikelihoods { action_id: 1, state: "inside" })
        );

        bad = action(
            1,
            short_budget(),
            100,
            StateTransition {
                inside_to_outside_ppm: PROBABILITY_SCALE + 1,
                outside_to_inside_ppm: 0,
            },
            likelihoods(0, 500_000, 500_000),
            likelihoods(0, 500_000, 500_000),
        );
        assert_eq!(
            plan_time_aware_horizon_two(&problem(500_000), &[bad], short_budget()),
            Err(TimeAwarePolicyError::InvalidTransition(1))
        );

        bad = action(
            1,
            short_budget(),
            100,
            identity(),
            likelihoods(0, 500_000, 500_000),
            likelihoods(0, 500_000, 500_000),
        );
        bad.transition_model_sha256 = [0; 32];
        assert_eq!(
            plan_time_aware_horizon_two(&problem(500_000), &[bad], short_budget()),
            Err(TimeAwarePolicyError::MissingModelCommitment(1))
        );
    }

    #[test]
    fn candidate_order_is_deterministic_and_policy_never_reuses_an_action() {
        let problem = problem(500_000);
        let mut actions = transition_sensitive_fixture();
        let first = plan_time_aware_horizon_two(&problem, &actions, short_budget()).unwrap();
        actions.reverse();
        let second = plan_time_aware_horizon_two(&problem, &actions, short_budget()).unwrap();
        assert_eq!(first, second);
        for branch in &first.branches {
            if let Some(second_action) = branch.second_action_id {
                assert_ne!(Some(second_action), first.first_action_id);
            }
            assert!(branch.resource_use.fits(short_budget()));
        }
    }

    #[test]
    fn no_feasible_or_positive_value_action_abstains() {
        let neutral = action(
            1,
            ResourceVector { latency_ns: 0, energy_uj: 0, bandwidth_bytes: 0 },
            0,
            identity(),
            likelihoods(0, 500_000, 500_000),
            likelihoods(0, 500_000, 500_000),
        );
        let plan = plan_time_aware_horizon_two(
            &problem(500_000),
            &[neutral],
            ResourceVector::default(),
        ).unwrap();
        assert_eq!(plan.first_action_id, None);
        assert!(plan.branches.is_empty());
        assert!(plan.authority_transition_not_authorized_by_plan);
    }
}
