//! Exact finite-horizon decision-tree optimization over action-realization-aware beliefs.
//!
//! This module uses exact rational expected experiment cost, terminal loss,
//! campaign hazard probability and abstention probability. It keeps the full
//! non-dominated frontier and supplies independent non-memoized enumeration
//! and policy replay for small-domain verification.

use super::{DecisionLossTable, ObservationProtocol, bayes_decision_from_belief};
use crate::{
    ActionRealizationKernel, BeliefMass, ObservationField, PublicObservation, Rational,
    filter_one_step,
};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_DECISION_HORIZON: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecisionBudgets {
    pub max_hazard_probability: Rational,
    pub max_abstention_probability: Rational,
}

impl DecisionBudgets {
    pub fn validate(self) -> Result<(), String> {
        if !self.max_hazard_probability.is_probability()
            || !self.max_abstention_probability.is_probability()
        {
            return Err("hazard and abstention budgets must be probabilities".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionSolverConfig {
    pub horizon: usize,
    pub max_memo_states: usize,
    pub max_frontier_points: usize,
    pub max_generated_combinations: usize,
}

impl Default for DecisionSolverConfig {
    fn default() -> Self {
        Self {
            horizon: 3,
            max_memo_states: 100_000,
            max_frontier_points: 20_000,
            max_generated_combinations: 1_000_000,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DecisionPlan {
    /// Stop and take the Bayes-optimal terminal decision; None means abstain.
    TerminalDecision { decision: Option<usize> },
    /// A hazard has already occurred; no continuation is permitted.
    HazardTerminal,
    /// Run one command and select a continuation using only its public outcome.
    Experiment {
        command: usize,
        branches: Vec<DecisionBranch>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct DecisionBranch {
    pub observation: PublicObservation,
    pub plan: Box<DecisionPlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionFrontierPoint {
    pub expected_experiment_cost: Rational,
    pub expected_terminal_loss: Rational,
    pub hazard_probability: Rational,
    pub abstention_probability: Rational,
    pub plan: DecisionPlan,
}

impl DecisionFrontierPoint {
    pub fn expected_total_loss(&self) -> Result<Rational, String> {
        self.expected_experiment_cost
            .checked_add(self.expected_terminal_loss)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct DecisionSolverStats {
    pub memo_states: usize,
    pub generated_combinations: usize,
    pub maximum_frontier_points: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionOptimizationResult {
    pub frontier: Vec<DecisionFrontierPoint>,
    pub stats: DecisionSolverStats,
}

impl DecisionOptimizationResult {
    /// Select the minimum cost-plus-terminal-loss feasible policy.
    ///
    /// Exact ties are resolved by lower hazard, then lower abstention, then
    /// lower experiment cost, then canonical policy order.
    pub fn minimum_loss_under(
        &self,
        budgets: DecisionBudgets,
    ) -> Result<Option<&DecisionFrontierPoint>, String> {
        budgets.validate()?;
        let mut selected: Option<&DecisionFrontierPoint> = None;
        for point in &self.frontier {
            if point.hazard_probability > budgets.max_hazard_probability
                || point.abstention_probability > budgets.max_abstention_probability
            {
                continue;
            }
            let Some(current) = selected else {
                selected = Some(point);
                continue;
            };
            if selected_order(point, current)? == Ordering::Less {
                selected = Some(point);
            }
        }
        Ok(selected)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BeliefKey(Vec<(crate::LatentState, Rational)>);

impl BeliefKey {
    fn new(belief: &[BeliefMass]) -> Self {
        let mut points: Vec<_> = belief
            .iter()
            .map(|point| (point.state, point.mass))
            .collect();
        points.sort();
        Self(points)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct MemoKey {
    belief: BeliefKey,
    remaining_horizon: usize,
}

#[derive(Clone)]
struct PartialCombination {
    expected_experiment_cost: Rational,
    expected_terminal_loss: Rational,
    hazard_probability: Rational,
    abstention_probability: Rational,
    branches: Vec<DecisionBranch>,
}

struct DecisionOptimizer<'a> {
    kernel: &'a ActionRealizationKernel,
    table: &'a DecisionLossTable,
    protocol: ObservationProtocol,
    command_costs: &'a [Rational],
    hazard_loss: Rational,
    config: &'a DecisionSolverConfig,
    memo: BTreeMap<MemoKey, Vec<DecisionFrontierPoint>>,
    stats: DecisionSolverStats,
}

impl<'a> DecisionOptimizer<'a> {
    fn solve(
        &mut self,
        belief: &[BeliefMass],
        remaining_horizon: usize,
    ) -> Result<Vec<DecisionFrontierPoint>, String> {
        let key = MemoKey {
            belief: BeliefKey::new(belief),
            remaining_horizon,
        };
        if let Some(cached) = self.memo.get(&key) {
            return Ok(cached.clone());
        }

        self.stats.memo_states = self
            .stats
            .memo_states
            .checked_add(1)
            .ok_or_else(|| "memo-state counter overflow".to_string())?;
        if self.stats.memo_states > self.config.max_memo_states {
            return Err("decision optimizer memo-state budget exceeded".into());
        }

        let stop = terminal_point(self.kernel, self.table, belief)?;
        let mut candidates = vec![stop];

        if remaining_horizon > 0 {
            for command in 0..self.command_costs.len() {
                let observations = public_observations(
                    self.kernel,
                    belief,
                    command,
                    self.protocol,
                );
                let mut partials = vec![PartialCombination {
                    expected_experiment_cost: self.command_costs[command],
                    expected_terminal_loss: Rational::zero(),
                    hazard_probability: Rational::zero(),
                    abstention_probability: Rational::zero(),
                    branches: Vec::new(),
                }];

                for observation in observations {
                    let filtered = filter_one_step(
                        self.kernel,
                        belief,
                        command,
                        observation,
                    )
                    .map_err(|error| format!("decision filter failed: {error}"))?;
                    let branch_probability = filtered.observation_likelihood;
                    let child_frontier = if filtered.terminal_hazard {
                        vec![hazard_terminal_point(self.hazard_loss)]
                    } else {
                        let next_belief = filtered
                            .belief_for_next_step()
                            .map_err(|error| format!("next belief failed: {error}"))?;
                        self.solve(&next_belief, remaining_horizon - 1)?
                    };

                    let mut next_partials = Vec::new();
                    for partial in &partials {
                        for child in &child_frontier {
                            self.bump_combinations()?;
                            let mut next = partial.clone();
                            next.expected_experiment_cost = next
                                .expected_experiment_cost
                                .checked_add(
                                    branch_probability
                                        .checked_mul(child.expected_experiment_cost)?,
                                )?;
                            next.expected_terminal_loss = next
                                .expected_terminal_loss
                                .checked_add(
                                    branch_probability
                                        .checked_mul(child.expected_terminal_loss)?,
                                )?;
                            next.hazard_probability = next
                                .hazard_probability
                                .checked_add(
                                    branch_probability
                                        .checked_mul(child.hazard_probability)?,
                                )?;
                            next.abstention_probability = next
                                .abstention_probability
                                .checked_add(
                                    branch_probability
                                        .checked_mul(child.abstention_probability)?,
                                )?;
                            next.branches.push(DecisionBranch {
                                observation,
                                plan: Box::new(child.plan.clone()),
                            });
                            next_partials.push(next);
                        }
                    }
                    let partial_frontier = prune_partial(next_partials)?;
                    if partial_frontier.len() > self.config.max_frontier_points {
                        return Err("decision optimizer partial frontier cap exceeded".into());
                    }
                    partials = partial_frontier;
                }

                for partial in partials {
                    candidates.push(DecisionFrontierPoint {
                        expected_experiment_cost: partial.expected_experiment_cost,
                        expected_terminal_loss: partial.expected_terminal_loss,
                        hazard_probability: partial.hazard_probability,
                        abstention_probability: partial.abstention_probability,
                        plan: DecisionPlan::Experiment {
                            command,
                            branches: partial.branches,
                        },
                    });
                }
            }
        }

        let frontier = prune_frontier(candidates)?;
        if frontier.len() > self.config.max_frontier_points {
            return Err("decision optimizer frontier-point budget exceeded".into());
        }
        self.stats.maximum_frontier_points = self
            .stats
            .maximum_frontier_points
            .max(frontier.len());
        self.memo.insert(key, frontier.clone());
        Ok(frontier)
    }

    fn bump_combinations(&mut self) -> Result<(), String> {
        self.stats.generated_combinations = self
            .stats
            .generated_combinations
            .checked_add(1)
            .ok_or_else(|| "decision combination counter overflow".to_string())?;
        if self.stats.generated_combinations > self.config.max_generated_combinations {
            return Err("decision optimizer generated-combination budget exceeded".into());
        }
        Ok(())
    }
}

/// Optimize all finite-horizon public-observation policies exactly.
///
/// The objective frontier retains experiment cost, terminal Bayes loss, campaign
/// hazard probability and abstention probability separately. The same policy
/// branches on the public observation only; hidden state and realized action are
/// never exposed to the selector.
pub fn optimize_finite_horizon_decisions(
    kernel: &ActionRealizationKernel,
    table: &DecisionLossTable,
    protocol: ObservationProtocol,
    command_costs: &[Rational],
    hazard_loss: Rational,
    config: &DecisionSolverConfig,
) -> Result<DecisionOptimizationResult, String> {
    kernel.validate()?;
    table.validate(kernel)?;
    if command_costs.len() != kernel.command_ids.len() {
        return Err("one command cost is required for every command".into());
    }
    if config.horizon > MAX_DECISION_HORIZON {
        return Err("decision horizon exceeds the configured maximum".into());
    }
    if config.max_memo_states == 0
        || config.max_frontier_points == 0
        || config.max_generated_combinations == 0
    {
        return Err("all decision solver resource caps must be positive".into());
    }

    let mut optimizer = DecisionOptimizer {
        kernel,
        table,
        protocol,
        command_costs,
        hazard_loss,
        config,
        memo: BTreeMap::new(),
        stats: DecisionSolverStats::default(),
    };
    let frontier = optimizer.solve(&kernel.initial_belief, config.horizon)?;
    Ok(DecisionOptimizationResult {
        frontier,
        stats: optimizer.stats,
    })
}

fn terminal_point(
    kernel: &ActionRealizationKernel,
    table: &DecisionLossTable,
    belief: &[BeliefMass],
) -> Result<DecisionFrontierPoint, String> {
    let decision = bayes_decision_from_belief(kernel, belief, table)
        .map_err(|error| format!("Bayes decision failed: {error}"))?;
    Ok(DecisionFrontierPoint {
        expected_experiment_cost: Rational::zero(),
        expected_terminal_loss: decision.expected_loss,
        hazard_probability: Rational::zero(),
        abstention_probability: if decision.decision.is_none() {
            Rational::one()
        } else {
            Rational::zero()
        },
        plan: DecisionPlan::TerminalDecision {
            decision: decision.decision,
        },
    })
}

fn hazard_terminal_point(hazard_loss: Rational) -> DecisionFrontierPoint {
    DecisionFrontierPoint {
        expected_experiment_cost: Rational::zero(),
        expected_terminal_loss: hazard_loss,
        hazard_probability: Rational::one(),
        abstention_probability: Rational::zero(),
        plan: DecisionPlan::HazardTerminal,
    }
}

fn public_observations(
    kernel: &ActionRealizationKernel,
    belief: &[BeliefMass],
    command: usize,
    protocol: ObservationProtocol,
) -> Vec<PublicObservation> {
    let mut observations = BTreeSet::new();
    for point in belief {
        let row = &kernel.rows[kernel.row_index(point.state, command)];
        for outcome in &row.outcomes {
            observations.insert(PublicObservation {
                hazard: outcome.hazard,
                readback: visible_field(outcome.readback, protocol.observe_readback),
                measurement: visible_field(outcome.measurement, protocol.observe_measurement),
            });
        }
    }
    observations.into_iter().collect()
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

fn partial_dominates(left: &PartialCombination, right: &PartialCombination) -> bool {
    let no_worse = left.expected_experiment_cost <= right.expected_experiment_cost
        && left.expected_terminal_loss <= right.expected_terminal_loss
        && left.hazard_probability <= right.hazard_probability
        && left.abstention_probability <= right.abstention_probability;
    let strictly_better = left.expected_experiment_cost < right.expected_experiment_cost
        || left.expected_terminal_loss < right.expected_terminal_loss
        || left.hazard_probability < right.hazard_probability
        || left.abstention_probability < right.abstention_probability;
    no_worse && strictly_better
}

fn prune_partial(
    mut candidates: Vec<PartialCombination>,
) -> Result<Vec<PartialCombination>, String> {
    candidates.sort_by(|left, right| {
        left.expected_experiment_cost
            .cmp(&right.expected_experiment_cost)
            .then_with(|| left.expected_terminal_loss.cmp(&right.expected_terminal_loss))
            .then_with(|| left.hazard_probability.cmp(&right.hazard_probability))
            .then_with(|| left.abstention_probability.cmp(&right.abstention_probability))
            .then_with(|| left.branches.cmp(&right.branches))
    });
    let mut frontier: Vec<PartialCombination> = Vec::new();
    for candidate in candidates {
        if frontier.iter().any(|other| {
            same_partial_objectives(other, &candidate) || partial_dominates(other, &candidate)
        }) {
            continue;
        }
        frontier.retain(|other| !partial_dominates(&candidate, other));
        frontier.push(candidate);
    }
    Ok(frontier)
}

fn same_partial_objectives(left: &PartialCombination, right: &PartialCombination) -> bool {
    left.expected_experiment_cost == right.expected_experiment_cost
        && left.expected_terminal_loss == right.expected_terminal_loss
        && left.hazard_probability == right.hazard_probability
        && left.abstention_probability == right.abstention_probability
}

fn frontier_dominates(left: &DecisionFrontierPoint, right: &DecisionFrontierPoint) -> bool {
    let no_worse = left.expected_experiment_cost <= right.expected_experiment_cost
        && left.expected_terminal_loss <= right.expected_terminal_loss
        && left.hazard_probability <= right.hazard_probability
        && left.abstention_probability <= right.abstention_probability;
    let strictly_better = left.expected_experiment_cost < right.expected_experiment_cost
        || left.expected_terminal_loss < right.expected_terminal_loss
        || left.hazard_probability < right.hazard_probability
        || left.abstention_probability < right.abstention_probability;
    no_worse && strictly_better
}

fn same_frontier_objectives(
    left: &DecisionFrontierPoint,
    right: &DecisionFrontierPoint,
) -> bool {
    left.expected_experiment_cost == right.expected_experiment_cost
        && left.expected_terminal_loss == right.expected_terminal_loss
        && left.hazard_probability == right.hazard_probability
        && left.abstention_probability == right.abstention_probability
}

fn frontier_order(
    left: &DecisionFrontierPoint,
    right: &DecisionFrontierPoint,
) -> Ordering {
    left.expected_experiment_cost
        .cmp(&right.expected_experiment_cost)
        .then_with(|| left.expected_terminal_loss.cmp(&right.expected_terminal_loss))
        .then_with(|| left.hazard_probability.cmp(&right.hazard_probability))
        .then_with(|| left.abstention_probability.cmp(&right.abstention_probability))
        .then_with(|| left.plan.cmp(&right.plan))
}

fn prune_frontier(
    mut candidates: Vec<DecisionFrontierPoint>,
) -> Result<Vec<DecisionFrontierPoint>, String> {
    candidates.sort_by(frontier_order);
    let mut frontier: Vec<DecisionFrontierPoint> = Vec::new();
    for candidate in candidates {
        if frontier.iter().any(|other| {
            same_frontier_objectives(other, &candidate)
                || frontier_dominates(other, &candidate)
        }) {
            continue;
        }
        frontier.retain(|other| !frontier_dominates(&candidate, other));
        frontier.push(candidate);
    }
    Ok(frontier)
}

fn selected_order(
    left: &DecisionFrontierPoint,
    right: &DecisionFrontierPoint,
) -> Result<Ordering, String> {
    Ok(left
        .expected_total_loss()?
        .cmp(&right.expected_total_loss()?)
        .then_with(|| left.hazard_probability.cmp(&right.hazard_probability))
        .then_with(|| left.abstention_probability.cmp(&right.abstention_probability))
        .then_with(|| left.expected_experiment_cost.cmp(&right.expected_experiment_cost))
        .then_with(|| left.plan.cmp(&right.plan)))
}

/// Independently replay a returned policy against the exact kernel.
///
/// Every reachable public observation must have exactly one branch. A hazard
/// branch must terminate, and a policy cannot continue beyond its horizon.
pub fn replay_decision_policy(
    kernel: &ActionRealizationKernel,
    table: &DecisionLossTable,
    protocol: ObservationProtocol,
    command_costs: &[Rational],
    hazard_loss: Rational,
    plan: &DecisionPlan,
    horizon: usize,
) -> Result<DecisionFrontierPoint, String> {
    kernel.validate()?;
    table.validate(kernel)?;
    if command_costs.len() != kernel.command_ids.len() {
        return Err("one command cost is required for every command".into());
    }
    replay_from_belief(
        kernel,
        table,
        protocol,
        command_costs,
        hazard_loss,
        plan,
        &kernel.initial_belief,
        horizon,
    )
}

fn replay_from_belief(
    kernel: &ActionRealizationKernel,
    table: &DecisionLossTable,
    protocol: ObservationProtocol,
    command_costs: &[Rational],
    hazard_loss: Rational,
    plan: &DecisionPlan,
    belief: &[BeliefMass],
    remaining_horizon: usize,
) -> Result<DecisionFrontierPoint, String> {
    match plan {
        DecisionPlan::TerminalDecision { decision } => {
            let bayes = bayes_decision_from_belief(kernel, belief, table)
                .map_err(|error| format!("Bayes decision failed: {error}"))?;
            if bayes.decision != *decision {
                return Err("terminal action is not the Bayes-optimal decision".into());
            }
            Ok(DecisionFrontierPoint {
                expected_experiment_cost: Rational::zero(),
                expected_terminal_loss: bayes.expected_loss,
                hazard_probability: Rational::zero(),
                abstention_probability: if decision.is_none() {
                    Rational::one()
                } else {
                    Rational::zero()
                },
                plan: plan.clone(),
            })
        }
        DecisionPlan::HazardTerminal => {
            Err("hazard terminal is only valid below an observed hazard branch".into())
        }
        DecisionPlan::Experiment { command, branches } => {
            if remaining_horizon == 0 {
                return Err("policy exceeds its finite horizon".into());
            }
            let Some(command_cost) = command_costs.get(*command).copied() else {
                return Err("policy references an invalid command".into());
            };
            let observations =
                public_observations(kernel, belief, *command, protocol);
            if observations.len() != branches.len() {
                return Err("policy does not contain exactly one branch per public observation".into());
            }
            let branch_map: BTreeMap<_, _> = branches
                .iter()
                .map(|branch| (branch.observation, branch.plan.as_ref()))
                .collect();
            if branch_map.len() != branches.len()
                || observations.iter().any(|observation| !branch_map.contains_key(observation))
            {
                return Err("policy branches contain duplicates or unreachable observations".into());
            }

            let mut expected_experiment_cost = command_cost;
            let mut expected_terminal_loss = Rational::zero();
            let mut hazard_probability = Rational::zero();
            let mut abstention_probability = Rational::zero();
            for observation in observations {
                let filtered = filter_one_step(kernel, belief, *command, observation)
                    .map_err(|error| format!("decision filter failed: {error}"))?;
                let branch_probability = filtered.observation_likelihood;
                let child = branch_map[&observation];
                let child_point = if filtered.terminal_hazard {
                    if *child != DecisionPlan::HazardTerminal {
                        return Err("hazard branch does not terminate".into());
                    }
                    hazard_terminal_point(hazard_loss)
                } else {
                    if *child == DecisionPlan::HazardTerminal {
                        return Err("non-hazard branch uses a hazard terminal".into());
                    }
                    let next_belief = filtered
                        .belief_for_next_step()
                        .map_err(|error| format!("next belief failed: {error}"))?;
                    replay_from_belief(
                        kernel,
                        table,
                        protocol,
                        command_costs,
                        hazard_loss,
                        child,
                        &next_belief,
                        remaining_horizon - 1,
                    )?
                };
                expected_experiment_cost = expected_experiment_cost
                    .checked_add(
                        branch_probability.checked_mul(child_point.expected_experiment_cost)?,
                    )?;
                expected_terminal_loss = expected_terminal_loss
                    .checked_add(
                        branch_probability.checked_mul(child_point.expected_terminal_loss)?,
                    )?;
                hazard_probability = hazard_probability
                    .checked_add(branch_probability.checked_mul(child_point.hazard_probability)?)?;
                abstention_probability = abstention_probability
                    .checked_add(branch_probability.checked_mul(child_point.abstention_probability)?)?;
            }
            Ok(DecisionFrontierPoint {
                expected_experiment_cost,
                expected_terminal_loss,
                hazard_probability,
                abstention_probability,
                plan: plan.clone(),
            })
        }
    }
}

/// Non-memoized exhaustive policy-tree oracle for tiny finite domains.
///
/// It deliberately does not call the production Bellman solver and does not
/// memoize posterior states. Resource limits are explicit because policy-tree
/// enumeration is exponential.
pub fn exhaustive_decision_frontier(
    kernel: &ActionRealizationKernel,
    table: &DecisionLossTable,
    protocol: ObservationProtocol,
    command_costs: &[Rational],
    hazard_loss: Rational,
    horizon: usize,
    max_generated_combinations: usize,
) -> Result<Vec<DecisionFrontierPoint>, String> {
    kernel.validate()?;
    table.validate(kernel)?;
    if command_costs.len() != kernel.command_ids.len() {
        return Err("one command cost is required for every command".into());
    }
    if horizon > MAX_DECISION_HORIZON || max_generated_combinations == 0 {
        return Err("invalid exhaustive horizon or combination cap".into());
    }
    let mut generated = 0_usize;
    let candidates = exhaustive_from_belief(
        kernel,
        table,
        protocol,
        command_costs,
        hazard_loss,
        &kernel.initial_belief,
        horizon,
        max_generated_combinations,
        &mut generated,
    )?;
    prune_frontier(candidates)
}

#[allow(clippy::too_many_arguments)]
fn exhaustive_from_belief(
    kernel: &ActionRealizationKernel,
    table: &DecisionLossTable,
    protocol: ObservationProtocol,
    command_costs: &[Rational],
    hazard_loss: Rational,
    belief: &[BeliefMass],
    remaining_horizon: usize,
    cap: usize,
    generated: &mut usize,
) -> Result<Vec<DecisionFrontierPoint>, String> {
    let mut candidates = vec![terminal_point(kernel, table, belief)?];
    if remaining_horizon == 0 {
        return Ok(candidates);
    }

    for command in 0..command_costs.len() {
        let observations = public_observations(kernel, belief, command, protocol);
        let mut partials = vec![PartialCombination {
            expected_experiment_cost: command_costs[command],
            expected_terminal_loss: Rational::zero(),
            hazard_probability: Rational::zero(),
            abstention_probability: Rational::zero(),
            branches: Vec::new(),
        }];

        for observation in observations {
            let filtered = filter_one_step(kernel, belief, command, observation)
                .map_err(|error| format!("exhaustive filter failed: {error}"))?;
            let child_frontier = if filtered.terminal_hazard {
                vec![hazard_terminal_point(hazard_loss)]
            } else {
                let next_belief = filtered
                    .belief_for_next_step()
                    .map_err(|error| format!("exhaustive next belief failed: {error}"))?;
                exhaustive_from_belief(
                    kernel,
                    table,
                    protocol,
                    command_costs,
                    hazard_loss,
                    &next_belief,
                    remaining_horizon - 1,
                    cap,
                    generated,
                )?
            };

            let mut next_partials = Vec::new();
            for partial in &partials {
                for child in &child_frontier {
                    *generated = generated
                        .checked_add(1)
                        .ok_or_else(|| "exhaustive combination counter overflow".to_string())?;
                    if *generated > cap {
                        return Err("exhaustive policy-tree combination cap exceeded".into());
                    }
                    let mut next = partial.clone();
                    let probability = filtered.observation_likelihood;
                    next.expected_experiment_cost = next
                        .expected_experiment_cost
                        .checked_add(probability.checked_mul(child.expected_experiment_cost)?)?;
                    next.expected_terminal_loss = next
                        .expected_terminal_loss
                        .checked_add(probability.checked_mul(child.expected_terminal_loss)?)?;
                    next.hazard_probability = next
                        .hazard_probability
                        .checked_add(probability.checked_mul(child.hazard_probability)?)?;
                    next.abstention_probability = next
                        .abstention_probability
                        .checked_add(probability.checked_mul(child.abstention_probability)?)?;
                    next.branches.push(DecisionBranch {
                        observation,
                        plan: Box::new(child.plan.clone()),
                    });
                    next_partials.push(next);
                }
            }
            partials = next_partials;
        }

        candidates.extend(partials.into_iter().map(|partial| DecisionFrontierPoint {
            expected_experiment_cost: partial.expected_experiment_cost,
            expected_terminal_loss: partial.expected_terminal_loss,
            hazard_probability: partial.hazard_probability,
            abstention_probability: partial.abstention_probability,
            plan: DecisionPlan::Experiment {
                command,
                branches: partial.branches,
            },
        }));
    }
    Ok(candidates)
}

/// Canonical objective vectors for comparing independent enumerators.
pub fn frontier_objectives(
    frontier: &[DecisionFrontierPoint],
) -> Vec<(Rational, Rational, Rational, Rational)> {
    let mut objectives: Vec<_> = frontier
        .iter()
        .map(|point| {
            (
                point.expected_experiment_cost,
                point.expected_terminal_loss,
                point.hazard_probability,
                point.abstention_probability,
            )
        })
        .collect();
    objectives.sort();
    objectives.dedup();
    objectives
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ActionRealizationKernel, BeliefMass, KernelRow, LatentState, Rational,
        TransitionOutcome,
    };

    fn outcome(
        probability: Rational,
        next_state: LatentState,
        measurement: Option<usize>,
        hazard: bool,
    ) -> TransitionOutcome {
        TransitionOutcome {
            probability,
            next_state,
            realized_action: 0,
            readback: Some(0),
            measurement,
            hazard,
        }
    }

    fn noisy_decision_kernel() -> ActionRealizationKernel {
        let mut rows = Vec::new();
        for hypothesis in 0..2 {
            let state = LatentState {
                hypothesis,
                physical_state: 0,
                sensor_state: 0,
                apparatus_state: 0,
            };
            let (zero, one) = if hypothesis == 0 {
                (Rational::new(3, 4).unwrap(), Rational::new(1, 4).unwrap())
            } else {
                (Rational::new(1, 4).unwrap(), Rational::new(3, 4).unwrap())
            };
            rows.push(KernelRow {
                state,
                command: 0,
                outcomes: vec![
                    outcome(zero, state, Some(0), false),
                    outcome(one, state, Some(1), false),
                ],
            });
            rows.push(KernelRow {
                state,
                command: 1,
                outcomes: vec![
                    outcome(Rational::new(1, 10).unwrap(), state, None, true),
                    outcome(Rational::new(9, 10).unwrap(), state, Some(hypothesis), false),
                ],
            });
        }
        ActionRealizationKernel {
            hypothesis_ids: vec!["safe".into(), "hazardous".into()],
            physical_state_count: 1,
            sensor_state_count: 1,
            apparatus_state_count: 1,
            command_ids: vec!["measure".into(), "risky-measure".into()],
            realized_action_ids: vec!["executed".into()],
            readback_ids: vec!["ok".into()],
            measurement_ids: vec!["zero".into(), "one".into()],
            initial_belief: vec![
                BeliefMass {
                    state: LatentState {
                        hypothesis: 0,
                        physical_state: 0,
                        sensor_state: 0,
                        apparatus_state: 0,
                    },
                    mass: Rational::new(1, 2).unwrap(),
                },
                BeliefMass {
                    state: LatentState {
                        hypothesis: 1,
                        physical_state: 0,
                        sensor_state: 0,
                        apparatus_state: 0,
                    },
                    mass: Rational::new(1, 2).unwrap(),
                },
            ],
            rows,
        }
    }

    fn loss_table() -> DecisionLossTable {
        DecisionLossTable {
            decision_ids: vec!["accept".into(), "reject".into()],
            losses: vec![
                vec![
                    vec![Rational::zero()],
                    vec![Rational::one()],
                ],
                vec![
                    vec![Rational::one()],
                    vec![Rational::zero()],
                ],
            ],
            abstention_loss: Rational::new(2, 5).unwrap(),
        }
    }

    fn fixture_config(horizon: usize) -> DecisionSolverConfig {
        DecisionSolverConfig {
            horizon,
            max_memo_states: 10_000,
            max_frontier_points: 10_000,
            max_generated_combinations: 100_000,
        }
    }

    #[test]
    fn dynamic_program_frontier_matches_non_memoized_exhaustive_policy_trees() {
        let kernel = noisy_decision_kernel();
        let table = loss_table();
        let protocol = ObservationProtocol::fully_observed();
        let costs = vec![
            Rational::new(1, 10).unwrap(),
            Rational::new(1, 20).unwrap(),
        ];
        let config = fixture_config(2);

        let optimized = optimize_finite_horizon_decisions(
            &kernel,
            &table,
            protocol,
            &costs,
            Rational::new(10, 1).unwrap(),
            &config,
        )
        .unwrap();
        let exhaustive = exhaustive_decision_frontier(
            &kernel,
            &table,
            protocol,
            &costs,
            Rational::new(10, 1).unwrap(),
            2,
            100_000,
        )
        .unwrap();

        assert_eq!(
            frontier_objectives(&optimized.frontier),
            frontier_objectives(&exhaustive)
        );
    }

    #[test]
    fn every_frontier_policy_replays_to_its_exact_objectives() {
        let kernel = noisy_decision_kernel();
        let table = loss_table();
        let protocol = ObservationProtocol::fully_observed();
        let costs = vec![
            Rational::new(1, 10).unwrap(),
            Rational::new(1, 20).unwrap(),
        ];
        let optimized = optimize_finite_horizon_decisions(
            &kernel,
            &table,
            protocol,
            &costs,
            Rational::new(10, 1).unwrap(),
            &fixture_config(2),
        )
        .unwrap();

        for point in &optimized.frontier {
            let replay = replay_decision_policy(
                &kernel,
                &table,
                protocol,
                &costs,
                Rational::new(10, 1).unwrap(),
                &point.plan,
                2,
            )
            .unwrap();
            assert_eq!(replay.expected_experiment_cost, point.expected_experiment_cost);
            assert_eq!(replay.expected_terminal_loss, point.expected_terminal_loss);
            assert_eq!(replay.hazard_probability, point.hazard_probability);
            assert_eq!(replay.abstention_probability, point.abstention_probability);
        }
    }

    #[test]
    fn hazard_and_abstention_budgets_select_only_feasible_policies() {
        let kernel = noisy_decision_kernel();
        let table = loss_table();
        let costs = vec![
            Rational::new(1, 10).unwrap(),
            Rational::new(1, 20).unwrap(),
        ];
        let result = optimize_finite_horizon_decisions(
            &kernel,
            &table,
            ObservationProtocol::fully_observed(),
            &costs,
            Rational::new(10, 1).unwrap(),
            &fixture_config(2),
        )
        .unwrap();

        let budgets = DecisionBudgets {
            max_hazard_probability: Rational::zero(),
            max_abstention_probability: Rational::new(1, 2).unwrap(),
        };
        let selected = result.minimum_loss_under(budgets).unwrap().unwrap();
        assert!(selected.hazard_probability <= budgets.max_hazard_probability);
        assert!(selected.abstention_probability <= budgets.max_abstention_probability);
    }

    #[test]
    fn horizon_zero_allows_only_a_terminal_decision() {
        let kernel = noisy_decision_kernel();
        let result = optimize_finite_horizon_decisions(
            &kernel,
            &loss_table(),
            ObservationProtocol::fully_observed(),
            &[Rational::zero(), Rational::zero()],
            Rational::one(),
            &fixture_config(0),
        )
        .unwrap();
        assert_eq!(result.frontier.len(), 1);
        assert!(matches!(
            result.frontier[0].plan,
            DecisionPlan::TerminalDecision { .. }
        ));
    }

    #[test]
    fn zero_resource_limits_fail_closed() {
        let kernel = noisy_decision_kernel();
        let mut config = fixture_config(1);
        config.max_generated_combinations = 0;
        assert!(optimize_finite_horizon_decisions(
            &kernel,
            &loss_table(),
            ObservationProtocol::fully_observed(),
            &[Rational::zero(), Rational::zero()],
            Rational::one(),
            &config,
        )
        .is_err());
    }
}
