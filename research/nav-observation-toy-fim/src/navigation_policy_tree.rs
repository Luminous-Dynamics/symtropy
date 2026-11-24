//! Exact finite-horizon active-sensing oracle for a binary navigation decision.
//!
//! The model is intentionally small and auditable: two latent states, two
//! qualified measurement values plus an explicit no-observation outcome, and
//! a two-action horizon. All probabilities are integer parts-per-billion.
//! This is exact relative to those declared discrete likelihoods; it does not
//! estimate or authenticate them and it does not authorize physical actions.

use core::fmt;

pub const PROBABILITY_SCALE: u64 = 1_000_000_000;
pub const MAX_ACTIONS: usize = 16;
pub const MAX_TERMINAL_LOSS: u64 = 1_000_000_000;

const SCALE: u128 = PROBABILITY_SCALE as u128;
const RISK_DENOMINATOR: u128 = SCALE * SCALE * SCALE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryState {
    InsideServiceEnvelope,
    OutsideServiceEnvelope,
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

    fn checked_add(self, other: Self) -> Result<Self, PolicyTreeError> {
        Ok(Self {
            latency_ns: self
                .latency_ns
                .checked_add(other.latency_ns)
                .ok_or(PolicyTreeError::ArithmeticOverflow)?,
            energy_uj: self
                .energy_uj
                .checked_add(other.energy_uj)
                .ok_or(PolicyTreeError::ArithmeticOverflow)?,
            bandwidth_bytes: self
                .bandwidth_bytes
                .checked_add(other.bandwidth_bytes)
                .ok_or(PolicyTreeError::ArithmeticOverflow)?,
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
pub struct BinarySensorAction {
    pub action_id: u64,
    pub cost: ResourceVector,
    /// P(outcome | inside the declared service envelope).
    pub inside_likelihoods: OutcomeLikelihoods,
    /// P(outcome | outside the declared service envelope).
    pub outside_likelihoods: OutcomeLikelihoods,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BinaryDecisionModel {
    pub prior_outside_probability_parts_per_billion: u64,
    /// Cost of declaring the service safe when the true state is outside.
    pub false_safe_loss: u64,
    /// Cost of declaring the service degraded when the true state is inside.
    pub false_degraded_loss: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactProbability {
    /// Exact numerator of P(outside | this observation history).
    pub numerator: u128,
    /// Exact denominator of P(outside | this observation history).
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
    pub terminal_decision: TerminalDecision,
    /// This outcome's contribution to the common horizon-two risk numerator.
    pub terminal_loss_numerator: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FirstActionBranch {
    pub outcome: ActionOutcome,
    pub posterior_outside_probability: ExactProbability,
    pub second_action_id: Option<u64>,
    /// Present only when the planner stops after this first-stage outcome.
    pub terminal_decision: Option<TerminalDecision>,
    pub second_stage_outcomes: Vec<SecondStageOutcome>,
    pub resource_use: ResourceVector,
    /// Sum of this branch's terminal contributions.
    pub subtree_loss_numerator: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyTreePlan {
    pub planner: PlannerKind,
    pub first_action_id: Option<u64>,
    /// Expected terminal loss is exactly numerator / denominator for the
    /// supplied parts-per-billion likelihood model.
    pub expected_terminal_loss_numerator: u128,
    pub expected_terminal_loss_denominator: u128,
    /// Component-wise maximum resource use over all nonzero-probability paths.
    pub worst_case_resource_use: ResourceVector,
    /// Empty only when the policy abstains before the first acquisition.
    pub branches: Vec<FirstActionBranch>,
    pub terminal_decision_if_no_first_action: Option<TerminalDecision>,
    /// This research plan cannot authorize a navigation or actuation transition.
    pub authority_transition_not_authorized_by_plan: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyTreeError {
    InvalidDecisionModel,
    TooManyActions,
    InvalidActionId,
    DuplicateActionId(u64),
    InvalidLikelihoods {
        action_id: u64,
        state: BinaryState,
    },
    ActionNotFound(u64),
    ArithmeticOverflow,
}

impl fmt::Display for PolicyTreeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDecisionModel => {
                write!(formatter, "prior probability or terminal loss is outside its valid range")
            }
            Self::TooManyActions => {
                write!(formatter, "exact horizon-two oracle action count exceeds its fixed limit")
            }
            Self::InvalidActionId => write!(formatter, "action identity must be nonzero"),
            Self::DuplicateActionId(id) => {
                write!(formatter, "action identity is duplicated: {id}")
            }
            Self::InvalidLikelihoods { action_id, state } => write!(
                formatter,
                "action {action_id} likelihoods for {state:?} must sum exactly to one billion"
            ),
            Self::ActionNotFound(id) => write!(formatter, "first action does not exist: {id}"),
            Self::ArithmeticOverflow => {
                write!(formatter, "checked policy-tree probability or loss arithmetic overflowed")
            }
        }
    }
}

impl std::error::Error for PolicyTreeError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StateMass {
    inside: u128,
    outside: u128,
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
    second_stage_outcomes: Vec<SecondStageOutcome>,
    resource_use: ResourceVector,
    loss_numerator: u128,
}

fn validate_inputs<'a>(
    model: &BinaryDecisionModel,
    actions: &'a [BinarySensorAction],
) -> Result<Vec<&'a BinarySensorAction>, PolicyTreeError> {
    if model.prior_outside_probability_parts_per_billion > PROBABILITY_SCALE
        || model.false_safe_loss == 0
        || model.false_degraded_loss == 0
        || model.false_safe_loss > MAX_TERMINAL_LOSS
        || model.false_degraded_loss > MAX_TERMINAL_LOSS
    {
        return Err(PolicyTreeError::InvalidDecisionModel);
    }
    if actions.len() > MAX_ACTIONS {
        return Err(PolicyTreeError::TooManyActions);
    }

    let mut ordered: Vec<&BinarySensorAction> = actions.iter().collect();
    ordered.sort_by_key(|action| action.action_id);

    for (index, action) in ordered.iter().enumerate() {
        if action.action_id == 0 {
            return Err(PolicyTreeError::InvalidActionId);
        }
        if index > 0 && ordered[index - 1].action_id == action.action_id {
            return Err(PolicyTreeError::DuplicateActionId(action.action_id));
        }
        if action.inside_likelihoods.total() != Some(PROBABILITY_SCALE) {
            return Err(PolicyTreeError::InvalidLikelihoods {
                action_id: action.action_id,
                state: BinaryState::InsideServiceEnvelope,
            });
        }
        if action.outside_likelihoods.total() != Some(PROBABILITY_SCALE) {
            return Err(PolicyTreeError::InvalidLikelihoods {
                action_id: action.action_id,
                state: BinaryState::OutsideServiceEnvelope,
            });
        }
        if [
            action.inside_likelihoods.no_observation_parts_per_billion,
            action.inside_likelihoods.observed_zero_parts_per_billion,
            action.inside_likelihoods.observed_one_parts_per_billion,
            action.outside_likelihoods.no_observation_parts_per_billion,
            action.outside_likelihoods.observed_zero_parts_per_billion,
            action.outside_likelihoods.observed_one_parts_per_billion,
        ]
        .iter()
        .any(|probability| *probability > PROBABILITY_SCALE)
        {
            return Err(PolicyTreeError::InvalidLikelihoods {
                action_id: action.action_id,
                state: BinaryState::InsideServiceEnvelope,
            });
        }
    }

    Ok(ordered)
}

fn initial_mass(model: &BinaryDecisionModel) -> StateMass {
    let outside = model.prior_outside_probability_parts_per_billion as u128;
    StateMass {
        inside: SCALE - outside,
        outside,
    }
}

fn posterior_outside(mass: StateMass) -> Result<ExactProbability, PolicyTreeError> {
    let denominator = mass
        .inside
        .checked_add(mass.outside)
        .ok_or(PolicyTreeError::ArithmeticOverflow)?;
    if denominator == 0 {
        return Err(PolicyTreeError::ArithmeticOverflow);
    }
    Ok(ExactProbability {
        numerator: mass.outside,
        denominator,
    })
}

fn terminal_assessment(
    mass: StateMass,
    model: &BinaryDecisionModel,
) -> Result<(u128, TerminalDecision), PolicyTreeError> {
    let declare_safe_loss = mass
        .outside
        .checked_mul(model.false_safe_loss as u128)
        .ok_or(PolicyTreeError::ArithmeticOverflow)?;
    let declare_degraded_loss = mass
        .inside
        .checked_mul(model.false_degraded_loss as u128)
        .ok_or(PolicyTreeError::ArithmeticOverflow)?;

    // On exact ties, prefer degraded rather than declaring the service safe.
    if declare_safe_loss < declare_degraded_loss {
        Ok((declare_safe_loss, TerminalDecision::DeclareSafe))
    } else {
        Ok((declare_degraded_loss, TerminalDecision::DeclareDegraded))
    }
}

fn expand_action(
    mass: StateMass,
    action: &BinarySensorAction,
) -> Result<Vec<OutcomeMass>, PolicyTreeError> {
    let mut branches = Vec::with_capacity(OUTCOME_ORDER.len());
    for outcome in OUTCOME_ORDER {
        let inside = mass
            .inside
            .checked_mul(action.inside_likelihoods.probability(outcome) as u128)
            .ok_or(PolicyTreeError::ArithmeticOverflow)?;
        let outside = mass
            .outside
            .checked_mul(action.outside_likelihoods.probability(outcome) as u128)
            .ok_or(PolicyTreeError::ArithmeticOverflow)?;

        // A zero-mass event is impossible under both latent states and is not
        // exposed as a controller-visible branch.
        if inside != 0 || outside != 0 {
            branches.push(OutcomeMass {
                outcome,
                mass: StateMass { inside, outside },
            });
        }
    }
    Ok(branches)
}

fn scale_power(exponent: u8) -> Result<u128, PolicyTreeError> {
    let mut value = 1_u128;
    for _ in 0..exponent {
        value = value
            .checked_mul(SCALE)
            .ok_or(PolicyTreeError::ArithmeticOverflow)?;
    }
    Ok(value)
}

fn stopped_plan(
    model: &BinaryDecisionModel,
    planner: PlannerKind,
) -> Result<PolicyTreePlan, PolicyTreeError> {
    let (base_loss, terminal_decision) = terminal_assessment(initial_mass(model), model)?;
    let risk_numerator = base_loss
        .checked_mul(scale_power(2)?)
        .ok_or(PolicyTreeError::ArithmeticOverflow)?;
    Ok(PolicyTreePlan {
        planner,
        first_action_id: None,
        expected_terminal_loss_numerator: risk_numerator,
        expected_terminal_loss_denominator: RISK_DENOMINATOR,
        worst_case_resource_use: ResourceVector::default(),
        branches: Vec::new(),
        terminal_decision_if_no_first_action: Some(terminal_decision),
        authority_transition_not_authorized_by_plan: true,
    })
}

fn best_continuation(
    mass: StateMass,
    remaining_actions: &[&BinarySensorAction],
    remaining_budget: ResourceVector,
    model: &BinaryDecisionModel,
) -> Result<Continuation, PolicyTreeError> {
    let (base_stop_loss, stop_decision) = terminal_assessment(mass, model)?;
    let stop_loss = base_stop_loss
        .checked_mul(SCALE)
        .ok_or(PolicyTreeError::ArithmeticOverflow)?;

    // Stopping wins exact ties, preventing needless sensing when it has no
    // decision-value improvement.
    let mut best = Continuation {
        action_id: None,
        terminal_decision: Some(stop_decision),
        second_stage_outcomes: Vec::new(),
        resource_use: ResourceVector::default(),
        loss_numerator: stop_loss,
    };

    for action in remaining_actions {
        if !action.cost.fits_within(remaining_budget) {
            continue;
        }

        let mut action_loss = 0_u128;
        let mut second_stage_outcomes = Vec::new();
        for branch in expand_action(mass, action)? {
            let (loss, terminal_decision) = terminal_assessment(branch.mass, model)?;
            action_loss = action_loss
                .checked_add(loss)
                .ok_or(PolicyTreeError::ArithmeticOverflow)?;
            second_stage_outcomes.push(SecondStageOutcome {
                outcome: branch.outcome,
                posterior_outside_probability: posterior_outside(branch.mass)?,
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
    model: &BinaryDecisionModel,
    actions: &[&BinarySensorAction],
    budget: ResourceVector,
    first_action_id: u64,
    planner: PlannerKind,
) -> Result<PolicyTreePlan, PolicyTreeError> {
    let first_action = actions
        .iter()
        .copied()
        .find(|action| action.action_id == first_action_id)
        .ok_or(PolicyTreeError::ActionNotFound(first_action_id))?;
    let remaining_budget = budget
        .remaining_after(first_action.cost)
        .ok_or(PolicyTreeError::ArithmeticOverflow)?;
    let remaining_actions: Vec<&BinarySensorAction> = actions
        .iter()
        .copied()
        .filter(|action| action.action_id != first_action_id)
        .collect();

    let mut branches = Vec::new();
    let mut total_loss = 0_u128;
    let mut worst_case_resource_use = first_action.cost;

    for branch in expand_action(initial_mass(model), first_action)? {
        let continuation = best_continuation(
            branch.mass,
            &remaining_actions,
            remaining_budget,
            model,
        )?;
        let resource_use = first_action.cost.checked_add(continuation.resource_use)?;
        total_loss = total_loss
            .checked_add(continuation.loss_numerator)
            .ok_or(PolicyTreeError::ArithmeticOverflow)?;
        worst_case_resource_use = worst_case_resource_use.component_max(resource_use);
        branches.push(FirstActionBranch {
            outcome: branch.outcome,
            posterior_outside_probability: posterior_outside(branch.mass)?,
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

/// Exact non-anticipative horizon-two policy-tree oracle.
///
/// Every first-stage outcome gets its own optimal second action, chosen only
/// from the remaining actions and remaining resource budget. The no-observation
/// event is a genuine Bayesian branch: state-dependent dropout changes the
/// posterior rather than being silently treated as an uninformative event.
///
/// The oracle enumerates all first actions and all three outcomes, then all
/// feasible second actions for each branch. It abstains when no acquisition
/// strictly improves the declared terminal decision risk.
pub fn plan_exact_horizon_two(
    model: &BinaryDecisionModel,
    actions: &[BinarySensorAction],
    budget: ResourceVector,
) -> Result<PolicyTreePlan, PolicyTreeError> {
    let ordered = validate_inputs(model, actions)?;
    let mut best = stopped_plan(model, PlannerKind::ExactHorizonTwo)?;

    for action in &ordered {
        if !action.cost.fits_within(budget) {
            continue;
        }
        let candidate = build_plan_for_first_action(
            model,
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

/// Matched one-step-greedy comparator.
///
/// The first action minimizes immediate expected terminal decision risk. After
/// that fixed first action, each nonzero-probability branch receives its own
/// best feasible second action. This makes the comparator stronger than a
/// purely open-loop two-action sequence while preserving its greedy first
/// decision.
pub fn plan_one_step_greedy(
    model: &BinaryDecisionModel,
    actions: &[BinarySensorAction],
    budget: ResourceVector,
) -> Result<PolicyTreePlan, PolicyTreeError> {
    let ordered = validate_inputs(model, actions)?;
    let (base_stop_loss, _) = terminal_assessment(initial_mass(model), model)?;
    let mut best_immediate_loss = base_stop_loss
        .checked_mul(SCALE)
        .ok_or(PolicyTreeError::ArithmeticOverflow)?;
    let mut best_first_action_id = None;

    for action in &ordered {
        if !action.cost.fits_within(budget) {
            continue;
        }
        let mut action_loss = 0_u128;
        for branch in expand_action(initial_mass(model), action)? {
            let (loss, _) = terminal_assessment(branch.mass, model)?;
            action_loss = action_loss
                .checked_add(loss)
                .ok_or(PolicyTreeError::ArithmeticOverflow)?;
        }
        if action_loss < best_immediate_loss {
            best_immediate_loss = action_loss;
            best_first_action_id = Some(action.action_id);
        }
    }

    match best_first_action_id {
        Some(action_id) => build_plan_for_first_action(
            model,
            &ordered,
            budget,
            action_id,
            PlannerKind::OneStepGreedy,
        ),
        None => stopped_plan(model, PlannerKind::OneStepGreedy),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(prior_outside: u64, false_safe: u64, false_degraded: u64) -> BinaryDecisionModel {
        BinaryDecisionModel {
            prior_outside_probability_parts_per_billion: prior_outside,
            false_safe_loss: false_safe,
            false_degraded_loss: false_degraded,
        }
    }

    fn likelihoods(values: [u64; 3]) -> OutcomeLikelihoods {
        OutcomeLikelihoods {
            no_observation_parts_per_billion: values[0],
            observed_zero_parts_per_billion: values[1],
            observed_one_parts_per_billion: values[2],
        }
    }

    fn action(
        action_id: u64,
        inside: [u64; 3],
        outside: [u64; 3],
        cost: ResourceVector,
    ) -> BinarySensorAction {
        BinarySensorAction {
            action_id,
            cost,
            inside_likelihoods: likelihoods(inside),
            outside_likelihoods: likelihoods(outside),
        }
    }

    fn unit_cost() -> ResourceVector {
        ResourceVector {
            latency_ns: 1,
            energy_uj: 1,
            bandwidth_bytes: 1,
        }
    }

    fn two_action_counterexample() -> Vec<BinarySensorAction> {
        vec![
            action(
                1,
                [970_000_000, 20_000_000, 10_000_000],
                [470_000_000, 420_000_000, 110_000_000],
                unit_cost(),
            ),
            action(
                2,
                [350_000_000, 530_000_000, 120_000_000],
                [490_000_000, 480_000_000, 30_000_000],
                unit_cost(),
            ),
            action(
                3,
                [150_000_000, 730_000_000, 120_000_000],
                [10_000_000, 240_000_000, 750_000_000],
                unit_cost(),
            ),
        ]
    }

    fn two_action_budget() -> ResourceVector {
        ResourceVector {
            latency_ns: 2,
            energy_uj: 2,
            bandwidth_bytes: 2,
        }
    }

    #[test]
    fn exact_horizon_two_is_never_worse_than_greedy_first_action() {
        let model = model(480_000_000, 2, 5);
        let actions = two_action_counterexample();
        let exact = plan_exact_horizon_two(&model, &actions, two_action_budget()).unwrap();
        let greedy = plan_one_step_greedy(&model, &actions, two_action_budget()).unwrap();

        assert!(
            exact.expected_terminal_loss_numerator
                <= greedy.expected_terminal_loss_numerator
        );
        assert_eq!(exact.first_action_id, Some(3));
        assert_eq!(greedy.first_action_id, Some(1));
        assert_eq!(exact.expected_terminal_loss_denominator, RISK_DENOMINATOR);
        assert!(exact.authority_transition_not_authorized_by_plan);
        assert!(greedy.authority_transition_not_authorized_by_plan);
    }

    #[test]
    fn contingent_second_actions_depend_on_the_first_observation_branch() {
        let model = model(480_000_000, 2, 5);
        let actions = two_action_counterexample();
        let exact = plan_exact_horizon_two(&model, &actions, two_action_budget()).unwrap();

        assert_eq!(exact.first_action_id, Some(3));
        assert_eq!(exact.branches.len(), 3);
        assert_eq!(exact.branches[0].outcome, ActionOutcome::NoObservation);
        assert_eq!(exact.branches[1].outcome, ActionOutcome::ObservedZero);
        assert_eq!(exact.branches[2].outcome, ActionOutcome::ObservedOne);
        assert_eq!(
            exact.branches
                .iter()
                .map(|branch| branch.second_action_id)
                .collect::<Vec<_>>(),
            vec![None, Some(1), Some(2)]
        );
    }

    #[test]
    fn state_dependent_no_observation_updates_the_belief_exactly() {
        let action = action(
            7,
            [100_000_000, 450_000_000, 450_000_000],
            [900_000_000, 50_000_000, 50_000_000],
            ResourceVector::default(),
        );
        let prior = StateMass {
            inside: 500_000_000,
            outside: 500_000_000,
        };
        let branch = expand_action(prior, &action)
            .unwrap()
            .into_iter()
            .find(|branch| branch.outcome == ActionOutcome::NoObservation)
            .unwrap();
        let posterior = posterior_outside(branch.mass).unwrap();

        assert_eq!(posterior.numerator * 10, posterior.denominator * 9);
    }

    #[test]
    fn impossible_outcomes_are_not_materialized_as_policy_branches() {
        let action = action(
            8,
            [PROBABILITY_SCALE, 0, 0],
            [PROBABILITY_SCALE, 0, 0],
            ResourceVector::default(),
        );
        let branches = expand_action(
            StateMass {
                inside: 500_000_000,
                outside: 500_000_000,
            },
            &action,
        )
        .unwrap();

        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].outcome, ActionOutcome::NoObservation);
    }

    #[test]
    fn every_contingent_path_respects_each_resource_budget_and_never_reuses_an_action() {
        let model = model(500_000_000, 100, 100);
        let actions = vec![
            action(
                1,
                [100_000_000, 800_000_000, 100_000_000],
                [800_000_000, 100_000_000, 100_000_000],
                unit_cost(),
            ),
            action(
                2,
                [100_000_000, 100_000_000, 800_000_000],
                [800_000_000, 100_000_000, 100_000_000],
                unit_cost(),
            ),
        ];
        let budget = ResourceVector {
            latency_ns: 1,
            energy_uj: 1,
            bandwidth_bytes: 1,
        };
        let plan = plan_exact_horizon_two(&model, &actions, budget).unwrap();

        assert!(plan.first_action_id.is_some());
        for branch in &plan.branches {
            assert_ne!(branch.second_action_id, plan.first_action_id);
            assert!(branch.resource_use.fits_within(budget));
            assert_eq!(branch.second_action_id, None);
        }
    }

    #[test]
    fn candidate_order_does_not_change_the_exact_policy_or_risk() {
        let model = model(480_000_000, 2, 5);
        let actions = two_action_counterexample();
        let mut reversed = actions.clone();
        reversed.reverse();

        let first = plan_exact_horizon_two(&model, &actions, two_action_budget()).unwrap();
        let second = plan_exact_horizon_two(&model, &reversed, two_action_budget()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn invalid_probabilities_duplicate_ids_and_zero_ids_reject() {
        let model = model(500_000_000, 10, 10);
        let mut malformed = two_action_counterexample();
        malformed[0].inside_likelihoods.observed_zero_parts_per_billion = 19_000_000;
        assert!(matches!(
            plan_exact_horizon_two(&model, &malformed, two_action_budget()),
            Err(PolicyTreeError::InvalidLikelihoods { action_id: 1, .. })
        ));

        let duplicate = vec![
            two_action_counterexample()[0].clone(),
            two_action_counterexample()[0].clone(),
        ];
        assert_eq!(
            plan_exact_horizon_two(&model, &duplicate, two_action_budget()),
            Err(PolicyTreeError::DuplicateActionId(1))
        );

        let mut zero_id = two_action_counterexample()[0].clone();
        zero_id.action_id = 0;
        assert_eq!(
            plan_exact_horizon_two(&model, &[zero_id], two_action_budget()),
            Err(PolicyTreeError::InvalidActionId)
        );
    }

    #[test]
    fn terminal_decision_respects_asymmetric_false_safe_and_false_degraded_loss() {
        let safe = stopped_plan(&model(10_000_000, 1, 1_000), PlannerKind::ExactHorizonTwo)
            .unwrap();
        let degraded = stopped_plan(&model(10_000_000, 1_000, 1), PlannerKind::ExactHorizonTwo)
            .unwrap();

        assert_eq!(
            safe.terminal_decision_if_no_first_action,
            Some(TerminalDecision::DeclareSafe)
        );
        assert_eq!(
            degraded.terminal_decision_if_no_first_action,
            Some(TerminalDecision::DeclareDegraded)
        );
    }

    #[test]
    fn no_feasible_or_positive_value_action_produces_explicit_abstention() {
        let model = model(500_000_000, 10, 10);
        let no_value = action(
            1,
            [100_000_000, 450_000_000, 450_000_000],
            [100_000_000, 450_000_000, 450_000_000],
            unit_cost(),
        );
        let plan = plan_exact_horizon_two(
            &model,
            &[no_value],
            ResourceVector {
                latency_ns: 0,
                energy_uj: 0,
                bandwidth_bytes: 0,
            },
        )
        .unwrap();

        assert_eq!(plan.first_action_id, None);
        assert_eq!(
            plan.terminal_decision_if_no_first_action,
            Some(TerminalDecision::DeclareDegraded)
        );
        assert!(plan.branches.is_empty());
    }
}
