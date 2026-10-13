// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Provenance-bound survey evidence contracts.
//!
//! This module separates observation, registration, control adjustment,
//! qualification, and candidate world changes. It validates evidence identity
//! and declared uncertainty; it does not implement a geodesy engine, SLAM,
//! bundle adjustment, point-cloud registration, or a physical terrain commit.
//! A qualified result is only as credible as its declared inputs, calibration,
//! independent checkpoints, and the external validation of its algorithms.

use crate::world_contract::ArtifactRef;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const SURVEY_EVIDENCE_SCHEMA_VERSION: u16 = 1;
/// JSON evidence envelopes are intentionally bounded before parsing.
pub const MAX_SURVEY_EVIDENCE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_SURVEY_FRAMES: usize = 4_096;
pub const MAX_SURVEY_SESSIONS: usize = 65_536;
pub const MAX_SURVEY_OBSERVATIONS: usize = 1_000_000;
pub const MAX_SURVEY_TRANSFORMS: usize = 65_536;
pub const MAX_SURVEY_RECEIPTS: usize = 65_536;
pub const MAX_SURVEY_ARTIFACT_REFS: usize = 4_096;
const SHA256_HEX_LENGTH: usize = 64;
const MAX_ID_LENGTH: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurveyContractErrorV1 {
    UnsupportedSchemaVersion,
    EmptyIdentity(&'static str),
    InvalidArtifact(&'static str),
    InvalidDigest(&'static str),
    PayloadTooLarge,
    PayloadDigestMismatch,
    InvalidPayload,
    TooManyItems(&'static str),
    DuplicateIdentity(&'static str),
    InvalidCoordinateFrame(&'static str),
    UnknownCoordinateFrame(String),
    UnknownSurveySession(String),
    SessionTimeMismatch,
    InvalidObservation(&'static str),
    InvalidUncertainty,
    InvalidTransform(&'static str),
    InvalidProcessingReceipt(&'static str),
    CheckpointReusedForFit(String),
    UnknownObservation(String),
    StaleWorldGeneration,
    BaseGeometryMismatch,
    CoordinateFrameMismatch,
    AdmissionPolicyMismatch,
    QualificationReceiptMismatch,
    InsufficientIndependentCheckpoints,
    AccuracyThresholdExceeded,
    VolumeUncertaintyExceeded,
    ChangeBelowDetectionLimit,
}

impl fmt::Display for SurveyContractErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for SurveyContractErrorV1 {}

fn identity_is_valid(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_LENGTH
}

fn digest_is_valid(value: &str) -> bool {
    value.len() == SHA256_HEX_LENGTH && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn validate_artifact(
    artifact: &ArtifactRef,
    path: &'static str,
) -> Result<(), SurveyContractErrorV1> {
    if !identity_is_valid(&artifact.artifact_id) || !identity_is_valid(&artifact.uri) {
        return Err(SurveyContractErrorV1::InvalidArtifact(path));
    }
    if !digest_is_valid(&artifact.sha256) {
        return Err(SurveyContractErrorV1::InvalidDigest(path));
    }
    Ok(())
}

fn verify_content_addressed<T: Serialize>(
    value: &T,
    artifact: &ArtifactRef,
    path: &'static str,
) -> Result<(), SurveyContractErrorV1> {
    validate_artifact(artifact, path)?;
    let bytes = serde_json::to_vec(value).map_err(|_| SurveyContractErrorV1::InvalidPayload)?;
    if bytes.len() > MAX_SURVEY_EVIDENCE_BYTES {
        return Err(SurveyContractErrorV1::PayloadTooLarge);
    }
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if !actual.eq_ignore_ascii_case(&artifact.sha256) {
        return Err(SurveyContractErrorV1::PayloadDigestMismatch);
    }
    Ok(())
}

fn ensure_unique<'a>(
    values: impl IntoIterator<Item = &'a str>,
    field: &'static str,
) -> Result<(), SurveyContractErrorV1> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !identity_is_valid(value) {
            return Err(SurveyContractErrorV1::EmptyIdentity(field));
        }
        if !seen.insert(value) {
            return Err(SurveyContractErrorV1::DuplicateIdentity(field));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurveyAxisDirectionV1 {
    East,
    West,
    North,
    South,
    Up,
    Down,
    Longitude,
    Latitude,
    GeocentricX,
    GeocentricY,
    GeocentricZ,
    LocalX,
    LocalY,
    LocalZ,
}

impl SurveyAxisDirectionV1 {
    fn family(self) -> u8 {
        match self {
            Self::East | Self::West | Self::Longitude => 0,
            Self::North | Self::South | Self::Latitude => 1,
            Self::Up | Self::Down => 2,
            Self::GeocentricX => 3,
            Self::GeocentricY => 4,
            Self::GeocentricZ => 5,
            Self::LocalX => 6,
            Self::LocalY => 7,
            Self::LocalZ => 8,
        }
    }

    fn is_angular(self) -> bool {
        matches!(self, Self::Longitude | Self::Latitude)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurveyCoordinateUnitV1 {
    Metre,
    InternationalFoot,
    UsSurveyFoot,
    Degree,
    Radian,
}

impl SurveyCoordinateUnitV1 {
    fn is_angular(self) -> bool {
        matches!(self, Self::Degree | Self::Radian)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurveyHandednessV1 {
    RightHanded,
    LeftHanded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurveyHeightTypeV1 {
    Ellipsoidal,
    Orthometric,
    Normal,
    DepthBelowSurface,
    LocalVertical,
}

/// An explicitly described survey frame. Identifiers are preserved verbatim;
/// supporting a CRS identifier does not itself implement or certify its
/// transformation operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurveyCoordinateFrameV1 {
    pub frame_id: String,
    /// Qualified CRS identifier or a project-scoped local-frame identifier.
    pub crs_id: String,
    pub horizontal_datum_id: String,
    pub vertical_reference_id: String,
    pub height_type: SurveyHeightTypeV1,
    /// Axis semantics and units are explicit and ordered as stored coordinates.
    pub axis_directions: [SurveyAxisDirectionV1; 3],
    pub axis_units: [SurveyCoordinateUnitV1; 3],
    pub handedness: SurveyHandednessV1,
    /// Decimal year when the underlying reference frame is epoch-dependent.
    pub coordinate_epoch_year: Option<f64>,
}

impl SurveyCoordinateFrameV1 {
    pub fn validate(&self) -> Result<(), SurveyContractErrorV1> {
        for (value, name) in [
            (self.frame_id.as_str(), "frame_id"),
            (self.crs_id.as_str(), "crs_id"),
            (self.horizontal_datum_id.as_str(), "horizontal_datum_id"),
            (self.vertical_reference_id.as_str(), "vertical_reference_id"),
        ] {
            if !identity_is_valid(value) {
                return Err(SurveyContractErrorV1::InvalidCoordinateFrame(name));
            }
        }

        let families: BTreeSet<u8> = self
            .axis_directions
            .iter()
            .map(|direction| direction.family())
            .collect();
        if families.len() != 3 {
            return Err(SurveyContractErrorV1::InvalidCoordinateFrame(
                "axis_directions",
            ));
        }

        for (direction, unit) in self.axis_directions.iter().zip(self.axis_units) {
            if direction.is_angular() != unit.is_angular() {
                return Err(SurveyContractErrorV1::InvalidCoordinateFrame(
                    "axis_units",
                ));
            }
        }

        if self
            .coordinate_epoch_year
            .is_some_and(|epoch| !epoch.is_finite())
        {
            return Err(SurveyContractErrorV1::InvalidCoordinateFrame(
                "coordinate_epoch_year",
            ));
        }
        Ok(())
    }
}

/// Three-axis uncertainty in the coordinate frame's declared axis units.
/// Covariance entries use the corresponding products of axis units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SurveyUncertaintyV1 {
    StandardDeviation {
        standard_deviation: [f64; 3],
    },
    Covariance {
        covariance: [[f64; 3]; 3],
    },
    AbsoluteBounds {
        absolute_bounds: [f64; 3],
    },
}

impl SurveyUncertaintyV1 {
    pub fn validate(&self) -> Result<(), SurveyContractErrorV1> {
        match self {
            Self::StandardDeviation {
                standard_deviation,
            } => {
                if !standard_deviation
                    .iter()
                    .all(|value| value.is_finite() && *value >= 0.0)
                {
                    return Err(SurveyContractErrorV1::InvalidUncertainty);
                }
            }
            Self::AbsoluteBounds { absolute_bounds } => {
                if !absolute_bounds
                    .iter()
                    .all(|value| value.is_finite() && *value >= 0.0)
                {
                    return Err(SurveyContractErrorV1::InvalidUncertainty);
                }
            }
            Self::Covariance { covariance } => {
                if !covariance_is_positive_semidefinite(covariance) {
                    return Err(SurveyContractErrorV1::InvalidUncertainty);
                }
            }
        }
        Ok(())
    }
}

/// PSD check for a symmetric 3x3 covariance matrix. Scaling before the
/// principal-minor checks avoids a tolerance tied to the absolute unit scale.
#[allow(clippy::needless_range_loop)] // Indexed matrix algebra compares symmetric coordinate pairs.
fn covariance_is_positive_semidefinite(matrix: &[[f64; 3]; 3]) -> bool {
    if !matrix.iter().flatten().all(|value| value.is_finite()) {
        return false;
    }

    let scale = matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);

    if scale == 0.0 {
        return true;
    }

    let symmetry_tolerance = scale * 1.0e-12;
    for row in 0..3 {
        for column in (row + 1)..3 {
            if (matrix[row][column] - matrix[column][row]).abs() > symmetry_tolerance {
                return false;
            }
        }
    }

    let mut a = [[0.0_f64; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            a[row][column] = matrix[row][column] / scale;
        }
    }

    const TOLERANCE: f64 = 1.0e-12;
    if a[0][0] < -TOLERANCE || a[1][1] < -TOLERANCE || a[2][2] < -TOLERANCE {
        return false;
    }

    let minor_01 = a[0][0] * a[1][1] - a[0][1] * a[0][1];
    let minor_02 = a[0][0] * a[2][2] - a[0][2] * a[0][2];
    let minor_12 = a[1][1] * a[2][2] - a[1][2] * a[1][2];
    if minor_01 < -TOLERANCE || minor_02 < -TOLERANCE || minor_12 < -TOLERANCE {
        return false;
    }

    let determinant = a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[1][2])
        - a[0][1] * (a[0][1] * a[2][2] - a[1][2] * a[0][2])
        + a[0][2] * (a[0][1] * a[1][2] - a[1][1] * a[0][2]);
    determinant >= -TOLERANCE
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurveySessionV1 {
    pub schema_version: u16,
    pub session_id: String,
    pub instrument_id: String,
    pub instrument_profile_artifact: ArtifactRef,
    pub calibration_artifact: ArtifactRef,
    pub instrument_frame_id: String,
    pub started_at_unix_ns: u64,
    pub ended_at_unix_ns: Option<u64>,
    /// Instrument/session clock uncertainty, independent of coordinate error.
    pub clock_uncertainty_ns: u64,
    /// Immutable references to original capture data, not transformed products.
    pub raw_observation_artifacts: Vec<ArtifactRef>,
}

impl SurveySessionV1 {
    pub fn validate(&self) -> Result<(), SurveyContractErrorV1> {
        if self.schema_version != SURVEY_EVIDENCE_SCHEMA_VERSION {
            return Err(SurveyContractErrorV1::UnsupportedSchemaVersion);
        }
        for (value, field) in [
            (self.session_id.as_str(), "session_id"),
            (self.instrument_id.as_str(), "instrument_id"),
            (self.instrument_frame_id.as_str(), "instrument_frame_id"),
        ] {
            if !identity_is_valid(value) {
                return Err(SurveyContractErrorV1::EmptyIdentity(field));
            }
        }
        if self
            .ended_at_unix_ns
            .is_some_and(|end| end < self.started_at_unix_ns)
        {
            return Err(SurveyContractErrorV1::SessionTimeMismatch);
        }
        if self.raw_observation_artifacts.is_empty() {
            return Err(SurveyContractErrorV1::InvalidObservation(
                "raw_observation_artifacts",
            ));
        }
        if self.raw_observation_artifacts.len() > MAX_SURVEY_ARTIFACT_REFS {
            return Err(SurveyContractErrorV1::TooManyItems(
                "raw_observation_artifacts",
            ));
        }
        validate_artifact(&self.instrument_profile_artifact, "instrument_profile_artifact")?;
        validate_artifact(&self.calibration_artifact, "calibration_artifact")?;
        for artifact in &self.raw_observation_artifacts {
            validate_artifact(artifact, "raw_observation_artifacts")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurveyModalityV1 {
    TotalStation,
    Levelling,
    Gnss,
    TerrestrialLidar,
    HandheldLidar,
    MobileLidar,
    Photogrammetry,
    Slam,
    Subsurface,
    ControlMark,
    OtherDeclared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurveyObservationRoleV1 {
    FitOrCalibration,
    IndependentCheckpoint,
    ContextOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurveyQualityFlagV1 {
    SuspectedOutlier,
    Occlusion,
    DustOrWater,
    PoorGeometry,
    ClockUncertain,
    CalibrationUncertain,
    SensorSaturation,
    IncompleteCoverage,
    DynamicScene,
    OtherDeclared,
}

/// A point measurement or an immutable reference to a larger observation
/// (such as a complete point cloud). An absent scalar uncertainty means
/// unknown here, not zero; qualification rules decide whether that is usable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurveyObservationV1 {
    pub schema_version: u16,
    pub observation_id: String,
    pub session_id: String,
    pub source_id: String,
    pub sequence: u64,
    pub captured_at_unix_ns: u64,
    pub modality: SurveyModalityV1,
    pub role: SurveyObservationRoleV1,
    pub frame_id: String,
    pub payload_artifact: ArtifactRef,
    pub calibration_artifact: ArtifactRef,
    /// Optional three-axis coordinate, expressed in the frame's declared units.
    pub coordinate: Option<[f64; 3]>,
    /// A cloud may instead provide its full spatial uncertainty product as an artifact.
    pub uncertainty: Option<SurveyUncertaintyV1>,
    pub uncertainty_artifact: Option<ArtifactRef>,
    pub quality_flags: Vec<SurveyQualityFlagV1>,
}

impl SurveyObservationV1 {
    pub fn validate(
        &self,
        session: &SurveySessionV1,
        frame: &SurveyCoordinateFrameV1,
    ) -> Result<(), SurveyContractErrorV1> {
        if self.schema_version != SURVEY_EVIDENCE_SCHEMA_VERSION {
            return Err(SurveyContractErrorV1::UnsupportedSchemaVersion);
        }
        for (value, field) in [
            (self.observation_id.as_str(), "observation_id"),
            (self.session_id.as_str(), "session_id"),
            (self.source_id.as_str(), "source_id"),
            (self.frame_id.as_str(), "frame_id"),
        ] {
            if !identity_is_valid(value) {
                return Err(SurveyContractErrorV1::EmptyIdentity(field));
            }
        }
        if self.session_id != session.session_id {
            return Err(SurveyContractErrorV1::UnknownSurveySession(
                self.session_id.clone(),
            ));
        }
        if self.frame_id != frame.frame_id {
            return Err(SurveyContractErrorV1::UnknownCoordinateFrame(
                self.frame_id.clone(),
            ));
        }
        if self.captured_at_unix_ns < session.started_at_unix_ns
            || session
                .ended_at_unix_ns
                .is_some_and(|end| self.captured_at_unix_ns > end)
        {
            return Err(SurveyContractErrorV1::SessionTimeMismatch);
        }
        if self
            .coordinate
            .is_some_and(|coordinate| !coordinate.iter().all(|value| value.is_finite()))
        {
            return Err(SurveyContractErrorV1::InvalidObservation("coordinate"));
        }
        if let Some(uncertainty) = &self.uncertainty {
            uncertainty.validate()?;
        }
        validate_artifact(&self.payload_artifact, "payload_artifact")?;
        validate_artifact(&self.calibration_artifact, "calibration_artifact")?;
        if let Some(artifact) = &self.uncertainty_artifact {
            validate_artifact(artifact, "uncertainty_artifact")?;
        }
        Ok(())
    }
}

/// Exact transform operation and grid/parameter resources are content-addressed.
/// The transform is declarative: this crate does not execute the operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurveyTransformV1 {
    pub schema_version: u16,
    pub transform_id: String,
    pub source_frame_id: String,
    pub target_frame_id: String,
    pub operation_artifact: ArtifactRef,
    pub resource_artifacts: Vec<ArtifactRef>,
    pub valid_from_epoch_year: Option<f64>,
    pub valid_to_epoch_year: Option<f64>,
    pub declared_horizontal_accuracy_95_m: Option<f64>,
    pub declared_vertical_accuracy_95_m: Option<f64>,
}

impl SurveyTransformV1 {
    pub fn validate(
        &self,
        frames: &[SurveyCoordinateFrameV1],
    ) -> Result<(), SurveyContractErrorV1> {
        if self.schema_version != SURVEY_EVIDENCE_SCHEMA_VERSION {
            return Err(SurveyContractErrorV1::UnsupportedSchemaVersion);
        }
        for (value, field) in [
            (self.transform_id.as_str(), "transform_id"),
            (self.source_frame_id.as_str(), "source_frame_id"),
            (self.target_frame_id.as_str(), "target_frame_id"),
        ] {
            if !identity_is_valid(value) {
                return Err(SurveyContractErrorV1::EmptyIdentity(field));
            }
        }
        if self.source_frame_id == self.target_frame_id {
            return Err(SurveyContractErrorV1::InvalidTransform("same_frame"));
        }
        for frame_id in [&self.source_frame_id, &self.target_frame_id] {
            if !frames.iter().any(|frame| &frame.frame_id == frame_id) {
                return Err(SurveyContractErrorV1::UnknownCoordinateFrame(
                    frame_id.clone(),
                ));
            }
        }
        if self.resource_artifacts.len() > MAX_SURVEY_ARTIFACT_REFS {
            return Err(SurveyContractErrorV1::TooManyItems("resource_artifacts"));
        }
        if self
            .valid_from_epoch_year
            .is_some_and(|epoch| !epoch.is_finite())
            || self
                .valid_to_epoch_year
                .is_some_and(|epoch| !epoch.is_finite())
            || matches!(
                (self.valid_from_epoch_year, self.valid_to_epoch_year),
                (Some(start), Some(end)) if start > end
            )
        {
            return Err(SurveyContractErrorV1::InvalidTransform("validity_epoch"));
        }
        for (value, field) in [
            (
                self.declared_horizontal_accuracy_95_m,
                "horizontal_accuracy",
            ),
            (self.declared_vertical_accuracy_95_m, "vertical_accuracy"),
        ] {
            if value.is_some_and(|accuracy| !accuracy.is_finite() || accuracy < 0.0) {
                return Err(SurveyContractErrorV1::InvalidTransform(field));
            }
        }
        validate_artifact(&self.operation_artifact, "operation_artifact")?;
        for artifact in &self.resource_artifacts {
            validate_artifact(artifact, "resource_artifacts")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurveyProcessingStageV1 {
    RawCapture,
    RegisteredSurvey,
    AdjustedControlSolution,
    QualifiedChangeEstimate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurveyProcessingDispositionV1 {
    Qualified,
    Provisional,
    Rejected,
    Failed,
    OutOfDomain,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurveyAccuracyMetricsV1 {
    pub horizontal_rmse_m: f64,
    pub vertical_rmse_m: f64,
    pub horizontal_accuracy_95_m: f64,
    pub vertical_accuracy_95_m: f64,
    /// Minimum reliably detectable change under the declared method and domain.
    pub detection_limit_m: Option<f64>,
    pub estimated_changed_volume_m3: Option<f64>,
    pub changed_volume_standard_uncertainty_m3: Option<f64>,
    pub uncertainty_method_id: String,
}

impl SurveyAccuracyMetricsV1 {
    pub fn validate(&self) -> Result<(), SurveyContractErrorV1> {
        for value in [
            self.horizontal_rmse_m,
            self.vertical_rmse_m,
            self.horizontal_accuracy_95_m,
            self.vertical_accuracy_95_m,
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(SurveyContractErrorV1::InvalidProcessingReceipt(
                    "accuracy_metrics",
                ));
            }
        }
        if self
            .detection_limit_m
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
        {
            return Err(SurveyContractErrorV1::InvalidProcessingReceipt(
                "detection_limit_m",
            ));
        }
        if self
            .estimated_changed_volume_m3
            .is_some_and(|value| !value.is_finite() || value < 0.0)
            || self
                .changed_volume_standard_uncertainty_m3
                .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            return Err(SurveyContractErrorV1::InvalidProcessingReceipt(
                "volume_metrics",
            ));
        }
        if !identity_is_valid(&self.uncertainty_method_id) {
            return Err(SurveyContractErrorV1::EmptyIdentity(
                "uncertainty_method_id",
            ));
        }
        Ok(())
    }
}

/// Reproducible record of a processing stage. Fitting observations and
/// independent checkpoints are distinct sets; this prevents using calibration
/// data as a claim of external accuracy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurveyProcessingReceiptV1 {
    pub schema_version: u16,
    pub receipt_id: String,
    pub stage: SurveyProcessingStageV1,
    pub disposition: SurveyProcessingDispositionV1,
    pub input_artifacts: Vec<ArtifactRef>,
    pub output_artifact: Option<ArtifactRef>,
    pub algorithm_id: String,
    pub algorithm_version: String,
    pub algorithm_build_sha256: String,
    pub configuration_sha256: String,
    pub source_frame_ids: Vec<String>,
    pub output_frame_id: Option<String>,
    pub fitting_observation_ids: Vec<String>,
    pub independent_checkpoint_ids: Vec<String>,
    pub excluded_observation_ids: Vec<String>,
    pub accuracy_metrics: Option<SurveyAccuracyMetricsV1>,
}

impl SurveyProcessingReceiptV1 {
    fn validate_basic(&self) -> Result<(), SurveyContractErrorV1> {
        if self.schema_version != SURVEY_EVIDENCE_SCHEMA_VERSION {
            return Err(SurveyContractErrorV1::UnsupportedSchemaVersion);
        }
        for (value, field) in [
            (self.receipt_id.as_str(), "receipt_id"),
            (self.algorithm_id.as_str(), "algorithm_id"),
            (self.algorithm_version.as_str(), "algorithm_version"),
        ] {
            if !identity_is_valid(value) {
                return Err(SurveyContractErrorV1::EmptyIdentity(field));
            }
        }
        if !digest_is_valid(&self.algorithm_build_sha256)
            || !digest_is_valid(&self.configuration_sha256)
        {
            return Err(SurveyContractErrorV1::InvalidDigest("processing_receipt"));
        }
        if self.input_artifacts.is_empty() {
            return Err(SurveyContractErrorV1::InvalidProcessingReceipt(
                "input_artifacts",
            ));
        }
        if self.input_artifacts.len() > MAX_SURVEY_ARTIFACT_REFS {
            return Err(SurveyContractErrorV1::TooManyItems("input_artifacts"));
        }
        if self.source_frame_ids.is_empty() {
            return Err(SurveyContractErrorV1::InvalidProcessingReceipt(
                "source_frame_ids",
            ));
        }
        ensure_unique(
            self.source_frame_ids.iter().map(String::as_str),
            "source_frame_ids",
        )?;
        for frame_id in &self.source_frame_ids {
            if !identity_is_valid(frame_id) {
                return Err(SurveyContractErrorV1::EmptyIdentity("source_frame_ids"));
            }
        }
        if let Some(frame_id) = &self.output_frame_id {
            if !identity_is_valid(frame_id) {
                return Err(SurveyContractErrorV1::EmptyIdentity("output_frame_id"));
            }
        }
        ensure_unique(
            self.fitting_observation_ids.iter().map(String::as_str),
            "fitting_observation_ids",
        )?;
        ensure_unique(
            self.independent_checkpoint_ids.iter().map(String::as_str),
            "independent_checkpoint_ids",
        )?;
        ensure_unique(
            self.excluded_observation_ids.iter().map(String::as_str),
            "excluded_observation_ids",
        )?;

        let fit: BTreeSet<&str> = self
            .fitting_observation_ids
            .iter()
            .map(String::as_str)
            .collect();
        let checkpoints: BTreeSet<&str> = self
            .independent_checkpoint_ids
            .iter()
            .map(String::as_str)
            .collect();
        let excluded: BTreeSet<&str> = self
            .excluded_observation_ids
            .iter()
            .map(String::as_str)
            .collect();
        if !fit.is_disjoint(&checkpoints) || !fit.is_disjoint(&excluded) {
            return Err(SurveyContractErrorV1::InvalidProcessingReceipt(
                "overlapping_observation_roles",
            ));
        }
        if !checkpoints.is_disjoint(&excluded) {
            return Err(SurveyContractErrorV1::InvalidProcessingReceipt(
                "checkpoint_excluded",
            ));
        }

        for artifact in &self.input_artifacts {
            validate_artifact(artifact, "input_artifacts")?;
        }
        if let Some(artifact) = &self.output_artifact {
            validate_artifact(artifact, "output_artifact")?;
        }
        if let Some(metrics) = &self.accuracy_metrics {
            metrics.validate()?;
        }

        if self.disposition == SurveyProcessingDispositionV1::Qualified {
            if self.output_artifact.is_none()
                || self.accuracy_metrics.is_none()
                || self.independent_checkpoint_ids.is_empty()
                || self.output_frame_id.is_none()
            {
                return Err(SurveyContractErrorV1::InvalidProcessingReceipt(
                    "qualified_without_evidence",
                ));
            }
            if self.stage == SurveyProcessingStageV1::QualifiedChangeEstimate {
                let metrics = self.accuracy_metrics.as_ref().expect("checked above");
                if metrics.detection_limit_m.is_none()
                    || metrics.estimated_changed_volume_m3.is_none()
                    || metrics.changed_volume_standard_uncertainty_m3.is_none()
                {
                    return Err(SurveyContractErrorV1::InvalidProcessingReceipt(
                        "qualified_change_without_change_metrics",
                    ));
                }
            }
        } else if self.stage == SurveyProcessingStageV1::QualifiedChangeEstimate {
            return Err(SurveyContractErrorV1::InvalidProcessingReceipt(
                "change_estimate_not_qualified",
            ));
        }
        Ok(())
    }

    pub fn verify_artifact(
        &self,
        artifact: &ArtifactRef,
    ) -> Result<(), SurveyContractErrorV1> {
        self.validate_basic()?;
        verify_content_addressed(self, artifact, "processing_receipt_artifact")
    }

    pub fn validate_against(
        &self,
        observations: &[SurveyObservationV1],
        frames: &[SurveyCoordinateFrameV1],
    ) -> Result<(), SurveyContractErrorV1> {
        self.validate_basic()?;
        ensure_unique(
            observations.iter().map(|observation| observation.observation_id.as_str()),
            "observation_id",
        )?;

        let observation_map: BTreeMap<&str, &SurveyObservationV1> = observations
            .iter()
            .map(|observation| (observation.observation_id.as_str(), observation))
            .collect();

        for frame_id in &self.source_frame_ids {
            if !frames.iter().any(|frame| &frame.frame_id == frame_id) {
                return Err(SurveyContractErrorV1::UnknownCoordinateFrame(
                    frame_id.clone(),
                ));
            }
        }
        if let Some(frame_id) = &self.output_frame_id {
            if !frames.iter().any(|frame| &frame.frame_id == frame_id) {
                return Err(SurveyContractErrorV1::UnknownCoordinateFrame(
                    frame_id.clone(),
                ));
            }
        }

        for (ids, expected_role, field) in [
            (
                self.fitting_observation_ids.as_slice(),
                Some(SurveyObservationRoleV1::FitOrCalibration),
                "fitting_observation_ids",
            ),
            (
                self.independent_checkpoint_ids.as_slice(),
                Some(SurveyObservationRoleV1::IndependentCheckpoint),
                "independent_checkpoint_ids",
            ),
            (
                self.excluded_observation_ids.as_slice(),
                None,
                "excluded_observation_ids",
            ),
        ] {
            for id in ids {
                let observation = observation_map
                    .get(id.as_str())
                    .ok_or_else(|| SurveyContractErrorV1::UnknownObservation(id.clone()))?;
                if expected_role.is_some_and(|role| observation.role != role) {
                    return Err(SurveyContractErrorV1::InvalidProcessingReceipt(field));
                }
                if expected_role.is_some()
                    && !self.source_frame_ids.contains(&observation.frame_id)
                {
                    return Err(SurveyContractErrorV1::InvalidProcessingReceipt(
                        "observation_frame_not_declared",
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Candidate only. Admitting it still requires a separate caller-controlled,
/// atomic world-generation check and state commit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurveyChangeCandidateV1 {
    pub schema_version: u16,
    pub candidate_id: String,
    pub base_world_generation: u64,
    pub base_geometry_artifact: ArtifactRef,
    pub earlier_survey_artifact: ArtifactRef,
    pub later_survey_artifact: ArtifactRef,
    pub change_estimate_artifact: ArtifactRef,
    pub qualification_receipt_artifact: ArtifactRef,
    pub admission_policy_artifact: ArtifactRef,
    pub coordinate_frame_id: String,
    pub candidate_delta_artifact: ArtifactRef,
    /// Nonnegative displacement/change magnitude represented by the estimate.
    pub change_magnitude_m: f64,
    pub minimum_detectable_change_m: f64,
    pub changed_volume_m3: f64,
    pub volume_standard_uncertainty_m3: f64,
}

impl SurveyChangeCandidateV1 {
    fn validate_basic(&self) -> Result<(), SurveyContractErrorV1> {
        if self.schema_version != SURVEY_EVIDENCE_SCHEMA_VERSION {
            return Err(SurveyContractErrorV1::UnsupportedSchemaVersion);
        }
        for (value, field) in [
            (self.candidate_id.as_str(), "candidate_id"),
            (self.coordinate_frame_id.as_str(), "coordinate_frame_id"),
        ] {
            if !identity_is_valid(value) {
                return Err(SurveyContractErrorV1::EmptyIdentity(field));
            }
        }
        if self.earlier_survey_artifact == self.later_survey_artifact {
            return Err(SurveyContractErrorV1::InvalidPayload);
        }
        for (artifact, field) in [
            (&self.base_geometry_artifact, "base_geometry_artifact"),
            (&self.earlier_survey_artifact, "earlier_survey_artifact"),
            (&self.later_survey_artifact, "later_survey_artifact"),
            (&self.change_estimate_artifact, "change_estimate_artifact"),
            (
                &self.qualification_receipt_artifact,
                "qualification_receipt_artifact",
            ),
            (&self.admission_policy_artifact, "admission_policy_artifact"),
            (&self.candidate_delta_artifact, "candidate_delta_artifact"),
        ] {
            validate_artifact(artifact, field)?;
        }
        for value in [
            self.change_magnitude_m,
            self.minimum_detectable_change_m,
            self.changed_volume_m3,
            self.volume_standard_uncertainty_m3,
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(SurveyContractErrorV1::InvalidPayload);
            }
        }
        if self.minimum_detectable_change_m <= 0.0 {
            return Err(SurveyContractErrorV1::InvalidPayload);
        }
        Ok(())
    }

    /// Validate a candidate against the exact current world, frame, qualified
    /// receipt, held-out observations, and an explicit acceptance policy.
    /// This function performs no world mutation.
    #[allow(clippy::too_many_arguments)]
    pub fn validate_for_admission(
        &self,
        current_world_generation: u64,
        current_base_geometry: &ArtifactRef,
        current_frame_id: &str,
        qualification_receipt_artifact: &ArtifactRef,
        receipt: &SurveyProcessingReceiptV1,
        evidence: &SurveyEvidenceEnvelopeV1,
        policy: &SurveyAdmissionPolicyV1,
        policy_artifact: &ArtifactRef,
    ) -> Result<(), SurveyContractErrorV1> {
        self.validate_basic()?;
        policy.validate()?;

        if self.base_world_generation != current_world_generation {
            return Err(SurveyContractErrorV1::StaleWorldGeneration);
        }
        if &self.base_geometry_artifact != current_base_geometry {
            return Err(SurveyContractErrorV1::BaseGeometryMismatch);
        }
        if self.coordinate_frame_id != current_frame_id {
            return Err(SurveyContractErrorV1::CoordinateFrameMismatch);
        }
        if self.qualification_receipt_artifact != *qualification_receipt_artifact
            || self.admission_policy_artifact != *policy_artifact
        {
            return Err(SurveyContractErrorV1::AdmissionPolicyMismatch);
        }

        evidence.validate()?;
        receipt.verify_artifact(qualification_receipt_artifact)?;
        policy.verify_artifact(policy_artifact)?;
        receipt.validate_against(&evidence.observations, &evidence.frames)?;
        if receipt.stage != SurveyProcessingStageV1::QualifiedChangeEstimate
            || receipt.disposition != SurveyProcessingDispositionV1::Qualified
            || receipt.output_artifact.as_ref() != Some(&self.change_estimate_artifact)
            || receipt.output_frame_id.as_deref() != Some(self.coordinate_frame_id.as_str())
            || !receipt
                .input_artifacts
                .iter()
                .any(|artifact| artifact == &self.earlier_survey_artifact)
            || !receipt
                .input_artifacts
                .iter()
                .any(|artifact| artifact == &self.later_survey_artifact)
        {
            return Err(SurveyContractErrorV1::QualificationReceiptMismatch);
        }

        if receipt.independent_checkpoint_ids.len() < policy.minimum_independent_checkpoints {
            return Err(SurveyContractErrorV1::InsufficientIndependentCheckpoints);
        }
        let metrics = receipt
            .accuracy_metrics
            .as_ref()
            .ok_or(SurveyContractErrorV1::QualificationReceiptMismatch)?;
        if metrics.detection_limit_m != Some(self.minimum_detectable_change_m)
            || metrics.estimated_changed_volume_m3 != Some(self.changed_volume_m3)
            || metrics.changed_volume_standard_uncertainty_m3
                != Some(self.volume_standard_uncertainty_m3)
        {
            return Err(SurveyContractErrorV1::QualificationReceiptMismatch);
        }
        if metrics.horizontal_accuracy_95_m > policy.max_horizontal_accuracy_95_m
            || metrics.vertical_accuracy_95_m > policy.max_vertical_accuracy_95_m
        {
            return Err(SurveyContractErrorV1::AccuracyThresholdExceeded);
        }
        if self.volume_standard_uncertainty_m3 > policy.max_volume_standard_uncertainty_m3 {
            return Err(SurveyContractErrorV1::VolumeUncertaintyExceeded);
        }
        let signal_to_detection_limit =
            self.change_magnitude_m / self.minimum_detectable_change_m;
        if !signal_to_detection_limit.is_finite()
            || signal_to_detection_limit < policy.minimum_change_to_detection_limit_ratio
        {
            return Err(SurveyContractErrorV1::ChangeBelowDetectionLimit);
        }
        Ok(())
    }
}

/// Explicit admission thresholds. The caller must bind this policy to an
/// approved artifact; the type does not decide what tolerances are appropriate
/// for a mine, a cadastral survey, or another application.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurveyAdmissionPolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    pub minimum_independent_checkpoints: usize,
    pub max_horizontal_accuracy_95_m: f64,
    pub max_vertical_accuracy_95_m: f64,
    pub max_volume_standard_uncertainty_m3: f64,
    pub minimum_change_to_detection_limit_ratio: f64,
}

impl SurveyAdmissionPolicyV1 {
    pub fn validate(&self) -> Result<(), SurveyContractErrorV1> {
        if self.schema_version != SURVEY_EVIDENCE_SCHEMA_VERSION {
            return Err(SurveyContractErrorV1::UnsupportedSchemaVersion);
        }
        if !identity_is_valid(&self.policy_id) || self.minimum_independent_checkpoints == 0 {
            return Err(SurveyContractErrorV1::InvalidPayload);
        }
        for value in [
            self.max_horizontal_accuracy_95_m,
            self.max_vertical_accuracy_95_m,
            self.max_volume_standard_uncertainty_m3,
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(SurveyContractErrorV1::InvalidPayload);
            }
        }
        if !self.minimum_change_to_detection_limit_ratio.is_finite()
            || self.minimum_change_to_detection_limit_ratio < 1.0
        {
            return Err(SurveyContractErrorV1::InvalidPayload);
        }
        Ok(())
    }

    pub fn verify_artifact(
        &self,
        artifact: &ArtifactRef,
    ) -> Result<(), SurveyContractErrorV1> {
        self.validate()?;
        verify_content_addressed(self, artifact, "admission_policy_artifact")
    }
}

/// A transport envelope for importing a bounded set of survey evidence.
/// It provides structural validation and content-addressed serialization, not
/// an automatic merge with world truth.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurveyEvidenceEnvelopeV1 {
    pub schema_version: u16,
    pub frames: Vec<SurveyCoordinateFrameV1>,
    pub sessions: Vec<SurveySessionV1>,
    pub observations: Vec<SurveyObservationV1>,
    pub transforms: Vec<SurveyTransformV1>,
    pub processing_receipts: Vec<SurveyProcessingReceiptV1>,
    pub change_candidates: Vec<SurveyChangeCandidateV1>,
}

impl SurveyEvidenceEnvelopeV1 {
    pub fn validate(&self) -> Result<(), SurveyContractErrorV1> {
        if self.schema_version != SURVEY_EVIDENCE_SCHEMA_VERSION {
            return Err(SurveyContractErrorV1::UnsupportedSchemaVersion);
        }
        for (len, max, field) in [
            (self.frames.len(), MAX_SURVEY_FRAMES, "frames"),
            (self.sessions.len(), MAX_SURVEY_SESSIONS, "sessions"),
            (
                self.observations.len(),
                MAX_SURVEY_OBSERVATIONS,
                "observations",
            ),
            (
                self.transforms.len(),
                MAX_SURVEY_TRANSFORMS,
                "transforms",
            ),
            (
                self.processing_receipts.len(),
                MAX_SURVEY_RECEIPTS,
                "processing_receipts",
            ),
            (
                self.change_candidates.len(),
                MAX_SURVEY_RECEIPTS,
                "change_candidates",
            ),
        ] {
            if len > max {
                return Err(SurveyContractErrorV1::TooManyItems(field));
            }
        }
        if self.frames.is_empty() {
            return Err(SurveyContractErrorV1::InvalidPayload);
        }

        ensure_unique(
            self.frames.iter().map(|frame| frame.frame_id.as_str()),
            "frame_id",
        )?;
        ensure_unique(
            self.sessions.iter().map(|session| session.session_id.as_str()),
            "session_id",
        )?;
        ensure_unique(
            self.observations
                .iter()
                .map(|observation| observation.observation_id.as_str()),
            "observation_id",
        )?;
        ensure_unique(
            self.transforms.iter().map(|transform| transform.transform_id.as_str()),
            "transform_id",
        )?;
        ensure_unique(
            self.processing_receipts
                .iter()
                .map(|receipt| receipt.receipt_id.as_str()),
            "receipt_id",
        )?;
        ensure_unique(
            self.change_candidates
                .iter()
                .map(|candidate| candidate.candidate_id.as_str()),
            "candidate_id",
        )?;

        for frame in &self.frames {
            frame.validate()?;
        }
        for session in &self.sessions {
            session.validate()?;
            if !self
                .frames
                .iter()
                .any(|frame| frame.frame_id == session.instrument_frame_id)
            {
                return Err(SurveyContractErrorV1::UnknownCoordinateFrame(
                    session.instrument_frame_id.clone(),
                ));
            }
        }
        let session_map: BTreeMap<&str, &SurveySessionV1> = self
            .sessions
            .iter()
            .map(|session| (session.session_id.as_str(), session))
            .collect();
        let frame_map: BTreeMap<&str, &SurveyCoordinateFrameV1> = self
            .frames
            .iter()
            .map(|frame| (frame.frame_id.as_str(), frame))
            .collect();

        for observation in &self.observations {
            let session = session_map
                .get(observation.session_id.as_str())
                .copied()
                .ok_or_else(|| {
                    SurveyContractErrorV1::UnknownSurveySession(observation.session_id.clone())
                })?;
            let frame = frame_map
                .get(observation.frame_id.as_str())
                .copied()
                .ok_or_else(|| {
                    SurveyContractErrorV1::UnknownCoordinateFrame(observation.frame_id.clone())
                })?;
            observation.validate(session, frame)?;
        }
        for transform in &self.transforms {
            transform.validate(&self.frames)?;
        }
        for receipt in &self.processing_receipts {
            receipt.validate_against(&self.observations, &self.frames)?;
        }
        for candidate in &self.change_candidates {
            candidate.validate_basic()?;
            if !frame_map.contains_key(candidate.coordinate_frame_id.as_str()) {
                return Err(SurveyContractErrorV1::UnknownCoordinateFrame(
                    candidate.coordinate_frame_id.clone(),
                ));
            }
        }
        Ok(())
    }

    /// Serialize once, hash the exact emitted bytes, and return their artifact ref.
    pub fn encode_with_digest(
        &self,
        artifact_id: impl Into<String>,
        uri: impl Into<String>,
    ) -> Result<(Vec<u8>, ArtifactRef), SurveyContractErrorV1> {
        self.validate()?;
        let artifact_id = artifact_id.into();
        let uri = uri.into();
        if !identity_is_valid(&artifact_id) || !identity_is_valid(&uri) {
            return Err(SurveyContractErrorV1::InvalidArtifact("envelope"));
        }
        let bytes =
            serde_json::to_vec(self).map_err(|_| SurveyContractErrorV1::InvalidPayload)?;
        if bytes.len() > MAX_SURVEY_EVIDENCE_BYTES {
            return Err(SurveyContractErrorV1::PayloadTooLarge);
        }
        let sha256 = format!("{:x}", Sha256::digest(&bytes));
        Ok((
            bytes,
            ArtifactRef {
                artifact_id,
                uri,
                sha256,
            },
        ))
    }
}

/// Verified wrapper keeps the out-of-band artifact identity next to the parsed
/// payload. The payload intentionally cannot contain its own digest.
#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedSurveyEvidenceEnvelopeV1 {
    pub envelope: SurveyEvidenceEnvelopeV1,
    source_artifact: ArtifactRef,
}

impl VerifiedSurveyEvidenceEnvelopeV1 {
    pub fn source_artifact(&self) -> &ArtifactRef {
        &self.source_artifact
    }

    /// Hash exact bytes before parsing; untrusted JSON cannot bypass the digest
    /// check by reformatting fields or appending data.
    pub fn decode_verified_json(
        bytes: &[u8],
        source_artifact: ArtifactRef,
    ) -> Result<Self, SurveyContractErrorV1> {
        if bytes.len() > MAX_SURVEY_EVIDENCE_BYTES {
            return Err(SurveyContractErrorV1::PayloadTooLarge);
        }
        validate_artifact(&source_artifact, "source_artifact")?;
        let expected = format!("{:x}", Sha256::digest(bytes));
        if !expected.eq_ignore_ascii_case(&source_artifact.sha256) {
            return Err(SurveyContractErrorV1::PayloadDigestMismatch);
        }
        let envelope: SurveyEvidenceEnvelopeV1 =
            serde_json::from_slice(bytes).map_err(|_| SurveyContractErrorV1::InvalidPayload)?;
        envelope.validate()?;
        Ok(Self {
            envelope,
            source_artifact,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(id: &str) -> ArtifactRef {
        ArtifactRef {
            artifact_id: id.to_owned(),
            uri: format!("survey-test://{id}"),
            sha256: format!("{:x}", Sha256::digest(id.as_bytes())),
        }
    }

    fn frame(id: &str) -> SurveyCoordinateFrameV1 {
        SurveyCoordinateFrameV1 {
            frame_id: id.to_owned(),
            crs_id: format!("LOCAL:{id}"),
            horizontal_datum_id: "LOCAL:control-network-v1".to_owned(),
            vertical_reference_id: "LOCAL:mine-level-v1".to_owned(),
            height_type: SurveyHeightTypeV1::LocalVertical,
            axis_directions: [
                SurveyAxisDirectionV1::LocalX,
                SurveyAxisDirectionV1::LocalY,
                SurveyAxisDirectionV1::LocalZ,
            ],
            axis_units: [
                SurveyCoordinateUnitV1::Metre,
                SurveyCoordinateUnitV1::Metre,
                SurveyCoordinateUnitV1::Metre,
            ],
            handedness: SurveyHandednessV1::RightHanded,
            coordinate_epoch_year: None,
        }
    }

    fn session() -> SurveySessionV1 {
        SurveySessionV1 {
            schema_version: 1,
            session_id: "session-1".to_owned(),
            instrument_id: "scanner-1".to_owned(),
            instrument_profile_artifact: artifact("instrument-profile"),
            calibration_artifact: artifact("calibration"),
            instrument_frame_id: "mine-local".to_owned(),
            started_at_unix_ns: 100,
            ended_at_unix_ns: Some(500),
            clock_uncertainty_ns: 1_000,
            raw_observation_artifacts: vec![artifact("raw-capture")],
        }
    }

    fn observation(
        id: &str,
        role: SurveyObservationRoleV1,
        timestamp: u64,
    ) -> SurveyObservationV1 {
        SurveyObservationV1 {
            schema_version: 1,
            observation_id: id.to_owned(),
            session_id: "session-1".to_owned(),
            source_id: "scanner-1".to_owned(),
            sequence: timestamp,
            captured_at_unix_ns: timestamp,
            modality: SurveyModalityV1::TerrestrialLidar,
            role,
            frame_id: "mine-local".to_owned(),
            payload_artifact: artifact(&format!("payload-{id}")),
            calibration_artifact: artifact("calibration"),
            coordinate: Some([1.0, 2.0, 3.0]),
            uncertainty: Some(SurveyUncertaintyV1::StandardDeviation {
                standard_deviation: [0.01, 0.01, 0.02],
            }),
            uncertainty_artifact: None,
            quality_flags: Vec::new(),
        }
    }

    fn accuracy_metrics() -> SurveyAccuracyMetricsV1 {
        SurveyAccuracyMetricsV1 {
            horizontal_rmse_m: 0.01,
            vertical_rmse_m: 0.02,
            horizontal_accuracy_95_m: 0.02,
            vertical_accuracy_95_m: 0.04,
            detection_limit_m: Some(0.1),
            estimated_changed_volume_m3: Some(1.0),
            changed_volume_standard_uncertainty_m3: Some(0.1),
            uncertainty_method_id: "held-out-checkpoint-v1".to_owned(),
        }
    }

    fn qualified_receipt(
        earlier: &ArtifactRef,
        later: &ArtifactRef,
    ) -> SurveyProcessingReceiptV1 {
        SurveyProcessingReceiptV1 {
            schema_version: 1,
            receipt_id: "receipt-1".to_owned(),
            stage: SurveyProcessingStageV1::QualifiedChangeEstimate,
            disposition: SurveyProcessingDispositionV1::Qualified,
            input_artifacts: vec![earlier.clone(), later.clone()],
            output_artifact: Some(artifact("change-estimate")),
            algorithm_id: "change-detector".to_owned(),
            algorithm_version: "1.0.0".to_owned(),
            algorithm_build_sha256: artifact("build").sha256,
            configuration_sha256: artifact("config").sha256,
            source_frame_ids: vec!["mine-local".to_owned()],
            output_frame_id: Some("mine-local".to_owned()),
            fitting_observation_ids: vec!["fit".to_owned()],
            independent_checkpoint_ids: vec!["check".to_owned()],
            excluded_observation_ids: Vec::new(),
            accuracy_metrics: Some(accuracy_metrics()),
        }
    }

    fn serialized_artifact<T: Serialize>(id: &str, value: &T) -> ArtifactRef {
        let bytes = serde_json::to_vec(value).expect("serialize artifact payload");
        ArtifactRef {
            artifact_id: id.to_owned(),
            uri: format!("survey-test://{id}"),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
        }
    }

    fn evidence_for_admission() -> SurveyEvidenceEnvelopeV1 {
        let mut evidence = envelope();
        evidence.observations = vec![
            observation("fit", SurveyObservationRoleV1::FitOrCalibration, 200),
            observation("check", SurveyObservationRoleV1::IndependentCheckpoint, 300),
        ];
        evidence
    }

    fn policy() -> SurveyAdmissionPolicyV1 {
        SurveyAdmissionPolicyV1 {
            schema_version: 1,
            policy_id: "mine-survey-admission-v1".to_owned(),
            minimum_independent_checkpoints: 1,
            max_horizontal_accuracy_95_m: 0.05,
            max_vertical_accuracy_95_m: 0.05,
            max_volume_standard_uncertainty_m3: 0.2,
            minimum_change_to_detection_limit_ratio: 2.0,
        }
    }

    fn candidate(
        earlier: &ArtifactRef,
        later: &ArtifactRef,
        receipt_artifact: &ArtifactRef,
        policy_artifact: &ArtifactRef,
    ) -> SurveyChangeCandidateV1 {
        SurveyChangeCandidateV1 {
            schema_version: 1,
            candidate_id: "candidate-1".to_owned(),
            base_world_generation: 7,
            base_geometry_artifact: artifact("geometry-before"),
            earlier_survey_artifact: earlier.clone(),
            later_survey_artifact: later.clone(),
            change_estimate_artifact: artifact("change-estimate"),
            qualification_receipt_artifact: receipt_artifact.clone(),
            admission_policy_artifact: policy_artifact.clone(),
            coordinate_frame_id: "mine-local".to_owned(),
            candidate_delta_artifact: artifact("candidate-delta"),
            change_magnitude_m: 0.5,
            minimum_detectable_change_m: 0.1,
            changed_volume_m3: 1.0,
            volume_standard_uncertainty_m3: 0.1,
        }
    }

    fn envelope() -> SurveyEvidenceEnvelopeV1 {
        SurveyEvidenceEnvelopeV1 {
            schema_version: 1,
            frames: vec![frame("mine-local")],
            sessions: vec![session()],
            observations: vec![observation(
                "raw-1",
                SurveyObservationRoleV1::ContextOnly,
                200,
            )],
            transforms: Vec::new(),
            processing_receipts: Vec::new(),
            change_candidates: Vec::new(),
        }
    }

    #[test]
    fn frame_requires_explicit_consistent_axis_units() {
        let mut good = frame("mine-local");
        assert!(good.validate().is_ok());
        good.axis_units[0] = SurveyCoordinateUnitV1::Degree;
        assert_eq!(
            good.validate(),
            Err(SurveyContractErrorV1::InvalidCoordinateFrame("axis_units"))
        );
    }

    #[test]
    fn frame_rejects_duplicate_axis_semantics() {
        let mut bad = frame("mine-local");
        bad.axis_directions = [
            SurveyAxisDirectionV1::LocalX,
            SurveyAxisDirectionV1::LocalX,
            SurveyAxisDirectionV1::LocalZ,
        ];
        assert_eq!(
            bad.validate(),
            Err(SurveyContractErrorV1::InvalidCoordinateFrame(
                "axis_directions"
            ))
        );
    }

    #[test]
    fn uncertainty_accepts_positive_semidefinite_covariance() {
        let uncertainty = SurveyUncertaintyV1::Covariance {
            covariance: [[1.0, 1.0, 0.0], [1.0, 1.0, 0.0], [0.0, 0.0, 0.0]],
        };
        assert!(uncertainty.validate().is_ok());
    }

    #[test]
    fn uncertainty_rejects_asymmetric_or_indefinite_covariance() {
        let asymmetric = SurveyUncertaintyV1::Covariance {
            covariance: [[1.0, 0.4, 0.0], [0.2, 1.0, 0.0], [0.0, 0.0, 1.0]],
        };
        let indefinite = SurveyUncertaintyV1::Covariance {
            covariance: [[1.0, 2.0, 0.0], [2.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        };
        assert_eq!(
            asymmetric.validate(),
            Err(SurveyContractErrorV1::InvalidUncertainty)
        );
        assert_eq!(
            indefinite.validate(),
            Err(SurveyContractErrorV1::InvalidUncertainty)
        );
    }

    #[test]
    fn raw_observation_can_be_retained_without_a_claim_of_uncertainty() {
        let mut item = observation("raw", SurveyObservationRoleV1::ContextOnly, 200);
        item.uncertainty = None;
        item.uncertainty_artifact = None;
        assert!(item.validate(&session(), &frame("mine-local")).is_ok());
    }

    #[test]
    fn observation_outside_session_time_is_rejected() {
        let item = observation("late", SurveyObservationRoleV1::ContextOnly, 501);
        assert_eq!(
            item.validate(&session(), &frame("mine-local")),
            Err(SurveyContractErrorV1::SessionTimeMismatch)
        );
    }

    #[test]
    fn transform_requires_declared_distinct_frames() {
        let transform = SurveyTransformV1 {
            schema_version: 1,
            transform_id: "survey-to-world".to_owned(),
            source_frame_id: "mine-local".to_owned(),
            target_frame_id: "missing-world".to_owned(),
            operation_artifact: artifact("transform-operation"),
            resource_artifacts: vec![artifact("grid-resource")],
            valid_from_epoch_year: Some(2020.0),
            valid_to_epoch_year: Some(2030.0),
            declared_horizontal_accuracy_95_m: Some(0.03),
            declared_vertical_accuracy_95_m: Some(0.05),
        };
        assert_eq!(
            transform.validate(&[frame("mine-local")]),
            Err(SurveyContractErrorV1::UnknownCoordinateFrame(
                "missing-world".to_owned()
            ))
        );
    }

    #[test]
    fn processing_receipt_rejects_checkpoint_reused_for_fitting() {
        let earlier = artifact("earlier-survey");
        let later = artifact("later-survey");
        let mut receipt = qualified_receipt(&earlier, &later);
        receipt.fitting_observation_ids = vec!["check".to_owned()];
        assert_eq!(
            receipt.validate_against(
                &[
                    observation("fit", SurveyObservationRoleV1::FitOrCalibration, 200),
                    observation("check", SurveyObservationRoleV1::IndependentCheckpoint, 300),
                ],
                &[frame("mine-local")]
            ),
            Err(SurveyContractErrorV1::InvalidProcessingReceipt(
                "overlapping_observation_roles"
            ))
        );
    }

    #[test]
    fn candidate_admission_binds_generation_geometry_frame_and_policy() {
        let earlier = artifact("earlier-survey");
        let later = artifact("later-survey");
        let admission_policy = policy();
        let policy_artifact = serialized_artifact("policy-artifact", &admission_policy);
        let receipt = qualified_receipt(&earlier, &later);
        let receipt_artifact = serialized_artifact("receipt-artifact", &receipt);
        let evidence = evidence_for_admission();
        let candidate = candidate(
            &earlier,
            &later,
            &receipt_artifact,
            &policy_artifact,
        );

        assert!(candidate
            .validate_for_admission(
                7,
                &artifact("geometry-before"),
                "mine-local",
                &receipt_artifact,
                &receipt,
                &evidence,
                &admission_policy,
                &policy_artifact,
            )
            .is_ok());
        assert_eq!(
            candidate.validate_for_admission(
                8,
                &artifact("geometry-before"),
                "mine-local",
                &receipt_artifact,
                &receipt,
                &evidence,
                &admission_policy,
                &policy_artifact,
            ),
            Err(SurveyContractErrorV1::StaleWorldGeneration)
        );
        assert_eq!(
            candidate.validate_for_admission(
                7,
                &artifact("changed-geometry"),
                "mine-local",
                &receipt_artifact,
                &receipt,
                &evidence,
                &admission_policy,
                &policy_artifact,
            ),
            Err(SurveyContractErrorV1::BaseGeometryMismatch)
        );
    }

    #[test]
    fn candidate_rejects_uncertainty_over_policy_limit() {
        let earlier = artifact("earlier-survey");
        let later = artifact("later-survey");
        let admission_policy = policy();
        let mut receipt = qualified_receipt(&earlier, &later);
        receipt
            .accuracy_metrics
            .as_mut()
            .expect("metrics")
            .changed_volume_standard_uncertainty_m3 = Some(0.21);
        let receipt_artifact = serialized_artifact("receipt-artifact", &receipt);
        let policy_artifact = serialized_artifact("policy-artifact", &admission_policy);
        let mut candidate = candidate(
            &earlier,
            &later,
            &receipt_artifact,
            &policy_artifact,
        );
        candidate.volume_standard_uncertainty_m3 = 0.21;

        assert_eq!(
            candidate.validate_for_admission(
                7,
                &artifact("geometry-before"),
                "mine-local",
                &receipt_artifact,
                &receipt,
                &evidence_for_admission(),
                &admission_policy,
                &policy_artifact,
            ),
            Err(SurveyContractErrorV1::VolumeUncertaintyExceeded)
        );
    }

    #[test]
    fn candidate_below_detection_limit_ratio_is_not_admissible() {
        let earlier = artifact("earlier-survey");
        let later = artifact("later-survey");
        let admission_policy = policy();
        let receipt = qualified_receipt(&earlier, &later);
        let receipt_artifact = serialized_artifact("receipt-artifact", &receipt);
        let policy_artifact = serialized_artifact("policy-artifact", &admission_policy);
        let mut candidate = candidate(
            &earlier,
            &later,
            &receipt_artifact,
            &policy_artifact,
        );
        candidate.change_magnitude_m = 0.15;

        assert_eq!(
            candidate.validate_for_admission(
                7,
                &artifact("geometry-before"),
                "mine-local",
                &receipt_artifact,
                &receipt,
                &evidence_for_admission(),
                &admission_policy,
                &policy_artifact,
            ),
            Err(SurveyContractErrorV1::ChangeBelowDetectionLimit)
        );
    }

    #[test]
    fn envelope_encoding_is_deterministic_and_digest_verified() {
        let envelope = envelope();
        let (first_bytes, first_artifact) = envelope
            .encode_with_digest("survey-envelope", "survey-test://envelope")
            .expect("valid envelope");
        let (second_bytes, second_artifact) = envelope
            .encode_with_digest("survey-envelope", "survey-test://envelope")
            .expect("same inputs");
        assert_eq!(first_bytes, second_bytes);
        assert_eq!(first_artifact, second_artifact);

        let decoded = VerifiedSurveyEvidenceEnvelopeV1::decode_verified_json(
            &first_bytes,
            first_artifact.clone(),
        )
        .expect("verified envelope");
        assert_eq!(decoded.envelope, envelope);
        assert_eq!(decoded.source_artifact(), &first_artifact);

        let mut modified = first_bytes;
        modified.push(b' ');
        assert_eq!(
            VerifiedSurveyEvidenceEnvelopeV1::decode_verified_json(
                &modified,
                first_artifact
            ),
            Err(SurveyContractErrorV1::PayloadDigestMismatch)
        );
    }

    #[test]
    fn decoder_rejects_unknown_fields_after_digest_verification() {
        let mut bytes = serde_json::to_vec(&envelope()).expect("serialize");
        let last = bytes.pop().expect("closing brace");
        assert_eq!(last, b'}');
        bytes.extend_from_slice(b",\"unexpected_field\":true}");
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let source_artifact = ArtifactRef {
            artifact_id: "unknown-field-test".to_owned(),
            uri: "survey-test://unknown-field".to_owned(),
            sha256: digest,
        };
        assert_eq!(
            VerifiedSurveyEvidenceEnvelopeV1::decode_verified_json(&bytes, source_artifact),
            Err(SurveyContractErrorV1::InvalidPayload)
        );
    }

    #[test]
    fn payload_size_is_checked_before_json_parsing() {
        let bytes = vec![b' '; MAX_SURVEY_EVIDENCE_BYTES + 1];
        let source_artifact = ArtifactRef {
            artifact_id: "oversized".to_owned(),
            uri: "survey-test://oversized".to_owned(),
            sha256: "0".repeat(SHA256_HEX_LENGTH),
        };
        assert_eq!(
            VerifiedSurveyEvidenceEnvelopeV1::decode_verified_json(&bytes, source_artifact),
            Err(SurveyContractErrorV1::PayloadTooLarge)
        );
    }

    #[test]
    fn missing_session_or_frame_cannot_be_inferred_from_measurements() {
        let mut invalid = envelope();
        invalid.observations[0].frame_id = "missing-frame".to_owned();
        assert_eq!(
            invalid.validate(),
            Err(SurveyContractErrorV1::UnknownCoordinateFrame(
                "missing-frame".to_owned()
            ))
        );

        let mut invalid_session = envelope();
        invalid_session.observations[0].session_id = "missing-session".to_owned();
        assert_eq!(
            invalid_session.validate(),
            Err(SurveyContractErrorV1::UnknownSurveySession(
                "missing-session".to_owned()
            ))
        );
    }
}
