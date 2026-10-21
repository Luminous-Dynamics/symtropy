// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicit quantity types and checked conversion to canonical SI values.
//!
//! This is deliberately not a free-form unit parser. Callers choose a named
//! constructor for a supported unit; solver wrappers accept only the matching
//! quantity type and pass canonical SI values to the analytical reference.
//! Type checking prevents mixing quantities across the wrapper boundary, while
//! runtime checks reject non-finite, overflowed, and invalid positive quantities.

use crate::Check;
use crate::mechanics::{
    self, AxialBarResult, CantileverTipLoadResult, SimplySupportedUniformLoadResult,
};

const MILLIMETERS_TO_METERS: f64 = 1.0e-3;
const KILONEWTONS_TO_NEWTONS: f64 = 1.0e3;
const SQUARE_MILLIMETERS_TO_SQUARE_METERS: f64 = 1.0e-6;
const GIGAPASCALS_TO_PASCALS: f64 = 1.0e9;
const MILLIMETERS4_TO_METERS4: f64 = 1.0e-12;
const KILONEWTONS_PER_METER_TO_NEWTONS_PER_METER: f64 = 1.0e3;

fn finite(value: f64, label: &str) -> Check<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("{label} must be finite"))
    }
}

fn positive(value: f64, label: &str) -> Check<f64> {
    finite(value, label)?;
    if value > 0.0 {
        Ok(value)
    } else {
        Err(format!("{label} must be positive"))
    }
}

fn finite_scaled(value: f64, factor: f64, label: &str) -> Check<f64> {
    finite(value, label)?;
    let converted = value * factor;
    finite(converted, &format!("converted {label}"))
}

fn positive_scaled(value: f64, factor: f64, label: &str) -> Check<f64> {
    positive(value, label)?;
    let converted = value * factor;
    positive(converted, &format!("converted {label}"))
}

/// Canonical positive length in metres.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct LengthM(f64);

impl LengthM {
    pub fn from_meters(value: f64) -> Check<Self> {
        Ok(Self(positive(value, "length in metres")?))
    }

    pub fn from_millimeters(value: f64) -> Check<Self> {
        Ok(Self(positive_scaled(
            value,
            MILLIMETERS_TO_METERS,
            "length in millimetres",
        )?))
    }

    pub const fn as_meters(self) -> f64 {
        self.0
    }
}

/// Canonical signed force in newtons. Sign is preserved for tension/compression
/// or load direction according to the consuming mechanics API.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct ForceN(f64);

impl ForceN {
    pub fn from_newtons(value: f64) -> Check<Self> {
        Ok(Self(finite(value, "force in newtons")?))
    }

    pub fn from_kilonewtons(value: f64) -> Check<Self> {
        Ok(Self(finite_scaled(
            value,
            KILONEWTONS_TO_NEWTONS,
            "force in kilonewtons",
        )?))
    }

    pub const fn as_newtons(self) -> f64 {
        self.0
    }
}

/// Canonical positive cross-sectional area in square metres.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct AreaM2(f64);

impl AreaM2 {
    pub fn from_square_meters(value: f64) -> Check<Self> {
        Ok(Self(positive(value, "area in square metres")?))
    }

    pub fn from_square_millimeters(value: f64) -> Check<Self> {
        Ok(Self(positive_scaled(
            value,
            SQUARE_MILLIMETERS_TO_SQUARE_METERS,
            "area in square millimetres",
        )?))
    }

    pub const fn as_square_meters(self) -> f64 {
        self.0
    }
}

/// Canonical positive Young's modulus in pascals.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct YoungsModulusPa(f64);

impl YoungsModulusPa {
    pub fn from_pascals(value: f64) -> Check<Self> {
        Ok(Self(positive(value, "Young's modulus in pascals")?))
    }

    pub fn from_gigapascals(value: f64) -> Check<Self> {
        Ok(Self(positive_scaled(
            value,
            GIGAPASCALS_TO_PASCALS,
            "Young's modulus in gigapascals",
        )?))
    }

    pub const fn as_pascals(self) -> f64 {
        self.0
    }
}

/// Canonical positive second moment of area in metres to the fourth power.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct SecondMomentM4(f64);

impl SecondMomentM4 {
    pub fn from_meters4(value: f64) -> Check<Self> {
        Ok(Self(positive(
            value,
            "second moment in metres to the fourth",
        )?))
    }

    pub fn from_millimeters4(value: f64) -> Check<Self> {
        Ok(Self(positive_scaled(
            value,
            MILLIMETERS4_TO_METERS4,
            "second moment in millimetres to the fourth",
        )?))
    }

    pub const fn as_meters4(self) -> f64 {
        self.0
    }
}

/// Canonical signed distributed load in newtons per metre.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct DistributedLoadNPerM(f64);

impl DistributedLoadNPerM {
    pub fn from_newtons_per_meter(value: f64) -> Check<Self> {
        Ok(Self(finite(
            value,
            "distributed load in newtons per metre",
        )?))
    }

    pub fn from_kilonewtons_per_meter(value: f64) -> Check<Self> {
        Ok(Self(finite_scaled(
            value,
            KILONEWTONS_PER_METER_TO_NEWTONS_PER_METER,
            "distributed load in kilonewtons per metre",
        )?))
    }

    pub const fn as_newtons_per_meter(self) -> f64 {
        self.0
    }
}

/// Typed-input axial-bar reference. The mechanics formula still applies only
/// to its documented uniform, linear-elastic idealization.
pub fn axial_bar(
    force: ForceN,
    length: LengthM,
    area: AreaM2,
    youngs_modulus: YoungsModulusPa,
) -> Check<AxialBarResult> {
    mechanics::axial_bar(
        force.as_newtons(),
        length.as_meters(),
        area.as_square_meters(),
        youngs_modulus.as_pascals(),
    )
}

/// Typed-input cantilever reference under a perpendicular tip force.
pub fn cantilever_tip_load(
    force: ForceN,
    length: LengthM,
    youngs_modulus: YoungsModulusPa,
    second_moment: SecondMomentM4,
    outer_fiber_distance: LengthM,
) -> Check<CantileverTipLoadResult> {
    mechanics::cantilever_tip_load(
        force.as_newtons(),
        length.as_meters(),
        youngs_modulus.as_pascals(),
        second_moment.as_meters4(),
        outer_fiber_distance.as_meters(),
    )
}

/// Typed-input simply supported beam reference under a full-span uniform load.
pub fn simply_supported_uniform_load(
    load: DistributedLoadNPerM,
    length: LengthM,
    youngs_modulus: YoungsModulusPa,
    second_moment: SecondMomentM4,
    outer_fiber_distance: LengthM,
) -> Check<SimplySupportedUniformLoadResult> {
    mechanics::simply_supported_uniform_load(
        load.as_newtons_per_meter(),
        length.as_meters(),
        youngs_modulus.as_pascals(),
        second_moment.as_meters4(),
        outer_fiber_distance.as_meters(),
    )
}
