// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Receipt-oriented frame-time scaling summary for SCENE-LIGHT-01.
//!
//! The summarizer refuses to combine reports from different hardware, renderer,
//! fixture, camera, workload profile, resolution, source revision, or toolchain.
//! Per-run render-settings digests may differ only because object count varies.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    env,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

const MAX_REPORT_BYTES: u64 = 4 * 1024 * 1024;
const MIN_RUNS: usize = 2;
const CLAIM_CEILING: &str =
    "same-context-real-frame-interval-scaling-summary-only; not-gpu-timing-or-physical-validation";

#[derive(Clone, Debug, Deserialize)]
struct FrameTimeReport {
    schema_version: u32,
    fixture_id: String,
    camera_id: String,
    renderer: String,
    profile: String,
    object_count: usize,
    shadows_enabled: bool,
    width_px: u32,
    height_px: u32,
    source_commit: Option<String>,
    fixture_sha256: Option<String>,
    render_settings_sha256: Option<String>,
    gpu_adapter: Option<String>,
    gpu_driver: Option<String>,
    render_backend: Option<String>,
    rustc_version: Option<String>,
    operating_system: Option<String>,
    sampling_method: String,
    sample_window_start_secs: f32,
    sample_window_end_secs: f32,
    sample_count: usize,
    min_required_samples: usize,
    mean_frame_interval_ms: Option<f64>,
    p50_frame_interval_ms: Option<f64>,
    p95_frame_interval_ms: Option<f64>,
    p99_frame_interval_ms: Option<f64>,
    over_16_67_ms_fraction: Option<f64>,
    over_33_3_ms_fraction: Option<f64>,
    measurement_status: String,
    claim_ceiling: String,
}

#[derive(Clone, Debug, Serialize)]
struct SweepPoint {
    input_path: String,
    input_report_sha256: String,
    object_count: usize,
    sample_count: usize,
    mean_frame_interval_ms: f64,
    p50_frame_interval_ms: f64,
    p95_frame_interval_ms: f64,
    p99_frame_interval_ms: f64,
    fraction_over_16_67_ms: f64,
    fraction_over_33_3_ms: f64,
    p95_relative_to_smallest_workload: Option<f64>,
    p95_within_60_fps_budget: bool,
    p95_within_30_fps_budget: bool,
    render_settings_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
struct SweepContext {
    fixture_id: String,
    fixture_sha256: String,
    camera_id: String,
    renderer: String,
    source_commit: String,
    profile: String,
    shadows_enabled: bool,
    width_px: u32,
    height_px: u32,
    gpu_adapter: String,
    gpu_driver: String,
    render_backend: String,
    rustc_version: String,
    operating_system: String,
    sampling_method: String,
    sample_window_start_secs: f32,
    sample_window_end_secs: f32,
}

#[derive(Clone, Debug)]
struct LoadedReport {
    path: PathBuf,
    sha256: String,
    report: FrameTimeReport,
}

#[derive(Clone, Debug, Serialize)]
struct SweepSummary {
    schema_version: u32,
    result: &'static str,
    comparison_method: &'static str,
    context: SweepContext,
    minimum_run_count: usize,
    run_count: usize,
    smallest_workload_object_count: usize,
    points: Vec<SweepPoint>,
    claim_ceiling: &'static str,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("symtropy-render-sweep-report: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let output = args.next().ok_or_else(|| {
        "usage: symtropy-render-sweep-report <output.json> <run-1.json> <run-2.json> [...]"
            .to_owned()
    })?;
    let input_paths: Vec<PathBuf> = args.map(PathBuf::from).collect();
    if input_paths.len() < MIN_RUNS {
        return Err(format!("at least {MIN_RUNS} input reports are required"));
    }
    let output_path = PathBuf::from(output);

    ensure_output_is_distinct(&output_path, &input_paths)?;
    let mut reports = input_paths
        .iter()
        .map(|path| read_report(path))
        .collect::<Result<Vec<_>, _>>()?;
    for loaded in &reports {
        validate_report(&loaded.report, &loaded.path)?;
    }
    reports.sort_by_key(|loaded| loaded.report.object_count);
    let comparable_reports: Vec<FrameTimeReport> =
        reports.iter().map(|loaded| loaded.report.clone()).collect();
    validate_comparable_context(&comparable_reports)?;

    let object_counts: BTreeSet<usize> = reports
        .iter()
        .map(|loaded| loaded.report.object_count)
        .collect();
    if object_counts.len() != reports.len() {
        return Err("duplicate object_count reports are not allowed in one sweep".to_owned());
    }

    let smallest = reports
        .first()
        .map(|loaded| &loaded.report)
        .ok_or_else(|| "no reports after validation".to_owned())?;
    let baseline_p95 = required_number(
        smallest.p95_frame_interval_ms,
        &smallest.object_count,
        "p95_frame_interval_ms",
    )?;
    let context = make_context(smallest)?;

    let mut points = Vec::with_capacity(reports.len());
    for loaded in &reports {
        let report = &loaded.report;
        let p95 = required_number(
            report.p95_frame_interval_ms,
            &report.object_count,
            "p95_frame_interval_ms",
        )?;
        points.push(SweepPoint {
            input_path: loaded.path.display().to_string(),
            input_report_sha256: loaded.sha256.clone(),
            object_count: report.object_count,
            sample_count: report.sample_count,
            mean_frame_interval_ms: required_number(
                report.mean_frame_interval_ms,
                &report.object_count,
                "mean_frame_interval_ms",
            )?,
            p50_frame_interval_ms: required_number(
                report.p50_frame_interval_ms,
                &report.object_count,
                "p50_frame_interval_ms",
            )?,
            p95_frame_interval_ms: p95,
            p99_frame_interval_ms: required_number(
                report.p99_frame_interval_ms,
                &report.object_count,
                "p99_frame_interval_ms",
            )?,
            fraction_over_16_67_ms: required_number(
                report.over_16_67_ms_fraction,
                &report.object_count,
                "over_16_67_ms_fraction",
            )?,
            fraction_over_33_3_ms: required_number(
                report.over_33_3_ms_fraction,
                &report.object_count,
                "over_33_3_ms_fraction",
            )?,
            p95_relative_to_smallest_workload: if baseline_p95 > f64::EPSILON {
                Some(p95 / baseline_p95)
            } else {
                None
            },
            p95_within_60_fps_budget: p95 <= 16.67,
            p95_within_30_fps_budget: p95 <= 33.3,
            render_settings_sha256: required_string(
                &report.render_settings_sha256,
                &report.object_count,
                "render_settings_sha256",
            )?
            .to_owned(),
        });
    }

    let summary = SweepSummary {
        schema_version: 1,
        result: "comparable_scaling_reports",
        comparison_method: "sorted-same-context-bevy-real-frame-intervals",
        context,
        minimum_run_count: MIN_RUNS,
        run_count: points.len(),
        smallest_workload_object_count: smallest.object_count,
        points,
        claim_ceiling: CLAIM_CEILING,
    };
    write_json_atomically(&output_path, &summary)?;
    println!(
        "wrote {} comparable workload points to {}",
        summary.run_count,
        output_path.display()
    );
    Ok(())
}

fn read_report(path: &Path) -> Result<LoadedReport, String> {
    let file =
        File::open(path).map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let length = file
        .metadata()
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?
        .len();
    if length > MAX_REPORT_BYTES {
        return Err(format!(
            "{} exceeds {MAX_REPORT_BYTES} bytes",
            path.display()
        ));
    }
    let mut bytes = Vec::with_capacity(length as usize);
    file.take(MAX_REPORT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if bytes.len() as u64 > MAX_REPORT_BYTES {
        return Err(format!("{} grew beyond the size limit", path.display()));
    }
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    let report = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid JSON report {}: {error}", path.display()))?;
    Ok(LoadedReport {
        path: path.to_path_buf(),
        sha256,
        report,
    })
}

fn validate_report(report: &FrameTimeReport, path: &Path) -> Result<(), String> {
    if report.schema_version != 1 {
        return Err(format!(
            "{}: unsupported schema version {}",
            path.display(),
            report.schema_version
        ));
    }
    if report.fixture_id != "SCENE-LIGHT-01" {
        return Err(format!(
            "{}: unexpected fixture_id {:?}",
            path.display(),
            report.fixture_id
        ));
    }
    if report.measurement_status != "measured_metadata_qualified" {
        return Err(format!(
            "{}: measurement_status must be measured_metadata_qualified, got {:?}",
            path.display(),
            report.measurement_status
        ));
    }
    if report.claim_ceiling
        != "observed-real-frame-intervals-and-caller-declared-metadata-only; not-gpu-timestamp-or-visual-or-physical-validation"
    {
        return Err(format!(
            "{}: unexpected measurement claim ceiling",
            path.display()
        ));
    }
    if report.sample_count < report.min_required_samples || report.sample_count < 30 {
        return Err(format!("{}: insufficient frame samples", path.display()));
    }
    if report.object_count == 0 || report.width_px == 0 || report.height_px == 0 {
        return Err(format!(
            "{}: object count and dimensions must be nonzero",
            path.display()
        ));
    }
    if report.object_count > 10_000 {
        return Err(format!(
            "{}: object_count exceeds the supported 10000-object fixture limit",
            path.display()
        ));
    }
    for (name, value) in [
        ("camera_id", report.camera_id.as_str()),
        ("renderer", report.renderer.as_str()),
        ("profile", report.profile.as_str()),
        ("sampling_method", report.sampling_method.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!(
                "{}: required context field {name} is empty",
                path.display()
            ));
        }
    }
    for (name, value) in [
        ("source_commit", report.source_commit.as_deref()),
        ("fixture_sha256", report.fixture_sha256.as_deref()),
        (
            "render_settings_sha256",
            report.render_settings_sha256.as_deref(),
        ),
        ("gpu_adapter", report.gpu_adapter.as_deref()),
        ("gpu_driver", report.gpu_driver.as_deref()),
        ("render_backend", report.render_backend.as_deref()),
        ("rustc_version", report.rustc_version.as_deref()),
        ("operating_system", report.operating_system.as_deref()),
    ] {
        let value = value
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("{}: required metadata {name} is absent", path.display()))?;
        if name.ends_with("sha256") && !is_sha256(value) {
            return Err(format!(
                "{}: {name} is not a SHA-256 hex digest",
                path.display()
            ));
        }
        if name == "source_commit" && !is_commit_id(value) {
            return Err(format!(
                "{}: source_commit must be 40 or 64 hexadecimal characters",
                path.display()
            ));
        }
    }
    required_number(
        report.mean_frame_interval_ms,
        &report.object_count,
        "mean_frame_interval_ms",
    )?;
    let p50 = required_number(
        report.p50_frame_interval_ms,
        &report.object_count,
        "p50_frame_interval_ms",
    )?;
    let p95 = required_number(
        report.p95_frame_interval_ms,
        &report.object_count,
        "p95_frame_interval_ms",
    )?;
    let p99 = required_number(
        report.p99_frame_interval_ms,
        &report.object_count,
        "p99_frame_interval_ms",
    )?;
    if p50 > p95 || p95 > p99 {
        return Err(format!(
            "{}: frame-time mean/percentiles are inconsistent",
            path.display()
        ));
    }
    for (name, value) in [
        ("over_16_67_ms_fraction", report.over_16_67_ms_fraction),
        ("over_33_3_ms_fraction", report.over_33_3_ms_fraction),
    ] {
        let value = required_number(value, &report.object_count, name)?;
        if value > 1.0 {
            return Err(format!("{}: {name} must be within 0..=1", path.display()));
        }
    }
    if !report.sample_window_start_secs.is_finite()
        || !report.sample_window_end_secs.is_finite()
        || report.sample_window_start_secs >= report.sample_window_end_secs
    {
        return Err(format!("{}: invalid sample window", path.display()));
    }
    Ok(())
}

fn validate_comparable_context(reports: &[FrameTimeReport]) -> Result<(), String> {
    let first = reports
        .first()
        .ok_or_else(|| "no reports supplied".to_owned())?;
    for report in reports.iter().skip(1) {
        let differences = [
            ("fixture_id", first.fixture_id != report.fixture_id),
            ("camera_id", first.camera_id != report.camera_id),
            ("renderer", first.renderer != report.renderer),
            ("profile", first.profile != report.profile),
            (
                "shadows_enabled",
                first.shadows_enabled != report.shadows_enabled,
            ),
            ("width_px", first.width_px != report.width_px),
            ("height_px", first.height_px != report.height_px),
            ("source_commit", first.source_commit != report.source_commit),
            (
                "fixture_sha256",
                first.fixture_sha256 != report.fixture_sha256,
            ),
            ("gpu_adapter", first.gpu_adapter != report.gpu_adapter),
            ("gpu_driver", first.gpu_driver != report.gpu_driver),
            (
                "render_backend",
                first.render_backend != report.render_backend,
            ),
            ("rustc_version", first.rustc_version != report.rustc_version),
            (
                "operating_system",
                first.operating_system != report.operating_system,
            ),
            (
                "sampling_method",
                first.sampling_method != report.sampling_method,
            ),
            (
                "sample_window_start_secs",
                first.sample_window_start_secs != report.sample_window_start_secs,
            ),
            (
                "sample_window_end_secs",
                first.sample_window_end_secs != report.sample_window_end_secs,
            ),
        ];
        for (field, differs) in differences {
            if differs {
                return Err(format!(
                    "incomparable sweep: {field} differs for object_count {}",
                    report.object_count
                ));
            }
        }
    }
    Ok(())
}

fn make_context(report: &FrameTimeReport) -> Result<SweepContext, String> {
    Ok(SweepContext {
        fixture_id: report.fixture_id.clone(),
        fixture_sha256: required_string(
            &report.fixture_sha256,
            &report.object_count,
            "fixture_sha256",
        )?
        .to_owned(),
        camera_id: report.camera_id.clone(),
        renderer: report.renderer.clone(),
        source_commit: required_string(
            &report.source_commit,
            &report.object_count,
            "source_commit",
        )?
        .to_owned(),
        profile: report.profile.clone(),
        shadows_enabled: report.shadows_enabled,
        width_px: report.width_px,
        height_px: report.height_px,
        gpu_adapter: required_string(&report.gpu_adapter, &report.object_count, "gpu_adapter")?
            .to_owned(),
        gpu_driver: required_string(&report.gpu_driver, &report.object_count, "gpu_driver")?
            .to_owned(),
        render_backend: required_string(
            &report.render_backend,
            &report.object_count,
            "render_backend",
        )?
        .to_owned(),
        rustc_version: required_string(
            &report.rustc_version,
            &report.object_count,
            "rustc_version",
        )?
        .to_owned(),
        operating_system: required_string(
            &report.operating_system,
            &report.object_count,
            "operating_system",
        )?
        .to_owned(),
        sampling_method: report.sampling_method.clone(),
        sample_window_start_secs: report.sample_window_start_secs,
        sample_window_end_secs: report.sample_window_end_secs,
    })
}

fn required_number(value: Option<f64>, object_count: &usize, name: &str) -> Result<f64, String> {
    let value = value.ok_or_else(|| format!("object_count {object_count}: {name} is absent"))?;
    if !value.is_finite() || value < 0.0 {
        return Err(format!(
            "object_count {object_count}: {name} must be finite and non-negative"
        ));
    }
    Ok(value)
}

fn required_string<'a>(
    value: &'a Option<String>,
    object_count: &usize,
    name: &str,
) -> Result<&'a str, String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("object_count {object_count}: {name} is absent"))
}

fn is_commit_id(value: &str) -> bool {
    (value.len() == 40 || value.len() == 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn ensure_output_is_distinct(output: &Path, inputs: &[PathBuf]) -> Result<(), String> {
    let parent = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "cannot create output directory {}: {error}",
            parent.display()
        )
    })?;
    let file_name = output
        .file_name()
        .ok_or_else(|| "output path must name a file".to_owned())?;
    let output_path = if output.exists() {
        fs::canonicalize(output)
            .map_err(|error| format!("cannot resolve output path {}: {error}", output.display()))?
    } else {
        fs::canonicalize(parent)
            .map_err(|error| {
                format!(
                    "cannot resolve output directory {}: {error}",
                    parent.display()
                )
            })?
            .join(file_name)
    };
    for input in inputs {
        let input_path = fs::canonicalize(input)
            .map_err(|error| format!("cannot resolve input path {}: {error}", input.display()))?;
        if output_path == input_path {
            return Err(format!(
                "output path {} must not overwrite an input report",
                output.display()
            ));
        }
    }
    Ok(())
}

fn write_json_atomically<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("cannot serialize summary JSON: {error}"))?;
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
    let result = (|| -> io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        #[cfg(windows)]
        if absolute_path.exists() {
            fs::remove_file(&absolute_path)?;
        }
        fs::rename(&temporary, &absolute_path)
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "cannot write summary {}: {error}",
            absolute_path.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_report(object_count: usize) -> FrameTimeReport {
        FrameTimeReport {
            schema_version: 1,
            fixture_id: "SCENE-LIGHT-01".to_owned(),
            camera_id: "auto-grid-camera-v1".to_owned(),
            renderer: "bevy-0.19.0".to_owned(),
            profile: "balanced-1080p".to_owned(),
            object_count,
            shadows_enabled: true,
            width_px: 1920,
            height_px: 1080,
            source_commit: Some("a".repeat(40)),
            fixture_sha256: Some("b".repeat(64)),
            render_settings_sha256: Some("c".repeat(64)),
            gpu_adapter: Some("reference-gpu".to_owned()),
            gpu_driver: Some("driver-1".to_owned()),
            render_backend: Some("vulkan".to_owned()),
            rustc_version: Some("rustc 1.97.1".to_owned()),
            operating_system: Some("Linux".to_owned()),
            sampling_method: "bevy-time-real-frame-delta-ms; not-gpu-timestamp".to_owned(),
            sample_window_start_secs: 1.0,
            sample_window_end_secs: 8.0,
            sample_count: 400,
            min_required_samples: 30,
            mean_frame_interval_ms: Some(10.0),
            p50_frame_interval_ms: Some(10.0),
            p95_frame_interval_ms: Some(16.0),
            p99_frame_interval_ms: Some(18.0),
            over_16_67_ms_fraction: Some(0.03),
            over_33_3_ms_fraction: Some(0.0),
            measurement_status: "measured_metadata_qualified".to_owned(),
            claim_ceiling: "observed-real-frame-intervals-and-caller-declared-metadata-only; not-gpu-timestamp-or-visual-or-physical-validation".to_owned(),
        }
    }

    #[test]
    fn comparable_reports_are_ordered_by_object_count() {
        let mut reports = [test_report(256), test_report(64)];
        reports.sort_by_key(|report| report.object_count);
        assert_eq!(reports[0].object_count, 64);
        assert_eq!(reports[1].object_count, 256);
    }

    #[test]
    fn context_mismatch_is_rejected() {
        let first = test_report(64);
        let mut second = test_report(256);
        second.gpu_driver = Some("different-driver".to_owned());
        assert!(validate_comparable_context(&[first, second]).is_err());
    }

    #[test]
    fn a_qualified_frame_time_report_passes_the_input_contract() {
        let report = test_report(64);
        assert!(validate_report(&report, Path::new("valid.json")).is_ok());
    }

    #[test]
    fn invalid_sample_count_is_rejected() {
        let path = Path::new("synthetic.json");
        let mut report = test_report(64);
        report.sample_count = 29;
        assert!(validate_report(&report, path).is_err());
    }

    #[test]
    fn source_and_fixture_digest_formats_are_checked() {
        assert!(is_commit_id(&"a".repeat(40)));
        assert!(is_sha256(&"b".repeat(64)));
        assert!(!is_sha256("bad"));
    }
}
