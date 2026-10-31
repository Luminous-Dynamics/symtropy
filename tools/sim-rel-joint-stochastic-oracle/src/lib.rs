//! Exact finite-state joint stochastic experimental design.
//!
//! The hidden state is a joint tuple (mechanism, physical state, sensor state).
//! A single explicitly supplied kernel maps that tuple and an action to a
//! probability distribution over the next tuple, observation, and hazard.
//! The implementation never factorizes this kernel or marginalizes the belief
//! before applying an action.

#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub mod baseline;
pub mod robust;

const MAX_HYPOTHESES: usize = 32;
const MAX_PHYSICAL_STATES: usize = 64;
const MAX_SENSOR_STATES: usize = 64;
const MAX_ACTIONS: usize = 16;
const MAX_OBSERVATIONS: usize = 32;
const MAX_KERNEL_ROWS: usize = 1_000_000;
const MAX_OUTCOMES_PER_ROW: usize = 64;
const MAX_TOTAL_OUTCOMES: usize = 4_000_000;
const MAX_INITIAL_BELIEF_TUPLES: usize = 65_536;
const MAX_HORIZON: usize = 12;
const MAX_POLICY_NODES: usize = 65_536;

type JointTupleKey = (usize, usize, usize);
type ObservationMassMap = BTreeMap<usize, BTreeMap<JointTupleKey, Rational>>;

/// A normalized non-negative rational with checked arithmetic.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
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

    pub const fn from_u64(value: u64) -> Self {
        Self {
            numerator: value as u128,
            denominator: 1,
        }
    }

    pub const fn numerator(self) -> u128 {
        self.numerator
    }

    pub const fn denominator(self) -> u128 {
        self.denominator
    }

    pub const fn is_zero(self) -> bool {
        self.numerator == 0
    }

    pub fn is_probability(self) -> bool {
        self <= Self::one()
    }

    pub fn checked_add(self, other: Self) -> Result<Self, String> {
        let common = gcd(self.denominator, other.denominator);
        let left_scale = other.denominator / common;
        let right_scale = self.denominator / common;
        let left = self
            .numerator
            .checked_mul(left_scale)
            .ok_or_else(|| "rational addition numerator overflow".to_string())?;
        let right = other
            .numerator
            .checked_mul(right_scale)
            .ok_or_else(|| "rational addition numerator overflow".to_string())?;
        let numerator = left
            .checked_add(right)
            .ok_or_else(|| "rational addition numerator overflow".to_string())?;
        let denominator = self
            .denominator
            .checked_mul(left_scale)
            .ok_or_else(|| "rational addition denominator overflow".to_string())?;
        Self::new(numerator, denominator)
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, String> {
        let common = gcd(self.denominator, other.denominator);
        let left_scale = other.denominator / common;
        let right_scale = self.denominator / common;
        let left = self
            .numerator
            .checked_mul(left_scale)
            .ok_or_else(|| "rational subtraction numerator overflow".to_string())?;
        let right = other
            .numerator
            .checked_mul(right_scale)
            .ok_or_else(|| "rational subtraction numerator overflow".to_string())?;
        if left < right {
            return Err("rational subtraction would be negative".into());
        }
        let denominator = self
            .denominator
            .checked_mul(left_scale)
            .ok_or_else(|| "rational subtraction denominator overflow".to_string())?;
        Self::new(left - right, denominator)
    }

    pub fn checked_mul(self, other: Self) -> Result<Self, String> {
        let cross_left = gcd(self.numerator, other.denominator);
        let cross_right = gcd(other.numerator, self.denominator);
        let left_num = self.numerator / cross_left;
        let right_num = other.numerator / cross_right;
        let left_den = self.denominator / cross_right;
        let right_den = other.denominator / cross_left;
        let numerator = left_num
            .checked_mul(right_num)
            .ok_or_else(|| "rational multiplication numerator overflow".to_string())?;
        let denominator = left_den
            .checked_mul(right_den)
            .ok_or_else(|| "rational multiplication denominator overflow".to_string())?;
        Self::new(numerator, denominator)
    }

    pub fn checked_div(self, other: Self) -> Result<Self, String> {
        if other.numerator == 0 {
            return Err("rational division by zero".into());
        }
        self.checked_mul(Self::new(other.denominator, other.numerator)?)
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

/// Compare non-negative fractions without cross-multiplication overflow.
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

/// A non-zero-mass point in the initial or posterior joint belief.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct JointMass {
    pub hypothesis: usize,
    pub physical_state: usize,
    pub sensor_state: usize,
    pub mass: Rational,
}

/// One outcome in the explicitly joint transition/observation/hazard kernel.
/// observation=None denotes a terminal hazard; safe outcomes use Some(id).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointOutcome {
    pub probability: Rational,
    pub next_physical_state: usize,
    pub next_sensor_state: usize,
    pub observation: Option<usize>,
}

/// One row for a specific (hypothesis, physical, sensor, action) tuple.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelRow {
    pub hypothesis: usize,
    pub physical_state: usize,
    pub sensor_state: usize,
    pub action: usize,
    pub outcomes: Vec<JointOutcome>,
}

/// Finite stochastic model with a coherent joint latent state.
///
/// Rows use canonical order: hypothesis, physical state, sensor state, action.
/// Every row and the initial joint belief must sum exactly to one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointKernel {
    pub hypothesis_ids: Vec<String>,
    pub physical_state_count: usize,
    pub sensor_state_count: usize,
    pub action_ids: Vec<String>,
    pub observation_ids: Vec<String>,
    pub action_costs: Vec<u64>,
    pub initial_belief: Vec<JointMass>,
    pub rows: Vec<KernelRow>,
}

impl JointKernel {
    pub fn validate(&self) -> Result<(), String> {
        let hypothesis_count = self.hypothesis_ids.len();
        let action_count = self.action_ids.len();
        let observation_count = self.observation_ids.len();

        if hypothesis_count == 0 || hypothesis_count > MAX_HYPOTHESES {
            return Err(format!("hypothesis count must be in 1..={MAX_HYPOTHESES}"));
        }
        if self.physical_state_count == 0 || self.physical_state_count > MAX_PHYSICAL_STATES {
            return Err(format!(
                "physical state count must be in 1..={MAX_PHYSICAL_STATES}"
            ));
        }
        if self.sensor_state_count == 0 || self.sensor_state_count > MAX_SENSOR_STATES {
            return Err(format!(
                "sensor state count must be in 1..={MAX_SENSOR_STATES}"
            ));
        }
        if action_count == 0 || action_count > MAX_ACTIONS {
            return Err(format!("action count must be in 1..={MAX_ACTIONS}"));
        }
        if observation_count == 0 || observation_count > MAX_OBSERVATIONS {
            return Err(format!(
                "observation count must be in 1..={MAX_OBSERVATIONS}"
            ));
        }
        if self.action_costs.len() != action_count || self.action_costs.contains(&0) {
            return Err("every action must have one positive integer cost".into());
        }
        validate_ids(&self.hypothesis_ids, "hypothesis")?;
        validate_ids(&self.action_ids, "action")?;
        validate_ids(&self.observation_ids, "observation")?;

        if self.initial_belief.is_empty() || self.initial_belief.len() > MAX_INITIAL_BELIEF_TUPLES {
            return Err("initial joint belief is empty or exceeds its tuple cap".into());
        }
        let mut initial_mass = Rational::zero();
        let mut initial_tuples = BTreeSet::new();
        for point in &self.initial_belief {
            if point.hypothesis >= hypothesis_count
                || point.physical_state >= self.physical_state_count
                || point.sensor_state >= self.sensor_state_count
                || point.mass.is_zero()
            {
                return Err("initial belief contains an invalid tuple or non-positive mass".into());
            }
            let key = (point.hypothesis, point.physical_state, point.sensor_state);
            if !initial_tuples.insert(key) {
                return Err("initial joint belief contains a duplicate tuple".into());
            }
            initial_mass = initial_mass.checked_add(point.mass)?;
        }
        if initial_mass != Rational::one() {
            return Err("initial joint belief mass must sum exactly to one".into());
        }

        let row_count = hypothesis_count
            .checked_mul(self.physical_state_count)
            .and_then(|n| n.checked_mul(self.sensor_state_count))
            .and_then(|n| n.checked_mul(action_count))
            .ok_or_else(|| "kernel row count overflow".to_string())?;
        if row_count > MAX_KERNEL_ROWS || self.rows.len() != row_count {
            return Err(
                "kernel must contain exactly one row for every joint state/action tuple".into(),
            );
        }

        let mut total_outcomes = 0_usize;
        let mut row_index = 0_usize;
        for hypothesis in 0..hypothesis_count {
            for physical_state in 0..self.physical_state_count {
                for sensor_state in 0..self.sensor_state_count {
                    for action in 0..action_count {
                        let row = &self.rows[row_index];
                        if row.hypothesis != hypothesis
                            || row.physical_state != physical_state
                            || row.sensor_state != sensor_state
                            || row.action != action
                        {
                            return Err(format!(
                                "kernel row {row_index} violates canonical joint-state/action ordering"
                            ));
                        }
                        if row.outcomes.is_empty() || row.outcomes.len() > MAX_OUTCOMES_PER_ROW {
                            return Err(format!(
                                "kernel row {row_index} has an invalid outcome count"
                            ));
                        }
                        total_outcomes = total_outcomes
                            .checked_add(row.outcomes.len())
                            .ok_or_else(|| "kernel outcome count overflow".to_string())?;
                        if total_outcomes > MAX_TOTAL_OUTCOMES {
                            return Err("kernel exceeds the total outcome cap".into());
                        }

                        let mut row_mass = Rational::zero();
                        for outcome in &row.outcomes {
                            if outcome.probability.is_zero()
                                || !outcome.probability.is_probability()
                            {
                                return Err(format!(
                                    "kernel row {row_index} has a non-positive or invalid probability"
                                ));
                            }
                            if outcome.next_physical_state >= self.physical_state_count
                                || outcome.next_sensor_state >= self.sensor_state_count
                            {
                                return Err(format!(
                                    "kernel row {row_index} points outside the joint state space"
                                ));
                            }
                            if outcome
                                .observation
                                .is_some_and(|id| id >= observation_count)
                            {
                                return Err(format!(
                                    "kernel row {row_index} contains an invalid observation ID"
                                ));
                            }
                            row_mass = row_mass.checked_add(outcome.probability)?;
                        }
                        if row_mass != Rational::one() {
                            return Err(format!(
                                "kernel row {row_index} probability mass must sum exactly to one"
                            ));
                        }
                        row_index += 1;
                    }
                }
            }
        }
        Ok(())
    }

    fn row_index(
        &self,
        hypothesis: usize,
        physical_state: usize,
        sensor_state: usize,
        action: usize,
    ) -> usize {
        (((hypothesis * self.physical_state_count + physical_state) * self.sensor_state_count
            + sensor_state)
            * self.action_ids.len())
            + action
    }
}

fn validate_ids(ids: &[String], kind: &str) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if id.trim().is_empty() || !seen.insert(id.as_str()) {
            return Err(format!("{kind} IDs must be non-empty and unique"));
        }
    }
    Ok(())
}

/// A policy tree over public observations only; it never receives a hidden tuple.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum PolicyPlan {
    Claim {
        hypothesis: usize,
    },
    Abstain,
    Action {
        action: usize,
        branches: Vec<PolicyBranch>,
    },
}

/// Continuation selected after a public observation.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PolicyBranch {
    pub observation: usize,
    pub plan: Box<PolicyPlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ObservationBranch {
    observation: usize,
    probability: Rational,
    belief: Vec<JointMass>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Candidate {
    expected_cost: Rational,
    hazard_probability: Rational,
    false_identification_probability: Rational,
    abstention_probability: Rational,
    correct_identification_probability: Rational,
    plan: PolicyPlan,
}

impl Candidate {
    fn same_objectives(&self, other: &Self) -> bool {
        self.expected_cost == other.expected_cost
            && self.hazard_probability == other.hazard_probability
            && self.false_identification_probability == other.false_identification_probability
            && self.abstention_probability == other.abstention_probability
    }
}

fn dominates(left: &Candidate, right: &Candidate) -> bool {
    let no_worse = left.expected_cost <= right.expected_cost
        && left.hazard_probability <= right.hazard_probability
        && left.false_identification_probability <= right.false_identification_probability
        && left.abstention_probability <= right.abstention_probability;
    let strictly_better = left.expected_cost < right.expected_cost
        || left.hazard_probability < right.hazard_probability
        || left.false_identification_probability < right.false_identification_probability
        || left.abstention_probability < right.abstention_probability;
    no_worse && strictly_better
}

fn candidate_order(left: &Candidate, right: &Candidate) -> Ordering {
    (
        left.expected_cost,
        left.hazard_probability,
        left.false_identification_probability,
        left.abstention_probability,
        &left.plan,
    )
        .cmp(&(
            right.expected_cost,
            right.hazard_probability,
            right.false_identification_probability,
            right.abstention_probability,
            &right.plan,
        ))
}

fn prune_candidates(mut candidates: Vec<Candidate>) -> Vec<Candidate> {
    candidates.sort_by(candidate_order);
    let mut frontier: Vec<Candidate> = Vec::new();
    for candidate in candidates {
        if frontier
            .iter()
            .any(|other| other.same_objectives(&candidate) || dominates(other, &candidate))
        {
            continue;
        }
        frontier.retain(|other| !dominates(&candidate, other));
        frontier.push(candidate);
    }
    frontier.sort_by(candidate_order);
    frontier
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PartialAction {
    expected_cost: Rational,
    hazard_probability: Rational,
    false_identification_probability: Rational,
    abstention_probability: Rational,
    correct_identification_probability: Rational,
    branches: Vec<PolicyBranch>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SolverConfig {
    pub horizon: usize,
    pub max_memo_states: usize,
    pub max_generated_combinations: usize,
    pub max_frontier_points: usize,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            horizon: 4,
            max_memo_states: 100_000,
            max_generated_combinations: 1_000_000,
            max_frontier_points: 20_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SolverStats {
    pub expanded_belief_states: usize,
    pub memo_hits: usize,
    pub generated_combinations: usize,
    pub maximum_frontier_points: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontierPoint {
    pub expected_cost: Rational,
    pub hazard_probability: Rational,
    pub false_identification_probability: Rational,
    pub abstention_probability: Rational,
    pub correct_identification_probability: Rational,
    pub plan: PolicyPlan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimizationResult {
    pub frontier: Vec<FrontierPoint>,
    pub stats: SolverStats,
}

impl OptimizationResult {
    /// Select minimum cost under explicit hazard, false-claim, and abstention budgets.
    pub fn minimum_cost_under(
        &self,
        max_hazard: Rational,
        max_false_identification: Rational,
        max_abstention: Rational,
    ) -> Result<Option<&FrontierPoint>, String> {
        for (name, budget) in [
            ("hazard", max_hazard),
            ("false-identification", max_false_identification),
            ("abstention", max_abstention),
        ] {
            if !budget.is_probability() {
                return Err(format!("{name} budget must lie in [0, 1]"));
            }
        }
        Ok(self
            .frontier
            .iter()
            .filter(|point| {
                point.hazard_probability <= max_hazard
                    && point.false_identification_probability <= max_false_identification
                    && point.abstention_probability <= max_abstention
            })
            .min_by(|left, right| {
                (
                    left.expected_cost,
                    left.hazard_probability,
                    left.false_identification_probability,
                    left.abstention_probability,
                    &left.plan,
                )
                    .cmp(&(
                        right.expected_cost,
                        right.hazard_probability,
                        right.false_identification_probability,
                        right.abstention_probability,
                        &right.plan,
                    ))
            }))
    }
}

/// Optimize a finite-horizon policy over the exact joint (h, physical, sensor) belief.
pub fn optimize_joint_kernel(
    kernel: &JointKernel,
    config: &SolverConfig,
) -> Result<OptimizationResult, String> {
    kernel.validate()?;
    if config.horizon > MAX_HORIZON {
        return Err(format!("horizon exceeds the {MAX_HORIZON}-step cap"));
    }
    if config.max_memo_states == 0
        || config.max_generated_combinations == 0
        || config.max_frontier_points == 0
    {
        return Err("all solver resource caps must be positive".into());
    }

    let mut initial_belief = kernel.initial_belief.clone();
    initial_belief.sort();
    let mut solver = Solver {
        kernel,
        config: *config,
        stats: SolverStats::default(),
        memo: HashMap::new(),
    };
    let candidates = solver.solve_belief(&initial_belief, config.horizon)?;
    let frontier = candidates
        .into_iter()
        .map(|candidate| FrontierPoint {
            expected_cost: candidate.expected_cost,
            hazard_probability: candidate.hazard_probability,
            false_identification_probability: candidate.false_identification_probability,
            abstention_probability: candidate.abstention_probability,
            correct_identification_probability: candidate.correct_identification_probability,
            plan: candidate.plan,
        })
        .collect();
    Ok(OptimizationResult {
        frontier,
        stats: solver.stats,
    })
}

struct Solver<'a> {
    kernel: &'a JointKernel,
    config: SolverConfig,
    stats: SolverStats,
    memo: HashMap<(Vec<JointMass>, usize), Vec<Candidate>>,
}

impl Solver<'_> {
    fn solve_belief(
        &mut self,
        belief: &[JointMass],
        remaining: usize,
    ) -> Result<Vec<Candidate>, String> {
        let key = (belief.to_vec(), remaining);
        if let Some(cached) = self.memo.get(&key) {
            self.stats.memo_hits += 1;
            return Ok(cached.clone());
        }
        if self.memo.len() >= self.config.max_memo_states {
            return Err("solver memo-state budget exceeded".into());
        }
        self.stats.expanded_belief_states += 1;

        let mut candidates = terminal_candidates(belief, self.kernel.hypothesis_ids.len())?;
        if remaining > 0 {
            for action in 0..self.kernel.action_ids.len() {
                let (hazard_probability, branches) = self.partition(belief, action)?;
                let mut partials = vec![PartialAction {
                    expected_cost: Rational::from_u64(self.kernel.action_costs[action]),
                    hazard_probability,
                    false_identification_probability: Rational::zero(),
                    abstention_probability: Rational::zero(),
                    correct_identification_probability: Rational::zero(),
                    branches: Vec::new(),
                }];

                for branch in branches {
                    let child_frontier = self.solve_belief(&branch.belief, remaining - 1)?;
                    let mut next_partials = Vec::new();
                    for partial in &partials {
                        for child in &child_frontier {
                            self.stats.generated_combinations = self
                                .stats
                                .generated_combinations
                                .checked_add(1)
                                .ok_or_else(|| {
                                    "generated-combination counter overflow".to_string()
                                })?;
                            if self.stats.generated_combinations
                                > self.config.max_generated_combinations
                            {
                                return Err("solver generated-combination budget exceeded".into());
                            }

                            let mut next = partial.clone();
                            next.expected_cost = next.expected_cost.checked_add(
                                branch.probability.checked_mul(child.expected_cost)?,
                            )?;
                            next.hazard_probability = next.hazard_probability.checked_add(
                                branch.probability.checked_mul(child.hazard_probability)?,
                            )?;
                            next.false_identification_probability =
                                next.false_identification_probability.checked_add(
                                    branch
                                        .probability
                                        .checked_mul(child.false_identification_probability)?,
                                )?;
                            next.abstention_probability = next.abstention_probability.checked_add(
                                branch
                                    .probability
                                    .checked_mul(child.abstention_probability)?,
                            )?;
                            next.correct_identification_probability =
                                next.correct_identification_probability.checked_add(
                                    branch
                                        .probability
                                        .checked_mul(child.correct_identification_probability)?,
                                )?;
                            next.branches.push(PolicyBranch {
                                observation: branch.observation,
                                plan: Box::new(child.plan.clone()),
                            });
                            next_partials.push(next);
                        }
                    }
                    partials = next_partials;
                }

                for partial in partials {
                    validate_probability_partition(
                        partial.hazard_probability,
                        partial.false_identification_probability,
                        partial.abstention_probability,
                        partial.correct_identification_probability,
                    )?;
                    candidates.push(Candidate {
                        expected_cost: partial.expected_cost,
                        hazard_probability: partial.hazard_probability,
                        false_identification_probability: partial.false_identification_probability,
                        abstention_probability: partial.abstention_probability,
                        correct_identification_probability: partial
                            .correct_identification_probability,
                        plan: PolicyPlan::Action {
                            action,
                            branches: partial.branches,
                        },
                    });
                }
            }
        }

        let frontier = prune_candidates(candidates);
        if frontier.len() > self.config.max_frontier_points {
            return Err("solver frontier-point budget exceeded".into());
        }
        self.stats.maximum_frontier_points = self.stats.maximum_frontier_points.max(frontier.len());
        self.memo.insert(key, frontier.clone());
        Ok(frontier)
    }

    fn partition(
        &self,
        belief: &[JointMass],
        action: usize,
    ) -> Result<(Rational, Vec<ObservationBranch>), String> {
        let mut hazard_probability = Rational::zero();
        let mut by_observation: ObservationMassMap = BTreeMap::new();

        for point in belief {
            let row_index = self.kernel.row_index(
                point.hypothesis,
                point.physical_state,
                point.sensor_state,
                action,
            );
            let row = &self.kernel.rows[row_index];
            for outcome in &row.outcomes {
                let path_mass = point.mass.checked_mul(outcome.probability)?;
                match outcome.observation {
                    None => {
                        hazard_probability = hazard_probability.checked_add(path_mass)?;
                    }
                    Some(observation) => {
                        let key = (
                            point.hypothesis,
                            outcome.next_physical_state,
                            outcome.next_sensor_state,
                        );
                        let states = by_observation.entry(observation).or_default();
                        let previous = states.get(&key).copied().unwrap_or(Rational::zero());
                        states.insert(key, previous.checked_add(path_mass)?);
                    }
                }
            }
        }

        let mut branches = Vec::with_capacity(by_observation.len());
        let mut total_mass = hazard_probability;
        for (observation, states) in by_observation {
            let probability = states
                .values()
                .try_fold(Rational::zero(), |sum, mass| sum.checked_add(*mass))?;
            if probability.is_zero() {
                return Err("internal zero-mass observation branch".into());
            }
            let posterior = states
                .into_iter()
                .map(|((hypothesis, physical_state, sensor_state), mass)| {
                    Ok(JointMass {
                        hypothesis,
                        physical_state,
                        sensor_state,
                        mass: mass.checked_div(probability)?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            total_mass = total_mass.checked_add(probability)?;
            branches.push(ObservationBranch {
                observation,
                probability,
                belief: posterior,
            });
        }
        if total_mass != Rational::one() {
            return Err("joint action partition did not conserve probability mass".into());
        }
        Ok((hazard_probability, branches))
    }
}

fn terminal_candidates(
    belief: &[JointMass],
    hypothesis_count: usize,
) -> Result<Vec<Candidate>, String> {
    let mut hypothesis_mass = vec![Rational::zero(); hypothesis_count];
    for point in belief {
        hypothesis_mass[point.hypothesis] =
            hypothesis_mass[point.hypothesis].checked_add(point.mass)?;
    }

    let mut candidates = vec![Candidate {
        expected_cost: Rational::zero(),
        hazard_probability: Rational::zero(),
        false_identification_probability: Rational::zero(),
        abstention_probability: Rational::one(),
        correct_identification_probability: Rational::zero(),
        plan: PolicyPlan::Abstain,
    }];
    for (hypothesis, probability) in hypothesis_mass.into_iter().enumerate() {
        if probability.is_zero() {
            continue;
        }
        candidates.push(Candidate {
            expected_cost: Rational::zero(),
            hazard_probability: Rational::zero(),
            false_identification_probability: Rational::one().checked_sub(probability)?,
            abstention_probability: Rational::zero(),
            correct_identification_probability: probability,
            plan: PolicyPlan::Claim { hypothesis },
        });
    }
    Ok(candidates)
}

fn validate_probability_partition(
    hazard: Rational,
    false_identification: Rational,
    abstention: Rational,
    correct_identification: Rational,
) -> Result<(), String> {
    let total = hazard
        .checked_add(false_identification)?
        .checked_add(abstention)?
        .checked_add(correct_identification)?;
    if total != Rational::one() {
        return Err(
            "hazard, false claim, abstention, and correct claim must sum exactly to one".into(),
        );
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Accumulator {
    expected_cost: Rational,
    hazard: Rational,
    false_identification: Rational,
    abstention: Rational,
    correct_identification: Rational,
}

impl Default for Accumulator {
    fn default() -> Self {
        Self {
            expected_cost: Rational::zero(),
            hazard: Rational::zero(),
            false_identification: Rational::zero(),
            abstention: Rational::zero(),
            correct_identification: Rational::zero(),
        }
    }
}

/// Independently replay a policy from every initial joint-state possibility.
pub fn evaluate_policy(
    kernel: &JointKernel,
    plan: &PolicyPlan,
    max_steps: usize,
) -> Result<FrontierPoint, String> {
    kernel.validate()?;
    if max_steps > MAX_HORIZON {
        return Err(format!("policy horizon exceeds the {MAX_HORIZON}-step cap"));
    }
    let mut node_count = 0_usize;
    validate_plan_shape(plan, kernel, 0, max_steps, &mut node_count)?;
    let mut accumulator = Accumulator::default();
    for point in &kernel.initial_belief {
        replay_from(
            kernel,
            plan,
            point.hypothesis,
            point.physical_state,
            point.sensor_state,
            point.mass,
            0,
            max_steps,
            &mut accumulator,
        )?;
    }
    validate_probability_partition(
        accumulator.hazard,
        accumulator.false_identification,
        accumulator.abstention,
        accumulator.correct_identification,
    )?;
    Ok(FrontierPoint {
        expected_cost: accumulator.expected_cost,
        hazard_probability: accumulator.hazard,
        false_identification_probability: accumulator.false_identification,
        abstention_probability: accumulator.abstention,
        correct_identification_probability: accumulator.correct_identification,
        plan: plan.clone(),
    })
}

fn validate_plan_shape(
    plan: &PolicyPlan,
    kernel: &JointKernel,
    depth: usize,
    max_steps: usize,
    node_count: &mut usize,
) -> Result<(), String> {
    *node_count = node_count
        .checked_add(1)
        .ok_or_else(|| "policy node count overflow".to_string())?;
    if *node_count > MAX_POLICY_NODES {
        return Err("policy exceeds the node cap".into());
    }
    match plan {
        PolicyPlan::Claim { hypothesis } => {
            if *hypothesis >= kernel.hypothesis_ids.len() {
                return Err("policy claims an invalid hypothesis".into());
            }
        }
        PolicyPlan::Abstain => {}
        PolicyPlan::Action { action, branches } => {
            if depth >= max_steps || *action >= kernel.action_ids.len() {
                return Err("policy action exceeds its horizon or uses an invalid action".into());
            }
            let mut observations = BTreeSet::new();
            for branch in branches {
                if branch.observation >= kernel.observation_ids.len()
                    || !observations.insert(branch.observation)
                {
                    return Err("policy contains an invalid or duplicate observation branch".into());
                }
                validate_plan_shape(&branch.plan, kernel, depth + 1, max_steps, node_count)?;
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn replay_from(
    kernel: &JointKernel,
    plan: &PolicyPlan,
    hypothesis: usize,
    physical_state: usize,
    sensor_state: usize,
    mass: Rational,
    depth: usize,
    max_steps: usize,
    accumulator: &mut Accumulator,
) -> Result<(), String> {
    match plan {
        PolicyPlan::Claim {
            hypothesis: claimed,
        } => {
            if *claimed == hypothesis {
                accumulator.correct_identification =
                    accumulator.correct_identification.checked_add(mass)?;
            } else {
                accumulator.false_identification =
                    accumulator.false_identification.checked_add(mass)?;
            }
        }
        PolicyPlan::Abstain => {
            accumulator.abstention = accumulator.abstention.checked_add(mass)?;
        }
        PolicyPlan::Action { action, branches } => {
            if depth >= max_steps {
                return Err("policy replay exceeded the declared action horizon".into());
            }
            let cost = Rational::from_u64(kernel.action_costs[*action]);
            accumulator.expected_cost = accumulator
                .expected_cost
                .checked_add(mass.checked_mul(cost)?)?;
            let row_index = kernel.row_index(hypothesis, physical_state, sensor_state, *action);
            let row = &kernel.rows[row_index];
            for outcome in &row.outcomes {
                let path_mass = mass.checked_mul(outcome.probability)?;
                match outcome.observation {
                    None => {
                        accumulator.hazard = accumulator.hazard.checked_add(path_mass)?;
                    }
                    Some(observation) => {
                        let branch = branches
                            .iter()
                            .find(|branch| branch.observation == observation)
                            .ok_or_else(|| {
                                format!("policy has no continuation for reachable observation {observation}")
                            })?;
                        replay_from(
                            kernel,
                            &branch.plan,
                            hypothesis,
                            outcome.next_physical_state,
                            outcome.next_sensor_state,
                            path_mass,
                            depth + 1,
                            max_steps,
                            accumulator,
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn half() -> Rational {
        Rational::new(1, 2).unwrap()
    }

    fn three_quarters() -> Rational {
        Rational::new(3, 4).unwrap()
    }

    fn quarter() -> Rational {
        Rational::new(1, 4).unwrap()
    }

    fn safe(
        probability: Rational,
        physical: usize,
        sensor: usize,
        observation: usize,
    ) -> JointOutcome {
        JointOutcome {
            probability,
            next_physical_state: physical,
            next_sensor_state: sensor,
            observation: Some(observation),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn base_kernel(
        hypothesis_count: usize,
        physical_count: usize,
        sensor_count: usize,
        action_ids: Vec<&str>,
        observation_ids: Vec<&str>,
        action_costs: Vec<u64>,
        initial_belief: Vec<JointMass>,
        mut make_outcomes: impl FnMut(usize, usize, usize, usize) -> Vec<JointOutcome>,
    ) -> JointKernel {
        let mut rows = Vec::new();
        for hypothesis in 0..hypothesis_count {
            for physical_state in 0..physical_count {
                for sensor_state in 0..sensor_count {
                    for action in 0..action_ids.len() {
                        rows.push(KernelRow {
                            hypothesis,
                            physical_state,
                            sensor_state,
                            action,
                            outcomes: make_outcomes(
                                hypothesis,
                                physical_state,
                                sensor_state,
                                action,
                            ),
                        });
                    }
                }
            }
        }
        JointKernel {
            hypothesis_ids: (0..hypothesis_count).map(|id| format!("h{id}")).collect(),
            physical_state_count: physical_count,
            sensor_state_count: sensor_count,
            action_ids: action_ids.into_iter().map(str::to_string).collect(),
            observation_ids: observation_ids.into_iter().map(str::to_string).collect(),
            action_costs,
            initial_belief,
            rows,
        }
    }

    fn correlated_state_kernel() -> JointKernel {
        base_kernel(
            2,
            2,
            2,
            vec!["read-physical-state"],
            vec!["zero", "one"],
            vec![1],
            vec![
                JointMass {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 0,
                    mass: half(),
                },
                JointMass {
                    hypothesis: 1,
                    physical_state: 1,
                    sensor_state: 1,
                    mass: half(),
                },
            ],
            |_h, physical, sensor, _action| vec![safe(Rational::one(), physical, sensor, physical)],
        )
    }

    fn calibration_kernel(include_calibration: bool) -> JointKernel {
        let action_ids = if include_calibration {
            vec!["measure-combined-signal", "calibrate-sensor"]
        } else {
            vec!["measure-combined-signal"]
        };
        let action_costs = if include_calibration {
            vec![1, 1]
        } else {
            vec![1]
        };
        base_kernel(
            2,
            2,
            2,
            action_ids,
            vec!["zero", "one"],
            action_costs,
            vec![
                JointMass {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 1,
                    mass: half(),
                },
                JointMass {
                    hypothesis: 1,
                    physical_state: 1,
                    sensor_state: 0,
                    mass: half(),
                },
            ],
            |_h, physical, sensor, action| {
                if action == 0 {
                    // Persistent sensor bias and specimen state are confounded.
                    vec![safe(Rational::one(), physical, sensor, physical ^ sensor)]
                } else {
                    // Calibration changes sensor state but not physical state.
                    vec![safe(Rational::one(), physical, 0, 0)]
                }
            },
        )
    }

    fn noisy_binary_kernel() -> JointKernel {
        base_kernel(
            2,
            1,
            1,
            vec!["noisy-measurement"],
            vec!["zero", "one"],
            vec![1],
            vec![
                JointMass {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 0,
                    mass: half(),
                },
                JointMass {
                    hypothesis: 1,
                    physical_state: 0,
                    sensor_state: 0,
                    mass: half(),
                },
            ],
            |hypothesis, physical, sensor, _action| {
                let (p_zero, p_one) = if hypothesis == 0 {
                    (three_quarters(), quarter())
                } else {
                    (quarter(), three_quarters())
                };
                vec![
                    safe(p_zero, physical, sensor, 0),
                    safe(p_one, physical, sensor, 1),
                ]
            },
        )
    }

    fn assert_partition(point: &FrontierPoint) {
        validate_probability_partition(
            point.hazard_probability,
            point.false_identification_probability,
            point.abstention_probability,
            point.correct_identification_probability,
        )
        .unwrap();
    }

    #[test]
    fn rational_arithmetic_is_exact_and_comparison_does_not_cross_multiply() {
        assert_eq!(half().checked_add(half()).unwrap(), Rational::one());
        assert_eq!(three_quarters().checked_sub(quarter()).unwrap(), half());
        assert_eq!(half().checked_mul(half()).unwrap(), quarter());
        assert_eq!(
            half().checked_div(quarter()).unwrap(),
            Rational::from_u64(2)
        );
        let large = Rational::new(u128::MAX - 1, u128::MAX).unwrap();
        assert!(large > Rational::new(u128::MAX - 2, u128::MAX).unwrap());
        assert!(Rational::one().checked_add(Rational::one()).unwrap() > Rational::one());
    }

    #[test]
    fn joint_belief_preserves_mechanism_physical_sensor_correlation() {
        let kernel = correlated_state_kernel();
        kernel.validate().unwrap();
        let policy = PolicyPlan::Action {
            action: 0,
            branches: vec![
                PolicyBranch {
                    observation: 0,
                    plan: Box::new(PolicyPlan::Claim { hypothesis: 0 }),
                },
                PolicyBranch {
                    observation: 1,
                    plan: Box::new(PolicyPlan::Claim { hypothesis: 1 }),
                },
            ],
        };
        let report = evaluate_policy(&kernel, &policy, 1).unwrap();
        assert_eq!(report.expected_cost, Rational::one());
        assert_eq!(report.correct_identification_probability, Rational::one());
        assert_eq!(report.false_identification_probability, Rational::zero());
        assert_eq!(report.abstention_probability, Rational::zero());
        assert_partition(&report);

        let optimized = optimize_joint_kernel(
            &kernel,
            &SolverConfig {
                horizon: 1,
                ..SolverConfig::default()
            },
        )
        .unwrap();
        let selected = optimized
            .minimum_cost_under(Rational::zero(), Rational::zero(), Rational::zero())
            .unwrap()
            .unwrap();
        assert_eq!(selected.correct_identification_probability, Rational::one());
        assert_eq!(selected.expected_cost, Rational::one());
    }

    #[test]
    fn calibration_then_measurement_resolves_persistent_sensor_physical_confounding() {
        let kernel = calibration_kernel(true);
        let result = optimize_joint_kernel(
            &kernel,
            &SolverConfig {
                horizon: 2,
                ..SolverConfig::default()
            },
        )
        .unwrap();
        let selected = result
            .minimum_cost_under(Rational::zero(), Rational::zero(), Rational::zero())
            .unwrap()
            .unwrap();
        assert_eq!(selected.expected_cost, Rational::from_u64(2));
        assert_eq!(selected.hazard_probability, Rational::zero());
        assert_eq!(selected.false_identification_probability, Rational::zero());
        assert_eq!(selected.abstention_probability, Rational::zero());
        assert_eq!(selected.correct_identification_probability, Rational::one());

        match &selected.plan {
            PolicyPlan::Action { action, branches } => {
                assert_eq!(kernel.action_ids[*action], "calibrate-sensor");
                assert_eq!(branches.len(), 1);
                match branches[0].plan.as_ref() {
                    PolicyPlan::Action { action: next, .. } => {
                        assert_eq!(kernel.action_ids[*next], "measure-combined-signal");
                    }
                    _ => panic!("expected a measurement after calibration"),
                }
            }
            _ => panic!("expected calibration as the first action"),
        }
        let replayed = evaluate_policy(&kernel, &selected.plan, 2).unwrap();
        assert_eq!(replayed, *selected);
    }

    #[test]
    fn repeated_measurements_do_not_turn_persistent_sensor_bias_into_iid_evidence() {
        let kernel = calibration_kernel(false);
        let result = optimize_joint_kernel(
            &kernel,
            &SolverConfig {
                horizon: 4,
                ..SolverConfig::default()
            },
        )
        .unwrap();
        let selected = result
            .minimum_cost_under(Rational::zero(), Rational::zero(), Rational::one())
            .unwrap()
            .unwrap();
        assert_eq!(selected.plan, PolicyPlan::Abstain);
        assert_eq!(selected.expected_cost, Rational::zero());
        assert_eq!(selected.abstention_probability, Rational::one());
        assert_eq!(
            selected.correct_identification_probability,
            Rational::zero()
        );
    }

    #[test]
    fn noisy_stochastic_observations_conserve_mass_and_replay_exactly() {
        let kernel = noisy_binary_kernel();
        let result = optimize_joint_kernel(
            &kernel,
            &SolverConfig {
                horizon: 2,
                ..SolverConfig::default()
            },
        )
        .unwrap();
        assert!(!result.frontier.is_empty());
        for point in &result.frontier {
            assert_partition(point);
            let replay = evaluate_policy(&kernel, &point.plan, 2).unwrap();
            assert_eq!(replay.expected_cost, point.expected_cost);
            assert_eq!(replay.hazard_probability, point.hazard_probability);
            assert_eq!(
                replay.false_identification_probability,
                point.false_identification_probability
            );
            assert_eq!(replay.abstention_probability, point.abstention_probability);
            assert_eq!(
                replay.correct_identification_probability,
                point.correct_identification_probability
            );
        }
    }

    #[test]
    fn malformed_probability_rows_and_missing_kernel_rows_fail_closed() {
        let mut bad_mass = noisy_binary_kernel();
        bad_mass.rows[0].outcomes[0].probability = half();
        assert!(
            bad_mass
                .validate()
                .unwrap_err()
                .contains("mass must sum exactly to one")
        );

        let mut missing_row = noisy_binary_kernel();
        missing_row.rows.pop();
        assert!(missing_row.validate().is_err());

        let mut bad_prior = noisy_binary_kernel();
        bad_prior.initial_belief[0].mass = quarter();
        assert!(
            bad_prior
                .validate()
                .unwrap_err()
                .contains("initial joint belief mass")
        );
    }

    #[test]
    fn action_cost_is_charged_even_when_the_transition_is_hazardous() {
        let kernel = base_kernel(
            1,
            1,
            1,
            vec!["dangerous-test"],
            vec!["irrelevant"],
            vec![7],
            vec![JointMass {
                hypothesis: 0,
                physical_state: 0,
                sensor_state: 0,
                mass: Rational::one(),
            }],
            |_h, _physical, _sensor, _action| {
                vec![JointOutcome {
                    probability: Rational::one(),
                    next_physical_state: 0,
                    next_sensor_state: 0,
                    observation: None,
                }]
            },
        );
        let policy = PolicyPlan::Action {
            action: 0,
            branches: Vec::new(),
        };
        let report = evaluate_policy(&kernel, &policy, 1).unwrap();
        assert_eq!(report.expected_cost, Rational::from_u64(7));
        assert_eq!(report.hazard_probability, Rational::one());
        assert_eq!(report.correct_identification_probability, Rational::zero());
        assert_partition(&report);
    }

    fn brute_observation_branches(
        kernel: &JointKernel,
        belief: &[JointMass],
        action: usize,
    ) -> Vec<(usize, Vec<JointMass>)> {
        let mut groups: ObservationMassMap = BTreeMap::new();
        for point in belief {
            let index = (((point.hypothesis * kernel.physical_state_count + point.physical_state)
                * kernel.sensor_state_count
                + point.sensor_state)
                * kernel.action_ids.len())
                + action;
            for outcome in &kernel.rows[index].outcomes {
                if let Some(observation) = outcome.observation {
                    let weighted = point.mass.checked_mul(outcome.probability).unwrap();
                    let key = (
                        point.hypothesis,
                        outcome.next_physical_state,
                        outcome.next_sensor_state,
                    );
                    let row = groups.entry(observation).or_default();
                    let old = row.get(&key).copied().unwrap_or(Rational::zero());
                    row.insert(key, old.checked_add(weighted).unwrap());
                }
            }
        }
        groups
            .into_iter()
            .map(|(observation, states)| {
                let probability = states
                    .values()
                    .copied()
                    .fold(Rational::zero(), |sum, value| {
                        sum.checked_add(value).unwrap()
                    });
                let posterior = states
                    .into_iter()
                    .map(
                        |((hypothesis, physical_state, sensor_state), mass)| JointMass {
                            hypothesis,
                            physical_state,
                            sensor_state,
                            mass: mass.checked_div(probability).unwrap(),
                        },
                    )
                    .collect();
                (observation, posterior)
            })
            .collect()
    }

    fn enumerate_all_policies(
        kernel: &JointKernel,
        belief: &[JointMass],
        remaining: usize,
    ) -> Vec<PolicyPlan> {
        let mut plans = vec![PolicyPlan::Abstain];
        for hypothesis in 0..kernel.hypothesis_ids.len() {
            if belief
                .iter()
                .any(|point| point.hypothesis == hypothesis && !point.mass.is_zero())
            {
                plans.push(PolicyPlan::Claim { hypothesis });
            }
        }
        if remaining == 0 {
            return plans;
        }
        for action in 0..kernel.action_ids.len() {
            let observation_branches = brute_observation_branches(kernel, belief, action);
            let mut combinations: Vec<Vec<PolicyBranch>> = vec![Vec::new()];
            for (observation, posterior) in observation_branches {
                let child_plans = enumerate_all_policies(kernel, &posterior, remaining - 1);
                let mut expanded = Vec::new();
                for prefix in &combinations {
                    for child in &child_plans {
                        let mut branches = prefix.clone();
                        branches.push(PolicyBranch {
                            observation,
                            plan: Box::new(child.clone()),
                        });
                        expanded.push(branches);
                    }
                }
                combinations = expanded;
            }
            for branches in combinations {
                plans.push(PolicyPlan::Action { action, branches });
            }
        }
        plans
    }

    fn brute_frontier(mut points: Vec<FrontierPoint>) -> Vec<FrontierPoint> {
        points.sort_by(|left, right| {
            (
                left.expected_cost,
                left.hazard_probability,
                left.false_identification_probability,
                left.abstention_probability,
                &left.plan,
            )
                .cmp(&(
                    right.expected_cost,
                    right.hazard_probability,
                    right.false_identification_probability,
                    right.abstention_probability,
                    &right.plan,
                ))
        });
        let mut frontier: Vec<FrontierPoint> = Vec::new();
        for point in points {
            let dominated = frontier.iter().any(|other| {
                let no_worse = other.expected_cost <= point.expected_cost
                    && other.hazard_probability <= point.hazard_probability
                    && other.false_identification_probability
                        <= point.false_identification_probability
                    && other.abstention_probability <= point.abstention_probability;
                let strict = other.expected_cost < point.expected_cost
                    || other.hazard_probability < point.hazard_probability
                    || other.false_identification_probability
                        < point.false_identification_probability
                    || other.abstention_probability < point.abstention_probability;
                no_worse && strict
                    || (other.expected_cost == point.expected_cost
                        && other.hazard_probability == point.hazard_probability
                        && other.false_identification_probability
                            == point.false_identification_probability
                        && other.abstention_probability == point.abstention_probability)
            });
            if !dominated {
                frontier.push(point);
            }
        }
        frontier
    }

    type ObjectiveKey = (Rational, Rational, Rational, Rational, Rational);

    fn objective_keys(points: &[FrontierPoint]) -> Vec<ObjectiveKey> {
        let mut keys: Vec<_> = points
            .iter()
            .map(|point| {
                (
                    point.expected_cost,
                    point.hazard_probability,
                    point.false_identification_probability,
                    point.abstention_probability,
                    point.correct_identification_probability,
                )
            })
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    #[test]
    fn dynamic_program_matches_independent_exhaustive_policy_enumeration() {
        let kernel = noisy_binary_kernel();
        let config = SolverConfig {
            horizon: 2,
            ..SolverConfig::default()
        };
        let optimized = optimize_joint_kernel(&kernel, &config).unwrap();
        let mut initial = kernel.initial_belief.clone();
        initial.sort();
        let all_policies = enumerate_all_policies(&kernel, &initial, config.horizon);
        let evaluated = all_policies
            .iter()
            .map(|policy| evaluate_policy(&kernel, policy, config.horizon).unwrap())
            .collect();
        let brute = brute_frontier(evaluated);
        assert_eq!(
            objective_keys(&optimized.frontier),
            objective_keys(&brute),
            "DP frontier must match independently enumerated and replayed policies"
        );
    }

    #[test]
    fn invalid_budgets_and_solver_resource_caps_are_rejected() {
        let kernel = noisy_binary_kernel();
        let result = optimize_joint_kernel(
            &kernel,
            &SolverConfig {
                horizon: 1,
                ..SolverConfig::default()
            },
        )
        .unwrap();
        assert!(
            result
                .minimum_cost_under(
                    Rational::new(2, 1).unwrap(),
                    Rational::zero(),
                    Rational::zero(),
                )
                .is_err()
        );
        assert!(
            optimize_joint_kernel(
                &kernel,
                &SolverConfig {
                    horizon: MAX_HORIZON + 1,
                    ..SolverConfig::default()
                },
            )
            .is_err()
        );
        assert!(
            optimize_joint_kernel(
                &kernel,
                &SolverConfig {
                    max_generated_combinations: 0,
                    ..SolverConfig::default()
                },
            )
            .is_err()
        );
        assert!(
            optimize_joint_kernel(
                &kernel,
                &SolverConfig {
                    max_memo_states: 1,
                    horizon: 2,
                    ..SolverConfig::default()
                },
            )
            .is_err()
        );
    }

    #[test]
    fn one_step_eig_uses_joint_predictive_model_but_cannot_value_a_purely_enabling_calibration() {
        let kernel = calibration_kernel(true);
        let scores =
            super::baseline::rank_by_one_step_eig(&kernel, &kernel.initial_belief).unwrap();
        assert_eq!(scores.len(), 2);
        assert!(
            scores
                .iter()
                .all(|score| score.expected_information_gain_nats.abs() < 1e-12)
        );

        let exact = optimize_joint_kernel(
            &kernel,
            &SolverConfig {
                horizon: 2,
                ..SolverConfig::default()
            },
        )
        .unwrap();
        let selected = exact
            .minimum_cost_under(Rational::zero(), Rational::zero(), Rational::zero())
            .unwrap()
            .unwrap();
        match &selected.plan {
            PolicyPlan::Action { action, .. } => {
                assert_eq!(kernel.action_ids[*action], "calibrate-sensor");
            }
            _ => panic!("expected a multi-step calibration policy"),
        }
    }

    #[test]
    fn one_step_eig_matches_the_analytic_binary_channel_value() {
        let kernel = noisy_binary_kernel();
        let scores =
            super::baseline::rank_by_one_step_eig(&kernel, &kernel.initial_belief).unwrap();
        let expected = std::f64::consts::LN_2 + 0.75 * 0.75_f64.ln() + 0.25 * 0.25_f64.ln();
        assert_eq!(scores.len(), 1);
        assert!((scores[0].expected_information_gain_nats - expected).abs() < 1e-12);
        assert!((scores[0].information_gain_per_cost - expected).abs() < 1e-12);
        assert_eq!(scores[0].hazard_probability, Rational::zero());
    }
}
