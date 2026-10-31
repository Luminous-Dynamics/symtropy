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

    let initial_beliefs: BeliefVector = canonical_worlds
        .iter()
        .map(|world| {
            let mut belief = world.kernel.initial_belief.clone();
            belief.sort_by_key(|point| point.state);
            Some(belief)
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
    if !ids.contains(nominal_world_id) {
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
        let mut per_world = Vec::with_capacity(worlds.len());
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

fn worst_metric(
    metrics: &[WorldObjectives],
    select: impl Fn(&WorldObjectives) -> Rational,
) -> Rational {
    metrics
        .iter()
        .map(select)
        .max()
        .unwrap_or_else(Rational::zero)
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
    let initial_beliefs: BeliefVector = canonical_worlds
        .iter()
        .map(|world| {
            let mut belief = world.kernel.initial_belief.clone();
            belief.sort_by_key(|point| point.state);
            Some(belief)
        })
        .collect();
    if matches!(plan, DecisionPlan::HazardTerminal) {
        return Err("a campaign cannot begin in a hazard-terminal state".into());
    }
    let metrics = replay_node(
        &canonical_worlds,
        table,
        protocol,
        command_costs,
        hazard_loss,
        plan,
        &initial_beliefs,
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

#[allow(clippy::too_many_arguments)]
fn replay_node(
    worlds: &[DecisionModelWorld],
    table: &DecisionLossTable,
    protocol: ObservationProtocol,
    command_costs: &[Rational],
    hazard_loss: Rational,
    plan: &DecisionPlan,
    beliefs: &BeliefVector,
    remaining_horizon: usize,
) -> Result<Vec<WorldObjectives>, String> {
    match plan {
        DecisionPlan::TerminalDecision { decision } => {
            let mut metrics = Vec::with_capacity(worlds.len());
            for belief in beliefs {
                if let Some(belief) = belief {
                    metrics.push(WorldObjectives {
                        expected_terminal_loss: expected_terminal_loss(table, belief, *decision)?,
                        abstention_probability: if decision.is_none() {
                            Rational::one()
                        } else {
                            Rational::zero()
                        },
                        ..WorldObjectives::zero()
                    });
                } else {
                    metrics.push(WorldObjectives::zero());
                }
            }
            Ok(metrics)
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
            let partitions =
                command_partitions_for_beliefs(worlds, beliefs, *command, protocol)?;
            let observations = union_observations(&partitions);
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

            let mut metrics: Vec<WorldObjectives> = beliefs
                .iter()
                .map(|belief| {
                    if belief.is_some() {
                        WorldObjectives {
                            expected_experiment_cost: command_cost,
                            ..WorldObjectives::zero()
                        }
                    } else {
                        WorldObjectives::zero()
                    }
                })
                .collect();

            for observation in observations {
                let (probabilities, branch_beliefs) =
                    branch_vectors(&partitions, &observation);
                let child_plan = branch_map[&observation];
                let child_metrics = if observation.hazard {
                    if *child_plan != DecisionPlan::HazardTerminal {
                        return Err("observed hazard branch must terminate immediately".into());
                    }
                    probabilities
                        .iter()
                        .map(|probability| {
                            if probability.is_zero() {
                                WorldObjectives::zero()
                            } else {
                                WorldObjectives::hazard_terminal(hazard_loss)
                            }
                        })
                        .collect()
                } else {
                    if *child_plan == DecisionPlan::HazardTerminal {
                        return Err("non-hazard observation cannot use a hazard terminal".into());
                    }
                    replay_node(
                        worlds,
                        table,
                        protocol,
                        command_costs,
                        hazard_loss,
                        child_plan,
                        &branch_beliefs,
                        remaining_horizon - 1,
                    )?
                };
                for index in 0..metrics.len() {
                    if !probabilities[index].is_zero() {
                        metrics[index].add_scaled(child_metrics[index], probabilities[index])?;
                    }
                }
            }
            Ok(metrics)
        }
    }
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
            independent_same_objectives(&other.per_world, &candidate.per_world)
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
            independent_same_objectives(&other.per_world, &candidate.per_world)
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
