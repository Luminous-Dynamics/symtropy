//! Deterministic, provenance-preserving sensor/control fault transformations.
//!
//! Clean inputs are immutable. Non-drop faults produce policy-visible child
//! receipts via the renderer's receipt-derivation API and evaluator-only
//! derivation sidecars. Camera drops produce missing events, never fake frames.
//! This crate applies software perturbations; it does not simulate physical faults.

use action_conditioned_offscreen_camera::{
    MissingFrameReason, PolicyObservationReceipt, PolicyReceiptOverrides,
    SoftwareCameraRenderer, sha256_hex,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const FAULT_APPLICATOR_REVISION: &str = "deterministic-driving-fault-applicator-v0.1";
pub const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_OBSERVATIONS: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MaskMode {
    FixedPerEpisode,
    ResampledPerFrame,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FaultTransform {
    CameraBurstDrop {
        duration_ticks: u64,
    },
    CameraOcclusion {
        occlusion_ppm: u32,
        mask_mode: MaskMode,
    },
    GpsLocalizationNoise {
        sigma_mm: u64,
    },
    SpeedMultiplierNoise {
        mean_ppm: i64,
        standard_deviation_ppm: i64,
        minimum_multiplier_ppm: i64,
        maximum_multiplier_ppm: i64,
    },
    InferenceControlDelay {
        delay_us: u64,
    },
    StaleFrameReplay,
    PixelCorruption {
        corrupt_byte_ppm: u32,
    },
    ClockShift {
        capture_delta_us: i64,
        render_completed_delta_us: i64,
        delivered_delta_us: i64,
    },
    CalibrationMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FaultEvent {
    pub event_id: String,
    pub start_sequence: u64,
    pub end_sequence: u64,
    pub transform: FaultTransform,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FaultPlan {
    pub schema_version: u32,
    pub campaign_id: String,
    pub pair_id: String,
    pub scenario_id: String,
    pub episode_id: String,
    pub sensor_id: String,
    pub scenario_seed_sha256: String,
    pub actor_randomness_sha256: String,
    pub event: FaultEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleSignals {
    /// Vehicle-estimated position in millimetres.
    pub gps_x_mm: i64,
    pub gps_y_mm: i64,
    /// Estimated speed in milli-metres per second.
    pub speed_mm_per_s: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ControlTiming {
    pub command_id: String,
    pub observation_timestamp_us: u64,
    pub inference_completed_timestamp_us: u64,
    pub command_timestamp_us: u64,
    pub original_applied_control_timestamp_us: u64,
    pub applied_control_timestamp_us: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CleanObservation {
    pub sequence: u64,
    pub capture_timestamp_us: u64,
    pub render_completed_timestamp_us: u64,
    pub delivered_timestamp_us: u64,
    pub pixels_file: String,
    pub parent_policy_receipt: PolicyObservationReceipt,
    pub vehicle_signals: VehicleSignals,
    pub control_timing: Option<ControlTiming>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationStatus {
    Clean,
    Derived,
    Missing,
    StaleReplayNegativeControl,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MissingFrameEvent {
    pub episode_id: String,
    pub pair_id: String,
    pub event_id: String,
    pub sequence: u64,
    pub capture_timestamp_us: u64,
    pub noticed_timestamp_us: u64,
    pub reason: MissingFrameReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FaultDerivationReceipt {
    pub schema_version: u32,
    pub campaign_id: String,
    pub pair_id: String,
    pub scenario_id: String,
    pub episode_id: String,
    pub event_id: String,
    pub transform_revision: String,
    pub parameters_sha256: String,
    pub seed_sha256: String,
    /// Clean observation for this episode tick; remains immutable.
    pub parent_observation_id: String,
    pub parent_receipt_sha256: String,
    /// For stale replay this identifies the older image source, not the current tick.
    pub source_observation_id: String,
    pub source_sequence: u64,
    pub child_observation_id: String,
    pub child_receipt_sha256: String,
    pub parent_frame_sha256: String,
    pub child_frame_sha256: String,
    pub parent_vehicle_signals_sha256: String,
    pub child_vehicle_signals_sha256: String,
    pub sampled_values: BTreeMap<String, i64>,
    pub geometry: BTreeMap<String, u64>,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultedObservation {
    pub sequence: u64,
    pub status: ObservationStatus,
    /// None for a camera-drop event. Stale replay intentionally retains an old
    /// sequence/capture identity so the downstream bridge can reject it.
    pub policy_receipt: Option<PolicyObservationReceipt>,
    pub pixels_rgb: Option<Vec<u8>>,
    pub vehicle_signals: Option<VehicleSignals>,
    pub control_timing: Option<ControlTiming>,
    /// Evaluator-only sidecar; never pass this field to the driving policy.
    pub derivation: Option<FaultDerivationReceipt>,
    pub missing_event: Option<MissingFrameEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FaultApplicationReport {
    pub schema_version: u32,
    pub campaign_id: String,
    pub pair_id: String,
    pub episode_id: String,
    pub event_id: String,
    pub transform_kind: String,
    pub expected_observations: usize,
    pub delivered_observations: usize,
    pub derived_observations: usize,
    pub missing_observations: usize,
    pub stale_replay_negative_controls: usize,
    pub clean_source_frames_verified: usize,
    pub clean_source_bytes_mutated: u64,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultApplication {
    pub observations: Vec<FaultedObservation>,
    pub report: FaultApplicationReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultApplyError {
    UnsupportedSchemaVersion,
    EmptyIdentity,
    InvalidDigest,
    InvalidEventWindow,
    InvalidFaultSeverity,
    BurstDurationMismatch,
    TooManyObservations,
    EmptyEpisode,
    ObservationSequenceMismatch,
    ObservationClockRegression,
    ParentReceiptIdentityMismatch,
    ParentReceiptGeometryMismatch,
    ParentReceiptDigestMismatch,
    ParentFrameDigestMismatch,
    ParentFrameSizeExceeded,
    MissingPixels,
    PixelLengthMismatch,
    PixelTransformOverflow,
    InvalidControlTiming,
    MissingControlTiming,
    ControlTimestampOverflow,
    MissingPriorFrameForReplay,
    TimestampOverflow,
    TimestampUnderflow,
    DuplicateObservationSequence,
}

impl std::fmt::Display for FaultApplyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for FaultApplyError {}

#[derive(Debug, Clone, Copy)]
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn from_seed(seed: &str) -> Self {
        let bytes = seed.as_bytes();
        let mut word = [0_u8; 8];
        for (index, slot) in word.iter_mut().enumerate() {
            *slot = bytes.get(index).copied().unwrap_or(0);
        }
        Self {
            state: u64::from_le_bytes(word),
        }
    }

    fn next_u64(&mut self) -> u64 {
        // SplitMix64: fully specified arithmetic; no ambient process randomness.
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^ (value >> 31)
    }

    fn below(&mut self, upper_exclusive: u64) -> u64 {
        if upper_exclusive == 0 {
            0
        } else {
            self.next_u64() % upper_exclusive
        }
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn transform_name(transform: &FaultTransform) -> &'static str {
    match transform {
        FaultTransform::CameraBurstDrop { .. } => "camera_burst_drop",
        FaultTransform::CameraOcclusion { .. } => "camera_occlusion",
        FaultTransform::GpsLocalizationNoise { .. } => "gps_localization_noise",
        FaultTransform::SpeedMultiplierNoise { .. } => "speed_multiplier_noise",
        FaultTransform::InferenceControlDelay { .. } => "inference_control_delay",
        FaultTransform::StaleFrameReplay => "stale_frame_replay",
        FaultTransform::PixelCorruption { .. } => "pixel_corruption",
        FaultTransform::ClockShift { .. } => "clock_shift",
        FaultTransform::CalibrationMismatch => "calibration_mismatch",
    }
}

fn validate_plan(plan: &FaultPlan, observations: &[CleanObservation]) -> Result<(), FaultApplyError> {
    if plan.schema_version != 1 {
        return Err(FaultApplyError::UnsupportedSchemaVersion);
    }
    for value in [
        plan.campaign_id.as_str(),
        plan.pair_id.as_str(),
        plan.scenario_id.as_str(),
        plan.episode_id.as_str(),
        plan.sensor_id.as_str(),
        plan.event.event_id.as_str(),
    ] {
        if value.trim().is_empty() {
            return Err(FaultApplyError::EmptyIdentity);
        }
    }
    if !is_sha256(&plan.scenario_seed_sha256) || !is_sha256(&plan.actor_randomness_sha256) {
        return Err(FaultApplyError::InvalidDigest);
    }
    if observations.is_empty() {
        return Err(FaultApplyError::EmptyEpisode);
    }
    if observations.len() > MAX_OBSERVATIONS {
        return Err(FaultApplyError::TooManyObservations);
    }
    if plan.event.start_sequence == 0
        || plan.event.start_sequence > plan.event.end_sequence
        || plan.event.start_sequence < observations[0].sequence
        || plan.event.end_sequence > observations[observations.len() - 1].sequence
    {
        return Err(FaultApplyError::InvalidEventWindow);
    }
    match plan.event.transform {
        FaultTransform::CameraBurstDrop { duration_ticks } => {
            let actual = plan.event.end_sequence - plan.event.start_sequence + 1;
            if ![20, 60].contains(&duration_ticks) || duration_ticks != actual {
                return Err(FaultApplyError::BurstDurationMismatch);
            }
        }
        FaultTransform::CameraOcclusion { occlusion_ppm, .. } => {
            if ![500_000, 800_000].contains(&occlusion_ppm) {
                return Err(FaultApplyError::InvalidFaultSeverity);
            }
        }
        FaultTransform::GpsLocalizationNoise { sigma_mm } => {
            if ![5_000, 15_000].contains(&sigma_mm) {
                return Err(FaultApplyError::InvalidFaultSeverity);
            }
        }
        FaultTransform::SpeedMultiplierNoise {
            mean_ppm,
            standard_deviation_ppm,
            minimum_multiplier_ppm,
            maximum_multiplier_ppm,
        } => {
            if ![500_000, 200_000].contains(&mean_ppm)
                || standard_deviation_ppm != 200_000
                || minimum_multiplier_ppm < 0
                || maximum_multiplier_ppm <= minimum_multiplier_ppm
                || maximum_multiplier_ppm > 2_000_000
            {
                return Err(FaultApplyError::InvalidFaultSeverity);
            }
        }
        FaultTransform::InferenceControlDelay { delay_us } => {
            if ![100_000, 200_000, 500_000].contains(&delay_us) {
                return Err(FaultApplyError::InvalidFaultSeverity);
            }
        }
        FaultTransform::PixelCorruption { corrupt_byte_ppm } => {
            if corrupt_byte_ppm == 0 || corrupt_byte_ppm > 100_000 {
                return Err(FaultApplyError::InvalidFaultSeverity);
            }
        }
        FaultTransform::StaleFrameReplay
        | FaultTransform::ClockShift { .. }
        | FaultTransform::CalibrationMismatch => {}
    }
    let mut previous_sequence = None;
    let mut previous_capture = None;
    let mut previous_delivery = None;
    let mut seen = BTreeSet::new();
    for observation in observations {
        if observation.sequence == 0 || !seen.insert(observation.sequence) {
            return Err(FaultApplyError::DuplicateObservationSequence);
        }
        if previous_sequence.is_some_and(|last| observation.sequence != last + 1) {
            return Err(FaultApplyError::ObservationSequenceMismatch);
        }
        if previous_capture.is_some_and(|last| observation.capture_timestamp_us <= last)
            || previous_delivery.is_some_and(|last| observation.delivered_timestamp_us < last)
        {
            return Err(FaultApplyError::ObservationClockRegression);
        }
        if observation.parent_policy_receipt.episode_id != plan.episode_id
            || observation.parent_policy_receipt.camera_id != plan.sensor_id
            || observation.parent_policy_receipt.sequence != observation.sequence
            || observation.parent_policy_receipt.capture_timestamp_us != observation.capture_timestamp_us
        {
            return Err(FaultApplyError::ParentReceiptIdentityMismatch);
        }
        let receipt = &observation.parent_policy_receipt;
        let expected_length = (receipt.frame_width as usize)
            .checked_mul(receipt.frame_height as usize)
            .and_then(|pixels| pixels.checked_mul(receipt.channels as usize))
            .ok_or(FaultApplyError::ParentReceiptGeometryMismatch)?;
        if receipt.channels != 3
            || receipt.pixel_format
                != action_conditioned_offscreen_camera::PixelFormat::Rgb8Srgb
            || receipt.frame_byte_len != expected_length
            || expected_length > MAX_FRAME_BYTES
        {
            return Err(FaultApplyError::ParentReceiptGeometryMismatch);
        }
        if !is_sha256(&receipt.observation_id)
            || !is_sha256(&receipt.receipt_sha256)
            || !is_sha256(&receipt.frame_sha256)
        {
            return Err(FaultApplyError::ParentReceiptDigestMismatch);
        }
        if let Some(control) = &observation.control_timing {
            if control.observation_timestamp_us != observation.capture_timestamp_us
                || control.inference_completed_timestamp_us < control.observation_timestamp_us
                || control.command_timestamp_us < control.inference_completed_timestamp_us
                || control.original_applied_control_timestamp_us < control.command_timestamp_us
                || control.applied_control_timestamp_us < control.command_timestamp_us
                || control.command_id.trim().is_empty()
            {
                return Err(FaultApplyError::InvalidControlTiming);
            }
        }
        previous_sequence = Some(observation.sequence);
        previous_capture = Some(observation.capture_timestamp_us);
        previous_delivery = Some(observation.delivered_timestamp_us);
    }
    Ok(())
}

fn seed_for(plan: &FaultPlan, observation: &CleanObservation, channel: &str) -> String {
    let material = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        plan.campaign_id,
        plan.pair_id,
        plan.scenario_id,
        plan.episode_id,
        plan.sensor_id,
        plan.event.event_id,
        observation.sequence,
        channel
    );
    sha256_hex(material.as_bytes())
}

fn parameters_digest(transform: &FaultTransform) -> String {
    let bytes = serde_json::to_vec(transform).unwrap_or_default();
    sha256_hex(&bytes)
}

fn normal_q16(rng: &mut DeterministicRng) -> i64 {
    // Irwin-Hall CLT approximation: sum of 12 independent U[0,1) samples.
    // Centered sum has standard deviation close to 1.0 in Q16.16 units.
    let mut sum = 0_i64;
    for _ in 0..12 {
        sum += (rng.next_u64() & 0xffff) as i64;
    }
    sum - 393_210
}

fn checked_add_signed(value: u64, delta: i64) -> Result<u64, FaultApplyError> {
    if delta >= 0 {
        value
            .checked_add(delta as u64)
            .ok_or(FaultApplyError::TimestampOverflow)
    } else {
        value
            .checked_sub(delta.unsigned_abs())
            .ok_or(FaultApplyError::TimestampUnderflow)
    }
}

fn build_derivation(
    plan: &FaultPlan,
    source: &CleanObservation,
    current: &CleanObservation,
    receipt: &PolicyObservationReceipt,
    source_pixels: &[u8],
    vehicle_signals: &VehicleSignals,
    sampled_values: BTreeMap<String, i64>,
    geometry: BTreeMap<String, u64>,
    seed_sha256: String,
) -> Result<FaultDerivationReceipt, FaultApplyError> {
    let source_state_bytes =
        serde_json::to_vec(&current.vehicle_signals).map_err(|_| FaultApplyError::InvalidDigest)?;
    let child_state_bytes =
        serde_json::to_vec(vehicle_signals).map_err(|_| FaultApplyError::InvalidDigest)?;
    let mut result = FaultDerivationReceipt {
        schema_version: 1,
        campaign_id: plan.campaign_id.clone(),
        pair_id: plan.pair_id.clone(),
        scenario_id: plan.scenario_id.clone(),
        episode_id: plan.episode_id.clone(),
        event_id: plan.event.event_id.clone(),
        transform_revision: FAULT_APPLICATOR_REVISION.to_string(),
        parameters_sha256: parameters_digest(&plan.event.transform),
        seed_sha256,
        parent_observation_id: current.parent_policy_receipt.observation_id.clone(),
        parent_receipt_sha256: current.parent_policy_receipt.receipt_sha256.clone(),
        source_observation_id: source.parent_policy_receipt.observation_id.clone(),
        source_sequence: source.sequence,
        child_observation_id: receipt.observation_id.clone(),
        child_receipt_sha256: receipt.receipt_sha256.clone(),
        parent_frame_sha256: current.parent_policy_receipt.frame_sha256.clone(),
        child_frame_sha256: sha256_hex(source_pixels),
        parent_vehicle_signals_sha256: sha256_hex(&source_state_bytes),
        child_vehicle_signals_sha256: sha256_hex(&child_state_bytes),
        sampled_values,
        geometry,
        receipt_sha256: String::new(),
    };
    let bytes = serde_json::to_vec(&result).map_err(|_| FaultApplyError::InvalidDigest)?;
    result.receipt_sha256 = sha256_hex(&bytes);
    Ok(result)
}

fn apply_occlusion(
    plan: &FaultPlan,
    observation: &CleanObservation,
    original_pixels: &[u8],
    occlusion_ppm: u32,
    mask_mode: MaskMode,
) -> Result<(Vec<u8>, BTreeMap<String, u64>, String), FaultApplyError> {
    let receipt = &observation.parent_policy_receipt;
    let width = receipt.frame_width as usize;
    let height = receipt.frame_height as usize;
    let channels = receipt.channels as usize;
    let columns = ((width as u128 * occlusion_ppm as u128) + 500_000) / 1_000_000;
    let occluded_width = usize::try_from(columns).map_err(|_| FaultApplyError::PixelTransformOverflow)?;
    if occluded_width == 0 || occluded_width >= width {
        return Err(FaultApplyError::InvalidFaultSeverity);
    }
    let channel = match mask_mode {
        MaskMode::FixedPerEpisode => "camera-occlusion-mask-fixed",
        MaskMode::ResampledPerFrame => "camera-occlusion-mask-frame",
    };
    let seed_context = if mask_mode == MaskMode::FixedPerEpisode {
        format!("{}|{}|{}|{}|{}|{}|{}",
            plan.campaign_id, plan.pair_id, plan.scenario_id, plan.episode_id,
            plan.sensor_id, plan.event.event_id, channel)
    } else {
        seed_for(plan, observation, channel)
    };
    let seed = sha256_hex(seed_context.as_bytes());
    let mut rng = DeterministicRng::from_seed(&seed);
    let offset = rng.below((width - occluded_width + 1) as u64) as usize;
    let mut output = original_pixels.to_vec();
    for row in 0..height {
        let start = (row * width + offset) * channels;
        let end = (row * width + offset + occluded_width) * channels;
        output[start..end].fill(0);
    }
    let occluded_pixels = occluded_width * height;
    let actual_ppm = ((occluded_pixels as u128 * 1_000_000) / (width * height) as u128) as u64;
    let mut geometry = BTreeMap::new();
    geometry.insert("mask_offset_px".into(), offset as u64);
    geometry.insert("mask_width_px".into(), occluded_width as u64);
    geometry.insert("occluded_pixels".into(), occluded_pixels as u64);
    geometry.insert("actual_occlusion_ppm".into(), actual_ppm);
    Ok((output, geometry, seed))
}

fn apply_corruption(
    plan: &FaultPlan,
    observation: &CleanObservation,
    original_pixels: &[u8],
    corrupt_byte_ppm: u32,
) -> Result<(Vec<u8>, BTreeMap<String, u64>, String), FaultApplyError> {
    let count = ((original_pixels.len() as u128 * corrupt_byte_ppm as u128) / 1_000_000)
        .max(1) as usize;
    if count > original_pixels.len() {
        return Err(FaultApplyError::InvalidFaultSeverity);
    }
    let seed = seed_for(plan, observation, "pixel-corruption");
    let mut rng = DeterministicRng::from_seed(&seed);
    let start = rng.below((original_pixels.len() - count + 1) as u64) as usize;
    let mut output = original_pixels.to_vec();
    for byte in &mut output[start..start + count] {
        *byte ^= 0x80;
    }
    let mut geometry = BTreeMap::new();
    geometry.insert("corrupted_byte_start".into(), start as u64);
    geometry.insert("corrupted_byte_count".into(), count as u64);
    Ok((output, geometry, seed))
}

fn apply_gps_noise(
    plan: &FaultPlan,
    observation: &CleanObservation,
    state: &mut VehicleSignals,
    sigma_mm: u64,
) -> (BTreeMap<String, i64>, String) {
    let seed = seed_for(plan, observation, "gps-localization");
    let mut rng = DeterministicRng::from_seed(&seed);
    let x_q16 = normal_q16(&mut rng);
    let y_q16 = normal_q16(&mut rng);
    let dx = (sigma_mm as i128 * x_q16 as i128 / 65_536) as i64;
    let dy = (sigma_mm as i128 * y_q16 as i128 / 65_536) as i64;
    state.gps_x_mm = state.gps_x_mm.saturating_add(dx);
    state.gps_y_mm = state.gps_y_mm.saturating_add(dy);
    let mut values = BTreeMap::new();
    values.insert("gps_delta_x_mm".into(), dx);
    values.insert("gps_delta_y_mm".into(), dy);
    values.insert("gps_sigma_mm".into(), sigma_mm as i64);
    values.insert("gps_x_normal_q16".into(), x_q16);
    values.insert("gps_y_normal_q16".into(), y_q16);
    (values, seed)
}

fn apply_speed_noise(
    plan: &FaultPlan,
    observation: &CleanObservation,
    state: &mut VehicleSignals,
    mean_ppm: i64,
    standard_deviation_ppm: i64,
    minimum_multiplier_ppm: i64,
    maximum_multiplier_ppm: i64,
) -> (BTreeMap<String, i64>, String) {
    let seed = seed_for(plan, observation, "speed-estimation");
    let mut rng = DeterministicRng::from_seed(&seed);
    let z_q16 = normal_q16(&mut rng);
    let sampled = mean_ppm + (standard_deviation_ppm as i128 * z_q16 as i128 / 65_536) as i64;
    let clamped = sampled.clamp(minimum_multiplier_ppm, maximum_multiplier_ppm);
    state.speed_mm_per_s =
        (state.speed_mm_per_s as i128 * clamped as i128 / 1_000_000) as i64;
    let mut values = BTreeMap::new();
    values.insert("speed_normal_q16".into(), z_q16);
    values.insert("speed_sampled_multiplier_ppm".into(), sampled);
    values.insert("speed_applied_multiplier_ppm".into(), clamped);
    values.insert("speed_clipped".into(), i64::from(sampled != clamped));
    (values, seed)
}

fn apply_control_delay(
    plan: &FaultPlan,
    observation: &CleanObservation,
    control: &mut Option<ControlTiming>,
    delay_us: u64,
) -> Result<(BTreeMap<String, i64>, String), FaultApplyError> {
    let timing = control.as_mut().ok_or(FaultApplyError::MissingControlTiming)?;
    let seed = seed_for(plan, observation, "inference-control-delay");
    let original = timing.original_applied_control_timestamp_us;
    let delayed = timing
        .command_timestamp_us
        .checked_add(delay_us)
        .ok_or(FaultApplyError::ControlTimestampOverflow)?;
    timing.applied_control_timestamp_us = delayed.max(original);
    let mut values = BTreeMap::new();
    values.insert("requested_delay_us".into(), delay_us as i64);
    values.insert("original_applied_control_timestamp_us".into(), original as i64);
    values.insert("delayed_applied_control_timestamp_us".into(), timing.applied_control_timestamp_us as i64);
    Ok((values, seed))
}

fn apply_clock_shift(
    plan: &FaultPlan,
    observation: &CleanObservation,
    overrides: &mut PolicyReceiptOverrides,
    capture_delta_us: i64,
    render_completed_delta_us: i64,
    delivered_delta_us: i64,
) -> Result<(BTreeMap<String, i64>, String), FaultApplyError> {
    let capture = checked_add_signed(observation.capture_timestamp_us, capture_delta_us)?;
    let render = checked_add_signed(observation.render_completed_timestamp_us, render_completed_delta_us)?;
    let delivered = checked_add_signed(observation.delivered_timestamp_us, delivered_delta_us)?;
    overrides.capture_timestamp_us = Some(capture);
    overrides.render_completed_timestamp_us = Some(render);
    overrides.delivered_timestamp_us = Some(delivered);
    let mut values = BTreeMap::new();
    values.insert("capture_delta_us".into(), capture_delta_us);
    values.insert("render_completed_delta_us".into(), render_completed_delta_us);
    values.insert("delivered_delta_us".into(), delivered_delta_us);
    Ok((values, seed_for(plan, observation, "clock-shift")))
}

fn calibration_mismatch_override(plan: &FaultPlan, observation: &CleanObservation) -> (String, String) {
    let seed = seed_for(plan, observation, "camera-calibration-mismatch");
    let material = format!("faulted-calibration|{seed}|{}", observation.parent_policy_receipt.camera_calibration_sha256);
    (sha256_hex(material.as_bytes()), seed)
}

/// Apply a single event to a clean episode. Every clean frame is hash-checked
/// before any transformation. The source slice and source receipt objects are
/// never mutated; output frames are owned child buffers.
pub fn apply_fault_plan(
    plan: &FaultPlan,
    observations: &[CleanObservation],
    pixels_by_sequence: &BTreeMap<u64, Vec<u8>>,
) -> Result<FaultApplication, FaultApplyError> {
    validate_plan(plan, observations)?;
    let transform = &plan.event.transform;
    let parameter_sha256 = parameters_digest(transform);
    let mut output = Vec::with_capacity(observations.len());
    let mut missing_count = 0_usize;
    let mut derived_count = 0_usize;
    let mut stale_count = 0_usize;
    let mut clean_source_bytes = 0_u64;
    let clean_frames = observations.iter().map(|item| item.sequence).collect::<Vec<_>>();

    for (index, current) in observations.iter().enumerate() {
        let parent_pixels = pixels_by_sequence
            .get(&current.sequence)
            .ok_or(FaultApplyError::MissingPixels)?;
        let parent_receipt = &current.parent_policy_receipt;
        if parent_pixels.len() != parent_receipt.frame_byte_len {
            return Err(FaultApplyError::PixelLengthMismatch);
        }
        if sha256_hex(parent_pixels) != parent_receipt.frame_sha256 {
            return Err(FaultApplyError::ParentFrameDigestMismatch);
        }
        clean_source_bytes = clean_source_bytes
            .checked_add(parent_pixels.len() as u64)
            .ok_or(FaultApplyError::PixelTransformOverflow)?;

        let active = (plan.event.start_sequence..=plan.event.end_sequence).contains(&current.sequence);
        if !active {
            output.push(FaultedObservation {
                sequence: current.sequence,
                status: ObservationStatus::Clean,
                policy_receipt: Some(parent_receipt.clone()),
                pixels_rgb: Some(parent_pixels.clone()),
                vehicle_signals: Some(current.vehicle_signals.clone()),
                control_timing: current.control_timing.clone(),
                derivation: None,
                missing_event: None,
            });
            continue;
        }

        if matches!(transform, FaultTransform::CameraBurstDrop { .. }) {
            missing_count += 1;
            output.push(FaultedObservation {
                sequence: current.sequence,
                status: ObservationStatus::Missing,
                policy_receipt: None,
                pixels_rgb: None,
                vehicle_signals: None,
                control_timing: current.control_timing.clone(),
                derivation: None,
                missing_event: Some(MissingFrameEvent {
                    episode_id: plan.episode_id.clone(),
                    pair_id: plan.pair_id.clone(),
                    event_id: plan.event.event_id.clone(),
                    sequence: current.sequence,
                    capture_timestamp_us: current.capture_timestamp_us,
                    noticed_timestamp_us: current
                        .capture_timestamp_us
                        .saturating_add(1),
                    reason: MissingFrameReason::CameraDrop,
                }),
            });
            continue;
        }

        let mut child_pixels = parent_pixels.clone();
        let mut child_state = current.vehicle_signals.clone();
        let mut child_control = current.control_timing.clone();
        let mut receipt_overrides = PolicyReceiptOverrides::default();
        let mut sampled_values = BTreeMap::new();
        let mut geometry = BTreeMap::new();
        let mut seed_sha256 = seed_for(plan, current, "identity");
        let mut source_observation = current;
        let mut status = ObservationStatus::Derived;

        match transform {
            FaultTransform::CameraBurstDrop { .. } => unreachable!(),
            FaultTransform::CameraOcclusion {
                occlusion_ppm,
                mask_mode,
            } => {
                let (pixels, mask, seed) =
                    apply_occlusion(plan, current, parent_pixels, *occlusion_ppm, *mask_mode)?;
                child_pixels = pixels;
                geometry = mask;
                seed_sha256 = seed;
            }
            FaultTransform::GpsLocalizationNoise { sigma_mm } => {
                let (values, seed) =
                    apply_gps_noise(plan, current, &mut child_state, *sigma_mm);
                sampled_values = values;
                seed_sha256 = seed;
            }
            FaultTransform::SpeedMultiplierNoise {
                mean_ppm,
                standard_deviation_ppm,
                minimum_multiplier_ppm,
                maximum_multiplier_ppm,
            } => {
                let (values, seed) = apply_speed_noise(
                    plan,
                    current,
                    &mut child_state,
                    *mean_ppm,
                    *standard_deviation_ppm,
                    *minimum_multiplier_ppm,
                    *maximum_multiplier_ppm,
                );
                sampled_values = values;
                seed_sha256 = seed;
            }
            FaultTransform::InferenceControlDelay { delay_us } => {
                let (values, seed) =
                    apply_control_delay(plan, current, &mut child_control, *delay_us)?;
                sampled_values = values;
                seed_sha256 = seed;
            }
            FaultTransform::StaleFrameReplay => {
                if index == 0 {
                    return Err(FaultApplyError::MissingPriorFrameForReplay);
                }
                source_observation = &observations[index - 1];
                let previous = pixels_by_sequence
                    .get(&source_observation.sequence)
                    .ok_or(FaultApplyError::MissingPixels)?;
                child_pixels = previous.clone();
                // Keep the old upstream policy receipt, sequence and capture time.
                // The bridge should reject this when presented at the current tick.
                status = ObservationStatus::StaleReplayNegativeControl;
                seed_sha256 = seed_for(plan, current, "stale-frame-replay");
            }
            FaultTransform::PixelCorruption { corrupt_byte_ppm } => {
                let (pixels, mask, seed) =
                    apply_corruption(plan, current, parent_pixels, *corrupt_byte_ppm)?;
                child_pixels = pixels;
                geometry = mask;
                seed_sha256 = seed;
            }
            FaultTransform::ClockShift {
                capture_delta_us,
                render_completed_delta_us,
                delivered_delta_us,
            } => {
                let (values, seed) = apply_clock_shift(
                    plan,
                    current,
                    &mut receipt_overrides,
                    *capture_delta_us,
                    *render_completed_delta_us,
                    *delivered_delta_us,
                )?;
                sampled_values = values;
                seed_sha256 = seed;
            }
            FaultTransform::CalibrationMismatch => {
                let (digest, seed) = calibration_mismatch_override(plan, current);
                receipt_overrides.camera_calibration_sha256 = Some(digest.clone());
                sampled_values.insert("replacement_camera_calibration_digest_prefix".into(),
                    i64::from_str_radix(&digest[..8], 16).unwrap_or(0));
                seed_sha256 = seed;
            }
        }

        let source_pixels = if source_observation.sequence == current.sequence {
            parent_pixels.as_slice()
        } else {
            pixels_by_sequence
                .get(&source_observation.sequence)
                .ok_or(FaultApplyError::MissingPixels)?
                .as_slice()
        };
        let child_receipt = if status == ObservationStatus::StaleReplayNegativeControl {
            source_observation.parent_policy_receipt.clone()
        } else {
            SoftwareCameraRenderer::derive_policy_observation(
                parent_receipt,
                source_pixels,
                &child_pixels,
                receipt_overrides,
            )
            .map_err(|_| FaultApplyError::ParentReceiptDigestMismatch)?
        };

        let sidecar = build_derivation(
            plan,
            source_observation,
            current,
            &child_receipt,
            &child_pixels,
            &child_state,
            sampled_values,
            geometry,
            seed_sha256,
        )?;
        derived_count += 1;
        if status == ObservationStatus::StaleReplayNegativeControl {
            stale_count += 1;
        }

        output.push(FaultedObservation {
            sequence: current.sequence,
            status,
            policy_receipt: Some(child_receipt),
            pixels_rgb: Some(child_pixels),
            vehicle_signals: Some(child_state),
            control_timing: child_control,
            derivation: Some(sidecar),
            missing_event: None,
        });
    }

    let delivered = output
        .iter()
        .filter(|item| item.status != ObservationStatus::Missing)
        .count();
    let report = FaultApplicationReport {
        schema_version: 1,
        campaign_id: plan.campaign_id.clone(),
        pair_id: plan.pair_id.clone(),
        episode_id: plan.episode_id.clone(),
        event_id: plan.event.event_id.clone(),
        transform_kind: transform_name(transform).to_string(),
        expected_observations: observations.len(),
        delivered_observations: delivered,
        derived_observations: derived_count,
        missing_observations: missing_count,
        stale_replay_negative_controls: stale_count,
        clean_source_frames_verified: clean_frames.len(),
        clean_source_bytes_mutated: 0,
        claim_ceiling: "Deterministic software transform and receipt replay for declared clean frame artifacts only. This is not physical fault injection, trained-policy robustness or a safety case.".to_string(),
    };
    Ok(FaultApplication {
        observations: output,
        report,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use action_conditioned_offscreen_camera::{
        ActorClass, ActorCuboid, AppliedActionRef, CameraCalibration, Q16_ONE,
        RenderClocks, RoadMarkingKind, RoadPolyline, WorldRenderSnapshot,
    };

    fn clean_fixture(count: u64, width: u32, height: u32) -> (Vec<CleanObservation>, BTreeMap<u64, Vec<u8>>) {
        let mut observations = Vec::new();
        let mut pixels = BTreeMap::new();
        for sequence in 1..=count {
            let mut world = WorldRenderSnapshot {
                episode_id: "episode-fault-fixture-v1".into(),
                scenario_id: "paired-driving-fault-fixture-v1".into(),
                agent_id: "ego".into(),
                tick: sequence,
                world_timestamp_us: 1_000_000 + sequence * 33_333,
                clock_id: "sim-monotonic-fixture-v1".into(),
                simulator_revision: "snapshot-fixture-v1".into(),
                applied_action: Some(AppliedActionRef {
                    receipt_id: format!("action-{sequence}"),
                    receipt_sha256: format!("{:064x}", sequence + 100),
                    command_id: format!("control-{sequence}"),
                    applied_tick: sequence,
                }),
                camera: CameraCalibration {
                    camera_id: "front-rgb".into(),
                    position_world_mm: [0, 0, 1_500],
                    world_to_camera_q16: [
                        [Q16_ONE as i32, 0, 0],
                        [0, 0, -(Q16_ONE as i32)],
                        [0, Q16_ONE as i32, 0],
                    ],
                    focal_x_millipx: 450_000,
                    focal_y_millipx: 450_000,
                    principal_x_millipx: width * 500,
                    principal_y_millipx: height * 500,
                    width,
                    height,
                    near_plane_mm: 100,
                    far_plane_mm: 200_000,
                },
                road_markings: vec![RoadPolyline {
                    road_id: "road-1".into(),
                    marking_id: "centerline-1".into(),
                    kind: RoadMarkingKind::LaneWhite,
                    points_world_mm: vec![[0, 10_000, 0], [0, 50_000, 0]],
                }],
                actors: vec![ActorCuboid {
                    actor_id: "lead-car".into(),
                    actor_policy_revision: "lead-policy-v1".into(),
                    class: ActorClass::Car,
                    center_world_mm: [0, 20_000, 500],
                    half_extents_mm: [800, 1_500, 500],
                }],
            };
            let _ = &mut world;
            let capture = world.world_timestamp_us;
            let rendered = SoftwareCameraRenderer::render(
                &world,
                sequence,
                RenderClocks {
                    capture_timestamp_us: capture,
                    render_completed_timestamp_us: capture + 500,
                    delivered_timestamp_us: capture + 800,
                },
            )
            .unwrap();
            pixels.insert(sequence, rendered.pixels_rgb.clone());
            observations.push(CleanObservation {
                sequence,
                capture_timestamp_us: rendered.policy_receipt.capture_timestamp_us,
                render_completed_timestamp_us: rendered.policy_receipt.render_completed_timestamp_us,
                delivered_timestamp_us: rendered.policy_receipt.delivered_timestamp_us,
                pixels_file: format!("frames/frame_{sequence:04}.rgb"),
                parent_policy_receipt: rendered.policy_receipt,
                vehicle_signals: VehicleSignals {
                    gps_x_mm: sequence as i64 * 100,
                    gps_y_mm: sequence as i64 * 200,
                    speed_mm_per_s: 10_000,
                },
                control_timing: Some(ControlTiming {
                    command_id: format!("control-{sequence}"),
                    observation_timestamp_us: capture,
                    inference_completed_timestamp_us: capture + 4_000,
                    command_timestamp_us: capture + 5_000,
                    original_applied_control_timestamp_us: capture + 6_000,
                    applied_control_timestamp_us: capture + 6_000,
                }),
            });
        }
        (observations, pixels)
    }

    fn plan(transform: FaultTransform, count: u64) -> FaultPlan {
        FaultPlan {
            schema_version: 1,
            campaign_id: "campaign-fault-fixture-v1".into(),
            pair_id: "pair-fault-fixture-v1".into(),
            scenario_id: "paired-driving-fault-fixture-v1".into(),
            episode_id: "episode-fault-fixture-v1".into(),
            sensor_id: "front-rgb".into(),
            scenario_seed_sha256: "a".repeat(64),
            actor_randomness_sha256: "b".repeat(64),
            event: FaultEvent {
                event_id: "fault-event-001".into(),
                start_sequence: 1,
                end_sequence: count,
                transform,
            },
        }
    }

    #[test]
    fn occlusion_50_and_80_percent_are_deterministic_and_leave_clean_source_unchanged() {
        let (observations, frames) = clean_fixture(2, 640, 360);
        let source = frames[&1].clone();
        for occlusion_ppm in [500_000, 800_000] {
            let p = plan(
                FaultTransform::CameraOcclusion {
                    occlusion_ppm,
                    mask_mode: MaskMode::FixedPerEpisode,
                },
                2,
            );
            let first = apply_fault_plan(&p, &observations, &frames).unwrap();
            let second = apply_fault_plan(&p, &observations, &frames).unwrap();
            assert_eq!(first, second);
            assert_eq!(frames[&1], source);
            let geometry = &first.observations[0].derivation.as_ref().unwrap().geometry;
            assert_eq!(geometry["actual_occlusion_ppm"], occlusion_ppm as u64);
            assert_ne!(
                first.observations[0].policy_receipt.as_ref().unwrap().frame_sha256,
                observations[0].parent_policy_receipt.frame_sha256
            );
        }
    }

    #[test]
    fn camera_drop_emits_one_missing_event_per_tick_and_no_fake_frame() {
        let (observations, frames) = clean_fixture(20, 32, 24);
        let result = apply_fault_plan(
            &plan(FaultTransform::CameraBurstDrop { duration_ticks: 20 }, 20),
            &observations,
            &frames,
        )
        .unwrap();
        assert_eq!(result.report.expected_observations, 20);
        assert_eq!(result.report.missing_observations, 20);
        assert_eq!(result.report.delivered_observations, 0);
        assert!(result.observations.iter().all(|item| {
            item.status == ObservationStatus::Missing
                && item.policy_receipt.is_none()
                && item.pixels_rgb.is_none()
                && item.missing_event.is_some()
        }));
    }

    #[test]
    fn gps_noise_is_deterministic_and_preserves_original_receipts() {
        let (observations, frames) = clean_fixture(3, 32, 24);
        let p = plan(FaultTransform::GpsLocalizationNoise { sigma_mm: 5_000 }, 3);
        let first = apply_fault_plan(&p, &observations, &frames).unwrap();
        let second = apply_fault_plan(&p, &observations, &frames).unwrap();
        assert_eq!(first, second);
        for (index, output) in first.observations.iter().enumerate() {
            assert_eq!(
                output.policy_receipt.as_ref().unwrap(),
                &observations[index].parent_policy_receipt
            );
            assert_ne!(
                output.vehicle_signals.as_ref().unwrap(),
                &observations[index].vehicle_signals
            );
            assert_eq!(output.derivation.as_ref().unwrap().parent_observation_id,
                observations[index].parent_policy_receipt.observation_id);
        }
    }

    #[test]
    fn speed_noise_records_sampled_and_clipped_multiplier() {
        let (observations, frames) = clean_fixture(4, 32, 24);
        let p = plan(
            FaultTransform::SpeedMultiplierNoise {
                mean_ppm: 500_000,
                standard_deviation_ppm: 200_000,
                minimum_multiplier_ppm: 0,
                maximum_multiplier_ppm: 1_500_000,
            },
            4,
        );
        let result = apply_fault_plan(&p, &observations, &frames).unwrap();
        for output in result.observations {
            let values = &output.derivation.unwrap().sampled_values;
            assert!(values["speed_applied_multiplier_ppm"] >= 0);
            assert!(values["speed_applied_multiplier_ppm"] <= 1_500_000);
            assert_eq!(
                values["speed_clipped"],
                i64::from(
                    values["speed_sampled_multiplier_ppm"]
                        != values["speed_applied_multiplier_ppm"]
                )
            );
        }
    }

    #[test]
    fn fixed_control_delay_preserves_capture_clocks_and_fifo_order() {
        let (observations, frames) = clean_fixture(5, 32, 24);
        let p = plan(FaultTransform::InferenceControlDelay { delay_us: 200_000 }, 5);
        let result = apply_fault_plan(&p, &observations, &frames).unwrap();
        let applied = result
            .observations
            .iter()
            .map(|item| {
                assert_eq!(
                    item.policy_receipt.as_ref().unwrap().capture_timestamp_us,
                    observations[(item.sequence - 1) as usize].capture_timestamp_us
                );
                item.control_timing.as_ref().unwrap().applied_control_timestamp_us
            })
            .collect::<Vec<_>>();
        assert!(applied.windows(2).all(|window| window[1] > window[0]));
        assert_eq!(
            applied[0] - observations[0].control_timing.as_ref().unwrap().command_timestamp_us,
            200_000
        );
    }

    #[test]
    fn stale_replay_preserves_the_old_capture_and_sequence_identity() {
        let (observations, frames) = clean_fixture(3, 32, 24);
        let p = plan(FaultTransform::StaleFrameReplay, 3);
        let result = apply_fault_plan(&p, &observations, &frames).unwrap();
        let second = &result.observations[1];
        assert_eq!(second.status, ObservationStatus::StaleReplayNegativeControl);
        assert_eq!(
            second.policy_receipt.as_ref().unwrap().sequence,
            observations[0].sequence
        );
        assert_eq!(
            second.policy_receipt.as_ref().unwrap().capture_timestamp_us,
            observations[0].capture_timestamp_us
        );
        assert_ne!(
            second.policy_receipt.as_ref().unwrap().sequence,
            second.sequence
        );
    }

    #[test]
    fn pixel_corruption_is_deterministic_and_child_receipt_binds_bytes() {
        let (observations, frames) = clean_fixture(2, 32, 24);
        let p = plan(FaultTransform::PixelCorruption { corrupt_byte_ppm: 10_000 }, 2);
        let a = apply_fault_plan(&p, &observations, &frames).unwrap();
        let b = apply_fault_plan(&p, &observations, &frames).unwrap();
        assert_eq!(a, b);
        let item = &a.observations[0];
        let bytes = item.pixels_rgb.as_ref().unwrap();
        let receipt = item.policy_receipt.as_ref().unwrap();
        assert_eq!(sha256_hex(bytes), receipt.frame_sha256);
        assert_ne!(receipt.frame_sha256, observations[0].parent_policy_receipt.frame_sha256);
    }

    #[test]
    fn clock_shift_can_create_a_self_consistent_regression_negative_control() {
        let (observations, frames) = clean_fixture(2, 32, 24);
        let p = plan(
            FaultTransform::ClockShift {
                capture_delta_us: -100_000,
                render_completed_delta_us: -100_000,
                delivered_delta_us: -100_000,
            },
            2,
        );
        let result = apply_fault_plan(&p, &observations, &frames).unwrap();
        assert_eq!(
            result.observations[0].policy_receipt.as_ref().unwrap().capture_timestamp_us,
            observations[0].capture_timestamp_us - 100_000
        );
        assert_eq!(
            result.observations[0].policy_receipt.as_ref().unwrap().receipt_sha256,
            result.observations[0].derivation.as_ref().unwrap().child_receipt_sha256
        );
    }

    #[test]
    fn calibration_mismatch_is_digest_valid_but_distinct_from_parent() {
        let (observations, frames) = clean_fixture(1, 32, 24);
        let result = apply_fault_plan(
            &plan(FaultTransform::CalibrationMismatch, 1),
            &observations,
            &frames,
        )
        .unwrap();
        let child = result.observations[0].policy_receipt.as_ref().unwrap();
        assert_ne!(
            child.camera_calibration_sha256,
            observations[0].parent_policy_receipt.camera_calibration_sha256
        );
        assert!(is_sha256(&child.camera_calibration_sha256));
        assert_ne!(child.receipt_sha256, observations[0].parent_policy_receipt.receipt_sha256);
    }

    #[test]
    fn rejects_mismatched_parent_pixels_before_transforming() {
        let (observations, mut frames) = clean_fixture(1, 32, 24);
        frames.get_mut(&1).unwrap()[0] ^= 1;
        assert_eq!(
            apply_fault_plan(
                &plan(FaultTransform::PixelCorruption { corrupt_byte_ppm: 10_000 }, 1),
                &observations,
                &frames
            )
            .unwrap_err(),
            FaultApplyError::ParentFrameDigestMismatch
        );
    }

    #[test]
    fn rejects_wrong_bench2drive_robust_severity() {
        let (observations, frames) = clean_fixture(1, 32, 24);
        let error = apply_fault_plan(
            &plan(FaultTransform::CameraOcclusion {
                occlusion_ppm: 750_000,
                mask_mode: MaskMode::FixedPerEpisode,
            }, 1),
            &observations,
            &frames,
        )
        .unwrap_err();
        assert_eq!(error, FaultApplyError::InvalidFaultSeverity);
    }
}
