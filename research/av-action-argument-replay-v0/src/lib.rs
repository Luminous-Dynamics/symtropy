// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Canonical audiovisual acquisition arguments and replay receipts.
//!
//! This crate binds the exact requested crop/FPS/ROI or audio sample window
//! from an all-action forecast through request acceptance and action result.
//! It does not select actions, schedule sensors, or infer physical delivery.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Current manifest schema version.
pub const ACTION_ARGUMENT_SCHEMA_V1: u32 = 1;
/// Reserved action ID for the explicit stop/no-acquisition alternative.
pub const STOP_ACTION_ID_V1: &str = "__stop__";

/// Deterministic validation error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionArgumentError {
    message: String,
}

impl ActionArgumentError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for ActionArgumentError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ActionArgumentError {}

/// Immutable identity of a source media object and its clock generation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceIdentityV1 {
    /// Stable source/media identifier.
    pub source_id: String,
    /// SHA-256 digest of the source media or immutable source handle.
    pub source_content_sha256: String,
    /// Clock-domain identifier for all requested source timestamps.
    pub clock_domain_id: String,
    /// Clock-domain generation; changes after a reset/reconfiguration.
    pub clock_generation: u64,
}

/// Requested source-time interval, represented as nanoseconds in the declared clock.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceIntervalV1 {
    /// Inclusive start time in source-clock nanoseconds.
    pub start_ns: u64,
    /// Exclusive end time in source-clock nanoseconds.
    pub end_ns: u64,
}

/// Normalized region of interest encoded as integer basis points.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedRoiV1 {
    /// Left coordinate in basis points (0..=10,000).
    pub x_bp: u32,
    /// Top coordinate in basis points (0..=10,000).
    pub y_bp: u32,
    /// Width in basis points.
    pub width_bp: u32,
    /// Height in basis points.
    pub height_bp: u32,
}

/// Pixel-space region of interest with its source image dimensions.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PixelRoiV1 {
    /// Left pixel coordinate.
    pub x: u32,
    /// Top pixel coordinate.
    pub y: u32,
    /// ROI width in pixels.
    pub width: u32,
    /// ROI height in pixels.
    pub height: u32,
    /// Source frame width in pixels.
    pub source_width: u32,
    /// Source frame height in pixels.
    pub source_height: u32,
}

/// Region of interest with explicit coordinate system.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "coordinate_space",
    content = "region",
    rename_all = "snake_case"
)]
pub enum RegionOfInterestV1 {
    /// Normalized [0,1] coordinates.
    Normalized(NormalizedRoiV1),
    /// Source pixel coordinates.
    Pixels(PixelRoiV1),
}

/// Requested output resolution.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OutputResolutionV1 {
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
}

/// Fixed-rate or explicit timestamp video sampling plan.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "sampling_mode", rename_all = "snake_case")]
pub enum VideoSamplingV1 {
    /// Rational frame rate and exact expected schedule length.
    FixedRate {
        /// FPS numerator, e.g. 30 for 30/1 or 60000 for 60000/1001.
        fps_numerator: u32,
        /// FPS denominator, e.g. 1001 for 60000/1001.
        fps_denominator: u32,
        /// First scheduled capture timestamp in source-clock nanoseconds.
        phase_ns: u64,
        /// Number of scheduled frames in this request.
        frame_count: u32,
    },
    /// Exact precommitted timestamp list.
    ExplicitTimestamps {
        /// Strictly increasing capture timestamps, in source-clock nanoseconds.
        timestamps_ns: Vec<u64>,
    },
}

/// Video crop / frame acquisition parameters.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VideoCropParametersV1 {
    /// Sampling-rate or timestamp-list request.
    pub sampling: VideoSamplingV1,
    /// Optional crop region.
    pub roi: Option<RegionOfInterestV1>,
    /// Requested output resolution.
    pub output_resolution: OutputResolutionV1,
    /// Stable crop/tiling policy identifier.
    pub crop_policy_id: String,
    /// SHA-256 digest of crop/tiling policy implementation and configuration.
    pub crop_policy_sha256: String,
    /// SHA-256 digest of preprocessing/resizing implementation and configuration.
    pub preprocessing_sha256: String,
}

/// Audio sample format.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioSampleFormatV1 {
    /// Signed 16-bit little-endian PCM.
    S16Le,
    /// Signed 24-bit little-endian PCM.
    S24Le,
    /// Signed 32-bit little-endian PCM.
    S32Le,
    /// 32-bit floating-point little-endian PCM.
    F32Le,
}

/// Requested audio acquisition window. End sample is exclusive.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AudioWindowParametersV1 {
    /// First sample index, inclusive.
    pub start_sample: u64,
    /// Last sample index, exclusive.
    pub end_sample: u64,
    /// Samples per second.
    pub sample_rate_hz: u32,
    /// Number of channels.
    pub channels: u16,
    /// PCM sample format.
    pub sample_format: AudioSampleFormatV1,
    /// Requested chunk size in frames.
    pub chunk_frames: u32,
    /// Requested overlap between successive chunks in frames.
    pub overlap_frames: u32,
    /// SHA-256 digest of any filtering/resampling/preprocessing transform.
    pub transform_sha256: String,
}

/// Non-sensor request for external user or simulator feedback.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FeedbackRequestParametersV1 {
    /// Expected source role for the response.
    pub source_role: FeedbackSourceRoleV1,
    /// Expected feedback type.
    pub feedback_kind: FeedbackKindV1,
    /// Earliest expected logical delivery tick.
    pub expected_delivery_tick: u64,
}

/// External feedback source role.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackSourceRoleV1 {
    /// Benchmark user simulator.
    UserSimulator,
    /// Human user.
    HumanUser,
    /// Authorized operator.
    Operator,
}

/// External feedback kind.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackKindV1 {
    /// Hint.
    Hint,
    /// Clarification.
    Clarification,
    /// Correction.
    Correction,
    /// Challenge.
    Challenge,
    /// Request.
    Request,
}

/// Typed action-specific argument payload.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "action_kind", content = "parameters", rename_all = "snake_case")]
pub enum ActionParametersV1 {
    /// Video crop/frame sampling request.
    VideoCrop(VideoCropParametersV1),
    /// Audio sample-window request.
    AudioWindow(AudioWindowParametersV1),
    /// External feedback request; its response is not a sensor observation.
    FeedbackRequest(FeedbackRequestParametersV1),
}

/// Canonical, digest-bound parameters for a single acquisition action.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionParameterManifestV1 {
    /// Schema version.
    pub schema_version: u32,
    /// Stable action ID from the candidate-visible action catalogue.
    pub action_id: String,
    /// Immutable source identity and source clock generation.
    pub source: SourceIdentityV1,
    /// Requested interval in the source clock. Feedback requests use the interval
    /// during which the response is expected, not a sensor capture interval.
    pub requested_interval: SourceIntervalV1,
    /// Permitted capture timestamp deviation from the requested schedule.
    pub timestamp_tolerance_ns: u64,
    /// Typed action parameters.
    pub parameters: ActionParametersV1,
}

/// Frame actually delivered by a video acquisition.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveredFrameV1 {
    /// Immutable observation/frame identity.
    pub observation_ref: String,
    /// Frame sequence index within this request.
    pub frame_index: u32,
    /// Capture timestamp in the requested source clock.
    pub capture_timestamp_ns: u64,
    /// Delivered byte count.
    pub byte_count: u64,
}

/// Audio sample range actually delivered. End sample is exclusive.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveredAudioRangeV1 {
    /// First delivered sample, inclusive.
    pub start_sample: u64,
    /// End delivered sample, exclusive.
    pub end_sample: u64,
    /// Delivered sample rate.
    pub sample_rate_hz: u32,
    /// Delivered channel count.
    pub channels: u16,
    /// Delivered PCM format.
    pub sample_format: AudioSampleFormatV1,
}

/// Feedback event delivered by a feedback-request action.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveredFeedbackV1 {
    /// Unique feedback event ID.
    pub feedback_id: String,
    /// Actual source role.
    pub source_role: FeedbackSourceRoleV1,
    /// Actual feedback kind.
    pub feedback_kind: FeedbackKindV1,
    /// Logical tick at which feedback entered the candidate transcript.
    pub delivered_tick: u64,
    /// SHA-256 digest of the exact candidate-visible feedback content.
    pub content_sha256: String,
}

/// Result status for one requested action.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionResultStatusV1 {
    /// Complete result delivered.
    Delivered,
    /// Partial result delivered.
    Partial,
    /// Result arrived after the decision deadline.
    Late,
    /// Requested source/evidence was unavailable.
    Unavailable,
    /// Action failed.
    Failed,
    /// Action was cancelled.
    Cancelled,
}

/// Request-acceptance receipt binding the selected action to the accepted request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequestAcceptanceV1 {
    /// Episode identity.
    pub episode_id: String,
    /// Candidate turn identity.
    pub turn_id: String,
    /// Unique executor request ID.
    pub request_id: String,
    /// Selected catalogue action ID.
    pub action_id: String,
    /// Canonical manifest digest selected before the outcome.
    pub parameter_manifest_sha256: String,
    /// Candidate transcript head against which the action was requested.
    pub parent_transcript_head_sha256: String,
    /// Logical acceptance tick.
    pub accepted_tick: u64,
    /// Expected arrival tick.
    pub expected_arrival_tick: u64,
    /// Decision deadline for this episode.
    pub decision_deadline_tick: u64,
}

/// Action result receipt. The manifest digest must be unchanged from forecast and acceptance.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionResultV1 {
    /// Episode identity.
    pub episode_id: String,
    /// Candidate turn identity.
    pub turn_id: String,
    /// Unique executor request ID.
    pub request_id: String,
    /// Selected action ID.
    pub action_id: String,
    /// Canonical manifest digest.
    pub parameter_manifest_sha256: String,
    /// Candidate transcript head on which this result is causally based.
    pub parent_transcript_head_sha256: String,
    /// Actual result status.
    pub status: ActionResultStatusV1,
    /// Logical tick when the executor emitted this result.
    pub result_tick: u64,
    /// Delivered video frames, when this was a video action.
    pub delivered_frames: Vec<DeliveredFrameV1>,
    /// Actual region of interest applied by the executor for a video action.
    pub delivered_roi: Option<RegionOfInterestV1>,
    /// Actual output resolution applied by the executor for a video action.
    pub delivered_output_resolution: Option<OutputResolutionV1>,
    /// Digest of the actual crop policy applied by the executor.
    pub delivered_crop_policy_sha256: Option<String>,
    /// Digest of the actual preprocessing transform applied by the executor.
    pub delivered_preprocessing_sha256: Option<String>,
    /// Delivered audio sample range, when this was an audio action.
    pub delivered_audio: Option<DeliveredAudioRangeV1>,
    /// Delivered feedback, when this was a feedback action.
    pub delivered_feedback: Option<DeliveredFeedbackV1>,
    /// Actual resource usage in bytes/frames/samples/cost units, provided by executor.
    pub actual_resource_digest_sha256: String,
    /// Result event digest from the event-sourced executor.
    pub result_event_sha256: String,
}

/// One action manifest entry in the all-action forecast catalogue.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ForecastActionManifestV1 {
    /// Stable action ID.
    pub action_id: String,
    /// Full action parameter manifest.
    pub manifest: ActionParameterManifestV1,
    /// Digest of the exact canonical manifest.
    pub manifest_sha256: String,
}

/// Candidate selection bound to one of the forecasted manifests.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedActionManifestV1 {
    /// Selected action ID.
    pub action_id: String,
    /// Selected canonical manifest digest.
    pub manifest_sha256: String,
}

/// One pre-outcome value forecast for an eligible acquisition or stop.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionEvsiForecastV1 {
    /// Action ID, or the reserved stop/no-acquisition ID.
    pub action_id: String,
    /// Point estimate of gross EVSI.
    pub estimated_gross_evsi: f64,
    /// Lower forecast interval bound.
    pub lower_gross_evsi: f64,
    /// Upper forecast interval bound.
    pub upper_gross_evsi: f64,
}

/// Full all-action parameter catalogue and selected action for one transcript head.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionParameterForecastV1 {
    /// Episode identity.
    pub episode_id: String,
    /// Turn identity.
    pub turn_id: String,
    /// Candidate transcript head.
    pub transcript_head_sha256: String,
    /// Complete currently eligible acquisition action IDs, excluding reserved stop.
    pub eligible_action_ids: Vec<String>,
    /// Complete per-action parameter manifests for every eligible acquisition action.
    pub actions: Vec<ForecastActionManifestV1>,
    /// Complete #6753-style gross EVSI forecasts, including explicit stop.
    pub evsi_forecasts: Vec<ActionEvsiForecastV1>,
    /// Selected action ID. May be the reserved stop ID.
    pub selected_action_id: String,
    /// Selected manifest digest. None only when selecting stop/no-acquisition.
    pub selected_manifest_sha256: Option<String>,
}

/// Digest an exact manifest using the canonical serde JSON representation.
/// Struct fields have fixed declaration order; there are no free-form maps in v1.
pub fn manifest_sha256(
    manifest: &ActionParameterManifestV1,
) -> Result<String, ActionArgumentError> {
    let canonical_value = serde_json::to_value(manifest).map_err(|error| {
        ActionArgumentError::new(format!("manifest serialization failed: {error}"))
    })?;
    let encoded = serde_json::to_vec(&canonical_value).map_err(|error| {
        ActionArgumentError::new(format!("canonical manifest encoding failed: {error}"))
    })?;
    Ok(hex_sha256(&encoded))
}

/// Validate a manifest and return its canonical digest.
pub fn validate_manifest(
    manifest: &ActionParameterManifestV1,
) -> Result<String, ActionArgumentError> {
    validate_manifest_fields(manifest)?;
    manifest_sha256(manifest)
}

#[derive(Serialize)]
struct ForecastDigestMaterialV1<'a> {
    episode_id: &'a str,
    turn_id: &'a str,
    transcript_head_sha256: &'a str,
    eligible_action_ids: Vec<String>,
    action_manifests: Vec<(String, String)>,
    selected_action_id: &'a str,
    selected_manifest_sha256: Option<&'a str>,
}

/// Validate every forecasted manifest, require one unique entry per action ID,
/// and prove the selected action is one of the exact forecasted manifests.
pub fn validate_forecast(
    forecast: &ActionParameterForecastV1,
) -> Result<String, ActionArgumentError> {
    require_text(&forecast.episode_id, "episode_id")?;
    require_text(&forecast.transcript_head_sha256, "transcript_head_sha256")?;
    validate_digest(&forecast.transcript_head_sha256, "transcript_head_sha256")?;
    if forecast.actions.is_empty() {
        return Err(ActionArgumentError::new(
            "forecast acquisition catalogue cannot be empty",
        ));
    }
    if forecast.eligible_action_ids.is_empty() {
        return Err(ActionArgumentError::new(
            "eligible_action_ids cannot be empty",
        ));
    }
    let eligible_ids: BTreeSet<String> = forecast.eligible_action_ids.iter().cloned().collect();
    if eligible_ids.len() != forecast.eligible_action_ids.len()
        || eligible_ids.contains(STOP_ACTION_ID_V1)
    {
        return Err(ActionArgumentError::new(
            "eligible action IDs must be unique and cannot contain the reserved stop ID",
        ));
    }
    let mut action_ids = BTreeSet::new();
    for entry in &forecast.actions {
        require_text(&entry.action_id, "forecast action_id")?;
        if !action_ids.insert(entry.action_id.clone()) {
            return Err(ActionArgumentError::new(
                "forecast contains duplicate action IDs",
            ));
        }
        if entry.manifest.action_id != entry.action_id {
            return Err(ActionArgumentError::new(
                "forecast action ID differs from manifest action ID",
            ));
        }
        let digest = validate_manifest(&entry.manifest)?;
        validate_digest(&entry.manifest_sha256, "forecast manifest digest")?;
        if digest != entry.manifest_sha256.to_ascii_lowercase() {
            return Err(ActionArgumentError::new(
                "forecast manifest digest mismatch",
            ));
        }
    }
    if action_ids != eligible_ids {
        return Err(ActionArgumentError::new(
            "forecast action manifests do not exactly cover the eligible action catalogue",
        ));
    }
    let mut forecast_ids = BTreeSet::new();
    for entry in &forecast.evsi_forecasts {
        require_text(&entry.action_id, "EVSI forecast action_id")?;
        if !forecast_ids.insert(entry.action_id.clone()) {
            return Err(ActionArgumentError::new(
                "EVSI forecast contains duplicate action IDs",
            ));
        }
        let values = [
            entry.estimated_gross_evsi,
            entry.lower_gross_evsi,
            entry.upper_gross_evsi,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(ActionArgumentError::new(
                "EVSI forecast values must be finite",
            ));
        }
        if entry.lower_gross_evsi > entry.estimated_gross_evsi
            || entry.estimated_gross_evsi > entry.upper_gross_evsi
        {
            return Err(ActionArgumentError::new(
                "EVSI forecast interval is inconsistent",
            ));
        }
        if entry.action_id == STOP_ACTION_ID_V1 && values.iter().any(|value| *value != 0.0) {
            return Err(ActionArgumentError::new(
                "stop/no-acquisition EVSI forecast must be zero",
            ));
        }
    }
    let mut expected_forecast_ids = eligible_ids.clone();
    expected_forecast_ids.insert(STOP_ACTION_ID_V1.to_string());
    if forecast_ids != expected_forecast_ids {
        return Err(ActionArgumentError::new(
            "EVSI forecasts must cover every eligible action plus explicit stop",
        ));
    }
    require_text(&forecast.selected_action_id, "selected_action_id")?;
    if forecast.selected_action_id == STOP_ACTION_ID_V1 {
        if forecast.selected_manifest_sha256.is_some() {
            return Err(ActionArgumentError::new(
                "stop/no-acquisition selection must not carry an action-parameter digest",
            ));
        }
    } else {
        let selected = forecast
            .actions
            .iter()
            .find(|entry| entry.action_id == forecast.selected_action_id)
            .ok_or_else(|| {
                ActionArgumentError::new(
                    "selected action is not present in the forecasted parameter catalogue",
                )
            })?;
        let selected_digest = forecast
            .selected_manifest_sha256
            .as_deref()
            .ok_or_else(|| {
                ActionArgumentError::new(
                    "selected acquisition requires a parameter manifest digest",
                )
            })?;
        validate_digest(selected_digest, "selected manifest digest")?;
        if selected.manifest_sha256 != selected_digest {
            return Err(ActionArgumentError::new(
                "selected manifest digest differs from forecast catalogue entry",
            ));
        }
    }
    let mut eligible_ids: Vec<String> = forecast.eligible_action_ids.clone();
    eligible_ids.sort();
    let mut manifest_entries: Vec<(String, String)> = forecast
        .actions
        .iter()
        .map(|entry| {
            (
                entry.action_id.clone(),
                entry.manifest_sha256.to_ascii_lowercase(),
            )
        })
        .collect();
    manifest_entries.sort_by(|left, right| left.0.cmp(&right.0));
    let material = ForecastDigestMaterialV1 {
        episode_id: &forecast.episode_id,
        turn_id: &forecast.turn_id,
        transcript_head_sha256: &forecast.transcript_head_sha256,
        eligible_action_ids: eligible_ids,
        action_manifests: manifest_entries,
        selected_action_id: &forecast.selected_action_id,
        selected_manifest_sha256: forecast.selected_manifest_sha256.as_deref(),
    };
    let canonical_value = serde_json::to_value(&material).map_err(|error| {
        ActionArgumentError::new(format!("forecast serialization failed: {error}"))
    })?;
    let encoded = serde_json::to_vec(&canonical_value).map_err(|error| {
        ActionArgumentError::new(format!("canonical forecast encoding failed: {error}"))
    })?;
    Ok(hex_sha256(&encoded))
}

/// Validate acceptance and result receipts against the selected forecast manifest.
/// All identifiers, transcript head and parameter digests must be unchanged.
pub fn validate_action_lifecycle(
    forecast: &ActionParameterForecastV1,
    acceptance: &RequestAcceptanceV1,
    result: &ActionResultV1,
) -> Result<(), ActionArgumentError> {
    validate_forecast(forecast)?;
    if forecast.selected_action_id == STOP_ACTION_ID_V1 {
        return Err(ActionArgumentError::new(
            "stop/no-acquisition has no request acceptance or action result lifecycle",
        ));
    }
    let selected_action_id = &forecast.selected_action_id;
    let selected_digest = forecast
        .selected_manifest_sha256
        .as_deref()
        .ok_or_else(|| {
            ActionArgumentError::new("selected action is missing its manifest digest")
        })?;
    let manifest = forecast
        .actions
        .iter()
        .find(|entry| entry.action_id == *selected_action_id)
        .ok_or_else(|| ActionArgumentError::new("selected manifest disappeared"))?;

    if acceptance.episode_id != forecast.episode_id
        || acceptance.turn_id != forecast.turn_id
        || acceptance.action_id != *selected_action_id
        || acceptance.parameter_manifest_sha256 != selected_digest
        || acceptance.parent_transcript_head_sha256 != forecast.transcript_head_sha256
    {
        return Err(ActionArgumentError::new(
            "request acceptance does not match selected forecast action",
        ));
    }
    require_text(&acceptance.request_id, "acceptance request_id")?;
    validate_digest(
        &acceptance.parameter_manifest_sha256,
        "acceptance manifest digest",
    )?;
    validate_digest(
        &acceptance.parent_transcript_head_sha256,
        "acceptance parent transcript head",
    )?;
    if acceptance.expected_arrival_tick < acceptance.accepted_tick {
        return Err(ActionArgumentError::new(
            "expected arrival tick precedes request acceptance",
        ));
    }
    if acceptance.expected_arrival_tick > acceptance.decision_deadline_tick {
        return Err(ActionArgumentError::new(
            "selected action is predicted to arrive after the decision deadline",
        ));
    }

    if result.episode_id != acceptance.episode_id
        || result.turn_id != acceptance.turn_id
        || result.request_id != acceptance.request_id
        || result.action_id != acceptance.action_id
        || result.parameter_manifest_sha256 != acceptance.parameter_manifest_sha256
        || result.parent_transcript_head_sha256 != acceptance.parent_transcript_head_sha256
    {
        return Err(ActionArgumentError::new(
            "action result receipt does not match accepted request",
        ));
    }
    validate_digest(&result.parameter_manifest_sha256, "result manifest digest")?;
    validate_digest(
        &result.parent_transcript_head_sha256,
        "result parent transcript head",
    )?;
    validate_digest(
        &result.actual_resource_digest_sha256,
        "actual resource digest",
    )?;
    validate_digest(&result.result_event_sha256, "result event digest")?;
    if result.result_tick < acceptance.expected_arrival_tick {
        return Err(ActionArgumentError::new(
            "action result arrived before its expected arrival tick",
        ));
    }
    if result.status == ActionResultStatusV1::Late
        && result.result_tick <= acceptance.decision_deadline_tick
    {
        return Err(ActionArgumentError::new(
            "late result tick is not after the decision deadline",
        ));
    }
    if matches!(
        result.status,
        ActionResultStatusV1::Delivered | ActionResultStatusV1::Partial
    ) && result.result_tick > acceptance.decision_deadline_tick
    {
        return Err(ActionArgumentError::new(
            "post-deadline delivery must be marked late and cannot be decision-eligible",
        ));
    }
    validate_result_payload(&manifest.manifest, result, acceptance.expected_arrival_tick)?;
    Ok(())
}

/// Derive the exact expected video frame times for a manifest.
pub fn expected_video_timestamps(
    manifest: &ActionParameterManifestV1,
) -> Result<Vec<u64>, ActionArgumentError> {
    validate_manifest_fields(manifest)?;
    let ActionParametersV1::VideoCrop(parameters) = &manifest.parameters else {
        return Err(ActionArgumentError::new("action is not a video crop"));
    };
    sampling_timestamps(&parameters.sampling)
}

fn sampling_timestamps(sampling: &VideoSamplingV1) -> Result<Vec<u64>, ActionArgumentError> {
    match sampling {
        VideoSamplingV1::FixedRate {
            fps_numerator,
            fps_denominator,
            phase_ns,
            frame_count,
        } => {
            if *fps_numerator == 0 || *fps_denominator == 0 || *frame_count == 0 {
                return Err(ActionArgumentError::new(
                    "fixed-rate FPS numerator/denominator/frame_count must be positive",
                ));
            }
            let scale = 1_000_000_000_u128
                .checked_mul(u128::from(*fps_denominator))
                .ok_or_else(|| ActionArgumentError::new("FPS period overflow"))?;
            let mut timestamps = Vec::with_capacity(*frame_count as usize);
            for index in 0..*frame_count {
                let numerator = u128::from(index)
                    .checked_mul(scale)
                    .ok_or_else(|| ActionArgumentError::new("video timestamp overflow"))?;
                let delta = numerator / u128::from(*fps_numerator);
                let timestamp = u128::from(*phase_ns)
                    .checked_add(delta)
                    .ok_or_else(|| ActionArgumentError::new("video timestamp overflow"))?;
                let timestamp = u64::try_from(timestamp)
                    .map_err(|_| ActionArgumentError::new("video timestamp exceeds u64"))?;
                timestamps.push(timestamp);
            }
            Ok(timestamps)
        }
        VideoSamplingV1::ExplicitTimestamps { timestamps_ns } => {
            if timestamps_ns.is_empty() {
                return Err(ActionArgumentError::new(
                    "explicit video timestamp list cannot be empty",
                ));
            }
            if timestamps_ns.windows(2).any(|pair| pair[1] <= pair[0]) {
                return Err(ActionArgumentError::new(
                    "explicit video timestamps must be strictly increasing",
                ));
            }
            Ok(timestamps_ns.clone())
        }
    }
}

fn validate_manifest_fields(
    manifest: &ActionParameterManifestV1,
) -> Result<(), ActionArgumentError> {
    if manifest.schema_version != ACTION_ARGUMENT_SCHEMA_V1 {
        return Err(ActionArgumentError::new(
            "unsupported action manifest schema_version",
        ));
    }
    require_text(&manifest.action_id, "action_id")?;
    require_text(&manifest.source.source_id, "source_id")?;
    validate_digest(
        &manifest.source.source_content_sha256,
        "source content digest",
    )?;
    require_text(&manifest.source.clock_domain_id, "clock_domain_id")?;
    if manifest.requested_interval.end_ns <= manifest.requested_interval.start_ns {
        return Err(ActionArgumentError::new(
            "requested source interval is empty or reversed",
        ));
    }

    match &manifest.parameters {
        ActionParametersV1::VideoCrop(parameters) => {
            if parameters.output_resolution.width == 0 || parameters.output_resolution.height == 0 {
                return Err(ActionArgumentError::new(
                    "video output resolution must be positive",
                ));
            }
            require_text(&parameters.crop_policy_id, "crop_policy_id")?;
            validate_digest(&parameters.crop_policy_sha256, "crop_policy_sha256")?;
            validate_digest(&parameters.preprocessing_sha256, "preprocessing_sha256")?;
            if let Some(roi) = &parameters.roi {
                validate_roi(roi)?;
            }
            let expected = sampling_timestamps(&parameters.sampling)?;
            if expected.iter().any(|timestamp| {
                *timestamp < manifest.requested_interval.start_ns
                    || *timestamp >= manifest.requested_interval.end_ns
            }) {
                return Err(ActionArgumentError::new(
                    "requested frame timestamp lies outside the source interval",
                ));
            }
        }
        ActionParametersV1::AudioWindow(parameters) => {
            if parameters.sample_rate_hz == 0 || parameters.channels == 0 {
                return Err(ActionArgumentError::new(
                    "audio sample rate and channel count must be positive",
                ));
            }
            if parameters.end_sample <= parameters.start_sample {
                return Err(ActionArgumentError::new(
                    "audio sample range is empty or reversed",
                ));
            }
            if parameters.chunk_frames == 0 || parameters.overlap_frames >= parameters.chunk_frames
            {
                return Err(ActionArgumentError::new(
                    "audio chunk size must be positive and overlap smaller than chunk",
                ));
            }
            validate_digest(&parameters.transform_sha256, "audio transform digest")?;
            let start_ns = sample_to_ns(parameters.start_sample, parameters.sample_rate_hz)?;
            let end_ns = sample_to_ns(parameters.end_sample, parameters.sample_rate_hz)?;
            let tolerance_ns = 1_000_000_000_u64 / u64::from(parameters.sample_rate_hz) + 1;
            if abs_diff(start_ns, manifest.requested_interval.start_ns) > tolerance_ns
                || abs_diff(end_ns, manifest.requested_interval.end_ns) > tolerance_ns
            {
                return Err(ActionArgumentError::new(
                    "audio sample range does not match requested source-time interval",
                ));
            }
        }
        ActionParametersV1::FeedbackRequest(parameters) => {
            if parameters.expected_delivery_tick == 0 {
                return Err(ActionArgumentError::new(
                    "feedback expected_delivery_tick must be positive",
                ));
            }
        }
    }
    Ok(())
}

fn validate_roi(roi: &RegionOfInterestV1) -> Result<(), ActionArgumentError> {
    match roi {
        RegionOfInterestV1::Normalized(region) => {
            if region.width_bp == 0
                || region.height_bp == 0
                || region
                    .x_bp
                    .checked_add(region.width_bp)
                    .is_none_or(|end| end > 10_000)
                || region
                    .y_bp
                    .checked_add(region.height_bp)
                    .is_none_or(|end| end > 10_000)
            {
                return Err(ActionArgumentError::new(
                    "normalized ROI must be a positive rectangle inside [0,1]",
                ));
            }
        }
        RegionOfInterestV1::Pixels(region) => {
            if region.source_width == 0
                || region.source_height == 0
                || region.width == 0
                || region.height == 0
                || region
                    .x
                    .checked_add(region.width)
                    .is_none_or(|end| end > region.source_width)
                || region
                    .y
                    .checked_add(region.height)
                    .is_none_or(|end| end > region.source_height)
            {
                return Err(ActionArgumentError::new(
                    "pixel ROI must be a positive rectangle inside source dimensions",
                ));
            }
        }
    }
    Ok(())
}

fn validate_result_payload(
    manifest: &ActionParameterManifestV1,
    result: &ActionResultV1,
    expected_arrival_tick: u64,
) -> Result<(), ActionArgumentError> {
    match &manifest.parameters {
        ActionParametersV1::VideoCrop(parameters) => {
            if result.delivered_audio.is_some() || result.delivered_feedback.is_some() {
                return Err(ActionArgumentError::new(
                    "video action result cannot carry audio or external feedback payload",
                ));
            }
            let schedule = sampling_timestamps(&parameters.sampling)?;
            if !result.delivered_frames.is_empty() {
                if result.delivered_roi != parameters.roi
                    || result.delivered_output_resolution != Some(parameters.output_resolution)
                    || result.delivered_crop_policy_sha256.as_deref()
                        != Some(parameters.crop_policy_sha256.as_str())
                    || result.delivered_preprocessing_sha256.as_deref()
                        != Some(parameters.preprocessing_sha256.as_str())
                {
                    return Err(ActionArgumentError::new(
                        "actual video ROI/resolution/crop/preprocessing differs from requested manifest",
                    ));
                }
            } else if result.delivered_roi.is_some()
                || result.delivered_output_resolution.is_some()
                || result.delivered_crop_policy_sha256.is_some()
                || result.delivered_preprocessing_sha256.is_some()
            {
                return Err(ActionArgumentError::new(
                    "video transform receipt cannot exist without delivered frames",
                ));
            }
            let mut refs = BTreeSet::new();
            let mut indices = BTreeSet::new();
            let mut timestamps = BTreeSet::new();
            for frame in &result.delivered_frames {
                require_text(&frame.observation_ref, "delivered frame observation_ref")?;
                if frame.byte_count == 0
                    || !refs.insert(frame.observation_ref.clone())
                    || !indices.insert(frame.frame_index)
                    || !timestamps.insert(frame.capture_timestamp_ns)
                    || frame.capture_timestamp_ns < manifest.requested_interval.start_ns
                    || frame.capture_timestamp_ns >= manifest.requested_interval.end_ns
                    || schedule
                        .get(frame.frame_index as usize)
                        .is_none_or(|expected| {
                            abs_diff(*expected, frame.capture_timestamp_ns)
                                > manifest.timestamp_tolerance_ns
                        })
                {
                    return Err(ActionArgumentError::new(
                        "delivered frame identity/index/time is not a unique member of the request schedule",
                    ));
                }
            }
            if result.status == ActionResultStatusV1::Delivered
                && result.delivered_frames.len() != schedule.len()
            {
                return Err(ActionArgumentError::new(
                    "complete video delivery must contain every requested frame",
                ));
            }
            if matches!(
                result.status,
                ActionResultStatusV1::Failed
                    | ActionResultStatusV1::Unavailable
                    | ActionResultStatusV1::Cancelled
            ) && !result.delivered_frames.is_empty()
            {
                return Err(ActionArgumentError::new(
                    "failed/unavailable/cancelled video action cannot deliver frames",
                ));
            }
            if result.status == ActionResultStatusV1::Partial
                && (result.delivered_frames.is_empty()
                    || result.delivered_frames.len() >= schedule.len())
            {
                return Err(ActionArgumentError::new(
                    "partial video result must retain a non-empty strict subset of requested frames",
                ));
            }
        }
        ActionParametersV1::AudioWindow(parameters) => {
            if !result.delivered_frames.is_empty()
                || result.delivered_roi.is_some()
                || result.delivered_output_resolution.is_some()
                || result.delivered_crop_policy_sha256.is_some()
                || result.delivered_preprocessing_sha256.is_some()
                || result.delivered_feedback.is_some()
            {
                return Err(ActionArgumentError::new(
                    "audio action result cannot carry video frames or external feedback",
                ));
            }
            if let Some(delivered) = result.delivered_audio {
                if delivered.sample_rate_hz != parameters.sample_rate_hz
                    || delivered.channels != parameters.channels
                    || delivered.sample_format != parameters.sample_format
                    || delivered.start_sample < parameters.start_sample
                    || delivered.end_sample > parameters.end_sample
                    || delivered.end_sample <= delivered.start_sample
                {
                    return Err(ActionArgumentError::new(
                        "delivered audio sample range does not match the requested source window",
                    ));
                }
                if result.status == ActionResultStatusV1::Delivered
                    && (delivered.start_sample != parameters.start_sample
                        || delivered.end_sample != parameters.end_sample)
                {
                    return Err(ActionArgumentError::new(
                        "complete audio delivery must contain the entire requested sample range",
                    ));
                }
                if result.status == ActionResultStatusV1::Partial
                    && delivered.start_sample == parameters.start_sample
                    && delivered.end_sample == parameters.end_sample
                {
                    return Err(ActionArgumentError::new(
                        "partial audio delivery must be a strict subrange of the request",
                    ));
                }
            } else if matches!(
                result.status,
                ActionResultStatusV1::Delivered | ActionResultStatusV1::Partial
            ) {
                return Err(ActionArgumentError::new(
                    "delivered/partial audio result requires a sample-range receipt",
                ));
            }
        }
        ActionParametersV1::FeedbackRequest(parameters) => {
            if !result.delivered_frames.is_empty()
                || result.delivered_roi.is_some()
                || result.delivered_output_resolution.is_some()
                || result.delivered_crop_policy_sha256.is_some()
                || result.delivered_preprocessing_sha256.is_some()
                || result.delivered_audio.is_some()
            {
                return Err(ActionArgumentError::new(
                    "feedback request cannot produce a sensor observation payload",
                ));
            }
            if let Some(feedback) = &result.delivered_feedback {
                if feedback.source_role != parameters.source_role
                    || feedback.feedback_kind != parameters.feedback_kind
                    || feedback.delivered_tick < parameters.expected_delivery_tick
                    || feedback.delivered_tick < expected_arrival_tick
                    || feedback.delivered_tick != result.result_tick
                {
                    return Err(ActionArgumentError::new(
                        "delivered feedback does not match the requested source/type/arrival",
                    ));
                }
                require_text(&feedback.feedback_id, "feedback_id")?;
                validate_digest(&feedback.content_sha256, "feedback content digest")?;
            } else if result.status == ActionResultStatusV1::Delivered {
                return Err(ActionArgumentError::new(
                    "complete feedback delivery requires a feedback event receipt",
                ));
            }
        }
    }

    if result.status == ActionResultStatusV1::Late
        && result.delivered_frames.is_empty()
        && result.delivered_audio.is_none()
        && result.delivered_feedback.is_none()
    {
        return Err(ActionArgumentError::new(
            "late result must retain the late payload receipt",
        ));
    }
    if matches!(
        result.status,
        ActionResultStatusV1::Failed
            | ActionResultStatusV1::Unavailable
            | ActionResultStatusV1::Cancelled
    ) && (result.delivered_audio.is_some()
        || result.delivered_feedback.is_some()
        || !result.delivered_frames.is_empty()
        || result.delivered_roi.is_some()
        || result.delivered_output_resolution.is_some()
        || result.delivered_crop_policy_sha256.is_some()
        || result.delivered_preprocessing_sha256.is_some())
    {
        return Err(ActionArgumentError::new(
            "failed/unavailable/cancelled action cannot deliver a payload",
        ));
    }
    Ok(())
}

fn sample_to_ns(sample: u64, sample_rate_hz: u32) -> Result<u64, ActionArgumentError> {
    let numerator = u128::from(sample)
        .checked_mul(1_000_000_000_u128)
        .ok_or_else(|| ActionArgumentError::new("audio sample timestamp overflow"))?;
    let timestamp = numerator / u128::from(sample_rate_hz);
    u64::try_from(timestamp)
        .map_err(|_| ActionArgumentError::new("audio sample timestamp exceeds u64"))
}

fn abs_diff(left: u64, right: u64) -> u64 {
    left.max(right) - left.min(right)
}

fn require_text(value: &str, field: &str) -> Result<(), ActionArgumentError> {
    if value.trim().is_empty() {
        return Err(ActionArgumentError::new(format!("{field} cannot be empty")));
    }
    Ok(())
}

fn validate_digest(value: &str, field: &str) -> Result<(), ActionArgumentError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ActionArgumentError::new(format!(
            "{field} must be a SHA-256 hex digest"
        )));
    }
    Ok(())
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: &str) -> String {
        hex_sha256(value.as_bytes())
    }

    fn source() -> SourceIdentityV1 {
        SourceIdentityV1 {
            source_id: "video-source-1".to_string(),
            source_content_sha256: digest("video-source"),
            clock_domain_id: "camera-clock".to_string(),
            clock_generation: 7,
        }
    }

    fn video_manifest() -> ActionParameterManifestV1 {
        ActionParameterManifestV1 {
            schema_version: ACTION_ARGUMENT_SCHEMA_V1,
            action_id: "crop-video".to_string(),
            source: source(),
            requested_interval: SourceIntervalV1 {
                start_ns: 0,
                end_ns: 1_000_000_000,
            },
            timestamp_tolerance_ns: 0,
            parameters: ActionParametersV1::VideoCrop(VideoCropParametersV1 {
                sampling: VideoSamplingV1::FixedRate {
                    fps_numerator: 2,
                    fps_denominator: 1,
                    phase_ns: 0,
                    frame_count: 2,
                },
                roi: Some(RegionOfInterestV1::Normalized(NormalizedRoiV1 {
                    x_bp: 1_000,
                    y_bp: 2_000,
                    width_bp: 5_000,
                    height_bp: 4_000,
                })),
                output_resolution: OutputResolutionV1 {
                    width: 640,
                    height: 360,
                },
                crop_policy_id: "center-crop-v1".to_string(),
                crop_policy_sha256: digest("crop-policy"),
                preprocessing_sha256: digest("preprocess"),
            }),
        }
    }

    fn audio_manifest() -> ActionParameterManifestV1 {
        ActionParameterManifestV1 {
            schema_version: ACTION_ARGUMENT_SCHEMA_V1,
            action_id: "listen-audio".to_string(),
            source: SourceIdentityV1 {
                source_id: "mic-source-1".to_string(),
                source_content_sha256: digest("mic-source"),
                clock_domain_id: "microphone-clock".to_string(),
                clock_generation: 2,
            },
            requested_interval: SourceIntervalV1 {
                start_ns: 0,
                end_ns: 1_000_000_000,
            },
            timestamp_tolerance_ns: 0,
            parameters: ActionParametersV1::AudioWindow(AudioWindowParametersV1 {
                start_sample: 0,
                end_sample: 16_000,
                sample_rate_hz: 16_000,
                channels: 1,
                sample_format: AudioSampleFormatV1::S16Le,
                chunk_frames: 1_600,
                overlap_frames: 200,
                transform_sha256: digest("audio-transform"),
            }),
        }
    }

    fn feedback_manifest() -> ActionParameterManifestV1 {
        ActionParameterManifestV1 {
            schema_version: ACTION_ARGUMENT_SCHEMA_V1,
            action_id: "ask-user".to_string(),
            source: SourceIdentityV1 {
                source_id: "user-simulator".to_string(),
                source_content_sha256: digest("user-sim-revision"),
                clock_domain_id: "episode-logical-clock".to_string(),
                clock_generation: 1,
            },
            requested_interval: SourceIntervalV1 {
                start_ns: 10,
                end_ns: 20,
            },
            timestamp_tolerance_ns: 0,
            parameters: ActionParametersV1::FeedbackRequest(FeedbackRequestParametersV1 {
                source_role: FeedbackSourceRoleV1::UserSimulator,
                feedback_kind: FeedbackKindV1::Hint,
                expected_delivery_tick: 11,
            }),
        }
    }

    fn evsi_forecasts(action_id: &str) -> Vec<ActionEvsiForecastV1> {
        vec![
            ActionEvsiForecastV1 {
                action_id: action_id.to_string(),
                estimated_gross_evsi: 0.4,
                lower_gross_evsi: 0.2,
                upper_gross_evsi: 0.6,
            },
            ActionEvsiForecastV1 {
                action_id: STOP_ACTION_ID_V1.to_string(),
                estimated_gross_evsi: 0.0,
                lower_gross_evsi: 0.0,
                upper_gross_evsi: 0.0,
            },
        ]
    }

    #[test]
    fn same_manifest_replays_same_digest() {
        let manifest = video_manifest();
        assert_eq!(validate_manifest(&manifest), validate_manifest(&manifest));
    }

    #[test]
    fn changing_crop_roi_changes_digest() {
        let first = video_manifest();
        let mut second = video_manifest();
        let ActionParametersV1::VideoCrop(parameters) = &mut second.parameters else {
            unreachable!();
        };
        parameters.roi = Some(RegionOfInterestV1::Normalized(NormalizedRoiV1 {
            x_bp: 2_000,
            y_bp: 2_000,
            width_bp: 5_000,
            height_bp: 4_000,
        }));
        assert_ne!(
            validate_manifest(&first).unwrap(),
            validate_manifest(&second).unwrap()
        );
    }

    #[test]
    fn changing_fps_changes_digest() {
        let first = video_manifest();
        let mut second = video_manifest();
        let ActionParametersV1::VideoCrop(parameters) = &mut second.parameters else {
            unreachable!();
        };
        parameters.sampling = VideoSamplingV1::FixedRate {
            fps_numerator: 4,
            fps_denominator: 1,
            phase_ns: 0,
            frame_count: 4,
        };
        assert_ne!(
            validate_manifest(&first).unwrap(),
            validate_manifest(&second).unwrap()
        );
    }

    #[test]
    fn changing_clock_generation_changes_digest() {
        let first = video_manifest();
        let mut second = video_manifest();
        second.source.clock_generation += 1;
        assert_ne!(
            validate_manifest(&first).unwrap(),
            validate_manifest(&second).unwrap()
        );
    }

    #[test]
    fn rational_fixed_rate_schedule_is_exact() {
        let manifest = video_manifest();
        assert_eq!(
            expected_video_timestamps(&manifest).unwrap(),
            vec![0, 500_000_000]
        );
    }

    #[test]
    fn video_schedule_outside_requested_interval_is_rejected() {
        let mut manifest = video_manifest();
        manifest.requested_interval.end_ns = 400_000_000;
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn nonmonotonic_explicit_frame_timestamps_are_rejected() {
        let mut manifest = video_manifest();
        let ActionParametersV1::VideoCrop(parameters) = &mut manifest.parameters else {
            unreachable!();
        };
        parameters.sampling = VideoSamplingV1::ExplicitTimestamps {
            timestamps_ns: vec![0, 200, 100],
        };
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn normalized_roi_outside_frame_is_rejected() {
        let mut manifest = video_manifest();
        let ActionParametersV1::VideoCrop(parameters) = &mut manifest.parameters else {
            unreachable!();
        };
        parameters.roi = Some(RegionOfInterestV1::Normalized(NormalizedRoiV1 {
            x_bp: 8_000,
            y_bp: 2_000,
            width_bp: 4_000,
            height_bp: 4_000,
        }));
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn pixel_roi_outside_source_dimensions_is_rejected() {
        let mut manifest = video_manifest();
        let ActionParametersV1::VideoCrop(parameters) = &mut manifest.parameters else {
            unreachable!();
        };
        parameters.roi = Some(RegionOfInterestV1::Pixels(PixelRoiV1 {
            x: 900,
            y: 700,
            width: 200,
            height: 100,
            source_width: 1_000,
            source_height: 800,
        }));
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn audio_window_matches_sample_rate_and_source_interval() {
        assert!(validate_manifest(&audio_manifest()).is_ok());
    }

    #[test]
    fn audio_sample_range_mismatch_is_rejected() {
        let mut manifest = audio_manifest();
        let ActionParametersV1::AudioWindow(parameters) = &mut manifest.parameters else {
            unreachable!();
        };
        parameters.end_sample = 8_000;
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn chunk_overlap_must_be_less_than_chunk_size() {
        let mut manifest = audio_manifest();
        let ActionParametersV1::AudioWindow(parameters) = &mut manifest.parameters else {
            unreachable!();
        };
        parameters.overlap_frames = parameters.chunk_frames;
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn forecast_requires_all_unique_action_manifests_and_selected_digest() {
        let manifest = video_manifest();
        let digest = validate_manifest(&manifest).unwrap();
        let forecast = ActionParameterForecastV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-3".to_string(),
            transcript_head_sha256: hex_sha256(b"head"),
            eligible_action_ids: vec![manifest.action_id.clone()],
            actions: vec![ForecastActionManifestV1 {
                action_id: manifest.action_id.clone(),
                manifest: manifest.clone(),
                manifest_sha256: digest.clone(),
            }],
            evsi_forecasts: evsi_forecasts(&manifest.action_id),
            selected_action_id: manifest.action_id.clone(),
            selected_manifest_sha256: Some(digest),
        };
        assert!(validate_forecast(&forecast).is_ok());
    }

    #[test]
    fn selected_action_digest_mismatch_is_rejected() {
        let manifest = video_manifest();
        let digest = validate_manifest(&manifest).unwrap();
        let forecast = ActionParameterForecastV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-3".to_string(),
            transcript_head_sha256: hex_sha256(b"head"),
            eligible_action_ids: vec![manifest.action_id.clone()],
            actions: vec![ForecastActionManifestV1 {
                action_id: manifest.action_id.clone(),
                manifest: manifest.clone(),
                manifest_sha256: digest,
            }],
            evsi_forecasts: evsi_forecasts(&manifest.action_id),
            selected_action_id: "crop-video".to_string(),
            selected_manifest_sha256: Some(hex_sha256(b"mutated")),
        };
        assert!(validate_forecast(&forecast).is_err());
    }

    #[test]
    fn result_manifest_digest_cannot_change_after_acceptance() {
        let manifest = video_manifest();
        let manifest_digest = validate_manifest(&manifest).unwrap();
        let forecast = ActionParameterForecastV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-4".to_string(),
            transcript_head_sha256: hex_sha256(b"head"),
            eligible_action_ids: vec![manifest.action_id.clone()],
            actions: vec![ForecastActionManifestV1 {
                action_id: manifest.action_id.clone(),
                manifest: manifest.clone(),
                manifest_sha256: manifest_digest.clone(),
            }],
            evsi_forecasts: evsi_forecasts(&manifest.action_id),
            selected_action_id: "crop-video".to_string(),
            selected_manifest_sha256: Some(manifest_digest.clone()),
        };
        let acceptance = RequestAcceptanceV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-4".to_string(),
            request_id: "request-1".to_string(),
            action_id: "crop-video".to_string(),
            parameter_manifest_sha256: manifest_digest.clone(),
            parent_transcript_head_sha256: hex_sha256(b"head"),
            accepted_tick: 10,
            expected_arrival_tick: 12,
            decision_deadline_tick: 20,
        };
        let result = ActionResultV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-4".to_string(),
            request_id: "request-1".to_string(),
            action_id: "crop-video".to_string(),
            parameter_manifest_sha256: hex_sha256(b"other-manifest"),
            parent_transcript_head_sha256: hex_sha256(b"head"),
            status: ActionResultStatusV1::Failed,
            result_tick: 12,
            delivered_frames: Vec::new(),
            delivered_roi: None,
            delivered_output_resolution: None,
            delivered_crop_policy_sha256: None,
            delivered_preprocessing_sha256: None,
            delivered_audio: None,
            delivered_feedback: None,
            actual_resource_digest_sha256: hex_sha256(b"resources"),
            result_event_sha256: hex_sha256(b"result"),
        };
        assert!(validate_action_lifecycle(&forecast, &acceptance, &result).is_err());
    }

    #[test]
    fn delivered_video_must_match_requested_timestamps_and_indices() {
        let manifest = video_manifest();
        let manifest_digest = validate_manifest(&manifest).unwrap();
        let forecast = ActionParameterForecastV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-1".to_string(),
            transcript_head_sha256: hex_sha256(b"head"),
            eligible_action_ids: vec![manifest.action_id.clone()],
            actions: vec![ForecastActionManifestV1 {
                action_id: manifest.action_id.clone(),
                manifest: manifest.clone(),
                manifest_sha256: manifest_digest.clone(),
            }],
            evsi_forecasts: evsi_forecasts(&manifest.action_id),
            selected_action_id: manifest.action_id.clone(),
            selected_manifest_sha256: Some(manifest_digest.clone()),
        };
        let acceptance = RequestAcceptanceV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-1".to_string(),
            request_id: "req-1".to_string(),
            action_id: manifest.action_id.clone(),
            parameter_manifest_sha256: manifest_digest.clone(),
            parent_transcript_head_sha256: hex_sha256(b"head"),
            accepted_tick: 1,
            expected_arrival_tick: 2,
            decision_deadline_tick: 10,
        };
        let result = ActionResultV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-1".to_string(),
            request_id: "req-1".to_string(),
            action_id: manifest.action_id.clone(),
            parameter_manifest_sha256: manifest_digest,
            parent_transcript_head_sha256: hex_sha256(b"head"),
            status: ActionResultStatusV1::Delivered,
            result_tick: 2,
            delivered_frames: vec![
                DeliveredFrameV1 {
                    observation_ref: "frame-0".to_string(),
                    frame_index: 0,
                    capture_timestamp_ns: 0,
                    byte_count: 100,
                },
                DeliveredFrameV1 {
                    observation_ref: "frame-1".to_string(),
                    frame_index: 1,
                    capture_timestamp_ns: 500_000_000,
                    byte_count: 100,
                },
            ],
            delivered_roi: Some(RegionOfInterestV1::Normalized(NormalizedRoiV1 {
                x_bp: 1_000,
                y_bp: 2_000,
                width_bp: 5_000,
                height_bp: 4_000,
            })),
            delivered_output_resolution: Some(OutputResolutionV1 {
                width: 640,
                height: 360,
            }),
            delivered_crop_policy_sha256: Some(digest("crop-policy")),
            delivered_preprocessing_sha256: Some(digest("preprocess")),
            delivered_audio: None,
            delivered_feedback: None,
            actual_resource_digest_sha256: hex_sha256(b"resources"),
            result_event_sha256: hex_sha256(b"result"),
        };
        assert!(validate_action_lifecycle(&forecast, &acceptance, &result).is_ok());
    }

    #[test]
    fn nominal_fps_with_wrong_actual_frames_is_rejected() {
        let manifest = video_manifest();
        let manifest_digest = validate_manifest(&manifest).unwrap();
        let forecast = ActionParameterForecastV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-1".to_string(),
            transcript_head_sha256: hex_sha256(b"head"),
            eligible_action_ids: vec![manifest.action_id.clone()],
            actions: vec![ForecastActionManifestV1 {
                action_id: manifest.action_id.clone(),
                manifest: manifest.clone(),
                manifest_sha256: manifest_digest.clone(),
            }],
            evsi_forecasts: vec![
                ActionEvsiForecastV1 {
                    action_id: manifest.action_id.clone(),
                    estimated_gross_evsi: 0.4,
                    lower_gross_evsi: 0.2,
                    upper_gross_evsi: 0.6,
                },
                ActionEvsiForecastV1 {
                    action_id: STOP_ACTION_ID_V1.to_string(),
                    estimated_gross_evsi: 0.0,
                    lower_gross_evsi: 0.0,
                    upper_gross_evsi: 0.0,
                },
            ],
            selected_action_id: manifest.action_id.clone(),
            selected_manifest_sha256: Some(manifest_digest.clone()),
        };
        let acceptance = RequestAcceptanceV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-1".to_string(),
            request_id: "req-1".to_string(),
            action_id: manifest.action_id.clone(),
            parameter_manifest_sha256: manifest_digest.clone(),
            parent_transcript_head_sha256: hex_sha256(b"head"),
            accepted_tick: 1,
            expected_arrival_tick: 2,
            decision_deadline_tick: 10,
        };
        let mut result = ActionResultV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-1".to_string(),
            request_id: "req-1".to_string(),
            action_id: manifest.action_id,
            parameter_manifest_sha256: manifest_digest,
            parent_transcript_head_sha256: hex_sha256(b"head"),
            status: ActionResultStatusV1::Delivered,
            result_tick: 2,
            delivered_frames: vec![
                DeliveredFrameV1 {
                    observation_ref: "frame-0".to_string(),
                    frame_index: 0,
                    capture_timestamp_ns: 0,
                    byte_count: 100,
                },
                DeliveredFrameV1 {
                    observation_ref: "frame-1".to_string(),
                    frame_index: 1,
                    capture_timestamp_ns: 250_000_000,
                    byte_count: 100,
                },
            ],
            delivered_roi: Some(RegionOfInterestV1::Normalized(NormalizedRoiV1 {
                x_bp: 1_000,
                y_bp: 2_000,
                width_bp: 5_000,
                height_bp: 4_000,
            })),
            delivered_output_resolution: Some(OutputResolutionV1 {
                width: 640,
                height: 360,
            }),
            delivered_crop_policy_sha256: Some(digest("crop-policy")),
            delivered_preprocessing_sha256: Some(digest("preprocess")),
            delivered_audio: None,
            delivered_feedback: None,
            actual_resource_digest_sha256: hex_sha256(b"resources"),
            result_event_sha256: hex_sha256(b"result"),
        };
        assert!(validate_action_lifecycle(&forecast, &acceptance, &result).is_err());
        result.status = ActionResultStatusV1::Partial;
        assert!(validate_action_lifecycle(&forecast, &acceptance, &result).is_err());
    }

    #[test]
    fn failed_result_keeps_action_manifest_but_mints_no_observation() {
        let manifest = video_manifest();
        let manifest_digest = validate_manifest(&manifest).unwrap();
        let forecast = ActionParameterForecastV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-1".to_string(),
            transcript_head_sha256: hex_sha256(b"head"),
            eligible_action_ids: vec![manifest.action_id.clone()],
            actions: vec![ForecastActionManifestV1 {
                action_id: manifest.action_id.clone(),
                manifest: manifest.clone(),
                manifest_sha256: manifest_digest.clone(),
            }],
            evsi_forecasts: vec![
                ActionEvsiForecastV1 {
                    action_id: manifest.action_id.clone(),
                    estimated_gross_evsi: 0.4,
                    lower_gross_evsi: 0.2,
                    upper_gross_evsi: 0.6,
                },
                ActionEvsiForecastV1 {
                    action_id: STOP_ACTION_ID_V1.to_string(),
                    estimated_gross_evsi: 0.0,
                    lower_gross_evsi: 0.0,
                    upper_gross_evsi: 0.0,
                },
            ],
            selected_action_id: manifest.action_id.clone(),
            selected_manifest_sha256: Some(manifest_digest.clone()),
        };
        let acceptance = RequestAcceptanceV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-1".to_string(),
            request_id: "req-1".to_string(),
            action_id: manifest.action_id.clone(),
            parameter_manifest_sha256: manifest_digest.clone(),
            parent_transcript_head_sha256: hex_sha256(b"head"),
            accepted_tick: 1,
            expected_arrival_tick: 2,
            decision_deadline_tick: 10,
        };
        let result = ActionResultV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-1".to_string(),
            request_id: "req-1".to_string(),
            action_id: manifest.action_id,
            parameter_manifest_sha256: manifest_digest,
            parent_transcript_head_sha256: hex_sha256(b"head"),
            status: ActionResultStatusV1::Failed,
            result_tick: 2,
            delivered_frames: Vec::new(),
            delivered_roi: None,
            delivered_output_resolution: None,
            delivered_crop_policy_sha256: None,
            delivered_preprocessing_sha256: None,
            delivered_audio: None,
            delivered_feedback: None,
            actual_resource_digest_sha256: hex_sha256(b"resources"),
            result_event_sha256: hex_sha256(b"failed-result"),
        };
        assert!(validate_action_lifecycle(&forecast, &acceptance, &result).is_ok());
    }

    #[test]
    fn stop_is_a_required_forecast_alternative_but_has_no_manifest() {
        let manifest = video_manifest();
        let manifest_digest = validate_manifest(&manifest).unwrap();
        let forecast = ActionParameterForecastV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-2".to_string(),
            transcript_head_sha256: hex_sha256(b"head"),
            eligible_action_ids: vec![manifest.action_id.clone()],
            actions: vec![ForecastActionManifestV1 {
                action_id: manifest.action_id.clone(),
                manifest,
                manifest_sha256: manifest_digest,
            }],
            evsi_forecasts: evsi_forecasts("crop-video"),
            selected_action_id: STOP_ACTION_ID_V1.to_string(),
            selected_manifest_sha256: None,
        };
        assert!(validate_forecast(&forecast).is_ok());
    }

    #[test]
    fn forecast_missing_stop_or_eligible_action_is_rejected() {
        let manifest = video_manifest();
        let manifest_digest = validate_manifest(&manifest).unwrap();
        let mut forecast = ActionParameterForecastV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-2".to_string(),
            transcript_head_sha256: hex_sha256(b"head"),
            eligible_action_ids: vec![manifest.action_id.clone()],
            actions: vec![ForecastActionManifestV1 {
                action_id: manifest.action_id.clone(),
                manifest,
                manifest_sha256: manifest_digest.clone(),
            }],
            evsi_forecasts: evsi_forecasts("crop-video"),
            selected_action_id: "crop-video".to_string(),
            selected_manifest_sha256: Some(manifest_digest),
        };
        forecast.evsi_forecasts.pop();
        assert!(validate_forecast(&forecast).is_err());
    }

    #[test]
    fn complete_audio_delivery_must_match_requested_sample_range_and_format() {
        let manifest = audio_manifest();
        let manifest_digest = validate_manifest(&manifest).unwrap();
        let forecast = ActionParameterForecastV1 {
            episode_id: "episode-audio".to_string(),
            turn_id: "turn-1".to_string(),
            transcript_head_sha256: hex_sha256(b"audio-head"),
            eligible_action_ids: vec![manifest.action_id.clone()],
            actions: vec![ForecastActionManifestV1 {
                action_id: manifest.action_id.clone(),
                manifest: manifest.clone(),
                manifest_sha256: manifest_digest.clone(),
            }],
            evsi_forecasts: evsi_forecasts(&manifest.action_id),
            selected_action_id: manifest.action_id.clone(),
            selected_manifest_sha256: Some(manifest_digest.clone()),
        };
        let acceptance = RequestAcceptanceV1 {
            episode_id: "episode-audio".to_string(),
            turn_id: "turn-1".to_string(),
            request_id: "audio-request".to_string(),
            action_id: manifest.action_id.clone(),
            parameter_manifest_sha256: manifest_digest.clone(),
            parent_transcript_head_sha256: hex_sha256(b"audio-head"),
            accepted_tick: 1,
            expected_arrival_tick: 2,
            decision_deadline_tick: 10,
        };
        let result = ActionResultV1 {
            episode_id: "episode-audio".to_string(),
            turn_id: "turn-1".to_string(),
            request_id: "audio-request".to_string(),
            action_id: manifest.action_id.clone(),
            parameter_manifest_sha256: manifest_digest,
            parent_transcript_head_sha256: hex_sha256(b"audio-head"),
            status: ActionResultStatusV1::Delivered,
            result_tick: 2,
            delivered_frames: Vec::new(),
            delivered_roi: None,
            delivered_output_resolution: None,
            delivered_crop_policy_sha256: None,
            delivered_preprocessing_sha256: None,
            delivered_audio: Some(DeliveredAudioRangeV1 {
                start_sample: 0,
                end_sample: 16_000,
                sample_rate_hz: 16_000,
                channels: 1,
                sample_format: AudioSampleFormatV1::S16Le,
            }),
            delivered_feedback: None,
            actual_resource_digest_sha256: hex_sha256(b"audio-resources"),
            result_event_sha256: hex_sha256(b"audio-result"),
        };
        assert!(validate_action_lifecycle(&forecast, &acceptance, &result).is_ok());
    }

    #[test]
    fn actual_video_roi_mismatch_is_rejected() {
        let manifest = video_manifest();
        let digest_value = validate_manifest(&manifest).unwrap();
        let forecast = ActionParameterForecastV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-1".to_string(),
            transcript_head_sha256: hex_sha256(b"head"),
            eligible_action_ids: vec![manifest.action_id.clone()],
            actions: vec![ForecastActionManifestV1 {
                action_id: manifest.action_id.clone(),
                manifest: manifest.clone(),
                manifest_sha256: digest_value.clone(),
            }],
            evsi_forecasts: evsi_forecasts(&manifest.action_id),
            selected_action_id: manifest.action_id.clone(),
            selected_manifest_sha256: Some(digest_value.clone()),
        };
        let acceptance = RequestAcceptanceV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-1".to_string(),
            request_id: "req-1".to_string(),
            action_id: manifest.action_id.clone(),
            parameter_manifest_sha256: digest_value.clone(),
            parent_transcript_head_sha256: hex_sha256(b"head"),
            accepted_tick: 1,
            expected_arrival_tick: 2,
            decision_deadline_tick: 10,
        };
        let result = ActionResultV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-1".to_string(),
            request_id: "req-1".to_string(),
            action_id: manifest.action_id,
            parameter_manifest_sha256: digest_value,
            parent_transcript_head_sha256: hex_sha256(b"head"),
            status: ActionResultStatusV1::Delivered,
            result_tick: 2,
            delivered_frames: vec![
                DeliveredFrameV1 {
                    observation_ref: "frame-0".to_string(),
                    frame_index: 0,
                    capture_timestamp_ns: 0,
                    byte_count: 100,
                },
                DeliveredFrameV1 {
                    observation_ref: "frame-1".to_string(),
                    frame_index: 1,
                    capture_timestamp_ns: 500_000_000,
                    byte_count: 100,
                },
            ],
            delivered_roi: Some(RegionOfInterestV1::Normalized(NormalizedRoiV1 {
                x_bp: 2_000,
                y_bp: 2_000,
                width_bp: 5_000,
                height_bp: 4_000,
            })),
            delivered_output_resolution: Some(OutputResolutionV1 {
                width: 640,
                height: 360,
            }),
            delivered_crop_policy_sha256: Some(digest("crop-policy")),
            delivered_preprocessing_sha256: Some(digest("preprocess")),
            delivered_audio: None,
            delivered_feedback: None,
            actual_resource_digest_sha256: hex_sha256(b"resources"),
            result_event_sha256: hex_sha256(b"result"),
        };
        assert!(validate_action_lifecycle(&forecast, &acceptance, &result).is_err());
    }

    #[test]
    fn post_deadline_media_delivery_must_be_marked_late() {
        let manifest = video_manifest();
        let manifest_digest = validate_manifest(&manifest).unwrap();
        let forecast = ActionParameterForecastV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-1".to_string(),
            transcript_head_sha256: hex_sha256(b"head"),
            eligible_action_ids: vec![manifest.action_id.clone()],
            actions: vec![ForecastActionManifestV1 {
                action_id: manifest.action_id.clone(),
                manifest: manifest.clone(),
                manifest_sha256: manifest_digest.clone(),
            }],
            evsi_forecasts: evsi_forecasts(&manifest.action_id),
            selected_action_id: manifest.action_id.clone(),
            selected_manifest_sha256: Some(manifest_digest.clone()),
        };
        let acceptance = RequestAcceptanceV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-1".to_string(),
            request_id: "late-request".to_string(),
            action_id: manifest.action_id.clone(),
            parameter_manifest_sha256: manifest_digest.clone(),
            parent_transcript_head_sha256: hex_sha256(b"head"),
            accepted_tick: 1,
            expected_arrival_tick: 2,
            decision_deadline_tick: 4,
        };
        let mut result = ActionResultV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-1".to_string(),
            request_id: "late-request".to_string(),
            action_id: manifest.action_id,
            parameter_manifest_sha256: manifest_digest,
            parent_transcript_head_sha256: hex_sha256(b"head"),
            status: ActionResultStatusV1::Delivered,
            result_tick: 5,
            delivered_frames: vec![
                DeliveredFrameV1 {
                    observation_ref: "frame-0".to_string(),
                    frame_index: 0,
                    capture_timestamp_ns: 0,
                    byte_count: 100,
                },
                DeliveredFrameV1 {
                    observation_ref: "frame-1".to_string(),
                    frame_index: 1,
                    capture_timestamp_ns: 500_000_000,
                    byte_count: 100,
                },
            ],
            delivered_roi: Some(RegionOfInterestV1::Normalized(NormalizedRoiV1 {
                x_bp: 1_000,
                y_bp: 2_000,
                width_bp: 5_000,
                height_bp: 4_000,
            })),
            delivered_output_resolution: Some(OutputResolutionV1 {
                width: 640,
                height: 360,
            }),
            delivered_crop_policy_sha256: Some(digest("crop-policy")),
            delivered_preprocessing_sha256: Some(digest("preprocess")),
            delivered_audio: None,
            delivered_feedback: None,
            actual_resource_digest_sha256: hex_sha256(b"resources"),
            result_event_sha256: hex_sha256(b"late-result"),
        };
        assert!(validate_action_lifecycle(&forecast, &acceptance, &result).is_err());
        result.status = ActionResultStatusV1::Late;
        assert!(validate_action_lifecycle(&forecast, &acceptance, &result).is_ok());
    }

    #[test]
    fn canonical_manifest_digest_is_independent_of_json_key_order() {
        let first = r#"{"b":2,"a":1}"#;
        let second = r#"{"a":1,"b":2}"#;
        let first_value: serde_json::Value = serde_json::from_str(first).unwrap();
        let second_value: serde_json::Value = serde_json::from_str(second).unwrap();
        assert_eq!(
            hex_sha256(&serde_json::to_vec(&first_value).unwrap()),
            hex_sha256(&serde_json::to_vec(&second_value).unwrap())
        );
    }

    #[test]
    fn delivered_video_timestamp_must_respect_declared_tolerance() {
        let mut manifest = video_manifest();
        manifest.timestamp_tolerance_ns = 2;
        let manifest_digest = validate_manifest(&manifest).unwrap();
        let forecast = ActionParameterForecastV1 {
            episode_id: "episode-tolerance".to_string(),
            turn_id: "turn-1".to_string(),
            transcript_head_sha256: hex_sha256(b"head"),
            eligible_action_ids: vec![manifest.action_id.clone()],
            actions: vec![ForecastActionManifestV1 {
                action_id: manifest.action_id.clone(),
                manifest: manifest.clone(),
                manifest_sha256: manifest_digest.clone(),
            }],
            evsi_forecasts: evsi_forecasts(&manifest.action_id),
            selected_action_id: manifest.action_id.clone(),
            selected_manifest_sha256: Some(manifest_digest.clone()),
        };
        let acceptance = RequestAcceptanceV1 {
            episode_id: "episode-tolerance".to_string(),
            turn_id: "turn-1".to_string(),
            request_id: "req-tolerance".to_string(),
            action_id: manifest.action_id.clone(),
            parameter_manifest_sha256: manifest_digest.clone(),
            parent_transcript_head_sha256: hex_sha256(b"head"),
            accepted_tick: 1,
            expected_arrival_tick: 2,
            decision_deadline_tick: 10,
        };
        let result = ActionResultV1 {
            episode_id: "episode-tolerance".to_string(),
            turn_id: "turn-1".to_string(),
            request_id: "req-tolerance".to_string(),
            action_id: manifest.action_id,
            parameter_manifest_sha256: manifest_digest,
            parent_transcript_head_sha256: hex_sha256(b"head"),
            status: ActionResultStatusV1::Delivered,
            result_tick: 2,
            delivered_frames: vec![
                DeliveredFrameV1 {
                    observation_ref: "frame-0".to_string(),
                    frame_index: 0,
                    capture_timestamp_ns: 1,
                    byte_count: 100,
                },
                DeliveredFrameV1 {
                    observation_ref: "frame-1".to_string(),
                    frame_index: 1,
                    capture_timestamp_ns: 500_000_001,
                    byte_count: 100,
                },
            ],
            delivered_roi: Some(RegionOfInterestV1::Normalized(NormalizedRoiV1 {
                x_bp: 1_000,
                y_bp: 2_000,
                width_bp: 5_000,
                height_bp: 4_000,
            })),
            delivered_output_resolution: Some(OutputResolutionV1 {
                width: 640,
                height: 360,
            }),
            delivered_crop_policy_sha256: Some(digest("crop-policy")),
            delivered_preprocessing_sha256: Some(digest("preprocess")),
            delivered_audio: None,
            delivered_feedback: None,
            actual_resource_digest_sha256: hex_sha256(b"resources"),
            result_event_sha256: hex_sha256(b"result"),
        };
        assert!(validate_action_lifecycle(&forecast, &acceptance, &result).is_ok());
    }

    #[test]
    fn video_roi_basis_points_are_bounded_to_ten_thousand() {
        let mut manifest = video_manifest();
        let ActionParametersV1::VideoCrop(parameters) = &mut manifest.parameters else {
            unreachable!();
        };
        parameters.roi = Some(RegionOfInterestV1::Normalized(NormalizedRoiV1 {
            x_bp: 8_000,
            y_bp: 2_000,
            width_bp: 3_000,
            height_bp: 4_000,
        }));
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn action_forecast_manifest_must_cover_each_eligible_action_exactly_once() {
        let manifest = video_manifest();
        let digest_value = validate_manifest(&manifest).unwrap();
        let forecast = ActionParameterForecastV1 {
            episode_id: "episode-1".to_string(),
            turn_id: "turn-1".to_string(),
            transcript_head_sha256: hex_sha256(b"head"),
            eligible_action_ids: vec!["crop-video".to_string(), "listen-audio".to_string()],
            actions: vec![ForecastActionManifestV1 {
                action_id: "crop-video".to_string(),
                manifest: manifest.clone(),
                manifest_sha256: digest_value.clone(),
            }],
            evsi_forecasts: vec![
                ActionEvsiForecastV1 {
                    action_id: "crop-video".to_string(),
                    estimated_gross_evsi: 0.4,
                    lower_gross_evsi: 0.2,
                    upper_gross_evsi: 0.6,
                },
                ActionEvsiForecastV1 {
                    action_id: "listen-audio".to_string(),
                    estimated_gross_evsi: 0.5,
                    lower_gross_evsi: 0.3,
                    upper_gross_evsi: 0.7,
                },
                ActionEvsiForecastV1 {
                    action_id: STOP_ACTION_ID_V1.to_string(),
                    estimated_gross_evsi: 0.0,
                    lower_gross_evsi: 0.0,
                    upper_gross_evsi: 0.0,
                },
            ],
            selected_action_id: manifest.action_id,
            selected_manifest_sha256: Some(digest_value),
        };
        assert!(validate_forecast(&forecast).is_err());
    }

    #[test]
    fn feedback_request_is_not_a_sensor_observation() {
        let manifest = feedback_manifest();
        assert!(validate_manifest(&manifest).is_ok());
    }
}
