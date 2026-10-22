// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Receipt-bound visual regression comparison for PNG reference scenes.
//!
//! This tool reports raw RGBA8 code-value differences. It deliberately does
//! not claim perceptual similarity or physical correctness.

use image::{ImageFormat, RgbaImage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, Cursor, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

const MAX_ARTIFACT_BYTES: u64 = 256 * 1024 * 1024;
const MAX_RECEIPT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_IMAGE_PIXELS: u64 = 20_000_000;
const CLAIM_CEILING: &str =
    "receipt-bound-rgba8-difference-metrics-only; not-perceptual-quality-or-physical-validation";

#[derive(Debug)]
struct Options {
    baseline: PathBuf,
    baseline_receipt: PathBuf,
    candidate: PathBuf,
    candidate_receipt: PathBuf,
    output: PathBuf,
    diff_image: Option<PathBuf>,
    pixel_threshold: u8,
    max_mean_channel_delta: f64,
    max_changed_pixel_fraction: f64,
    max_p95_pixel_delta: u8,
    allow_renderer_version_change: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct CaptureReceipt {
    schema_version: u32,
    capture_status: String,
    capture_complete: bool,
    qualification_status: String,
    metadata: CaptureMetadata,
    images: Vec<ReceiptImage>,
}

#[derive(Clone, Debug, Deserialize)]
struct CaptureMetadata {
    source_commit: Option<String>,
    fixture_id: Option<String>,
    fixture_sha256: Option<String>,
    simulation_seed: Option<String>,
    simulation_tick: Option<String>,
    quality_profile: Option<String>,
    camera_id: Option<String>,
    renderer_version: Option<String>,
    render_settings_sha256: Option<String>,
    gpu_adapter: Option<String>,
    gpu_driver: Option<String>,
    render_backend: Option<String>,
    width_px: Option<u32>,
    height_px: Option<u32>,
    operating_system: Option<String>,
    rustc_version: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct ReceiptImage {
    status: String,
    byte_len: Option<u64>,
    sha256: Option<String>,
    width_px: Option<u32>,
    height_px: Option<u32>,
}

#[derive(Debug)]
struct LoadedImage {
    path: PathBuf,
    sha256: String,
    width_px: u32,
    height_px: u32,
    pixels: RgbaImage,
}

#[derive(Clone, Debug, Serialize)]
struct ArtifactSummary {
    path: String,
    sha256: String,
    width_px: u32,
    height_px: u32,
    source_commit: String,
    renderer_version: String,
}

#[derive(Clone, Debug, Serialize)]
struct MatchedContext {
    fixture_id: String,
    fixture_sha256: String,
    simulation_seed: String,
    simulation_tick: String,
    quality_profile: String,
    camera_id: String,
    render_settings_sha256: String,
    gpu_adapter: String,
    gpu_driver: String,
    render_backend: String,
    operating_system: String,
    rustc_version: String,
}

#[derive(Clone, Debug, Serialize)]
struct DifferenceMetrics {
    pixel_count: u64,
    channel_count: u64,
    mean_absolute_channel_delta: f64,
    root_mean_square_channel_delta: f64,
    p95_max_channel_delta: u8,
    changed_pixel_count: u64,
    changed_pixel_fraction: f64,
    pixel_threshold: u8,
}

#[derive(Clone, Debug, Serialize)]
struct DiffImageSummary {
    path: String,
    sha256: String,
    width_px: u32,
    height_px: u32,
    encoding: &'static str,
    legend: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct RegressionBudget {
    max_mean_absolute_channel_delta: f64,
    max_changed_pixel_fraction: f64,
    max_p95_max_channel_delta: u8,
}

#[derive(Clone, Debug, Serialize)]
struct VisualDiffReport {
    schema_version: u32,
    result: &'static str,
    comparison_method: &'static str,
    baseline: ArtifactSummary,
    candidate: ArtifactSummary,
    matched_context: MatchedContext,
    metrics: DifferenceMetrics,
    diff_image: Option<DiffImageSummary>,
    budget: RegressionBudget,
    claim_ceiling: &'static str,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("symtropy-visual-diff: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let options = match parse_options(env::args().skip(1).collect())? {
        Some(options) => options,
        None => return Ok(ExitCode::SUCCESS),
    };
    ensure_outputs_are_distinct(
        &options.output,
        options.diff_image.as_deref(),
        &[
            &options.baseline,
            &options.baseline_receipt,
            &options.candidate,
            &options.candidate_receipt,
        ],
    )?;

    let baseline_receipt = read_receipt(&options.baseline_receipt)?;
    let candidate_receipt = read_receipt(&options.candidate_receipt)?;
    validate_receipt(&baseline_receipt, "baseline")?;
    validate_receipt(&candidate_receipt, "candidate")?;
    let matched_context = validate_context(
        &baseline_receipt.metadata,
        &candidate_receipt.metadata,
        options.allow_renderer_version_change,
    )?;

    let baseline = load_verified_image(&options.baseline, &baseline_receipt)?;
    let candidate = load_verified_image(&options.candidate, &candidate_receipt)?;
    if baseline.width_px != candidate.width_px || baseline.height_px != candidate.height_px {
        return Err(format!(
            "dimension mismatch: baseline is {}x{}, candidate is {}x{}",
            baseline.width_px, baseline.height_px, candidate.width_px, candidate.height_px
        ));
    }

    let metrics = calculate_metrics(&baseline.pixels, &candidate.pixels, options.pixel_threshold)?;
    let diff_image = match options.diff_image.as_deref() {
        Some(path) => Some(write_diff_heatmap(
            path,
            &baseline.pixels,
            &candidate.pixels,
        )?),
        None => None,
    };
    let budget = RegressionBudget {
        max_mean_absolute_channel_delta: options.max_mean_channel_delta,
        max_changed_pixel_fraction: options.max_changed_pixel_fraction,
        max_p95_max_channel_delta: options.max_p95_pixel_delta,
    };
    let within_budget = metrics.mean_absolute_channel_delta
        <= budget.max_mean_absolute_channel_delta
        && metrics.changed_pixel_fraction <= budget.max_changed_pixel_fraction
        && metrics.p95_max_channel_delta <= budget.max_p95_max_channel_delta;

    let report = VisualDiffReport {
        schema_version: 1,
        result: if within_budget {
            "within_budget"
        } else {
            "exceeds_budget"
        },
        comparison_method: "rgba8-srgb-code-value-difference",
        baseline: ArtifactSummary {
            path: baseline.path.display().to_string(),
            sha256: baseline.sha256,
            width_px: baseline.width_px,
            height_px: baseline.height_px,
            source_commit: required(
                &baseline_receipt.metadata.source_commit,
                "baseline.source_commit",
            )?
            .to_owned(),
            renderer_version: required(
                &baseline_receipt.metadata.renderer_version,
                "baseline.renderer_version",
            )?
            .to_owned(),
        },
        candidate: ArtifactSummary {
            path: candidate.path.display().to_string(),
            sha256: candidate.sha256,
            width_px: candidate.width_px,
            height_px: candidate.height_px,
            source_commit: required(
                &candidate_receipt.metadata.source_commit,
                "candidate.source_commit",
            )?
            .to_owned(),
            renderer_version: required(
                &candidate_receipt.metadata.renderer_version,
                "candidate.renderer_version",
            )?
            .to_owned(),
        },
        matched_context,
        metrics,
        diff_image,
        budget,
        claim_ceiling: CLAIM_CEILING,
    };

    write_report(&options.output, &report)?;
    println!(
        "{}: report written to {}",
        report.result,
        options.output.display()
    );

    Ok(if within_budget {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn parse_options(args: Vec<String>) -> Result<Option<Options>, String> {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return Ok(None);
    }

    let mut values = std::collections::BTreeMap::<String, String>::new();
    let mut allow_renderer_version_change = false;
    let mut index = 0;
    while index < args.len() {
        let key = &args[index];
        if key == "--allow-renderer-version-change" {
            if allow_renderer_version_change {
                return Err("duplicate option --allow-renderer-version-change".to_owned());
            }
            allow_renderer_version_change = true;
            index += 1;
            continue;
        }
        if !key.starts_with("--") {
            return Err(format!("unexpected argument: {key}"));
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {key}"))?;
        if value.starts_with("--") {
            return Err(format!("missing value for {key}"));
        }
        if values.insert(key.clone(), value.clone()).is_some() {
            return Err(format!("duplicate option: {key}"));
        }
        index += 2;
    }

    let baseline = required_option(&mut values, "--baseline")?;
    let baseline_receipt = required_option(&mut values, "--baseline-receipt")?;
    let candidate = required_option(&mut values, "--candidate")?;
    let candidate_receipt = required_option(&mut values, "--candidate-receipt")?;
    let output = required_option(&mut values, "--output")?;
    let pixel_threshold = parse_u8(
        required_option(&mut values, "--pixel-threshold")?,
        "--pixel-threshold",
    )?;
    let max_mean_channel_delta = parse_f64(
        required_option(&mut values, "--max-mean-channel-delta")?,
        "--max-mean-channel-delta",
    )?;
    let max_changed_pixel_fraction = parse_f64(
        required_option(&mut values, "--max-changed-pixel-fraction")?,
        "--max-changed-pixel-fraction",
    )?;
    let max_p95_pixel_delta = parse_u8(
        required_option(&mut values, "--max-p95-pixel-delta")?,
        "--max-p95-pixel-delta",
    )?;
    let diff_image = values.remove("--diff-image").map(PathBuf::from);

    if !values.is_empty() {
        return Err(format!(
            "unknown option: {}",
            values.keys().next().expect("map is non-empty")
        ));
    }
    if !(0.0..=255.0).contains(&max_mean_channel_delta) || !max_mean_channel_delta.is_finite() {
        return Err("--max-mean-channel-delta must be finite and within 0..=255".to_owned());
    }
    if !(0.0..=1.0).contains(&max_changed_pixel_fraction) || !max_changed_pixel_fraction.is_finite()
    {
        return Err("--max-changed-pixel-fraction must be within 0..=1".to_owned());
    }

    Ok(Some(Options {
        baseline: PathBuf::from(baseline),
        baseline_receipt: PathBuf::from(baseline_receipt),
        candidate: PathBuf::from(candidate),
        candidate_receipt: PathBuf::from(candidate_receipt),
        output: PathBuf::from(output),
        diff_image,
        pixel_threshold,
        max_mean_channel_delta,
        max_changed_pixel_fraction,
        max_p95_pixel_delta,
        allow_renderer_version_change,
    }))
}

fn print_usage() {
    println!(
        r#"Receipt-bound visual comparison

Usage: symtropy-visual-diff
  --baseline base.png
  --baseline-receipt base.capture-receipt.json
  --candidate next.png
  --candidate-receipt next.capture-receipt.json
  --output report.json
  --pixel-threshold 8
  --max-mean-channel-delta 1.0
  --max-changed-pixel-fraction 0.01
  --max-p95-pixel-delta 8
  [--diff-image diff-heatmap.png]
  [--allow-renderer-version-change]

Exit codes: 0 within budget; 1 exceeds visual-difference budget; 2 invalid or incomparable input.
The optional heatmap visualizes raw per-pixel channel deltas and is not a perceptual-quality score.
The renderer-version-change flag is for explicit migration comparisons; all other declared context must still match."#
    );
}

fn required_option(
    values: &mut std::collections::BTreeMap<String, String>,
    key: &str,
) -> Result<String, String> {
    values
        .remove(key)
        .ok_or_else(|| format!("required option missing: {key}"))
}

fn parse_u8(value: String, key: &str) -> Result<u8, String> {
    value
        .parse::<u8>()
        .map_err(|_| format!("{key} must be an integer in 0..=255"))
}

fn parse_f64(value: String, key: &str) -> Result<f64, String> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| format!("{key} must be a number"))?;
    if !parsed.is_finite() {
        return Err(format!("{key} must be finite"));
    }
    Ok(parsed)
}

fn read_receipt(path: &Path) -> Result<CaptureReceipt, String> {
    let bytes = read_bounded(path, MAX_RECEIPT_BYTES)
        .map_err(|error| format!("cannot read receipt {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid receipt {}: {error}", path.display()))
}

fn validate_receipt(receipt: &CaptureReceipt, label: &str) -> Result<(), String> {
    if receipt.schema_version != 1 {
        return Err(format!(
            "{label} receipt schema {} is unsupported",
            receipt.schema_version
        ));
    }
    if !receipt.capture_complete || receipt.capture_status != "complete" {
        return Err(format!(
            "{label} receipt does not record a complete capture"
        ));
    }
    if receipt.qualification_status != "qualified_metadata" {
        return Err(format!("{label} receipt has unqualified metadata"));
    }
    if receipt.images.is_empty() {
        return Err(format!("{label} receipt has no image records"));
    }
    Ok(())
}

fn validate_context(
    baseline: &CaptureMetadata,
    candidate: &CaptureMetadata,
    allow_renderer_version_change: bool,
) -> Result<MatchedContext, String> {
    let text_fields = [
        ("fixture_id", &baseline.fixture_id, &candidate.fixture_id),
        (
            "fixture_sha256",
            &baseline.fixture_sha256,
            &candidate.fixture_sha256,
        ),
        (
            "simulation_seed",
            &baseline.simulation_seed,
            &candidate.simulation_seed,
        ),
        (
            "simulation_tick",
            &baseline.simulation_tick,
            &candidate.simulation_tick,
        ),
        (
            "quality_profile",
            &baseline.quality_profile,
            &candidate.quality_profile,
        ),
        ("camera_id", &baseline.camera_id, &candidate.camera_id),
        (
            "render_settings_sha256",
            &baseline.render_settings_sha256,
            &candidate.render_settings_sha256,
        ),
        ("gpu_adapter", &baseline.gpu_adapter, &candidate.gpu_adapter),
        ("gpu_driver", &baseline.gpu_driver, &candidate.gpu_driver),
        (
            "render_backend",
            &baseline.render_backend,
            &candidate.render_backend,
        ),
        (
            "operating_system",
            &baseline.operating_system,
            &candidate.operating_system,
        ),
        (
            "rustc_version",
            &baseline.rustc_version,
            &candidate.rustc_version,
        ),
    ];

    for (field, left, right) in text_fields {
        let left = required(left, &format!("baseline.{field}"))?;
        let right = required(right, &format!("candidate.{field}"))?;
        if left != right {
            return Err(format!(
                "incomparable context: {field} differs (baseline={left:?}, candidate={right:?})"
            ));
        }
    }

    let source_commit = required(&baseline.source_commit, "baseline.source_commit")?;
    let candidate_commit = required(&candidate.source_commit, "candidate.source_commit")?;
    if !is_commit_id(source_commit) || !is_commit_id(candidate_commit) {
        return Err("source_commit must be a 40- or 64-character hexadecimal ID".to_owned());
    }
    let renderer_baseline = required(&baseline.renderer_version, "baseline.renderer_version")?;
    let renderer_candidate = required(&candidate.renderer_version, "candidate.renderer_version")?;
    if renderer_baseline != renderer_candidate && !allow_renderer_version_change {
        return Err(format!(
            "renderer_version differs; use --allow-renderer-version-change only for an explicit migration comparison ({renderer_baseline:?} vs {renderer_candidate:?})"
        ));
    }

    let width_baseline = baseline
        .width_px
        .filter(|value| *value > 0)
        .ok_or_else(|| "baseline.width_px is missing or zero".to_owned())?;
    let width_candidate = candidate
        .width_px
        .filter(|value| *value > 0)
        .ok_or_else(|| "candidate.width_px is missing or zero".to_owned())?;
    let height_baseline = baseline
        .height_px
        .filter(|value| *value > 0)
        .ok_or_else(|| "baseline.height_px is missing or zero".to_owned())?;
    let height_candidate = candidate
        .height_px
        .filter(|value| *value > 0)
        .ok_or_else(|| "candidate.height_px is missing or zero".to_owned())?;
    if (width_baseline, height_baseline) != (width_candidate, height_candidate) {
        return Err("incomparable context: declared image dimensions differ".to_owned());
    }

    let fixture_sha = required(&baseline.fixture_sha256, "baseline.fixture_sha256")?;
    let render_settings_sha = required(
        &baseline.render_settings_sha256,
        "baseline.render_settings_sha256",
    )?;
    if !is_sha256(fixture_sha) || !is_sha256(render_settings_sha) {
        return Err(
            "fixture_sha256 and render_settings_sha256 must be 64-character SHA-256 hex digests"
                .to_owned(),
        );
    }

    Ok(MatchedContext {
        fixture_id: required(&baseline.fixture_id, "baseline.fixture_id")?.to_owned(),
        fixture_sha256: fixture_sha.to_owned(),
        simulation_seed: required(&baseline.simulation_seed, "baseline.simulation_seed")?
            .to_owned(),
        simulation_tick: required(&baseline.simulation_tick, "baseline.simulation_tick")?
            .to_owned(),
        quality_profile: required(&baseline.quality_profile, "baseline.quality_profile")?
            .to_owned(),
        camera_id: required(&baseline.camera_id, "baseline.camera_id")?.to_owned(),
        render_settings_sha256: render_settings_sha.to_owned(),
        gpu_adapter: required(&baseline.gpu_adapter, "baseline.gpu_adapter")?.to_owned(),
        gpu_driver: required(&baseline.gpu_driver, "baseline.gpu_driver")?.to_owned(),
        render_backend: required(&baseline.render_backend, "baseline.render_backend")?.to_owned(),
        operating_system: required(&baseline.operating_system, "baseline.operating_system")?
            .to_owned(),
        rustc_version: required(&baseline.rustc_version, "baseline.rustc_version")?.to_owned(),
    })
}

fn load_verified_image(path: &Path, receipt: &CaptureReceipt) -> Result<LoadedImage, String> {
    let bytes = read_bounded(path, MAX_ARTIFACT_BYTES)
        .map_err(|error| format!("cannot read image {}: {error}", path.display()))?;
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    let dimensions =
        image::ImageReader::with_format(Cursor::new(bytes.as_slice()), ImageFormat::Png)
            .into_dimensions()
            .map_err(|error| {
                format!("cannot read PNG dimensions for {}: {error}", path.display())
            })?;
    let (width_px, height_px) = dimensions;
    let pixel_count = u64::from(width_px)
        .checked_mul(u64::from(height_px))
        .ok_or_else(|| format!("image dimensions overflow for {}", path.display()))?;
    if width_px == 0 || height_px == 0 || pixel_count > MAX_IMAGE_PIXELS {
        return Err(format!(
            "image {} dimensions {}x{} exceed the supported pixel budget",
            path.display(),
            width_px,
            height_px
        ));
    }

    let record_matches = receipt.images.iter().any(|record| {
        record.status == "verified_png"
            && record.sha256.as_deref() == Some(sha256.as_str())
            && record.byte_len == Some(bytes.len() as u64)
            && record.width_px == Some(width_px)
            && record.height_px == Some(height_px)
    });
    if !record_matches {
        return Err(format!(
            "image {} is not bound to a verified image record in its receipt",
            path.display()
        ));
    }

    let expected_width = receipt.metadata.width_px;
    let expected_height = receipt.metadata.height_px;
    if expected_width != Some(width_px) || expected_height != Some(height_px) {
        return Err(format!(
            "image {} dimensions do not match the receipt metadata",
            path.display()
        ));
    }

    let decoded = image::load_from_memory_with_format(&bytes, ImageFormat::Png)
        .map_err(|error| format!("cannot decode PNG {}: {error}", path.display()))?
        .to_rgba8();

    Ok(LoadedImage {
        path: path.to_path_buf(),
        sha256,
        width_px,
        height_px,
        pixels: decoded,
    })
}

fn calculate_metrics(
    baseline: &RgbaImage,
    candidate: &RgbaImage,
    pixel_threshold: u8,
) -> Result<DifferenceMetrics, String> {
    if baseline.dimensions() != candidate.dimensions() {
        return Err("pixel buffers must have identical dimensions".to_owned());
    }
    let (width, height) = baseline.dimensions();
    let pixel_count = u64::from(width) * u64::from(height);
    if pixel_count == 0 {
        return Err("cannot compare empty images".to_owned());
    }

    let mut total_absolute_delta = 0_u128;
    let mut total_squared_delta = 0_u128;
    let mut changed_pixel_count = 0_u64;
    let mut pixel_max_deltas = Vec::with_capacity(pixel_count as usize);

    for (left, right) in baseline.pixels().zip(candidate.pixels()) {
        let mut max_delta = 0_u8;
        for channel in 0..4 {
            let delta = left[channel].abs_diff(right[channel]);
            max_delta = max_delta.max(delta);
            total_absolute_delta += u128::from(delta);
            total_squared_delta += u128::from(delta) * u128::from(delta);
        }
        if max_delta > pixel_threshold {
            changed_pixel_count += 1;
        }
        pixel_max_deltas.push(max_delta);
    }

    pixel_max_deltas.sort_unstable();
    let p95_rank = ((pixel_count * 95).div_ceil(100)).max(1) - 1;
    let p95_max_channel_delta = pixel_max_deltas[p95_rank as usize];
    let channel_count = pixel_count * 4;
    let mean_absolute_channel_delta = total_absolute_delta as f64 / channel_count as f64;
    let root_mean_square_channel_delta = (total_squared_delta as f64 / channel_count as f64).sqrt();

    Ok(DifferenceMetrics {
        pixel_count,
        channel_count,
        mean_absolute_channel_delta,
        root_mean_square_channel_delta,
        p95_max_channel_delta,
        changed_pixel_count,
        changed_pixel_fraction: changed_pixel_count as f64 / pixel_count as f64,
        pixel_threshold,
    })
}

fn required<'a>(value: &'a Option<String>, name: &str) -> Result<&'a str, String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("required metadata is missing: {name}"))
}

fn is_commit_id(value: &str) -> bool {
    (value.len() == 40 || value.len() == 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn read_bounded(path: &Path, max_bytes: u64) -> io::Result<Vec<u8>> {
    let file = File::open(path)?;
    if file.metadata()?.len() > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("file exceeds {max_bytes} byte limit"),
        ));
    }
    let mut bytes = Vec::new();
    file.take(max_bytes + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("file grew beyond {max_bytes} byte limit while being read"),
        ));
    }
    Ok(bytes)
}

fn ensure_outputs_are_distinct(
    report_output: &Path,
    diff_output: Option<&Path>,
    inputs: &[&Path],
) -> Result<(), String> {
    let report_path = normalized_destination_path(report_output)?;
    let diff_path = diff_output.map(normalized_destination_path).transpose()?;
    if diff_path.as_ref().is_some_and(|path| path == &report_path) {
        return Err(
            "report JSON and difference heatmap must use different output paths".to_owned(),
        );
    }

    for input in inputs {
        let input_path = fs::canonicalize(input)
            .map_err(|error| format!("cannot resolve input path {}: {error}", input.display()))?;
        if report_path == input_path || diff_path.as_ref().is_some_and(|path| path == &input_path) {
            return Err(format!(
                "output artifacts must not overwrite input artifact {}",
                input.display()
            ));
        }
    }
    Ok(())
}

fn normalized_destination_path(path: &Path) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "cannot create output directory {}: {error}",
            parent.display()
        )
    })?;
    let file_name = path
        .file_name()
        .ok_or_else(|| "output path must name a file".to_owned())?;
    if path.exists() {
        fs::canonicalize(path)
            .map_err(|error| format!("cannot resolve output path {}: {error}", path.display()))
    } else {
        fs::canonicalize(parent)
            .map(|parent| parent.join(file_name))
            .map_err(|error| {
                format!(
                    "cannot resolve output directory {}: {error}",
                    parent.display()
                )
            })
    }
}

fn write_diff_heatmap(
    path: &Path,
    baseline: &RgbaImage,
    candidate: &RgbaImage,
) -> Result<DiffImageSummary, String> {
    let heatmap = build_diff_heatmap(baseline, candidate)?;
    let (width_px, height_px) = heatmap.dimensions();
    let mut cursor = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(heatmap)
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|error| format!("cannot encode difference heatmap: {error}"))?;
    let bytes = cursor.into_inner();
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    write_bytes_atomically(path, &bytes)?;

    Ok(DiffImageSummary {
        path: path.display().to_string(),
        sha256,
        width_px,
        height_px,
        encoding: "PNG",
        legend: "black=no-change; blue/cyan/yellow/red=small-to-large raw RGBA channel delta; numeric pass/fail threshold is recorded separately in JSON metrics",
    })
}

/// Build an inspectable heatmap over the exact pixel-aligned comparison.
/// This is a review aid only: its colors encode raw maximum RGBA code-value
/// deltas, not perceptual salience or correctness.
fn build_diff_heatmap(baseline: &RgbaImage, candidate: &RgbaImage) -> Result<RgbaImage, String> {
    if baseline.dimensions() != candidate.dimensions() {
        return Err("heatmap inputs must have identical dimensions".to_owned());
    }

    let (width, height) = baseline.dimensions();
    let mut heatmap = RgbaImage::new(width, height);
    for (index, (left, right)) in baseline.pixels().zip(candidate.pixels()).enumerate() {
        let delta = (0..4)
            .map(|channel| left[channel].abs_diff(right[channel]))
            .max()
            .unwrap_or(0);
        let color = if delta == 0 {
            [0, 0, 0, 255]
        } else if delta < 64 {
            [0, 0, delta.saturating_mul(4), 255]
        } else if delta < 128 {
            [0, (delta - 64).saturating_mul(4), 255, 255]
        } else if delta < 192 {
            [
                (delta - 128).saturating_mul(4),
                255,
                255_u8.saturating_sub((delta - 128).saturating_mul(4)),
                255,
            ]
        } else {
            [
                255,
                255_u8.saturating_sub((delta - 192).saturating_mul(4)),
                0,
                255,
            ]
        };
        let x = (index as u32) % width.max(1);
        let y = (index as u32) / width.max(1);
        heatmap.put_pixel(x, y, image::Rgba(color));
    }
    Ok(heatmap)
}

fn write_bytes_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "cannot create output directory {}: {error}",
            parent.display()
        )
    })?;
    let file_name = path
        .file_name()
        .ok_or_else(|| "output path must name a file".to_owned())?
        .to_string_lossy()
        .into_owned();
    let absolute_path = fs::canonicalize(parent)
        .map_err(|error| {
            format!(
                "cannot resolve output directory {}: {error}",
                parent.display()
            )
        })?
        .join(file_name);
    let temporary = absolute_path.with_file_name(format!(
        ".{}.{}.tmp",
        absolute_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        std::process::id()
    ));
    let write_result = (|| -> io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);

        #[cfg(windows)]
        if absolute_path.exists() {
            fs::remove_file(&absolute_path)?;
        }
        fs::rename(&temporary, &absolute_path)
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "cannot write artifact {}: {error}",
            absolute_path.display()
        ));
    }
    Ok(())
}

fn write_report(path: &Path, report: &VisualDiffReport) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "cannot create report directory {}: {error}",
            parent.display()
        )
    })?;
    let file_name = path
        .file_name()
        .ok_or_else(|| "report path must name a file".to_owned())?
        .to_string_lossy()
        .into_owned();
    let absolute_path = fs::canonicalize(parent)
        .map_err(|error| {
            format!(
                "cannot resolve report directory {}: {error}",
                parent.display()
            )
        })?
        .join(file_name);
    let report_bytes = serde_json::to_vec_pretty(report)
        .map_err(|error| format!("cannot serialize report JSON: {error}"))?;

    let temporary = absolute_path.with_file_name(format!(
        ".{}.{}.tmp",
        absolute_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        std::process::id()
    ));
    let write_result = (|| -> io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&report_bytes)?;
        file.sync_all()?;
        drop(file);

        #[cfg(windows)]
        if absolute_path.exists() {
            fs::remove_file(&absolute_path)?;
        }
        fs::rename(&temporary, &absolute_path)
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "cannot write report {}: {error}",
            absolute_path.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixels(values: &[[u8; 4]]) -> RgbaImage {
        RgbaImage::from_raw(
            values.len() as u32,
            1,
            values.iter().flatten().copied().collect(),
        )
        .expect("test pixel data should form an image")
    }

    #[test]
    fn identical_images_have_zero_difference() {
        let image = pixels(&[[10, 20, 30, 255], [200, 100, 40, 255]]);
        let metrics = calculate_metrics(&image, &image, 8).expect("metrics should calculate");
        assert_eq!(metrics.mean_absolute_channel_delta, 0.0);
        assert_eq!(metrics.root_mean_square_channel_delta, 0.0);
        assert_eq!(metrics.p95_max_channel_delta, 0);
        assert_eq!(metrics.changed_pixel_count, 0);
        assert_eq!(metrics.changed_pixel_fraction, 0.0);
    }

    #[test]
    fn one_changed_pixel_is_counted_and_measured() {
        let baseline = pixels(&[[0, 0, 0, 255], [0, 0, 0, 255]]);
        let candidate = pixels(&[[0, 0, 0, 255], [32, 0, 0, 255]]);
        let metrics =
            calculate_metrics(&baseline, &candidate, 8).expect("metrics should calculate");
        assert_eq!(metrics.pixel_count, 2);
        assert_eq!(metrics.channel_count, 8);
        assert_eq!(metrics.changed_pixel_count, 1);
        assert_eq!(metrics.changed_pixel_fraction, 0.5);
        assert_eq!(metrics.mean_absolute_channel_delta, 4.0);
    }

    #[test]
    fn identical_images_produce_black_heatmap() {
        let image = pixels(&[[10, 20, 30, 255], [200, 100, 40, 255]]);
        let heatmap = build_diff_heatmap(&image, &image).expect("heatmap should build");
        assert_eq!(heatmap.get_pixel(0, 0).0, [0, 0, 0, 255]);
        assert_eq!(heatmap.get_pixel(1, 0).0, [0, 0, 0, 255]);
    }

    #[test]
    fn changed_pixels_produce_heatmap_with_delta_intensity() {
        let baseline = pixels(&[[0, 0, 0, 255], [0, 0, 0, 255], [0, 0, 0, 255]]);
        let candidate = pixels(&[[0, 0, 0, 255], [16, 0, 0, 255], [220, 0, 0, 255]]);
        let heatmap = build_diff_heatmap(&baseline, &candidate).expect("heatmap should build");
        assert_eq!(heatmap.get_pixel(0, 0).0, [0, 0, 0, 255]);
        assert_ne!(heatmap.get_pixel(1, 0).0, heatmap.get_pixel(2, 0).0);
        assert_eq!(heatmap.get_pixel(2, 0).0[0], 255);
        assert_eq!(heatmap.get_pixel(2, 0).0[3], 255);
    }

    #[test]
    fn incompatible_dimensions_are_rejected() {
        let baseline = pixels(&[[0, 0, 0, 255]]);
        let candidate = pixels(&[[0, 0, 0, 255], [1, 1, 1, 255]]);
        assert!(calculate_metrics(&baseline, &candidate, 8).is_err());
    }

    fn qualified_context() -> CaptureMetadata {
        CaptureMetadata {
            source_commit: Some("a".repeat(40)),
            fixture_id: Some("SCENE-LIGHT-01".to_owned()),
            fixture_sha256: Some("b".repeat(64)),
            simulation_seed: Some("7".to_owned()),
            simulation_tick: Some("120".to_owned()),
            quality_profile: Some("balanced-1080p".to_owned()),
            camera_id: Some("main-camera".to_owned()),
            renderer_version: Some("bevy-0.19.0".to_owned()),
            render_settings_sha256: Some("c".repeat(64)),
            gpu_adapter: Some("reference-gpu".to_owned()),
            gpu_driver: Some("driver-1".to_owned()),
            render_backend: Some("vulkan".to_owned()),
            width_px: Some(1920),
            height_px: Some(1080),
            operating_system: Some("linux".to_owned()),
            rustc_version: Some("rustc 1.96.0".to_owned()),
        }
    }

    #[test]
    fn equivalent_receipt_context_is_comparable() {
        let metadata = qualified_context();
        assert!(validate_context(&metadata, &metadata, false).is_ok());
    }

    #[test]
    fn camera_or_settings_mismatch_is_rejected() {
        let baseline = qualified_context();
        let mut candidate = baseline.clone();
        candidate.camera_id = Some("debug-camera".to_owned());
        let error = validate_context(&baseline, &candidate, false)
            .expect_err("camera mismatch must fail closed");
        assert!(error.contains("camera_id"));
    }

    #[test]
    fn renderer_version_change_requires_explicit_opt_in() {
        let baseline = qualified_context();
        let mut candidate = baseline.clone();
        candidate.renderer_version = Some("bevy-0.20.0".to_owned());
        assert!(validate_context(&baseline, &candidate, false).is_err());
        assert!(validate_context(&baseline, &candidate, true).is_ok());
    }

    #[test]
    fn commit_and_sha_digests_require_hexadecimal_shape() {
        assert!(is_commit_id(&"a".repeat(40)));
        assert!(is_commit_id(&"b".repeat(64)));
        assert!(!is_commit_id("not-a-commit"));
        assert!(is_sha256(&"c".repeat(64)));
        assert!(!is_sha256(&"f".repeat(63)));
    }
}
