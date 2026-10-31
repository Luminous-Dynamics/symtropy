//! Exact finite-horizon optimization for stateful adaptive scientific experiments.
//!
//! The solver returns a Pareto frontier over expected experiment cost, campaign hazard
//! probability, and abstention probability. Each action may update a hypothesis-specific
//! hidden physical state, and repeated actions are allowed. The returned policy is a
//! public-observation policy tree and can be replayed by the independent
//! sim-rel-dynamics-evaluator crate.
//!
//! This is a deterministic, finite-world reference. It is not a stochastic state-space
//! inference engine and does not establish that the supplied world is physically adequate.

#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::collections::BTreeMap;

use sim_rel_dynamics_evaluator::{PolicyNode, PolicyTree, Transition, World};

const MAX_HYPOTHESES: usize = 4_096;
const MAX_ACTIONS: usize = 64;
const MAX_MACHINE_STATES: usize = 4_096;
const MAX_TRANSITION_ENTRIES: usize = 1_000_000;
const MAX_POLICY_STEPS: usize = 128;
const MAX_POLICY_NODES: usize = 8_192;

/// Exact non-negative rational arithmetic used for all frontier comparisons.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rational {
    numerator: u128,
    denominator: u128,
}

impl Rational {
    pub fn new(numerator: u128, denominator: u128) -> Result<Self, String> {
        if denominator == 0 {
            return Err("rational denominator must be positive".into());
        }
        let divisor = gcd(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    pub const fn zero() -> Self {
        Self {
            numerator: 0,
            denominator: 1,
        }
    }

    pub const fn one() -> Self {
        Self {
            numerator: 1,
            denominator: 1,
        }
    }

    pub fn from_u64(value: u64) -> Self {
        Self {
            numerator: u128::from(value),
            denominator: 1,
        }
    }

    pub const fn numerator(self) -> u128 {
        self.numerator
    }

    pub const fn denominator(self) -> u128 {
        self.denominator
    }

    pub fn checked_add(self, other: Self) -> Result<Self, String> {
        let common = gcd(self.denominator, other.denominator);
        let left_scale = other.denominator / common;
        let right_scale = self.denominator / common;
        let left = self
            .numerator
            .checked_mul(left_scale)
            .ok_or_else(|| "rational addition numerator overflowed u128".to_string())?;
        let right = other
            .numerator
            .checked_mul(right_scale)
            .ok_or_else(|| "rational addition numerator overflowed u128".to_string())?;
        let numerator = left
            .checked_add(right)
            .ok_or_else(|| "rational addition sum overflowed u128".to_string())?;
        let denominator = self
            .denominator
            .checked_mul(left_scale)
            .ok_or_else(|| "rational addition denominator overflowed u128".to_string())?;
        Self::new(numerator, denominator)
    }

    pub fn checked_mul(self, other: Self) -> Result<Self, String> {
        let left_cancel = gcd(self.numerator, other.denominator);
        let right_cancel = gcd(other.numerator, self.denominator);
        let left_num = self.numerator / left_cancel;
        let right_num = other.numerator / right_cancel;
        let left_den = self.denominator / right_cancel;
        let right_den = other.denominator / left_cancel;
        let numerator = left_num
            .checked_mul(right_num)
            .ok_or_else(|| "rational multiplication numerator overflowed u128".to_string())?;
        let denominator = left_den
            .checked_mul(right_den)
            .ok_or_else(|| "rational multiplication denominator overflowed u128".to_string())?;
        Self::new(numerator, denominator)
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_ratios(
            self.numerator,
            self.denominator,
            other.numerator,
            other.denominator,
        )
    }
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

/// Compare non-negative ratios without cross-multiplication overflow.
fn compare_ratios(
    mut left_num: u128,
    mut left_den: u128,
    mut right_num: u128,
    mut right_den: u128,
) -> Ordering {
    let mut reversed = false;
    loop {
        let left_quotient = left_num / left_den;
        let right_quotient = right_num / right_den;
        if left_quotient != right_quotient {
            return if reversed {
                right_quotient.cmp(&left_quotient)
            } else {
                left_quotient.cmp(&right_quotient)
            };
        }

        let left_remainder = left_num % left_den;
        let right_remainder = right_num % right_den;
        match (left_remainder == 0, right_remainder == 0) {
            (true, true) => return Ordering::Equal,
            (true, false) => {
                return if reversed {
                    Ordering::Greater
                } else {
                    Ordering::Less
                };
            }
            (false, true) => {
                return if reversed {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
            }
            (false, false) => {}
        }

        left_num = left_den;
        left_den = left_remainder;
        right_num = right_den;
        right_den = right_remainder;
        reversed = !reversed;
    }
}

/// Finite solver bounds. A limit hit returns an error, never a partial frontier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SolverConfig {
    pub horizon: usize,
    pub max_states: usize,
    pub max_frontier_points: usize,
    pub max_generated_combinations: u64,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            horizon: 4,
            max_states: 100_000,
            max_frontier_points: 4_096,
            max_generated_combinations: 1_000_000,
        }
    }
}

impl SolverConfig {
    fn validate(&self) -> Result<(), String> {
        if self.horizon > MAX_POLICY_STEPS {
            return Err(format!("horizon exceeds {MAX_POLICY_STEPS} steps"));
        }
        if self.max_states == 0
            || self.max_frontier_points == 0
            || self.max_generated_combinations == 0
        {
            return Err("all solver limits must be positive".into());
        }
        Ok(())
    }
}

/// Exact objective values for one deterministic adaptive policy.
type MetricKey = (Rational, Rational, Rational, Rational);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontierPoint {
    pub expected_experiment_cost: Rational,
    pub campaign_hazard_probability: Rational,
    pub abstention_probability: Rational,
    pub identification_probability: Rational,
    pub policy: PolicyTree,
}

/// Bounded-solver accounting, reported with every complete frontier.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SolverStats {
    pub states_expanded: usize,
    pub memo_hits: usize,
    pub generated_combinations: u64,
    pub memoized_states: usize,
    pub maximum_frontier_points: usize,
}

/// Result of exact dynamic programming for one deterministic stateful world.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimizationResult {
    pub frontier: Vec<FrontierPoint>,
    pub stats: SolverStats,
}

impl OptimizationResult {
    /// Select minimum experiment cost under explicit hazard and abstention limits.
    ///
    /// Ties are resolved by lower hazard probability, then lower abstention probability,
    /// then the canonical policy ordering established by stable action IDs.
    pub fn minimum_cost_under(
        &self,
        maximum_hazard_probability: Rational,
        maximum_abstention_probability: Rational,
    ) -> Result<Option<&FrontierPoint>, String> {
        if maximum_hazard_probability > Rational::one()
            || maximum_abstention_probability > Rational::one()
        {
            return Err("risk and abstention budgets must lie in [0, 1]".into());
        }
        Ok(self
            .frontier
            .iter()
            .filter(|point| {
                point.campaign_hazard_probability <= maximum_hazard_probability
                    && point.abstention_probability <= maximum_abstention_probability
            })
            .min_by(|left, right| {
                left.expected_experiment_cost
                    .cmp(&right.expected_experiment_cost)
                    .then_with(|| {
                        left.campaign_hazard_probability
                            .cmp(&right.campaign_hazard_probability)
                    })
                    .then_with(|| {
                        left.abstention_probability
                            .cmp(&right.abstention_probability)
                    })
            }))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct BeliefEntry {
    hypothesis_id: String,
    hypothesis_index: usize,
    physical_state: usize,
    prior_mass: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct StateKey {
    remaining_steps: usize,
    entries: Vec<BeliefEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum PolicyPlan {
    Claim {
        hypothesis_id: String,
        hypothesis_index: usize,
    },
    Abstain,
    Experiment {
        action_id: String,
        action_index: usize,
        on_zero: Box<PolicyPlan>,
        on_one: Box<PolicyPlan>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Candidate {
    expected_experiment_cost: Rational,
    campaign_hazard_probability: Rational,
    abstention_probability: Rational,
    identification_probability: Rational,
    plan: PolicyPlan,
}

impl Candidate {
    fn abstain() -> Self {
        Self {
            expected_experiment_cost: Rational::zero(),
            campaign_hazard_probability: Rational::zero(),
            abstention_probability: Rational::one(),
            identification_probability: Rational::zero(),
            plan: PolicyPlan::Abstain,
        }
    }
}

struct DynamicOptimizer<'a> {
    world: &'a World,
    config: &'a SolverConfig,
    action_order: Vec<usize>,
    memo: BTreeMap<StateKey, Vec<Candidate>>,
    stats: SolverStats,
}

/// Solve the exact deterministic finite-horizon (cost, hazard, abstention) frontier.
///
/// The optimizer receives one declared complete world, not evaluator truth. It uses only
/// the world tables and prior mass. Each safe transition updates physical state separately
/// for each surviving hypothesis. Hazard branches terminate immediately. Repeated actions
/// are allowed and are interpreted against the current physical state.
pub fn optimize_stateful_world(
    world: &World,
    config: &SolverConfig,
) -> Result<OptimizationResult, String> {
    config.validate()?;
    validate_world(world)?;

    let mut action_order: Vec<usize> = (0..world.action_ids.len()).collect();
    action_order.sort_by(|left, right| world.action_ids[*left].cmp(&world.action_ids[*right]));

    let mut entries = Vec::new();
    for (hypothesis_index, weight) in world.prior_weights.iter().copied().enumerate() {
        if weight > 0 {
            entries.push(BeliefEntry {
                hypothesis_id: world.hypothesis_ids[hypothesis_index].clone(),
                hypothesis_index,
                physical_state: world.machines[hypothesis_index].initial_state,
                prior_mass: weight,
            });
        }
    }
    entries.sort();

    let mut optimizer = DynamicOptimizer {
        world,
        config,
        action_order,
        memo: BTreeMap::new(),
        stats: SolverStats::default(),
    };
    let candidates = optimizer.solve_state(entries, config.horizon)?;

    let mut frontier = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let mut nodes = Vec::new();
        let root = flatten_policy(&candidate.plan, &mut nodes)?;
        frontier.push(FrontierPoint {
            expected_experiment_cost: candidate.expected_experiment_cost,
            campaign_hazard_probability: candidate.campaign_hazard_probability,
            abstention_probability: candidate.abstention_probability,
            identification_probability: candidate.identification_probability,
            policy: PolicyTree {
                nodes,
                root,
                max_steps: config.horizon,
            },
        });
    }

    optimizer.stats.memoized_states = optimizer.memo.len();
    Ok(OptimizationResult {
        frontier,
        stats: optimizer.stats,
    })
}

impl DynamicOptimizer<'_> {
    fn solve_state(
        &mut self,
        entries: Vec<BeliefEntry>,
        remaining_steps: usize,
    ) -> Result<Vec<Candidate>, String> {
        if entries.is_empty() {
            return Err("optimizer received an empty live belief state".into());
        }
        let key = StateKey {
            remaining_steps,
            entries: entries.clone(),
        };
        if let Some(cached) = self.memo.get(&key) {
            self.stats.memo_hits = self
                .stats
                .memo_hits
                .checked_add(1)
                .ok_or_else(|| "memo-hit counter overflowed".to_string())?;
            return Ok(cached.clone());
        }
        if self.memo.len() >= self.config.max_states {
            return Err(format!(
                "state budget exhausted: configured maximum is {}",
                self.config.max_states
            ));
        }
        self.stats.states_expanded = self
            .stats
            .states_expanded
            .checked_add(1)
            .ok_or_else(|| "state expansion counter overflowed".to_string())?;

        if entries
            .iter()
            .all(|entry| entry.hypothesis_index == entries[0].hypothesis_index)
        {
            let candidate = Candidate {
                expected_experiment_cost: Rational::zero(),
                campaign_hazard_probability: Rational::zero(),
                abstention_probability: Rational::zero(),
                identification_probability: Rational::one(),
                plan: PolicyPlan::Claim {
                    hypothesis_id: entries[0].hypothesis_id.clone(),
                    hypothesis_index: entries[0].hypothesis_index,
                },
            };
            let frontier = vec![candidate];
            self.memo.insert(key, frontier.clone());
            self.record_frontier_size(frontier.len());
            return Ok(frontier);
        }

        let total_mass = entries.iter().try_fold(0_u128, |sum, entry| {
            sum.checked_add(u128::from(entry.prior_mass))
                .ok_or_else(|| "live prior mass overflowed u128".to_string())
        })?;
        if total_mass == 0 {
            return Err("live belief state has zero prior mass".into());
        }

        let mut candidates = vec![Candidate::abstain()];

        if remaining_steps > 0 {
            for action_index in self.action_order.clone() {
                let mut hazard_mass = 0_u128;
                let mut zero_entries = Vec::new();
                let mut one_entries = Vec::new();

                for entry in &entries {
                    let transition = self.world.machines[entry.hypothesis_index].transitions
                        [action_index]
                        .get(entry.physical_state)
                        .ok_or_else(|| {
                            format!(
                                "missing transition for hypothesis {}, action {}, state {}",
                                entry.hypothesis_id,
                                self.world.action_ids[action_index],
                                entry.physical_state
                            )
                        })?;
                    match transition {
                        Transition::Hazard => {
                            hazard_mass = hazard_mass
                                .checked_add(u128::from(entry.prior_mass))
                                .ok_or_else(|| "hazard prior mass overflowed".to_string())?;
                        }
                        Transition::Safe {
                            observation,
                            next_state,
                        } => {
                            let next_entry = BeliefEntry {
                                hypothesis_id: entry.hypothesis_id.clone(),
                                hypothesis_index: entry.hypothesis_index,
                                physical_state: *next_state,
                                prior_mass: entry.prior_mass,
                            };
                            if *observation {
                                one_entries.push(next_entry);
                            } else {
                                zero_entries.push(next_entry);
                            }
                        }
                    }
                }

                zero_entries.sort();
                one_entries.sort();
                let zero_mass = mass_of(&zero_entries)?;
                let one_mass = mass_of(&one_entries)?;
                let probability_zero = Rational::new(zero_mass, total_mass)?;
                let probability_one = Rational::new(one_mass, total_mass)?;
                let probability_hazard = Rational::new(hazard_mass, total_mass)?;

                let zero_frontier = if zero_entries.is_empty() {
                    vec![Candidate::abstain()]
                } else {
                    self.solve_state(zero_entries, remaining_steps - 1)?
                };
                let one_frontier = if one_entries.is_empty() {
                    vec![Candidate::abstain()]
                } else {
                    self.solve_state(one_entries, remaining_steps - 1)?
                };

                let action_cost = Rational::from_u64(self.world.action_costs[action_index]);
                for zero_candidate in &zero_frontier {
                    for one_candidate in &one_frontier {
                        self.stats.generated_combinations = self
                            .stats
                            .generated_combinations
                            .checked_add(1)
                            .ok_or_else(|| "combination counter overflowed".to_string())?;
                        if self.stats.generated_combinations
                            > self.config.max_generated_combinations
                        {
                            return Err(format!(
                                "combination budget exhausted: configured maximum is {}",
                                self.config.max_generated_combinations
                            ));
                        }

                        let expected_experiment_cost = action_cost
                            .checked_add(
                                probability_zero
                                    .checked_mul(zero_candidate.expected_experiment_cost)?,
                            )?
                            .checked_add(
                                probability_one
                                    .checked_mul(one_candidate.expected_experiment_cost)?,
                            )?;
                        let campaign_hazard_probability = probability_hazard
                            .checked_add(
                                probability_zero
                                    .checked_mul(zero_candidate.campaign_hazard_probability)?,
                            )?
                            .checked_add(
                                probability_one
                                    .checked_mul(one_candidate.campaign_hazard_probability)?,
                            )?;
                        let abstention_probability = probability_zero
                            .checked_mul(zero_candidate.abstention_probability)?
                            .checked_add(
                                probability_one
                                    .checked_mul(one_candidate.abstention_probability)?,
                            )?;
                        let identification_probability = probability_zero
                            .checked_mul(zero_candidate.identification_probability)?
                            .checked_add(
                                probability_one
                                    .checked_mul(one_candidate.identification_probability)?,
                            )?;

                        candidates.push(Candidate {
                            expected_experiment_cost,
                            campaign_hazard_probability,
                            abstention_probability,
                            identification_probability,
                            plan: PolicyPlan::Experiment {
                                action_id: self.world.action_ids[action_index].clone(),
                                action_index,
                                on_zero: Box::new(zero_candidate.plan.clone()),
                                on_one: Box::new(one_candidate.plan.clone()),
                            },
                        });
                    }
                }
            }
        }

        let frontier = prune_frontier(candidates, self.config.max_frontier_points)?;
        self.record_frontier_size(frontier.len());
        self.memo.insert(key, frontier.clone());
        Ok(frontier)
    }

    fn record_frontier_size(&mut self, size: usize) {
        self.stats.maximum_frontier_points = self.stats.maximum_frontier_points.max(size);
    }
}

fn mass_of(entries: &[BeliefEntry]) -> Result<u128, String> {
    entries.iter().try_fold(0_u128, |sum, entry| {
        sum.checked_add(u128::from(entry.prior_mass))
            .ok_or_else(|| "branch prior mass overflowed u128".to_string())
    })
}

fn prune_frontier(
    mut candidates: Vec<Candidate>,
    maximum_points: usize,
) -> Result<Vec<Candidate>, String> {
    candidates.sort_by(|left, right| {
        left.expected_experiment_cost
            .cmp(&right.expected_experiment_cost)
            .then_with(|| {
                left.campaign_hazard_probability
                    .cmp(&right.campaign_hazard_probability)
            })
            .then_with(|| {
                left.abstention_probability
                    .cmp(&right.abstention_probability)
            })
            .then_with(|| left.plan.cmp(&right.plan))
    });

    let mut frontier: Vec<Candidate> = Vec::new();
    for candidate in candidates {
        let dominated = frontier.iter().any(|kept| {
            kept.expected_experiment_cost <= candidate.expected_experiment_cost
                && kept.campaign_hazard_probability <= candidate.campaign_hazard_probability
                && kept.abstention_probability <= candidate.abstention_probability
        });
        if !dominated {
            frontier.push(candidate);
            if frontier.len() > maximum_points {
                return Err(format!(
                    "frontier budget exhausted: configured maximum is {maximum_points}"
                ));
            }
        }
    }
    Ok(frontier)
}

fn flatten_policy(plan: &PolicyPlan, nodes: &mut Vec<PolicyNode>) -> Result<usize, String> {
    if nodes.len() >= MAX_POLICY_NODES {
        return Err(format!(
            "policy node budget exhausted: configured maximum is {MAX_POLICY_NODES}"
        ));
    }
    let index = nodes.len();
    nodes.push(PolicyNode::Abstain);

    match plan {
        PolicyPlan::Claim {
            hypothesis_index, ..
        } => {
            nodes[index] = PolicyNode::Claim {
                hypothesis: *hypothesis_index,
            };
        }
        PolicyPlan::Abstain => {
            nodes[index] = PolicyNode::Abstain;
        }
        PolicyPlan::Experiment {
            action_index,
            on_zero,
            on_one,
            ..
        } => {
            let zero_index = flatten_policy(on_zero, nodes)?;
            let one_index = flatten_policy(on_one, nodes)?;
            nodes[index] = PolicyNode::Action {
                action: *action_index,
                on_zero: zero_index,
                on_one: one_index,
            };
        }
    }
    Ok(index)
}

fn validate_world(world: &World) -> Result<(), String> {
    if world.name.trim().is_empty() {
        return Err("world name must not be empty".into());
    }
    let hypothesis_count = world.hypothesis_ids.len();
    let action_count = world.action_ids.len();
    if hypothesis_count == 0
        || hypothesis_count > MAX_HYPOTHESES
        || world.prior_weights.len() != hypothesis_count
        || world.machines.len() != hypothesis_count
    {
        return Err(format!(
            "world must contain 1..={MAX_HYPOTHESES} hypotheses with matching priors and machines"
        ));
    }
    if action_count == 0 || action_count > MAX_ACTIONS || world.action_costs.len() != action_count {
        return Err(format!(
            "world must contain 1..={MAX_ACTIONS} actions with matching costs"
        ));
    }
    if world.action_costs.contains(&0) {
        return Err("all experiment costs must be positive".into());
    }

    validate_unique_ids(&world.hypothesis_ids, "hypothesis")?;
    validate_unique_ids(&world.action_ids, "action")?;

    let total_prior = world.prior_weights.iter().try_fold(0_u128, |sum, weight| {
        sum.checked_add(u128::from(*weight))
            .ok_or_else(|| "world prior mass overflowed u128".to_string())
    })?;
    if total_prior == 0 {
        return Err("world prior has zero total mass".into());
    }

    let mut total_transition_entries = 0_usize;
    for (hypothesis, machine) in world.machines.iter().enumerate() {
        if machine.transitions.len() != action_count {
            return Err(format!(
                "hypothesis {hypothesis} has an inconsistent action-row count"
            ));
        }
        let state_count = machine
            .transitions
            .first()
            .map(Vec::len)
            .ok_or_else(|| format!("hypothesis {hypothesis} has no transition rows"))?;
        if state_count == 0
            || state_count > MAX_MACHINE_STATES
            || machine.initial_state >= state_count
        {
            return Err(format!(
                "hypothesis {hypothesis} has an invalid state space"
            ));
        }
        let machine_entries = state_count
            .checked_mul(action_count)
            .ok_or_else(|| "transition-entry count overflowed".to_string())?;
        total_transition_entries = total_transition_entries
            .checked_add(machine_entries)
            .ok_or_else(|| "transition-entry count overflowed".to_string())?;
        if total_transition_entries > MAX_TRANSITION_ENTRIES {
            return Err(format!(
                "world exceeds the {MAX_TRANSITION_ENTRIES} transition-entry limit"
            ));
        }

        for (action, row) in machine.transitions.iter().enumerate() {
            if row.len() != state_count {
                return Err(format!(
                    "hypothesis {hypothesis}, action {action} has an incomplete transition row"
                ));
            }
            for (state, transition) in row.iter().enumerate() {
                if let Transition::Safe { next_state, .. } = transition {
                    if *next_state >= state_count {
                        return Err(format!(
                            "hypothesis {hypothesis}, action {action}, state {state} has an invalid next state"
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_unique_ids(ids: &[String], kind: &str) -> Result<(), String> {
    for (index, id) in ids.iter().enumerate() {
        if id.trim().is_empty() {
            return Err(format!("{kind} ID must not be empty"));
        }
        if ids[..index].iter().any(|previous| previous == id) {
            return Err(format!("duplicate {kind} ID {id}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_rel_dynamics_evaluator::evaluate_shared_policy;

    fn order_effect_world() -> World {
        use Transition::Safe;
        let h0 = sim_rel_dynamics_evaluator::HypothesisMachine {
            initial_state: 0,
            transitions: vec![
                vec![
                    Safe {
                        observation: false,
                        next_state: 1,
                    },
                    Safe {
                        observation: false,
                        next_state: 1,
                    },
                ],
                vec![
                    Safe {
                        observation: false,
                        next_state: 0,
                    },
                    Safe {
                        observation: false,
                        next_state: 1,
                    },
                ],
            ],
        };
        let h1 = sim_rel_dynamics_evaluator::HypothesisMachine {
            initial_state: 0,
            transitions: vec![
                vec![
                    Safe {
                        observation: false,
                        next_state: 1,
                    },
                    Safe {
                        observation: false,
                        next_state: 1,
                    },
                ],
                vec![
                    Safe {
                        observation: false,
                        next_state: 0,
                    },
                    Safe {
                        observation: true,
                        next_state: 1,
                    },
                ],
            ],
        };
        World {
            name: "order-effect".into(),
            hypothesis_ids: vec!["h0".into(), "h1".into()],
            action_ids: vec!["A-condition".into(), "B-measure".into()],
            prior_weights: vec![1, 1],
            action_costs: vec![1, 1],
            machines: vec![h0, h1],
        }
    }

    fn risk_frontier_world() -> World {
        use Transition::{Hazard, Safe};
        let h0 = sim_rel_dynamics_evaluator::HypothesisMachine {
            initial_state: 0,
            transitions: vec![
                vec![Safe {
                    observation: false,
                    next_state: 0,
                }],
                vec![Safe {
                    observation: false,
                    next_state: 0,
                }],
            ],
        };
        let h1 = sim_rel_dynamics_evaluator::HypothesisMachine {
            initial_state: 0,
            transitions: vec![
                vec![Hazard],
                vec![Safe {
                    observation: true,
                    next_state: 0,
                }],
            ],
        };
        World {
            name: "risk-frontier".into(),
            hypothesis_ids: vec!["h0".into(), "h1".into()],
            action_ids: vec!["A-risky".into(), "B-safe".into()],
            prior_weights: vec![1, 1],
            action_costs: vec![1, 3],
            machines: vec![h0, h1],
        }
    }

    fn zero() -> Rational {
        Rational::zero()
    }

    fn half() -> Rational {
        Rational::new(1, 2).unwrap()
    }

    fn one() -> Rational {
        Rational::one()
    }

    fn point(
        frontier: &[FrontierPoint],
        cost: u128,
        hazard: Rational,
        abstention: Rational,
    ) -> &FrontierPoint {
        frontier
            .iter()
            .find(|candidate| {
                candidate.expected_experiment_cost == Rational::new(cost, 1).unwrap()
                    && candidate.campaign_hazard_probability == hazard
                    && candidate.abstention_probability == abstention
            })
            .expect("expected frontier point")
    }

    #[derive(Clone, Copy, Debug, Default)]
    struct BruteTerminal {
        cost: u128,
        hazard: bool,
        identified: bool,
        abstained: bool,
        false_claim: bool,
    }

    /// Exhaustively materialize every tiny policy tree without memoization.
    /// This is intentionally separate from the production Bellman recursion.
    fn enumerate_all_policy_trees(
        world: &World,
        entries: Vec<BeliefEntry>,
        remaining_steps: usize,
    ) -> Vec<PolicyPlan> {
        if entries
            .iter()
            .all(|entry| entry.hypothesis_index == entries[0].hypothesis_index)
        {
            return vec![PolicyPlan::Claim {
                hypothesis_id: entries[0].hypothesis_id.clone(),
                hypothesis_index: entries[0].hypothesis_index,
            }];
        }
        let mut plans = vec![PolicyPlan::Abstain];
        if remaining_steps == 0 {
            return plans;
        }

        for action_index in 0..world.action_ids.len() {
            let mut zero_entries = Vec::new();
            let mut one_entries = Vec::new();
            for entry in &entries {
                match &world.machines[entry.hypothesis_index].transitions[action_index]
                    [entry.physical_state]
                {
                    Transition::Hazard => {}
                    Transition::Safe {
                        observation,
                        next_state,
                    } => {
                        let next = BeliefEntry {
                            hypothesis_id: entry.hypothesis_id.clone(),
                            hypothesis_index: entry.hypothesis_index,
                            physical_state: *next_state,
                            prior_mass: entry.prior_mass,
                        };
                        if *observation {
                            one_entries.push(next);
                        } else {
                            zero_entries.push(next);
                        }
                    }
                }
            }

            let zero_plans = if zero_entries.is_empty() {
                vec![PolicyPlan::Abstain]
            } else {
                enumerate_all_policy_trees(world, zero_entries, remaining_steps - 1)
            };
            let one_plans = if one_entries.is_empty() {
                vec![PolicyPlan::Abstain]
            } else {
                enumerate_all_policy_trees(world, one_entries, remaining_steps - 1)
            };

            for zero_plan in &zero_plans {
                for one_plan in &one_plans {
                    plans.push(PolicyPlan::Experiment {
                        action_id: world.action_ids[action_index].clone(),
                        action_index,
                        on_zero: Box::new(zero_plan.clone()),
                        on_one: Box::new(one_plan.clone()),
                    });
                }
            }
        }
        plans
    }

    /// Replay one fully materialized policy independently for one evaluator hypothesis.
    fn brute_replay_one(
        plan: &PolicyPlan,
        world: &World,
        hypothesis_index: usize,
        physical_state: usize,
    ) -> Result<BruteTerminal, String> {
        match plan {
            PolicyPlan::Claim {
                hypothesis_index: claimed,
                ..
            } => Ok(BruteTerminal {
                identified: *claimed == hypothesis_index,
                false_claim: *claimed != hypothesis_index,
                ..BruteTerminal::default()
            }),
            PolicyPlan::Abstain => Ok(BruteTerminal {
                abstained: true,
                ..BruteTerminal::default()
            }),
            PolicyPlan::Experiment {
                action_index,
                on_zero,
                on_one,
                ..
            } => {
                let action_cost = u128::from(world.action_costs[*action_index]);
                match &world.machines[hypothesis_index].transitions[*action_index][physical_state] {
                    Transition::Hazard => Ok(BruteTerminal {
                        cost: action_cost,
                        hazard: true,
                        ..BruteTerminal::default()
                    }),
                    Transition::Safe {
                        observation,
                        next_state,
                    } => {
                        let child = if *observation { on_one } else { on_zero };
                        let mut terminal =
                            brute_replay_one(child, world, hypothesis_index, *next_state)?;
                        terminal.cost = terminal
                            .cost
                            .checked_add(action_cost)
                            .ok_or_else(|| "brute replay cost overflowed u128".to_string())?;
                        Ok(terminal)
                    }
                }
            }
        }
    }

    /// Build the oracle frontier from full tree enumeration and per-hypothesis replay.
    fn brute_force_frontier(world: &World, horizon: usize) -> Vec<Candidate> {
        let mut entries = Vec::new();
        for (hypothesis_index, prior_mass) in world.prior_weights.iter().copied().enumerate() {
            if prior_mass > 0 {
                entries.push(BeliefEntry {
                    hypothesis_id: world.hypothesis_ids[hypothesis_index].clone(),
                    hypothesis_index,
                    physical_state: world.machines[hypothesis_index].initial_state,
                    prior_mass,
                });
            }
        }
        entries.sort();

        let plans = enumerate_all_policy_trees(world, entries, horizon);
        let total_mass: u128 = world
            .prior_weights
            .iter()
            .map(|weight| u128::from(*weight))
            .sum();
        let mut unique: BTreeMap<(Rational, Rational, Rational), Candidate> = BTreeMap::new();

        for plan in plans {
            let mut weighted_cost = 0_u128;
            let mut hazard_mass = 0_u128;
            let mut identification_mass = 0_u128;
            let mut abstention_mass = 0_u128;
            let mut false_claim_mass = 0_u128;

            for (hypothesis_index, prior_mass) in world.prior_weights.iter().copied().enumerate() {
                if prior_mass == 0 {
                    continue;
                }
                let terminal = brute_replay_one(
                    &plan,
                    world,
                    hypothesis_index,
                    world.machines[hypothesis_index].initial_state,
                )
                .unwrap();
                let weight = u128::from(prior_mass);
                weighted_cost = weighted_cost
                    .checked_add(weight.checked_mul(terminal.cost).unwrap())
                    .unwrap();
                if terminal.hazard {
                    hazard_mass += weight;
                }
                if terminal.identified {
                    identification_mass += weight;
                }
                if terminal.abstained {
                    abstention_mass += weight;
                }
                if terminal.false_claim {
                    false_claim_mass += weight;
                }
            }
            assert_eq!(
                false_claim_mass, 0,
                "enumerator produced a false exact claim"
            );

            let candidate = Candidate {
                expected_experiment_cost: Rational::new(weighted_cost, total_mass).unwrap(),
                campaign_hazard_probability: Rational::new(hazard_mass, total_mass).unwrap(),
                abstention_probability: Rational::new(abstention_mass, total_mass).unwrap(),
                identification_probability: Rational::new(identification_mass, total_mass).unwrap(),
                plan,
            };
            let key = (
                candidate.expected_experiment_cost,
                candidate.campaign_hazard_probability,
                candidate.abstention_probability,
            );
            match unique.get(&key) {
                Some(existing) if existing.plan.cmp(&candidate.plan) != Ordering::Greater => {}
                _ => {
                    unique.insert(key, candidate);
                }
            }
        }

        let all: Vec<Candidate> = unique.into_values().collect();
        let mut frontier: Vec<Candidate> = Vec::new();
        for (index, candidate) in all.iter().enumerate() {
            let dominated = all.iter().enumerate().any(|(other_index, other)| {
                other_index != index
                    && other.expected_experiment_cost <= candidate.expected_experiment_cost
                    && other.campaign_hazard_probability <= candidate.campaign_hazard_probability
                    && other.abstention_probability <= candidate.abstention_probability
                    && (other.expected_experiment_cost < candidate.expected_experiment_cost
                        || other.campaign_hazard_probability
                            < candidate.campaign_hazard_probability
                        || other.abstention_probability < candidate.abstention_probability)
            });
            if !dominated {
                frontier.push(candidate.clone());
            }
        }
        frontier.sort_by(|left, right| {
            left.expected_experiment_cost
                .cmp(&right.expected_experiment_cost)
                .then_with(|| {
                    left.campaign_hazard_probability
                        .cmp(&right.campaign_hazard_probability)
                })
                .then_with(|| {
                    left.abstention_probability
                        .cmp(&right.abstention_probability)
                })
                .then_with(|| left.plan.cmp(&right.plan))
        });
        frontier
    }

    fn metric_keys(frontier: &[Candidate]) -> Vec<MetricKey> {
        frontier
            .iter()
            .map(|point| {
                (
                    point.expected_experiment_cost,
                    point.campaign_hazard_probability,
                    point.abstention_probability,
                    point.identification_probability,
                )
            })
            .collect()
    }

    fn assert_evaluator_agreement(world: &World, result: &OptimizationResult) {
        for candidate in &result.frontier {
            let report = evaluate_shared_policy(&candidate.policy, std::slice::from_ref(world), 0)
                .expect("independent evaluator must replay optimizer policy");
            let measured = &report.per_world[0];
            assert_eq!(
                Rational::new(
                    measured.expected_cost.numerator(),
                    measured.expected_cost.denominator()
                )
                .unwrap(),
                candidate.expected_experiment_cost
            );
            assert_eq!(
                Rational::new(
                    measured.hazard_probability.numerator(),
                    measured.hazard_probability.denominator()
                )
                .unwrap(),
                candidate.campaign_hazard_probability
            );
            assert_eq!(
                Rational::new(
                    measured.identification_probability.numerator(),
                    measured.identification_probability.denominator()
                )
                .unwrap(),
                candidate.identification_probability
            );
            assert_eq!(
                Rational::new(
                    measured.abstention_probability.numerator(),
                    measured.abstention_probability.denominator()
                )
                .unwrap(),
                candidate.abstention_probability
            );
            assert_eq!(
                Rational::new(
                    measured.false_identification_probability.numerator(),
                    measured.false_identification_probability.denominator()
                )
                .unwrap(),
                zero()
            );
            let terminal_mass = candidate
                .campaign_hazard_probability
                .checked_add(candidate.abstention_probability)
                .unwrap()
                .checked_add(candidate.identification_probability)
                .unwrap();
            assert_eq!(terminal_mass, one());
        }
    }

    #[test]
    fn memoized_optimizer_matches_independent_exhaustive_policy_tree_frontiers() {
        for (world, horizon) in [(order_effect_world(), 2), (risk_frontier_world(), 1)] {
            let result = optimize_stateful_world(
                &world,
                &SolverConfig {
                    horizon,
                    ..SolverConfig::default()
                },
            )
            .unwrap();
            let brute = brute_force_frontier(&world, horizon);
            let optimizer_candidates: Vec<Candidate> = result
                .frontier
                .iter()
                .map(|point| Candidate {
                    expected_experiment_cost: point.expected_experiment_cost,
                    campaign_hazard_probability: point.campaign_hazard_probability,
                    abstention_probability: point.abstention_probability,
                    identification_probability: point.identification_probability,
                    plan: PolicyPlan::Abstain,
                })
                .collect();
            assert_eq!(metric_keys(&optimizer_candidates), metric_keys(&brute));
            assert_evaluator_agreement(&world, &result);
        }
    }

    #[test]
    fn exact_rational_arithmetic_and_comparison_are_checked() {
        let half_value = Rational::new(50, 100).unwrap();
        assert_eq!(half_value, half());
        assert_eq!(half_value.checked_add(half_value).unwrap(), one());
        assert_eq!(
            half_value.checked_mul(half_value).unwrap(),
            Rational::new(1, 4).unwrap()
        );
        assert!(Rational::new(1, 0).is_err());

        let left = Rational::new(u128::MAX - 1, u128::MAX).unwrap();
        let right = Rational::new(u128::MAX - 2, u128::MAX - 1).unwrap();
        assert!(left > right);
    }

    #[test]
    fn dynamic_frontier_finds_condition_then_measure_order_effect() {
        let world = order_effect_world();
        let result = optimize_stateful_world(
            &world,
            &SolverConfig {
                horizon: 2,
                ..SolverConfig::default()
            },
        )
        .unwrap();

        let optimum = point(&result.frontier, 2, zero(), zero());
        assert_eq!(optimum.identification_probability, one());
        match &optimum.policy.nodes[optimum.policy.root] {
            PolicyNode::Action {
                action, on_zero, ..
            } => {
                assert_eq!(world.action_ids[*action], "A-condition");
                match &optimum.policy.nodes[*on_zero] {
                    PolicyNode::Action { action, .. } => {
                        assert_eq!(world.action_ids[*action], "B-measure");
                    }
                    _ => panic!("expected second-stage measurement action"),
                }
            }
            _ => panic!("expected conditioning action at root"),
        }
        assert_evaluator_agreement(&world, &result);
    }

    #[test]
    fn exact_frontier_contains_abstain_risky_and_safe_identification() {
        let world = risk_frontier_world();
        let result = optimize_stateful_world(
            &world,
            &SolverConfig {
                horizon: 1,
                ..SolverConfig::default()
            },
        )
        .unwrap();

        point(&result.frontier, 0, zero(), one());
        let risky = point(&result.frontier, 1, half(), zero());
        let safe = point(&result.frontier, 3, zero(), zero());
        assert_eq!(risky.identification_probability, half());
        assert_eq!(safe.identification_probability, one());
        assert_eq!(result.frontier.len(), 3);

        assert_evaluator_agreement(&world, &result);
    }

    #[test]
    fn explicit_risk_and_abstention_budgets_select_distinct_policies() {
        let world = risk_frontier_world();
        let result = optimize_stateful_world(
            &world,
            &SolverConfig {
                horizon: 1,
                ..SolverConfig::default()
            },
        )
        .unwrap();

        let zero_risk_zero_abstention = result.minimum_cost_under(zero(), zero()).unwrap().unwrap();
        assert_eq!(
            zero_risk_zero_abstention.expected_experiment_cost,
            Rational::from_u64(3)
        );
        assert_eq!(zero_risk_zero_abstention.identification_probability, one());

        let half_risk_zero_abstention = result.minimum_cost_under(half(), zero()).unwrap().unwrap();
        assert_eq!(
            half_risk_zero_abstention.expected_experiment_cost,
            Rational::from_u64(1)
        );
        assert_eq!(
            half_risk_zero_abstention.campaign_hazard_probability,
            half()
        );

        assert!(
            result
                .minimum_cost_under(zero(), Rational::new(1, 2).unwrap())
                .unwrap()
                .is_some()
        );
        assert!(result.minimum_cost_under(zero(), zero()).unwrap().is_some());
    }

    #[test]
    fn repeated_action_can_be_the_only_identifying_experiment_sequence() {
        use Transition::Safe;
        let h0 = sim_rel_dynamics_evaluator::HypothesisMachine {
            initial_state: 0,
            transitions: vec![vec![
                Safe {
                    observation: false,
                    next_state: 1,
                },
                Safe {
                    observation: false,
                    next_state: 1,
                },
            ]],
        };
        let h1 = sim_rel_dynamics_evaluator::HypothesisMachine {
            initial_state: 0,
            transitions: vec![vec![
                Safe {
                    observation: false,
                    next_state: 1,
                },
                Safe {
                    observation: true,
                    next_state: 1,
                },
            ]],
        };
        let world = World {
            name: "repeat-identification".into(),
            hypothesis_ids: vec!["h0".into(), "h1".into()],
            action_ids: vec!["pulse".into()],
            prior_weights: vec![1, 1],
            action_costs: vec![2],
            machines: vec![h0, h1],
        };
        let result = optimize_stateful_world(
            &world,
            &SolverConfig {
                horizon: 2,
                ..SolverConfig::default()
            },
        )
        .unwrap();
        let optimum = point(&result.frontier, 4, zero(), zero());
        assert_eq!(optimum.identification_probability, one());
        match &optimum.policy.nodes[optimum.policy.root] {
            PolicyNode::Action {
                action, on_zero, ..
            } => {
                assert_eq!(world.action_ids[*action], "pulse");
                match &optimum.policy.nodes[*on_zero] {
                    PolicyNode::Action {
                        action: repeated, ..
                    } => {
                        assert_eq!(*repeated, *action);
                    }
                    _ => panic!("expected repeated pulse after the first observation"),
                }
            }
            _ => panic!("expected pulse at policy root"),
        }
        assert_evaluator_agreement(&world, &result);
        let brute = brute_force_frontier(&world, 2);
        let optimizer_candidates: Vec<Candidate> = result
            .frontier
            .iter()
            .map(|candidate| Candidate {
                expected_experiment_cost: candidate.expected_experiment_cost,
                campaign_hazard_probability: candidate.campaign_hazard_probability,
                abstention_probability: candidate.abstention_probability,
                identification_probability: candidate.identification_probability,
                plan: PolicyPlan::Abstain,
            })
            .collect();
        assert_eq!(metric_keys(&optimizer_candidates), metric_keys(&brute));
    }

    #[test]
    fn memoryless_world_does_not_need_dynamic_experiment_history() {
        use Transition::Safe;
        let world = World {
            name: "memoryless".into(),
            hypothesis_ids: vec!["h0".into(), "h1".into()],
            action_ids: vec!["measure".into()],
            prior_weights: vec![3, 1],
            action_costs: vec![2],
            machines: vec![
                sim_rel_dynamics_evaluator::HypothesisMachine {
                    initial_state: 0,
                    transitions: vec![vec![Safe {
                        observation: false,
                        next_state: 0,
                    }]],
                },
                sim_rel_dynamics_evaluator::HypothesisMachine {
                    initial_state: 0,
                    transitions: vec![vec![Safe {
                        observation: true,
                        next_state: 0,
                    }]],
                },
            ],
        };
        let result = optimize_stateful_world(
            &world,
            &SolverConfig {
                horizon: 4,
                ..SolverConfig::default()
            },
        )
        .unwrap();
        let optimum = point(&result.frontier, 2, zero(), zero());
        assert_eq!(optimum.identification_probability, one());
        assert_evaluator_agreement(&world, &result);
    }

    #[test]
    fn input_order_permutations_preserve_semantic_frontier_and_first_action() {
        let world = order_effect_world();
        let baseline = optimize_stateful_world(
            &world,
            &SolverConfig {
                horizon: 2,
                ..SolverConfig::default()
            },
        )
        .unwrap();

        let mut permuted = world.clone();
        permuted.hypothesis_ids.swap(0, 1);
        permuted.prior_weights.swap(0, 1);
        permuted.machines.swap(0, 1);
        permuted.action_ids.swap(0, 1);
        permuted.action_costs.swap(0, 1);
        for machine in &mut permuted.machines {
            machine.transitions.swap(0, 1);
        }
        let reordered = optimize_stateful_world(
            &permuted,
            &SolverConfig {
                horizon: 2,
                ..SolverConfig::default()
            },
        )
        .unwrap();

        assert_eq!(baseline.frontier.len(), reordered.frontier.len());
        for (left, right) in baseline.frontier.iter().zip(reordered.frontier.iter()) {
            assert_eq!(
                left.expected_experiment_cost,
                right.expected_experiment_cost
            );
            assert_eq!(
                left.campaign_hazard_probability,
                right.campaign_hazard_probability
            );
            assert_eq!(left.abstention_probability, right.abstention_probability);
            assert_eq!(
                left.identification_probability,
                right.identification_probability
            );
        }
        let left = baseline.minimum_cost_under(zero(), zero()).unwrap();
        let right = reordered.minimum_cost_under(zero(), zero()).unwrap();
        let left_root = match &left.policy.nodes[left.policy.root] {
            PolicyNode::Action { action, .. } => world.action_ids[*action].as_str(),
            _ => "terminal",
        };
        let right_root = match &right.policy.nodes[right.policy.root] {
            PolicyNode::Action { action, .. } => permuted.action_ids[*action].as_str(),
            _ => "terminal",
        };
        assert_eq!(left_root, right_root);
    }

    #[test]
    fn infeasible_and_invalid_risk_budgets_are_distinguished() {
        use Transition::Hazard;
        let machine = |initial_state| sim_rel_dynamics_evaluator::HypothesisMachine {
            initial_state,
            transitions: vec![vec![Hazard]],
        };
        let world = World {
            name: "all-hazard".into(),
            hypothesis_ids: vec!["h0".into(), "h1".into()],
            action_ids: vec!["unsafe".into()],
            prior_weights: vec![1, 1],
            action_costs: vec![1],
            machines: vec![machine(0), machine(0)],
        };
        let result = optimize_stateful_world(
            &world,
            &SolverConfig {
                horizon: 1,
                ..SolverConfig::default()
            },
        )
        .unwrap();
        assert!(result.minimum_cost_under(zero(), zero()).unwrap().is_none());
        assert!(
            result
                .minimum_cost_under(Rational::new(2, 1).unwrap(), zero())
                .is_err()
        );
    }

    #[test]
    fn invalid_worlds_and_all_solver_caps_fail_closed() {
        let world = order_effect_world();
        let mut duplicate = world.clone();
        duplicate.action_ids[1] = duplicate.action_ids[0].clone();
        assert!(optimize_stateful_world(&duplicate, &SolverConfig::default()).is_err());

        let state_limited = SolverConfig {
            horizon: 2,
            max_states: 1,
            ..SolverConfig::default()
        };
        assert!(
            optimize_stateful_world(&world, &state_limited)
                .unwrap_err()
                .contains("state budget")
        );

        let combinations_limited = SolverConfig {
            horizon: 2,
            max_generated_combinations: 1,
            ..SolverConfig::default()
        };
        assert!(
            optimize_stateful_world(&world, &combinations_limited)
                .unwrap_err()
                .contains("combination budget")
        );

        let zero_frontier = SolverConfig {
            max_frontier_points: 0,
            ..SolverConfig::default()
        };
        assert!(optimize_stateful_world(&world, &zero_frontier).is_err());

        let capped_frontier = SolverConfig {
            horizon: 1,
            max_frontier_points: 2,
            ..SolverConfig::default()
        };
        assert!(
            optimize_stateful_world(&risk_frontier_world(), &capped_frontier)
                .unwrap_err()
                .contains("frontier budget")
        );

        let over_horizon = SolverConfig {
            horizon: MAX_POLICY_STEPS + 1,
            ..SolverConfig::default()
        };
        assert!(optimize_stateful_world(&world, &over_horizon).is_err());
    }

    #[test]
    fn horizon_zero_abstains_when_multiple_hypotheses_remain() {
        let world = order_effect_world();
        let result = optimize_stateful_world(
            &world,
            &SolverConfig {
                horizon: 0,
                ..SolverConfig::default()
            },
        )
        .unwrap();
        assert_eq!(result.frontier.len(), 1);
        assert_eq!(result.frontier[0].expected_experiment_cost, zero());
        assert_eq!(result.frontier[0].campaign_hazard_probability, zero());
        assert_eq!(result.frontier[0].abstention_probability, one());
        assert_eq!(result.frontier[0].identification_probability, zero());
        assert_evaluator_agreement(&world, &result);
    }
}
