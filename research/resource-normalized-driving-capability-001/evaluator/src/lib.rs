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

        if self.schema_version != "0.1.0" {
            errors.push(format!(
                "schema_version must be 0.1.0, got {:?}",
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
        ) {
            if p50 > p95 {
                errors.push("inference p50 latency exceeds p95".into());
            }
        }
        if let (Some(p95), Some(p99)) = (
            self.compute.inference_p95_ms,
            self.compute.inference_p99_ms,
        ) {
            if p95 > p99 {
                errors.push("inference p95 latency exceeds p99".into());
            }
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

        if self.status == RunStatus::NotRun {
            if self.evidence.expected_episode_count != 0
                || self.evidence.observed_episode_count != 0
                || self.outcomes.completed_episodes != 0
                || self.outcomes.failed_episodes != 0
                || self.outcomes.incomplete_episodes != 0
                || self.outcomes.censored_episodes != 0
                || self.outcomes.native_metrics.values().any(Option::is_some)
            {
                errors.push("not_run cards must not contain episode outcomes or metric values".into());
            }
        }
        if self.status == RunStatus::Complete {
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
    inference_energy: f64,
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
            card.compute.inference_energy_joules_per_frame,
            card.compute.inference_p99_ms,
            card.compute.peak_memory_bytes,
        ) {
            (Some(primary_metric), Some(inference_energy), Some(p99_latency), Some(peak_memory)) => {
                Some(FrontierPoint {
                    primary_metric,
                    inference_energy,
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
    let energy_no_worse = candidate.inference_energy <= other.inference_energy;
    let latency_no_worse = candidate.p99_latency <= other.p99_latency;
    let memory_no_worse = candidate.peak_memory <= other.peak_memory;
    let strictly_better = primary_better
        || candidate.inference_energy < other.inference_energy
        || candidate.p99_latency < other.p99_latency
        || candidate.peak_memory < other.peak_memory;

    primary_no_worse
        && energy_no_worse
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
            schema_version: "0.1.0".into(),
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
        let incomplete = card("incomplete", 99.0, None, Some(10.0), Some(500));
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
        let template = include_str!("../result_card.template.json");
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
