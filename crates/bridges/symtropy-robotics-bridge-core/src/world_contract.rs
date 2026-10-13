// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Versioned contracts for replayable robotics digital-twin worlds.
//!
//! This module validates scenario, platform, sensor, solver, and result
//! identities at the boundary. It does not implement physics and does not
//! certify that any declared solver is physically accurate.

use crate::platform::PlatformType;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const WORLD_MANIFEST_SCHEMA_VERSION: u16 = 1;
pub const SOLVER_RESULT_SCHEMA_VERSION: u16 = 1;
pub const SHA256_HEX_LENGTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhysicsDomain {
    RigidBody,
    Terrain,
    MechanicalCutting,
    Acoustic,
    Thermal,
    Electromagnetic,
    ElectricalDischarge,
    Fluid,
    ContinuumMechanics,
    Fracture,
    SensorModel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhysicsFidelity {
    Kinematic,
    DeterministicReference,
    ReducedOrder,
    DomainSolver,
    CoupledRegion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensorModality {
    Camera,
    Lidar,
    Imu,
    Encoder,
    ForceTorque,
    Vibration,
    AcousticEmission,
    Ultrasound,
    ThermalImager,
    Thermocouple,
    Pressure,
    Flow,
    Gas,
    Electrical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub artifact_id: String,
    pub uri: String,
    /// SHA-256 of the exact artifact bytes, encoded as 64 hexadecimal digits.
    pub sha256: String,
}

impl ArtifactRef {
    fn validate_at(&self, path: &str, issues: &mut Vec<WorldValidationIssue>) {
        if self.artifact_id.trim().is_empty() || self.uri.trim().is_empty() {
            issues.push(WorldValidationIssue::new(
                WorldValidationCode::InvalidArtifact,
                path,
            ));
        }
        if !is_sha256_hex(&self.sha256) {
            issues.push(WorldValidationIssue::new(
                WorldValidationCode::InvalidDigest,
                format!("{path}.sha256"),
            ));
        }
    }
}

/// A named coordinate frame in the world's frame topology.
///
/// The root frame is always "world". Static transforms map a point expressed
/// in this frame into its parent frame using
/// p_parent = R_parent_child * p_child + t_parent_child. Kinematic transforms
/// name the exact robot-kinematics artifact and the frame inside that artifact
/// which supplies the time-varying transform; this contract validates the
/// reference, not the external kinematics implementation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoordinateFrameV1 {
    pub frame_id: String,
    pub parent_frame_id: Option<String>,
    pub transform_to_parent: FrameTransformV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FrameTransformV1 {
    /// Only the unique "world" root may use this variant.
    Root,
    /// Transform from this frame to its parent, in metres and XYZW quaternion order.
    Static {
        translation_m: [f64; 3],
        rotation_xyzw: [f64; 4],
    },
    /// The transform is provided at runtime by a declared platform kinematics
    /// artifact. The referenced provider frame name is artifact-scoped.
    Kinematic {
        provider_artifact_id: String,
        provider_frame_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorProfileV1 {
    pub sensor_id: String,
    pub modality: SensorModality,
    /// Frame in which the sensor measurement origin/orientation is defined.
    pub frame_id: String,
    pub model_solver_id: Option<String>,
    pub calibration_artifact: Option<ArtifactRef>,
    pub sample_rate_hz: f64,
    pub latency_ns: u64,
    pub jitter_ns: u64,
    pub noise_std: f64,
    pub noise_unit: String,
    pub dropout_probability: f64,
    pub saturation_min: Option<f64>,
    pub saturation_max: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActuatorProfileV1 {
    pub actuator_id: String,
    pub min_command: f64,
    pub max_command: f64,
    pub command_unit: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlatformProfileV1 {
    pub platform_id: String,
    pub platform_type: PlatformType,
    pub geometry: ArtifactRef,
    pub kinematics: ArtifactRef,
    /// Frame representing the platform's base pose in the world frame graph.
    pub base_frame_id: String,
    pub mass_kg: f64,
    pub control_rate_hz: f64,
    pub controller_interface_version: u16,
    pub actuators: Vec<ActuatorProfileV1>,
    pub sensor_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolverDeclarationV1 {
    pub solver_id: String,
    pub domain: PhysicsDomain,
    pub implementation: String,
    pub version: String,
    pub build_sha256: String,
    pub parameters_sha256: String,
    pub output_contract_version: u16,
    pub fidelity: PhysicsFidelity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldManifestV1 {
    pub schema_version: u16,
    pub scenario_id: String,
    /// Monotonic identity for a physics-relevant world generation. Any change
    /// to canonical geometry, materials, solver/profile configuration, or
    /// platform state that can invalidate a result must advance this value.
    pub world_generation: u64,
    pub seed: u64,
    /// Fixed world step in integer nanoseconds; wall-clock cadence is separate.
    pub fixed_step_ns: u64,
    pub gravity_mps2: [f64; 3],
    /// Authoritative coordinate-frame registry/parent topology for this world.
    pub frames: Vec<CoordinateFrameV1>,
    pub fidelity: PhysicsFidelity,
    pub world_assets: Vec<ArtifactRef>,
    pub solvers: Vec<SolverDeclarationV1>,
    pub sensors: Vec<SensorProfileV1>,
    pub platforms: Vec<PlatformProfileV1>,
}

impl WorldManifestV1 {
    /// Validate identities, units/ranges, references, and reproducibility fields.
    ///
    /// This checks whether the manifest is structurally well-formed. It does
    /// not prove the declared physics, calibrations, or material parameters are
    /// correct for the real world.
    pub fn validate(&self) -> Result<(), Vec<WorldValidationIssue>> {
        let mut issues = Vec::new();

        if self.schema_version != WORLD_MANIFEST_SCHEMA_VERSION {
            issues.push(WorldValidationIssue::new(
                WorldValidationCode::UnsupportedSchemaVersion,
                "schema_version",
            ));
        }
        if self.scenario_id.trim().is_empty() {
            issues.push(WorldValidationIssue::new(
                WorldValidationCode::EmptyIdentity,
                "scenario_id",
            ));
        }
        if self.world_generation == 0 {
            issues.push(WorldValidationIssue::new(
                WorldValidationCode::InvalidWorldGeneration,
                "world_generation",
            ));
        }
        if self.fixed_step_ns == 0 {
            issues.push(WorldValidationIssue::new(
                WorldValidationCode::InvalidTimeStep,
                "fixed_step_ns",
            ));
        }
        if !self.gravity_mps2.iter().all(|v| v.is_finite()) {
            issues.push(WorldValidationIssue::new(
                WorldValidationCode::NonFiniteValue,
                "gravity_mps2",
            ));
        }

        for (index, artifact) in self.world_assets.iter().enumerate() {
            artifact.validate_at(&format!("world_assets[{index}]"), &mut issues);
        }

        let mut solver_ids = BTreeSet::new();
        for (index, solver) in self.solvers.iter().enumerate() {
            let path = format!("solvers[{index}]");
            if solver.solver_id.trim().is_empty()
                || solver.implementation.trim().is_empty()
                || solver.version.trim().is_empty()
                || solver.output_contract_version == 0
            {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::InvalidSolver,
                    &path,
                ));
            }
            if !solver_ids.insert(solver.solver_id.as_str()) {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::DuplicateIdentity,
                    format!("{path}.solver_id"),
                ));
            }
            if !is_sha256_hex(&solver.build_sha256) {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::InvalidDigest,
                    format!("{path}.build_sha256"),
                ));
            }
            if !is_sha256_hex(&solver.parameters_sha256) {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::InvalidDigest,
                    format!("{path}.parameters_sha256"),
                ));
            }
        }

        let mut sensor_ids = BTreeSet::new();
        for (index, sensor) in self.sensors.iter().enumerate() {
            let path = format!("sensors[{index}]");
            if sensor.sensor_id.trim().is_empty() {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::InvalidSensor,
                    format!("{path}.sensor_id"),
                ));
            }
            if !sensor_ids.insert(sensor.sensor_id.as_str()) {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::DuplicateIdentity,
                    format!("{path}.sensor_id"),
                ));
            }
            if !sensor.sample_rate_hz.is_finite()
                || !(1.0e-9..=1.0e9).contains(&sensor.sample_rate_hz)
            {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::InvalidSensorRate,
                    format!("{path}.sample_rate_hz"),
                ));
            }
            if sensor.jitter_ns > i64::MAX as u64 {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::InvalidRange,
                    format!("{path}.jitter_ns"),
                ));
            }
            if !sensor.noise_std.is_finite() || sensor.noise_std < 0.0 {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::InvalidSensorNoise,
                    format!("{path}.noise_std"),
                ));
            }
            if sensor.noise_unit.trim().is_empty() {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::MissingUnit,
                    format!("{path}.noise_unit"),
                ));
            }
            if !sensor.dropout_probability.is_finite()
                || !(0.0..=1.0).contains(&sensor.dropout_probability)
            {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::InvalidProbability,
                    format!("{path}.dropout_probability"),
                ));
            }
            if sensor.saturation_min.is_some() != sensor.saturation_max.is_some() {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::InvalidRange,
                    format!("{path}.saturation"),
                ));
            }
            if let (Some(min), Some(max)) = (sensor.saturation_min, sensor.saturation_max) {
                if !min.is_finite() || !max.is_finite() || min >= max {
                    issues.push(WorldValidationIssue::new(
                        WorldValidationCode::InvalidRange,
                        format!("{path}.saturation"),
                    ));
                }
            } else if sensor
                .saturation_min
                .is_some_and(|value| !value.is_finite())
                || sensor
                    .saturation_max
                    .is_some_and(|value| !value.is_finite())
            {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::InvalidRange,
                    format!("{path}.saturation"),
                ));
            }
            if let Some(artifact) = &sensor.calibration_artifact {
                artifact.validate_at(&format!("{path}.calibration_artifact"), &mut issues);
            }
        }

        for (index, sensor) in self.sensors.iter().enumerate() {
            if let Some(solver_id) = sensor.model_solver_id.as_deref() {
                if !solver_ids.contains(solver_id) {
                    issues.push(WorldValidationIssue::new(
                        WorldValidationCode::UnknownSolverReference,
                        format!("sensors[{index}].model_solver_id"),
                    ));
                }
            }
        }

        let mut platform_ids = BTreeSet::new();
        for (index, platform) in self.platforms.iter().enumerate() {
            let path = format!("platforms[{index}]");
            if platform.platform_id.trim().is_empty()
                || !platform.mass_kg.is_finite()
                || platform.mass_kg <= 0.0
                || !platform.control_rate_hz.is_finite()
                || platform.control_rate_hz <= 0.0
                || platform.controller_interface_version == 0
                || platform.actuators.is_empty()
            {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::InvalidPlatform,
                    &path,
                ));
            }
            if !platform_ids.insert(platform.platform_id.as_str()) {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::DuplicateIdentity,
                    format!("{path}.platform_id"),
                ));
            }
            platform
                .geometry
                .validate_at(&format!("{path}.geometry"), &mut issues);
            platform
                .kinematics
                .validate_at(&format!("{path}.kinematics"), &mut issues);

            let mut actuator_ids = BTreeSet::new();
            for (actuator_index, actuator) in platform.actuators.iter().enumerate() {
                let actuator_path = format!("{path}.actuators[{actuator_index}]");
                if actuator.actuator_id.trim().is_empty()
                    || !actuator_ids.insert(actuator.actuator_id.as_str())
                {
                    issues.push(WorldValidationIssue::new(
                        WorldValidationCode::InvalidActuator,
                        format!("{actuator_path}.actuator_id"),
                    ));
                }
                if !actuator.min_command.is_finite()
                    || !actuator.max_command.is_finite()
                    || actuator.min_command >= actuator.max_command
                {
                    issues.push(WorldValidationIssue::new(
                        WorldValidationCode::InvalidRange,
                        format!("{actuator_path}.command_range"),
                    ));
                }
                if actuator.command_unit.trim().is_empty() {
                    issues.push(WorldValidationIssue::new(
                        WorldValidationCode::MissingUnit,
                        format!("{actuator_path}.command_unit"),
                    ));
                }
            }
            let mut attached_sensors = BTreeSet::new();
            for (sensor_index, sensor_id) in platform.sensor_ids.iter().enumerate() {
                if !sensor_ids.contains(sensor_id.as_str()) {
                    issues.push(WorldValidationIssue::new(
                        WorldValidationCode::UnknownSensorReference,
                        format!("{path}.sensor_ids[{sensor_index}]"),
                    ));
                }
                if !attached_sensors.insert(sensor_id.as_str()) {
                    issues.push(WorldValidationIssue::new(
                        WorldValidationCode::DuplicateIdentity,
                        format!("{path}.sensor_ids[{sensor_index}]"),
                    ));
                }
            }
        }

        self.validate_coordinate_frames(&mut issues);

        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }

    fn validate_coordinate_frames(&self, issues: &mut Vec<WorldValidationIssue>) {
        if self.frames.is_empty() {
            issues.push(WorldValidationIssue::new(
                WorldValidationCode::MissingFrame,
                "frames",
            ));
            return;
        }

        let mut frame_ids = BTreeSet::new();
        for (index, frame) in self.frames.iter().enumerate() {
            let path = format!("frames[{index}]");
            if frame.frame_id.trim().is_empty()
                || frame.frame_id.trim() != frame.frame_id.as_str()
            {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::InvalidFrame,
                    format!("{path}.frame_id"),
                ));
            }
            if !frame_ids.insert(frame.frame_id.as_str()) {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::DuplicateIdentity,
                    format!("{path}.frame_id"),
                ));
            }
        }

        let roots = self.frames.iter().filter(|frame| frame.parent_frame_id.is_none()).count();
        let world_root_is_valid = self.frames.iter().any(|frame| {
            frame.frame_id == "world"
                && frame.parent_frame_id.is_none()
                && matches!(&frame.transform_to_parent, FrameTransformV1::Root)
        });
        if roots != 1 || !world_root_is_valid {
            issues.push(WorldValidationIssue::new(
                WorldValidationCode::MissingFrame,
                "frames.world_root",
            ));
        }

        let parent_by_id: BTreeMap<&str, Option<&str>> = self
            .frames
            .iter()
            .map(|frame| (frame.frame_id.as_str(), frame.parent_frame_id.as_deref()))
            .collect();

        for (index, frame) in self.frames.iter().enumerate() {
            let path = format!("frames[{index}]");
            if let Some(parent_id) = frame.parent_frame_id.as_deref() {
                if parent_id == frame.frame_id.as_str() {
                    issues.push(WorldValidationIssue::new(
                        WorldValidationCode::InvalidFrame,
                        format!("{path}.parent_frame_id"),
                    ));
                } else if !frame_ids.contains(parent_id) {
                    issues.push(WorldValidationIssue::new(
                        WorldValidationCode::UnknownFrameReference,
                        format!("{path}.parent_frame_id"),
                    ));
                }
            }

            match &frame.transform_to_parent {
                FrameTransformV1::Root => {
                    if frame.frame_id != "world" || frame.parent_frame_id.is_some() {
                        issues.push(WorldValidationIssue::new(
                            WorldValidationCode::InvalidFrame,
                            format!("{path}.transform_to_parent"),
                        ));
                    }
                }
                FrameTransformV1::Static {
                    translation_m,
                    rotation_xyzw,
                } => {
                    if frame.parent_frame_id.is_none() {
                        issues.push(WorldValidationIssue::new(
                            WorldValidationCode::InvalidFrame,
                            format!("{path}.parent_frame_id"),
                        ));
                    }
                    let translation_is_finite = translation_m.iter().all(|value| value.is_finite());
                    let quaternion_is_finite =
                        rotation_xyzw.iter().all(|value| value.is_finite());
                    let quaternion_norm_squared =
                        rotation_xyzw.iter().map(|value| value * value).sum::<f64>();
                    if !translation_is_finite
                        || !quaternion_is_finite
                        || (quaternion_norm_squared - 1.0).abs() > 1e-6
                    {
                        issues.push(WorldValidationIssue::new(
                            WorldValidationCode::InvalidTransform,
                            format!("{path}.transform_to_parent"),
                        ));
                    }
                }
                FrameTransformV1::Kinematic {
                    provider_artifact_id,
                    provider_frame_id,
                } => {
                    if frame.parent_frame_id.is_none()
                        || provider_artifact_id.trim().is_empty()
                        || provider_artifact_id.trim() != provider_artifact_id.as_str()
                        || provider_frame_id.trim().is_empty()
                        || provider_frame_id.trim() != provider_frame_id.as_str()
                    {
                        issues.push(WorldValidationIssue::new(
                            WorldValidationCode::InvalidTransform,
                            format!("{path}.transform_to_parent"),
                        ));
                    }
                    let provider_is_declared = self.platforms.iter().any(|platform| {
                        platform.kinematics.artifact_id.as_str()
                            == provider_artifact_id.as_str()
                    });
                    if !provider_is_declared {
                        issues.push(WorldValidationIssue::new(
                            WorldValidationCode::UnknownFrameProvider,
                            format!("{path}.transform_to_parent.provider_artifact_id"),
                        ));
                    }
                }
            }
        }

        // A valid parent registry is a forest with exactly one world root;
        // reject cycles explicitly rather than relying on root-count checks.
        for (index, frame) in self.frames.iter().enumerate() {
            let mut visited = BTreeSet::new();
            let mut cursor = Some(frame.frame_id.as_str());
            while let Some(frame_id) = cursor {
                if !visited.insert(frame_id) {
                    issues.push(WorldValidationIssue::new(
                        WorldValidationCode::FrameCycle,
                        format!("frames[{index}].parent_frame_id"),
                    ));
                    break;
                }
                cursor = parent_by_id.get(frame_id).copied().flatten();
            }
        }

        for (index, sensor) in self.sensors.iter().enumerate() {
            if !frame_ids.contains(sensor.frame_id.as_str()) {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::UnknownFrameReference,
                    format!("sensors[{index}].frame_id"),
                ));
            }
        }
        for (index, platform) in self.platforms.iter().enumerate() {
            if !frame_ids.contains(platform.base_frame_id.as_str()) {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::UnknownFrameReference,
                    format!("platforms[{index}].base_frame_id"),
                ));
            }
        }

        let frame_by_id: BTreeMap<&str, &CoordinateFrameV1> = self
            .frames
            .iter()
            .map(|frame| (frame.frame_id.as_str(), frame))
            .collect();
        for (platform_index, platform) in self.platforms.iter().enumerate() {
            if let Some(base_frame) = frame_by_id.get(platform.base_frame_id.as_str()) {
                match &base_frame.transform_to_parent {
                    FrameTransformV1::Kinematic {
                        provider_artifact_id,
                        ..
                    } if provider_artifact_id.as_str()
                        == platform.kinematics.artifact_id.as_str() => {}
                    _ => issues.push(WorldValidationIssue::new(
                        WorldValidationCode::InvalidPlatformBaseFrame,
                        format!("platforms[{platform_index}].base_frame_id"),
                    )),
                }
            }

            for (sensor_index, sensor_id) in platform.sensor_ids.iter().enumerate() {
                let Some(sensor) = self
                    .sensors
                    .iter()
                    .find(|sensor| sensor.sensor_id.as_str() == sensor_id.as_str())
                else {
                    // The unknown sensor reference is already reported above.
                    continue;
                };

                let mut visited = BTreeSet::new();
                let mut cursor = Some(sensor.frame_id.as_str());
                let mut descends_from_platform = false;
                while let Some(frame_id) = cursor {
                    if !visited.insert(frame_id) {
                        break;
                    }
                    if frame_id == platform.base_frame_id.as_str() {
                        descends_from_platform = true;
                        break;
                    }
                    cursor = parent_by_id.get(frame_id).copied().flatten();
                }
                if !descends_from_platform {
                    issues.push(WorldValidationIssue::new(
                        WorldValidationCode::SensorFrameNotAttachedToPlatform,
                        format!(
                            "platforms[{platform_index}].sensor_ids[{sensor_index}]"
                        ),
                    ));
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolverRequestIdentityV1 {
    pub run_id: String,
    pub request_id: u64,
    /// Scenario identity must match the manifest in which this request runs.
    pub scenario_id: String,
    pub world_generation: u64,
    pub solver_id: String,
    pub input_digest_sha256: String,
    pub simulation_time_start_ns: u64,
    pub simulation_time_end_ns: u64,
}

impl SolverRequestIdentityV1 {
    pub fn validate(&self) -> Result<(), Vec<WorldValidationIssue>> {
        let mut issues = Vec::new();
        if self.run_id.trim().is_empty()
            || self.scenario_id.trim().is_empty()
            || self.solver_id.trim().is_empty()
        {
            issues.push(WorldValidationIssue::new(
                WorldValidationCode::EmptyIdentity,
                "request_identity",
            ));
        }
        if self.scenario_id.trim() != self.scenario_id.as_str() {
            issues.push(WorldValidationIssue::new(
                WorldValidationCode::EmptyIdentity,
                "request_identity.scenario_id",
            ));
        }
        if self.simulation_time_end_ns < self.simulation_time_start_ns {
            issues.push(WorldValidationIssue::new(
                WorldValidationCode::InvalidTimeInterval,
                "request_identity.simulation_time",
            ));
        }
        if !is_sha256_hex(&self.input_digest_sha256) {
            issues.push(WorldValidationIssue::new(
                WorldValidationCode::InvalidDigest,
                "request_identity.input_digest_sha256",
            ));
        }
        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuantityValueV1 {
    pub quantity: String,
    pub value: f64,
    pub unit: String,
    pub frame_id: String,
    pub uncertainty_std: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolverResultDisposition {
    CompletedWithinEnvelope,
    CompletedWithWarnings,
    OutOfDomain,
    InsufficientInputEvidence,
    NumericalNonConvergence,
    DeadlineExceeded,
    InvalidResult,
    Cancelled,
}

impl SolverResultDisposition {
    const fn requires_output_digest(self) -> bool {
        matches!(
            self,
            Self::CompletedWithinEnvelope | Self::CompletedWithWarnings
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolverResultEnvelopeV1 {
    pub schema_version: u16,
    pub request: SolverRequestIdentityV1,
    pub disposition: SolverResultDisposition,
    pub output_digest_sha256: Option<String>,
    pub outputs: Vec<QuantityValueV1>,
    pub warnings: Vec<String>,
}

impl SolverResultEnvelopeV1 {
    /// Validate the result's own structure, then bind it to the exact request.
    ///
    /// Results for a different request, world generation, input digest, or
    /// simulation-time interval are rejected as stale/mismatched evidence.
    /// This validates structure and request identity only; world admission must
    /// use WorldManifestV1::validate_solver_result_against_world to also check
    /// output frames and that the requested solver is declared by the world.
    pub fn validate_against(
        &self,
        expected: &SolverRequestIdentityV1,
    ) -> Result<(), Vec<WorldValidationIssue>> {
        let mut issues = Vec::new();

        if self.schema_version != SOLVER_RESULT_SCHEMA_VERSION {
            issues.push(WorldValidationIssue::new(
                WorldValidationCode::UnsupportedSchemaVersion,
                "schema_version",
            ));
        }
        if let Err(mut request_issues) = self.request.validate() {
            issues.append(&mut request_issues);
        }
        if &self.request != expected {
            issues.push(WorldValidationIssue::new(
                WorldValidationCode::RequestIdentityMismatch,
                "request",
            ));
        }
        if self.disposition.requires_output_digest() {
            match self.output_digest_sha256.as_deref() {
                Some(digest) if is_sha256_hex(digest) => {}
                _ => issues.push(WorldValidationIssue::new(
                    WorldValidationCode::InvalidDigest,
                    "output_digest_sha256",
                )),
            }
        } else if self
            .output_digest_sha256
            .as_deref()
            .is_some_and(|digest| !is_sha256_hex(digest))
        {
            issues.push(WorldValidationIssue::new(
                WorldValidationCode::InvalidDigest,
                "output_digest_sha256",
            ));
        }

        for (index, output) in self.outputs.iter().enumerate() {
            let path = format!("outputs[{index}]");
            if output.quantity.trim().is_empty() {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::InvalidOutput,
                    format!("{path}.quantity"),
                ));
            }
            if !output.value.is_finite()
                || output
                    .uncertainty_std
                    .is_some_and(|uncertainty| !uncertainty.is_finite() || uncertainty < 0.0)
            {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::NonFiniteValue,
                    format!("{path}.value_or_uncertainty"),
                ));
            }
            if output.unit.trim().is_empty() {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::MissingUnit,
                    format!("{path}.unit"),
                ));
            }
            if output.frame_id.trim().is_empty() {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::MissingFrame,
                    format!("{path}.frame_id"),
                ));
            }
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }
}

impl WorldManifestV1 {
    /// Validate the world, exact request identity, result structure, and every
    /// result frame reference before the result is admitted to this world.
    pub fn validate_solver_result_against_world(
        &self,
        expected_request: &SolverRequestIdentityV1,
        result: &SolverResultEnvelopeV1,
    ) -> Result<(), Vec<WorldValidationIssue>> {
        let mut issues = Vec::new();
        if let Err(mut manifest_issues) = self.validate() {
            issues.append(&mut manifest_issues);
        }
        if let Err(mut result_issues) = result.validate_against(expected_request) {
            issues.append(&mut result_issues);
        }

        if expected_request.scenario_id.as_str() != self.scenario_id.as_str() {
            issues.push(WorldValidationIssue::new(
                WorldValidationCode::ScenarioIdentityMismatch,
                "request.scenario_id",
            ));
        }
        if expected_request.world_generation != self.world_generation {
            issues.push(WorldValidationIssue::new(
                WorldValidationCode::WorldGenerationMismatch,
                "request.world_generation",
            ));
        }

        if !self
            .solvers
            .iter()
            .any(|solver| solver.solver_id.as_str() == expected_request.solver_id.as_str())
        {
            issues.push(WorldValidationIssue::new(
                WorldValidationCode::UnknownSolverReference,
                "request.solver_id",
            ));
        }

        let frame_ids: BTreeSet<&str> = self
            .frames
            .iter()
            .map(|frame| frame.frame_id.as_str())
            .collect();
        for (index, output) in result.outputs.iter().enumerate() {
            if !frame_ids.contains(output.frame_id.as_str()) {
                issues.push(WorldValidationIssue::new(
                    WorldValidationCode::UnknownFrameReference,
                    format!("outputs[{index}].frame_id"),
                ));
            }
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldValidationIssue {
    pub code: WorldValidationCode,
    pub path: String,
}

impl WorldValidationIssue {
    fn new(code: WorldValidationCode, path: impl Into<String>) -> Self {
        Self {
            code,
            path: path.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldValidationCode {
    UnsupportedSchemaVersion,
    EmptyIdentity,
    InvalidTimeStep,
    InvalidTimeInterval,
    NonFiniteValue,
    InvalidArtifact,
    InvalidDigest,
    DuplicateIdentity,
    InvalidSolver,
    InvalidSensor,
    InvalidSensorRate,
    InvalidSensorNoise,
    InvalidProbability,
    InvalidRange,
    MissingUnit,
    MissingFrame,
    InvalidPlatform,
    InvalidActuator,
    UnknownSolverReference,
    UnknownSensorReference,
    InvalidOutput,
    RequestIdentityMismatch,
    InvalidFrame,
    FrameCycle,
    InvalidTransform,
    MissingFrame,
    UnknownFrameReference,
    UnknownFrameProvider,
    InvalidWorldGeneration,
    ScenarioIdentityMismatch,
    WorldGenerationMismatch,
    InvalidPlatformBaseFrame,
    SensorFrameNotAttachedToPlatform,
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == SHA256_HEX_LENGTH && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(id: &str) -> ArtifactRef {
        ArtifactRef {
            artifact_id: id.to_owned(),
            uri: format!("urn:symtropy:test:{id}"),
            sha256: "a".repeat(SHA256_HEX_LENGTH),
        }
    }

    fn valid_manifest() -> WorldManifestV1 {
        WorldManifestV1 {
            schema_version: WORLD_MANIFEST_SCHEMA_VERSION,
            scenario_id: "hard-rock-workcell-smoke-v1".to_owned(),
            world_generation: 12,
            seed: 42,
            fixed_step_ns: 1_000_000,
            gravity_mps2: [0.0, -9.81, 0.0],
            frames: vec![
                CoordinateFrameV1 {
                    frame_id: "world".to_owned(),
                    parent_frame_id: None,
                    transform_to_parent: FrameTransformV1::Root,
                },
                CoordinateFrameV1 {
                    frame_id: "rock-scout-base".to_owned(),
                    parent_frame_id: Some("world".to_owned()),
                    transform_to_parent: FrameTransformV1::Kinematic {
                        provider_artifact_id: "robot-kinematics".to_owned(),
                        provider_frame_id: "base_link".to_owned(),
                    },
                },
                CoordinateFrameV1 {
                    frame_id: "ae-front-left".to_owned(),
                    parent_frame_id: Some("rock-scout-base".to_owned()),
                    transform_to_parent: FrameTransformV1::Static {
                        translation_m: [0.1, 0.05, 0.2],
                        rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
                    },
                },
            ],
            fidelity: PhysicsFidelity::ReducedOrder,
            world_assets: vec![artifact("tunnel-geometry")],
            solvers: vec![SolverDeclarationV1 {
                solver_id: "acoustic-reference".to_owned(),
                domain: PhysicsDomain::Acoustic,
                implementation: "low-order-wave-propagation".to_owned(),
                version: "0.1.0".to_owned(),
                build_sha256: "b".repeat(SHA256_HEX_LENGTH),
                parameters_sha256: "c".repeat(SHA256_HEX_LENGTH),
                output_contract_version: 1,
                fidelity: PhysicsFidelity::DeterministicReference,
            }],
            sensors: vec![SensorProfileV1 {
                sensor_id: "ae-front-left".to_owned(),
                modality: SensorModality::AcousticEmission,
                frame_id: "ae-front-left".to_owned(),
                model_solver_id: Some("acoustic-reference".to_owned()),
                calibration_artifact: Some(artifact("ae-calibration")),
                sample_rate_hz: 48_000.0,
                latency_ns: 2_000_000,
                jitter_ns: 10_000,
                noise_std: 0.01,
                noise_unit: "m/s".to_owned(),
                dropout_probability: 0.01,
                saturation_min: Some(-1.0),
                saturation_max: Some(1.0),
            }],
            platforms: vec![PlatformProfileV1 {
                platform_id: "rock-scout-01".to_owned(),
                platform_type: PlatformType::Manipulator,
                geometry: artifact("robot-geometry"),
                kinematics: artifact("robot-kinematics"),
                base_frame_id: "rock-scout-base".to_owned(),
                mass_kg: 25.0,
                control_rate_hz: 200.0,
                controller_interface_version: 1,
                actuators: vec![ActuatorProfileV1 {
                    actuator_id: "cutter-feed".to_owned(),
                    min_command: 0.0,
                    max_command: 0.01,
                    command_unit: "m/s".to_owned(),
                }],
                sensor_ids: vec!["ae-front-left".to_owned()],
            }],
        }
    }

    fn valid_request() -> SolverRequestIdentityV1 {
        SolverRequestIdentityV1 {
            run_id: "run-0001".to_owned(),
            request_id: 7,
            scenario_id: "hard-rock-workcell-smoke-v1".to_owned(),
            world_generation: 12,
            solver_id: "acoustic-reference".to_owned(),
            input_digest_sha256: "d".repeat(SHA256_HEX_LENGTH),
            simulation_time_start_ns: 10_000_000,
            simulation_time_end_ns: 11_000_000,
        }
    }

    #[test]
    fn valid_world_manifest_is_admitted() {
        assert_eq!(valid_manifest().validate(), Ok(()));
    }

    #[test]
    fn invalid_time_and_non_finite_gravity_are_rejected() {
        let mut manifest = valid_manifest();
        manifest.fixed_step_ns = 0;
        manifest.gravity_mps2[1] = f64::NAN;
        let issues = manifest.validate().unwrap_err();
        assert!(issues.iter().any(|issue| issue.code == WorldValidationCode::InvalidTimeStep));
        assert!(issues.iter().any(|issue| issue.code == WorldValidationCode::NonFiniteValue));
    }

    #[test]
    fn missing_sensor_solver_and_bad_sensor_units_are_rejected() {
        let mut manifest = valid_manifest();
        manifest.sensors[0].model_solver_id = Some("missing-solver".to_owned());
        manifest.sensors[0].noise_unit.clear();
        let issues = manifest.validate().unwrap_err();
        assert!(issues.iter().any(|issue| issue.code == WorldValidationCode::UnknownSolverReference));
        assert!(issues.iter().any(|issue| issue.code == WorldValidationCode::MissingUnit));
    }

    #[test]
    fn sensor_timing_parameters_must_fit_integer_nanosecond_clock() {
        let mut manifest = valid_manifest();
        manifest.sensors[0].sample_rate_hz = 1.0e10;
        manifest.sensors[0].jitter_ns = (i64::MAX as u64) + 1;
        let issues = manifest.validate().unwrap_err();
        assert!(issues.iter().any(|issue| {
            issue.code == WorldValidationCode::InvalidSensorRate
                && issue.path == "sensors[0].sample_rate_hz"
        }));
        assert!(issues.iter().any(|issue| {
            issue.code == WorldValidationCode::InvalidRange
                && issue.path == "sensors[0].jitter_ns"
        }));
    }

    #[test]
    fn duplicate_sensor_identity_is_rejected() {
        let mut manifest = valid_manifest();
        let duplicate = manifest.sensors[0].clone();
        manifest.sensors.push(duplicate);
        let issues = manifest.validate().unwrap_err();
        assert!(issues.iter().any(|issue| {
            issue.code == WorldValidationCode::DuplicateIdentity
                && issue.path == "sensors[1].sensor_id"
        }));
    }

    #[test]
    fn solver_result_must_match_exact_request_generation_and_input_digest() {
        let expected = valid_request();
        let mut result = SolverResultEnvelopeV1 {
            schema_version: SOLVER_RESULT_SCHEMA_VERSION,
            request: expected.clone(),
            disposition: SolverResultDisposition::CompletedWithinEnvelope,
            output_digest_sha256: Some("e".repeat(SHA256_HEX_LENGTH)),
            outputs: vec![QuantityValueV1 {
                quantity: "wave_speed".to_owned(),
                value: 5200.0,
                unit: "m/s".to_owned(),
                frame_id: "world".to_owned(),
                uncertainty_std: Some(50.0),
            }],
            warnings: Vec::new(),
        };
        assert_eq!(result.validate_against(&expected), Ok(()));

        result.request.world_generation += 1;
        let issues = result.validate_against(&expected).unwrap_err();
        assert!(issues.iter().any(|issue| {
            issue.code == WorldValidationCode::RequestIdentityMismatch
        }));
    }

    #[test]
    fn solver_result_rejects_stale_input_digest_and_time_interval() {
        let expected = valid_request();
        let mut result = SolverResultEnvelopeV1 {
            schema_version: SOLVER_RESULT_SCHEMA_VERSION,
            request: expected.clone(),
            disposition: SolverResultDisposition::CompletedWithinEnvelope,
            output_digest_sha256: Some("e".repeat(SHA256_HEX_LENGTH)),
            outputs: Vec::new(),
            warnings: Vec::new(),
        };

        result.request.input_digest_sha256 = "f".repeat(SHA256_HEX_LENGTH);
        assert!(result.validate_against(&expected).unwrap_err().iter().any(|issue| {
            issue.code == WorldValidationCode::RequestIdentityMismatch
        }));

        result.request = expected.clone();
        result.request.simulation_time_end_ns += 1;
        assert!(result.validate_against(&expected).unwrap_err().iter().any(|issue| {
            issue.code == WorldValidationCode::RequestIdentityMismatch
        }));
    }

    #[test]
    fn successful_result_requires_digest_and_unit_bearing_finite_outputs() {
        let expected = valid_request();
        let mut result = SolverResultEnvelopeV1 {
            schema_version: SOLVER_RESULT_SCHEMA_VERSION,
            request: expected.clone(),
            disposition: SolverResultDisposition::CompletedWithinEnvelope,
            output_digest_sha256: None,
            outputs: vec![QuantityValueV1 {
                quantity: "penetration_rate".to_owned(),
                value: f64::INFINITY,
                unit: String::new(),
                frame_id: String::new(),
                uncertainty_std: Some(-1.0),
            }],
            warnings: Vec::new(),
        };
        let issues = result.validate_against(&expected).unwrap_err();
        assert!(issues.iter().any(|issue| issue.code == WorldValidationCode::InvalidDigest));
        assert!(issues.iter().any(|issue| issue.code == WorldValidationCode::NonFiniteValue));
        assert!(issues.iter().any(|issue| issue.code == WorldValidationCode::MissingUnit));
        assert!(issues.iter().any(|issue| issue.code == WorldValidationCode::MissingFrame));
        result.disposition = SolverResultDisposition::NumericalNonConvergence;
        result.output_digest_sha256 = None;
        result.outputs.clear();
        assert_eq!(result.validate_against(&expected), Ok(()));
    }

    #[test]
    fn sensor_and_platform_frames_must_exist_in_world_registry() {
        let mut manifest = valid_manifest();
        manifest.sensors[0].frame_id = "missing-sensor-frame".to_owned();
        manifest.platforms[0].base_frame_id = "missing-base-frame".to_owned();
        let issues = manifest.validate().unwrap_err();
        assert!(issues.iter().any(|issue| {
            issue.code == WorldValidationCode::UnknownFrameReference
                && issue.path == "sensors[0].frame_id"
        }));
        assert!(issues.iter().any(|issue| {
            issue.code == WorldValidationCode::UnknownFrameReference
                && issue.path == "platforms[0].base_frame_id"
        }));
    }

    #[test]
    fn coordinate_frame_cycles_are_rejected() {
        let mut manifest = valid_manifest();
        manifest.frames[1].parent_frame_id = Some("ae-front-left".to_owned());
        let issues = manifest.validate().unwrap_err();
        assert!(issues.iter().any(|issue| issue.code == WorldValidationCode::FrameCycle));
    }

    #[test]
    fn static_transform_requires_finite_translation_and_unit_quaternion() {
        let mut manifest = valid_manifest();
        manifest.frames[2].transform_to_parent = FrameTransformV1::Static {
            translation_m: [0.0, f64::NAN, 0.0],
            rotation_xyzw: [0.0, 0.0, 0.0, 2.0],
        };
        let issues = manifest.validate().unwrap_err();
        assert!(issues.iter().any(|issue| {
            issue.code == WorldValidationCode::InvalidTransform
                && issue.path == "frames[2].transform_to_parent"
        }));
    }

    #[test]
    fn kinematic_frame_must_reference_declared_platform_kinematics() {
        let mut manifest = valid_manifest();
        manifest.frames[1].transform_to_parent = FrameTransformV1::Kinematic {
            provider_artifact_id: "unregistered-kinematics".to_owned(),
            provider_frame_id: "base_link".to_owned(),
        };
        let issues = manifest.validate().unwrap_err();
        assert!(issues.iter().any(|issue| {
            issue.code == WorldValidationCode::UnknownFrameProvider
                && issue.path == "frames[1].transform_to_parent.provider_artifact_id"
        }));
    }

    #[test]
    fn world_admission_rejects_undeclared_solver() {
        let manifest = valid_manifest();
        let mut expected = valid_request();
        expected.solver_id = "missing-solver".to_owned();
        let result = SolverResultEnvelopeV1 {
            schema_version: SOLVER_RESULT_SCHEMA_VERSION,
            request: expected.clone(),
            disposition: SolverResultDisposition::NumericalNonConvergence,
            output_digest_sha256: None,
            outputs: Vec::new(),
            warnings: Vec::new(),
        };
        let issues = manifest
            .validate_solver_result_against_world(&expected, &result)
            .unwrap_err();
        assert!(issues.iter().any(|issue| {
            issue.code == WorldValidationCode::UnknownSolverReference
                && issue.path == "request.solver_id"
        }));
    }

    #[test]
    fn world_admission_rejects_result_in_undeclared_frame() {
        let manifest = valid_manifest();
        let expected = valid_request();
        let result = SolverResultEnvelopeV1 {
            schema_version: SOLVER_RESULT_SCHEMA_VERSION,
            request: expected.clone(),
            disposition: SolverResultDisposition::CompletedWithinEnvelope,
            output_digest_sha256: Some("e".repeat(SHA256_HEX_LENGTH)),
            outputs: vec![QuantityValueV1 {
                quantity: "penetration_rate".to_owned(),
                value: 0.002,
                unit: "m/s".to_owned(),
                frame_id: "invented-frame".to_owned(),
                uncertainty_std: Some(0.0001),
            }],
            warnings: Vec::new(),
        };
        let issues = manifest
            .validate_solver_result_against_world(&expected, &result)
            .unwrap_err();
        assert!(issues.iter().any(|issue| {
            issue.code == WorldValidationCode::UnknownFrameReference
                && issue.path == "outputs[0].frame_id"
        }));
    }


    #[test]
    fn world_admission_rejects_stale_world_generation() {
        let manifest = valid_manifest();
        let mut expected = valid_request();
        expected.world_generation += 1;
        let result = SolverResultEnvelopeV1 {
            schema_version: SOLVER_RESULT_SCHEMA_VERSION,
            request: expected.clone(),
            disposition: SolverResultDisposition::NumericalNonConvergence,
            output_digest_sha256: None,
            outputs: Vec::new(),
            warnings: Vec::new(),
        };
        let issues = manifest
            .validate_solver_result_against_world(&expected, &result)
            .unwrap_err();
        assert!(issues.iter().any(|issue| {
            issue.code == WorldValidationCode::WorldGenerationMismatch
                && issue.path == "request.world_generation"
        }));
    }

    #[test]
    fn world_admission_rejects_cross_scenario_result() {
        let manifest = valid_manifest();
        let mut expected = valid_request();
        expected.scenario_id = "different-scenario".to_owned();
        let result = SolverResultEnvelopeV1 {
            schema_version: SOLVER_RESULT_SCHEMA_VERSION,
            request: expected.clone(),
            disposition: SolverResultDisposition::NumericalNonConvergence,
            output_digest_sha256: None,
            outputs: Vec::new(),
            warnings: Vec::new(),
        };
        let issues = manifest
            .validate_solver_result_against_world(&expected, &result)
            .unwrap_err();
        assert!(issues.iter().any(|issue| {
            issue.code == WorldValidationCode::ScenarioIdentityMismatch
                && issue.path == "request.scenario_id"
        }));
    }


    #[test]
    fn platform_base_frame_must_use_its_declared_kinematics_provider() {
        let mut manifest = valid_manifest();
        manifest.frames[1].transform_to_parent = FrameTransformV1::Kinematic {
            provider_artifact_id: "different-robot-kinematics".to_owned(),
            provider_frame_id: "base_link".to_owned(),
        };
        let issues = manifest.validate().unwrap_err();
        assert!(issues.iter().any(|issue| {
            issue.code == WorldValidationCode::InvalidPlatformBaseFrame
                && issue.path == "platforms[0].base_frame_id"
        }));
    }

    #[test]
    fn attached_sensor_frame_must_descend_from_platform_base() {
        let mut manifest = valid_manifest();
        manifest.sensors[0].frame_id = "world".to_owned();
        let issues = manifest.validate().unwrap_err();
        assert!(issues.iter().any(|issue| {
            issue.code == WorldValidationCode::SensorFrameNotAttachedToPlatform
                && issue.path == "platforms[0].sensor_ids[0]"
        }));
    }


}
