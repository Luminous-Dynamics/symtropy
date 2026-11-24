//! Exact mission-global minimax policy-tree oracle for a finite persistent
//! observation-model set. One observable policy tree is evaluated under each
//! fixed mission model. A separate branchwise model-switching diagnostic is
//! reported to make the non-rectangular/rectangular distinction executable.

use core::fmt;

pub const PROBABILITY_SCALE: u64 = 1_000_000_000;
pub const MAX_ACTIONS: usize = 4;
pub const MAX_TERMINAL_LOSS: u64 = 1_000_000_000;
const MODELS: usize = 2;
const STATES: usize = 4;
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

    fn add(self, other: Self) -> Result<Self, MissionMinimaxError> {
        Ok(Self {
            latency_ns: self.latency_ns.checked_add(other.latency_ns)
                .ok_or(MissionMinimaxError::ArithmeticOverflow)?,
            energy_uj: self.energy_uj.checked_add(other.energy_uj)
                .ok_or(MissionMinimaxError::ArithmeticOverflow)?,
            bandwidth_bytes: self.bandwidth_bytes.checked_add(other.bandwidth_bytes)
                .ok_or(MissionMinimaxError::ArithmeticOverflow)?,
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
pub struct PersistentModelAction {
    pub action_id: u64,
    pub cost: ResourceVector,
    /// Likelihoods indexed by [model][common-cause/navigation state].
    /// State order per model: nominal/inside, nominal/outside,
    /// shared-fault/inside, shared-fault/outside.
    pub likelihoods: [[OutcomeLikelihoods; STATES]; MODELS],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MissionMinimaxProblem {
    /// Each model has its own initial distribution over common-cause and
    /// navigation states. Each row must independently sum to 1e9 PPB.
    pub prior_by_model_ppb: [[u64; STATES]; MODELS],
    pub false_safe_loss: u64,
    pub false_degraded_loss: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalDecision {
    DeclareSafe,
    DeclareDegraded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BranchPolicy {
    Stop {
        terminal_decision: TerminalDecision,
    },
    Acquire {
        second_action_id: u64,
        terminal_decisions: [TerminalDecision; 3],
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RobustPolicyBranch {
    pub first_outcome: ActionOutcome,
    /// One controller policy, common to both persistent models.
    pub policy: BranchPolicy,
    /// Contributions under model 0 and model 1 when that model persists
    /// across both sensing stages.
    pub model_loss_contributions: [u128; MODELS],
    /// For each first-stage root model, the larger contribution obtained by
    /// allowing a separate model choice for this branch's second stage.
    pub rectangular_root_model_branch_maxima: [u128; MODELS],
    pub resource_use: ResourceVector,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionMinimaxPlan {
    pub first_action_id: Option<u64>,
    pub root_terminal_decision: Option<TerminalDecision>,
    pub branches: Vec<RobustPolicyBranch>,
    pub model_risk_numerators: [u128; MODELS],
    pub risk_denominator: u128,
    pub worst_case_model_index: usize,
    pub worst_case_risk_numerator: u128,
    /// Risk of a different adversary that may switch model identity
    /// independently after each first-stage observation.
    pub branchwise_switching_risk_numerator: u128,
    pub worst_case_resource_use: ResourceVector,
    pub policy_trees_evaluated: u64,
    pub authority_transition_not_authorized_by_plan: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionMinimaxRegretPlan {
    pub shared_policy: MissionMinimaxPlan,
    /// Best achievable risk if each model identity were revealed before
    /// planning. These are evaluator-only oracle values, not controller data.
    pub model_aware_oracle_risk_numerators: [u128; MODELS],
    /// Shared-policy risk minus the model-aware oracle risk, one per model.
    pub model_regret_numerators: [u128; MODELS],
    pub minimax_regret_numerator: u128,
    pub risk_denominator: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MissionMinimaxError {
    InvalidPrior(usize),
    InvalidDecisionModel,
    TooManyActions,
    InvalidActionId,
    DuplicateActionId(u64),
    InvalidLikelihoods { action_id: u64, model_index: usize, state_index: usize },
    ArithmeticOverflow,
}

impl fmt::Display for MissionMinimaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPrior(model) => write!(f, "prior for model {model} must sum exactly to one billion"),
            Self::InvalidDecisionModel => write!(f, "terminal decision losses are outside their valid range"),
            Self::TooManyActions => write!(f, "exact minimax oracle supports at most four actions"),
            Self::InvalidActionId => write!(f, "action identity must be nonzero"),
            Self::DuplicateActionId(id) => write!(f, "action identity is duplicated: {id}"),
            Self::InvalidLikelihoods { action_id, model_index, state_index } => write!(
                f, "action {action_id} likelihoods for model {model_index}, state {state_index} must sum to one billion"
            ),
            Self::ArithmeticOverflow => write!(f, "checked minimax probability or loss arithmetic overflowed"),
        }
    }
}

impl std::error::Error for MissionMinimaxError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StateMass {
    parts: [u128; STATES],
}

fn validate<'a>(
    problem: &MissionMinimaxProblem,
    actions: &'a [PersistentModelAction],
) -> Result<Vec<&'a PersistentModelAction>, MissionMinimaxError> {
    for model in 0..MODELS {
        let total = problem.prior_by_model_ppb[model].iter()
            .try_fold(0_u64, |sum, mass| sum.checked_add(*mass));
        if total != Some(PROBABILITY_SCALE) {
            return Err(MissionMinimaxError::InvalidPrior(model));
        }
    }
    if problem.false_safe_loss == 0
        || problem.false_degraded_loss == 0
        || problem.false_safe_loss > MAX_TERMINAL_LOSS
        || problem.false_degraded_loss > MAX_TERMINAL_LOSS
    {
        return Err(MissionMinimaxError::InvalidDecisionModel);
    }
    if actions.len() > MAX_ACTIONS {
        return Err(MissionMinimaxError::TooManyActions);
    }

    let mut ordered: Vec<&PersistentModelAction> = actions.iter().collect();
    ordered.sort_by_key(|action| action.action_id);
    for (index, action) in ordered.iter().enumerate() {
        if action.action_id == 0 {
            return Err(MissionMinimaxError::InvalidActionId);
        }
        if index > 0 && ordered[index - 1].action_id == action.action_id {
            return Err(MissionMinimaxError::DuplicateActionId(action.action_id));
        }
        for model in 0..MODELS {
            for state in 0..STATES {
                if action.likelihoods[model][state].total() != Some(PROBABILITY_SCALE)
                    || [
                        action.likelihoods[model][state].no_observation_ppb,
                        action.likelihoods[model][state].observed_zero_ppb,
                        action.likelihoods[model][state].observed_one_ppb,
                    ].iter().any(|p| *p > PROBABILITY_SCALE)
                {
                    return Err(MissionMinimaxError::InvalidLikelihoods {
                        action_id: action.action_id,
                        model_index: model,
                        state_index: state,
                    });
                }
            }
        }
    }
    Ok(ordered)
}

fn initial_mass(problem: &MissionMinimaxProblem, model: usize) -> StateMass {
    StateMass { parts: problem.prior_by_model_ppb[model].map(u128::from) }
}

fn expand(
    mass: StateMass,
    action: &PersistentModelAction,
    model: usize,
) -> Result<[StateMass; 3], MissionMinimaxError> {
    let mut branches = [StateMass { parts: [0; STATES] }; 3];
    for (outcome_index, outcome) in OUTCOMES.iter().enumerate() {
        for state in 0..STATES {
            branches[outcome_index].parts[state] = mass.parts[state]
                .checked_mul(action.likelihoods[model][state].probability(*outcome) as u128)
                .ok_or(MissionMinimaxError::ArithmeticOverflow)?;
        }
    }
    Ok(branches)
}

fn terminal_loss(
    mass: StateMass,
    decision: TerminalDecision,
    problem: &MissionMinimaxProblem,
) -> Result<u128, MissionMinimaxError> {
    let inside = mass.parts[0].checked_add(mass.parts[2])
        .ok_or(MissionMinimaxError::ArithmeticOverflow)?;
    let outside = mass.parts[1].checked_add(mass.parts[3])
        .ok_or(MissionMinimaxError::ArithmeticOverflow)?;
    match decision {
        TerminalDecision::DeclareSafe => outside.checked_mul(problem.false_safe_loss as u128)
            .ok_or(MissionMinimaxError::ArithmeticOverflow),
        TerminalDecision::DeclareDegraded => inside.checked_mul(problem.false_degraded_loss as u128)
            .ok_or(MissionMinimaxError::ArithmeticOverflow),
    }
}

fn action_by_id<'a>(actions: &'a [&PersistentModelAction], id: u64) -> Option<&'a PersistentModelAction> {
    actions.iter().copied().find(|action| action.action_id == id)
}

fn branch_contribution(
    problem: &MissionMinimaxProblem,
    actions: &[&PersistentModelAction],
    root_model: usize,
    second_model: usize,
    first_branch_mass: StateMass,
    policy: BranchPolicy,
) -> Result<u128, MissionMinimaxError> {
    match policy {
        BranchPolicy::Stop { terminal_decision } => {
            terminal_loss(first_branch_mass, terminal_decision, problem)?
                .checked_mul(SCALE)
                .ok_or(MissionMinimaxError::ArithmeticOverflow)
        }
        BranchPolicy::Acquire { second_action_id, terminal_decisions } => {
            let action = action_by_id(actions, second_action_id)
                .ok_or(MissionMinimaxError::ArithmeticOverflow)?;
            let outcomes = expand(first_branch_mass, action, second_model)?;
            let mut total = 0_u128;
            for outcome_index in 0..3 {
                let loss = terminal_loss(outcomes[outcome_index], terminal_decisions[outcome_index], problem)?;
                total = total.checked_add(loss).ok_or(MissionMinimaxError::ArithmeticOverflow)?;
            }
            let _ = root_model; // The first-stage mass already encodes the root model.
            Ok(total)
        }
    }
}

fn branch_options(
    first_action_id: u64,
    actions: &[&PersistentModelAction],
    remaining_budget: ResourceVector,
) -> Vec<BranchPolicy> {
    let mut options = vec![
        BranchPolicy::Stop { terminal_decision: TerminalDecision::DeclareDegraded },
        BranchPolicy::Stop { terminal_decision: TerminalDecision::DeclareSafe },
    ];
    for action in actions {
        if action.action_id == first_action_id || !action.cost.fits(remaining_budget) {
            continue;
        }
        for mask in 0_u8..8 {
            let decisions = [
                if mask & 1 == 0 { TerminalDecision::DeclareDegraded } else { TerminalDecision::DeclareSafe },
                if mask & 2 == 0 { TerminalDecision::DeclareDegraded } else { TerminalDecision::DeclareSafe },
                if mask & 4 == 0 { TerminalDecision::DeclareDegraded } else { TerminalDecision::DeclareSafe },
            ];
            options.push(BranchPolicy::Acquire {
                second_action_id: action.action_id,
                terminal_decisions: decisions,
            });
        }
    }
    options
}

fn policy_resource_use(
    policy: BranchPolicy,
    actions: &[&PersistentModelAction],
) -> Result<ResourceVector, MissionMinimaxError> {
    match policy {
        BranchPolicy::Stop { .. } => Ok(ResourceVector::default()),
        BranchPolicy::Acquire { second_action_id, .. } => action_by_id(actions, second_action_id)
            .map(|action| action.cost)
            .ok_or(MissionMinimaxError::ArithmeticOverflow),
    }
}

fn evaluate_policy(
    problem: &MissionMinimaxProblem,
    actions: &[&PersistentModelAction],
    first: &PersistentModelAction,
    branch_policies: [BranchPolicy; 3],
    budget: ResourceVector,
    trees_evaluated: u64,
) -> Result<MissionMinimaxPlan, MissionMinimaxError> {
    let first_masses = [
        expand(initial_mass(problem, 0), first, 0)?,
        expand(initial_mass(problem, 1), first, 1)?,
    ];
    let mut model_risks = [0_u128; MODELS];
    let mut rectangular_root_totals = [0_u128; MODELS];
    let mut worst_resources = first.cost;
    let mut branches = Vec::with_capacity(3);

    for branch_index in 0..3 {
        let policy = branch_policies[branch_index];
        let mass0 = first_masses[0][branch_index];
        let mass1 = first_masses[1][branch_index];

        let contribution0 = branch_contribution(problem, actions, 0, 0, mass0, policy)?;
        let contribution1 = branch_contribution(problem, actions, 1, 1, mass1, policy)?;

        let rectangular_for_root0 = branch_contribution(problem, actions, 0, 0, mass0, policy)?
            .max(branch_contribution(problem, actions, 0, 1, mass0, policy)?);
        let rectangular_for_root1 = branch_contribution(problem, actions, 1, 0, mass1, policy)?
            .max(branch_contribution(problem, actions, 1, 1, mass1, policy)?);

        model_risks[0] = model_risks[0].checked_add(contribution0)
            .ok_or(MissionMinimaxError::ArithmeticOverflow)?;
        model_risks[1] = model_risks[1].checked_add(contribution1)
            .ok_or(MissionMinimaxError::ArithmeticOverflow)?;
        rectangular_root_totals[0] = rectangular_root_totals[0]
            .checked_add(rectangular_for_root0).ok_or(MissionMinimaxError::ArithmeticOverflow)?;
        rectangular_root_totals[1] = rectangular_root_totals[1]
            .checked_add(rectangular_for_root1).ok_or(MissionMinimaxError::ArithmeticOverflow)?;

        let second_cost = policy_resource_use(policy, actions)?;
        let path_resources = first.cost.add(second_cost)?;
        if !path_resources.fits(budget) {
            return Err(MissionMinimaxError::ArithmeticOverflow);
        }
        worst_resources = worst_resources.component_max(path_resources);

        branches.push(RobustPolicyBranch {
            first_outcome: OUTCOMES[branch_index],
            policy,
            model_loss_contributions: [contribution0, contribution1],
            rectangular_root_model_branch_maxima: [rectangular_for_root0, rectangular_for_root1],
            resource_use: path_resources,
        });
    }

    let worst_model = if model_risks[0] >= model_risks[1] { 0 } else { 1 };
    let rectangular = rectangular_root_totals[0].max(rectangular_root_totals[1]);

    Ok(MissionMinimaxPlan {
        first_action_id: Some(first.action_id),
        root_terminal_decision: None,
        branches,
        model_risk_numerators: model_risks,
        risk_denominator: RISK_DENOMINATOR,
        worst_case_model_index: worst_model,
        worst_case_risk_numerator: model_risks[worst_model],
        branchwise_switching_risk_numerator: rectangular,
        worst_case_resource_use: worst_resources,
        policy_trees_evaluated: trees_evaluated,
        authority_transition_not_authorized_by_plan: true,
    })
}

fn stop_candidate(
    problem: &MissionMinimaxProblem,
    decision: TerminalDecision,
    trees_evaluated: u64,
) -> Result<MissionMinimaxPlan, MissionMinimaxError> {
    let losses = [
        terminal_loss(initial_mass(problem, 0), decision, problem)?,
        terminal_loss(initial_mass(problem, 1), decision, problem)?,
    ];
    let risks = [
        losses[0].checked_mul(SCALE * SCALE).ok_or(MissionMinimaxError::ArithmeticOverflow)?,
        losses[1].checked_mul(SCALE * SCALE).ok_or(MissionMinimaxError::ArithmeticOverflow)?,
    ];
    let worst_model = if risks[0] >= risks[1] { 0 } else { 1 };
    Ok(MissionMinimaxPlan {
        first_action_id: None,
        root_terminal_decision: Some(decision),
        branches: Vec::new(),
        model_risk_numerators: risks,
        risk_denominator: RISK_DENOMINATOR,
        worst_case_model_index: worst_model,
        worst_case_risk_numerator: risks[worst_model],
        branchwise_switching_risk_numerator: risks[worst_model],
        worst_case_resource_use: ResourceVector::default(),
        policy_trees_evaluated: trees_evaluated,
        authority_transition_not_authorized_by_plan: true,
    })
}

/// Exhaustive finite mission-global minimax policy-tree oracle.
///
/// The same root action, branch action, and terminal decision mapping is used
/// under both fixed mission models. The objective maximizes model-wise loss
/// only after evaluating the complete shared observable policy tree. The
/// returned branchwise-switching value is a separate rectangular diagnostic:
/// it permits the second-stage likelihood model to differ by first observation
/// branch while preserving the root-stage model.
pub fn plan_mission_global_minimax(
    problem: &MissionMinimaxProblem,
    actions: &[PersistentModelAction],
    budget: ResourceVector,
) -> Result<MissionMinimaxPlan, MissionMinimaxError> {
    let ordered = validate(problem, actions)?;
    let mut evaluated = 0_u64;

    let stop_degraded = stop_candidate(problem, TerminalDecision::DeclareDegraded, 1)?;
    let stop_safe = stop_candidate(problem, TerminalDecision::DeclareSafe, 2)?;
    evaluated = 2;
    let mut best = if stop_degraded.worst_case_risk_numerator <= stop_safe.worst_case_risk_numerator {
        stop_degraded
    } else {
        stop_safe
    };

    for first in &ordered {
        if !first.cost.fits(budget) {
            continue;
        }
        let remaining_budget = budget.remaining(first.cost).ok_or(MissionMinimaxError::ArithmeticOverflow)?;
        let options = branch_options(first.action_id, &ordered, remaining_budget);
        for policy0 in &options {
            for policy1 in &options {
                for policy2 in &options {
                    evaluated = evaluated.checked_add(1).ok_or(MissionMinimaxError::ArithmeticOverflow)?;
                    let candidate = evaluate_policy(
                        problem,
                        &ordered,
                        first,
                        [*policy0, *policy1, *policy2],
                        budget,
                        evaluated,
                    )?;
                    if candidate.worst_case_risk_numerator < best.worst_case_risk_numerator {
                        best = candidate;
                    }
                }
            }
        }
    }

    best.policy_trees_evaluated = evaluated;
    Ok(best)
}

/// Compute the best model-aware policy risk for one fixed model. Duplicating
/// that model into both model slots turns the existing shared-policy minimax
/// oracle into an exact single-model oracle.
fn model_aware_oracle_risk(
    problem: &MissionMinimaxProblem,
    actions: &[PersistentModelAction],
    budget: ResourceVector,
    model_index: usize,
) -> Result<u128, MissionMinimaxError> {
    let model_problem = MissionMinimaxProblem {
        prior_by_model_ppb: [
            problem.prior_by_model_ppb[model_index],
            problem.prior_by_model_ppb[model_index],
        ],
        false_safe_loss: problem.false_safe_loss,
        false_degraded_loss: problem.false_degraded_loss,
    };
    let model_actions: Vec<PersistentModelAction> = actions
        .iter()
        .map(|action| PersistentModelAction {
            action_id: action.action_id,
            cost: action.cost,
            likelihoods: [
                action.likelihoods[model_index],
                action.likelihoods[model_index],
            ],
        })
        .collect();
    let oracle = plan_mission_global_minimax(&model_problem, &model_actions, budget)?;
    Ok(oracle.worst_case_risk_numerator)
}

fn regret_for_policy(
    policy: &MissionMinimaxPlan,
    model_oracle_risk: [u128; MODELS],
) -> Result<[u128; MODELS], MissionMinimaxError> {
    Ok([
        policy.model_risk_numerators[0]
            .checked_sub(model_oracle_risk[0])
            .ok_or(MissionMinimaxError::ArithmeticOverflow)?,
        policy.model_risk_numerators[1]
            .checked_sub(model_oracle_risk[1])
            .ok_or(MissionMinimaxError::ArithmeticOverflow)?,
    ])
}

fn maximum_regret(regrets: [u128; MODELS]) -> u128 {
    regrets[0].max(regrets[1])
}

/// Exact mission-global minimax-regret selector over the finite observable
/// policy-tree class. Each model-aware oracle may choose a different policy,
/// but the selected deployable policy remains one shared observable tree.
pub fn plan_mission_global_minimax_regret(
    problem: &MissionMinimaxProblem,
    actions: &[PersistentModelAction],
    budget: ResourceVector,
) -> Result<MissionMinimaxRegretPlan, MissionMinimaxError> {
    let ordered = validate(problem, actions)?;
    let oracle_risks = [
        model_aware_oracle_risk(problem, actions, budget, 0)?,
        model_aware_oracle_risk(problem, actions, budget, 1)?,
    ];

    let stop_degraded = stop_candidate(problem, TerminalDecision::DeclareDegraded, 1)?;
    let stop_safe = stop_candidate(problem, TerminalDecision::DeclareSafe, 2)?;
    let degraded_regret = regret_for_policy(&stop_degraded, oracle_risks)?;
    let safe_regret = regret_for_policy(&stop_safe, oracle_risks)?;
    let mut evaluated = 2_u64;
    let (mut best, mut best_regrets) =
        if maximum_regret(degraded_regret) <= maximum_regret(safe_regret) {
            (stop_degraded, degraded_regret)
        } else {
            (stop_safe, safe_regret)
        };

    for first in &ordered {
        if !first.cost.fits(budget) {
            continue;
        }
        let remaining_budget = budget
            .remaining(first.cost)
            .ok_or(MissionMinimaxError::ArithmeticOverflow)?;
        let options = branch_options(first.action_id, &ordered, remaining_budget);
        for policy0 in &options {
            for policy1 in &options {
                for policy2 in &options {
                    evaluated = evaluated
                        .checked_add(1)
                        .ok_or(MissionMinimaxError::ArithmeticOverflow)?;
                    let candidate = evaluate_policy(
                        problem,
                        &ordered,
                        first,
                        [*policy0, *policy1, *policy2],
                        budget,
                        evaluated,
                    )?;
                    let candidate_regrets = regret_for_policy(&candidate, oracle_risks)?;
                    if maximum_regret(candidate_regrets) < maximum_regret(best_regrets) {
                        best = candidate;
                        best_regrets = candidate_regrets;
                    }
                }
            }
        }
    }

    best.policy_trees_evaluated = evaluated;
    Ok(MissionMinimaxRegretPlan {
        shared_policy: best,
        model_aware_oracle_risk_numerators: oracle_risks,
        model_regret_numerators: best_regrets,
        minimax_regret_numerator: maximum_regret(best_regrets),
        risk_denominator: RISK_DENOMINATOR,
    })
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

    fn problem(
        model0: [u64; 4],
        model1: [u64; 4],
        false_safe: u64,
        false_degraded: u64,
    ) -> MissionMinimaxProblem {
        MissionMinimaxProblem {
            prior_by_model_ppb: [model0, model1],
            false_safe_loss: false_safe,
            false_degraded_loss: false_degraded,
        }
    }

    fn action(
        id: u64,
        m0: [OutcomeLikelihoods; 4],
        m1: [OutcomeLikelihoods; 4],
        cost: ResourceVector,
    ) -> PersistentModelAction {
        PersistentModelAction { action_id: id, cost, likelihoods: [m0, m1] }
    }

    fn cost() -> ResourceVector {
        ResourceVector { latency_ns: 1, energy_uj: 1, bandwidth_bytes: 1 }
    }

    fn equal_prior() -> [u64; 4] {
        [500_000_000, 500_000_000, 0, 0]
    }

    fn asymmetric_fixture() -> Vec<PersistentModelAction> {
        let first0 = [l(0, 900_000_000, 100_000_000); 4];
        let first1 = [l(0, 100_000_000, 900_000_000); 4];

        // Second action A is accurate under model 0 and reversed under model 1.
        let good = [
            l(0, 900_000_000, 100_000_000),
            l(0, 100_000_000, 900_000_000),
            l(0, 900_000_000, 100_000_000),
            l(0, 100_000_000, 900_000_000),
        ];
        let bad = [
            l(0, 100_000_000, 900_000_000),
            l(0, 900_000_000, 100_000_000),
            l(0, 100_000_000, 900_000_000),
            l(0, 900_000_000, 100_000_000),
        ];
        vec![
            action(1, first0, first1, cost()),
            action(2, good, bad, cost()),
            action(3, bad, good, cost()),
        ]
    }

    #[test]
    fn mission_global_model_persistence_beats_branchwise_model_switching() {
        let problem = problem(equal_prior(), equal_prior(), 1, 1);
        let plan = plan_mission_global_minimax(
            &problem,
            &asymmetric_fixture(),
            ResourceVector { latency_ns: 2, energy_uj: 2, bandwidth_bytes: 2 },
        ).unwrap();

        assert_eq!(plan.first_action_id, Some(1));
        assert_eq!(plan.branches[1].first_outcome, ActionOutcome::ObservedZero);
        assert_eq!(plan.branches[2].first_outcome, ActionOutcome::ObservedOne);
        assert_eq!(plan.branches[0].policy, BranchPolicy::Stop {
            terminal_decision: TerminalDecision::DeclareDegraded,
        });
        assert_eq!(plan.branches[1].policy, BranchPolicy::Acquire {
            second_action_id: 2,
            terminal_decisions: [TerminalDecision::DeclareDegraded, TerminalDecision::DeclareSafe, TerminalDecision::DeclareDegraded],
        });
        assert_eq!(plan.branches[2].policy, BranchPolicy::Acquire {
            second_action_id: 2,
            terminal_decisions: [TerminalDecision::DeclareDegraded, TerminalDecision::DeclareDegraded, TerminalDecision::DeclareSafe],
        });
        assert_eq!(
            plan.worst_case_risk_numerator,
            180_000_000_000_000_000_000_000_000
        );
        assert_eq!(
            plan.branchwise_switching_risk_numerator,
            900_000_000_000_000_000_000_000_000
        );
        assert!(plan.branchwise_switching_risk_numerator > plan.worst_case_risk_numerator);
        assert_eq!(plan.model_risk_numerators[0], plan.model_risk_numerators[1]);
    }

    #[test]
    fn policy_actions_are_shared_across_models() {
        let problem = problem(equal_prior(), equal_prior(), 1, 1);
        let plan = plan_mission_global_minimax(
            &problem,
            &asymmetric_fixture(),
            ResourceVector { latency_ns: 2, energy_uj: 2, bandwidth_bytes: 2 },
        ).unwrap();
        assert!(plan.branches.iter().all(|branch| match branch.policy {
            BranchPolicy::Stop { .. } => true,
            BranchPolicy::Acquire { second_action_id, .. } => [2, 3].contains(&second_action_id),
        }));
        assert!(plan.authority_transition_not_authorized_by_plan);
    }

    #[test]
    fn resource_limits_apply_to_every_observable_path() {
        let problem = problem(equal_prior(), equal_prior(), 1, 1);
        let budget = ResourceVector { latency_ns: 2, energy_uj: 2, bandwidth_bytes: 2 };
        let plan = plan_mission_global_minimax(
            &problem,
            &asymmetric_fixture(),
            budget,
        ).unwrap();
        assert_eq!(plan.first_action_id, Some(1));
        assert!(plan.branches.iter().all(|branch| branch.resource_use.fits(budget)));
        assert!(plan.worst_case_resource_use.fits(budget));
    }

    #[test]
    fn malformed_priors_likelihoods_and_duplicate_ids_reject() {
        let bad_prior = problem([500_000_001, 500_000_000, 0, 0], equal_prior(), 1, 1);
        assert_eq!(
            plan_mission_global_minimax(&bad_prior, &[], ResourceVector::default()),
            Err(MissionMinimaxError::InvalidPrior(0))
        );

        let valid = problem(equal_prior(), equal_prior(), 1, 1);
        let bad_action = action(
            1,
            [l(0, 0, PROBABILITY_SCALE - 1); 4],
            [l(0, 0, PROBABILITY_SCALE); 4],
            cost(),
        );
        assert!(matches!(
            plan_mission_global_minimax(&valid, &[bad_action], cost()),
            Err(MissionMinimaxError::InvalidLikelihoods { action_id: 1, model_index: 0, state_index: 0 })
        ));
        let duplicate = vec![asymmetric_fixture()[0].clone(), asymmetric_fixture()[0].clone()];
        assert_eq!(
            plan_mission_global_minimax(&valid, &duplicate, cost()),
            Err(MissionMinimaxError::DuplicateActionId(1))
        );
    }

    #[test]
    fn no_feasible_action_returns_robust_stop_policy() {
        let problem = problem(equal_prior(), equal_prior(), 1, 1);
        let plan = plan_mission_global_minimax(
            &problem,
            &asymmetric_fixture(),
            ResourceVector::default(),
        ).unwrap();
        assert_eq!(plan.first_action_id, None);
        assert!(plan.branches.is_empty());
        assert!(plan.authority_transition_not_authorized_by_plan);
    }

    #[test]
    fn minimax_regret_is_measured_against_each_model_aware_oracle() {
        let problem = problem(equal_prior(), equal_prior(), 1, 1);
        let actions = asymmetric_fixture();
        let budget = ResourceVector { latency_ns: 2, energy_uj: 2, bandwidth_bytes: 2 };
        let result = plan_mission_global_minimax_regret(&problem, &actions, budget).unwrap();

        assert!(
            result.shared_policy.model_risk_numerators[0]
                >= result.model_aware_oracle_risk_numerators[0]
        );
        assert!(
            result.shared_policy.model_risk_numerators[1]
                >= result.model_aware_oracle_risk_numerators[1]
        );
        assert_eq!(
            result.minimax_regret_numerator,
            result.model_regret_numerators[0].max(result.model_regret_numerators[1])
        );
        assert_eq!(result.risk_denominator, RISK_DENOMINATOR);
        assert!(result.shared_policy.authority_transition_not_authorized_by_plan);
    }

    fn one_shot_model_action(
        id: u64,
        model_zero_error_ppb: u64,
        model_one_error_ppb: u64,
    ) -> PersistentModelAction {
        let observation_rows = |error: u64| {
            [
                l(0, PROBABILITY_SCALE - error, error),
                l(0, error, PROBABILITY_SCALE - error),
                l(0, PROBABILITY_SCALE - error, error),
                l(0, error, PROBABILITY_SCALE - error),
            ]
        };
        action(
            id,
            observation_rows(model_zero_error_ppb),
            observation_rows(model_one_error_ppb),
            cost(),
        )
    }

    #[test]
    fn minimax_risk_and_minimax_regret_can_select_different_actions() {
        let problem = problem(equal_prior(), equal_prior(), 1, 1);
        let actions = vec![
            // Model 0 prefers action 2; model 1 slightly prefers action 1.
            one_shot_model_action(1, 100_000_000, 450_000_000),
            one_shot_model_action(2, 0, 475_000_000),
        ];
        // Only one acquisition fits in the budget.
        let budget = cost();

        let worst_case = plan_mission_global_minimax(&problem, &actions, budget).unwrap();
        let minimax_regret =
            plan_mission_global_minimax_regret(&problem, &actions, budget).unwrap();

        assert_eq!(worst_case.first_action_id, Some(1));
        assert_eq!(worst_case.worst_case_risk_numerator, 450_000_000_000_000_000_000_000_000);
        assert_eq!(minimax_regret.shared_policy.first_action_id, Some(2));
        assert_eq!(minimax_regret.model_aware_oracle_risk_numerators, [
            0,
            450_000_000_000_000_000_000_000_000,
        ]);
        assert_eq!(
            minimax_regret.minimax_regret_numerator,
            25_000_000_000_000_000_000_000_000
        );
    }

    #[test]
    fn minimax_regret_policy_is_candidate_order_invariant() {
        let problem = problem(equal_prior(), equal_prior(), 1, 1);
        let mut actions = asymmetric_fixture();
        let budget = ResourceVector { latency_ns: 2, energy_uj: 2, bandwidth_bytes: 2 };
        let first = plan_mission_global_minimax_regret(&problem, &actions, budget).unwrap();
        actions.reverse();
        let second = plan_mission_global_minimax_regret(&problem, &actions, budget).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn exact_policy_is_order_invariant() {
        let problem = problem(equal_prior(), equal_prior(), 1, 1);
        let mut actions = asymmetric_fixture();
        let budget = ResourceVector { latency_ns: 2, energy_uj: 2, bandwidth_bytes: 2 };
        let first = plan_mission_global_minimax(&problem, &actions, budget).unwrap();
        actions.reverse();
        let second = plan_mission_global_minimax(&problem, &actions, budget).unwrap();
        assert_eq!(first, second);
    }
}
