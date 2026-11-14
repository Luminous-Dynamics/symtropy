// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Reproducible, renderer-free human-motion trace comparison.
//!
//! Usage:
//!   cargo run -p symtropy-bevy --bin human-motion-bench -- reference.json prediction.json
//!   cargo run -p symtropy-bevy --bin human-motion-bench -- reference.json prediction.json --step-sec 0.033333 --output report.json
//!
//! Output is JSON on stdout unless --output is supplied. A short human-readable summary
//! goes to stderr. Traces are interpolated onto one uniform grid over their shared time
//! interval; they are never silently compared by sample index alone.

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env,
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
    process,
    time::Instant,
};
use symtropy_bevy::human_motion_metrics::{
    mean_squared_jerk, stance_foot_drift, trajectory_errors,
};

const MAX_GRID_SAMPLES: usize = 1_000_000;
const GRID_EPSILON_RATIO: f64 = 1.0e-8;
const DEFAULT_CONTACT_EVENT_TOLERANCE_SEC: f64 = 0.1;
const QUATERNION_NORM_TOLERANCE: f64 = 1.0e-3;

#[derive(Debug)]
struct BenchError(String);

impl fmt::Display for BenchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl Error for BenchError {}

fn fail<T>(message: impl Into<String>) -> Result<T, BenchError> {
    Err(BenchError(message.into()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SourceKind {
    SyntheticTestFixture,
    LicensedAsset,
    OriginalConsentCapture,
    ExternalModel,
}

impl SourceKind {
    fn is_synthetic(self) -> bool {
        self == Self::SyntheticTestFixture
    }
}

fn has_held_out_human_reference(trace: &Trace) -> bool {
    trace.run.source_kind == SourceKind::OriginalConsentCapture
        && trace.run.split.as_deref() == Some("test")
        && is_hex_digest(&trace.run.git_commit, 40)
        && trace
            .run
            .asset_sha256
            .as_deref()
            .is_some_and(|value| is_hex_digest(value, 64))
        && trace
            .run
            .asset_license
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && trace
            .retarget_map_sha256
            .0
            .as_deref()
            .is_some_and(|value| is_hex_digest(value, 64))
        && trace
            .run
            .dataset_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && trace
            .run
            .dataset_version
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && trace
            .run
            .actor_pseudonym
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && trace
            .run
            .recording_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

fn evidence_scope(reference: &Trace, prediction: &Trace) -> &'static str {
    if reference.run.source_kind.is_synthetic() || prediction.run.source_kind.is_synthetic() {
        "synthetic_software_validation_only"
    } else if reference.trace_id != prediction.trace_id && has_held_out_human_reference(reference) {
        "declared_held_out_human_reference"
    } else {
        "source_provenance_present_reference_not_held_out_human_verified"
    }
}

fn validate_human_reference_claim(reference: &Trace, prediction: &Trace) -> Result<(), BenchError> {
    if reference.trace_id == prediction.trace_id {
        return fail("human-reference claims require distinct reference and prediction trace IDs");
    }
    if reference.run.source_kind != SourceKind::OriginalConsentCapture {
        return fail(
            "human-reference claims require a reference declared as an original consented human capture",
        );
    }
    if reference.run.split.as_deref() != Some("test") {
        return fail("human-reference claims require reference.run.split = test");
    }
    for (field, value) in [
        ("dataset_id", reference.run.dataset_id.as_deref()),
        ("dataset_version", reference.run.dataset_version.as_deref()),
        ("actor_pseudonym", reference.run.actor_pseudonym.as_deref()),
        ("recording_id", reference.run.recording_id.as_deref()),
    ] {
        if !value.is_some_and(|value| !value.trim().is_empty()) {
            return fail(format!(
                "human-reference claims require non-empty reference.run.{field}"
            ));
        }
    }
    if prediction.run.source_kind.is_synthetic() {
        return fail("human-reference claims cannot use a synthetic prediction trace");
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RunMetadata {
    source_kind: SourceKind,
    git_commit: String,
    configuration_sha256: String,
    #[serde(default)]
    model_sha256: Option<String>,
    #[serde(default)]
    asset_id: Option<String>,
    #[serde(default)]
    asset_sha256: Option<String>,
    #[serde(default)]
    asset_license: Option<String>,
    #[serde(default)]
    dataset_id: Option<String>,
    #[serde(default)]
    dataset_version: Option<String>,
    #[serde(default)]
    split: Option<String>,
    #[serde(default)]
    actor_pseudonym: Option<String>,
    #[serde(default)]
    recording_id: Option<String>,
    #[serde(default)]
    seed: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoordinateFrame {
    length_unit: String,
    up_axis: String,
    forward_axis: String,
    position_space: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
struct NullableDigest(Option<String>);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConstraintTarget {
    time_sec: f64,
    target: String,
    position_world_m: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Trace {
    schema_version: u32,
    trace_id: String,
    run: RunMetadata,
    coordinate_frame: CoordinateFrame,
    skeleton_signature_sha256: String,
    // Wrapper type makes this JSON key mandatory while permitting null for synthetic fixtures.
    retarget_map_sha256: NullableDigest,
    timestamps_sec: Vec<f64>,
    root_world_m: Vec<[f64; 3]>,
    #[serde(default)]
    root_rotation_world_xyzw: Option<Vec<[f64; 4]>>,
    left_foot_world_m: Vec<[f64; 3]>,
    left_foot_contact: Vec<bool>,
    right_foot_world_m: Vec<[f64; 3]>,
    right_foot_contact: Vec<bool>,
    #[serde(default)]
    joint_world_m: BTreeMap<String, Vec<[f64; 3]>>,
    #[serde(default)]
    constraint_targets: Vec<ConstraintTarget>,
    #[serde(default)]
    notes: Option<String>,
}

impl Trace {
    fn validate(&self, path: &Path) -> Result<(), BenchError> {
        let source = path.display();
        if !matches!(self.schema_version, 1 | 2) {
            return fail(format!(
                "{source}: unsupported schema_version {}; expected 1 or 2",
                self.schema_version
            ));
        }
        match (self.schema_version, self.root_rotation_world_xyzw.as_ref()) {
            (1, Some(_)) => {
                return fail(format!(
                    "{source}: root_rotation_world_xyzw requires schema_version 2"
                ));
            }
            (2, None) => {
                return fail(format!(
                    "{source}: schema_version 2 requires root_rotation_world_xyzw"
                ));
            }
            _ => {}
        }
        if self.trace_id.trim().is_empty() {
            return fail(format!("{source}: trace_id must not be empty"));
        }

        let frame = &self.coordinate_frame;
        if frame.length_unit != "metre"
            || frame.up_axis != "+Y"
            || !matches!(frame.forward_axis.as_str(), "+Z" | "-Z")
            || frame.position_space != "world"
        {
            return fail(format!(
                "{source}: unsupported coordinate frame; expected metres, +Y up, +Z or -Z forward, and world positions"
            ));
        }

        if !is_hex_digest(&self.run.configuration_sha256, 64) {
            return fail(format!(
                "{source}: run.configuration_sha256 must be 64 hexadecimal characters"
            ));
        }
        if !is_hex_digest(&self.skeleton_signature_sha256, 64) {
            return fail(format!(
                "{source}: skeleton_signature_sha256 must be 64 hexadecimal characters"
            ));
        }
        if self
            .retarget_map_sha256
            .0
            .as_deref()
            .is_some_and(|hash| !is_hex_digest(hash, 64))
        {
            return fail(format!(
                "{source}: retarget_map_sha256 must be null or 64 hexadecimal characters"
            ));
        }

        if self
            .run
            .model_sha256
            .as_deref()
            .is_some_and(|hash| !is_hex_digest(hash, 64))
        {
            return fail(format!(
                "{source}: run.model_sha256 must be null or 64 hexadecimal characters"
            ));
        }
        if self
            .run
            .asset_sha256
            .as_deref()
            .is_some_and(|hash| !is_hex_digest(hash, 64))
        {
            return fail(format!(
                "{source}: run.asset_sha256 must be null or 64 hexadecimal characters"
            ));
        }
        if self
            .run
            .split
            .as_deref()
            .is_some_and(|split| !matches!(split, "train" | "validation" | "test" | "synthetic"))
        {
            return fail(format!("{source}: unknown run.split value"));
        }
        if !self.run.source_kind.is_synthetic() && self.run.split.as_deref() == Some("synthetic") {
            return fail(format!(
                "{source}: non-synthetic source_kind cannot use run.split = synthetic"
            ));
        }

        if !self.run.source_kind.is_synthetic() {
            if !is_hex_digest(&self.run.git_commit, 40) {
                return fail(format!(
                    "{source}: non-synthetic traces require a 40-character hexadecimal git commit"
                ));
            }
            if !self
                .run
                .asset_sha256
                .as_deref()
                .is_some_and(|hash| is_hex_digest(hash, 64))
            {
                return fail(format!(
                    "{source}: non-synthetic traces require run.asset_sha256"
                ));
            }
            if !self
                .run
                .asset_license
                .as_deref()
                .is_some_and(|license| !license.trim().is_empty())
            {
                return fail(format!(
                    "{source}: non-synthetic traces require run.asset_license"
                ));
            }
            if self.retarget_map_sha256.0.is_none() {
                return fail(format!(
                    "{source}: non-synthetic traces require retarget_map_sha256; use a canonical identity-map digest if no retargeting was applied"
                ));
            }
            if self.run.source_kind == SourceKind::ExternalModel
                && !self
                    .run
                    .model_sha256
                    .as_deref()
                    .is_some_and(|hash| is_hex_digest(hash, 64))
            {
                return fail(format!(
                    "{source}: external_model traces require run.model_sha256"
                ));
            }
        }

        if self.timestamps_sec.is_empty() {
            return fail(format!(
                "{source}: timestamps_sec must contain at least one sample"
            ));
        }
        for (index, time) in self.timestamps_sec.iter().copied().enumerate() {
            if !time.is_finite() {
                return fail(format!("{source}: timestamps_sec[{index}] is not finite"));
            }
            if index > 0 && time <= self.timestamps_sec[index - 1] {
                return fail(format!(
                    "{source}: timestamps_sec must be strictly increasing (index {index})"
                ));
            }
        }
        if self
            .timestamps_sec
            .windows(2)
            .any(|pair| !(pair[1] - pair[0]).is_finite())
        {
            return fail(format!("{source}: timestamp difference overflowed"));
        }

        if let Some(rotations) = &self.root_rotation_world_xyzw {
            validate_length(
                self.timestamps_sec.len(),
                rotations.len(),
                "root_rotation_world_xyzw",
                path,
            )?;
            for (index, rotation) in rotations.iter().copied().enumerate() {
                validate_unit_quaternion(rotation).map_err(|error| {
                    BenchError(format!(
                        "{source}: root_rotation_world_xyzw[{index}] {error}"
                    ))
                })?;
            }
        }

        validate_positions(
            &self.timestamps_sec,
            &self.root_world_m,
            "root_world_m",
            path,
        )?;
        validate_positions(
            &self.timestamps_sec,
            &self.left_foot_world_m,
            "left_foot_world_m",
            path,
        )?;
        validate_positions(
            &self.timestamps_sec,
            &self.right_foot_world_m,
            "right_foot_world_m",
            path,
        )?;
        validate_length(
            self.timestamps_sec.len(),
            self.left_foot_contact.len(),
            "left_foot_contact",
            path,
        )?;
        validate_length(
            self.timestamps_sec.len(),
            self.right_foot_contact.len(),
            "right_foot_contact",
            path,
        )?;

        for (joint, positions) in &self.joint_world_m {
            if joint.trim().is_empty() {
                return fail(format!(
                    "{source}: joint_world_m contains an empty joint name"
                ));
            }
            validate_positions(
                &self.timestamps_sec,
                positions,
                &format!("joint_world_m[{joint:?}]"),
                path,
            )?;
        }
        for (index, constraint) in self.constraint_targets.iter().enumerate() {
            if !constraint.time_sec.is_finite()
                || constraint.target.trim().is_empty()
                || constraint
                    .position_world_m
                    .iter()
                    .any(|value| !value.is_finite())
            {
                return fail(format!(
                    "{source}: constraint_targets[{index}] contains invalid values"
                ));
            }
        }

        Ok(())
    }
}

fn validate_length(
    expected: usize,
    actual: usize,
    field: &str,
    path: &Path,
) -> Result<(), BenchError> {
    if expected != actual {
        return fail(format!(
            "{}: {field} has {actual} samples but timestamps_sec has {expected}",
            path.display()
        ));
    }
    Ok(())
}

fn validate_positions(
    timestamps: &[f64],
    positions: &[[f64; 3]],
    field: &str,
    path: &Path,
) -> Result<(), BenchError> {
    validate_length(timestamps.len(), positions.len(), field, path)?;
    for (index, position) in positions.iter().enumerate() {
        if position.iter().any(|value| !value.is_finite()) {
            return fail(format!(
                "{}: {field}[{index}] contains a non-finite coordinate",
                path.display()
            ));
        }
    }
    Ok(())
}

fn is_hex_digest(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn read_trace(path: &Path) -> Result<Trace, Box<dyn Error>> {
    let text = fs::read_to_string(path)?;
    let trace: Trace = serde_json::from_str(&text)?;
    trace.validate(path)?;
    Ok(trace)
}

fn median_sample_interval(timestamps: &[f64]) -> Result<f64, BenchError> {
    if timestamps.len() < 2 {
        return fail("each input trace requires at least two timestamped samples");
    }
    let mut differences: Vec<f64> = timestamps
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .collect();
    differences.sort_by(f64::total_cmp);
    let middle = differences.len() / 2;
    let median = if differences.len() % 2 == 0 {
        differences[middle - 1] + (differences[middle] - differences[middle - 1]) / 2.0
    } else {
        differences[middle]
    };
    if !median.is_finite() || median <= 0.0 {
        return fail("could not determine a finite positive median sample interval");
    }
    Ok(median)
}

#[derive(Debug)]
struct TimeGrid {
    timestamps_sec: Vec<f64>,
    step_sec: f64,
    overlap_start_sec: f64,
    overlap_end_sec: f64,
    reference_median_sample_interval_sec: f64,
    reference_max_sample_interval_sec: f64,
    prediction_median_sample_interval_sec: f64,
    prediction_max_sample_interval_sec: f64,
}

fn maximum_sample_interval_sec(timestamps: &[f64]) -> Result<f64, BenchError> {
    if timestamps.len() < 2 {
        return fail("each input trace requires at least two timestamped samples");
    }
    let maximum = timestamps
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .max_by(f64::total_cmp)
        .unwrap_or(0.0);
    if !maximum.is_finite() || maximum <= 0.0 {
        return fail("could not determine a finite positive maximum sample interval");
    }
    Ok(maximum)
}

fn build_time_grid(
    reference: &[f64],
    prediction: &[f64],
    requested_step: Option<f64>,
) -> Result<TimeGrid, BenchError> {
    let reference_median_sample_interval_sec = median_sample_interval(reference)?;
    let prediction_median_sample_interval_sec = median_sample_interval(prediction)?;
    let reference_max_sample_interval_sec = maximum_sample_interval_sec(reference)?;
    let prediction_max_sample_interval_sec = maximum_sample_interval_sec(prediction)?;

    let start = reference[0].max(prediction[0]);
    let end = reference[reference.len() - 1].min(prediction[prediction.len() - 1]);
    let duration = end - start;
    if !duration.is_finite() || duration <= 0.0 {
        return fail("reference and prediction traces have no positive-duration time overlap");
    }

    let step = if let Some(step) = requested_step {
        if !step.is_finite() || step <= 0.0 {
            return fail("--step-sec must be finite and greater than zero");
        }
        step
    } else {
        // The coarser median avoids inventing high-frequency detail from the lower-rate trace.
        reference_median_sample_interval_sec.max(prediction_median_sample_interval_sec)
    };

    let ratio = duration / step;
    if !ratio.is_finite() || ratio < 0.0 || ratio > MAX_GRID_SAMPLES as f64 {
        return fail(format!(
            "time grid would exceed {MAX_GRID_SAMPLES} samples; choose a larger --step-sec"
        ));
    }
    let intervals = (ratio + 1.0e-9).floor() as usize;
    let sample_count = intervals.saturating_add(1);
    if sample_count > MAX_GRID_SAMPLES {
        return fail(format!("time grid exceeds {MAX_GRID_SAMPLES} samples"));
    }
    if sample_count < 4 {
        return fail(format!(
            "common time grid has {sample_count} samples; at least 4 are required for jerk estimation"
        ));
    }

    let mut grid = Vec::with_capacity(sample_count);
    for index in 0..sample_count {
        let mut timestamp = start + index as f64 * step;
        if (end - timestamp).abs() <= step * GRID_EPSILON_RATIO {
            timestamp = end;
        }
        if timestamp > end {
            break;
        }
        grid.push(timestamp);
    }
    if grid.len() < 4 {
        return fail("common time grid contains fewer than 4 usable samples");
    }

    Ok(TimeGrid {
        timestamps_sec: grid,
        step_sec: step,
        overlap_start_sec: start,
        overlap_end_sec: end,
        reference_median_sample_interval_sec,
        reference_max_sample_interval_sec,
        prediction_median_sample_interval_sec,
        prediction_max_sample_interval_sec,
    })
}

fn interpolate_positions(
    source_times: &[f64],
    source_positions: &[[f64; 3]],
    target_times: &[f64],
) -> Vec<[f64; 3]> {
    target_times
        .iter()
        .map(|target| interpolate_position(source_times, source_positions, *target))
        .collect()
}

fn quaternion_norm(q: [f64; 4]) -> f64 {
    q.iter()
        .fold(0.0_f64, |norm, component| norm.hypot(*component))
}

fn validate_unit_quaternion(q: [f64; 4]) -> Result<(), &'static str> {
    if q.iter().any(|component| !component.is_finite()) {
        return Err("contains a non-finite component");
    }
    let norm = quaternion_norm(q);
    if !norm.is_finite() || norm <= f64::MIN_POSITIVE {
        return Err("has a zero or invalid norm");
    }
    if (norm - 1.0).abs() > QUATERNION_NORM_TOLERANCE {
        return Err("must be unit length within 1e-3 norm tolerance");
    }
    Ok(())
}

fn normalize_quaternion(q: [f64; 4]) -> Result<[f64; 4], BenchError> {
    let norm = quaternion_norm(q);
    if !norm.is_finite() || norm <= f64::MIN_POSITIVE {
        return fail("cannot normalize a zero or invalid quaternion");
    }
    Ok([q[0] / norm, q[1] / norm, q[2] / norm, q[3] / norm])
}

fn quaternion_dot(a: [f64; 4], b: [f64; 4]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3]
}

/// Shortest-arc SLERP for unit quaternions in XYZW order.
fn slerp_quaternion(a: [f64; 4], b: [f64; 4], fraction: f64) -> Result<[f64; 4], BenchError> {
    let a = normalize_quaternion(a)?;
    let mut b = normalize_quaternion(b)?;
    let mut dot = quaternion_dot(a, b);
    if dot < 0.0 {
        b = [-b[0], -b[1], -b[2], -b[3]];
        dot = -dot;
    }
    dot = dot.clamp(-1.0, 1.0);
    let fraction = fraction.clamp(0.0, 1.0);

    let result = if dot > 0.9995 {
        [
            a[0] + fraction * (b[0] - a[0]),
            a[1] + fraction * (b[1] - a[1]),
            a[2] + fraction * (b[2] - a[2]),
            a[3] + fraction * (b[3] - a[3]),
        ]
    } else {
        let theta = dot.acos();
        let sin_theta = theta.sin();
        if !sin_theta.is_finite() || sin_theta.abs() <= f64::MIN_POSITIVE {
            return fail("SLERP encountered a degenerate interpolation arc");
        }
        let weight_a = ((1.0 - fraction) * theta).sin() / sin_theta;
        let weight_b = (fraction * theta).sin() / sin_theta;
        [
            weight_a * a[0] + weight_b * b[0],
            weight_a * a[1] + weight_b * b[1],
            weight_a * a[2] + weight_b * b[2],
            weight_a * a[3] + weight_b * b[3],
        ]
    };
    normalize_quaternion(result)
}

fn interpolate_quaternion(
    source_times: &[f64],
    source_rotations: &[[f64; 4]],
    target: f64,
) -> Result<[f64; 4], BenchError> {
    let upper = source_times.partition_point(|time| *time < target);
    if upper == 0 {
        return normalize_quaternion(source_rotations[0]);
    }
    if upper >= source_times.len() {
        return normalize_quaternion(source_rotations[source_rotations.len() - 1]);
    }
    let lower = upper - 1;
    let span = source_times[upper] - source_times[lower];
    let fraction = ((target - source_times[lower]) / span).clamp(0.0, 1.0);
    slerp_quaternion(source_rotations[lower], source_rotations[upper], fraction)
}

fn interpolate_quaternions(
    source_times: &[f64],
    source_rotations: &[[f64; 4]],
    target_times: &[f64],
) -> Result<Vec<[f64; 4]>, BenchError> {
    target_times
        .iter()
        .map(|target| interpolate_quaternion(source_times, source_rotations, *target))
        .collect()
}

/// Geodesic angular distance between 3D rotations, in radians. q and -q are
/// equivalent quaternion encodings and therefore produce the same error.
fn quaternion_angular_error_rad(a: [f64; 4], b: [f64; 4]) -> Result<f64, BenchError> {
    let a = normalize_quaternion(a)?;
    let b = normalize_quaternion(b)?;
    let error = 2.0 * quaternion_dot(a, b).abs().clamp(0.0, 1.0).acos();
    if error.is_finite() {
        Ok(error)
    } else {
        fail("quaternion angular error is not finite")
    }
}

/// Extract the Y-up yaw component from an XYZW quaternion. Full geodesic rotation
/// error remains the primary metric because yaw alone does not capture pitch or roll.
fn quaternion_yaw_rad(q: [f64; 4]) -> Result<Option<f64>, BenchError> {
    let q = normalize_quaternion(q)?;
    let [x, y, z, w] = q;
    let projected_forward_x = 2.0 * (w * y + x * z);
    let projected_forward_z = 1.0 - 2.0 * (y * y + x * x);
    if projected_forward_x.hypot(projected_forward_z) <= 1.0e-6 {
        // Heading is undefined when the character's forward vector is vertical.
        return Ok(None);
    }
    let yaw = projected_forward_x.atan2(projected_forward_z);
    if yaw.is_finite() {
        Ok(Some(yaw))
    } else {
        fail("quaternion yaw is not finite")
    }
}

fn wrapped_angle_difference_rad(a: f64, b: f64) -> f64 {
    let delta = a - b;
    delta.sin().atan2(delta.cos())
}

fn interpolate_position(
    source_times: &[f64],
    source_positions: &[[f64; 3]],
    target: f64,
) -> [f64; 3] {
    let upper = source_times.partition_point(|time| *time < target);
    if upper == 0 {
        return source_positions[0];
    }
    if upper >= source_times.len() {
        return source_positions[source_positions.len() - 1];
    }

    let lower = upper - 1;
    let span = source_times[upper] - source_times[lower];
    let fraction = ((target - source_times[lower]) / span).clamp(0.0, 1.0);
    let a = source_positions[lower];
    let b = source_positions[upper];
    [
        a[0] + (b[0] - a[0]) * fraction,
        a[1] + (b[1] - a[1]) * fraction,
        a[2] + (b[2] - a[2]) * fraction,
    ]
}

fn euclidean_position_error_m(a: [f64; 3], b: [f64; 3]) -> Result<f64, BenchError> {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    let error = (dx * dx + dy * dy + dz * dz).sqrt();
    if error.is_finite() {
        Ok(error)
    } else {
        fail("endpoint displacement error overflowed")
    }
}

fn sample_contact_nearest(
    source_times: &[f64],
    contacts: &[bool],
    target_times: &[f64],
) -> Vec<bool> {
    target_times
        .iter()
        .map(|target| {
            let upper = source_times.partition_point(|time| *time < target);
            if upper == 0 {
                return contacts[0];
            }
            if upper >= source_times.len() {
                return contacts[contacts.len() - 1];
            }
            let lower = upper - 1;
            if (*target - source_times[lower]) <= (source_times[upper] - *target) {
                contacts[lower]
            } else {
                contacts[upper]
            }
        })
        .collect()
}

/// Compare binary stance labels after each trace is aligned to the common time grid.
/// Undefined precision/recall are represented as null instead of silently coerced to zero.
fn contact_agreement(
    reference: &[bool],
    prediction: &[bool],
) -> Result<ContactAgreementReport, BenchError> {
    if reference.is_empty() {
        return fail("contact agreement requires at least one aligned sample");
    }
    if reference.len() != prediction.len() {
        return fail("contact agreement requires equal aligned sample counts");
    }

    let mut true_positive = 0usize;
    let mut true_negative = 0usize;
    let mut false_positive = 0usize;
    let mut false_negative = 0usize;
    for (expected, actual) in reference.iter().zip(prediction) {
        match (*expected, *actual) {
            (true, true) => true_positive += 1,
            (false, false) => true_negative += 1,
            (false, true) => false_positive += 1,
            (true, false) => false_negative += 1,
        }
    }

    let predicted_contact = true_positive + false_positive;
    let reference_contact = true_positive + false_negative;
    let precision = if predicted_contact > 0 {
        Some(true_positive as f64 / predicted_contact as f64)
    } else {
        None
    };
    let recall = if reference_contact > 0 {
        Some(true_positive as f64 / reference_contact as f64)
    } else {
        None
    };
    let f1_denominator = 2 * true_positive + false_positive + false_negative;
    let f1 = if f1_denominator > 0 {
        Some((2 * true_positive) as f64 / f1_denominator as f64)
    } else {
        None
    };

    Ok(ContactAgreementReport {
        samples: reference.len(),
        true_positive,
        true_negative,
        false_positive,
        false_negative,
        accuracy: (true_positive + true_negative) as f64 / reference.len() as f64,
        precision,
        recall,
        f1,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContactEventKind {
    Touchdown,
    Liftoff,
}

impl ContactEventKind {
    fn label(self) -> &'static str {
        match self {
            Self::Touchdown => "touchdown",
            Self::Liftoff => "liftoff",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ContactEvent {
    time_sec: f64,
    kind: ContactEventKind,
}

#[derive(Debug, Serialize)]
struct ContactEventClassReport {
    event_type: &'static str,
    matching_method: &'static str,
    tolerance_sec: f64,
    reference_events: usize,
    prediction_events: usize,
    matched_events: usize,
    missed_reference_events: usize,
    unmatched_prediction_events: usize,
    precision: Option<f64>,
    recall: Option<f64>,
    f1: Option<f64>,
    mean_signed_timing_error_sec: Option<f64>,
    median_absolute_timing_error_sec: Option<f64>,
    p95_absolute_timing_error_sec: Option<f64>,
    max_absolute_timing_error_sec: Option<f64>,
}

#[derive(Debug, Serialize)]
struct FootContactEventReport {
    touchdown: ContactEventClassReport,
    liftoff: ContactEventClassReport,
}

fn extract_contact_events(
    timestamps: &[f64],
    contacts: &[bool],
    overlap_start_sec: f64,
    overlap_end_sec: f64,
) -> Vec<ContactEvent> {
    let mut events = Vec::new();
    // The first sample's contact state is left-censored: without a prior sample,
    // we cannot claim that touchdown or liftoff occurred at the start timestamp.
    for index in 1..contacts.len() {
        let kind = match (contacts[index - 1], contacts[index]) {
            (false, true) => Some(ContactEventKind::Touchdown),
            (true, false) => Some(ContactEventKind::Liftoff),
            _ => None,
        };
        if let Some(kind) = kind {
            let time_sec = timestamps[index];
            // The shared interval's left boundary is excluded: event state before it may
            // be unavailable in one trace. The right endpoint is included.
            if time_sec > overlap_start_sec && time_sec <= overlap_end_sec {
                events.push(ContactEvent { time_sec, kind });
            }
        }
    }
    events
}

fn percentile_nearest_rank(sorted_values: &[f64], percentile: f64) -> Option<f64> {
    if sorted_values.is_empty() || !percentile.is_finite() || !(0.0..=1.0).contains(&percentile) {
        return None;
    }
    let index = ((percentile * sorted_values.len() as f64).ceil() as usize)
        .saturating_sub(1)
        .min(sorted_values.len() - 1);
    Some(sorted_values[index])
}

fn score_contact_event_class(
    event_type: ContactEventKind,
    reference_events: &[ContactEvent],
    prediction_events: &[ContactEvent],
    tolerance_sec: f64,
) -> ContactEventClassReport {
    let reference: Vec<f64> = reference_events
        .iter()
        .filter(|event| event.kind == event_type)
        .map(|event| event.time_sec)
        .collect();
    let prediction: Vec<f64> = prediction_events
        .iter()
        .filter(|event| event.kind == event_type)
        .map(|event| event.time_sec)
        .collect();

    // Inputs are timestamp-ordered. Pair each earliest eligible event at most once.
    // This deterministic chronological matcher is O(n + m), bounded for long recordings.
    let mut reference_index = 0usize;
    let mut prediction_index = 0usize;
    let mut signed_errors_sec = Vec::new();
    let mut missed_reference_events = 0usize;
    let mut unmatched_prediction_events = 0usize;
    while reference_index < reference.len() && prediction_index < prediction.len() {
        let delta = prediction[prediction_index] - reference[reference_index];
        if delta.abs() <= tolerance_sec {
            signed_errors_sec.push(delta);
            reference_index += 1;
            prediction_index += 1;
        } else if delta < -tolerance_sec {
            unmatched_prediction_events += 1;
            prediction_index += 1;
        } else {
            missed_reference_events += 1;
            reference_index += 1;
        }
    }
    missed_reference_events += reference.len() - reference_index;
    unmatched_prediction_events += prediction.len() - prediction_index;

    let matched_events = signed_errors_sec.len();
    let precision = if !prediction.is_empty() {
        Some(matched_events as f64 / prediction.len() as f64)
    } else {
        None
    };
    let recall = if !reference.is_empty() {
        Some(matched_events as f64 / reference.len() as f64)
    } else {
        None
    };
    let f1_denominator = 2 * matched_events + missed_reference_events + unmatched_prediction_events;
    let f1 = if f1_denominator > 0 {
        Some(2.0 * matched_events as f64 / f1_denominator as f64)
    } else {
        None
    };
    let mean_signed_timing_error_sec = if matched_events > 0 {
        Some(signed_errors_sec.iter().sum::<f64>() / matched_events as f64)
    } else {
        None
    };
    let mut absolute_errors_sec: Vec<f64> =
        signed_errors_sec.iter().map(|error| error.abs()).collect();
    absolute_errors_sec.sort_by(f64::total_cmp);
    let median_absolute_timing_error_sec = if absolute_errors_sec.is_empty() {
        None
    } else {
        let middle = absolute_errors_sec.len() / 2;
        Some(if absolute_errors_sec.len() % 2 == 0 {
            absolute_errors_sec[middle - 1]
                + (absolute_errors_sec[middle] - absolute_errors_sec[middle - 1]) / 2.0
        } else {
            absolute_errors_sec[middle]
        })
    };

    ContactEventClassReport {
        event_type: event_type.label(),
        matching_method: "chronological_first_eligible",
        tolerance_sec,
        reference_events: reference.len(),
        prediction_events: prediction.len(),
        matched_events,
        missed_reference_events,
        unmatched_prediction_events,
        precision,
        recall,
        f1,
        mean_signed_timing_error_sec,
        median_absolute_timing_error_sec,
        p95_absolute_timing_error_sec: percentile_nearest_rank(&absolute_errors_sec, 0.95),
        max_absolute_timing_error_sec: absolute_errors_sec.last().copied(),
    }
}

fn contact_event_report(
    reference_times: &[f64],
    reference_contacts: &[bool],
    prediction_times: &[f64],
    prediction_contacts: &[bool],
    overlap_start_sec: f64,
    overlap_end_sec: f64,
    tolerance_sec: f64,
) -> Result<FootContactEventReport, BenchError> {
    if !tolerance_sec.is_finite() || tolerance_sec <= 0.0 {
        return fail("contact-event tolerance must be finite and greater than zero");
    }
    if reference_times.len() != reference_contacts.len()
        || prediction_times.len() != prediction_contacts.len()
    {
        return fail("contact-event timestamps and labels must have equal lengths");
    }
    let reference_events = extract_contact_events(
        reference_times,
        reference_contacts,
        overlap_start_sec,
        overlap_end_sec,
    );
    let prediction_events = extract_contact_events(
        prediction_times,
        prediction_contacts,
        overlap_start_sec,
        overlap_end_sec,
    );

    Ok(FootContactEventReport {
        touchdown: score_contact_event_class(
            ContactEventKind::Touchdown,
            &reference_events,
            &prediction_events,
            tolerance_sec,
        ),
        liftoff: score_contact_event_class(
            ContactEventKind::Liftoff,
            &reference_events,
            &prediction_events,
            tolerance_sec,
        ),
    })
}

#[derive(Debug, Serialize)]
struct RootOrientationReport {
    samples: usize,
    mean_angular_error_rad: f64,
    mean_angular_error_deg: f64,
    final_angular_error_rad: f64,
    final_angular_error_deg: f64,
    p95_angular_error_rad: f64,
    p95_angular_error_deg: f64,
    max_angular_error_rad: f64,
    max_angular_error_deg: f64,
    yaw_samples_scored: usize,
    yaw_samples_undefined: usize,
    mean_signed_yaw_error_rad: Option<f64>,
    mean_signed_yaw_error_deg: Option<f64>,
    mean_absolute_yaw_error_rad: Option<f64>,
    mean_absolute_yaw_error_deg: Option<f64>,
    final_signed_yaw_error_rad: Option<f64>,
    final_signed_yaw_error_deg: Option<f64>,
    final_absolute_yaw_error_rad: Option<f64>,
    final_absolute_yaw_error_deg: Option<f64>,
    p95_absolute_yaw_error_rad: Option<f64>,
    p95_absolute_yaw_error_deg: Option<f64>,
    max_absolute_yaw_error_rad: Option<f64>,
    max_absolute_yaw_error_deg: Option<f64>,
}

fn evaluate_root_orientation(
    reference_times: &[f64],
    reference_rotations: &[[f64; 4]],
    prediction_times: &[f64],
    prediction_rotations: &[[f64; 4]],
    grid_times: &[f64],
    overlap_end_sec: f64,
) -> Result<RootOrientationReport, BenchError> {
    let reference_aligned =
        interpolate_quaternions(reference_times, reference_rotations, grid_times)?;
    let prediction_aligned =
        interpolate_quaternions(prediction_times, prediction_rotations, grid_times)?;

    let mut errors_rad = Vec::with_capacity(grid_times.len());
    let mut yaw_errors_rad = Vec::with_capacity(grid_times.len());
    for (reference, prediction) in reference_aligned.iter().zip(&prediction_aligned) {
        errors_rad.push(quaternion_angular_error_rad(*prediction, *reference)?);
        if let (Some(reference_yaw), Some(prediction_yaw)) = (
            quaternion_yaw_rad(*reference)?,
            quaternion_yaw_rad(*prediction)?,
        ) {
            yaw_errors_rad.push(wrapped_angle_difference_rad(prediction_yaw, reference_yaw));
        }
    }
    if errors_rad.is_empty() {
        return fail("root orientation evaluation requires at least one aligned sample");
    }
    let error_sum: f64 = errors_rad.iter().sum();
    let yaw_error_sum: f64 = yaw_errors_rad.iter().sum();
    let yaw_absolute_sum: f64 = yaw_errors_rad.iter().map(|error| error.abs()).sum();
    if !error_sum.is_finite() || !yaw_error_sum.is_finite() || !yaw_absolute_sum.is_finite() {
        return fail("root orientation error aggregation overflowed");
    }
    let mean_error_rad = error_sum / errors_rad.len() as f64;
    let yaw_samples_scored = yaw_errors_rad.len();
    let yaw_samples_undefined = grid_times.len().saturating_sub(yaw_samples_scored);
    let mean_signed_yaw_error_rad = if yaw_samples_scored > 0 {
        Some(yaw_error_sum / yaw_samples_scored as f64)
    } else {
        None
    };
    let mean_absolute_yaw_error_rad = if yaw_samples_scored > 0 {
        Some(yaw_absolute_sum / yaw_samples_scored as f64)
    } else {
        None
    };
    let mut ordered_errors = errors_rad;
    ordered_errors.sort_by(f64::total_cmp);
    let p95_error_rad = percentile_nearest_rank(&ordered_errors, 0.95)
        .ok_or_else(|| BenchError("could not compute root orientation P95".to_owned()))?;
    let mut ordered_yaw_errors: Vec<f64> = yaw_errors_rad.iter().map(|error| error.abs()).collect();
    ordered_yaw_errors.sort_by(f64::total_cmp);
    let p95_absolute_yaw_error_rad = percentile_nearest_rank(&ordered_yaw_errors, 0.95);

    let reference_endpoint =
        interpolate_quaternion(reference_times, reference_rotations, overlap_end_sec)?;
    let prediction_endpoint =
        interpolate_quaternion(prediction_times, prediction_rotations, overlap_end_sec)?;
    let final_error_rad = quaternion_angular_error_rad(prediction_endpoint, reference_endpoint)?;
    let final_reference_yaw = quaternion_yaw_rad(reference_endpoint)?;
    let final_prediction_yaw = quaternion_yaw_rad(prediction_endpoint)?;
    let final_signed_yaw_error_rad = match (final_prediction_yaw, final_reference_yaw) {
        (Some(prediction_yaw), Some(reference_yaw)) => {
            Some(wrapped_angle_difference_rad(prediction_yaw, reference_yaw))
        }
        _ => None,
    };
    let max_error_rad = ordered_errors.last().copied().unwrap_or(0.0);
    let max_absolute_yaw_error_rad = ordered_yaw_errors.last().copied();
    const RAD_TO_DEG: f64 = 180.0 / std::f64::consts::PI;

    Ok(RootOrientationReport {
        samples: grid_times.len(),
        mean_angular_error_rad: mean_error_rad,
        mean_angular_error_deg: mean_error_rad * RAD_TO_DEG,
        final_angular_error_rad: final_error_rad,
        final_angular_error_deg: final_error_rad * RAD_TO_DEG,
        p95_angular_error_rad: p95_error_rad,
        p95_angular_error_deg: p95_error_rad * RAD_TO_DEG,
        max_angular_error_rad: max_error_rad,
        max_angular_error_deg: max_error_rad * RAD_TO_DEG,
        yaw_samples_scored,
        yaw_samples_undefined,
        mean_signed_yaw_error_rad,
        mean_signed_yaw_error_deg: mean_signed_yaw_error_rad.map(|error| error * RAD_TO_DEG),
        mean_absolute_yaw_error_rad,
        mean_absolute_yaw_error_deg: mean_absolute_yaw_error_rad.map(|error| error * RAD_TO_DEG),
        final_signed_yaw_error_rad,
        final_signed_yaw_error_deg: final_signed_yaw_error_rad.map(|error| error * RAD_TO_DEG),
        final_absolute_yaw_error_rad: final_signed_yaw_error_rad.map(f64::abs),
        final_absolute_yaw_error_deg: final_signed_yaw_error_rad
            .map(|error| error.abs() * RAD_TO_DEG),
        p95_absolute_yaw_error_rad,
        p95_absolute_yaw_error_deg: p95_absolute_yaw_error_rad.map(|error| error * RAD_TO_DEG),
        max_absolute_yaw_error_rad,
        max_absolute_yaw_error_deg: max_absolute_yaw_error_rad.map(|error| error * RAD_TO_DEG),
    })
}

#[derive(Debug, Serialize)]
struct RootTrajectoryReport {
    samples: usize,
    mean_displacement_error_m: f64,
    final_displacement_error_m: f64,
    max_displacement_error_m: f64,
}

#[derive(Debug, Serialize)]
struct JointTrajectoryMetric {
    joint_name: String,
    samples: usize,
    mean_displacement_error_m: f64,
    final_displacement_error_m: f64,
    max_displacement_error_m: f64,
}

#[derive(Debug, Serialize)]
struct JointTrajectoryReport {
    joints_scored: usize,
    macro_mean_ade_m: Option<f64>,
    max_joint_fde_m: Option<f64>,
    max_joint_error_m: Option<f64>,
    per_joint: Vec<JointTrajectoryMetric>,
}

#[derive(Debug, Serialize)]
struct FootDriftReport {
    samples_in_contact: usize,
    rms_drift_m: f64,
    max_drift_m: f64,
}

#[derive(Debug, Serialize)]
struct ContactAgreementReport {
    samples: usize,
    true_positive: usize,
    true_negative: usize,
    false_positive: usize,
    false_negative: usize,
    accuracy: f64,
    precision: Option<f64>,
    recall: Option<f64>,
    f1: Option<f64>,
}

#[derive(Debug, Serialize)]
struct AlignmentReport {
    method: &'static str,
    overlap_start_sec: f64,
    overlap_end_sec: f64,
    overlap_duration_sec: f64,
    grid_step_sec: f64,
    reference_median_sample_interval_sec: f64,
    reference_max_sample_interval_sec: f64,
    prediction_median_sample_interval_sec: f64,
    prediction_max_sample_interval_sec: f64,
    samples: usize,
    invalid_samples: usize,
}

#[derive(Debug, Serialize)]
struct TraceIdentityReport {
    trace_id: String,
    trace_schema_version: u32,
    run: RunMetadata,
    skeleton_signature_sha256: String,
    retarget_map_sha256: Option<String>,
}

impl From<&Trace> for TraceIdentityReport {
    fn from(trace: &Trace) -> Self {
        Self {
            trace_id: trace.trace_id.clone(),
            trace_schema_version: trace.schema_version,
            run: trace.run.clone(),
            skeleton_signature_sha256: trace.skeleton_signature_sha256.clone(),
            retarget_map_sha256: trace.retarget_map_sha256.0.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ProvenanceReport {
    reference: TraceIdentityReport,
    prediction: TraceIdentityReport,
    configuration_hashes_match: bool,
    asset_hashes_match: bool,
}

#[derive(Debug, Serialize)]
struct BenchmarkReport {
    schema_version: u32,
    validity: &'static str,
    evidence_scope: &'static str,
    alignment: AlignmentReport,
    provenance: ProvenanceReport,
    root_trajectory_error: RootTrajectoryReport,
    root_orientation_error: Option<RootOrientationReport>,
    joint_trajectory_error: JointTrajectoryReport,
    left_stance_foot_drift: FootDriftReport,
    right_stance_foot_drift: FootDriftReport,
    left_foot_contact_agreement: ContactAgreementReport,
    right_foot_contact_agreement: ContactAgreementReport,
    left_foot_contact_events: FootContactEventReport,
    right_foot_contact_events: FootContactEventReport,
    prediction_root_mean_squared_jerk_m2_s6: f64,
    elapsed_metric_computation_ms: f64,
    caveats: Vec<&'static str>,
}

fn evaluate(
    reference: &Trace,
    prediction: &Trace,
    requested_step: Option<f64>,
) -> Result<BenchmarkReport, BenchError> {
    if reference.schema_version != prediction.schema_version {
        return fail(
            "trace schema versions differ; convert both traces to the same version before comparison",
        );
    }
    if reference.coordinate_frame != prediction.coordinate_frame {
        return fail("coordinate frames differ; convert both traces to the same declared frame");
    }
    if reference.skeleton_signature_sha256 != prediction.skeleton_signature_sha256 {
        return fail(
            "skeleton signature hashes differ; retarget both traces to the same canonical rig",
        );
    }
    if reference.joint_world_m.keys().collect::<Vec<_>>()
        != prediction.joint_world_m.keys().collect::<Vec<_>>()
    {
        return fail(
            "joint trajectory names differ; compare both traces on the same canonical joint set",
        );
    }
    if reference.retarget_map_sha256.0 != prediction.retarget_map_sha256.0 {
        return fail(
            "retarget-map hashes differ; do not compare traces built with incompatible retargeting",
        );
    }

    let timer = Instant::now();
    let grid = build_time_grid(
        &reference.timestamps_sec,
        &prediction.timestamps_sec,
        requested_step,
    )?;

    let reference_root = interpolate_positions(
        &reference.timestamps_sec,
        &reference.root_world_m,
        &grid.timestamps_sec,
    );
    let prediction_root = interpolate_positions(
        &prediction.timestamps_sec,
        &prediction.root_world_m,
        &grid.timestamps_sec,
    );
    let root = trajectory_errors(&prediction_root, &reference_root)
        .map_err(|error| BenchError(format!("root trajectory metric failed: {error}")))?;

    // ADE/max error use the uniform common grid; FDE is interpolated at the exact end
    // of the shared time interval even when the requested grid step does not divide it.
    let reference_endpoint = interpolate_position(
        &reference.timestamps_sec,
        &reference.root_world_m,
        grid.overlap_end_sec,
    );
    let prediction_endpoint = interpolate_position(
        &prediction.timestamps_sec,
        &prediction.root_world_m,
        grid.overlap_end_sec,
    );
    let endpoint_error_m = euclidean_position_error_m(prediction_endpoint, reference_endpoint)?;

    let root_orientation_error = match (
        reference.root_rotation_world_xyzw.as_deref(),
        prediction.root_rotation_world_xyzw.as_deref(),
    ) {
        (Some(reference_rotations), Some(prediction_rotations)) => Some(evaluate_root_orientation(
            &reference.timestamps_sec,
            reference_rotations,
            &prediction.timestamps_sec,
            prediction_rotations,
            &grid.timestamps_sec,
            grid.overlap_end_sec,
        )?),
        (None, None) => None,
        _ => {
            return fail(
                "both traces must provide root rotations when either trace has orientation data",
            );
        }
    };

    let mut joint_metrics = Vec::with_capacity(reference.joint_world_m.len());
    let mut joint_ade_sum_m = 0.0;
    let mut max_joint_fde_m: f64 = 0.0;
    let mut max_joint_error_m: f64 = 0.0;
    for (joint_name, reference_positions) in &reference.joint_world_m {
        let prediction_positions = prediction
            .joint_world_m
            .get(joint_name)
            .expect("joint-name sets were checked before scoring");
        let reference_aligned = interpolate_positions(
            &reference.timestamps_sec,
            reference_positions,
            &grid.timestamps_sec,
        );
        let prediction_aligned = interpolate_positions(
            &prediction.timestamps_sec,
            prediction_positions,
            &grid.timestamps_sec,
        );
        let metric =
            trajectory_errors(&prediction_aligned, &reference_aligned).map_err(|error| {
                BenchError(format!(
                    "joint {joint_name:?} trajectory metric failed: {error}"
                ))
            })?;
        let reference_joint_endpoint = interpolate_position(
            &reference.timestamps_sec,
            reference_positions,
            grid.overlap_end_sec,
        );
        let prediction_joint_endpoint = interpolate_position(
            &prediction.timestamps_sec,
            prediction_positions,
            grid.overlap_end_sec,
        );
        let joint_fde_m =
            euclidean_position_error_m(prediction_joint_endpoint, reference_joint_endpoint)?;
        joint_ade_sum_m += metric.mean_displacement_error_m;
        if !joint_ade_sum_m.is_finite() {
            return fail("joint ADE aggregation overflowed");
        }
        max_joint_fde_m = max_joint_fde_m.max(joint_fde_m);
        max_joint_error_m = max_joint_error_m.max(metric.max_displacement_error_m);
        joint_metrics.push(JointTrajectoryMetric {
            joint_name: joint_name.clone(),
            samples: metric.samples,
            mean_displacement_error_m: metric.mean_displacement_error_m,
            final_displacement_error_m: joint_fde_m,
            max_displacement_error_m: metric.max_displacement_error_m,
        });
    }
    let joints_scored = joint_metrics.len();
    let joint_trajectory_error = JointTrajectoryReport {
        joints_scored,
        macro_mean_ade_m: (joints_scored > 0).then_some(joint_ade_sum_m / joints_scored as f64),
        max_joint_fde_m: (joints_scored > 0).then_some(max_joint_fde_m),
        max_joint_error_m: (joints_scored > 0).then_some(max_joint_error_m),
        per_joint: joint_metrics,
    };

    let reference_left_contact = sample_contact_nearest(
        &reference.timestamps_sec,
        &reference.left_foot_contact,
        &grid.timestamps_sec,
    );
    let left_foot = interpolate_positions(
        &prediction.timestamps_sec,
        &prediction.left_foot_world_m,
        &grid.timestamps_sec,
    );
    let left_contact = sample_contact_nearest(
        &prediction.timestamps_sec,
        &prediction.left_foot_contact,
        &grid.timestamps_sec,
    );
    let left_contact_agreement = contact_agreement(&reference_left_contact, &left_contact)?;
    let left_drift = stance_foot_drift(&left_foot, &left_contact)
        .map_err(|error| BenchError(format!("left stance-foot metric failed: {error}")))?;

    let reference_right_contact = sample_contact_nearest(
        &reference.timestamps_sec,
        &reference.right_foot_contact,
        &grid.timestamps_sec,
    );
    let right_foot = interpolate_positions(
        &prediction.timestamps_sec,
        &prediction.right_foot_world_m,
        &grid.timestamps_sec,
    );
    let right_contact = sample_contact_nearest(
        &prediction.timestamps_sec,
        &prediction.right_foot_contact,
        &grid.timestamps_sec,
    );
    let right_contact_agreement = contact_agreement(&reference_right_contact, &right_contact)?;
    let right_drift = stance_foot_drift(&right_foot, &right_contact)
        .map_err(|error| BenchError(format!("right stance-foot metric failed: {error}")))?;

    // Event timing is measured at native source timestamps before grid resampling.
    let left_contact_events = contact_event_report(
        &reference.timestamps_sec,
        &reference.left_foot_contact,
        &prediction.timestamps_sec,
        &prediction.left_foot_contact,
        grid.overlap_start_sec,
        grid.overlap_end_sec,
        DEFAULT_CONTACT_EVENT_TOLERANCE_SEC,
    )?;
    let right_contact_events = contact_event_report(
        &reference.timestamps_sec,
        &reference.right_foot_contact,
        &prediction.timestamps_sec,
        &prediction.right_foot_contact,
        grid.overlap_start_sec,
        grid.overlap_end_sec,
        DEFAULT_CONTACT_EVENT_TOLERANCE_SEC,
    )?;

    let jerk = mean_squared_jerk(&prediction_root, &grid.timestamps_sec)
        .map_err(|error| BenchError(format!("prediction jerk metric failed: {error}")))?;
    let elapsed_metric_computation_ms = timer.elapsed().as_secs_f64() * 1000.0;

    let synthetic =
        reference.run.source_kind.is_synthetic() || prediction.run.source_kind.is_synthetic();
    let mut caveats = vec![
        "Low-level trajectory and stance-foot metrics do not prove visual, biomechanical, or behavioral realism.",
        "Foot drift is measured only over predicted contact intervals; inspect contact precision, recall and F1 because incorrect labels can make drift look deceptively good.",
        "Touchdown/liftoff errors use native source timestamps before grid resampling; event extraction excludes the shared interval's left boundary because it is left-censored, and includes the right boundary.",
        "Root jerk is noise-sensitive. Interpret it with tracking error and task success, never as a standalone quality score.",
        "ADE and maximum error use the common uniform grid; FDE is evaluated at the exact overlap end by position interpolation.",
        "Root-path error alone is not a full-body fidelity score; inspect named-joint trajectory metrics when the trace includes them.",
        "Root orientation is only scored for v0.2 traces with quaternion samples; a null orientation report means the v0.1 contract did not provide orientation, not that orientation error was zero.",
        "Configuration hashes may differ between methods by design; both are recorded so the comparison is auditable.",
        "Evidence scope is derived from declared trace metadata and is not independent authentication of consent, legal rights, dataset provenance, or source-file integrity.",
    ];
    let validity = if synthetic {
        caveats.push("At least one trace is a synthetic fixture; this report is a software test, not evidence about human motion.");
        "synthetic_fixture_only"
    } else if joints_scored == 0 {
        caveats.push("Neither trace contains named joint trajectories; this is a root/contact-only comparison, not a whole-body pose evaluation.");
        "provenance_complete_root_only"
    } else {
        "provenance_complete_comparison"
    };

    Ok(BenchmarkReport {
        schema_version: 2,
        validity,
        evidence_scope: evidence_scope(reference, prediction),
        alignment: AlignmentReport {
            method: "linear_position_interpolation_and_nearest_neighbor_contact_on_uniform_overlap_grid",
            overlap_start_sec: grid.overlap_start_sec,
            overlap_end_sec: grid.overlap_end_sec,
            overlap_duration_sec: grid.overlap_end_sec - grid.overlap_start_sec,
            grid_step_sec: grid.step_sec,
            reference_median_sample_interval_sec: grid.reference_median_sample_interval_sec,
            reference_max_sample_interval_sec: grid.reference_max_sample_interval_sec,
            prediction_median_sample_interval_sec: grid.prediction_median_sample_interval_sec,
            prediction_max_sample_interval_sec: grid.prediction_max_sample_interval_sec,
            samples: grid.timestamps_sec.len(),
            // Invalid samples are rejected before this stage; they are never silently dropped.
            invalid_samples: 0,
        },
        provenance: ProvenanceReport {
            reference: TraceIdentityReport::from(reference),
            prediction: TraceIdentityReport::from(prediction),
            configuration_hashes_match: reference.run.configuration_sha256
                == prediction.run.configuration_sha256,
            // Two absent hashes are not evidence that assets match.
            asset_hashes_match: reference
                .run
                .asset_sha256
                .as_ref()
                .zip(prediction.run.asset_sha256.as_ref())
                .is_some_and(|(left, right)| left == right),
        },
        root_trajectory_error: RootTrajectoryReport {
            samples: root.samples,
            mean_displacement_error_m: root.mean_displacement_error_m,
            final_displacement_error_m: endpoint_error_m,
            max_displacement_error_m: root.max_displacement_error_m,
        },
        root_orientation_error,
        joint_trajectory_error,
        left_stance_foot_drift: FootDriftReport {
            samples_in_contact: left_drift.samples_in_contact,
            rms_drift_m: left_drift.rms_drift_m,
            max_drift_m: left_drift.max_drift_m,
        },
        right_stance_foot_drift: FootDriftReport {
            samples_in_contact: right_drift.samples_in_contact,
            rms_drift_m: right_drift.rms_drift_m,
            max_drift_m: right_drift.max_drift_m,
        },
        left_foot_contact_agreement: left_contact_agreement,
        right_foot_contact_agreement: right_contact_agreement,
        left_foot_contact_events: left_contact_events,
        right_foot_contact_events: right_contact_events,
        prediction_root_mean_squared_jerk_m2_s6: jerk,
        elapsed_metric_computation_ms,
        caveats,
    })
}

fn print_usage() {
    eprintln!(
        "Usage: human-motion-bench <reference.json> <prediction.json> [--step-sec SECONDS] [--output report.json]\n\
         \n\
         Trace contracts: docs/research/human-motion-trace-v0.1.schema.json (legacy) and docs/research/human-motion-trace-v0.2.schema.json (orientation)\n\
         Metrics: root/joint error, contact agreement/events, root orientation, and root jerk.\n\
         Without --step-sec the coarser of the two median input sample intervals is used.\n\
         Synthetic traces are software-validation only. Use --require-held-out-human-reference for a strict held-out human-capture claim."
    );
}

fn run_cli() -> Result<(), Box<dyn Error>> {
    let mut positional: Vec<PathBuf> = Vec::new();
    let mut requested_step: Option<f64> = None;
    let mut output_path: Option<PathBuf> = None;
    let mut require_held_out_human_reference = false;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            "--step-sec" => {
                let value = args.next().ok_or_else(|| {
                    BenchError("--step-sec requires a positive numeric value".to_owned())
                })?;
                requested_step = Some(
                    value
                        .parse::<f64>()
                        .map_err(|_| BenchError(format!("invalid --step-sec value {value:?}")))?,
                );
            }
            "--output" => {
                let value = args
                    .next()
                    .ok_or_else(|| BenchError("--output requires a path".to_owned()))?;
                output_path = Some(PathBuf::from(value));
            }
            "--require-held-out-human-reference" => {
                require_held_out_human_reference = true;
            }
            _ if arg.starts_with('-') => {
                return Err(Box::new(BenchError(format!(
                    "unknown option {arg:?}; use --help for usage"
                ))));
            }
            _ => positional.push(PathBuf::from(arg)),
        }
    }

    if positional.len() != 2 {
        print_usage();
        return Err(Box::new(BenchError(
            "expected a reference and a prediction trace path".to_owned(),
        )));
    }

    let reference = read_trace(&positional[0])?;
    let prediction = read_trace(&positional[1])?;
    if require_held_out_human_reference {
        validate_human_reference_claim(&reference, &prediction)?;
    }
    let report = evaluate(&reference, &prediction, requested_step)?;
    let json = serde_json::to_string_pretty(&report)?;

    let joint_ade = report
        .joint_trajectory_error
        .macro_mean_ade_m
        .map_or_else(|| "n/a".to_owned(), |value| format!("{value:.4} m"));
    let joint_fde = report
        .joint_trajectory_error
        .max_joint_fde_m
        .map_or_else(|| "n/a".to_owned(), |value| format!("{value:.4} m"));
    eprintln!(
        concat!(
            "Motion benchmark: validity={}, evidence_scope={}, samples={}, joints_scored={}, ",
            "source max interval ref/pred={:.1}/{:.1} ms, ",
            "root ADE={:.4} m, FDE={:.4} m, max={:.4} m, ",
            "macro joint ADE={}, max joint FDE={}, ",
            "left-foot drift RMS={:.4} m (contact F1={}), ",
            "right-foot drift RMS={:.4} m (contact F1={}), ",
            "event median abs L touchdown/liftoff={}/{}, R touchdown/liftoff={}/{}, ",
            "root orient P95 (rad/deg)={}/{}, yaw P95 (rad/deg)={}/{}, root jerk={:.4} m²/s⁶"
        ),
        report.validity,
        report.evidence_scope,
        report.alignment.samples,
        report.joint_trajectory_error.joints_scored,
        report.alignment.reference_max_sample_interval_sec * 1000.0,
        report.alignment.prediction_max_sample_interval_sec * 1000.0,
        report.root_trajectory_error.mean_displacement_error_m,
        report.root_trajectory_error.final_displacement_error_m,
        report.root_trajectory_error.max_displacement_error_m,
        joint_ade,
        joint_fde,
        report.left_stance_foot_drift.rms_drift_m,
        report
            .left_foot_contact_agreement
            .f1
            .map_or_else(|| "n/a".to_owned(), |value| format!("{value:.3}")),
        report.right_stance_foot_drift.rms_drift_m,
        report
            .right_foot_contact_agreement
            .f1
            .map_or_else(|| "n/a".to_owned(), |value| format!("{value:.3}")),
        report
            .left_foot_contact_events
            .touchdown
            .median_absolute_timing_error_sec
            .map_or_else(|| "n/a".to_owned(), |value| format!("{value:.3}s")),
        report
            .left_foot_contact_events
            .liftoff
            .median_absolute_timing_error_sec
            .map_or_else(|| "n/a".to_owned(), |value| format!("{value:.3}s")),
        report
            .right_foot_contact_events
            .touchdown
            .median_absolute_timing_error_sec
            .map_or_else(|| "n/a".to_owned(), |value| format!("{value:.3}s")),
        report
            .right_foot_contact_events
            .liftoff
            .median_absolute_timing_error_sec
            .map_or_else(|| "n/a".to_owned(), |value| format!("{value:.3}s")),
        report.root_orientation_error.as_ref().map_or_else(
            || "n/a".to_owned(),
            |value| format!("{:.2}", value.p95_angular_error_rad),
        ),
        report.root_orientation_error.as_ref().map_or_else(
            || "n/a".to_owned(),
            |value| format!("{:.2}", value.p95_angular_error_deg),
        ),
        report
            .root_orientation_error
            .as_ref()
            .and_then(|value| value.p95_absolute_yaw_error_rad)
            .map_or_else(|| "n/a".to_owned(), |value| format!("{value:.2}")),
        report
            .root_orientation_error
            .as_ref()
            .and_then(|value| value.p95_absolute_yaw_error_deg)
            .map_or_else(|| "n/a".to_owned(), |value| format!("{value:.2}")),
        report.prediction_root_mean_squared_jerk_m2_s6
    );

    if let Some(path) = output_path {
        fs::write(&path, json)?;
        eprintln!("JSON report written to {}", path.display());
    } else {
        println!("{json}");
    }
    Ok(())
}

fn main() {
    if let Err(error) = run_cli() {
        eprintln!("human-motion-bench: {error}");
        process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Trace {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/research/fixtures/human-motion-trace-synthetic-v0.1.json");
        read_trace(&path).expect("the committed synthetic trace fixture should validate")
    }

    fn orientation_fixture() -> Trace {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../../docs/research/fixtures/human-motion-trace-orientation-synthetic-v0.2.json",
        );
        read_trace(&path).expect("the committed v0.2 orientation fixture should validate")
    }

    fn held_out_human_reference_pair() -> (Trace, Trace) {
        let mut reference = orientation_fixture();
        let mut prediction = orientation_fixture();

        reference.trace_id = "consented-test-reference-001".to_owned();
        prediction.trace_id = "candidate-run-001".to_owned();

        reference.run.source_kind = SourceKind::OriginalConsentCapture;
        reference.run.git_commit = "a".repeat(40);
        reference.run.asset_sha256 = Some("b".repeat(64));
        reference.run.asset_license =
            Some("documented consent and evaluation authorization".to_owned());
        reference.retarget_map_sha256.0 = Some("c".repeat(64));
        reference.run.split = Some("test".to_owned());
        reference.run.dataset_id = Some("consented-dataset".to_owned());
        reference.run.dataset_version = Some("v1".to_owned());
        reference.run.actor_pseudonym = Some("actor-pseudonym-007".to_owned());
        reference.run.recording_id = Some("session-2026-01".to_owned());

        prediction.run.source_kind = SourceKind::ExternalModel;
        prediction.run.split = Some("test".to_owned());
        prediction.run.git_commit = "d".repeat(40);
        prediction.run.asset_sha256 = Some("e".repeat(64));
        prediction.run.asset_license = Some("model evaluation license".to_owned());
        prediction.run.model_sha256 = Some("1".repeat(64));
        prediction.retarget_map_sha256.0 = Some("c".repeat(64));

        (reference, prediction)
    }

    #[test]
    fn evidence_scope_distinguishes_synthetic_checks_from_held_out_human_references() {
        let synthetic = orientation_fixture();
        assert_eq!(
            evidence_scope(&synthetic, &synthetic),
            "synthetic_software_validation_only"
        );

        let (reference, prediction) = held_out_human_reference_pair();
        reference.validate(Path::new("reference.json")).unwrap();
        prediction.validate(Path::new("prediction.json")).unwrap();
        validate_human_reference_claim(&reference, &prediction).unwrap();
        let report = evaluate(&reference, &prediction, None).unwrap();
        assert_eq!(report.evidence_scope, "declared_held_out_human_reference");
        assert!(
            report
                .caveats
                .iter()
                .any(|value| value.contains("not independent authentication"))
        );
    }

    #[test]
    fn held_out_human_reference_gate_rejects_training_split_and_missing_identifiers() {
        let (mut reference, prediction) = held_out_human_reference_pair();
        reference.run.split = Some("train".to_owned());
        assert!(
            validate_human_reference_claim(&reference, &prediction)
                .unwrap_err()
                .to_string()
                .contains("split = test")
        );

        let (mut reference, prediction) = held_out_human_reference_pair();
        reference.run.recording_id = None;
        assert!(
            validate_human_reference_claim(&reference, &prediction)
                .unwrap_err()
                .to_string()
                .contains("recording_id")
        );
    }

    #[test]
    fn held_out_human_reference_gate_rejects_synthetic_prediction_and_same_trace_id() {
        let (reference, mut prediction) = held_out_human_reference_pair();
        prediction.run.source_kind = SourceKind::SyntheticTestFixture;
        assert!(
            validate_human_reference_claim(&reference, &prediction)
                .unwrap_err()
                .to_string()
                .contains("cannot use a synthetic prediction")
        );

        let (reference, mut prediction) = held_out_human_reference_pair();
        prediction.trace_id = reference.trace_id.clone();
        assert!(
            validate_human_reference_claim(&reference, &prediction)
                .unwrap_err()
                .to_string()
                .contains("distinct reference and prediction")
        );
    }

    #[test]
    fn committed_orientation_fixture_has_zero_error_against_itself() {
        let trace = orientation_fixture();
        assert_eq!(trace.schema_version, 2);
        let report =
            evaluate(&trace, &trace, None).expect("identical orientation fixtures should compare");
        assert_eq!(report.schema_version, 2);
        let orientation = report
            .root_orientation_error
            .expect("v0.2 traces include root orientation");
        assert_eq!(orientation.samples, 4);
        assert_eq!(orientation.mean_angular_error_rad, 0.0);
        assert_eq!(orientation.final_angular_error_rad, 0.0);
        assert_eq!(orientation.p95_angular_error_deg, 0.0);
        assert_eq!(orientation.max_angular_error_deg, 0.0);
        assert_eq!(report.validity, "synthetic_fixture_only");
    }

    #[test]
    fn quaternion_geodesic_metric_detects_ninety_degree_yaw_error() {
        let reference = orientation_fixture();
        let mut prediction = orientation_fixture();
        let half_angle = std::f64::consts::FRAC_PI_4;
        let yaw_90 = [0.0, half_angle.sin(), 0.0, half_angle.cos()];
        prediction.root_rotation_world_xyzw = Some(vec![yaw_90; prediction.timestamps_sec.len()]);

        let report = evaluate(&reference, &prediction, None)
            .expect("valid quaternion traces should compare");
        let orientation = report.root_orientation_error.unwrap();
        assert!((orientation.mean_angular_error_rad - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        assert!((orientation.p95_angular_error_deg - 90.0).abs() < 1e-7);
        assert!((orientation.max_angular_error_deg - 90.0).abs() < 1e-7);
        assert!((orientation.mean_absolute_yaw_error_deg.unwrap() - 90.0).abs() < 1e-7);
        assert!((orientation.final_absolute_yaw_error_deg.unwrap() - 90.0).abs() < 1e-7);
    }

    #[test]
    fn yaw_difference_wraps_across_the_pi_boundary_and_vertical_heading_is_undefined() {
        let degrees = std::f64::consts::PI / 180.0;
        let yaw_179 = quaternion_yaw_rad([
            0.0,
            (179.0 * degrees / 2.0).sin(),
            0.0,
            (179.0 * degrees / 2.0).cos(),
        ])
        .unwrap()
        .unwrap();
        let yaw_minus_179 = quaternion_yaw_rad([
            0.0,
            (-179.0 * degrees / 2.0).sin(),
            0.0,
            (-179.0 * degrees / 2.0).cos(),
        ])
        .unwrap()
        .unwrap();
        let wrapped = wrapped_angle_difference_rad(yaw_179, yaw_minus_179);
        assert!((wrapped.abs() / degrees - 2.0).abs() < 1e-7);

        let half_angle = std::f64::consts::FRAC_PI_4;
        let forward_vertical = [half_angle.sin(), 0.0, 0.0, half_angle.cos()];
        assert_eq!(quaternion_yaw_rad(forward_vertical).unwrap(), None);
    }

    #[test]
    fn orientation_fde_is_evaluated_at_the_exact_overlap_endpoint() {
        let mut reference = orientation_fixture();
        let mut prediction = orientation_fixture();
        let times = vec![0.0, 0.1, 0.2, 0.3, 0.35];
        let identity = [0.0, 0.0, 0.0, 1.0];
        let yaw_90 = [
            0.0,
            std::f64::consts::FRAC_PI_4.sin(),
            0.0,
            std::f64::consts::FRAC_PI_4.cos(),
        ];

        for trace in [&mut reference, &mut prediction] {
            trace.timestamps_sec = times.clone();
            trace.root_world_m = times.iter().map(|time| [*time, 0.0, 0.0]).collect();
            trace.left_foot_world_m = vec![[0.0, 0.0, 0.0]; times.len()];
            trace.right_foot_world_m = vec![[0.0, 0.0, 0.0]; times.len()];
            trace.left_foot_contact = vec![true; times.len()];
            trace.right_foot_contact = vec![true; times.len()];
        }
        reference.root_rotation_world_xyzw = Some(vec![identity; times.len()]);
        prediction.root_rotation_world_xyzw =
            Some(vec![identity, identity, identity, identity, yaw_90]);

        let report = evaluate(&reference, &prediction, None)
            .expect("valid traces with a non-grid-aligned endpoint should compare");
        let orientation = report.root_orientation_error.unwrap();
        assert_eq!(report.alignment.samples, 4);
        assert_eq!(orientation.mean_angular_error_rad, 0.0);
        assert!((orientation.final_angular_error_deg - 90.0).abs() < 1e-7);
        assert!((orientation.final_absolute_yaw_error_deg.unwrap() - 90.0).abs() < 1e-7);
        assert_eq!(orientation.max_angular_error_deg, 0.0);
    }

    #[test]
    fn quaternion_sign_equivalence_and_shortest_arc_slerp_are_respected() {
        let identity = [0.0, 0.0, 0.0, 1.0];
        let negative_identity = [0.0, 0.0, 0.0, -1.0];
        assert_eq!(
            quaternion_angular_error_rad(identity, negative_identity).unwrap(),
            0.0
        );
        let midpoint = slerp_quaternion(identity, negative_identity, 0.5).unwrap();
        assert!(quaternion_angular_error_rad(identity, midpoint).unwrap() < 1e-9);
    }

    #[test]
    fn v2_schema_rejects_invalid_quaternion_norm_and_wrong_sample_count() {
        let mut trace = orientation_fixture();
        trace.root_rotation_world_xyzw.as_mut().unwrap()[0] = [0.0, 0.0, 0.0, 0.0];
        assert!(
            trace
                .validate(Path::new("zero-quaternion.json"))
                .unwrap_err()
                .to_string()
                .contains("zero or invalid norm")
        );

        let mut trace = orientation_fixture();
        trace.root_rotation_world_xyzw.as_mut().unwrap()[0] = [0.0, 0.0, 0.0, 2.0];
        assert!(
            trace
                .validate(Path::new("non-unit-quaternion.json"))
                .unwrap_err()
                .to_string()
                .contains("unit length")
        );

        let mut trace = orientation_fixture();
        trace.root_rotation_world_xyzw.as_mut().unwrap().pop();
        assert!(
            trace
                .validate(Path::new("short-quaternion-array.json"))
                .unwrap_err()
                .to_string()
                .contains("root_rotation_world_xyzw has")
        );

        let mut trace = orientation_fixture();
        trace.root_rotation_world_xyzw.as_mut().unwrap()[0] = [f64::NAN, 0.0, 0.0, 1.0];
        assert!(
            trace
                .validate(Path::new("non-finite-quaternion.json"))
                .unwrap_err()
                .to_string()
                .contains("non-finite")
        );

        let mut trace = orientation_fixture();
        trace.root_rotation_world_xyzw = None;
        assert!(
            trace
                .validate(Path::new("missing-v2-quaternion.json"))
                .unwrap_err()
                .to_string()
                .contains("requires root_rotation_world_xyzw")
        );
    }

    #[test]
    fn yaw_metrics_are_null_when_forward_heading_is_vertical_for_the_whole_trace() {
        let mut trace = orientation_fixture();
        let half_angle = std::f64::consts::FRAC_PI_4;
        let vertical = [half_angle.sin(), 0.0, 0.0, half_angle.cos()];
        trace.root_rotation_world_xyzw = Some(vec![vertical; trace.timestamps_sec.len()]);

        let report = evaluate(&trace, &trace, None)
            .expect("full rotation error remains measurable when yaw is undefined");
        let orientation = report.root_orientation_error.unwrap();
        assert_eq!(orientation.samples, 4);
        assert_eq!(orientation.yaw_samples_scored, 0);
        assert_eq!(orientation.yaw_samples_undefined, 4);
        assert_eq!(orientation.mean_absolute_yaw_error_rad, None);
        assert_eq!(orientation.p95_absolute_yaw_error_rad, None);
        assert_eq!(orientation.final_absolute_yaw_error_rad, None);
    }

    #[test]
    fn trace_versions_must_match_and_v1_cannot_smuggle_orientation() {
        let legacy = fixture();
        let v2 = orientation_fixture();
        assert!(
            evaluate(&legacy, &v2, None)
                .unwrap_err()
                .to_string()
                .contains("schema versions differ")
        );

        let mut legacy_with_rotation = legacy;
        legacy_with_rotation.root_rotation_world_xyzw = Some(vec![[0.0, 0.0, 0.0, 1.0]; 4]);
        assert!(
            legacy_with_rotation
                .validate(Path::new("illegal-v1-orientation.json"))
                .unwrap_err()
                .to_string()
                .contains("requires schema_version 2")
        );
    }

    #[test]
    fn committed_synthetic_fixture_validates_and_is_labelled_non_evidence() {
        let trace = fixture();
        assert_eq!(trace.trace_id, "synthetic-straight-line-smoke-001");
        let report = evaluate(&trace, &trace, None).expect("identical fixtures should compare");
        assert_eq!(report.validity, "synthetic_fixture_only");
        assert_eq!(report.alignment.samples, 4);
        assert_eq!(report.root_trajectory_error.mean_displacement_error_m, 0.0);
        assert_eq!(report.root_trajectory_error.final_displacement_error_m, 0.0);
        assert_eq!(report.left_stance_foot_drift.max_drift_m, 0.0);
        assert_eq!(report.right_stance_foot_drift.max_drift_m, 0.0);
        assert_eq!(report.left_foot_contact_agreement.f1, Some(1.0));
        assert_eq!(report.right_foot_contact_agreement.f1, Some(1.0));
        assert!(report.root_orientation_error.is_none());
        let serialized = serde_json::to_value(&report).unwrap();
        assert_eq!(serialized["schema_version"], 2);
        assert_eq!(
            serialized["evidence_scope"],
            "synthetic_software_validation_only"
        );
        assert!(serialized["root_orientation_error"].is_null());
        assert!(!report.provenance.asset_hashes_match);
        assert!(
            report
                .caveats
                .iter()
                .any(|value| value.contains("not evidence"))
        );
    }

    #[test]
    fn contact_negative_control_detects_mistimed_stance_labels() {
        let reference = fixture();
        let mut prediction = fixture();
        // Invert left-foot support labels while leaving the foot trajectory unchanged.
        // The drift metric can still look perfect for a stationary foot; contact F1 cannot.
        prediction.left_foot_contact = vec![false, false, true, true];

        let report =
            evaluate(&reference, &prediction, None).expect("contact labels remain evaluable");
        let contact = &report.left_foot_contact_agreement;
        assert_eq!(contact.true_positive, 0);
        assert_eq!(contact.true_negative, 0);
        assert_eq!(contact.false_positive, 2);
        assert_eq!(contact.false_negative, 2);
        assert_eq!(contact.accuracy, 0.0);
        assert_eq!(contact.precision, Some(0.0));
        assert_eq!(contact.recall, Some(0.0));
        assert_eq!(contact.f1, Some(0.0));
        assert_eq!(report.left_stance_foot_drift.rms_drift_m, 0.0);
    }

    #[test]
    fn contact_agreement_represents_undefined_positive_class_metrics_as_null() {
        let report =
            contact_agreement(&[false, false], &[false, false]).expect("aligned labels compare");
        assert_eq!(report.accuracy, 1.0);
        assert_eq!(report.precision, None);
        assert_eq!(report.recall, None);
        assert_eq!(report.f1, None);
    }

    #[test]
    fn rejects_contact_label_length_mismatch() {
        assert!(
            contact_agreement(&[true, false], &[true])
                .unwrap_err()
                .to_string()
                .contains("equal aligned sample counts")
        );
    }

    #[test]
    fn contact_event_timing_recovers_a_fifty_millisecond_shift() {
        let reference_times = [0.0, 0.1, 0.2, 0.3, 0.4, 0.5];
        let prediction_times = [0.0, 0.1, 0.25, 0.35, 0.45, 0.5];
        let contacts = [false, false, true, true, false, false];
        let report = contact_event_report(
            &reference_times,
            &contacts,
            &prediction_times,
            &contacts,
            0.0,
            0.5,
            0.06,
        )
        .expect("valid event sequences should compare");

        assert_eq!(report.touchdown.matched_events, 1);
        assert_eq!(report.liftoff.matched_events, 1);
        assert!((report.alignment.reference_max_sample_interval_sec - 0.1).abs() < 1e-9);
        assert!((report.alignment.prediction_max_sample_interval_sec - 0.15).abs() < 1e-9);
        assert!((report.touchdown.mean_signed_timing_error_sec.unwrap() - 0.05).abs() < 1e-9);
        assert!((report.liftoff.mean_signed_timing_error_sec.unwrap() - 0.05).abs() < 1e-9);
        assert!((report.touchdown.median_absolute_timing_error_sec.unwrap() - 0.05).abs() < 1e-9);
        assert!((report.liftoff.p95_absolute_timing_error_sec.unwrap() - 0.05).abs() < 1e-9);
    }

    #[test]
    fn contact_event_matching_is_one_to_one_and_counts_duplicate_detections() {
        let reference = vec![ContactEvent {
            time_sec: 0.1,
            kind: ContactEventKind::Touchdown,
        }];
        let prediction = vec![
            ContactEvent {
                time_sec: 0.08,
                kind: ContactEventKind::Touchdown,
            },
            ContactEvent {
                time_sec: 0.11,
                kind: ContactEventKind::Touchdown,
            },
        ];
        let report =
            score_contact_event_class(ContactEventKind::Touchdown, &reference, &prediction, 0.05);
        assert_eq!(report.matched_events, 1);
        assert_eq!(report.unmatched_prediction_events, 1);
        assert_eq!(report.missed_reference_events, 0);
        assert_eq!(report.precision, Some(0.5));
        assert_eq!(report.recall, Some(1.0));
    }

    #[test]
    fn contact_event_extraction_uses_native_times_and_clips_to_overlap() {
        let times = [0.0, 0.1, 0.2, 0.3, 0.4];
        let contacts = [false, false, true, true, false];
        let events = extract_contact_events(&times, &contacts, 0.15, 0.3);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, ContactEventKind::Touchdown);
        assert_eq!(events[0].time_sec, 0.2);
    }

    #[test]
    fn contact_event_metrics_fail_closed_on_invalid_tolerance_or_lengths() {
        assert!(
            contact_event_report(&[0.0], &[false], &[0.0], &[false], 0.0, 0.0, 0.0)
                .unwrap_err()
                .to_string()
                .contains("tolerance")
        );
        assert!(
            contact_event_report(&[0.0, 0.1], &[false], &[0.0], &[false], 0.0, 0.1, 0.05)
                .unwrap_err()
                .to_string()
                .contains("equal lengths")
        );
    }

    #[test]
    fn negative_control_detects_a_five_centimetre_root_offset() {
        let reference = fixture();
        let mut prediction = fixture();
        for position in &mut prediction.root_world_m {
            position[0] += 0.05;
        }
        let report = evaluate(&reference, &prediction, None).expect("valid traces should compare");
        assert!((report.root_trajectory_error.mean_displacement_error_m - 0.05).abs() < 1e-9);
        assert!((report.root_trajectory_error.final_displacement_error_m - 0.05).abs() < 1e-9);
        assert!((report.root_trajectory_error.max_displacement_error_m - 0.05).abs() < 1e-9);
    }

    #[test]
    fn final_displacement_error_uses_exact_overlap_endpoint() {
        let mut reference = fixture();
        let mut prediction = fixture();
        let times = vec![0.0, 0.1, 0.2, 0.3, 0.35];

        reference.timestamps_sec = times.clone();
        prediction.timestamps_sec = times.clone();
        reference.root_world_m = times.iter().map(|time| [*time, 0.0, 0.0]).collect();
        prediction.root_world_m = reference.root_world_m.clone();
        prediction.root_world_m[4][0] += 0.05;

        for trace in [&mut reference, &mut prediction] {
            trace.left_foot_world_m = vec![[0.0, 0.0, 0.0]; 5];
            trace.right_foot_world_m = vec![[0.0, 0.0, 0.0]; 5];
            trace.left_foot_contact = vec![true; 5];
            trace.right_foot_contact = vec![true; 5];
        }

        let report = evaluate(&reference, &prediction, None)
            .expect("valid traces with a 350 ms interval should compare");
        assert_eq!(report.alignment.samples, 4);
        assert_eq!(report.root_trajectory_error.mean_displacement_error_m, 0.0);
        assert_eq!(report.root_trajectory_error.max_displacement_error_m, 0.0);
        assert!((report.root_trajectory_error.final_displacement_error_m - 0.05).abs() < 1e-9);
    }

    #[test]
    fn scores_named_joint_trajectories_separately_from_root() {
        let mut reference = fixture();
        let mut prediction = fixture();
        let positions = vec![[0.0, 1.0, 0.0]; 4];
        reference
            .joint_world_m
            .insert("head".to_owned(), positions.clone());
        prediction.joint_world_m.insert(
            "head".to_owned(),
            positions
                .iter()
                .map(|position| [position[0] + 0.03, position[1], position[2]])
                .collect(),
        );

        let report = evaluate(&reference, &prediction, None)
            .expect("matching named joints should be scored");
        assert_eq!(report.joint_trajectory_error.joints_scored, 1);
        assert!(
            (report.joint_trajectory_error.per_joint[0].mean_displacement_error_m - 0.03).abs()
                < 1e-9
        );
        assert!(
            (report.joint_trajectory_error.per_joint[0].final_displacement_error_m - 0.03).abs()
                < 1e-9
        );
        assert_eq!(report.root_trajectory_error.mean_displacement_error_m, 0.0);
    }

    #[test]
    fn rejects_different_joint_sets_instead_of_scoring_only_the_overlap() {
        let reference = fixture();
        let mut prediction = fixture();
        prediction
            .joint_world_m
            .insert("head".to_owned(), vec![[0.0, 1.0, 0.0]; 4]);
        assert!(
            evaluate(&reference, &prediction, None)
                .unwrap_err()
                .to_string()
                .contains("joint trajectory names differ")
        );
    }

    #[test]
    fn rejects_mismatched_sample_lengths() {
        let mut trace = fixture();
        trace.left_foot_world_m.pop();
        let path = Path::new("negative-control.json");
        assert!(
            trace
                .validate(path)
                .unwrap_err()
                .to_string()
                .contains("left_foot_world_m")
        );
    }

    #[test]
    fn rejects_trace_missing_required_nullable_retarget_field() {
        let mut value = serde_json::to_value(fixture()).unwrap();
        value.as_object_mut().unwrap().remove("retarget_map_sha256");
        assert!(serde_json::from_value::<Trace>(value).is_err());
    }

    #[test]
    fn rejects_non_synthetic_trace_with_synthetic_split() {
        let mut trace = fixture();
        trace.run.source_kind = SourceKind::LicensedAsset;
        assert!(
            trace
                .validate(Path::new("mismatched-source-and-split.json"))
                .unwrap_err()
                .to_string()
                .contains("non-synthetic source_kind cannot use run.split = synthetic")
        );
    }

    #[test]
    fn rejects_non_synthetic_trace_without_rights_or_provenance() {
        let mut trace = fixture();
        trace.run.source_kind = SourceKind::LicensedAsset;
        trace.run.split = None;
        assert!(
            trace
                .validate(Path::new("missing-provenance.json"))
                .unwrap_err()
                .to_string()
                .contains("40-character hexadecimal git commit")
        );
    }

    #[test]
    fn rejects_malformed_json_instead_of_skipping_it() {
        assert!(serde_json::from_str::<Trace>("{not-json").is_err());
    }

    #[test]
    fn rejects_non_monotonic_time() {
        let mut trace = fixture();
        trace.timestamps_sec[2] = trace.timestamps_sec[1];
        assert!(
            trace
                .validate(Path::new("negative-control.json"))
                .unwrap_err()
                .to_string()
                .contains("strictly increasing")
        );
    }

    #[test]
    fn refuses_incompatible_skeleton_or_retarget_map_hashes() {
        let reference = fixture();
        let mut prediction = fixture();
        prediction.skeleton_signature_sha256 = "1".repeat(64);
        assert!(
            evaluate(&reference, &prediction, None)
                .unwrap_err()
                .to_string()
                .contains("skeleton signature")
        );
        let mut prediction = fixture();
        prediction.retarget_map_sha256 = NullableDigest(Some("1".repeat(64)));
        assert!(
            evaluate(&reference, &prediction, None)
                .unwrap_err()
                .to_string()
                .contains("retarget-map hashes")
        );
    }

    #[test]
    fn common_grid_interpolates_different_sample_rates() {
        let reference = vec![0.0, 0.1, 0.2, 0.3, 0.4];
        let prediction = vec![0.0, 0.2, 0.4];
        let grid = build_time_grid(&reference, &prediction, Some(0.1)).unwrap();
        assert_eq!(grid.timestamps_sec.len(), 5);
        let positions = vec![[0.0, 0.0, 0.0], [0.2, 0.0, 0.0], [0.4, 0.0, 0.0]];
        let resampled = interpolate_positions(&prediction, &positions, &grid.timestamps_sec);
        assert!((resampled[1][0] - 0.1).abs() < 1e-9);
        assert!((resampled[3][0] - 0.3).abs() < 1e-9);
    }

    #[test]
    fn rejects_non_overlapping_or_too_short_grids() {
        assert!(build_time_grid(&[0.0, 0.1, 0.2, 0.3], &[1.0, 1.1, 1.2, 1.3], None).is_err());
        assert!(build_time_grid(&[0.0, 0.1, 0.2, 0.3], &[0.0, 0.1, 0.2, 0.3], Some(0.2)).is_err());
    }
}
