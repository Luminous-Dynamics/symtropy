// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evidence-bearing, dimensioned rock-material profiles for robotics worlds.
//!
//! These records describe inputs and provenance. They do not implement a
//! constitutive law or establish that a measured property generalizes to a site.

use crate::world_contract::ArtifactRef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RockLithologyV1 {
    Granite,
    Basalt,
    Diorite,
    Gabbro,
    Quartzite,
    Dolomite,
    Limestone,
    Sandstone,
    Shale,
    Unclassified,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MaterialEvidenceClassV1 {
    SyntheticScenario,
    Inferred,
    Literature,
    LaboratoryMeasured,
    FieldMeasured,
}

impl MaterialEvidenceClassV1 {
    pub const fn requires_source(self) -> bool {
        !matches!(self, Self::SyntheticScenario)
    }

    pub const fn requires_sample_or_site(self) -> bool {
        matches!(self, Self::LaboratoryMeasured | Self::FieldMeasured)
    }
}

/// Quantity identity also defines the canonical SI unit accepted by the schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MaterialQuantityV1 {
    BulkDensity,
    UnconfinedCompressiveStrength,
    TensileStrength,
    YoungsModulus,
    PoissonRatio,
    ModeIFractureToughness,
    Porosity,
    IntrinsicPermeability,
    PWaveVelocity,
    SWaveVelocity,
    ThermalConductivity,
    SpecificHeatCapacity,
    LinearThermalExpansion,
    CercharAbrasivityIndex,
}

impl MaterialQuantityV1 {
    pub const fn canonical_unit(self) -> &'static str {
        match self {
            Self::BulkDensity => "kg/m^3",
            Self::UnconfinedCompressiveStrength
            | Self::TensileStrength
            | Self::YoungsModulus => "Pa",
            Self::PoissonRatio | Self::Porosity => "1",
            Self::ModeIFractureToughness => "Pa*sqrt(m)",
            Self::IntrinsicPermeability => "m^2",
            Self::PWaveVelocity | Self::SWaveVelocity => "m/s",
            Self::ThermalConductivity => "W/(m*K)",
            Self::SpecificHeatCapacity => "J/(kg*K)",
            Self::LinearThermalExpansion => "1/K",
            Self::CercharAbrasivityIndex => "index",
        }
    }

    pub fn value_is_in_physical_range(self, value: f64) -> bool {
        if !value.is_finite() {
            return false;
        }
        match self {
            Self::BulkDensity
            | Self::UnconfinedCompressiveStrength
            | Self::TensileStrength
            | Self::YoungsModulus
            | Self::ModeIFractureToughness
            | Self::PWaveVelocity
            | Self::SWaveVelocity
            | Self::ThermalConductivity
            | Self::SpecificHeatCapacity => value > 0.0,
            Self::PoissonRatio => value > -1.0 && value < 0.5,
            Self::Porosity => (0.0..=1.0).contains(&value),
            Self::IntrinsicPermeability | Self::CercharAbrasivityIndex => value >= 0.0,
            // Negative thermal expansion is physically possible for some materials.
            Self::LinearThermalExpansion => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RockMaterialConditionV1 {
    /// Stable identity for this specimen/site state and conditioning protocol.
    pub condition_id: String,
    /// Absolute temperature, kelvin.
    pub temperature_k: Option<f64>,
    /// Water saturation fraction in [0, 1], not a percentage.
    pub water_saturation_fraction: Option<f64>,
    /// Confining pressure in pascals.
    pub confining_pressure_pa: Option<f64>,
    /// Orientation relative to the declared specimen/site coordinate frame.
    pub orientation_description: Option<String>,
    /// Weathering, alteration, fractures, or conditioning note.
    pub alteration_description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialPropertyV1 {
    pub quantity: MaterialQuantityV1,
    pub value_si: f64,
    /// Must equal MaterialQuantityV1::canonical_unit() for this schema version.
    pub unit: String,
    /// Standard uncertainty in the same canonical unit, when known.
    pub uncertainty_std_si: Option<f64>,
    pub evidence_class: MaterialEvidenceClassV1,
    /// Digest-bound provenance for the measurement, publication, or inference input.
    pub source: Option<ArtifactRef>,
    /// Required for laboratory- and field-measured claims.
    pub sample_or_site_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RockMaterialProfileV1 {
    pub profile_id: String,
    pub lithology: RockLithologyV1,
    pub display_label: String,
    pub condition: RockMaterialConditionV1,
    /// Sparse profiles are allowed, but downstream models must declare their
    /// required quantities and return OutOfDomain/InsufficientEvidence if absent.
    pub properties: Vec<MaterialPropertyV1>,
    pub notes: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_quantities_expose_canonical_units() {
        assert_eq!(MaterialQuantityV1::BulkDensity.canonical_unit(), "kg/m^3");
        assert_eq!(
            MaterialQuantityV1::ModeIFractureToughness.canonical_unit(),
            "Pa*sqrt(m)"
        );
        assert_eq!(
            MaterialQuantityV1::LinearThermalExpansion.canonical_unit(),
            "1/K"
        );
    }

    #[test]
    fn physical_range_checks_reject_nonphysical_strength_and_porosity() {
        assert!(!MaterialQuantityV1::YoungsModulus.value_is_in_physical_range(0.0));
        assert!(!MaterialQuantityV1::Porosity.value_is_in_physical_range(1.1));
        assert!(!MaterialQuantityV1::PoissonRatio.value_is_in_physical_range(0.5));
        assert!(MaterialQuantityV1::LinearThermalExpansion.value_is_in_physical_range(-1e-6));
    }

    #[test]
    fn evidence_classes_define_source_requirements() {
        assert!(!MaterialEvidenceClassV1::SyntheticScenario.requires_source());
        assert!(MaterialEvidenceClassV1::Inferred.requires_source());
        assert!(MaterialEvidenceClassV1::LaboratoryMeasured.requires_sample_or_site());
        assert!(!MaterialEvidenceClassV1::Literature.requires_sample_or_site());
    }
}
