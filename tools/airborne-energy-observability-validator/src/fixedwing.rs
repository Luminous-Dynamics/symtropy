// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! First-principles steady, level fixed-wing cruise estimate.
//!
//! This is a low-fidelity analytical model, not an aircraft aerodynamic
//! validation. The user supplies the drag polar and propulsive efficiency.
//! It does not model climb, descent, stall dynamics, takeoff, VTOL, transition,
//! trim drag, compressibility, Reynolds effects, propwash, aeroelasticity,
//! structural margins, or certification requirements.

use std::fmt;

const GRAVITY_M_S2: f64 = 9.806_65;
const AIR_GAS_CONSTANT_J_KG_K: f64 = 287.052_87;
const SEA_LEVEL_TEMPERATURE_K: f64 = 288.15;
const SEA_LEVEL_PRESSURE_PA: f64 = 101_325.0;
const TROPOSPHERIC_LAPSE_RATE_K_M: f64 = 0.0065;
const AIR_HEAT_CAPACITY_RATIO: f64 = 1.4;
const MAX_TROPOPAUSE_ALTITUDE_M: f64 = 11_000.0;
const PI: f64 = std::f64::consts::PI;

/// International Standard Atmosphere approximation over the troposphere.
///
/// Valid only from 0 through 11,000 m geometric altitude. A more complete
/// atmosphere model is required above this range or for non-standard weather.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtmosphereState {
    pub altitude_m: f64,
    pub temperature_k: f64,
    pub pressure_pa: f64,
    pub density_kg_m3: f64,
    pub speed_of_sound_m_s: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FixedWingCruiseConfig {
    pub mass_kg: f64,
    pub wing_area_m2: f64,
    pub aspect_ratio: f64,
    /// Oswald span-efficiency factor, supplied from a cited polar or test.
    pub oswald_efficiency: f64,
    /// Zero-lift drag coefficient, supplied from a cited polar or test.
    pub zero_lift_drag_coefficient: f64,
    /// Maximum lift coefficient at the declared configuration.
    pub maximum_lift_coefficient: f64,
    /// Propulsor efficiency from shaft mechanical power to useful thrust power.
    pub propulsive_efficiency: f64,
    pub altitude_m: f64,
    pub true_airspeed_m_s: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixedWingCruiseEstimate {
    pub atmosphere: AtmosphereState,
    pub dynamic_pressure_pa: f64,
    pub required_lift_n: f64,
    pub lift_coefficient: f64,
    pub induced_drag_coefficient: f64,
    pub total_drag_coefficient: f64,
    pub drag_n: f64,
    pub useful_thrust_power_w: f64,
    /// Mechanical power required at the propulsor shaft.
    pub propulsor_shaft_power_w: f64,
    /// Ratio of declared maximum lift coefficient to required cruise lift coefficient.
    pub lift_coefficient_margin_ratio: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FixedWingCruiseError {
    InvalidInput(&'static str),
    UnsupportedAltitude {
        altitude_m: f64,
    },
    InsufficientLiftMargin {
        required_lift_coefficient: f64,
        maximum_lift_coefficient: f64,
    },
    NonFiniteResult(&'static str),
}

impl fmt::Display for FixedWingCruiseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(reason) => write!(f, "invalid fixed-wing cruise input: {reason}"),
            Self::UnsupportedAltitude { altitude_m } => {
                write!(f, "altitude {altitude_m} m is outside the 0..=11000 m model domain")
            }
            Self::InsufficientLiftMargin {
                required_lift_coefficient,
                maximum_lift_coefficient,
            } => write!(
                f,
                "required lift coefficient {required_lift_coefficient} reaches or exceeds maximum {maximum_lift_coefficient}"
            ),
            Self::NonFiniteResult(name) => {
                write!(f, "fixed-wing cruise model produced non-finite {name}")
            }
        }
    }
}

impl std::error::Error for FixedWingCruiseError {}

/// Compute tropospheric ISA conditions from sea level through 11 km.
pub fn standard_atmosphere(
    altitude_m: f64,
) -> Result<AtmosphereState, FixedWingCruiseError> {
    if !altitude_m.is_finite() {
        return Err(FixedWingCruiseError::InvalidInput(
            "altitude must be finite",
        ));
    }
    if !(0.0..=MAX_TROPOPAUSE_ALTITUDE_M).contains(&altitude_m) {
        return Err(FixedWingCruiseError::UnsupportedAltitude { altitude_m });
    }

    let temperature_k =
        SEA_LEVEL_TEMPERATURE_K - TROPOSPHERIC_LAPSE_RATE_K_M * altitude_m;
    let pressure_exponent = GRAVITY_M_S2
        / (AIR_GAS_CONSTANT_J_KG_K * TROPOSPHERIC_LAPSE_RATE_K_M);
    let pressure_pa = SEA_LEVEL_PRESSURE_PA
        * (temperature_k / SEA_LEVEL_TEMPERATURE_K).powf(pressure_exponent);
    let density_kg_m3 = pressure_pa / (AIR_GAS_CONSTANT_J_KG_K * temperature_k);
    let speed_of_sound_m_s =
        (AIR_HEAT_CAPACITY_RATIO * AIR_GAS_CONSTANT_J_KG_K * temperature_k).sqrt();

    for (name, value) in [
        ("temperature", temperature_k),
        ("pressure", pressure_pa),
        ("density", density_kg_m3),
        ("speed of sound", speed_of_sound_m_s),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(FixedWingCruiseError::NonFiniteResult(name));
        }
    }

    Ok(AtmosphereState {
        altitude_m,
        temperature_k,
        pressure_pa,
        density_kg_m3,
        speed_of_sound_m_s,
    })
}

/// Estimate steady, level, unaccelerated fixed-wing cruise power.
///
/// The model uses:
/// - lift equals aircraft weight;
/// - dynamic pressure equals 0.5 * density * true_airspeed^2;
/// - induced drag coefficient equals CL^2 / (pi * Oswald * aspect_ratio);
/// - total drag coefficient equals CD0 + CDi;
/// - useful thrust power equals drag * true_airspeed;
/// - shaft power equals useful thrust power / propulsive efficiency.
///
/// The supplied polar and propulsive efficiency must be independently sourced.
/// This model is not suitable for VTOL hover, conversion, manoeuvres, climb,
/// takeoff, degraded-propulsor cases, or certification credit.
pub fn estimate_steady_level_cruise(
    config: &FixedWingCruiseConfig,
) -> Result<FixedWingCruiseEstimate, FixedWingCruiseError> {
    for (name, value) in [
        ("mass_kg", config.mass_kg),
        ("wing_area_m2", config.wing_area_m2),
        ("aspect_ratio", config.aspect_ratio),
        ("oswald_efficiency", config.oswald_efficiency),
        (
            "zero_lift_drag_coefficient",
            config.zero_lift_drag_coefficient,
        ),
        (
            "maximum_lift_coefficient",
            config.maximum_lift_coefficient,
        ),
        ("propulsive_efficiency", config.propulsive_efficiency),
        ("true_airspeed_m_s", config.true_airspeed_m_s),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(FixedWingCruiseError::InvalidInput(match name {
                "mass_kg" => "mass must be finite and positive",
                "wing_area_m2" => "wing area must be finite and positive",
                "aspect_ratio" => "aspect ratio must be finite and positive",
                "oswald_efficiency" => "Oswald efficiency must be finite and positive",
                "zero_lift_drag_coefficient" => "zero-lift drag coefficient must be finite and positive",
                "maximum_lift_coefficient" => "maximum lift coefficient must be finite and positive",
                "propulsive_efficiency" => "propulsive efficiency must be finite and positive",
                "true_airspeed_m_s" => "true airspeed must be finite and positive",
                _ => "invalid numeric input",
            }));
        }
    }
    if config.oswald_efficiency > 1.0 {
        return Err(FixedWingCruiseError::InvalidInput(
            "Oswald efficiency must not exceed 1.0",
        ));
    }
    if config.propulsive_efficiency > 1.0 {
        return Err(FixedWingCruiseError::InvalidInput(
            "propulsive efficiency must not exceed 1.0",
        ));
    }

    let atmosphere = standard_atmosphere(config.altitude_m)?;
    let dynamic_pressure_pa =
        0.5 * atmosphere.density_kg_m3 * config.true_airspeed_m_s.powi(2);
    let required_lift_n = config.mass_kg * GRAVITY_M_S2;
    let lift_coefficient =
        required_lift_n / (dynamic_pressure_pa * config.wing_area_m2);

    if !dynamic_pressure_pa.is_finite() || dynamic_pressure_pa <= 0.0 {
        return Err(FixedWingCruiseError::NonFiniteResult(
            "dynamic pressure",
        ));
    }
    if !lift_coefficient.is_finite() || lift_coefficient <= 0.0 {
        return Err(FixedWingCruiseError::NonFiniteResult(
            "lift coefficient",
        ));
    }
    if lift_coefficient >= config.maximum_lift_coefficient {
        return Err(FixedWingCruiseError::InsufficientLiftMargin {
            required_lift_coefficient: lift_coefficient,
            maximum_lift_coefficient: config.maximum_lift_coefficient,
        });
    }

    let induced_drag_coefficient = lift_coefficient.powi(2)
        / (PI * config.oswald_efficiency * config.aspect_ratio);
    let total_drag_coefficient =
        config.zero_lift_drag_coefficient + induced_drag_coefficient;
    let drag_n = dynamic_pressure_pa * config.wing_area_m2 * total_drag_coefficient;
    let useful_thrust_power_w = drag_n * config.true_airspeed_m_s;
    let propulsor_shaft_power_w = useful_thrust_power_w / config.propulsive_efficiency;
    let lift_coefficient_margin_ratio =
        config.maximum_lift_coefficient / lift_coefficient;

    for (name, value) in [
        ("induced drag coefficient", induced_drag_coefficient),
        ("total drag coefficient", total_drag_coefficient),
        ("drag", drag_n),
        ("useful thrust power", useful_thrust_power_w),
        ("propulsor shaft power", propulsor_shaft_power_w),
        ("lift coefficient margin ratio", lift_coefficient_margin_ratio),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(FixedWingCruiseError::NonFiniteResult(name));
        }
    }

    Ok(FixedWingCruiseEstimate {
        atmosphere,
        dynamic_pressure_pa,
        required_lift_n,
        lift_coefficient,
        induced_drag_coefficient,
        total_drag_coefficient,
        drag_n,
        useful_thrust_power_w,
        propulsor_shaft_power_w,
        lift_coefficient_margin_ratio,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cruise_config() -> FixedWingCruiseConfig {
        FixedWingCruiseConfig {
            mass_kg: 1_000.0,
            wing_area_m2: 20.0,
            aspect_ratio: 8.0,
            oswald_efficiency: 0.8,
            zero_lift_drag_coefficient: 0.025,
            maximum_lift_coefficient: 1.5,
            propulsive_efficiency: 0.82,
            altitude_m: 0.0,
            true_airspeed_m_s: 50.0,
        }
    }

    #[test]
    fn isa_sea_level_matches_reference_density() {
        let atmosphere = standard_atmosphere(0.0).unwrap();
        assert!((atmosphere.temperature_k - 288.15).abs() < 1e-10);
        assert!((atmosphere.pressure_pa - 101_325.0).abs() < 1e-8);
        assert!((atmosphere.density_kg_m3 - 1.225_000_018).abs() < 1e-6);
    }

    #[test]
    fn isa_tropopause_is_finite_and_bounded() {
        let atmosphere = standard_atmosphere(11_000.0).unwrap();
        assert!((atmosphere.temperature_k - 216.65).abs() < 1e-8);
        assert!(atmosphere.density_kg_m3 > 0.36);
        assert!(atmosphere.density_kg_m3 < 0.37);
        assert!(atmosphere.speed_of_sound_m_s.is_finite());
    }

    #[test]
    fn cruise_estimate_closes_lift_drag_and_power_equations() {
        let estimate = estimate_steady_level_cruise(&cruise_config()).unwrap();
        assert!((estimate.required_lift_n - 1_000.0 * GRAVITY_M_S2).abs() < 1e-8);
        assert!(
            (estimate.induced_drag_coefficient
                - estimate.lift_coefficient.powi(2) / (PI * 0.8 * 8.0))
                .abs()
                < 1e-12
        );
        assert!(
            (estimate.propulsor_shaft_power_w
                - estimate.drag_n * 50.0 / 0.82)
                .abs()
                < 1e-8
        );
        assert!(estimate.lift_coefficient_margin_ratio > 1.0);
    }

    #[test]
    fn cruise_beyond_declared_maximum_lift_is_rejected() {
        let mut config = cruise_config();
        config.mass_kg = 100_000.0;
        config.true_airspeed_m_s = 30.0;
        assert!(matches!(
            estimate_steady_level_cruise(&config),
            Err(FixedWingCruiseError::InsufficientLiftMargin { .. })
        ));
    }

    #[test]
    fn invalid_efficiency_and_non_finite_inputs_fail_closed() {
        let mut config = cruise_config();
        config.propulsive_efficiency = 1.2;
        assert!(matches!(
            estimate_steady_level_cruise(&config),
            Err(FixedWingCruiseError::InvalidInput(_))
        ));
        config = cruise_config();
        config.mass_kg = f64::NAN;
        assert!(matches!(
            estimate_steady_level_cruise(&config),
            Err(FixedWingCruiseError::InvalidInput(_))
        ));
    }

    #[test]
    fn unsupported_atmosphere_altitude_is_rejected() {
        assert!(matches!(
            standard_atmosphere(12_000.0),
            Err(FixedWingCruiseError::UnsupportedAltitude { .. })
        ));
        assert!(matches!(
            standard_atmosphere(f64::NAN),
            Err(FixedWingCruiseError::InvalidInput(_))
        ));
    }
}
