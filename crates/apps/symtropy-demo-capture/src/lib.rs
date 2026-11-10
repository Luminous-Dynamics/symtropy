// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Headless screenshot scheduler shared across the symtropy demo crates.
//!
//! Usage inside a demo's `main.rs`:
//!
//! ```ignore
//! App::new()
//!     .add_plugins(DefaultPlugins.set(/* ... */))
//!     .add_plugins(MyDemoPlugin)
//!     // Add this line — stem is used in the PNG filenames:
//!     .add_plugins(symtropy_demo_capture::CapturePlugin::new("flight"))
//!     .run();
//! ```
//!
//! When `SYMTROPY_DEMO_CAPTURE_DIR` is unset the plugin is a no-op. When
//! set, it schedules PNG screenshots at `t = 1.5, 4.0, 7.0 s` of real
//! time, then writes `AppExit::Success` at `t = 8.5 s` so the process
//! terminates cleanly.
//!
//! Pattern cloned verbatim from
//! `symtropy/crates/symtropy-bevy/examples/pendulum_swarm.rs` (which is
//! the first demo where this approach was proven).

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

/// Per-capture-run scheduling state. Present as a resource only when the
/// env var was set at plugin-build time.
#[derive(Resource)]
struct CaptureSchedule {
    /// Output directory (read from `SYMTROPY_DEMO_CAPTURE_DIR`).
    dir: String,
    /// Filename stem (e.g. "flight" → `flight_t1.5.png`).
    stem: String,
    /// Screenshot timestamps (seconds since startup, ascending).
    schedule: Vec<f32>,
    /// How many screenshots have been queued so far.
    fired: usize,
    /// Wall-clock time (seconds since startup) at which to exit.
    exit_at: f32,
    /// Last time a screenshot request was queued. Exit waits briefly after this
    /// so Bevy can flush the screenshot observer to disk.
    last_capture_at: Option<f32>,
    /// Expected capture files. The plugin exits only after these exist.
    pending_paths: Vec<String>,
    /// Invalid capture configuration is a hard failure, never a silent fallback.
    configuration_error: Option<String>,
}

/// Plugin that wires up headless capture when
/// `SYMTROPY_DEMO_CAPTURE_DIR` is set in the environment. No-op otherwise.
pub struct CapturePlugin {
    stem: String,
}

impl CapturePlugin {
    /// `stem` is used as the filename prefix (e.g. "flight" →
    /// `flight_t1.5.png`). Typically pass the demo's short name.
    pub fn new(stem: impl Into<String>) -> Self {
        Self { stem: stem.into() }
    }
}

impl Plugin for CapturePlugin {
    fn build(&self, app: &mut App) {
        let Ok(dir) = std::env::var("SYMTROPY_DEMO_CAPTURE_DIR") else {
            // Default path: env not set — plugin is a pure no-op.
            return;
        };
        let configuration = if dir.trim().is_empty() {
            Err("SYMTROPY_DEMO_CAPTURE_DIR must not be empty".to_string())
        } else {
            capture_configuration_from_env()
        };
        let (schedule, exit_at, configuration_error) = match configuration {
            Ok((schedule, exit_at)) => (schedule, exit_at, None),
            Err(error) => (Vec::new(), 0.0, Some(error)),
        };
        app.insert_resource(CaptureSchedule {
            dir,
            stem: self.stem.clone(),
            schedule,
            fired: 0,
            exit_at,
            last_capture_at: None,
            pending_paths: Vec::new(),
            configuration_error,
        });
        app.add_systems(Update, headless_capture);
    }
}

fn capture_configuration_from_env() -> Result<(Vec<f32>, f32), String> {
    let schedule = match std::env::var("SYMTROPY_DEMO_CAPTURE_TIMES") {
        Ok(raw) => parse_capture_schedule(&raw)?,
        Err(std::env::VarError::NotPresent) => vec![1.5, 4.0, 7.0],
        Err(error) => {
            return Err(format!("could not read SYMTROPY_DEMO_CAPTURE_TIMES: {error}"));
        }
    };

    let exit_at_raw = match std::env::var("SYMTROPY_DEMO_CAPTURE_EXIT_AT") {
        Ok(raw) => Some(raw),
        Err(std::env::VarError::NotPresent) => None,
        Err(error) => {
            return Err(format!("could not read SYMTROPY_DEMO_CAPTURE_EXIT_AT: {error}"));
        }
    };
    let exit_at = parse_capture_exit_at(exit_at_raw.as_deref(), &schedule)?;
    Ok((schedule, exit_at))
}

/// Parse a capture schedule without silently dropping malformed entries.
/// Exact duplicates are removed after canonical sorting.
fn parse_capture_schedule(raw: &str) -> Result<Vec<f32>, String> {
    if raw.trim().is_empty() {
        return Err("SYMTROPY_DEMO_CAPTURE_TIMES must not be empty".to_string());
    }

    let mut schedule = Vec::new();
    for (index, part) in raw.split(',').enumerate() {
        let value = part.trim();
        if value.is_empty() {
            return Err(format!(
                "SYMTROPY_DEMO_CAPTURE_TIMES entry {} is empty",
                index + 1
            ));
        }

        let time = value.parse::<f32>().map_err(|error| {
            format!(
                "SYMTROPY_DEMO_CAPTURE_TIMES entry {} ({value:?}) is invalid: {error}",
                index + 1
            )
        })?;
        if !time.is_finite() || time < 0.0 {
            return Err(format!(
                "SYMTROPY_DEMO_CAPTURE_TIMES entry {} must be finite and non-negative",
                index + 1
            ));
        }
        schedule.push(time);
    }

    schedule.sort_by(f32::total_cmp);
    schedule.dedup_by(|a, b| *a == *b);
    if schedule.is_empty() {
        return Err("SYMTROPY_DEMO_CAPTURE_TIMES contains no capture times".to_string());
    }
    Ok(schedule)
}

fn parse_capture_exit_at(raw: Option<&str>, schedule: &[f32]) -> Result<f32, String> {
    let last_capture = schedule
        .last()
        .copied()
        .ok_or_else(|| "capture schedule must contain at least one time".to_string())?;
    let exit_at = match raw {
        Some(value) => value.parse::<f32>().map_err(|error| {
            format!("SYMTROPY_DEMO_CAPTURE_EXIT_AT ({value:?}) is invalid: {error}")
        })?,
        None => last_capture + 1.5,
    };

    if !exit_at.is_finite() || exit_at < last_capture {
        return Err(format!(
            "SYMTROPY_DEMO_CAPTURE_EXIT_AT must be finite and at least the final capture time ({last_capture})"
        ));
    }
    Ok(exit_at)
}

fn capture_path(dir: &str, stem: &str, index: usize, time: f32) -> String {
    std::path::Path::new(dir)
        .join(format!("{stem}_capture_{index:03}_t{time:.3}.png"))
        .to_string_lossy()
        .into_owned()
}

fn headless_capture(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut sched: ResMut<CaptureSchedule>,
    mut exit: MessageWriter<AppExit>,
) {
    if let Some(error) = sched.configuration_error.take() {
        error!("symtropy-demo-capture: invalid capture configuration: {error}");
        exit.write(AppExit::error());
        return;
    }

    let now = time.elapsed_secs();
    if sched.fired < sched.schedule.len() && now >= sched.schedule[sched.fired] {
        let label = sched.schedule[sched.fired];
        let path = capture_path(&sched.dir, &sched.stem, sched.fired, label);
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
        info!("symtropy-demo-capture: queued screenshot {}", path);
        sched.fired += 1;
        sched.last_capture_at = Some(now);
        sched.pending_paths.push(path);
    }
    let captures_flushed = sched
        .last_capture_at
        .map(|last_capture_at| now - last_capture_at >= 1.0)
        .unwrap_or(true);
    let captures_on_disk = sched.pending_paths.len() >= sched.schedule.len()
        && sched.pending_paths.iter().all(|path| {
            std::fs::metadata(path)
                .map(|metadata| metadata.len() > 0)
                .unwrap_or(false)
        });
    let timed_out = now >= sched.exit_at + 15.0;
    if timed_out && !captures_on_disk {
        error!(
            "symtropy-demo-capture: timed out waiting for screenshots: {:?}",
            sched.pending_paths
        );
    }
    if sched.fired >= sched.schedule.len()
        && now >= sched.exit_at
        && captures_flushed
        && (captures_on_disk || timed_out)
    {
        if captures_on_disk {
            info!("symtropy-demo-capture: all expected screenshots are present");
            exit.write(AppExit::Success);
        } else {
            error!(
                "symtropy-demo-capture: exiting with failure because one or more screenshots are missing"
            );
            exit.write(AppExit::error());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_schedule_is_sorted_and_exact_duplicates_are_removed() {
        assert_eq!(
            parse_capture_schedule("7.0,1.5,4.0,4.0").expect("valid schedule"),
            vec![1.5, 4.0, 7.0]
        );
    }

    #[test]
    fn capture_schedule_rejects_empty_malformed_negative_and_non_finite_entries() {
        for raw in ["", " ", "1.0,,2.0", "1.0,not-a-time", "-0.1", "NaN", "inf"] {
            assert!(
                parse_capture_schedule(raw).is_err(),
                "schedule should be rejected: {raw:?}"
            );
        }
    }

    #[test]
    fn exit_time_defaults_after_the_last_capture() {
        assert_eq!(
            parse_capture_exit_at(None, &[1.5, 4.0, 7.0]).expect("default exit time"),
            8.5
        );
    }

    #[test]
    fn exit_time_rejects_malformed_non_finite_or_early_values() {
        for raw in ["not-a-time", "NaN", "inf", "6.99"] {
            assert!(
                parse_capture_exit_at(Some(raw), &[1.5, 4.0, 7.0]).is_err(),
                "exit time should be rejected: {raw:?}"
            );
        }
        assert!(parse_capture_exit_at(None, &[]).is_err());
    }

    #[test]
    fn capture_paths_remain_unique_for_close_capture_times() {
        let first = capture_path("/tmp/captures", "scene", 0, 1.51);
        let second = capture_path("/tmp/captures", "scene", 1, 1.52);
        assert_ne!(first, second);
        assert!(first.ends_with("scene_capture_000_t1.510.png"));
        assert!(second.ends_with("scene_capture_001_t1.520.png"));
    }
}
