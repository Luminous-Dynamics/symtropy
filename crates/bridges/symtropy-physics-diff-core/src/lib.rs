// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Dependency-light records shared by independent physics backends.
//!
//! The schema deliberately has no engine dependencies. Semantic body IDs must be
//! supplied by a declared scenario mapping; adapters must never infer cross-engine
//! identity from insertion order or a physics engine's arena handle. Measurements
//! that are unavailable or not comparable are represented as `None`, not as zero.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Current version of the common observation record schema.
pub const OBSERVATION_SCHEMA_VERSION: u16 = 1;

/// Immutable provenance for one physics-backend execution profile.
///
/// The scenario and configuration digests should be computed from canonicalized
/// scenario/configuration inputs, not from runtime handles, renderer state, or timing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunIdentity {
    pub schema_version: u16,
    pub scenario_id: String,
    pub scenario_digest: String,
    pub backend_id: String,
    pub backend_version: String,
    pub solver_profile: String,
    pub configuration_digest: String,
    /// Fixed simulation timestep in seconds.
    pub timestep_seconds: f64,
}

impl RunIdentity {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        scenario_id: impl Into<String>,
        scenario_digest: impl Into<String>,
        backend_id: impl Into<String>,
        backend_version: impl Into<String>,
        solver_profile: impl Into<String>,
        configuration_digest: impl Into<String>,
        timestep_seconds: f64,
    ) -> Self {
        Self {
            schema_version: OBSERVATION_SCHEMA_VERSION,
            scenario_id: scenario_id.into(),
            scenario_digest: scenario_digest.into(),
            backend_id: backend_id.into(),
            backend_version: backend_version.into(),
            solver_profile: solver_profile.into(),
            configuration_digest: configuration_digest.into(),
            timestep_seconds,
        }
    }

    pub fn validate(&self) -> Result<(), ObservationError> {
        if self.schema_version != OBSERVATION_SCHEMA_VERSION {
            return Err(ObservationError::UnsupportedSchemaVersion {
                found: self.schema_version,
            });
        }
        for (field, value) in [
            ("scenario_id", self.scenario_id.as_str()),
            ("scenario_digest", self.scenario_digest.as_str()),
            ("backend_id", self.backend_id.as_str()),
            ("backend_version", self.backend_version.as_str()),
            ("solver_profile", self.solver_profile.as_str()),
            ("configuration_digest", self.configuration_digest.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(ObservationError::EmptyIdentityField(field));
            }
        }
        if !self.timestep_seconds.is_finite() || self.timestep_seconds <= 0.0 {
            return Err(ObservationError::InvalidTimestep);
        }
        Ok(())
    }
}

/// Per-body observations in SI units wherever a physical unit applies.
///
/// Orientation uses a scalar-first quaternion `[w, x, y, z]`. Values are named
/// with units to discourage accidental comparison of incompatible conventions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyObservation {
    /// Stable scenario-level identity supplied by the caller, not an engine handle.
    pub semantic_body_id: String,
    pub position_m: [f64; 3],
    pub orientation_wxyz: [f64; 4],
    pub linear_velocity_mps: [f64; 3],
    pub angular_velocity_radps: [f64; 3],
    /// Engine-local sleep/active information when semantics are comparable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sleeping: Option<bool>,
    /// Per-body contact count; `None` means the adapter does not report it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_count: Option<u64>,
    /// Maximum observed penetration depth for this body, if derivable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_penetration_m: Option<f64>,
}

impl BodyObservation {
    pub fn new(
        semantic_body_id: impl Into<String>,
        position_m: [f64; 3],
        orientation_wxyz: [f64; 4],
        linear_velocity_mps: [f64; 3],
        angular_velocity_radps: [f64; 3],
    ) -> Self {
        Self {
            semantic_body_id: semantic_body_id.into(),
            position_m,
            orientation_wxyz,
            linear_velocity_mps,
            angular_velocity_radps,
            sleeping: None,
            contact_count: None,
            max_penetration_m: None,
        }
    }

    fn validate(&self) -> Result<(), ObservationError> {
        if self.semantic_body_id.trim().is_empty() {
            return Err(ObservationError::EmptySemanticBodyId);
        }
        for (field, values) in [
            ("position_m", self.position_m.as_slice()),
            ("orientation_wxyz", self.orientation_wxyz.as_slice()),
            ("linear_velocity_mps", self.linear_velocity_mps.as_slice()),
            (
                "angular_velocity_radps",
                self.angular_velocity_radps.as_slice(),
            ),
        ] {
            if !values.iter().all(|value| value.is_finite()) {
                return Err(ObservationError::NonFiniteBodyValue {
                    semantic_body_id: self.semantic_body_id.clone(),
                    field,
                });
            }
        }
        let quaternion_norm_squared = self
            .orientation_wxyz
            .iter()
            .map(|value| value * value)
            .sum::<f64>();
        if !quaternion_norm_squared.is_finite() || (quaternion_norm_squared - 1.0).abs() > 1.0e-3 {
            return Err(ObservationError::InvalidQuaternion {
                semantic_body_id: self.semantic_body_id.clone(),
            });
        }
        validate_nonnegative(self.max_penetration_m, "body", "max_penetration_m")?;
        Ok(())
    }
}

/// Contact metrics reported only when the source backend can measure them reliably.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ContactSummary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifold_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_point_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_penetration_m: Option<f64>,
}

/// Energy diagnostics for which the scenario and backend profile define a meaningful
/// accounting convention. Potential energy may be negative because its zero is chosen.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EnergySummary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kinetic_energy_j: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub potential_energy_j: Option<f64>,
}

/// One timestamped observation from a single backend run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservationFrame {
    pub identity: RunIdentity,
    pub step_index: u64,
    pub simulation_time_seconds: f64,
    pub bodies: Vec<BodyObservation>,
    pub contacts: ContactSummary,
    pub energy: EnergySummary,
}

impl ObservationFrame {
    pub fn new(identity: RunIdentity, step_index: u64, simulation_time_seconds: f64) -> Self {
        Self {
            identity,
            step_index,
            simulation_time_seconds,
            bodies: Vec::new(),
            contacts: ContactSummary::default(),
            energy: EnergySummary::default(),
        }
    }

    /// Validate provenance, units-bearing physical values, body identity uniqueness,
    /// and optional metrics without substituting zero for unavailable measurements.
    pub fn validate(&self) -> Result<(), ObservationError> {
        self.identity.validate()?;
        if !self.simulation_time_seconds.is_finite() || self.simulation_time_seconds < 0.0 {
            return Err(ObservationError::InvalidSimulationTime);
        }

        for (index, body) in self.bodies.iter().enumerate() {
            body.validate()?;
            if self.bodies[..index]
                .iter()
                .any(|prior| prior.semantic_body_id == body.semantic_body_id)
            {
                return Err(ObservationError::DuplicateSemanticBodyId(
                    body.semantic_body_id.clone(),
                ));
            }
        }

        validate_nonnegative(
            self.contacts.max_penetration_m,
            "contact_summary",
            "max_penetration_m",
        )?;
        validate_nonnegative(self.energy.kinetic_energy_j, "energy", "kinetic_energy_j")?;
        validate_finite_optional(
            self.energy.potential_energy_j,
            "energy",
            "potential_energy_j",
        )?;
        Ok(())
    }
}

/// Complete sampled execution for one backend profile.
///
/// An observation trace is the canonical cross-engine output envelope. Scenario
/// manifests remain separate inputs; every frame carries the same immutable run
/// identity so a partial or mixed-backend trace cannot be mistaken for a valid run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservationTrace {
    pub identity: RunIdentity,
    pub frames: Vec<ObservationFrame>,
}

impl ObservationTrace {
    pub fn new(identity: RunIdentity) -> Self {
        Self {
            identity,
            frames: Vec::new(),
        }
    }

    /// Append one validated sample. The trace is not mutated on failure.
    pub fn push(&mut self, frame: ObservationFrame) -> Result<(), ObservationError> {
        self.identity.validate()?;
        frame.validate()?;
        if frame.identity != self.identity {
            return Err(ObservationError::FrameIdentityMismatch);
        }
        validate_frame_clock(&self.identity, &frame)?;

        if let Some(previous) = self.frames.last() {
            if frame.step_index <= previous.step_index {
                return Err(ObservationError::NonIncreasingStep {
                    previous: previous.step_index,
                    current: frame.step_index,
                });
            }
            if frame.simulation_time_seconds <= previous.simulation_time_seconds {
                return Err(ObservationError::NonIncreasingSimulationTime {
                    previous: previous.simulation_time_seconds,
                    current: frame.simulation_time_seconds,
                });
            }
        }
        self.frames.push(frame);
        Ok(())
    }

    /// Re-validate the complete trace at serialization/import boundaries.
    pub fn validate(&self) -> Result<(), ObservationError> {
        self.identity.validate()?;
        if self.frames.is_empty() {
            return Err(ObservationError::EmptyTrace);
        }

        let mut previous: Option<&ObservationFrame> = None;
        for frame in &self.frames {
            frame.validate()?;
            if frame.identity != self.identity {
                return Err(ObservationError::FrameIdentityMismatch);
            }
            validate_frame_clock(&self.identity, frame)?;

            if let Some(prior) = previous {
                if frame.step_index <= prior.step_index {
                    return Err(ObservationError::NonIncreasingStep {
                        previous: prior.step_index,
                        current: frame.step_index,
                    });
                }
                if frame.simulation_time_seconds <= prior.simulation_time_seconds {
                    return Err(ObservationError::NonIncreasingSimulationTime {
                        previous: prior.simulation_time_seconds,
                        current: frame.simulation_time_seconds,
                    });
                }
            }
            previous = Some(frame);
        }
        Ok(())
    }

    pub fn final_frame(&self) -> Option<&ObservationFrame> {
        self.frames.last()
    }
}

fn validate_frame_clock(
    identity: &RunIdentity,
    frame: &ObservationFrame,
) -> Result<(), ObservationError> {
    let expected_time = frame.step_index as f64 * identity.timestep_seconds;
    let tolerance = 1.0e-9 * (1.0 + expected_time.abs());
    if !expected_time.is_finite()
        || (frame.simulation_time_seconds - expected_time).abs() > tolerance
    {
        return Err(ObservationError::FrameTimeDoesNotMatchTimestep {
            step_index: frame.step_index,
            actual_seconds: frame.simulation_time_seconds,
            expected_seconds: expected_time,
        });
    }
    Ok(())
}

/// Per-run numerical differences between two validated observation traces.
///
/// Optional measurements are compared only where both adapters provide them.
/// Not-comparable counters prevent unavailable measurements from silently
/// becoming zero-valued agreements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceDifferenceMetrics {
    pub scenario_id: String,
    pub left_backend: String,
    pub left_backend_version: String,
    pub right_backend: String,
    pub right_backend_version: String,
    pub compared_frames: u64,
    pub compared_body_observations: u64,
    pub max_position_error_m: f64,
    pub rms_position_error_m: f64,
    pub max_orientation_error_rad: f64,
    pub rms_orientation_error_rad: f64,
    pub max_linear_velocity_error_mps: f64,
    pub rms_linear_velocity_error_mps: f64,
    pub max_angular_velocity_error_radps: f64,
    pub rms_angular_velocity_error_radps: f64,
    pub sleeping_comparisons: u64,
    pub sleeping_mismatches: u64,
    pub sleeping_not_comparable: u64,
    pub contact_count_comparisons: u64,
    pub contact_count_mismatches: u64,
    pub contact_count_not_comparable: u64,
    pub penetration_comparisons: u64,
    pub max_penetration_error_m: f64,
    pub kinetic_energy_comparisons: u64,
    pub max_kinetic_energy_error_j: f64,
    pub potential_energy_comparisons: u64,
    pub max_potential_energy_error_j: f64,
}

/// Why two otherwise-valid traces cannot be compared under the same scenario.
#[derive(Debug, Clone, PartialEq)]
pub enum TraceComparisonError {
    InvalidLeftTrace(ObservationError),
    InvalidRightTrace(ObservationError),
    ScenarioIdMismatch,
    ScenarioDigestMismatch,
    TimestepMismatch { left_seconds: f64, right_seconds: f64 },
    FrameCountMismatch { left: usize, right: usize },
    FrameStepMismatch { left: u64, right: u64 },
    FrameTimeMismatch { step: u64, left_seconds: f64, right_seconds: f64 },
    BodySetMismatch { step: u64 },
    NonFiniteDifference { step: u64, semantic_body_id: String, field: &'static str },
}

/// Stable sum-of-squares accumulator (the scaled LASSQ algorithm).
#[derive(Clone, Copy, Debug, Default)]
struct StableSumSquares {
    scale: f64,
    sum_squares: f64,
    count: u64,
}

impl StableSumSquares {
    fn add(&mut self, value: f64) {
        let magnitude = value.abs();
        self.count += 1;
        if magnitude == 0.0 {
            return;
        }
        if self.scale < magnitude {
            let ratio = if self.scale == 0.0 { 0.0 } else { self.scale / magnitude };
            self.sum_squares = 1.0 + self.sum_squares * ratio * ratio;
            self.scale = magnitude;
        } else {
            let ratio = magnitude / self.scale;
            self.sum_squares += ratio * ratio;
        }
    }

    fn rms(self) -> f64 {
        if self.count == 0 || self.scale == 0.0 {
            0.0
        } else {
            self.scale * (self.sum_squares / self.count as f64).sqrt()
        }
    }
}

fn checked_difference(
    left: f64,
    right: f64,
    step: u64,
    semantic_body_id: &str,
    field: &'static str,
) -> Result<f64, TraceComparisonError> {
    let difference = left - right;
    if difference.is_finite() {
        Ok(difference.abs())
    } else {
        Err(TraceComparisonError::NonFiniteDifference {
            step,
            semantic_body_id: semantic_body_id.to_owned(),
            field,
        })
    }
}

fn stable_vec3_error(
    left: [f64; 3],
    right: [f64; 3],
    step: u64,
    semantic_body_id: &str,
    field: &'static str,
) -> Result<f64, TraceComparisonError> {
    let dx = checked_difference(left[0], right[0], step, semantic_body_id, field)?;
    let dy = checked_difference(left[1], right[1], step, semantic_body_id, field)?;
    let dz = checked_difference(left[2], right[2], step, semantic_body_id, field)?;
    let error = dx.hypot(dy).hypot(dz);
    if error.is_finite() {
        Ok(error)
    } else {
        Err(TraceComparisonError::NonFiniteDifference {
            step,
            semantic_body_id: semantic_body_id.to_owned(),
            field,
        })
    }
}

fn quaternion_geodesic_error(
    left: [f64; 4],
    right: [f64; 4],
    step: u64,
    semantic_body_id: &str,
) -> Result<f64, TraceComparisonError> {
    let mut dot = 0.0;
    for index in 0..4 {
        dot += left[index] * right[index];
    }
    if !dot.is_finite() {
        return Err(TraceComparisonError::NonFiniteDifference {
            step,
            semantic_body_id: semantic_body_id.to_owned(),
            field: "orientation_wxyz",
        });
    }
    // q and -q represent the same rotation.
    Ok(2.0 * dot.abs().clamp(0.0, 1.0).acos())
}

/// Compare two traces only when scenario digest, timestep, sampled clock and
/// semantic body sets agree. Optional fields are compared only where both
/// backends provide them; unavailable values are counted separately.
pub fn compare_observation_traces(
    left: &ObservationTrace,
    right: &ObservationTrace,
) -> Result<TraceDifferenceMetrics, TraceComparisonError> {
    left.validate()
        .map_err(TraceComparisonError::InvalidLeftTrace)?;
    right
        .validate()
        .map_err(TraceComparisonError::InvalidRightTrace)?;

    if left.identity.scenario_id != right.identity.scenario_id {
        return Err(TraceComparisonError::ScenarioIdMismatch);
    }
    if left.identity.scenario_digest != right.identity.scenario_digest {
        return Err(TraceComparisonError::ScenarioDigestMismatch);
    }
    let timestep_delta =
        (left.identity.timestep_seconds - right.identity.timestep_seconds).abs();
    let timestep_scale = 1.0
        + left.identity.timestep_seconds.abs()
        + right.identity.timestep_seconds.abs();
    if timestep_delta > 1.0e-12 * timestep_scale {
        return Err(TraceComparisonError::TimestepMismatch {
            left_seconds: left.identity.timestep_seconds,
            right_seconds: right.identity.timestep_seconds,
        });
    }
    if left.frames.len() != right.frames.len() {
        return Err(TraceComparisonError::FrameCountMismatch {
            left: left.frames.len(),
            right: right.frames.len(),
        });
    }

    let mut position = StableSumSquares::default();
    let mut orientation = StableSumSquares::default();
    let mut linear_velocity = StableSumSquares::default();
    let mut angular_velocity = StableSumSquares::default();
    let mut metrics = TraceDifferenceMetrics {
        scenario_id: left.identity.scenario_id.clone(),
        left_backend: left.identity.backend_id.clone(),
        left_backend_version: left.identity.backend_version.clone(),
        right_backend: right.identity.backend_id.clone(),
        right_backend_version: right.identity.backend_version.clone(),
        compared_frames: 0,
        compared_body_observations: 0,
        max_position_error_m: 0.0,
        rms_position_error_m: 0.0,
        max_orientation_error_rad: 0.0,
        rms_orientation_error_rad: 0.0,
        max_linear_velocity_error_mps: 0.0,
        rms_linear_velocity_error_mps: 0.0,
        max_angular_velocity_error_radps: 0.0,
        rms_angular_velocity_error_radps: 0.0,
        sleeping_comparisons: 0,
        sleeping_mismatches: 0,
        sleeping_not_comparable: 0,
        contact_count_comparisons: 0,
        contact_count_mismatches: 0,
        contact_count_not_comparable: 0,
        penetration_comparisons: 0,
        max_penetration_error_m: 0.0,
        kinetic_energy_comparisons: 0,
        max_kinetic_energy_error_j: 0.0,
        potential_energy_comparisons: 0,
        max_potential_energy_error_j: 0.0,
    };

    for (left_frame, right_frame) in left.frames.iter().zip(&right.frames) {
        if left_frame.step_index != right_frame.step_index {
            return Err(TraceComparisonError::FrameStepMismatch {
                left: left_frame.step_index,
                right: right_frame.step_index,
            });
        }
        let time_delta =
            (left_frame.simulation_time_seconds - right_frame.simulation_time_seconds).abs();
        let time_scale = 1.0
            + left_frame.simulation_time_seconds.abs()
            + right_frame.simulation_time_seconds.abs();
        if time_delta > 1.0e-12 * time_scale {
            return Err(TraceComparisonError::FrameTimeMismatch {
                step: left_frame.step_index,
                left_seconds: left_frame.simulation_time_seconds,
                right_seconds: right_frame.simulation_time_seconds,
            });
        }

        let left_bodies: BTreeMap<&str, &BodyObservation> = left_frame
            .bodies
            .iter()
            .map(|body| (body.semantic_body_id.as_str(), body))
            .collect();
        let right_bodies: BTreeMap<&str, &BodyObservation> = right_frame
            .bodies
            .iter()
            .map(|body| (body.semantic_body_id.as_str(), body))
            .collect();
        if left_bodies.keys().ne(right_bodies.keys()) {
            return Err(TraceComparisonError::BodySetMismatch {
                step: left_frame.step_index,
            });
        }

        for (semantic_body_id, left_body) in &left_bodies {
            let right_body = right_bodies[semantic_body_id];
            let step = left_frame.step_index;
            let position_error = stable_vec3_error(
                left_body.position_m,
                right_body.position_m,
                step,
                semantic_body_id,
                "position_m",
            )?;
            let orientation_error = quaternion_geodesic_error(
                left_body.orientation_wxyz,
                right_body.orientation_wxyz,
                step,
                semantic_body_id,
            )?;
            let linear_error = stable_vec3_error(
                left_body.linear_velocity_mps,
                right_body.linear_velocity_mps,
                step,
                semantic_body_id,
                "linear_velocity_mps",
            )?;
            let angular_error = stable_vec3_error(
                left_body.angular_velocity_radps,
                right_body.angular_velocity_radps,
                step,
                semantic_body_id,
                "angular_velocity_radps",
            )?;

            position.add(position_error);
            orientation.add(orientation_error);
            linear_velocity.add(linear_error);
            angular_velocity.add(angular_error);
            metrics.max_position_error_m = metrics.max_position_error_m.max(position_error);
            metrics.max_orientation_error_rad =
                metrics.max_orientation_error_rad.max(orientation_error);
            metrics.max_linear_velocity_error_mps =
                metrics.max_linear_velocity_error_mps.max(linear_error);
            metrics.max_angular_velocity_error_radps =
                metrics.max_angular_velocity_error_radps.max(angular_error);
            metrics.compared_body_observations += 1;

            match (left_body.sleeping, right_body.sleeping) {
                (Some(left_value), Some(right_value)) => {
                    metrics.sleeping_comparisons += 1;
                    metrics.sleeping_mismatches += u64::from(left_value != right_value);
                }
                _ => metrics.sleeping_not_comparable += 1,
            }
            match (left_body.contact_count, right_body.contact_count) {
                (Some(left_value), Some(right_value)) => {
                    metrics.contact_count_comparisons += 1;
                    metrics.contact_count_mismatches += u64::from(left_value != right_value);
                }
                _ => metrics.contact_count_not_comparable += 1,
            }
            match (left_body.max_penetration_m, right_body.max_penetration_m) {
                (Some(left_value), Some(right_value)) => {
                    let error = checked_difference(
                        left_value,
                        right_value,
                        step,
                        semantic_body_id,
                        "max_penetration_m",
                    )?;
                    metrics.penetration_comparisons += 1;
                    metrics.max_penetration_error_m =
                        metrics.max_penetration_error_m.max(error);
                }
                _ => {}
            }
        }

        metrics.compared_frames += 1;
        for (left_value, right_value, counter, maximum) in [
            (
                left_frame.energy.kinetic_energy_j,
                right_frame.energy.kinetic_energy_j,
                &mut metrics.kinetic_energy_comparisons,
                &mut metrics.max_kinetic_energy_error_j,
            ),
            (
                left_frame.energy.potential_energy_j,
                right_frame.energy.potential_energy_j,
                &mut metrics.potential_energy_comparisons,
                &mut metrics.max_potential_energy_error_j,
            ),
        ] {
            if let (Some(left_value), Some(right_value)) = (left_value, right_value) {
                let error = checked_difference(
                    left_value,
                    right_value,
                    left_frame.step_index,
                    "<frame>",
                    "energy_j",
                )?;
                *counter += 1;
                *maximum = maximum.max(error);
            }
        }
    }

    metrics.rms_position_error_m = position.rms();
    metrics.rms_orientation_error_rad = orientation.rms();
    metrics.rms_linear_velocity_error_mps = linear_velocity.rms();
    metrics.rms_angular_velocity_error_radps = angular_velocity.rms();
    Ok(metrics)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationError {
    UnsupportedSchemaVersion {
        found: u16,
    },
    EmptyIdentityField(&'static str),
    InvalidTimestep,
    InvalidSimulationTime,
    EmptySemanticBodyId,
    DuplicateSemanticBodyId(String),
    NonFiniteBodyValue {
        semantic_body_id: String,
        field: &'static str,
    },
    InvalidQuaternion {
        semantic_body_id: String,
    },
    NonFiniteOptionalValue {
        subject: &'static str,
        field: &'static str,
    },
    NegativeOptionalValue {
        subject: &'static str,
        field: &'static str,
    },
    EmptyTrace,
    FrameIdentityMismatch,
    NonIncreasingStep {
        previous: u64,
        current: u64,
    },
    NonIncreasingSimulationTime {
        previous: f64,
        current: f64,
    },
    FrameTimeDoesNotMatchTimestep {
        step_index: u64,
        actual_seconds: f64,
        expected_seconds: f64,
    },
}

fn validate_finite_optional(
    value: Option<f64>,
    subject: &'static str,
    field: &'static str,
) -> Result<(), ObservationError> {
    if value.is_some_and(|number| !number.is_finite()) {
        return Err(ObservationError::NonFiniteOptionalValue { subject, field });
    }
    Ok(())
}

fn validate_nonnegative(
    value: Option<f64>,
    subject: &'static str,
    field: &'static str,
) -> Result<(), ObservationError> {
    validate_finite_optional(value, subject, field)?;
    if value.is_some_and(|number| number < 0.0) {
        return Err(ObservationError::NegativeOptionalValue { subject, field });
    }
    Ok(())
}

impl std::fmt::Display for ObservationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion { found } => {
                write!(f, "unsupported observation schema version {found}")
            }
            Self::EmptyIdentityField(field) => write!(f, "run identity field {field} is empty"),
            Self::InvalidTimestep => write!(f, "timestep must be finite and positive"),
            Self::InvalidSimulationTime => {
                write!(f, "simulation time must be finite and nonnegative")
            }
            Self::EmptySemanticBodyId => write!(f, "semantic body ID must not be empty"),
            Self::DuplicateSemanticBodyId(body_id) => {
                write!(f, "duplicate semantic body ID {body_id:?}")
            }
            Self::NonFiniteBodyValue {
                semantic_body_id,
                field,
            } => write!(f, "body {semantic_body_id:?} has non-finite {field}"),
            Self::InvalidQuaternion { semantic_body_id } => {
                write!(f, "body {semantic_body_id:?} has a non-unit quaternion")
            }
            Self::NonFiniteOptionalValue { subject, field } => {
                write!(f, "{subject}.{field} must be finite when present")
            }
            Self::NegativeOptionalValue { subject, field } => {
                write!(f, "{subject}.{field} must be nonnegative when present")
            }
            Self::EmptyTrace => write!(f, "observation trace must contain at least one frame"),
            Self::FrameIdentityMismatch => {
                write!(f, "observation frame run identity differs from its trace identity")
            }
            Self::NonIncreasingStep { previous, current } => {
                write!(f, "observation steps must increase: previous={previous}, current={current}")
            }
            Self::NonIncreasingSimulationTime { previous, current } => {
                write!(
                    f,
                    "observation simulation time must increase: previous={previous}, current={current}"
                )
            }
            Self::FrameTimeDoesNotMatchTimestep {
                step_index,
                actual_seconds,
                expected_seconds,
            } => write!(
                f,
                "frame time {actual_seconds} s at step {step_index} differs from fixed-step time {expected_seconds} s"
            ),
        }
    }
}

impl std::error::Error for ObservationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> RunIdentity {
        RunIdentity::new(
            "free-fall",
            "sha256:scenario-fixture",
            "symtropy-native",
            "test-build",
            "deterministic-profile",
            "sha256:configuration-fixture",
            1.0 / 60.0,
        )
    }

    fn body(id: &str) -> BodyObservation {
        BodyObservation::new(
            id,
            [0.0, 10.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, -0.1, 0.0],
            [0.0, 0.0, 0.0],
        )
    }

    fn frame() -> ObservationFrame {
        let mut frame = ObservationFrame::new(identity(), 1, 1.0 / 60.0);
        frame.bodies.push(body("ball/main"));
        frame
    }

    #[test]
    fn valid_frame_accepts_missing_engine_specific_measurements() {
        let mut frame = frame();
        frame.bodies[0].sleeping = Some(false);
        frame.bodies[0].contact_count = None;
        frame.energy.potential_energy_j = Some(-2.5);

        assert_eq!(frame.validate(), Ok(()));
    }

    #[test]
    fn observation_frame_json_round_trip_preserves_unknown_measurements_as_absent() {
        let frame = frame();
        let encoded = serde_json::to_string(&frame).expect("serialize validated frame");
        assert!(
            !encoded.contains("contact_count"),
            "unavailable metrics should be omitted rather than encoded as fake zero/null values",
        );

        let decoded: ObservationFrame =
            serde_json::from_str(&encoded).expect("deserialize observation frame");
        assert_eq!(decoded, frame);
        assert_eq!(decoded.validate(), Ok(()));
    }

    #[test]
    fn duplicate_semantic_body_ids_are_rejected() {
        let mut frame = frame();
        frame.bodies.push(body("ball/main"));

        assert_eq!(
            frame.validate(),
            Err(ObservationError::DuplicateSemanticBodyId(
                "ball/main".to_owned()
            )),
        );
    }

    #[test]
    fn invalid_pose_and_motion_values_are_rejected() {
        let mut non_finite_position = frame();
        non_finite_position.bodies[0].position_m[0] = f64::NAN;
        assert!(matches!(
            non_finite_position.validate(),
            Err(ObservationError::NonFiniteBodyValue {
                field: "position_m",
                ..
            })
        ));

        let mut invalid_quaternion = frame();
        invalid_quaternion.bodies[0].orientation_wxyz = [2.0, 0.0, 0.0, 0.0];
        assert!(matches!(
            invalid_quaternion.validate(),
            Err(ObservationError::InvalidQuaternion { .. })
        ));
    }

    fn frame_at(step_index: u64, time_s: f64) -> ObservationFrame {
        let mut frame = ObservationFrame::new(identity(), step_index, time_s);
        frame.bodies.push(body("ball/main"));
        frame
    }

    #[test]
    fn trace_accepts_sparse_monotonic_fixed_step_samples_and_round_trips() {
        let mut trace = ObservationTrace::new(identity());
        trace.push(frame_at(0, 0.0)).unwrap();
        trace.push(frame_at(3, 3.0 / 60.0)).unwrap();
        trace.push(frame_at(60, 1.0)).unwrap();

        assert_eq!(trace.validate(), Ok(()));
        assert_eq!(trace.final_frame().unwrap().step_index, 60);

        let encoded = serde_json::to_string(&trace).expect("serialize observation trace");
        let decoded: ObservationTrace =
            serde_json::from_str(&encoded).expect("deserialize observation trace");
        assert_eq!(decoded, trace);
        assert_eq!(decoded.validate(), Ok(()));
    }

    #[test]
    fn trace_rejects_mixed_backend_identity() {
        let mut trace = ObservationTrace::new(identity());
        trace.push(frame_at(0, 0.0)).unwrap();

        let mut foreign = frame_at(1, 1.0 / 60.0);
        foreign.identity.backend_id = "rapier".to_owned();
        assert_eq!(
            trace.push(foreign),
            Err(ObservationError::FrameIdentityMismatch)
        );
        assert_eq!(trace.frames.len(), 1, "failed push must not mutate trace");
    }

    #[test]
    fn trace_rejects_duplicate_or_nonmonotonic_steps_and_times() {
        let mut duplicate_step = ObservationTrace::new(identity());
        duplicate_step.push(frame_at(1, 1.0 / 60.0)).unwrap();
        assert_eq!(
            duplicate_step.push(frame_at(1, 1.0 / 60.0)),
            Err(ObservationError::NonIncreasingStep {
                previous: 1,
                current: 1
            })
        );

        let mut reversed_time = ObservationTrace::new(identity());
        reversed_time.push(frame_at(2, 2.0 / 60.0)).unwrap();
        let mut earlier = frame_at(3, 3.0 / 60.0);
        earlier.simulation_time_seconds = 1.0 / 60.0;
        assert_eq!(
            reversed_time.push(earlier),
            Err(ObservationError::FrameTimeDoesNotMatchTimestep {
                step_index: 3,
                actual_seconds: 1.0 / 60.0,
                expected_seconds: 3.0 / 60.0
            })
        );
    }

    #[test]
    fn trace_rejects_empty_and_wrong_fixed_step_clock() {
        let empty = ObservationTrace::new(identity());
        assert_eq!(empty.validate(), Err(ObservationError::EmptyTrace));

        let mut wrong_clock = ObservationTrace::new(identity());
        assert_eq!(
            wrong_clock.push(frame_at(3, 0.04)),
            Err(ObservationError::FrameTimeDoesNotMatchTimestep {
                step_index: 3,
                actual_seconds: 0.04,
                expected_seconds: 3.0 / 60.0
            })
        );
    }

    #[test]
    fn invalid_provenance_and_physical_summaries_are_rejected() {
        let mut invalid_dt = frame();
        invalid_dt.identity.timestep_seconds = 0.0;
        assert_eq!(
            invalid_dt.validate(),
            Err(ObservationError::InvalidTimestep)
        );

        let mut invalid_penetration = frame();
        invalid_penetration.contacts.max_penetration_m = Some(-0.01);
        assert_eq!(
            invalid_penetration.validate(),
            Err(ObservationError::NegativeOptionalValue {
                subject: "contact_summary",
                field: "max_penetration_m",
            }),
        );

        let mut invalid_kinetic_energy = frame();
        invalid_kinetic_energy.energy.kinetic_energy_j = Some(f64::INFINITY);
        assert_eq!(
            invalid_kinetic_energy.validate(),
            Err(ObservationError::NonFiniteOptionalValue {
                subject: "energy",
                field: "kinetic_energy_j",
            }),
        );
    }
}
