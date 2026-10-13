// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Explicit coordinate-axis conventions and qualified-world admission.
//!
//! The legacy WorldManifestV1 commitment is not modified by this module.
//! QualifiedWorldManifestV2 composes the v1 world commitment with a canonical,
//! complete frame-convention registry so solver admission cannot ignore axes.

use crate::world_contract::{
    SolverRequestIdentityV1, SolverResultEnvelopeV1, WorldManifestV1, WorldValidationIssue,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;

pub const FRAME_CONVENTION_REGISTRY_SCHEMA_VERSION_V1: u16 = 1;
pub const FRAME_CONVENTION_FORMAT_VERSION_V1: u16 = 1;
pub const QUALIFIED_WORLD_SCHEMA_VERSION_V2: u16 = 2;
pub const QUALIFIED_WORLD_COMMITMENT_FORMAT_VERSION_V2: u16 = 2;
pub const MAX_FRAME_CONVENTION_BYTES: usize = 4 * 1024 * 1024;
pub const FRAME_CONVENTION_DOMAIN_V1: &[u8] = b"symtropy.robotics.frame-conventions.v1\0";
pub const QUALIFIED_WORLD_DOMAIN_V2: &[u8] = b"symtropy.robotics.qualified-world.v2\0";

/// A named, explicit meaning for the positive axes of a coordinate frame.
///
/// Body FLU, ENU, NED, and camera optical variants follow ROS REP-103; FRD
/// follows MAVLink's frame definitions. ExternalSpec pins a custom convention by
/// both URI and digest; the URI alone is not immutable specification identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameAxisConventionV1 {
    /// +X forward, +Y left, +Z up.
    BodyForwardLeftUp,
    /// +X east, +Y north, +Z up.
    EnuEastNorthUp,
    /// +X north, +Y east, +Z down.
    NedNorthEastDown,
    /// +X right, +Y down, +Z forward.
    CameraOpticalRightDownForward,
    /// +X forward, +Y right, +Z down.
    FrdForwardRightDown,
    /// Convention defined by an external, content-pinned specification.
    ExternalSpec {
        handedness: FrameHandednessV1,
        specification_uri: String,
        specification_sha256: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameHandednessV1 {
    RightHanded,
    LeftHanded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameConventionBindingV1 {
    pub frame_id: String,
    pub convention: FrameAxisConventionV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameConventionRegistryV1 {
    pub schema_version: u16,
    pub frames: Vec<FrameConventionBindingV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameConventionIssueCode {
    UnsupportedSchemaVersion,
    EmptyFrameIdentity,
    DuplicateFrameIdentity,
    UnknownFrameReference,
    MissingFrameConvention,
    InvalidExternalSpecification,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameConventionIssue {
    pub code: FrameConventionIssueCode,
    pub path: String,
}

impl FrameConventionIssue {
    fn new(code: FrameConventionIssueCode, path: impl Into<String>) -> Self {
        Self { code, path: path.into() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameConventionDigestV1([u8; 32]);

impl FrameConventionDigestV1 {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(self) -> String {
        let mut out = String::with_capacity(64);
        for byte in self.0 {
            use fmt::Write as _;
            let _ = write!(out, "{byte:02x}");
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QualifiedWorldDigestV2([u8; 32]);

impl QualifiedWorldDigestV2 {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

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
pub enum QualifiedWorldError {
    InvalidWorldManifest(Vec<WorldValidationIssue>),
    InvalidSolverRequestIdentity(Vec<WorldValidationIssue>),
    InvalidFrameConventionRegistry(Vec<FrameConventionIssue>),
    FrameConventionBytesTooLarge { limit_bytes: usize },
    WorldManifestCommitmentTooLarge { limit_bytes: usize },
    UnsupportedQualifiedWorldSchemaVersion,
    QualifiedWorldDigestMismatch,
    RequestQualifiedWorldDigestMismatch,
    ResultQualifiedWorldDigestMismatch,
    InnerSolverResultRejected(Vec<WorldValidationIssue>),
}

impl fmt::Display for QualifiedWorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorldManifest(_) => f.write_str("world manifest is invalid"),
            Self::InvalidSolverRequestIdentity(_) => {
                f.write_str("solver request identity is invalid")
            }
            Self::InvalidFrameConventionRegistry(_) => {
                f.write_str("frame convention registry is invalid")
            }
            Self::FrameConventionBytesTooLarge { limit_bytes } => write!(
                f,
                "frame convention registry exceeds {limit_bytes} canonical bytes"
            ),
            Self::WorldManifestCommitmentTooLarge { limit_bytes } => write!(
                f,
                "base world manifest exceeds {limit_bytes} canonical bytes"
            ),
            Self::UnsupportedQualifiedWorldSchemaVersion => {
                f.write_str("unsupported qualified-world schema version")
            }
            Self::QualifiedWorldDigestMismatch => {
                f.write_str("qualified world commitment does not match its components")
            }
            Self::RequestQualifiedWorldDigestMismatch => {
                f.write_str("solver request is bound to a different qualified world")
            }
            Self::ResultQualifiedWorldDigestMismatch => {
                f.write_str("solver result claims a different qualified world")
            }
            Self::InnerSolverResultRejected(_) => {
                f.write_str("base world solver-result admission rejected the result")
            }
        }
    }
}

impl std::error::Error for QualifiedWorldError {}

impl FrameConventionRegistryV1 {
    /// Validate basic registry structure independently of any world manifest.
    pub fn validate(&self) -> Result<(), Vec<FrameConventionIssue>> {
        let mut issues = Vec::new();
        if self.schema_version != FRAME_CONVENTION_REGISTRY_SCHEMA_VERSION_V1 {
            issues.push(FrameConventionIssue::new(
                FrameConventionIssueCode::UnsupportedSchemaVersion,
                "schema_version",
            ));
        }

        let mut seen = BTreeSet::new();
        for (index, binding) in self.frames.iter().enumerate() {
            let path = format!("frames[{index}]");
            if binding.frame_id.trim().is_empty() {
                issues.push(FrameConventionIssue::new(
                    FrameConventionIssueCode::EmptyFrameIdentity,
                    format!("{path}.frame_id"),
                ));
            }
            if !seen.insert(binding.frame_id.as_str()) {
                issues.push(FrameConventionIssue::new(
                    FrameConventionIssueCode::DuplicateFrameIdentity,
                    format!("{path}.frame_id"),
                ));
            }
            if let FrameAxisConventionV1::ExternalSpec {
                handedness,
                specification_uri,
                specification_sha256,
            } = &binding.convention
            {
                if *handedness != FrameHandednessV1::RightHanded
                    || specification_uri.trim().is_empty()
                    || !is_sha256_hex(specification_sha256)
                {
                    issues.push(FrameConventionIssue::new(
                        FrameConventionIssueCode::InvalidExternalSpecification,
                        format!("{path}.convention"),
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

    /// Require exactly one convention binding for every declared world frame.
    pub fn validate_for_world(
        &self,
        world: &WorldManifestV1,
    ) -> Result<(), Vec<FrameConventionIssue>> {
        let mut issues = self.validate().err().unwrap_or_default();
        let world_ids: BTreeSet<&str> =
            world.frames.iter().map(|frame| frame.frame_id.as_str()).collect();
        let mut binding_ids = BTreeSet::new();

        for (index, binding) in self.frames.iter().enumerate() {
            if world_ids.contains(binding.frame_id.as_str()) {
                binding_ids.insert(binding.frame_id.as_str());
            } else {
                issues.push(FrameConventionIssue::new(
                    FrameConventionIssueCode::UnknownFrameReference,
                    format!("frames[{index}].frame_id"),
                ));
            }
        }

        for (index, frame) in world.frames.iter().enumerate() {
            if !binding_ids.contains(frame.frame_id.as_str()) {
                issues.push(FrameConventionIssue::new(
                    FrameConventionIssueCode::MissingFrameConvention,
                    format!("world.frames[{index}].frame_id"),
                ));
            }
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }

    /// Canonical registry bytes, independent of JSON/Serde and input table order.
    pub fn canonical_bytes_v1(&self) -> Result<Vec<u8>, QualifiedWorldError> {
        self.validate().map_err(QualifiedWorldError::InvalidFrameConventionRegistry)?;
        let mut encoder = RegistryEncoder::new();
        encoder.raw(FRAME_CONVENTION_DOMAIN_V1);
        encoder.u16(FRAME_CONVENTION_FORMAT_VERSION_V1);
        encoder.u16(self.schema_version);
        let mut frames: Vec<&FrameConventionBindingV1> = self.frames.iter().collect();
        frames.sort_by(|a, b| a.frame_id.cmp(&b.frame_id));
        encoder.count(frames.len());
        for binding in frames {
            encoder.string(&binding.frame_id);
            encode_convention(&mut encoder, &binding.convention);
        }
        encoder.finish()
    }

    pub fn content_digest_v1(&self) -> Result<FrameConventionDigestV1, QualifiedWorldError> {
        let bytes = self.canonical_bytes_v1()?;
        let digest = Sha256::digest(bytes);
        let mut fixed = [0u8; 32];
        fixed.copy_from_slice(&digest);
        Ok(FrameConventionDigestV1(fixed))
    }
}

/// Schema-v2 qualified world. The legacy v1 world model and its frozen
/// commitment remain unchanged; this wrapper requires a convention for every
/// frame and binds both component digests into one solver-admission identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualifiedWorldManifestV2 {
    pub schema_version: u16,
    pub world: WorldManifestV1,
    pub frame_conventions: FrameConventionRegistryV1,
}

impl QualifiedWorldManifestV2 {
    pub fn validate(&self) -> Result<(), QualifiedWorldError> {
        if self.schema_version != QUALIFIED_WORLD_SCHEMA_VERSION_V2 {
            return Err(QualifiedWorldError::UnsupportedQualifiedWorldSchemaVersion);
        }
        self.world
            .validate()
            .map_err(QualifiedWorldError::InvalidWorldManifest)?;
        self.frame_conventions
            .validate_for_world(&self.world)
            .map_err(QualifiedWorldError::InvalidFrameConventionRegistry)?;
        Ok(())
    }

    /// V2 commits to the v1 world digest and complete canonical frame registry.
    /// Fixed-size component digests make the envelope bounded regardless of the
    /// canonical registry's size; both component encoders validate before use.
    pub fn canonical_bytes_v2(&self) -> Result<Vec<u8>, QualifiedWorldError> {
        self.validate()?;
        let world_digest = self.world
            .content_digest_v1()
            .map_err(|error| match error {
                crate::world_manifest_commitment::WorldManifestCommitmentError::InvalidManifest(
                    issues,
                ) => QualifiedWorldError::InvalidWorldManifest(issues),
                crate::world_manifest_commitment::WorldManifestCommitmentError::CanonicalBytesTooLarge {
                    limit_bytes,
                } => QualifiedWorldError::WorldManifestCommitmentTooLarge { limit_bytes },
            })?;
        let convention_digest = self.frame_conventions.content_digest_v1()?;
        let mut bytes = Vec::with_capacity(
            QUALIFIED_WORLD_DOMAIN_V2.len() + 2 + world_digest.as_bytes().len()
                + convention_digest.as_bytes().len(),
        );
        bytes.extend_from_slice(QUALIFIED_WORLD_DOMAIN_V2);
        bytes.extend_from_slice(&QUALIFIED_WORLD_COMMITMENT_FORMAT_VERSION_V2.to_be_bytes());
        bytes.extend_from_slice(world_digest.as_bytes());
        bytes.extend_from_slice(convention_digest.as_bytes());
        Ok(bytes)
    }

    pub fn content_digest_v2(&self) -> Result<QualifiedWorldDigestV2, QualifiedWorldError> {
        let bytes = self.canonical_bytes_v2()?;
        let digest = Sha256::digest(bytes);
        let mut fixed = [0u8; 32];
        fixed.copy_from_slice(&digest);
        Ok(QualifiedWorldDigestV2(fixed))
    }

    /// Fail-closed admission for results whose frame-axis meanings matter.
    /// Callers use this API instead of base-manifest-only admission whenever
    /// a solver consumes or emits frame-dependent quantities.
    pub fn validate_solver_result(
        &self,
        expected_request: &QualifiedSolverRequestV2,
        result: &QualifiedSolverResultEnvelopeV2,
    ) -> Result<(), QualifiedWorldError> {
        self.validate()?;
        let actual_digest = self.content_digest_v2()?;
        if expected_request.qualified_world_digest != actual_digest {
            return Err(QualifiedWorldError::RequestQualifiedWorldDigestMismatch);
        }
        if result.qualified_world_digest != actual_digest {
            return Err(QualifiedWorldError::ResultQualifiedWorldDigestMismatch);
        }
        self.world
            .validate_solver_result_against_world(&expected_request.identity, &result.envelope)
            .map_err(QualifiedWorldError::InnerSolverResultRejected)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualifiedSolverRequestV2 {
    pub identity: SolverRequestIdentityV1,
    pub qualified_world_digest: QualifiedWorldDigestV2,
}

impl QualifiedSolverRequestV2 {
    pub fn bind(
        identity: SolverRequestIdentityV1,
        world: &QualifiedWorldManifestV2,
    ) -> Result<Self, QualifiedWorldError> {
        world.validate()?;
        identity
            .validate()
            .map_err(QualifiedWorldError::InvalidSolverRequestIdentity)?;
        let expected_inner_digest = world
            .world
            .content_digest_v1()
            .map_err(|error| match error {
                crate::world_manifest_commitment::WorldManifestCommitmentError::InvalidManifest(
                    issues,
                ) => QualifiedWorldError::InvalidWorldManifest(issues),
                crate::world_manifest_commitment::WorldManifestCommitmentError::CanonicalBytesTooLarge {
                    limit_bytes,
                } => QualifiedWorldError::WorldManifestCommitmentTooLarge { limit_bytes },
            })?;
        if identity.world_manifest_digest != expected_inner_digest {
            return Err(QualifiedWorldError::QualifiedWorldDigestMismatch);
        }
        Ok(Self {
            identity,
            qualified_world_digest: world.content_digest_v2()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualifiedSolverResultEnvelopeV2 {
    pub envelope: SolverResultEnvelopeV1,
    pub qualified_world_digest: QualifiedWorldDigestV2,
}

impl QualifiedSolverResultEnvelopeV2 {
    pub fn bind(envelope: SolverResultEnvelopeV1, request: &QualifiedSolverRequestV2) -> Self {
        Self {
            envelope,
            qualified_world_digest: request.qualified_world_digest,
        }
    }
}

fn encode_convention(encoder: &mut RegistryEncoder, convention: &FrameAxisConventionV1) {
    match convention {
        FrameAxisConventionV1::BodyForwardLeftUp => encoder.u8(0),
        FrameAxisConventionV1::EnuEastNorthUp => encoder.u8(1),
        FrameAxisConventionV1::NedNorthEastDown => encoder.u8(2),
        FrameAxisConventionV1::CameraOpticalRightDownForward => encoder.u8(3),
        FrameAxisConventionV1::FrdForwardRightDown => encoder.u8(4),
        FrameAxisConventionV1::ExternalSpec {
            handedness,
            specification_uri,
            specification_sha256,
        } => {
            encoder.u8(255);
            encoder.u8(match handedness {
                FrameHandednessV1::RightHanded => 0,
                FrameHandednessV1::LeftHanded => 1,
            });
            encoder.string(specification_uri);
            encoder.raw(&decode_sha256_hex(specification_sha256));
        }
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.as_bytes().iter().all(u8::is_ascii_hexdigit)
}

fn decode_sha256_hex(value: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        let source = value.as_bytes();
        let high = hex_nibble(source[index * 2]).expect("SHA-256 hex was validated");
        let low = hex_nibble(source[index * 2 + 1]).expect("SHA-256 hex was validated");
        *byte = (high << 4) | low;
    }
    out
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

struct RegistryEncoder {
    bytes: Vec<u8>,
    too_large: bool,
}

impl RegistryEncoder {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            too_large: false,
        }
    }

    fn raw(&mut self, data: &[u8]) {
        if self.too_large {
            return;
        }
        let Some(next) = self.bytes.len().checked_add(data.len()) else {
            self.too_large = true;
            return;
        };
        if next > MAX_FRAME_CONVENTION_BYTES {
            self.too_large = true;
            return;
        }
        self.bytes.extend_from_slice(data);
    }

    fn u8(&mut self, value: u8) {
        self.raw(&[value]);
    }

    fn u16(&mut self, value: u16) {
        self.raw(&value.to_be_bytes());
    }

    fn count(&mut self, count: usize) {
        match u64::try_from(count) {
            Ok(value) => self.raw(&value.to_be_bytes()),
            Err(_) => self.too_large = true,
        }
    }

    fn string(&mut self, value: &str) {
        self.count(value.len());
        self.raw(value.as_bytes());
    }

    fn finish(self) -> Result<Vec<u8>, QualifiedWorldError> {
        if self.too_large {
            Err(QualifiedWorldError::FrameConventionBytesTooLarge {
                limit_bytes: MAX_FRAME_CONVENTION_BYTES,
            })
        } else {
            Ok(self.bytes)
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::world_contract::{
        ActuatorProfileV1, ArtifactRef, CoordinateFrameV1, FrameTransformV1, PhysicsDomain,
        PhysicsFidelity, PlatformProfileV1, SensorModality, SensorProfileV1, SolverDeclarationV1,
        WORLD_MANIFEST_SCHEMA_VERSION, SOLVER_RESULT_SCHEMA_VERSION,
    };
    use crate::platform::PlatformType;

    fn artifact(id: &str) -> ArtifactRef {
        ArtifactRef {
            artifact_id: id.to_owned(),
            uri: format!("urn:symtropy:test:{id}"),
            sha256: "ab".repeat(32),
        }
    }

    fn sample_world() -> WorldManifestV1 {
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

    fn registry_for(world: &WorldManifestV1) -> FrameConventionRegistryV1 {
        let frames = world.frames.iter().map(|frame| FrameConventionBindingV1 {
            frame_id: frame.frame_id.clone(),
            convention: match frame.frame_id.as_str() {
                "world" => FrameAxisConventionV1::EnuEastNorthUp,
                "robot-base" => FrameAxisConventionV1::BodyForwardLeftUp,
                "camera-optical" => FrameAxisConventionV1::CameraOpticalRightDownForward,
                _ => FrameAxisConventionV1::FrdForwardRightDown,
            },
        }).collect();
        FrameConventionRegistryV1 {
            schema_version: FRAME_CONVENTION_REGISTRY_SCHEMA_VERSION_V1,
            frames,
        }
    }

    fn qualified_world() -> QualifiedWorldManifestV2 {
        let world = sample_world();
        QualifiedWorldManifestV2 {
            schema_version: QUALIFIED_WORLD_SCHEMA_VERSION_V2,
            frame_conventions: registry_for(&world),
            world,
        }
    }

    fn solver_identity(world: &WorldManifestV1) -> SolverRequestIdentityV1 {
        SolverRequestIdentityV1 {
            run_id: "run-conventions-1".to_owned(),
            request_id: 9,
            scenario_id: world.scenario_id.clone(),
            world_generation: world.world_generation,
            world_manifest_digest: world.content_digest_v1().unwrap(),
            solver: world.solvers[0].clone(),
            input_digest_sha256: "ac".repeat(32),
            simulation_time_start_ns: 0,
            simulation_time_end_ns: 1_000_000,
        }
    }

    fn solver_result(identity: &SolverRequestIdentityV1) -> SolverResultEnvelopeV1 {
        SolverResultEnvelopeV1 {
            schema_version: SOLVER_RESULT_SCHEMA_VERSION,
            request: identity.clone(),
            disposition: crate::world_contract::SolverResultDisposition::NumericalNonConvergence,
            output_digest_sha256: None,
            outputs: Vec::new(),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn frame_convention_registry_matches_independent_golden_vector() {
        let registry = registry_for(&sample_world());
        let bytes = registry.canonical_bytes_v1().unwrap();
        assert_eq!(bytes.len(), 107);
        assert_eq!(
            registry.content_digest_v1().unwrap().to_hex(),
            "2ff85b064397f5e636b8b7510c7f7322581300d9d0d35c8a348dce6d50c7c0d2"
        );
    }

    #[test]
    fn qualified_world_matches_independent_golden_vector() {
        assert_eq!(
            qualified_world().content_digest_v2().unwrap().to_hex(),
            "962c4a535296a60b34905d73393e820c193d1eac7c87b1f53d2702b2e1f9f38b"
        );
    }

    #[test]
    fn registry_digest_is_independent_of_entry_order() {
        let world = sample_world();
        let registry = registry_for(&world);
        let digest = registry.content_digest_v1().unwrap();
        let mut reordered = registry;
        reordered.frames.reverse();
        assert_eq!(digest, reordered.content_digest_v1().unwrap());
    }

    #[test]
    fn every_world_frame_requires_exactly_one_convention() {
        let world = sample_world();
        let mut missing = registry_for(&world);
        missing.frames.pop();
        assert!(matches!(
            missing.validate_for_world(&world),
            Err(issues) if issues.iter().any(|issue|
                issue.code == FrameConventionIssueCode::MissingFrameConvention)
        ));

        let mut unknown = registry_for(&world);
        unknown.frames.push(FrameConventionBindingV1 {
            frame_id: "ghost-frame".to_owned(),
            convention: FrameAxisConventionV1::EnuEastNorthUp,
        });
        assert!(matches!(
            unknown.validate_for_world(&world),
            Err(issues) if issues.iter().any(|issue|
                issue.code == FrameConventionIssueCode::UnknownFrameReference)
        ));
    }

    #[test]
    fn custom_convention_is_pinned_and_hex_case_normalized() {
        let registry = FrameConventionRegistryV1 {
            schema_version: FRAME_CONVENTION_REGISTRY_SCHEMA_VERSION_V1,
            frames: vec![FrameConventionBindingV1 {
                frame_id: "custom".to_owned(),
                convention: FrameAxisConventionV1::ExternalSpec {
                    handedness: FrameHandednessV1::RightHanded,
                    specification_uri: "urn:example:coordinate-spec:1".to_owned(),
                    specification_sha256: "ab".repeat(32),
                },
            }],
        };
        let digest = registry.content_digest_v1().unwrap();
        let mut upper = registry;
        let FrameAxisConventionV1::ExternalSpec { specification_sha256, .. } =
            &mut upper.frames[0].convention else { unreachable!() };
        *specification_sha256 = specification_sha256.to_uppercase();
        assert_eq!(digest, upper.content_digest_v1().unwrap());
    }

    #[test]
    fn left_handed_external_spec_is_rejected_before_commitment() {
        let registry = FrameConventionRegistryV1 {
            schema_version: FRAME_CONVENTION_REGISTRY_SCHEMA_VERSION_V1,
            frames: vec![FrameConventionBindingV1 {
                frame_id: "custom".to_owned(),
                convention: FrameAxisConventionV1::ExternalSpec {
                    handedness: FrameHandednessV1::LeftHanded,
                    specification_uri: "urn:example:left-handed:1".to_owned(),
                    specification_sha256: "ab".repeat(32),
                },
            }],
        };
        assert!(matches!(
            registry.validate(),
            Err(issues) if issues.iter().any(|issue|
                issue.code == FrameConventionIssueCode::InvalidExternalSpecification)
        ));
    }

    #[test]
    fn request_binding_rejects_invalid_inner_identity() {
        let world = qualified_world();
        let mut identity = solver_identity(&world.world);
        identity.input_digest_sha256 = "not-a-sha256".to_owned();
        assert!(matches!(
            QualifiedSolverRequestV2::bind(identity, &world),
            Err(QualifiedWorldError::InvalidSolverRequestIdentity(_))
        ));
    }

    #[test]
    fn qualified_world_digest_changes_when_a_frame_basis_changes() {
        let original = qualified_world();
        let digest = original.content_digest_v2().unwrap();
        let mut altered = original;
        altered.frame_conventions.frames[1].convention =
            FrameAxisConventionV1::FrdForwardRightDown;
        assert_ne!(digest, altered.content_digest_v2().unwrap());
    }

    #[test]
    fn qualified_admission_rejects_a_request_after_frame_basis_changes() {
        let world = qualified_world();
        let identity = solver_identity(&world.world);
        let request = QualifiedSolverRequestV2::bind(identity.clone(), &world).unwrap();
        let result = QualifiedSolverResultEnvelopeV2::bind(solver_result(&identity), &request);
        assert!(world.validate_solver_result(&request, &result).is_ok());

        let mut changed_world = world;
        changed_world.frame_conventions.frames[1].convention =
            FrameAxisConventionV1::FrdForwardRightDown;
        assert!(matches!(
            changed_world.validate_solver_result(&request, &result),
            Err(QualifiedWorldError::RequestQualifiedWorldDigestMismatch)
        ));
    }

    #[test]
    fn qualified_admission_rejects_a_result_with_wrong_world_digest() {
        let world = qualified_world();
        let identity = solver_identity(&world.world);
        let request = QualifiedSolverRequestV2::bind(identity.clone(), &world).unwrap();
        let mut result = QualifiedSolverResultEnvelopeV2::bind(solver_result(&identity), &request);
        result.qualified_world_digest = QualifiedWorldDigestV2::from_bytes([0u8; 32]);
        assert!(matches!(
            world.validate_solver_result(&request, &result),
            Err(QualifiedWorldError::ResultQualifiedWorldDigestMismatch)
        ));
    }
}
