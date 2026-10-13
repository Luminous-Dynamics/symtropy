// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fail-closed contract for material-aware excavation solver responses.
//!
//! This module defines an admission boundary, not an excavation constitutive law.
//! It prevents a result from being accepted for a different world generation,
//! input digest, tool, material or solver. Numerical results still require
//! independent benchmark and held-out physical validation before promotion.

use crate::world_contract::{
    ArtifactRef, PhysicsDomain, SolverDeclarationV1, WorldManifestV1, WorldValidationIssue,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MATERIAL_AWARE_EXCAVATION_SCHEMA_VERSION: u16 = 1;
pub const EXCAVATION_STATE_DELTA_SCHEMA_VERSION: u16 = 1;
const SHA256_HEX_LENGTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExcavationToolProfileV1 {
    pub tool_id: String,
    /// This profile artifact's ID must equal tool_id and be a declared world asset.
    pub profile_artifact: ArtifactRef,
    /// The exact geometry artifact must also be declared by the world's assets.
    pub geometry_artifact: ArtifactRef,
    pub feed_speed_limit_m_s: f64,
    pub angular_speed_limit_rad_s: f64,
    /// Maximum tool-interface mechanical power for this operating envelope.
    pub power_limit_w: f64,
    pub thrust_limit_n: f64,
    pub torque_limit_nm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialAwareExcavationRequestV1 {
    pub schema_version: u16,
    pub request_id: String,
    /// Caller-controlled generation; responses from older generations are stale.
    pub world_generation: u64,
    /// Digest of the canonical serialized manifest, computed by the caller.
    pub world_manifest_sha256: String,
    /// Digest of the complete command and solver-input bundle.
    pub input_digest_sha256: String,
    pub material_profile_id: String,
    /// Exact initial rock/terrain geometry targeted by this operation.
    pub target_geometry_artifact: ArtifactRef,
    /// Frame in which the spatial state delta is expressed.
    pub target_frame_id: String,
    pub solver_id: String,
    pub tool: ExcavationToolProfileV1,
    /// Commanded linear feed speed, in m/s.
    pub feed_speed_m_s: f64,
    /// Commanded angular speed magnitude, in rad/s.
    pub angular_speed_rad_s: f64,
    pub requested_duration_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExcavationDispositionV1 {
    CompletedWithinEnvelope,
    CompletedWithWarnings,
    OutOfDomain,
    InsufficientInputEvidence,
    NumericalNonConvergence,
    DeadlineExceeded,
    InvalidResult,
    Cancelled,
}

/// Supported artifact encodings for an excavation spatial state transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExcavationStateDeltaFormatV1 {
    /// Per-voxel material/occupancy update with explicit grid dimensions and origin.
    VoxelMaterialPatch,
    /// Surface triangle mesh and associated material/property patch.
    TriangleMeshPatch,
    /// Signed-distance-field or occupancy-field patch with declared sampling.
    SpatialFieldPatch,
    /// Sparse, indexed set of removed cells for a declared volumetric grid.
    CellRemovalMask,
}

/// A spatial state change produced by a completed excavation attempt.
///
/// The transition artifacts identify the exact previous geometry and world
/// generation and carry the resulting geometry plus a versioned, frame-bound
/// delta artifact. This contract validates identifiers/digests but does not
/// decode or certify the geometry update.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExcavationSpatialDeltaV1 {
    pub base_world_generation: u64,
    pub result_world_generation: u64,
    pub base_geometry_artifact: ArtifactRef,
    pub result_geometry_artifact: ArtifactRef,
    pub state_delta_artifact: ArtifactRef,
    pub state_delta_format: ExcavationStateDeltaFormatV1,
    pub state_delta_schema_version: u16,
    pub coordinate_frame_id: String,
}

/** Supported schema for a decoded cell-removal-mask artifact. */
pub const CELL_REMOVAL_MASK_SCHEMA_VERSION: u16 = 1;

/// Serializable JSON payload for a grid-indexed cell-removal mask.
///
/// The payload intentionally excludes its own digest: that digest is computed
/// over these exact JSON bytes and carried out-of-band by an artifact reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellRemovalMaskPayloadV1 {
    pub schema_version: u16,
    pub base_world_generation: u64,
    pub base_geometry_artifact: ArtifactRef,
    pub material_profile_id: String,
    pub coordinate_frame_id: String,
    /// Grid origin in the declared coordinate frame, metres.
    pub grid_origin_m: [f64; 3],
    /// Full cell dimensions, metres.
    pub cell_size_m: [f64; 3],
    /// Number of cells on each axis; each value must be nonzero.
    pub grid_dimensions: [u32; 3],
    /// Unique cell indices in strict lexicographic order for deterministic replay.
    pub removed_cells: Vec<[u32; 3]>,
}

/// A decoded cell-removal mask bound to the exact bytes from which it was parsed.
///
/// Use `decode_verified_json` for untrusted data paths. Semantic validation
/// does not mutate terrain; a separate generation-checked commit is required.
#[derive(Debug, Clone, PartialEq)]
pub struct CellRemovalMaskV1 {
    pub payload: CellRemovalMaskPayloadV1,
    source_artifact: ArtifactRef,
}

/// A measured or solver-reported excavation output. Values use named SI units.
///
/// These fields are observations/outputs supplied by a model or instrument. This
/// record does not infer rock fracture or make synthetic voxel removal physical.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExcavationResponseV1 {
    /// Removed *solid* rock volume, excluding loose-spoil swell, in m^3.
    pub removed_solid_rock_volume_m3: f64,
    /// Mechanical energy delivered at the defined tool boundary, in joules.
    pub mechanical_energy_j: f64,
    pub peak_thrust_n: f64,
    pub peak_torque_nm: f64,
    /// Incremental tool-wear fraction in [0, 1] over this attempt.
    pub wear_delta_fraction: f64,
    /// Energy assigned to heat by the declared model/measurement boundary, in J.
    pub heat_energy_j: f64,
    /// Loose spoil volume in m^3; may differ from solid volume due to swell.
    pub loose_spoil_volume_m3: f64,
    pub removed_solid_rock_volume_standard_uncertainty_m3: Option<f64>,
    pub mechanical_energy_standard_uncertainty_j: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialAwareExcavationResultV1 {
    pub schema_version: u16,
    /// Echo the entire request so exact request identity is checked, not sampled.
    pub request: MaterialAwareExcavationRequestV1,
    pub disposition: ExcavationDispositionV1,
    /// Digest-bound result or diagnostic bundle; failure logs can be retained without outputs.
    pub result_artifact: Option<ArtifactRef>,
    /// Non-completed dispositions must not carry accepted physical outputs.
    pub response: Option<ExcavationResponseV1>,
    /// Physical state transition, required for completed results only.
    pub spatial_delta: Option<ExcavationSpatialDeltaV1>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExcavationContractErrorV1 {
    UnsupportedSchemaVersion,
    InvalidWorldManifest(Vec<WorldValidationIssue>),
    EmptyIdentity(&'static str),
    InvalidDigest(&'static str),
    WorldManifestDigestMismatch,
    InvalidToolGeometryArtifact,
    UndeclaredToolGeometryArtifact,
    InvalidToolProfileArtifact,
    UndeclaredToolProfileArtifact,
    ToolProfileIdentityMismatch,
    UndeclaredTargetGeometryArtifact,
    UnknownTargetFrame(String),
    InvalidSpatialDelta,
    InvalidCellRemovalMaskSchema,
    InvalidCellMaskPayload,
    InvalidCellMaskArtifact,
    CellMaskPayloadDigestMismatch,
    CellMaskFormatMismatch,
    CellMaskArtifactMismatch,
    CellMaskMaterialMismatch,
    InvalidCellGrid,
    EmptyCellRemovalMask,
    CellIndexOutOfBounds([u32; 3]),
    CellIndicesNotCanonical,
    UnknownMaterialProfile(String),
    UnknownSolver(String),
    UnsupportedSolverDomain(PhysicsDomain),
    InvalidCommand(&'static str),
    RequestIdentityMismatch,
    InvalidResultArtifact,
    InconsistentDisposition,
    InvalidOutput(&'static str),
    OperatingEnvelopeExceeded(&'static str),
    WarningStatusWithoutWarning,
}

impl CellRemovalMaskV1 {
    /// Decode strict JSON only when its exact bytes match the supplied artifact digest.
    ///
    /// The artifact reference is out-of-band; embedding its own digest in the
    /// bytes being hashed would make the digest self-referential.
    pub fn decode_verified_json(
        bytes: &[u8],
        source_artifact: ArtifactRef,
    ) -> Result<Self, ExcavationContractErrorV1> {
        if !valid_artifact(&source_artifact) {
            return Err(ExcavationContractErrorV1::InvalidCellMaskArtifact);
        }
        let actual_digest = format!("{:x}", Sha256::digest(bytes));
        if !actual_digest.eq_ignore_ascii_case(&source_artifact.sha256) {
            return Err(ExcavationContractErrorV1::CellMaskPayloadDigestMismatch);
        }
        let payload: CellRemovalMaskPayloadV1 = serde_json::from_slice(bytes)
            .map_err(|_| ExcavationContractErrorV1::InvalidCellMaskPayload)?;
        Ok(Self { payload, source_artifact })
    }

    /// Validate a decoded mask against the exact request, world and result delta.
    ///
    /// The caller must supply the digest of the canonical world-manifest bytes
    /// and must verify the mask's byte digest before constructing this value.
    /// No cell or terrain state is mutated by validation.
    pub fn validate_against(
        &self,
        world: &WorldManifestV1,
        canonical_world_manifest_sha256: &str,
        request: &MaterialAwareExcavationRequestV1,
        delta: &ExcavationSpatialDeltaV1,
    ) -> Result<(), ExcavationContractErrorV1> {
        request.validate_against_world(world, canonical_world_manifest_sha256)?;
        validate_spatial_delta(delta, request)?;
        if delta.state_delta_format != ExcavationStateDeltaFormatV1::CellRemovalMask {
            return Err(ExcavationContractErrorV1::CellMaskFormatMismatch);
        }
        let payload = &self.payload;
        if payload.schema_version != CELL_REMOVAL_MASK_SCHEMA_VERSION {
            return Err(ExcavationContractErrorV1::InvalidCellRemovalMaskSchema);
        }
        if !valid_artifact(&self.source_artifact)
            || self.source_artifact != delta.state_delta_artifact
            || payload.base_geometry_artifact != delta.base_geometry_artifact
            || payload.base_world_generation != delta.base_world_generation
            || payload.coordinate_frame_id != delta.coordinate_frame_id
        {
            return Err(ExcavationContractErrorV1::CellMaskArtifactMismatch);
        }
        if payload.material_profile_id != request.material_profile_id {
            return Err(ExcavationContractErrorV1::CellMaskMaterialMismatch);
        }
        if payload.grid_dimensions.iter().any(|dimension| *dimension == 0)
            || !payload.grid_origin_m.iter().all(|coordinate| coordinate.is_finite())
            || !payload.cell_size_m.iter().all(|size| size.is_finite() && *size > 0.0)
        {
            return Err(ExcavationContractErrorV1::InvalidCellGrid);
        }
        for axis in 0..3 {
            let extent = f64::from(payload.grid_dimensions[axis]) * payload.cell_size_m[axis];
            if !extent.is_finite() || !(payload.grid_origin_m[axis] + extent).is_finite() {
                return Err(ExcavationContractErrorV1::InvalidCellGrid);
            }
        }
        if !world.frames.iter().any(|frame| frame.frame_id == payload.coordinate_frame_id) {
            return Err(ExcavationContractErrorV1::UnknownTargetFrame(
                payload.coordinate_frame_id.clone(),
            ));
        }
        if payload.removed_cells.is_empty() {
            return Err(ExcavationContractErrorV1::EmptyCellRemovalMask);
        }
        for cell in &payload.removed_cells {
            if (0..3).any(|axis| cell[axis] >= payload.grid_dimensions[axis]) {
                return Err(ExcavationContractErrorV1::CellIndexOutOfBounds(*cell));
            }
        }
        if !payload.removed_cells.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(ExcavationContractErrorV1::CellIndicesNotCanonical);
        }
        Ok(())
    }
}

impl MaterialAwareExcavationRequestV1 {
    /// Check the request against the exact world supplied by the caller.
    ///
    /// The manifest digest is not recomputed here: the caller must hash the
    /// canonical manifest representation and keep that exact artifact available.
    pub fn validate_against_world(
        &self,
        world: &WorldManifestV1,
        canonical_world_manifest_sha256: &str,
    ) -> Result<(), ExcavationContractErrorV1> {
        world
            .validate()
            .map_err(ExcavationContractErrorV1::InvalidWorldManifest)?;

        if self.schema_version != MATERIAL_AWARE_EXCAVATION_SCHEMA_VERSION {
            return Err(ExcavationContractErrorV1::UnsupportedSchemaVersion);
        }
        for (field, value) in [
            ("request_id", self.request_id.as_str()),
            ("material_profile_id", self.material_profile_id.as_str()),
            ("solver_id", self.solver_id.as_str()),
            ("tool_id", self.tool.tool_id.as_str()),
        ] {
            if value.trim().is_empty() || value.trim() != value {
                return Err(ExcavationContractErrorV1::EmptyIdentity(field));
            }
        }
        if !is_sha256_hex(&self.world_manifest_sha256)
            || !is_sha256_hex(canonical_world_manifest_sha256)
        {
            return Err(ExcavationContractErrorV1::InvalidDigest(
                "world_manifest_sha256",
            ));
        }
        if self.world_manifest_sha256 != canonical_world_manifest_sha256 {
            return Err(ExcavationContractErrorV1::WorldManifestDigestMismatch);
        }
        if !is_sha256_hex(&self.input_digest_sha256) {
            return Err(ExcavationContractErrorV1::InvalidDigest(
                "input_digest_sha256",
            ));
        }
        if !valid_artifact(&self.tool.profile_artifact) {
            return Err(ExcavationContractErrorV1::InvalidToolProfileArtifact);
        }
        if self.tool.profile_artifact.artifact_id != self.tool.tool_id {
            return Err(ExcavationContractErrorV1::ToolProfileIdentityMismatch);
        }
        if !world.world_assets.iter().any(|asset| asset == &self.tool.profile_artifact) {
            return Err(ExcavationContractErrorV1::UndeclaredToolProfileArtifact);
        }
        if !valid_artifact(&self.tool.geometry_artifact) {
            return Err(ExcavationContractErrorV1::InvalidToolGeometryArtifact);
        }
        if !world.world_assets.iter().any(|asset| asset == &self.tool.geometry_artifact) {
            return Err(ExcavationContractErrorV1::UndeclaredToolGeometryArtifact);
        }
        if !world
            .material_profiles
            .iter()
            .any(|profile| profile.profile_id == self.material_profile_id)
        {
            return Err(ExcavationContractErrorV1::UnknownMaterialProfile(
                self.material_profile_id.clone(),
            ));
        }
        if !valid_artifact(&self.target_geometry_artifact) {
            return Err(ExcavationContractErrorV1::UndeclaredTargetGeometryArtifact);
        }
        if !world
            .world_assets
            .iter()
            .any(|asset| asset == &self.target_geometry_artifact)
        {
            return Err(ExcavationContractErrorV1::UndeclaredTargetGeometryArtifact);
        }
        if self.target_frame_id.trim().is_empty()
            || !world.frames.iter().any(|frame| frame.frame_id == self.target_frame_id)
        {
            return Err(ExcavationContractErrorV1::UnknownTargetFrame(
                self.target_frame_id.clone(),
            ));
        }

        let solver = world
            .solvers
            .iter()
            .find(|solver| solver.solver_id == self.solver_id)
            .ok_or_else(|| ExcavationContractErrorV1::UnknownSolver(self.solver_id.clone()))?;
        if !supports_material_excavation(solver) {
            return Err(ExcavationContractErrorV1::UnsupportedSolverDomain(
                solver.domain,
            ));
        }

        if !self.feed_speed_m_s.is_finite() || self.feed_speed_m_s < 0.0 {
            return Err(ExcavationContractErrorV1::InvalidCommand(
                "feed_speed_m_s",
            ));
        }
        if !self.angular_speed_rad_s.is_finite() || self.angular_speed_rad_s < 0.0 {
            return Err(ExcavationContractErrorV1::InvalidCommand(
                "angular_speed_rad_s",
            ));
        }
        if self.requested_duration_ns == 0 {
            return Err(ExcavationContractErrorV1::InvalidCommand(
                "requested_duration_ns",
            ));
        }
        for (field, value) in [
            ("feed_speed_limit_m_s", self.tool.feed_speed_limit_m_s),
            ("angular_speed_limit_rad_s", self.tool.angular_speed_limit_rad_s),
            ("power_limit_w", self.tool.power_limit_w),
            ("thrust_limit_n", self.tool.thrust_limit_n),
            ("torque_limit_nm", self.tool.torque_limit_nm),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(ExcavationContractErrorV1::InvalidCommand(field));
            }
        }
        if self.feed_speed_m_s > self.tool.feed_speed_limit_m_s {
            return Err(ExcavationContractErrorV1::OperatingEnvelopeExceeded(
                "feed_speed_m_s",
            ));
        }
        if self.angular_speed_rad_s > self.tool.angular_speed_limit_rad_s {
            return Err(ExcavationContractErrorV1::OperatingEnvelopeExceeded(
                "angular_speed_rad_s",
            ));
        }
        Ok(())
    }
}

impl MaterialAwareExcavationResultV1 {
    /// Admit a result only for the exact expected request and a structurally
    /// valid world. Completion-with-warnings preserves over-limit outputs as
    /// diagnostic evidence instead of mislabelling them as in-envelope.
    pub fn validate_against(
        &self,
        world: &WorldManifestV1,
        canonical_world_manifest_sha256: &str,
        expected_request: &MaterialAwareExcavationRequestV1,
    ) -> Result<(), ExcavationContractErrorV1> {
        expected_request.validate_against_world(world, canonical_world_manifest_sha256)?;
        if self.schema_version != MATERIAL_AWARE_EXCAVATION_SCHEMA_VERSION {
            return Err(ExcavationContractErrorV1::UnsupportedSchemaVersion);
        }
        if &self.request != expected_request {
            return Err(ExcavationContractErrorV1::RequestIdentityMismatch);
        }

        match self.disposition {
            ExcavationDispositionV1::CompletedWithinEnvelope
            | ExcavationDispositionV1::CompletedWithWarnings => {
                let artifact = self
                    .result_artifact
                    .as_ref()
                    .ok_or(ExcavationContractErrorV1::InvalidResultArtifact)?;
                if !valid_artifact(artifact) {
                    return Err(ExcavationContractErrorV1::InvalidResultArtifact);
                }
                let response = self
                    .response
                    .as_ref()
                    .ok_or(ExcavationContractErrorV1::InconsistentDisposition)?;
                validate_response(response)?;
                let spatial_delta = self
                    .spatial_delta
                    .as_ref()
                    .ok_or(ExcavationContractErrorV1::InvalidSpatialDelta)?;
                validate_spatial_delta(
                    spatial_delta,
                    expected_request,
                )?;
                if self.disposition == ExcavationDispositionV1::CompletedWithinEnvelope {
                    validate_operating_envelope(expected_request, response)?;
                    if !self.warnings.is_empty() {
                        return Err(ExcavationContractErrorV1::InconsistentDisposition);
                    }
                } else if self.warnings.iter().all(|warning| warning.trim().is_empty()) {
                    return Err(ExcavationContractErrorV1::WarningStatusWithoutWarning);
                }
            }
            _ => {
                if self.response.is_some() || self.spatial_delta.is_some() {
                    return Err(ExcavationContractErrorV1::InconsistentDisposition);
                }
                if self
                    .result_artifact
                    .as_ref()
                    .is_some_and(|artifact| !valid_artifact(artifact))
                {
                    return Err(ExcavationContractErrorV1::InvalidResultArtifact);
                }
            }
        }
        Ok(())
    }
}

fn validate_spatial_delta(
    delta: &ExcavationSpatialDeltaV1,
    request: &MaterialAwareExcavationRequestV1,
) -> Result<(), ExcavationContractErrorV1> {
    let expected_result_generation = request
        .world_generation
        .checked_add(1)
        .ok_or(ExcavationContractErrorV1::InvalidSpatialDelta)?;
    if delta.base_world_generation != request.world_generation
        || delta.result_world_generation != expected_result_generation
        || delta.base_geometry_artifact != request.target_geometry_artifact
        || delta.coordinate_frame_id != request.target_frame_id
        || delta.coordinate_frame_id.trim().is_empty()
        || delta.state_delta_schema_version != EXCAVATION_STATE_DELTA_SCHEMA_VERSION
        || !valid_artifact(&delta.base_geometry_artifact)
        || !valid_artifact(&delta.result_geometry_artifact)
        || !valid_artifact(&delta.state_delta_artifact)
    {
        return Err(ExcavationContractErrorV1::InvalidSpatialDelta);
    }
    Ok(())
}

fn supports_material_excavation(solver: &SolverDeclarationV1) -> bool {
    matches!(
        solver.domain,
        PhysicsDomain::MechanicalCutting
            | PhysicsDomain::Fracture
            | PhysicsDomain::Thermal
            | PhysicsDomain::Electromagnetic
            | PhysicsDomain::ElectricalDischarge
    )
}

fn validate_response(
    response: &ExcavationResponseV1,
) -> Result<(), ExcavationContractErrorV1> {
    for (field, value) in [
        ("removed_solid_rock_volume_m3", response.removed_solid_rock_volume_m3),
        ("mechanical_energy_j", response.mechanical_energy_j),
        ("peak_thrust_n", response.peak_thrust_n),
        ("peak_torque_nm", response.peak_torque_nm),
        ("wear_delta_fraction", response.wear_delta_fraction),
        ("heat_energy_j", response.heat_energy_j),
        ("loose_spoil_volume_m3", response.loose_spoil_volume_m3),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(ExcavationContractErrorV1::InvalidOutput(field));
        }
    }
    if response.wear_delta_fraction > 1.0 {
        return Err(ExcavationContractErrorV1::InvalidOutput(
            "wear_delta_fraction",
        ));
    }
    for (field, value) in [
        (
            "removed_solid_rock_volume_standard_uncertainty_m3",
            response.removed_solid_rock_volume_standard_uncertainty_m3,
        ),
        (
            "mechanical_energy_standard_uncertainty_j",
            response.mechanical_energy_standard_uncertainty_j,
        ),
    ] {
        if value.is_some_and(|value| !value.is_finite() || value < 0.0) {
            return Err(ExcavationContractErrorV1::InvalidOutput(field));
        }
    }
    Ok(())
}

fn validate_operating_envelope(
    request: &MaterialAwareExcavationRequestV1,
    response: &ExcavationResponseV1,
) -> Result<(), ExcavationContractErrorV1> {
    if response.peak_thrust_n > request.tool.thrust_limit_n {
        return Err(ExcavationContractErrorV1::OperatingEnvelopeExceeded(
            "peak_thrust_n",
        ));
    }
    if response.peak_torque_nm > request.tool.torque_limit_nm {
        return Err(ExcavationContractErrorV1::OperatingEnvelopeExceeded(
            "peak_torque_nm",
        ));
    }
    let duration_s = request.requested_duration_ns as f64 / 1_000_000_000.0;
    let energy_limit_j = request.tool.power_limit_w * duration_s;
    if !energy_limit_j.is_finite() || response.mechanical_energy_j > energy_limit_j {
        return Err(ExcavationContractErrorV1::OperatingEnvelopeExceeded(
            "mechanical_energy_j",
        ));
    }
    Ok(())
}

fn valid_artifact(artifact: &ArtifactRef) -> bool {
    !artifact.artifact_id.trim().is_empty()
        && !artifact.uri.trim().is_empty()
        && is_sha256_hex(&artifact.sha256)
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == SHA256_HEX_LENGTH && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material_profile::{
        MaterialEvidenceClassV1, MaterialPropertyV1, MaterialQuantityV1, RockLithologyV1,
        RockMaterialConditionV1, RockMaterialProfileV1,
    };
    use crate::world_contract::{
        CoordinateFrameV1, FrameTransformV1, PhysicsFidelity, SolverDeclarationV1,
        WORLD_MANIFEST_SCHEMA_VERSION,
    };

    fn artifact(id: &str, digest_char: char) -> ArtifactRef {
        ArtifactRef {
            artifact_id: id.to_owned(),
            uri: format!("urn:symtropy:synthetic:{id}"),
            sha256: digest_char.to_string().repeat(SHA256_HEX_LENGTH),
        }
    }

    fn world() -> WorldManifestV1 {
        WorldManifestV1 {
            schema_version: WORLD_MANIFEST_SCHEMA_VERSION,
            scenario_id: "synthetic-excavation-contract-test".to_owned(),
            seed: 9,
            fixed_step_ns: 1_000_000,
            gravity_mps2: [0.0, -9.81, 0.0],
            frames: vec![CoordinateFrameV1 {
                frame_id: "world".to_owned(),
                parent_frame_id: None,
                transform_to_parent: FrameTransformV1::Root,
            }],
            fidelity: PhysicsFidelity::DeterministicReference,
            world_assets: vec![
                artifact("disc-cutter-001", '9'),
                artifact("disc-cutter-geometry", 'a'),
                artifact("rock-face-geometry", '8'),
            ],
            material_profiles: vec![RockMaterialProfileV1 {
                profile_id: "granite-synthetic-001".to_owned(),
                lithology: RockLithologyV1::Granite,
                display_label: "Synthetic granite contract fixture".to_owned(),
                condition: RockMaterialConditionV1 {
                    condition_id: "synthetic-condition-v1".to_owned(),
                    temperature_k: Some(293.15),
                    water_saturation_fraction: Some(0.0),
                    confining_pressure_pa: Some(0.0),
                    orientation_description: None,
                    alteration_description: None,
                },
                properties: vec![MaterialPropertyV1 {
                    quantity: MaterialQuantityV1::BulkDensity,
                    value_si: 2600.0,
                    unit: "kg/m^3".to_owned(),
                    uncertainty_std_si: None,
                    evidence_class: MaterialEvidenceClassV1::SyntheticScenario,
                    source: None,
                    sample_or_site_id: None,
                }],
                notes: Some("Contract fixture only; not measured granite.".to_owned()),
            }],
            solvers: vec![SolverDeclarationV1 {
                solver_id: "reduced-order-cutter-v1".to_owned(),
                domain: PhysicsDomain::MechanicalCutting,
                implementation: "synthetic-contract-fixture".to_owned(),
                version: "0.1.0".to_owned(),
                build_sha256: "b".repeat(SHA256_HEX_LENGTH),
                parameters_sha256: "c".repeat(SHA256_HEX_LENGTH),
                output_contract_version: 1,
                fidelity: PhysicsFidelity::DeterministicReference,
            }],
            sensors: Vec::new(),
            platforms: Vec::new(),
        }
    }

    fn request() -> MaterialAwareExcavationRequestV1 {
        MaterialAwareExcavationRequestV1 {
            schema_version: MATERIAL_AWARE_EXCAVATION_SCHEMA_VERSION,
            request_id: "excavation-request-001".to_owned(),
            world_generation: 7,
            world_manifest_sha256: "d".repeat(SHA256_HEX_LENGTH),
            input_digest_sha256: "e".repeat(SHA256_HEX_LENGTH),
            material_profile_id: "granite-synthetic-001".to_owned(),
            target_geometry_artifact: artifact("rock-face-geometry", '8'),
            target_frame_id: "world".to_owned(),
            solver_id: "reduced-order-cutter-v1".to_owned(),
            tool: ExcavationToolProfileV1 {
                tool_id: "disc-cutter-001".to_owned(),
                profile_artifact: artifact("disc-cutter-001", '9'),
                geometry_artifact: artifact("disc-cutter-geometry", 'a'),
                feed_speed_limit_m_s: 0.005,
                angular_speed_limit_rad_s: 150.0,
                power_limit_w: 2_500.0,
                thrust_limit_n: 1_000.0,
                torque_limit_nm: 25.0,
            },
            feed_speed_m_s: 0.001,
            angular_speed_rad_s: 100.0,
            requested_duration_ns: 1_000_000_000,
        }
    }

    fn response() -> ExcavationResponseV1 {
        ExcavationResponseV1 {
            removed_solid_rock_volume_m3: 1.0e-5,
            mechanical_energy_j: 2_001.0,
            peak_thrust_n: 900.0,
            peak_torque_nm: 20.0,
            wear_delta_fraction: 0.01,
            heat_energy_j: 400.0,
            loose_spoil_volume_m3: 1.2e-5,
            removed_solid_rock_volume_standard_uncertainty_m3: Some(1.0e-7),
            mechanical_energy_standard_uncertainty_j: Some(10.0),
        }
    }

    fn result(disposition: ExcavationDispositionV1) -> MaterialAwareExcavationResultV1 {
        let completed = matches!(
            disposition,
            ExcavationDispositionV1::CompletedWithinEnvelope
                | ExcavationDispositionV1::CompletedWithWarnings
        );
        MaterialAwareExcavationResultV1 {
            schema_version: MATERIAL_AWARE_EXCAVATION_SCHEMA_VERSION,
            request: request(),
            disposition,
            result_artifact: completed.then(|| artifact("excavation-output-001", 'f')),
            response: completed.then(response),
            spatial_delta: completed.then(|| ExcavationSpatialDeltaV1 {
                base_world_generation: 7,
                result_world_generation: 8,
                base_geometry_artifact: artifact("rock-face-geometry", '8'),
                result_geometry_artifact: artifact("rock-face-after-cut-001", '6'),
                state_delta_artifact: artifact("rock-face-delta-001", '7'),
                state_delta_format: ExcavationStateDeltaFormatV1::CellRemovalMask,
                state_delta_schema_version: 1,
                coordinate_frame_id: "world".to_owned(),
            }),
            warnings: if disposition == ExcavationDispositionV1::CompletedWithWarnings {
                vec!["Synthetic warning fixture".to_owned()]
            } else {
                Vec::new()
            },
        }
    }

    #[test]
    fn request_requires_declared_material_tool_geometry_and_solver() {
        assert!(request()
            .validate_against_world(&world(), &"d".repeat(SHA256_HEX_LENGTH))
            .is_ok());
        let mut unknown_material = request();
        unknown_material.material_profile_id = "unlisted-basalt-profile".to_owned();
        assert_eq!(
            unknown_material.validate_against_world(&world(), &"d".repeat(SHA256_HEX_LENGTH)),
            Err(ExcavationContractErrorV1::UnknownMaterialProfile(
                "unlisted-basalt-profile".to_owned()
            ))
        );
        let mut unknown_tool = request();
        unknown_tool.tool.profile_artifact = artifact("unregistered-cutter", '9');
        unknown_tool.tool.tool_id = "unregistered-cutter".to_owned();
        assert_eq!(
            unknown_tool.validate_against_world(&world(), &"d".repeat(SHA256_HEX_LENGTH)),
            Err(ExcavationContractErrorV1::UndeclaredToolProfileArtifact)
        );

        let mut mismatched_tool = request();
        mismatched_tool.tool.profile_artifact = artifact("unregistered-cutter", '9');
        assert_eq!(
            mismatched_tool.validate_against_world(&world(), &"d".repeat(SHA256_HEX_LENGTH)),
            Err(ExcavationContractErrorV1::ToolProfileIdentityMismatch)
        );

        let mut unknown_geometry = request();
        unknown_geometry.tool.geometry_artifact = artifact("undeclared-tool-geometry", 'a');
        assert_eq!(
            unknown_geometry.validate_against_world(&world(), &"d".repeat(SHA256_HEX_LENGTH)),
            Err(ExcavationContractErrorV1::UndeclaredToolGeometryArtifact)
        );

        let mut unknown_target_geometry = request();
        unknown_target_geometry.target_geometry_artifact = artifact("undeclared-rock-face", '8');
        assert_eq!(
            unknown_target_geometry
                .validate_against_world(&world(), &"d".repeat(SHA256_HEX_LENGTH)),
            Err(ExcavationContractErrorV1::UndeclaredTargetGeometryArtifact)
        );

        let mut unknown_frame = request();
        unknown_frame.target_frame_id = "unregistered-frame".to_owned();
        assert_eq!(
            unknown_frame.validate_against_world(&world(), &"d".repeat(SHA256_HEX_LENGTH)),
            Err(ExcavationContractErrorV1::UnknownTargetFrame(
                "unregistered-frame".to_owned()
            ))
        );

        let mut excessive_feed = request();
        excessive_feed.feed_speed_m_s = 0.01;
        assert_eq!(
            excessive_feed.validate_against_world(&world(), &"d".repeat(SHA256_HEX_LENGTH)),
            Err(ExcavationContractErrorV1::OperatingEnvelopeExceeded("feed_speed_m_s"))
        );

        let mut excessive_rotation = request();
        excessive_rotation.angular_speed_rad_s = 200.0;
        assert_eq!(
            excessive_rotation.validate_against_world(&world(), &"d".repeat(SHA256_HEX_LENGTH)),
            Err(ExcavationContractErrorV1::OperatingEnvelopeExceeded("angular_speed_rad_s"))
        );
    }

    #[test]
    fn request_rejects_mismatched_canonical_world_digest() {
        assert_eq!(
            request().validate_against_world(&world(), &"0".repeat(SHA256_HEX_LENGTH)),
            Err(ExcavationContractErrorV1::WorldManifestDigestMismatch)
        );
    }

    fn valid_cell_removal_mask() -> CellRemovalMaskV1 {
        CellRemovalMaskV1 {
            source_artifact: artifact("rock-face-delta-001", '7'),
            payload: CellRemovalMaskPayloadV1 {
                schema_version: CELL_REMOVAL_MASK_SCHEMA_VERSION,
                base_world_generation: 7,
                base_geometry_artifact: artifact("rock-face-geometry", '8'),
                material_profile_id: "granite-synthetic-001".to_owned(),
                coordinate_frame_id: "world".to_owned(),
                grid_origin_m: [0.0, 0.0, 0.0],
                cell_size_m: [0.1, 0.1, 0.1],
                grid_dimensions: [10, 8, 6],
                removed_cells: vec![[1, 2, 3], [1, 2, 4], [2, 2, 4]],
            },
        }
    }

    #[test]
    fn cell_mask_json_decoder_verifies_exact_artifact_bytes() {
        use sha2::{Digest, Sha256};

        let payload = valid_cell_removal_mask().payload;
        let bytes = serde_json::to_vec(&payload).unwrap();
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let source_artifact = ArtifactRef {
            artifact_id: "rock-face-delta-001".to_owned(),
            uri: "urn:symtropy:synthetic:rock-face-delta-001".to_owned(),
            sha256: digest,
        };
        let mut delta = result(ExcavationDispositionV1::CompletedWithinEnvelope)
            .spatial_delta
            .unwrap();
        delta.state_delta_artifact = source_artifact.clone();
        let decoded =
            CellRemovalMaskV1::decode_verified_json(&bytes, source_artifact.clone()).unwrap();
        assert_eq!(decoded.payload, payload);
        assert!(decoded
            .validate_against(
                &world(),
                &"d".repeat(SHA256_HEX_LENGTH),
                &request(),
                &delta
            )
            .is_ok());

        let mut tampered = bytes.clone();
        tampered.push(b' ');
        assert_eq!(
            CellRemovalMaskV1::decode_verified_json(&tampered, source_artifact),
            Err(ExcavationContractErrorV1::CellMaskPayloadDigestMismatch)
        );
    }

    #[test]
    fn cell_mask_json_decoder_rejects_unknown_fields() {
        use sha2::{Digest, Sha256};

        let bytes = br#"{"schema_version":1,"unknown":true}"#;
        let digest = format!("{:x}", Sha256::digest(bytes));
        let source_artifact = ArtifactRef {
            artifact_id: "mask-payload".to_owned(),
            uri: "urn:symtropy:test:mask-payload".to_owned(),
            sha256: digest,
        };
        assert_eq!(
            CellRemovalMaskV1::decode_verified_json(bytes, source_artifact),
            Err(ExcavationContractErrorV1::InvalidCellMaskPayload)
        );
    }

    #[test]
    fn cell_mask_accepts_canonical_in_bounds_patch() {
        let world = world();
        let request = request();
        let delta = result(ExcavationDispositionV1::CompletedWithinEnvelope)
            .spatial_delta
            .unwrap();
        assert!(valid_cell_removal_mask()
            .validate_against(
                &world,
                &"d".repeat(SHA256_HEX_LENGTH),
                &request,
                &delta
            )
            .is_ok());
    }

    #[test]
    fn cell_mask_rejects_duplicate_or_noncanonical_cell_order() {
        let world = world();
        let request = request();
        let delta = result(ExcavationDispositionV1::CompletedWithinEnvelope)
            .spatial_delta
            .unwrap();
        let mut mask = valid_cell_removal_mask();
        mask.payload.removed_cells = vec![[1, 2, 3], [1, 2, 3]];
        assert_eq!(
            mask.validate_against(&world, &"d".repeat(SHA256_HEX_LENGTH), &request, &delta),
            Err(ExcavationContractErrorV1::CellIndicesNotCanonical)
        );
        mask.payload.removed_cells = vec![[2, 2, 4], [1, 2, 3]];
        assert_eq!(
            mask.validate_against(&world, &"d".repeat(SHA256_HEX_LENGTH), &request, &delta),
            Err(ExcavationContractErrorV1::CellIndicesNotCanonical)
        );
    }

    #[test]
    fn cell_mask_rejects_out_of_bounds_cells_and_invalid_grid() {
        let world = world();
        let request = request();
        let delta = result(ExcavationDispositionV1::CompletedWithinEnvelope)
            .spatial_delta
            .unwrap();
        let mut mask = valid_cell_removal_mask();
        mask.payload.removed_cells = vec![[10, 0, 0]];
        assert_eq!(
            mask.validate_against(&world, &"d".repeat(SHA256_HEX_LENGTH), &request, &delta),
            Err(ExcavationContractErrorV1::CellIndexOutOfBounds([10, 0, 0]))
        );
        mask = valid_cell_removal_mask();
        mask.payload.cell_size_m[1] = f64::NAN;
        assert_eq!(
            mask.validate_against(&world, &"d".repeat(SHA256_HEX_LENGTH), &request, &delta),
            Err(ExcavationContractErrorV1::InvalidCellGrid)
        );
    }

    #[test]
    fn cell_mask_must_match_delta_artifact_geometry_material_and_frame() {
        let world = world();
        let request = request();
        let delta = result(ExcavationDispositionV1::CompletedWithinEnvelope)
            .spatial_delta
            .unwrap();
        let mut mask = valid_cell_removal_mask();
        mask.source_artifact.sha256 = "0".repeat(SHA256_HEX_LENGTH);
        assert_eq!(
            mask.validate_against(&world, &"d".repeat(SHA256_HEX_LENGTH), &request, &delta),
            Err(ExcavationContractErrorV1::CellMaskArtifactMismatch)
        );
        mask = valid_cell_removal_mask();
        mask.payload.material_profile_id = "different-material".to_owned();
        assert_eq!(
            mask.validate_against(&world, &"d".repeat(SHA256_HEX_LENGTH), &request, &delta),
            Err(ExcavationContractErrorV1::CellMaskMaterialMismatch)
        );
    }

    #[test]
    fn completed_result_requires_spatial_delta_for_exact_geometry_and_generation() {
        let world = world();
        let expected = request();
        let mut missing_delta = result(ExcavationDispositionV1::CompletedWithinEnvelope);
        missing_delta.spatial_delta = None;
        assert_eq!(
            missing_delta.validate_against(&world, &"d".repeat(SHA256_HEX_LENGTH), &expected),
            Err(ExcavationContractErrorV1::InvalidSpatialDelta)
        );

        let mut stale_geometry = result(ExcavationDispositionV1::CompletedWithinEnvelope);
        stale_geometry.spatial_delta.as_mut().unwrap().base_geometry_artifact =
            artifact("different-rock-face", '8');
        assert_eq!(
            stale_geometry.validate_against(&world, &"d".repeat(SHA256_HEX_LENGTH), &expected),
            Err(ExcavationContractErrorV1::InvalidSpatialDelta)
        );

        let mut skipped_generation = result(ExcavationDispositionV1::CompletedWithinEnvelope);
        skipped_generation.spatial_delta.as_mut().unwrap().result_world_generation = 9;
        assert_eq!(
            skipped_generation.validate_against(&world, &"d".repeat(SHA256_HEX_LENGTH), &expected),
            Err(ExcavationContractErrorV1::InvalidSpatialDelta)
        );

        let mut bad_delta_digest = result(ExcavationDispositionV1::CompletedWithinEnvelope);
        bad_delta_digest.spatial_delta.as_mut().unwrap().state_delta_artifact.sha256 =
            "not-a-sha256".to_owned();
        assert_eq!(
            bad_delta_digest.validate_against(&world, &"d".repeat(SHA256_HEX_LENGTH), &expected),
            Err(ExcavationContractErrorV1::InvalidSpatialDelta)
        );

        let mut wrong_frame = result(ExcavationDispositionV1::CompletedWithinEnvelope);
        wrong_frame.spatial_delta.as_mut().unwrap().coordinate_frame_id =
            "unregistered-frame".to_owned();
        assert_eq!(
            wrong_frame.validate_against(
                &world,
                &"d".repeat(SHA256_HEX_LENGTH),
                &expected
            ),
            Err(ExcavationContractErrorV1::InvalidSpatialDelta)
        );

        for unsupported_version in [0, 2] {
            let mut unsupported_delta_schema =
                result(ExcavationDispositionV1::CompletedWithinEnvelope);
            unsupported_delta_schema
                .spatial_delta
                .as_mut()
                .unwrap()
                .state_delta_schema_version = unsupported_version;
            assert_eq!(
                unsupported_delta_schema.validate_against(
                    &world,
                    &"d".repeat(SHA256_HEX_LENGTH),
                    &expected
                ),
                Err(ExcavationContractErrorV1::InvalidSpatialDelta)
            );
        }
    }

    #[test]
    fn failed_result_cannot_carry_spatial_state_change() {
        let mut failed = result(ExcavationDispositionV1::OutOfDomain);
        failed.spatial_delta = Some(ExcavationSpatialDeltaV1 {
            base_world_generation: 7,
            result_world_generation: 8,
            base_geometry_artifact: artifact("rock-face-geometry", '8'),
            result_geometry_artifact: artifact("rock-face-after-cut-001", '6'),
            state_delta_artifact: artifact("rock-face-delta-001", '7'),
            state_delta_format: ExcavationStateDeltaFormatV1::CellRemovalMask,
            state_delta_schema_version: 1,
            coordinate_frame_id: "world".to_owned(),
        });
        assert_eq!(
            failed.validate_against(&world(), &"d".repeat(SHA256_HEX_LENGTH), &request()),
            Err(ExcavationContractErrorV1::InconsistentDisposition)
        );
    }

    #[test]
    fn completed_result_is_bound_to_exact_request_and_generation() {
        let world = world();
        let expected = request();
        assert!(result(ExcavationDispositionV1::CompletedWithinEnvelope)
            .validate_against(&world, &"d".repeat(SHA256_HEX_LENGTH), &expected)
            .is_ok());
        let mut stale = result(ExcavationDispositionV1::CompletedWithinEnvelope);
        stale.request.world_generation -= 1;
        assert_eq!(
            stale.validate_against(&world, &"d".repeat(SHA256_HEX_LENGTH), &expected),
            Err(ExcavationContractErrorV1::RequestIdentityMismatch)
        );
    }

    #[test]
    fn noncompleted_result_cannot_claim_removed_volume() {
        let mut failed = result(ExcavationDispositionV1::OutOfDomain);
        failed.result_artifact = Some(artifact("solver-diagnostics-001", '8'));
        assert!(failed
            .validate_against(&world(), &"d".repeat(SHA256_HEX_LENGTH), &request())
            .is_ok());

        failed.response = Some(response());
        assert_eq!(
            failed.validate_against(&world(), &"d".repeat(SHA256_HEX_LENGTH), &request()),
            Err(ExcavationContractErrorV1::InconsistentDisposition)
        );
    }

    #[test]
    fn completed_result_rejects_nonfinite_or_negative_outputs() {
        let mut invalid = result(ExcavationDispositionV1::CompletedWithinEnvelope);
        invalid.response.as_mut().unwrap().removed_solid_rock_volume_m3 = f64::NAN;
        assert_eq!(
            invalid.validate_against(&world(), &"d".repeat(SHA256_HEX_LENGTH), &request()),
            Err(ExcavationContractErrorV1::InvalidOutput(
                "removed_solid_rock_volume_m3"
            ))
        );
    }

    #[test]
    fn normal_completion_cannot_hide_overlimit_loads() {
        let mut overlimit = result(ExcavationDispositionV1::CompletedWithinEnvelope);
        overlimit.response.as_mut().unwrap().peak_thrust_n = 1_200.0;
        assert_eq!(
            overlimit.validate_against(&world(), &"d".repeat(SHA256_HEX_LENGTH), &request()),
            Err(ExcavationContractErrorV1::OperatingEnvelopeExceeded(
                "peak_thrust_n"
            ))
        );

        let mut over_torque = result(ExcavationDispositionV1::CompletedWithinEnvelope);
        over_torque.response.as_mut().unwrap().peak_torque_nm = 30.0;
        assert_eq!(
            over_torque.validate_against(&world(), &"d".repeat(SHA256_HEX_LENGTH), &request()),
            Err(ExcavationContractErrorV1::OperatingEnvelopeExceeded(
                "peak_torque_nm"
            ))
        );

        let mut over_energy = result(ExcavationDispositionV1::CompletedWithinEnvelope);
        over_energy.response.as_mut().unwrap().mechanical_energy_j = 2_501.0;
        assert_eq!(
            over_energy.validate_against(&world(), &"d".repeat(SHA256_HEX_LENGTH), &request()),
            Err(ExcavationContractErrorV1::OperatingEnvelopeExceeded(
                "mechanical_energy_j"
            ))
        );

        overlimit.disposition = ExcavationDispositionV1::CompletedWithWarnings;
        overlimit.warnings = vec!["Peak thrust exceeded declared tool limit.".to_owned()];
        assert!(overlimit
            .validate_against(&world(), &"d".repeat(SHA256_HEX_LENGTH), &request())
            .is_ok());
    }

    #[test]
    fn warning_completion_requires_an_explicit_warning() {
        let mut warningless = result(ExcavationDispositionV1::CompletedWithWarnings);
        warningless.warnings.clear();
        assert_eq!(
            warningless.validate_against(&world(), &"d".repeat(SHA256_HEX_LENGTH), &request()),
            Err(ExcavationContractErrorV1::WarningStatusWithoutWarning)
        );
    }
}
