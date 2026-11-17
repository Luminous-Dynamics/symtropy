// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Explicit semantics for the shared LifeSim field channels.
//!
//! A deterministic scalar field is not automatically a physical stock. In
//! particular, names such as Biomass, Nutrient, Moisture, or Toxin do not
//! establish units, cell geometry, conservation, or canonical ownership.
//!
//! This registry classifies the current shared channels conservatively:
//! - F-S: signal/intensity used to convey information;
//! - F-I: ecological index or derived availability/suitability estimate;
//! - F-P: dimensioned physical field, which requires a separate qualified
//!   physical-field contract and is not currently represented by FieldGrid.
//!
//! All current FieldGrid layers are non-authoritative. This is a classification
//! boundary only; it does not change the existing field update equations or
//! their intended use as deterministic research/presentation substrates.

use crate::FieldLayer;

/// Semantic class of a shared FieldGrid channel.
///
/// F-P is reserved for a future dimensioned field adapter. Assigning F-P alone
/// must never grant canonical stock authority: units, geometry, boundary
/// conditions, update semantics, conservation tolerance, and the relationship
/// to the canonical stock owner must be qualified separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldSemanticClass {
    /// F-S: information/intensity rather than material stock.
    Signal,
    /// F-I: normalized, heuristic, or domain-scaled ecological index.
    DerivedIndex,
    /// F-P: dimensioned physical quantity; not currently assigned to a
    /// FieldGrid layer.
    Physical,
}

impl FieldSemanticClass {
    /// Stable short code used in manifests and diagnostics.
    pub const fn code(self) -> &'static str {
        match self {
            Self::Signal => "F-S",
            Self::DerivedIndex => "F-I",
            Self::Physical => "F-P",
        }
    }

    /// Whether this class is eligible to carry a physical quantity contract.
    ///
    /// Eligibility is not qualification or canonical ownership.
    pub const fn requires_physical_contract(self) -> bool {
        matches!(self, Self::Physical)
    }
}

impl FieldLayer {
    /// Conservative default semantic class for this shared field channel.
    ///
    /// Current pheromone/noise channels are signals. The remaining channels are
    /// derived indices or masks in the generic LifeSim substrate, even where
    /// their names resemble physical quantities. A domain-specific physical
    /// adapter must carry its own units/geometry/conservation contract rather
    /// than silently upgrading this generic channel.
    pub const fn semantic_class(self) -> FieldSemanticClass {
        match self {
            Self::FoodPheromone
            | Self::HomePheromone
            | Self::DangerPheromone
            | Self::SignalNoise => FieldSemanticClass::Signal,
            Self::Moisture
            | Self::Obstacle
            | Self::Nutrient
            | Self::Toxin
            | Self::Biomass
            | Self::Heat
            | Self::Light
            | Self::Oxygen
            | Self::Disease
            | Self::NullContamination => FieldSemanticClass::DerivedIndex,
        }
    }

    /// Shared FieldGrid channels never own exact ecological stock.
    ///
    /// F-P fields need a separately qualified adapter/authority path. No
    /// current FieldLayer satisfies that contract.
    pub const fn has_canonical_stock_authority(self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_LAYERS: [FieldLayer; FieldLayer::COUNT] = [
        FieldLayer::FoodPheromone,
        FieldLayer::HomePheromone,
        FieldLayer::DangerPheromone,
        FieldLayer::Moisture,
        FieldLayer::Obstacle,
        FieldLayer::Nutrient,
        FieldLayer::Toxin,
        FieldLayer::Biomass,
        FieldLayer::Heat,
        FieldLayer::Light,
        FieldLayer::Oxygen,
        FieldLayer::Disease,
        FieldLayer::SignalNoise,
        FieldLayer::NullContamination,
    ];

    #[test]
    fn every_shared_field_layer_has_an_explicit_nonphysical_default() {
        assert_eq!(ALL_LAYERS.len(), FieldLayer::COUNT);
        for layer in ALL_LAYERS {
            assert_ne!(layer.semantic_class(), FieldSemanticClass::Physical);
            assert!(!layer.has_canonical_stock_authority());
        }
    }

    #[test]
    fn pheromone_and_noise_channels_are_signals() {
        for layer in [
            FieldLayer::FoodPheromone,
            FieldLayer::HomePheromone,
            FieldLayer::DangerPheromone,
            FieldLayer::SignalNoise,
        ] {
            assert_eq!(layer.semantic_class(), FieldSemanticClass::Signal);
        }
    }

    #[test]
    fn physical_sounding_names_remain_derived_indices_in_the_generic_grid() {
        for layer in [
            FieldLayer::Moisture,
            FieldLayer::Nutrient,
            FieldLayer::Toxin,
            FieldLayer::Biomass,
            FieldLayer::Heat,
            FieldLayer::Light,
            FieldLayer::Oxygen,
        ] {
            assert_eq!(layer.semantic_class(), FieldSemanticClass::DerivedIndex);
            assert!(!layer.has_canonical_stock_authority());
        }
    }

    #[test]
    fn semantic_class_codes_are_stable_and_distinct() {
        assert_eq!(FieldSemanticClass::Signal.code(), "F-S");
        assert_eq!(FieldSemanticClass::DerivedIndex.code(), "F-I");
        assert_eq!(FieldSemanticClass::Physical.code(), "F-P");
        assert!(FieldSemanticClass::Physical.requires_physical_contract());
        assert!(!FieldSemanticClass::Signal.requires_physical_contract());
        assert!(!FieldSemanticClass::DerivedIndex.requires_physical_contract());
    }
}
