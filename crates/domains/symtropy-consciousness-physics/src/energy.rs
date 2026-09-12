// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Persistent energy reservoir for consciousness-coupled entities.
//!
//! Energy is a finite quantity. It depletes through movement,
//! consciousness maintenance, and collisions. It regenerates through
//! harmony resonance, energy wells, and slow ambient recovery.
//!
//! When usable energy reaches zero, the entity collapses, motor output halts,
//! and recovery requires a positive regeneration transfer.

use std::fmt;

const ENERGY_EPSILON: f64 = 1e-10;
const DEFAULT_TEMPERATURE_K: f64 = 310.0;
const DEFAULT_HEAT_CAPACITY_J_PER_K: f64 = 100.0;

/// Validation failures for persisted or externally reconstructed energy state.
///
/// Validation reports corruption; it never silently repairs historical state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyBudgetStateError {
    InvalidCapacity,
    InvalidAvailableEnergy,
    InvalidTemperature,
    InvalidEntropy,
    InvalidHeatCapacity,
    InvalidConsumedCounter,
    InvalidRegeneratedCounter,
    InvalidLifetimeConsumed,
    CollapseMismatch,
}

impl fmt::Display for EnergyBudgetStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidCapacity => "energy capacity must be finite and non-negative",
            Self::InvalidAvailableEnergy => {
                "available energy must be finite and within [0, max_energy]"
            }
            Self::InvalidTemperature => "temperature must be finite and greater than 0 K",
            Self::InvalidEntropy => "entropy must be finite and non-negative",
            Self::InvalidHeatCapacity => "heat capacity must be finite and greater than zero",
            Self::InvalidConsumedCounter => {
                "consumed-this-tick must be finite and non-negative"
            }
            Self::InvalidRegeneratedCounter => {
                "regenerated-this-tick must be finite and non-negative"
            }
            Self::InvalidLifetimeConsumed => {
                "lifetime consumed energy must be finite and non-negative"
            }
            Self::CollapseMismatch => {
                "collapsed flag is inconsistent with the usable-energy threshold"
            }
        };
        f.write_str(message)
    }
}

impl std::error::Error for EnergyBudgetStateError {}

/// Persistent energy reservoir for consciousness-coupled entities.
///
/// Tracks internal energy (U), temperature (T), and entropy (S)
/// for 2nd Law accounting. Available work = Helmholtz free energy F = U - TS.
#[derive(Debug, Clone)]
pub struct EnergyBudget {
    /// Maximum energy capacity.
    pub max_energy: f64,
    /// Current internal energy U (persistent — does NOT reset each tick).
    pub available: f64,
    /// Entity temperature in Kelvin (rises with dissipated heat).
    pub temperature: f64,
    /// Cumulative entropy in J/K (monotonically increases per 2nd Law).
    pub entropy: f64,
    /// Heat capacity in J/K.
    pub heat_capacity: f64,
    /// Total energy consumed this tick (reset each tick for telemetry).
    pub consumed_this_tick: f64,
    /// Total energy actually regenerated this tick (reset each tick for telemetry).
    pub regenerated_this_tick: f64,
    /// Cumulative energy spent across all ticks.
    pub lifetime_consumed: f64,
    /// Whether this entity has collapsed (no usable energy).
    pub collapsed: bool,
}

impl EnergyBudget {
    fn zero_collapsed() -> Self {
        Self {
            max_energy: 0.0,
            available: 0.0,
            temperature: DEFAULT_TEMPERATURE_K,
            entropy: 0.0,
            heat_capacity: DEFAULT_HEAT_CAPACITY_J_PER_K,
            consumed_this_tick: 0.0,
            regenerated_this_tick: 0.0,
            lifetime_consumed: 0.0,
            collapsed: true,
        }
    }

    /// Create a validated energy budget.
    ///
    /// Zero capacity is a valid, explicitly collapsed reservoir. Negative and
    /// non-finite capacities are rejected rather than normalized.
    pub fn try_new(max_energy: f64) -> Result<Self, EnergyBudgetStateError> {
        if !max_energy.is_finite() || max_energy < 0.0 {
            return Err(EnergyBudgetStateError::InvalidCapacity);
        }

        let budget = Self {
            max_energy,
            available: max_energy,
            temperature: DEFAULT_TEMPERATURE_K,
            entropy: 0.0,
            heat_capacity: DEFAULT_HEAT_CAPACITY_J_PER_K,
            consumed_this_tick: 0.0,
            regenerated_this_tick: 0.0,
            lifetime_consumed: 0.0,
            collapsed: max_energy <= ENERGY_EPSILON,
        };
        budget.validate()?;
        Ok(budget)
    }

    /// Compatibility constructor for trusted/static configuration.
    ///
    /// Valid finite non-negative capacities behave like [`Self::try_new`]. Invalid
    /// capacity fails closed into a zero-capacity collapsed reservoir so legacy callers
    /// cannot create NaN/∞/negative persistent state. Callers that need to distinguish
    /// configuration errors should use [`Self::try_new`] instead.
    pub fn new(max_energy: f64) -> Self {
        Self::try_new(max_energy).unwrap_or_else(|_| Self::zero_collapsed())
    }

    /// Validate the complete persistent state without modifying it.
    ///
    /// This is the persistence/network boundary: corrupted historical state is reported
    /// to the caller rather than silently clamped into a different history.
    pub fn validate(&self) -> Result<(), EnergyBudgetStateError> {
        if !self.max_energy.is_finite() || self.max_energy < 0.0 {
            return Err(EnergyBudgetStateError::InvalidCapacity);
        }
        if !self.available.is_finite()
            || self.available < 0.0
            || self.available > self.max_energy
        {
            return Err(EnergyBudgetStateError::InvalidAvailableEnergy);
        }
        if !self.temperature.is_finite() || self.temperature <= 0.0 {
            return Err(EnergyBudgetStateError::InvalidTemperature);
        }
        if !self.entropy.is_finite() || self.entropy < 0.0 {
            return Err(EnergyBudgetStateError::InvalidEntropy);
        }
        if !self.heat_capacity.is_finite() || self.heat_capacity <= 0.0 {
            return Err(EnergyBudgetStateError::InvalidHeatCapacity);
        }
        if !self.consumed_this_tick.is_finite() || self.consumed_this_tick < 0.0 {
            return Err(EnergyBudgetStateError::InvalidConsumedCounter);
        }
        if !self.regenerated_this_tick.is_finite() || self.regenerated_this_tick < 0.0 {
            return Err(EnergyBudgetStateError::InvalidRegeneratedCounter);
        }
        if !self.lifetime_consumed.is_finite() || self.lifetime_consumed < 0.0 {
            return Err(EnergyBudgetStateError::InvalidLifetimeConsumed);
        }

        let should_be_collapsed = self.available <= ENERGY_EPSILON;
        if self.collapsed != should_be_collapsed {
            return Err(EnergyBudgetStateError::CollapseMismatch);
        }

        Ok(())
    }

    /// Whether all persistent state invariants currently hold.
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }

    /// Reset per-tick counters (call at start of each tick).
    ///
    /// Counter reset is deliberately allowed even when another persistent field is
    /// invalid so telemetry corruption can be cleared before a state is quarantined.
    pub fn tick_reset(&mut self) {
        self.consumed_this_tick = 0.0;
        self.regenerated_this_tick = 0.0;
    }

    /// Try to consume finite positive energy. Returns the amount actually consumed
    /// (may be less than requested if the budget is exhausted).
    ///
    /// Requests fail closed if either the input or pre-existing reservoir state is
    /// invalid. The update is transactional: all resulting counters/entropy are checked
    /// for finiteness before any field is committed.
    #[inline]
    pub fn consume(&mut self, amount: f64) -> f64 {
        if self.collapsed
            || !amount.is_finite()
            || amount <= 0.0
            || self.validate().is_err()
        {
            return 0.0;
        }

        let actual = amount.min(self.available);
        if !actual.is_finite() || actual <= 0.0 {
            return 0.0;
        }

        let mut new_available = self.available - actual;
        if !new_available.is_finite() || new_available < 0.0 {
            return 0.0;
        }
        if new_available <= ENERGY_EPSILON {
            new_available = 0.0;
        }

        let entropy_delta = actual / self.temperature;
        let new_entropy = self.entropy + entropy_delta;
        let new_consumed = self.consumed_this_tick + actual;
        let new_lifetime = self.lifetime_consumed + actual;
        if !entropy_delta.is_finite()
            || entropy_delta < 0.0
            || !new_entropy.is_finite()
            || !new_consumed.is_finite()
            || !new_lifetime.is_finite()
        {
            return 0.0;
        }

        self.available = new_available;
        self.entropy = new_entropy;
        self.consumed_this_tick = new_consumed;
        self.lifetime_consumed = new_lifetime;
        self.collapsed = self.available <= ENERGY_EPSILON;
        actual
    }

    /// Absorb finite positive heat from dissipation (damping, friction, collision).
    /// Increases temperature and entropy per 2nd Law.
    ///
    /// Invalid input or invalid pre-existing reservoir state fails closed. The update is
    /// computed first and only committed when the resulting thermal state remains finite.
    pub fn dissipate_heat(&mut self, energy: f64) {
        if !energy.is_finite() || energy <= 0.0 || self.validate().is_err() {
            return;
        }

        let temperature_delta = energy / self.heat_capacity;
        if !temperature_delta.is_finite() || temperature_delta <= 0.0 {
            return;
        }
        let new_temperature = self.temperature + temperature_delta;
        if !new_temperature.is_finite() || new_temperature <= 0.0 {
            return;
        }
        let entropy_delta = energy / new_temperature;
        let new_entropy = self.entropy + entropy_delta;
        if !entropy_delta.is_finite()
            || entropy_delta < 0.0
            || !new_entropy.is_finite()
            || new_entropy < 0.0
        {
            return;
        }

        self.temperature = new_temperature;
        self.entropy = new_entropy;
    }

    /// Helmholtz free energy: maximum extractable work at constant T.
    /// F = U - TS.
    ///
    /// Invalid/corrupted state fails closed to zero extractable work.
    pub fn available_work(&self) -> f64 {
        if self.validate().is_err() {
            return 0.0;
        }
        let thermal_term = self.temperature * self.entropy;
        if !thermal_term.is_finite() {
            return 0.0;
        }
        let work = self.available - thermal_term;
        if work.is_finite() {
            work.max(0.0)
        } else {
            0.0
        }
    }

    /// Add energy from a regeneration source, capped at `max_energy`.
    ///
    /// Returns the amount that actually entered the reservoir. Source-backed callers
    /// can therefore debit their finite source by the accepted transfer rather than by
    /// an offer that the reservoir may not have had capacity to receive.
    ///
    /// Invalid input or invalid pre-existing state fails closed without mutation.
    #[inline]
    pub fn regenerate(&mut self, amount: f64) -> f64 {
        if !amount.is_finite() || amount <= 0.0 || self.validate().is_err() {
            return 0.0;
        }

        let room = self.max_energy - self.available;
        if !room.is_finite() || room <= 0.0 {
            return 0.0;
        }
        let actual = amount.min(room);
        if !actual.is_finite() || actual <= 0.0 {
            return 0.0;
        }

        let new_available = self.available + actual;
        let new_regenerated = self.regenerated_this_tick + actual;
        if !new_available.is_finite()
            || new_available < 0.0
            || new_available > self.max_energy
            || !new_regenerated.is_finite()
        {
            return 0.0;
        }

        self.available = new_available;
        self.regenerated_this_tick = new_regenerated;
        self.collapsed = self.available <= ENERGY_EPSILON;
        actual
    }

    /// Whether usable energy remains and the reservoir state is valid.
    #[inline]
    pub fn has_energy(&self) -> bool {
        self.validate().is_ok() && self.available > ENERGY_EPSILON && !self.collapsed
    }

    /// Whether this entity should be treated as collapsed.
    ///
    /// Invalid state fails closed to collapsed. Call [`Self::validate`] when the caller
    /// needs to distinguish genuine depletion from corrupted persisted state.
    #[inline]
    pub fn is_collapsed(&self) -> bool {
        self.validate().map(|_| self.collapsed).unwrap_or(true)
    }

    /// Fraction of energy remaining [0.0, 1.0].
    /// Invalid/corrupted state fails closed to zero.
    #[inline]
    pub fn fraction_remaining(&self) -> f64 {
        if self.validate().is_err() || self.max_energy <= ENERGY_EPSILON {
            return 0.0;
        }
        (self.available / self.max_energy).clamp(0.0, 1.0)
    }

    /// Net energy flow this tick (positive = regenerating, negative = depleting).
    /// Invalid/corrupted state fails closed to zero telemetry.
    pub fn net_flow_this_tick(&self) -> f64 {
        if self.validate().is_err() {
            return 0.0;
        }
        let flow = self.regenerated_this_tick - self.consumed_this_tick;
        if flow.is_finite() { flow } else { 0.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_budget_has_full_energy() {
        let budget = EnergyBudget::new(100.0);
        assert!((budget.available - 100.0).abs() < 1e-10);
        assert!(!budget.collapsed);
        assert!(budget.is_valid());
    }

    #[test]
    fn zero_capacity_is_explicitly_collapsed() {
        let budget = EnergyBudget::try_new(0.0).expect("zero capacity is a valid reservoir");
        assert_eq!(budget.max_energy, 0.0);
        assert_eq!(budget.available, 0.0);
        assert!(budget.collapsed);
        assert!(budget.is_collapsed());
        assert!(budget.is_valid());
    }

    #[test]
    fn try_new_rejects_invalid_capacity() {
        for capacity in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                EnergyBudget::try_new(capacity).unwrap_err(),
                EnergyBudgetStateError::InvalidCapacity
            );
        }
    }

    #[test]
    fn compatibility_new_fails_closed_for_invalid_capacity() {
        for capacity in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let budget = EnergyBudget::new(capacity);
            assert_eq!(budget.max_energy, 0.0);
            assert_eq!(budget.available, 0.0);
            assert!(budget.is_collapsed());
            assert!(budget.is_valid());
        }
    }

    #[test]
    fn validation_reports_corrupted_persistent_state_without_repairing_it() {
        let mut budget = EnergyBudget::new(100.0);
        budget.available = 101.0;
        assert_eq!(
            budget.validate(),
            Err(EnergyBudgetStateError::InvalidAvailableEnergy)
        );
        assert_eq!(budget.available, 101.0, "validation must not rewrite history");

        let mut budget = EnergyBudget::new(100.0);
        budget.temperature = f64::NAN;
        assert_eq!(
            budget.validate(),
            Err(EnergyBudgetStateError::InvalidTemperature)
        );
        assert!(budget.temperature.is_nan());

        let mut budget = EnergyBudget::new(100.0);
        budget.collapsed = true;
        assert_eq!(
            budget.validate(),
            Err(EnergyBudgetStateError::CollapseMismatch)
        );
    }

    #[test]
    fn consume_reduces_available() {
        let mut budget = EnergyBudget::new(100.0);
        let consumed = budget.consume(30.0);
        assert!((consumed - 30.0).abs() < 1e-10);
        assert!((budget.available - 70.0).abs() < 1e-10);
    }

    #[test]
    fn consume_capped_by_available() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(80.0);
        let consumed = budget.consume(50.0); // only 20 left
        assert!((consumed - 20.0).abs() < 1e-10);
        assert!(budget.is_collapsed());
    }

    #[test]
    fn consume_rejects_non_positive_and_non_finite_requests() {
        let mut budget = EnergyBudget::new(100.0);

        for amount in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!((budget.consume(amount) - 0.0).abs() < 1e-10);
        }

        assert!((budget.available - 100.0).abs() < 1e-10);
        assert!((budget.consumed_this_tick - 0.0).abs() < 1e-10);
        assert!((budget.lifetime_consumed - 0.0).abs() < 1e-10);
        assert!(!budget.is_collapsed());
    }

    #[test]
    fn consume_rejects_invalid_existing_state_without_partial_mutation() {
        let mut budget = EnergyBudget::new(100.0);
        budget.temperature = f64::NAN;
        let available_before = budget.available;
        let entropy_before = budget.entropy;
        let lifetime_before = budget.lifetime_consumed;

        assert_eq!(budget.consume(10.0), 0.0);
        assert_eq!(budget.available, available_before);
        assert_eq!(budget.entropy, entropy_before);
        assert_eq!(budget.lifetime_consumed, lifetime_before);
    }

    #[test]
    fn collapse_on_zero_energy() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(100.0);
        assert!(budget.is_collapsed());
        assert!(!budget.has_energy());

        let consumed = budget.consume(10.0);
        assert!((consumed - 0.0).abs() < 1e-10);
    }

    #[test]
    fn tiny_residual_energy_collapses_to_zero_consistently() {
        let mut budget = EnergyBudget::new(1.0);
        let consumed = budget.consume(1.0 - ENERGY_EPSILON / 2.0);
        assert!(consumed > 0.0);
        assert_eq!(budget.available, 0.0);
        assert!(budget.collapsed);
        assert!(budget.is_valid());
    }

    #[test]
    fn dissipate_heat_increases_finite_temperature_and_entropy() {
        let mut budget = EnergyBudget::new(100.0);
        let temperature_before = budget.temperature;
        let entropy_before = budget.entropy;

        budget.dissipate_heat(50.0);

        assert!(budget.temperature.is_finite());
        assert!(budget.entropy.is_finite());
        assert!(budget.temperature > temperature_before);
        assert!(budget.entropy > entropy_before);
    }

    #[test]
    fn dissipate_heat_rejects_invalid_energy_without_poisoning_state() {
        for energy in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut budget = EnergyBudget::new(100.0);
            let temperature_before = budget.temperature;
            let entropy_before = budget.entropy;

            budget.dissipate_heat(energy);

            assert_eq!(budget.temperature, temperature_before);
            assert_eq!(budget.entropy, entropy_before);
        }
    }

    #[test]
    fn dissipate_heat_rejects_invalid_existing_state_without_further_mutation() {
        let mut bad_capacity = EnergyBudget::new(100.0);
        bad_capacity.heat_capacity = f64::NAN;
        let temperature_before = bad_capacity.temperature;
        let entropy_before = bad_capacity.entropy;
        bad_capacity.dissipate_heat(10.0);
        assert_eq!(bad_capacity.temperature, temperature_before);
        assert_eq!(bad_capacity.entropy, entropy_before);

        let mut bad_temperature = EnergyBudget::new(100.0);
        bad_temperature.temperature = f64::NAN;
        let entropy_before = bad_temperature.entropy;
        bad_temperature.dissipate_heat(10.0);
        assert!(bad_temperature.temperature.is_nan());
        assert_eq!(bad_temperature.entropy, entropy_before);
    }

    #[test]
    fn available_work_fails_closed_on_invalid_state() {
        let mut budget = EnergyBudget::new(100.0);
        budget.temperature = f64::NAN;
        assert_eq!(budget.available_work(), 0.0);
        assert!(!budget.has_energy());
        assert!(budget.is_collapsed());
        assert_eq!(budget.fraction_remaining(), 0.0);
        assert_eq!(budget.net_flow_this_tick(), 0.0);
    }

    #[test]
    fn regenerate_adds_energy_and_returns_actual_gain() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(50.0);
        let accepted = budget.regenerate(20.0);
        assert!((accepted - 20.0).abs() < 1e-10);
        assert!((budget.available - 70.0).abs() < 1e-10);
        assert!((budget.regenerated_this_tick - 20.0).abs() < 1e-10);
    }

    #[test]
    fn regenerate_capped_at_max_returns_and_records_zero_gain() {
        let mut budget = EnergyBudget::new(100.0);
        let accepted = budget.regenerate(50.0); // already at max
        assert!((accepted - 0.0).abs() < 1e-10);
        assert!((budget.available - 100.0).abs() < 1e-10);
        assert!((budget.regenerated_this_tick - 0.0).abs() < 1e-10);
    }

    #[test]
    fn partial_regeneration_returns_and_records_only_remaining_capacity() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(5.0);
        budget.tick_reset();

        let accepted = budget.regenerate(20.0);

        assert!((accepted - 5.0).abs() < 1e-10);
        assert!((budget.available - 100.0).abs() < 1e-10);
        assert!((budget.regenerated_this_tick - 5.0).abs() < 1e-10);
        assert!((budget.net_flow_this_tick() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn regenerate_recovers_from_collapse_and_returns_restored_energy() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(100.0);
        assert!(budget.is_collapsed());
        budget.tick_reset();

        let accepted = budget.regenerate(10.0);

        assert!((accepted - 10.0).abs() < 1e-10);
        assert!(!budget.is_collapsed());
        assert!(budget.has_energy());
        assert!((budget.available - 10.0).abs() < 1e-10);
        assert!((budget.regenerated_this_tick - 10.0).abs() < 1e-10);
    }

    #[test]
    fn regenerate_rejects_non_positive_and_non_finite_requests() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(50.0);
        budget.tick_reset();

        for amount in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!((budget.regenerate(amount) - 0.0).abs() < 1e-10);
        }

        assert!((budget.available - 50.0).abs() < 1e-10);
        assert!((budget.regenerated_this_tick - 0.0).abs() < 1e-10);
    }

    #[test]
    fn regenerate_rejects_invalid_existing_state_without_mutation() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(50.0);
        budget.tick_reset();
        budget.entropy = f64::NAN;
        let available_before = budget.available;

        assert_eq!(budget.regenerate(10.0), 0.0);
        assert_eq!(budget.available, available_before);
        assert_eq!(budget.regenerated_this_tick, 0.0);
    }

    #[test]
    fn lifetime_accumulates() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(30.0);
        budget.tick_reset();
        budget.consume(20.0);
        assert!((budget.lifetime_consumed - 50.0).abs() < 1e-10);
    }

    #[test]
    fn net_flow_tracks_correctly() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(20.0);
        budget.tick_reset();
        let accepted = budget.regenerate(15.0);
        assert!((accepted - 15.0).abs() < 1e-10);
        assert!((budget.net_flow_this_tick() - 15.0).abs() < 1e-10);
    }

    #[test]
    fn tick_reset_clears_per_tick() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(10.0);
        let _ = budget.regenerate(5.0);
        budget.tick_reset();
        assert!((budget.consumed_this_tick - 0.0).abs() < 1e-10);
        assert!((budget.regenerated_this_tick - 0.0).abs() < 1e-10);
        assert!((budget.available - 95.0).abs() < 1e-10);
    }

    #[test]
    fn fraction_remaining() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(25.0);
        assert!((budget.fraction_remaining() - 0.75).abs() < 1e-10);
    }
}
