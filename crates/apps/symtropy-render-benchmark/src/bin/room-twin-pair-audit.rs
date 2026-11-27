// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Verifies ROOM-TWIN-01 capture receipts, decoded RGBA8 equality and exports
//! the same verified pixels as raw files for the #6201 observation adapter.
//!
//! The public A/B output folders are evaluator-side development artifacts.
//! Their names and truth crosswalk must not be mounted into a blind candidate.

use image::{DynamicImage, ImageFormat};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

const CAPTURE_TIMES: [f64; 3] = [1.5, 4.0, 7.0];
const CAPTURE_STEM: &str = "room-twin";
const CAMERA_PROFILE_PATH: &str = "benchmarks/render/ROOM-TWIN-01.camera-profile.json";
const MAX_PNG_BYTES: u64 = 256 * 1024 * 1024;
const MAX_IMAGE_PIXELS: u64 = 20_000_000;
const MAX_PROFILE_BYTES: u64 = 1024 * 1024;

struct DecodedCapture {
    width: u32,
    height: u32,
    png_byte_len: u64,
    png_sha256: String,
    rgba: Vec<u8>,
    rgba_sha256: String,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn png_receipt_matches_bytes(receipt_image: &Value, frame: &DecodedCapture) -> bool {
    receipt_image["byte_len"].as_u64() == Some(frame.png_byte_len)
        && receipt_image["sha256"].as_str() == Some(frame.png_sha256.as_str())
}

fn decode_capture(path: &Path) -> Result<DecodedCapture, Box<dyn Error>> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_PNG_BYTES {
        return Err(format!("PNG exceeds {} byte limit", MAX_PNG_BYTES).into());
    }

    let png_bytes = fs::read(path)?;
    let png_byte_len = u64::try_from(png_bytes.len())?;
    if png_byte_len != metadata.len() {
        return Err("PNG size changed while reading".into());
    }
    if png_byte_len > MAX_PNG_BYTES {
        return Err(format!("PNG exceeds {} byte limit", MAX_PNG_BYTES).into());
    }

    let png_sha256 = sha256_hex(&png_bytes);
    let decoded: DynamicImage = image::load_from_memory_with_format(&png_bytes, ImageFormat::Png)?;
    let rgba = decoded.to_rgba8();
    let (width, height) = rgba.dimensions();
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or("decoded PNG pixel count overflow")?;
    if width == 0 || height == 0 || pixels > MAX_IMAGE_PIXELS {
        return Err(format!("decoded PNG exceeds {MAX_IMAGE_PIXELS} pixel limit").into());
    }
    let bytes = rgba.into_raw();

    Ok(DecodedCapture {
        width,
        height,
        png_byte_len,
        png_sha256,
        rgba_sha256: sha256_hex(&bytes),
        rgba: bytes,
    })
}

fn read_receipt(directory: &Path) -> Result<Value, Box<dyn Error>> {
    let receipt_path = directory.join(format!("{CAPTURE_STEM}.capture-receipt.json"));
    let bytes = fs::read(receipt_path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn read_and_validate_camera_profile(
    root: &Path,
    metadata: &Value,
) -> Result<(Value, String), Box<dyn Error>> {
    let path = root.join(CAMERA_PROFILE_PATH);
    let size = fs::metadata(&path)?.len();
    if size > MAX_PROFILE_BYTES {
        return Err("camera profile exceeds 1 MiB limit".into());
    }
    let bytes = fs::read(path)?;
    if u64::try_from(bytes.len())? != size {
        return Err("camera profile changed while reading".into());
    }
    let digest = sha256_hex(&bytes);
    let profile: Value = serde_json::from_slice(&bytes)?;

    if profile["schema_version"] != 1
        || profile["fixture_id"] != "ROOM-TWIN-01"
        || profile["camera_profile_id"] != "room-twin-camera-v1"
        || profile["sensor_id"] != "camera-front"
        || profile["renderer"] != "Bevy 0.19.0"
        || profile["pose_delivery"]["status"] != "not_agent_visible"
        || profile["pose_delivery"]["pose_source"] != "renderer_evaluator_only"
        || profile.get("camera_rail").is_some()
        || profile["image"]["width_px"] != metadata["width_px"]
        || profile["image"]["height_px"] != metadata["height_px"]
        || profile["image"]["pixel_format"] != "rgba8"
        || profile["image"]["color_space"] != "srgb"
        || profile["image"]["color_range"] != "full"
        || profile["clock_domain"]["domain_id"] != "room-twin-bevy-real-time"
        || profile["clock_domain"]["generation"] != 0
        || profile["clock_domain"]["time_scale_id"]
            != "monotonic-seconds-since-app-start-v1"
        || profile["transport"]["availability_ticks"] != json!([1, 2, 3])
        || profile["transport"]["prefix_decision_cutoff_tick"] != 2
        || profile["transport"]["first_distinguishing_observation_tick"] != 3
        || profile["capture_time_bounds"]["upper_bound_secs"] != 23.5
        || profile["capture_time_bounds"]["exact_exposure_timestamp_available"] != false
        || profile["capture_time_bounds"]["lower_bound_semantics"] != "scheduled screenshot request time"
        || profile["calibration"]["calibration_profile_sha256"] != Value::Null
        || profile["calibration"]["status"] != "synthetic_un-calibrated"
    {
        return Err("camera profile violates the frozen ROOM-TWIN-01 sensor contract".into());
    }

    let profile_times = profile["capture_time_bounds"]["capture_schedule_secs"]
        .as_array()
        .ok_or("camera profile lacks capture schedule")?;
    if profile_times.len() != CAPTURE_TIMES.len()
        || profile_times
            .iter()
            .zip(CAPTURE_TIMES)
            .any(|(actual, expected)| actual.as_f64().is_none_or(|value| (value - expected).abs() > 0.01))
    {
        return Err("camera profile capture times do not match the audit schedule".into());
    }

    let renderer_version = metadata["renderer_version"]
        .as_str()
        .ok_or("capture receipt lacks renderer version")?;
    if renderer_version != "bevy-0.19.0" {
        return Err("capture receipt renderer version differs from the camera profile".into());
    }

    Ok((profile, digest))
}

fn read_and_validate_run_settings(
    run_root: &Path,
    metadata: &Value,
    camera_profile_sha256: &str,
) -> Result<(Value, String, String), Box<dyn Error>> {
    let settings_path = run_root.join("render-settings.json");
    let metadata_size = fs::metadata(&settings_path)?.len();
    if metadata_size > MAX_PROFILE_BYTES {
        return Err("render settings exceed 1 MiB limit".into());
    }
    let settings_bytes = fs::read(&settings_path)?;
    if u64::try_from(settings_bytes.len())? != metadata_size {
        return Err("render settings changed while reading".into());
    }
    let settings_sha256 = sha256_hex(&settings_bytes);
    let settings: Value = serde_json::from_slice(&settings_bytes)?;
    if settings["camera_id"] != "room-twin-camera-v1"
        || settings["camera_profile_sha256"].as_str() != Some(camera_profile_sha256)
        || settings["renderer"] != "bevy-0.19.0"
        || settings["width_px"] != metadata["width_px"]
        || settings["height_px"] != metadata["height_px"]
        || settings["capture_times_secs"] != json!([1.5, 4.0, 7.0])
        || settings["shadows_enabled"] != false
    {
        return Err("render settings do not bind the expected ROOM-TWIN camera profile and capture contract".into());
    }
    if metadata["render_settings_sha256"].as_str() != Some(settings_sha256.as_str()) {
        return Err("capture receipt render-settings digest does not match exact settings bytes".into());
    }

    let fixture_path = env::current_dir()?.join("benchmarks/render/ROOM-TWIN-01.json");
    let fixture_size = fs::metadata(&fixture_path)?.len();
    if fixture_size > MAX_PROFILE_BYTES {
        return Err("ROOM-TWIN fixture exceeds 1 MiB limit".into());
    }
    let fixture_bytes = fs::read(fixture_path)?;
    if u64::try_from(fixture_bytes.len())? != fixture_size {
        return Err("ROOM-TWIN fixture changed while reading".into());
    }
    let fixture_sha256 = sha256_hex(&fixture_bytes);
    if metadata["fixture_sha256"].as_str() != Some(fixture_sha256.as_str()) {
        return Err("capture receipt fixture digest does not match exact fixture bytes".into());
    }

    Ok((settings, settings_sha256, fixture_sha256))
}

fn validate_receipt(
    receipt: &Value,
    directory: &Path,
) -> Result<Vec<DecodedCapture>, Box<dyn Error>> {
    if receipt["schema_version"] != 1
        || receipt["capture_status"] != "complete"
        || receipt["capture_complete"] != true
        || receipt["qualification_status"] != "qualified_metadata"
        || receipt["expected_image_count"].as_u64() != Some(CAPTURE_TIMES.len() as u64)
        || receipt["capture_stem"].as_str() != Some(CAPTURE_STEM)
    {
        return Err(format!(
            "capture receipt is incomplete or unqualified in {}",
            directory.display()
        )
        .into());
    }

    if receipt["metadata"]["fixture_id"].as_str() != Some("ROOM-TWIN-01")
        || receipt["metadata"]["camera_id"].as_str() != Some("room-twin-camera-v1")
    {
        return Err(format!(
            "capture receipt has the wrong fixture/camera identity in {}",
            directory.display()
        )
        .into());
    }

    let width = u32::try_from(
        receipt["metadata"]["width_px"]
            .as_u64()
            .ok_or("receipt lacks width_px")?,
    )?;
    let height = u32::try_from(
        receipt["metadata"]["height_px"]
            .as_u64()
            .ok_or("receipt lacks height_px")?,
    )?;
    let images = receipt["images"]
        .as_array()
        .ok_or("receipt images field is not an array")?;
    if images.len() != CAPTURE_TIMES.len() {
        return Err(format!(
            "expected three images, found {}",
            images.len()
        )
        .into());
    }

    let mut decoded = Vec::with_capacity(CAPTURE_TIMES.len());
    for (index, (image, expected_time)) in images.iter().zip(CAPTURE_TIMES).enumerate() {
        if image["status"].as_str() != Some("verified_png") {
            return Err(format!("capture {index} is not a verified PNG").into());
        }
        let actual_time = image["scheduled_time_secs"]
            .as_f64()
            .ok_or("capture lacks scheduled_time_secs")?;
        if (actual_time - expected_time).abs() > 0.01 {
            return Err(format!(
                "capture {index} scheduled at {actual_time}, expected {expected_time}"
            )
            .into());
        }

        let expected_name = format!("{CAPTURE_STEM}_t{expected_time:.1}.png");
        let receipt_path = image["path"]
            .as_str()
            .ok_or("capture receipt lacks image path")?;
        if Path::new(receipt_path)
            .file_name()
            .and_then(|name| name.to_str())
            != Some(expected_name.as_str())
        {
            return Err(
                format!("capture {index} receipt path does not match {expected_name}").into(),
            );
        }
        let path = directory.join(&expected_name);
        let frame = decode_capture(&path)?;
        if !png_receipt_matches_bytes(image, &frame) {
            return Err(format!(
                "capture {index} PNG bytes do not match the receipt digest/length"
            )
            .into());
        }
        if frame.width != width || frame.height != height {
            return Err(format!(
                "capture {index} dimensions {}x{} differ from receipt {}x{}",
                frame.width, frame.height, width, height
            )
            .into());
        }
        if image["width_px"].as_u64() != Some(u64::from(width))
            || image["height_px"].as_u64() != Some(u64::from(height))
        {
            return Err(
                format!("capture {index} decoded dimensions differ from receipt fields").into(),
            );
        }
        decoded.push(frame);
    }
    Ok(decoded)
}

fn export_raw_rgba(directory: &Path, frames: &[DecodedCapture]) -> Result<Vec<Value>, Box<dyn Error>> {
    let export_dir = directory.join("decoded-rgba8");
    fs::create_dir(&export_dir)?;
    let mut exports = Vec::with_capacity(frames.len());
    for (index, frame) in frames.iter().enumerate() {
        let name = format!("frame-{:04}.rgba8", index + 1);
        let path = export_dir.join(&name);
        fs::write(&path, &frame.rgba)?;
        exports.push(json!({
            "relative_path": format!("decoded-rgba8/{name}"),
            "byte_len": frame.rgba.len(),
            "sha256": frame.rgba_sha256,
            "width_px": frame.width,
            "height_px": frame.height,
            "pixel_format": "rgba8",
            "color_space": "srgb",
            "color_range": "full"
        }));
    }
    Ok(exports)
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    let world_a = PathBuf::from(
        args.next()
            .ok_or("usage: room-twin-pair-audit <world-a-dir> <world-b-dir>")?,
    );
    let world_b = PathBuf::from(
        args.next()
            .ok_or("usage: room-twin-pair-audit <world-a-dir> <world-b-dir>")?,
    );
    if args.next().is_some() {
        return Err("too many arguments".into());
    }

    let receipt_a = read_receipt(&world_a)?;
    let receipt_b = read_receipt(&world_b)?;
    if receipt_a["metadata"] != receipt_b["metadata"] {
        return Err("candidate-visible capture metadata differs between world variants".into());
    }
    if receipt_a["expected_image_count"] != receipt_b["expected_image_count"] {
        return Err("world variants have different expected image counts".into());
    }

    let root_a = world_a
        .parent()
        .ok_or("world A output directory lacks a run root")?
        .canonicalize()?;
    let root_b = world_b
        .parent()
        .ok_or("world B output directory lacks a run root")?
        .canonicalize()?;
    if root_a != root_b {
        return Err("world A and B captures do not share one settings/output root".into());
    }
    let repository_root = env::current_dir()?;
    let (camera_profile, camera_profile_sha256) =
        read_and_validate_camera_profile(&repository_root, &receipt_a["metadata"])?;
    let (_settings, settings_sha256, fixture_sha256) =
        read_and_validate_run_settings(&root_a, &receipt_a["metadata"], &camera_profile_sha256)?;

    let captures_a = validate_receipt(&receipt_a, &world_a)?;
    let captures_b = validate_receipt(&receipt_b, &world_b)?;
    let prefix_1_equal = captures_a[0].rgba == captures_b[0].rgba;
    let prefix_2_equal = captures_a[1].rgba == captures_b[1].rgba;
    let within_a_stable = captures_a[0].rgba == captures_a[1].rgba;
    let within_b_stable = captures_b[0].rgba == captures_b[1].rgba;
    let reveal_differs = captures_a[2].rgba != captures_b[2].rgba;

    if !(prefix_1_equal && prefix_2_equal && within_a_stable && within_b_stable && reveal_differs) {
        return Err(format!(
            "ROOM-TWIN-01 pixel audit failed: prefix1={prefix_1_equal}, prefix2={prefix_2_equal}, \\
             stableA={within_a_stable}, stableB={within_b_stable}, revealDiffers={reveal_differs}"
        )
        .into());
    }

    let exports_a = export_raw_rgba(&world_a, &captures_a)?;
    let exports_b = export_raw_rgba(&world_b, &captures_b)?;

    let metadata_bytes = serde_json::to_vec(&receipt_a["metadata"])?;
    let report = json!({
        "schema_version": 1,
        "fixture_id": "ROOM-TWIN-01",
        "camera_id": "room-twin-camera-v1",
        "camera_profile_path": CAMERA_PROFILE_PATH,
        "camera_profile_sha256": camera_profile_sha256,
        "render_settings_sha256": settings_sha256,
        "fixture_sha256": fixture_sha256,
        "camera_profile_claim_ceiling": camera_profile["calibration"]["status"],
        "capture_times_secs": CAPTURE_TIMES,
        "candidate_visible_metadata_sha256": sha256_hex(&metadata_bytes),
        "dimensions_px": [captures_a[0].width, captures_a[0].height],
        "world_a_decoded_rgba_sha256": captures_a.iter().map(|frame| frame.rgba_sha256.as_str()).collect::<Vec<_>>(),
        "world_b_decoded_rgba_sha256": captures_b.iter().map(|frame| frame.rgba_sha256.as_str()).collect::<Vec<_>>(),
        "world_a_raw_rgba_exports": exports_a,
        "world_b_raw_rgba_exports": exports_b,
        "prefix_byte_equivalence": [prefix_1_equal, prefix_2_equal],
        "within_world_prefix_stability": [within_a_stable, within_b_stable],
        "final_observation_distinguishes_worlds": reveal_differs,
        "result": "DEVELOPMENT_PIXEL_AUDIT_PASS",
        "claim_ceiling": "decoded RGBA8 equality and raw-frame export for declared captures only; not hidden-custody, renderer correctness, candidate replay, physical calibration, map accuracy, or model performance"
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("room-twin-pair-audit: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_the_standard_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn receipt_rejects_changed_png_digest_or_byte_length() {
        let frame = DecodedCapture {
            width: 1,
            height: 1,
            png_byte_len: 3,
            png_sha256: sha256_hex(b"abc"),
            rgba: vec![0, 0, 0, 255],
            rgba_sha256: sha256_hex(&[0, 0, 0, 255]),
        };
        let valid = json!({
            "byte_len": 3,
            "sha256": frame.png_sha256
        });
        assert!(png_receipt_matches_bytes(&valid, &frame));

        let bad_digest = json!({
            "byte_len": 3,
            "sha256": "0".repeat(64)
        });
        assert!(!png_receipt_matches_bytes(&bad_digest, &frame));

        let bad_length = json!({
            "byte_len": 4,
            "sha256": frame.png_sha256
        });
        assert!(!png_receipt_matches_bytes(&bad_length, &frame));
    }
}
