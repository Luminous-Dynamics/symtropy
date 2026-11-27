// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Frozen, asset-free PBR scene for repeatable renderer comparisons.
//!
//! Geometry is generated from an integer lattice with no random inputs. Keep
//! the fixture implementation version bound to the source commit in captures.

use bevy::prelude::*;
use serde::Serialize;
use std::{
    env,
    path::{Path, PathBuf},
};
use symtropy_demo_capture::CapturePlugin;

const FIXTURE_ID: &str = "SCENE-LIGHT-01";
const CAMERA_ID: &str = "auto-grid-camera-v1";
const CUBE_SIZE: f32 = 0.72;
const GRID_SPACING: f32 = 1.05;
const MAX_OBJECTS: usize = 10_000;
const MIN_FRAME_SAMPLES: usize = 30;

#[derive(Resource, Clone, Debug)]
struct BenchConfig {
    profile: String,
    width_px: u32,
    height_px: u32,
    object_count: usize,
    shadows_enabled: bool,
}

#[derive(Resource)]
struct FrameTimeSampler {
    samples_ms: Vec<f64>,
    written: bool,
    output_path: PathBuf,
    context: FrameTimeContext,
}

#[derive(Clone)]
struct FrameTimeContext {
    source_commit: Option<String>,
    fixture_sha256: Option<String>,
    render_settings_sha256: Option<String>,
    renderer_version: Option<String>,
    gpu_adapter: Option<String>,
    gpu_driver: Option<String>,
    render_backend: Option<String>,
    rustc_version: Option<String>,
    operating_system: Option<String>,
}

#[derive(Serialize)]
struct FrameTimeReport {
    schema_version: u32,
    fixture_id: &'static str,
    camera_id: &'static str,
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
    sampling_method: &'static str,
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
    measurement_status: &'static str,
    claim_ceiling: &'static str,
}

impl FrameTimeSampler {
    fn from_env(config: &BenchConfig) -> Self {
        let default_dir = format!("target/render-captures/{}", config.capture_profile());
        let output_path = env::var("SYMTROPY_RENDER_BENCH_METRICS_OUTPUT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(default_dir).join("frame-time.json"));
        let context = FrameTimeContext {
            source_commit: env_nonempty("SYMTROPY_DEMO_CAPTURE_COMMIT"),
            fixture_sha256: env_nonempty("SYMTROPY_DEMO_CAPTURE_FIXTURE_SHA256"),
            render_settings_sha256: env_nonempty("SYMTROPY_DEMO_CAPTURE_RENDER_SETTINGS_SHA256"),
            renderer_version: env_nonempty("SYMTROPY_DEMO_CAPTURE_RENDERER_VERSION"),
            gpu_adapter: env_nonempty("SYMTROPY_DEMO_CAPTURE_GPU_ADAPTER"),
            gpu_driver: env_nonempty("SYMTROPY_DEMO_CAPTURE_GPU_DRIVER"),
            render_backend: env_nonempty("SYMTROPY_DEMO_CAPTURE_RENDER_BACKEND"),
            rustc_version: env_nonempty("SYMTROPY_DEMO_CAPTURE_RUSTC_VERSION"),
            operating_system: env_nonempty("SYMTROPY_DEMO_CAPTURE_OS"),
        };
        Self {
            samples_ms: Vec::new(),
            written: false,
            output_path,
            context,
        }
    }
}

fn env_nonempty(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

impl BenchConfig {
    fn from_env() -> Result<Self, String> {
        let profile = env::var("SYMTROPY_RENDER_BENCH_PROFILE")
            .unwrap_or_else(|_| "balanced-1080p".to_owned());
        let (width_px, height_px, default_count, default_shadows) = profile_defaults(&profile)?;

        let object_count = match env::var("SYMTROPY_RENDER_BENCH_OBJECTS") {
            Ok(raw) => raw
                .parse::<usize>()
                .map_err(|_| "SYMTROPY_RENDER_BENCH_OBJECTS must be an integer".to_owned())?,
            Err(_) => default_count,
        };
        if !(1..=MAX_OBJECTS).contains(&object_count) {
            return Err(format!(
                "object count must be in 1..={MAX_OBJECTS}, got {object_count}"
            ));
        }

        let shadows_enabled = match env::var("SYMTROPY_RENDER_BENCH_SHADOWS") {
            Ok(raw) => match raw.to_ascii_lowercase().as_str() {
                "1" | "true" | "on" => true,
                "0" | "false" | "off" => false,
                _ => {
                    return Err(
                        "SYMTROPY_RENDER_BENCH_SHADOWS must be on/off, true/false, or 1/0"
                            .to_owned(),
                    );
                }
            },
            Err(_) => default_shadows,
        };

        Ok(Self {
            profile,
            width_px,
            height_px,
            object_count,
            shadows_enabled,
        })
    }

    fn capture_profile(&self) -> String {
        format!(
            "{}-n{}-shadows-{}",
            self.profile,
            self.object_count,
            if self.shadows_enabled { "on" } else { "off" }
        )
    }
}

fn main() {
    let config = match BenchConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("symtropy-render-benchmark: {error}");
            std::process::exit(2);
        }
    };

    let sampler = FrameTimeSampler::from_env(&config);
    let window = Window {
        title: format!("{FIXTURE_ID} — {}", config.capture_profile()),
        resolution: (config.width_px, config.height_px).into(),
        ..default()
    };

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(window),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.035, 0.045, 0.065)))
        .insert_resource(bevy::light::GlobalAmbientLight {
            color: Color::srgb(0.82, 0.86, 0.96),
            brightness: 180.0,
            ..default()
        })
        .insert_resource(sampler)
        .insert_resource(config)
        .add_plugins(CapturePlugin::new("scene-light-01"))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, sample_and_write_frame_times)
        .run();
}

fn sample_and_write_frame_times(
    time: Res<Time<Real>>,
    config: Res<BenchConfig>,
    mut sampler: ResMut<FrameTimeSampler>,
) {
    const SAMPLE_START_SECS: f32 = 1.0;
    const SAMPLE_END_SECS: f32 = 8.0;
    let elapsed = time.elapsed_secs();
    let frame_interval_ms = f64::from(time.delta_secs()) * 1000.0;

    if (SAMPLE_START_SECS..SAMPLE_END_SECS).contains(&elapsed)
        && frame_interval_ms.is_finite()
        && frame_interval_ms > 0.0
    {
        sampler.samples_ms.push(frame_interval_ms);
    }

    if elapsed < SAMPLE_END_SECS || sampler.written {
        return;
    }

    let report = build_frame_time_report(&sampler.samples_ms, &sampler.context, &config);
    match write_report_atomically(&sampler.output_path, &report) {
        Ok(()) => {
            info!(
                "symtropy-render-benchmark: wrote {} frame-time samples to {}",
                report.sample_count,
                sampler.output_path.display()
            );
            sampler.written = true;
        }
        Err(error) => {
            error!(
                "symtropy-render-benchmark: failed to write frame-time report {}: {error}",
                sampler.output_path.display()
            );
            // Retry on the next update; the runner also verifies that a fresh
            // report exists before treating the benchmark invocation as complete.
        }
    }
}

fn build_frame_time_report(
    samples_ms: &[f64],
    context: &FrameTimeContext,
    config: &BenchConfig,
) -> FrameTimeReport {
    let mut sorted = samples_ms.to_vec();
    sorted.sort_by(f64::total_cmp);
    let count = sorted.len();
    let percentile = |percent: usize| percentile_value(&sorted, percent);
    let mean = if count == 0 {
        None
    } else {
        Some(sorted.iter().sum::<f64>() / count as f64)
    };
    let fraction_over = |limit: f64| -> Option<f64> {
        if count == 0 {
            None
        } else {
            Some(sorted.iter().filter(|sample| **sample > limit).count() as f64 / count as f64)
        }
    };
    let qualified = context.source_commit.as_deref().is_some_and(is_commit_id)
        && context.renderer_version.as_deref().is_some_and(not_empty)
        && context.fixture_sha256.as_deref().is_some_and(is_sha256)
        && context
            .render_settings_sha256
            .as_deref()
            .is_some_and(is_sha256)
        && context.gpu_adapter.as_deref().is_some_and(not_empty)
        && context.gpu_driver.as_deref().is_some_and(not_empty)
        && context.render_backend.as_deref().is_some_and(not_empty)
        && context.rustc_version.as_deref().is_some_and(not_empty)
        && context.operating_system.as_deref().is_some_and(not_empty);
    FrameTimeReport {
        schema_version: 1,
        fixture_id: FIXTURE_ID,
        camera_id: CAMERA_ID,
        renderer: context
            .renderer_version
            .clone()
            .unwrap_or_else(|| "unknown".to_owned()),
        profile: config.profile.clone(),
        object_count: config.object_count,
        shadows_enabled: config.shadows_enabled,
        width_px: config.width_px,
        height_px: config.height_px,
        source_commit: context.source_commit.clone(),
        fixture_sha256: context.fixture_sha256.clone(),
        render_settings_sha256: context.render_settings_sha256.clone(),
        gpu_adapter: context.gpu_adapter.clone(),
        gpu_driver: context.gpu_driver.clone(),
        render_backend: context.render_backend.clone(),
        rustc_version: context.rustc_version.clone(),
        operating_system: context.operating_system.clone(),
        sampling_method: "bevy-time-real-frame-delta-ms; not-gpu-timestamp",
        sample_window_start_secs: 1.0,
        sample_window_end_secs: 8.0,
        sample_count: count,
        min_required_samples: MIN_FRAME_SAMPLES,
        mean_frame_interval_ms: mean,
        p50_frame_interval_ms: percentile(50),
        p95_frame_interval_ms: percentile(95),
        p99_frame_interval_ms: percentile(99),
        over_16_67_ms_fraction: fraction_over(16.67),
        over_33_3_ms_fraction: fraction_over(33.3),
        measurement_status: measurement_status(count, qualified),
        claim_ceiling: "observed-real-frame-intervals-and-caller-declared-metadata-only; not-gpu-timestamp-or-visual-or-physical-validation",
    }
}

fn measurement_status(sample_count: usize, metadata_qualified: bool) -> &'static str {
    if sample_count < MIN_FRAME_SAMPLES {
        "insufficient_samples"
    } else if metadata_qualified {
        "measured_metadata_qualified"
    } else {
        "measured_metadata_incomplete"
    }
}

fn percentile_value(sorted_values: &[f64], percent: usize) -> Option<f64> {
    if sorted_values.is_empty() || percent == 0 || percent > 100 {
        return None;
    }
    let mut sorted = sorted_values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = (sorted.len() * percent).div_ceil(100).max(1) - 1;
    sorted.get(rank).copied()
}

fn not_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_commit_id(value: &str) -> bool {
    (value.len() == 40 || value.len() == 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn write_report_atomically(path: &Path, report: &FrameTimeReport) -> std::io::Result<()> {
    use std::io::Write as _;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    std::fs::create_dir_all(parent)?;
    let temporary = path.with_file_name(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));
    let bytes = serde_json::to_vec_pretty(report)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        #[cfg(windows)]
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        std::fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<BenchConfig>,
) {
    let (columns, rows) = grid_dimensions(config.object_count);
    let grid_width = (columns.saturating_sub(1) as f32 * GRID_SPACING) + CUBE_SIZE;
    let grid_depth = (rows.saturating_sub(1) as f32 * GRID_SPACING) + CUBE_SIZE;
    let grid_span = grid_width.max(grid_depth);
    let camera_distance = grid_span * 1.55 + 7.0;

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(
            camera_distance * 0.70,
            camera_distance * 0.78,
            camera_distance * 0.92,
        )
        .looking_at(Vec3::new(0.0, 0.35, 0.0), Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            color: Color::linear_rgb(1.0, 0.97, 0.91),
            shadow_maps_enabled: config.shadows_enabled,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.72, -0.62, 0.0)),
    ));

    // One shared mesh and eight shared materials make the workload repeatable
    // without adding random texture/material asset loading to the fixture.
    let cube_mesh = meshes.add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE));
    let material_profiles = [
        (Color::srgb(0.82, 0.12, 0.10), 0.0, 0.88),
        (Color::srgb(0.08, 0.26, 0.86), 0.82, 0.22),
        (Color::srgb(0.12, 0.66, 0.34), 0.05, 0.72),
        (Color::srgb(0.92, 0.58, 0.08), 0.76, 0.28),
        (Color::srgb(0.08, 0.67, 0.73), 0.34, 0.42),
        (Color::srgb(0.60, 0.16, 0.76), 0.12, 0.55),
        (Color::srgb(0.72, 0.75, 0.80), 0.0, 0.94),
        (Color::srgb(0.16, 0.18, 0.22), 0.92, 0.18),
    ];
    let material_handles: Vec<Handle<StandardMaterial>> = material_profiles
        .into_iter()
        .map(|(base_color, metallic, perceptual_roughness)| {
            materials.add(StandardMaterial {
                base_color,
                metallic,
                perceptual_roughness,
                ..default()
            })
        })
        .collect();

    let floor_mesh = meshes.add(Cuboid::new(grid_width + 5.0, 0.2, grid_depth + 5.0));
    let floor_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.11, 0.13, 0.16),
        metallic: 0.0,
        perceptual_roughness: 0.94,
        ..default()
    });
    commands.spawn((
        Mesh3d(floor_mesh),
        MeshMaterial3d(floor_material),
        Transform::from_xyz(0.0, -0.12, 0.0),
    ));

    for index in 0..config.object_count {
        let column = index % columns;
        let row = index / columns;
        let x =
            column as f32 * GRID_SPACING - (columns.saturating_sub(1) as f32 * GRID_SPACING * 0.5);
        let z = row as f32 * GRID_SPACING - (rows.saturating_sub(1) as f32 * GRID_SPACING * 0.5);
        let scale_y = if index % 11 == 0 { 1.45 } else { 1.0 };
        let material = material_handles[index % material_handles.len()].clone();

        commands.spawn((
            Mesh3d(cube_mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_xyz(x, CUBE_SIZE * scale_y * 0.5, z)
                .with_scale(Vec3::new(1.0, scale_y, 1.0)),
        ));
    }

    commands.spawn((
        Text::new(format!(
            "{FIXTURE_ID}  |  {} objects  |  profile {}  |  shadows {}",
            config.object_count,
            config.profile,
            if config.shadows_enabled { "on" } else { "off" }
        )),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            top: Val::Px(14.0),
            ..default()
        },
    ));
}

fn profile_defaults(profile: &str) -> Result<(u32, u32, usize, bool), String> {
    match profile {
        "quality-1080p" => Ok((1920, 1080, 256, true)),
        "balanced-1080p" => Ok((1920, 1080, 144, true)),
        "low-power-720p" => Ok((1280, 720, 64, false)),
        _ => Err(format!(
            "unsupported profile {profile:?}; choose quality-1080p, balanced-1080p, or low-power-720p"
        )),
    }
}

fn grid_dimensions(object_count: usize) -> (usize, usize) {
    let columns = (object_count as f64).sqrt().ceil() as usize;
    let rows = object_count.div_ceil(columns);
    (columns, rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_profiles_have_fixed_resolution_and_default_workload() {
        let quality = profile_defaults("quality-1080p").expect("profile should exist");
        assert_eq!(quality, (1920, 1080, 256, true));

        let balanced = profile_defaults("balanced-1080p").expect("profile should exist");
        assert_eq!(balanced, (1920, 1080, 144, true));

        let low_power = profile_defaults("low-power-720p").expect("profile should exist");
        assert_eq!(low_power, (1280, 720, 64, false));
    }

    #[test]
    fn insufficient_frame_samples_cannot_be_qualified() {
        assert_eq!(measurement_status(0, true), "insufficient_samples");
        assert_eq!(
            measurement_status(MIN_FRAME_SAMPLES - 1, true),
            "insufficient_samples"
        );
        assert_eq!(
            measurement_status(MIN_FRAME_SAMPLES, true),
            "measured_metadata_qualified"
        );
        assert_eq!(
            measurement_status(MIN_FRAME_SAMPLES, false),
            "measured_metadata_incomplete"
        );
    }

    #[test]
    fn percentiles_use_nearest_rank_and_boundaries() {
        let values = [1.0, 2.0, 3.0, 4.0, 100.0];
        assert_eq!(percentile_value(&values, 50), Some(3.0));
        assert_eq!(percentile_value(&values, 95), Some(100.0));
        assert_eq!(percentile_value(&values, 99), Some(100.0));
        assert_eq!(percentile_value(&[], 95), None);
    }

    #[test]
    fn grid_dimensions_cover_all_objects_without_randomness() {
        assert_eq!(grid_dimensions(1), (1, 1));
        assert_eq!(grid_dimensions(13), (4, 4));
        assert_eq!(grid_dimensions(144), (12, 12));
    }
}
