// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed, persistent interfaces between fabricated workpieces.
//!
//! Interfaces describe how workpieces may mate. They do not decide whether a
//! complete assembly satisfies an engineering design; service suitability and
//! derating belong to later functional evaluators.

use serde::{Deserialize, Serialize};
use std::fmt;
use symtropy_game_state::StableId;

use crate::WorkpieceId;

/// Stable identity of one exposed interface on a workpiece.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InterfaceId(StableId);

impl InterfaceId {
    pub const fn new(id: StableId) -> Self {
        Self(id)
    }

    pub const fn stable_id(&self) -> &StableId {
        &self.0
    }
}

impl fmt::Display for InterfaceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Shared deterministic geometric mating description.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatingGeometry {
    /// Standard/profile identity such as a flange, thread, rail, or connector.
    pub profile_id: StableId,
    /// Nominal characteristic size in micrometres.
    pub nominal_size_um: u64,
    /// Permitted symmetric dimensional deviation in micrometres.
    pub tolerance_um: u64,
}

impl MatingGeometry {
    /// Returns true when the two intervals overlap and the profiles match.
    pub fn mates_with(&self, other: &Self) -> bool {
        self.profile_id == other.profile_id
            && interval_distance(self.nominal_size_um, other.nominal_size_um)
                <= self.tolerance_um.saturating_add(other.tolerance_um)
    }
}

/// Six foundation interface families. New subtypes should be added inside a
/// family before inventing cross-cutting stringly-typed categories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "family", rename_all = "snake_case")]
pub enum InterfaceKind {
    Mechanical {
        geometry: MatingGeometry,
        motion_profile: StableId,
    },
    Structural {
        geometry: MatingGeometry,
        load_transfer_profile: StableId,
    },
    Fluid {
        geometry: MatingGeometry,
        medium_class: StableId,
    },
    Electrical {
        geometry: MatingGeometry,
        conductor_profile: StableId,
    },
    Thermal {
        geometry: MatingGeometry,
        contact_profile: StableId,
    },
    DataControl {
        geometry: MatingGeometry,
        protocol_profile: StableId,
    },
}

impl InterfaceKind {
    pub const fn family(&self) -> InterfaceFamily {
        match self {
            Self::Mechanical { .. } => InterfaceFamily::Mechanical,
            Self::Structural { .. } => InterfaceFamily::Structural,
            Self::Fluid { .. } => InterfaceFamily::Fluid,
            Self::Electrical { .. } => InterfaceFamily::Electrical,
            Self::Thermal { .. } => InterfaceFamily::Thermal,
            Self::DataControl { .. } => InterfaceFamily::DataControl,
        }
    }

    fn geometry(&self) -> &MatingGeometry {
        match self {
            Self::Mechanical { geometry, .. }
            | Self::Structural { geometry, .. }
            | Self::Fluid { geometry, .. }
            | Self::Electrical { geometry, .. }
            | Self::Thermal { geometry, .. }
            | Self::DataControl { geometry, .. } => geometry,
        }
    }

    fn semantic_profile(&self) -> &StableId {
        match self {
            Self::Mechanical { motion_profile, .. } => motion_profile,
            Self::Structural {
                load_transfer_profile,
                ..
            } => load_transfer_profile,
            Self::Fluid { medium_class, .. } => medium_class,
            Self::Electrical {
                conductor_profile,
                ..
            } => conductor_profile,
            Self::Thermal {
                contact_profile, ..
            } => contact_profile,
            Self::DataControl {
                protocol_profile, ..
            } => protocol_profile,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceFamily {
    Mechanical,
    Structural,
    Fluid,
    Electrical,
    Thermal,
    DataControl,
}

/// One persistent exposed interface owned by a workpiece.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interface {
    pub id: InterfaceId,
    pub workpiece_id: WorkpieceId,
    pub kind: InterfaceKind,
}

impl Interface {
    /// Deterministically evaluates physical/semantic mating compatibility.
    /// The result is symmetric: swapping `self` and `other` preserves it.
    pub fn compatibility_with(&self, other: &Self) -> InterfaceCompatibility {
        if self.id == other.id {
            return InterfaceCompatibility::Incompatible(InterfaceMismatch::SameInterface);
        }
        if self.kind.family() != other.kind.family() {
            return InterfaceCompatibility::Incompatible(InterfaceMismatch::Family {
                left: self.kind.family(),
                right: other.kind.family(),
            });
        }
        if self.kind.geometry().profile_id != other.kind.geometry().profile_id {
            return InterfaceCompatibility::Incompatible(InterfaceMismatch::GeometryProfile);
        }
        if !self.kind.geometry().mates_with(other.kind.geometry()) {
            return InterfaceCompatibility::Incompatible(InterfaceMismatch::DimensionalTolerance {
                left_nominal_um: self.kind.geometry().nominal_size_um,
                right_nominal_um: other.kind.geometry().nominal_size_um,
                combined_tolerance_um: self
                    .kind
                    .geometry()
                    .tolerance_um
                    .saturating_add(other.kind.geometry().tolerance_um),
            });
        }
        if self.kind.semantic_profile() != other.kind.semantic_profile() {
            return InterfaceCompatibility::Incompatible(InterfaceMismatch::SemanticProfile);
        }
        InterfaceCompatibility::Compatible
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceCompatibility {
    Compatible,
    Incompatible(InterfaceMismatch),
}

impl InterfaceCompatibility {
    pub const fn is_compatible(&self) -> bool {
        matches!(self, Self::Compatible)
    }
}

/// Explainable mismatch rather than a bare compatibility boolean.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum InterfaceMismatch {
    SameInterface,
    Family {
        left: InterfaceFamily,
        right: InterfaceFamily,
    },
    GeometryProfile,
    DimensionalTolerance {
        left_nominal_um: u64,
        right_nominal_um: u64,
        combined_tolerance_um: u64,
    },
    SemanticProfile,
}

const fn interval_distance(left: u64, right: u64) -> u64 {
    left.abs_diff(right)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn interface(
        interface_id: &str,
        workpiece_id: &str,
        nominal_size_um: u64,
        tolerance_um: u64,
        medium: &str,
    ) -> Interface {
        Interface {
            id: InterfaceId::new(id(interface_id)),
            workpiece_id: WorkpieceId::new(id(workpiece_id)),
            kind: InterfaceKind::Fluid {
                geometry: MatingGeometry {
                    profile_id: id("profile:flange:a"),
                    nominal_size_um,
                    tolerance_um,
                },
                medium_class: id(medium),
            },
        }
    }

    #[test]
    fn compatibility_is_symmetric() {
        let a = interface("interface:a", "workpiece:a", 50_000, 100, "medium:water");
        let b = interface("interface:b", "workpiece:b", 50_120, 50, "medium:water");
        assert_eq!(a.compatibility_with(&b), b.compatibility_with(&a));
        assert!(a.compatibility_with(&b).is_compatible());
    }

    #[test]
    fn tolerance_mismatch_is_explainable() {
        let a = interface("interface:a", "workpiece:a", 50_000, 25, "medium:water");
        let b = interface("interface:b", "workpiece:b", 50_100, 25, "medium:water");
        assert!(matches!(
            a.compatibility_with(&b),
            InterfaceCompatibility::Incompatible(InterfaceMismatch::DimensionalTolerance {
                left_nominal_um: 50_000,
                right_nominal_um: 50_100,
                combined_tolerance_um: 50,
            })
        ));
    }

    #[test]
    fn semantic_mismatch_does_not_hide_behind_matching_geometry() {
        let water = interface("interface:a", "workpiece:a", 50_000, 100, "medium:water");
        let fuel = interface("interface:b", "workpiece:b", 50_000, 100, "medium:fuel");
        assert!(matches!(
            water.compatibility_with(&fuel),
            InterfaceCompatibility::Incompatible(InterfaceMismatch::SemanticProfile)
        ));
    }

    #[test]
    fn distinct_interfaces_on_one_workpiece_can_form_a_seam() {
        let a = interface("interface:a", "workpiece:a", 50_000, 100, "medium:water");
        let b = interface("interface:b", "workpiece:a", 50_000, 100, "medium:water");
        assert!(a.compatibility_with(&b).is_compatible());
    }

    #[test]
    fn interface_cannot_mate_with_itself() {
        let a = interface("interface:a", "workpiece:a", 50_000, 100, "medium:water");
        assert!(matches!(
            a.compatibility_with(&a),
            InterfaceCompatibility::Incompatible(InterfaceMismatch::SameInterface)
        ));
    }
}
