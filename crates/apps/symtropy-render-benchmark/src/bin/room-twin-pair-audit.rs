// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Verifies ROOM-TWIN-01 capture receipts and decoded candidate-prefix pixels.
//!
//! The evaluator expects a development-only A/B folder pair. Variant labels
//! and source paths remain outside the candidate observation payload.

use image::DynamicImage;
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

struct DecodedCapture {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    rgba_sha256: String,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn decode_capture(path: &Path) -> Result<DecodedCapture, Box<dyn Error>> {
    let decoded: DynamicImage = image::open(path)?;
    let rgba = decoded.to_rgba8();
    let (width, height) = rgba.dimensions();
    let bytes = rgba.into_raw();
    Ok(DecodedCapture {
        width,
        height,
        rgba_sha256: sha256_hex(&bytes),
        rgba: bytes,
    })
}

fn read_receipt(directory: &Path) -> Result<Value, Box<dyn Error>> {
    let receipt_path = directory.join(format!("{CAPTURE_STEM}.capture-receipt.json"));
    let bytes = fs::read(receipt_path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn validate_receipt(
    receipt: &Value,
    directory: &Path,
) -> Result<Vec<DecodedCapture>, Box<dyn Error>> {
    if receipt["capture_status"] != "complete"
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

    let width = receipt["metadata"]["width_px"]
        .as_u64()
        .ok_or("receipt lacks width_px")? as u32;
    let height = receipt["metadata"]["height_px"]
        .as_u64()
        .ok_or("receipt lacks height_px")? as u32;
    let images = receipt["images"]
        .as_array()
        .ok_or("receipt images field is not an array")?;
    if images.len() != CAPTURE_TIMES.len() {
        return Err(format!("expected three images, found {}", images.len()).into());
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
        let path = directory.join(expected_name);
        let frame = decode_capture(&path)?;
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

    let metadata_bytes = serde_json::to_vec(&receipt_a["metadata"])?;
    let report = json!({
        "schema_version": 1,
        "fixture_id": "ROOM-TWIN-01",
        "camera_id": "room-twin-camera-v1",
        "capture_times_secs": CAPTURE_TIMES,
        "candidate_visible_metadata_sha256": sha256_hex(&metadata_bytes),
        "dimensions_px": [captures_a[0].width, captures_a[0].height],
        "world_a_decoded_rgba_sha256": captures_a.iter().map(|frame| frame.rgba_sha256.as_str()).collect::<Vec<_>>(),
        "world_b_decoded_rgba_sha256": captures_b.iter().map(|frame| frame.rgba_sha256.as_str()).collect::<Vec<_>>(),
        "prefix_byte_equivalence": [prefix_1_equal, prefix_2_equal],
        "within_world_prefix_stability": [within_a_stable, within_b_stable],
        "final_observation_distinguishes_worlds": reveal_differs,
        "result": "DEVELOPMENT_PIXEL_AUDIT_PASS",
        "claim_ceiling": "decoded RGBA8 equality for declared captures only; not hidden-custody, renderer correctness, candidate replay, physical calibration, map accuracy, or model performance"
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
