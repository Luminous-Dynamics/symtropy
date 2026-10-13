// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Canonical, content-addressed identity for a robotics world manifest.
//!
//! This module owns the v1 byte grammar and content digest only. It does not
//! authorize a world, authenticate an issuer, or establish physical validity.
//! Serde output, Debug/Display text, pointer layout, and map iteration order
//! are never part of the canonical bytes.

use crate::platform::PlatformType;
use crate::world_contract::{
    ActuatorProfileV1, ArtifactRef, CoordinateFrameV1, FrameTransformV1, PhysicsDomain,
    PhysicsFidelity, PlatformProfileV1, SensorModality, SensorProfileV1, SolverDeclarationV1,
    WorldManifestV1, WorldValidationIssue,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

pub const WORLD_MANIFEST_CANONICAL_FORMAT_VERSION_V1: u16 = 1;
pub const WORLD_MANIFEST_COMMITMENT_DOMAIN_V1: &[u8] =
    b"symtropy.robotics.world-manifest.v1\0";
pub const MAX_CANONICAL_WORLD_MANIFEST_BYTES: usize = 64 * 1024 * 1024;

/// A SHA-256 commitment to the canonical v1 bytes of one complete world
/// manifest. Equal values are content-address equality under SHA-256's
/// collision-resistance assumption, not a mathematical proof of equal preimages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorldManifestDigestV1([u8; 32]);

impl WorldManifestDigestV1 {
    /// Construct a claimed digest from bytes received over a boundary.
    ///
    /// The bytes are not trusted merely because they have this type: admission
    /// must compare this claim with a digest recomputed from the actual manifest.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Lowercase hexadecimal display form; presentation is not canonical input.
    pub fn to_hex(self) -> String {
        let mut out = String::with_capacity(64);
        for byte in self.0 {
            use fmt::Write as _;
            let _ = write!(out, "{byte:02x}");
        }
        out
    }
}

#[derive(Debug)]
pub enum WorldManifestCommitmentError {
    InvalidManifest(Vec<WorldValidationIssue>),
    CanonicalBytesTooLarge {
        limit_bytes: usize,
    },
}

impl fmt::Display for WorldManifestCommitmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidManifest(_) => {
                f.write_str("cannot commit an invalid robotics world manifest")
            }
            Self::CanonicalBytesTooLarge { limit_bytes } => write!(
                f,
                "canonical robotics world manifest exceeds {limit_bytes} bytes"
            ),
        }
    }
}

impl std::error::Error for WorldManifestCommitmentError {}

impl WorldManifestV1 {
    /// Encode the complete manifest using the frozen, language-neutral v1
    /// binary grammar. Collection fields that represent sets are sorted by
    /// stable identity; semantically ordered numeric arrays are kept in order.
    ///
    /// Floating-point values are encoded as their exact IEEE-754 binary64 bit
    /// patterns, big-endian. Non-finite values are rejected by validate().
    /// Consequently, +0.0 and -0.0 have distinct content identities in v1.
    pub fn canonical_bytes_v1(
        &self,
    ) -> Result<Vec<u8>, WorldManifestCommitmentError> {
        self.validate()
            .map_err(WorldManifestCommitmentError::InvalidManifest)?;

        let mut encoder = Encoder::new();
        encoder.raw(WORLD_MANIFEST_COMMITMENT_DOMAIN_V1);
        encoder.u16(WORLD_MANIFEST_CANONICAL_FORMAT_VERSION_V1);

        encoder.u16(self.schema_version);
        encoder.string(&self.scenario_id);
        encoder.u64(self.world_generation);
        encoder.u64(self.seed);
        encoder.u64(self.fixed_step_ns);
        for value in self.gravity_mps2 {
            encoder.f64_bits(value);
        }

        let mut frames: Vec<&CoordinateFrameV1> = self.frames.iter().collect();
        frames.sort_by(|a, b| a.frame_id.cmp(&b.frame_id));
        encoder.count(frames.len());
        for frame in frames {
            encode_frame(&mut encoder, frame);
        }

        encoder.u8(fidelity_tag(self.fidelity));

        let mut assets: Vec<&ArtifactRef> = self.world_assets.iter().collect();
        assets.sort_by(|a, b| {
            (&a.artifact_id, &a.uri, &a.sha256).cmp(&(&b.artifact_id, &b.uri, &b.sha256))
        });
        encoder.count(assets.len());
        for asset in assets {
            encode_artifact(&mut encoder, asset);
        }

        let mut solvers: Vec<&SolverDeclarationV1> = self.solvers.iter().collect();
        solvers.sort_by(|a, b| a.solver_id.cmp(&b.solver_id));
        encoder.count(solvers.len());
        for solver in solvers {
            encode_solver(&mut encoder, solver);
        }

        let mut sensors: Vec<&SensorProfileV1> = self.sensors.iter().collect();
        sensors.sort_by(|a, b| a.sensor_id.cmp(&b.sensor_id));
        encoder.count(sensors.len());
        for sensor in sensors {
            encode_sensor(&mut encoder, sensor);
        }

        let mut platforms: Vec<&PlatformProfileV1> = self.platforms.iter().collect();
        platforms.sort_by(|a, b| a.platform_id.cmp(&b.platform_id));
        encoder.count(platforms.len());
        for platform in platforms {
            encode_platform(&mut encoder, platform);
        }

        encoder.finish()
    }

    /// Compute a domain-separated SHA-256 digest over canonical_bytes_v1().
    ///
    /// The digest is a content identifier, not authorization, authentication,
    /// physical validation, or proof that the manifest was actually executed.
    pub fn content_digest_v1(
        &self,
    ) -> Result<WorldManifestDigestV1, WorldManifestCommitmentError> {
        let bytes = self.canonical_bytes_v1()?;
        let digest = Sha256::digest(bytes);
        let mut fixed = [0u8; 32];
        fixed.copy_from_slice(&digest);
        Ok(WorldManifestDigestV1(fixed))
    }
}

struct Encoder {
    bytes: Vec<u8>,
    too_large: bool,
}

impl Encoder {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            too_large: false,
        }
    }

    fn raw(&mut self, bytes: &[u8]) {
        if self.too_large {
            return;
        }
        let Some(next_len) = self.bytes.len().checked_add(bytes.len()) else {
            self.too_large = true;
            return;
        };
        if next_len > MAX_CANONICAL_WORLD_MANIFEST_BYTES {
            self.too_large = true;
            return;
        }
        self.bytes.extend_from_slice(bytes);
    }

    fn u8(&mut self, value: u8) {
        self.raw(&[value]);
    }

    fn u16(&mut self, value: u16) {
        self.raw(&value.to_be_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.raw(&value.to_be_bytes());
    }

    fn count(&mut self, count: usize) {
        match u64::try_from(count) {
            Ok(value) => self.u64(value),
            Err(_) => self.too_large = true,
        }
    }

    fn string(&mut self, value: &str) {
        self.count(value.len());
        self.raw(value.as_bytes());
    }

    fn optional_string(&mut self, value: Option<&str>) {
        match value {
            None => self.u8(0),
            Some(value) => {
                self.u8(1);
                self.string(value);
            }
        }
    }

    fn optional_f64(&mut self, value: Option<f64>) {
        match value {
            None => self.u8(0),
            Some(value) => {
                self.u8(1);
                self.f64_bits(value);
            }
        }
    }

    fn f64_bits(&mut self, value: f64) {
        self.u64(value.to_bits());
    }

    fn sha256_hex(&mut self, value: &str) {
        // The enclosing manifest validator has already checked the exact
        // length and hexadecimal alphabet. Decode to 32 raw bytes so uppercase
        // and lowercase spellings of the same digest have one encoding.
        let source = value.as_bytes();
        let mut digest = [0u8; 32];
        for (index, out) in digest.iter_mut().enumerate() {
            let high = hex_nibble(source[index * 2]).expect("validated SHA-256 hex");
            let low = hex_nibble(source[index * 2 + 1]).expect("validated SHA-256 hex");
            *out = (high << 4) | low;
        }
        self.raw(&digest);
    }

    fn finish(self) -> Result<Vec<u8>, WorldManifestCommitmentError> {
        if self.too_large {
            Err(WorldManifestCommitmentError::CanonicalBytesTooLarge {
                limit_bytes: MAX_CANONICAL_WORLD_MANIFEST_BYTES,
            })
        } else {
            Ok(self.bytes)
        }
    }
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn encode_artifact(encoder: &mut Encoder, artifact: &ArtifactRef) {
    encoder.string(&artifact.artifact_id);
    encoder.string(&artifact.uri);
    encoder.sha256_hex(&artifact.sha256);
}

fn encode_frame(encoder: &mut Encoder, frame: &CoordinateFrameV1) {
    encoder.string(&frame.frame_id);
    encoder.optional_string(frame.parent_frame_id.as_deref());
    match &frame.transform_to_parent {
        FrameTransformV1::Root => encoder.u8(0),
        FrameTransformV1::Static {
            translation_m,
            rotation_xyzw,
        } => {
            encoder.u8(1);
            for value in translation_m {
                encoder.f64_bits(*value);
            }
            for value in rotation_xyzw {
                encoder.f64_bits(*value);
            }
        }
        FrameTransformV1::Kinematic {
            provider_artifact_id,
            provider_frame_id,
        } => {
            encoder.u8(2);
            encoder.string(provider_artifact_id);
            encoder.string(provider_frame_id);
        }
    }
}

fn encode_solver(encoder: &mut Encoder, solver: &SolverDeclarationV1) {
    encoder.string(&solver.solver_id);
    encoder.u8(domain_tag(solver.domain));
    encoder.string(&solver.implementation);
    encoder.string(&solver.version);
    encoder.sha256_hex(&solver.build_sha256);
    encoder.sha256_hex(&solver.parameters_sha256);
    encoder.u16(solver.output_contract_version);
    encoder.u8(fidelity_tag(solver.fidelity));
}

fn encode_sensor(encoder: &mut Encoder, sensor: &SensorProfileV1) {
    encoder.string(&sensor.sensor_id);
    encoder.u8(sensor_tag(sensor.modality));
    encoder.string(&sensor.frame_id);
    encoder.optional_string(sensor.model_solver_id.as_deref());
    match &sensor.calibration_artifact {
        None => encoder.u8(0),
        Some(artifact) => {
            encoder.u8(1);
            encode_artifact(encoder, artifact);
        }
    }
    encoder.f64_bits(sensor.sample_rate_hz);
    encoder.u64(sensor.latency_ns);
    encoder.u64(sensor.jitter_ns);
    encoder.f64_bits(sensor.noise_std);
    encoder.string(&sensor.noise_unit);
    encoder.f64_bits(sensor.dropout_probability);
    encoder.optional_f64(sensor.saturation_min);
    encoder.optional_f64(sensor.saturation_max);
}

fn encode_platform(encoder: &mut Encoder, platform: &PlatformProfileV1) {
    encoder.string(&platform.platform_id);
    encoder.u8(platform_tag(platform.platform_type));
    encode_artifact(encoder, &platform.geometry);
    encode_artifact(encoder, &platform.kinematics);
    encoder.string(&platform.base_frame_id);
    encoder.f64_bits(platform.mass_kg);
    encoder.f64_bits(platform.control_rate_hz);
    encoder.u16(platform.controller_interface_version);

    let mut actuators: Vec<&ActuatorProfileV1> = platform.actuators.iter().collect();
    actuators.sort_by(|a, b| a.actuator_id.cmp(&b.actuator_id));
    encoder.count(actuators.len());
    for actuator in actuators {
        encoder.string(&actuator.actuator_id);
        encoder.f64_bits(actuator.min_command);
        encoder.f64_bits(actuator.max_command);
        encoder.string(&actuator.command_unit);
    }

    let mut sensor_ids: Vec<&String> = platform.sensor_ids.iter().collect();
    sensor_ids.sort();
    encoder.count(sensor_ids.len());
    for sensor_id in sensor_ids {
        encoder.string(sensor_id);
    }
}

const fn domain_tag(domain: PhysicsDomain) -> u8 {
    match domain {
        PhysicsDomain::RigidBody => 0,
        PhysicsDomain::Terrain => 1,
        PhysicsDomain::MechanicalCutting => 2,
        PhysicsDomain::Acoustic => 3,
        PhysicsDomain::Thermal => 4,
        PhysicsDomain::Electromagnetic => 5,
        PhysicsDomain::ElectricalDischarge => 6,
        PhysicsDomain::Fluid => 7,
        PhysicsDomain::ContinuumMechanics => 8,
        PhysicsDomain::Fracture => 9,
        PhysicsDomain::SensorModel => 10,
    }
}

const fn fidelity_tag(fidelity: PhysicsFidelity) -> u8 {
    match fidelity {
        PhysicsFidelity::Kinematic => 0,
        PhysicsFidelity::DeterministicReference => 1,
        PhysicsFidelity::ReducedOrder => 2,
        PhysicsFidelity::DomainSolver => 3,
        PhysicsFidelity::CoupledRegion => 4,
    }
}

const fn sensor_tag(sensor: SensorModality) -> u8 {
    match sensor {
        SensorModality::Camera => 0,
        SensorModality::Lidar => 1,
        SensorModality::Imu => 2,
        SensorModality::Encoder => 3,
        SensorModality::ForceTorque => 4,
        SensorModality::Vibration => 5,
        SensorModality::AcousticEmission => 6,
        SensorModality::Ultrasound => 7,
        SensorModality::ThermalImager => 8,
        SensorModality::Thermocouple => 9,
        SensorModality::Pressure => 10,
        SensorModality::Flow => 11,
        SensorModality::Gas => 12,
        SensorModality::Electrical => 13,
    }
}

const fn platform_tag(platform: PlatformType) -> u8 {
    match platform {
        PlatformType::Quadrotor => 0,
        PlatformType::Vehicle => 1,
        PlatformType::Humanoid => 2,
        PlatformType::Auv => 3,
        PlatformType::Helicopter => 4,
        PlatformType::Manipulator => 5,
        PlatformType::Surgical => 6,
        PlatformType::Orbital => 7,
        PlatformType::Quadruped => 8,
        PlatformType::Exoskeleton => 9,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_contract::{
        ActuatorProfileV1, ArtifactRef, CoordinateFrameV1, FrameTransformV1, PhysicsDomain,
        PhysicsFidelity, PlatformProfileV1, SensorModality, SensorProfileV1,
        SolverDeclarationV1, WORLD_MANIFEST_SCHEMA_VERSION,
    };

    fn artifact(id: &str) -> ArtifactRef {
        ArtifactRef {
            artifact_id: id.to_owned(),
            uri: format!("urn:symtropy:test:{id}"),
            sha256: "ab".repeat(32),
        }
    }

    fn valid_manifest() -> WorldManifestV1 {
        WorldManifestV1 {
            schema_version: WORLD_MANIFEST_SCHEMA_VERSION,
            scenario_id: "commitment-smoke-v1".to_owned(),
            world_generation: 3,
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
                    frame_id: "robot-base".to_owned(),
                    parent_frame_id: Some("world".to_owned()),
                    transform_to_parent: FrameTransformV1::Kinematic {
                        provider_artifact_id: "robot-kinematics".to_owned(),
                        provider_frame_id: "base_link".to_owned(),
                    },
                },
                CoordinateFrameV1 {
                    frame_id: "camera-optical".to_owned(),
                    parent_frame_id: Some("robot-base".to_owned()),
                    transform_to_parent: FrameTransformV1::Static {
                        translation_m: [0.1, 0.0, 0.2],
                        rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
                    },
                },
            ],
            fidelity: PhysicsFidelity::ReducedOrder,
            world_assets: vec![artifact("environment"), artifact("robot-cad")],
            solvers: vec![SolverDeclarationV1 {
                solver_id: "rigid-reference".to_owned(),
                domain: PhysicsDomain::RigidBody,
                implementation: "symtropy-reference".to_owned(),
                version: "1.0.0".to_owned(),
                build_sha256: "cd".repeat(32),
                parameters_sha256: "ef".repeat(32),
                output_contract_version: 1,
                fidelity: PhysicsFidelity::DeterministicReference,
            }],
            sensors: vec![SensorProfileV1 {
                sensor_id: "camera-0".to_owned(),
                modality: SensorModality::Camera,
                frame_id: "camera-optical".to_owned(),
                model_solver_id: Some("rigid-reference".to_owned()),
                calibration_artifact: Some(artifact("camera-calibration")),
                sample_rate_hz: 30.0,
                latency_ns: 5_000_000,
                jitter_ns: 100_000,
                noise_std: 0.001,
                noise_unit: "m".to_owned(),
                dropout_probability: 0.01,
                saturation_min: Some(0.0),
                saturation_max: Some(1.0),
            }],
            platforms: vec![PlatformProfileV1 {
                platform_id: "robot-0".to_owned(),
                platform_type: PlatformType::Manipulator,
                geometry: artifact("robot-cad"),
                kinematics: artifact("robot-kinematics"),
                base_frame_id: "robot-base".to_owned(),
                mass_kg: 25.0,
                control_rate_hz: 200.0,
                controller_interface_version: 1,
                actuators: vec![
                    ActuatorProfileV1 {
                        actuator_id: "arm".to_owned(),
                        min_command: -1.0,
                        max_command: 1.0,
                        command_unit: "N*m".to_owned(),
                    },
                    ActuatorProfileV1 {
                        actuator_id: "feed".to_owned(),
                        min_command: 0.0,
                        max_command: 0.01,
                        command_unit: "m/s".to_owned(),
                    },
                ],
                sensor_ids: vec!["camera-0".to_owned()],
            }],
        }
    }

    #[test]
    fn sha256_primitive_matches_fips_abc_vector() {
        let digest = Sha256::digest(b"abc");
        assert_eq!(
            digest.iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn manifest_content_digest_matches_independent_v1_golden_vector() {
        // Generated by an independent Python struct/hashlib encoder using the
        // specified v1 field order, big-endian lengths/numbers, exact f64 bits,
        // stable-ID collection sorting, and SHA-256.
        let digest = valid_manifest().content_digest_v1().unwrap();
        assert_eq!(
            digest.to_hex(),
            "136ddd6d2d42cd9d496c53c1141f5bafc576b6a8c1e5517dea80770171fe8f26"
        );
    }

    #[test]
    fn manifest_content_digest_is_repeatable_and_domain_separated() {
        let manifest = valid_manifest();
        assert_eq!(
            manifest.content_digest_v1().unwrap(),
            manifest.content_digest_v1().unwrap()
        );
        let bytes = manifest.canonical_bytes_v1().unwrap();
        assert!(bytes.starts_with(WORLD_MANIFEST_COMMITMENT_DOMAIN_V1));
    }

    #[test]
    fn manifest_table_permutation_does_not_change_content_identity() {
        let manifest = valid_manifest();
        let baseline = manifest.content_digest_v1().unwrap();

        let mut reordered = manifest.clone();
        reordered.frames.reverse();
        reordered.world_assets.reverse();
        reordered.solvers.reverse();
        reordered.sensors.reverse();
        reordered.platforms.reverse();
        reordered.platforms[0].actuators.reverse();
        reordered.platforms[0].sensor_ids.reverse();

        assert_eq!(baseline, reordered.content_digest_v1().unwrap());
    }

    #[test]
    fn physics_semantic_change_changes_manifest_digest() {
        let manifest = valid_manifest();
        let digest = manifest.content_digest_v1().unwrap();

        let mut changed_gravity = manifest.clone();
        changed_gravity.gravity_mps2[0] = 0.25;
        assert_ne!(digest, changed_gravity.content_digest_v1().unwrap());

        let mut changed_mass = manifest;
        changed_mass.platforms[0].mass_kg += 1.0;
        assert_ne!(digest, changed_mass.content_digest_v1().unwrap());
    }

    #[test]
    fn sha256_hex_case_is_normalized_inside_manifest_bytes() {
        let manifest = valid_manifest();
        let digest = manifest.content_digest_v1().unwrap();

        let mut uppercase = manifest;
        uppercase.world_assets[0].sha256 = uppercase.world_assets[0].sha256.to_uppercase();
        assert_eq!(digest, uppercase.content_digest_v1().unwrap());
    }

    #[test]
    fn signed_zero_has_explicit_bitwise_identity_in_v1() {
        let mut positive_zero = valid_manifest();
        positive_zero.gravity_mps2[0] = 0.0;
        let positive_digest = positive_zero.content_digest_v1().unwrap();

        let mut negative_zero = positive_zero;
        negative_zero.gravity_mps2[0] = -0.0;
        assert_ne!(positive_digest, negative_zero.content_digest_v1().unwrap());
    }

    #[test]
    fn invalid_manifest_cannot_be_committed() {
        let mut manifest = valid_manifest();
        manifest.fixed_step_ns = 0;
        assert!(matches!(
            manifest.content_digest_v1(),
            Err(WorldManifestCommitmentError::InvalidManifest(_))
        ));
    }
}
