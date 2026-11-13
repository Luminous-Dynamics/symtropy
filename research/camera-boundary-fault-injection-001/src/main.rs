//! Fresh-process fault injection against the actual Symthaea camera-driving bridge.
//!
//! Usage:
//!   camera-boundary-fault-injection <symtropy-episode-dir> <camera-driving-bridge> <output-dir>
//!
//! The episode directory is produced by #4618 and contains episode-request.json
//! plus its relative RGB frame artifacts. Each case receives an independent copy
//! of the request and all frame bytes, and is executed in a fresh bridge process.
//! Negative controls must match their exact BridgeError variant and must produce
//! no receipt file and no policy-input artifact.

use serde::Serialize;
use serde_json::{Value, json};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

const REQUEST_FILE: &str = "episode-request.json";
const MAX_REQUEST_BYTES: u64 = 64 * 1024 * 1024;
const MAX_FRAME_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FaultCase {
    CleanControl,
    StaleFrameReuse,
    PixelCorruption,
    CalibrationShift,
    CaptureTimestampMismatch,
    ZeroSequence,
    ReceiptCommitmentTamper,
    StaleSequenceReplay,
    CaptureClockRegression,
    CameraIdentityMismatch,
    ClockOrderViolation,
    SensorClockSkew,
    DeliveryAgeExceeded,
    MissingActionReceipt,
}

impl FaultCase {
    const ALL: [Self; 14] = [
        Self::CleanControl,
        Self::StaleFrameReuse,
        Self::PixelCorruption,
        Self::CalibrationShift,
        Self::CaptureTimestampMismatch,
        Self::ZeroSequence,
        Self::ReceiptCommitmentTamper,
        Self::StaleSequenceReplay,
        Self::CaptureClockRegression,
        Self::CameraIdentityMismatch,
        Self::ClockOrderViolation,
        Self::SensorClockSkew,
        Self::DeliveryAgeExceeded,
        Self::MissingActionReceipt,
    ];

    fn id(self) -> &'static str {
        match self {
            Self::CleanControl => "clean-control",
            Self::StaleFrameReuse => "stale-frame-reuse",
            Self::PixelCorruption => "pixel-corruption",
            Self::CalibrationShift => "calibration-shift",
            Self::CaptureTimestampMismatch => "capture-timestamp-mismatch",
            Self::ZeroSequence => "zero-sequence",
            Self::ReceiptCommitmentTamper => "receipt-commitment-tamper",
            Self::StaleSequenceReplay => "stale-sequence-replay",
            Self::CaptureClockRegression => "capture-clock-regression",
            Self::CameraIdentityMismatch => "camera-identity-mismatch",
            Self::ClockOrderViolation => "clock-order-violation",
            Self::SensorClockSkew => "sensor-clock-skew",
            Self::DeliveryAgeExceeded => "delivery-age-exceeded",
            Self::MissingActionReceipt => "missing-action-receipt",
        }
    }

    fn expected_error(self) -> Option<&'static str> {
        match self {
            Self::CleanControl => None,
            Self::StaleFrameReuse | Self::PixelCorruption => Some("UpstreamFrameMismatch"),
            Self::CalibrationShift => Some("UpstreamCalibrationMismatch"),
            Self::CaptureTimestampMismatch => Some("UpstreamTimestampMismatch"),
            Self::ZeroSequence => Some("UpstreamSequenceMismatch"),
            Self::ReceiptCommitmentTamper => Some("UpstreamReceiptCommitmentMismatch"),
            Self::StaleSequenceReplay => Some("SequenceRegression"),
            Self::CaptureClockRegression => Some("TimestampRegression"),
            Self::CameraIdentityMismatch => Some("UpstreamIdentityMismatch"),
            Self::ClockOrderViolation => Some("UpstreamClockOrderInvalid"),
            Self::SensorClockSkew => Some("SensorClockSkewExceeded"),
            Self::DeliveryAgeExceeded => Some("UpstreamDeliveryAgeExceeded"),
            Self::MissingActionReceipt => Some("UpstreamActionReferenceInvalid"),
        }
    }
}

#[derive(Debug, Clone)]
struct CorruptTarget {
    relative_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct CaseResult {
    case_id: String,
    expected_error: Option<String>,
    actual_exit_code: Option<i32>,
    child_process_spawned: bool,
    accepted: bool,
    expected_error_observed: bool,
    receipt_output_exists: bool,
    policy_input_directory_exists: bool,
    policy_input_artifact_count: usize,
    no_policy_output_committed: bool,
    expected_frames_processed_before_rejection: Option<u64>,
    frames_processed_before_rejection: Option<u64>,
    rejected_observation_did_not_advance_temporal_state: bool,
    input_observation_count: usize,
    stderr: String,
}

#[derive(Debug, Serialize)]
struct FaultCampaignReport {
    schema_version: u32,
    campaign_id: String,
    status: String,
    source_episode_id: String,
    source_episode_report_sha256: String,
    source_episode_request_sha256: String,
    bridge_binary: String,
    bridge_binary_sha256: String,
    camera_calibration_pin_sha256: String,
    maximum_observation_delivery_age_us: u64,
    maximum_sensor_clock_skew_us: u64,
    cases_expected: usize,
    cases_observed: usize,
    fresh_processes_spawned: usize,
    all_negative_controls_rejected_before_policy_output: bool,
    all_rejections_preflighted_before_temporal_advance: bool,
    clean_control_accepted: bool,
    controller_invoked: bool,
    vehicle_actuator_invoked: bool,
    cases: Vec<CaseResult>,
    claim_ceiling: String,
}

fn safe_relative_path(value: &str) -> Result<PathBuf, String> {
    if value.trim().is_empty() {
        return Err("pixels_file must be non-empty".into());
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "pixels_file is not a safe relative path: {value:?}"
        ));
    }
    Ok(path.to_path_buf())
}

fn resolve_inside(base: &Path, relative: &Path) -> Result<PathBuf, String> {
    let canonical_base =
        fs::canonicalize(base).map_err(|error| format!("cannot resolve input base: {error}"))?;
    let candidate = fs::canonicalize(canonical_base.join(relative))
        .map_err(|error| format!("cannot resolve source artifact {relative:?}: {error}"))?;
    if !candidate.starts_with(&canonical_base) {
        return Err(format!(
            "source artifact escapes episode directory: {relative:?}"
        ));
    }
    let metadata = fs::metadata(&candidate)
        .map_err(|error| format!("cannot inspect source artifact: {error}"))?;
    if !metadata.is_file() || metadata.len() > MAX_FRAME_BYTES {
        return Err("source frame must be a regular file no larger than 64 MiB".into());
    }
    Ok(candidate)
}

fn different_digest(value: &str) -> Result<String, String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("camera calibration pin is not lowercase SHA-256".into());
    }
    let first = value.as_bytes()[0];
    let replacement = if first == b'f' { b'e' } else { b'f' };
    let mut result = value.to_string();
    result.replace_range(0..1, &(replacement as char).to_string());
    Ok(result)
}

fn append_len_prefixed_string(material: &mut Vec<u8>, value: &str) {
    material.extend_from_slice(&(value.len() as u64).to_le_bytes());
    material.extend_from_slice(value.as_bytes());
}

/// Recompute the renderer's unkeyed policy receipt commitment after a
/// deliberate metadata mutation. This checks consistency, not authenticity.
fn recompute_upstream_policy_receipt(upstream: &mut Value) -> Result<(), String> {
    let mut material = Vec::new();
    let schema_version = upstream["schema_version"]
        .as_u64()
        .ok_or("upstream schema_version missing")?;
    material.extend_from_slice(&(schema_version as u32).to_le_bytes());
    append_len_prefixed_string(
        &mut material,
        upstream["observation_id"]
            .as_str()
            .ok_or("observation_id missing")?,
    );
    if upstream["pixel_format"].as_str() != Some("rgb8_srgb") {
        return Err("only rgb8_srgb is supported by the renderer receipt adapter".into());
    }
    material.push(0_u8);
    for field in ["episode_id", "agent_id", "clock_id"] {
        append_len_prefixed_string(
            &mut material,
            upstream[field].as_str().ok_or(format!("{field} missing"))?,
        );
    }
    for field in ["tick", "sequence"] {
        material.extend_from_slice(
            &upstream[field]
                .as_u64()
                .ok_or(format!("{field} missing"))?
                .to_le_bytes(),
        );
    }
    for field in [
        "applied_action_receipt_id",
        "applied_action_receipt_sha256",
        "applied_action_command_id",
        "camera_id",
        "camera_calibration_sha256",
        "renderer_revision",
        "frame_sha256",
    ] {
        append_len_prefixed_string(&mut material, upstream[field].as_str().unwrap_or(""));
    }
    let action_tick = upstream["applied_action_tick"].as_u64();
    material.push(u8::from(action_tick.is_some()));
    material.extend_from_slice(&action_tick.unwrap_or(0).to_le_bytes());
    for field in [
        "capture_timestamp_us",
        "render_completed_timestamp_us",
        "delivered_timestamp_us",
        "frame_width",
        "frame_height",
        "channels",
        "frame_byte_len",
    ] {
        material.extend_from_slice(
            &upstream[field]
                .as_u64()
                .ok_or(format!("{field} missing"))?
                .to_le_bytes(),
        );
    }
    upstream["receipt_sha256"] = json!(sha256_hex(&material));
    Ok(())
}

fn sha256_hex(input: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut data = input.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());
    let mut hash = [
        0x6a09e667_u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    for chunk in data.chunks_exact(64) {
        let mut schedule = [0_u32; 64];
        for (index, word) in schedule.iter_mut().enumerate().take(16) {
            let offset = index * 4;
            *word = u32::from_be_bytes([
                chunk[offset],
                chunk[offset + 1],
                chunk[offset + 2],
                chunk[offset + 3],
            ]);
        }
        for index in 16..64 {
            let sigma_zero = schedule[index - 15].rotate_right(7)
                ^ schedule[index - 15].rotate_right(18)
                ^ (schedule[index - 15] >> 3);
            let sigma_one = schedule[index - 2].rotate_right(17)
                ^ schedule[index - 2].rotate_right(19)
                ^ (schedule[index - 2] >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(sigma_zero)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(sigma_one);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = hash;
        for index in 0..64 {
            let upper_sigma_one = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ (!e & g);
            let temp_one = h
                .wrapping_add(upper_sigma_one)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(schedule[index]);
            let upper_sigma_zero = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp_two = upper_sigma_zero.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp_one);
            d = c;
            c = b;
            b = a;
            a = temp_one.wrapping_add(temp_two);
        }
        hash[0] = hash[0].wrapping_add(a);
        hash[1] = hash[1].wrapping_add(b);
        hash[2] = hash[2].wrapping_add(c);
        hash[3] = hash[3].wrapping_add(d);
        hash[4] = hash[4].wrapping_add(e);
        hash[5] = hash[5].wrapping_add(f);
        hash[6] = hash[6].wrapping_add(g);
        hash[7] = hash[7].wrapping_add(h);
    }
    let mut output = String::with_capacity(64);
    use std::fmt::Write as _;
    for word in hash {
        let _ = write!(&mut output, "{word:08x}");
    }
    output
}

fn prepare_request(
    base_request: &Value,
    fault: FaultCase,
) -> Result<(Value, Option<CorruptTarget>, String, u64, u64), String> {
    let mut request = base_request.clone();
    let observations = request["observations"]
        .as_array()
        .ok_or("episode request requires observations[]")?;
    if observations.is_empty() {
        return Err("episode request has no observations".into());
    }
    if matches!(
        fault,
        FaultCase::StaleFrameReuse
            | FaultCase::StaleSequenceReplay
            | FaultCase::CaptureClockRegression
    ) && observations.len() < 2
    {
        return Err("stale/replay/regression controls require at least two observations".into());
    }

    let calibration_pin = observations[0]["upstream"]["camera_calibration_sha256"]
        .as_str()
        .ok_or("first upstream receipt lacks camera_calibration_sha256")?
        .to_string();
    let bridge_config = request["bridge_config"]
        .as_object_mut()
        .ok_or("episode request lacks bridge_config object")?;
    let current_pin = bridge_config
        .get("expected_camera_calibration_sha256")
        .and_then(Value::as_str);
    if current_pin.is_some_and(|existing| existing != calibration_pin) {
        return Err(
            "provided BridgeConfig calibration pin differs from the clean renderer receipt".into(),
        );
    }
    bridge_config.insert(
        "expected_camera_calibration_sha256".to_string(),
        json!(calibration_pin),
    );

    let maximum_delivery_age = request["bridge_config"]["maximum_observation_delivery_age_us"]
        .as_u64()
        .filter(|value| *value > 0)
        .ok_or("BridgeConfig must declare a non-zero maximum_observation_delivery_age_us")?;
    let maximum_clock_skew = request["bridge_config"]["maximum_sensor_clock_skew_us"]
        .as_u64()
        .ok_or("BridgeConfig lacks maximum_sensor_clock_skew_us")?;

    let observations = request["observations"]
        .as_array_mut()
        .ok_or("episode request requires observations[]")?;
    let mut corrupt_target = None;

    match fault {
        FaultCase::CleanControl => {}
        FaultCase::StaleFrameReuse => {
            observations[1]["pixels_file"] = observations[0]["pixels_file"].clone();
        }
        FaultCase::PixelCorruption => {
            let path = observations[0]["pixels_file"]
                .as_str()
                .ok_or("first observation lacks pixels_file")?;
            corrupt_target = Some(CorruptTarget {
                relative_path: safe_relative_path(path)?,
            });
        }
        FaultCase::CalibrationShift => {
            let existing = observations[0]["upstream"]["camera_calibration_sha256"]
                .as_str()
                .ok_or("first upstream receipt lacks calibration digest")?;
            observations[0]["upstream"]["camera_calibration_sha256"] =
                json!(different_digest(existing)?);
            recompute_upstream_policy_receipt(&mut observations[0]["upstream"])?;
        }
        FaultCase::CaptureTimestampMismatch => {
            let capture = observations[0]["timestamp_us"]
                .as_u64()
                .ok_or("outer capture timestamp missing")?;
            observations[0]["timestamp_us"] = json!(capture + 1);
        }
        FaultCase::ZeroSequence => {
            observations[0]["sequence"] = json!(0);
            observations[0]["upstream"]["sequence"] = json!(0);
        }
        FaultCase::ReceiptCommitmentTamper => {
            observations[0]["upstream"]["receipt_sha256"] = json!("e".repeat(64));
        }
        FaultCase::StaleSequenceReplay => {
            // Replay the entire prior accepted observation: RGB path, upstream
            // receipt, sequence, capture time and vehicle-state clock. The
            // receipt remains valid, so the persistent bridge sequence guard
            // (rather than an earlier metadata mismatch) must reject it.
            observations[1]["sequence"] = observations[0]["sequence"].clone();
            observations[1]["timestamp_us"] = observations[0]["timestamp_us"].clone();
            observations[1]["vehicle_state_timestamp_us"] =
                observations[0]["vehicle_state_timestamp_us"].clone();
            observations[1]["pixels_file"] = observations[0]["pixels_file"].clone();
            observations[1]["vehicle_state"] = observations[0]["vehicle_state"].clone();
            observations[1]["upstream"] = observations[0]["upstream"].clone();
        }
        FaultCase::CaptureClockRegression => {
            let previous_capture = observations[0]["upstream"]["capture_timestamp_us"]
                .as_u64()
                .ok_or("first capture timestamp missing")?;
            if previous_capture == 0 {
                return Err(
                    "capture-clock regression fixture requires a positive first timestamp".into(),
                );
            }
            let regressed_capture = previous_capture - 1;
            observations[1]["timestamp_us"] = json!(regressed_capture);
            observations[1]["vehicle_state_timestamp_us"] = json!(regressed_capture);
            observations[1]["vehicle_state"]["timestamp"] =
                json!(regressed_capture as f64 / 1_000_000.0);
            let upstream = &mut observations[1]["upstream"];
            upstream["capture_timestamp_us"] = json!(regressed_capture);
            upstream["render_completed_timestamp_us"] = json!(regressed_capture + 800);
            upstream["delivered_timestamp_us"] = json!(regressed_capture + 1_000);
            recompute_upstream_policy_receipt(upstream)?;
        }
        FaultCase::CameraIdentityMismatch => {
            observations[0]["upstream"]["camera_id"] = json!("fault-injected-wrong-camera");
            recompute_upstream_policy_receipt(&mut observations[0]["upstream"])?;
        }
        FaultCase::ClockOrderViolation => {
            let render_completed = observations[0]["upstream"]["render_completed_timestamp_us"]
                .as_u64()
                .ok_or("first receipt lacks render completion timestamp")?;
            observations[0]["upstream"]["delivered_timestamp_us"] =
                json!(render_completed.saturating_sub(1));
            recompute_upstream_policy_receipt(&mut observations[0]["upstream"])?;
        }
        FaultCase::SensorClockSkew => {
            let capture = observations[0]["timestamp_us"]
                .as_u64()
                .ok_or("first observation lacks timestamp_us")?;
            let skewed_timestamp = capture.saturating_add(maximum_clock_skew).saturating_add(1);
            observations[0]["vehicle_state_timestamp_us"] = json!(skewed_timestamp);
            observations[0]["vehicle_state"]["timestamp"] =
                json!(skewed_timestamp as f64 / 1_000_000.0);
        }
        FaultCase::DeliveryAgeExceeded => {
            let capture = observations[0]["upstream"]["capture_timestamp_us"]
                .as_u64()
                .ok_or("first receipt lacks capture timestamp")?;
            let render_completed = observations[0]["upstream"]["render_completed_timestamp_us"]
                .as_u64()
                .ok_or("first receipt lacks render completion timestamp")?;
            let too_late = capture
                .saturating_add(maximum_delivery_age)
                .saturating_add(1);
            observations[0]["upstream"]["delivered_timestamp_us"] =
                json!(too_late.max(render_completed));
            recompute_upstream_policy_receipt(&mut observations[0]["upstream"])?;
        }
        FaultCase::MissingActionReceipt => {
            observations[0]["upstream"]["applied_action_receipt_id"] = Value::Null;
            observations[0]["upstream"]["applied_action_receipt_sha256"] = Value::Null;
            observations[0]["upstream"]["applied_action_command_id"] = Value::Null;
            observations[0]["upstream"]["applied_action_tick"] = Value::Null;
            recompute_upstream_policy_receipt(&mut observations[0]["upstream"])?;
        }
    }

    Ok((
        request,
        corrupt_target,
        calibration_pin,
        maximum_delivery_age,
        maximum_clock_skew,
    ))
}

fn copy_episode_frames(
    source_episode_dir: &Path,
    target_input_dir: &Path,
    request: &Value,
) -> Result<(), Box<dyn Error>> {
    for observation in request["observations"]
        .as_array()
        .ok_or("episode request lacks observations[]")?
    {
        let relative_text = observation["pixels_file"]
            .as_str()
            .ok_or("observation lacks pixels_file")?;
        let relative = safe_relative_path(relative_text)?;
        let source = resolve_inside(source_episode_dir, &relative)
            .map_err(|error| format!("invalid source frame: {error}"))?;
        let destination = target_input_dir.join(&relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, destination)?;
    }
    Ok(())
}

fn corrupt_frame(target_input_dir: &Path, target: &CorruptTarget) -> Result<(), Box<dyn Error>> {
    let path = target_input_dir.join(&target.relative_path);
    let mut bytes = fs::read(&path)?;
    if bytes.is_empty() {
        return Err("cannot corrupt an empty frame artifact".into());
    }
    bytes[0] ^= 0x01;
    fs::write(path, bytes)?;
    Ok(())
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn count_policy_artifacts(path: &Path) -> Result<usize, Box<dyn Error>> {
    if !path.exists() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            count += 1;
        } else {
            return Err("policy output directory unexpectedly contains a nested directory".into());
        }
    }
    Ok(count)
}

fn expected_frames_processed_before_rejection(case: FaultCase) -> Option<u64> {
    match case {
        FaultCase::CleanControl => None,
        FaultCase::StaleFrameReuse
        | FaultCase::StaleSequenceReplay
        | FaultCase::CaptureClockRegression => Some(1),
        _ => Some(0),
    }
}

fn parse_frames_processed_before_rejection(stderr: &str) -> Option<u64> {
    let marker = "frames_processed_before_rejection=";
    let suffix = stderr.split(marker).nth(1)?;
    let digits = suffix
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

fn run_case(
    case: FaultCase,
    source_episode_dir: &Path,
    base_request: &Value,
    bridge_binary: &Path,
    output_root: &Path,
) -> Result<CaseResult, Box<dyn Error>> {
    let case_root = output_root.join(case.id());
    let input_root = case_root.join("input");
    let output_path = case_root.join("bridge-receipts.json");
    let policy_input_dir = case_root.join("policy-inputs");
    fs::create_dir_all(&input_root)?;

    let (request, corrupt_target, _calibration_pin, _, _) = prepare_request(base_request, case)
        .map_err(|error| format!("cannot prepare {} case: {error}", case.id()))?;
    copy_episode_frames(source_episode_dir, &input_root, &request)?;
    if let Some(target) = &corrupt_target {
        corrupt_frame(&input_root, target)?;
    }

    let observation_count = request["observations"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0);
    let request_path = input_root.join(REQUEST_FILE);
    write_json(&request_path, &request)?;

    let child_output = Command::new(bridge_binary)
        .arg(&request_path)
        .arg(&output_path)
        .arg(&policy_input_dir)
        .output()?;

    let stderr = String::from_utf8_lossy(&child_output.stderr).into_owned();
    let receipt_exists = output_path.exists();
    let policy_dir_exists = policy_input_dir.exists();
    let policy_artifact_count = count_policy_artifacts(&policy_input_dir)?;
    let expected_error = case.expected_error();
    let expected_frames = expected_frames_processed_before_rejection(case);
    let frames_processed_before_rejection = parse_frames_processed_before_rejection(&stderr);
    let rejected_observation_did_not_advance_temporal_state =
        expected_frames.is_some() && frames_processed_before_rejection == expected_frames;

    let (accepted, expected_error_observed, no_policy_output_committed) = if case
        == FaultCase::CleanControl
    {
        let output_valid = if child_output.status.success() && receipt_exists {
            let output: Value = serde_json::from_slice(&fs::read(&output_path)?)?;
            let receipts = output["receipts"].as_array();
            let artifacts = output["policy_input_artifacts"].as_array();
            let receipt_count = receipts.map(Vec::len).unwrap_or(0);
            let artifact_count = artifacts.map(Vec::len).unwrap_or(0);
            let mut every_artifact_reconciled = true;
            if let (Some(receipts), Some(artifacts)) = (receipts, artifacts) {
                for (index, (receipt, artifact)) in receipts.iter().zip(artifacts).enumerate() {
                    let upstream = &request["observations"][index]["upstream"];
                    if receipt["upstream_observation_id"] != upstream["observation_id"]
                        || receipt["upstream_receipt_sha256"] != upstream["receipt_sha256"]
                        || receipt["upstream_frame_sha256"] != upstream["frame_sha256"]
                        || receipt["capture_sequence"] != request["observations"][index]["sequence"]
                        || artifact["sequence"] != request["observations"][index]["sequence"]
                        || artifact["dimension"].as_u64() != Some(16_384)
                        || artifact["sha256"] != receipt["fused_policy_input_sha256"]
                    {
                        every_artifact_reconciled = false;
                        break;
                    }
                    if index == 0 {
                        if !receipt["previous_receipt_sha256"].is_null() {
                            every_artifact_reconciled = false;
                            break;
                        }
                    } else if receipt["previous_receipt_sha256"]
                        != receipts[index - 1]["receipt_sha256"]
                    {
                        every_artifact_reconciled = false;
                        break;
                    }
                    let relative_name = match artifact["relative_path"].as_str() {
                        Some(value) => value,
                        None => {
                            every_artifact_reconciled = false;
                            break;
                        }
                    };
                    let relative = match safe_relative_path(relative_name) {
                        Ok(value) => value,
                        Err(_) => {
                            every_artifact_reconciled = false;
                            break;
                        }
                    };
                    let bytes = match fs::read(policy_input_dir.join(relative)) {
                        Ok(value) => value,
                        Err(_) => {
                            every_artifact_reconciled = false;
                            break;
                        }
                    };
                    if bytes.len() != 8 + 16_384 * 4
                        || artifact["byte_length"].as_u64() != Some(bytes.len() as u64)
                        || artifact["sha256"].as_str() != Some(sha256_hex(&bytes).as_str())
                    {
                        every_artifact_reconciled = false;
                        break;
                    }
                }
            } else {
                every_artifact_reconciled = false;
            }
            output["expected_observations"].as_u64() == Some(observation_count as u64)
                && output["processed_observations"].as_u64() == Some(observation_count as u64)
                && output["frames_processed"].as_u64() == Some(observation_count as u64)
                && receipt_count == observation_count
                && artifact_count == observation_count
                && policy_artifact_count == observation_count
                && output["vision_learning_frozen"].as_bool() == Some(true)
                && output["vision_training_steps_before"] == output["vision_training_steps_after"]
                && every_artifact_reconciled
        } else {
            false
        };
        (output_valid, false, false)
    } else {
        let expected_observed = !child_output.status.success()
            && expected_error.is_some_and(|error| stderr.contains(error));
        let no_commit = !receipt_exists && policy_artifact_count == 0;
        (false, expected_observed, no_commit)
    };

    Ok(CaseResult {
        case_id: case.id().to_string(),
        expected_error: expected_error.map(str::to_string),
        actual_exit_code: child_output.status.code(),
        child_process_spawned: true,
        accepted,
        expected_error_observed,
        receipt_output_exists: receipt_exists,
        policy_input_directory_exists: policy_dir_exists,
        policy_input_artifact_count: policy_artifact_count,
        no_policy_output_committed,
        expected_frames_processed_before_rejection: expected_frames,
        frames_processed_before_rejection,
        rejected_observation_did_not_advance_temporal_state,
        input_observation_count: observation_count,
        stderr,
    })
}

fn validate_source_episode(
    episode_dir: &Path,
    request: &Value,
) -> Result<(String, String, String), Box<dyn Error>> {
    let report_path = episode_dir.join("episode-report.json");
    let report_bytes = fs::read(&report_path)?;
    if report_bytes.len() as u64 > MAX_REQUEST_BYTES {
        return Err("episode report exceeds 64 MiB".into());
    }
    let report: Value = serde_json::from_slice(&report_bytes)?;
    for field in [
        "fresh_process_renderer_replay_exact",
        "fresh_process_symthaea_bridge_replay_exact",
        "branch_evidence_audit_valid",
        "stale_actor_replay_negative_control_rejected",
        "symthaea_receipt_chain_valid",
        "policy_input_artifacts_valid",
    ] {
        if report[field].as_bool() != Some(true) {
            return Err(
                format!("source #4618 episode report is missing a passing {field} result").into(),
            );
        }
    }
    if report["status"].as_str() != Some("complete_for_declared_non_learning_fixture") {
        return Err(
            "source episode report does not declare a complete non-learning fixture".into(),
        );
    }
    let expected = report["expected_ticks"]
        .as_u64()
        .ok_or("source report lacks expected_ticks")?;
    let delivered = report["delivered_observations"]
        .as_u64()
        .ok_or("source report lacks delivered_observations")?;
    if delivered
        != request["observations"]
            .as_array()
            .map(Vec::len)
            .unwrap_or(0) as u64
        || delivered + report["missing_observations"].as_u64().unwrap_or(u64::MAX) != expected
    {
        return Err("source episode report and request observation denominators disagree".into());
    }

    let observations = request["observations"]
        .as_array()
        .ok_or("episode request lacks observations[]")?;
    let mut seen = std::collections::BTreeSet::new();
    for observation in observations {
        let sequence = observation["sequence"]
            .as_u64()
            .ok_or("observation sequence missing")?;
        if !seen.insert(sequence) {
            return Err("source episode contains duplicate observation sequences".into());
        }
        let relative_text = observation["pixels_file"]
            .as_str()
            .ok_or("observation lacks pixels_file")?;
        let relative = safe_relative_path(relative_text)?;
        let source = resolve_inside(episode_dir, &relative)?;
        let bytes = fs::read(source)?;
        let expected_frame = observation["upstream"]["frame_sha256"]
            .as_str()
            .ok_or("upstream frame SHA-256 missing")?;
        if sha256_hex(&bytes) != expected_frame {
            return Err(format!("source frame digest mismatch at sequence {sequence}").into());
        }
        let receipt_path = episode_dir
            .join("policy-receipts")
            .join(format!("policy_receipt_{sequence:04}.json"));
        let receipt: Value = serde_json::from_slice(&fs::read(&receipt_path)?)?;
        if receipt != observation["upstream"] {
            return Err(format!(
                "persisted policy receipt does not match episode request at sequence {sequence}"
            )
            .into());
        }
    }

    let episode_id = request["bridge_config"]["identity"]["episode_id"]
        .as_str()
        .ok_or("BridgeConfig episode identity missing")?
        .to_string();
    if report["episode_id"].as_str() != Some(episode_id.as_str()) {
        return Err("episode report and bridge request episode IDs differ".into());
    }
    let request_path = episode_dir.join(REQUEST_FILE);
    let request_bytes = fs::read(request_path)?;
    Ok((
        sha256_hex(&report_bytes),
        sha256_hex(&request_bytes),
        episode_id,
    ))
}

fn run_campaign(
    episode_dir: &Path,
    bridge_binary: &Path,
    output_root: &Path,
) -> Result<FaultCampaignReport, Box<dyn Error>> {
    if !episode_dir.is_dir() {
        return Err("episode directory must exist".into());
    }
    if output_root.exists() {
        let mut existing = fs::read_dir(output_root)?;
        if existing.next().is_some() {
            return Err(
                "output directory must be empty; choose a new directory for each campaign".into(),
            );
        }
    } else {
        fs::create_dir_all(output_root)?;
    }
    let episode_dir = fs::canonicalize(episode_dir)?;
    let bridge_binary = fs::canonicalize(bridge_binary)?;
    if !bridge_binary.is_file() {
        return Err("bridge binary path must name a regular file".into());
    }
    let request_path = episode_dir.join(REQUEST_FILE);
    let metadata = fs::metadata(&request_path)?;
    if !metadata.is_file() || metadata.len() > MAX_REQUEST_BYTES {
        return Err("episode request must be a regular file no larger than 64 MiB".into());
    }
    let request_bytes = fs::read(&request_path)?;
    let base_request: Value = serde_json::from_slice(&request_bytes)?;
    let (source_report_sha256, source_request_sha256, source_episode_id) =
        validate_source_episode(&episode_dir, &base_request)?;
    let bridge_binary_bytes = fs::read(&bridge_binary)?;
    let bridge_binary_sha256 = sha256_hex(&bridge_binary_bytes);

    let (_, _, calibration_pin, maximum_age, maximum_skew) =
        prepare_request(&base_request, FaultCase::CleanControl)?;
    let mut results = Vec::with_capacity(FaultCase::ALL.len());
    for case in FaultCase::ALL {
        results.push(run_case(
            case,
            &episode_dir,
            &base_request,
            &bridge_binary,
            output_root,
        )?);
    }

    let clean_control_accepted = results
        .iter()
        .find(|result| result.case_id == FaultCase::CleanControl.id())
        .is_some_and(|result| result.accepted);
    let all_negative_controls_pass = results
        .iter()
        .filter(|result| result.expected_error.is_some())
        .all(|result| result.expected_error_observed && result.no_policy_output_committed);
    let all_rejections_preflighted = results
        .iter()
        .filter(|result| result.expected_error.is_some())
        .all(|result| result.rejected_observation_did_not_advance_temporal_state);
    let fresh_processes = results
        .iter()
        .filter(|result| result.child_process_spawned)
        .count();
    let all_pass = clean_control_accepted
        && all_negative_controls_pass
        && all_rejections_preflighted
        && results.len() == FaultCase::ALL.len()
        && fresh_processes == FaultCase::ALL.len();

    let report = FaultCampaignReport {
        schema_version: 1,
        campaign_id: format!(
            "camera-boundary-fault-injection-{}",
            base_request["bridge_config"]["identity"]["episode_id"]
                .as_str()
                .unwrap_or("unknown-episode")
        ),
        status: if all_pass { "all_cases_passed" } else { "rejected" }.to_string(),
        source_episode_id,
        source_episode_report_sha256: source_report_sha256,
        source_episode_request_sha256: source_request_sha256,
        bridge_binary: bridge_binary.display().to_string(),
        bridge_binary_sha256,
        camera_calibration_pin_sha256: calibration_pin,
        maximum_observation_delivery_age_us: maximum_age,
        maximum_sensor_clock_skew_us: maximum_skew,
        cases_expected: FaultCase::ALL.len(),
        cases_observed: results.len(),
        fresh_processes_spawned: fresh_processes,
        all_negative_controls_rejected_before_policy_output: all_negative_controls_pass,
        all_rejections_preflighted_before_temporal_advance: all_rejections_preflighted,
        clean_control_accepted,
        controller_invoked: false,
        vehicle_actuator_invoked: false,
        cases: results,
        claim_ceiling: "Software boundary rejection only for the exact Symthaea bridge binary and #4618 episode bundle. This does not demonstrate learned-policy fault detection, closed-loop degradation/recovery, physical sensor fault fidelity, safe-stop performance or vehicle safety.".to_string(),
    };

    write_json(&output_root.join("fault-campaign-report.json"), &report)?;
    Ok(report)
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(
            "usage: camera-boundary-fault-injection <symtropy-episode-dir> <camera-driving-bridge-binary> <empty-output-dir>"
                .into(),
        );
    }
    let report = run_campaign(
        Path::new(&args[0]),
        Path::new(&args[1]),
        Path::new(&args[2]),
    )?;
    println!(
        "camera boundary fault campaign: {} ({} / {} cases; {} fresh bridge processes)",
        report.status, report.cases_observed, report.cases_expected, report.fresh_processes_spawned
    );
    println!(
        "report: {}",
        Path::new(&args[2])
            .join("fault-campaign-report.json")
            .display()
    );
    if report.status != "all_cases_passed" {
        return Err("one or more camera boundary fault-injection controls failed".into());
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("camera-boundary-fault-injection: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_request() -> Value {
        let observation = |sequence: u64, tick: u64| {
            json!({
                "sequence": sequence,
                "timestamp_us": tick * 10_000,
                "vehicle_state_timestamp_us": tick * 10_000,
                "pixels_file": format!("frames/frame_{sequence:04}.rgb"),
                "vehicle_state": { "timestamp": tick as f64 * 0.01 },
                "upstream": {
                    "schema_version": 1,
                    "observation_id": format!("{:064x}", sequence + 1),
                    "pixel_format": "rgb8_srgb",
                    "frame_byte_len": 768,
                    "receipt_sha256": "e".repeat(64),
                    "frame_sha256": format!("{:064x}", sequence + 10),
                    "episode_id": "episode-test",
                    "agent_id": "ego",
                    "clock_id": "clock-test",
                    "tick": tick,
                    "sequence": sequence,
                    "camera_id": "front-rgb",
                    "camera_calibration_sha256": "c".repeat(64),
                    "renderer_revision": "renderer-fixture-v1",
                    "capture_timestamp_us": tick * 10_000,
                    "render_completed_timestamp_us": tick * 10_000 + 800,
                    "delivered_timestamp_us": tick * 10_000 + 1_000,
                    "frame_width": 16,
                    "frame_height": 16,
                    "channels": 3,
                    "applied_action_receipt_id": format!("action-{tick}"),
                    "applied_action_receipt_sha256": "a".repeat(64),
                    "applied_action_command_id": format!("command-{tick}"),
                    "applied_action_tick": tick
                }
            })
        };
        let mut request = json!({
            "schema_version": 1,
            "bridge_config": {
                "maximum_observation_delivery_age_us": 5000,
                "maximum_sensor_clock_skew_us": 1000,
                "expected_camera_calibration_sha256": null
            },
            "observations": [observation(1, 1), observation(2, 2)]
        });
        for observation in request["observations"].as_array_mut().unwrap() {
            recompute_upstream_policy_receipt(&mut observation["upstream"]).unwrap();
        }
        request
    }

    #[test]
    fn receipt_sha256_implementation_matches_known_answers() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn clean_request_is_pinned_to_the_exact_renderer_calibration() {
        let (request, _, pin, max_age, max_skew) =
            prepare_request(&base_request(), FaultCase::CleanControl).unwrap();
        assert_eq!(pin, "c".repeat(64));
        assert_eq!(
            request["bridge_config"]["expected_camera_calibration_sha256"],
            pin
        );
        assert_eq!(max_age, 5_000);
        assert_eq!(max_skew, 1_000);
    }

    #[test]
    fn pixel_corruption_keeps_the_original_receipt_and_targets_a_safe_frame_path() {
        let base = base_request();
        let (request, corrupt_target, _, _, _) =
            prepare_request(&base, FaultCase::PixelCorruption).unwrap();
        let target = corrupt_target.expect("pixel corruption must identify one frame artifact");
        assert_eq!(target.relative_path, PathBuf::from("frames/frame_0001.rgb"));
        assert_eq!(
            request["observations"][0]["upstream"]["frame_sha256"],
            base["observations"][0]["upstream"]["frame_sha256"]
        );
        assert_eq!(
            request["observations"][0]["upstream"]["receipt_sha256"],
            base["observations"][0]["upstream"]["receipt_sha256"]
        );
    }

    #[test]
    fn replay_mutations_require_at_least_two_observations() {
        let mut base = base_request();
        base["observations"].as_array_mut().unwrap().pop();
        for case in [
            FaultCase::StaleFrameReuse,
            FaultCase::StaleSequenceReplay,
            FaultCase::CaptureClockRegression,
        ] {
            assert!(prepare_request(&base, case).is_err());
        }
    }

    #[test]
    fn local_sha256_adapter_matches_known_answer_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn capture_regression_and_receipt_tamper_have_distinct_exact_controls() {
        let base = base_request();
        let (regressed, _, _, _, _) =
            prepare_request(&base, FaultCase::CaptureClockRegression).unwrap();
        assert_eq!(
            regressed["observations"][1]["upstream"]["capture_timestamp_us"],
            regressed["observations"][1]["timestamp_us"]
        );
        assert_ne!(
            regressed["observations"][1]["upstream"]["receipt_sha256"],
            base["observations"][1]["upstream"]["receipt_sha256"]
        );
        assert_eq!(
            FaultCase::CaptureClockRegression.expected_error(),
            Some("TimestampRegression")
        );

        let (tampered, _, _, _, _) =
            prepare_request(&base, FaultCase::ReceiptCommitmentTamper).unwrap();
        assert_eq!(
            tampered["observations"][0]["upstream"]["receipt_sha256"],
            "e".repeat(64)
        );
        assert_eq!(
            FaultCase::ReceiptCommitmentTamper.expected_error(),
            Some("UpstreamReceiptCommitmentMismatch")
        );
    }

    #[test]
    fn stale_frame_reuse_preserves_later_receipt_but_changes_selected_frame_path() {
        let base = base_request();
        let (request, _, _, _, _) = prepare_request(&base, FaultCase::StaleFrameReuse).unwrap();
        assert_eq!(
            request["observations"][1]["pixels_file"],
            request["observations"][0]["pixels_file"]
        );
        assert_ne!(
            request["observations"][1]["upstream"]["frame_sha256"],
            request["observations"][0]["upstream"]["frame_sha256"]
        );
    }

    #[test]
    fn receipt_recomputation_preserves_internal_consistency_after_camera_mutation() {
        let base = base_request();
        let mut upstream = base["observations"][0]["upstream"].clone();
        let old_receipt = upstream["receipt_sha256"].clone();
        upstream["camera_id"] = json!("front-camera-shifted");
        recompute_upstream_policy_receipt(&mut upstream).unwrap();
        assert_ne!(upstream["receipt_sha256"], old_receipt);
        assert_eq!(
            upstream["observation_id"],
            base["observations"][0]["upstream"]["observation_id"]
        );
    }

    #[test]
    fn capture_timestamp_mismatch_and_zero_sequence_mutations_are_distinct() {
        let base = base_request();
        let (mismatch, _, _, _, _) =
            prepare_request(&base, FaultCase::CaptureTimestampMismatch).unwrap();
        assert_ne!(
            mismatch["observations"][0]["timestamp_us"],
            mismatch["observations"][0]["upstream"]["capture_timestamp_us"]
        );
        let (zero, _, _, _, _) = prepare_request(&base, FaultCase::ZeroSequence).unwrap();
        assert_eq!(zero["observations"][0]["sequence"], 0);
        assert_eq!(zero["observations"][0]["upstream"]["sequence"], 0);
    }

    #[test]
    fn calibration_shift_keeps_a_valid_but_different_digest() {
        let (request, _, pin, _, _) =
            prepare_request(&base_request(), FaultCase::CalibrationShift).unwrap();
        let mutated = request["observations"][0]["upstream"]["camera_calibration_sha256"]
            .as_str()
            .unwrap();
        assert_eq!(mutated.len(), 64);
        assert_ne!(mutated, pin);
        assert_eq!(
            request["bridge_config"]["expected_camera_calibration_sha256"],
            pin
        );
    }

    #[test]
    fn delay_and_clock_order_mutations_are_distinct() {
        let base = base_request();
        let (delayed, _, _, max_age, _) =
            prepare_request(&base, FaultCase::DeliveryAgeExceeded).unwrap();
        let capture = delayed["observations"][0]["upstream"]["capture_timestamp_us"]
            .as_u64()
            .unwrap();
        let delivered = delayed["observations"][0]["upstream"]["delivered_timestamp_us"]
            .as_u64()
            .unwrap();
        let render_completed =
            delayed["observations"][0]["upstream"]["render_completed_timestamp_us"]
                .as_u64()
                .unwrap();
        assert!(delivered - capture > max_age);
        assert!(delivered >= render_completed);

        let (reordered, _, _, _, _) =
            prepare_request(&base, FaultCase::ClockOrderViolation).unwrap();
        assert!(
            reordered["observations"][0]["upstream"]["delivered_timestamp_us"]
                .as_u64()
                .unwrap()
                < reordered["observations"][0]["upstream"]["render_completed_timestamp_us"]
                    .as_u64()
                    .unwrap()
        );
    }

    #[test]
    fn sensor_skew_and_stale_sequence_mutations_are_separate() {
        let base = base_request();
        let (skewed, _, _, _, max_skew) =
            prepare_request(&base, FaultCase::SensorClockSkew).unwrap();
        let camera = skewed["observations"][0]["timestamp_us"].as_u64().unwrap();
        let state = skewed["observations"][0]["vehicle_state_timestamp_us"]
            .as_u64()
            .unwrap();
        assert_eq!(state - camera, max_skew + 1);
        assert!(
            (skewed["observations"][0]["vehicle_state"]["timestamp"]
                .as_f64()
                .unwrap()
                - state as f64 / 1_000_000.0)
                .abs()
                < 1e-9
        );

        let (stale, _, _, _, _) = prepare_request(&base, FaultCase::StaleSequenceReplay).unwrap();
        assert_eq!(
            stale["observations"][1]["sequence"],
            stale["observations"][0]["sequence"]
        );
        assert_eq!(
            stale["observations"][1]["upstream"],
            stale["observations"][0]["upstream"]
        );
        assert_eq!(
            stale["observations"][1]["pixels_file"],
            stale["observations"][0]["pixels_file"]
        );
        assert_eq!(
            FaultCase::StaleSequenceReplay.expected_error(),
            Some("SequenceRegression")
        );
    }

    #[test]
    fn capture_clock_regression_has_a_valid_recomputed_upstream_receipt() {
        let base = base_request();
        let (request, _, _, _, _) =
            prepare_request(&base, FaultCase::CaptureClockRegression).unwrap();
        let first_capture = request["observations"][0]["timestamp_us"].as_u64().unwrap();
        let second_capture = request["observations"][1]["timestamp_us"].as_u64().unwrap();
        assert_eq!(second_capture, first_capture - 1);
        assert_eq!(
            request["observations"][1]["upstream"]["capture_timestamp_us"],
            json!(second_capture)
        );
        assert_ne!(
            request["observations"][1]["upstream"]["receipt_sha256"],
            base["observations"][1]["upstream"]["receipt_sha256"]
        );
    }

    #[test]
    fn missing_action_reference_and_camera_identity_mutations_are_explicit() {
        let base = base_request();
        let (missing_action, _, _, _, _) =
            prepare_request(&base, FaultCase::MissingActionReceipt).unwrap();
        assert!(
            missing_action["observations"][0]["upstream"]["applied_action_receipt_id"].is_null()
        );
        assert_ne!(
            missing_action["observations"][0]["upstream"]["receipt_sha256"],
            base["observations"][0]["upstream"]["receipt_sha256"]
        );
        let mut recommit = missing_action["observations"][0]["upstream"].clone();
        let expected_commitment = recommit["receipt_sha256"].clone();
        recompute_upstream_policy_receipt(&mut recommit).unwrap();
        assert_eq!(recommit["receipt_sha256"], expected_commitment);

        let (wrong_camera, _, _, _, _) =
            prepare_request(&base, FaultCase::CameraIdentityMismatch).unwrap();
        assert_ne!(
            wrong_camera["observations"][0]["upstream"]["camera_id"],
            "front-rgb"
        );
    }

    #[test]
    fn later_observation_faults_report_one_prior_frame_and_first_frame_faults_report_zero() {
        assert_eq!(
            expected_frames_processed_before_rejection(FaultCase::CleanControl),
            None
        );
        assert_eq!(
            expected_frames_processed_before_rejection(FaultCase::StaleFrameReuse),
            Some(1)
        );
        assert_eq!(
            expected_frames_processed_before_rejection(FaultCase::StaleSequenceReplay),
            Some(1)
        );
        assert_eq!(
            expected_frames_processed_before_rejection(FaultCase::CaptureClockRegression),
            Some(1)
        );
        for case in [
            FaultCase::PixelCorruption,
            FaultCase::CalibrationShift,
            FaultCase::CaptureTimestampMismatch,
            FaultCase::ZeroSequence,
            FaultCase::ReceiptCommitmentTamper,
            FaultCase::CameraIdentityMismatch,
            FaultCase::ClockOrderViolation,
            FaultCase::SensorClockSkew,
            FaultCase::DeliveryAgeExceeded,
            FaultCase::MissingActionReceipt,
        ] {
            assert_eq!(expected_frames_processed_before_rejection(case), Some(0));
        }
    }

    #[test]
    fn source_episode_gate_requires_passing_replay_receipts_and_byte_digests() {
        let base = std::env::temp_dir().join(format!(
            "camera-boundary-source-episode-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("frames")).unwrap();
        fs::create_dir_all(base.join("policy-receipts")).unwrap();

        let pixels = vec![17_u8; 16 * 16 * 3];
        let frame_sha = sha256_hex(&pixels);
        fs::write(base.join("frames/frame_0001.rgb"), &pixels).unwrap();
        let upstream = json!({
            "sequence": 1,
            "frame_sha256": frame_sha,
            "observation_id": "b".repeat(64),
            "receipt_sha256": "c".repeat(64)
        });
        write_json(
            &base.join("policy-receipts/policy_receipt_0001.json"),
            &upstream,
        )
        .unwrap();
        let request = json!({
            "bridge_config": {
                "identity": {
                    "episode_id": "source-episode-test"
                }
            },
            "observations": [{
                "sequence": 1,
                "pixels_file": "frames/frame_0001.rgb",
                "upstream": upstream
            }]
        });
        let report = json!({
            "status": "complete_for_declared_non_learning_fixture",
            "episode_id": "source-episode-test",
            "expected_ticks": 1,
            "delivered_observations": 1,
            "missing_observations": 0,
            "fresh_process_renderer_replay_exact": true,
            "fresh_process_symthaea_bridge_replay_exact": true,
            "branch_evidence_audit_valid": true,
            "stale_actor_replay_negative_control_rejected": true,
            "symthaea_receipt_chain_valid": true,
            "policy_input_artifacts_valid": true
        });
        write_json(&base.join(REQUEST_FILE), &request).unwrap();
        write_json(&base.join("episode-report.json"), &report).unwrap();

        assert!(validate_source_episode(&base, &request).is_ok());

        let mut invalid_report = report;
        invalid_report["fresh_process_symthaea_bridge_replay_exact"] = json!(false);
        write_json(&base.join("episode-report.json"), &invalid_report).unwrap();
        assert!(validate_source_episode(&base, &request).is_err());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn source_episode_gate_rejects_modified_frame_bytes_and_receipt_mismatch() {
        let base = std::env::temp_dir().join(format!(
            "camera-boundary-source-integrity-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("frames")).unwrap();
        fs::create_dir_all(base.join("policy-receipts")).unwrap();
        let pixels = vec![29_u8; 16 * 16 * 3];
        let frame_sha = sha256_hex(&pixels);
        fs::write(base.join("frames/frame_0001.rgb"), &pixels).unwrap();
        let upstream = json!({
            "sequence": 1,
            "frame_sha256": frame_sha,
            "observation_id": "b".repeat(64),
            "receipt_sha256": "c".repeat(64)
        });
        write_json(
            &base.join("policy-receipts/policy_receipt_0001.json"),
            &upstream,
        )
        .unwrap();
        let request = json!({
            "bridge_config": {"identity":{"episode_id":"source-episode-test"}},
            "observations":[{
                "sequence":1,
                "pixels_file":"frames/frame_0001.rgb",
                "upstream":upstream
            }]
        });
        let report = json!({
            "status":"complete_for_declared_non_learning_fixture",
            "episode_id":"source-episode-test",
            "expected_ticks":1,
            "delivered_observations":1,
            "missing_observations":0,
            "fresh_process_renderer_replay_exact":true,
            "fresh_process_symthaea_bridge_replay_exact":true,
            "branch_evidence_audit_valid":true,
            "stale_actor_replay_negative_control_rejected":true,
            "symthaea_receipt_chain_valid":true,
            "policy_input_artifacts_valid":true
        });
        write_json(&base.join(REQUEST_FILE), &request).unwrap();
        write_json(&base.join("episode-report.json"), &report).unwrap();

        let mut changed_pixels = pixels.clone();
        changed_pixels[0] ^= 1;
        fs::write(base.join("frames/frame_0001.rgb"), &changed_pixels).unwrap();
        assert!(validate_source_episode(&base, &request).is_err());

        fs::write(base.join("frames/frame_0001.rgb"), pixels).unwrap();
        write_json(
            &base.join("policy-receipts/policy_receipt_0001.json"),
            &json!({"not":"the request receipt"}),
        )
        .unwrap();
        assert!(validate_source_episode(&base, &request).is_err());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn parses_temporal_state_count_from_bridge_rejection_diagnostic() {
        assert_eq!(
            parse_frames_processed_before_rejection(
                "camera-driving-bridge: SequenceRegression (frames_processed_before_rejection=1)"
            ),
            Some(1)
        );
        assert_eq!(
            parse_frames_processed_before_rejection(
                "camera-driving-bridge: UpstreamFrameMismatch (frames_processed_before_rejection=0)"
            ),
            Some(0)
        );
        assert_eq!(
            parse_frames_processed_before_rejection("camera-driving-bridge: UpstreamFrameMismatch"),
            None
        );
    }

    #[test]
    fn every_negative_control_has_a_declared_expected_bridge_error() {
        let cases = FaultCase::ALL
            .into_iter()
            .filter(|case| *case != FaultCase::CleanControl)
            .collect::<Vec<_>>();
        assert_eq!(cases.len(), FaultCase::ALL.len() - 1);
        assert!(cases.iter().all(|case| case.expected_error().is_some()));
        assert_eq!(
            cases
                .iter()
                .filter(|case| case.expected_error() == Some("UpstreamFrameMismatch"))
                .count(),
            2
        );
    }
}
