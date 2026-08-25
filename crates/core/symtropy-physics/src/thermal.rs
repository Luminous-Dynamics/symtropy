// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Deterministic thermodynamic primitives for the physics core.
//!
//! This module intentionally starts small: temperature, material heat capacity,
//! and pairwise conductive exchange. The conductive kernel is conservative by
//! construction and clamps a step at pair equilibrium so a large timestep cannot
//! numerically overshoot and make heat flow from cold to hot in a single update.
//!
//! Phase changes, radiation, fluid advection, and thermo-mechanical coupling can
//! build on this layer without changing its energy-accounting contract.

use serde::{Deserialize, Serialize};

/// Absolute zero in kelvin.
pub const ABSOLUTE_ZERO_K: f64 = 0.0;

/// Thermophysical properties that are intrinsic to a material.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThermalMaterial {
    /// Specific heat capacity in J/(kg*K). Must be finite and strictly positive.
    pub specific_heat_capacity: f64,
    /// Thermal conductivity in W/(m*K). Must be finite and non-negative.
    pub thermal_conductivity: f64,
    /// Surface emissivity in [0, 1]. Reserved for radiative exchange.
    pub emissivity: f64,
}

impl ThermalMaterial {
    pub fn new(
        specific_heat_capacity: f64,
        thermal_conductivity: f64,
        emissivity: f64,
    ) -> Result<Self, ThermalError> {
        if !specific_heat_capacity.is_finite() || specific_heat_capacity <= 0.0 {
            return Err(ThermalError::InvalidSpecificHeatCapacity);
        }
        if !thermal_conductivity.is_finite() || thermal_conductivity < 0.0 {
            return Err(ThermalError::InvalidThermalConductivity);
        }
        if !emissivity.is_finite() || !(0.0..=1.0).contains(&emissivity) {
            return Err(ThermalError::InvalidEmissivity);
        }

        Ok(Self {
            specific_heat_capacity,
            thermal_conductivity,
            emissivity,
        })
    }

    /// Heat capacity `m c_p` in J/K for a body of the supplied mass.
    pub fn heat_capacity(self, mass_kg: f64) -> Result<f64, ThermalError> {
        validate_positive_finite(mass_kg, ThermalError::InvalidMass)?;
        Ok(mass_kg * self.specific_heat_capacity)
    }
}

/// Minimal thermodynamic state for a body or material cell.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThermalState {
    /// Absolute temperature in kelvin.
    pub temperature_kelvin: f64,
}

impl ThermalState {
    pub fn new(temperature_kelvin: f64) -> Result<Self, ThermalError> {
        validate_temperature(temperature_kelvin)?;
        Ok(Self { temperature_kelvin })
    }

    /// Sensible thermal energy relative to `reference_temperature_kelvin`.
    ///
    /// The reference is explicit because only energy differences are needed by
    /// the conservative kernel. A later phase model can add latent-energy terms.
    pub fn sensible_energy_joules(
        self,
        reference_temperature_kelvin: f64,
        mass_kg: f64,
        material: ThermalMaterial,
    ) -> Result<f64, ThermalError> {
        validate_temperature(reference_temperature_kelvin)?;
        let heat_capacity = material.heat_capacity(mass_kg)?;
        Ok(heat_capacity * (self.temperature_kelvin - reference_temperature_kelvin))
    }

    /// Add sensible heat to this state and return the resulting temperature.
    pub fn add_heat_joules(
        &mut self,
        energy_joules: f64,
        mass_kg: f64,
        material: ThermalMaterial,
    ) -> Result<f64, ThermalError> {
        if !energy_joules.is_finite() {
            return Err(ThermalError::InvalidEnergy);
        }
        let heat_capacity = material.heat_capacity(mass_kg)?;
        let next = self.temperature_kelvin + energy_joules / heat_capacity;
        validate_temperature(next)?;
        self.temperature_kelvin = next;
        Ok(next)
    }
}

/// Result of a conservative pairwise heat-transfer step.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeatExchange {
    /// Signed heat transferred from A to B in joules. Positive means A -> B.
    pub joules_from_a_to_b: f64,
    /// True when the requested Euler transfer was capped at pair equilibrium.
    pub equilibrium_limited: bool,
}

/// Errors returned when a thermodynamic state would be physically invalid.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ThermalError {
    InvalidTemperature,
    InvalidSpecificHeatCapacity,
    InvalidThermalConductivity,
    InvalidEmissivity,
    InvalidMass,
    InvalidConductance,
    InvalidTimestep,
    InvalidEnergy,
}

/// Exchange sensible heat between two lumped thermal states.
///
/// `conductance_w_per_k` is an effective pair conductance (`UA`) in W/K. It can
/// be derived by a caller from conductivity, contact area, and path length. The
/// transfer is exactly antisymmetric: energy removed from A is added to B.
/// Large timesteps are capped at the pair's equilibrium transfer, which prevents
/// a single step from crossing equilibrium and reversing the temperature order.
pub fn conductive_exchange(
    a: &mut ThermalState,
    material_a: ThermalMaterial,
    mass_a_kg: f64,
    b: &mut ThermalState,
    material_b: ThermalMaterial,
    mass_b_kg: f64,
    conductance_w_per_k: f64,
    dt_seconds: f64,
) -> Result<HeatExchange, ThermalError> {
    validate_temperature(a.temperature_kelvin)?;
    validate_temperature(b.temperature_kelvin)?;
    let capacity_a = material_a.heat_capacity(mass_a_kg)?;
    let capacity_b = material_b.heat_capacity(mass_b_kg)?;

    if !conductance_w_per_k.is_finite() || conductance_w_per_k < 0.0 {
        return Err(ThermalError::InvalidConductance);
    }
    if !dt_seconds.is_finite() || dt_seconds < 0.0 {
        return Err(ThermalError::InvalidTimestep);
    }

    let delta_t = a.temperature_kelvin - b.temperature_kelvin;
    if delta_t == 0.0 || conductance_w_per_k == 0.0 || dt_seconds == 0.0 {
        return Ok(HeatExchange {
            joules_from_a_to_b: 0.0,
            equilibrium_limited: false,
        });
    }

    let requested = conductance_w_per_k * delta_t * dt_seconds;
    let equilibrium_transfer = delta_t / (capacity_a.recip() + capacity_b.recip());
    let equilibrium_limited = requested.abs() > equilibrium_transfer.abs();
    let transfer = if equilibrium_limited {
        equilibrium_transfer
    } else {
        requested
    };

    a.temperature_kelvin -= transfer / capacity_a;
    b.temperature_kelvin += transfer / capacity_b;

    debug_assert!(a.temperature_kelvin >= ABSOLUTE_ZERO_K);
    debug_assert!(b.temperature_kelvin >= ABSOLUTE_ZERO_K);

    Ok(HeatExchange {
        joules_from_a_to_b: transfer,
        equilibrium_limited,
    })
}

fn validate_temperature(temperature_kelvin: f64) -> Result<(), ThermalError> {
    if temperature_kelvin.is_finite() && temperature_kelvin >= ABSOLUTE_ZERO_K {
        Ok(())
    } else {
        Err(ThermalError::InvalidTemperature)
    }
}

fn validate_positive_finite(value: f64, error: ThermalError) -> Result<(), ThermalError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material(cp: f64) -> ThermalMaterial {
        ThermalMaterial::new(cp, 1.0, 0.5).unwrap()
    }

    #[test]
    fn rejects_unphysical_inputs() {
        assert_eq!(
            ThermalState::new(-0.001),
            Err(ThermalError::InvalidTemperature)
        );
        assert_eq!(
            ThermalMaterial::new(0.0, 1.0, 0.5),
            Err(ThermalError::InvalidSpecificHeatCapacity)
        );
        assert_eq!(
            ThermalMaterial::new(1000.0, -1.0, 0.5),
            Err(ThermalError::InvalidThermalConductivity)
        );
        assert_eq!(
            ThermalMaterial::new(1000.0, 1.0, 1.01),
            Err(ThermalError::InvalidEmissivity)
        );
    }

    #[test]
    fn conductive_exchange_conserves_pair_sensible_energy() {
        let mat_a = material(500.0);
        let mat_b = material(1000.0);
        let mut a = ThermalState::new(400.0).unwrap();
        let mut b = ThermalState::new(300.0).unwrap();
        let mass_a = 2.0;
        let mass_b = 3.0;

        let before = a.sensible_energy_joules(0.0, mass_a, mat_a).unwrap()
            + b.sensible_energy_joules(0.0, mass_b, mat_b).unwrap();

        let exchange =
            conductive_exchange(&mut a, mat_a, mass_a, &mut b, mat_b, mass_b, 25.0, 0.5).unwrap();

        let after = a.sensible_energy_joules(0.0, mass_a, mat_a).unwrap()
            + b.sensible_energy_joules(0.0, mass_b, mat_b).unwrap();

        assert!(exchange.joules_from_a_to_b > 0.0);
        assert!((after - before).abs() < 1e-8);
        assert!(a.temperature_kelvin < 400.0);
        assert!(b.temperature_kelvin > 300.0);
    }

    #[test]
    fn large_timestep_stops_at_equilibrium_without_overshoot() {
        let mat = material(1000.0);
        let mut a = ThermalState::new(500.0).unwrap();
        let mut b = ThermalState::new(300.0).unwrap();

        let exchange = conductive_exchange(&mut a, mat, 1.0, &mut b, mat, 1.0, 1.0e9, 1.0).unwrap();

        assert!(exchange.equilibrium_limited);
        assert!((a.temperature_kelvin - 400.0).abs() < 1e-12);
        assert!((b.temperature_kelvin - 400.0).abs() < 1e-12);
    }

    #[test]
    fn sign_convention_is_positive_from_a_to_b() {
        let mat = material(1000.0);
        let mut a = ThermalState::new(250.0).unwrap();
        let mut b = ThermalState::new(350.0).unwrap();

        let exchange =
            conductive_exchange(&mut a, mat, 1.0, &mut b, mat, 1.0, 10.0, 1.0).unwrap();

        assert!(exchange.joules_from_a_to_b < 0.0);
        assert!(a.temperature_kelvin > 250.0);
        assert!(b.temperature_kelvin < 350.0);
    }
}
