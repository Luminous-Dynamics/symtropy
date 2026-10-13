// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Traceable mechanical-specific-energy (MSE) calculations for rock-cutting
//! measurements.
//!
//! This module calculates a descriptive measurement metric. It is not a
//! rock-strength estimator, cutting constitutive law, or predictive model.
//! Accuracy depends on power measurement boundaries and removed-volume data.

use crate::world_contract::{ArtifactRef, WorldManifestV1, WorldValidationIssue};
use serde::{Deserialize, Serialize};
use std::fmt;

pub const CUTTING_METRIC_SCHEMA_VERSION: u16 = 1;
const SHA256_HEX_LENGTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CuttingPowerInputV1 {
    /// Mechanical power measured at the tool interface, in watts.
    ///
    /// Electrical input power must not be labelled as tool-interface power
    /// without an explicit validated efficiency/loss model.
    MeasuredToolInterfacePower {
        mechanical_power_w: f64,
    },
    /// Power estimated from magnitudes measured at the tool interface.
    ///
    /// Rotary power = torque magnitude × angular-speed magnitude.
    /// Axial power = thrust magnitude × feed-speed magnitude.
    ///
    /// Torque and force conventions must refer to the tool interface; using
    /// upstream motor or surface measurements can include drivetrain losses.
    DerivedFromToolLoads {
        rotary_torque_magnitude_nm: f64,
        angular_speed_magnitude_rad_s: f64,
        axial_force_magnitude_n: f64,
        feed_speed_magnitude_m_s: f64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RockCuttingObservationV1 {
    pub schema_version: u16,
    pub observation_id: String,
    pub material_profile_id: String,
    pub tool_id: String,
    /// Rate of removed solid-rock volume, m^3/s. This must be based on a
    /// declared measurement/estimation method and must exclude void/spoil
    /// swelling unless a conversion is explicitly documented.
    pub removed_volume_rate_m3_s: f64,
    /// Duration represented by this observation. Zero-length windows are not
    /// admitted even when the sensor reports an instantaneous value.
    pub measurement_interval_ns: u64,
    pub power_input: CuttingPowerInputV1,
    /// Digest-bound raw telemetry, calibration record, or measurement bundle.
    pub source: ArtifactRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CuttingPowerBasisV1 {
    MeasuredToolInterfacePower,
    DerivedFromToolLoads,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MechanicalSpecificEnergyV1 {
    pub schema_version: u16,
    pub observation_id: String,
    pub material_profile_id: String,
    pub tool_id: String,
    pub power_basis: CuttingPowerBasisV1,
    pub mechanical_power_w: f64,
    pub rotary_power_w: Option<f64>,
    pub axial_power_w: Option<f64>,
    pub removed_volume_rate_m3_s: f64,
    /// Mechanical energy per unit removed volume, J/m^3 == Pa.
    pub mse_pa: f64,
    pub source: ArtifactRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CuttingMetricError {
    UnsupportedSchemaVersion,
    EmptyIdentity(&'static str),
    InvalidSourceArtifact,
    InvalidSourceDigest,
    InvalidMeasurementInterval,
    InvalidRemovedVolumeRate,
    NonFiniteInput(&'static str),
    NegativeMagnitude(&'static str),
    ComputedPowerNotFinite,
    MseNotFinite,
    /// The measurement names a material profile not declared by this world.
    UnknownMaterialProfile(String),
    /// The world did not pass structural validation before admitting this result.
    InvalidWorldManifest(Vec<WorldValidationIssue>),
}

impl fmt::Display for CuttingMetricError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion => {
                formatter.write_str("unsupported rock-cutting observation schema version")
            }
            Self::EmptyIdentity(field) => write!(formatter, "{field} must not be empty"),
            Self::InvalidSourceArtifact => {
                formatter.write_str("source artifact identity and URI must not be empty")
            }
            Self::InvalidSourceDigest => {
                formatter.write_str("source artifact SHA-256 must be 64 hexadecimal digits")
            }
            Self::InvalidMeasurementInterval => {
                formatter.write_str("measurement interval must be greater than zero")
            }
            Self::InvalidRemovedVolumeRate => formatter.write_str(
                "removed rock-volume rate must be finite and strictly greater than zero",
            ),
            Self::NonFiniteInput(field) => write!(formatter, "{field} must be finite"),
            Self::NegativeMagnitude(field) => {
                write!(formatter, "{field} must be a non-negative magnitude")
            }
            Self::ComputedPowerNotFinite => {
                formatter.write_str("computed mechanical power is not finite")
            }
            Self::MseNotFinite => formatter.write_str("mechanical specific energy is not finite"),
            Self::UnknownMaterialProfile(profile_id) => {
                write!(formatter, "material profile {profile_id:?} is not declared by the world")
            }
            Self::InvalidWorldManifest(issues) => write!(
                formatter,
                "world manifest failed admission validation with {} issue(s)",
                issues.len()
            ),
        }
    }
}

impl std::error::Error for CuttingMetricError {}

impl RockCuttingObservationV1 {
    /// Calculate this observation's MSE only after admitting it against a valid world.
    ///
    /// This is the preferred boundary for simulation use: a mathematically valid
    /// observation is not admitted unless its material profile is declared by the
    /// exact world manifest supplied by the caller. The world is structurally
    /// validated first; that validation does not certify physical fidelity or the
    /// truth of any measurement. Tool identity remains measurement provenance and
    /// must be checked against the owning tool/platform registry by the caller.
    pub fn calculate_mse_in_world(
        &self,
        world: &WorldManifestV1,
    ) -> Result<MechanicalSpecificEnergyV1, CuttingMetricError> {
        world
            .validate()
            .map_err(CuttingMetricError::InvalidWorldManifest)?;
        if !world
            .material_profiles
            .iter()
            .any(|profile| profile.profile_id == self.material_profile_id)
        {
            return Err(CuttingMetricError::UnknownMaterialProfile(
                self.material_profile_id.clone(),
            ));
        }
        self.calculate_mse()
    }

    /// Calculate MSE from measured input. Validation rejects missing provenance,
    /// invalid units by convention, non-finite/negative magnitudes, and a zero
    /// removed-volume rate. No rock material constants are invented here.
    pub fn calculate_mse(&self) -> Result<MechanicalSpecificEnergyV1, CuttingMetricError> {
        if self.schema_version != CUTTING_METRIC_SCHEMA_VERSION {
            return Err(CuttingMetricError::UnsupportedSchemaVersion);
        }
        if self.observation_id.trim().is_empty() {
            return Err(CuttingMetricError::EmptyIdentity("observation_id"));
        }
        if self.material_profile_id.trim().is_empty() {
            return Err(CuttingMetricError::EmptyIdentity("material_profile_id"));
        }
        if self.tool_id.trim().is_empty() {
            return Err(CuttingMetricError::EmptyIdentity("tool_id"));
        }
        if self.measurement_interval_ns == 0 {
            return Err(CuttingMetricError::InvalidMeasurementInterval);
        }
        if !self.removed_volume_rate_m3_s.is_finite()
            || self.removed_volume_rate_m3_s <= 0.0
        {
            return Err(CuttingMetricError::InvalidRemovedVolumeRate);
        }
        if self.source.artifact_id.trim().is_empty() || self.source.uri.trim().is_empty() {
            return Err(CuttingMetricError::InvalidSourceArtifact);
        }
        if !is_sha256_hex(&self.source.sha256) {
            return Err(CuttingMetricError::InvalidSourceDigest);
        }

        let (power_basis, mechanical_power_w, rotary_power_w, axial_power_w) =
            match self.power_input {
                CuttingPowerInputV1::MeasuredToolInterfacePower { mechanical_power_w } => {
                    validate_magnitude(mechanical_power_w, "mechanical_power_w")?;
                    (
                        CuttingPowerBasisV1::MeasuredToolInterfacePower,
                        mechanical_power_w,
                        None,
                        None,
                    )
                }
                CuttingPowerInputV1::DerivedFromToolLoads {
                    rotary_torque_magnitude_nm,
                    angular_speed_magnitude_rad_s,
                    axial_force_magnitude_n,
                    feed_speed_magnitude_m_s,
                } => {
                    validate_magnitude(
                        rotary_torque_magnitude_nm,
                        "rotary_torque_magnitude_nm",
                    )?;
                    validate_magnitude(
                        angular_speed_magnitude_rad_s,
                        "angular_speed_magnitude_rad_s",
                    )?;
                    validate_magnitude(
                        axial_force_magnitude_n,
                        "axial_force_magnitude_n",
                    )?;
                    validate_magnitude(feed_speed_magnitude_m_s, "feed_speed_magnitude_m_s")?;

                    let rotary = rotary_torque_magnitude_nm * angular_speed_magnitude_rad_s;
                    let axial = axial_force_magnitude_n * feed_speed_magnitude_m_s;
                    let total = rotary + axial;
                    if !rotary.is_finite() || !axial.is_finite() || !total.is_finite() {
                        return Err(CuttingMetricError::ComputedPowerNotFinite);
                    }
                    (
                        CuttingPowerBasisV1::DerivedFromToolLoads,
                        total,
                        Some(rotary),
                        Some(axial),
                    )
                }
            };

        let mse_pa = mechanical_power_w / self.removed_volume_rate_m3_s;
        if !mse_pa.is_finite() {
            return Err(CuttingMetricError::MseNotFinite);
        }

        Ok(MechanicalSpecificEnergyV1 {
            schema_version: CUTTING_METRIC_SCHEMA_VERSION,
            observation_id: self.observation_id.clone(),
            material_profile_id: self.material_profile_id.clone(),
            tool_id: self.tool_id.clone(),
            power_basis,
            mechanical_power_w,
            rotary_power_w,
            axial_power_w,
            removed_volume_rate_m3_s: self.removed_volume_rate_m3_s,
            mse_pa,
            source: self.source.clone(),
        })
    }
}

fn validate_magnitude(value: f64, name: &'static str) -> Result<(), CuttingMetricError> {
    if !value.is_finite() {
        return Err(CuttingMetricError::NonFiniteInput(name));
    }
    if value < 0.0 {
        return Err(CuttingMetricError::NegativeMagnitude(name));
    }
    Ok(())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == SHA256_HEX_LENGTH && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material_profile::{RockLithologyV1, RockMaterialConditionV1, RockMaterialProfileV1};
    use crate::world_contract::{
        CoordinateFrameV1, FrameTransformV1, PhysicsFidelity, WORLD_MANIFEST_SCHEMA_VERSION,
    };

    fn world_with_profiles(profile_ids: &[&str]) -> WorldManifestV1 {
        WorldManifestV1 {
            schema_version: WORLD_MANIFEST_SCHEMA_VERSION,
            scenario_id: "mse-admission-test-v1".to_owned(),
            seed: 17,
            fixed_step_ns: 1_000_000,
            gravity_mps2: [0.0, -9.81, 0.0],
            frames: vec![CoordinateFrameV1 {
                frame_id: "world".to_owned(),
                parent_frame_id: None,
                transform_to_parent: FrameTransformV1::Root,
            }],
            fidelity: PhysicsFidelity::DeterministicReference,
            world_assets: Vec::new(),
            material_profiles: profile_ids
                .iter()
                .map(|profile_id| RockMaterialProfileV1 {
                    profile_id: (*profile_id).to_owned(),
                    lithology: RockLithologyV1::Granite,
                    display_label: format!("Synthetic fixture {profile_id}"),
                    condition: RockMaterialConditionV1 {
                        condition_id: "synthetic-condition-v1".to_owned(),
                        temperature_k: Some(293.15),
                        water_saturation_fraction: Some(0.0),
                        confining_pressure_pa: Some(0.0),
                        orientation_description: None,
                        alteration_description: None,
                    },
                    properties: Vec::new(),
                    notes: Some("Synthetic schema fixture only; not physical granite data.".to_owned()),
                })
                .collect(),
            solvers: Vec::new(),
            sensors: Vec::new(),
            platforms: Vec::new(),
        }
    }

    fn source() -> ArtifactRef {
        ArtifactRef {
            artifact_id: "rock-cutting-sensor-bundle-001".to_owned(),
            uri: "urn:symtropy:test:rock-cutting-sensor-bundle-001".to_owned(),
            sha256: "a".repeat(SHA256_HEX_LENGTH),
        }
    }

    fn observation(power_input: CuttingPowerInputV1) -> RockCuttingObservationV1 {
        RockCuttingObservationV1 {
            schema_version: CUTTING_METRIC_SCHEMA_VERSION,
            observation_id: "cut-test-001".to_owned(),
            material_profile_id: "granite-lab-001".to_owned(),
            tool_id: "disc-cutter-01".to_owned(),
            removed_volume_rate_m3_s: 1.0e-5,
            measurement_interval_ns: 1_000_000_000,
            power_input,
            source: source(),
        }
    }

    #[test]
    fn world_admission_accepts_only_a_declared_material_profile() {
        let observation = observation(CuttingPowerInputV1::MeasuredToolInterfacePower {
            mechanical_power_w: 1_500.0,
        });
        let world = world_with_profiles(&["granite-lab-001"]);
        let result = observation.calculate_mse_in_world(&world).unwrap();
        assert_eq!(result.material_profile_id, "granite-lab-001");
        assert_eq!(result.mse_pa, 150_000_000.0);
    }

    #[test]
    fn world_admission_rejects_an_undeclared_material_profile() {
        let observation = observation(CuttingPowerInputV1::MeasuredToolInterfacePower {
            mechanical_power_w: 1_500.0,
        });
        let world = world_with_profiles(&["basalt-lab-001"]);
        assert_eq!(
            observation.calculate_mse_in_world(&world),
            Err(CuttingMetricError::UnknownMaterialProfile(
                "granite-lab-001".to_owned()
            ))
        );
    }

    #[test]
    fn world_admission_rejects_a_structurally_invalid_world() {
        let observation = observation(CuttingPowerInputV1::MeasuredToolInterfacePower {
            mechanical_power_w: 1_500.0,
        });
        let mut world = world_with_profiles(&["granite-lab-001"]);
        world.scenario_id.clear();
        assert!(matches!(
            observation.calculate_mse_in_world(&world),
            Err(CuttingMetricError::InvalidWorldManifest(issues))
                if issues.iter().any(|issue| issue.path == "scenario_id")
        ));
    }

    #[test]
    fn derived_mse_accounts_for_rotary_and_axial_power() {
        let observation = observation(CuttingPowerInputV1::DerivedFromToolLoads {
            rotary_torque_magnitude_nm: 20.0,
            angular_speed_magnitude_rad_s: 100.0,
            axial_force_magnitude_n: 1_000.0,
            feed_speed_magnitude_m_s: 0.001,
        });
        let result = observation.calculate_mse().unwrap();
        assert_eq!(result.rotary_power_w, Some(2_000.0));
        assert_eq!(result.axial_power_w, Some(1.0));
        assert_eq!(result.mechanical_power_w, 2_001.0);
        assert_eq!(result.mse_pa, 200_100_000.0);
        assert_eq!(
            result.power_basis,
            CuttingPowerBasisV1::DerivedFromToolLoads
        );
    }

    #[test]
    fn measured_power_path_does_not_fabricate_a_power_breakdown() {
        let observation = observation(CuttingPowerInputV1::MeasuredToolInterfacePower {
            mechanical_power_w: 1_500.0,
        });
        let result = observation.calculate_mse().unwrap();
        assert_eq!(result.mse_pa, 150_000_000.0);
        assert_eq!(result.rotary_power_w, None);
        assert_eq!(result.axial_power_w, None);
    }

    #[test]
    fn zero_removed_volume_rate_is_rejected_instead_of_returning_infinite_mse() {
        let mut observation =
            observation(CuttingPowerInputV1::MeasuredToolInterfacePower {
                mechanical_power_w: 1_500.0,
            });
        observation.removed_volume_rate_m3_s = 0.0;
        assert_eq!(
            observation.calculate_mse(),
            Err(CuttingMetricError::InvalidRemovedVolumeRate)
        );
    }

    #[test]
    fn non_finite_and_negative_load_magnitudes_are_rejected() {
        let non_finite = observation(CuttingPowerInputV1::DerivedFromToolLoads {
            rotary_torque_magnitude_nm: f64::INFINITY,
            angular_speed_magnitude_rad_s: 1.0,
            axial_force_magnitude_n: 1.0,
            feed_speed_magnitude_m_s: 1.0,
        });
        assert_eq!(
            non_finite.calculate_mse(),
            Err(CuttingMetricError::NonFiniteInput(
                "rotary_torque_magnitude_nm"
            ))
        );

        let negative = observation(CuttingPowerInputV1::DerivedFromToolLoads {
            rotary_torque_magnitude_nm: -1.0,
            angular_speed_magnitude_rad_s: 1.0,
            axial_force_magnitude_n: 1.0,
            feed_speed_magnitude_m_s: 1.0,
        });
        assert_eq!(
            negative.calculate_mse(),
            Err(CuttingMetricError::NegativeMagnitude(
                "rotary_torque_magnitude_nm"
            ))
        );
    }

    #[test]
    fn invalid_source_digest_is_rejected() {
        let mut observation =
            observation(CuttingPowerInputV1::MeasuredToolInterfacePower {
                mechanical_power_w: 1_500.0,
            });
        observation.source.sha256 = "not-a-digest".to_owned();
        assert_eq!(
            observation.calculate_mse(),
            Err(CuttingMetricError::InvalidSourceDigest)
        );
    }

    #[test]
    fn schema_requires_exact_version_and_nonzero_interval() {
        let mut observation =
            observation(CuttingPowerInputV1::MeasuredToolInterfacePower {
                mechanical_power_w: 1_500.0,
            });
        observation.schema_version += 1;
        assert_eq!(
            observation.calculate_mse(),
            Err(CuttingMetricError::UnsupportedSchemaVersion)
        );

        observation.schema_version = CUTTING_METRIC_SCHEMA_VERSION;
        observation.measurement_interval_ns = 0;
        assert_eq!(
            observation.calculate_mse(),
            Err(CuttingMetricError::InvalidMeasurementInterval)
        );
    }
}
