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
use serde_json::{json, Value};
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
    PixelCorruption,
    CalibrationShift,
    StaleSequenceReplay,
    ClockOrderViolation,
    SensorClockSkew,
    DeliveryAgeExceeded,
    CameraIdentityMismatch,
    MissingActionReceipt,
}

impl FaultCase {
    const ALL: [Self; 9] = [
        Self::CleanControl,
        Self::PixelCorruption,
        Self::CalibrationShift,
        Self::StaleSequenceReplay,
        Self::ClockOrderViolation,
        Self::SensorClockSkew,
        Self::DeliveryAgeExceeded,
        Self::CameraIdentityMismatch,
        Self::MissingActionReceipt,
    ];

    fn id(self) -> &'static str {
        match self {
            Self::CleanControl => "clean-control",
            Self::PixelCorruption => "pixel-corruption",
            Self::CalibrationShift => "calibration-shift",
            Self::StaleSequenceReplay => "stale-sequence-replay",
            Self::ClockOrderViolation => "clock-order-violation",
            Self::SensorClockSkew => "sensor-clock-skew",
            Self::DeliveryAgeExceeded => "delivery-age-exceeded",
            Self::CameraIdentityMismatch => "camera-identity-mismatch",
            Self::MissingActionReceipt => "missing-action-receipt",
        }
    }

    fn expected_error(self) -> Option<&'static str> {
        match self {
            Self::CleanControl => None,
            Self::PixelCorruption => Some("UpstreamFrameMismatch"),
            Self::CalibrationShift => Some("UpstreamCalibrationMismatch"),
            Self::StaleSequenceReplay => Some("UpstreamSequenceMismatch"),
            Self::ClockOrderViolation => Some("UpstreamClockOrderInvalid"),
            Self::SensorClockSkew => Some("SensorClockSkewExceeded"),
            Self::DeliveryAgeExceeded => Some("UpstreamDeliveryAgeExceeded"),
            Self::CameraIdentityMismatch => Some("UpstreamIdentityMismatch"),
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
    input_observation_count: usize,
    stderr: String,
}

#[derive(Debug, Serialize)]
struct FaultCampaignReport {
    schema_version: u32,
    campaign_id: String,
    status: String,
    source_episode_id: String,
    bridge_binary: String,
    camera_calibration_pin_sha256: String,
    maximum_observation_delivery_age_us: u64,
    maximum_sensor_clock_skew_us: u64,
    cases_expected: usize,
    cases_observed: usize,
    fresh_processes_spawned: usize,
    all_negative_controls_rejected_before_policy_output: bool,
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
        return Err(format!("pixels_file is not a safe relative path: {value:?}"));
    }
    Ok(path.to_path_buf())
}

fn resolve_inside(base: &Path, relative: &Path) -> Result<PathBuf, String> {
    let canonical_base = fs::canonicalize(base)
        .map_err(|error| format!("cannot resolve input base: {error}"))?;
    let candidate = fs::canonicalize(canonical_base.join(relative))
        .map_err(|error| format!("cannot resolve source artifact {relative:?}: {error}"))?;
    if !candidate.starts_with(&canonical_base) {
        return Err(format!("source artifact escapes episode directory: {relative:?}"));
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
    if fault == FaultCase::StaleSequenceReplay && observations.len() < 2 {
        return Err("stale-sequence replay control requires at least two observations".into());
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
        return Err("provided BridgeConfig calibration pin differs from the clean renderer receipt".into());
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
        }
        FaultCase::StaleSequenceReplay => {
            let previous = observations[0]["upstream"]["sequence"]
                .as_u64()
                .ok_or("first upstream receipt lacks sequence")?;
            // Keep the outer request sequence unchanged but replay an older
            // upstream sequence, creating a stale/inconsistent boundary record.
            observations[1]["upstream"]["sequence"] = json!(previous);
        }
        FaultCase::ClockOrderViolation => {
            let render_completed = observations[0]["upstream"]["render_completed_timestamp_us"]
                .as_u64()
                .ok_or("first receipt lacks render completion timestamp")?;
            observations[0]["upstream"]["delivered_timestamp_us"] =
                json!(render_completed.saturating_sub(1));
        }
        FaultCase::SensorClockSkew => {
            let capture = observations[0]["timestamp_us"]
                .as_u64()
                .ok_or("first observation lacks timestamp_us")?;
            observations[0]["vehicle_state_timestamp_us"] =
                json!(capture.saturating_add(maximum_clock_skew).saturating_add(1));
        }
        FaultCase::DeliveryAgeExceeded => {
            let capture = observations[0]["upstream"]["capture_timestamp_us"]
                .as_u64()
                .ok_or("first receipt lacks capture timestamp")?;
            let render_completed = observations[0]["upstream"]["render_completed_timestamp_us"]
                .as_u64()
                .ok_or("first receipt lacks render completion timestamp")?;
            let too_late = capture.saturating_add(maximum_delivery_age).saturating_add(1);
            observations[0]["upstream"]["delivered_timestamp_us"] =
                json!(too_late.max(render_completed));
        }
        FaultCase::CameraIdentityMismatch => {
            observations[0]["upstream"]["camera_id"] = json!("fault-injected-wrong-camera");
        }
        FaultCase::MissingActionReceipt => {
            observations[0]["upstream"]["applied_action_receipt_id"] = Value::Null;
            observations[0]["upstream"]["applied_action_receipt_sha256"] = Value::Null;
            observations[0]["upstream"]["applied_action_command_id"] = Value::Null;
            observations[0]["upstream"]["applied_action_tick"] = Value::Null;
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

    let (mut request, corrupt_target, _calibration_pin, _, _) =
        prepare_request(base_request, case)
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

    let (accepted, expected_error_observed, no_policy_output_committed) =
        if case == FaultCase::CleanControl {
            let output_valid = if child_output.status.success() && receipt_exists {
                let output: Value = serde_json::from_slice(&fs::read(&output_path)?)?;
                let receipt_count = output["receipts"].as_array().map(Vec::len).unwrap_or(0);
                let artifact_count = output["policy_input_artifacts"]
                    .as_array()
                    .map(Vec::len)
                    .unwrap_or(0);
                output["expected_observations"].as_u64() == Some(observation_count as u64)
                    && output["processed_observations"].as_u64() == Some(observation_count as u64)
                    && receipt_count == observation_count
                    && artifact_count == observation_count
                    && policy_artifact_count == observation_count
                    && output["vision_learning_frozen"].as_bool() == Some(true)
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
        input_observation_count: observation_count,
        stderr,
    })
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
            return Err("output directory must be empty; choose a new directory for each campaign".into());
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
    let base_request: Value = serde_json::from_slice(&fs::read(&request_path)?)?;

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
    let fresh_processes = results.iter().filter(|result| result.child_process_spawned).count();
    let all_pass = clean_control_accepted
        && all_negative_controls_pass
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
        source_episode_id: base_request["bridge_config"]["identity"]["episode_id"]
            .as_str()
            .unwrap_or("unknown-episode")
            .to_string(),
        bridge_binary: bridge_binary.display().to_string(),
        camera_calibration_pin_sha256: calibration_pin,
        maximum_observation_delivery_age_us: maximum_age,
        maximum_sensor_clock_skew_us: maximum_skew,
        cases_expected: FaultCase::ALL.len(),
        cases_observed: results.len(),
        fresh_processes_spawned: fresh_processes,
        all_negative_controls_rejected_before_policy_output: all_negative_controls_pass,
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
        report.status,
        report.cases_observed,
        report.cases_expected,
        report.fresh_processes_spawned
    );
    println!(
        "report: {}",
        Path::new(&args[2]).join("fault-campaign-report.json").display()
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
                    "sequence": sequence,
                    "tick": tick,
                    "capture_timestamp_us": tick * 10_000,
                    "render_completed_timestamp_us": tick * 10_000 + 800,
                    "delivered_timestamp_us": tick * 10_000 + 1_000,
                    "camera_calibration_sha256": "c".repeat(64),
                    "camera_id": "front-rgb",
                    "applied_action_receipt_id": format!("action-{tick}"),
                    "applied_action_receipt_sha256": "a".repeat(64),
                    "applied_action_command_id": format!("command-{tick}"),
                    "applied_action_tick": tick
                }
            })
        };
        json!({
            "schema_version": 1,
            "bridge_config": {
                "maximum_observation_delivery_age_us": 5000,
                "maximum_sensor_clock_skew_us": 1000,
                "expected_camera_calibration_sha256": null
            },
            "observations": [observation(1, 1), observation(2, 2)]
        })
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
        let render_completed = delayed["observations"][0]["upstream"]
            ["render_completed_timestamp_us"]
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
        let camera = skewed["observations"][0]["timestamp_us"]
            .as_u64()
            .unwrap();
        let state = skewed["observations"][0]["vehicle_state_timestamp_us"]
            .as_u64()
            .unwrap();
        assert_eq!(state - camera, max_skew + 1);

        let (stale, _, _, _, _) =
            prepare_request(&base, FaultCase::StaleSequenceReplay).unwrap();
        assert_ne!(
            stale["observations"][1]["sequence"],
            stale["observations"][1]["upstream"]["sequence"]
        );
    }

    #[test]
    fn missing_action_reference_and_camera_identity_mutations_are_explicit() {
        let base = base_request();
        let (missing_action, _, _, _, _) =
            prepare_request(&base, FaultCase::MissingActionReceipt).unwrap();
        assert!(missing_action["observations"][0]["upstream"]["applied_action_receipt_id"].is_null());

        let (wrong_camera, _, _, _, _) =
            prepare_request(&base, FaultCase::CameraIdentityMismatch).unwrap();
        assert_ne!(
            wrong_camera["observations"][0]["upstream"]["camera_id"],
            "front-rgb"
        );
    }

    #[test]
    fn every_negative_control_has_a_distinct_expected_bridge_error() {
        let mut errors = std::collections::BTreeSet::new();
        for case in FaultCase::ALL.into_iter().filter(|case| *case != FaultCase::CleanControl) {
            let error = case.expected_error().unwrap();
            assert!(errors.insert(error));
        }
        assert_eq!(errors.len(), FaultCase::ALL.len() - 1);
    }
}
