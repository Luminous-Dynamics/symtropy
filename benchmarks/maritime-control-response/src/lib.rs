#![forbid(unsafe_code)]

//! Independent, offline evaluator primitives for the LiU-ICE industrial
//! fault-diagnosis benchmark and Symtropy's maritime control-aware track.
//!
//! This crate does not train a diagnosis model, control an engine, or claim
//! physical validation. It provides strict signal-role metadata, whole-recording
//! split checks, exact reproduction of the public sample-level competition
//! score, run-level split-conformal calibration, and command-conditioned
//! residual evaluation. A mismatch is not an actuator-fault diagnosis.

pub mod active_probe;
pub mod behavioral_sets;
pub mod csv_audit;
pub mod event_metrics;
pub mod external_dataset;
pub mod joint_scenario_posterior;
pub mod published_baseline_audit;
pub mod sequential_evidence;
pub mod startup_adaptation;
pub mod streaming_contract;
pub mod temporal;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::temporal::BoundedEventTimeInterval;

pub const LIU_ICE_BENCHMARK_URL: &str =
    "https://vehsys.gitlab-pages.liu.se/dx_benchmarks/liuice/liuice_index";
pub const LIU_ICE_NOMINAL_SAMPLE_RATE_HZ: f64 = 20.0;
pub const LIU_ICE_TRAINING_RECORDING_COUNT_REPORTED: usize = 26;
pub const LIU_ICE_FAULT_CLASS_COUNT: usize = 4;
pub const COMPETITION_ISOLATION_CLASSES: usize = 5;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SignalRole {
    ControlIntent,
    CandidateActuatorFeedback,
    PhysicalResponse,
    DisturbanceContext,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SemanticEvidence {
    PublicDescriptionNamesRequestedCommand,
    PublicDescriptionNamesMeasuredSignal,
    PublicDescriptionOnlyNeedsInstrumentReview,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SignalBinding {
    pub signal_id: String,
    pub description: String,
    pub role: SignalRole,
    pub semantic_evidence: SemanticEvidence,
    pub instrument_map_reviewed: bool,
}

/// Explicit mapping of the ten signals listed on the LiU-ICE benchmark page.
/// The yxpos throttle-actuator-position channel is only candidate feedback
/// pending its instrument/I-O dictionary. It is not a measured wastegate
/// position signal.
pub fn liu_ice_signal_registry() -> Vec<SignalBinding> {
    let entries = [
        (
            "umf",
            "Requested injected fuel mass",
            SignalRole::ControlIntent,
            SemanticEvidence::PublicDescriptionNamesRequestedCommand,
        ),
        (
            "uwg",
            "Requested wastegate actuator position",
            SignalRole::ControlIntent,
            SemanticEvidence::PublicDescriptionNamesRequestedCommand,
        ),
        (
            "yxpos",
            "Throttle actuator position",
            SignalRole::CandidateActuatorFeedback,
            SemanticEvidence::PublicDescriptionOnlyNeedsInstrumentReview,
        ),
        (
            "ypic",
            "Intercooler pressure",
            SignalRole::PhysicalResponse,
            SemanticEvidence::PublicDescriptionNamesMeasuredSignal,
        ),
        (
            "yTic",
            "Intercooler temperature",
            SignalRole::PhysicalResponse,
            SemanticEvidence::PublicDescriptionNamesMeasuredSignal,
        ),
        (
            "ypim",
            "Intake manifold pressure",
            SignalRole::PhysicalResponse,
            SemanticEvidence::PublicDescriptionNamesMeasuredSignal,
        ),
        (
            "ywaf",
            "Mass flow through the air filter",
            SignalRole::PhysicalResponse,
            SemanticEvidence::PublicDescriptionNamesMeasuredSignal,
        ),
        (
            "yomega",
            "Engine speed",
            SignalRole::PhysicalResponse,
            SemanticEvidence::PublicDescriptionNamesMeasuredSignal,
        ),
        (
            "ypamb",
            "Ambient pressure",
            SignalRole::DisturbanceContext,
            SemanticEvidence::PublicDescriptionNamesMeasuredSignal,
        ),
        (
            "yTamb",
            "Ambient temperature",
            SignalRole::DisturbanceContext,
            SemanticEvidence::PublicDescriptionNamesMeasuredSignal,
        ),
    ];

    entries
        .into_iter()
        .map(
            |(signal_id, description, role, semantic_evidence)| SignalBinding {
                signal_id: signal_id.to_owned(),
                description: description.to_owned(),
                role,
                semantic_evidence,
                instrument_map_reviewed: false,
            },
        )
        .collect()
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum FaultClass {
    FPic,
    FPim,
    FWaf,
    FIntakeManifoldLeak,
}

impl FaultClass {
    pub const ALL: [Self; 4] = [
        Self::FPic,
        Self::FPim,
        Self::FWaf,
        Self::FIntakeManifoldLeak,
    ];

    pub fn isolation_index(self) -> usize {
        match self {
            Self::FPic => 0,
            Self::FPim => 1,
            Self::FWaf => 2,
            Self::FIntakeManifoldLeak => 3,
        }
    }
}

/// Ground truth is evaluator-only and must never be included in model inputs.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LiuIceTruth {
    NoFault,
    KnownFault(FaultClass),
    UnknownFault,
}

impl LiuIceTruth {
    fn is_fault(self) -> bool {
        !matches!(self, Self::NoFault)
    }

    fn isolation_index(self) -> Option<usize> {
        match self {
            Self::NoFault => None,
            Self::KnownFault(fault) => Some(fault.isolation_index()),
            Self::UnknownFault => Some(4),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct SampleKey {
    pub recording_id: String,
    pub sample_index: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LabeledSample {
    pub key: SampleKey,
    pub truth: LiuIceTruth,
}

/// Model output uses the official benchmark's five isolation slots:
/// f_pic, f_pim, f_waf, f_iml and f_x.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FaultIsolationProbabilities {
    pub probabilities: [f64; COMPETITION_ISOLATION_CLASSES],
}

impl FaultIsolationProbabilities {
    fn validate_for_detection(&self, detection: bool, key: &SampleKey) -> Result<(), EvalError> {
        if self
            .probabilities
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(EvalError::InvalidPrediction(format!(
                "{}:{} has a negative or non-finite isolation probability",
                key.recording_id, key.sample_index
            )));
        }
        let sum: f64 = self.probabilities.iter().sum();
        if detection {
            if (sum - 1.0).abs() > 1e-6 {
                return Err(EvalError::InvalidPrediction(format!(
                    "{}:{} detected fault must have isolation probabilities summing to 1 (got {sum})",
                    key.recording_id, key.sample_index
                )));
            }
        } else if self.probabilities.iter().any(|value| *value != 0.0) {
            return Err(EvalError::InvalidPrediction(format!(
                "{}:{} no-fault decision must have an all-zero isolation vector",
                key.recording_id, key.sample_index
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Prediction {
    pub key: SampleKey,
    pub detection: bool,
    pub isolation: FaultIsolationProbabilities,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CompetitionScore {
    pub samples: usize,
    pub no_fault_samples: usize,
    pub fault_samples: usize,
    pub false_alarm_samples: usize,
    pub detected_fault_samples: usize,
    pub missed_fault_samples: usize,
    pub isolation_scored_samples: usize,
    pub false_alarm_rate: f64,
    pub true_detection_rate: f64,
    /// FIA is the mean probability assigned to the true diagnosis for
    /// fault samples that were correctly detected, not top-1 accuracy.
    /// The public competition formula uses zero when that set is empty.
    pub fault_isolation_accuracy: f64,
    pub fault_isolation_denominator_defined: bool,
    pub total_score: f64,
    pub naive_no_alarm_reference_score: f64,
}

/// Reproduce the public LiU-ICE sample-level score exactly:
/// ((1 - FAR) + TDR + FIA) / 3.
/// This is a competition metric, not an operational risk metric. Always pair
/// it with per-recording/event metrics and grouped uncertainty estimates.
pub fn evaluate_competition_score(
    labels: &[LabeledSample],
    predictions: &[Prediction],
) -> Result<CompetitionScore, EvalError> {
    if labels.is_empty() {
        return Err(EvalError::EmptyEvaluation);
    }

    let mut truth_by_key = BTreeMap::new();
    for label in labels {
        if label.key.recording_id.trim().is_empty() {
            return Err(EvalError::EmptyIdentity);
        }
        if truth_by_key
            .insert(label.key.clone(), label.truth)
            .is_some()
        {
            return Err(EvalError::DuplicateSampleKey(label.key.clone()));
        }
    }

    let mut prediction_by_key = BTreeMap::new();
    for prediction in predictions {
        prediction
            .isolation
            .validate_for_detection(prediction.detection, &prediction.key)?;
        if prediction_by_key
            .insert(prediction.key.clone(), prediction)
            .is_some()
        {
            return Err(EvalError::DuplicateSampleKey(prediction.key.clone()));
        }
    }

    for key in truth_by_key.keys() {
        if !prediction_by_key.contains_key(key) {
            return Err(EvalError::MissingPrediction(key.clone()));
        }
    }
    for key in prediction_by_key.keys() {
        if !truth_by_key.contains_key(key) {
            return Err(EvalError::UnexpectedPrediction(key.clone()));
        }
    }

    let mut no_fault_samples = 0usize;
    let mut fault_samples = 0usize;
    let mut false_alarm_samples = 0usize;
    let mut detected_fault_samples = 0usize;
    let mut missed_fault_samples = 0usize;
    let mut isolation_probability_sum = 0.0;

    for (key, truth) in &truth_by_key {
        let prediction = prediction_by_key
            .get(key)
            .expect("prediction keys checked for exact equality");
        if !truth.is_fault() {
            no_fault_samples += 1;
            if prediction.detection {
                false_alarm_samples += 1;
            }
            continue;
        }

        fault_samples += 1;
        if !prediction.detection {
            missed_fault_samples += 1;
            continue;
        }

        detected_fault_samples += 1;
        let true_index = truth
            .isolation_index()
            .expect("fault truth has an isolation index");
        isolation_probability_sum += prediction.isolation.probabilities[true_index];
    }

    if no_fault_samples == 0 {
        return Err(EvalError::MissingNoFaultSamples);
    }
    if fault_samples == 0 {
        return Err(EvalError::MissingFaultSamples);
    }

    let false_alarm_rate = false_alarm_samples as f64 / no_fault_samples as f64;
    let true_detection_rate = detected_fault_samples as f64 / fault_samples as f64;
    let fia_defined = detected_fault_samples > 0;
    let fault_isolation_accuracy = if fia_defined {
        isolation_probability_sum / detected_fault_samples as f64
    } else {
        0.0
    };
    let total_score =
        ((1.0 - false_alarm_rate) + true_detection_rate + fault_isolation_accuracy) / 3.0;

    Ok(CompetitionScore {
        samples: labels.len(),
        no_fault_samples,
        fault_samples,
        false_alarm_samples,
        detected_fault_samples,
        missed_fault_samples,
        isolation_scored_samples: detected_fault_samples,
        false_alarm_rate,
        true_detection_rate,
        fault_isolation_accuracy,
        fault_isolation_denominator_defined: fia_defined,
        total_score,
        naive_no_alarm_reference_score: 1.0 / 3.0,
    })
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DatasetSplit {
    Train,
    Calibration,
    Validation,
    Test,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SplitMode {
    WholeRecording,
    HeldOutDrivingCycle,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RecordingAssignment {
    pub recording_id: String,
    pub driving_cycle: String,
    pub split: DatasetSplit,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RecordingSplitManifest {
    pub format_version: u32,
    pub mode: SplitMode,
    pub assignments: Vec<RecordingAssignment>,
}

impl RecordingSplitManifest {
    /// Validate whole-recording assignment and, when requested, keep every
    /// occurrence of a driving cycle within exactly one split.
    pub fn validate(&self) -> Result<(), EvalError> {
        if self.format_version != 1 || self.assignments.is_empty() {
            return Err(EvalError::InvalidSplitManifest);
        }
        let mut recording_ids = BTreeSet::new();
        let mut cycle_split = BTreeMap::<String, DatasetSplit>::new();
        let mut train = 0usize;
        let mut test = 0usize;
        for assignment in &self.assignments {
            if assignment.recording_id.trim().is_empty()
                || assignment.driving_cycle.trim().is_empty()
            {
                return Err(EvalError::EmptyIdentity);
            }
            if !recording_ids.insert(assignment.recording_id.clone()) {
                return Err(EvalError::DuplicateRecording(
                    assignment.recording_id.clone(),
                ));
            }
            train += if assignment.split == DatasetSplit::Train {
                1
            } else {
                0
            };
            test += if assignment.split == DatasetSplit::Test {
                1
            } else {
                0
            };
            if self.mode == SplitMode::HeldOutDrivingCycle {
                if let Some(previous) = cycle_split.get(&assignment.driving_cycle) {
                    if *previous != assignment.split {
                        return Err(EvalError::DrivingCycleSplitLeakage(
                            assignment.driving_cycle.clone(),
                        ));
                    }
                } else {
                    cycle_split.insert(assignment.driving_cycle.clone(), assignment.split);
                }
            }
        }
        if train == 0 || test == 0 {
            return Err(EvalError::InvalidSplitManifest);
        }
        Ok(())
    }

    pub fn recording_ids_for(&self, split: DatasetSplit) -> BTreeSet<String> {
        self.assignments
            .iter()
            .filter(|assignment| assignment.split == split)
            .map(|assignment| assignment.recording_id.clone())
            .collect()
    }

    /// Calibration recordings must not have been used to fit the model and
    /// must not be reused as test recordings.
    pub fn validate_calibration_separation(&self) -> Result<(), EvalError> {
        self.validate()?;
        let train = self.recording_ids_for(DatasetSplit::Train);
        let calibration = self.recording_ids_for(DatasetSplit::Calibration);
        let test = self.recording_ids_for(DatasetSplit::Test);
        if !train.is_disjoint(&calibration)
            || !train.is_disjoint(&test)
            || !calibration.is_disjoint(&test)
        {
            return Err(EvalError::SplitLeakage);
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String, EvalError> {
        sha256_serializable(b"symtropy-liu-ice-recording-split-v1\0", self)
    }

    /// Canonical digest of the sorted recording IDs assigned to one split.
    pub fn recording_manifest_digest(&self, split: DatasetSplit) -> Result<String, EvalError> {
        let ids = self
            .recording_ids_for(split)
            .into_iter()
            .collect::<Vec<_>>();
        sha256_serializable(b"symtropy-liu-ice-recording-members-v1\0", &ids)
    }
}

/// Require the calibration receipt to refer to exactly the recording IDs
/// assigned to the calibration split, and bind model/regime/training/split IDs.
/// This closes the caller-supplied-hash gap at evaluation time.
pub fn validate_conformal_threshold_against_split(
    threshold: &ConformalThreshold,
    split_manifest: &RecordingSplitManifest,
) -> Result<(), EvalError> {
    threshold.validate()?;
    split_manifest.validate_calibration_separation()?;
    let split_digest = split_manifest.digest()?;
    let training_digest = split_manifest.recording_manifest_digest(DatasetSplit::Train)?;
    let calibration_ids = split_manifest
        .recording_ids_for(DatasetSplit::Calibration)
        .into_iter()
        .collect::<Vec<_>>();
    if split_digest != threshold.split_manifest_sha256
        || training_digest != threshold.training_recordings_manifest_sha256
        || calibration_ids != threshold.calibration_recording_ids
    {
        return Err(EvalError::CalibrationBindingMismatch);
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FeatureRow {
    pub key: SampleKey,
    pub time_seconds: f64,
    /// Missing channels are absent from this map; they are never filled with
    /// zero or a nominal command by this contract.
    pub signals: BTreeMap<String, f64>,
}

pub fn validate_recording_rows(rows: &[FeatureRow]) -> Result<(), EvalError> {
    if rows.is_empty() {
        return Err(EvalError::EmptyRecording);
    }
    let recording_id = rows[0].key.recording_id.as_str();
    if recording_id.trim().is_empty() {
        return Err(EvalError::EmptyIdentity);
    }
    let mut prior_index = None;
    let mut prior_time = None;
    for row in rows {
        if row.key.recording_id != recording_id
            || !row.time_seconds.is_finite()
            || row.time_seconds < 0.0
        {
            return Err(EvalError::InvalidSample);
        }
        if prior_index.is_some_and(|index| row.key.sample_index <= index)
            || prior_time.is_some_and(|time| row.time_seconds <= time)
        {
            return Err(EvalError::NonMonotonicSample);
        }
        if row
            .signals
            .iter()
            .any(|(name, value)| name.trim().is_empty() || !value.is_finite())
        {
            return Err(EvalError::InvalidSample);
        }
        prior_index = Some(row.key.sample_index);
        prior_time = Some(row.time_seconds);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SignalQuality {
    Current,
    Missing,
    Stale,
    Conflicted,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    WithinCalibratedEnvelope,
    MismatchOutsideCalibratedEnvelope,
    NotEvaluableMissingCommand,
    NotEvaluableMissingResponse,
    NotEvaluableTimebase,
    NotEvaluableTemporalOrder,
    NotEvaluableFutureCommand,
    NotEvaluableResponseBeforePrediction,
    NotEvaluableResponseOutsideHorizon,
    NotEvaluableStaleCommand,
    NotEvaluableCommandQuality,
    NotEvaluableResponseQuality,
    NotEvaluableUnsupportedRegime,
    NotEvaluableMissingModelPrediction,
    NotEvaluableUncalibrated,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CauseClaim {
    NotIsolated,
}

/// One already-paired command/response observation. The command timestamp
/// means when the command became available to the diagnostic process, not an
/// inferred controller output timestamp.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CommandResponsePair {
    pub recording_id: String,
    pub sample_index: u64,
    pub prediction_time_seconds: f64,
    pub prediction_timestamp_uncertainty_seconds: f64,
    pub timebase_id: String,
    pub timebase_qualified: bool,
    pub timebase_qualification_sha256: String,
    pub command_signal: String,
    pub response_signal: String,
    pub regime_id: String,
    pub model_sha256: String,
    pub training_recordings_manifest_sha256: String,
    pub split_manifest_sha256: String,
    pub command_value: Option<f64>,
    /// Time at which the requested command became available to the predictor.
    pub command_time_seconds: Option<f64>,
    pub command_timestamp_uncertainty_seconds: Option<f64>,
    pub response_value: Option<f64>,
    /// Time at which the independent physical response was observed.
    pub response_time_seconds: Option<f64>,
    pub response_timestamp_uncertainty_seconds: Option<f64>,
    pub maximum_response_horizon_seconds: f64,
    pub expected_response: Option<f64>,
    /// Training-only residual standard deviation of the frozen response model.
    pub model_residual_std: f64,
    /// Measurement uncertainty standard deviation for this response sample.
    pub measurement_std: f64,
    pub maximum_command_age_seconds: f64,
    pub command_quality: SignalQuality,
    pub response_quality: SignalQuality,
    pub regime_supported: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CommandResponseTemporalEvidence {
    pub command_available_earliest_seconds: f64,
    pub command_available_latest_seconds: f64,
    pub prediction_earliest_seconds: f64,
    pub prediction_latest_seconds: f64,
    pub response_observed_earliest_seconds: f64,
    pub response_observed_latest_seconds: f64,
    pub command_age_worst_case_seconds: f64,
    pub response_lag_minimum_seconds: f64,
    pub response_lag_maximum_seconds: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ResponseEvaluation {
    pub recording_id: String,
    pub sample_index: u64,
    pub prediction_time_seconds: f64,
    pub timebase_id: String,
    pub timebase_qualification_sha256: String,
    pub temporal_evidence: Option<CommandResponseTemporalEvidence>,
    pub command_signal: String,
    pub response_signal: String,
    pub regime_id: String,
    pub model_sha256: String,
    pub training_recordings_manifest_sha256: String,
    pub split_manifest_sha256: String,
    pub calibration_recordings_manifest_sha256: Option<String>,
    pub status: ResponseStatus,
    pub observed_residual: Option<f64>,
    pub standardized_absolute_residual: Option<f64>,
    pub conformal_threshold: Option<f64>,
    /// Deliberately never isolated by a residual threshold alone.
    pub cause_claim: CauseClaim,
}

/// Compare an observed plant response with a frozen command-conditioned
/// response prediction. Future commands, stale commands, unsupported regimes,
/// missing data, and uncalibrated residuals are non-evaluable—not normal.
pub fn evaluate_command_response(
    pair: &CommandResponsePair,
    threshold: Option<&ConformalThreshold>,
) -> Result<ResponseEvaluation, EvalError> {
    if pair.recording_id.trim().is_empty()
        || pair.command_signal.trim().is_empty()
        || pair.response_signal.trim().is_empty()
        || pair.regime_id.trim().is_empty()
        || !is_sha256_hex(&pair.model_sha256)
        || !is_sha256_hex(&pair.training_recordings_manifest_sha256)
        || !is_sha256_hex(&pair.split_manifest_sha256)
        || pair.training_recordings_manifest_sha256 == pair.split_manifest_sha256
        || !pair.model_residual_std.is_finite()
        || pair.model_residual_std < 0.0
        || !pair.measurement_std.is_finite()
        || pair.measurement_std < 0.0
        || !pair.maximum_command_age_seconds.is_finite()
        || pair.maximum_command_age_seconds < 0.0
        || !pair.prediction_time_seconds.is_finite()
        || pair.prediction_time_seconds < 0.0
        || !pair.prediction_timestamp_uncertainty_seconds.is_finite()
        || pair.prediction_timestamp_uncertainty_seconds < 0.0
        || pair.timebase_id.trim().is_empty()
        || !is_sha256_hex(&pair.timebase_qualification_sha256)
        || !pair.maximum_response_horizon_seconds.is_finite()
        || pair.maximum_response_horizon_seconds <= 0.0
    {
        return Err(EvalError::InvalidCommandResponsePair);
    }

    let mut result = ResponseEvaluation {
        recording_id: pair.recording_id.clone(),
        sample_index: pair.sample_index,
        prediction_time_seconds: pair.prediction_time_seconds,
        timebase_id: pair.timebase_id.clone(),
        timebase_qualification_sha256: pair.timebase_qualification_sha256.clone(),
        temporal_evidence: None,
        command_signal: pair.command_signal.clone(),
        response_signal: pair.response_signal.clone(),
        regime_id: pair.regime_id.clone(),
        model_sha256: pair.model_sha256.clone(),
        training_recordings_manifest_sha256: pair.training_recordings_manifest_sha256.clone(),
        split_manifest_sha256: pair.split_manifest_sha256.clone(),
        calibration_recordings_manifest_sha256: None,
        status: ResponseStatus::NotEvaluableUncalibrated,
        observed_residual: None,
        standardized_absolute_residual: None,
        conformal_threshold: threshold.map(|item| item.threshold),
        cause_claim: CauseClaim::NotIsolated,
    };

    if !pair.timebase_qualified {
        result.status = ResponseStatus::NotEvaluableTimebase;
        return Ok(result);
    }
    if pair.command_quality == SignalQuality::Missing || pair.command_value.is_none() {
        result.status = ResponseStatus::NotEvaluableMissingCommand;
        return Ok(result);
    }
    if pair.command_quality != SignalQuality::Current {
        result.status = ResponseStatus::NotEvaluableCommandQuality;
        return Ok(result);
    }
    if pair.response_quality == SignalQuality::Missing || pair.response_value.is_none() {
        result.status = ResponseStatus::NotEvaluableMissingResponse;
        return Ok(result);
    }
    if pair.response_quality != SignalQuality::Current {
        result.status = ResponseStatus::NotEvaluableResponseQuality;
        return Ok(result);
    }
    if !pair.regime_supported {
        result.status = ResponseStatus::NotEvaluableUnsupportedRegime;
        return Ok(result);
    }

    if pair.command_value.is_some_and(|value| !value.is_finite()) {
        return Err(EvalError::InvalidCommandResponsePair);
    }
    let command_time = match (
        pair.command_time_seconds,
        pair.command_timestamp_uncertainty_seconds,
    ) {
        (Some(value), Some(uncertainty))
            if value.is_finite() && uncertainty.is_finite() && uncertainty >= 0.0 =>
        {
            match BoundedEventTimeInterval::new(value, uncertainty) {
                Ok(interval) => interval,
                Err(_) => return Err(EvalError::InvalidCommandResponsePair),
            }
        }
        _ => {
            result.status = ResponseStatus::NotEvaluableMissingCommand;
            return Ok(result);
        }
    };
    let response_time = match (
        pair.response_time_seconds,
        pair.response_timestamp_uncertainty_seconds,
    ) {
        (Some(value), Some(uncertainty))
            if value.is_finite() && uncertainty.is_finite() && uncertainty >= 0.0 =>
        {
            match BoundedEventTimeInterval::new(value, uncertainty) {
                Ok(interval) => interval,
                Err(_) => return Err(EvalError::InvalidCommandResponsePair),
            }
        }
        _ => {
            result.status = ResponseStatus::NotEvaluableMissingResponse;
            return Ok(result);
        }
    };
    let prediction_time = match BoundedEventTimeInterval::new(
        pair.prediction_time_seconds,
        pair.prediction_timestamp_uncertainty_seconds,
    ) {
        Ok(interval) => interval,
        Err(_) => return Err(EvalError::InvalidCommandResponsePair),
    };

    if command_time.is_strictly_after(prediction_time) {
        result.status = ResponseStatus::NotEvaluableFutureCommand;
        return Ok(result);
    }
    if command_time.latest_seconds() > prediction_time.earliest_seconds() {
        result.status = ResponseStatus::NotEvaluableTemporalOrder;
        return Ok(result);
    }
    if response_time.is_strictly_before(prediction_time) {
        result.status = ResponseStatus::NotEvaluableResponseBeforePrediction;
        return Ok(result);
    }
    if !response_time.is_strictly_after(prediction_time) {
        result.status = ResponseStatus::NotEvaluableTemporalOrder;
        return Ok(result);
    }
    let command_age_worst_case =
        prediction_time.latest_seconds() - command_time.earliest_seconds();
    if command_age_worst_case > pair.maximum_command_age_seconds {
        result.status = ResponseStatus::NotEvaluableStaleCommand;
        return Ok(result);
    }
    let response_lag_minimum =
        response_time.earliest_seconds() - prediction_time.latest_seconds();
    let response_lag_maximum =
        response_time.latest_seconds() - prediction_time.earliest_seconds();
    if response_lag_minimum > pair.maximum_response_horizon_seconds {
        result.status = ResponseStatus::NotEvaluableResponseOutsideHorizon;
        return Ok(result);
    }
    if response_lag_maximum > pair.maximum_response_horizon_seconds {
        result.status = ResponseStatus::NotEvaluableTemporalOrder;
        return Ok(result);
    }
    result.temporal_evidence = Some(CommandResponseTemporalEvidence {
        command_available_earliest_seconds: command_time.earliest_seconds(),
        command_available_latest_seconds: command_time.latest_seconds(),
        prediction_earliest_seconds: prediction_time.earliest_seconds(),
        prediction_latest_seconds: prediction_time.latest_seconds(),
        response_observed_earliest_seconds: response_time.earliest_seconds(),
        response_observed_latest_seconds: response_time.latest_seconds(),
        command_age_worst_case_seconds: command_age_worst_case,
        response_lag_minimum_seconds: response_lag_minimum,
        response_lag_maximum_seconds: response_lag_maximum,
    });

    let observed = pair.response_value.expect("checked above");
    let Some(expected) = pair.expected_response else {
        result.status = ResponseStatus::NotEvaluableMissingModelPrediction;
        return Ok(result);
    };
    if !observed.is_finite() || !expected.is_finite() {
        return Err(EvalError::InvalidCommandResponsePair);
    }
    let scale = pair
        .model_residual_std
        .mul_add(
            pair.model_residual_std,
            pair.measurement_std * pair.measurement_std,
        )
        .sqrt();
    if !scale.is_finite() || scale <= f64::EPSILON {
        return Err(EvalError::InvalidCommandResponsePair);
    }

    let residual = observed - expected;
    let standardized = residual.abs() / scale;
    result.observed_residual = Some(residual);
    result.standardized_absolute_residual = Some(standardized);
    let Some(threshold) = threshold else {
        result.status = ResponseStatus::NotEvaluableUncalibrated;
        return Ok(result);
    };
    threshold.validate()?;
    if threshold.model_sha256 != pair.model_sha256
        || threshold.training_recordings_manifest_sha256 != pair.training_recordings_manifest_sha256
        || threshold.split_manifest_sha256 != pair.split_manifest_sha256
        || threshold.regime_id != pair.regime_id
    {
        return Err(EvalError::CalibrationBindingMismatch);
    }
    result.conformal_threshold = Some(threshold.threshold);
    result.calibration_recordings_manifest_sha256 =
        Some(threshold.calibration_recordings_manifest_sha256.clone());
    result.status = if standardized > threshold.threshold {
        ResponseStatus::MismatchOutsideCalibratedEnvelope
    } else {
        ResponseStatus::WithinCalibratedEnvelope
    };
    Ok(result)
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EpisodeCalibrationScore {
    pub recording_id: String,
    pub model_sha256: String,
    pub regime_id: String,
    /// Maximum absolute standardized residual over the frozen scored window
    /// for this independent recording/episode.
    pub maximum_absolute_standardized_residual: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ConformalThreshold {
    pub target_coverage_ppm: u32,
    pub model_sha256: String,
    pub training_recordings_manifest_sha256: String,
    pub split_manifest_sha256: String,
    pub calibration_recordings_manifest_sha256: String,
    pub regime_id: String,
    pub calibration_recording_ids: Vec<String>,
    /// Maximum residual scores aligned positionally with sorted recording IDs.
    pub calibration_max_scores: Vec<f64>,
    pub independent_calibration_episodes: usize,
    pub order_statistic_rank_1_based: usize,
    pub threshold: f64,
    pub calibration_scores_sha256: String,
}

impl ConformalThreshold {
    /// Recompute the finite-sample order statistic and both calibration digests.
    pub fn validate(&self) -> Result<(), EvalError> {
        let count = self.calibration_recording_ids.len();
        if self.target_coverage_ppm == 0
            || self.target_coverage_ppm >= 1_000_000
            || count == 0
            || self.independent_calibration_episodes != count
            || self.calibration_max_scores.len() != count
            || self.order_statistic_rank_1_based == 0
            || self.order_statistic_rank_1_based > count
            || !self.threshold.is_finite()
            || self.threshold < 0.0
            || self.regime_id.trim().is_empty()
            || !is_sha256_hex(&self.model_sha256)
            || !is_sha256_hex(&self.training_recordings_manifest_sha256)
            || !is_sha256_hex(&self.split_manifest_sha256)
            || !is_sha256_hex(&self.calibration_recordings_manifest_sha256)
            || self.training_recordings_manifest_sha256
                == self.calibration_recordings_manifest_sha256
            || self.training_recordings_manifest_sha256 == self.split_manifest_sha256
            || !is_sha256_hex(&self.calibration_scores_sha256)
            || self
                .calibration_recording_ids
                .iter()
                .any(|id| id.trim().is_empty())
            || self
                .calibration_recording_ids
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || self
                .calibration_max_scores
                .iter()
                .any(|score| !score.is_finite() || *score < 0.0)
        {
            return Err(EvalError::InvalidConformalThreshold);
        }
        let manifest_digest = sha256_serializable(
            b"symtropy-liu-ice-calibration-recording-manifest-v1\0",
            &self.calibration_recording_ids,
        )?;
        if manifest_digest != self.calibration_recordings_manifest_sha256 {
            return Err(EvalError::InvalidConformalThreshold);
        }
        let score_digest = sha256_serializable(
            b"symtropy-liu-ice-episode-calibration-scores-v1\0",
            &(
                self.model_sha256.as_str(),
                self.regime_id.as_str(),
                &self.calibration_recording_ids,
                &self.calibration_max_scores,
            ),
        )?;
        if score_digest != self.calibration_scores_sha256 {
            return Err(EvalError::InvalidConformalThreshold);
        }
        let expected_rank = ((count as u128 + 1) * u128::from(self.target_coverage_ppm))
            .div_ceil(1_000_000) as usize;
        if expected_rank > count || self.order_statistic_rank_1_based != expected_rank {
            return Err(EvalError::InvalidConformalThreshold);
        }
        let mut ordered = self.calibration_max_scores.clone();
        ordered.sort_by(f64::total_cmp);
        if self.threshold != ordered[expected_rank - 1] {
            return Err(EvalError::InvalidConformalThreshold);
        }
        Ok(())
    }
}

/// Split-conformal upper-tail calibration over one maximum residual score per
/// independent recording/episode. This avoids treating 20 Hz rows or overlapping
/// windows as independent calibration units.
///
/// Exchangeability of whole-recording scores remains an empirical assumption;
/// this helper does not prove that different drive cycles, faults or regimes
/// are exchangeable. Calibrate separately by a predeclared regime where data
/// volume permits, otherwise return InsufficientCalibrationEpisodes.
pub fn fit_episode_conformal_threshold(
    scores: &[EpisodeCalibrationScore],
    target_coverage_ppm: u32,
    model_sha256: &str,
    training_recordings_manifest_sha256: &str,
    split_manifest_sha256: &str,
    regime_id: &str,
) -> Result<ConformalThreshold, EvalError> {
    if target_coverage_ppm == 0 || target_coverage_ppm >= 1_000_000 {
        return Err(EvalError::InvalidConformalCoverage);
    }
    if regime_id.trim().is_empty()
        || !is_sha256_hex(model_sha256)
        || !is_sha256_hex(training_recordings_manifest_sha256)
        || !is_sha256_hex(split_manifest_sha256)
        || training_recordings_manifest_sha256 == split_manifest_sha256
    {
        return Err(EvalError::InvalidCalibrationProvenance);
    }
    if scores.is_empty() {
        return Err(EvalError::InsufficientCalibrationEpisodes {
            required: 1,
            available: 0,
        });
    }

    let mut score_by_recording = BTreeMap::new();
    for score in scores {
        if score.recording_id.trim().is_empty()
            || score.model_sha256 != model_sha256
            || score.regime_id != regime_id
            || !score.maximum_absolute_standardized_residual.is_finite()
            || score.maximum_absolute_standardized_residual < 0.0
        {
            return Err(EvalError::InvalidCalibrationScore);
        }
        if score_by_recording
            .insert(
                score.recording_id.clone(),
                score.maximum_absolute_standardized_residual,
            )
            .is_some()
        {
            return Err(EvalError::DuplicateRecording(score.recording_id.clone()));
        }
    }
    let calibration_recording_ids = score_by_recording.keys().cloned().collect::<Vec<_>>();
    let calibration_max_scores = score_by_recording.values().copied().collect::<Vec<_>>();
    let mut ordered_scores = calibration_max_scores.clone();
    ordered_scores.sort_by(f64::total_cmp);

    let numerator = (scores.len() as u128 + 1) * u128::from(target_coverage_ppm);
    let rank = numerator.div_ceil(1_000_000) as usize;
    if rank > ordered_scores.len() {
        return Err(EvalError::InsufficientCalibrationEpisodes {
            required: minimum_calibration_episodes(target_coverage_ppm),
            available: ordered_scores.len(),
        });
    }

    let calibration_recordings_manifest_sha256 = sha256_serializable(
        b"symtropy-liu-ice-calibration-recording-manifest-v1\0",
        &calibration_recording_ids,
    )?;
    let digest = sha256_serializable(
        b"symtropy-liu-ice-episode-calibration-scores-v1\0",
        &(
            model_sha256,
            regime_id,
            &calibration_recording_ids,
            &calibration_max_scores,
        ),
    )?;
    let threshold = ConformalThreshold {
        target_coverage_ppm,
        model_sha256: model_sha256.to_owned(),
        training_recordings_manifest_sha256: training_recordings_manifest_sha256.to_owned(),
        split_manifest_sha256: split_manifest_sha256.to_owned(),
        calibration_recordings_manifest_sha256,
        regime_id: regime_id.to_owned(),
        calibration_recording_ids,
        calibration_max_scores,
        independent_calibration_episodes: scores.len(),
        order_statistic_rank_1_based: rank,
        threshold: ordered_scores[rank - 1],
        calibration_scores_sha256: digest,
    };
    threshold.validate()?;
    Ok(threshold)
}

/// Minimum n for which ceil((n+1)*coverage) <= n.
/// Examples: 9 independent episodes for 90%; 19 for 95%; 99 for 99%.
pub fn minimum_calibration_episodes(target_coverage_ppm: u32) -> usize {
    if target_coverage_ppm == 0 || target_coverage_ppm >= 1_000_000 {
        return usize::MAX;
    }
    let numerator = u128::from(target_coverage_ppm);
    let denominator = 1_000_000_u128 - numerator;
    (numerator.div_ceil(denominator)) as usize
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalError {
    EmptyEvaluation,
    EmptyIdentity,
    EmptyRecording,
    InvalidSample,
    NonMonotonicSample,
    DuplicateSampleKey(SampleKey),
    MissingPrediction(SampleKey),
    UnexpectedPrediction(SampleKey),
    InvalidPrediction(String),
    MissingNoFaultSamples,
    MissingFaultSamples,
    InvalidSplitManifest,
    DuplicateRecording(String),
    DrivingCycleSplitLeakage(String),
    SplitLeakage,
    InvalidCalibrationScore,
    InvalidConformalCoverage,
    InvalidConformalThreshold,
    InvalidCalibrationProvenance,
    CalibrationBindingMismatch,
    InsufficientCalibrationEpisodes { required: usize, available: usize },
    InvalidCommandResponsePair,
    Serialization,
}

impl fmt::Display for EvalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyEvaluation => write!(formatter, "evaluation set is empty"),
            Self::EmptyIdentity => write!(formatter, "recording or sample identity is empty"),
            Self::EmptyRecording => write!(formatter, "recording has no rows"),
            Self::InvalidSample => write!(
                formatter,
                "sample contains invalid identity, time, or signal value"
            ),
            Self::NonMonotonicSample => write!(
                formatter,
                "sample index and timestamp must be strictly increasing"
            ),
            Self::DuplicateSampleKey(key) => write!(
                formatter,
                "duplicate sample key {}:{}",
                key.recording_id, key.sample_index
            ),
            Self::MissingPrediction(key) => write!(
                formatter,
                "missing prediction {}:{}",
                key.recording_id, key.sample_index
            ),
            Self::UnexpectedPrediction(key) => write!(
                formatter,
                "prediction has no matching truth {}:{}",
                key.recording_id, key.sample_index
            ),
            Self::InvalidPrediction(message) => write!(formatter, "invalid prediction: {message}"),
            Self::MissingNoFaultSamples => {
                write!(formatter, "evaluation needs at least one no-fault sample")
            }
            Self::MissingFaultSamples => {
                write!(formatter, "evaluation needs at least one fault sample")
            }
            Self::InvalidSplitManifest => write!(
                formatter,
                "split manifest is empty or lacks train/test recordings"
            ),
            Self::DuplicateRecording(id) => write!(formatter, "duplicate recording identity: {id}"),
            Self::DrivingCycleSplitLeakage(cycle) => write!(
                formatter,
                "driving cycle appears in more than one split: {cycle}"
            ),
            Self::SplitLeakage => write!(
                formatter,
                "train, calibration and test recordings must be disjoint"
            ),
            Self::InvalidCalibrationScore => write!(
                formatter,
                "calibration score is negative, non-finite, or lacks an identity"
            ),
            Self::InvalidCalibrationProvenance => write!(
                formatter,
                "calibration provenance is incomplete or inconsistent"
            ),
            Self::CalibrationBindingMismatch => write!(
                formatter,
                "calibration report belongs to a different model, regime, or split"
            ),
            Self::InvalidConformalCoverage => write!(
                formatter,
                "conformal target coverage must be in 1..999999 ppm"
            ),
            Self::InvalidConformalThreshold => {
                write!(formatter, "conformal threshold claims are invalid")
            }
            Self::InsufficientCalibrationEpisodes {
                required,
                available,
            } => write!(
                formatter,
                "insufficient independent calibration episodes: need at least {required}, have {available}"
            ),
            Self::InvalidCommandResponsePair => write!(
                formatter,
                "command-response pair contains invalid numerical or timing claims"
            ),
            Self::Serialization => write!(
                formatter,
                "failed to serialize canonical evaluation evidence"
            ),
        }
    }
}

impl std::error::Error for EvalError {}

fn sha256_serializable<T: Serialize>(domain: &[u8], value: &T) -> Result<String, EvalError> {
    let encoded = serde_json::to_vec(value).map_err(|_| EvalError::Serialization)?;
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update((encoded.len() as u64).to_le_bytes());
    hasher.update(encoded);
    Ok(format!("{:x}", hasher.finalize()))
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(recording: &str, sample_index: u64) -> SampleKey {
        SampleKey {
            recording_id: recording.to_owned(),
            sample_index,
        }
    }

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn calibration_scores(count: usize) -> Vec<EpisodeCalibrationScore> {
        (0..count)
            .map(|index| EpisodeCalibrationScore {
                recording_id: format!("cal-{index:02}"),
                model_sha256: digest('a'),
                regime_id: "wltp".into(),
                maximum_absolute_standardized_residual: index as f64 + 1.0,
            })
            .collect()
    }

    fn prediction(
        recording: &str,
        sample_index: u64,
        detection: bool,
        probabilities: [f64; 5],
    ) -> Prediction {
        Prediction {
            key: key(recording, sample_index),
            detection,
            isolation: FaultIsolationProbabilities { probabilities },
        }
    }

    #[test]
    fn signal_roles_keep_requested_commands_separate_from_measured_response() {
        let registry = liu_ice_signal_registry();
        let lookup = registry
            .iter()
            .map(|binding| (binding.signal_id.as_str(), binding))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(lookup["umf"].role, SignalRole::ControlIntent);
        assert_eq!(lookup["uwg"].role, SignalRole::ControlIntent);
        assert_eq!(lookup["yxpos"].role, SignalRole::CandidateActuatorFeedback);
        assert_eq!(lookup["ypic"].role, SignalRole::PhysicalResponse);
        assert_eq!(lookup["yomega"].role, SignalRole::PhysicalResponse);
        assert!(!lookup["yxpos"].instrument_map_reviewed);
        assert!(!registry.iter().any(|binding| binding.signal_id == "uwg"
            && binding.role == SignalRole::CandidateActuatorFeedback));
    }

    #[test]
    fn official_competition_score_reproduces_the_published_formula() {
        let labels = vec![
            LabeledSample {
                key: key("run-a", 0),
                truth: LiuIceTruth::NoFault,
            },
            LabeledSample {
                key: key("run-a", 1),
                truth: LiuIceTruth::NoFault,
            },
            LabeledSample {
                key: key("run-b", 0),
                truth: LiuIceTruth::KnownFault(FaultClass::FPic),
            },
            LabeledSample {
                key: key("run-b", 1),
                truth: LiuIceTruth::KnownFault(FaultClass::FPic),
            },
        ];
        let predictions = vec![
            prediction("run-a", 0, false, [0.0; 5]),
            prediction("run-a", 1, true, [0.4, 0.2, 0.2, 0.1, 0.1]),
            prediction("run-b", 0, true, [0.8, 0.05, 0.05, 0.05, 0.05]),
            prediction("run-b", 1, false, [0.0; 5]),
        ];
        let score = evaluate_competition_score(&labels, &predictions).unwrap();
        assert!((score.false_alarm_rate - 0.5).abs() < 1e-12);
        assert!((score.true_detection_rate - 0.5).abs() < 1e-12);
        assert!((score.fault_isolation_accuracy - 0.8).abs() < 1e-12);
        assert!((score.total_score - 0.6).abs() < 1e-12);
    }

    #[test]
    fn all_no_alarm_baseline_scores_one_third_and_reports_undefined_fia_denominator() {
        let labels = vec![
            LabeledSample {
                key: key("run-a", 0),
                truth: LiuIceTruth::NoFault,
            },
            LabeledSample {
                key: key("run-b", 0),
                truth: LiuIceTruth::KnownFault(FaultClass::FWaf),
            },
        ];
        let predictions = vec![
            prediction("run-a", 0, false, [0.0; 5]),
            prediction("run-b", 0, false, [0.0; 5]),
        ];
        let score = evaluate_competition_score(&labels, &predictions).unwrap();
        assert!((score.total_score - (1.0 / 3.0)).abs() < 1e-12);
        assert!(!score.fault_isolation_denominator_defined);
    }

    #[test]
    fn detection_false_requires_zero_isolation_probabilities() {
        let labels = vec![
            LabeledSample {
                key: key("run-a", 0),
                truth: LiuIceTruth::NoFault,
            },
            LabeledSample {
                key: key("run-b", 0),
                truth: LiuIceTruth::KnownFault(FaultClass::FPim),
            },
        ];
        let predictions = vec![
            prediction("run-a", 0, false, [0.0, 0.1, 0.0, 0.0, 0.0]),
            prediction("run-b", 0, true, [0.0, 0.0, 1.0, 0.0, 0.0]),
        ];
        assert!(matches!(
            evaluate_competition_score(&labels, &predictions),
            Err(EvalError::InvalidPrediction(_))
        ));
    }

    #[test]
    fn held_out_driving_cycle_cannot_cross_the_split_boundary() {
        let manifest = RecordingSplitManifest {
            format_version: 1,
            mode: SplitMode::HeldOutDrivingCycle,
            assignments: vec![
                RecordingAssignment {
                    recording_id: "a".into(),
                    driving_cycle: "WLTP".into(),
                    split: DatasetSplit::Train,
                },
                RecordingAssignment {
                    recording_id: "b".into(),
                    driving_cycle: "WLTP".into(),
                    split: DatasetSplit::Test,
                },
            ],
        };
        assert_eq!(
            manifest.validate(),
            Err(EvalError::DrivingCycleSplitLeakage("WLTP".into()))
        );
    }

    #[test]
    fn conformal_threshold_uses_independent_recording_maxima_and_finite_sample_rank() {
        let scores = calibration_scores(9);
        let threshold = fit_episode_conformal_threshold(
            &scores,
            900_000,
            &digest('a'),
            &digest('b'),
            &digest('c'),
            "wltp",
        )
        .unwrap();
        assert_eq!(threshold.order_statistic_rank_1_based, 9);
        assert_eq!(threshold.threshold, 9.0);
        assert_eq!(threshold.independent_calibration_episodes, 9);
    }

    #[test]
    fn conformal_calibration_fails_closed_when_finite_sample_coverage_is_impossible() {
        let scores = calibration_scores(8);
        assert_eq!(
            fit_episode_conformal_threshold(
                &scores,
                900_000,
                &digest('a'),
                &digest('b'),
                &digest('c'),
                "wltp",
            ),
            Err(EvalError::InsufficientCalibrationEpisodes {
                required: 9,
                available: 8
            })
        );
        assert_eq!(minimum_calibration_episodes(950_000), 19);
        assert_eq!(minimum_calibration_episodes(990_000), 99);
    }

    fn response_pair() -> CommandResponsePair {
        CommandResponsePair {
            recording_id: "held-out-run".into(),
            sample_index: 10,
            command_signal: "uwg".into(),
            response_signal: "ypic".into(),
            regime_id: "wltp".into(),
            model_sha256: digest('a'),
            training_recordings_manifest_sha256: digest('b'),
            split_manifest_sha256: digest('c'),
            command_value: Some(0.6),
            command_time_seconds: Some(0.45),
            command_timestamp_uncertainty_seconds: Some(0.0),
            prediction_time_seconds: 0.45,
            prediction_timestamp_uncertainty_seconds: 0.0,
            timebase_id: "held-out-run-clock".into(),
            timebase_qualified: true,
            timebase_qualification_sha256: digest('d'),
            response_value: Some(1.4),
            response_time_seconds: Some(0.5),
            response_timestamp_uncertainty_seconds: Some(0.0),
            maximum_response_horizon_seconds: 0.1,
            expected_response: Some(1.0),
            model_residual_std: 0.1,
            measurement_std: 0.1,
            maximum_command_age_seconds: 0.1,
            command_quality: SignalQuality::Current,
            response_quality: SignalQuality::Current,
            regime_supported: true,
        }
    }

    fn threshold() -> ConformalThreshold {
        fit_episode_conformal_threshold(
            &calibration_scores(9),
            900_000,
            &digest('a'),
            &digest('b'),
            &digest('c'),
            "wltp",
        )
        .unwrap()
    }

    fn split_bound_calibration() -> (ConformalThreshold, RecordingSplitManifest) {
        let manifest = RecordingSplitManifest {
            format_version: 1,
            mode: SplitMode::WholeRecording,
            assignments: (0..9)
                .map(|index| RecordingAssignment {
                    recording_id: format!("cal-{index:02}"),
                    driving_cycle: "WLTP".into(),
                    split: DatasetSplit::Calibration,
                })
                .chain((0..2).map(|index| RecordingAssignment {
                    recording_id: format!("train-{index:02}"),
                    driving_cycle: "WLTP".into(),
                    split: DatasetSplit::Train,
                }))
                .chain((0..2).map(|index| RecordingAssignment {
                    recording_id: format!("test-{index:02}"),
                    driving_cycle: "Artemis".into(),
                    split: DatasetSplit::Test,
                }))
                .collect(),
        };
        let training_digest = manifest
            .recording_manifest_digest(DatasetSplit::Train)
            .unwrap();
        let split_digest = manifest.digest().unwrap();
        let threshold = fit_episode_conformal_threshold(
            &calibration_scores(9),
            900_000,
            &digest('a'),
            &training_digest,
            &split_digest,
            "wltp",
        )
        .unwrap();
        (threshold, manifest)
    }

    #[test]
    fn calibration_receipt_must_match_the_exact_frozen_split_manifest() {
        let (threshold, manifest) = split_bound_calibration();
        assert!(validate_conformal_threshold_against_split(&threshold, &manifest).is_ok());

        let mut altered = manifest;
        altered.assignments[0].split = DatasetSplit::Train;
        assert_eq!(
            validate_conformal_threshold_against_split(&threshold, &altered),
            Err(EvalError::CalibrationBindingMismatch)
        );
    }

    #[test]
    fn response_mismatch_is_not_promoted_to_an_actuator_fault() {
        let mut pair = response_pair();
        pair.model_residual_std = 0.02;
        pair.measurement_std = 0.02;
        let result = evaluate_command_response(&pair, Some(&threshold())).unwrap();
        assert_eq!(
            result.status,
            ResponseStatus::MismatchOutsideCalibratedEnvelope
        );
        assert_eq!(result.cause_claim, CauseClaim::NotIsolated);
        assert_eq!(result.prediction_time_seconds, 0.45);
        assert!(result.standardized_absolute_residual.unwrap() > 2.0);
    }

    #[test]
    fn future_command_is_not_used_to_explain_past_response() {
        let mut pair = response_pair();
        pair.command_time_seconds = Some(0.46);
        let result = evaluate_command_response(&pair, Some(&threshold())).unwrap();
        assert_eq!(result.status, ResponseStatus::NotEvaluableFutureCommand);
        assert!(result.observed_residual.is_none());
    }

    #[test]
    fn prediction_time_is_between_command_and_physical_response() {
        let mut pair = response_pair();
        pair.response_time_seconds = Some(0.44);
        assert_eq!(
            evaluate_command_response(&pair, Some(&threshold()))
                .unwrap()
                .status,
            ResponseStatus::NotEvaluableResponseBeforePrediction
        );
        pair = response_pair();
        pair.response_time_seconds = Some(0.61);
        assert_eq!(
            evaluate_command_response(&pair, Some(&threshold()))
                .unwrap()
                .status,
            ResponseStatus::NotEvaluableResponseOutsideHorizon
        );
    }

    #[test]
    fn overlapping_command_prediction_interval_is_not_evaluable() {
        let mut pair = response_pair();
        pair.command_timestamp_uncertainty_seconds = Some(0.01);
        pair.prediction_timestamp_uncertainty_seconds = 0.01;
        let result = evaluate_command_response(&pair, Some(&threshold())).unwrap();
        assert_eq!(result.status, ResponseStatus::NotEvaluableTemporalOrder);
        assert!(result.observed_residual.is_none());
    }

    #[test]
    fn overlapping_prediction_response_interval_is_not_evaluable() {
        let mut pair = response_pair();
        pair.response_time_seconds = Some(0.46);
        pair.response_timestamp_uncertainty_seconds = Some(0.02);
        pair.prediction_timestamp_uncertainty_seconds = 0.02;
        let result = evaluate_command_response(&pair, Some(&threshold())).unwrap();
        assert_eq!(result.status, ResponseStatus::NotEvaluableTemporalOrder);
        assert!(result.observed_residual.is_none());
    }

    #[test]
    fn unqualified_timebase_never_proves_command_response_causality() {
        let mut pair = response_pair();
        pair.timebase_qualified = false;
        let result = evaluate_command_response(&pair, Some(&threshold())).unwrap();
        assert_eq!(result.status, ResponseStatus::NotEvaluableTimebase);
        assert!(result.temporal_evidence.is_none());
    }

    #[test]
    fn successful_residual_receipt_exposes_conservative_temporal_bounds() {
        let pair = response_pair();
        let result = evaluate_command_response(&pair, Some(&threshold())).unwrap();
        assert_eq!(result.status, ResponseStatus::MismatchOutsideCalibratedEnvelope);
        let evidence = result.temporal_evidence.unwrap();
        assert_eq!(evidence.command_available_earliest_seconds, 0.45);
        assert_eq!(evidence.prediction_latest_seconds, 0.45);
        assert!(evidence.response_lag_minimum_seconds > 0.0);
        assert!(evidence.response_lag_maximum_seconds >= evidence.response_lag_minimum_seconds);
    }

    #[test]
    fn missing_command_response_and_unsupported_regime_are_not_normal() {
        let mut missing_command = response_pair();
        missing_command.command_quality = SignalQuality::Missing;
        missing_command.command_value = None;
        assert_eq!(
            evaluate_command_response(&missing_command, Some(&threshold()))
                .unwrap()
                .status,
            ResponseStatus::NotEvaluableMissingCommand
        );

        let mut missing_response = response_pair();
        missing_response.response_quality = SignalQuality::Missing;
        missing_response.response_value = None;
        assert_eq!(
            evaluate_command_response(&missing_response, Some(&threshold()))
                .unwrap()
                .status,
            ResponseStatus::NotEvaluableMissingResponse
        );

        let mut unsupported = response_pair();
        unsupported.regime_supported = false;
        assert_eq!(
            evaluate_command_response(&unsupported, Some(&threshold()))
                .unwrap()
                .status,
            ResponseStatus::NotEvaluableUnsupportedRegime
        );
    }

    #[test]
    fn uncalibrated_or_stale_command_never_receives_a_normal_status() {
        let mut pair = response_pair();
        assert_eq!(
            evaluate_command_response(&pair, None).unwrap().status,
            ResponseStatus::NotEvaluableUncalibrated
        );
        pair.command_time_seconds = Some(0.1);
        assert_eq!(
            evaluate_command_response(&pair, Some(&threshold()))
                .unwrap()
                .status,
            ResponseStatus::NotEvaluableStaleCommand
        );
    }

    #[test]
    fn response_rejects_calibration_from_another_model_or_regime() {
        let mut pair = response_pair();
        pair.regime_id = "high_load".into();
        assert_eq!(
            evaluate_command_response(&pair, Some(&threshold())),
            Err(EvalError::CalibrationBindingMismatch)
        );
        let mut pair = response_pair();
        pair.model_sha256 = digest('e');
        assert_eq!(
            evaluate_command_response(&pair, Some(&threshold())),
            Err(EvalError::CalibrationBindingMismatch)
        );
    }

    #[test]
    fn calibration_threshold_replays_its_order_statistic_and_digests() {
        let mut fitted = threshold();
        assert!(fitted.validate().is_ok());
        fitted.calibration_max_scores[0] += 0.25;
        assert_eq!(fitted.validate(), Err(EvalError::InvalidConformalThreshold));
    }

    #[test]
    fn ninety_nine_percent_marginal_episode_coverage_needs_ninety_nine_recordings() {
        assert_eq!(minimum_calibration_episodes(990_000), 99);
        let scores = calibration_scores(98);
        assert_eq!(
            fit_episode_conformal_threshold(
                &scores,
                990_000,
                &digest('a'),
                &digest('b'),
                &digest('c'),
                "wltp",
            ),
            Err(EvalError::InsufficientCalibrationEpisodes {
                required: 99,
                available: 98
            })
        );
    }

    #[test]
    fn recording_rows_require_monotonic_time_and_preserve_missing_values() {
        let rows = vec![
            FeatureRow {
                key: key("run", 0),
                time_seconds: 0.0,
                signals: BTreeMap::from([("uwg".into(), 0.2)]),
            },
            FeatureRow {
                key: key("run", 1),
                time_seconds: 0.05,
                signals: BTreeMap::from([("uwg".into(), 0.25), ("ypic".into(), 1.1)]),
            },
        ];
        assert!(validate_recording_rows(&rows).is_ok());
        let mut reversed = rows.clone();
        reversed.swap(0, 1);
        assert_eq!(
            validate_recording_rows(&reversed),
            Err(EvalError::NonMonotonicSample)
        );
    }

    #[test]
    fn source_bound_split_digest_is_deterministic() {
        let manifest = RecordingSplitManifest {
            format_version: 1,
            mode: SplitMode::WholeRecording,
            assignments: vec![
                RecordingAssignment {
                    recording_id: "r1".into(),
                    driving_cycle: "WLTP".into(),
                    split: DatasetSplit::Train,
                },
                RecordingAssignment {
                    recording_id: "r2".into(),
                    driving_cycle: "Artemis".into(),
                    split: DatasetSplit::Test,
                },
            ],
        };
        assert!(manifest.validate().is_ok());
        assert_eq!(manifest.digest().unwrap(), manifest.digest().unwrap());
    }
}
