// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Real-to-sim fidelity scorecard reference.
//!
//! This crate compares timestamp-aligned simulation traces against physical
//! observations on independent, recording-level blind holdouts. It deliberately
//! keeps sensor channels separate and reports task-stage scores separately from
//! end-to-end binary success. It does not validate the provenance or accuracy
//! of physical sensors; those are upstream protocol obligations.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{Display, Formatter};

pub const MIN_BLIND_HOLDOUT_TRIALS: usize = 10;
pub const MIN_BLIND_TASK_CLUSTERS: usize = 10;
pub const MIN_BOOTSTRAP_REPLICATES: usize = 1_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Digest(pub [u8; 32]);

impl Digest {
    pub fn to_hex(self) -> String {
        let mut output = String::with_capacity(64);
        for byte in self.0 {
            use std::fmt::Write as _;
            let _ = write!(output, "{byte:02x}");
        }
        output
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SignalKind {
    JointPositionRad,
    JointVelocityRadPerSec,
    JointTorqueNm,
    CurrentDerivedTorqueNm,
    EndEffectorPositionM,
    ObjectPositionM,
    ObjectOrientationRad,
    ActuatorTemperatureC,
    ContactEventTimeSec,
    ContactNormalImpulseNs,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FidelityError {
    EmptySimulatorCommit,
    InvalidSimulatorCommit,
    EmptySimulatorConfigurationDigest,
    InvalidSimulatorConfigurationDigest,
    EmptyRobotArticleId,
    EmptyAssetRevision,
    EmptyCommandTraceDigest,
    InvalidCommandTraceDigest,
    EmptyCalibrationDigest,
    InvalidCalibrationDigest,
    EmptyRecordingId,
    DuplicateRecordingId,
    RecordingSplitLeakage,
    InsufficientBlindHoldoutTrials,
    EmptyTrialId,
    EmptyTaskId,
    InsufficientBlindTaskClusters,
    DuplicateTrialId,
    EmptyTraceSet,
    DuplicateSignalSeries,
    InconsistentSignalSeries,
    EmptySeriesId,
    InsufficientSamples,
    NonMonotonicTimestamp,
    TimestampMismatch,
    NonFiniteSignal,
    InvalidNormalizationScale,
    InvalidTaskStageScore,
    InvalidBootstrapReplicateCount,
}

impl Display for FidelityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySimulatorCommit => write!(formatter, "simulator commit must not be empty"),
            Self::InvalidSimulatorCommit => write!(formatter, "simulator commit must be a 40-character lowercase hex SHA"),
            Self::EmptySimulatorConfigurationDigest => write!(formatter, "simulator configuration digest must not be empty"),
            Self::InvalidSimulatorConfigurationDigest => write!(formatter, "simulator configuration digest must be a 64-character lowercase hex SHA-256"),
            Self::EmptyRobotArticleId => write!(formatter, "robot/article ID must not be empty"),
            Self::EmptyAssetRevision => write!(formatter, "asset revision must not be empty"),
            Self::EmptyCommandTraceDigest => write!(formatter, "command-trace digest must not be empty"),
            Self::InvalidCommandTraceDigest => write!(formatter, "command-trace digest must be a 64-character lowercase hex SHA-256"),
            Self::EmptyCalibrationDigest => write!(formatter, "sensor-calibration digest must not be empty"),
            Self::InvalidCalibrationDigest => write!(formatter, "sensor-calibration digest must be a 64-character lowercase hex SHA-256"),
            Self::EmptyRecordingId => write!(formatter, "recording ID must not be empty"),
            Self::DuplicateRecordingId => write!(formatter, "recording IDs must be unique within each split"),
            Self::RecordingSplitLeakage => write!(formatter, "a physical recording appears in more than one split"),
            Self::InsufficientBlindHoldoutTrials => write!(formatter, "at least ten independent blind physical recordings are required"),
            Self::EmptyTrialId => write!(formatter, "trial ID must not be empty"),
            Self::EmptyTaskId => write!(formatter, "task ID must not be empty"),
            Self::InsufficientBlindTaskClusters => write!(formatter, "at least ten independent task clusters are required"),
            Self::DuplicateTrialId => write!(formatter, "trial IDs must be unique"),
            Self::EmptyTraceSet => write!(formatter, "each blind trial must contain at least one measured signal series"),
            Self::DuplicateSignalSeries => write!(formatter, "signal series IDs must be unique within a trial"),
            Self::InconsistentSignalSeries => write!(formatter, "all blind trials must contain the same declared signal series"),
            Self::EmptySeriesId => write!(formatter, "signal series ID must not be empty"),
            Self::InsufficientSamples => write!(formatter, "each signal series requires at least two paired samples"),
            Self::NonMonotonicTimestamp => write!(formatter, "timestamps must be strictly increasing"),
            Self::TimestampMismatch => write!(formatter, "real and simulated samples must share exact timestamps"),
            Self::NonFiniteSignal => write!(formatter, "all real and simulated signal values must be finite"),
            Self::InvalidNormalizationScale => write!(formatter, "normalization scale must be finite and positive"),
            Self::InvalidTaskStageScore => write!(formatter, "task-stage scores must be finite and in [0, 1]"),
            Self::InvalidBootstrapReplicateCount => write!(formatter, "at least 1000 bootstrap replicates are required"),
        }
    }
}

impl Error for FidelityError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolIdentity {
    pub simulator_commit: String,
    /// Frozen digest of solver, integrator, timestep, substeps, contact model,
    /// controller, filters and all runtime parameters.
    pub simulator_configuration_digest: String,
    pub robot_article_id: String,
    pub asset_revision: String,
    pub command_trace_digest: String,
    pub sensor_calibration_digest: String,
}

fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

impl ProtocolIdentity {
    pub fn validate(&self) -> Result<(), FidelityError> {
        if self.simulator_commit.trim().is_empty() {
            return Err(FidelityError::EmptySimulatorCommit);
        }
        if self.simulator_commit.len() != 40
            || !self
                .simulator_commit
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(FidelityError::InvalidSimulatorCommit);
        }
        if self.simulator_configuration_digest.trim().is_empty() {
            return Err(FidelityError::EmptySimulatorConfigurationDigest);
        }
        if !is_lower_hex_sha256(&self.simulator_configuration_digest) {
            return Err(FidelityError::InvalidSimulatorConfigurationDigest);
        }
        if self.robot_article_id.trim().is_empty() {
            return Err(FidelityError::EmptyRobotArticleId);
        }
        if self.asset_revision.trim().is_empty() {
            return Err(FidelityError::EmptyAssetRevision);
        }
        if self.command_trace_digest.trim().is_empty() {
            return Err(FidelityError::EmptyCommandTraceDigest);
        }
        if !is_lower_hex_sha256(&self.command_trace_digest) {
            return Err(FidelityError::InvalidCommandTraceDigest);
        }
        if self.sensor_calibration_digest.trim().is_empty() {
            return Err(FidelityError::EmptyCalibrationDigest);
        }
        if !is_lower_hex_sha256(&self.sensor_calibration_digest) {
            return Err(FidelityError::InvalidCalibrationDigest);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PairedSample {
    pub real_timestamp_ns: u64,
    pub simulated_timestamp_ns: u64,
    pub real_value: f64,
    pub simulated_value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TraceSeries {
    pub signal: SignalKind,
    /// Stable series key, e.g. `left_arm_j1` or `object_03_x`.
    pub series_id: String,
    /// Predeclared physical normalization scale, typically a sensor/task tolerance
    /// or measured reference range. It must not be fitted on blind holdouts.
    pub normalization_scale: f64,
    pub samples: Vec<PairedSample>,
}

impl TraceSeries {
    pub fn validate(&self) -> Result<(), FidelityError> {
        if self.series_id.trim().is_empty() {
            return Err(FidelityError::EmptySeriesId);
        }
        if !self.normalization_scale.is_finite() || self.normalization_scale <= 0.0 {
            return Err(FidelityError::InvalidNormalizationScale);
        }
        if self.samples.len() < 2 {
            return Err(FidelityError::InsufficientSamples);
        }
        for sample in &self.samples {
            if sample.real_timestamp_ns != sample.simulated_timestamp_ns {
                return Err(FidelityError::TimestampMismatch);
            }
        }
        for pair in self.samples.windows(2) {
            if pair[1].real_timestamp_ns <= pair[0].real_timestamp_ns
                || pair[1].simulated_timestamp_ns <= pair[0].simulated_timestamp_ns
            {
                return Err(FidelityError::NonMonotonicTimestamp);
            }
        }
        if self.samples.iter().any(|sample| {
            !sample.real_value.is_finite() || !sample.simulated_value.is_finite()
        }) {
            return Err(FidelityError::NonFiniteSignal);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TaskOutcomePair {
    /// Fraction of predeclared task-stage criteria satisfied, [0, 1].
    pub real_stage_score: f64,
    pub simulated_stage_score: f64,
    /// Final task predicate; distinct from the stage-average score.
    pub real_end_to_end_success: bool,
    pub simulated_end_to_end_success: bool,
}

impl TaskOutcomePair {
    fn validate(self) -> Result<(), FidelityError> {
        if !self.real_stage_score.is_finite()
            || !self.simulated_stage_score.is_finite()
            || !(0.0..=1.0).contains(&self.real_stage_score)
            || !(0.0..=1.0).contains(&self.simulated_stage_score)
        {
            return Err(FidelityError::InvalidTaskStageScore);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlindHoldoutTrial {
    pub trial_id: String,
    /// Semantic task/activity ID; independent tasks are the outer cluster.
    pub task_id: String,
    /// Independent physical recording ID nested within the task cluster.
    pub source_recording_id: String,
    pub traces: Vec<TraceSeries>,
    pub task_outcome: TaskOutcomePair,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationManifest {
    pub protocol: ProtocolIdentity,
    /// Complete source-recording IDs, never frame/window IDs.
    pub training_recording_ids: Vec<String>,
    pub calibration_recording_ids: Vec<String>,
    /// Each item must originate from a unique, independently acquired physical recording.
    pub blind_holdout_trials: Vec<BlindHoldoutTrial>,
}

impl EvaluationManifest {
    pub fn validate(&self) -> Result<(), FidelityError> {
        self.protocol.validate()?;
        if self.blind_holdout_trials.len() < MIN_BLIND_HOLDOUT_TRIALS {
            return Err(FidelityError::InsufficientBlindHoldoutTrials);
        }

        let mut all_recordings = BTreeSet::new();
        for recording_id in self
            .training_recording_ids
            .iter()
            .chain(self.calibration_recording_ids.iter())
        {
            if recording_id.trim().is_empty() {
                return Err(FidelityError::EmptyRecordingId);
            }
            if !all_recordings.insert(recording_id.as_str()) {
                return Err(FidelityError::DuplicateRecordingId);
            }
        }

        let mut trial_ids = BTreeSet::new();
        let mut task_ids = BTreeSet::new();
        let mut expected_series: Option<BTreeMap<(SignalKind, String), u64>> = None;
        for trial in &self.blind_holdout_trials {
            if trial.trial_id.trim().is_empty() {
                return Err(FidelityError::EmptyTrialId);
            }
            if !trial_ids.insert(trial.trial_id.as_str()) {
                return Err(FidelityError::DuplicateTrialId);
            }
            if trial.task_id.trim().is_empty() {
                return Err(FidelityError::EmptyTaskId);
            }
            task_ids.insert(trial.task_id.as_str());
            if trial.source_recording_id.trim().is_empty() {
                return Err(FidelityError::EmptyRecordingId);
            }
            if !all_recordings.insert(trial.source_recording_id.as_str()) {
                return Err(FidelityError::RecordingSplitLeakage);
            }
            trial.task_outcome.validate()?;
            if trial.traces.is_empty() {
                return Err(FidelityError::EmptyTraceSet);
            }

            let mut series_keys = BTreeMap::new();
            for trace in &trial.traces {
                trace.validate()?;
                if series_keys
                    .insert(
                        (trace.signal, trace.series_id.clone()),
                        trace.normalization_scale.to_bits(),
                    )
                    .is_some()
                {
                    return Err(FidelityError::DuplicateSignalSeries);
                }
            }
            if expected_series
                .as_ref()
                .is_some_and(|expected| expected != &series_keys)
            {
                return Err(FidelityError::InconsistentSignalSeries);
            }
            if expected_series.is_none() {
                expected_series = Some(series_keys);
            }
        }
        if task_ids.len() < MIN_BLIND_TASK_CLUSTERS {
            return Err(FidelityError::InsufficientBlindTaskClusters);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TraceMetrics {
    pub sample_count: usize,
    pub duration_ns: u64,
    pub mean_absolute_error: f64,
    pub root_mean_square_error: f64,
    pub signed_bias: f64,
    pub p95_absolute_error: f64,
    pub maximum_absolute_error: f64,
    pub normalized_root_mean_square_error: f64,
}

pub fn score_trace(trace: &TraceSeries) -> Result<TraceMetrics, FidelityError> {
    trace.validate()?;
    let mut absolute_errors = Vec::with_capacity(trace.samples.len());
    let mut squared_sum = 0.0;
    let mut signed_sum = 0.0;
    let mut absolute_sum = 0.0;
    let mut maximum_absolute_error: f64 = 0.0;

    for sample in &trace.samples {
        let error = sample.simulated_value - sample.real_value;
        let absolute = error.abs();
        absolute_errors.push(absolute);
        squared_sum += error * error;
        signed_sum += error;
        absolute_sum += absolute;
        maximum_absolute_error = maximum_absolute_error.max(absolute);
    }

    absolute_errors.sort_by(f64::total_cmp);
    let count = trace.samples.len();
    let mean_absolute_error = absolute_sum / count as f64;
    let root_mean_square_error = (squared_sum / count as f64).sqrt();
    let signed_bias = signed_sum / count as f64;
    let p95_absolute_error = percentile(&absolute_errors, 0.95);
    let duration_ns = trace
        .samples
        .last()
        .expect("validated samples")
        .real_timestamp_ns
        - trace
            .samples
            .first()
            .expect("validated samples")
            .real_timestamp_ns;

    Ok(TraceMetrics {
        sample_count: count,
        duration_ns,
        mean_absolute_error,
        root_mean_square_error,
        signed_bias,
        p95_absolute_error,
        maximum_absolute_error,
        normalized_root_mean_square_error: root_mean_square_error
            / trace.normalization_scale,
    })
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
            seed: 0x5349_4D32_5245_414C,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConfidenceInterval {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChannelSummary {
    pub signal: SignalKind,
    pub series_id: String,
    pub physical_trial_count: usize,
    pub independent_task_clusters: usize,
    /// Equal-weight mean across task clusters; repetitions are averaged within each task.
    pub mean_task_mae: f64,
    pub mean_task_rmse: f64,
    pub mean_task_bias: f64,
    pub mean_task_p95_absolute_error: f64,
    pub worst_trial_rmse: f64,
    /// Bootstrap interval over task-cluster mean RMSE, not correlated time samples.
    pub mean_task_rmse_95: ConfidenceInterval,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TaskAgreement {
    pub independent_trials: usize,
    pub independent_task_clusters: usize,
    pub real_mean_stage_score: f64,
    pub simulated_mean_stage_score: f64,
    /// Simulated minus real stage score.
    pub mean_signed_stage_gap: f64,
    pub stage_score_mae: f64,
    pub stage_score_rmse: f64,
    /// Pearson correlation across equal-weight task-cluster means; None if variance is zero.
    pub stage_score_pearson_r: Option<f64>,
    pub real_end_to_end_success_rate: f64,
    pub simulated_end_to_end_success_rate: f64,
    pub end_to_end_outcome_agreement_rate: f64,
    /// Simulation success while physical trial failed.
    pub false_safe_count: usize,
    /// Simulation failure while physical trial succeeded.
    pub false_failure_count: usize,
    /// Bootstrap interval for mean (simulated stage score - real stage score).
    pub mean_signed_stage_gap_95: ConfidenceInterval,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FidelityReport {
    pub protocol: ProtocolIdentity,
    pub independent_blind_trials: usize,
    pub channel_summaries: Vec<ChannelSummary>,
    pub task_agreement: TaskAgreement,
}

pub fn score_evaluation(
    manifest: &EvaluationManifest,
    bootstrap: BootstrapConfig,
) -> Result<FidelityReport, FidelityError> {
    manifest.validate()?;
    if bootstrap.replicates < MIN_BOOTSTRAP_REPLICATES {
        return Err(FidelityError::InvalidBootstrapReplicateCount);
    }

    let first = &manifest.blind_holdout_trials[0];
    let series_keys: Vec<(SignalKind, String)> = first
        .traces
        .iter()
        .map(|trace| (trace.signal, trace.series_id.clone()))
        .collect();

    let mut channel_summaries = Vec::with_capacity(series_keys.len());
    for (signal, series_id) in series_keys {
        let mut metrics_by_task: BTreeMap<String, Vec<TraceMetrics>> = BTreeMap::new();
        let mut worst_trial_rmse: f64 = 0.0;
        let mut physical_trial_count = 0_usize;
        for trial in &manifest.blind_holdout_trials {
            let trace = trial
                .traces
                .iter()
                .find(|trace| trace.signal == signal && trace.series_id == series_id)
                .expect("manifest validates identical series layout");
            let metrics = score_trace(trace)?;
            worst_trial_rmse = worst_trial_rmse.max(metrics.root_mean_square_error);
            physical_trial_count += 1;
            metrics_by_task
                .entry(trial.task_id.clone())
                .or_default()
                .push(metrics);
        }
        let task_mae: Vec<f64> = metrics_by_task
            .values()
            .map(|values| mean(values.iter().map(|metric| metric.mean_absolute_error)))
            .collect();
        let task_rmse: Vec<f64> = metrics_by_task
            .values()
            .map(|values| mean(values.iter().map(|metric| metric.root_mean_square_error)))
            .collect();
        let task_bias: Vec<f64> = metrics_by_task
            .values()
            .map(|values| mean(values.iter().map(|metric| metric.signed_bias)))
            .collect();
        let task_p95: Vec<f64> = metrics_by_task
            .values()
            .map(|values| mean(values.iter().map(|metric| metric.p95_absolute_error)))
            .collect();
        channel_summaries.push(ChannelSummary {
            signal,
            series_id,
            physical_trial_count,
            independent_task_clusters: metrics_by_task.len(),
            mean_task_mae: mean(task_mae.iter().copied()),
            mean_task_rmse: mean(task_rmse.iter().copied()),
            mean_task_bias: mean(task_bias.iter().copied()),
            mean_task_p95_absolute_error: mean(task_p95.iter().copied()),
            worst_trial_rmse,
            mean_task_rmse_95: bootstrap_mean_interval(
                &task_rmse,
                bootstrap,
            ),
        });
    }

    let task_agreement =
        score_task_agreement(&manifest.blind_holdout_trials, bootstrap);
    Ok(FidelityReport {
        protocol: manifest.protocol.clone(),
        independent_blind_trials: manifest.blind_holdout_trials.len(),
        channel_summaries,
        task_agreement,
    })
}

fn score_task_agreement(
    trials: &[BlindHoldoutTrial],
    bootstrap: BootstrapConfig,
) -> TaskAgreement {
    let count = trials.len();
    let mut tasks: BTreeMap<String, Vec<&BlindHoldoutTrial>> = BTreeMap::new();
    for trial in trials {
        tasks.entry(trial.task_id.clone()).or_default().push(trial);
    }
    let real: Vec<f64> = tasks
        .values()
        .map(|task_trials| {
            mean(task_trials.iter().map(|trial| trial.task_outcome.real_stage_score))
        })
        .collect();
    let simulated: Vec<f64> = tasks
        .values()
        .map(|task_trials| {
            mean(
                task_trials
                    .iter()
                    .map(|trial| trial.task_outcome.simulated_stage_score),
            )
        })
        .collect();
    let deltas: Vec<f64> = simulated
        .iter()
        .zip(&real)
        .map(|(simulated, real)| simulated - real)
        .collect();
    let absolute: Vec<f64> = deltas.iter().map(|delta| delta.abs()).collect();
    let squared: Vec<f64> = deltas.iter().map(|delta| delta * delta).collect();
    let real_successes = trials
        .iter()
        .filter(|trial| trial.task_outcome.real_end_to_end_success)
        .count();
    let simulated_successes = trials
        .iter()
        .filter(|trial| trial.task_outcome.simulated_end_to_end_success)
        .count();
    let agreements = trials
        .iter()
        .filter(|trial| {
            trial.task_outcome.real_end_to_end_success
                == trial.task_outcome.simulated_end_to_end_success
        })
        .count();
    let false_safe_count = trials
        .iter()
        .filter(|trial| {
            trial.task_outcome.simulated_end_to_end_success
                && !trial.task_outcome.real_end_to_end_success
        })
        .count();
    let false_failure_count = trials
        .iter()
        .filter(|trial| {
            !trial.task_outcome.simulated_end_to_end_success
                && trial.task_outcome.real_end_to_end_success
        })
        .count();

    TaskAgreement {
        independent_trials: count,
        independent_task_clusters: tasks.len(),
        real_mean_stage_score: mean(real.iter().copied()),
        simulated_mean_stage_score: mean(simulated.iter().copied()),
        mean_signed_stage_gap: mean(deltas.iter().copied()),
        stage_score_mae: mean(absolute),
        stage_score_rmse: mean(squared).sqrt(),
        stage_score_pearson_r: pearson_correlation(&real, &simulated),
        real_end_to_end_success_rate: real_successes as f64 / count as f64,
        simulated_end_to_end_success_rate: simulated_successes as f64 / count as f64,
        end_to_end_outcome_agreement_rate: agreements as f64 / count as f64,
        false_safe_count,
        false_failure_count,
        mean_signed_stage_gap_95: bootstrap_mean_interval(&deltas, bootstrap),
    }
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let mut count = 0_usize;
    let mut sum = 0.0;
    for value in values {
        count += 1;
        sum += value;
    }
    sum / count as f64
}

fn pearson_correlation(left: &[f64], right: &[f64]) -> Option<f64> {
    if left.len() != right.len() || left.len() < 2 {
        return None;
    }
    let left_mean = mean(left.iter().copied());
    let right_mean = mean(right.iter().copied());
    let mut covariance = 0.0;
    let mut left_variance = 0.0;
    let mut right_variance = 0.0;
    for (left, right) in left.iter().zip(right) {
        let dx = left - left_mean;
        let dy = right - right_mean;
        covariance += dx * dy;
        left_variance += dx * dx;
        right_variance += dy * dy;
    }
    if left_variance <= f64::EPSILON || right_variance <= f64::EPSILON {
        None
    } else {
        Some(covariance / (left_variance * right_variance).sqrt())
    }
}

fn bootstrap_mean_interval(
    values: &[f64],
    config: BootstrapConfig,
) -> ConfidenceInterval {
    let mut random = SplitMix64::new(config.seed);
    let mut bootstrap_means = Vec::with_capacity(config.replicates);
    for _ in 0..config.replicates {
        let mut sum = 0.0;
        for _ in 0..values.len() {
            let selected = (random.next_u64() % values.len() as u64) as usize;
            sum += values[selected];
        }
        bootstrap_means.push(sum / values.len() as f64);
    }
    bootstrap_means.sort_by(f64::total_cmp);
    ConfidenceInterval {
        lower: percentile(&bootstrap_means, 0.025),
        upper: percentile(&bootstrap_means, 0.975),
    }
}

fn percentile(sorted: &[f64], probability: f64) -> f64 {
    debug_assert!(!sorted.is_empty());
    let index = probability.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;
    let fraction = index - lower as f64;
    sorted[lower] * (1.0 - fraction) + sorted[upper] * fraction
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

    fn protocol() -> ProtocolIdentity {
        ProtocolIdentity {
            simulator_commit: "12".repeat(20),
            simulator_configuration_digest: "ab".repeat(32),
            robot_article_id: "robot-article-01".to_owned(),
            asset_revision: "asset-revision-a".to_owned(),
            command_trace_digest: "cd".repeat(32),
            sensor_calibration_digest: "ef".repeat(32),
        }
    }

    fn trace(signal: SignalKind, series_id: &str, real: &[f64], simulated: &[f64]) -> TraceSeries {
        assert_eq!(real.len(), simulated.len());
        TraceSeries {
            signal,
            series_id: series_id.to_owned(),
            normalization_scale: 1.0,
            samples: real
                .iter()
                .zip(simulated)
                .enumerate()
                .map(|(index, (real, simulated))| PairedSample {
                    real_timestamp_ns: index as u64 * 1_000_000,
                    simulated_timestamp_ns: index as u64 * 1_000_000,
                    real_value: *real,
                    simulated_value: *simulated,
                })
                .collect(),
        }
    }

    fn manifest() -> EvaluationManifest {
        let mut holdout = Vec::new();
        for index in 0..MIN_BLIND_HOLDOUT_TRIALS {
            let real = [0.0, 1.0, 2.0, 3.0];
            let shift = index as f64 * 0.01;
            let simulated = real.map(|value| value + 0.1 + shift);
            holdout.push(BlindHoldoutTrial {
                trial_id: format!("trial-{index}"),
                task_id: format!("task-{index}"),
                source_recording_id: format!("blind-recording-{index}"),
                traces: vec![
                    trace(
                        SignalKind::JointPositionRad,
                        "left_j1",
                        &real,
                        &simulated,
                    ),
                    trace(
                        SignalKind::CurrentDerivedTorqueNm,
                        "left_j1",
                        &[0.0, 1.0, 0.0, -1.0],
                        &[0.0, 1.1, 0.0, -1.1],
                    ),
                ],
                task_outcome: TaskOutcomePair {
                    real_stage_score: 0.5 + index as f64 * 0.01,
                    simulated_stage_score: 0.52 + index as f64 * 0.01,
                    real_end_to_end_success: index % 2 == 0,
                    simulated_end_to_end_success: index % 2 == 0,
                },
            });
        }
        let mut repeated_trial = holdout[0].clone();
        repeated_trial.trial_id = "trial-0-repeat".to_owned();
        repeated_trial.source_recording_id = "blind-recording-0-repeat".to_owned();
        holdout.push(repeated_trial);

        EvaluationManifest {
            protocol: protocol(),
            training_recording_ids: vec!["training-recording-01".to_owned()],
            calibration_recording_ids: vec!["calibration-recording-01".to_owned()],
            blind_holdout_trials: holdout,
        }
    }

    #[test]
    fn known_answer_trace_metrics_are_correct() {
        let series = trace(
            SignalKind::JointPositionRad,
            "left_j1",
            &[0.0, 1.0, 2.0],
            &[0.1, 1.1, 2.1],
        );
        let metrics = score_trace(&series).expect("valid trace");
        assert_eq!(metrics.sample_count, 3);
        assert_eq!(metrics.duration_ns, 2_000_000);
        assert!((metrics.mean_absolute_error - 0.1).abs() < 1.0e-12);
        assert!((metrics.root_mean_square_error - 0.1).abs() < 1.0e-12);
        assert!((metrics.signed_bias - 0.1).abs() < 1.0e-12);
        assert!((metrics.p95_absolute_error - 0.1).abs() < 1.0e-12);
        assert!((metrics.normalized_root_mean_square_error - 0.1).abs() < 1.0e-12);
    }

    #[test]
    fn rejects_duplicate_or_nonmonotonic_timestamps() {
        let mut series = trace(
            SignalKind::JointPositionRad,
            "left_j1",
            &[0.0, 1.0, 2.0],
            &[0.0, 1.0, 2.0],
        );
        series.samples[1].real_timestamp_ns = series.samples[0].real_timestamp_ns;
        series.samples[1].simulated_timestamp_ns = series.samples[0].simulated_timestamp_ns;
        assert_eq!(
            score_trace(&series),
            Err(FidelityError::NonMonotonicTimestamp)
        );
    }

    #[test]
    fn rejects_real_simulated_timestamp_misalignment() {
        let mut series = trace(
            SignalKind::JointPositionRad,
            "left_j1",
            &[0.0, 1.0, 2.0],
            &[0.0, 1.0, 2.0],
        );
        series.samples[1].simulated_timestamp_ns += 500_000;
        assert_eq!(score_trace(&series), Err(FidelityError::TimestampMismatch));
    }

    #[test]
    fn rejects_nonfinite_values_and_invalid_normalization_scale() {
        let mut series = trace(
            SignalKind::JointTorqueNm,
            "left_j1",
            &[0.0, 1.0],
            &[0.0, 1.0],
        );
        series.samples[1].simulated_value = f64::NAN;
        assert_eq!(score_trace(&series), Err(FidelityError::NonFiniteSignal));

        series.samples[1].simulated_value = 1.0;
        series.normalization_scale = 0.0;
        assert_eq!(
            score_trace(&series),
            Err(FidelityError::InvalidNormalizationScale)
        );
    }

    #[test]
    fn each_trial_must_have_identical_timestamped_signal_layouts() {
        let mut input = manifest();
        input.blind_holdout_trials[1].traces.pop();
        assert_eq!(
            input.validate(),
            Err(FidelityError::InconsistentSignalSeries)
        );
    }

    #[test]
    fn training_and_calibration_recordings_cannot_leak_into_holdout() {
        let mut input = manifest();
        input.blind_holdout_trials[0].source_recording_id =
            input.training_recording_ids[0].clone();
        assert_eq!(
            input.validate(),
            Err(FidelityError::RecordingSplitLeakage)
        );
    }

    #[test]
    fn simulator_configuration_is_frozen_across_evaluation_manifest() {
        let mut input = manifest();
        input.protocol.simulator_commit = "not-a-sha".to_owned();
        assert_eq!(
            input.validate(),
            Err(FidelityError::InvalidSimulatorCommit)
        );
    }

    #[test]
    fn normalization_scale_is_frozen_for_each_channel_across_trials() {
        let mut input = manifest();
        input.blind_holdout_trials[3].traces[0].normalization_scale = 0.2;
        assert_eq!(
            input.validate(),
            Err(FidelityError::InconsistentSignalSeries)
        );
    }

    #[test]
    fn repeated_recording_cannot_be_counted_as_an_independent_trial() {
        let mut input = manifest();
        input.blind_holdout_trials[1].source_recording_id =
            input.blind_holdout_trials[0].source_recording_id.clone();
        assert_eq!(
            input.validate(),
            Err(FidelityError::RecordingSplitLeakage)
        );
    }

    #[test]
    fn requires_ten_independent_blind_recordings() {
        let mut input = manifest();
        input.blind_holdout_trials.truncate(MIN_BLIND_HOLDOUT_TRIALS - 1);
        assert_eq!(
            input.validate(),
            Err(FidelityError::InsufficientBlindHoldoutTrials)
        );
    }

    #[test]
    fn scorecard_keeps_channels_and_task_outcomes_separate() {
        let report = score_evaluation(
            &manifest(),
            BootstrapConfig {
                replicates: 2_000,
                seed: 5,
            },
        )
        .expect("scorecard");
        assert_eq!(report.independent_blind_trials, 11);
        assert_eq!(report.task_agreement.independent_task_clusters, 10);
        assert_eq!(report.channel_summaries.len(), 2);
        assert_eq!(report.channel_summaries[0].physical_trial_count, 11);
        assert_eq!(report.channel_summaries[0].independent_task_clusters, 10);
        assert_ne!(
            report.channel_summaries[0].signal,
            report.channel_summaries[1].signal
        );
        assert!((report.task_agreement.mean_signed_stage_gap - 0.02).abs() < 1.0e-12);
        assert_eq!(report.task_agreement.false_safe_count, 0);
        assert_eq!(report.task_agreement.false_failure_count, 0);
    }

    #[test]
    fn task_score_match_does_not_hide_false_safe_outcomes() {
        let mut input = manifest();
        input.blind_holdout_trials[0]
            .task_outcome
            .simulated_end_to_end_success = true;
        input.blind_holdout_trials[0]
            .task_outcome
            .real_end_to_end_success = false;
        let report = score_evaluation(
            &input,
            BootstrapConfig {
                replicates: 2_000,
                seed: 7,
            },
        )
        .expect("scorecard");
        assert_eq!(report.task_agreement.false_safe_count, 1);
    }

    #[test]
    fn bootstrap_intervals_are_deterministic_over_task_clusters() {
        let input = manifest();
        let config = BootstrapConfig {
            replicates: 3_000,
            seed: 1234,
        };
        let first = score_evaluation(&input, config).expect("first");
        let second = score_evaluation(&input, config).expect("second");
        assert_eq!(first, second);
    }

    #[test]
    fn insufficient_bootstrap_replicates_are_rejected() {
        assert_eq!(
            score_evaluation(
                &manifest(),
                BootstrapConfig {
                    replicates: 999,
                    seed: 1,
                }
            ),
            Err(FidelityError::InvalidBootstrapReplicateCount)
        );
    }

    #[test]
    fn task_score_correlation_is_none_for_constant_channels() {
        let values = [0.5, 0.5, 0.5];
        assert_eq!(pearson_correlation(&values, &values), None);
        assert_eq!(pearson_correlation(&[0.0, 1.0], &[0.0, 1.0]), Some(1.0));
    }
}
