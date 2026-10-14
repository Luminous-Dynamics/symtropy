// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Versioned contracts for replayable robotics digital-twin worlds.
//!
//! This module validates scenario, platform, sensor, solver, and result
//! identities at the boundary. It does not implement physics and does not
//! certify that any declared solver is physically accurate.

use crate::platform::PlatformType;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorProfileV1 {
    pub sensor_id: String,
    pub modality: SensorModality,
    pub model_solver_id: Option<String>,
    pub calibration_artifact: Option<ArtifactRef>,
    /// Nominal sample frequency in hertz; runtime model supports 1 nHz through 1 GHz.
    pub sample_rate_hz: f64,
    /// Nominal acquisition-to-delivery latency, excluding timestamp jitter.
    pub latency_ns: u64,
    /// Maximum absolute timestamp jitter in nanoseconds; the reference model draws uniformly.
    pub jitter_ns: u64,
    /// Scalar additive-noise standard deviation in noise_unit.
    pub noise_std: f64,
    pub noise_unit: String,
    /// Independent per-sample dropout probability in [0, 1].
    pub dropout_probability: f64,
    /// Optional scalar lower saturation bound, expressed in noise_unit.
    pub saturation_min: Option<f64>,
    /// Optional scalar upper saturation bound, expressed in noise_unit.
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
    pub seed: u64,
    /// Fixed world step in integer nanoseconds; wall-clock cadence is separate.
    pub fixed_step_ns: u64,
    pub gravity_mps2: [f64; 3],
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
            if sensor.jitter_ns > i64::MAX as u64
                || sensor.jitter_ns > sensor.latency_ns
            {
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

        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolverRequestIdentityV1 {
    pub run_id: String,
    pub request_id: u64,
    pub world_generation: u64,
    pub solver_id: String,
    pub input_digest_sha256: String,
    pub simulation_time_start_ns: u64,
    pub simulation_time_end_ns: u64,
}

impl SolverRequestIdentityV1 {
    pub fn validate(&self) -> Result<(), Vec<WorldValidationIssue>> {
        let mut issues = Vec::new();
        if self.run_id.trim().is_empty() || self.solver_id.trim().is_empty() {
            issues.push(WorldValidationIssue::new(
                WorldValidationCode::EmptyIdentity,
                "request_identity",
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
            seed: 42,
            fixed_step_ns: 1_000_000,
            gravity_mps2: [0.0, -9.81, 0.0],
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
        manifest.sensors[0].latency_ns = 1;
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
    fn jitter_cannot_exceed_latency_and_bias_the_timestamp_distribution() {
        let mut manifest = valid_manifest();
        manifest.sensors[0].latency_ns = 1;
        manifest.sensors[0].jitter_ns = 2;
        let issues = manifest.validate().unwrap_err();
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
}
