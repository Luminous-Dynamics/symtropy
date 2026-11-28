// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Imports verified ROOM-TWIN-01 RGBA8 frames into the existing #6201
//! agent-visible observation bundle and causal transport replay contract.
//!
//! This binary accepts one variant's decoded frames at a time. It never reads
//! the public truth crosswalk and emits no variant/world label into the bundle.

use av_observation_replay_v0::{
    AgentObservationBundleV1, AgentObservationV1, ArtifactRefV1, BoundedTimeV1, CaptureIntervalV1,
    ClockDomainV1, ColorRangeV1, ColorSpaceV1, PixelFormatV1, ReplayCursorV1, SCHEMA_VERSION_V1,
    TransportDispositionV1, TransportEventV1, VideoFrameObservationV1,
    read_verified_artifact_under_root_v1, sha256_hex_v1, validate_bundle_v1,
    verify_artifact_bytes_v1,
};
use serde_json::{Value, json};
use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

const EXPECTED_CAPTURE_SECS: [f64; 3] = [1.5, 4.0, 7.0];
const EXPECTED_NAMES: [&str; 3] = ["frame-0001.rgba8", "frame-0002.rgba8", "frame-0003.rgba8"];
const MAX_PROFILE_BYTES: u64 = 1024 * 1024;
const MAX_FRAME_BYTES: u64 = 128 * 1024 * 1024;
const STREAM_ID: &str = "room-twin-01-agent-observation-stream-v1";

fn read_json_limited(path: &Path, max_bytes: u64) -> Result<(Vec<u8>, Value), Box<dyn Error>> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() || metadata.len() > max_bytes {
        return Err(
            format!("input JSON is not a regular file or exceeds {max_bytes} bytes").into(),
        );
    }
    let bytes = fs::read(path)?;
    if u64::try_from(bytes.len())? != metadata.len() {
        return Err("input JSON changed while reading".into());
    }
    let value = serde_json::from_slice(&bytes)?;
    Ok((bytes, value))
}

fn seconds_to_ns(value: f64) -> Result<i64, Box<dyn Error>> {
    if !value.is_finite() || value < 0.0 {
        return Err("capture time must be finite and non-negative".into());
    }
    let nanoseconds = value * 1_000_000_000.0;
    if nanoseconds > i64::MAX as f64 {
        return Err("capture time exceeds signed nanosecond range".into());
    }
    Ok(nanoseconds.round() as i64)
}

fn read_input_frame(root: &Path, name: &str, expected_len: u64) -> Result<Vec<u8>, Box<dyn Error>> {
    let canonical_root = root.canonicalize()?;
    let candidate = canonical_root.join(name);
    let canonical_file = candidate.canonicalize()?;
    if !canonical_file.starts_with(&canonical_root) {
        return Err(format!("frame path escapes decoded frame root: {name}").into());
    }
    let metadata = fs::metadata(&canonical_file)?;
    if !metadata.is_file() || metadata.len() != expected_len || metadata.len() > MAX_FRAME_BYTES {
        return Err(format!("frame {name} has an unexpected file type or byte length").into());
    }
    let bytes = fs::read(&canonical_file)?;
    if u64::try_from(bytes.len())? != expected_len {
        return Err(format!("frame {name} changed while reading").into());
    }
    Ok(bytes)
}

fn validate_camera_profile(
    profile: &Value,
) -> Result<(u32, u32, u64, ClockDomainV1), Box<dyn Error>> {
    if profile["schema_version"] != 1
        || profile["fixture_id"] != "ROOM-TWIN-01"
        || profile["camera_profile_id"] != "room-twin-camera-v1"
        || profile["sensor_id"] != "camera-front"
        || profile["renderer"] != "Bevy 0.19.0"
        || profile["pose_delivery"]["status"] != "not_agent_visible"
        || profile["pose_delivery"]["pose_source"] != "renderer_evaluator_only"
        || profile.get("camera_rail").is_some()
        || profile["image"]["pixel_format"] != "rgba8"
        || profile["image"]["channel_order"] != "rgba"
        || profile["image"]["color_space"] != "srgb"
        || profile["image"]["color_range"] != "full"
        || profile["capture_time_bounds"]["exact_exposure_timestamp_available"] != false
        || profile["capture_time_bounds"]["upper_bound_secs"] != 23.5
        || profile["calibration"]["calibration_profile_sha256"] != Value::Null
        || profile["calibration"]["status"] != "synthetic_un-calibrated"
    {
        return Err("camera profile does not satisfy the ROOM-TWIN-01 raw-frame contract".into());
    }

    let width = u32::try_from(
        profile["image"]["width_px"]
            .as_u64()
            .ok_or("camera profile lacks width_px")?,
    )?;
    let height = u32::try_from(
        profile["image"]["height_px"]
            .as_u64()
            .ok_or("camera profile lacks height_px")?,
    )?;
    let stride = u64::from(width)
        .checked_mul(4)
        .ok_or("RGBA8 stride overflow")?;
    if width == 0 || height == 0 {
        return Err("camera profile declares zero image dimensions".into());
    }

    let clock_domain = ClockDomainV1 {
        domain_id: profile["clock_domain"]["domain_id"]
            .as_str()
            .ok_or("camera profile lacks clock domain ID")?
            .to_owned(),
        generation: profile["clock_domain"]["generation"]
            .as_u64()
            .ok_or("camera profile lacks clock generation")?,
        time_scale_id: profile["clock_domain"]["time_scale_id"]
            .as_str()
            .ok_or("camera profile lacks time scale ID")?
            .to_owned(),
    };

    let capture_times = profile["capture_time_bounds"]["capture_schedule_secs"]
        .as_array()
        .ok_or("camera profile lacks capture schedule")?;
    if capture_times.len() != EXPECTED_CAPTURE_SECS.len()
        || capture_times
            .iter()
            .zip(EXPECTED_CAPTURE_SECS)
            .any(|(actual, expected)| {
                actual
                    .as_f64()
                    .is_none_or(|value| (value - expected).abs() > 0.01)
            })
    {
        return Err("camera profile schedule differs from ROOM-TWIN-01 schedule".into());
    }

    let ticks = profile["transport"]["availability_ticks"]
        .as_array()
        .ok_or("camera profile lacks availability ticks")?;
    if ticks.len() != EXPECTED_CAPTURE_SECS.len()
        || ticks
            .iter()
            .enumerate()
            .any(|(index, value)| value.as_u64() != Some((index + 1) as u64))
        || profile["transport"]["prefix_decision_cutoff_tick"] != 2
        || profile["transport"]["first_distinguishing_observation_tick"] != 3
    {
        return Err("camera profile transport schedule is invalid".into());
    }

    Ok((width, height, stride, clock_domain))
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    let frames_dir = PathBuf::from(args.next().ok_or(
        "usage: room-twin-observation-import <decoded-rgba8-dir> <camera-profile.json> <output-dir>",
    )?);
    let profile_path = PathBuf::from(args.next().ok_or(
        "usage: room-twin-observation-import <decoded-rgba8-dir> <camera-profile.json> <output-dir>",
    )?);
    let output_dir = PathBuf::from(args.next().ok_or(
        "usage: room-twin-observation-import <decoded-rgba8-dir> <camera-profile.json> <output-dir>",
    )?);
    if args.next().is_some() {
        return Err("too many arguments".into());
    }

    let (profile_bytes, profile) = read_json_limited(&profile_path, MAX_PROFILE_BYTES)?;
    let profile_sha256 = sha256_hex_v1(&profile_bytes);
    let (width, height, stride, clock_domain) = validate_camera_profile(&profile)?;
    let upper_ns = seconds_to_ns(
        profile["capture_time_bounds"]["upper_bound_secs"]
            .as_f64()
            .ok_or("camera profile lacks upper bound")?,
    )?;
    let expected_frame_len = stride
        .checked_mul(u64::from(height))
        .ok_or("RGBA8 frame byte length overflow")?;
    if expected_frame_len > MAX_FRAME_BYTES {
        return Err("RGBA8 frame exceeds import byte cap".into());
    }

    if output_dir.exists() {
        return Err(format!(
            "refusing to overwrite output directory {}",
            output_dir.display()
        )
        .into());
    }
    let output_parent = output_dir
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(output_parent)?;
    fs::create_dir(&output_dir)?;
    let frames_out = output_dir.join("frames");
    fs::create_dir(&frames_out)?;
    fs::write(output_dir.join("camera-profile.json"), &profile_bytes)?;

    let sensor_id = profile["sensor_id"]
        .as_str()
        .ok_or("camera profile lacks sensor ID")?
        .to_owned();
    let mut observations = Vec::with_capacity(EXPECTED_CAPTURE_SECS.len());
    let mut frame_digests = Vec::with_capacity(EXPECTED_CAPTURE_SECS.len());

    for (index, expected_name) in EXPECTED_NAMES.iter().enumerate() {
        let bytes = read_input_frame(&frames_dir, expected_name, expected_frame_len)?;
        let digest = sha256_hex_v1(&bytes);
        let source_name = format!("frame-{:04}.rgba8", index + 1);
        let relative_path = format!("frames/{source_name}");
        let artifact = ArtifactRefV1 {
            relative_path: relative_path.clone(),
            sha256: digest.clone(),
            byte_len: u64::try_from(bytes.len())?,
        };
        verify_artifact_bytes_v1(&artifact, &bytes)?;
        fs::write(output_dir.join(&relative_path), &bytes)?;
        let reloaded = read_verified_artifact_under_root_v1(&output_dir, &artifact)?;
        if reloaded != bytes {
            return Err(format!(
                "exported artifact failed root-confined verification: {relative_path}"
            )
            .into());
        }

        let capture_ns = seconds_to_ns(EXPECTED_CAPTURE_SECS[index])?;
        let uncertainty_bound = BoundedTimeV1 {
            lower_ns: capture_ns,
            upper_ns,
        };
        let capture_interval = CaptureIntervalV1 {
            start: uncertainty_bound,
            end: uncertainty_bound,
        };

        observations.push(AgentObservationV1::VideoFrame(VideoFrameObservationV1 {
            observation_id: format!("room-twin-frame-{:04}", index + 1),
            sensor_id: sensor_id.clone(),
            source_sequence: (index + 1) as u64,
            artifact,
            width,
            height,
            stride_bytes: u32::try_from(stride)?,
            pixel_format: PixelFormatV1::Rgba8,
            color_space: ColorSpaceV1::Srgb,
            color_range: ColorRangeV1::Full,
            camera_profile_sha256: profile_sha256.clone(),
            calibration_profile_sha256: None,
            clock_domain: clock_domain.clone(),
            capture_interval,
        }));
        frame_digests.push(digest);
    }

    let availability_ticks = profile["transport"]["availability_ticks"]
        .as_array()
        .ok_or("camera profile lacks availability ticks")?;
    let mut transport_events = Vec::with_capacity(EXPECTED_CAPTURE_SECS.len());
    for (index, availability_tick) in availability_ticks.iter().enumerate() {
        let sequence = u64::try_from(index + 1)?;
        transport_events.push(TransportEventV1 {
            event_id: format!("room-twin-arrival-{sequence:04}"),
            sensor_id: sensor_id.clone(),
            source_sequence: sequence,
            clock_domain: clock_domain.clone(),
            availability_tick: availability_tick
                .as_u64()
                .ok_or("availability tick is invalid")?,
            arrival_order: sequence,
            disposition: TransportDispositionV1::Delivered,
            observation_id: Some(format!("room-twin-frame-{sequence:04}")),
        });
    }

    let bundle = AgentObservationBundleV1 {
        schema_version: SCHEMA_VERSION_V1,
        stream_id: STREAM_ID.to_owned(),
        observations,
        transport_events,
    };
    validate_bundle_v1(&bundle)?;
    let serialized = serde_json::to_vec_pretty(&bundle)?;
    let bundle_sha256 = sha256_hex_v1(&serialized);
    fs::write(output_dir.join("agent-observations-v1.json"), &serialized)?;

    let mut cursor = ReplayCursorV1::new(&bundle)?;
    let prefix = cursor.advance_to(2)?;
    let prefix_ids: Vec<&str> = prefix
        .iter()
        .filter_map(|delivery| delivery.observation.map(AgentObservationV1::observation_id))
        .collect();
    let prefix_ticks: Vec<u64> = prefix
        .iter()
        .map(|delivery| delivery.transport.availability_tick)
        .collect();
    if prefix_ids != ["room-twin-frame-0001", "room-twin-frame-0002"]
        || prefix_ticks != [1, 2]
        || cursor.pending_event_count() != 1
    {
        return Err(
            "prefix replay did not release only observations 1 and 2 at ticks 1 and 2".into(),
        );
    }
    let reveal = cursor.advance_to(3)?;
    let reveal_ids: Vec<&str> = reveal
        .iter()
        .filter_map(|delivery| delivery.observation.map(AgentObservationV1::observation_id))
        .collect();
    if reveal_ids != ["room-twin-frame-0003"] || cursor.pending_event_count() != 0 {
        return Err("delayed observation 3 did not arrive at availability tick 3".into());
    }

    let report = json!({
        "schema_version": 1,
        "fixture_id": "ROOM-TWIN-01",
        "camera_profile_sha256": profile_sha256,
        "bundle_sha256": bundle_sha256,
        "frame_rgba8_sha256": frame_digests,
        "frame_count": EXPECTED_CAPTURE_SECS.len(),
        "prefix_observation_count_at_tick_2": prefix.len(),
        "pending_observation_count_at_tick_2": 1,
        "reveal_observation_count_at_tick_3": reveal.len(),
        "claim_ceiling": "typed raw-frame agent observation bundle and deterministic #6201 replay-cursor semantics; not blind custody, physical calibration, candidate model correctness or spatial SOTA"
    });
    fs::write(
        output_dir.join("import-report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("room-twin-observation-import: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room_twin_rgb_profile() -> Value {
        json!({
            "schema_version": 1,
            "fixture_id": "ROOM-TWIN-01",
            "camera_profile_id": "room-twin-camera-v1",
            "sensor_id": "camera-front",
            "renderer": "Bevy 0.19.0",
            "pose_delivery": {
                "status": "not_agent_visible",
                "pose_source": "renderer_evaluator_only"
            },
            "image": {
                "width_px": 1280,
                "height_px": 720,
                "pixel_format": "rgba8",
                "channel_order": "rgba",
                "color_space": "srgb",
                "color_range": "full"
            },
            "clock_domain": {
                "domain_id": "room-twin-bevy-real-time",
                "generation": 0,
                "time_scale_id": "monotonic-seconds-since-app-start-v1"
            },
            "capture_time_bounds": {
                "capture_schedule_secs": [1.5, 4.0, 7.0],
                "upper_bound_secs": 23.5,
                "exact_exposure_timestamp_available": false
            },
            "transport": {
                "availability_ticks": [1, 2, 3],
                "prefix_decision_cutoff_tick": 2,
                "first_distinguishing_observation_tick": 3
            },
            "calibration": {
                "calibration_profile_sha256": null,
                "status": "synthetic_un-calibrated"
            }
        })
    }

    #[test]
    fn camera_profile_is_rgb_only_and_rejects_renderer_pose_leakage() {
        let profile = room_twin_rgb_profile();
        let (width, height, stride, clock) =
            validate_camera_profile(&profile).expect("valid RGB-only profile");
        assert_eq!((width, height, stride), (1280, 720, 5_120));
        assert_eq!(clock.domain_id, "room-twin-bevy-real-time");
        assert_eq!(clock.generation, 0);

        let mut leaked = profile;
        leaked["camera_rail"] = json!([]);
        assert!(validate_camera_profile(&leaked).is_err());
    }

    #[test]
    fn capture_seconds_become_integer_nanoseconds_without_clock_guessing() {
        assert_eq!(seconds_to_ns(1.5).unwrap(), 1_500_000_000);
        assert_eq!(seconds_to_ns(4.0).unwrap(), 4_000_000_000);
        assert_eq!(seconds_to_ns(7.0).unwrap(), 7_000_000_000);
        assert!(seconds_to_ns(f64::NAN).is_err());
        assert!(seconds_to_ns(-0.1).is_err());
    }

    #[test]
    fn rgba8_size_uses_four_channels_and_checked_integer_arithmetic() {
        let width = 1280_u32;
        let height = 720_u32;
        let stride = u64::from(width).checked_mul(4).unwrap();
        assert_eq!(stride, 5_120);
        assert_eq!(stride.checked_mul(u64::from(height)).unwrap(), 3_686_400);
    }
}
