// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Cross-setup portability scorecard for matched policy checkpoints.
//!
//! A policy-pair matchup is decided only when their 90% Wilson intervals do
//! not overlap. Uncertain matchups remain undecided. Agreement confidence
//! intervals resample whole semantic task clusters, preserving dependence
//! among the many policy pairs drawn from one task.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{Display, Formatter};

pub const MIN_COMMON_TASK_CLUSTERS: usize = 10;
pub const MIN_BOOTSTRAP_REPLICATES: usize = 1_000;
const WILSON_Z_90: f64 = 1.644_853_626_951_472_2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SetupKind {
    Simulation,
    Physical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComparisonTier {
    WithinSetupRepeatability,
    MatchedSimulatorTwin,
    WithinTeamArticle,
    CrossTeamOrLab,
    CrossEmbodiment,
    CrossEngine,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PortabilityError {
    EmptySetupId,
    EmptyTeamId,
    EmptyRobotArticleId,
    EmptyEmbodiment,
    EmptyPhysicsEngineId,
    InvalidAssetDigest,
    InvalidControllerDigest,
    InvalidTaskContractDigest,
    InvalidPolicyId,
    InvalidPolicyCheckpointDigest,
    InvalidTaskId,
    InvalidSuccessCounts,
    DuplicateOutcome,
    DuplicatePolicyId,
    InconsistentTaskContract,
    PolicyCheckpointMismatch,
    InsufficientCommonPolicies,
    InsufficientCommonTasks,
    IncompletePolicyTaskMatrix,
    InvalidComparisonTier,
    InvalidBootstrapReplicateCount,
}

impl Display for PortabilityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySetupId => write!(formatter, "setup ID must not be empty"),
            Self::EmptyTeamId => write!(formatter, "team/lab ID must not be empty"),
            Self::EmptyRobotArticleId => write!(formatter, "robot article ID must not be empty"),
            Self::EmptyEmbodiment => write!(formatter, "embodiment ID must not be empty"),
            Self::EmptyPhysicsEngineId => write!(formatter, "physics-engine ID must not be empty"),
            Self::InvalidAssetDigest => write!(formatter, "asset digest must be a 64-character lowercase hex SHA-256"),
            Self::InvalidControllerDigest => write!(formatter, "controller digest must be a 64-character lowercase hex SHA-256"),
            Self::InvalidTaskContractDigest => write!(formatter, "task contract digest must be a 64-character lowercase hex SHA-256"),
            Self::InvalidPolicyId => write!(formatter, "policy ID must not be empty"),
            Self::InvalidPolicyCheckpointDigest => write!(formatter, "policy checkpoint digest must be a 64-character lowercase hex SHA-256"),
            Self::InvalidTaskId => write!(formatter, "task ID must not be empty"),
            Self::InvalidSuccessCounts => write!(formatter, "trial count must be positive and successes cannot exceed trials"),
            Self::DuplicateOutcome => write!(formatter, "a policy may have only one aggregate outcome per task and setup"),
            Self::DuplicatePolicyId => write!(formatter, "policy IDs must be unique within a setup"),
            Self::InconsistentTaskContract => write!(formatter, "source and target setups must use the same pinned task contract"),
            Self::PolicyCheckpointMismatch => write!(formatter, "the same policy ID must reference the same immutable checkpoint in both setups"),
            Self::InsufficientCommonPolicies => write!(formatter, "at least two identical policy checkpoints must be present in both setups"),
            Self::InsufficientCommonTasks => write!(formatter, "at least ten common task clusters are required"),
            Self::IncompletePolicyTaskMatrix => write!(formatter, "each common task must contain outcomes for every common policy in both setups"),
            Self::InvalidComparisonTier => write!(formatter, "setup identities do not satisfy the declared comparison tier"),
            Self::InvalidBootstrapReplicateCount => write!(formatter, "at least 1000 bootstrap replicates are required"),
        }
    }
}

impl Error for PortabilityError {}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetupIdentity {
    pub setup_id: String,
    pub team_id: String,
    pub robot_article_id: String,
    pub embodiment_id: String,
    pub physics_engine_id: String,
    pub kind: SetupKind,
    pub asset_manifest_digest: String,
    pub controller_config_digest: String,
    pub task_contract_digest: String,
}

impl SetupIdentity {
    fn validate(&self) -> Result<(), PortabilityError> {
        if self.setup_id.trim().is_empty() {
            return Err(PortabilityError::EmptySetupId);
        }
        if self.team_id.trim().is_empty() {
            return Err(PortabilityError::EmptyTeamId);
        }
        if self.robot_article_id.trim().is_empty() {
            return Err(PortabilityError::EmptyRobotArticleId);
        }
        if self.embodiment_id.trim().is_empty() {
            return Err(PortabilityError::EmptyEmbodiment);
        }
        if self.physics_engine_id.trim().is_empty() {
            return Err(PortabilityError::EmptyPhysicsEngineId);
        }
        if !is_sha256(&self.asset_manifest_digest) {
            return Err(PortabilityError::InvalidAssetDigest);
        }
        if !is_sha256(&self.controller_config_digest) {
            return Err(PortabilityError::InvalidControllerDigest);
        }
        if !is_sha256(&self.task_contract_digest) {
            return Err(PortabilityError::InvalidTaskContractDigest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PolicyIdentity {
    pub policy_id: String,
    pub checkpoint_digest: String,
}

impl PolicyIdentity {
    fn validate(&self) -> Result<(), PortabilityError> {
        if self.policy_id.trim().is_empty() {
            return Err(PortabilityError::InvalidPolicyId);
        }
        if !is_sha256(&self.checkpoint_digest) {
            return Err(PortabilityError::InvalidPolicyCheckpointDigest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyTaskOutcome {
    pub task_id: String,
    pub policy: PolicyIdentity,
    pub successes: u64,
    pub trials: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetupEvaluation {
    pub setup: SetupIdentity,
    pub outcomes: Vec<PolicyTaskOutcome>,
}

impl SetupEvaluation {
    pub fn validate(&self) -> Result<(), PortabilityError> {
        self.setup.validate()?;
        let mut outcomes = BTreeSet::new();
        let mut policy_versions = BTreeMap::<&str, &str>::new();
        for outcome in &self.outcomes {
            if outcome.task_id.trim().is_empty() {
                return Err(PortabilityError::InvalidTaskId);
            }
            outcome.policy.validate()?;
            if outcome.trials == 0 || outcome.successes > outcome.trials {
                return Err(PortabilityError::InvalidSuccessCounts);
            }
            if let Some(previous) = policy_versions.insert(
                outcome.policy.policy_id.as_str(),
                outcome.policy.checkpoint_digest.as_str(),
            ) {
                if previous != outcome.policy.checkpoint_digest {
                    return Err(PortabilityError::DuplicatePolicyId);
                }
            }
            if !outcomes.insert((
                outcome.task_id.as_str(),
                outcome.policy.policy_id.as_str(),
            )) {
                return Err(PortabilityError::DuplicateOutcome);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WilsonInterval {
    pub rate: f64,
    pub lower: f64,
    pub upper: f64,
}

pub fn wilson_90(successes: u64, trials: u64) -> Option<WilsonInterval> {
    if trials == 0 || successes > trials {
        return None;
    }
    let n = trials as f64;
    let p = successes as f64 / n;
    let z2 = WILSON_Z_90 * WILSON_Z_90;
    let denominator = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denominator;
    let half = WILSON_Z_90
        * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt()
        / denominator;
    Some(WilsonInterval {
        rate: p,
        lower: (center - half).clamp(0.0, 1.0),
        upper: (center + half).clamp(0.0, 1.0),
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PairWinner {
    First,
    Second,
    Undecided,
}

fn decide_pair(first: WilsonInterval, second: WilsonInterval) -> PairWinner {
    if first.lower > second.upper {
        PairWinner::First
    } else if second.lower > first.upper {
        PairWinner::Second
    } else {
        PairWinner::Undecided
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatchupOutcome {
    pub task_id: String,
    pub first_policy_id: String,
    pub second_policy_id: String,
    pub source_winner: PairWinner,
    pub target_winner: PairWinner,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConfidenceInterval {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootstrapConfig {
    pub replicates: usize,
    pub seed: u64,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            replicates: 10_000,
            seed: 0x4352_4F53_5353_4554,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CrossSetupReport {
    pub source_setup: SetupIdentity,
    pub target_setup: SetupIdentity,
    pub comparison_tier: ComparisonTier,
    pub source_task_count: usize,
    pub target_task_count: usize,
    pub common_task_clusters: usize,
    pub common_policy_checkpoints: usize,
    pub possible_matchups: usize,
    pub source_decided_matchups: usize,
    pub target_decided_matchups: usize,
    pub jointly_decided_matchups: usize,
    pub agreements: usize,
    pub rank_reversals: usize,
    pub source_decision_coverage: f64,
    pub target_decision_coverage: f64,
    pub joint_decision_coverage: f64,
    pub winner_agreement_rate: Option<f64>,
    pub rank_reversal_rate: Option<f64>,
    /// Task-cluster bootstrap interval; None when fewer than ten task clusters
    /// have at least one jointly decided policy-pair matchup.
    pub winner_agreement_cluster_bootstrap_90: Option<ConfidenceInterval>,
    /// Spearman correlation over equal-task macro policy success rates.
    pub policy_rank_spearman: Option<f64>,
    pub inferential_task_clusters: usize,
    pub matchups: Vec<MatchupOutcome>,
}

pub fn compare_setup_portability(
    source: &SetupEvaluation,
    target: &SetupEvaluation,
    tier: ComparisonTier,
    bootstrap: BootstrapConfig,
) -> Result<CrossSetupReport, PortabilityError> {
    source.validate()?;
    target.validate()?;
    if source.setup.task_contract_digest != target.setup.task_contract_digest {
        return Err(PortabilityError::InconsistentTaskContract);
    }
    validate_tier(&source.setup, &target.setup, tier)?;
    if bootstrap.replicates < MIN_BOOTSTRAP_REPLICATES {
        return Err(PortabilityError::InvalidBootstrapReplicateCount);
    }

    let source_policy_versions = policy_versions(source);
    let target_policy_versions = policy_versions(target);
    for (policy_id, source_digest) in &source_policy_versions {
        if let Some(target_digest) = target_policy_versions.get(policy_id) {
            if source_digest != target_digest {
                return Err(PortabilityError::PolicyCheckpointMismatch);
            }
        }
    }

    let source_policies: BTreeSet<String> =
        source_policy_versions.keys().map(|id| (*id).to_owned()).collect();
    let target_policies: BTreeSet<String> =
        target_policy_versions.keys().map(|id| (*id).to_owned()).collect();
    let common_policies: Vec<String> = source_policies
        .intersection(&target_policies)
        .cloned()
        .collect();
    if common_policies.len() < 2 {
        return Err(PortabilityError::InsufficientCommonPolicies);
    }

    let source_data = build_outcome_map(source);
    let target_data = build_outcome_map(target);
    let source_tasks: BTreeSet<String> = source_data.keys().cloned().collect();
    let target_tasks: BTreeSet<String> = target_data.keys().cloned().collect();
    let common_tasks: Vec<String> = source_tasks
        .intersection(&target_tasks)
        .cloned()
        .collect();
    if common_tasks.len() < MIN_COMMON_TASK_CLUSTERS {
        return Err(PortabilityError::InsufficientCommonTasks);
    }

    let pairs = policy_pairs(&common_policies);
    let possible_matchups = common_tasks.len() * pairs.len();
    let mut outcomes = Vec::with_capacity(possible_matchups);
    let mut source_decided = 0;
    let mut target_decided = 0;
    let mut jointly_decided = 0;
    let mut agreements = 0;
    let mut rank_reversals = 0;
    let mut task_cluster_counts = Vec::<(usize, usize)>::with_capacity(common_tasks.len());

    for task_id in &common_tasks {
        let source_task = source_data.get(task_id).expect("common source task");
        let target_task = target_data.get(task_id).expect("common target task");
        for policy_id in &common_policies {
            if !source_task.contains_key(policy_id) || !target_task.contains_key(policy_id) {
                return Err(PortabilityError::IncompletePolicyTaskMatrix);
            }
        }

        let mut task_agreements = 0;
        let mut task_joint_decided = 0;
        for (first, second) in &pairs {
            let source_first = source_task.get(first).expect("complete source matrix");
            let source_second = source_task.get(second).expect("complete source matrix");
            let target_first = target_task.get(first).expect("complete target matrix");
            let target_second = target_task.get(second).expect("complete target matrix");
            let source_winner = decide_pair(
                wilson_90(source_first.successes, source_first.trials)
                    .expect("validated source counts"),
                wilson_90(source_second.successes, source_second.trials)
                    .expect("validated source counts"),
            );
            let target_winner = decide_pair(
                wilson_90(target_first.successes, target_first.trials)
                    .expect("validated target counts"),
                wilson_90(target_second.successes, target_second.trials)
                    .expect("validated target counts"),
            );

            source_decided += usize::from(source_winner != PairWinner::Undecided);
            target_decided += usize::from(target_winner != PairWinner::Undecided);
            if source_winner != PairWinner::Undecided
                && target_winner != PairWinner::Undecided
            {
                jointly_decided += 1;
                task_joint_decided += 1;
                if source_winner == target_winner {
                    agreements += 1;
                    task_agreements += 1;
                } else {
                    rank_reversals += 1;
                }
            }

            outcomes.push(MatchupOutcome {
                task_id: task_id.clone(),
                first_policy_id: first.clone(),
                second_policy_id: second.clone(),
                source_winner,
                target_winner,
            });
        }
        task_cluster_counts.push((task_agreements, task_joint_decided));
    }

    let source_decision_coverage = source_decided as f64 / possible_matchups as f64;
    let target_decision_coverage = target_decided as f64 / possible_matchups as f64;
    let joint_decision_coverage = jointly_decided as f64 / possible_matchups as f64;
    let winner_agreement_rate =
        (jointly_decided > 0).then_some(agreements as f64 / jointly_decided as f64);
    let rank_reversal_rate =
        (jointly_decided > 0).then_some(rank_reversals as f64 / jointly_decided as f64);
    let inferential_task_clusters = task_cluster_counts
        .iter()
        .filter(|(_, jointly)| *jointly > 0)
        .count();
    let agreement_interval = if inferential_task_clusters >= MIN_COMMON_TASK_CLUSTERS {
        Some(bootstrap_agreement_interval(&task_cluster_counts, bootstrap))
    } else {
        None
    };

    let source_macro = macro_policy_rates(&source_data, &common_tasks, &common_policies);
    let target_macro = macro_policy_rates(&target_data, &common_tasks, &common_policies);
    let policy_rank_spearman = spearman_common(
        &common_policies,
        &source_macro,
        &target_macro,
    );

    Ok(CrossSetupReport {
        source_setup: source.setup.clone(),
        target_setup: target.setup.clone(),
        comparison_tier: tier,
        source_task_count: source_tasks.len(),
        target_task_count: target_tasks.len(),
        common_task_clusters: common_tasks.len(),
        common_policy_checkpoints: common_policies.len(),
        possible_matchups,
        source_decided_matchups: source_decided,
        target_decided_matchups: target_decided,
        jointly_decided_matchups: jointly_decided,
        agreements,
        rank_reversals,
        source_decision_coverage,
        target_decision_coverage,
        joint_decision_coverage,
        winner_agreement_rate,
        rank_reversal_rate,
        winner_agreement_cluster_bootstrap_90: agreement_interval,
        policy_rank_spearman,
        inferential_task_clusters,
        matchups: outcomes,
    })
}

fn validate_tier(
    source: &SetupIdentity,
    target: &SetupIdentity,
    tier: ComparisonTier,
) -> Result<(), PortabilityError> {
    let valid = match tier {
        ComparisonTier::WithinSetupRepeatability => {
            source.team_id == target.team_id
                && source.robot_article_id == target.robot_article_id
                && source.embodiment_id == target.embodiment_id
                && source.kind == target.kind
        }
        ComparisonTier::MatchedSimulatorTwin => {
            source.team_id == target.team_id
                && source.robot_article_id == target.robot_article_id
                && source.embodiment_id == target.embodiment_id
                && source.kind != target.kind
        }
        ComparisonTier::WithinTeamArticle => {
            source.team_id == target.team_id
                && source.robot_article_id != target.robot_article_id
                && source.embodiment_id == target.embodiment_id
        }
        ComparisonTier::CrossTeamOrLab => source.team_id != target.team_id,
        ComparisonTier::CrossEmbodiment => source.embodiment_id != target.embodiment_id,
        ComparisonTier::CrossEngine => {
            source.kind == SetupKind::Simulation
                && target.kind == SetupKind::Simulation
                && source.physics_engine_id != target.physics_engine_id
        }
    };
    if valid {
        Ok(())
    } else {
        Err(PortabilityError::InvalidComparisonTier)
    }
}

fn policy_versions<'a>(evaluation: &'a SetupEvaluation) -> BTreeMap<&'a str, &'a str> {
    let mut versions = BTreeMap::new();
    for outcome in &evaluation.outcomes {
        versions.insert(
            outcome.policy.policy_id.as_str(),
            outcome.policy.checkpoint_digest.as_str(),
        );
    }
    versions
}

fn build_outcome_map<'a>(
    evaluation: &'a SetupEvaluation,
) -> BTreeMap<String, BTreeMap<String, &'a PolicyTaskOutcome>> {
    let mut output = BTreeMap::new();
    for outcome in &evaluation.outcomes {
        output
            .entry(outcome.task_id.clone())
            .or_insert_with(BTreeMap::new)
            .insert(outcome.policy.policy_id.clone(), outcome);
    }
    output
}

fn policy_pairs(policies: &[String]) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for first in 0..policies.len() {
        for second in (first + 1)..policies.len() {
            pairs.push((policies[first].clone(), policies[second].clone()));
        }
    }
    pairs
}

fn macro_policy_rates(
    data: &BTreeMap<String, BTreeMap<String, &PolicyTaskOutcome>>,
    tasks: &[String],
    policies: &[String],
) -> BTreeMap<String, f64> {
    let mut rates = BTreeMap::new();
    for policy in policies {
        let sum = tasks
            .iter()
            .map(|task_id| {
                let score = data[task_id][policy];
                score.successes as f64 / score.trials as f64
            })
            .sum::<f64>();
        rates.insert(policy.clone(), sum / tasks.len() as f64);
    }
    rates
}

fn average_ranks(values: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|left, right| values[*left].total_cmp(&values[*right]));
    let mut ranks = vec![0.0; values.len()];
    let mut index = 0;
    while index < order.len() {
        let mut end = index + 1;
        while end < order.len()
            && values[order[end]].total_cmp(&values[order[index]]).is_eq()
        {
            end += 1;
        }
        let average_rank = (index + 1 + end) as f64 / 2.0;
        for position in index..end {
            ranks[order[position]] = average_rank;
        }
        index = end;
    }
    ranks
}

fn spearman_common(
    policies: &[String],
    source: &BTreeMap<String, f64>,
    target: &BTreeMap<String, f64>,
) -> Option<f64> {
    let source_values: Vec<f64> = policies.iter().map(|policy| source[policy]).collect();
    let target_values: Vec<f64> = policies.iter().map(|policy| target[policy]).collect();
    let source_ranks = average_ranks(&source_values);
    let target_ranks = average_ranks(&target_values);
    pearson(&source_ranks, &target_ranks)
}

fn pearson(first: &[f64], second: &[f64]) -> Option<f64> {
    if first.len() != second.len() || first.len() < 2 {
        return None;
    }
    let first_mean = first.iter().sum::<f64>() / first.len() as f64;
    let second_mean = second.iter().sum::<f64>() / second.len() as f64;
    let mut covariance = 0.0;
    let mut first_variance = 0.0;
    let mut second_variance = 0.0;
    for (first, second) in first.iter().zip(second) {
        let dx = first - first_mean;
        let dy = second - second_mean;
        covariance += dx * dy;
        first_variance += dx * dx;
        second_variance += dy * dy;
    }
    if first_variance <= f64::EPSILON || second_variance <= f64::EPSILON {
        None
    } else {
        Some(covariance / (first_variance * second_variance).sqrt())
    }
}

fn bootstrap_agreement_interval(
    clusters: &[(usize, usize)],
    config: BootstrapConfig,
) -> ConfidenceInterval {
    let eligible: Vec<(usize, usize)> = clusters
        .iter()
        .copied()
        .filter(|(_, jointly_decided)| *jointly_decided > 0)
        .collect();
    let mut rng = SplitMix64::new(config.seed);
    let mut ratios = Vec::with_capacity(config.replicates);
    for _ in 0..config.replicates {
        let mut agreements = 0_usize;
        let mut decisions = 0_usize;
        for _ in 0..eligible.len() {
            let index = (rng.next_u64() % eligible.len() as u64) as usize;
            agreements += eligible[index].0;
            decisions += eligible[index].1;
        }
        ratios.push(agreements as f64 / decisions as f64);
    }
    ratios.sort_by(f64::total_cmp);
    ConfidenceInterval {
        lower: percentile(&ratios, 0.05),
        upper: percentile(&ratios, 0.95),
    }
}

fn percentile(sorted: &[f64], probability: f64) -> f64 {
    let position = probability * (sorted.len() - 1) as f64;
    let low = position.floor() as usize;
    let high = position.ceil() as usize;
    let fraction = position - low as f64;
    sorted[low] * (1.0 - fraction) + sorted[high] * fraction
}

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: &str) -> String {
        byte.repeat(64)
    }

    fn setup(
        setup_id: &str,
        team: &str,
        article: &str,
        embodiment: &str,
        engine: &str,
        kind: SetupKind,
    ) -> SetupIdentity {
        SetupIdentity {
            setup_id: setup_id.to_owned(),
            team_id: team.to_owned(),
            robot_article_id: article.to_owned(),
            embodiment_id: embodiment.to_owned(),
            physics_engine_id: engine.to_owned(),
            kind,
            asset_manifest_digest: digest("a"),
            controller_config_digest: digest("b"),
            task_contract_digest: digest("c"),
        }
    }

    fn make_eval(setup: SetupIdentity, rates: [u64; 3]) -> SetupEvaluation {
        let mut outcomes = Vec::new();
        for task_index in 0..MIN_COMMON_TASK_CLUSTERS {
            for (policy_index, policy_id) in ["policy-a", "policy-b", "policy-c"].iter().enumerate() {
                outcomes.push(PolicyTaskOutcome {
                    task_id: format!("task-{task_index}"),
                    policy: PolicyIdentity {
                        policy_id: (*policy_id).to_owned(),
                        checkpoint_digest: digest(["1", "2", "3"][policy_index]),
                    },
                    successes: rates[policy_index],
                    trials: 20,
                });
            }
        }
        SetupEvaluation { setup, outcomes }
    }

    #[test]
    fn matched_twin_preserves_policy_winners_and_reports_full_coverage() {
        let source = make_eval(
            setup("sim-a", "lab-a", "robot-a", "franka", "mujoco", SetupKind::Simulation),
            [20, 10, 0],
        );
        let target = make_eval(
            setup("real-a", "lab-a", "robot-a", "franka", "real-hardware", SetupKind::Physical),
            [20, 10, 0],
        );
        let report = compare_setup_portability(
            &source,
            &target,
            ComparisonTier::MatchedSimulatorTwin,
            BootstrapConfig { replicates: 2_000, seed: 9 },
        )
        .expect("matched twin");
        assert_eq!(report.common_task_clusters, 10);
        assert_eq!(report.common_policy_checkpoints, 3);
        assert_eq!(report.possible_matchups, 30);
        assert_eq!(report.jointly_decided_matchups, 30);
        assert_eq!(report.agreements, 30);
        assert_eq!(report.rank_reversals, 0);
        assert_eq!(report.winner_agreement_rate, Some(1.0));
        assert_eq!(report.policy_rank_spearman, Some(1.0));
        assert_eq!(
            report.winner_agreement_cluster_bootstrap_90,
            Some(ConfidenceInterval { lower: 1.0, upper: 1.0 })
        );
    }

    #[test]
    fn cross_setup_rank_reversal_is_reported_not_averaged_away() {
        let source = make_eval(
            setup("sim-a", "lab-a", "robot-a", "franka", "mujoco", SetupKind::Simulation),
            [20, 10, 0],
        );
        let target = make_eval(
            setup("real-b", "lab-b", "robot-b", "ur10", "real-hardware", SetupKind::Physical),
            [0, 10, 20],
        );
        let report = compare_setup_portability(
            &source,
            &target,
            ComparisonTier::CrossTeamOrLab,
            BootstrapConfig { replicates: 2_000, seed: 9 },
        )
        .expect("cross team");
        assert_eq!(report.agreements, 0);
        assert_eq!(report.rank_reversals, 30);
        assert_eq!(report.winner_agreement_rate, Some(0.0));
        assert_eq!(report.rank_reversal_rate, Some(1.0));
        assert_eq!(report.policy_rank_spearman, Some(-1.0));
    }

    #[test]
    fn overlapping_intervals_remain_undecided_and_reduce_coverage() {
        let source = make_eval(
            setup("sim-a", "lab-a", "robot-a", "franka", "mujoco", SetupKind::Simulation),
            [10, 10, 10],
        );
        let target = make_eval(
            setup("real-a", "lab-a", "robot-a", "franka", "real-hardware", SetupKind::Physical),
            [10, 10, 10],
        );
        let report = compare_setup_portability(
            &source,
            &target,
            ComparisonTier::MatchedSimulatorTwin,
            BootstrapConfig::default(),
        )
        .expect("ambiguous");
        assert_eq!(report.jointly_decided_matchups, 0);
        assert_eq!(report.winner_agreement_rate, None);
        assert_eq!(report.winner_agreement_cluster_bootstrap_90, None);
    }

    #[test]
    fn same_policy_name_with_different_checkpoint_is_rejected() {
        let source = make_eval(
            setup("sim-a", "lab-a", "robot-a", "franka", "mujoco", SetupKind::Simulation),
            [20, 10, 0],
        );
        let mut target = make_eval(
            setup("real-a", "lab-a", "robot-a", "franka", "real-hardware", SetupKind::Physical),
            [20, 10, 0],
        );
        target.outcomes[0].policy.checkpoint_digest = digest("f");
        assert_eq!(
            compare_setup_portability(
                &source,
                &target,
                ComparisonTier::MatchedSimulatorTwin,
                BootstrapConfig::default()
            ),
            Err(PortabilityError::PolicyCheckpointMismatch)
        );
    }

    #[test]
    fn mismatched_task_contract_is_rejected() {
        let source = make_eval(
            setup("sim-a", "lab-a", "robot-a", "franka", "mujoco", SetupKind::Simulation),
            [20, 10, 0],
        );
        let mut target = make_eval(
            setup("real-a", "lab-a", "robot-a", "franka", "real-hardware", SetupKind::Physical),
            [20, 10, 0],
        );
        target.setup.task_contract_digest = digest("d");
        assert_eq!(
            compare_setup_portability(
                &source,
                &target,
                ComparisonTier::MatchedSimulatorTwin,
                BootstrapConfig::default()
            ),
            Err(PortabilityError::InconsistentTaskContract)
        );
    }

    #[test]
    fn invalid_tier_label_is_rejected() {
        let source = make_eval(
            setup("sim-a", "lab-a", "robot-a", "franka", "mujoco", SetupKind::Simulation),
            [20, 10, 0],
        );
        let target = make_eval(
            setup("real-a", "lab-a", "robot-a", "franka", "real-hardware", SetupKind::Physical),
            [20, 10, 0],
        );
        assert_eq!(
            compare_setup_portability(
                &source,
                &target,
                ComparisonTier::CrossTeamOrLab,
                BootstrapConfig::default()
            ),
            Err(PortabilityError::InvalidComparisonTier)
        );
    }

    #[test]
    fn duplicate_outcomes_and_invalid_counts_are_rejected() {
        let mut evaluation = make_eval(
            setup("sim-a", "lab-a", "robot-a", "franka", "mujoco", SetupKind::Simulation),
            [20, 10, 0],
        );
        evaluation.outcomes.push(evaluation.outcomes[0].clone());
        assert_eq!(evaluation.validate(), Err(PortabilityError::DuplicateOutcome));

        let mut invalid = make_eval(
            setup("sim-b", "lab-a", "robot-a", "franka", "mujoco", SetupKind::Simulation),
            [20, 10, 0],
        );
        invalid.outcomes[0].successes = 21;
        assert_eq!(invalid.validate(), Err(PortabilityError::InvalidSuccessCounts));
    }

    #[test]
    fn bootstrap_is_deterministic_and_clustered_by_task() {
        let source = make_eval(
            setup("sim-a", "lab-a", "robot-a", "franka", "mujoco", SetupKind::Simulation),
            [20, 10, 0],
        );
        let target = make_eval(
            setup("real-a", "lab-a", "robot-a", "franka", "real-hardware", SetupKind::Physical),
            [20, 10, 0],
        );
        let config = BootstrapConfig { replicates: 2_000, seed: 77 };
        let first = compare_setup_portability(&source, &target, ComparisonTier::MatchedSimulatorTwin, config).unwrap();
        let second = compare_setup_portability(&source, &target, ComparisonTier::MatchedSimulatorTwin, config).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn task_count_and_bootstrap_floor_are_enforced() {
        let mut source = make_eval(
            setup("sim-a", "lab-a", "robot-a", "franka", "mujoco", SetupKind::Simulation),
            [20, 10, 0],
        );
        let mut target = make_eval(
            setup("real-a", "lab-a", "robot-a", "franka", "real-hardware", SetupKind::Physical),
            [20, 10, 0],
        );
        source.outcomes.retain(|outcome| outcome.task_id != "task-9");
        target.outcomes.retain(|outcome| outcome.task_id != "task-9");
        assert_eq!(
            compare_setup_portability(
                &source,
                &target,
                ComparisonTier::MatchedSimulatorTwin,
                BootstrapConfig::default()
            ),
            Err(PortabilityError::InsufficientCommonTasks)
        );
    }
}
