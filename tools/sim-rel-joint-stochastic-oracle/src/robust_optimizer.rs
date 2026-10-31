//! Exact shared-policy dynamic programming over persistent joint stochastic worlds.
//!
//! The recursive state is a vector of model-indexed joint beliefs. Each public
//! observation updates each complete world using that world's own kernel. The
//! world identity remains fixed for the whole campaign; no rectangularized
//! per-step model switching is permitted.

use super::{
    JointKernel, JointMass, PolicyBranch, PolicyPlan, Rational, robust::ModelWorld,
    robust::WorldEvaluation, robust::evaluate_policy_across_worlds,
};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};

const MAX_ROBUST_HORIZON: usize = 12;

type JointTupleKey = (usize, usize, usize);
type BeliefVector = Vec<Option<Vec<JointMass>>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RobustSolverConfig {
    pub horizon: usize,
    pub max_memo_states: usize,
    pub max_generated_combinations: usize,
    pub max_frontier_points: usize,
}

impl Default for RobustSolverConfig {
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
pub struct RobustSolverStats {
    pub expanded_belief_vectors: usize,
    pub memo_hits: usize,
    pub generated_combinations: usize,
    pub maximum_frontier_points: usize,
}

/// One model world's conditional objective vector at an observation history.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct WorldObjectives {
    expected_cost: Rational,
    hazard_probability: Rational,
    false_identification_probability: Rational,
    abstention_probability: Rational,
    correct_identification_probability: Rational,
}

impl WorldObjectives {
    fn zero() -> Self {
        Self {
            expected_cost: Rational::zero(),
            hazard_probability: Rational::zero(),
            false_identification_probability: Rational::zero(),
            abstention_probability: Rational::zero(),
            correct_identification_probability: Rational::zero(),
        }
    }

    fn terminal_abstention() -> Self {
        Self {
            abstention_probability: Rational::one(),
            ..Self::zero()
        }
    }

    fn expected_cost(&self) -> Rational {
        self.expected_cost
    }

    fn hazard_probability(&self) -> Rational {
        self.hazard_probability
    }

    fn false_identification_probability(&self) -> Rational {
        self.false_identification_probability
    }

    fn abstention_probability(&self) -> Rational {
        self.abstention_probability
    }

    fn correct_identification_probability(&self) -> Rational {
        self.correct_identification_probability
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Candidate {
    per_world: Vec<WorldObjectives>,
    policy: PolicyPlan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PartialCandidate {
    per_world: Vec<WorldObjectives>,
    branches: Vec<PolicyBranch>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ObservationBranch {
    probability: Rational,
    posterior: Vec<JointMass>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WorldPartition {
    hazard_probability: Rational,
    observations: BTreeMap<usize, ObservationBranch>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RobustFrontierPoint {
    pub per_world: Vec<WorldEvaluation>,
    pub policy: PolicyPlan,
}

impl RobustFrontierPoint {
    pub fn worst_case_expected_cost(&self) -> Rational {
        self.per_world
            .iter()
            .map(|metric| metric.expected_cost)
            .max()
            .unwrap_or_else(Rational::zero)
    }

    pub fn worst_case_hazard_probability(&self) -> Rational {
        self.per_world
            .iter()
            .map(|metric| metric.hazard_probability)
            .max()
            .unwrap_or_else(Rational::zero)
    }

    pub fn worst_case_false_identification_probability(&self) -> Rational {
        self.per_world
            .iter()
            .map(|metric| metric.false_identification_probability)
            .max()
            .unwrap_or_else(Rational::zero)
    }

    pub fn worst_case_abstention_probability(&self) -> Rational {
        self.per_world
            .iter()
            .map(|metric| metric.abstention_probability)
            .max()
            .unwrap_or_else(Rational::zero)
    }

    pub fn worst_case_correct_identification_probability(&self) -> Rational {
        self.per_world
            .iter()
            .map(|metric| metric.correct_identification_probability)
            .min()
            .unwrap_or_else(Rational::zero)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RobustOptimizationResult {
    pub world_ids: Vec<String>,
    pub nominal_world_id: String,
    pub frontier: Vec<RobustFrontierPoint>,
    pub stats: RobustSolverStats,
}

impl RobustOptimizationResult {
    /// Select minimum worst-case expected cost under explicit worst-case budgets.
    pub fn minimum_worst_case_cost_under(
        &self,
        max_hazard: Rational,
        max_false_identification: Rational,
        max_abstention: Rational,
    ) -> Result<Option<&RobustFrontierPoint>, String> {
        validate_budgets(max_hazard, max_false_identification, max_abstention)?;
        Ok(self
            .frontier
            .iter()
            .filter(|point| {
                point.worst_case_hazard_probability() <= max_hazard
                    && point.worst_case_false_identification_probability()
                        <= max_false_identification
                    && point.worst_case_abstention_probability() <= max_abstention
            })
            .min_by(|left, right| robust_point_order(left, right, &self.nominal_world_id)))
    }

    /// Select minimum nominal cost while retaining worst-case risk constraints.
    pub fn minimum_nominal_cost_under(
        &self,
        max_hazard: Rational,
        max_false_identification: Rational,
        max_abstention: Rational,
    ) -> Result<Option<&RobustFrontierPoint>, String> {
        validate_budgets(max_hazard, max_false_identification, max_abstention)?;
        let nominal_index = self
            .world_ids
            .iter()
            .position(|world_id| world_id == &self.nominal_world_id)
            .ok_or_else(|| "nominal world is absent from robust result".to_string())?;
        Ok(self
            .frontier
            .iter()
            .filter(|point| {
                point.worst_case_hazard_probability() <= max_hazard
                    && point.worst_case_false_identification_probability()
                        <= max_false_identification
                    && point.worst_case_abstention_probability() <= max_abstention
            })
            .min_by(|left, right| {
                left.per_world[nominal_index]
                    .expected_cost
                    .cmp(&right.per_world[nominal_index].expected_cost)
                    .then_with(|| robust_point_order(left, right, &self.nominal_world_id))
            }))
    }
}

fn validate_budgets(
    max_hazard: Rational,
    max_false_identification: Rational,
    max_abstention: Rational,
) -> Result<(), String> {
    for (name, budget) in [
        ("hazard", max_hazard),
        ("false-identification", max_false_identification),
        ("abstention", max_abstention),
    ] {
        if !budget.is_probability() {
            return Err(format!("{name} budget must lie in [0, 1]"));
        }
    }
    Ok(())
}

fn robust_point_order(
    left: &RobustFrontierPoint,
    right: &RobustFrontierPoint,
    nominal_world_id: &str,
) -> Ordering {
    let left_nominal = left
        .per_world
        .iter()
        .find(|metric| metric.world_id == nominal_world_id)
        .map(|metric| metric.expected_cost)
        .unwrap_or_else(Rational::zero);
    let right_nominal = right
        .per_world
        .iter()
        .find(|metric| metric.world_id == nominal_world_id)
        .map(|metric| metric.expected_cost)
        .unwrap_or_else(Rational::zero);
    (
        left.worst_case_expected_cost(),
        left.worst_case_hazard_probability(),
        left.worst_case_false_identification_probability(),
        left.worst_case_abstention_probability(),
        left_nominal,
        &left.policy,
    )
        .cmp(&(
            right.worst_case_expected_cost(),
            right.worst_case_hazard_probability(),
            right.worst_case_false_identification_probability(),
            right.worst_case_abstention_probability(),
            right_nominal,
            &right.policy,
        ))
}

/// Optimize one shared public-observation policy over a finite persistent model family.
///
/// The model set is validated through the robust evaluator's compatibility
/// contract. Each observation branch carries a vector of world-specific
/// posteriors. Worlds assigning zero probability to a branch are inactive only
/// on that branch; their model identity is not changed or reselected.
pub fn optimize_robust_worlds(
    worlds: &[ModelWorld],
    nominal_world_id: &str,
    config: &RobustSolverConfig,
) -> Result<RobustOptimizationResult, String> {
    // Reuse the evaluator's full world-set validation with the neutral terminal policy.
    evaluate_policy_across_worlds(worlds, nominal_world_id, &PolicyPlan::Abstain, 0)?;

    if config.horizon > MAX_ROBUST_HORIZON {
        return Err(format!(
            "robust horizon exceeds the {MAX_ROBUST_HORIZON}-step cap"
        ));
    }
    if config.max_memo_states == 0
        || config.max_generated_combinations == 0
        || config.max_frontier_points == 0
    {
        return Err("all robust solver resource caps must be positive".into());
    }

    let initial_beliefs: BeliefVector = worlds
        .iter()
        .map(|world| {
            let mut belief = world.kernel.initial_belief.clone();
            belief.sort();
            Some(belief)
        })
        .collect();

    let mut solver = RobustSolver {
        worlds,
        config: *config,
        stats: RobustSolverStats::default(),
        memo: HashMap::new(),
    };
    let candidates = solver.solve(&initial_beliefs, config.horizon)?;

    let world_ids: Vec<String> = worlds.iter().map(|world| world.id.clone()).collect();
    let frontier = candidates
        .into_iter()
        .map(|candidate| RobustFrontierPoint {
            per_world: candidate
                .per_world
                .into_iter()
                .zip(world_ids.iter())
                .map(|(metric, world_id)| WorldEvaluation {
                    world_id: world_id.clone(),
                    expected_cost: metric.expected_cost(),
                    hazard_probability: metric.hazard_probability(),
                    false_identification_probability: metric
                        .false_identification_probability(),
                    abstention_probability: metric.abstention_probability(),
                    correct_identification_probability: metric
                        .correct_identification_probability(),
                })
                .collect(),
            policy: candidate.policy,
        })
        .collect();

    Ok(RobustOptimizationResult {
        world_ids,
        nominal_world_id: nominal_world_id.to_string(),
        frontier,
        stats: solver.stats,
    })
}

struct RobustSolver<'a> {
    worlds: &'a [ModelWorld],
    config: RobustSolverConfig,
    stats: RobustSolverStats,
    memo: HashMap<(BeliefVector, usize), Vec<Candidate>>,
}

impl RobustSolver<'_> {
    fn solve(
        &mut self,
        beliefs: &BeliefVector,
        remaining: usize,
    ) -> Result<Vec<Candidate>, String> {
        let key = (beliefs.clone(), remaining);
        if let Some(cached) = self.memo.get(&key) {
            self.stats.memo_hits = self
                .stats
                .memo_hits
                .checked_add(1)
                .ok_or_else(|| "robust memo-hit counter overflow".to_string())?;
            return Ok(cached.clone());
        }
        if self.memo.len() >= self.config.max_memo_states {
            return Err("robust solver memo-state budget exceeded".into());
        }
        self.stats.expanded_belief_vectors = self
            .stats
            .expanded_belief_vectors
            .checked_add(1)
            .ok_or_else(|| "robust state-expansion counter overflow".to_string())?;

        let mut candidates = terminal_candidates(
            beliefs,
            self.worlds[0].kernel.hypothesis_ids.len(),
        )?;
        if remaining > 0 {
            for action in 0..self.worlds[0].kernel.action_ids.len() {
                let partitions = beliefs
                    .iter()
                    .enumerate()
                    .map(|(world_index, belief)| {
                        match belief {
                            Some(belief) => partition_world(
                                &self.worlds[world_index].kernel,
                                belief,
                                action,
                            ),
                            None => Ok(WorldPartition {
                                hazard_probability: Rational::zero(),
                                observations: BTreeMap::new(),
                            }),
                        }
                    })
                    .collect::<Result<Vec<_>, String>>()?;

                let observation_ids: BTreeSet<usize> = partitions
                    .iter()
                    .flat_map(|partition| partition.observations.keys().copied())
                    .collect();

                let initial_metrics = partitions
                    .iter()
                    .zip(beliefs.iter())
                    .enumerate()
                    .map(|(world_index, (partition, belief))| {
                        if belief.is_some() {
                            WorldObjectives {
                                expected_cost: Rational::from_u64(
                                    self.worlds[world_index].kernel.action_costs[action],
                                ),
                                hazard_probability: partition.hazard_probability,
                                ..WorldObjectives::zero()
                            }
                        } else {
                            WorldObjectives::zero()
                        }
                    })
                    .collect();

                let mut partials = vec![PartialCandidate {
                    per_world: initial_metrics,
                    branches: Vec::new(),
                }];

                for observation in observation_ids {
                    let mut branch_beliefs = Vec::with_capacity(self.worlds.len());
                    let mut branch_probabilities = Vec::with_capacity(self.worlds.len());
                    for partition in &partitions {
                        match partition.observations.get(&observation) {
                            Some(branch) => {
                                branch_beliefs.push(Some(branch.posterior.clone()));
                                branch_probabilities.push(branch.probability);
                            }
                            None => {
                                branch_beliefs.push(None);
                                branch_probabilities.push(Rational::zero());
                            }
                        }
                    }

                    let child_frontier = self.solve(&branch_beliefs, remaining - 1)?;
                    if child_frontier.is_empty() {
                        return Err("robust child frontier is unexpectedly empty".into());
                    }

                    let combination_count = partials
                        .len()
                        .checked_mul(child_frontier.len())
                        .ok_or_else(|| "robust generated-combination count overflow".to_string())?;
                    self.stats.generated_combinations = self
                        .stats
                        .generated_combinations
                        .checked_add(combination_count)
                        .ok_or_else(|| "robust generated-combination counter overflow".to_string())?;
                    if self.stats.generated_combinations
                        > self.config.max_generated_combinations
                    {
                        return Err("robust solver generated-combination budget exceeded".into());
                    }

                    let mut expanded = Vec::with_capacity(combination_count);
                    for partial in &partials {
                        for child in &child_frontier {
                            let mut next = partial.clone();
                            for world_index in 0..self.worlds.len() {
                                let probability = branch_probabilities[world_index];
                                if probability.is_zero() {
                                    continue;
                                }
                                let child_metrics = child.per_world[world_index];
                                let current = &mut next.per_world[world_index];
                                current.expected_cost = current.expected_cost.checked_add(
                                    probability.checked_mul(child_metrics.expected_cost)?,
                                )?;
                                current.hazard_probability =
                                    current.hazard_probability.checked_add(
                                        probability.checked_mul(
                                            child_metrics.hazard_probability,
                                        )?,
                                    )?;
                                current.false_identification_probability =
                                    current.false_identification_probability.checked_add(
                                        probability.checked_mul(
                                            child_metrics.false_identification_probability,
                                        )?,
                                    )?;
                                current.abstention_probability =
                                    current.abstention_probability.checked_add(
                                        probability.checked_mul(
                                            child_metrics.abstention_probability,
                                        )?,
                                    )?;
                                current.correct_identification_probability =
                                    current.correct_identification_probability.checked_add(
                                        probability.checked_mul(
                                            child_metrics.correct_identification_probability,
                                        )?,
                                    )?;
                            }
                            next.branches.push(PolicyBranch {
                                observation,
                                plan: Box::new(child.policy.clone()),
                            });
                            expanded.push(next);
                        }
                    }
                    partials = expanded;
                }

                for partial in partials {
                    for (world_index, belief) in beliefs.iter().enumerate() {
                        if belief.is_some() {
                            validate_probability_partition(
                                partial.per_world[world_index],
                            )?;
                        }
                    }
                    candidates.push(Candidate {
                        per_world: partial.per_world,
                        policy: PolicyPlan::Action {
                            action,
                            branches: partial.branches,
                        },
                    });
                }
            }
        }

        let frontier = prune_candidates(candidates, self.config.max_frontier_points)?;
        self.stats.maximum_frontier_points = self
            .stats
            .maximum_frontier_points
            .max(frontier.len());
        self.memo.insert(key, frontier.clone());
        Ok(frontier)
    }
}

fn terminal_candidates(
    beliefs: &BeliefVector,
    hypothesis_count: usize,
) -> Result<Vec<Candidate>, String> {
    let mut candidates = Vec::new();
    let abstention = beliefs
        .iter()
        .map(|belief| {
            if belief.is_some() {
                WorldObjectives::terminal_abstention()
            } else {
                WorldObjectives::zero()
            }
        })
        .collect();
    candidates.push(Candidate {
        per_world: abstention,
        policy: PolicyPlan::Abstain,
    });

    // Every semantically valid terminal claim is part of the policy class,
    // even when its posterior mass is zero. Such a claim can be dominated
    // under strict budgets, but dropping it would make the frontier incomplete.
    for hypothesis in 0..hypothesis_count {
        let mut per_world = Vec::with_capacity(beliefs.len());
        for belief in beliefs {
            match belief {
                Some(belief) => {
                    let mut probability = Rational::zero();
                    for point in belief {
                        if point.hypothesis == hypothesis {
                            probability = probability.checked_add(point.mass)?;
                        }
                    }
                    per_world.push(WorldObjectives {
                        false_identification_probability: Rational::one()
                            .checked_sub(probability)?,
                        correct_identification_probability: probability,
                        ..WorldObjectives::zero()
                    });
                }
                None => {
                    // This world assigns this entire public-history branch zero mass.
                    per_world.push(WorldObjectives::zero());
                }
            }
        }
        candidates.push(Candidate {
            per_world,
            policy: PolicyPlan::Claim { hypothesis },
        });
    }
    Ok(candidates)
}

fn partition_world(
    kernel: &JointKernel,
    belief: &[JointMass],
    action: usize,
) -> Result<WorldPartition, String> {
    let mut hazard_probability = Rational::zero();
    let mut by_observation: BTreeMap<
        usize,
        BTreeMap<JointTupleKey, Rational>,
    > = BTreeMap::new();

    for point in belief {
        let row_index = (((point.hypothesis * kernel.physical_state_count
            + point.physical_state)
            * kernel.sensor_state_count
            + point.sensor_state)
            * kernel.action_ids.len())
            + action;
        let row = &kernel.rows[row_index];
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
                    let old = states.get(&key).copied().unwrap_or(Rational::zero());
                    states.insert(key, old.checked_add(path_mass)?);
                }
            }
        }
    }

    let mut observations = BTreeMap::new();
    let mut total_mass = hazard_probability;
    for (observation, states) in by_observation {
        let probability = states.values().try_fold(
            Rational::zero(),
            |sum, mass| sum.checked_add(*mass),
        )?;
        if probability.is_zero() {
            return Err("robust optimizer produced a zero-probability observation branch".into());
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
        observations.insert(
            observation,
            ObservationBranch {
                probability,
                posterior,
            },
        );
    }
    if total_mass != Rational::one() {
        return Err("robust action partition failed exact probability conservation".into());
    }

    Ok(WorldPartition {
        hazard_probability,
        observations,
    })
}

fn validate_probability_partition(metrics: WorldObjectives) -> Result<(), String> {
    let total = metrics
        .hazard_probability
        .checked_add(metrics.false_identification_probability)?
        .checked_add(metrics.abstention_probability)?
        .checked_add(metrics.correct_identification_probability)?;
    if total != Rational::one() {
        return Err(
            "robust per-world hazard, false claim, abstention and correct claim do not sum to one"
                .into(),
        );
    }
    Ok(())
}

fn dominates(left: &Candidate, right: &Candidate) -> bool {
    let mut no_worse = true;
    let mut strictly_better = false;
    for (left_world, right_world) in left.per_world.iter().zip(right.per_world.iter()) {
        no_worse &= left_world.expected_cost <= right_world.expected_cost
            && left_world.hazard_probability <= right_world.hazard_probability
            && left_world.false_identification_probability
                <= right_world.false_identification_probability
            && left_world.abstention_probability <= right_world.abstention_probability;
        strictly_better |= left_world.expected_cost < right_world.expected_cost
            || left_world.hazard_probability < right_world.hazard_probability
            || left_world.false_identification_probability
                < right_world.false_identification_probability
            || left_world.abstention_probability < right_world.abstention_probability;
    }
    no_worse && strictly_better
}

fn same_objectives(left: &Candidate, right: &Candidate) -> bool {
    left.per_world.iter().zip(right.per_world.iter()).all(|(a, b)| {
        a.expected_cost == b.expected_cost
            && a.hazard_probability == b.hazard_probability
            && a.false_identification_probability == b.false_identification_probability
            && a.abstention_probability == b.abstention_probability
    })
}

fn candidate_order(left: &Candidate, right: &Candidate) -> Ordering {
    (
        worst_case(&left.per_world, |metric| metric.expected_cost),
        worst_case(&left.per_world, |metric| metric.hazard_probability),
        worst_case(&left.per_world, |metric| metric.false_identification_probability),
        worst_case(&left.per_world, |metric| metric.abstention_probability),
        &left.per_world,
        &left.policy,
    )
        .cmp(&(
            worst_case(&right.per_world, |metric| metric.expected_cost),
            worst_case(&right.per_world, |metric| metric.hazard_probability),
            worst_case(&right.per_world, |metric| metric.false_identification_probability),
            worst_case(&right.per_world, |metric| metric.abstention_probability),
            &right.per_world,
            &right.policy,
        ))
}

fn worst_case(
    metrics: &[WorldObjectives],
    select: impl Fn(&WorldObjectives) -> Rational,
) -> Rational {
    metrics
        .iter()
        .map(select)
        .max()
        .unwrap_or_else(Rational::zero)
}

fn prune_candidates(
    mut candidates: Vec<Candidate>,
    max_frontier_points: usize,
) -> Result<Vec<Candidate>, String> {
    candidates.sort_by(candidate_order);
    let mut frontier: Vec<Candidate> = Vec::new();
    for candidate in candidates {
        if frontier.iter().any(|other| {
            same_objectives(other, &candidate) || dominates(other, &candidate)
        }) {
            continue;
        }
        frontier.retain(|other| !dominates(&candidate, other));
        frontier.push(candidate);
        if frontier.len() > max_frontier_points {
            return Err("robust solver frontier-point budget exceeded".into());
        }
    }
    frontier.sort_by(candidate_order);
    Ok(frontier)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        JointOutcome, KernelRow,
        robust::{evaluate_policy_across_worlds, ModelWorld},
    };

    fn half() -> Rational {
        Rational::new(1, 2).unwrap()
    }

    fn quarter() -> Rational {
        Rational::new(1, 4).unwrap()
    }

    fn three_quarters() -> Rational {
        Rational::new(3, 4).unwrap()
    }

    fn make_world(id: &str, reverse_nominal_action: bool) -> ModelWorld {
        let mut rows = Vec::new();
        for hypothesis in 0..2 {
            for action in 0..2 {
                let observation = if action == 0 {
                    if (hypothesis == 0) == reverse_nominal_action { 1 } else { 0 }
                } else if hypothesis == 0 {
                    0
                } else {
                    1
                };
                rows.push(KernelRow {
                    hypothesis,
                    physical_state: 0,
                    sensor_state: 0,
                    action,
                    outcomes: vec![JointOutcome {
                        probability: Rational::one(),
                        next_physical_state: 0,
                        next_sensor_state: 0,
                        observation: Some(observation),
                    }],
                });
            }
        }
        ModelWorld {
            id: id.to_string(),
            kernel: JointKernel {
                hypothesis_ids: vec!["h0".into(), "h1".into()],
                physical_state_count: 1,
                sensor_state_count: 1,
                action_ids: vec!["model-sensitive-read".into(), "robust-read".into()],
                observation_ids: vec!["zero".into(), "one".into()],
                action_costs: vec![1, 3],
                initial_belief: vec![
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
                rows,
            },
        }
    }

    fn partial_observation_world(id: &str, distinguishes_hypotheses: bool) -> ModelWorld {
        let mut rows = Vec::new();
        for hypothesis in 0..2 {
            rows.push(KernelRow {
                hypothesis,
                physical_state: 0,
                sensor_state: 0,
                action: 0,
                outcomes: vec![JointOutcome {
                    probability: Rational::one(),
                    next_physical_state: 0,
                    next_sensor_state: 0,
                    observation: Some(if distinguishes_hypotheses { hypothesis } else { 0 }),
                }],
            });
        }
        ModelWorld {
            id: id.to_string(),
            kernel: JointKernel {
                hypothesis_ids: vec!["h0".into(), "h1".into()],
                physical_state_count: 1,
                sensor_state_count: 1,
                action_ids: vec!["measure".into()],
                observation_ids: vec!["zero".into(), "one".into()],
                action_costs: vec![1],
                initial_belief: vec![
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
                rows,
            },
        }
    }

    fn stochastic_world(
        id: &str,
        h0_observation_zero: Rational,
        h1_observation_zero: Rational,
    ) -> ModelWorld {
        let mut rows = Vec::new();
        for (hypothesis, p_zero) in [
            (0, h0_observation_zero),
            (1, h1_observation_zero),
        ] {
            rows.push(KernelRow {
                hypothesis,
                physical_state: 0,
                sensor_state: 0,
                action: 0,
                outcomes: vec![
                    JointOutcome {
                        probability: p_zero,
                        next_physical_state: 0,
                        next_sensor_state: 0,
                        observation: Some(0),
                    },
                    JointOutcome {
                        probability: Rational::one().checked_sub(p_zero).unwrap(),
                        next_physical_state: 0,
                        next_sensor_state: 0,
                        observation: Some(1),
                    },
                ],
            });
        }
        ModelWorld {
            id: id.to_string(),
            kernel: JointKernel {
                hypothesis_ids: vec!["h0".into(), "h1".into()],
                physical_state_count: 1,
                sensor_state_count: 1,
                action_ids: vec!["stochastic-read".into()],
                observation_ids: vec!["zero".into(), "one".into()],
                action_costs: vec![1],
                initial_belief: vec![
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
                rows,
            },
        }
    }

    fn brute_one_action_policies() -> Vec<PolicyPlan> {
        let terminals = vec![
            PolicyPlan::Abstain,
            PolicyPlan::Claim { hypothesis: 0 },
            PolicyPlan::Claim { hypothesis: 1 },
        ];
        let mut policies = terminals.clone();
        for on_zero in &terminals {
            for on_one in &terminals {
                policies.push(PolicyPlan::Action {
                    action: 0,
                    branches: vec![
                        PolicyBranch {
                            observation: 0,
                            plan: Box::new(on_zero.clone()),
                        },
                        PolicyBranch {
                            observation: 1,
                            plan: Box::new(on_one.clone()),
                        },
                    ],
                });
            }
        }
        policies
    }

    fn objective_key(
        point: &RobustFrontierPoint,
    ) -> Vec<(Rational, Rational, Rational, Rational, Rational)> {
        point
            .per_world
            .iter()
            .map(|metric| {
                (
                    metric.expected_cost,
                    metric.hazard_probability,
                    metric.false_identification_probability,
                    metric.abstention_probability,
                    metric.correct_identification_probability,
                )
            })
            .collect()
    }

    fn brute_policies() -> Vec<PolicyPlan> {
        let terminals = vec![
            PolicyPlan::Abstain,
            PolicyPlan::Claim { hypothesis: 0 },
            PolicyPlan::Claim { hypothesis: 1 },
        ];
        let mut policies = terminals.clone();
        for action in 0..2 {
            for on_zero in &terminals {
                for on_one in &terminals {
                    policies.push(PolicyPlan::Action {
                        action,
                        branches: vec![
                            PolicyBranch {
                                observation: 0,
                                plan: Box::new(on_zero.clone()),
                            },
                            PolicyBranch {
                                observation: 1,
                                plan: Box::new(on_one.clone()),
                            },
                        ],
                    });
                }
            }
        }
        policies
    }

    fn brute_frontier(mut points: Vec<RobustFrontierPoint>) -> Vec<RobustFrontierPoint> {
        points.sort_by(|left, right| {
            (
                left.worst_case_expected_cost(),
                left.worst_case_hazard_probability(),
                left.worst_case_false_identification_probability(),
                left.worst_case_abstention_probability(),
                objective_key(left),
                &left.policy,
            )
                .cmp(&(
                    right.worst_case_expected_cost(),
                    right.worst_case_hazard_probability(),
                    right.worst_case_false_identification_probability(),
                    right.worst_case_abstention_probability(),
                    objective_key(right),
                    &right.policy,
                ))
        });

        let mut frontier: Vec<RobustFrontierPoint> = Vec::new();
        for point in points {
            let dominated = frontier.iter().any(|other| {
                let mut no_worse = true;
                let mut strictly_better = false;
                for (left, right) in other.per_world.iter().zip(point.per_world.iter()) {
                    no_worse &= left.expected_cost <= right.expected_cost
                        && left.hazard_probability <= right.hazard_probability
                        && left.false_identification_probability
                            <= right.false_identification_probability
                        && left.abstention_probability <= right.abstention_probability;
                    strictly_better |= left.expected_cost < right.expected_cost
                        || left.hazard_probability < right.hazard_probability
                        || left.false_identification_probability
                            < right.false_identification_probability
                        || left.abstention_probability < right.abstention_probability;
                }
                no_worse && strictly_better
                    || objective_key(other) == objective_key(&point)
            });
            if !dominated {
                frontier.push(point);
            }
        }
        frontier
    }

    #[test]
    fn robust_optimizer_selects_action_safe_across_persistent_model_family() {
        let worlds = vec![
            make_world("nominal", false),
            make_world("shifted", true),
        ];
        let result = optimize_robust_worlds(
            &worlds,
            "nominal",
            &RobustSolverConfig {
                horizon: 1,
                ..RobustSolverConfig::default()
            },
        )
        .unwrap();

        let selected = result
            .minimum_worst_case_cost_under(
                Rational::zero(),
                Rational::zero(),
                Rational::zero(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(selected.worst_case_expected_cost(), Rational::from_u64(3));
        assert_eq!(selected.worst_case_false_identification_probability(), Rational::zero());
        assert_eq!(selected.worst_case_hazard_probability(), Rational::zero());
        assert_eq!(selected.worst_case_abstention_probability(), Rational::zero());
        match &selected.policy {
            PolicyPlan::Action { action, .. } => assert_eq!(*action, 1),
            _ => panic!("expected robust-read action"),
        }

        let replay = evaluate_policy_across_worlds(
            &worlds,
            "nominal",
            &selected.policy,
            1,
        )
        .unwrap();
        for (expected, actual) in selected.per_world.iter().zip(replay.per_world.iter()) {
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn robust_abstention_is_chosen_when_it_is_the_cheapest_zero_false_claim_policy() {
        let worlds = vec![
            make_world("nominal", false),
            make_world("shifted", true),
        ];
        let result = optimize_robust_worlds(
            &worlds,
            "nominal",
            &RobustSolverConfig {
                horizon: 1,
                ..RobustSolverConfig::default()
            },
        )
        .unwrap();

        let selected = result
            .minimum_worst_case_cost_under(
                Rational::zero(),
                Rational::zero(),
                Rational::one(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(selected.worst_case_expected_cost(), Rational::zero());
        assert_eq!(selected.worst_case_abstention_probability(), Rational::one());
        assert_eq!(selected.policy, PolicyPlan::Abstain);
    }

    #[test]
    fn dynamic_program_matches_exhaustive_multiworld_policy_enumeration() {
        let worlds = vec![
            make_world("nominal", false),
            make_world("shifted", true),
        ];
        let optimized = optimize_robust_worlds(
            &worlds,
            "nominal",
            &RobustSolverConfig {
                horizon: 1,
                ..RobustSolverConfig::default()
            },
        )
        .unwrap();
        let enumerated = brute_policies()
            .iter()
            .map(|policy| {
                let report = evaluate_policy_across_worlds(&worlds, "nominal", policy, 1)
                    .unwrap();
                RobustFrontierPoint {
                    per_world: report.per_world,
                    policy: policy.clone(),
                }
            })
            .collect();
        let brute = brute_frontier(enumerated);
        let mut optimized_keys: Vec<_> = optimized.frontier.iter().map(objective_key).collect();
        let mut brute_keys: Vec<_> = brute.iter().map(objective_key).collect();
        optimized_keys.sort();
        brute_keys.sort();
        optimized_keys.dedup();
        brute_keys.dedup();
        assert_eq!(optimized_keys, brute_keys);
    }

    #[test]
    fn world_with_zero_probability_observation_is_inactive_only_on_that_branch() {
        let worlds = vec![
            partial_observation_world("distinguishing", true),
            partial_observation_world("non-distinguishing", false),
        ];
        let optimized = optimize_robust_worlds(
            &worlds,
            "distinguishing",
            &RobustSolverConfig {
                horizon: 1,
                ..RobustSolverConfig::default()
            },
        ).unwrap();

        let selected = optimized.frontier.iter().find(|point| {
            match &point.policy {
                PolicyPlan::Action { action: 0, branches } if branches.len() == 2 => {
                    matches!(
                        branches[0].plan.as_ref(),
                        PolicyPlan::Claim { hypothesis: 0 }
                    ) && matches!(
                        branches[1].plan.as_ref(),
                        PolicyPlan::Claim { hypothesis: 1 }
                    )
                }
                _ => false,
            }
        }).expect("shared policy with observation-specific claims must be retained");

        assert_eq!(
            selected.per_world[0].correct_identification_probability,
            Rational::one()
        );
        assert_eq!(
            selected.per_world[0].false_identification_probability,
            Rational::zero()
        );
        assert_eq!(
            selected.per_world[1].correct_identification_probability,
            half()
        );
        assert_eq!(
            selected.per_world[1].false_identification_probability,
            half()
        );

        let replay = evaluate_policy_across_worlds(
            &worlds,
            "distinguishing",
            &selected.policy,
            1,
        ).unwrap();
        for (expected, actual) in selected.per_world.iter().zip(replay.per_world.iter()) {
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn dynamic_program_matches_exhaustive_multiworld_stochastic_policies_with_different_branch_masses() {
        let worlds = vec![
            stochastic_world("world-a", three_quarters(), quarter()),
            stochastic_world("world-b", quarter(), quarter()),
        ];
        let optimized = optimize_robust_worlds(
            &worlds,
            "world-a",
            &RobustSolverConfig {
                horizon: 1,
                ..RobustSolverConfig::default()
            },
        )
        .unwrap();
        let enumerated = brute_one_action_policies()
            .iter()
            .map(|policy| {
                let report = evaluate_policy_across_worlds(&worlds, "world-a", policy, 1)
                    .unwrap();
                RobustFrontierPoint {
                    per_world: report.per_world,
                    policy: policy.clone(),
                }
            })
            .collect();
        let brute = brute_frontier(enumerated);
        let mut optimized_keys: Vec<_> = optimized.frontier.iter().map(objective_key).collect();
        let mut brute_keys: Vec<_> = brute.iter().map(objective_key).collect();
        optimized_keys.sort();
        brute_keys.sort();
        optimized_keys.dedup();
        brute_keys.dedup();
        assert_eq!(optimized_keys, brute_keys);

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
        let report = evaluate_policy_across_worlds(&worlds, "world-a", &policy, 1).unwrap();
        assert_eq!(
            report.per_world[0].correct_identification_probability,
            three_quarters()
        );
        assert_eq!(
            report.per_world[1].correct_identification_probability,
            half()
        );
        assert_eq!(
            report.per_world[0].expected_cost,
            Rational::one()
        );
        assert_eq!(
            report.per_world[1].expected_cost,
            Rational::one()
        );
    }

    #[test]
    fn invalid_budget_and_resource_caps_fail_closed() {
        let worlds = vec![
            make_world("nominal", false),
            make_world("shifted", true),
        ];
        let result = optimize_robust_worlds(
            &worlds,
            "nominal",
            &RobustSolverConfig {
                horizon: 1,
                ..RobustSolverConfig::default()
            },
        )
        .unwrap();
        assert!(result.minimum_worst_case_cost_under(
            Rational::new(2, 1).unwrap(),
            Rational::zero(),
            Rational::zero(),
        ).is_err());
        assert!(optimize_robust_worlds(
            &worlds,
            "nominal",
            &RobustSolverConfig { max_generated_combinations: 0, ..RobustSolverConfig::default() },
        ).is_err());
        assert!(optimize_robust_worlds(
            &worlds,
            "nominal",
            &RobustSolverConfig { max_memo_states: 1, horizon: 1, ..RobustSolverConfig::default() },
        ).is_err());
    }
}
