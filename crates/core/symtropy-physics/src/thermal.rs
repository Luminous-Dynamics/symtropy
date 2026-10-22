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
        let material = Self {
            specific_heat_capacity,
            thermal_conductivity,
            emissivity,
        };
        material.validate()?;
        Ok(material)
    }

    /// Re-validate material properties after construction or public-field mutation.
    pub fn validate(self) -> Result<(), ThermalError> {
        if !self.specific_heat_capacity.is_finite() || self.specific_heat_capacity <= 0.0 {
            return Err(ThermalError::InvalidSpecificHeatCapacity);
        }
        if !self.thermal_conductivity.is_finite() || self.thermal_conductivity < 0.0 {
            return Err(ThermalError::InvalidThermalConductivity);
        }
        if !self.emissivity.is_finite() || !(0.0..=1.0).contains(&self.emissivity) {
            return Err(ThermalError::InvalidEmissivity);
        }
        Ok(())
    }

    /// Heat capacity `m c_p` in J/K for a body of the supplied mass.
    ///
    /// Individual finite inputs are not sufficient: their product must also be
    /// representable as a finite positive `f64`, otherwise downstream energy
    /// accounting could report a transfer that produces no representable state
    /// change.
    pub fn heat_capacity(self, mass_kg: f64) -> Result<f64, ThermalError> {
        self.validate()?;
        validate_positive_finite(mass_kg, ThermalError::InvalidMass)?;
        let heat_capacity = mass_kg * self.specific_heat_capacity;
        validate_positive_finite(heat_capacity, ThermalError::InvalidHeatCapacity)?;
        Ok(heat_capacity)
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
        let state = Self { temperature_kelvin };
        state.validate()?;
        Ok(state)
    }

    /// Re-validate state after construction or public-field mutation.
    pub fn validate(self) -> Result<(), ThermalError> {
        validate_temperature(self.temperature_kelvin)
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
        self.validate()?;
        validate_temperature(reference_temperature_kelvin)?;
        let heat_capacity = material.heat_capacity(mass_kg)?;
        let energy = heat_capacity * (self.temperature_kelvin - reference_temperature_kelvin);
        if !energy.is_finite() {
            return Err(ThermalError::InvalidEnergy);
        }
        Ok(energy)
    }

    /// Add sensible heat to this state and return the resulting temperature.
    pub fn add_heat_joules(
        &mut self,
        energy_joules: f64,
        mass_kg: f64,
        material: ThermalMaterial,
    ) -> Result<f64, ThermalError> {
        self.validate()?;
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

/// Thermodynamic state attached to a physics body.
///
/// Thermal mass is explicit rather than borrowing rigid-body mass. That keeps
/// the model usable for static geometry and reduced-order thermal reservoirs,
/// where mechanical inverse mass may be zero even though the represented matter
/// has finite thermal inertia.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThermalBody {
    pub material: ThermalMaterial,
    pub state: ThermalState,
    /// Effective material mass participating in the lumped thermal model.
    pub thermal_mass_kg: f64,
}

impl ThermalBody {
    pub fn new(
        material: ThermalMaterial,
        state: ThermalState,
        thermal_mass_kg: f64,
    ) -> Result<Self, ThermalError> {
        let thermal = Self {
            material,
            state,
            thermal_mass_kg,
        };
        thermal.validate()?;
        Ok(thermal)
    }

    /// Re-validate the complete body-attached thermal reservoir.
    pub fn validate(self) -> Result<(), ThermalError> {
        self.material.validate()?;
        self.state.validate()?;
        validate_positive_finite(self.thermal_mass_kg, ThermalError::InvalidMass)?;
        self.material.heat_capacity(self.thermal_mass_kg)?;
        Ok(())
    }

    pub fn sensible_energy_joules(
        self,
        reference_temperature_kelvin: f64,
    ) -> Result<f64, ThermalError> {
        self.validate()?;
        self.state.sensible_energy_joules(
            reference_temperature_kelvin,
            self.thermal_mass_kg,
            self.material,
        )
    }

    pub fn add_heat_joules(&mut self, energy_joules: f64) -> Result<f64, ThermalError> {
        self.validate()?;
        self.state
            .add_heat_joules(energy_joules, self.thermal_mass_kg, self.material)
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

/// Errors returned when a thermodynamic state or derived quantity would be
/// physically invalid or unrepresentable in the current `f64` model.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ThermalError {
    InvalidTemperature,
    InvalidSpecificHeatCapacity,
    InvalidThermalConductivity,
    InvalidEmissivity,
    InvalidMass,
    InvalidHeatCapacity,
    InvalidConductance,
    InvalidTimestep,
    InvalidEnergy,
    MissingThermalState,
}

/// Exchange sensible heat between two lumped thermal states.
///
/// `conductance_w_per_k` is an effective pair conductance (`UA`) in W/K. It can
/// be derived by a caller from conductivity, contact area, and path length. The
/// transfer is exactly antisymmetric: energy removed from A is added to B.
/// Large timesteps are capped at the pair's equilibrium transfer, which prevents
/// a single step from crossing equilibrium and reversing the temperature order.
///
/// The update is transactional: both next temperatures are computed and
/// validated before either input state is mutated.
#[allow(clippy::too_many_arguments)]
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
    a.validate()?;
    b.validate()?;
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

    // Stable form of C_a*C_b/(C_a+C_b): using the smaller capacity avoids
    // overflow in the product and avoids reciprocal-sum overflow for tiny
    // positive capacities.
    let (smaller_capacity, larger_capacity) = if capacity_a <= capacity_b {
        (capacity_a, capacity_b)
    } else {
        (capacity_b, capacity_a)
    };
    let effective_pair_capacity =
        smaller_capacity / (1.0 + smaller_capacity / larger_capacity);
    let equilibrium_transfer = delta_t * effective_pair_capacity;

    // An overflowing requested Euler transfer still has a meaningful outcome
    // when the finite equilibrium limiter is representable: clamp to it. If the
    // selected physical transfer itself is not representable, fail without
    // mutating either state.
    let equilibrium_limited =
        !requested.is_finite() || requested.abs() > equilibrium_transfer.abs();
    let transfer = if equilibrium_limited {
        equilibrium_transfer
    } else {
        requested
    };
    if !transfer.is_finite() {
        return Err(ThermalError::InvalidEnergy);
    }

    let next_a = a.temperature_kelvin - transfer / capacity_a;
    let next_b = b.temperature_kelvin + transfer / capacity_b;
    validate_temperature(next_a)?;
    validate_temperature(next_b)?;

    a.temperature_kelvin = next_a;
    b.temperature_kelvin = next_b;

    Ok(HeatExchange {
        joules_from_a_to_b: transfer,
        equilibrium_limited,
    })
}

/// Convenience wrapper for conductive exchange between body-attached thermal states.
pub fn conductive_exchange_bodies(
    a: &mut ThermalBody,
    b: &mut ThermalBody,
    conductance_w_per_k: f64,
    dt_seconds: f64,
) -> Result<HeatExchange, ThermalError> {
    a.validate()?;
    b.validate()?;
    conductive_exchange(
        &mut a.state,
        a.material,
        a.thermal_mass_kg,
        &mut b.state,
        b.material,
        b.thermal_mass_kg,
        conductance_w_per_k,
        dt_seconds,
    )
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
        assert_eq!(
            ThermalBody::new(material(1000.0), ThermalState::new(300.0).unwrap(), 0.0),
            Err(ThermalError::InvalidMass)
        );
    }

    #[test]
    fn public_field_mutation_is_revalidated_before_accounting() {
        let mut thermal = ThermalBody::new(
            material(1000.0),
            ThermalState::new(300.0).unwrap(),
            1.0,
        )
        .unwrap();

        thermal.state.temperature_kelvin = -1.0;
        assert_eq!(thermal.validate(), Err(ThermalError::InvalidTemperature));
        assert_eq!(
            thermal.sensible_energy_joules(0.0),
            Err(ThermalError::InvalidTemperature)
        );

        thermal.state.temperature_kelvin = 300.0;
        thermal.material.emissivity = 1.5;
        assert_eq!(thermal.validate(), Err(ThermalError::InvalidEmissivity));

        thermal.material.emissivity = 0.5;
        thermal.material.specific_heat_capacity = -10.0;
        assert_eq!(
            thermal.validate(),
            Err(ThermalError::InvalidSpecificHeatCapacity)
        );
    }

    #[test]
    fn derived_heat_capacity_overflow_is_rejected() {
        let material = ThermalMaterial::new(f64::MAX, 1.0, 0.5).unwrap();
        assert_eq!(
            material.heat_capacity(2.0),
            Err(ThermalError::InvalidHeatCapacity)
        );
        assert_eq!(
            ThermalBody::new(material, ThermalState::new(300.0).unwrap(), 2.0),
            Err(ThermalError::InvalidHeatCapacity)
        );
    }

    #[test]
    fn sensible_energy_overflow_is_rejected() {
        let material = ThermalMaterial::new(f64::MAX / 4.0, 1.0, 0.5).unwrap();
        let state = ThermalState::new(10.0).unwrap();
        assert_eq!(
            state.sensible_energy_joules(0.0, 1.0, material),
            Err(ThermalError::InvalidEnergy)
        );
    }

    #[test]
    fn failed_unrepresentable_exchange_is_transactional() {
        let material = ThermalMaterial::new(f64::MAX / 2.0, 1.0, 0.5).unwrap();
        let mut a = ThermalState::new(f64::MAX).unwrap();
        let mut b = ThermalState::new(1.0).unwrap();
        let original_a = a;
        let original_b = b;

        assert_eq!(
            conductive_exchange(&mut a, material, 1.0, &mut b, material, 1.0, 2.0, 1.0),
            Err(ThermalError::InvalidEnergy)
        );
        assert_eq!(a, original_a);
        assert_eq!(b, original_b);
    }

    #[test]
    fn tiny_capacities_use_stable_equilibrium_limit() {
        let material = ThermalMaterial::new(1.0e-308, 1.0, 0.5).unwrap();
        let mut a = ThermalState::new(400.0).unwrap();
        let mut b = ThermalState::new(300.0).unwrap();

        let exchange = conductive_exchange(
            &mut a,
            material,
            1.0,
            &mut b,
            material,
            1.0,
            1.0,
            1.0,
        )
        .unwrap();

        assert!(exchange.equilibrium_limited);
        assert!(exchange.joules_from_a_to_b > 0.0);
        assert!((a.temperature_kelvin - 350.0).abs() < 1e-9);
        assert!((b.temperature_kelvin - 350.0).abs() < 1e-9);
    }

    #[test]
    fn conductive_exchange_conserves_pair_sensible_energy() {
        let mat_a = material(500.0);
        let mat_b = material(1000.0);
        let mut a = ThermalState::new(400.0).unwrap();
        let mut b = ThermalState::new(300.0).unwrap();
        let mass_a = 2.0;
        let mass_b = 3.0;

        let before = a
            .sensible_energy_joules(0.0, mass_a, mat_a)
            .unwrap()
            + b.sensible_energy_joules(0.0, mass_b, mat_b).unwrap();

        let exchange =
            conductive_exchange(&mut a, mat_a, mass_a, &mut b, mat_b, mass_b, 25.0, 0.5).unwrap();

        let after = a
            .sensible_energy_joules(0.0, mass_a, mat_a)
            .unwrap()
            + b.sensible_energy_joules(0.0, mass_b, mat_b).unwrap();

        assert!(exchange.joules_from_a_to_b > 0.0);
        assert!((after - before).abs() < 1e-8);
        assert!(a.temperature_kelvin < 400.0);
        assert!(b.temperature_kelvin > 300.0);
    }

    #[test]
    fn body_exchange_conserves_sensible_energy() {
        let mat = material(900.0);
        let mut a = ThermalBody::new(mat, ThermalState::new(450.0).unwrap(), 2.0).unwrap();
        let mut b = ThermalBody::new(mat, ThermalState::new(300.0).unwrap(), 1.0).unwrap();
        let before = a.sensible_energy_joules(0.0).unwrap()
            + b.sensible_energy_joules(0.0).unwrap();

        conductive_exchange_bodies(&mut a, &mut b, 50.0, 1.0).unwrap();

        let after = a.sensible_energy_joules(0.0).unwrap()
            + b.sensible_energy_joules(0.0).unwrap();
        assert!((after - before).abs() < 1e-8);
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

        let exchange = conductive_exchange(&mut a, mat, 1.0, &mut b, mat, 1.0, 10.0, 1.0).unwrap();

        assert!(exchange.joules_from_a_to_b < 0.0);
        assert!(a.temperature_kelvin > 250.0);
        assert!(b.temperature_kelvin < 350.0);
    }
}
