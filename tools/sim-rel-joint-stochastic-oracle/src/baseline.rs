//! Matched information-gain baselines for the exact joint stochastic model.

use super::{JointKernel, JointMass, Rational};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// One-step expected information gain about the mechanism hypothesis.
///
/// Physical and sensor states are marginalized only after applying the full
/// joint transition-observation-hazard kernel. Hazard is treated as an observed
/// terminal event. The score is a myopic baseline and does not value actions
/// whose information appears only after a later intervention.
#[derive(Clone, Debug, PartialEq)]
pub struct OneStepEigScore {
    pub action_index: usize,
    pub action_id: String,
    pub expected_information_gain_nats: f64,
    pub information_gain_per_cost: f64,
    pub hazard_probability: Rational,
    pub outcome_count: usize,
}

/// Rank actions by one-step mechanism EIG per unit action cost.
///
/// This is a model-based comparator, not the exact finite-horizon optimizer.
/// It receives the current public posterior over joint hidden tuples and
/// integrates sensor and physical uncertainty using the same declared kernel.
pub fn rank_by_one_step_eig(
    kernel: &JointKernel,
    belief: &[JointMass],
) -> Result<Vec<OneStepEigScore>, String> {
    kernel.validate()?;
    validate_belief(kernel, belief)?;

    let mut prior_by_hypothesis = vec![Rational::zero(); kernel.hypothesis_ids.len()];
    for point in belief {
        prior_by_hypothesis[point.hypothesis] =
            prior_by_hypothesis[point.hypothesis].checked_add(point.mass)?;
    }
    let prior_entropy = entropy(&prior_by_hypothesis, Rational::one())?;

    let mut scores = Vec::with_capacity(kernel.action_ids.len());
    for action in 0..kernel.action_ids.len() {
        let mut event_by_hypothesis: BTreeMap<Option<usize>, Vec<Rational>> = BTreeMap::new();
        for point in belief {
            let row_index = kernel.row_index(
                point.hypothesis,
                point.physical_state,
                point.sensor_state,
                action,
            );
            for outcome in &kernel.rows[row_index].outcomes {
                let weighted_mass = point.mass.checked_mul(outcome.probability)?;
                let event = event_by_hypothesis
                    .entry(outcome.observation)
                    .or_insert_with(|| vec![Rational::zero(); kernel.hypothesis_ids.len()]);
                event[point.hypothesis] =
                    event[point.hypothesis].checked_add(weighted_mass)?;
            }
        }

        let mut total_event_mass = Rational::zero();
        let mut hazard_probability = Rational::zero();
        let mut expected_posterior_entropy = 0.0_f64;
        let outcome_count = event_by_hypothesis.len();
        for (observation, hypothesis_masses) in event_by_hypothesis {
            let event_mass = hypothesis_masses.iter().try_fold(
                Rational::zero(),
                |sum, mass| sum.checked_add(*mass),
            )?;
            total_event_mass = total_event_mass.checked_add(event_mass)?;
            if observation.is_none() {
                hazard_probability = hazard_probability.checked_add(event_mass)?;
            }
            expected_posterior_entropy +=
                to_f64(event_mass) * entropy(&hypothesis_masses, event_mass)?;
        }
        if total_event_mass != Rational::one() {
            return Err("one-step EIG event partition did not conserve probability mass".into());
        }

        let mut information_gain = prior_entropy - expected_posterior_entropy;
        if information_gain < -1e-10 {
            return Err("one-step EIG became materially negative; check model arithmetic".into());
        }
        information_gain = information_gain.max(0.0);
        let cost = kernel.action_costs[action] as f64;
        scores.push(OneStepEigScore {
            action_index: action,
            action_id: kernel.action_ids[action].clone(),
            expected_information_gain_nats: information_gain,
            information_gain_per_cost: information_gain / cost,
            hazard_probability,
            outcome_count,
        });
    }

    scores.sort_by(|left, right| {
        right
            .information_gain_per_cost
            .total_cmp(&left.information_gain_per_cost)
            .then_with(|| {
                right
                    .expected_information_gain_nats
                    .total_cmp(&left.expected_information_gain_nats)
            })
            .then_with(|| left.action_id.cmp(&right.action_id))
    });
    Ok(scores)
}

fn validate_belief(kernel: &JointKernel, belief: &[JointMass]) -> Result<(), String> {
    if belief.is_empty() || belief.len() > 65_536 {
        return Err("joint belief is empty or exceeds its tuple cap".into());
    }
    let mut seen = BTreeSet::new();
    let mut total = Rational::zero();
    for point in belief {
        if point.hypothesis >= kernel.hypothesis_ids.len()
            || point.physical_state >= kernel.physical_state_count
            || point.sensor_state >= kernel.sensor_state_count
            || point.mass.is_zero()
        {
            return Err("joint belief contains an invalid tuple or non-positive mass".into());
        }
        if !seen.insert((
            point.hypothesis,
            point.physical_state,
            point.sensor_state,
        )) {
            return Err("joint belief contains a duplicate tuple".into());
        }
        total = total.checked_add(point.mass)?;
    }
    if total != Rational::one() {
        return Err("joint belief probability mass must sum exactly to one".into());
    }
    Ok(())
}

fn entropy(masses: &[Rational], normalizer: Rational) -> Result<f64, String> {
    if normalizer.is_zero() {
        return Err("cannot calculate entropy for a zero-mass event".into());
    }
    let mut result = 0.0_f64;
    for mass in masses {
        if mass.is_zero() {
            continue;
        }
        let probability = to_f64(mass.checked_div(normalizer)?);
        result -= probability * probability.ln();
    }
    Ok(result)
}

fn to_f64(value: Rational) -> f64 {
    value.numerator() as f64 / value.denominator() as f64
}


/// Bounded finite-horizon scenario-tree EIG configuration.
///
/// max_path_cost is a hard per-trajectory action-cost budget, not an
/// expected-cost penalty. The campaign hazard constraint is applied at the
/// root to the exact probability returned by the joint kernel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LookaheadEigConfig {
    pub horizon: usize,
    pub max_path_cost: u64,
    pub max_campaign_hazard: Rational,
    pub max_memo_states: usize,
    pub max_generated_combinations: usize,
    pub max_frontier_points: usize,
}

impl Default for LookaheadEigConfig {
    fn default() -> Self {
        Self {
            horizon: 3,
            max_path_cost: 32,
            max_campaign_hazard: Rational::one(),
            max_memo_states: 100_000,
            max_generated_combinations: 1_000_000,
            max_frontier_points: 20_000,
        }
    }
}

/// Work counters for the finite-lookahead scenario-tree baseline.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LookaheadEigStats {
    pub expanded_belief_states: usize,
    pub memo_hits: usize,
    pub generated_combinations: usize,
    pub maximum_frontier_points: usize,
}

/// Best finite-lookahead information-gain policy under explicit constraints.
#[derive(Clone, Debug, PartialEq)]
pub struct LookaheadEigResult {
    pub expected_information_gain_nats: f64,
    pub expected_terminal_entropy_nats: f64,
    pub expected_experiment_cost: Rational,
    pub campaign_hazard_probability: Rational,
    pub maximum_path_cost: u64,
    pub policy: super::PolicyPlan,
    pub stats: LookaheadEigStats,
}

#[derive(Clone, Debug, PartialEq)]
struct LookaheadCandidate {
    expected_terminal_entropy: f64,
    expected_cost: Rational,
    hazard_probability: Rational,
    maximum_path_cost: u64,
    policy: super::PolicyPlan,
}

#[derive(Clone, Debug, PartialEq)]
struct PartialLookaheadCandidate {
    expected_terminal_entropy: f64,
    expected_cost: Rational,
    hazard_probability: Rational,
    maximum_path_cost: u64,
    branches: Vec<super::PolicyBranch>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct JointOutcomeBranch {
    observation: Option<usize>,
    probability: Rational,
    posterior: Vec<JointMass>,
}

struct LookaheadSolver<'a> {
    kernel: &'a JointKernel,
    config: LookaheadEigConfig,
    stats: LookaheadEigStats,
    memo: HashMap<(Vec<JointMass>, usize, u64), Vec<LookaheadCandidate>>,
}

const MAX_LOOKAHEAD_HORIZON: usize = 12;

/// Optimize expected terminal mechanism entropy over a finite observable-history
/// policy tree, subject to a path-cost budget and campaign hazard budget.
///
/// The action space and transition probabilities are discrete and exact. The
/// entropy objective is floating point. This is a finite scenario-tree comparator,
/// not a reproduction of an amortized or continuous-design optimizer.
pub fn optimize_finite_lookahead_eig(
    kernel: &JointKernel,
    belief: &[JointMass],
    config: &LookaheadEigConfig,
) -> Result<LookaheadEigResult, String> {
    kernel.validate()?;
    validate_belief(kernel, belief)?;
    if config.horizon > MAX_LOOKAHEAD_HORIZON {
        return Err(format!(
            "lookahead horizon exceeds the {MAX_LOOKAHEAD_HORIZON}-step cap"
        ));
    }
    if !config.max_campaign_hazard.is_probability() {
        return Err("campaign hazard budget must lie in [0, 1]".into());
    }
    if config.max_memo_states == 0
        || config.max_generated_combinations == 0
        || config.max_frontier_points == 0
    {
        return Err("all lookahead solver resource caps must be positive".into());
    }

    let mut normalized_belief = belief.to_vec();
    normalized_belief.sort();
    let initial_entropy = belief_entropy(
        &normalized_belief,
        kernel.hypothesis_ids.len(),
    )?;
    let mut solver = LookaheadSolver {
        kernel,
        config: *config,
        stats: LookaheadEigStats::default(),
        memo: HashMap::new(),
    };
    let frontier = solver.solve(
        &normalized_belief,
        config.horizon,
        config.max_path_cost,
    )?;
    let selected = frontier
        .iter()
        .filter(|candidate| {
            candidate.hazard_probability <= config.max_campaign_hazard
        })
        .min_by(lookahead_candidate_order)
        .ok_or_else(|| "lookahead solver produced no feasible policy".to_string())?;

    let information_gain =
        (initial_entropy - selected.expected_terminal_entropy).max(0.0);
    Ok(LookaheadEigResult {
        expected_information_gain_nats: information_gain,
        expected_terminal_entropy_nats: selected.expected_terminal_entropy,
        expected_experiment_cost: selected.expected_cost,
        campaign_hazard_probability: selected.hazard_probability,
        maximum_path_cost: selected.maximum_path_cost,
        policy: selected.policy.clone(),
        stats: solver.stats,
    })
}

impl LookaheadSolver<'_> {
    fn solve(
        &mut self,
        belief: &[JointMass],
        remaining_horizon: usize,
        remaining_path_cost: u64,
    ) -> Result<Vec<LookaheadCandidate>, String> {
        let key = (
            belief.to_vec(),
            remaining_horizon,
            remaining_path_cost,
        );
        if let Some(cached) = self.memo.get(&key) {
            self.stats.memo_hits = self.stats.memo_hits
                .checked_add(1)
                .ok_or_else(|| "lookahead memo-hit counter overflow".to_string())?;
            return Ok(cached.clone());
        }
        if self.memo.len() >= self.config.max_memo_states {
            return Err("lookahead memo-state budget exceeded".into());
        }
        self.stats.expanded_belief_states = self.stats.expanded_belief_states
            .checked_add(1)
            .ok_or_else(|| "lookahead state counter overflow".to_string())?;

        let mut candidates = vec![LookaheadCandidate {
            expected_terminal_entropy: belief_entropy(
                belief,
                self.kernel.hypothesis_ids.len(),
            )?,
            expected_cost: Rational::zero(),
            hazard_probability: Rational::zero(),
            maximum_path_cost: 0,
            policy: super::PolicyPlan::Abstain,
        }];

        if remaining_horizon > 0 {
            for action in 0..self.kernel.action_ids.len() {
                let action_cost = self.kernel.action_costs[action];
                if action_cost > remaining_path_cost {
                    continue;
                }

                let outcomes = partition_joint_belief(self.kernel, belief, action)?;
                let hazard_branch = outcomes.iter().find(|branch| branch.observation.is_none());
                let hazard_probability = hazard_branch
                    .map(|branch| branch.probability)
                    .unwrap_or_else(Rational::zero);
                let hazard_entropy = match hazard_branch {
                    Some(branch) => {
                        to_f64(branch.probability)
                            * belief_entropy(
                                &branch.posterior,
                                self.kernel.hypothesis_ids.len(),
                            )?
                    }
                    None => 0.0,
                };

                let mut partials = vec![PartialLookaheadCandidate {
                    expected_terminal_entropy: hazard_entropy,
                    expected_cost: Rational::from_u64(action_cost),
                    hazard_probability,
                    maximum_path_cost: action_cost,
                    branches: Vec::new(),
                }];

                for branch in outcomes.iter().filter(|branch| branch.observation.is_some()) {
                    let observation = branch.observation
                        .expect("filtered safe outcome must have an observation");
                    let child_frontier = self.solve(
                        &branch.posterior,
                        remaining_horizon - 1,
                        remaining_path_cost - action_cost,
                    )?;
                    let combination_count = partials.len()
                        .checked_mul(child_frontier.len())
                        .ok_or_else(|| "lookahead combination count overflow".to_string())?;
                    self.stats.generated_combinations = self.stats.generated_combinations
                        .checked_add(combination_count)
                        .ok_or_else(|| "lookahead combination counter overflow".to_string())?;
                    if self.stats.generated_combinations
                        > self.config.max_generated_combinations
                    {
                        return Err("lookahead generated-combination budget exceeded".into());
                    }

                    let mut expanded = Vec::with_capacity(combination_count);
                    for partial in &partials {
                        for child in &child_frontier {
                            let probability = branch.probability;
                            let mut next = partial.clone();
                            next.expected_terminal_entropy +=
                                to_f64(probability) * child.expected_terminal_entropy;
                            next.expected_cost = next.expected_cost.checked_add(
                                probability.checked_mul(child.expected_cost)?,
                            )?;
                            next.hazard_probability = next.hazard_probability.checked_add(
                                probability.checked_mul(child.hazard_probability)?,
                            )?;
                            let child_path = action_cost
                                .checked_add(child.maximum_path_cost)
                                .ok_or_else(|| "lookahead path-cost counter overflow".to_string())?;
                            next.maximum_path_cost =
                                next.maximum_path_cost.max(child_path);
                            next.branches.push(super::PolicyBranch {
                                observation,
                                plan: Box::new(child.policy.clone()),
                            });
                            expanded.push(next);
                        }
                    }
                    partials = expanded;
                }

                for partial in partials {
                    if partial.maximum_path_cost <= remaining_path_cost
                        && partial.hazard_probability.is_probability()
                    {
                        candidates.push(LookaheadCandidate {
                            expected_terminal_entropy: partial.expected_terminal_entropy,
                            expected_cost: partial.expected_cost,
                            hazard_probability: partial.hazard_probability,
                            maximum_path_cost: partial.maximum_path_cost,
                            policy: super::PolicyPlan::Action {
                                action,
                                branches: partial.branches,
                            },
                        });
                    }
                }
            }
        }

        let frontier = prune_lookahead_candidates(
            candidates,
            self.config.max_frontier_points,
        )?;
        self.stats.maximum_frontier_points = self.stats.maximum_frontier_points
            .max(frontier.len());
        self.memo.insert(key, frontier.clone());
        Ok(frontier)
    }
}

fn partition_joint_belief(
    kernel: &JointKernel,
    belief: &[JointMass],
    action: usize,
) -> Result<Vec<JointOutcomeBranch>, String> {
    let mut by_outcome: BTreeMap<
        Option<usize>,
        BTreeMap<(usize, usize, usize), Rational>,
    > = BTreeMap::new();

    for point in belief {
        let row_index = kernel.row_index(
            point.hypothesis,
            point.physical_state,
            point.sensor_state,
            action,
        );
        for outcome in &kernel.rows[row_index].outcomes {
            let path_mass = point.mass.checked_mul(outcome.probability)?;
            let states = by_outcome.entry(outcome.observation).or_default();
            let key = (
                point.hypothesis,
                outcome.next_physical_state,
                outcome.next_sensor_state,
            );
            let old = states.get(&key).copied().unwrap_or_else(Rational::zero);
            states.insert(key, old.checked_add(path_mass)?);
        }
    }

    let mut total_probability = Rational::zero();
    let mut branches = Vec::with_capacity(by_outcome.len());
    for (observation, states) in by_outcome {
        let probability = states.values().try_fold(
            Rational::zero(),
            |sum, mass| sum.checked_add(*mass),
        )?;
        if probability.is_zero() {
            return Err("lookahead partition contains a zero-mass outcome".into());
        }
        total_probability = total_probability.checked_add(probability)?;
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
        branches.push(JointOutcomeBranch {
            observation,
            probability,
            posterior,
        });
    }
    if total_probability != Rational::one() {
        return Err("lookahead joint outcome partition did not conserve probability mass".into());
    }
    Ok(branches)
}

fn belief_entropy(
    belief: &[JointMass],
    hypothesis_count: usize,
) -> Result<f64, String> {
    let mut hypothesis_masses = vec![Rational::zero(); hypothesis_count];
    for point in belief {
        hypothesis_masses[point.hypothesis] =
            hypothesis_masses[point.hypothesis].checked_add(point.mass)?;
    }
    entropy(&hypothesis_masses, Rational::one())
}

fn lookahead_candidate_order(
    left: &LookaheadCandidate,
    right: &LookaheadCandidate,
) -> std::cmp::Ordering {
    left.expected_terminal_entropy
        .total_cmp(&right.expected_terminal_entropy)
        .then_with(|| left.expected_cost.cmp(&right.expected_cost))
        .then_with(|| left.hazard_probability.cmp(&right.hazard_probability))
        .then_with(|| left.maximum_path_cost.cmp(&right.maximum_path_cost))
        .then_with(|| left.policy.cmp(&right.policy))
}

fn lookahead_dominates(
    left: &LookaheadCandidate,
    right: &LookaheadCandidate,
) -> bool {
    let no_worse = left.expected_terminal_entropy <= right.expected_terminal_entropy
        && left.expected_cost <= right.expected_cost
        && left.hazard_probability <= right.hazard_probability
        && left.maximum_path_cost <= right.maximum_path_cost;
    let strictly_better =
        left.expected_terminal_entropy < right.expected_terminal_entropy
            || left.expected_cost < right.expected_cost
            || left.hazard_probability < right.hazard_probability
            || left.maximum_path_cost < right.maximum_path_cost;
    no_worse && strictly_better
}

fn prune_lookahead_candidates(
    mut candidates: Vec<LookaheadCandidate>,
    max_frontier_points: usize,
) -> Result<Vec<LookaheadCandidate>, String> {
    candidates.sort_by(lookahead_candidate_order);
    let mut frontier: Vec<LookaheadCandidate> = Vec::new();
    for candidate in candidates {
        let dominated = frontier.iter().any(|other| {
            (other.expected_terminal_entropy.total_cmp(
                &candidate.expected_terminal_entropy,
            ).is_eq()
                && other.expected_cost == candidate.expected_cost
                && other.hazard_probability == candidate.hazard_probability
                && other.maximum_path_cost == candidate.maximum_path_cost)
                || lookahead_dominates(other, &candidate)
        });
        if dominated {
            continue;
        }
        frontier.retain(|other| !lookahead_dominates(&candidate, other));
        frontier.push(candidate);
        if frontier.len() > max_frontier_points {
            return Err("lookahead frontier-point budget exceeded".into());
        }
    }
    frontier.sort_by(lookahead_candidate_order);
    Ok(frontier)
}
