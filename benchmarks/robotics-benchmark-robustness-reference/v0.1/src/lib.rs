// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Paired, task-clustered robotics benchmark scorecard.
//!
//! The library computes transparent descriptive rates and a deterministic
//! task-cluster bootstrap interval for paired candidate-vs-baseline deltas.
//! It does not execute a benchmark, assess causal validity, or establish
//! generalization beyond the declared task and shift groups.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{Display, Formatter};

const MIN_BOOTSTRAP_REPLICATES: usize = 1_000;
const MIN_BOOTSTRAP_TASK_CLUSTERS: usize = 10;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScorecardError {
    EmptyBenchmarkId,
    InvalidBenchmarkCommit,
    InvalidProtocolDigest,
    EmptyEmbodiment,
    EmptyEvaluationSplit,
    InsufficientTaskClusters,
    EmptyTaskSet,
    EmptyTaskId,
    DuplicateTaskId,
    EmptyTaskCategory,
    EmptyShiftFamily,
    ZeroHorizon,
    EmptyTaskCluster,
    EmptyEpisodeId,
    DuplicateEpisodeId,
    NoEpisodes,
    InvalidBootstrapReplicateCount,
}

impl Display for ScorecardError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyBenchmarkId => write!(formatter, "benchmark ID must not be empty"),
            Self::InvalidBenchmarkCommit => write!(formatter, "benchmark commit must be a 40-character lowercase hex SHA"),
            Self::InvalidProtocolDigest => write!(formatter, "protocol digest must be a 64-character lowercase hex SHA-256"),
            Self::EmptyEmbodiment => write!(formatter, "embodiment must not be empty"),
            Self::EmptyEvaluationSplit => write!(formatter, "evaluation split must not be empty"),
            Self::InsufficientTaskClusters => write!(formatter, "at least ten independent task clusters are required for inferential cluster bootstrap"),
            Self::EmptyTaskSet => write!(formatter, "at least one task cluster is required"),
            Self::EmptyTaskId => write!(formatter, "task ID must not be empty"),
            Self::DuplicateTaskId => write!(formatter, "task cluster IDs must be unique"),
            Self::EmptyTaskCategory => write!(formatter, "task category must not be empty"),
            Self::EmptyShiftFamily => write!(formatter, "shift family must not be empty; use in_distribution explicitly"),
            Self::ZeroHorizon => write!(formatter, "task horizon must be positive"),
            Self::EmptyTaskCluster => write!(formatter, "each task cluster must include at least one matched episode"),
            Self::EmptyEpisodeId => write!(formatter, "episode ID must not be empty"),
            Self::DuplicateEpisodeId => write!(formatter, "episode IDs must be unique within a task cluster"),
            Self::NoEpisodes => write!(formatter, "no matched episodes supplied"),
            Self::InvalidBootstrapReplicateCount => write!(formatter, "at least 100 bootstrap replicates are required"),
        }
    }
}

impl Error for ScorecardError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpisodePair {
    pub episode_id: String,
    pub baseline_success: bool,
    pub candidate_success: bool,
}

impl EpisodePair {
    pub fn new(
        episode_id: impl Into<String>,
        baseline_success: bool,
        candidate_success: bool,
    ) -> Self {
        Self {
            episode_id: episode_id.into(),
            baseline_success,
            candidate_success,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskCluster {
    pub task_id: String,
    pub category: String,
    pub horizon: u32,
    pub shift_family: String,
    pub episodes: Vec<EpisodePair>,
}

impl TaskCluster {
    pub fn new(
        task_id: impl Into<String>,
        category: impl Into<String>,
        horizon: u32,
        shift_family: impl Into<String>,
        episodes: Vec<EpisodePair>,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            category: category.into(),
            horizon,
            shift_family: shift_family.into(),
            episodes,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BenchmarkIdentity {
    pub benchmark_id: String,
    /// Exact 40-character benchmark code/version commit, e.g. LADDER commit.
    pub benchmark_commit: String,
    /// Digest of the task/scene/episode manifest used by both methods.
    pub protocol_digest_hex: String,
    pub embodiment: String,
    pub evaluation_split: String,
    pub tuned_on_benchmark: bool,
}

impl BenchmarkIdentity {
    pub fn new(
        benchmark_id: impl Into<String>,
        benchmark_commit: impl Into<String>,
        protocol_digest_hex: impl Into<String>,
        embodiment: impl Into<String>,
        evaluation_split: impl Into<String>,
        tuned_on_benchmark: bool,
    ) -> Self {
        Self {
            benchmark_id: benchmark_id.into(),
            benchmark_commit: benchmark_commit.into(),
            protocol_digest_hex: protocol_digest_hex.into(),
            embodiment: embodiment.into(),
            evaluation_split: evaluation_split.into(),
            tuned_on_benchmark,
        }
    }

    fn validate(&self) -> Result<(), ScorecardError> {
        if self.benchmark_id.trim().is_empty() {
            return Err(ScorecardError::EmptyBenchmarkId);
        }
        if self.benchmark_commit.len() != 40
            || !self
                .benchmark_commit
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(ScorecardError::InvalidBenchmarkCommit);
        }
        if self.protocol_digest_hex.len() != 64
            || !self
                .protocol_digest_hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(ScorecardError::InvalidProtocolDigest);
        }
        if self.embodiment.trim().is_empty() {
            return Err(ScorecardError::EmptyEmbodiment);
        }
        if self.evaluation_split.trim().is_empty() {
            return Err(ScorecardError::EmptyEvaluationSplit);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RateEstimate {
    pub successes: u64,
    pub trials: u64,
    pub rate: f64,
    pub wilson_95_lower: f64,
    pub wilson_95_upper: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConfidenceInterval {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GroupComparison {
    pub group: String,
    pub baseline: RateEstimate,
    pub candidate: RateEstimate,
    /// Candidate minus baseline, on the interval [−1,+1].
    pub paired_delta: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RobustnessScorecard {
    pub identity: BenchmarkIdentity,
    pub baseline: RateEstimate,
    pub candidate: RateEstimate,
    /// Pooled episode-level success delta (descriptive).
    pub pooled_success_delta: f64,
    /// Paired cluster-bootstrap interval, resampling whole task clusters.
    pub paired_task_cluster_delta_95: ConfidenceInterval,
    /// Equal-weight task macro average; useful when clusters have unequal sizes.
    pub macro_task_success_delta: f64,
    pub by_horizon: Vec<GroupComparison>,
    pub by_category: Vec<GroupComparison>,
    pub by_shift_family: Vec<GroupComparison>,
    /// True only when the 95% paired cluster-bootstrap interval lies above zero.
    pub positive_improvement_supported: bool,
    pub task_cluster_count: usize,
    pub episode_pair_count: usize,
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
            seed: 0x4C55_4D49_4E4F_5553,
        }
    }
}

pub fn wilson_interval(successes: u64, trials: u64, z: f64) -> Option<RateEstimate> {
    if trials == 0 || !z.is_finite() || z <= 0.0 || successes > trials {
        return None;
    }
    let n = trials as f64;
    let p = successes as f64 / n;
    let z2 = z * z;
    let denominator = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denominator;
    let half_width =
        z * ((p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt()) / denominator;
    Some(RateEstimate {
        successes,
        trials,
        rate: p,
        wilson_95_lower: (center - half_width).clamp(0.0, 1.0),
        wilson_95_upper: (center + half_width).clamp(0.0, 1.0),
    })
}

pub fn evaluate_paired_comparison(
    identity: BenchmarkIdentity,
    tasks: &[TaskCluster],
    bootstrap: BootstrapConfig,
) -> Result<RobustnessScorecard, ScorecardError> {
    identity.validate()?;
    if tasks.is_empty() {
        return Err(ScorecardError::EmptyTaskSet);
    }
    if bootstrap.replicates < MIN_BOOTSTRAP_REPLICATES {
        return Err(ScorecardError::InvalidBootstrapReplicateCount);
    }

    let mut seen_tasks = BTreeSet::new();
    let mut total_pairs = 0_u64;
    let mut baseline_successes = 0_u64;
    let mut candidate_successes = 0_u64;
    let mut macro_task_delta_sum = 0.0;

    let mut horizon_groups: BTreeMap<u32, Vec<&TaskCluster>> = BTreeMap::new();
    let mut category_groups: BTreeMap<String, Vec<&TaskCluster>> = BTreeMap::new();
    let mut shift_groups: BTreeMap<String, Vec<&TaskCluster>> = BTreeMap::new();

    for task in tasks {
        if task.task_id.trim().is_empty() {
            return Err(ScorecardError::EmptyTaskId);
        }
        if !seen_tasks.insert(task.task_id.as_str()) {
            return Err(ScorecardError::DuplicateTaskId);
        }
        if task.category.trim().is_empty() {
            return Err(ScorecardError::EmptyTaskCategory);
        }
        if task.shift_family.trim().is_empty() {
            return Err(ScorecardError::EmptyShiftFamily);
        }
        if task.horizon == 0 {
            return Err(ScorecardError::ZeroHorizon);
        }
        if task.episodes.is_empty() {
            return Err(ScorecardError::EmptyTaskCluster);
        }

        let mut seen_episodes = BTreeSet::new();
        let mut task_baseline = 0_u64;
        let mut task_candidate = 0_u64;
        for episode in &task.episodes {
            if episode.episode_id.trim().is_empty() {
                return Err(ScorecardError::EmptyEpisodeId);
            }
            if !seen_episodes.insert(episode.episode_id.as_str()) {
                return Err(ScorecardError::DuplicateEpisodeId);
            }
            total_pairs += 1;
            task_baseline += u64::from(episode.baseline_success);
            task_candidate += u64::from(episode.candidate_success);
        }
        baseline_successes += task_baseline;
        candidate_successes += task_candidate;
        let task_n = task.episodes.len() as f64;
        macro_task_delta_sum += task_candidate as f64 / task_n - task_baseline as f64 / task_n;
        horizon_groups.entry(task.horizon).or_default().push(task);
        category_groups
            .entry(task.category.clone())
            .or_default()
            .push(task);
        shift_groups
            .entry(task.shift_family.clone())
            .or_default()
            .push(task);
    }

    if tasks.len() < MIN_BOOTSTRAP_TASK_CLUSTERS {
        return Err(ScorecardError::InsufficientTaskClusters);
    }
    if total_pairs == 0 {
        return Err(ScorecardError::NoEpisodes);
    }

    let baseline =
        wilson_interval(baseline_successes, total_pairs, 1.959_963_984_540_054)
            .ok_or(ScorecardError::NoEpisodes)?;
    let candidate =
        wilson_interval(candidate_successes, total_pairs, 1.959_963_984_540_054)
            .ok_or(ScorecardError::NoEpisodes)?;

    let paired_task_cluster_delta_95 =
        bootstrap_task_cluster_delta(tasks, bootstrap);
    let by_horizon = compare_groups(
        horizon_groups
            .into_iter()
            .map(|(horizon, group_tasks)| (horizon.to_string(), group_tasks))
            .collect(),
    );
    let by_category = compare_groups(category_groups.into_iter().collect());
    let by_shift_family = compare_groups(shift_groups.into_iter().collect());

    Ok(RobustnessScorecard {
        identity,
        baseline,
        candidate,
        pooled_success_delta: candidate.rate - baseline.rate,
        paired_task_cluster_delta_95,
        macro_task_success_delta: macro_task_delta_sum / tasks.len() as f64,
        positive_improvement_supported: paired_task_cluster_delta_95.lower > 0.0,
        by_horizon,
        by_category,
        by_shift_family,
        task_cluster_count: tasks.len(),
        episode_pair_count: total_pairs as usize,
    })
}

fn compare_groups(groups: Vec<(String, Vec<&TaskCluster>)>) -> Vec<GroupComparison> {
    groups
        .into_iter()
        .map(|(group, tasks)| {
            let mut baseline_successes = 0_u64;
            let mut candidate_successes = 0_u64;
            let mut trials = 0_u64;
            for task in tasks {
                for episode in &task.episodes {
                    baseline_successes += u64::from(episode.baseline_success);
                    candidate_successes += u64::from(episode.candidate_success);
                    trials += 1;
                }
            }
            let baseline = wilson_interval(
                baseline_successes,
                trials,
                1.959_963_984_540_054,
            )
            .expect("validated non-empty task clusters");
            let candidate = wilson_interval(
                candidate_successes,
                trials,
                1.959_963_984_540_054,
            )
            .expect("validated non-empty task clusters");
            GroupComparison {
                group,
                baseline,
                candidate,
                paired_delta: candidate.rate - baseline.rate,
            }
        })
        .collect()
}

fn bootstrap_task_cluster_delta(
    tasks: &[TaskCluster],
    config: BootstrapConfig,
) -> ConfidenceInterval {
    let mut random = SplitMix64::new(config.seed);
    let mut deltas = Vec::with_capacity(config.replicates);
    for _ in 0..config.replicates {
        let mut baseline_successes = 0_u64;
        let mut candidate_successes = 0_u64;
        let mut trials = 0_u64;
        for _ in 0..tasks.len() {
            let selected = (random.next_u64() % tasks.len() as u64) as usize;
            for episode in &tasks[selected].episodes {
                baseline_successes += u64::from(episode.baseline_success);
                candidate_successes += u64::from(episode.candidate_success);
                trials += 1;
            }
        }
        let baseline_rate = baseline_successes as f64 / trials as f64;
        let candidate_rate = candidate_successes as f64 / trials as f64;
        deltas.push(candidate_rate - baseline_rate);
    }

    deltas.sort_by(f64::total_cmp);
    ConfidenceInterval {
        lower: percentile(&deltas, 0.025),
        upper: percentile(&deltas, 0.975),
    }
}

fn percentile(sorted_values: &[f64], probability: f64) -> f64 {
    debug_assert!(!sorted_values.is_empty());
    let index = probability.clamp(0.0, 1.0) * (sorted_values.len() - 1) as f64;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;
    let fraction = index - lower as f64;
    sorted_values[lower] * (1.0 - fraction) + sorted_values[upper] * fraction
}

/// Small, fixed PRNG for deterministic bootstrap resampling.
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

    fn identity() -> BenchmarkIdentity {
        BenchmarkIdentity::new(
            "LADDER",
            "e4e204073cc64b0dc49f9669c2951e6bc91fe0af",
            "ab".repeat(32),
            "widowx",
            "full",
            false,
        )
    }

    fn task(
        id: &str,
        horizon: u32,
        shift: &str,
        baseline: &[bool],
        candidate: &[bool],
    ) -> TaskCluster {
        assert_eq!(baseline.len(), candidate.len());
        TaskCluster::new(
            id,
            "pick_place",
            horizon,
            shift,
            baseline
                .iter()
                .zip(candidate)
                .enumerate()
                .map(|(index, (base, candidate))| {
                    EpisodePair::new(format!("{id}-{index}"), *base, *candidate)
                })
                .collect(),
        )
    }

    #[test]
    fn wilson_interval_contains_observed_rate() {
        let interval = wilson_interval(75, 100, 1.959_963_984_540_054).expect("valid interval");
        assert!((interval.rate - 0.75).abs() < 1.0e-12);
        assert!(interval.wilson_95_lower < interval.rate);
        assert!(interval.wilson_95_upper > interval.rate);
    }

    #[test]
    fn zero_trials_or_invalid_counts_are_rejected() {
        assert_eq!(wilson_interval(0, 0, 1.96), None);
        assert_eq!(wilson_interval(2, 1, 1.96), None);
        assert_eq!(wilson_interval(1, 2, f64::NAN), None);
    }

    #[test]
    fn matched_episode_scorecard_reports_pooled_and_group_metrics() {
        let mut tasks = vec![
            task("task-a", 1, "in_distribution", &[false, true], &[true, true]),
            task("task-b", 3, "held_out_layout", &[false, false], &[true, false]),
        ];
        for index in 0..8 {
            tasks.push(task(
                &format!("task-extra-{index}"),
                if index % 2 == 0 { 1 } else { 3 },
                if index % 2 == 0 { "in_distribution" } else { "held_out_layout" },
                &[false],
                &[false],
            ));
        }
        let result = evaluate_paired_comparison(
            identity(),
            &tasks,
            BootstrapConfig { replicates: 2_000, seed: 7 },
        ).expect("scorecard");
        assert_eq!(result.baseline.successes, 1);
        assert_eq!(result.candidate.successes, 3);
        assert_eq!(result.episode_pair_count, 12);
        assert!((result.pooled_success_delta - (2.0 / 12.0)).abs() < 1.0e-12);
        assert_eq!(result.by_horizon.len(), 2);
        assert_eq!(result.by_category.len(), 1);
        assert_eq!(result.by_shift_family.len(), 2);
    }

    #[test]
    fn cluster_bootstrap_is_deterministic_and_paired() {
        let mut tasks = vec![
            task("task-a", 2, "in_distribution", &[false, false, false], &[true, true, true]),
            task("task-b", 4, "held_out_object", &[false, false, false], &[true, true, true]),
            task("task-c", 6, "held_out_layout", &[false, false, false], &[true, true, true]),
        ];
        for index in 0..7 {
            tasks.push(task(
                &format!("task-extra-{index}"),
                2 + index as u32,
                "held_out_layout",
                &[false, false],
                &[true, true],
            ));
        }
        let config = BootstrapConfig { replicates: 5_000, seed: 12345 };
        let first = evaluate_paired_comparison(identity(), &tasks, config).expect("first");
        let second = evaluate_paired_comparison(identity(), &tasks, config).expect("second");
        assert_eq!(first, second);
        assert_eq!(first.paired_task_cluster_delta_95, ConfidenceInterval { lower: 1.0, upper: 1.0 });
        assert!(first.positive_improvement_supported);
    }

    #[test]
    fn no_improvement_does_not_claim_positive_significance() {
        let mut tasks = vec![
            task("task-a", 1, "in_distribution", &[true, false], &[true, false]),
            task("task-b", 2, "held_out_layout", &[false, true], &[false, true]),
        ];
        for index in 0..8 {
            tasks.push(task(
                &format!("task-extra-{index}"),
                1 + (index % 2) as u32,
                if index % 2 == 0 { "in_distribution" } else { "held_out_layout" },
                &[index % 2 == 0],
                &[index % 2 == 0],
            ));
        }
        let result = evaluate_paired_comparison(
            identity(),
            &tasks,
            BootstrapConfig { replicates: 2_000, seed: 17 },
        ).expect("scorecard");
        assert_eq!(result.pooled_success_delta, 0.0);
        assert_eq!(result.paired_task_cluster_delta_95, ConfidenceInterval { lower: 0.0, upper: 0.0 });
        assert!(!result.positive_improvement_supported);
    }

    #[test]
    fn rejects_duplicate_task_or_episode_ids() {
        let duplicate_task = vec![
            task("task-a", 1, "in_distribution", &[false], &[true]),
            task("task-a", 1, "in_distribution", &[false], &[true]),
        ];
        assert_eq!(
            evaluate_paired_comparison(identity(), &duplicate_task, BootstrapConfig::default()),
            Err(ScorecardError::DuplicateTaskId)
        );

        let duplicate_episode = TaskCluster::new(
            "task-a",
            "pick_place",
            1,
            "in_distribution",
            vec![
                EpisodePair::new("episode-1", false, true),
                EpisodePair::new("episode-1", false, true),
            ],
        );
        assert_eq!(
            evaluate_paired_comparison(identity(), &[duplicate_episode], BootstrapConfig::default()),
            Err(ScorecardError::DuplicateEpisodeId)
        );
    }

    #[test]
    fn rejects_invalid_or_incomplete_benchmark_identity() {
        let invalid = BenchmarkIdentity::new(
            "LADDER",
            "not-a-commit",
            "ab".repeat(32),
            "widowx",
            "full",
            false,
        );
        assert_eq!(
            evaluate_paired_comparison(
                invalid,
                &[task("task-a", 1, "in_distribution", &[false], &[true])],
                BootstrapConfig::default()
            ),
            Err(ScorecardError::InvalidBenchmarkCommit)
        );
    }

    #[test]
    fn rejects_single_cluster_as_inferential_comparison() {
        assert_eq!(
            evaluate_paired_comparison(
                identity(),
                &[task("task-a", 1, "in_distribution", &[false], &[true])],
                BootstrapConfig::default()
            ),
            Err(ScorecardError::InsufficientTaskClusters)
        );
    }

    #[test]
    fn rejects_invalid_protocol_digest_and_empty_run_identity() {
        let invalid_digest = BenchmarkIdentity::new(
            "LADDER",
            "e4e204073cc64b0dc49f9669c2951e6bc91fe0af",
            "not-a-digest",
            "widowx",
            "full",
            false,
        );
        assert_eq!(
            evaluate_paired_comparison(
                invalid_digest,
                &[task("task-a", 1, "in_distribution", &[false], &[true]),
                  task("task-b", 2, "held_out_layout", &[false], &[true])],
                BootstrapConfig::default()
            ),
            Err(ScorecardError::InvalidProtocolDigest)
        );
    }

    #[test]
    fn bootstrap_rejects_too_few_replicates() {
        assert_eq!(
            evaluate_paired_comparison(
                identity(),
                &[task("task-a", 1, "in_distribution", &[false], &[true])],
                BootstrapConfig { replicates: 999, seed: 1 }
            ),
            Err(ScorecardError::InvalidBootstrapReplicateCount)
        );
    }
}
