//! Exact mission-global robust active sensing with temporal state evolution.
//!
//! Hidden state is model x persistent common-cause regime x navigation state.
//! Model and common-cause identity remain fixed for the complete mission;
//! only the navigation state evolves through each action's evidence-age
//! transition. The controller uses one observable policy tree for every
//! candidate model. This is an offline finite reference, not a physical model
//! calibration, live navigation controller, or authority API.

use core::fmt;

pub const PROBABILITY_SCALE: u64 = 1_000_000;
pub const MAX_ACTIONS: usize = 4;
pub const MAX_TERMINAL_LOSS: u64 = 1_000_000;

const MODEL_COUNT: usize = 2;
const CAUSE_COUNT: usize = 2;
const STATE_COUNT: usize = CAUSE_COUNT * 2;
const SCALE: u128 = PROBABILITY_SCALE as u128;
const RISK_DENOMINATOR: u128 = SCALE * SCALE * SCALE * SCALE * SCALE;

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

impl ActionOutcome {
    fn index(self) -> usize {
        match self {
            Self::NoObservation => 0,
            Self::ObservedZero => 1,
            Self::ObservedOne => 2,
        }
    }
}

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

    fn checked_add(self, other: Self) -> Result<Self, TemporalMinimaxError> {
        Ok(Self {
            latency_ns: self.latency_ns.checked_add(other.latency_ns)
                .ok_or(TemporalMinimaxError::ArithmeticOverflow)?,
            energy_uj: self.energy_uj.checked_add(other.energy_uj)
                .ok_or(TemporalMinimaxError::ArithmeticOverflow)?,
            bandwidth_bytes: self.bandwidth_bytes.checked_add(other.bandwidth_bytes)
                .ok_or(TemporalMinimaxError::ArithmeticOverflow)?,
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

/// Likelihoods are indexed by [persistent model][cause * 2 + navigation state].
/// Within each cause: state 0 is inside and state 1 is outside.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimedJointAction {
    pub action_id: u64,
    pub cost: ResourceVector,
    pub evidence_age_ns: u64,
    pub likelihoods: [[OutcomeLikelihoods; STATE_COUNT]; MODEL_COUNT],
    /// Transition is indexed by [persistent model][persistent cause].
    pub transitions: [[StateTransition; CAUSE_COUNT]; MODEL_COUNT],
    /// Commitment presence is checked; authenticity is not established here.
    pub outcome_model_sha256: [[[u8; 32]; CAUSE_COUNT]; MODEL_COUNT],
    pub transition_model_sha256: [[[u8; 32]; CAUSE_COUNT]; MODEL_COUNT],
}

/// Each model's prior is over [nominal/inside, nominal/outside,
/// shared-fault/inside, shared-fault/outside]. Each row sums to one PPM scale.
/// Model identity itself is not assigned a Bayesian prior: it is the
/// mission-persistent adversarial model index for robust evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TemporalMinimaxProblem {
    pub prior_by_model_ppm: [[u64; STATE_COUNT]; MODEL_COUNT],
    pub false_safe_loss: u64,
    pub false_degraded_loss: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalDecision {
    DeclareSafe,
    DeclareDegraded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservableBranchChoice {
    Stop {
        terminal_decision: TerminalDecision,
    },
    Acquire {
        second_action_id: u64,
        /// Decisions are indexed in OUTCOMES order. One decision map is
        /// shared across all persistent models.
        terminal_decisions: [TerminalDecision; 3],
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservablePolicyBranch {
    pub first_outcome: ActionOutcome,
    pub choice: ObservableBranchChoice,
    /// Unnormalised expected-loss contributions, denominator is shared by
    /// both models and all branches.
    pub model_loss_contributions: [u128; MODEL_COUNT],
    pub path_resource_use: ResourceVector,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyObjective {
    MissionGlobalMinimaxRisk,
    MissionGlobalMinimaxRegret,
    RectangularBranchSwitchingDiagnostic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservablePolicy {
    pub objective: PolicyObjective,
    pub first_action_id: Option<u64>,
    /// Present only when no first action is selected.
    pub root_terminal_decision: Option<TerminalDecision>,
    pub branches: Vec<ObservablePolicyBranch>,
    pub model_loss_numerators: [u128; MODEL_COUNT],
    pub risk_denominator: u128,
    pub worst_case_risk_numerator: u128,
    pub worst_case_model_index: usize,
    pub maximum_model_regret_numerator: u128,
    /// Sum over first-observation branches of the per-branch maximum over
    /// models. This is the branchwise model-switching diagnostic.
    pub rectangular_switching_risk_numerator: u128,
    pub worst_case_resource_use: ResourceVector,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemporalMinimaxPlans {
    pub mission_global_minimax_risk: ObservablePolicy,
    pub mission_global_minimax_regret: ObservablePolicy,
    pub rectangular_switching_diagnostic: ObservablePolicy,
    pub model_aware_oracle_risk_numerators: [u128; MODEL_COUNT],
    pub risk_denominator: u128,
    pub policy_trees_evaluated: u64,
    pub authority_transition_not_authorized_by_plan: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TemporalMinimaxError {
    InvalidPrior(usize),
    InvalidDecisionModel,
    TooManyActions,
    InvalidActionId,
    DuplicateActionId(u64),
    InvalidLikelihoods { action_id: u64, model_index: usize, state_index: usize },
    InvalidTransition { action_id: u64, model_index: usize, cause_index: usize },
    InvalidEvidenceAge(u64),
    ZeroAgeNonIdentityTransition { action_id: u64, model_index: usize, cause_index: usize },
    MissingModelCommitment { action_id: u64, model_index: usize, cause_index: usize },
    NoFeasiblePolicy,
    ArithmeticOverflow,
}

impl fmt::Display for TemporalMinimaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPrior(model) => write!(f, "prior for model {model} must sum exactly to one million"),
            Self::InvalidDecisionModel => write!(f, "terminal losses are outside the valid range"),
            Self::TooManyActions => write!(f, "exact temporal minimax oracle supports at most four actions"),
            Self::InvalidActionId => write!(f, "action identity must be nonzero"),
            Self::DuplicateActionId(id) => write!(f, "action identity is duplicated: {id}"),
            Self::InvalidLikelihoods { action_id, model_index, state_index } => write!(
                f, "action {action_id} likelihoods for model {model_index}, joint state {state_index} must sum to one million"
            ),
            Self::InvalidTransition { action_id, model_index, cause_index } => write!(
                f, "action {action_id} transition for model {model_index}, cause {cause_index} is invalid"
            ),
            Self::InvalidEvidenceAge(id) => write!(f, "action {id} evidence age exceeds declared action latency"),
            Self::ZeroAgeNonIdentityTransition { action_id, model_index, cause_index } => write!(
                f, "action {action_id} has zero age but non-identity transition for model {model_index}, cause {cause_index}"
            ),
            Self::MissingModelCommitment { action_id, model_index, cause_index } => write!(
                f, "action {action_id} is missing an observation/transition commitment for model {model_index}, cause {cause_index}"
            ),
            Self::NoFeasiblePolicy => write!(f, "no feasible observable policy exists"),
            Self::ArithmeticOverflow => write!(f, "checked temporal minimax probability, loss, or resource arithmetic overflowed"),
        }
    }
}

impl std::error::Error for TemporalMinimaxError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StateMass {
    parts: [u128; STATE_COUNT],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OutcomeMass {
    outcome: ActionOutcome,
    use_mass: StateMass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BranchCandidate {
    first_outcome: ActionOutcome,
    choice: ObservableBranchChoice,
    contributions: [u128; MODEL_COUNT],
    path_resource_use: ResourceVector,
}

fn scale_power(exponent: u8) -> Result<u128, TemporalMinimaxError> {
    let mut value = 1_u128;
    for _ in 0..exponent {
        value = value.checked_mul(SCALE).ok_or(TemporalMinimaxError::ArithmeticOverflow)?;
    }
    Ok(value)
}

fn initial_mass(problem: &TemporalMinimaxProblem, model: usize) -> StateMass {
    StateMass {
        parts: problem.prior_by_model_ppm[model].map(u128::from),
    }
}

fn validate<'a>(
    problem: &TemporalMinimaxProblem,
    actions: &'a [TimedJointAction],
) -> Result<Vec<&'a TimedJointAction>, TemporalMinimaxError> {
    if problem.false_safe_loss == 0
        || problem.false_degraded_loss == 0
        || problem.false_safe_loss > MAX_TERMINAL_LOSS
        || problem.false_degraded_loss > MAX_TERMINAL_LOSS
    {
        return Err(TemporalMinimaxError::InvalidDecisionModel);
    }
    for model in 0..MODEL_COUNT {
        let total = problem.prior_by_model_ppm[model]
            .iter()
            .try_fold(0_u64, |sum, value| sum.checked_add(*value));
        if total != Some(PROBABILITY_SCALE) {
            return Err(TemporalMinimaxError::InvalidPrior(model));
        }
    }
    if actions.len() > MAX_ACTIONS {
        return Err(TemporalMinimaxError::TooManyActions);
    }

    let mut ordered: Vec<&TimedJointAction> = actions.iter().collect();
    ordered.sort_by_key(|action| action.action_id);
    for (index, action) in ordered.iter().enumerate() {
        if action.action_id == 0 {
            return Err(TemporalMinimaxError::InvalidActionId);
        }
        if index > 0 && ordered[index - 1].action_id == action.action_id {
            return Err(TemporalMinimaxError::DuplicateActionId(action.action_id));
        }
        if action.evidence_age_ns > action.cost.latency_ns {
            return Err(TemporalMinimaxError::InvalidEvidenceAge(action.action_id));
        }
        for model in 0..MODEL_COUNT {
            for state_index in 0..STATE_COUNT {
                let row = action.likelihoods[model][state_index];
                if row.total() != Some(PROBABILITY_SCALE)
                    || [
                        row.no_observation_ppm,
                        row.observed_zero_ppm,
                        row.observed_one_ppm,
                    ].iter().any(|value| *value > PROBABILITY_SCALE)
                {
                    return Err(TemporalMinimaxError::InvalidLikelihoods {
                        action_id: action.action_id,
                        model_index: model,
                        state_index,
                    });
                }
            }
            for cause in 0..CAUSE_COUNT {
                let transition = action.transitions[model][cause];
                if !transition.is_valid() {
                    return Err(TemporalMinimaxError::InvalidTransition {
                        action_id: action.action_id,
                        model_index: model,
                        cause_index: cause,
                    });
                }
                if action.evidence_age_ns == 0 && !transition.is_identity() {
                    return Err(TemporalMinimaxError::ZeroAgeNonIdentityTransition {
                        action_id: action.action_id,
                        model_index: model,
                        cause_index: cause,
                    });
                }
                if action.outcome_model_sha256[model][cause].iter().all(|byte| *byte == 0)
                    || action.transition_model_sha256[model][cause].iter().all(|byte| *byte == 0)
                {
                    return Err(TemporalMinimaxError::MissingModelCommitment {
                        action_id: action.action_id,
                        model_index: model,
                        cause_index: cause,
                    });
                }
            }
        }
    }
    Ok(ordered)
}

/// Condition on the capture-time outcome and then propagate only the
/// navigation-state component to the action's use epoch. Model and common-cause
/// indices never move. This is also applied to no-observation.
fn apply_action(
    model: usize,
    mass: StateMass,
    action: &TimedJointAction,
) -> Result<Vec<OutcomeMass>, TemporalMinimaxError> {
    let mut results = Vec::with_capacity(OUTCOMES.len());
    for outcome in OUTCOMES {
        let mut capture = [0_u128; STATE_COUNT];
        for state_index in 0..STATE_COUNT {
            capture[state_index] = mass.parts[state_index]
                .checked_mul(action.likelihoods[model][state_index].probability(outcome) as u128)
                .ok_or(TemporalMinimaxError::ArithmeticOverflow)?;
        }

        let mut use_parts = [0_u128; STATE_COUNT];
        for cause in 0..CAUSE_COUNT {
            let inside_index = cause * 2;
            let outside_index = inside_index + 1;
            let transition = action.transitions[model][cause];
            let inside_to_outside = transition.inside_to_outside_ppm as u128;
            let outside_to_inside = transition.outside_to_inside_ppm as u128;
            let stay_inside = SCALE - inside_to_outside;
            let stay_outside = SCALE - outside_to_inside;

            use_parts[inside_index] = capture[inside_index]
                .checked_mul(stay_inside)
                .and_then(|value| value.checked_add(
                    capture[outside_index].checked_mul(outside_to_inside)?,
                ))
                .ok_or(TemporalMinimaxError::ArithmeticOverflow)?;
            use_parts[outside_index] = capture[inside_index]
                .checked_mul(inside_to_outside)
                .and_then(|value| value.checked_add(
                    capture[outside_index].checked_mul(stay_outside)?,
                ))
                .ok_or(TemporalMinimaxError::ArithmeticOverflow)?;
        }
        results.push(OutcomeMass {
            outcome,
            use_mass: StateMass { parts: use_parts },
        });
    }
    Ok(results)
}

fn terminal_loss(
    mass: StateMass,
    problem: &TemporalMinimaxProblem,
    decision: TerminalDecision,
) -> Result<u128, TemporalMinimaxError> {
    let mut inside = 0_u128;
    let mut outside = 0_u128;
    for cause in 0..CAUSE_COUNT {
        inside = inside.checked_add(mass.parts[cause * 2])
            .ok_or(TemporalMinimaxError::ArithmeticOverflow)?;
        outside = outside.checked_add(mass.parts[cause * 2 + 1])
            .ok_or(TemporalMinimaxError::ArithmeticOverflow)?;
    }
    match decision {
        TerminalDecision::DeclareSafe => outside.checked_mul(problem.false_safe_loss as u128),
        TerminalDecision::DeclareDegraded => inside.checked_mul(problem.false_degraded_loss as u128),
    }.ok_or(TemporalMinimaxError::ArithmeticOverflow)
}

fn build_branch_candidates(
    first_outcome: ActionOutcome,
    first_masses: [StateMass; MODEL_COUNT],
    first_action: &TimedJointAction,
    ordered_actions: &[&TimedJointAction],
    problem: &TemporalMinimaxProblem,
    budget: ResourceVector,
) -> Result<Vec<BranchCandidate>, TemporalMinimaxError> {
    let mut options = Vec::new();
    let stop_padding = scale_power(2)?;

    for decision in [TerminalDecision::DeclareSafe, TerminalDecision::DeclareDegraded] {
        let mut contributions = [0_u128; MODEL_COUNT];
        for model in 0..MODEL_COUNT {
            contributions[model] = terminal_loss(first_masses[model], problem, decision)?
                .checked_mul(stop_padding)
                .ok_or(TemporalMinimaxError::ArithmeticOverflow)?;
        }
        options.push(BranchCandidate {
            first_outcome,
            choice: ObservableBranchChoice::Stop { terminal_decision: decision },
            contributions,
            path_resource_use: first_action.cost,
        });
    }

    for second_action in ordered_actions {
        if second_action.action_id == first_action.action_id {
            continue;
        }
        let Some(path_cost) = first_action.cost.checked_add(second_action.cost)
            .ok().filter(|cost| cost.fits(budget)) else {
            continue;
        };
        let second_outcomes = [
            apply_action(0, first_masses[0], second_action)?,
            apply_action(1, first_masses[1], second_action)?,
        ];

        for decision_mask in 0_u8..8 {
            let decisions = [
                if decision_mask & 1 == 0 { TerminalDecision::DeclareSafe } else { TerminalDecision::DeclareDegraded },
                if decision_mask & 2 == 0 { TerminalDecision::DeclareSafe } else { TerminalDecision::DeclareDegraded },
                if decision_mask & 4 == 0 { TerminalDecision::DeclareSafe } else { TerminalDecision::DeclareDegraded },
            ];
            let mut contributions = [0_u128; MODEL_COUNT];
            for model in 0..MODEL_COUNT {
                let mut total = 0_u128;
                for second_outcome in &second_outcomes[model] {
                    let decision = decisions[second_outcome.outcome.index()];
                    total = total.checked_add(terminal_loss(second_outcome.use_mass, problem, decision)?)
                        .ok_or(TemporalMinimaxError::ArithmeticOverflow)?;
                }
                contributions[model] = total;
            }
            options.push(BranchCandidate {
                first_outcome,
                choice: ObservableBranchChoice::Acquire {
                    second_action_id: second_action.action_id,
                    terminal_decisions: decisions,
                },
                contributions,
                path_resource_use: path_cost,
            });
        }
    }
    Ok(options)
}

fn sum_checked(values: [u128; MODEL_COUNT]) -> Result<u128, TemporalMinimaxError> {
    values[0].checked_add(values[1]).ok_or(TemporalMinimaxError::ArithmeticOverflow)
}

fn maximum(values: [u128; MODEL_COUNT]) -> u128 {
    values[0].max(values[1])
}

fn max_regret(
    losses: [u128; MODEL_COUNT],
    oracles: [u128; MODEL_COUNT],
) -> Result<u128, TemporalMinimaxError> {
    Ok(maximum([
        losses[0].checked_sub(oracles[0]).ok_or(TemporalMinimaxError::ArithmeticOverflow)?,
        losses[1].checked_sub(oracles[1]).ok_or(TemporalMinimaxError::ArithmeticOverflow)?,
    ]))
}

fn make_policy(
    objective: PolicyObjective,
    root_action_id: Option<u64>,
    root_terminal_decision: Option<TerminalDecision>,
    selected: &[BranchCandidate],
    losses: [u128; MODEL_COUNT],
    oracles: [u128; MODEL_COUNT],
) -> Result<ObservablePolicy, TemporalMinimaxError> {
    let mut branches = Vec::with_capacity(selected.len());
    let mut rectangular = 0_u128;
    let mut worst_resources = ResourceVector::default();
    for candidate in selected {
        rectangular = rectangular.checked_add(maximum(candidate.contributions))
            .ok_or(TemporalMinimaxError::ArithmeticOverflow)?;
        worst_resources = worst_resources.component_max(candidate.path_resource_use);
        branches.push(ObservablePolicyBranch {
            first_outcome: candidate.first_outcome,
            choice: candidate.choice.clone(),
            model_loss_contributions: candidate.contributions,
            path_resource_use: candidate.path_resource_use,
        });
    }
    let worst_case = maximum(losses);
    let worst_model = if losses[0] >= losses[1] { 0 } else { 1 };
    Ok(ObservablePolicy {
        objective,
        first_action_id: root_action_id,
        root_terminal_decision,
        branches,
        model_loss_numerators: losses,
        risk_denominator: RISK_DENOMINATOR,
        worst_case_risk_numerator: worst_case,
        worst_case_model_index: worst_model,
        maximum_model_regret_numerator: max_regret(losses, oracles)?,
        rectangular_switching_risk_numerator: rectangular,
        worst_case_resource_use: worst_resources,
    })
}

fn better_risk(candidate: &ObservablePolicy, incumbent: &ObservablePolicy) -> Result<bool, TemporalMinimaxError> {
    if candidate.worst_case_risk_numerator != incumbent.worst_case_risk_numerator {
        return Ok(candidate.worst_case_risk_numerator < incumbent.worst_case_risk_numerator);
    }
    if candidate.maximum_model_regret_numerator != incumbent.maximum_model_regret_numerator {
        return Ok(candidate.maximum_model_regret_numerator < incumbent.maximum_model_regret_numerator);
    }
    Ok(sum_checked(candidate.model_loss_numerators)? < sum_checked(incumbent.model_loss_numerators)?)
}

fn better_regret(candidate: &ObservablePolicy, incumbent: &ObservablePolicy) -> Result<bool, TemporalMinimaxError> {
    if candidate.maximum_model_regret_numerator != incumbent.maximum_model_regret_numerator {
        return Ok(candidate.maximum_model_regret_numerator < incumbent.maximum_model_regret_numerator);
    }
    if candidate.worst_case_risk_numerator != incumbent.worst_case_risk_numerator {
        return Ok(candidate.worst_case_risk_numerator < incumbent.worst_case_risk_numerator);
    }
    Ok(sum_checked(candidate.model_loss_numerators)? < sum_checked(incumbent.model_loss_numerators)?)
}

fn better_rectangular(candidate: &ObservablePolicy, incumbent: &ObservablePolicy) -> Result<bool, TemporalMinimaxError> {
    if candidate.rectangular_switching_risk_numerator != incumbent.rectangular_switching_risk_numerator {
        return Ok(candidate.rectangular_switching_risk_numerator < incumbent.rectangular_switching_risk_numerator);
    }
    if candidate.worst_case_risk_numerator != incumbent.worst_case_risk_numerator {
        return Ok(candidate.worst_case_risk_numerator < incumbent.worst_case_risk_numerator);
    }
    Ok(sum_checked(candidate.model_loss_numerators)? < sum_checked(incumbent.model_loss_numerators)?)
}

fn consider(
    policy: ObservablePolicy,
    best_risk: &mut Option<ObservablePolicy>,
    best_regret: &mut Option<ObservablePolicy>,
    best_rectangular: &mut Option<ObservablePolicy>,
) -> Result<(), TemporalMinimaxError> {
    if best_risk.as_ref().is_none_or(|best| better_risk(&policy, best).unwrap_or(false)) {
        *best_risk = Some(policy.clone());
    }
    if best_regret.as_ref().is_none_or(|best| better_regret(&policy, best).unwrap_or(false)) {
        *best_regret = Some(policy.clone());
    }
    if best_rectangular.as_ref().is_none_or(|best| better_rectangular(&policy, best).unwrap_or(false)) {
        *best_rectangular = Some(policy);
    }
    Ok(())
}

/// Enumerates one observable two-action policy tree. Model identity and the
/// common-cause regime are fixed through the mission; only navigation state
/// transitions between capture and use epochs. The returned policies are
/// exact over the supplied finite model set and integer probabilities.
pub fn plan_temporal_mission_minimax(
    problem: &TemporalMinimaxProblem,
    actions: &[TimedJointAction],
    budget: ResourceVector,
) -> Result<TemporalMinimaxPlans, TemporalMinimaxError> {
    let ordered = validate(problem, actions)?;
    let stop_padding = scale_power(4)?;
    let mut model_oracles = [u128::MAX; MODEL_COUNT];

    // No-sensing model-aware terminal oracles.
    for decision in [TerminalDecision::DeclareSafe, TerminalDecision::DeclareDegraded] {
        for model in 0..MODEL_COUNT {
            let risk = terminal_loss(initial_mass(problem, model), problem, decision)?
                .checked_mul(stop_padding)
                .ok_or(TemporalMinimaxError::ArithmeticOverflow)?;
            model_oracles[model] = model_oracles[model].min(risk);
        }
    }

    // Precompute each first action's observable branch choices and use them
    // to compute exact single-model oracles before robust regret selection.
    let mut roots: Vec<(u64, Vec<Vec<BranchCandidate>>)> = Vec::new();
    for first_action in &ordered {
        if !first_action.cost.fits(budget) {
            continue;
        }
        let first_outcomes = [
            apply_action(0, initial_mass(problem, 0), first_action)?,
            apply_action(1, initial_mass(problem, 1), first_action)?,
        ];
        let mut branches = Vec::with_capacity(OUTCOMES.len());
        for outcome in OUTCOMES {
            let index = outcome.index();
            let first_masses = [
                first_outcomes[0][index].use_mass,
                first_outcomes[1][index].use_mass,
            ];
            let options = build_branch_candidates(
                outcome,
                first_masses,
                first_action,
                &ordered,
                problem,
                budget,
            )?;
            for model in 0..MODEL_COUNT {
                let best_branch = options.iter().map(|option| option.contributions[model]).min()
                    .ok_or(TemporalMinimaxError::NoFeasiblePolicy)?;
                model_oracles[model] = model_oracles[model].min(
                    model_oracles[model].checked_sub(model_oracles[model])
                        .and_then(|_| Some(best_branch))
                        .ok_or(TemporalMinimaxError::ArithmeticOverflow)?,
                );
            }
            branches.push(options);
        }
        roots.push((first_action.action_id, branches));
    }

    // The model oracle is the minimum over root actions of the sum of each
    // observable branch's individually optimal continuation. Recompute this
    // aggregation per root because branch options are conditionally independent
    // when the model is known to the oracle.
    for model in 0..MODEL_COUNT {
        let mut best = {
            let safe = terminal_loss(initial_mass(problem, model), problem, TerminalDecision::DeclareSafe)?
                .checked_mul(stop_padding).ok_or(TemporalMinimaxError::ArithmeticOverflow)?;
            let degraded = terminal_loss(initial_mass(problem, model), problem, TerminalDecision::DeclareDegraded)?
                .checked_mul(stop_padding).ok_or(TemporalMinimaxError::ArithmeticOverflow)?;
            safe.min(degraded)
        };
        for (_, branches) in &roots {
            let mut root_loss = 0_u128;
            for options in branches {
                let branch = options.iter().map(|option| option.contributions[model]).min()
                    .ok_or(TemporalMinimaxError::NoFeasiblePolicy)?;
                root_loss = root_loss.checked_add(branch).ok_or(TemporalMinimaxError::ArithmeticOverflow)?;
            }
            best = best.min(root_loss);
        }
        model_oracles[model] = best;
    }

    let mut best_risk: Option<ObservablePolicy> = None;
    let mut best_regret: Option<ObservablePolicy> = None;
    let mut best_rectangular: Option<ObservablePolicy> = None;
    let mut evaluated = 0_u64;

    // No-action policies are candidates too; the terminal decision is shared
    // by both mission models because hidden model identity is not observable.
    for decision in [TerminalDecision::DeclareSafe, TerminalDecision::DeclareDegraded] {
        let losses = [
            terminal_loss(initial_mass(problem, 0), problem, decision)?
                .checked_mul(stop_padding).ok_or(TemporalMinimaxError::ArithmeticOverflow)?,
            terminal_loss(initial_mass(problem, 1), problem, decision)?
                .checked_mul(stop_padding).ok_or(TemporalMinimaxError::ArithmeticOverflow)?,
        ];
        let policy = make_policy(
            PolicyObjective::MissionGlobalMinimaxRisk,
            None,
            Some(decision),
            &[],
            losses,
            model_oracles,
        )?;
        consider(policy, &mut best_risk, &mut best_regret, &mut best_rectangular)?;
        evaluated = evaluated.checked_add(1).ok_or(TemporalMinimaxError::ArithmeticOverflow)?;
    }

    for (root_id, branches) in &roots {
        // At most (2 + 3*8) = 26 options on each of three outcome branches.
        // Product enumeration is at most 26^3 per root, with four roots max.
        for first in &branches[0] {
            for second in &branches[1] {
                for third in &branches[2] {
                    let selected = [first, second, third];
                    let mut losses = [0_u128; MODEL_COUNT];
                    for option in &selected {
                        for model in 0..MODEL_COUNT {
                            losses[model] = losses[model].checked_add(option.contributions[model])
                                .ok_or(TemporalMinimaxError::ArithmeticOverflow)?;
                        }
                    }
                    let selected_owned = selected.iter().map(|item| (*item).clone()).collect::<Vec<_>>();
                    let policy = make_policy(
                        PolicyObjective::MissionGlobalMinimaxRisk,
                        Some(*root_id),
                        None,
                        &selected_owned,
                        losses,
                        model_oracles,
                    )?;
                    consider(policy, &mut best_risk, &mut best_regret, &mut best_rectangular)?;
                    evaluated = evaluated.checked_add(1).ok_or(TemporalMinimaxError::ArithmeticOverflow)?;
                }
            }
        }
    }

    let mut mission_global_minimax_risk = best_risk.ok_or(TemporalMinimaxError::NoFeasiblePolicy)?;
    mission_global_minimax_risk.objective = PolicyObjective::MissionGlobalMinimaxRisk;
    let mut mission_global_minimax_regret = best_regret.ok_or(TemporalMinimaxError::NoFeasiblePolicy)?;
    mission_global_minimax_regret.objective = PolicyObjective::MissionGlobalMinimaxRegret;
    let mut rectangular_switching_diagnostic = best_rectangular.ok_or(TemporalMinimaxError::NoFeasiblePolicy)?;
    rectangular_switching_diagnostic.objective = PolicyObjective::RectangularBranchSwitchingDiagnostic;

    Ok(TemporalMinimaxPlans {
        mission_global_minimax_risk,
        mission_global_minimax_regret,
        rectangular_switching_diagnostic,
        model_aware_oracle_risk_numerators: model_oracles,
        risk_denominator: RISK_DENOMINATOR,
        policy_trees_evaluated: evaluated,
        authority_transition_not_authorized_by_plan: true,
    })
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

    fn identity() -> StateTransition {
        StateTransition { inside_to_outside_ppm: 0, outside_to_inside_ppm: 0 }
    }

    fn problem() -> TemporalMinimaxProblem {
        TemporalMinimaxProblem {
            prior_by_model_ppm: [
                [250_000, 250_000, 250_000, 250_000],
                [250_000, 250_000, 250_000, 250_000],
            ],
            false_safe_loss: 1,
            false_degraded_loss: 1,
        }
    }

    fn action(id: u64, age: u64, latency: u64, inside_error: u64, outside_error: u64) -> TimedJointAction {
        let inside = likelihoods(0, PROBABILITY_SCALE - inside_error, inside_error);
        let outside = likelihoods(0, outside_error, PROBABILITY_SCALE - outside_error);
        TimedJointAction {
            action_id: id,
            cost: ResourceVector { latency_ns: latency, energy_uj: 1, bandwidth_bytes: 1 },
            evidence_age_ns: age,
            likelihoods: [[inside; STATE_COUNT]; MODEL_COUNT],
            transitions: [[identity(); CAUSE_COUNT]; MODEL_COUNT],
            outcome_model_sha256: [[[0x31; 32]; CAUSE_COUNT]; MODEL_COUNT],
            transition_model_sha256: [[[0x42; 32]; CAUSE_COUNT]; MODEL_COUNT],
        }
    }

    fn budget(latency: u64) -> ResourceVector {
        ResourceVector { latency_ns: latency, energy_uj: 10, bandwidth_bytes: 10 }
    }

    fn action_with_correct_outside(id: u64, age: u64, latency: u64) -> TimedJointAction {
        let inside = likelihoods(0, PROBABILITY_SCALE, 0);
        let outside = likelihoods(0, 0, PROBABILITY_SCALE);
        let mut a = action(id, age, latency, 0, 0);
        for model in 0..MODEL_COUNT {
            for cause in 0..CAUSE_COUNT {
                a.likelihoods[model][cause * 2] = inside;
                a.likelihoods[model][cause * 2 + 1] = outside;
            }
        }
        a
    }

    #[test]
    fn opposing_persistent_transition_models_change_the_robust_sensor_choice() {
        let mut stale_perfect = action_with_correct_outside(1, 100, 100);
        let fresh_noisy = action(2, 0, 100, 100_000, 100_000);
        stale_perfect.transitions[0] = [
            StateTransition { inside_to_outside_ppm: 500_000, outside_to_inside_ppm: 0 },
            StateTransition { inside_to_outside_ppm: 500_000, outside_to_inside_ppm: 0 },
        ];
        let plans = plan_temporal_mission_minimax(&problem(), &[stale_perfect, fresh_noisy], budget(100)).unwrap();
        assert_eq!(plans.mission_global_minimax_risk.first_action_id, Some(2));
        assert_eq!(plans.mission_global_minimax_regret.first_action_id, Some(2));
        assert_eq!(plans.mission_global_minimax_risk.worst_case_risk_numerator, 100_000_000_000_000_000_000_000_000_000);
        assert!(plans.mission_global_minimax_risk.model_loss_numerators[0]
            >= plans.mission_global_minimax_risk.model_loss_numerators[1]);
    }

    #[test]
    fn mission_global_risk_and_rectangular_switching_are_separate_objectives() {
        let mut a = action_with_correct_outside(1, 100, 100);
        a.transitions[0][0] = StateTransition { inside_to_outside_ppm: 400_000, outside_to_inside_ppm: 50_000 };
        a.transitions[1][1] = StateTransition { inside_to_outside_ppm: 50_000, outside_to_inside_ppm: 400_000 };
        let b = action(2, 0, 100, 200_000, 200_000);
        let plans = plan_temporal_mission_minimax(&problem(), &[a, b], budget(100)).unwrap();
        assert!(plans.rectangular_switching_diagnostic.rectangular_switching_risk_numerator
            >= plans.mission_global_minimax_risk.worst_case_risk_numerator);
        assert_eq!(plans.mission_global_minimax_risk.risk_denominator, scale_power(5).unwrap());
    }

    #[test]
    fn common_cause_mass_is_persistent_while_navigation_state_transitions() {
        let mut p = problem();
        p.prior_by_model_ppm[0] = [400_000, 100_000, 100_000, 400_000];
        let a = action(1, 100, 100, 250_000, 250_000);
        let initial = initial_mass(&p, 0);
        let outcomes = apply_action(0, initial, &a).unwrap();
        for outcome in outcomes {
            let nominal_before = initial.parts[0] + initial.parts[1];
            let shared_before = initial.parts[2] + initial.parts[3];
            let nominal_after = outcome.use_mass.parts[0] + outcome.use_mass.parts[1];
            let shared_after = outcome.use_mass.parts[2] + outcome.use_mass.parts[3];
            assert_eq!(nominal_after + shared_after, outcome.use_mass.parts.iter().sum());
            // Both cause slices survive independently; transition cannot move mass between them.
            assert!(nominal_after <= nominal_before * SCALE * SCALE);
            assert!(shared_after <= shared_before * SCALE * SCALE);
        }
    }

    #[test]
    fn missing_transition_evidence_and_zero_age_nonidentity_reject() {
        let p = problem();
        let mut missing = action(1, 100, 100, 100_000, 100_000);
        missing.transition_model_sha256[1][1] = [0; 32];
        assert_eq!(
            plan_temporal_mission_minimax(&p, &[missing], budget(100)),
            Err(TemporalMinimaxError::MissingModelCommitment {
                action_id: 1,
                model_index: 1,
                cause_index: 1,
            })
        );

        let mut zero_age = action(2, 0, 100, 100_000, 100_000);
        zero_age.transitions[0][1].inside_to_outside_ppm = 1;
        assert_eq!(
            plan_temporal_mission_minimax(&p, &[zero_age], budget(100)),
            Err(TemporalMinimaxError::ZeroAgeNonIdentityTransition {
                action_id: 2,
                model_index: 0,
                cause_index: 1,
            })
        );
    }

    #[test]
    fn model_label_permutation_does_not_change_robust_objective() {
        let mut a = action_with_correct_outside(1, 100, 100);
        a.transitions[0][0] = StateTransition { inside_to_outside_ppm: 300_000, outside_to_inside_ppm: 0 };
        let b = action(2, 0, 100, 150_000, 150_000);
        let p = problem();
        let first = plan_temporal_mission_minimax(&p, &[a.clone(), b.clone()], budget(100)).unwrap();

        let mut p_swapped = p;
        p_swapped.prior_by_model_ppm.swap(0, 1);
        let mut a_swapped = a;
        a_swapped.likelihoods.swap(0, 1);
        a_swapped.transitions.swap(0, 1);
        a_swapped.outcome_model_sha256.swap(0, 1);
        a_swapped.transition_model_sha256.swap(0, 1);
        let second = plan_temporal_mission_minimax(&p_swapped, &[a_swapped, b], budget(100)).unwrap();
        assert_eq!(
            first.mission_global_minimax_risk.worst_case_risk_numerator,
            second.mission_global_minimax_risk.worst_case_risk_numerator
        );
        assert_eq!(
            first.mission_global_minimax_regret.maximum_model_regret_numerator,
            second.mission_global_minimax_regret.maximum_model_regret_numerator
        );
        assert_eq!(
            first.mission_global_minimax_risk.first_action_id,
            second.mission_global_minimax_risk.first_action_id
        );
    }

    #[test]
    fn model_set_extension_cannot_beat_the_old_model_oracle() {
        let p = problem();
        let a = action_with_correct_outside(1, 100, 100);
        let b = action(2, 0, 100, 200_000, 200_000);
        let plans = plan_temporal_mission_minimax(&p, &[a, b], budget(100)).unwrap();
        // The robust policy is evaluated under both models, so its worst case
        // cannot be lower than the optimal model-aware risk for either model.
        assert!(plans.mission_global_minimax_risk.worst_case_risk_numerator
            >= plans.model_aware_oracle_risk_numerators[0]);
        assert!(plans.mission_global_minimax_risk.worst_case_risk_numerator
            >= plans.model_aware_oracle_risk_numerators[1]);
    }

    #[test]
    fn candidate_permutation_is_invariant_and_every_branch_respects_resources() {
        let p = problem();
        let mut actions = vec![
            action_with_correct_outside(1, 100, 100),
            action(2, 0, 100, 100_000, 100_000),
            action(3, 0, 100, 250_000, 250_000),
        ];
        let first = plan_temporal_mission_minimax(&p, &actions, budget(200)).unwrap();
        actions.reverse();
        let second = plan_temporal_mission_minimax(&p, &actions, budget(200)).unwrap();
        assert_eq!(first, second);
        for policy in [
            &first.mission_global_minimax_risk,
            &first.mission_global_minimax_regret,
            &first.rectangular_switching_diagnostic,
        ] {
            if let Some(root) = policy.first_action_id {
                for branch in &policy.branches {
                    assert!(branch.path_resource_use.fits(budget(200)));
                    if let ObservableBranchChoice::Acquire { second_action_id, .. } = branch.choice {
                        assert_ne!(Some(second_action_id), Some(root));
                    }
                }
            }
        }
        assert!(first.authority_transition_not_authorized_by_plan);
    }

    #[test]
    fn age_bounds_probability_rows_and_action_ids_fail_closed() {
        let p = problem();
        let mut invalid = action(1, 101, 100, 100_000, 100_000);
        assert_eq!(
            plan_temporal_mission_minimax(&p, &[invalid.clone()], budget(200)),
            Err(TemporalMinimaxError::InvalidEvidenceAge(1))
        );
        invalid = action(1, 100, 100, 100_000, 100_000);
        invalid.likelihoods[1][3].observed_one_ppm += 1;
        assert!(matches!(
            plan_temporal_mission_minimax(&p, &[invalid], budget(200)),
            Err(TemporalMinimaxError::InvalidLikelihoods { action_id: 1, model_index: 1, state_index: 3 })
        ));
        let duplicate = vec![
            action(4, 0, 100, 100_000, 100_000),
            action(4, 0, 100, 100_000, 100_000),
        ];
        assert_eq!(
            plan_temporal_mission_minimax(&p, &duplicate, budget(200)),
            Err(TemporalMinimaxError::DuplicateActionId(4))
        );
    }
}
