//! Exact finite-horizon observational equivalence and distinguishability.
//!
//! For each pair of mechanism hypotheses, this module maximizes total variation
//! between the public observation-transcript distributions over every
//! deterministic adaptive policy in the declared finite horizon. The policy
//! sees only public observations and is shared by both hypotheses.

use super::{
    JointKernel, Rational,
    benchmark::fingerprint_model_family,
    robust::ModelWorld,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};

const MAX_IDENTIFIABILITY_HORIZON: usize = 12;
const MAX_PAIRWISE_HYPOTHESES: usize = 4_096;
const MAX_PAIR_POLICY_NODES: usize = 65_536;

type StateBelief = Vec<Rational>;
type MemoKey = (StateBelief, StateBelief, usize);
type Transcript = Vec<TranscriptEvent>;
type TranscriptMass = BTreeMap<Transcript, Rational>;

/// A finite-horizon distinguishing policy over public observations only.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DistinguishingPolicy {
    Stop,
    Action {
        action: usize,
        branches: Vec<DistinguishingBranch>,
    },
}

/// Continuation after one public observation.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct DistinguishingBranch {
    pub observation: usize,
    pub continuation: Box<DistinguishingPolicy>,
}

/// Public transcript events used by independent policy replay.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum TranscriptEvent {
    Observation(usize),
    Hazard,
    Stop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DistinguishabilityConfig {
    pub horizon: usize,
    pub max_pair_count: usize,
    pub max_memo_states_per_pair: usize,
    pub max_generated_branch_combinations_per_pair: usize,
    pub max_policy_nodes: usize,
}

impl Default for DistinguishabilityConfig {
    fn default() -> Self {
        Self {
            horizon: 4,
            max_pair_count: MAX_PAIRWISE_HYPOTHESES,
            max_memo_states_per_pair: 100_000,
            max_generated_branch_combinations_per_pair: 1_000_000,
            max_policy_nodes: MAX_PAIR_POLICY_NODES,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DistinguishabilityStats {
    pub memoized_states: usize,
    pub memo_hits: usize,
    pub generated_branch_combinations: usize,
    pub maximum_policy_nodes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairwiseDistinguishability {
    pub hypothesis_a: String,
    pub hypothesis_b: String,
    /// Maximum total variation between transcript distributions over all
    /// admissible shared public-history policies within the horizon.
    pub max_total_variation: Rational,
    pub distinguishing_policy: DistinguishingPolicy,
    pub stats: DistinguishabilityStats,
}

impl PairwiseDistinguishability {
    pub fn observationally_equivalent(&self) -> bool {
        self.max_total_variation.is_zero()
    }

    pub fn perfectly_separable(&self) -> bool {
        self.max_total_variation == Rational::one()
    }

    /// Bayes error for discriminating this pair with equal prior probabilities.
    ///
    /// For equal priors, the optimal one-shot transcript classifier has error
    /// (1 - TV) / 2. This is conditional on the witness-optimal transcript
    /// experiment and does not describe unequal hypothesis priors.
    pub fn minimum_equal_prior_classification_error(&self) -> Result<Rational, String> {
        Rational::one()
            .checked_sub(self.max_total_variation)?
            .checked_mul(Rational::new(1, 2)?)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationalEquivalenceClass {
    /// Stable representative: first member in kernel hypothesis order.
    pub representative: String,
    pub members: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentifiabilityCertificate {
    pub horizon: usize,
    pub model_kernel_fingerprint: String,
    pub hypothesis_ids: Vec<String>,
    pub equivalence_classes: Vec<ObservationalEquivalenceClass>,
    pub pairwise: Vec<PairwiseDistinguishability>,
    pub aggregate_stats: DistinguishabilityStats,
}

/// A terminal claim at the finest resolution the finite-horizon certificate supports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EquivalenceClassClaim {
    pub representative: String,
    pub members: Vec<String>,
}

/// A terminal decision against the certified observational quotient.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalIdentityDecision {
    /// None means the decision abstains.
    pub claim: Option<EquivalenceClassClaim>,
    pub correct_identification_probability: Rational,
    pub false_identification_probability: Rational,
    pub abstention_probability: Rational,
}

/// Choose a terminal equivalence-class claim under an explicit false-claim budget.
///
/// This does not change the adaptive policy optimizer. It is a safe terminal
/// decision layer: hypotheses that cannot be distinguished under the supplied
/// horizon are claimed together. A tie between distinct quotient classes
/// abstains rather than arbitrarily selecting one representative.
pub fn select_terminal_equivalence_class(
    certificate: &IdentifiabilityCertificate,
    posterior: &[Rational],
    max_false_identification: Rational,
) -> Result<TerminalIdentityDecision, String> {
    if posterior.len() != certificate.hypothesis_ids.len() {
        return Err("terminal posterior length does not match certificate hypothesis vocabulary".into());
    }
    if !max_false_identification.is_probability() {
        return Err("maximum false-identification budget must lie in [0, 1]".into());
    }
    let posterior_mass = posterior.iter().try_fold(
        Rational::zero(),
        |sum, mass| sum.checked_add(*mass),
    )?;
    if posterior_mass != Rational::one() {
        return Err("terminal posterior mass must sum exactly to one".into());
    }

    let mut seen = BTreeSet::new();
    let mut class_masses = Vec::with_capacity(certificate.equivalence_classes.len());
    for class in &certificate.equivalence_classes {
        if class.members.is_empty() || class.representative != class.members[0] {
            return Err("certificate contains an empty or non-canonical equivalence class".into());
        }
        let mut mass = Rational::zero();
        for member in &class.members {
            let index = certificate.hypothesis_ids
                .iter()
                .position(|hypothesis| hypothesis == member)
                .ok_or_else(|| format!("certificate class contains unknown hypothesis {member}"))?;
            if !seen.insert(index) {
                return Err("certificate equivalence classes contain duplicate hypotheses".into());
            }
            mass = mass.checked_add(posterior[index])?;
        }
        class_masses.push((class, mass));
    }
    if seen.len() != certificate.hypothesis_ids.len() {
        return Err("certificate equivalence classes do not partition the hypothesis vocabulary".into());
    }
    if class_masses.is_empty() {
        return Err("certificate contains no equivalence classes".into());
    }

    let maximum = class_masses.iter()
        .map(|(_, mass)| *mass)
        .max()
        .ok_or_else(|| "certificate contains no class masses".to_string())?;
    let maximum_count = class_masses.iter()
        .filter(|(_, mass)| *mass == maximum)
        .count();
    if maximum.is_zero()
        || maximum_count != 1
        || Rational::one().checked_sub(maximum)? > max_false_identification
    {
        return Ok(TerminalIdentityDecision {
            claim: None,
            correct_identification_probability: Rational::zero(),
            false_identification_probability: Rational::zero(),
            abstention_probability: Rational::one(),
        });
    }

    let (class, _) = class_masses.into_iter()
        .find(|(_, mass)| *mass == maximum)
        .ok_or_else(|| "maximum equivalence class disappeared".to_string())?;
    Ok(TerminalIdentityDecision {
        claim: Some(EquivalenceClassClaim {
            representative: class.representative.clone(),
            members: class.members.clone(),
        }),
        correct_identification_probability: maximum,
        false_identification_probability: Rational::one().checked_sub(maximum)?,
        abstention_probability: Rational::zero(),
    })
}

/// Compute exact pairwise finite-horizon distinguishability and quotient classes.
///
/// The initial belief is conditioned separately on each mechanism hypothesis.
/// Physical and sensor state distributions remain joint. A zero-prior
/// hypothesis is rejected because its conditional state distribution is
/// undefined under the supplied kernel.
pub fn analyze_identifiability(
    kernel: &JointKernel,
    config: &DistinguishabilityConfig,
) -> Result<IdentifiabilityCertificate, String> {
    kernel.validate()?;
    if config.horizon > MAX_IDENTIFIABILITY_HORIZON {
        return Err(format!(
            "identifiability horizon exceeds {MAX_IDENTIFIABILITY_HORIZON}"
        ));
    }
    if config.max_pair_count == 0
        || config.max_memo_states_per_pair == 0
        || config.max_generated_branch_combinations_per_pair == 0
        || config.max_policy_nodes == 0
    {
        return Err("all identifiability resource caps must be positive".into());
    }

    let hypothesis_count = kernel.hypothesis_ids.len();
    let pair_count = hypothesis_count
        .checked_mul(hypothesis_count.saturating_sub(1))
        .map(|value| value / 2)
        .ok_or_else(|| "hypothesis-pair count overflow".to_string())?;
    if pair_count > config.max_pair_count {
        return Err("identifiability hypothesis-pair budget exceeded".into());
    }

    let conditional_beliefs = conditional_initial_beliefs(kernel)?;
    let model_kernel_fingerprint = fingerprint_model_family(&[ModelWorld {
        id: "identifiability-kernel".into(),
        kernel: kernel.clone(),
    }])?;

    let mut pairwise = Vec::with_capacity(pair_count);
    let mut aggregate_stats = DistinguishabilityStats::default();
    let mut equivalence = UnionFind::new(hypothesis_count);

    for left in 0..hypothesis_count {
        for right in left + 1..hypothesis_count {
            let mut solver = PairSolver {
                kernel,
                left_hypothesis: left,
                right_hypothesis: right,
                config: *config,
                memo: HashMap::new(),
                stats: DistinguishabilityStats::default(),
            };
            let solved = solver.solve(
                &conditional_beliefs[left],
                &conditional_beliefs[right],
                config.horizon,
            )?;
            let node_count = policy_node_count(&solved.policy)?;
            if node_count > config.max_policy_nodes {
                return Err("distinguishing-policy node budget exceeded".into());
            }
            solver.stats.maximum_policy_nodes = node_count;

            let replayed = evaluate_pair_policy(
                kernel,
                left,
                right,
                &solved.policy,
                config.horizon,
            )?;
            if replayed != solved.value {
                return Err(
                    "independent distinguishing-policy replay disagrees with dynamic program"
                        .into(),
                );
            }

            if solved.value.is_zero() {
                equivalence.union(left, right);
            }
            aggregate_stats.memoized_states = aggregate_stats
                .memoized_states
                .checked_add(solver.stats.memoized_states)
                .ok_or_else(|| "aggregate memo-state counter overflow".to_string())?;
            aggregate_stats.memo_hits = aggregate_stats
                .memo_hits
                .checked_add(solver.stats.memo_hits)
                .ok_or_else(|| "aggregate memo-hit counter overflow".to_string())?;
            aggregate_stats.generated_branch_combinations = aggregate_stats
                .generated_branch_combinations
                .checked_add(solver.stats.generated_branch_combinations)
                .ok_or_else(|| "aggregate branch counter overflow".to_string())?;
            aggregate_stats.maximum_policy_nodes = aggregate_stats
                .maximum_policy_nodes
                .max(node_count);

            pairwise.push(PairwiseDistinguishability {
                hypothesis_a: kernel.hypothesis_ids[left].clone(),
                hypothesis_b: kernel.hypothesis_ids[right].clone(),
                max_total_variation: solved.value,
                distinguishing_policy: solved.policy,
                stats: solver.stats,
            });
        }
    }

    let mut grouped: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for hypothesis in 0..hypothesis_count {
        let root = equivalence.find(hypothesis);
        grouped.entry(root).or_default().push(hypothesis);
    }
    let equivalence_classes = grouped
        .into_values()
        .map(|members| {
            let names: Vec<String> = members
                .iter()
                .map(|index| kernel.hypothesis_ids[*index].clone())
                .collect();
            ObservationalEquivalenceClass {
                representative: names[0].clone(),
                members: names,
            }
        })
        .collect();

    Ok(IdentifiabilityCertificate {
        horizon: config.horizon,
        model_kernel_fingerprint,
        hypothesis_ids: kernel.hypothesis_ids.clone(),
        equivalence_classes,
        pairwise,
        aggregate_stats,
    })
}

fn conditional_initial_beliefs(kernel: &JointKernel) -> Result<Vec<StateBelief>, String> {
    let state_count = kernel
        .physical_state_count
        .checked_mul(kernel.sensor_state_count)
        .ok_or_else(|| "joint state-count overflow".to_string())?;
    let hypothesis_count = kernel.hypothesis_ids.len();
    let mut beliefs = vec![vec![Rational::zero(); state_count]; hypothesis_count];
    let mut hypothesis_mass = vec![Rational::zero(); hypothesis_count];

    for point in &kernel.initial_belief {
        let state = point.physical_state * kernel.sensor_state_count + point.sensor_state;
        beliefs[point.hypothesis][state] =
            beliefs[point.hypothesis][state].checked_add(point.mass)?;
        hypothesis_mass[point.hypothesis] =
            hypothesis_mass[point.hypothesis].checked_add(point.mass)?;
    }

    for hypothesis in 0..hypothesis_count {
        if hypothesis_mass[hypothesis].is_zero() {
            return Err(format!(
                "hypothesis {} has zero initial prior mass; conditional identifiability is undefined",
                kernel.hypothesis_ids[hypothesis]
            ));
        }
        for mass in &mut beliefs[hypothesis] {
            *mass = mass.checked_div(hypothesis_mass[hypothesis])?;
        }
    }
    Ok(beliefs)
}

struct SolveResult {
    value: Rational,
    policy: DistinguishingPolicy,
}

struct PairSolver<'a> {
    kernel: &'a JointKernel,
    left_hypothesis: usize,
    right_hypothesis: usize,
    config: DistinguishabilityConfig,
    memo: HashMap<MemoKey, SolveResult>,
    stats: DistinguishabilityStats,
}

impl PairSolver<'_> {
    fn solve(
        &mut self,
        left: &StateBelief,
        right: &StateBelief,
        remaining: usize,
    ) -> Result<SolveResult, String> {
        let key = (left.clone(), right.clone(), remaining);
        if let Some(cached) = self.memo.get(&key) {
            self.stats.memo_hits = self.stats.memo_hits
                .checked_add(1)
                .ok_or_else(|| "memo-hit counter overflow".to_string())?;
            return Ok(SolveResult {
                value: cached.value,
                policy: cached.policy.clone(),
            });
        }
        if self.memo.len() >= self.config.max_memo_states_per_pair {
            return Err("identifiability memo-state budget exceeded".into());
        }
        self.stats.memoized_states = self.stats.memoized_states
            .checked_add(1)
            .ok_or_else(|| "memo-state counter overflow".to_string())?;

        let left_mass = sum_masses(left)?;
        let right_mass = sum_masses(right)?;
        let terminal_value = half_abs_difference(left_mass, right_mass)?;
        let mut best = SolveResult {
            value: terminal_value,
            policy: DistinguishingPolicy::Stop,
        };

        if remaining > 0 && !(left_mass.is_zero() && right_mass.is_zero()) {
            for action in 0..self.kernel.action_ids.len() {
                let left_partition = partition_one_hypothesis(
                    self.kernel,
                    self.left_hypothesis,
                    left,
                    action,
                )?;
                let right_partition = partition_one_hypothesis(
                    self.kernel,
                    self.right_hypothesis,
                    right,
                    action,
                )?;

                let mut value = half_abs_difference(
                    left_partition.hazard_probability,
                    right_partition.hazard_probability,
                )?;
                let observation_ids: BTreeSet<usize> = left_partition
                    .observations
                    .keys()
                    .chain(right_partition.observations.keys())
                    .copied()
                    .collect();
                let mut branches = Vec::with_capacity(observation_ids.len());

                for observation in observation_ids {
                    self.stats.generated_branch_combinations = self.stats
                        .generated_branch_combinations
                        .checked_add(1)
                        .ok_or_else(|| "branch-combination counter overflow".to_string())?;
                    if self.stats.generated_branch_combinations
                        > self.config.max_generated_branch_combinations_per_pair
                    {
                        return Err("identifiability branch-combination budget exceeded".into());
                    }

                    let zero = vec![
                        Rational::zero();
                        self.kernel.physical_state_count * self.kernel.sensor_state_count
                    ];
                    let left_next = left_partition
                        .observations
                        .get(&observation)
                        .unwrap_or(&zero);
                    let right_next = right_partition
                        .observations
                        .get(&observation)
                        .unwrap_or(&zero);
                    let child = self.solve(left_next, right_next, remaining - 1)?;
                    value = value.checked_add(child.value)?;
                    branches.push(DistinguishingBranch {
                        observation,
                        continuation: Box::new(child.policy),
                    });
                }

                let policy = DistinguishingPolicy::Action { action, branches };
                let nodes = policy_node_count(&policy)?;
                if nodes > self.config.max_policy_nodes {
                    return Err("distinguishing-policy node budget exceeded".into());
                }
                if value > best.value {
                    best = SolveResult { value, policy };
                }
            }
        }

        self.memo.insert(
            key,
            SolveResult {
                value: best.value,
                policy: best.policy.clone(),
            },
        );
        Ok(best)
    }
}

struct Partition {
    hazard_probability: Rational,
    observations: BTreeMap<usize, StateBelief>,
}

fn partition_one_hypothesis(
    kernel: &JointKernel,
    hypothesis: usize,
    belief: &StateBelief,
    action: usize,
) -> Result<Partition, String> {
    let mut hazard_probability = Rational::zero();
    let mut observations: BTreeMap<usize, StateBelief> = BTreeMap::new();
    for (state_index, state_mass) in belief.iter().enumerate() {
        if state_mass.is_zero() {
            continue;
        }
        let physical_state = state_index / kernel.sensor_state_count;
        let sensor_state = state_index % kernel.sensor_state_count;
        let row = &kernel.rows[kernel.row_index(
            hypothesis,
            physical_state,
            sensor_state,
            action,
        )];
        for outcome in &row.outcomes {
            let path_mass = state_mass.checked_mul(outcome.probability)?;
            match outcome.observation {
                None => {
                    hazard_probability = hazard_probability.checked_add(path_mass)?;
                }
                Some(observation) => {
                    let next_index = outcome.next_physical_state
                        * kernel.sensor_state_count
                        + outcome.next_sensor_state;
                    let next = observations.entry(observation).or_insert_with(|| {
                        vec![
                            Rational::zero();
                            kernel.physical_state_count * kernel.sensor_state_count
                        ]
                    });
                    next[next_index] = next[next_index].checked_add(path_mass)?;
                }
            }
        }
    }
    Ok(Partition {
        hazard_probability,
        observations,
    })
}

fn sum_masses(masses: &[Rational]) -> Result<Rational, String> {
    masses.iter().try_fold(
        Rational::zero(),
        |sum, mass| sum.checked_add(*mass),
    )
}

fn half_abs_difference(left: Rational, right: Rational) -> Result<Rational, String> {
    let difference = if left >= right {
        left.checked_sub(right)?
    } else {
        right.checked_sub(left)?
    };
    difference.checked_mul(Rational::new(1, 2)?)
}

fn policy_node_count(policy: &DistinguishingPolicy) -> Result<usize, String> {
    match policy {
        DistinguishingPolicy::Stop => Ok(1),
        DistinguishingPolicy::Action { branches, .. } => {
            let children = branches.iter().try_fold(0_usize, |total, branch| {
                total.checked_add(policy_node_count(&branch.continuation)?)
                    .ok_or_else(|| "policy node count overflow".to_string())
            })?;
            1_usize.checked_add(children)
                .ok_or_else(|| "policy node count overflow".to_string())
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum TranscriptEvent {
    Observation(usize),
    Hazard,
    Stop,
}

/// Independently replay one distinguishing policy under one hypothesis.
fn replay_hypothesis(
    kernel: &JointKernel,
    hypothesis: usize,
    policy: &DistinguishingPolicy,
    belief: &[Rational],
    remaining: usize,
) -> Result<TranscriptMass, String> {
    let mut result = TranscriptMass::new();
    let mut path = Vec::new();
    replay_node(
        kernel,
        hypothesis,
        policy,
        belief,
        0,
        remaining,
        &mut path,
        &mut result,
    )?;
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn replay_node(
    kernel: &JointKernel,
    hypothesis: usize,
    policy: &DistinguishingPolicy,
    belief: &[Rational],
    depth: usize,
    max_steps: usize,
    path: &mut Transcript,
    result: &mut TranscriptMass,
) -> Result<(), String> {
    match policy {
        DistinguishingPolicy::Stop => {
            path.push(TranscriptEvent::Stop);
            add_transcript_mass(result, path, sum_masses(belief)?)?;
            path.pop();
        }
        DistinguishingPolicy::Action { action, branches } => {
            if depth >= max_steps || *action >= kernel.action_ids.len() {
                return Err("distinguishing policy exceeds horizon or uses invalid action".into());
            }
            let mut seen_observations = BTreeSet::new();
            for branch in branches {
                if branch.observation >= kernel.observation_ids.len()
                    || !seen_observations.insert(branch.observation)
                {
                    return Err("distinguishing policy has invalid or duplicate observation branches".into());
                }
            }

            for (state_index, state_mass) in belief.iter().enumerate() {
                if state_mass.is_zero() {
                    continue;
                }
                let physical_state = state_index / kernel.sensor_state_count;
                let sensor_state = state_index % kernel.sensor_state_count;
                let row = &kernel.rows[kernel.row_index(
                    hypothesis,
                    physical_state,
                    sensor_state,
                    *action,
                )];
                for outcome in &row.outcomes {
                    let path_mass = state_mass.checked_mul(outcome.probability)?;
                    match outcome.observation {
                        None => {
                            path.push(TranscriptEvent::Hazard);
                            add_transcript_mass(result, path, path_mass)?;
                            path.pop();
                        }
                        Some(observation) => {
                            let branch = branches.iter()
                                .find(|branch| branch.observation == observation)
                                .ok_or_else(|| format!(
                                    "distinguishing policy lacks reachable observation branch {observation}"
                                ))?;
                            let next_index = outcome.next_physical_state
                                * kernel.sensor_state_count
                                + outcome.next_sensor_state;
                            let mut next_belief = vec![
                                Rational::zero();
                                kernel.physical_state_count * kernel.sensor_state_count
                            ];
                            next_belief[next_index] = path_mass;
                            path.push(TranscriptEvent::Observation(observation));
                            replay_node(
                                kernel,
                                hypothesis,
                                &branch.continuation,
                                &next_belief,
                                depth + 1,
                                max_steps,
                                path,
                                result,
                            )?;
                            path.pop();
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn add_transcript_mass(
    result: &mut TranscriptMass,
    path: &[TranscriptEvent],
    mass: Rational,
) -> Result<(), String> {
    let current = result.get(path).copied().unwrap_or(Rational::zero());
    result.insert(path.to_vec(), current.checked_add(mass)?);
    Ok(())
}

/// Independently replay the policy under both mechanisms and return exact TV.
pub fn evaluate_pair_policy(
    kernel: &JointKernel,
    hypothesis_a: usize,
    hypothesis_b: usize,
    policy: &DistinguishingPolicy,
    horizon: usize,
) -> Result<Rational, String> {
    kernel.validate()?;
    if hypothesis_a >= kernel.hypothesis_ids.len()
        || hypothesis_b >= kernel.hypothesis_ids.len()
        || hypothesis_a == hypothesis_b
    {
        return Err("pair policy replay requires two distinct valid hypotheses".into());
    }
    if horizon > MAX_IDENTIFIABILITY_HORIZON {
        return Err("pair policy replay horizon exceeds cap".into());
    }
    if policy_node_count(policy)? > MAX_PAIR_POLICY_NODES {
        return Err("distinguishing-policy node budget exceeded".into());
    }
    let initial = conditional_initial_beliefs(kernel)?;
    let left = replay_hypothesis(kernel, hypothesis_a, policy, &initial[hypothesis_a], horizon)?;
    let right = replay_hypothesis(kernel, hypothesis_b, policy, &initial[hypothesis_b], horizon)?;
    let mut transcripts = BTreeSet::new();
    transcripts.extend(left.keys().cloned());
    transcripts.extend(right.keys().cloned());

    let mut l1 = Rational::zero();
    for transcript in transcripts {
        let left_mass = left.get(&transcript).copied().unwrap_or(Rational::zero());
        let right_mass = right.get(&transcript).copied().unwrap_or(Rational::zero());
        let difference = if left_mass >= right_mass {
            left_mass.checked_sub(right_mass)?
        } else {
            right_mass.checked_sub(left_mass)?
        };
        l1 = l1.checked_add(difference)?;
    }
    l1.checked_mul(Rational::new(1, 2)?)
}

/// Disjoint-set structure for exact pairwise observational equivalence.
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl UnionFind {
    fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
            rank: vec![0; size],
        }
    }

    fn find(&mut self, value: usize) -> usize {
        let parent = self.parent[value];
        if parent != value {
            let root = self.find(parent);
            self.parent[value] = root;
        }
        self.parent[value]
    }

    fn union(&mut self, left: usize, right: usize) {
        let left_root = self.find(left);
        let right_root = self.find(right);
        if left_root == right_root {
            return;
        }
        if self.rank[left_root] < self.rank[right_root] {
            self.parent[left_root] = right_root;
        } else if self.rank[left_root] > self.rank[right_root] {
            self.parent[right_root] = left_root;
        } else {
            self.parent[right_root] = left_root;
            self.rank[left_root] = self.rank[left_root].saturating_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JointMass, JointOutcome, KernelRow};

    fn half() -> Rational {
        Rational::new(1, 2).unwrap()
    }

    fn safe(probability: Rational, next_physical: usize, observation: usize) -> JointOutcome {
        JointOutcome {
            probability,
            next_physical_state: next_physical,
            next_sensor_state: 0,
            observation: Some(observation),
        }
    }

    fn kernel(
        hypothesis_count: usize,
        physical_count: usize,
        action_ids: Vec<&str>,
        make: impl Fn(usize, usize, usize) -> Vec<JointOutcome>,
    ) -> JointKernel {
        let mut rows = Vec::new();
        for hypothesis in 0..hypothesis_count {
            for physical in 0..physical_count {
                for action in 0..action_ids.len() {
                    rows.push(KernelRow {
                        hypothesis,
                        physical_state: physical,
                        sensor_state: 0,
                        action,
                        outcomes: make(hypothesis, physical, action),
                    });
                }
            }
        }
        let action_count = action_ids.len();
        JointKernel {
            hypothesis_ids: (0..hypothesis_count).map(|id| format!("h{id}")).collect(),
            physical_state_count: physical_count,
            sensor_state_count: 1,
            physical_state_ids: (0..physical_count).map(|id| format!("state-{id}")).collect(),
            sensor_state_ids: vec!["sensor-stable".into()],
            action_ids: action_ids.into_iter().map(str::to_string).collect(),
            observation_ids: vec!["zero".into(), "one".into()],
            action_costs: vec![1; action_count],
            initial_belief: (0..hypothesis_count)
                .map(|hypothesis| JointMass {
                    hypothesis,
                    physical_state: 0,
                    sensor_state: 0,
                    mass: Rational::new(1, hypothesis_count as u128).unwrap(),
                })
                .collect(),
            rows,
        }
    }

    fn config(horizon: usize) -> DistinguishabilityConfig {
        DistinguishabilityConfig {
            horizon,
            ..DistinguishabilityConfig::default()
        }
    }

    fn enumerate_policies(action_count: usize, observation_count: usize, horizon: usize)
        -> Vec<DistinguishingPolicy>
    {
        let mut policies = vec![DistinguishingPolicy::Stop];
        if horizon == 0 {
            return policies;
        }
        let child_policies = enumerate_policies(action_count, observation_count, horizon - 1);
        for action in 0..action_count {
            let mut combinations: Vec<Vec<DistinguishingBranch>> = vec![Vec::new()];
            for observation in 0..observation_count {
                let mut expanded = Vec::new();
                for prefix in &combinations {
                    for child in &child_policies {
                        let mut next = prefix.clone();
                        next.push(DistinguishingBranch {
                            observation,
                            continuation: Box::new(child.clone()),
                        });
                        expanded.push(next);
                    }
                }
                combinations = expanded;
            }
            policies.extend(combinations.into_iter().map(|branches| {
                DistinguishingPolicy::Action { action, branches }
            }));
        }
        policies
    }

    #[test]
    fn identical_observation_kernels_form_one_exact_equivalence_class() {
        let identical = kernel(3, 1, vec!["read"], |hypothesis, physical, _action| {
            let p_zero = if hypothesis == 2 {
                Rational::new(1, 4).unwrap()
            } else {
                Rational::new(3, 4).unwrap()
            };
            vec![
                safe(p_zero, physical, 0),
                safe(Rational::one().checked_sub(p_zero).unwrap(), physical, 1),
            ]
        });
        let certificate = analyze_identifiability(&identical, &config(4)).unwrap();
        assert_eq!(certificate.pairwise.len(), 3);
        assert_eq!(certificate.pairwise[0].max_total_variation, Rational::zero());
        assert!(certificate.pairwise[0].observationally_equivalent());
        assert_eq!(
            certificate.pairwise[1].max_total_variation,
            Rational::new(1, 2).unwrap()
        );
        assert_eq!(certificate.equivalence_classes.len(), 2);
        let equivalent = certificate.equivalence_classes.iter()
            .find(|class| class.members.len() == 2)
            .unwrap();
        assert_eq!(equivalent.members, vec!["h0".to_string(), "h1".to_string()]);
    }

    #[test]
    fn terminal_decision_claims_observable_quotient_class_not_arbitrary_member() {
        let model = kernel(3, 1, vec!["read"], |hypothesis, physical, _action| {
            let p_zero = if hypothesis == 2 {
                Rational::new(1, 4).unwrap()
            } else {
                Rational::new(3, 4).unwrap()
            };
            vec![
                safe(p_zero, physical, 0),
                safe(Rational::one().checked_sub(p_zero).unwrap(), physical, 1),
            ]
        });
        let certificate = analyze_identifiability(&model, &config(2)).unwrap();
        let posterior = vec![
            Rational::new(3, 8).unwrap(),
            Rational::new(3, 8).unwrap(),
            Rational::new(1, 4).unwrap(),
        ];
        let selected = select_terminal_equivalence_class(
            &certificate,
            &posterior,
            Rational::new(1, 4).unwrap(),
        ).unwrap();
        let claim = selected.claim.unwrap();
        assert_eq!(claim.members, vec!["h0".to_string(), "h1".to_string()]);
        assert_eq!(selected.correct_identification_probability, Rational::new(3, 4).unwrap());
        assert_eq!(selected.false_identification_probability, Rational::new(1, 4).unwrap());
        assert_eq!(selected.abstention_probability, Rational::zero());

        let strict = select_terminal_equivalence_class(
            &certificate,
            &posterior,
            Rational::zero(),
        ).unwrap();
        assert!(strict.claim.is_none());
        assert_eq!(strict.abstention_probability, Rational::one());
    }

    #[test]
    fn terminal_class_decision_abstains_on_quotient_mass_ties() {
        let model = kernel(2, 1, vec!["read"], |hypothesis, physical, _action| {
            vec![safe(Rational::one(), physical, hypothesis)]
        });
        let certificate = analyze_identifiability(&model, &config(1)).unwrap();
        let posterior = vec![half(), half()];
        let decision = select_terminal_equivalence_class(
            &certificate,
            &posterior,
            Rational::one(),
        ).unwrap();
        assert!(decision.claim.is_none());
        assert_eq!(decision.abstention_probability, Rational::one());
    }

    #[test]
    fn deterministic_measurement_is_perfectly_distinguishing() {
        let separating = kernel(2, 1, vec!["read"], |hypothesis, physical, _action| {
            vec![safe(Rational::one(), physical, hypothesis)]
        });
        let certificate = analyze_identifiability(&separating, &config(1)).unwrap();
        let pair = &certificate.pairwise[0];
        assert_eq!(pair.max_total_variation, Rational::one());
        assert!(pair.perfectly_separable());
        assert!(matches!(
            &pair.distinguishing_policy,
            DistinguishingPolicy::Action { action: 0, .. }
        ));
    }

    #[test]
    fn noisy_binary_channel_has_exact_half_total_variation() {
        let noisy = kernel(2, 1, vec!["read"], |hypothesis, physical, _action| {
            let p_zero = if hypothesis == 0 {
                Rational::new(3, 4).unwrap()
            } else {
                Rational::new(1, 4).unwrap()
            };
            vec![
                safe(p_zero, physical, 0),
                safe(Rational::one().checked_sub(p_zero).unwrap(), physical, 1),
            ]
        });
        let certificate = analyze_identifiability(&noisy, &config(1)).unwrap();
        assert_eq!(
            certificate.pairwise[0].max_total_variation,
            Rational::new(1, 2).unwrap()
        );
        assert_eq!(
            certificate.pairwise[0]
                .minimum_equal_prior_classification_error()
                .unwrap(),
            Rational::new(1, 4).unwrap()
        );
    }

    #[test]
    fn condition_then_measure_is_distinguishable_only_at_horizon_two() {
        let order_effect = kernel(
            2,
            2,
            vec!["condition", "measure"],
            |hypothesis, physical, action| {
                if action == 0 {
                    // Conditioning looks identical, but leaves a mechanism-specific
                    // physical state for the later measurement.
                    vec![safe(Rational::one(), if hypothesis == 0 { 1 } else { 0 }, 0)]
                } else {
                    vec![safe(Rational::one(), physical, physical)]
                }
            },
        );
        let one_step = analyze_identifiability(&order_effect, &config(1)).unwrap();
        let two_step = analyze_identifiability(&order_effect, &config(2)).unwrap();
        assert_eq!(one_step.pairwise[0].max_total_variation, Rational::zero());
        assert_eq!(two_step.pairwise[0].max_total_variation, Rational::one());
        assert!(matches!(
            &two_step.pairwise[0].distinguishing_policy,
            DistinguishingPolicy::Action { action: 0, .. }
        ));
    }

    #[test]
    fn persistent_sensor_bias_is_non_identifying_until_calibration() {
        let family = crate::challenge::generate_stateful_confounding_family(0).unwrap();
        let nominal = &family.worlds[0].kernel;
        let one_step = analyze_identifiability(nominal, &config(1)).unwrap();
        let two_steps = analyze_identifiability(nominal, &config(2)).unwrap();
        assert_eq!(one_step.pairwise[0].max_total_variation, Rational::zero());
        assert_eq!(two_steps.pairwise[0].max_total_variation, Rational::new(3, 4).unwrap());
    }

    #[test]
    fn horizon_two_dynamic_program_matches_independent_policy_enumeration() {
        let order_effect = kernel(
            2,
            2,
            vec!["condition", "measure"],
            |hypothesis, physical, action| {
                if action == 0 {
                    vec![safe(Rational::one(), if hypothesis == 0 { 1 } else { 0 }, 0)]
                } else {
                    vec![safe(Rational::one(), physical, physical)]
                }
            },
        );
        let certificate = analyze_identifiability(&order_effect, &config(2)).unwrap();
        let policies = enumerate_policies(2, 2, 2);
        let initial = conditional_initial_beliefs(&order_effect).unwrap();
        let mut brute = Rational::zero();
        for policy in policies {
            let score = evaluate_pair_policy(
                &order_effect,
                0,
                1,
                &policy,
                2,
            ).unwrap();
            brute = brute.max(score);
        }
        assert_eq!(certificate.pairwise[0].max_total_variation, brute);
        assert_eq!(evaluate_pair_policy(
            &order_effect,
            0,
            1,
            &certificate.pairwise[0].distinguishing_policy,
            2,
        ).unwrap(), brute);
        assert_eq!(initial.len(), 2);
    }

    #[test]
    fn observable_hazard_difference_is_a_distinguishing_event() {
        let hazard_shift = kernel(2, 1, vec!["test"], |hypothesis, physical, _action| {
            if hypothesis == 0 {
                vec![safe(Rational::one(), physical, 0)]
            } else {
                vec![JointOutcome {
                    probability: Rational::one(),
                    next_physical_state: physical,
                    next_sensor_state: 0,
                    observation: None,
                }]
            }
        });
        let certificate = analyze_identifiability(&hazard_shift, &config(1)).unwrap();
        assert_eq!(certificate.pairwise[0].max_total_variation, Rational::one());
    }

    #[test]
    fn zero_prior_hypotheses_and_resource_exhaustion_fail_closed() {
        let mut zero_prior_kernel = kernel(2, 1, vec!["read"], |_hypothesis, physical, _action| {
            vec![safe(Rational::one(), physical, 0)]
        });
        zero_prior_kernel.initial_belief.retain(|point| point.hypothesis == 0);
        zero_prior_kernel.initial_belief[0].mass = Rational::one();
        assert!(analyze_identifiability(&zero_prior_kernel, &config(1))
            .unwrap_err()
            .contains("zero initial prior mass"));

        let valid = kernel(2, 1, vec!["read"], |_hypothesis, physical, _action| {
            vec![safe(Rational::one(), physical, 0)]
        });
        assert!(analyze_identifiability(
            &valid,
            &DistinguishabilityConfig {
                max_memo_states_per_pair: 1,
                ..config(3)
            }
        ).is_err());
    }
}
