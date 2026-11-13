//! Typed result-card validation and resource-normalized comparison.
//!
//! This evaluator preserves native benchmark metrics. It does not create an
//! aggregate driving score, authenticate upstream artifacts, or infer safety.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    NotRun,
    Running,
    Complete,
    Failed,
    Incomplete,
    Censored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricDirection {
    HigherIsBetter,
    LowerIsBetter,
}

impl MetricDirection {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "maximize" | "higher" => Ok(Self::HigherIsBetter),
            "minimize" | "lower" => Ok(Self::LowerIsBetter),
            _ => Err(format!(
                "unknown metric direction {value:?}; use maximize or minimize"
            )),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResultCard {
    pub schema_version: String,
    pub run_id: String,
    pub status: RunStatus,
    pub model_arm_id: String,
    pub benchmark_id: String,
    pub protocol_revision: String,
    #[serde(default)]
    pub seed_manifest_sha256: Option<String>,
    pub model: ModelRecord,
    pub data: DataRecord,
    pub experience_selection: ExperienceSelectionRecord,
    pub inference_allocation: InferenceAllocationRecord,
    pub compute: ComputeRecord,
    pub outcomes: OutcomeRecord,
    pub evidence: EvidenceRecord,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRecord {
    pub code_revision: String,
    pub checkpoint_revision: Option<String>,
    pub trainable_parameters: Option<u64>,
    pub input_privileges: Vec<String>,
    pub precision: Option<String>,
    pub teacher_privileges: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DataRecord {
    pub dataset_revision: String,
    pub train_episodes: u64,
    pub train_frames: u64,
    pub simulator_transitions: u64,
    pub human_intervention_minutes: f64,
    pub teacher_generated_labels: u64,
    pub holdout_split_sha256: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExperienceSelectionRecord {
    /// "uniform", "rarity", "entropy", "prediction_error", "decision_margin",
    /// "sam_decision_value", or "sam_recoverability".
    pub strategy_id: String,
    pub selector_revision: String,
    pub candidate_pool_sha256: Option<String>,
    pub selection_receipt_manifest_sha256: Option<String>,
    pub candidates_scored: u64,
    pub queries_issued: u64,
    pub queries_accepted: u64,
    pub queries_rejected: u64,
    pub query_deadline_misses: u64,
    pub query_simulator_transitions: u64,
    pub query_human_intervention_minutes: f64,
    /// Selector-specific compute attribution. This is a subset of total
    /// training compute, not an additional budget to sum twice.
    pub query_compute_gpu_hours: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InferenceAllocationRecord {
    /// "fixed_rollouts" or "deadline_aware_anytime".
    pub allocator_id: String,
    pub allocator_revision: String,
    pub allocation_receipt_manifest_sha256: Option<String>,
    pub latency_bound_manifest_sha256: Option<String>,
    pub decision_opportunities: u64,
    pub additional_rollouts: u64,
    /// Equal-compute comparisons must pin the same rollout compute-unit budget.
    pub rollout_compute_units_budgeted: Option<u64>,
    pub rollout_compute_units_used: Option<u64>,
    pub actuation_reserve_required_us: u64,
    pub actuation_reserve_violations: u64,
    pub deadline_skips: u64,
    /// Sum of allocator-predicted regret reductions; not a measured outcome.
    pub predicted_regret_reduction: Option<f64>,
    /// Sum of evaluator-measured regret reduction; can be negative.
    pub observed_regret_reduction: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComputeRecord {
    pub hardware_id: String,
    pub training_gpu_hours: Option<f64>,
    pub training_energy_kwh: Option<f64>,
    pub inference_energy_joules_per_frame: Option<f64>,
    pub inference_p50_ms: Option<f64>,
    pub inference_p95_ms: Option<f64>,
    pub inference_p99_ms: Option<f64>,
    pub deadline_miss_rate: Option<f64>,
    pub peak_memory_bytes: Option<u64>,
    pub precision: Option<String>,
    pub measurement_method: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeDisposition {
    Completed,
    Failed,
    Incomplete,
    Censored,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EpisodeOutcome {
    pub episode_id: String,
    /// Stable scenario-family stratum, such as intersection/unprotected-left.
    pub scenario_family: String,
    /// Shared physical/simulated scenario identity across repeated seeds/rollouts.
    /// Bootstrap resampling occurs at this cluster level, not per frame.
    pub scenario_cluster_id: String,
    pub disposition: EpisodeDisposition,
    /// A failed route can still have a defined success/collision metric.
    /// Missing values are never converted to zero by the evaluator.
    pub native_metrics: BTreeMap<String, Option<f64>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OutcomeRecord {
    pub native_metrics: BTreeMap<String, Option<f64>>,
    pub completed_episodes: u64,
    pub failed_episodes: u64,
    pub incomplete_episodes: u64,
    pub censored_episodes: u64,
    pub worst_family_metrics: BTreeMap<String, Option<f64>>,
    pub calibration_metrics: BTreeMap<String, Option<f64>>,
    pub robustness_metrics: BTreeMap<String, Option<f64>>,
    pub replay_exact_match_percent: Option<f64>,
    #[serde(default)]
    pub episode_outcomes: Vec<EpisodeOutcome>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRecord {
    pub raw_artifact_manifest_sha256: Option<String>,
    pub evaluator_revision: String,
    pub expected_episode_count: u64,
    pub observed_episode_count: u64,
    pub notes: Vec<String>,
}

impl ResultCard {
    pub fn from_json(input: &str) -> Result<Self, String> {
        serde_json::from_str(input).map_err(|error| format!("result-card JSON: {error}"))
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.schema_version != "0.2.0" {
            errors.push(format!(
                "schema_version must be 0.2.0, got {:?}",
                self.schema_version
            ));
        }
        for (name, value) in [
            ("run_id", self.run_id.as_str()),
            ("model_arm_id", self.model_arm_id.as_str()),
            ("benchmark_id", self.benchmark_id.as_str()),
            ("protocol_revision", self.protocol_revision.as_str()),
            ("model.code_revision", self.model.code_revision.as_str()),
            ("data.dataset_revision", self.data.dataset_revision.as_str()),
            ("compute.hardware_id", self.compute.hardware_id.as_str()),
            ("compute.measurement_method", self.compute.measurement_method.as_str()),
            ("evidence.evaluator_revision", self.evidence.evaluator_revision.as_str()),
        ] {
            if value.trim().is_empty() {
                errors.push(format!("{name} must be non-empty"));
            }
        }

        if self.model.input_privileges.is_empty() {
            errors.push("model.input_privileges must declare at least one policy input".into());
        }
        if has_duplicates(&self.model.input_privileges) {
            errors.push("model.input_privileges contains duplicates".into());
        }
        if has_duplicates(&self.model.teacher_privileges) {
            errors.push("model.teacher_privileges contains duplicates".into());
        }

        if !self.data.human_intervention_minutes.is_finite()
            || self.data.human_intervention_minutes < 0.0
        {
            errors.push("data.human_intervention_minutes must be finite and non-negative".into());
        }

        for (name, value) in [
            ("experience_selection.strategy_id", self.experience_selection.strategy_id.as_str()),
            ("experience_selection.selector_revision", self.experience_selection.selector_revision.as_str()),
            ("inference_allocation.allocator_id", self.inference_allocation.allocator_id.as_str()),
            ("inference_allocation.allocator_revision", self.inference_allocation.allocator_revision.as_str()),
        ] {
            if value.trim().is_empty() {
                errors.push(format!("{name} must be non-empty"));
            }
        }

        match self
            .experience_selection
            .queries_accepted
            .checked_add(self.experience_selection.queries_rejected)
        {
            Some(total) if total == self.experience_selection.queries_issued => {}
            Some(_) => errors.push(
                "experience_selection queries_issued must equal accepted + rejected".into(),
            ),
            None => errors.push("experience_selection query count overflow".into()),
        }
        if self.experience_selection.candidates_scored
            < self.experience_selection.queries_issued
        {
            errors.push("experience_selection.candidates_scored cannot be less than queries_issued".into());
        }
        if self.experience_selection.query_deadline_misses
            > self.experience_selection.queries_rejected
        {
            errors.push("experience_selection.query_deadline_misses cannot exceed rejected queries".into());
        }
        if !self.experience_selection.query_human_intervention_minutes.is_finite()
            || self.experience_selection.query_human_intervention_minutes < 0.0
            || self.experience_selection.query_human_intervention_minutes
                > self.data.human_intervention_minutes + 1e-9
        {
            errors.push("experience_selection query human minutes must be finite, non-negative and within total human intervention minutes".into());
        }
        if self.experience_selection.query_simulator_transitions
            > self.data.simulator_transitions
        {
            errors.push("experience_selection query simulator transitions exceed total simulator transitions".into());
        }
        if self.experience_selection.query_compute_gpu_hours.is_some_and(|value| {
            !value.is_finite()
                || value < 0.0
                || self.compute.training_gpu_hours.is_some_and(|total| value > total + 1e-9)
        }) {
            errors.push("experience_selection.query_compute_gpu_hours must be finite, non-negative and within total training GPU-hours".into());
        }

        if self.inference_allocation.rollout_compute_units_budgeted.is_some_and(|budget| {
            self.inference_allocation.rollout_compute_units_used.is_some_and(|used| used > budget)
        }) {
            errors.push("inference_allocation.rollout_compute_units_used exceeds its declared budget".into());
        }
        if self.inference_allocation.predicted_regret_reduction.is_some_and(|value| {
            !value.is_finite() || value < 0.0
        }) {
            errors.push("inference_allocation.predicted_regret_reduction must be finite and non-negative".into());
        }
        if self.inference_allocation.observed_regret_reduction.is_some_and(|value| !value.is_finite()) {
            errors.push("inference_allocation.observed_regret_reduction must be finite when present".into());
        }

        for (name, value) in [
            ("compute.training_gpu_hours", self.compute.training_gpu_hours),
            ("compute.training_energy_kwh", self.compute.training_energy_kwh),
            (
                "compute.inference_energy_joules_per_frame",
                self.compute.inference_energy_joules_per_frame,
            ),
            ("compute.inference_p50_ms", self.compute.inference_p50_ms),
            ("compute.inference_p95_ms", self.compute.inference_p95_ms),
            ("compute.inference_p99_ms", self.compute.inference_p99_ms),
        ] {
            if value.is_some_and(|metric| !metric.is_finite() || metric < 0.0) {
                errors.push(format!("{name} must be finite and non-negative when present"));
            }
        }
        if self
            .compute
            .deadline_miss_rate
            .is_some_and(|rate| !rate.is_finite() || !(0.0..=1.0).contains(&rate))
        {
            errors.push("compute.deadline_miss_rate must be in [0, 1] when present".into());
        }
        if let (Some(p50), Some(p95)) = (
            self.compute.inference_p50_ms,
            self.compute.inference_p95_ms,
        ) && p50 > p95
        {
            errors.push("inference p50 latency exceeds p95".into());
        }
        if let (Some(p95), Some(p99)) = (
            self.compute.inference_p95_ms,
            self.compute.inference_p99_ms,
        ) && p95 > p99
        {
            errors.push("inference p95 latency exceeds p99".into());
        }

        for (group, values) in [
            ("native_metrics", &self.outcomes.native_metrics),
            ("worst_family_metrics", &self.outcomes.worst_family_metrics),
            ("calibration_metrics", &self.outcomes.calibration_metrics),
            ("robustness_metrics", &self.outcomes.robustness_metrics),
        ] {
            for (name, value) in values {
                if value.is_some_and(|metric| !metric.is_finite()) {
                    errors.push(format!("outcomes.{group}.{name} must be finite when present"));
                }
            }
        }
        if self
            .outcomes
            .replay_exact_match_percent
            .is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
        {
            errors.push("outcomes.replay_exact_match_percent must be in [0, 100]".into());
        }

        let disposition_total = self
            .outcomes
            .completed_episodes
            .checked_add(self.outcomes.failed_episodes)
            .and_then(|value| value.checked_add(self.outcomes.incomplete_episodes))
            .and_then(|value| value.checked_add(self.outcomes.censored_episodes));
        match disposition_total {
            Some(total) if total == self.evidence.expected_episode_count => {}
            Some(total) => errors.push(format!(
                "episode denominator mismatch: expected {}, explicit dispositions sum to {total}",
                self.evidence.expected_episode_count
            )),
            None => errors.push("episode disposition count overflow".into()),
        }
        if self.evidence.observed_episode_count > self.evidence.expected_episode_count {
            errors.push("observed_episode_count exceeds expected_episode_count".into());
        }

        if self.status == RunStatus::NotRun
            && (self.evidence.expected_episode_count != 0
                || self.evidence.observed_episode_count != 0
                || self.outcomes.completed_episodes != 0
                || self.outcomes.failed_episodes != 0
                || self.outcomes.incomplete_episodes != 0
                || self.outcomes.censored_episodes != 0
                || self.outcomes.native_metrics.values().any(Option::is_some))
        {
            errors.push("not_run cards must not contain episode outcomes or metric values".into());
        }
        let mut episode_ids = std::collections::BTreeSet::new();
        let mut cluster_families = BTreeMap::<&str, &str>::new();
        let mut disposition_counts = [0_u64; 4];
        for episode in &self.outcomes.episode_outcomes {
            if episode.episode_id.trim().is_empty() {
                errors.push("episode_outcomes contains an empty episode_id".into());
            }
            if episode.scenario_family.trim().is_empty() {
                errors.push(format!("episode {:?} has an empty scenario_family", episode.episode_id));
            }
            if episode.scenario_cluster_id.trim().is_empty() {
                errors.push(format!("episode {:?} has an empty scenario_cluster_id", episode.episode_id));
            }
            if !episode_ids.insert(episode.episode_id.as_str()) {
                errors.push(format!("duplicate episode_outcomes episode_id {:?}", episode.episode_id));
            }
            if let Some(existing_family) = cluster_families.insert(
                episode.scenario_cluster_id.as_str(),
                episode.scenario_family.as_str(),
            ) && existing_family != episode.scenario_family
            {
                errors.push(format!(
                    "scenario cluster {:?} appears in multiple scenario families",
                    episode.scenario_cluster_id
                ));
            }
            let disposition_index = match episode.disposition {
                EpisodeDisposition::Completed => 0,
                EpisodeDisposition::Failed => 1,
                EpisodeDisposition::Incomplete => 2,
                EpisodeDisposition::Censored => 3,
            };
            disposition_counts[disposition_index] = disposition_counts[disposition_index].saturating_add(1);
            for (name, value) in &episode.native_metrics {
                if value.is_some_and(|metric| !metric.is_finite()) {
                    errors.push(format!(
                        "episode {:?} metric {name:?} must be finite when present",
                        episode.episode_id
                    ));
                }
            }
        }

        if self.status == RunStatus::Complete {
            if self.outcomes.episode_outcomes.len() as u64 != self.evidence.expected_episode_count {
                errors.push(format!(
                    "complete result requires one per-episode record per expected episode: got {}, expected {}",
                    self.outcomes.episode_outcomes.len(),
                    self.evidence.expected_episode_count
                ));
            }
            if disposition_counts != [
                self.outcomes.completed_episodes,
                self.outcomes.failed_episodes,
                self.outcomes.incomplete_episodes,
                self.outcomes.censored_episodes,
            ] {
                errors.push("per-episode dispositions do not match aggregate episode denominators".into());
            }
        }

        if self.status == RunStatus::Complete {
            for (name, digest) in [
                ("experience_selection.candidate_pool_sha256", self.experience_selection.candidate_pool_sha256.as_deref()),
                ("experience_selection.selection_receipt_manifest_sha256", self.experience_selection.selection_receipt_manifest_sha256.as_deref()),
                ("inference_allocation.allocation_receipt_manifest_sha256", self.inference_allocation.allocation_receipt_manifest_sha256.as_deref()),
                ("inference_allocation.latency_bound_manifest_sha256", self.inference_allocation.latency_bound_manifest_sha256.as_deref()),
            ] {
                if digest.is_none() {
                    errors.push(format!("complete result requires {name}"));
                }
            }
            if self.inference_allocation.rollout_compute_units_budgeted.is_none()
                || self.inference_allocation.rollout_compute_units_used.is_none()
            {
                errors.push("complete result requires inference rollout compute budget and used units".into());
            }
            if self.experience_selection.query_compute_gpu_hours.is_none() {
                errors.push("complete result requires experience-selection compute attribution".into());
            }
            if self.evidence.expected_episode_count == 0 {
                errors.push("complete result must declare a non-zero expected denominator".into());
            }
            if self.evidence.raw_artifact_manifest_sha256.is_none() {
                errors.push("complete result requires raw_artifact_manifest_sha256".into());
            }
            if self.seed_manifest_sha256.is_none() {
                errors.push("complete result requires seed_manifest_sha256".into());
            }
            if self.data.holdout_split_sha256.is_none() {
                errors.push("complete result requires holdout_split_sha256".into());
            }
        }

        for (name, digest) in [
            ("seed_manifest_sha256", self.seed_manifest_sha256.as_deref()),
            (
                "data.holdout_split_sha256",
                self.data.holdout_split_sha256.as_deref(),
            ),
            (
                "evidence.raw_artifact_manifest_sha256",
                self.evidence.raw_artifact_manifest_sha256.as_deref(),
            ),
            (
                "experience_selection.candidate_pool_sha256",
                self.experience_selection.candidate_pool_sha256.as_deref(),
            ),
            (
                "experience_selection.selection_receipt_manifest_sha256",
                self.experience_selection.selection_receipt_manifest_sha256.as_deref(),
            ),
            (
                "inference_allocation.allocation_receipt_manifest_sha256",
                self.inference_allocation.allocation_receipt_manifest_sha256.as_deref(),
            ),
            (
                "inference_allocation.latency_bound_manifest_sha256",
                self.inference_allocation.latency_bound_manifest_sha256.as_deref(),
            ),
        ] {
            if digest.is_some_and(|value| !is_sha256_hex(value)) {
                errors.push(format!("{name} must be a lowercase SHA-256 digest"));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn native_metric(&self, name: &str) -> Option<f64> {
        self.outcomes.native_metrics.get(name).copied().flatten()
    }
}

fn has_duplicates(values: &[String]) -> bool {
    let mut sorted = values.to_vec();
    sorted.sort();
    sorted.windows(2).any(|pair| pair[0] == pair[1])
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonClass {
    MatchedCapability,
    CapabilityFrontierOnly,
}

#[derive(Debug, Clone, Serialize)]
pub struct PairedBootstrapSummary {
    pub paired_episode_count: usize,
    pub scenario_cluster_count: usize,
    pub scenario_family_count: usize,
    pub bootstrap_replicates: usize,
    pub bootstrap_seed: u64,
    pub point_estimate_candidate_minus_baseline: f64,
    pub confidence_level: f64,
    pub confidence_interval_lower: f64,
    pub confidence_interval_upper: f64,
    /// True only when the two-sided percentile interval lies entirely in the
    /// candidate-favorable direction. This is not a multiplicity-adjusted claim.
    pub interval_supports_candidate_advantage: bool,
}

fn splitmix64_next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn bounded_index(state: &mut u64, upper: usize) -> usize {
    (((splitmix64_next(state) as u128) * (upper as u128)) >> 64) as usize
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn percentile(sorted_values: &[f64], probability: f64) -> f64 {
    if sorted_values.len() == 1 {
        return sorted_values[0];
    }
    let position = probability * (sorted_values.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let fraction = position - lower as f64;
    sorted_values[lower] * (1.0 - fraction) + sorted_values[upper] * fraction
}

fn bootstrap_seed_from_manifest(digest: &str) -> Result<u64, String> {
    if !is_sha256_hex(digest) {
        return Err("paired bootstrap requires a lowercase SHA-256 seed manifest".into());
    }
    u64::from_str_radix(&digest[..16], 16)
        .map_err(|error| format!("cannot derive bootstrap seed from manifest: {error}"))
}

/// Paired, scenario-family-stratified cluster bootstrap of a native metric.
///
/// - Both runs must contain exactly the same episode IDs, scenario families,
///   cluster IDs and dispositions.
/// - Each paired episode must have an explicit finite value for the metric.
/// - Repeated rollouts/seeds from one scenario are averaged inside their
///   scenario cluster before resampling, avoiding frame/rollout pseudoreplication.
/// - Clusters are sampled with replacement within each scenario family.
/// - No missing, failed, censored or null outcome is silently converted to zero.
pub fn paired_stratified_cluster_bootstrap(
    baseline: &ResultCard,
    candidate: &ResultCard,
    metric_name: &str,
    direction: MetricDirection,
    replicates: usize,
    seed: u64,
) -> Result<PairedBootstrapSummary, String> {
    validate_complete(baseline)?;
    validate_complete(candidate)?;
    if baseline.run_id == candidate.run_id {
        return Err("baseline and candidate must have distinct run_id values".into());
    }
    let protocol_differences = evaluation_differences(baseline, candidate);
    if !protocol_differences.is_empty() {
        return Err(format!(
            "paired bootstrap requires one evaluation protocol: {}",
            protocol_differences.join(", ")
        ));
    }
    if !(1_000..=200_000).contains(&replicates) {
        return Err("bootstrap replicates must be in 1,000..=200,000".into());
    }

    let baseline_by_id = baseline.outcomes.episode_outcomes
        .iter()
        .map(|episode| (episode.episode_id.as_str(), episode))
        .collect::<BTreeMap<_, _>>();
    let candidate_by_id = candidate.outcomes.episode_outcomes
        .iter()
        .map(|episode| (episode.episode_id.as_str(), episode))
        .collect::<BTreeMap<_, _>>();
    if baseline_by_id.len() != baseline.outcomes.episode_outcomes.len()
        || candidate_by_id.len() != candidate.outcomes.episode_outcomes.len()
    {
        return Err("paired bootstrap found duplicate episode IDs".into());
    }
    if baseline_by_id.keys().ne(candidate_by_id.keys()) {
        return Err("paired bootstrap requires identical episode ID denominators".into());
    }

    let mut grouped = BTreeMap::<(String, String), Vec<f64>>::new();
    for (episode_id, baseline_episode) in &baseline_by_id {
        let candidate_episode = candidate_by_id
            .get(episode_id)
            .ok_or_else(|| format!("candidate lacks paired episode {episode_id:?}"))?;
        if baseline_episode.scenario_family != candidate_episode.scenario_family {
            return Err(format!("scenario family mismatch for episode {episode_id:?}"));
        }
        if baseline_episode.scenario_cluster_id != candidate_episode.scenario_cluster_id {
            return Err(format!("scenario cluster mismatch for episode {episode_id:?}"));
        }
        if baseline_episode.disposition != candidate_episode.disposition {
            return Err(format!("episode disposition mismatch for episode {episode_id:?}"));
        }
        let baseline_value = baseline_episode.native_metrics
            .get(metric_name)
            .copied()
            .flatten()
            .ok_or_else(|| format!("baseline episode {episode_id:?} lacks metric {metric_name:?}"))?;
        let candidate_value = candidate_episode.native_metrics
            .get(metric_name)
            .copied()
            .flatten()
            .ok_or_else(|| format!("candidate episode {episode_id:?} lacks metric {metric_name:?}"))?;
        if !baseline_value.is_finite() || !candidate_value.is_finite() {
            return Err(format!("non-finite paired metric at episode {episode_id:?}"));
        }
        let group = (
            baseline_episode.scenario_family.clone(),
            baseline_episode.scenario_cluster_id.clone(),
        );
        grouped.entry(group).or_default().push(candidate_value - baseline_value);
    }

    let mut families = BTreeMap::<String, Vec<Vec<f64>>>::new();
    let mut all_episode_differences = Vec::new();
    for ((family, _cluster_id), differences) in grouped {
        all_episode_differences.extend(differences.iter().copied());
        families.entry(family).or_default().push(differences);
    }
    if families.is_empty() {
        return Err("paired bootstrap has no episodes".into());
    }
    for (family, clusters) in &families {
        if clusters.len() < 2 {
            return Err(format!(
                "scenario family {family:?} has fewer than two independent clusters"
            ));
        }
    }

    let mut point_cluster_means = Vec::new();
    for clusters in families.values() {
        for cluster in clusters {
            point_cluster_means.push(mean(cluster));
        }
    }
    let point_estimate = mean(&point_cluster_means);

    let mut state = seed;
    let mut bootstrap_values = Vec::with_capacity(replicates);
    for _ in 0..replicates {
        let mut resampled_cluster_means = Vec::with_capacity(point_cluster_means.len());
        for clusters in families.values() {
            for _ in 0..clusters.len() {
                let selected = bounded_index(&mut state, clusters.len());
                resampled_cluster_means.push(mean(&clusters[selected]));
            }
        }
        bootstrap_values.push(mean(&resampled_cluster_means));
    }
    bootstrap_values.sort_by(f64::total_cmp);
    let lower = percentile(&bootstrap_values, 0.025);
    let upper = percentile(&bootstrap_values, 0.975);
    let advantage = match direction {
        MetricDirection::HigherIsBetter => lower > 0.0,
        MetricDirection::LowerIsBetter => upper < 0.0,
    };

    Ok(PairedBootstrapSummary {
        paired_episode_count: all_episode_differences.len(),
        scenario_cluster_count: point_cluster_means.len(),
        scenario_family_count: families.len(),
        bootstrap_replicates: replicates,
        bootstrap_seed: seed,
        point_estimate_candidate_minus_baseline: point_estimate,
        confidence_level: 0.95,
        confidence_interval_lower: lower,
        confidence_interval_upper: upper,
        interval_supports_candidate_advantage: advantage,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct PairComparison {
    pub baseline_run_id: String,
    pub candidate_run_id: String,
    pub native_metric: String,
    pub baseline_value: f64,
    pub candidate_value: f64,
    pub candidate_minus_baseline: f64,
    pub direction: String,
    pub class: ComparisonClass,
    pub budget_differences: Vec<String>,
    pub candidate_to_baseline_inference_energy_ratio: Option<f64>,
    pub candidate_to_baseline_p99_latency_ratio: Option<f64>,
    pub candidate_to_baseline_rollout_compute_units_ratio: Option<f64>,
    pub paired_bootstrap: PairedBootstrapSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncompatibleEvaluation {
    pub differences: Vec<String>,
}

pub fn compare_metric(
    baseline: &ResultCard,
    candidate: &ResultCard,
    metric_name: &str,
    direction: MetricDirection,
) -> Result<PairComparison, String> {
    validate_complete(baseline)?;
    validate_complete(candidate)?;
    if baseline.run_id == candidate.run_id {
        return Err("baseline and candidate must have distinct run_id values".into());
    }

    let evaluation_differences = evaluation_differences(baseline, candidate);
    if !evaluation_differences.is_empty() {
        return Err(format!(
            "evaluation protocols are not directly comparable: {}",
            evaluation_differences.join(", ")
        ));
    }

    let baseline_value = baseline
        .native_metric(metric_name)
        .ok_or_else(|| format!("baseline metric {metric_name:?} is missing or null"))?;
    let candidate_value = candidate
        .native_metric(metric_name)
        .ok_or_else(|| format!("candidate metric {metric_name:?} is missing or null"))?;

    let budget_differences = budget_differences(baseline, candidate);
    let bootstrap_seed = bootstrap_seed_from_manifest(
        baseline.seed_manifest_sha256.as_deref().ok_or("bootstrap seed manifest is missing")?
    )?;
    let paired_bootstrap = paired_stratified_cluster_bootstrap(
        baseline,
        candidate,
        metric_name,
        direction,
        10_000,
        bootstrap_seed,
    )?;
    let class = if budget_differences.is_empty() {
        ComparisonClass::MatchedCapability
    } else {
        ComparisonClass::CapabilityFrontierOnly
    };
    let energy_ratio = ratio(
        candidate.compute.inference_energy_joules_per_frame,
        baseline.compute.inference_energy_joules_per_frame,
    );
    let latency_ratio = ratio(
        candidate.compute.inference_p99_ms,
        baseline.compute.inference_p99_ms,
    );
    let rollout_compute_ratio = ratio(
        candidate.inference_allocation.rollout_compute_units_used.map(|value| value as f64),
        baseline.inference_allocation.rollout_compute_units_used.map(|value| value as f64),
    );

    Ok(PairComparison {
        baseline_run_id: baseline.run_id.clone(),
        candidate_run_id: candidate.run_id.clone(),
        native_metric: metric_name.to_string(),
        baseline_value,
        candidate_value,
        candidate_minus_baseline: candidate_value - baseline_value,
        direction: match direction {
            MetricDirection::HigherIsBetter => "higher_is_better".to_string(),
            MetricDirection::LowerIsBetter => "lower_is_better".to_string(),
        },
        class,
        budget_differences,
        candidate_to_baseline_inference_energy_ratio: energy_ratio,
        candidate_to_baseline_p99_latency_ratio: latency_ratio,
        candidate_to_baseline_rollout_compute_units_ratio: rollout_compute_ratio,
        paired_bootstrap,
    })
}

pub fn point_estimate_within_margin(
    baseline: f64,
    candidate: f64,
    margin: f64,
    direction: MetricDirection,
) -> bool {
    if !baseline.is_finite() || !candidate.is_finite() || !margin.is_finite() || margin < 0.0 {
        return false;
    }
    match direction {
        MetricDirection::HigherIsBetter => candidate >= baseline - margin,
        MetricDirection::LowerIsBetter => candidate <= baseline + margin,
    }
}

fn validate_complete(card: &ResultCard) -> Result<(), String> {
    card.validate().map_err(|errors| errors.join("; "))?;
    if card.status != RunStatus::Complete {
        return Err(format!(
            "run {} is {:?}, not complete",
            card.run_id, card.status
        ));
    }
    Ok(())
}

fn evaluation_differences(baseline: &ResultCard, candidate: &ResultCard) -> Vec<String> {
    let mut differences = Vec::new();
    if baseline.benchmark_id != candidate.benchmark_id {
        differences.push("benchmark_id".to_string());
    }
    if baseline.protocol_revision != candidate.protocol_revision {
        differences.push("protocol_revision".to_string());
    }
    if baseline.evidence.evaluator_revision != candidate.evidence.evaluator_revision {
        differences.push("evaluator_revision".to_string());
    }
    if baseline.data.holdout_split_sha256 != candidate.data.holdout_split_sha256 {
        differences.push("holdout_split_sha256".to_string());
    }
    if baseline.seed_manifest_sha256 != candidate.seed_manifest_sha256 {
        differences.push("seed_manifest_sha256".to_string());
    }
    if sorted(&baseline.model.input_privileges) != sorted(&candidate.model.input_privileges) {
        differences.push("input_privileges".to_string());
    }
    if sorted(&baseline.model.teacher_privileges)
        != sorted(&candidate.model.teacher_privileges)
    {
        differences.push("teacher_privileges".to_string());
    }
    differences
}

fn budget_differences(baseline: &ResultCard, candidate: &ResultCard) -> Vec<String> {
    let mut differences = Vec::new();
    if baseline.data.dataset_revision != candidate.data.dataset_revision {
        differences.push("dataset_revision".to_string());
    }
    if baseline.data.train_episodes != candidate.data.train_episodes {
        differences.push("train_episodes".to_string());
    }
    if baseline.data.train_frames != candidate.data.train_frames {
        differences.push("train_frames".to_string());
    }
    if baseline.data.simulator_transitions != candidate.data.simulator_transitions {
        differences.push("simulator_transitions".to_string());
    }
    if (baseline.data.human_intervention_minutes - candidate.data.human_intervention_minutes).abs()
        > 1e-9
    {
        differences.push("human_intervention_minutes".to_string());
    }
    if baseline.data.teacher_generated_labels != candidate.data.teacher_generated_labels {
        differences.push("teacher_generated_labels".to_string());
    }
    match (baseline.compute.training_gpu_hours, candidate.compute.training_gpu_hours) {
        (Some(left), Some(right)) if (left - right).abs() <= 1e-9 => {}
        _ => differences.push("training_gpu_hours_unmatched_or_unreported".to_string()),
    }
    match (baseline.compute.training_energy_kwh, candidate.compute.training_energy_kwh) {
        (Some(left), Some(right)) if (left - right).abs() <= 1e-9 => {}
        _ => differences.push("training_energy_kwh_unmatched_or_unreported".to_string()),
    }
    for (field, left_present, right_present) in [
        (
            "trainable_parameters_unreported",
            baseline.model.trainable_parameters.is_some(),
            candidate.model.trainable_parameters.is_some(),
        ),
        (
            "inference_energy_unreported",
            baseline.compute.inference_energy_joules_per_frame.is_some(),
            candidate.compute.inference_energy_joules_per_frame.is_some(),
        ),
        (
            "inference_p99_latency_unreported",
            baseline.compute.inference_p99_ms.is_some(),
            candidate.compute.inference_p99_ms.is_some(),
        ),
        (
            "peak_memory_unreported",
            baseline.compute.peak_memory_bytes.is_some(),
            candidate.compute.peak_memory_bytes.is_some(),
        ),
        (
            "deadline_miss_rate_unreported",
            baseline.compute.deadline_miss_rate.is_some(),
            candidate.compute.deadline_miss_rate.is_some(),
        ),
    ] {
        if !left_present || !right_present {
            differences.push(field.to_string());
        }
    }
    if baseline.compute.hardware_id != candidate.compute.hardware_id {
        differences.push("inference_hardware_id".to_string());
    }
    if baseline.compute.precision != candidate.compute.precision {
        differences.push("inference_precision".to_string());
    }
    if baseline.model.precision != candidate.model.precision {
        differences.push("model_precision".to_string());
    }
    if baseline.compute.measurement_method != candidate.compute.measurement_method {
        differences.push("measurement_method".to_string());
    }
    match (
        baseline.inference_allocation.rollout_compute_units_budgeted,
        candidate.inference_allocation.rollout_compute_units_budgeted,
    ) {
        (Some(left), Some(right)) if left == right => {}
        _ => differences.push("rollout_compute_budget_unmatched_or_unreported".to_string()),
    }
    if baseline.inference_allocation.actuation_reserve_required_us
        != candidate.inference_allocation.actuation_reserve_required_us
    {
        differences.push("actuation_reserve_unmatched".to_string());
    }
    differences
}

fn sorted(values: &[String]) -> Vec<String> {
    let mut result = values.to_vec();
    result.sort();
    result
}

fn ratio(candidate: Option<f64>, baseline: Option<f64>) -> Option<f64> {
    match (candidate, baseline) {
        (Some(candidate), Some(baseline)) if baseline > 0.0 => Some(candidate / baseline),
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ParetoFrontier {
    pub benchmark_id: String,
    pub protocol_revision: String,
    pub primary_metric: String,
    pub metric_direction: String,
    pub frontier_run_ids: Vec<String>,
    pub incomplete_run_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct FrontierPoint {
    primary_metric: f64,
    trainable_parameters: u64,
    training_gpu_hours: f64,
    training_energy_kwh: f64,
    simulator_transitions: u64,
    human_intervention_minutes: f64,
    inference_energy: f64,
    rollout_compute_units_used: u64,
    p99_latency: f64,
    peak_memory: u64,
}

pub fn pareto_frontier(
    cards: &[ResultCard],
    primary_metric: &str,
    direction: MetricDirection,
) -> Result<ParetoFrontier, String> {
    if cards.is_empty() {
        return Err("at least one result card is required".into());
    }
    for card in cards {
        validate_complete(card)?;
    }
    let baseline = &cards[0];
    let mut unique_run_ids = std::collections::BTreeSet::new();
    for card in cards {
        if !unique_run_ids.insert(card.run_id.as_str()) {
            return Err(format!("duplicate run_id in Pareto input: {:?}", card.run_id));
        }
    }
    let mut protocol_differences = Vec::new();
    for card in cards.iter().skip(1) {
        protocol_differences.extend(evaluation_differences(baseline, card));
    }
    protocol_differences.sort();
    protocol_differences.dedup();
    if !protocol_differences.is_empty() {
        return Err(format!(
            "Pareto frontier requires one evaluation protocol; mismatches: {}",
            protocol_differences.join(", ")
        ));
    }

    let mut complete_points = Vec::new();
    let mut incomplete_run_ids = Vec::new();
    for card in cards {
        let point = match (
            card.native_metric(primary_metric),
            card.model.trainable_parameters,
            card.compute.training_gpu_hours,
            card.compute.training_energy_kwh,
            Some(card.data.simulator_transitions),
            Some(card.data.human_intervention_minutes),
            card.compute.inference_energy_joules_per_frame,
            card.inference_allocation.rollout_compute_units_used,
            card.compute.inference_p99_ms,
            card.compute.peak_memory_bytes,
        ) {
            (
                Some(primary_metric),
                Some(trainable_parameters),
                Some(training_gpu_hours),
                Some(training_energy_kwh),
                Some(simulator_transitions),
                Some(human_intervention_minutes),
                Some(inference_energy),
                Some(rollout_compute_units_used),
                Some(p99_latency),
                Some(peak_memory),
            ) => {
                Some(FrontierPoint {
                    primary_metric,
                    trainable_parameters,
                    training_gpu_hours,
                    training_energy_kwh,
                    simulator_transitions,
                    human_intervention_minutes,
                    inference_energy,
                    rollout_compute_units_used,
                    p99_latency,
                    peak_memory,
                })
            }
            _ => None,
        };
        if let Some(point) = point {
            complete_points.push((card.run_id.as_str(), point));
        } else {
            incomplete_run_ids.push(card.run_id.clone());
        }
    }

    let mut frontier_run_ids = Vec::new();
    for (candidate_id, candidate_point) in &complete_points {
        let dominated = complete_points.iter().any(|(other_id, other_point)| {
            other_id != candidate_id && dominates(*other_point, *candidate_point, direction)
        });
        if !dominated {
            frontier_run_ids.push((*candidate_id).to_string());
        }
    }
    frontier_run_ids.sort();
    incomplete_run_ids.sort();

    Ok(ParetoFrontier {
        benchmark_id: baseline.benchmark_id.clone(),
        protocol_revision: baseline.protocol_revision.clone(),
        primary_metric: primary_metric.to_string(),
        metric_direction: match direction {
            MetricDirection::HigherIsBetter => "higher_is_better".to_string(),
            MetricDirection::LowerIsBetter => "lower_is_better".to_string(),
        },
        frontier_run_ids,
        incomplete_run_ids,
    })
}

fn dominates(
    candidate: FrontierPoint,
    other: FrontierPoint,
    direction: MetricDirection,
) -> bool {
    let primary_no_worse = match direction {
        MetricDirection::HigherIsBetter => candidate.primary_metric >= other.primary_metric,
        MetricDirection::LowerIsBetter => candidate.primary_metric <= other.primary_metric,
    };
    let primary_better = match direction {
        MetricDirection::HigherIsBetter => candidate.primary_metric > other.primary_metric,
        MetricDirection::LowerIsBetter => candidate.primary_metric < other.primary_metric,
    };
    let params_no_worse = candidate.trainable_parameters <= other.trainable_parameters;
    let training_gpu_no_worse = candidate.training_gpu_hours <= other.training_gpu_hours;
    let training_energy_no_worse = candidate.training_energy_kwh <= other.training_energy_kwh;
    let transitions_no_worse = candidate.simulator_transitions <= other.simulator_transitions;
    let intervention_no_worse =
        candidate.human_intervention_minutes <= other.human_intervention_minutes;
    let energy_no_worse = candidate.inference_energy <= other.inference_energy;
    let rollout_compute_no_worse =
        candidate.rollout_compute_units_used <= other.rollout_compute_units_used;
    let latency_no_worse = candidate.p99_latency <= other.p99_latency;
    let memory_no_worse = candidate.peak_memory <= other.peak_memory;
    let strictly_better = primary_better
        || candidate.trainable_parameters < other.trainable_parameters
        || candidate.training_gpu_hours < other.training_gpu_hours
        || candidate.training_energy_kwh < other.training_energy_kwh
        || candidate.simulator_transitions < other.simulator_transitions
        || candidate.human_intervention_minutes < other.human_intervention_minutes
        || candidate.inference_energy < other.inference_energy
        || candidate.rollout_compute_units_used < other.rollout_compute_units_used
        || candidate.p99_latency < other.p99_latency
        || candidate.peak_memory < other.peak_memory;

    primary_no_worse
        && params_no_worse
        && training_gpu_no_worse
        && training_energy_no_worse
        && transitions_no_worse
        && intervention_no_worse
        && energy_no_worse
        && rollout_compute_no_worse
        && latency_no_worse
        && memory_no_worse
        && strictly_better
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(
        run_id: &str,
        score: f64,
        energy: Option<f64>,
        p99: Option<f64>,
        memory: Option<u64>,
    ) -> ResultCard {
        let mut native_metrics = BTreeMap::new();
        native_metrics.insert("driving_score".to_string(), Some(score));
        ResultCard {
            schema_version: "0.2.0".into(),
            run_id: run_id.into(),
            status: RunStatus::Complete,
            model_arm_id: "E".into(),
            benchmark_id: "bench2drive-v0.0.4".into(),
            protocol_revision: "bench2drive-v0.0.4-eval-v1".into(),
            seed_manifest_sha256: Some("a".repeat(64)),
            model: ModelRecord {
                code_revision: format!("model-{run_id}"),
                checkpoint_revision: Some(format!("checkpoint-{run_id}")),
                trainable_parameters: Some(10_000),
                input_privileges: vec!["camera-rgb".into(), "vehicle-state".into()],
                precision: Some("fp16".into()),
                teacher_privileges: vec![],
            },
            data: DataRecord {
                dataset_revision: "dataset-v1".into(),
                train_episodes: 100,
                train_frames: 10_000,
                simulator_transitions: 50_000,
                human_intervention_minutes: 30.0,
                teacher_generated_labels: 0,
                holdout_split_sha256: Some("b".repeat(64)),
            },
            experience_selection: ExperienceSelectionRecord {
                strategy_id: "uniform".into(),
                selector_revision: "uniform-baseline-v1".into(),
                candidate_pool_sha256: Some("d".repeat(64)),
                selection_receipt_manifest_sha256: Some("e".repeat(64)),
                candidates_scored: 50_000,
                queries_issued: 1_000,
                queries_accepted: 1_000,
                queries_rejected: 0,
                query_deadline_misses: 0,
                query_simulator_transitions: 0,
                query_human_intervention_minutes: 0.0,
                query_compute_gpu_hours: Some(0.0),
            },
            inference_allocation: InferenceAllocationRecord {
                allocator_id: "fixed_rollouts".into(),
                allocator_revision: "fixed-rollouts-v1".into(),
                allocation_receipt_manifest_sha256: Some("f".repeat(64)),
                latency_bound_manifest_sha256: Some("1".repeat(64)),
                decision_opportunities: 100,
                additional_rollouts: 0,
                rollout_compute_units_budgeted: Some(10_000),
                rollout_compute_units_used: Some(10_000),
                actuation_reserve_required_us: 5_000,
                actuation_reserve_violations: 0,
                deadline_skips: 0,
                predicted_regret_reduction: Some(0.0),
                observed_regret_reduction: Some(0.0),
            },
            compute: ComputeRecord {
                hardware_id: "same-gpu".into(),
                training_gpu_hours: Some(20.0),
                training_energy_kwh: Some(5.0),
                inference_energy_joules_per_frame: energy,
                inference_p50_ms: Some(10.0),
                inference_p95_ms: Some(15.0),
                inference_p99_ms: p99,
                deadline_miss_rate: Some(0.001),
                peak_memory_bytes: memory,
                precision: Some("fp16".into()),
                measurement_method: "wall-meter-and-synchronized-clock".into(),
            },
            outcomes: OutcomeRecord {
                native_metrics,
                completed_episodes: 100,
                failed_episodes: 0,
                incomplete_episodes: 0,
                censored_episodes: 0,
                worst_family_metrics: BTreeMap::new(),
                calibration_metrics: BTreeMap::new(),
                robustness_metrics: BTreeMap::new(),
                replay_exact_match_percent: Some(100.0),
                episode_outcomes: (0..100_u64).map(|index| {
                    let mut metrics = BTreeMap::new();
                    metrics.insert("driving_score".to_string(), Some(score));
                    EpisodeOutcome {
                        episode_id: format!("episode-{index:03}"),
                        scenario_family: "urban-interaction".to_string(),
                        scenario_cluster_id: format!("scenario-{:03}", index / 5),
                        disposition: EpisodeDisposition::Completed,
                        native_metrics: metrics,
                    }
                }).collect(),
            },
            evidence: EvidenceRecord {
                raw_artifact_manifest_sha256: Some("c".repeat(64)),
                evaluator_revision: "evaluator-v1".into(),
                expected_episode_count: 100,
                observed_episode_count: 100,
                notes: vec![],
            },
        }
    }

    #[test]
    fn complete_result_card_validates() {
        assert!(card("a", 90.0, Some(2.0), Some(20.0), Some(1_000)).validate().is_ok());
    }

    #[test]
    fn rejects_incomplete_episode_denominator() {
        let mut result = card("a", 90.0, Some(2.0), Some(20.0), Some(1_000));
        result.outcomes.failed_episodes = 1;
        assert!(result.validate().is_err());
    }

    #[test]
    fn rejects_nonmonotonic_latency_percentiles() {
        let mut result = card("a", 90.0, Some(2.0), Some(14.0), Some(1_000));
        result.compute.inference_p95_ms = Some(15.0);
        result.compute.inference_p99_ms = Some(14.0);
        assert!(result.validate().is_err());
    }

    #[test]
    fn rejects_sampler_query_receipt_count_mismatch() {
        let mut result = card("a", 90.0, Some(2.0), Some(20.0), Some(1_000));
        result.experience_selection.queries_accepted += 1;
        assert!(result.validate().is_err());
    }

    #[test]
    fn rejects_sampler_costs_that_exceed_total_training_exposure() {
        let mut result = card("a", 90.0, Some(2.0), Some(20.0), Some(1_000));
        result.experience_selection.query_simulator_transitions =
            result.data.simulator_transitions + 1;
        assert!(result.validate().is_err());

        result = card("b", 90.0, Some(2.0), Some(20.0), Some(1_000));
        result.experience_selection.query_human_intervention_minutes =
            result.data.human_intervention_minutes + 1.0;
        assert!(result.validate().is_err());

        result = card("c", 90.0, Some(2.0), Some(20.0), Some(1_000));
        result.experience_selection.query_compute_gpu_hours =
            Some(result.compute.training_gpu_hours.unwrap() + 1.0);
        assert!(result.validate().is_err());
    }

    #[test]
    fn rejects_anytime_rollout_compute_over_budget() {
        let mut result = card("a", 90.0, Some(2.0), Some(20.0), Some(1_000));
        result.inference_allocation.rollout_compute_units_used = Some(10_001);
        assert!(result.validate().is_err());
    }

    #[test]
    fn complete_result_requires_sampler_and_allocator_receipt_provenance() {
        let mut result = card("a", 90.0, Some(2.0), Some(20.0), Some(1_000));
        result.experience_selection.selection_receipt_manifest_sha256 = None;
        assert!(result.validate().is_err());

        result = card("b", 90.0, Some(2.0), Some(20.0), Some(1_000));
        result.inference_allocation.latency_bound_manifest_sha256 = None;
        assert!(result.validate().is_err());
    }

    #[test]
    fn equal_budget_sampler_and_anytime_allocator_remain_matched_comparison_arms() {
        let baseline = card("fixed", 90.0, Some(2.0), Some(20.0), Some(1_000));
        let mut candidate = card("anytime", 91.0, Some(2.0), Some(18.0), Some(900));
        candidate.experience_selection.strategy_id = "sam_decision_value".into();
        candidate.experience_selection.selector_revision = "sam-selector-v1".into();
        candidate.experience_selection.candidates_scored = 10_000;
        candidate.experience_selection.queries_issued = 100;
        candidate.experience_selection.queries_accepted = 90;
        candidate.experience_selection.queries_rejected = 10;
        candidate.experience_selection.query_deadline_misses = 2;
        candidate.experience_selection.query_simulator_transitions = 500;
        candidate.experience_selection.query_human_intervention_minutes = 5.0;
        candidate.experience_selection.query_compute_gpu_hours = Some(1.0);
        candidate.inference_allocation.allocator_id = "deadline_aware_anytime".into();
        candidate.inference_allocation.allocator_revision = "anytime-v1".into();
        candidate.inference_allocation.additional_rollouts = 250;
        candidate.inference_allocation.rollout_compute_units_used = Some(8_000);
        candidate.inference_allocation.predicted_regret_reduction = Some(20.0);
        candidate.inference_allocation.observed_regret_reduction = Some(12.5);
        let comparison = compare_metric(
            &baseline,
            &candidate,
            "driving_score",
            MetricDirection::HigherIsBetter,
        )
        .unwrap();
        assert_eq!(comparison.class, ComparisonClass::MatchedCapability);
        assert_eq!(
            comparison.candidate_to_baseline_rollout_compute_units_ratio,
            Some(0.8)
        );
    }

    #[test]
    fn matched_comparison_requires_equal_data_and_hardware_budgets() {
        let baseline = card("baseline", 90.0, Some(4.0), Some(20.0), Some(1_000));
        let candidate = card("candidate", 91.0, Some(2.0), Some(18.0), Some(900));
        let comparison = compare_metric(
            &baseline,
            &candidate,
            "driving_score",
            MetricDirection::HigherIsBetter,
        )
        .unwrap();
        assert_eq!(comparison.class, ComparisonClass::MatchedCapability);
        assert_eq!(comparison.candidate_minus_baseline, 1.0);
        assert_eq!(
            comparison.candidate_to_baseline_inference_energy_ratio,
            Some(0.5)
        );
    }

    #[test]
    fn different_training_exposure_is_frontier_only_not_matched() {
        let baseline = card("baseline", 90.0, Some(4.0), Some(20.0), Some(1_000));
        let mut candidate = card("candidate", 92.0, Some(2.0), Some(18.0), Some(900));
        candidate.data.simulator_transitions += 10_000;
        let comparison = compare_metric(
            &baseline,
            &candidate,
            "driving_score",
            MetricDirection::HigherIsBetter,
        )
        .unwrap();
        assert_eq!(comparison.class, ComparisonClass::CapabilityFrontierOnly);
        assert!(comparison
            .budget_differences
            .contains(&"simulator_transitions".to_string()));
    }

    #[test]
    fn missing_inference_resource_measurements_prevent_matched_label() {
        let mut baseline = card("baseline", 90.0, None, None, None);
        let mut candidate = card("candidate", 91.0, None, None, None);
        baseline.compute.deadline_miss_rate = None;
        candidate.compute.deadline_miss_rate = None;
        let comparison = compare_metric(
            &baseline,
            &candidate,
            "driving_score",
            MetricDirection::HigherIsBetter,
        ).unwrap();
        assert_eq!(comparison.class, ComparisonClass::CapabilityFrontierOnly);
        for expected in [
            "inference_energy_unreported",
            "inference_p99_latency_unreported",
            "peak_memory_unreported",
            "deadline_miss_rate_unreported",
        ] {
            assert!(comparison.budget_differences.iter().any(|item| item == expected));
        }
    }

    #[test]
    fn hidden_privilege_mismatch_blocks_direct_comparison() {
        let baseline = card("baseline", 90.0, Some(4.0), Some(20.0), Some(1_000));
        let mut candidate = card("candidate", 92.0, Some(2.0), Some(18.0), Some(900));
        candidate.model.input_privileges.push("simulator-truth".into());
        let error = compare_metric(
            &baseline,
            &candidate,
            "driving_score",
            MetricDirection::HigherIsBetter,
        )
        .unwrap_err();
        assert!(error.contains("input_privileges"));
    }

    #[test]
    fn benchmark_variant_mismatch_blocks_direct_comparison() {
        let baseline = card("baseline", 90.0, Some(4.0), Some(20.0), Some(1_000));
        let mut candidate = card("candidate", 92.0, Some(2.0), Some(18.0), Some(900));
        candidate.benchmark_id = "navsim-v2".into();
        let error = compare_metric(
            &baseline,
            &candidate,
            "driving_score",
            MetricDirection::HigherIsBetter,
        )
        .unwrap_err();
        assert!(error.contains("benchmark_id"));
    }

    #[test]
    fn missing_metric_is_not_treated_as_zero() {
        let mut result = card("a", 90.0, Some(2.0), Some(20.0), Some(1_000));
        result.outcomes.native_metrics.insert("driving_score".into(), None);
        assert!(compare_metric(
            &result,
            &card("b", 91.0, Some(1.0), Some(18.0), Some(900)),
            "driving_score",
            MetricDirection::HigherIsBetter,
        )
        .unwrap_err()
        .contains("missing or null"));
    }

    #[test]
    fn pareto_frontier_preserves_tradeoffs_without_weighted_score() {
        let score_first = card("score-first", 95.0, Some(5.0), Some(30.0), Some(2_000));
        let efficient = card("efficient", 92.0, Some(2.0), Some(18.0), Some(1_200));
        let dominated = card("dominated", 90.0, Some(6.0), Some(40.0), Some(2_400));
        let frontier = pareto_frontier(
            &[score_first, efficient, dominated],
            "driving_score",
            MetricDirection::HigherIsBetter,
        )
        .unwrap();
        assert_eq!(frontier.frontier_run_ids, vec!["efficient", "score-first"]);
        assert!(frontier.incomplete_run_ids.is_empty());
    }

    #[test]
    fn incomplete_resource_measurement_is_excluded_from_frontier() {
        let complete = card("complete", 90.0, Some(2.0), Some(20.0), Some(1_000));
        let incomplete = card("incomplete", 99.0, None, Some(20.0), Some(500));
        let frontier = pareto_frontier(
            &[complete, incomplete],
            "driving_score",
            MetricDirection::HigherIsBetter,
        )
        .unwrap();
        assert_eq!(frontier.frontier_run_ids, vec!["complete"]);
        assert_eq!(frontier.incomplete_run_ids, vec!["incomplete"]);
    }

    #[test]
    fn pareto_frontier_refuses_to_pool_different_protocols() {
        let baseline = card("a", 90.0, Some(2.0), Some(20.0), Some(1_000));
        let mut other = card("b", 91.0, Some(1.0), Some(18.0), Some(900));
        other.protocol_revision = "different-evaluator".into();
        assert!(pareto_frontier(
            &[baseline, other],
            "driving_score",
            MetricDirection::HigherIsBetter,
        )
        .unwrap_err()
        .contains("protocol_revision"));
    }

    #[test]
    fn documented_not_run_template_is_parseable_and_valid() {
        let template = include_str!("../../result_card.template.json");
        let card = ResultCard::from_json(template).unwrap();
        assert_eq!(card.status, RunStatus::NotRun);
        assert!(card.validate().is_ok());
    }

    #[test]
    fn duplicate_run_ids_are_rejected_by_frontier() {
        let first = card("same-run", 90.0, Some(2.0), Some(20.0), Some(1_000));
        let second = card("same-run", 91.0, Some(1.0), Some(18.0), Some(900));
        assert!(pareto_frontier(
            &[first, second],
            "driving_score",
            MetricDirection::HigherIsBetter,
        )
        .unwrap_err()
        .contains("duplicate run_id"));
    }

    #[test]
    fn manifest_and_json_schemas_are_parseable() {
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../../benchmark_manifest_v0_1.json")).unwrap();
        let manifest_schema: serde_json::Value =
            serde_json::from_str(include_str!("../../benchmark_manifest_v0_1.schema.json")).unwrap();
        let card_schema: serde_json::Value =
            serde_json::from_str(include_str!("../../result_card.schema.json")).unwrap();
        assert_eq!(manifest["status"], "specification_only_no_model_runs");
        assert_eq!(manifest["prospective_targets"]["status"], "not_measured");
        assert_eq!(manifest_schema["$schema"], "https://json-schema.org/draft/2020-12/schema");
        assert_eq!(card_schema["$schema"], "https://json-schema.org/draft/2020-12/schema");
    }

    #[test]
    fn paired_cluster_bootstrap_detects_constant_two_point_gain() {
        let baseline = card("baseline", 90.0, Some(4.0), Some(20.0), Some(1_000));
        let mut candidate = card("candidate", 92.0, Some(2.0), Some(18.0), Some(900));
        for episode in &mut candidate.outcomes.episode_outcomes {
            episode.native_metrics.insert("driving_score".into(), Some(92.0));
        }
        let result = paired_stratified_cluster_bootstrap(
            &baseline,
            &candidate,
            "driving_score",
            MetricDirection::HigherIsBetter,
            2_000,
            0x1234_5678,
        )
        .unwrap();
        assert_eq!(result.paired_episode_count, 100);
        assert_eq!(result.scenario_cluster_count, 20);
        assert_eq!(result.scenario_family_count, 1);
        assert_eq!(result.point_estimate_candidate_minus_baseline, 2.0);
        assert_eq!(result.confidence_interval_lower, 2.0);
        assert_eq!(result.confidence_interval_upper, 2.0);
        assert!(result.interval_supports_candidate_advantage);
    }

    #[test]
    fn paired_bootstrap_is_deterministic_for_fixed_manifest_seed() {
        let baseline = card("baseline", 90.0, Some(4.0), Some(20.0), Some(1_000));
        let mut candidate = card("candidate", 91.0, Some(2.0), Some(18.0), Some(900));
        for (index, episode) in candidate.outcomes.episode_outcomes.iter_mut().enumerate() {
            let offset = (index % 7) as f64 * 0.1;
            episode.native_metrics.insert("driving_score".into(), Some(91.0 + offset));
        }
        let first = paired_stratified_cluster_bootstrap(
            &baseline, &candidate, "driving_score",
            MetricDirection::HigherIsBetter, 2_000, 77,
        ).unwrap();
        let second = paired_stratified_cluster_bootstrap(
            &baseline, &candidate, "driving_score",
            MetricDirection::HigherIsBetter, 2_000, 77,
        ).unwrap();
        assert_eq!(first.confidence_interval_lower, second.confidence_interval_lower);
        assert_eq!(first.confidence_interval_upper, second.confidence_interval_upper);
        assert_eq!(first.point_estimate_candidate_minus_baseline, second.point_estimate_candidate_minus_baseline);
    }

    #[test]
    fn paired_bootstrap_refuses_unmatched_scenario_clusters() {
        let baseline = card("baseline", 90.0, Some(4.0), Some(20.0), Some(1_000));
        let mut candidate = card("candidate", 91.0, Some(2.0), Some(18.0), Some(900));
        candidate.outcomes.episode_outcomes[0].scenario_cluster_id = "different-cluster".into();
        assert!(paired_stratified_cluster_bootstrap(
            &baseline, &candidate, "driving_score",
            MetricDirection::HigherIsBetter, 1_000, 1,
        ).unwrap_err().contains("scenario cluster mismatch"));
    }

    #[test]
    fn paired_bootstrap_refuses_missing_episode_metric_instead_of_dropping_it() {
        let baseline = card("baseline", 90.0, Some(4.0), Some(20.0), Some(1_000));
        let mut candidate = card("candidate", 91.0, Some(2.0), Some(18.0), Some(900));
        candidate.outcomes.episode_outcomes[3].native_metrics.insert("driving_score".into(), None);
        assert!(paired_stratified_cluster_bootstrap(
            &baseline, &candidate, "driving_score",
            MetricDirection::HigherIsBetter, 1_000, 1,
        ).unwrap_err().contains("lacks metric"));
    }

    #[test]
    fn paired_bootstrap_does_not_call_zero_difference_an_advantage() {
        let baseline = card("baseline", 90.0, Some(4.0), Some(20.0), Some(1_000));
        let candidate = card("candidate", 90.0, Some(2.0), Some(18.0), Some(900));
        let result = paired_stratified_cluster_bootstrap(
            &baseline, &candidate, "driving_score",
            MetricDirection::HigherIsBetter, 1_000, 9,
        ).unwrap();
        assert_eq!(result.confidence_interval_lower, 0.0);
        assert_eq!(result.confidence_interval_upper, 0.0);
        assert!(!result.interval_supports_candidate_advantage);
    }

    #[test]
    fn missing_training_compute_prevents_matched_capability_label() {
        let mut baseline = card("baseline", 90.0, Some(4.0), Some(20.0), Some(1_000));
        let mut candidate = card("candidate", 91.0, Some(2.0), Some(18.0), Some(900));
        baseline.compute.training_gpu_hours = None;
        candidate.compute.training_gpu_hours = None;
        baseline.compute.training_energy_kwh = None;
        candidate.compute.training_energy_kwh = None;
        let comparison = compare_metric(
            &baseline, &candidate, "driving_score", MetricDirection::HigherIsBetter,
        ).unwrap();
        assert_eq!(comparison.class, ComparisonClass::CapabilityFrontierOnly);
        assert!(comparison.budget_differences.contains(
            &"training_gpu_hours_unmatched_or_unreported".to_string()
        ));
        assert!(comparison.budget_differences.contains(
            &"training_energy_kwh_unmatched_or_unreported".to_string()
        ));
    }

    #[test]
    fn point_estimate_margin_respects_metric_direction() {
        assert!(point_estimate_within_margin(
            95.0,
            93.7,
            1.5,
            MetricDirection::HigherIsBetter
        ));
        assert!(!point_estimate_within_margin(
            95.0,
            93.4,
            1.5,
            MetricDirection::HigherIsBetter
        ));
        assert!(point_estimate_within_margin(
            10.0,
            11.2,
            1.5,
            MetricDirection::LowerIsBetter
        ));
    }
}
