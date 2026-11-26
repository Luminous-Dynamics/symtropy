// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Cursor,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SCHEMA_VERSION: &str = "0.1";
pub const BENCHMARK: &str = "RRVR-REPLAY-FIDELITY";
pub const PROFILE_ID: &str = "RoboRecoverPublishedThresholdsV1";
pub const METRIC_PROFILE_ID: &str = "RRVR-SSIM-GAUSSIAN11-SIGMA1_5-RGB8-V0_1";
pub const POSE_THRESHOLD_MM: f64 = 2.0;
pub const SSIM_THRESHOLD: f64 = 0.99;
pub const FRAME_PAIR_N: usize = 5;
pub const SCREENING_TRIAL_N: usize = 20;
pub const SCREENING_SUCCESS_MIN: usize = 2;
pub const SCREENING_SUCCESS_MAX: usize = 18;
pub const SPOT_CHECK_SEED: u64 = 42;
pub const SPOT_CHECK_N: usize = 3;

const SSIM_WINDOW: usize = 11;
const SSIM_SIGMA: f64 = 1.5;
const SSIM_K1: f64 = 0.01;
const SSIM_K2: f64 = 0.03;
const SSIM_L: f64 = 255.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    RoboTwin,
    Libero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Split {
    Train,
    Test,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditStatus {
    Valid,
    Invalid,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DifficultyTrial {
    pub attempt_id: String,
    pub policy_id: String,
    pub trial_index: u8,
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DifficultyScreen {
    pub trials: Vec<DifficultyTrial>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpotCheckSelectionRecord {
    pub schema_version: String,
    pub seed: u64,
    pub selection_method_id: String,
    pub selected_attempt_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpotCheckAttempt {
    pub attempt_id: String,
    pub end_effector: PoseComparison,
    pub object_pose: PoseComparison,
    pub final_frame_pairs: Vec<FramePairAsset>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpotCheckManifest {
    pub seed: u64,
    pub selection_method_id: String,
    pub selection_record_path: String,
    pub selection_record_sha256: String,
    pub selected_attempt_ids: Vec<String>,
    pub attempts: Vec<SpotCheckAttempt>,
    pub manifest_sha256: String,
}

#[derive(Serialize)]
struct SpotCheckManifestMaterial<'a> {
    seed: u64,
    selection_method_id: &'a str,
    selection_record_path: &'a str,
    selection_record_sha256: &'a str,
    selected_attempt_ids: &'a [String],
    attempts: &'a [SpotCheckAttempt],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositionMm {
    pub coordinate_frame_id: String,
    pub x_mm: f64,
    pub y_mm: f64,
    pub z_mm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoseComparison {
    pub original: PositionMm,
    pub replayed: PositionMm,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FramePairAsset {
    pub frame_index: u8,
    pub original_path: String,
    pub original_sha256: String,
    pub replay_path: String,
    pub replay_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayCandidate {
    pub scenario_id: String,
    pub platform: Platform,
    pub split: Split,
    pub source_episode_sha256: String,
    pub source_trajectory_id: String,
    pub trajectory_cluster_id: String,
    pub instruction_sha256: String,
    pub action_prefix_sha256: String,
    pub split_leakage_keys_sha256: Vec<String>,
    pub simulator_profile_sha256: String,
    pub robot_model_profile_sha256: String,
    pub controller_profile_sha256: String,
    pub rendering_profile_sha256: String,
    pub reset_receipt_sha256: String,
    pub action_prefix_receipt_sha256: String,
    pub difficulty_screen: DifficultyScreen,
    pub spot_check: SpotCheckManifest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayManifest {
    pub schema_version: String,
    pub benchmark: String,
    pub profile_id: String,
    pub metric_profile_id: String,
    pub campaign_id: String,
    pub split_manifest_sha256: String,
    pub data_license_and_provenance: String,
    pub candidates: Vec<ReplayCandidate>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PoseCheck {
    pub observed_error_mm: Option<f64>,
    pub threshold_mm: f64,
    pub passed: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FrameCheck {
    pub frame_index: u8,
    pub original_sha256: String,
    pub replay_sha256: String,
    pub original_width: Option<u32>,
    pub original_height: Option<u32>,
    pub replay_width: Option<u32>,
    pub replay_height: Option<u32>,
    pub ssim: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SpotCheckAudit {
    pub attempt_id: String,
    pub status: AuditStatus,
    pub end_effector: PoseCheck,
    pub object_pose: PoseCheck,
    pub frame_checks: Vec<FrameCheck>,
    pub mean_final_five_ssim: Option<f64>,
    pub mean_ssim_threshold: f64,
    pub mean_ssim_passed: Option<bool>,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CandidateAudit {
    pub scenario_id: String,
    pub platform: Platform,
    pub split: Split,
    pub source_trajectory_id: String,
    pub trajectory_cluster_id: String,
    pub status: AuditStatus,
    pub difficulty_success_n: usize,
    pub difficulty_trial_n: usize,
    pub selection_method_id: String,
    pub spot_check_attempt_ids: Vec<String>,
    pub spot_checks: Vec<SpotCheckAudit>,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReplayAuditReport {
    pub schema_version: String,
    pub benchmark: String,
    pub profile_id: String,
    pub metric_profile_id: String,
    pub campaign_id: String,
    pub split_manifest_sha256: String,
    pub candidate_n: usize,
    pub valid_n: usize,
    pub invalid_n: usize,
    pub unresolved_n: usize,
    pub candidate_audits: Vec<CandidateAudit>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq)]
struct RgbImage {
    width: u32,
    height: u32,
    pixels: Vec<[u8; 3]>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn safe_artifact_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let candidate = Path::new(relative);
    if candidate.is_absolute()
        || candidate.components().any(|component| {
            !matches!(component, Component::Normal(_))
        })
    {
        return Err("ARTIFACT_PATH_NOT_RELATIVE_OR_TRAVERSAL".to_owned());
    }
    let root = root
        .canonicalize()
        .map_err(|error| format!("ARTIFACT_ROOT_UNAVAILABLE: {error}"))?;
    let path = root.join(candidate);
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("ARTIFACT_UNAVAILABLE: {relative}: {error}"))?;
    if !canonical.starts_with(&root) {
        return Err("ARTIFACT_PATH_ESCAPES_ROOT".to_owned());
    }
    Ok(canonical)
}

fn decode_png(bytes: &[u8]) -> Result<RgbImage, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|error| format!("PNG_HEADER_INVALID: {error}"))?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|error| format!("PNG_FRAME_INVALID: {error}"))?;
    if info.bit_depth != png::BitDepth::Eight {
        return Err("PNG_OUTPUT_NOT_8_BIT".to_owned());
    }
    let bytes = &buffer[..info.buffer_size()];
    let pixel_count = info.width as usize * info.height as usize;
    let mut pixels = Vec::with_capacity(pixel_count);
    match info.color_type {
        png::ColorType::Rgb => {
            if bytes.len() != pixel_count * 3 {
                return Err("PNG_RGB_BUFFER_LENGTH_MISMATCH".to_owned());
            }
            for pixel in bytes.chunks_exact(3) {
                pixels.push([pixel[0], pixel[1], pixel[2]]);
            }
        }
        png::ColorType::Rgba => {
            if bytes.len() != pixel_count * 4 {
                return Err("PNG_RGBA_BUFFER_LENGTH_MISMATCH".to_owned());
            }
            for pixel in bytes.chunks_exact(4) {
                let alpha = u16::from(pixel[3]);
                let inverse = 255_u16 - alpha;
                let composite = |channel: u8| {
                    ((u16::from(channel) * alpha + 255 * inverse + 127) / 255) as u8
                };
                pixels.push([composite(pixel[0]), composite(pixel[1]), composite(pixel[2])]);
            }
        }
        png::ColorType::Grayscale => {
            if bytes.len() != pixel_count {
                return Err("PNG_GRAYSCALE_BUFFER_LENGTH_MISMATCH".to_owned());
            }
            for value in bytes {
                pixels.push([*value, *value, *value]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            if bytes.len() != pixel_count * 2 {
                return Err("PNG_GRAYSCALE_ALPHA_BUFFER_LENGTH_MISMATCH".to_owned());
            }
            for pixel in bytes.chunks_exact(2) {
                let alpha = u16::from(pixel[1]);
                let value = ((u16::from(pixel[0]) * alpha + 255 * (255 - alpha) + 127) / 255) as u8;
                pixels.push([value, value, value]);
            }
        }
        png::ColorType::Indexed => return Err("PNG_PALETTE_NOT_EXPANDED".to_owned()),
    }
    Ok(RgbImage {
        width: info.width,
        height: info.height,
        pixels,
    })
}

fn gaussian_kernel() -> [[f64; SSIM_WINDOW]; SSIM_WINDOW] {
    let mut kernel = [[0.0; SSIM_WINDOW]; SSIM_WINDOW];
    let radius = (SSIM_WINDOW / 2) as isize;
    let mut sum = 0.0;
    for (y, row) in kernel.iter_mut().enumerate() {
        for (x, weight) in row.iter_mut().enumerate() {
            let dx = x as isize - radius;
            let dy = y as isize - radius;
            let value = (-((dx * dx + dy * dy) as f64) / (2.0 * SSIM_SIGMA * SSIM_SIGMA)).exp();
            *weight = value;
            sum += value;
        }
    }
    for row in &mut kernel {
        for weight in row {
            *weight /= sum;
        }
    }
    kernel
}

fn compute_ssim(original: &RgbImage, replay: &RgbImage) -> Result<f64, String> {
    if original.width != replay.width || original.height != replay.height {
        return Err("FRAME_DIMENSIONS_MISMATCH".to_owned());
    }
    if original.width < SSIM_WINDOW as u32 || original.height < SSIM_WINDOW as u32 {
        return Err("FRAME_TOO_SMALL_FOR_SSIM_WINDOW".to_owned());
    }
    if original.pixels.len() != replay.pixels.len() {
        return Err("FRAME_PIXEL_COUNT_MISMATCH".to_owned());
    }

    let kernel = gaussian_kernel();
    let radius = (SSIM_WINDOW / 2) as usize;
    let c1 = (SSIM_K1 * SSIM_L).powi(2);
    let c2 = (SSIM_K2 * SSIM_L).powi(2);
    let width = original.width as usize;
    let height = original.height as usize;
    let mut total = 0.0;
    let mut count = 0_u64;

    for y in radius..(height - radius) {
        for x in radius..(width - radius) {
            let mut mean_x = [0.0; 3];
            let mut mean_y = [0.0; 3];
            let mut ex2 = [0.0; 3];
            let mut ey2 = [0.0; 3];
            let mut exy = [0.0; 3];
            for ky in 0..SSIM_WINDOW {
                for kx in 0..SSIM_WINDOW {
                    let weight = kernel[ky][kx];
                    let px = x + kx - radius;
                    let py = y + ky - radius;
                    let index = py * width + px;
                    for channel in 0..3 {
                        let a = f64::from(original.pixels[index][channel]);
                        let b = f64::from(replay.pixels[index][channel]);
                        mean_x[channel] += weight * a;
                        mean_y[channel] += weight * b;
                        ex2[channel] += weight * a * a;
                        ey2[channel] += weight * b * b;
                        exy[channel] += weight * a * b;
                    }
                }
            }
            for channel in 0..3 {
                let variance_x = (ex2[channel] - mean_x[channel] * mean_x[channel]).max(0.0);
                let variance_y = (ey2[channel] - mean_y[channel] * mean_y[channel]).max(0.0);
                let covariance = exy[channel] - mean_x[channel] * mean_y[channel];
                let numerator = (2.0 * mean_x[channel] * mean_y[channel] + c1)
                    * (2.0 * covariance + c2);
                let denominator = (mean_x[channel].powi(2) + mean_y[channel].powi(2) + c1)
                    * (variance_x + variance_y + c2);
                let ssim = numerator / denominator;
                if !ssim.is_finite() {
                    return Err("SSIM_NONFINITE".to_owned());
                }
                total += ssim;
                count += 1;
            }
        }
    }
    if count == 0 {
        return Err("SSIM_NO_VALID_WINDOWS".to_owned());
    }
    Ok(total / count as f64)
}

fn mean_final_five_ssim(scores: &[Option<f64>]) -> Option<f64> {
    if scores.len() != FRAME_PAIR_N {
        return None;
    }
    let values: Option<Vec<f64>> = scores.iter().copied().collect();
    values.map(|values| values.iter().sum::<f64>() / FRAME_PAIR_N as f64)
}

fn pose_error_mm(original: &PositionMm, replayed: &PositionMm) -> Result<f64, String> {
    if original.coordinate_frame_id.trim().is_empty()
        || replayed.coordinate_frame_id.trim().is_empty()
        || original.coordinate_frame_id != replayed.coordinate_frame_id
    {
        return Err("POSE_COORDINATE_FRAME_UNRESOLVED".to_owned());
    }
    let values = [
        original.x_mm,
        original.y_mm,
        original.z_mm,
        replayed.x_mm,
        replayed.y_mm,
        replayed.z_mm,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err("POSE_NONFINITE".to_owned());
    }
    let dx = original.x_mm - replayed.x_mm;
    let dy = original.y_mm - replayed.y_mm;
    let dz = original.z_mm - replayed.z_mm;
    Ok(dx.hypot(dy).hypot(dz))
}

fn validate_difficulty_screen(screen: &DifficultyScreen) -> Result<usize, Vec<String>> {
    let mut reasons = Vec::new();
    if screen.trials.len() != SCREENING_TRIAL_N {
        reasons.push("DIFFICULTY_TRIAL_COUNT_NOT_20".to_owned());
    }
    let attempt_ids: BTreeSet<&str> = screen
        .trials
        .iter()
        .map(|trial| trial.attempt_id.as_str())
        .collect();
    if screen.trials.iter().any(|trial| trial.attempt_id.trim().is_empty())
        || attempt_ids.len() != screen.trials.len()
    {
        reasons.push("DIFFICULTY_ATTEMPT_IDENTITIES_INVALID".to_owned());
    }
    let policies: BTreeSet<&str> = screen
        .trials
        .iter()
        .map(|trial| trial.policy_id.as_str())
        .collect();
    if policies.len() != 4 {
        reasons.push("DIFFICULTY_POLICY_CARDINALITY_NOT_4".to_owned());
    }
    let mut policy_trial_counts: BTreeMap<&str, BTreeSet<u8>> = BTreeMap::new();
    for trial in &screen.trials {
        policy_trial_counts
            .entry(trial.policy_id.as_str())
            .or_default()
            .insert(trial.trial_index);
    }
    let expected_trial_indices: BTreeSet<u8> = (0..5).collect();
    if policy_trial_counts
        .values()
        .any(|indices| *indices != expected_trial_indices)
    {
        reasons.push("DIFFICULTY_POLICY_TRIAL_COUNT_NOT_5".to_owned());
    }
    let successes = screen.trials.iter().filter(|trial| trial.success).count();
    if !(SCREENING_SUCCESS_MIN..=SCREENING_SUCCESS_MAX).contains(&successes) {
        reasons.push("DIFFICULTY_SUCCESS_COUNT_OUT_OF_RANGE".to_owned());
    }
    if reasons.is_empty() {
        Ok(successes)
    } else {
        Err(reasons)
    }
}

fn spot_check_manifest_digest(spot: &SpotCheckManifest) -> Result<String, String> {
    let material = SpotCheckManifestMaterial {
        seed: spot.seed,
        selection_method_id: &spot.selection_method_id,
        selection_record_path: &spot.selection_record_path,
        selection_record_sha256: &spot.selection_record_sha256,
        selected_attempt_ids: &spot.selected_attempt_ids,
        attempts: &spot.attempts,
    };
    let bytes = serde_json::to_vec(&material)
        .map_err(|error| format!("SPOT_CHECK_MANIFEST_SERIALIZATION_FAILED: {error}"))?;
    Ok(sha256_hex(&bytes))
}

fn validate_spot_check(
    spot: &SpotCheckManifest,
    screen: &DifficultyScreen,
    artifact_root: &Path,
) -> Result<(), (AuditStatus, String)> {
    if spot.seed != SPOT_CHECK_SEED {
        return Err((AuditStatus::Invalid, "SPOT_CHECK_SEED_MISMATCH".to_owned()));
    }
    if spot.selection_method_id.trim().is_empty() {
        return Err((AuditStatus::Unresolved, "SPOT_CHECK_METHOD_ID_MISSING".to_owned()));
    }
    if spot.selected_attempt_ids.len() != SPOT_CHECK_N
        || spot.attempts.len() != SPOT_CHECK_N
        || spot.selected_attempt_ids.iter().any(String::is_empty)
        || spot.selected_attempt_ids.iter().collect::<BTreeSet<_>>().len() != SPOT_CHECK_N
    {
        return Err((AuditStatus::Invalid, "SPOT_CHECK_SELECTION_INVALID".to_owned()));
    }
    if spot
        .attempts
        .iter()
        .map(|attempt| attempt.attempt_id.as_str())
        .collect::<Vec<_>>()
        != spot
            .selected_attempt_ids
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    {
        return Err((AuditStatus::Invalid, "SPOT_CHECK_ATTEMPT_SELECTION_MISMATCH".to_owned()));
    }
    let available_attempt_ids: BTreeSet<&str> = screen
        .trials
        .iter()
        .map(|trial| trial.attempt_id.as_str())
        .collect();
    if spot
        .selected_attempt_ids
        .iter()
        .any(|attempt_id| !available_attempt_ids.contains(attempt_id.as_str()))
    {
        return Err((AuditStatus::Invalid, "SPOT_CHECK_ATTEMPT_NOT_IN_SCREENING_TRIALS".to_owned()));
    }
    if !is_sha256(&spot.selection_record_sha256) || !is_sha256(&spot.manifest_sha256) {
        return Err((AuditStatus::Invalid, "SPOT_CHECK_MANIFEST_DIGEST_INVALID".to_owned()));
    }

    let selection_path = safe_artifact_path(artifact_root, &spot.selection_record_path)
        .map_err(|reason| (AuditStatus::Unresolved, reason))?;
    let selection_bytes = fs::read(selection_path)
        .map_err(|error| (AuditStatus::Unresolved, format!("SPOT_CHECK_SELECTION_READ_FAILED: {error}")))?;
    if sha256_hex(&selection_bytes) != spot.selection_record_sha256 {
        return Err((AuditStatus::Invalid, "SPOT_CHECK_SELECTION_DIGEST_MISMATCH".to_owned()));
    }
    let selection_record: SpotCheckSelectionRecord = serde_json::from_slice(&selection_bytes)
        .map_err(|error| (AuditStatus::Invalid, format!("SPOT_CHECK_SELECTION_RECORD_INVALID: {error}")))?;
    if selection_record.schema_version != SCHEMA_VERSION
        || selection_record.seed != spot.seed
        || selection_record.selection_method_id != spot.selection_method_id
        || selection_record.selected_attempt_ids != spot.selected_attempt_ids
    {
        return Err((AuditStatus::Invalid, "SPOT_CHECK_SELECTION_RECORD_MISMATCH".to_owned()));
    }

    let expected_manifest_digest = spot_check_manifest_digest(spot)
        .map_err(|error| (AuditStatus::Invalid, error))?;
    if expected_manifest_digest != spot.manifest_sha256 {
        return Err((AuditStatus::Invalid, "SPOT_CHECK_MANIFEST_DIGEST_MISMATCH".to_owned()));
    }
    Ok(())
}

fn push_status(status: &mut AuditStatus, next: AuditStatus) {
    *status = match (*status, next) {
        (AuditStatus::Invalid, _) | (_, AuditStatus::Invalid) => AuditStatus::Invalid,
        (AuditStatus::Unresolved, _) | (_, AuditStatus::Unresolved) => AuditStatus::Unresolved,
        _ => AuditStatus::Valid,
    };
}

fn audit_pose(comparison: &PoseComparison, label: &str) -> (AuditStatus, PoseCheck, Vec<String>) {
    let mut check = PoseCheck {
        observed_error_mm: None,
        threshold_mm: POSE_THRESHOLD_MM,
        passed: None,
    };
    let mut reasons = Vec::new();
    match pose_error_mm(&comparison.original, &comparison.replayed) {
        Ok(error_mm) => {
            let passed = error_mm < POSE_THRESHOLD_MM;
            check.observed_error_mm = Some(error_mm);
            check.passed = Some(passed);
            if !passed {
                reasons.push(format!("{label}_POSE_ERROR_NOT_BELOW_2MM"));
                (AuditStatus::Invalid, check, reasons)
            } else {
                (AuditStatus::Valid, check, reasons)
            }
        }
        Err(reason) => {
            reasons.push(reason);
            (AuditStatus::Unresolved, check, reasons)
        }
    }
}

fn audit_frame_sequence(
    frame_pairs: &[FramePairAsset],
    artifact_root: &Path,
) -> (AuditStatus, Vec<FrameCheck>, Option<f64>, Option<bool>, Vec<String>) {
    let mut status = AuditStatus::Valid;
    let mut reasons = Vec::new();
    let mut frame_checks = Vec::with_capacity(frame_pairs.len());
    if frame_pairs.len() != FRAME_PAIR_N {
        reasons.push("FINAL_FRAME_PAIR_COUNT_NOT_5".to_owned());
        push_status(&mut status, AuditStatus::Invalid);
    }
    let mut seen_frame_indices = BTreeSet::new();
    for (expected_index, frame) in frame_pairs.iter().enumerate() {
        let mut check = FrameCheck {
            frame_index: frame.frame_index,
            original_sha256: frame.original_sha256.clone(),
            replay_sha256: frame.replay_sha256.clone(),
            original_width: None,
            original_height: None,
            replay_width: None,
            replay_height: None,
            ssim: None,
        };
        if frame.frame_index as usize != expected_index {
            reasons.push("FRAME_INDEX_ORDER_INVALID".to_owned());
            push_status(&mut status, AuditStatus::Invalid);
        }
        if !seen_frame_indices.insert(frame.frame_index) {
            reasons.push("FRAME_INDEX_DUPLICATED".to_owned());
            push_status(&mut status, AuditStatus::Invalid);
            frame_checks.push(check);
            continue;
        }
        if frame.frame_index as usize >= FRAME_PAIR_N {
            reasons.push("FRAME_INDEX_OUT_OF_RANGE".to_owned());
            push_status(&mut status, AuditStatus::Invalid);
            frame_checks.push(check);
            continue;
        }
        if !is_sha256(&frame.original_sha256) || !is_sha256(&frame.replay_sha256) {
            reasons.push("FRAME_DIGEST_INVALID".to_owned());
            push_status(&mut status, AuditStatus::Invalid);
            frame_checks.push(check);
            continue;
        }
        let original_path = match safe_artifact_path(artifact_root, &frame.original_path) {
            Ok(path) => path,
            Err(reason) => {
                reasons.push(reason);
                push_status(&mut status, AuditStatus::Unresolved);
                frame_checks.push(check);
                continue;
            }
        };
        let replay_path = match safe_artifact_path(artifact_root, &frame.replay_path) {
            Ok(path) => path,
            Err(reason) => {
                reasons.push(reason);
                push_status(&mut status, AuditStatus::Unresolved);
                frame_checks.push(check);
                continue;
            }
        };
        let original_bytes = match fs::read(original_path) {
            Ok(bytes) => bytes,
            Err(error) => {
                reasons.push(format!("ORIGINAL_FRAME_READ_FAILED: {error}"));
                push_status(&mut status, AuditStatus::Unresolved);
                frame_checks.push(check);
                continue;
            }
        };
        let replay_bytes = match fs::read(replay_path) {
            Ok(bytes) => bytes,
            Err(error) => {
                reasons.push(format!("REPLAY_FRAME_READ_FAILED: {error}"));
                push_status(&mut status, AuditStatus::Unresolved);
                frame_checks.push(check);
                continue;
            }
        };
        if sha256_hex(&original_bytes) != frame.original_sha256
            || sha256_hex(&replay_bytes) != frame.replay_sha256
        {
            reasons.push("FRAME_DIGEST_MISMATCH".to_owned());
            push_status(&mut status, AuditStatus::Invalid);
            frame_checks.push(check);
            continue;
        }
        let original_image = match decode_png(&original_bytes) {
            Ok(image) => image,
            Err(reason) => {
                reasons.push(format!("ORIGINAL_{reason}"));
                push_status(&mut status, AuditStatus::Invalid);
                frame_checks.push(check);
                continue;
            }
        };
        let replay_image = match decode_png(&replay_bytes) {
            Ok(image) => image,
            Err(reason) => {
                reasons.push(format!("REPLAY_{reason}"));
                push_status(&mut status, AuditStatus::Invalid);
                frame_checks.push(check);
                continue;
            }
        };
        check.original_width = Some(original_image.width);
        check.original_height = Some(original_image.height);
        check.replay_width = Some(replay_image.width);
        check.replay_height = Some(replay_image.height);
        match compute_ssim(&original_image, &replay_image) {
            Ok(ssim) => check.ssim = Some(ssim),
            Err(reason) => {
                reasons.push(reason);
                push_status(&mut status, AuditStatus::Invalid);
            }
        }
        frame_checks.push(check);
    }
    if (0..FRAME_PAIR_N as u8).any(|index| !seen_frame_indices.contains(&index)) {
        reasons.push("FRAME_INDEX_SEQUENCE_INCOMPLETE".to_owned());
        push_status(&mut status, AuditStatus::Invalid);
    }
    frame_checks.sort_by_key(|check| check.frame_index);

    let scores: Vec<Option<f64>> = frame_checks.iter().map(|check| check.ssim).collect();
    let mean_ssim = mean_final_five_ssim(&scores);
    let mean_ssim_passed = mean_ssim.map(|mean| mean > SSIM_THRESHOLD);
    if mean_ssim_passed == Some(false) {
        reasons.push("MEAN_FINAL_FIVE_FRAME_SSIM_NOT_ABOVE_0_99".to_owned());
        push_status(&mut status, AuditStatus::Invalid);
    } else if mean_ssim_passed.is_none() {
        push_status(&mut status, AuditStatus::Unresolved);
    }

    (status, frame_checks, mean_ssim, mean_ssim_passed, reasons)
}

fn audit_spot_check_attempt(attempt: &SpotCheckAttempt, artifact_root: &Path) -> SpotCheckAudit {
    let (ee_status, end_effector, mut reason_codes) =
        audit_pose(&attempt.end_effector, "END_EFFECTOR");
    let (object_status, object_pose, object_reasons) =
        audit_pose(&attempt.object_pose, "OBJECT");
    reason_codes.extend(object_reasons);

    let (frame_status, frame_checks, mean_final_five_ssim, mean_ssim_passed, frame_reasons) =
        audit_frame_sequence(&attempt.final_frame_pairs, artifact_root);
    reason_codes.extend(frame_reasons);

    let mut status = AuditStatus::Valid;
    push_status(&mut status, ee_status);
    push_status(&mut status, object_status);
    push_status(&mut status, frame_status);

    SpotCheckAudit {
        attempt_id: attempt.attempt_id.clone(),
        status,
        end_effector,
        object_pose,
        frame_checks,
        mean_final_five_ssim,
        mean_ssim_threshold: SSIM_THRESHOLD,
        mean_ssim_passed,
        reason_codes,
    }
}

fn audit_candidate(candidate: &ReplayCandidate, artifact_root: &Path) -> CandidateAudit {
    let mut status = AuditStatus::Valid;
    let mut reason_codes = Vec::new();

    let successes = match validate_difficulty_screen(&candidate.difficulty_screen) {
        Ok(successes) => successes,
        Err(reasons) => {
            reason_codes.extend(reasons);
            push_status(&mut status, AuditStatus::Invalid);
            candidate.difficulty_screen.trials.iter().filter(|trial| trial.success).count()
        }
    };

    if let Err((selection_status, reason)) = validate_spot_check(
        &candidate.spot_check,
        &candidate.difficulty_screen,
        artifact_root,
    ) {
        reason_codes.push(reason);
        push_status(&mut status, selection_status);
    }

    let spot_checks: Vec<SpotCheckAudit> = candidate
        .spot_check
        .attempts
        .iter()
        .map(|attempt| audit_spot_check_attempt(attempt, artifact_root))
        .collect();
    for attempt_audit in &spot_checks {
        push_status(&mut status, attempt_audit.status);
    }

    CandidateAudit {
        scenario_id: candidate.scenario_id.clone(),
        platform: candidate.platform,
        split: candidate.split,
        source_trajectory_id: candidate.source_trajectory_id.clone(),
        trajectory_cluster_id: candidate.trajectory_cluster_id.clone(),
        status,
        difficulty_success_n: successes,
        difficulty_trial_n: candidate.difficulty_screen.trials.len(),
        selection_method_id: candidate.spot_check.selection_method_id.clone(),
        spot_check_attempt_ids: candidate.spot_check.selected_attempt_ids.clone(),
        spot_checks,
        reason_codes,
    }
}

fn add_split_leakage_reasons(
    candidates: &[ReplayCandidate],
    audits: &mut [CandidateAudit],
) {
    let mut train_keys: BTreeMap<Platform, BTreeSet<&str>> = BTreeMap::new();
    let mut test_keys: BTreeMap<Platform, BTreeSet<&str>> = BTreeMap::new();
    for candidate in candidates {
        let destination = match candidate.split {
            Split::Train => &mut train_keys,
            Split::Test => &mut test_keys,
        };
        let keys = destination.entry(candidate.platform).or_default();
        for key in &candidate.split_leakage_keys_sha256 {
            keys.insert(key.as_str());
        }
    }
    for (index, candidate) in candidates.iter().enumerate() {
        let opposite = match candidate.split {
            Split::Train => test_keys.get(&candidate.platform),
            Split::Test => train_keys.get(&candidate.platform),
        };
        let Some(opposite) = opposite else {
            continue;
        };
        if candidate
            .split_leakage_keys_sha256
            .iter()
            .any(|key| opposite.contains(key.as_str()))
        {
            audits[index]
                .reason_codes
                .push("TRAIN_TEST_LEAKAGE_KEY_OVERLAP".to_owned());
            push_status(&mut audits[index].status, AuditStatus::Invalid);
        }
    }
}

pub fn audit_manifest(
    manifest: &ReplayManifest,
    artifact_root: &Path,
) -> Result<ReplayAuditReport, String> {
    if manifest.schema_version != SCHEMA_VERSION {
        return Err("MANIFEST_SCHEMA_VERSION_UNSUPPORTED".to_owned());
    }
    if manifest.benchmark != BENCHMARK {
        return Err("BENCHMARK_ID_MISMATCH".to_owned());
    }
    if manifest.profile_id != PROFILE_ID {
        return Err("REPLAY_THRESHOLD_PROFILE_UNSUPPORTED".to_owned());
    }
    if manifest.metric_profile_id != METRIC_PROFILE_ID {
        return Err("REPLAY_METRIC_PROFILE_UNSUPPORTED".to_owned());
    }
    if manifest.campaign_id.trim().is_empty() || manifest.data_license_and_provenance.trim().is_empty() {
        return Err("CAMPAIGN_OR_DATA_PROVENANCE_MISSING".to_owned());
    }
    if !is_sha256(&manifest.split_manifest_sha256) {
        return Err("SPLIT_MANIFEST_DIGEST_INVALID".to_owned());
    }
    if manifest.candidates.is_empty() {
        return Err("NO_REPLAY_CANDIDATES".to_owned());
    }
    let mut scenario_ids = BTreeSet::new();
    for candidate in &manifest.candidates {
        if candidate.scenario_id.trim().is_empty()
            || candidate.source_trajectory_id.trim().is_empty()
            || candidate.trajectory_cluster_id.trim().is_empty()
        {
            return Err("CANDIDATE_IDENTITY_INCOMPLETE".to_owned());
        }
        if !scenario_ids.insert((candidate.platform, candidate.scenario_id.as_str())) {
            return Err(format!("DUPLICATE_SCENARIO_ID: {}", candidate.scenario_id));
        }
        let digests = [
            candidate.source_episode_sha256.as_str(),
            candidate.instruction_sha256.as_str(),
            candidate.action_prefix_sha256.as_str(),
            candidate.simulator_profile_sha256.as_str(),
            candidate.robot_model_profile_sha256.as_str(),
            candidate.controller_profile_sha256.as_str(),
            candidate.rendering_profile_sha256.as_str(),
            candidate.reset_receipt_sha256.as_str(),
            candidate.action_prefix_receipt_sha256.as_str(),
        ];
        if digests.iter().any(|digest| !is_sha256(digest)) {
            return Err(format!("CANDIDATE_DIGEST_INVALID: {}", candidate.scenario_id));
        }
        if candidate.split_leakage_keys_sha256.is_empty()
            || candidate.split_leakage_keys_sha256.iter().any(|key| !is_sha256(key))
        {
            return Err(format!("SPLIT_LEAKAGE_KEYS_INVALID: {}", candidate.scenario_id));
        }
        if candidate
            .split_leakage_keys_sha256
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != candidate.split_leakage_keys_sha256.len()
        {
            return Err(format!("SPLIT_LEAKAGE_KEYS_DUPLICATED: {}", candidate.scenario_id));
        }
    }

    let mut audits: Vec<CandidateAudit> = manifest
        .candidates
        .iter()
        .map(|candidate| audit_candidate(candidate, artifact_root))
        .collect();
    add_split_leakage_reasons(&manifest.candidates, &mut audits);

    let count = |status| audits.iter().filter(|audit| audit.status == status).count();
    Ok(ReplayAuditReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        benchmark: BENCHMARK.to_owned(),
        profile_id: PROFILE_ID.to_owned(),
        metric_profile_id: METRIC_PROFILE_ID.to_owned(),
        campaign_id: manifest.campaign_id.clone(),
        split_manifest_sha256: manifest.split_manifest_sha256.clone(),
        candidate_n: audits.len(),
        valid_n: count(AuditStatus::Valid),
        invalid_n: count(AuditStatus::Invalid),
        unresolved_n: count(AuditStatus::Unresolved),
        candidate_audits: audits,
        claim_ceiling: "Offline replay-fidelity audit only; no robot safety, physical-effect, HIL, or field result.".to_owned(),
    })
}

pub fn audit_json(input: &str, artifact_root: &Path) -> Result<ReplayAuditReport, String> {
    let manifest: ReplayManifest =
        serde_json::from_str(input).map_err(|error| format!("MANIFEST_JSON_INVALID: {error}"))?;
    audit_manifest(&manifest, artifact_root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

    fn temp_dir() -> PathBuf {
        let suffix = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "rrvr-replay-audit-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("test directory");
        path
    }

    fn write_png(path: &Path, width: u32, height: u32, pixels: &[[u8; 3]]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("PNG parent directory");
        }
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, width, height);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("PNG header");
            let flattened: Vec<u8> = pixels.iter().flat_map(|pixel| pixel.iter().copied()).collect();
            writer.write_image_data(&flattened).expect("PNG data");
        }
        fs::write(path, bytes).expect("PNG artifact");
    }

    fn frame_asset(
        root: &Path,
        attempt_id: &str,
        index: u8,
        original: &[[u8; 3]],
        replay: &[[u8; 3]],
    ) -> FramePairAsset {
        let original_path = format!("frames/{attempt_id}/original-{index}.png");
        let replay_path = format!("frames/{attempt_id}/replay-{index}.png");
        write_png(&root.join(&original_path), 16, 16, original);
        write_png(&root.join(&replay_path), 16, 16, replay);
        let original_bytes = fs::read(root.join(&original_path)).expect("original read");
        let replay_bytes = fs::read(root.join(&replay_path)).expect("replay read");
        FramePairAsset {
            frame_index: index,
            original_path,
            original_sha256: sha256_hex(&original_bytes),
            replay_path,
            replay_sha256: sha256_hex(&replay_bytes),
        }
    }

    fn difficulty_screen(scenario_id: &str) -> DifficultyScreen {
        DifficultyScreen {
            trials: (0..4)
                .flat_map(|policy| {
                    (0..5).map(move |trial| DifficultyTrial {
                        attempt_id: format!("{scenario_id}-screen-{policy}-{trial}"),
                        policy_id: format!("policy-{policy}"),
                        trial_index: trial,
                        success: policy == 0 && trial < 2,
                    })
                })
                .collect(),
        }
    }

    fn pose_comparison(offset_mm: f64) -> PoseComparison {
        PoseComparison {
            original: PositionMm {
                coordinate_frame_id: "world".to_owned(),
                x_mm: 100.0,
                y_mm: 200.0,
                z_mm: 300.0,
            },
            replayed: PositionMm {
                coordinate_frame_id: "world".to_owned(),
                x_mm: 100.0 + offset_mm,
                y_mm: 200.0,
                z_mm: 300.0,
            },
        }
    }

    fn write_selection_record(
        root: &Path,
        scenario_id: &str,
        seed: u64,
        method_id: &str,
        attempt_ids: &[String],
    ) -> (String, String) {
        let record = SpotCheckSelectionRecord {
            schema_version: SCHEMA_VERSION.to_owned(),
            seed,
            selection_method_id: method_id.to_owned(),
            selected_attempt_ids: attempt_ids.to_vec(),
        };
        let bytes = serde_json::to_vec_pretty(&record).expect("selection record serialization");
        let relative_path = format!("metadata/{scenario_id}-selection.json");
        let full_path = root.join(&relative_path);
        fs::create_dir_all(full_path.parent().expect("metadata parent"))
            .expect("metadata directory");
        fs::write(&full_path, &bytes).expect("selection record write");
        (relative_path, sha256_hex(&bytes))
    }

    fn spot_check_manifest(
        root: &Path,
        scenario_id: &str,
        screen: &DifficultyScreen,
    ) -> SpotCheckManifest {
        let pixels = vec![[100, 120, 140]; 16 * 16];
        let selected_attempt_ids = vec![
            screen.trials[0].attempt_id.clone(),
            screen.trials[6].attempt_id.clone(),
            screen.trials[12].attempt_id.clone(),
        ];
        let attempts: Vec<SpotCheckAttempt> = selected_attempt_ids
            .iter()
            .map(|attempt_id| SpotCheckAttempt {
                attempt_id: attempt_id.clone(),
                end_effector: pose_comparison(1.0),
                object_pose: pose_comparison(1.0),
                final_frame_pairs: (0..FRAME_PAIR_N as u8)
                    .map(|index| frame_asset(root, attempt_id, index, &pixels, &pixels))
                    .collect(),
            })
            .collect();
        let selection_method_id = "upstream-exported-seed42-selection-v1".to_owned();
        let (selection_record_path, selection_record_sha256) = write_selection_record(
            root,
            scenario_id,
            SPOT_CHECK_SEED,
            &selection_method_id,
            &selected_attempt_ids,
        );
        let mut spot = SpotCheckManifest {
            seed: SPOT_CHECK_SEED,
            selection_method_id,
            selection_record_path,
            selection_record_sha256,
            selected_attempt_ids,
            attempts,
            manifest_sha256: String::new(),
        };
        spot.manifest_sha256 = spot_check_manifest_digest(&spot).expect("spot manifest digest");
        spot
    }

    fn passing_candidate(
        root: &Path,
        scenario_id: &str,
        split: Split,
        leak_key: &str,
    ) -> ReplayCandidate {
        let screen = difficulty_screen(scenario_id);
        let spot = spot_check_manifest(root, scenario_id, &screen);
        ReplayCandidate {
            scenario_id: scenario_id.to_owned(),
            platform: Platform::RoboTwin,
            split,
            source_episode_sha256: "a".repeat(64),
            source_trajectory_id: format!("trajectory-{scenario_id}"),
            trajectory_cluster_id: "cluster-test-may-repeat".to_owned(),
            instruction_sha256: "b".repeat(64),
            action_prefix_sha256: "c".repeat(64),
            split_leakage_keys_sha256: vec![leak_key.to_owned()],
            simulator_profile_sha256: "d".repeat(64),
            robot_model_profile_sha256: "e".repeat(64),
            controller_profile_sha256: "f".repeat(64),
            rendering_profile_sha256: "1".repeat(64),
            reset_receipt_sha256: "2".repeat(64),
            action_prefix_receipt_sha256: "3".repeat(64),
            difficulty_screen: screen,
            spot_check: spot,
        }
    }

    fn manifest(candidates: Vec<ReplayCandidate>) -> ReplayManifest {
        ReplayManifest {
            schema_version: SCHEMA_VERSION.to_owned(),
            benchmark: BENCHMARK.to_owned(),
            profile_id: PROFILE_ID.to_owned(),
            metric_profile_id: METRIC_PROFILE_ID.to_owned(),
            campaign_id: "test-campaign".to_owned(),
            split_manifest_sha256: "5".repeat(64),
            data_license_and_provenance: "Synthetic test fixture; no dataset redistribution.".to_owned(),
            candidates,
        }
    }

    #[test]
    fn all_three_seeded_spot_check_attempts_must_pass() {
        let root = temp_dir();
        let candidate = passing_candidate(&root, "scenario-1", Split::Test, &"a".repeat(64));
        let report = audit_manifest(&manifest(vec![candidate]), &root).expect("audit");
        assert_eq!(report.valid_n, 1);
        assert_eq!(report.invalid_n, 0);
        assert_eq!(report.candidate_audits[0].spot_checks.len(), SPOT_CHECK_N);
        assert!(
            report.candidate_audits[0]
                .spot_checks
                .iter()
                .all(|attempt| attempt.status == AuditStatus::Valid
                    && attempt.frame_checks.len() == FRAME_PAIR_N
                    && attempt.mean_final_five_ssim == Some(1.0))
        );
    }

    #[test]
    fn one_failed_attempt_invalidates_the_scenario() {
        let root = temp_dir();
        let mut candidate = passing_candidate(&root, "scenario-2", Split::Test, &"a".repeat(64));
        candidate.spot_check.attempts[1].end_effector = pose_comparison(2.0);
        let report = audit_manifest(&manifest(vec![candidate]), &root).expect("audit");
        assert_eq!(report.invalid_n, 1);
        assert_eq!(
            report.candidate_audits[0].spot_checks[1].status,
            AuditStatus::Invalid
        );
        assert!(
            report.candidate_audits[0].spot_checks[1]
                .reason_codes
                .contains(&"END_EFFECTOR_POSE_ERROR_NOT_BELOW_2MM".to_owned())
        );
    }

    #[test]
    fn five_frame_ssim_uses_published_mean_per_attempt_not_a_per_frame_minimum() {
        let passing_mean = mean_final_five_ssim(&[
            Some(0.98),
            Some(1.0),
            Some(1.0),
            Some(1.0),
            Some(1.0),
        ])
        .expect("five frame scores");
        assert!((passing_mean - 0.996).abs() < 1e-12);
        assert!(passing_mean > SSIM_THRESHOLD);

        let failing_mean =
            mean_final_five_ssim(&[Some(0.94), Some(1.0), Some(1.0), Some(1.0), Some(1.0)])
                .expect("five frame scores");
        assert!(failing_mean <= SSIM_THRESHOLD);
        let exactly_at_threshold =
            mean_final_five_ssim(&[Some(0.99); FRAME_PAIR_N]).expect("five exact threshold scores");
        assert_eq!(exactly_at_threshold, SSIM_THRESHOLD);
        assert!(exactly_at_threshold <= SSIM_THRESHOLD);
        assert!(mean_final_five_ssim(&[Some(1.0); 4]).is_none());
        assert!(
            mean_final_five_ssim(&[Some(1.0), None, Some(1.0), Some(1.0), Some(1.0)]).is_none()
        );
    }

    #[test]
    fn exactly_two_millimetres_is_not_below_strict_threshold() {
        let root = temp_dir();
        let mut candidate = passing_candidate(&root, "scenario-3", Split::Test, &"a".repeat(64));
        candidate.spot_check.attempts[0].object_pose = pose_comparison(2.0);
        let report = audit_manifest(&manifest(vec![candidate]), &root).expect("audit");
        assert_eq!(report.invalid_n, 1);
        assert_eq!(
            report.candidate_audits[0].spot_checks[0].status,
            AuditStatus::Invalid
        );
    }

    #[test]
    fn changed_black_to_white_frames_fail_one_attempt_mean_ssim() {
        let root = temp_dir();
        let black = vec![[0, 0, 0]; 16 * 16];
        let white = vec![[255, 255, 255]; 16 * 16];
        let mut candidate = passing_candidate(&root, "scenario-4", Split::Test, &"a".repeat(64));
        let attempt_id = candidate.spot_check.attempts[2].attempt_id.clone();
        candidate.spot_check.attempts[2].final_frame_pairs = (0..FRAME_PAIR_N as u8)
            .map(|index| frame_asset(&root, &attempt_id, index, &black, &white))
            .collect();
        let report = audit_manifest(&manifest(vec![candidate]), &root).expect("audit");
        assert_eq!(report.invalid_n, 1);
        assert_eq!(
            report.candidate_audits[0].spot_checks[2].status,
            AuditStatus::Invalid
        );
        assert!(
            report.candidate_audits[0].spot_checks[2]
                .reason_codes
                .contains(&"MEAN_FINAL_FIVE_FRAME_SSIM_NOT_ABOVE_0_99".to_owned())
        );
    }

    #[test]
    fn missing_frame_in_any_attempt_invalidates_the_scenario() {
        let root = temp_dir();
        let mut candidate = passing_candidate(&root, "scenario-5", Split::Test, &"a".repeat(64));
        candidate.spot_check.attempts[1].final_frame_pairs.pop();
        let report = audit_manifest(&manifest(vec![candidate]), &root).expect("audit");
        assert_eq!(report.invalid_n, 1);
        assert!(
            report.candidate_audits[0].spot_checks[1]
                .reason_codes
                .contains(&"FINAL_FRAME_PAIR_COUNT_NOT_5".to_owned())
        );
    }

    #[test]
    fn spot_check_attempts_must_come_from_the_twenty_screening_trials() {
        let root = temp_dir();
        let mut candidate = passing_candidate(&root, "scenario-6", Split::Test, &"a".repeat(64));
        candidate.spot_check.selected_attempt_ids[0] = "not-in-screening".to_owned();
        candidate.spot_check.attempts[0].attempt_id = "not-in-screening".to_owned();
        let report = audit_manifest(&manifest(vec![candidate]), &root).expect("audit");
        assert_eq!(report.invalid_n, 1);
        assert!(
            report.candidate_audits[0]
                .reason_codes
                .contains(&"SPOT_CHECK_ATTEMPT_NOT_IN_SCREENING_TRIALS".to_owned())
        );
    }

    #[test]
    fn missing_one_of_three_spot_checks_invalidates_the_scenario() {
        let root = temp_dir();
        let mut candidate = passing_candidate(&root, "scenario-7", Split::Test, &"a".repeat(64));
        candidate.spot_check.selected_attempt_ids.pop();
        candidate.spot_check.attempts.pop();
        let report = audit_manifest(&manifest(vec![candidate]), &root).expect("audit");
        assert_eq!(report.invalid_n, 1);
        assert!(
            report.candidate_audits[0]
                .reason_codes
                .contains(&"SPOT_CHECK_SELECTION_INVALID".to_owned())
        );
    }

    #[test]
    fn train_test_leakage_is_reported_but_repeated_cluster_inside_test_is_allowed() {
        let root = temp_dir();
        let train = passing_candidate(&root, "train-1", Split::Train, &"a".repeat(64));
        let test = passing_candidate(&root, "test-1", Split::Test, &"a".repeat(64));
        let mut test2 = passing_candidate(&root, "test-2", Split::Test, &"b".repeat(64));
        test2.trajectory_cluster_id = test.trajectory_cluster_id.clone();
        let report = audit_manifest(&manifest(vec![train, test, test2]), &root).expect("audit");
        assert_eq!(report.invalid_n, 2);
        assert_eq!(report.valid_n, 1);
        assert!(
            report.candidate_audits[0]
                .reason_codes
                .contains(&"TRAIN_TEST_LEAKAGE_KEY_OVERLAP".to_owned())
        );
    }

    #[test]
    fn unknown_coordinate_frame_remains_unresolved_for_attempt() {
        let root = temp_dir();
        let mut candidate = passing_candidate(&root, "scenario-8", Split::Test, &"a".repeat(64));
        candidate.spot_check.attempts[0]
            .end_effector
            .replayed
            .coordinate_frame_id = "camera".to_owned();
        let report = audit_manifest(&manifest(vec![candidate]), &root).expect("audit");
        assert_eq!(report.unresolved_n, 1);
        assert!(
            report.candidate_audits[0].spot_checks[0]
                .reason_codes
                .contains(&"POSE_COORDINATE_FRAME_UNRESOLVED".to_owned())
        );
    }

    #[test]
    fn difficulty_screen_requires_twenty_unique_attempts_and_intermediate_success_count() {
        let mut screen = DifficultyScreen {
            trials: (0..20)
                .map(|i| DifficultyTrial {
                    attempt_id: format!("attempt-{i}"),
                    policy_id: format!("policy-{}", i / 5),
                    trial_index: (i % 5) as u8,
                    success: i < 1,
                })
                .collect(),
        };
        let error = validate_difficulty_screen(&screen)
            .expect_err("one success is below the difficulty window");
        assert!(error.contains(&"DIFFICULTY_SUCCESS_COUNT_OUT_OF_RANGE".to_owned()));
        screen.trials[1].success = true;
        assert!(validate_difficulty_screen(&screen).is_ok());
        screen.trials.pop();
        let error = validate_difficulty_screen(&screen).expect_err("19 trials invalid");
        assert!(error.contains(&"DIFFICULTY_TRIAL_COUNT_NOT_20".to_owned()));
        let duplicate_trial = screen.trials[0].clone();
        screen.trials.push(duplicate_trial);
        let error = validate_difficulty_screen(&screen).expect_err("duplicate attempt invalid");
        assert!(error.contains(&"DIFFICULTY_ATTEMPT_IDENTITIES_INVALID".to_owned()));
    }

    #[test]
    fn selection_receipt_tampering_invalidates_spot_check() {
        let root = temp_dir();
        let candidate = passing_candidate(&root, "scenario-9", Split::Test, &"a".repeat(64));
        let selection_path = root.join(&candidate.spot_check.selection_record_path);
        fs::write(selection_path, br#"{"schema_version":"0.1"}"#)
            .expect("overwrite selection record");
        let report = audit_manifest(&manifest(vec![candidate]), &root).expect("audit");
        assert_eq!(report.invalid_n, 1);
        assert!(
            report.candidate_audits[0]
                .reason_codes
                .contains(&"SPOT_CHECK_SELECTION_DIGEST_MISMATCH".to_owned())
        );
    }

    #[test]
    fn artifact_digest_tampering_invalidates_frame_in_any_attempt() {
        let root = temp_dir();
        let mut candidate = passing_candidate(&root, "scenario-10", Split::Test, &"a".repeat(64));
        candidate.spot_check.attempts[1].final_frame_pairs[0].original_sha256 = "0".repeat(64);
        let report = audit_manifest(&manifest(vec![candidate]), &root).expect("audit");
        assert_eq!(report.invalid_n, 1);
        assert!(
            report.candidate_audits[0].spot_checks[1]
                .reason_codes
                .contains(&"FRAME_DIGEST_MISMATCH".to_owned())
        );
    }
}
