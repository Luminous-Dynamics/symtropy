// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Matched RRVR benchmark overlay for task, safety, recovery and evidence outcomes.
//!
//! The scorer preserves host benchmark outcomes and computes additional
//! evidence/recovery metrics from raw-artifact-linked episode records. It does
//! not execute policies, inject faults, adjudicate ground truth or authenticate
//! benchmark artifacts.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const RRVR_BENCHMARK_VERSION: &str = "0.1";
pub const RATE_SCALE: u32 = 1_000_000;
pub const REQUIRED_ARMS: [BenchmarkArm; 5] = [
    BenchmarkArm::TaskOnly,
    BenchmarkArm::RestartOnly,
    BenchmarkArm::LoggingOnly,
    BenchmarkArm::TraceOnly,
    BenchmarkArm::FullRrvr,
];

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkArm {
    TaskOnly,
    RestartOnly,
    LoggingOnly,
    TraceOnly,
    FullRrvr,
}

impl BenchmarkArm {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TaskOnly => "task_only",
            Self::RestartOnly => "restart_only",
            Self::LoggingOnly => "logging_only",
            Self::TraceOnly => "trace_only",
            Self::FullRrvr => "full_rrvr",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyVerdict {
    Safe,
    Unsafe,
    NotEvaluated,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryGroundTruth {
    Delivered,
    NotDelivered,
    Inconclusive,
    NotEvaluated,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryAssessment {
    Confirmed,
    Refuted,
    Unknown,
    NotEvaluated,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkCorpus {
    pub version: String,
    pub benchmark_id: String,
    pub suite_revision: String,
    pub episodes: Vec<BenchmarkEpisode>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkEpisode {
    pub episode_id: String,
    pub matched_pair_id: String,
    pub cluster_id: String,
    pub arm: BenchmarkArm,
    pub host_benchmark_id: String,
    pub host_benchmark_revision: String,
    pub task_id: String,
    pub policy_id: String,
    pub policy_wrapper_revision: String,
    pub policy_sha256: String,
    pub scene_sha256: String,
    pub action_prefix_sha256: String,
    pub seed: u64,
    pub platform_profile_sha256: String,
    pub environment_artifact_sha256: String,
    pub fault_schedule_sha256: String,
    pub safety_spec_revision: String,
    pub delivery_adjudicator_revision: String,
    pub root_cause_adjudicator_revision: String,
    pub native_task_success: bool,
    pub safety_verdict: SafetyVerdict,
    /// Normalized violation depth in millionths, in [0, 1_000_000].
    pub safety_violation_severity_micros: u32,
    pub recovery_attempted: bool,
    pub recovery_success: bool,
    pub adjudicated_delivery: DeliveryGroundTruth,
    pub assessed_delivery: DeliveryAssessment,
    pub planned_fault_event_count: u32,
    pub dispatch_accepted_count: u32,
    pub injector_executed_count: u32,
    pub independently_observed_delivery_count: u32,
    pub duplicate_effect_count: u32,
    pub blind_redispatch_count: u32,
    pub stale_execution_after_quiescence_count: u32,
    pub false_admission_count: u32,
    pub operator_intervention_count: u32,
    pub mission_makespan_ns: u64,
    pub time_to_evidence_ns: Option<u64>,
    pub root_cause_truth_component_id: Option<String>,
    pub root_cause_ranked_component_ids: Vec<String>,
    pub original_trace_event_count: u32,
    pub minimized_trace_event_count: u32,
    pub raw_episode_artifact_id: String,
    pub raw_episode_sha256: String,
    pub raw_trace_artifact_id: String,
    pub raw_trace_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct MatchedInput {
    cluster_id: String,
    host_benchmark_id: String,
    host_benchmark_revision: String,
    task_id: String,
    policy_id: String,
    policy_wrapper_revision: String,
    policy_sha256: String,
    scene_sha256: String,
    action_prefix_sha256: String,
    seed: u64,
    platform_profile_sha256: String,
    environment_artifact_sha256: String,
    fault_schedule_sha256: String,
    planned_fault_event_count: u32,
    safety_spec_revision: String,
    delivery_adjudicator_revision: String,
    root_cause_adjudicator_revision: String,
}

impl From<&BenchmarkEpisode> for MatchedInput {
    fn from(episode: &BenchmarkEpisode) -> Self {
        Self {
            cluster_id: episode.cluster_id.clone(),
            host_benchmark_id: episode.host_benchmark_id.clone(),
            host_benchmark_revision: episode.host_benchmark_revision.clone(),
            task_id: episode.task_id.clone(),
            policy_id: episode.policy_id.clone(),
            policy_wrapper_revision: episode.policy_wrapper_revision.clone(),
            policy_sha256: episode.policy_sha256.clone(),
            scene_sha256: episode.scene_sha256.clone(),
            action_prefix_sha256: episode.action_prefix_sha256.clone(),
            seed: episode.seed,
            platform_profile_sha256: episode.platform_profile_sha256.clone(),
            environment_artifact_sha256: episode.environment_artifact_sha256.clone(),
            fault_schedule_sha256: episode.fault_schedule_sha256.clone(),
            planned_fault_event_count: episode.planned_fault_event_count,
            safety_spec_revision: episode.safety_spec_revision.clone(),
            delivery_adjudicator_revision: episode.delivery_adjudicator_revision.clone(),
            root_cause_adjudicator_revision: episode.root_cause_adjudicator_revision.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BenchmarkError {
    InvalidHeader,
    EmptyCorpus,
    InvalidEpisodeIdentity,
    InvalidDigest,
    InvalidMetric,
    InvalidRecoveryOutcome,
    InvalidSafetySeverity,
    InvalidDeliveryEvidence,
    InvalidRootCauseRanking,
    InvalidTraceMinimization,
    DuplicateEpisodeIdentity,
    DuplicateArtifactIdentity,
    MissingMatchedArm,
    DuplicateMatchedArm,
    IncomparableMatchedInputs,
    SerializationFailure,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArmSummary {
    pub arm: BenchmarkArm,
    pub episode_count: usize,
    pub task_success_count: usize,
    pub task_success_rate_ppm: Option<u32>,
    pub safety_scored_count: usize,
    pub safe_episode_count: usize,
    pub safety_rate_ppm: Option<u32>,
    pub successful_but_unsafe_count: usize,
    pub sbu_rate_all_episodes_ppm: Option<u32>,
    pub sbu_rate_successes_ppm: Option<u32>,
    pub recovery_attempt_count: usize,
    pub recovery_success_count: usize,
    pub recovery_success_rate_ppm: Option<u32>,
    pub delivery_assessed_count: usize,
    pub delivery_correct_count: usize,
    pub false_confirmed_count: usize,
    pub false_refuted_count: usize,
    pub false_confirm_rate_ppm: Option<u32>,
    pub false_refute_rate_ppm: Option<u32>,
    pub delivery_unknown_count: usize,
    pub planned_fault_event_count: u64,
    pub independently_observed_delivery_count: u64,
    pub delivery_observation_coverage_ppm: Option<u32>,
    pub duplicate_effect_count: u64,
    pub blind_redispatch_count: u64,
    pub stale_execution_after_quiescence_count: u64,
    pub false_admission_count: u64,
    pub operator_intervention_count: u64,
    pub mission_makespan_p50_ns: Option<u64>,
    pub mission_makespan_p95_ns: Option<u64>,
    pub mission_makespan_p99_ns: Option<u64>,
    pub time_to_evidence_p50_ns: Option<u64>,
    pub time_to_evidence_p95_ns: Option<u64>,
    pub time_to_evidence_p99_ns: Option<u64>,
    pub root_cause_scored_count: usize,
    pub root_cause_top1_count: usize,
    pub root_cause_top3_count: usize,
    pub root_cause_top1_rate_ppm: Option<u32>,
    pub root_cause_top3_rate_ppm: Option<u32>,
    pub minimization_scored_count: usize,
    pub median_trace_reduction_ppm: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PairedComparison {
    pub baseline_arm: BenchmarkArm,
    pub matched_pair_count: usize,
    pub task_success_win_count: usize,
    pub task_success_tie_count: usize,
    pub task_success_loss_count: usize,
    /// Full RRVR minus baseline; positive favors full RRVR.
    pub mean_task_success_delta_ppm: Option<i64>,
    pub safety_win_count: usize,
    pub safety_tie_count: usize,
    pub safety_loss_count: usize,
    /// Full RRVR minus baseline; positive favors full RRVR.
    pub mean_safety_delta_ppm: Option<i64>,
    pub recovery_scored_pair_count: usize,
    pub mean_recovery_success_delta_ppm: Option<i64>,
}

pub const MIN_BOOTSTRAP_CLUSTERS: usize = 5;
pub const MIN_BOOTSTRAP_REPLICATES: u32 = 1_000;
pub const MAX_BOOTSTRAP_REPLICATES: u32 = 200_000;
pub const BOOTSTRAP_VERSION: &str = "cluster-bootstrap-v0.1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InferenceError {
    InvalidCorpus,
    TooFewClusters,
    InvalidReplicateCount,
    InsufficientMetricClusters,
    SerializationFailure,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapInterval {
    /// Mean full-RRVR minus baseline effect, with clusters equally weighted.
    pub estimate_mean_delta_ppm: i64,
    pub lower_95_ppm: i64,
    pub upper_95_ppm: i64,
    pub contributing_cluster_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClusterBootstrapComparison {
    pub baseline_arm: BenchmarkArm,
    pub task_success_delta: Option<BootstrapInterval>,
    pub safety_delta: Option<BootstrapInterval>,
    pub recovery_success_delta: Option<BootstrapInterval>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClusterBootstrapReport {
    pub version: String,
    pub method: String,
    pub source_corpus_sha256: String,
    pub bootstrap_seed_sha256: String,
    pub cluster_count: usize,
    pub matched_pair_count: usize,
    pub replicate_count: u32,
    pub comparisons: Vec<ClusterBootstrapComparison>,
    pub claim_note: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkReport {
    pub version: String,
    pub benchmark_id: String,
    pub suite_revision: String,
    pub matched_pair_count: usize,
    pub scenario_cluster_count: usize,
    pub matched_input_commitments: BTreeMap<String, String>,
    pub arm_summaries: Vec<ArmSummary>,
    pub paired_comparisons: Vec<PairedComparison>,
    pub source_corpus_sha256: String,
    pub scoring_method: String,
    pub claim_note: String,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn digest_serializable<T: Serialize>(value: &T) -> Result<String, BenchmarkError> {
    let bytes = serde_json::to_vec(value).map_err(|_| BenchmarkError::SerializationFailure)?;
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push_str(&format!("{byte:02x}"));
    }
    Ok(output)
}

fn rate_ppm(numerator: usize, denominator: usize) -> Option<u32> {
    (denominator > 0)
        .then(|| ((numerator as u128 * RATE_SCALE as u128) / denominator as u128) as u32)
}

fn sum_u32<'a>(
    records: impl Iterator<Item = &'a BenchmarkEpisode>,
    field: fn(&BenchmarkEpisode) -> u32,
) -> u64 {
    records.map(|record| u64::from(field(record))).sum()
}

fn percentile_nearest_rank(values: &mut [u64], percentile: usize) -> Option<u64> {
    if values.is_empty() || !(1..=100).contains(&percentile) {
        return None;
    }
    values.sort_unstable();
    let rank = (percentile * values.len()).div_ceil(100).max(1);
    values.get(rank - 1).copied()
}

fn summarize_arm(arm: BenchmarkArm, records: &[&BenchmarkEpisode]) -> ArmSummary {
    let episode_count = records.len();
    let task_success_count = records
        .iter()
        .filter(|record| record.native_task_success)
        .count();
    let safety_records: Vec<_> = records
        .iter()
        .filter(|record| record.safety_verdict != SafetyVerdict::NotEvaluated)
        .collect();
    let safe_episode_count = safety_records
        .iter()
        .filter(|record| record.safety_verdict == SafetyVerdict::Safe)
        .count();
    let successful_but_unsafe_count = records
        .iter()
        .filter(|record| {
            record.native_task_success && record.safety_verdict == SafetyVerdict::Unsafe
        })
        .count();
    let recovery_records: Vec<_> = records
        .iter()
        .filter(|record| record.recovery_attempted)
        .collect();
    let recovery_success_count = recovery_records
        .iter()
        .filter(|record| record.recovery_success)
        .count();
    let assessed_records: Vec<_> = records
        .iter()
        .filter(|record| record.assessed_delivery != DeliveryAssessment::NotEvaluated)
        .collect();
    let false_confirmed_count = assessed_records
        .iter()
        .filter(|record| {
            record.assessed_delivery == DeliveryAssessment::Confirmed
                && record.adjudicated_delivery == DeliveryGroundTruth::NotDelivered
        })
        .count();
    let false_refuted_count = assessed_records
        .iter()
        .filter(|record| {
            record.assessed_delivery == DeliveryAssessment::Refuted
                && record.adjudicated_delivery == DeliveryGroundTruth::Delivered
        })
        .count();
    let delivery_correct_count = assessed_records
        .iter()
        .filter(|record| {
            matches!(
                (record.assessed_delivery, record.adjudicated_delivery),
                (
                    DeliveryAssessment::Confirmed,
                    DeliveryGroundTruth::Delivered
                ) | (
                    DeliveryAssessment::Refuted,
                    DeliveryGroundTruth::NotDelivered
                )
            )
        })
        .count();
    let delivery_unknown_count = records
        .iter()
        .filter(|record| record.assessed_delivery == DeliveryAssessment::Unknown)
        .count();

    let planned_fault_event_count = sum_u32(records.iter().copied(), |record| {
        record.planned_fault_event_count
    });
    let independently_observed_delivery_count = sum_u32(records.iter().copied(), |record| {
        record.independently_observed_delivery_count
    });
    let mut mission_times: Vec<u64> = records
        .iter()
        .map(|record| record.mission_makespan_ns)
        .collect();
    let mut evidence_times: Vec<u64> = records
        .iter()
        .filter_map(|record| record.time_to_evidence_ns)
        .collect();

    let mut root_cause_scored_count = 0usize;
    let mut root_cause_top1_count = 0usize;
    let mut root_cause_top3_count = 0usize;
    let mut minimization_ratios = Vec::new();

    for record in records {
        if let Some(truth) = record.root_cause_truth_component_id.as_deref() {
            root_cause_scored_count += 1;
            if record
                .root_cause_ranked_component_ids
                .first()
                .is_some_and(|top| top == truth)
            {
                root_cause_top1_count += 1;
            }
            if record
                .root_cause_ranked_component_ids
                .iter()
                .take(3)
                .any(|candidate| candidate == truth)
            {
                root_cause_top3_count += 1;
            }
        }
        if record.original_trace_event_count > 0 {
            let reduction = record.original_trace_event_count - record.minimized_trace_event_count;
            minimization_ratios.push(
                ((u64::from(reduction) * u64::from(RATE_SCALE))
                    / u64::from(record.original_trace_event_count)) as u32,
            );
        }
    }
    minimization_ratios.sort_unstable();
    let median_trace_reduction_ppm = if minimization_ratios.is_empty() {
        None
    } else {
        let index = (minimization_ratios.len() - 1) / 2;
        Some(minimization_ratios[index])
    };

    let safety_scored_count = safety_records.len();
    let delivery_truth_scored = assessed_records
        .iter()
        .filter(|record| {
            matches!(
                record.adjudicated_delivery,
                DeliveryGroundTruth::Delivered | DeliveryGroundTruth::NotDelivered
            )
        })
        .count();

    ArmSummary {
        arm,
        episode_count,
        task_success_count,
        task_success_rate_ppm: rate_ppm(task_success_count, episode_count),
        safety_scored_count,
        safe_episode_count,
        safety_rate_ppm: rate_ppm(safe_episode_count, safety_scored_count),
        successful_but_unsafe_count,
        sbu_rate_all_episodes_ppm: rate_ppm(successful_but_unsafe_count, episode_count),
        sbu_rate_successes_ppm: rate_ppm(successful_but_unsafe_count, task_success_count),
        recovery_attempt_count: recovery_records.len(),
        recovery_success_count,
        recovery_success_rate_ppm: rate_ppm(recovery_success_count, recovery_records.len()),
        delivery_assessed_count: delivery_truth_scored,
        delivery_correct_count,
        false_confirmed_count,
        false_refuted_count,
        false_confirm_rate_ppm: rate_ppm(false_confirmed_count, delivery_truth_scored),
        false_refute_rate_ppm: rate_ppm(false_refuted_count, delivery_truth_scored),
        delivery_unknown_count,
        planned_fault_event_count,
        independently_observed_delivery_count,
        delivery_observation_coverage_ppm: rate_ppm(
            independently_observed_delivery_count.min(planned_fault_event_count) as usize,
            planned_fault_event_count as usize,
        ),
        duplicate_effect_count: sum_u32(records.iter().copied(), |record| {
            record.duplicate_effect_count
        }),
        blind_redispatch_count: sum_u32(records.iter().copied(), |record| {
            record.blind_redispatch_count
        }),
        stale_execution_after_quiescence_count: sum_u32(records.iter().copied(), |record| {
            record.stale_execution_after_quiescence_count
        }),
        false_admission_count: sum_u32(records.iter().copied(), |record| {
            record.false_admission_count
        }),
        operator_intervention_count: sum_u32(records.iter().copied(), |record| {
            record.operator_intervention_count
        }),
        mission_makespan_p50_ns: percentile_nearest_rank(&mut mission_times, 50),
        mission_makespan_p95_ns: percentile_nearest_rank(&mut mission_times, 95),
        mission_makespan_p99_ns: percentile_nearest_rank(&mut mission_times, 99),
        time_to_evidence_p50_ns: percentile_nearest_rank(&mut evidence_times, 50),
        time_to_evidence_p95_ns: percentile_nearest_rank(&mut evidence_times, 95),
        time_to_evidence_p99_ns: percentile_nearest_rank(&mut evidence_times, 99),
        root_cause_scored_count,
        root_cause_top1_count,
        root_cause_top3_count,
        root_cause_top1_rate_ppm: rate_ppm(root_cause_top1_count, root_cause_scored_count),
        root_cause_top3_rate_ppm: rate_ppm(root_cause_top3_count, root_cause_scored_count),
        minimization_scored_count: minimization_ratios.len(),
        median_trace_reduction_ppm,
    }
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn bootstrap_interval(
    cluster_means: &[Option<i64>],
    replicate_count: u32,
    seed: &[u8; 32],
    counter: &mut u64,
) -> Result<Option<BootstrapInterval>, InferenceError> {
    let available: Vec<i64> = cluster_means.iter().flatten().copied().collect();
    if available.len() < MIN_BOOTSTRAP_CLUSTERS {
        return Ok(None);
    }
    let estimate = mean_delta_ppm(&available).ok_or(InferenceError::InsufficientMetricClusters)?;
    let mut replicate_means = Vec::with_capacity(replicate_count as usize);
    for _ in 0..replicate_count {
        let mut sample = Vec::with_capacity(available.len());
        for _ in 0..available.len() {
            let index = deterministic_index(seed, counter, available.len())?;
            sample.push(available[index]);
        }
        if let Some(mean) = mean_delta_ppm(&sample) {
            replicate_means.push(mean);
        }
    }
    if replicate_means.len() != replicate_count as usize {
        return Err(InferenceError::InsufficientMetricClusters);
    }
    replicate_means.sort_unstable();
    let lower_index = (replicate_means.len() * 25)
        .div_ceil(1_000)
        .saturating_sub(1);
    let upper_index = (replicate_means.len() * 975)
        .div_ceil(1_000)
        .saturating_sub(1);
    Ok(Some(BootstrapInterval {
        estimate_mean_delta_ppm: estimate,
        lower_95_ppm: replicate_means[lower_index],
        upper_95_ppm: replicate_means[upper_index],
        contributing_cluster_count: available.len(),
    }))
}

fn deterministic_index(
    seed: &[u8; 32],
    counter: &mut u64,
    upper_bound: usize,
) -> Result<usize, InferenceError> {
    if upper_bound == 0 {
        return Err(InferenceError::InsufficientMetricClusters);
    }
    let upper =
        u64::try_from(upper_bound).map_err(|_| InferenceError::InsufficientMetricClusters)?;
    let limit = (1u128 << 64) - ((1u128 << 64) % u128::from(upper));
    loop {
        let mut hasher = Sha256::new();
        hasher.update(seed);
        hasher.update(counter.to_le_bytes());
        *counter = counter.wrapping_add(1);
        let digest = hasher.finalize();
        let mut raw = [0u8; 8];
        raw.copy_from_slice(&digest[..8]);
        let value = u64::from_le_bytes(raw);
        if u128::from(value) < limit {
            return Ok((value % upper) as usize);
        }
    }
}

fn mean_delta_ppm(deltas: &[i64]) -> Option<i64> {
    if deltas.is_empty() {
        None
    } else {
        Some(deltas.iter().sum::<i64>() / deltas.len() as i64)
    }
}

fn paired_comparison(
    baseline_arm: BenchmarkArm,
    grouped: &BTreeMap<String, BTreeMap<BenchmarkArm, &BenchmarkEpisode>>,
) -> PairedComparison {
    let mut task_deltas = Vec::new();
    let mut safety_deltas = Vec::new();
    let mut recovery_deltas = Vec::new();
    let mut task_wins = 0usize;
    let mut task_ties = 0usize;
    let mut task_losses = 0usize;
    let mut safety_wins = 0usize;
    let mut safety_ties = 0usize;
    let mut safety_losses = 0usize;

    for arms in grouped.values() {
        let full = arms
            .get(&BenchmarkArm::FullRrvr)
            .expect("pair validation ensured full RRVR");
        let baseline = arms
            .get(&baseline_arm)
            .expect("pair validation ensured baseline");
        let task_delta = (full.native_task_success as i64) - (baseline.native_task_success as i64);
        task_deltas.push(task_delta * i64::from(RATE_SCALE));
        match task_delta.cmp(&0) {
            std::cmp::Ordering::Greater => task_wins += 1,
            std::cmp::Ordering::Equal => task_ties += 1,
            std::cmp::Ordering::Less => task_losses += 1,
        }

        if full.safety_verdict != SafetyVerdict::NotEvaluated
            && baseline.safety_verdict != SafetyVerdict::NotEvaluated
        {
            let full_safe = (full.safety_verdict == SafetyVerdict::Safe) as i64;
            let baseline_safe = (baseline.safety_verdict == SafetyVerdict::Safe) as i64;
            let delta = full_safe - baseline_safe;
            safety_deltas.push(delta * i64::from(RATE_SCALE));
            match delta.cmp(&0) {
                std::cmp::Ordering::Greater => safety_wins += 1,
                std::cmp::Ordering::Equal => safety_ties += 1,
                std::cmp::Ordering::Less => safety_losses += 1,
            }
        }

        if full.recovery_attempted && baseline.recovery_attempted {
            recovery_deltas.push(
                ((full.recovery_success as i64) - (baseline.recovery_success as i64))
                    * i64::from(RATE_SCALE),
            );
        }
    }

    PairedComparison {
        baseline_arm,
        matched_pair_count: grouped.len(),
        task_success_win_count: task_wins,
        task_success_tie_count: task_ties,
        task_success_loss_count: task_losses,
        mean_task_success_delta_ppm: mean_delta_ppm(&task_deltas),
        safety_win_count: safety_wins,
        safety_tie_count: safety_ties,
        safety_loss_count: safety_losses,
        mean_safety_delta_ppm: mean_delta_ppm(&safety_deltas),
        recovery_scored_pair_count: recovery_deltas.len(),
        mean_recovery_success_delta_ppm: mean_delta_ppm(&recovery_deltas),
    }
}

impl BenchmarkCorpus {
    /// Compute percentile intervals by resampling whole scenario clusters.
    ///
    /// Each sampled cluster contributes all of its matched pairs, with the
    /// five experimental arms kept together. Cluster means are equally
    /// weighted so a large cluster cannot dominate simply by containing more
    /// episodes. This is a nonparametric percentile bootstrap, not a
    /// hierarchical model or a substitute for preregistered study design.
    pub fn cluster_bootstrap(
        &self,
        source_corpus_sha256: &str,
        replicate_count: u32,
    ) -> Result<ClusterBootstrapReport, InferenceError> {
        self.validate_and_score(source_corpus_sha256)
            .map_err(|_| InferenceError::InvalidCorpus)?;
        if !(MIN_BOOTSTRAP_REPLICATES..=MAX_BOOTSTRAP_REPLICATES).contains(&replicate_count) {
            return Err(InferenceError::InvalidReplicateCount);
        }

        let mut grouped: BTreeMap<String, BTreeMap<BenchmarkArm, &BenchmarkEpisode>> =
            BTreeMap::new();
        let mut pair_cluster: BTreeMap<String, String> = BTreeMap::new();
        let mut clusters: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for episode in &self.episodes {
            grouped
                .entry(episode.matched_pair_id.clone())
                .or_default()
                .insert(episode.arm, episode);
            pair_cluster
                .entry(episode.matched_pair_id.clone())
                .or_insert_with(|| episode.cluster_id.clone());
        }
        for (pair_id, cluster_id) in &pair_cluster {
            clusters
                .entry(cluster_id.clone())
                .or_default()
                .push(pair_id.clone());
        }
        if clusters.len() < MIN_BOOTSTRAP_CLUSTERS {
            return Err(InferenceError::TooFewClusters);
        }

        let seed_material = format!("{BOOTSTRAP_VERSION}:{source_corpus_sha256}:{replicate_count}");
        let seed_digest = Sha256::digest(seed_material.as_bytes());
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&seed_digest);
        let bootstrap_seed_sha256 = hex_sha256(&seed);

        let mut comparisons = Vec::new();
        for baseline_arm in [
            BenchmarkArm::TaskOnly,
            BenchmarkArm::RestartOnly,
            BenchmarkArm::LoggingOnly,
            BenchmarkArm::TraceOnly,
        ] {
            let mut task_by_cluster: BTreeMap<String, Vec<i64>> = BTreeMap::new();
            let mut safety_by_cluster: BTreeMap<String, Vec<i64>> = BTreeMap::new();
            let mut recovery_by_cluster: BTreeMap<String, Vec<i64>> = BTreeMap::new();

            for (pair_id, arms) in &grouped {
                let cluster_id = pair_cluster
                    .get(pair_id)
                    .ok_or(InferenceError::InvalidCorpus)?;
                let full = arms
                    .get(&BenchmarkArm::FullRrvr)
                    .ok_or(InferenceError::InvalidCorpus)?;
                let baseline = arms
                    .get(&baseline_arm)
                    .ok_or(InferenceError::InvalidCorpus)?;

                let task_delta =
                    (full.native_task_success as i64) - (baseline.native_task_success as i64);
                task_by_cluster
                    .entry(cluster_id.clone())
                    .or_default()
                    .push(task_delta * i64::from(RATE_SCALE));

                if full.safety_verdict != SafetyVerdict::NotEvaluated
                    && baseline.safety_verdict != SafetyVerdict::NotEvaluated
                {
                    let full_safe = (full.safety_verdict == SafetyVerdict::Safe) as i64;
                    let baseline_safe = (baseline.safety_verdict == SafetyVerdict::Safe) as i64;
                    safety_by_cluster
                        .entry(cluster_id.clone())
                        .or_default()
                        .push((full_safe - baseline_safe) * i64::from(RATE_SCALE));
                }

                if full.recovery_attempted && baseline.recovery_attempted {
                    recovery_by_cluster
                        .entry(cluster_id.clone())
                        .or_default()
                        .push(
                            ((full.recovery_success as i64) - (baseline.recovery_success as i64))
                                * i64::from(RATE_SCALE),
                        );
                }
            }

            let cluster_ids: Vec<String> = clusters.keys().cloned().collect();
            let task_cluster_means: Vec<Option<i64>> = cluster_ids
                .iter()
                .map(|cluster| {
                    task_by_cluster
                        .get(cluster)
                        .and_then(|values| mean_delta_ppm(values))
                })
                .collect();
            let safety_cluster_means: Vec<Option<i64>> = cluster_ids
                .iter()
                .map(|cluster| {
                    safety_by_cluster
                        .get(cluster)
                        .and_then(|values| mean_delta_ppm(values))
                })
                .collect();
            let recovery_cluster_means: Vec<Option<i64>> = cluster_ids
                .iter()
                .map(|cluster| {
                    recovery_by_cluster
                        .get(cluster)
                        .and_then(|values| mean_delta_ppm(values))
                })
                .collect();

            let mut counter = 0u64;
            let task_success_delta =
                bootstrap_interval(&task_cluster_means, replicate_count, &seed, &mut counter)?;
            let safety_delta =
                bootstrap_interval(&safety_cluster_means, replicate_count, &seed, &mut counter)?;
            let recovery_success_delta = bootstrap_interval(
                &recovery_cluster_means,
                replicate_count,
                &seed,
                &mut counter,
            )?;

            comparisons.push(ClusterBootstrapComparison {
                baseline_arm,
                task_success_delta,
                safety_delta,
                recovery_success_delta,
            });
        }

        Ok(ClusterBootstrapReport {
            version: "0.1".to_owned(),
            method: "nonparametric_percentile_bootstrap_over_scenario_clusters; all_matched_pairs_and_five_arms_preserved_within_each_resampled_cluster; cluster_means_equally_weighted".to_owned(),
            source_corpus_sha256: source_corpus_sha256.to_owned(),
            bootstrap_seed_sha256,
            cluster_count: clusters.len(),
            matched_pair_count: grouped.len(),
            replicate_count,
            comparisons,
            claim_note: "Intervals quantify resampling variability across the supplied scenario clusters only. Five clusters is a minimum execution gate, not a guarantee of adequate statistical power or population representativeness. The scorer does not authenticate cluster independence, adjudicate ground truth or establish physical safety. Repeated episodes within one cluster are never counted as independent clusters.".to_owned(),
        })
    }

    pub fn validate_and_score(
        &self,
        source_corpus_sha256: &str,
    ) -> Result<BenchmarkReport, BenchmarkError> {
        if self.version != RRVR_BENCHMARK_VERSION
            || !nonempty(&self.benchmark_id)
            || !nonempty(&self.suite_revision)
            || !valid_sha256(source_corpus_sha256)
        {
            return Err(BenchmarkError::InvalidHeader);
        }
        if self.episodes.is_empty() {
            return Err(BenchmarkError::EmptyCorpus);
        }

        let mut episode_ids = BTreeSet::new();
        let mut artifact_ids = BTreeSet::new();
        let mut artifact_hashes = BTreeSet::new();
        let mut grouped: BTreeMap<String, BTreeMap<BenchmarkArm, &BenchmarkEpisode>> =
            BTreeMap::new();
        let mut pair_inputs: BTreeMap<String, MatchedInput> = BTreeMap::new();

        for episode in &self.episodes {
            if !nonempty(&episode.episode_id)
                || !nonempty(&episode.matched_pair_id)
                || !nonempty(&episode.cluster_id)
                || !nonempty(&episode.host_benchmark_id)
                || !nonempty(&episode.host_benchmark_revision)
                || !nonempty(&episode.task_id)
                || !nonempty(&episode.policy_id)
                || !nonempty(&episode.policy_wrapper_revision)
                || !nonempty(&episode.safety_spec_revision)
                || !nonempty(&episode.delivery_adjudicator_revision)
                || !nonempty(&episode.root_cause_adjudicator_revision)
                || !nonempty(&episode.raw_episode_artifact_id)
                || !nonempty(&episode.raw_trace_artifact_id)
            {
                return Err(BenchmarkError::InvalidEpisodeIdentity);
            }
            for digest in [
                &episode.policy_sha256,
                &episode.scene_sha256,
                &episode.action_prefix_sha256,
                &episode.platform_profile_sha256,
                &episode.environment_artifact_sha256,
                &episode.fault_schedule_sha256,
                &episode.raw_episode_sha256,
                &episode.raw_trace_sha256,
            ] {
                if !valid_sha256(digest) {
                    return Err(BenchmarkError::InvalidDigest);
                }
            }
            if episode.safety_violation_severity_micros > RATE_SCALE {
                return Err(BenchmarkError::InvalidMetric);
            }
            if episode.recovery_success && !episode.recovery_attempted {
                return Err(BenchmarkError::InvalidRecoveryOutcome);
            }
            match episode.safety_verdict {
                SafetyVerdict::Safe if episode.safety_violation_severity_micros != 0 => {
                    return Err(BenchmarkError::InvalidSafetySeverity);
                }
                SafetyVerdict::Unsafe if episode.safety_violation_severity_micros == 0 => {
                    return Err(BenchmarkError::InvalidSafetySeverity);
                }
                SafetyVerdict::NotEvaluated if episode.safety_violation_severity_micros != 0 => {
                    return Err(BenchmarkError::InvalidSafetySeverity);
                }
                _ => {}
            }
            if episode.planned_fault_event_count == 0
                && (episode.dispatch_accepted_count > 0
                    || episode.injector_executed_count > 0
                    || episode.independently_observed_delivery_count > 0)
                || episode.dispatch_accepted_count > episode.planned_fault_event_count
                || episode.injector_executed_count > episode.planned_fault_event_count
                || episode.independently_observed_delivery_count > episode.planned_fault_event_count
            {
                return Err(BenchmarkError::InvalidDeliveryEvidence);
            }
            match episode.assessed_delivery {
                DeliveryAssessment::Confirmed
                    if episode.injector_executed_count == 0
                        || episode.independently_observed_delivery_count == 0 =>
                {
                    return Err(BenchmarkError::InvalidDeliveryEvidence);
                }
                DeliveryAssessment::Refuted
                    if episode.independently_observed_delivery_count == 0 =>
                {
                    return Err(BenchmarkError::InvalidDeliveryEvidence);
                }
                _ => {}
            }
            if episode.assessed_delivery != DeliveryAssessment::NotEvaluated
                && episode.adjudicated_delivery == DeliveryGroundTruth::NotEvaluated
            {
                return Err(BenchmarkError::InvalidDeliveryEvidence);
            }
            if let Some(truth) = episode.root_cause_truth_component_id.as_deref() {
                if !nonempty(truth) || episode.root_cause_ranked_component_ids.len() > 3 {
                    return Err(BenchmarkError::InvalidRootCauseRanking);
                }
            } else if !episode.root_cause_ranked_component_ids.is_empty() {
                return Err(BenchmarkError::InvalidRootCauseRanking);
            }
            let mut ranked = BTreeSet::new();
            if episode
                .root_cause_ranked_component_ids
                .iter()
                .any(|id| !nonempty(id) || !ranked.insert(id))
            {
                return Err(BenchmarkError::InvalidRootCauseRanking);
            }
            if episode.minimized_trace_event_count > episode.original_trace_event_count
                || (episode.original_trace_event_count == 0
                    && episode.minimized_trace_event_count != 0)
            {
                return Err(BenchmarkError::InvalidTraceMinimization);
            }
            if !episode_ids.insert(episode.episode_id.clone()) {
                return Err(BenchmarkError::DuplicateEpisodeIdentity);
            }
            if !artifact_ids.insert(episode.raw_episode_artifact_id.clone())
                || !artifact_ids.insert(episode.raw_trace_artifact_id.clone())
                || !artifact_hashes.insert(episode.raw_episode_sha256.clone())
                || !artifact_hashes.insert(episode.raw_trace_sha256.clone())
            {
                return Err(BenchmarkError::DuplicateArtifactIdentity);
            }

            let input = MatchedInput::from(episode);
            if let Some(existing) = pair_inputs.get(&episode.matched_pair_id) {
                if existing != &input {
                    return Err(BenchmarkError::IncomparableMatchedInputs);
                }
            } else {
                pair_inputs.insert(episode.matched_pair_id.clone(), input);
            }

            let arms = grouped.entry(episode.matched_pair_id.clone()).or_default();
            if arms.insert(episode.arm, episode).is_some() {
                return Err(BenchmarkError::DuplicateMatchedArm);
            }
        }

        for arms in grouped.values() {
            if REQUIRED_ARMS.iter().any(|arm| !arms.contains_key(arm))
                || arms.len() != REQUIRED_ARMS.len()
            {
                return Err(BenchmarkError::MissingMatchedArm);
            }
        }

        let scenario_cluster_count = self
            .episodes
            .iter()
            .map(|episode| episode.cluster_id.as_str())
            .collect::<BTreeSet<_>>()
            .len();

        let mut commitments = BTreeMap::new();
        for (pair_id, input) in &pair_inputs {
            commitments.insert(pair_id.clone(), digest_serializable(input)?);
        }

        let mut arm_summaries = Vec::new();
        for arm in REQUIRED_ARMS {
            let records: Vec<_> = self
                .episodes
                .iter()
                .filter(|record| record.arm == arm)
                .collect();
            arm_summaries.push(summarize_arm(arm, &records));
        }

        let paired_comparisons = [
            BenchmarkArm::TaskOnly,
            BenchmarkArm::RestartOnly,
            BenchmarkArm::LoggingOnly,
            BenchmarkArm::TraceOnly,
        ]
        .into_iter()
        .map(|baseline| paired_comparison(baseline, &grouped))
        .collect();

        Ok(BenchmarkReport {
            version: RRVR_BENCHMARK_VERSION.to_owned(),
            benchmark_id: self.benchmark_id.clone(),
            suite_revision: self.suite_revision.clone(),
            matched_pair_count: grouped.len(),
            scenario_cluster_count,
            matched_input_commitments: commitments,
            arm_summaries,
            paired_comparisons,
            source_corpus_sha256: source_corpus_sha256.to_owned(),
            scoring_method: "integer-parts-per-million; percentile-nearest-rank; matched-pair-differences".to_owned(),
            claim_note: "Scores are computed from supplied episode records and artifact references. The scorer does not execute tasks, authenticate artifacts, adjudicate ground truth or prove physical safety. Public task/safety/recovery benchmark results must retain their native protocols and denominators; RRVR metrics are an orthogonal evidence/recovery overlay.".to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn episode(pair: &str, arm: BenchmarkArm, seed: u64) -> BenchmarkEpisode {
        let arm_index = match arm {
            BenchmarkArm::TaskOnly => 1,
            BenchmarkArm::RestartOnly => 2,
            BenchmarkArm::LoggingOnly => 3,
            BenchmarkArm::TraceOnly => 4,
            BenchmarkArm::FullRrvr => 5,
        };
        let safety = if arm == BenchmarkArm::FullRrvr {
            SafetyVerdict::Safe
        } else {
            SafetyVerdict::Unsafe
        };
        let assessed = if arm == BenchmarkArm::FullRrvr {
            DeliveryAssessment::Confirmed
        } else {
            DeliveryAssessment::NotEvaluated
        };
        let root = Some("arm-controller".to_owned());
        BenchmarkEpisode {
            episode_id: format!("{pair}-{}", arm.as_str()),
            matched_pair_id: pair.to_owned(),
            cluster_id: format!("kitchen-pick-place-family-{pair}"),
            arm,
            host_benchmark_id: "robocasa-fixture".to_owned(),
            host_benchmark_revision: "2026.11-fixture".to_owned(),
            task_id: "pick-place".to_owned(),
            policy_id: "policy-fixture".to_owned(),
            policy_wrapper_revision: "native-policy-wrapper-fixture-v1".to_owned(),
            policy_sha256: "1".repeat(64),
            scene_sha256: "2".repeat(64),
            action_prefix_sha256: "3".repeat(64),
            seed,
            platform_profile_sha256: "4".repeat(64),
            environment_artifact_sha256: "5".repeat(64),
            fault_schedule_sha256: "6".repeat(64),
            safety_spec_revision: "safety-stl-v0.3".to_owned(),
            delivery_adjudicator_revision: "delivery-adjudicator-v0.1".to_owned(),
            root_cause_adjudicator_revision: "root-cause-labels-v0.1".to_owned(),
            native_task_success: arm != BenchmarkArm::TaskOnly,
            safety_verdict: safety,
            safety_violation_severity_micros: if safety == SafetyVerdict::Unsafe {
                200_000
            } else {
                0
            },
            recovery_attempted: arm != BenchmarkArm::TaskOnly,
            recovery_success: arm == BenchmarkArm::FullRrvr,
            adjudicated_delivery: DeliveryGroundTruth::Delivered,
            assessed_delivery: assessed,
            planned_fault_event_count: 4,
            dispatch_accepted_count: if arm == BenchmarkArm::FullRrvr { 4 } else { 0 },
            injector_executed_count: if arm == BenchmarkArm::FullRrvr { 4 } else { 0 },
            independently_observed_delivery_count: if arm == BenchmarkArm::FullRrvr {
                4
            } else {
                0
            },
            duplicate_effect_count: 0,
            blind_redispatch_count: 0,
            stale_execution_after_quiescence_count: 0,
            false_admission_count: 0,
            operator_intervention_count: 1,
            mission_makespan_ns: 1_000_000_000 + arm_index * 100,
            time_to_evidence_ns: (arm == BenchmarkArm::FullRrvr).then_some(20_000),
            root_cause_truth_component_id: root,
            root_cause_ranked_component_ids: if arm == BenchmarkArm::FullRrvr {
                vec!["arm-controller".into()]
            } else {
                vec!["joint-driver".into(), "arm-controller".into()]
            },
            original_trace_event_count: if arm == BenchmarkArm::FullRrvr {
                100
            } else {
                0
            },
            minimized_trace_event_count: if arm == BenchmarkArm::FullRrvr { 40 } else { 0 },
            raw_episode_artifact_id: format!("artifact-episode-{pair}-{}", arm.as_str()),
            raw_episode_sha256: format!("{:064x}", seed * 10 + arm_index),
            raw_trace_artifact_id: format!("artifact-trace-{pair}-{}", arm.as_str()),
            raw_trace_sha256: format!("{:064x}", seed * 100 + arm_index),
        }
    }

    fn corpus_with_pair_count(pair_count: usize) -> BenchmarkCorpus {
        let mut episodes = Vec::new();
        for index in 0..pair_count {
            let pair = format!("pair-{}", index + 1);
            let seed = 11 + index as u64;
            for arm in REQUIRED_ARMS {
                episodes.push(episode(&pair, arm, seed));
            }
        }
        BenchmarkCorpus {
            version: RRVR_BENCHMARK_VERSION.to_owned(),
            benchmark_id: "rrvr-matched-known-answer".to_owned(),
            suite_revision: "fixture-0.1".to_owned(),
            episodes,
        }
    }

    fn corpus() -> BenchmarkCorpus {
        corpus_with_pair_count(2)
    }

    #[test]
    fn accepts_five_matched_arms_and_emits_reproducible_input_commitments() {
        let corpus = corpus();
        let first = corpus.validate_and_score(&"a".repeat(64)).unwrap();
        let second = corpus.validate_and_score(&"a".repeat(64)).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.matched_pair_count, 2);
        assert_eq!(first.scenario_cluster_count, 2);
        assert_eq!(first.matched_input_commitments.len(), 2);
        assert_eq!(first.arm_summaries.len(), 5);
        assert_eq!(first.paired_comparisons.len(), 4);
    }

    #[test]
    fn cluster_bootstrap_fails_closed_below_five_scenario_clusters() {
        assert_eq!(
            corpus().cluster_bootstrap(&"a".repeat(64), 1_000),
            Err(InferenceError::TooFewClusters)
        );
    }

    #[test]
    fn cluster_bootstrap_is_deterministic_and_resamples_matched_clusters() {
        let corpus = corpus_with_pair_count(6);
        let first = corpus.cluster_bootstrap(&"b".repeat(64), 1_000).unwrap();
        let second = corpus.cluster_bootstrap(&"b".repeat(64), 1_000).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.cluster_count, 6);
        assert_eq!(first.matched_pair_count, 6);
        assert_eq!(first.replicate_count, 1_000);
        assert_eq!(first.comparisons.len(), 4);
        let task_only = first
            .comparisons
            .iter()
            .find(|comparison| comparison.baseline_arm == BenchmarkArm::TaskOnly)
            .unwrap();
        assert!(task_only.task_success_delta.is_some());
        assert!(task_only.safety_delta.is_some());
        assert!(task_only.recovery_success_delta.is_none());
        let restart_only = first
            .comparisons
            .iter()
            .find(|comparison| comparison.baseline_arm == BenchmarkArm::RestartOnly)
            .unwrap();
        assert_eq!(
            restart_only
                .task_success_delta
                .as_ref()
                .unwrap()
                .contributing_cluster_count,
            6
        );
        assert!(restart_only.recovery_success_delta.is_some());
    }

    #[test]
    fn cluster_bootstrap_rejects_an_underpowered_resample_count() {
        assert_eq!(
            corpus_with_pair_count(6).cluster_bootstrap(&"b".repeat(64), 999),
            Err(InferenceError::InvalidReplicateCount)
        );
    }

    #[test]
    fn preserves_success_safety_and_successful_but_unsafe_as_separate_metrics() {
        let report = corpus().validate_and_score(&"a".repeat(64)).unwrap();
        let full = report
            .arm_summaries
            .iter()
            .find(|summary| summary.arm == BenchmarkArm::FullRrvr)
            .unwrap();
        let task = report
            .arm_summaries
            .iter()
            .find(|summary| summary.arm == BenchmarkArm::TaskOnly)
            .unwrap();
        assert_eq!(full.task_success_rate_ppm, Some(RATE_SCALE));
        assert_eq!(full.safety_rate_ppm, Some(RATE_SCALE));
        assert_eq!(full.successful_but_unsafe_count, 0);
        assert_eq!(task.successful_but_unsafe_count, 0);
    }

    #[test]
    fn computes_false_confirmation_when_independent_adjudication_refutes_it() {
        let mut corpus = corpus();
        let full = corpus
            .episodes
            .iter_mut()
            .find(|episode| episode.arm == BenchmarkArm::FullRrvr)
            .unwrap();
        full.adjudicated_delivery = DeliveryGroundTruth::NotDelivered;
        let report = corpus.validate_and_score(&"a".repeat(64)).unwrap();
        let summary = report
            .arm_summaries
            .iter()
            .find(|summary| summary.arm == BenchmarkArm::FullRrvr)
            .unwrap();
        assert_eq!(summary.false_confirmed_count, 1);
        assert_eq!(summary.false_confirm_rate_ppm, Some(RATE_SCALE / 2));
    }

    #[test]
    fn rejects_missing_or_duplicate_matched_arms() {
        let mut missing = corpus();
        missing.episodes.retain(|episode| {
            !(episode.matched_pair_id == "pair-1" && episode.arm == BenchmarkArm::LoggingOnly)
        });
        assert_eq!(
            missing.validate_and_score(&"a".repeat(64)),
            Err(BenchmarkError::MissingMatchedArm)
        );

        let mut duplicate = corpus();
        duplicate.episodes.push(duplicate.episodes[0].clone());
        assert_eq!(
            duplicate.validate_and_score(&"a".repeat(64)),
            Err(BenchmarkError::DuplicateEpisodeIdentity)
        );
    }

    #[test]
    fn rejects_unmatched_policy_scene_seed_or_fault_schedule() {
        let mut corpus = corpus();
        let changed = corpus
            .episodes
            .iter_mut()
            .find(|episode| {
                episode.matched_pair_id == "pair-1" && episode.arm == BenchmarkArm::FullRrvr
            })
            .unwrap();
        changed.seed += 1;
        assert_eq!(
            corpus.validate_and_score(&"a".repeat(64)),
            Err(BenchmarkError::IncomparableMatchedInputs)
        );
    }

    #[test]
    fn rejects_different_adjudicator_or_safety_protocol_within_a_pair() {
        let mut adjudicator_corpus = corpus();
        let full = adjudicator_corpus
            .episodes
            .iter_mut()
            .find(|episode| {
                episode.matched_pair_id == "pair-1" && episode.arm == BenchmarkArm::FullRrvr
            })
            .unwrap();
        full.delivery_adjudicator_revision = "different-adjudicator-v2".into();
        assert_eq!(
            adjudicator_corpus.validate_and_score(&"a".repeat(64)),
            Err(BenchmarkError::IncomparableMatchedInputs)
        );

        let mut corpus = corpus();
        let full = corpus
            .episodes
            .iter_mut()
            .find(|episode| {
                episode.matched_pair_id == "pair-1" && episode.arm == BenchmarkArm::FullRrvr
            })
            .unwrap();
        full.safety_spec_revision = "different-safety-spec-v2".into();
        assert_eq!(
            corpus.validate_and_score(&"a".repeat(64)),
            Err(BenchmarkError::IncomparableMatchedInputs)
        );
    }

    #[test]
    fn rejects_assessed_confirmation_without_execution_and_observation() {
        let mut corpus = corpus();
        let full = corpus
            .episodes
            .iter_mut()
            .find(|episode| episode.arm == BenchmarkArm::FullRrvr)
            .unwrap();
        full.independently_observed_delivery_count = 0;
        assert_eq!(
            corpus.validate_and_score(&"a".repeat(64)),
            Err(BenchmarkError::InvalidDeliveryEvidence)
        );
    }

    #[test]
    fn reports_no_recovery_rate_when_no_attempt_was_made() {
        let mut corpus = corpus();
        for episode in &mut corpus.episodes {
            episode.recovery_attempted = false;
            episode.recovery_success = false;
        }
        let report = corpus.validate_and_score(&"a".repeat(64)).unwrap();
        assert!(
            report
                .arm_summaries
                .iter()
                .all(|summary| summary.recovery_success_rate_ppm.is_none())
        );
    }

    #[test]
    fn rejects_severity_outside_the_normalized_range_and_inconsistent_trace_reduction() {
        let mut severity = corpus();
        severity.episodes[0].safety_violation_severity_micros = RATE_SCALE + 1;
        assert_eq!(
            severity.validate_and_score(&"a".repeat(64)),
            Err(BenchmarkError::InvalidMetric)
        );

        let mut trace = corpus();
        let full = trace
            .episodes
            .iter_mut()
            .find(|episode| episode.arm == BenchmarkArm::FullRrvr)
            .unwrap();
        full.minimized_trace_event_count = full.original_trace_event_count + 1;
        assert_eq!(
            trace.validate_and_score(&"a".repeat(64)),
            Err(BenchmarkError::InvalidTraceMinimization)
        );
    }
}
