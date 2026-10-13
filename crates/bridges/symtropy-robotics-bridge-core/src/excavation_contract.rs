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

pub const MATERIAL_AWARE_EXCAVATION_SCHEMA_VERSION: u16 = 1;
const SHA256_HEX_LENGTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExcavationToolProfileV1 {
    pub tool_id: String,
    /// The exact geometry artifact must also be declared by the world's assets.
    pub geometry_artifact: ArtifactRef,
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
    /// Digest-bound solver/measurement result bundle; required only for completed outputs.
    pub output_artifact: Option<ArtifactRef>,
    /// Non-completed dispositions must not carry accepted physical outputs.
    pub response: Option<ExcavationResponseV1>,
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
    UnknownMaterialProfile(String),
    UnknownSolver(String),
    UnsupportedSolverDomain(PhysicsDomain),
    InvalidCommand(&'static str),
    RequestIdentityMismatch,
    InvalidOutputArtifact,
    InconsistentDisposition,
    InvalidOutput(&'static str),
    OperatingEnvelopeExceeded(&'static str),
    WarningStatusWithoutWarning,
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
            ("power_limit_w", self.tool.power_limit_w),
            ("thrust_limit_n", self.tool.thrust_limit_n),
            ("torque_limit_nm", self.tool.torque_limit_nm),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(ExcavationContractErrorV1::InvalidCommand(field));
            }
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
                    .output_artifact
                    .as_ref()
                    .ok_or(ExcavationContractErrorV1::InvalidOutputArtifact)?;
                if !valid_artifact(artifact) {
                    return Err(ExcavationContractErrorV1::InvalidOutputArtifact);
                }
                let response = self
                    .response
                    .as_ref()
                    .ok_or(ExcavationContractErrorV1::InconsistentDisposition)?;
                validate_response(response)?;
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
                if self.output_artifact.is_some() || self.response.is_some() {
                    return Err(ExcavationContractErrorV1::InconsistentDisposition);
                }
            }
        }
        Ok(())
    }
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
        MaterialEvidenceClassV1, MaterialPropertyV1, MaterialQuantityV1,
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
            world_assets: vec![artifact("disc-cutter-geometry", 'a')],
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
            solver_id: "reduced-order-cutter-v1".to_owned(),
            tool: ExcavationToolProfileV1 {
                tool_id: "disc-cutter-001".to_owned(),
                geometry_artifact: artifact("disc-cutter-geometry", 'a'),
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
            output_artifact: completed.then(|| artifact("excavation-output-001", 'f')),
            response: completed.then(response),
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
        unknown_tool.tool.geometry_artifact = artifact("undeclared-tool", 'a');
        assert_eq!(
            unknown_tool.validate_against_world(&world(), &"d".repeat(SHA256_HEX_LENGTH)),
            Err(ExcavationContractErrorV1::UndeclaredToolGeometryArtifact)
        );
    }

    #[test]
    fn request_rejects_mismatched_canonical_world_digest() {
        assert_eq!(
            request().validate_against_world(&world(), &"0".repeat(SHA256_HEX_LENGTH)),
            Err(ExcavationContractErrorV1::WorldManifestDigestMismatch)
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
