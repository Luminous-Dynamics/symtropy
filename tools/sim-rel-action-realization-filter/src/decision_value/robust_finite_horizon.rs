//! Exact shared-policy decision optimization over persistent stochastic worlds.
//!
//! Every world carries its own complete joint belief and transition kernel.
//! One public-observation policy is shared across the family. World identity is
//! never exposed to the policy, and a world that is impossible on one branch is
//! inactive only on that branch; it is not removed from other histories.

use super::finite_horizon::{DecisionBranch, DecisionBudgets, DecisionPlan, DecisionSolverConfig};
use super::{DecisionLossTable, ObservationProtocol};
use crate::{
    filter_one_step, ActionRealizationKernel, BeliefMass, LatentState, ObservationField,
    PublicObservation, Rational, TransitionOutcome,
};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

const MAX_MODEL_WORLDS: usize = 16;

#[derive(Clone, Debug)]
pub struct DecisionModelWorld {
    pub id: String,
    pub kernel: ActionRealizationKernel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct WorldObjectives {
    expected_experiment_cost: Rational,
    expected_terminal_loss: Rational,
    hazard_probability: Rational,
    abstention_probability: Rational,
}

impl WorldObjectives {
    fn zero() -> Self {
        Self {
            expected_experiment_cost: Rational::zero(),
            expected_terminal_loss: Rational::zero(),
            hazard_probability: Rational::zero(),
            abstention_probability: Rational::zero(),
        }
    }

    fn hazard_terminal(hazard_loss: Rational) -> Self {
        Self {
            expected_terminal_loss: hazard_loss,
            hazard_probability: Rational::one(),
            ..Self::zero()
        }
    }

    fn total_loss(self) -> Result<Rational, String> {
        self.expected_experiment_cost
            .checked_add(self.expected_terminal_loss)
    }

    fn add_scaled(&mut self, other: Self, probability: Rational) -> Result<(), String> {
        self.expected_experiment_cost = self
            .expected_experiment_cost
            .checked_add(probability.checked_mul(other.expected_experiment_cost)?)?;
        self.expected_terminal_loss = self
            .expected_terminal_loss
            .checked_add(probability.checked_mul(other.expected_terminal_loss)?)?;
        self.hazard_probability = self
            .hazard_probability
            .checked_add(probability.checked_mul(other.hazard_probability)?)?;
        self.abstention_probability = self
            .abstention_probability
            .checked_add(probability.checked_mul(other.abstention_probability)?)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Candidate {
    per_world: Vec<WorldObjectives>,
    plan: DecisionPlan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PartialCandidate {
    per_world: Vec<WorldObjectives>,
    branches: Vec<DecisionBranch>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ObservationBranch {
    probability: Rational,
    next_belief: Option<Vec<BeliefMass>>,
}

type BeliefVector = Vec<Option<Vec<BeliefMass>>>;
type WorldPartition = BTreeMap<PublicObservation, ObservationBranch>;
type RawStateMassMap = BTreeMap<LatentState, Rational>;
type RawObservationMassMap = BTreeMap<PublicObservation, (Rational, RawStateMassMap)>;
type ReplayPartition = BTreeMap<PublicObservation, ReplayBranch>;

#[derive(Clone, Debug, Eq, PartialEq)]
struct RawPathMass {
    state: LatentState,
    mass: Rational,
}

type RawPathMassVector = Vec<Option<Vec<RawPathMass>>>;

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReplayBranch {
    probability: Rational,
    next_mass: Option<Vec<RawPathMass>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct BeliefVectorKey(Vec<Option<Vec<(LatentState, Rational)>>>);

impl BeliefVectorKey {
    fn new(beliefs: &BeliefVector) -> Self {
        let worlds = beliefs
            .iter()
            .map(|belief| {
                belief.as_ref().map(|points| {
                    let mut state_masses: Vec<_> =
                        points.iter().map(|point| (point.state, point.mass)).collect();
                    state_masses.sort();
                    state_masses
                })
            })
            .collect();
        Self(worlds)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct MemoKey {
    beliefs: BeliefVectorKey,
    remaining_horizon: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RobustDecisionSolverStats {
    pub memo_states: usize,
    pub generated_combinations: usize,
    pub maximum_frontier_points: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldDecisionObjectives {
    pub world_id: String,
    pub expected_experiment_cost: Rational,
    pub expected_terminal_loss: Rational,
    pub hazard_probability: Rational,
    pub abstention_probability: Rational,
}

impl WorldDecisionObjectives {
    pub fn expected_total_loss(&self) -> Result<Rational, String> {
        self.expected_experiment_cost
            .checked_add(self.expected_terminal_loss)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RobustDecisionFrontierPoint {
    /// Objective vector in canonical, lexicographically sorted world-ID order.
    pub per_world: Vec<WorldDecisionObjectives>,
    /// One policy tree shared by every model world.
    pub plan: DecisionPlan,
}

impl RobustDecisionFrontierPoint {
    pub fn worst_case_total_loss(&self) -> Result<Rational, String> {
        let mut worst = Rational::zero();
        for world in &self.per_world {
            let value = world.expected_total_loss()?;
            if value > worst {
                worst = value;
            }
        }
        Ok(worst)
    }

    pub fn worst_case_terminal_loss(&self) -> Rational {
        self.per_world
            .iter()
            .map(|world| world.expected_terminal_loss)
            .max()
            .unwrap_or_else(Rational::zero)
    }

    pub fn worst_case_hazard_probability(&self) -> Rational {
        self.per_world
            .iter()
            .map(|world| world.hazard_probability)
            .max()
            .unwrap_or_else(Rational::zero)
    }

    pub fn worst_case_abstention_probability(&self) -> Rational {
        self.per_world
            .iter()
            .map(|world| world.abstention_probability)
            .max()
            .unwrap_or_else(Rational::zero)
    }

    /// Worst-case expected total loss over a total-variation ball around a
    /// nominal categorical distribution over the canonical world ordering.
    ///
    /// Radius zero is the nominal mixture expectation; radius one is the
    /// maximum loss over supplied worlds.
    pub fn worst_case_total_loss_under_tv(
        &self,
        nominal_world_weights: &[Rational],
        radius: Rational,
    ) -> Result<Rational, String> {
        let losses = self
            .per_world
            .iter()
            .map(WorldDecisionObjectives::expected_total_loss)
            .collect::<Result<Vec<_>, _>>()?;
        worst_case_tv_expectation(&losses, nominal_world_weights, radius)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RobustDecisionOptimizationResult {
    pub world_ids: Vec<String>,
    pub nominal_world_id: String,
    pub frontier: Vec<RobustDecisionFrontierPoint>,
    pub stats: RobustDecisionSolverStats,
}

impl RobustDecisionOptimizationResult {
    /// Minimize worst-case expected experiment cost plus downstream loss.
    pub fn minimum_worst_case_total_loss_under(
        &self,
        budgets: DecisionBudgets,
    ) -> Result<Option<&RobustDecisionFrontierPoint>, String> {
        budgets.validate()?;
        let mut selected: Option<&RobustDecisionFrontierPoint> = None;
        for point in &self.frontier {
            if !within_risk_budgets(point, budgets) {
                continue;
            }
            let Some(current) = selected else {
                selected = Some(point);
                continue;
            };
            let order = point
                .worst_case_total_loss()?
                .cmp(&current.worst_case_total_loss()?)
                .then_with(|| {
                    point
                        .worst_case_hazard_probability()
                        .cmp(&current.worst_case_hazard_probability())
                })
                .then_with(|| {
                    point
                        .worst_case_abstention_probability()
                        .cmp(&current.worst_case_abstention_probability())
                })
                .then_with(|| point.plan.cmp(&current.plan));
            if order == Ordering::Less {
                selected = Some(point);
            }
        }
        Ok(selected)
    }

    /// Minimize nominal expected cost plus loss under worst-case risk budgets.
    ///
    /// An optional worst-case terminal-loss ceiling makes the nominal objective
    /// explicit without allowing a nominally good policy to hide catastrophic
    /// downstream loss in another supplied world.
    pub fn minimum_nominal_total_loss_under(
        &self,
        budgets: DecisionBudgets,
        max_worst_case_terminal_loss: Option<Rational>,
    ) -> Result<Option<&RobustDecisionFrontierPoint>, String> {
        budgets.validate()?;
        let nominal_index = self
            .world_ids
            .iter()
            .position(|world_id| world_id == &self.nominal_world_id)
            .ok_or_else(|| "nominal world is absent from the robust result".to_string())?;
        let mut selected: Option<&RobustDecisionFrontierPoint> = None;
        for point in &self.frontier {
            if !within_risk_budgets(point, budgets)
                || max_worst_case_terminal_loss
                    .is_some_and(|limit| point.worst_case_terminal_loss() > limit)
            {
                continue;
            }
            let Some(current) = selected else {
                selected = Some(point);
                continue;
            };
            let nominal_order = point.per_world[nominal_index]
                .expected_total_loss()?
                .cmp(&current.per_world[nominal_index].expected_total_loss()?);
            let order = if nominal_order == Ordering::Equal {
                point
                    .worst_case_total_loss()?
                    .cmp(&current.worst_case_total_loss()?)
                    .then_with(|| point.plan.cmp(&current.plan))
            } else {
                nominal_order
            };
            if order == Ordering::Less {
                selected = Some(point);
            }
        }
        Ok(selected)
    }

    /// Minimize expected total loss over a finite-support TV ambiguity set.
    ///
    /// The supplied nominal weights are aligned to `self.world_ids`, which
    /// are always sorted lexicographically. Hazard and abstention budgets
    /// remain worst-case over every supplied world, independent of the TV
    /// distribution used for the loss objective.
    pub fn minimum_tv_expected_total_loss_under(
        &self,
        nominal_world_weights: &[Rational],
        tv_radius: Rational,
        budgets: DecisionBudgets,
    ) -> Result<Option<&RobustDecisionFrontierPoint>, String> {
        budgets.validate()?;
        validate_world_weights(self.world_ids.len(), nominal_world_weights, tv_radius)?;
        let mut selected: Option<&RobustDecisionFrontierPoint> = None;
        for point in &self.frontier {
            if !within_risk_budgets(point, budgets) {
                continue;
            }
            let candidate_loss =
                point.worst_case_total_loss_under_tv(nominal_world_weights, tv_radius)?;
            let Some(current) = selected else {
                selected = Some(point);
                continue;
            };
            let current_loss =
                current.worst_case_total_loss_under_tv(nominal_world_weights, tv_radius)?;
            let order = candidate_loss
                .cmp(&current_loss)
                .then_with(|| {
                    point
                        .worst_case_hazard_probability()
                        .cmp(&current.worst_case_hazard_probability())
                })
                .then_with(|| {
                    point
                        .worst_case_abstention_probability()
                        .cmp(&current.worst_case_abstention_probability())
                })
                .then_with(|| point.plan.cmp(&current.plan));
            if order == Ordering::Less {
                selected = Some(point);
            }
        }
        Ok(selected)
    }
}

fn validate_world_weights(
    world_count: usize,
    weights: &[Rational],
    radius: Rational,
) -> Result<(), String> {
    if weights.len() != world_count || weights.is_empty() {
        return Err("one nominal mixture weight is required for every canonical model world".into());
    }
    if !radius.is_probability() {
        return Err("total-variation radius must be in [0, 1]".into());
    }
    let mut total = Rational::zero();
    for weight in weights {
        if !weight.is_probability() {
            return Err("nominal model-world weights must be probabilities".into());
        }
        total = total.checked_add(*weight)?;
    }
    if total != Rational::one() {
        return Err("nominal model-world weights must sum exactly to one".into());
    }
    Ok(())
}

fn rational_gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

fn checked_sub_nonnegative(left: Rational, right: Rational) -> Result<Rational, String> {
    if left < right {
        return Err("non-negative rational subtraction underflow".into());
    }
    let common = rational_gcd(left.denominator(), right.denominator());
    let left_scale = right.denominator() / common;
    let right_scale = left.denominator() / common;
    let left_scaled = left
        .numerator()
        .checked_mul(left_scale)
        .ok_or_else(|| "rational subtraction numerator overflow".to_string())?;
    let right_scaled = right
        .numerator()
        .checked_mul(right_scale)
        .ok_or_else(|| "rational subtraction numerator overflow".to_string())?;
    let denominator = left
        .denominator()
        .checked_mul(left_scale)
        .ok_or_else(|| "rational subtraction denominator overflow".to_string())?;
    Rational::new(left_scaled - right_scaled, denominator)
}

fn worst_case_tv_expectation(
    losses: &[Rational],
    nominal_weights: &[Rational],
    radius: Rational,
) -> Result<Rational, String> {
    if losses.is_empty() {
        return Err("TV expectation requires at least one world".into());
    }
    validate_world_weights(losses.len(), nominal_weights, radius)?;

    let mut adjusted = nominal_weights.to_vec();
    let mut low_to_high: Vec<usize> = (0..losses.len()).collect();
    low_to_high.sort_by(|left, right| losses[*left].cmp(&losses[*right]).then(left.cmp(right)));
    let mut high_to_low = low_to_high.clone();
    high_to_low.reverse();

    let mut donor_index = 0;
    let mut receiver_index = 0;
    let mut remaining_radius = radius;
    while !remaining_radius.is_zero()
        && donor_index < low_to_high.len()
        && receiver_index < high_to_low.len()
    {
        let donor = low_to_high[donor_index];
        let receiver = high_to_low[receiver_index];
        if donor == receiver || losses[receiver] <= losses[donor] {
            break;
        }
        if adjusted[donor].is_zero() {
            donor_index += 1;
            continue;
        }
        let receiver_room = checked_sub_nonnegative(Rational::one(), adjusted[receiver])?;
        if receiver_room.is_zero() {
            receiver_index += 1;
            continue;
        }
        let transfer = remaining_radius
            .min(adjusted[donor])
            .min(receiver_room);
        adjusted[donor] = checked_sub_nonnegative(adjusted[donor], transfer)?;
        adjusted[receiver] = adjusted[receiver].checked_add(transfer)?;
        remaining_radius = checked_sub_nonnegative(remaining_radius, transfer)?;
    }

    let mut expectation = Rational::zero();
    for (weight, loss) in adjusted.iter().zip(losses.iter()) {
        expectation = expectation.checked_add(weight.checked_mul(*loss)?)?;
    }
    Ok(expectation)
}

fn within_risk_budgets(
    point: &RobustDecisionFrontierPoint,
    budgets: DecisionBudgets,
) -> bool {
    point.worst_case_hazard_probability() <= budgets.max_hazard_probability
        && point.worst_case_abstention_probability() <= budgets.max_abstention_probability
}

pub fn optimize_shared_policy_decisions(
    worlds: &[DecisionModelWorld],
    nominal_world_id: &str,
    table: &DecisionLossTable,
    protocol: ObservationProtocol,
    command_costs: &[Rational],
    hazard_loss: Rational,
    config: &DecisionSolverConfig,
) -> Result<RobustDecisionOptimizationResult, String> {
    let canonical_worlds = validate_world_family(worlds, nominal_world_id, table, command_costs)?;
    if config.horizon > super::finite_horizon::MAX_DECISION_HORIZON {
        return Err("robust decision horizon exceeds the configured maximum".into());
    }
    if config.max_memo_states == 0
        || config.max_frontier_points == 0
        || config.max_generated_combinations == 0
    {
        return Err("all robust decision solver resource caps must be positive".into());
    }

    let initial_masses: RawPathMassVector = canonical_worlds
        .iter()
        .map(|world| {
            Some(
                world
                    .kernel
                    .initial_belief
                    .iter()
                    .map(|point| RawPathMass {
                        state: point.state,
                        mass: point.mass,
                    })
                    .collect(),
            )
        })
        .collect();

    let mut solver = RobustDecisionOptimizer {
        worlds: &canonical_worlds,
        table,
        protocol,
        command_costs,
        hazard_loss,
        config,
        memo: BTreeMap::new(),
        stats: RobustDecisionSolverStats::default(),
    };
    let candidates = solver.solve(&initial_beliefs, config.horizon)?;
    let world_ids: Vec<String> = canonical_worlds.iter().map(|world| world.id.clone()).collect();
    let frontier = candidates
        .into_iter()
        .map(|candidate| RobustDecisionFrontierPoint {
            per_world: candidate
                .per_world
                .into_iter()
                .zip(world_ids.iter())
                .map(|(metric, world_id)| WorldDecisionObjectives {
                    world_id: world_id.clone(),
                    expected_experiment_cost: metric.expected_experiment_cost,
                    expected_terminal_loss: metric.expected_terminal_loss,
                    hazard_probability: metric.hazard_probability,
                    abstention_probability: metric.abstention_probability,
                })
                .collect(),
            plan: candidate.plan,
        })
        .collect();

    Ok(RobustDecisionOptimizationResult {
        world_ids,
        nominal_world_id: nominal_world_id.to_string(),
        frontier,
        stats: solver.stats,
    })
}

fn validate_world_family(
    worlds: &[DecisionModelWorld],
    nominal_world_id: &str,
    table: &DecisionLossTable,
    command_costs: &[Rational],
) -> Result<Vec<DecisionModelWorld>, String> {
    if worlds.is_empty() || worlds.len() > MAX_MODEL_WORLDS {
        return Err(format!("model-world count must be in 1..={MAX_MODEL_WORLDS}"));
    }
    if nominal_world_id.trim().is_empty() {
        return Err("nominal world ID must be non-empty".into());
    }

    let mut canonical = worlds.to_vec();
    canonical.sort_by(|left, right| left.id.cmp(&right.id));
    let mut ids = BTreeSet::new();
    if canonical.iter().any(|world| {
        world.id.trim().is_empty() || !ids.insert(world.id.as_str())
    }) {
        return Err("model-world IDs must be non-empty and unique".into());
    }
    if !canonical.iter().any(|world| world.id.as_str() == nominal_world_id) {
        return Err("nominal world ID is not present in the model family".into());
    }

    let reference = &canonical[0].kernel;
    reference.validate()?;
    if command_costs.len() != reference.command_ids.len() {
        return Err("one command cost is required for each shared command".into());
    }
    table.validate(reference)?;

    for world in &canonical[1..] {
        world.kernel.validate()?;
        table.validate(&world.kernel)?;
        let kernel = &world.kernel;
        if kernel.hypothesis_ids != reference.hypothesis_ids
            || kernel.physical_state_count != reference.physical_state_count
            || kernel.sensor_state_count != reference.sensor_state_count
            || kernel.apparatus_state_count != reference.apparatus_state_count
            || kernel.command_ids != reference.command_ids
            || kernel.realized_action_ids != reference.realized_action_ids
            || kernel.readback_ids != reference.readback_ids
            || kernel.measurement_ids != reference.measurement_ids
        {
            return Err(format!(
                "world {} does not share the family's state, command and observation semantics",
                world.id
            ));
        }
    }
    Ok(canonical)
}

struct RobustDecisionOptimizer<'a> {
    worlds: &'a [DecisionModelWorld],
    table: &'a DecisionLossTable,
    protocol: ObservationProtocol,
    command_costs: &'a [Rational],
    hazard_loss: Rational,
    config: &'a DecisionSolverConfig,
    memo: BTreeMap<MemoKey, Vec<Candidate>>,
    stats: RobustDecisionSolverStats,
}

impl RobustDecisionOptimizer<'_> {
    fn solve(
        &mut self,
        beliefs: &BeliefVector,
        remaining_horizon: usize,
    ) -> Result<Vec<Candidate>, String> {
        let key = MemoKey {
            beliefs: BeliefVectorKey::new(beliefs),
            remaining_horizon,
        };
        if let Some(cached) = self.memo.get(&key) {
            return Ok(cached.clone());
        }
        self.stats.memo_states = self
            .stats
            .memo_states
            .checked_add(1)
            .ok_or_else(|| "robust decision memo-state counter overflow".to_string())?;
        if self.stats.memo_states > self.config.max_memo_states {
            return Err("robust decision memo-state budget exceeded".into());
        }

        let mut candidates = terminal_candidates(beliefs, self.table)?;
        if remaining_horizon > 0 {
            for command in 0..self.command_costs.len() {
                let partitions = command_partitions_for_beliefs(
                    self.worlds,
                    beliefs,
                    command,
                    self.protocol,
                )?;
                let observations = union_observations(&partitions);
                let mut partials = vec![PartialCandidate {
                    per_world: beliefs
                        .iter()
                        .map(|belief| {
                            if belief.is_some() {
                                WorldObjectives {
                                    expected_experiment_cost: self.command_costs[command],
                                    ..WorldObjectives::zero()
                                }
                            } else {
                                WorldObjectives::zero()
                            }
                        })
                        .collect(),
                    branches: Vec::new(),
                }];
                for observation in &observations {
                    let (probabilities, branch_beliefs) =
                        branch_vectors(&partitions, observation);
                    let child_frontier = if observation.hazard {
                        vec![hazard_child(&probabilities, self.hazard_loss)]
                    } else {
                        self.solve(&branch_beliefs, remaining_horizon - 1)?
                    };
                    partials = combine_partial_candidates(
                        partials,
                        &child_frontier,
                        *observation,
                        &probabilities,
                        self.config,
                        &mut self.stats,
                    )?;
                }
                candidates.extend(partials.into_iter().map(|partial| Candidate {
                    per_world: partial.per_world,
                    plan: DecisionPlan::Experiment {
                        command,
                        branches: partial.branches,
                    },
                }));
            }
        }

        let frontier = prune_candidates(candidates)?;
        if frontier.len() > self.config.max_frontier_points {
            return Err("robust decision frontier-point budget exceeded".into());
        }
        self.stats.maximum_frontier_points =
            self.stats.maximum_frontier_points.max(frontier.len());
        self.memo.insert(key, frontier.clone());
        Ok(frontier)
    }
}

fn command_partitions_for_beliefs(
    worlds: &[DecisionModelWorld],
    beliefs: &BeliefVector,
    command: usize,
    protocol: ObservationProtocol,
) -> Result<Vec<WorldPartition>, String> {
    worlds
        .iter()
        .zip(beliefs.iter())
        .map(|(world, belief)| match belief {
            Some(belief) => {
                let kernel = &world.kernel;
                let mut observations = BTreeSet::new();
                for point in belief {
                    let row = &kernel.rows[kernel.row_index(point.state, command)];
                    for outcome in &row.outcomes {
                        observations.insert(public_observation(outcome, protocol));
                    }
                }
                let mut branches = BTreeMap::new();
                for observation in observations {
                    let filtered = filter_one_step(kernel, belief, command, observation)
                        .map_err(|error| format!("robust decision filter failed: {error}"))?;
                    let next_belief = if filtered.terminal_hazard {
                        None
                    } else {
                        Some(
                            filtered
                                .belief_for_next_step()
                                .map_err(|error| format!("robust next belief failed: {error}"))?,
                        )
                    };
                    branches.insert(
                        observation,
                        ObservationBranch {
                            probability: filtered.observation_likelihood,
                            next_belief,
                        },
                    );
                }
                Ok(branches)
            }
            None => Ok(BTreeMap::new()),
        })
        .collect()
}

fn public_observation(
    outcome: &TransitionOutcome,
    protocol: ObservationProtocol,
) -> PublicObservation {
    PublicObservation {
        hazard: outcome.hazard,
        readback: visible_field(outcome.readback, protocol.observe_readback),
        measurement: visible_field(outcome.measurement, protocol.observe_measurement),
    }
}

fn visible_field(value: Option<usize>, observed: bool) -> ObservationField {
    if !observed {
        ObservationField::Any
    } else {
        match value {
            Some(id) => ObservationField::Value(id),
            None => ObservationField::Missing,
        }
    }
}

fn union_observations(partitions: &[WorldPartition]) -> BTreeSet<PublicObservation> {
    partitions
        .iter()
        .flat_map(|partition| partition.keys().copied())
        .collect()
}

fn branch_vectors(
    partitions: &[WorldPartition],
    observation: &PublicObservation,
) -> (Vec<Rational>, BeliefVector) {
    let mut probabilities = Vec::with_capacity(partitions.len());
    let mut beliefs = Vec::with_capacity(partitions.len());
    for partition in partitions {
        match partition.get(observation) {
            Some(branch) => {
                probabilities.push(branch.probability);
                beliefs.push(branch.next_belief.clone());
            }
            None => {
                probabilities.push(Rational::zero());
                beliefs.push(None);
            }
        }
    }
    (probabilities, beliefs)
}

fn terminal_candidates(
    beliefs: &BeliefVector,
    table: &DecisionLossTable,
) -> Result<Vec<Candidate>, String> {
    if !beliefs.iter().any(Option::is_some) {
        return Err("a public history cannot have every model world inactive".into());
    }
    let mut candidates = Vec::with_capacity(table.decision_ids.len() + 1);
    // Abstention is inserted first, so an exact objective tie is deterministic.
    for decision in std::iter::once(None).chain((0..table.decision_ids.len()).map(Some)) {
        let mut per_world = Vec::with_capacity(beliefs.len());
        for belief in beliefs {
            match belief {
                Some(belief) => {
                    let loss = expected_terminal_loss(table, belief, decision)?;
                    per_world.push(WorldObjectives {
                        expected_terminal_loss: loss,
                        abstention_probability: if decision.is_none() {
                            Rational::one()
                        } else {
                            Rational::zero()
                        },
                        ..WorldObjectives::zero()
                    });
                }
                None => per_world.push(WorldObjectives::zero()),
            }
        }
        candidates.push(Candidate {
            per_world,
            plan: DecisionPlan::TerminalDecision { decision },
        });
    }
    Ok(candidates)
}

fn expected_terminal_loss(
    table: &DecisionLossTable,
    belief: &[BeliefMass],
    decision: Option<usize>,
) -> Result<Rational, String> {
    let mut expected = Rational::zero();
    for point in belief {
        let loss = match decision {
            Some(index) => {
                if index >= table.decision_ids.len() {
                    return Err("terminal policy references an invalid decision".into());
                }
                table.loss_for_state(index, point.state)
            }
            None => table.abstention_loss,
        };
        expected = expected.checked_add(point.mass.checked_mul(loss)?)?;
    }
    Ok(expected)
}

fn hazard_child(probabilities: &[Rational], hazard_loss: Rational) -> Candidate {
    Candidate {
        per_world: probabilities
            .iter()
            .map(|probability| {
                if probability.is_zero() {
                    WorldObjectives::zero()
                } else {
                    WorldObjectives::hazard_terminal(hazard_loss)
                }
            })
            .collect(),
        plan: DecisionPlan::HazardTerminal,
    }
}

fn combine_partial_candidates(
    partials: Vec<PartialCandidate>,
    children: &[Candidate],
    observation: PublicObservation,
    probabilities: &[Rational],
    config: &DecisionSolverConfig,
    stats: &mut RobustDecisionSolverStats,
) -> Result<Vec<PartialCandidate>, String> {
    let mut expanded = Vec::new();
    for partial in &partials {
        for child in children {
            stats.generated_combinations = stats
                .generated_combinations
                .checked_add(1)
                .ok_or_else(|| "robust decision combination counter overflow".to_string())?;
            if stats.generated_combinations > config.max_generated_combinations {
                return Err("robust decision generated-combination budget exceeded".into());
            }
            let mut next = partial.clone();
            for index in 0..next.per_world.len() {
                let probability = probabilities[index];
                if !probability.is_zero() {
                    next.per_world[index].add_scaled(child.per_world[index], probability)?;
                }
            }
            next.branches.push(DecisionBranch {
                observation,
                plan: Box::new(child.plan.clone()),
            });
            expanded.push(next);
        }
    }
    let frontier = prune_partial_candidates(expanded);
    if frontier.len() > config.max_frontier_points {
        return Err("robust decision partial-frontier cap exceeded".into());
    }
    Ok(frontier)
}

fn partial_dominates(left: &[WorldObjectives], right: &[WorldObjectives]) -> bool {
    let mut strictly_better = false;
    for (a, b) in left.iter().zip(right.iter()) {
        let no_worse = a.expected_experiment_cost <= b.expected_experiment_cost
            && a.expected_terminal_loss <= b.expected_terminal_loss
            && a.hazard_probability <= b.hazard_probability
            && a.abstention_probability <= b.abstention_probability;
        if !no_worse {
            return false;
        }
        strictly_better |= a.expected_experiment_cost < b.expected_experiment_cost
            || a.expected_terminal_loss < b.expected_terminal_loss
            || a.hazard_probability < b.hazard_probability
            || a.abstention_probability < b.abstention_probability;
    }
    strictly_better
}

fn same_objectives(left: &[WorldObjectives], right: &[WorldObjectives]) -> bool {
    left == right
}

fn partial_order(left: &PartialCandidate, right: &PartialCandidate) -> Ordering {
    left.per_world
        .cmp(&right.per_world)
        .then_with(|| left.branches.cmp(&right.branches))
}

fn prune_partial_candidates(mut candidates: Vec<PartialCandidate>) -> Vec<PartialCandidate> {
    candidates.sort_by(partial_order);
    let mut frontier: Vec<PartialCandidate> = Vec::new();
    for candidate in candidates {
        if frontier.iter().any(|other| {
            same_objectives(&other.per_world, &candidate.per_world)
                || partial_dominates(&other.per_world, &candidate.per_world)
        }) {
            continue;
        }
        frontier.retain(|other| !partial_dominates(&candidate.per_world, &other.per_world));
        frontier.push(candidate);
    }
    frontier
}

fn candidate_order(left: &Candidate, right: &Candidate) -> Ordering {
    left.per_world
        .cmp(&right.per_world)
        .then_with(|| left.plan.cmp(&right.plan))
}

fn prune_candidates(mut candidates: Vec<Candidate>) -> Result<Vec<Candidate>, String> {
    candidates.sort_by(candidate_order);
    let mut frontier: Vec<Candidate> = Vec::new();
    for candidate in candidates {
        if frontier.iter().any(|other| {
            same_objectives(&other.per_world, &candidate.per_world)
                || partial_dominates(&other.per_world, &candidate.per_world)
        }) {
            continue;
        }
        frontier.retain(|other| !partial_dominates(&candidate.per_world, &other.per_world));
        frontier.push(candidate);
    }
    Ok(frontier)
}

/// Independently replay one shared policy across all supplied worlds.
///
/// Branch coverage must equal the union of observations possible in all active
/// worlds at that public history. Each world is replayed with its own unchanged
/// kernel and conditional belief; the evaluator never chooses a world-specific
/// action or terminal decision.
pub fn replay_shared_policy_across_worlds(
    worlds: &[DecisionModelWorld],
    nominal_world_id: &str,
    table: &DecisionLossTable,
    protocol: ObservationProtocol,
    command_costs: &[Rational],
    hazard_loss: Rational,
    plan: &DecisionPlan,
    horizon: usize,
) -> Result<RobustDecisionFrontierPoint, String> {
    let canonical_worlds = validate_world_family(worlds, nominal_world_id, table, command_costs)?;
    let initial_masses: RawPathMassVector = canonical_worlds
        .iter()
        .map(|world| {
            Some(
                world
                    .kernel
                    .initial_belief
                    .iter()
                    .map(|point| RawPathMass {
                        state: point.state,
                        mass: point.mass,
                    })
                    .collect(),
            )
        })
        .collect();
    if matches!(plan, DecisionPlan::HazardTerminal) {
        return Err("a campaign cannot begin in a hazard-terminal state".into());
    }
    let metrics = replay_unnormalized_node(
        &canonical_worlds,
        table,
        protocol,
        command_costs,
        hazard_loss,
        plan,
        &initial_masses,
        horizon,
    )?;
    let per_world = metrics
        .into_iter()
        .zip(canonical_worlds.iter())
        .map(|(metric, world)| WorldDecisionObjectives {
            world_id: world.id.clone(),
            expected_experiment_cost: metric.expected_experiment_cost,
            expected_terminal_loss: metric.expected_terminal_loss,
            hazard_probability: metric.hazard_probability,
            abstention_probability: metric.abstention_probability,
        })
        .collect();
    Ok(RobustDecisionFrontierPoint {
        per_world,
        plan: plan.clone(),
    })
}

/// Independent forward-mass replay of a shared policy.
///
/// Unlike the optimizer and exhaustive policy oracle, this verifier never
/// computes a Bayesian posterior and never calls `filter_one_step`. It
/// propagates unnormalized path mass directly through raw kernel outcomes.
/// This independently checks posterior normalization and branch-probability
/// bookkeeping in addition to policy non-anticipativity.
#[allow(clippy::too_many_arguments)]
fn replay_unnormalized_node(
    worlds: &[DecisionModelWorld],
    table: &DecisionLossTable,
    protocol: ObservationProtocol,
    command_costs: &[Rational],
    hazard_loss: Rational,
    plan: &DecisionPlan,
    masses: &RawPathMassVector,
    remaining_horizon: usize,
) -> Result<Vec<WorldObjectives>, String> {
    match plan {
        DecisionPlan::TerminalDecision { decision } => {
            let mut result = Vec::with_capacity(worlds.len());
            for mass in masses {
                match mass {
                    Some(points) => {
                        let mut terminal_loss = Rational::zero();
                        let mut total_mass = Rational::zero();
                        for point in points {
                            total_mass = total_mass.checked_add(point.mass)?;
                            let loss = match decision {
                                Some(index) => {
                                    if *index >= table.decision_ids.len() {
                                        return Err("terminal policy references an invalid decision".into());
                                    }
                                    table.loss_for_state(*index, point.state)
                                }
                                None => table.abstention_loss,
                            };
                            terminal_loss = terminal_loss.checked_add(point.mass.checked_mul(loss)?)?;
                        }
                        result.push(WorldObjectives {
                            expected_terminal_loss: terminal_loss,
                            abstention_probability: if decision.is_none() {
                                total_mass
                            } else {
                                Rational::zero()
                            },
                            ..WorldObjectives::zero()
                        });
                    }
                    None => result.push(WorldObjectives::zero()),
                }
            }
            Ok(result)
        }
        DecisionPlan::HazardTerminal => {
            Err("hazard terminal is valid only below an observed hazard branch".into())
        }
        DecisionPlan::Experiment { command, branches } => {
            if remaining_horizon == 0 {
                return Err("shared policy exceeds its finite horizon".into());
            }
            let Some(command_cost) = command_costs.get(*command).copied() else {
                return Err("shared policy references an invalid command".into());
            };

            let partitions = raw_replay_partitions(worlds, masses, *command, protocol)?;
            let observations: BTreeSet<_> = partitions
                .iter()
                .flat_map(|partition| partition.keys().copied())
                .collect();
            let branch_map: BTreeMap<_, _> = branches
                .iter()
                .map(|branch| (branch.observation, branch.plan.as_ref()))
                .collect();
            if branch_map.len() != branches.len()
                || branch_map.len() != observations.len()
                || observations.iter().any(|observation| !branch_map.contains_key(observation))
            {
                return Err(
                    "shared policy must contain exactly one branch per reachable public observation"
                        .into(),
                );
            }

            let mut result = Vec::with_capacity(worlds.len());
            for mass in masses {
                let total_mass = match mass {
                    Some(points) => points.iter().try_fold(Rational::zero(), |sum, point| {
                        sum.checked_add(point.mass)
                    })?,
                    None => Rational::zero(),
                };
                result.push(WorldObjectives {
                    expected_experiment_cost: total_mass.checked_mul(command_cost)?,
                    ..WorldObjectives::zero()
                });
            }

            for observation in observations {
                let child_plan = branch_map[&observation];
                let branch_masses: RawPathMassVector = partitions
                    .iter()
                    .map(|partition| {
                        partition
                            .get(&observation)
                            .and_then(|branch| branch.next_mass.clone())
                    })
                    .collect();

                let child_metrics = if observation.hazard {
                    if *child_plan != DecisionPlan::HazardTerminal {
                        return Err("observed hazard branch must terminate immediately".into());
                    }
                    let mut hazards = Vec::with_capacity(partitions.len());
                    for partition in &partitions {
                        let probability = partition
                            .get(&observation)
                            .map(|branch| branch.probability)
                            .unwrap_or_else(Rational::zero);
                        if probability.is_zero() {
                            hazards.push(WorldObjectives::zero());
                        } else {
                            hazards.push(WorldObjectives {
                                expected_terminal_loss: probability.checked_mul(hazard_loss)?,
                                hazard_probability: probability,
                                ..WorldObjectives::zero()
                            });
                        }
                    }
                    hazards
                } else {
                    if *child_plan == DecisionPlan::HazardTerminal {
                        return Err("non-hazard observation cannot use a hazard terminal".into());
                    }
                    replay_unnormalized_node(
                        worlds,
                        table,
                        protocol,
                        command_costs,
                        hazard_loss,
                        child_plan,
                        &branch_masses,
                        remaining_horizon - 1,
                    )?
                };
                for (current, child) in result.iter_mut().zip(child_metrics.iter()) {
                    current.expected_experiment_cost =
                        current.expected_experiment_cost.checked_add(child.expected_experiment_cost)?;
                    current.expected_terminal_loss =
                        current.expected_terminal_loss.checked_add(child.expected_terminal_loss)?;
                    current.hazard_probability =
                        current.hazard_probability.checked_add(child.hazard_probability)?;
                    current.abstention_probability =
                        current.abstention_probability.checked_add(child.abstention_probability)?;
                }
            }
            Ok(result)
        }
    }
}

/// Group raw, unnormalized path mass by protocol-visible observation.
///
/// This intentionally uses direct outcome enumeration rather than the
/// Bayesian filter. For a non-hazard observation, the next-state masses retain
/// the total probability of reaching that branch instead of being normalized.
fn raw_replay_partitions(
    worlds: &[DecisionModelWorld],
    masses: &RawPathMassVector,
    command: usize,
    protocol: ObservationProtocol,
) -> Result<Vec<ReplayPartition>, String> {
    worlds
        .iter()
        .zip(masses.iter())
        .map(|(world, mass)| match mass {
            None => Ok(BTreeMap::new()),
            Some(points) => {
                let kernel = &world.kernel;
                if command >= kernel.command_ids.len() {
                    return Err("replay command is out of range for a model world".into());
                }
                let mut raw = RawObservationMassMap::new();
                for point in points {
                    let row = &kernel.rows[kernel.row_index(point.state, command)];
                    for outcome in &row.outcomes {
                        let observation = public_observation(outcome, protocol);
                        let path_mass = point.mass.checked_mul(outcome.probability)?;
                        let entry = raw
                            .entry(observation)
                            .or_insert_with(|| (Rational::zero(), BTreeMap::new()));
                        entry.0 = entry.0.checked_add(path_mass)?;
                        if !outcome.hazard {
                            let next = entry.1.entry(outcome.next_state)
                                .or_insert_with(Rational::zero);
                            *next = next.checked_add(path_mass)?;
                        }
                    }
                }
                let mut partitions = ReplayPartition::new();
                for (observation, (probability, next_states)) in raw {
                    if probability.is_zero() {
                        return Err("raw replay constructed a zero-mass observation".into());
                    }
                    let next_mass = if observation.hazard {
                        if !next_states.is_empty() {
                            return Err("hazard replay branch unexpectedly contains next-state mass".into());
                        }
                        None
                    } else {
                        let next_total = next_states
                            .values()
                            .try_fold(Rational::zero(), |sum, mass| sum.checked_add(*mass))?;
                        if next_total != probability || next_states.is_empty() {
                            return Err(
                                "safe replay branch next-state mass does not equal its observation mass"
                                    .into(),
                            );
                        }
                        Some(
                            next_states
                                .into_iter()
                                .map(|(state, mass)| RawPathMass { state, mass })
                                .collect(),
                        )
                    };
                    partitions.insert(
                        observation,
                        ReplayBranch {
                            probability,
                            next_mass,
                        },
                    );
                }
                let input_mass = points.iter().try_fold(
                    Rational::zero(),
                    |sum, point| sum.checked_add(point.mass),
                )?;
                let output_mass = partitions.values().try_fold(
                    Rational::zero(),
                    |sum, branch| sum.checked_add(branch.probability),
                )?;
                if input_mass != output_mass {
                    return Err(
                        "raw replay observation partitions do not conserve incoming path mass"
                            .into(),
                    );
                }
                Ok(partitions)
            }
        })
        .collect()
}

#[cfg(test)]
fn frontier_points_from_candidates(
    candidates: Vec<Candidate>,
    world_ids: &[String],
) -> Vec<RobustDecisionFrontierPoint> {
    candidates
        .into_iter()
        .map(|candidate| RobustDecisionFrontierPoint {
            per_world: candidate
                .per_world
                .into_iter()
                .zip(world_ids.iter())
                .map(|(metric, world_id)| WorldDecisionObjectives {
                    world_id: world_id.clone(),
                    expected_experiment_cost: metric.expected_experiment_cost,
                    expected_terminal_loss: metric.expected_terminal_loss,
                    hazard_probability: metric.hazard_probability,
                    abstention_probability: metric.abstention_probability,
                })
                .collect(),
            plan: candidate.plan,
        })
        .collect()
}

pub fn frontier_objectives(
    frontier: &[RobustDecisionFrontierPoint],
) -> Vec<Vec<(Rational, Rational, Rational, Rational)>> {
    let mut values: Vec<_> = frontier
        .iter()
        .map(|point| {
            point
                .per_world
                .iter()
                .map(|world| {
                    (
                        world.expected_experiment_cost,
                        world.expected_terminal_loss,
                        world.hazard_probability,
                        world.abstention_probability,
                    )
                })
                .collect()
        })
        .collect();
    values.sort();
    values.dedup();
    values
}

#[cfg(test)]
fn exhaustive_frontier(
    worlds: &[DecisionModelWorld],
    table: &DecisionLossTable,
    protocol: ObservationProtocol,
    command_costs: &[Rational],
    hazard_loss: Rational,
    beliefs: &BeliefVector,
    remaining_horizon: usize,
    cap: usize,
    generated: &mut usize,
) -> Result<Vec<Candidate>, String> {
    let mut candidates = terminal_candidates(beliefs, table)?;
    if remaining_horizon == 0 {
        return Ok(candidates);
    }
    for command in 0..command_costs.len() {
        let partitions =
            command_partitions_for_beliefs(worlds, beliefs, command, protocol)?;
        let observations = union_observations(&partitions);
        let mut partials = vec![PartialCandidate {
            per_world: beliefs
                .iter()
                .map(|belief| {
                    if belief.is_some() {
                        WorldObjectives {
                            expected_experiment_cost: command_costs[command],
                            ..WorldObjectives::zero()
                        }
                    } else {
                        WorldObjectives::zero()
                    }
                })
                .collect(),
            branches: Vec::new(),
        }];
        for observation in &observations {
            let (probabilities, branch_beliefs) = branch_vectors(&partitions, observation);
            let children = if observation.hazard {
                vec![hazard_child(&probabilities, hazard_loss)]
            } else {
                exhaustive_frontier(
                    worlds,
                    table,
                    protocol,
                    command_costs,
                    hazard_loss,
                    &branch_beliefs,
                    remaining_horizon - 1,
                    cap,
                    generated,
                )?
            };
            let mut expanded = Vec::new();
            for partial in &partials {
                for child in &children {
                    *generated = generated
                        .checked_add(1)
                        .ok_or_else(|| "exhaustive robust combination counter overflow".to_string())?;
                    if *generated > cap {
                        return Err("exhaustive robust policy-tree cap exceeded".into());
                    }
                    let mut next = partial.clone();
                    for index in 0..next.per_world.len() {
                        if !probabilities[index].is_zero() {
                            next.per_world[index]
                                .add_scaled(child.per_world[index], probabilities[index])?;
                        }
                    }
                    next.branches.push(DecisionBranch {
                        observation: *observation,
                        plan: Box::new(child.plan.clone()),
                    });
                    expanded.push(next);
                }
            }
            partials = independent_partial_frontier(expanded);
        }
        candidates.extend(partials.into_iter().map(|partial| Candidate {
            per_world: partial.per_world,
            plan: DecisionPlan::Experiment {
                command,
                branches: partial.branches,
            },
        }));
    }
    Ok(independent_candidate_frontier(candidates))
}

#[cfg(test)]
fn independent_same_objectives(left: &[WorldObjectives], right: &[WorldObjectives]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right.iter()).all(|(a, b)| {
            a.expected_experiment_cost == b.expected_experiment_cost
                && a.expected_terminal_loss == b.expected_terminal_loss
                && a.hazard_probability == b.hazard_probability
                && a.abstention_probability == b.abstention_probability
        })
}

#[cfg(test)]
fn independent_dominates(left: &[WorldObjectives], right: &[WorldObjectives]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut strict = false;
    for (a, b) in left.iter().zip(right.iter()) {
        if a.expected_experiment_cost > b.expected_experiment_cost
            || a.expected_terminal_loss > b.expected_terminal_loss
            || a.hazard_probability > b.hazard_probability
            || a.abstention_probability > b.abstention_probability
        {
            return false;
        }
        strict |= a.expected_experiment_cost < b.expected_experiment_cost
            || a.expected_terminal_loss < b.expected_terminal_loss
            || a.hazard_probability < b.hazard_probability
            || a.abstention_probability < b.abstention_probability;
    }
    strict
}

#[cfg(test)]
fn independent_partial_frontier(
    candidates: Vec<PartialCandidate>,
) -> Vec<PartialCandidate> {
    let mut retained = Vec::new();
    for (index, candidate) in candidates.iter().enumerate() {
        let dominated = candidates.iter().enumerate().any(|(other_index, other)| {
            if index == other_index {
                return false;
            }
            (other_index < index
                && independent_same_objectives(&other.per_world, &candidate.per_world))
                || independent_dominates(&other.per_world, &candidate.per_world)
        });
        if !dominated {
            retained.push(candidate.clone());
        }
    }
    retained.sort_by(partial_order);
    retained
}

#[cfg(test)]
fn independent_candidate_frontier(candidates: Vec<Candidate>) -> Vec<Candidate> {
    let mut retained = Vec::new();
    for (index, candidate) in candidates.iter().enumerate() {
        let dominated = candidates.iter().enumerate().any(|(other_index, other)| {
            if index == other_index {
                return false;
            }
            (other_index < index
                && independent_same_objectives(&other.per_world, &candidate.per_world))
                || independent_dominates(&other.per_world, &candidate.per_world)
        });
        if !dominated {
            retained.push(candidate.clone());
        }
    }
    retained.sort_by(candidate_order);
    retained
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KernelRow, TransitionOutcome};

    fn half() -> Rational {
        Rational::new(1, 2).unwrap()
    }

    fn make_inverted_instrument_world(id: &str, inverted: bool, marker_value: usize) -> DecisionModelWorld {
        let mut rows = Vec::new();
        for hypothesis in 0..2 {
            let state = LatentState {
                hypothesis,
                physical_state: 0,
                sensor_state: 0,
                apparatus_state: 0,
            };
            for command in 0..2 {
                let measurement = if command == 0 {
                    marker_value
                } else if inverted {
                    1 - hypothesis
                } else {
                    hypothesis
                };
                rows.push(KernelRow {
                    state,
                    command,
                    outcomes: vec![TransitionOutcome {
                        probability: Rational::one(),
                        next_state: state,
                        realized_action: 0,
                        readback: Some(0),
                        measurement: Some(measurement),
                        hazard: false,
                    }],
                });
            }
        }
        DecisionModelWorld {
            id: id.into(),
            kernel: ActionRealizationKernel {
                hypothesis_ids: vec!["h0".into(), "h1".into()],
                physical_state_count: 1,
                sensor_state_count: 1,
                apparatus_state_count: 1,
                command_ids: vec!["world-marker".into(), "cheap-reading".into()],
                realized_action_ids: vec!["executed".into()],
                readback_ids: vec!["ack".into()],
                measurement_ids: vec!["zero".into(), "one".into()],
                initial_belief: vec![
                    BeliefMass {
                        state: LatentState {
                            hypothesis: 0,
                            physical_state: 0,
                            sensor_state: 0,
                            apparatus_state: 0,
                        },
                        mass: half(),
                    },
                    BeliefMass {
                        state: LatentState {
                            hypothesis: 1,
                            physical_state: 0,
                            sensor_state: 0,
                            apparatus_state: 0,
                        },
                        mass: half(),
                    },
                ],
                rows,
            },
        }
    }

    fn stochastic_world(
        id: &str,
        h0_probability_zero: Rational,
        h1_probability_zero: Rational,
    ) -> DecisionModelWorld {
        let mut rows = Vec::new();
        for hypothesis in 0..2 {
            let state = LatentState {
                hypothesis,
                physical_state: 0,
                sensor_state: 0,
                apparatus_state: 0,
            };
            let probability_zero = if hypothesis == 0 {
                h0_probability_zero
            } else {
                h1_probability_zero
            };
            let probability_one = Rational::new(
                probability_zero.denominator() - probability_zero.numerator(),
                probability_zero.denominator(),
            )
            .unwrap();
            rows.push(KernelRow {
                state,
                command: 0,
                outcomes: vec![
                    TransitionOutcome {
                        probability: probability_zero,
                        next_state: state,
                        realized_action: 0,
                        readback: Some(0),
                        measurement: Some(0),
                        hazard: false,
                    },
                    TransitionOutcome {
                        probability: probability_one,
                        next_state: state,
                        realized_action: 0,
                        readback: Some(0),
                        measurement: Some(1),
                        hazard: false,
                    },
                ],
            });
        }
        DecisionModelWorld {
            id: id.into(),
            kernel: ActionRealizationKernel {
                hypothesis_ids: vec!["h0".into(), "h1".into()],
                physical_state_count: 1,
                sensor_state_count: 1,
                apparatus_state_count: 1,
                command_ids: vec!["stochastic-read".into()],
                realized_action_ids: vec!["executed".into()],
                readback_ids: vec!["ack".into()],
                measurement_ids: vec!["zero".into(), "one".into()],
                initial_belief: vec![
                    BeliefMass {
                        state: LatentState {
                            hypothesis: 0,
                            physical_state: 0,
                            sensor_state: 0,
                            apparatus_state: 0,
                        },
                        mass: half(),
                    },
                    BeliefMass {
                        state: LatentState {
                            hypothesis: 1,
                            physical_state: 0,
                            sensor_state: 0,
                            apparatus_state: 0,
                        },
                        mass: half(),
                    },
                ],
                rows,
            },
        }
    }

    fn hazard_shift_world() -> DecisionModelWorld {
        let mut world = make_inverted_instrument_world("world-hazard-shift", false, 0);
        for row in &mut world.kernel.rows {
            if row.command == 1 {
                let state = row.state;
                let measurement = row.outcomes[0].measurement;
                row.outcomes = vec![
                    TransitionOutcome {
                        probability: Rational::new(9, 10).unwrap(),
                        next_state: state,
                        realized_action: 0,
                        readback: Some(0),
                        measurement,
                        hazard: false,
                    },
                    TransitionOutcome {
                        probability: Rational::new(1, 10).unwrap(),
                        next_state: state,
                        realized_action: 0,
                        readback: Some(0),
                        measurement: None,
                        hazard: true,
                    },
                ];
            }
        }
        world
    }

    fn loss_table() -> DecisionLossTable {
        DecisionLossTable {
            decision_ids: vec!["claim-h0".into(), "claim-h1".into()],
            losses: vec![
                vec![
                    vec![vec![vec![Rational::zero()]]],
                    vec![vec![vec![Rational::one()]]],
                ],
                vec![
                    vec![vec![vec![Rational::one()]]],
                    vec![vec![vec![Rational::zero()]]],
                ],
            ],
            abstention_loss: Rational::one(),
        }
    }

    fn config(horizon: usize) -> DecisionSolverConfig {
        DecisionSolverConfig {
            horizon,
            max_memo_states: 20_000,
            max_frontier_points: 20_000,
            max_generated_combinations: 200_000,
        }
    }

    fn marker_then_read_policy() -> DecisionPlan {
        let m0 = PublicObservation {
            hazard: false,
            readback: ObservationField::Value(0),
            measurement: ObservationField::Value(0),
        };
        let m1 = PublicObservation {
            measurement: ObservationField::Value(1),
            ..m0
        };
        let read0 = PublicObservation {
            measurement: ObservationField::Value(0),
            ..m0
        };
        let read1 = PublicObservation {
            measurement: ObservationField::Value(1),
            ..m0
        };
        let normal = DecisionPlan::Experiment {
            command: 1,
            branches: vec![
                DecisionBranch {
                    observation: read0,
                    plan: Box::new(DecisionPlan::TerminalDecision { decision: Some(0) }),
                },
                DecisionBranch {
                    observation: read1,
                    plan: Box::new(DecisionPlan::TerminalDecision { decision: Some(1) }),
                },
            ],
        };
        let inverted = DecisionPlan::Experiment {
            command: 1,
            branches: vec![
                DecisionBranch {
                    observation: read0,
                    plan: Box::new(DecisionPlan::TerminalDecision { decision: Some(1) }),
                },
                DecisionBranch {
                    observation: read1,
                    plan: Box::new(DecisionPlan::TerminalDecision { decision: Some(0) }),
                },
            ],
        };
        DecisionPlan::Experiment {
            command: 0,
            branches: vec![
                DecisionBranch {
                    observation: m0,
                    plan: Box::new(normal),
                },
                DecisionBranch {
                    observation: m1,
                    plan: Box::new(inverted),
                },
            ],
        }
    }

    fn worlds() -> Vec<DecisionModelWorld> {
        vec![
            make_inverted_instrument_world("world-a", false, 0),
            make_inverted_instrument_world("world-b", true, 1),
        ]
    }

    fn costs() -> Vec<Rational> {
        vec![Rational::new(1, 10).unwrap(), Rational::new(1, 20).unwrap()]
    }

    #[test]
    fn robust_policy_uses_public_probe_to_eliminate_worlds_branch_locally() {
        let world_set = worlds();
        let replay = replay_shared_policy_across_worlds(
            &world_set,
            "world-a",
            &loss_table(),
            ObservationProtocol::fully_observed(),
            &costs(),
            Rational::one(),
            &marker_then_read_policy(),
            2,
        )
        .unwrap();
        assert_eq!(replay.per_world[0].expected_experiment_cost, Rational::new(3, 20).unwrap());
        assert_eq!(replay.per_world[1].expected_experiment_cost, Rational::new(3, 20).unwrap());
        assert_eq!(replay.per_world[0].expected_terminal_loss, Rational::zero());
        assert_eq!(replay.per_world[1].expected_terminal_loss, Rational::zero());
        assert_eq!(replay.per_world[0].hazard_probability, Rational::zero());
        assert_eq!(replay.per_world[1].hazard_probability, Rational::zero());
    }

    #[test]
    fn shared_robust_optimizer_avoids_the_nominally_optimal_inverted_probe() {
        let world_set = worlds();
        let table = loss_table();
        let protocol = ObservationProtocol::fully_observed();
        let costs = costs();
        let nominal = super::super::finite_horizon::optimize_finite_horizon_decisions(
            &world_set[0].kernel,
            &table,
            protocol,
            &costs,
            Rational::one(),
            &config(1),
        )
        .unwrap();
        let nominal_plan = nominal
            .minimum_loss_under(DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::one(),
            })
            .unwrap()
            .unwrap();
        assert_eq!(
            nominal_plan.expected_experiment_cost,
            Rational::new(1, 20).unwrap()
        );

        let nominal_replay = replay_shared_policy_across_worlds(
            &world_set,
            "world-a",
            &table,
            protocol,
            &costs,
            Rational::one(),
            &nominal_plan.plan,
            1,
        )
        .unwrap();
        assert_eq!(nominal_replay.per_world[0].expected_terminal_loss, Rational::zero());
        assert_eq!(nominal_replay.per_world[1].expected_terminal_loss, Rational::one());

        let robust = optimize_shared_policy_decisions(
            &world_set,
            "world-a",
            &table,
            protocol,
            &costs,
            Rational::one(),
            &config(2),
        )
        .unwrap();
        let selected = robust
            .minimum_worst_case_total_loss_under(DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::one(),
            })
            .unwrap()
            .unwrap();
        assert_eq!(selected.worst_case_total_loss().unwrap(), Rational::new(3, 20).unwrap());
        assert_eq!(selected.worst_case_terminal_loss(), Rational::zero());
    }

    #[test]
    fn optimized_frontier_matches_non_memoized_exhaustive_oracle_and_replays() {
        let world_set = worlds();
        let table = loss_table();
        let protocol = ObservationProtocol::fully_observed();
        let costs = costs();
        let optimized = optimize_shared_policy_decisions(
            &world_set,
            "world-a",
            &table,
            protocol,
            &costs,
            Rational::one(),
            &config(2),
        )
        .unwrap();
        let initial: BeliefVector = world_set
            .iter()
            .map(|world| Some(world.kernel.initial_belief.clone()))
            .collect();
        let mut generated = 0;
        let exhaustive = exhaustive_frontier(
            &world_set,
            &table,
            protocol,
            &costs,
            Rational::one(),
            &initial,
            2,
            200_000,
            &mut generated,
        )
        .unwrap();
        let ids = vec!["world-a".to_string(), "world-b".to_string()];
        let exhaustive_points = frontier_points_from_candidates(exhaustive, &ids);
        assert_eq!(
            frontier_objectives(&optimized.frontier),
            frontier_objectives(&exhaustive_points)
        );

        for point in &optimized.frontier {
            let replay = replay_shared_policy_across_worlds(
                &world_set,
                "world-a",
                &table,
                protocol,
                &costs,
                Rational::one(),
                &point.plan,
                2,
            )
            .unwrap();
            assert_eq!(replay.per_world, point.per_world);
        }
    }

    #[test]
    fn singleton_world_family_reduces_to_the_one_world_decision_dp() {
        let world = make_inverted_instrument_world("only", false, 0);
        let table = loss_table();
        let protocol = ObservationProtocol::fully_observed();
        let costs = costs();
        let robust = optimize_shared_policy_decisions(
            &[world.clone()],
            "only",
            &table,
            protocol,
            &costs,
            Rational::one(),
            &config(2),
        )
        .unwrap();
        let nominal = super::super::finite_horizon::optimize_finite_horizon_decisions(
            &world.kernel,
            &table,
            protocol,
            &costs,
            Rational::one(),
            &config(2),
        )
        .unwrap();
        let robust_as_one_world = robust
            .frontier
            .iter()
            .map(|point| {
                point.per_world.iter().map(|world| {
                    (
                        world.expected_experiment_cost,
                        world.expected_terminal_loss,
                        world.hazard_probability,
                        world.abstention_probability,
                    )
                }).collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let nominal_objectives = super::super::finite_horizon::frontier_objectives(&nominal.frontier)
            .into_iter()
            .map(|objective| vec![objective])
            .collect::<Vec<_>>();
        let mut robust_objectives = robust_as_one_world;
        robust_objectives.sort();
        robust_objectives.dedup();
        assert_eq!(robust_objectives, nominal_objectives);
    }

    #[test]
    fn model_world_input_order_does_not_change_frontier_or_receipt_order() {
        let forward = worlds();
        let reversed = vec![forward[1].clone(), forward[0].clone()];
        let a = optimize_shared_policy_decisions(
            &forward,
            "world-a",
            &loss_table(),
            ObservationProtocol::fully_observed(),
            &costs(),
            Rational::one(),
            &config(2),
        )
        .unwrap();
        let b = optimize_shared_policy_decisions(
            &reversed,
            "world-a",
            &loss_table(),
            ObservationProtocol::fully_observed(),
            &costs(),
            Rational::one(),
            &config(2),
        )
        .unwrap();
        assert_eq!(a.world_ids, b.world_ids);
        assert_eq!(frontier_objectives(&a.frontier), frontier_objectives(&b.frontier));
    }

    #[test]
    fn different_world_observation_probabilities_are_preserved_exactly() {
        let worlds = vec![
            stochastic_world(
                "world-a",
                Rational::new(3, 4).unwrap(),
                Rational::new(1, 4).unwrap(),
            ),
            stochastic_world(
                "world-b",
                Rational::new(19, 20).unwrap(),
                Rational::new(2, 5).unwrap(),
            ),
        ];
        let observation = PublicObservation {
            hazard: false,
            readback: ObservationField::Value(0),
            measurement: ObservationField::Value(0),
        };
        let probability_a = crate::filter_one_step(
            &worlds[0].kernel,
            &worlds[0].kernel.initial_belief,
            0,
            observation,
        )
        .unwrap()
        .observation_likelihood;
        let probability_b = crate::filter_one_step(
            &worlds[1].kernel,
            &worlds[1].kernel.initial_belief,
            0,
            observation,
        )
        .unwrap()
        .observation_likelihood;
        assert_ne!(probability_a, probability_b);

        let table = loss_table();
        let protocol = ObservationProtocol::fully_observed();
        let costs = vec![Rational::new(1, 10).unwrap()];
        let optimized = optimize_shared_policy_decisions(
            &worlds,
            "world-a",
            &table,
            protocol,
            &costs,
            Rational::one(),
            &config(2),
        )
        .unwrap();
        let initial: BeliefVector = worlds
            .iter()
            .map(|world| Some(world.kernel.initial_belief.clone()))
            .collect();
        let mut generated = 0;
        let exhaustive = exhaustive_frontier(
            &worlds,
            &table,
            protocol,
            &costs,
            Rational::one(),
            &initial,
            2,
            100_000,
            &mut generated,
        )
        .unwrap();
        let oracle = frontier_points_from_candidates(
            exhaustive,
            &["world-a".to_string(), "world-b".to_string()],
        );
        assert_eq!(
            frontier_objectives(&optimized.frontier),
            frontier_objectives(&oracle)
        );
        for point in &optimized.frontier {
            let raw_replay = replay_shared_policy_across_worlds(
                &worlds,
                "world-a",
                &table,
                protocol,
                &costs,
                Rational::one(),
                &point.plan,
                2,
            )
            .unwrap();
            assert_eq!(raw_replay.per_world, point.per_world);
        }
    }

    #[test]
    fn zero_worst_case_hazard_budget_rejects_a_nominally_safe_shifted_command() {
        let worlds = vec![
            make_inverted_instrument_world("world-nominal", false, 0),
            hazard_shift_world(),
        ];
        let nominal = super::super::finite_horizon::optimize_finite_horizon_decisions(
            &worlds[0].kernel,
            &loss_table(),
            ObservationProtocol::fully_observed(),
            &costs(),
            Rational::one(),
            &config(1),
        )
        .unwrap();
        let nominal_plan = nominal
            .minimum_loss_under(DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::one(),
            })
            .unwrap()
            .unwrap();
        assert!(matches!(
            &nominal_plan.plan,
            &DecisionPlan::Experiment { command: 1, .. }
        ));
        let shifted_replay = replay_shared_policy_across_worlds(
            &worlds,
            "world-nominal",
            &loss_table(),
            ObservationProtocol::fully_observed(),
            &costs(),
            Rational::one(),
            &nominal_plan.plan,
            1,
        )
        .unwrap();
        assert_eq!(shifted_replay.per_world[1].hazard_probability, Rational::new(1, 10).unwrap());

        let result = optimize_shared_policy_decisions(
            &worlds,
            "world-nominal",
            &loss_table(),
            ObservationProtocol::fully_observed(),
            &costs(),
            Rational::one(),
            &config(1),
        )
        .unwrap();
        let selected = result
            .minimum_worst_case_total_loss_under(DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::one(),
            })
            .unwrap()
            .unwrap();
        assert_eq!(selected.worst_case_hazard_probability(), Rational::zero());
        assert!(!matches!(
            &selected.plan,
            &DecisionPlan::Experiment { command: 1, .. }
        ));
    }

    #[test]
    fn overlapping_worlds_cannot_select_world_specific_terminal_decisions() {
        let worlds = vec![
            make_inverted_instrument_world("world-a", false, 0),
            make_inverted_instrument_world("world-b", true, 0),
        ];
        let result = optimize_shared_policy_decisions(
            &worlds,
            "world-a",
            &loss_table(),
            ObservationProtocol::fully_observed(),
            &costs(),
            Rational::one(),
            &config(1),
        )
        .unwrap();
        let selected = result
            .minimum_worst_case_total_loss_under(DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::one(),
            })
            .unwrap()
            .unwrap();
        assert_eq!(selected.worst_case_total_loss().unwrap(), Rational::new(1, 2).unwrap());
        assert!(matches!(
            &selected.plan,
            &DecisionPlan::TerminalDecision { decision: Some(_) }
        ));
        assert_eq!(
            selected.per_world[0].expected_terminal_loss,
            selected.per_world[1].expected_terminal_loss
        );
    }

    #[test]
    fn abstention_is_a_shared_terminal_action_and_can_be_robustly_optimal() {
        let worlds = worlds();
        let mut table = loss_table();
        table.abstention_loss = Rational::zero();
        let result = optimize_shared_policy_decisions(
            &worlds,
            "world-a",
            &table,
            ObservationProtocol::fully_observed(),
            &costs(),
            Rational::one(),
            &config(0),
        )
        .unwrap();
        let selected = result
            .minimum_worst_case_total_loss_under(DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::one(),
            })
            .unwrap()
            .unwrap();
        assert_eq!(
            selected.plan,
            DecisionPlan::TerminalDecision { decision: None }
        );
        assert_eq!(selected.worst_case_abstention_probability(), Rational::one());
    }

    #[test]
    fn finite_support_tv_ambiguity_interpolates_nominal_and_worst_case_loss() {
        let losses = vec![Rational::zero(), Rational::one()];
        let weights = vec![half(), half()];
        assert_eq!(
            worst_case_tv_expectation(&losses, &weights, Rational::zero()).unwrap(),
            half()
        );
        assert_eq!(
            worst_case_tv_expectation(
                &losses,
                &weights,
                Rational::new(1, 4).unwrap(),
            )
            .unwrap(),
            Rational::new(3, 4).unwrap()
        );
        assert_eq!(
            worst_case_tv_expectation(&losses, &weights, half()).unwrap(),
            Rational::one()
        );
        assert_eq!(
            worst_case_tv_expectation(&losses, &weights, Rational::one()).unwrap(),
            Rational::one()
        );
    }

    #[test]
    fn finite_support_tv_transfers_mass_across_multiple_loss_tiers_exactly() {
        let losses = vec![
            Rational::zero(),
            Rational::one(),
            Rational::from_u64(2),
        ];
        let weights = vec![
            Rational::new(1, 10).unwrap(),
            Rational::new(6, 10).unwrap(),
            Rational::new(3, 10).unwrap(),
        ];
        assert_eq!(
            worst_case_tv_expectation(
                &losses,
                &weights,
                Rational::new(1, 5).unwrap(),
            )
            .unwrap(),
            Rational::new(3, 2).unwrap()
        );
    }

    #[test]
    fn finite_support_tv_respects_zero_weight_and_single_world_edges() {
        assert_eq!(
            worst_case_tv_expectation(
                &[Rational::zero(), Rational::one()],
                &[Rational::zero(), Rational::one()],
                Rational::one(),
            )
            .unwrap(),
            Rational::one()
        );
        assert_eq!(
            worst_case_tv_expectation(
                &[Rational::new(7, 3).unwrap()],
                &[Rational::one()],
                Rational::one(),
            )
            .unwrap(),
            Rational::new(7, 3).unwrap()
        );
    }

    #[test]
    fn finite_support_tv_is_invariant_to_world_order_after_canonicalization() {
        let worlds = worlds();
        let table = loss_table();
        let protocol = ObservationProtocol::fully_observed();
        let costs = costs();
        let forward = optimize_shared_policy_decisions(
            &worlds,
            "world-a",
            &table,
            protocol,
            &costs,
            Rational::one(),
            &config(2),
        )
        .unwrap();
        let reversed = optimize_shared_policy_decisions(
            &[worlds[1].clone(), worlds[0].clone()],
            "world-a",
            &table,
            protocol,
            &costs,
            Rational::one(),
            &config(2),
        )
        .unwrap();
        let weights = vec![Rational::new(3, 4).unwrap(), Rational::new(1, 4).unwrap()];
        let budgets = DecisionBudgets {
            max_hazard_probability: Rational::zero(),
            max_abstention_probability: Rational::one(),
        };
        let a = forward
            .minimum_tv_expected_total_loss_under(&weights, Rational::new(1, 5).unwrap(), budgets)
            .unwrap()
            .unwrap();
        let b = reversed
            .minimum_tv_expected_total_loss_under(&weights, Rational::new(1, 5).unwrap(), budgets)
            .unwrap()
            .unwrap();
        assert_eq!(a.plan, b.plan);
        assert_eq!(
            a.worst_case_total_loss_under_tv(&weights, Rational::new(1, 5).unwrap()).unwrap(),
            b.worst_case_total_loss_under_tv(&weights, Rational::new(1, 5).unwrap()).unwrap()
        );
    }

    #[test]
    fn tv_radius_zero_and_one_recover_nominal_and_minimax_selectors() {
        let result = optimize_shared_policy_decisions(
            &worlds(),
            "world-a",
            &loss_table(),
            ObservationProtocol::fully_observed(),
            &costs(),
            Rational::one(),
            &config(2),
        )
        .unwrap();
        let weights = vec![Rational::one(), Rational::zero()];
        let budgets = DecisionBudgets {
            max_hazard_probability: Rational::zero(),
            max_abstention_probability: Rational::one(),
        };
        let nominal = result
            .minimum_nominal_total_loss_under(budgets, None)
            .unwrap()
            .unwrap();
        let tv_zero = result
            .minimum_tv_expected_total_loss_under(&weights, Rational::zero(), budgets)
            .unwrap()
            .unwrap();
        assert_eq!(
            nominal.per_world[0].expected_total_loss().unwrap(),
            tv_zero.per_world[0].expected_total_loss().unwrap()
        );

        let minimax = result
            .minimum_worst_case_total_loss_under(budgets)
            .unwrap()
            .unwrap();
        let tv_one = result
            .minimum_tv_expected_total_loss_under(&weights, Rational::one(), budgets)
            .unwrap()
            .unwrap();
        assert_eq!(
            minimax.worst_case_total_loss().unwrap(),
            tv_one.worst_case_total_loss().unwrap()
        );
    }

    #[test]
    fn total_variation_weight_contract_fails_closed() {
        let losses = vec![Rational::zero(), Rational::one()];
        assert!(worst_case_tv_expectation(
            &losses,
            &[Rational::new(1, 4).unwrap(), Rational::new(1, 4).unwrap()],
            half(),
        )
        .is_err());
        assert!(worst_case_tv_expectation(
            &losses,
            &[half(), half()],
            Rational::new(3, 2).unwrap(),
        )
        .is_err());
        assert!(worst_case_tv_expectation(
            &losses,
            &[half()],
            half(),
        )
        .is_err());
    }

    #[test]
    fn resource_caps_and_model_family_size_fail_closed() {
        let worlds = worlds();
        let table = loss_table();
        let protocol = ObservationProtocol::fully_observed();
        let costs = costs();

        let mut no_memo = config(1);
        no_memo.max_memo_states = 0;
        assert!(optimize_shared_policy_decisions(
            &worlds,
            "world-a",
            &table,
            protocol,
            &costs,
            Rational::one(),
            &no_memo,
        )
        .is_err());

        let mut oversized = Vec::new();
        for index in 0..=MAX_MODEL_WORLDS {
            oversized.push(make_inverted_instrument_world(
                &format!("world-{index:02}"),
                false,
                0,
            ));
        }
        assert!(optimize_shared_policy_decisions(
            &oversized,
            "world-00",
            &table,
            protocol,
            &costs,
            Rational::one(),
            &config(0),
        )
        .is_err());
    }

    #[test]
    fn replay_rejects_an_observation_outside_the_declared_world_family() {
        let worlds = worlds();
        let known_zero = PublicObservation {
            hazard: false,
            readback: ObservationField::Value(0),
            measurement: ObservationField::Value(0),
        };
        let known_one = PublicObservation {
            measurement: ObservationField::Value(1),
            ..known_zero
        };
        let impossible = PublicObservation {
            readback: ObservationField::Value(1),
            ..known_zero
        };
        let plan = DecisionPlan::Experiment {
            command: 0,
            branches: vec![
                DecisionBranch {
                    observation: known_zero,
                    plan: Box::new(DecisionPlan::TerminalDecision { decision: Some(0) }),
                },
                DecisionBranch {
                    observation: known_one,
                    plan: Box::new(DecisionPlan::TerminalDecision { decision: Some(1) }),
                },
                DecisionBranch {
                    observation: impossible,
                    plan: Box::new(DecisionPlan::TerminalDecision { decision: Some(0) }),
                },
            ],
        };
        assert!(replay_shared_policy_across_worlds(
            &worlds,
            "world-a",
            &loss_table(),
            ObservationProtocol::fully_observed(),
            &costs(),
            Rational::one(),
            &plan,
            1,
        )
        .is_err());
    }

    #[test]
    fn incompatible_model_world_semantics_fail_closed() {
        let mut incompatible = worlds();
        incompatible[1].kernel.measurement_ids.swap(0, 1);
        assert!(optimize_shared_policy_decisions(
            &incompatible,
            "world-a",
            &loss_table(),
            ObservationProtocol::fully_observed(),
            &costs(),
            Rational::one(),
            &config(1),
        )
        .is_err());
    }
}
