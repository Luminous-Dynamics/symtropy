// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Persistent energy reservoir for consciousness-coupled entities.
//!
//! Energy is a finite quantity. It depletes through movement, consciousness
//! maintenance, and collisions, and regenerates through explicit sources.
//!
//! This reservoir is part of the runtime accounting boundary, so requested
//! energy and actually accepted/consumed energy must never be conflated. Public
//! fields are retained for API compatibility; every checked mutation therefore
//! revalidates the reservoir and stages derived arithmetic before commit.

/// Persistent energy reservoir for a consciousness-coupled entity.
///
/// Tracks internal energy (U), temperature (T), and entropy (S). Available work
/// is modeled as Helmholtz free energy `F = U - TS`.
#[derive(Debug, Clone)]
pub struct EnergyBudget {
    /// Maximum energy capacity.
    pub max_energy: f64,
    /// Current internal energy U (persistent — does NOT reset each tick).
    pub available: f64,
    /// Entity temperature in Kelvin (rises with dissipated heat).
    pub temperature: f64,
    /// Cumulative entropy in J/K.
    pub entropy: f64,
    /// Heat capacity in J/K.
    pub heat_capacity: f64,
    /// Energy actually consumed this tick (reset each tick for telemetry).
    pub consumed_this_tick: f64,
    /// Energy actually accepted from regeneration this tick.
    pub regenerated_this_tick: f64,
    /// Cumulative energy actually consumed across all ticks.
    pub lifetime_consumed: f64,
    /// Whether this entity has collapsed (energy depleted to zero).
    pub collapsed: bool,
}

/// Typed validity failures for [`EnergyBudget`] and its checked mutations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyBudgetError {
    InvalidMaxEnergy,
    InvalidAvailableEnergy,
    AvailableExceedsCapacity,
    InvalidTemperature,
    InvalidEntropy,
    InvalidHeatCapacity,
    InvalidConsumedThisTick,
    InvalidRegeneratedThisTick,
    InvalidLifetimeConsumed,
    CollapsedStateMismatch,
    InvalidAmount,
    UnrepresentableArithmetic,
}

impl EnergyBudget {
    /// Construct a validated energy budget.
    pub fn try_new(max_energy: f64) -> Result<Self, EnergyBudgetError> {
        if !max_energy.is_finite() || max_energy < 0.0 {
            return Err(EnergyBudgetError::InvalidMaxEnergy);
        }

        let budget = Self {
            max_energy,
            available: max_energy,
            temperature: 310.0,
            entropy: 0.0,
            heat_capacity: 100.0,
            consumed_this_tick: 0.0,
            regenerated_this_tick: 0.0,
            lifetime_consumed: 0.0,
            collapsed: max_energy == 0.0,
        };
        budget.validate()?;
        Ok(budget)
    }

    /// Create a new energy budget with starting energy.
    ///
    /// # Panics
    ///
    /// Panics when `max_energy` is non-finite or negative. Runtime code that
    /// needs recoverable construction should use [`Self::try_new`].
    pub fn new(max_energy: f64) -> Self {
        Self::try_new(max_energy).expect("EnergyBudget max_energy must be finite and non-negative")
    }

    /// Revalidate the complete public reservoir state.
    ///
    /// Construction is not a permanent validity proof because the fields remain
    /// public for compatibility. Authoritative/accounting boundaries should use
    /// this method or one of the checked mutation helpers below.
    pub fn validate(&self) -> Result<(), EnergyBudgetError> {
        if !self.max_energy.is_finite() || self.max_energy < 0.0 {
            return Err(EnergyBudgetError::InvalidMaxEnergy);
        }
        if !self.available.is_finite() || self.available < 0.0 {
            return Err(EnergyBudgetError::InvalidAvailableEnergy);
        }
        if self.available > self.max_energy {
            return Err(EnergyBudgetError::AvailableExceedsCapacity);
        }
        if !self.temperature.is_finite() || self.temperature <= 0.0 {
            return Err(EnergyBudgetError::InvalidTemperature);
        }
        if !self.entropy.is_finite() || self.entropy < 0.0 {
            return Err(EnergyBudgetError::InvalidEntropy);
        }
        if !self.heat_capacity.is_finite() || self.heat_capacity <= 0.0 {
            return Err(EnergyBudgetError::InvalidHeatCapacity);
        }
        if !self.consumed_this_tick.is_finite() || self.consumed_this_tick < 0.0 {
            return Err(EnergyBudgetError::InvalidConsumedThisTick);
        }
        if !self.regenerated_this_tick.is_finite() || self.regenerated_this_tick < 0.0 {
            return Err(EnergyBudgetError::InvalidRegeneratedThisTick);
        }
        if !self.lifetime_consumed.is_finite() || self.lifetime_consumed < 0.0 {
            return Err(EnergyBudgetError::InvalidLifetimeConsumed);
        }
        if self.collapsed != (self.available == 0.0) {
            return Err(EnergyBudgetError::CollapsedStateMismatch);
        }
        Ok(())
    }

    /// Reset per-tick counters (call at start of each tick).
    ///
    /// This operation intentionally does not repair invalid persistent state; it
    /// only resets telemetry counters. A subsequent checked physical mutation
    /// will still reject any invalid reservoir fields.
    pub fn tick_reset(&mut self) {
        self.consumed_this_tick = 0.0;
        self.regenerated_this_tick = 0.0;
    }

    /// Consume energy transactionally and return the amount actually removed.
    pub fn consume_checked(&mut self, amount: f64) -> Result<f64, EnergyBudgetError> {
        self.validate()?;
        validate_amount(amount)?;
        if amount == 0.0 || self.collapsed {
            return Ok(0.0);
        }

        let actual = amount.min(self.available);
        let next_available = self.available - actual;
        let next_consumed = self.consumed_this_tick + actual;
        let next_lifetime = self.lifetime_consumed + actual;
        let entropy_increment = actual / self.temperature;
        let next_entropy = self.entropy + entropy_increment;

        if !next_available.is_finite()
            || !next_consumed.is_finite()
            || !next_lifetime.is_finite()
            || !entropy_increment.is_finite()
            || !next_entropy.is_finite()
        {
            return Err(EnergyBudgetError::UnrepresentableArithmetic);
        }

        self.available = next_available;
        self.consumed_this_tick = next_consumed;
        self.lifetime_consumed = next_lifetime;
        self.entropy = next_entropy;
        self.collapsed = self.available == 0.0;
        Ok(actual)
    }

    /// Try to consume energy. Invalid input/state fails closed and leaves the
    /// reservoir unchanged. Returns the amount actually consumed.
    #[inline]
    pub fn consume(&mut self, amount: f64) -> f64 {
        self.consume_checked(amount).unwrap_or(0.0)
    }

    /// Add regeneration transactionally and return the energy actually accepted.
    ///
    /// A full reservoir accepts `0 J`, so requested-but-clipped regeneration is
    /// never counted in `regenerated_this_tick`.
    pub fn regenerate_checked(&mut self, amount: f64) -> Result<f64, EnergyBudgetError> {
        self.validate()?;
        validate_amount(amount)?;
        if amount == 0.0 {
            return Ok(0.0);
        }

        let headroom = self.max_energy - self.available;
        if !headroom.is_finite() || headroom < 0.0 {
            return Err(EnergyBudgetError::UnrepresentableArithmetic);
        }
        let actual = amount.min(headroom);
        let next_available = self.available + actual;
        let next_regenerated = self.regenerated_this_tick + actual;

        if !next_available.is_finite() || !next_regenerated.is_finite() {
            return Err(EnergyBudgetError::UnrepresentableArithmetic);
        }

        self.available = next_available.min(self.max_energy);
        self.regenerated_this_tick = next_regenerated;
        self.collapsed = self.available == 0.0;
        Ok(actual)
    }

    /// Add energy from a regeneration source. Invalid input/state fails closed.
    #[inline]
    pub fn regenerate(&mut self, amount: f64) {
        let _ = self.regenerate_checked(amount);
    }

    /// Absorb heat transactionally and return the heat actually accepted.
    ///
    /// This model treats positive supplied heat as fully accepted when the
    /// temperature/entropy update remains representable.
    pub fn dissipate_heat_checked(&mut self, energy: f64) -> Result<f64, EnergyBudgetError> {
        self.validate()?;
        validate_amount(energy)?;
        if energy == 0.0 {
            return Ok(0.0);
        }

        let temperature_increment = energy / self.heat_capacity;
        let next_temperature = self.temperature + temperature_increment;
        if !temperature_increment.is_finite()
            || !next_temperature.is_finite()
            || next_temperature <= 0.0
        {
            return Err(EnergyBudgetError::UnrepresentableArithmetic);
        }

        // Preserve the established model's entropy convention: heat entropy is
        // evaluated at the post-heating temperature.
        let entropy_increment = energy / next_temperature;
        let next_entropy = self.entropy + entropy_increment;
        if !entropy_increment.is_finite() || !next_entropy.is_finite() {
            return Err(EnergyBudgetError::UnrepresentableArithmetic);
        }

        self.temperature = next_temperature;
        self.entropy = next_entropy;
        Ok(energy)
    }

    /// Absorb heat from dissipation. Invalid input/state fails closed.
    pub fn dissipate_heat(&mut self, energy: f64) {
        let _ = self.dissipate_heat_checked(energy);
    }

    /// Checked Helmholtz free energy: `F = U - TS`.
    pub fn available_work_checked(&self) -> Result<f64, EnergyBudgetError> {
        self.validate()?;
        let thermal_term = self.temperature * self.entropy;
        let work = self.available - thermal_term;
        if !thermal_term.is_finite() || !work.is_finite() {
            return Err(EnergyBudgetError::UnrepresentableArithmetic);
        }
        Ok(work.max(0.0))
    }

    /// Helmholtz free energy. Invalid/unrepresentable state fails closed to zero
    /// available work rather than returning a benign-looking NaN.
    pub fn available_work(&self) -> f64 {
        self.available_work_checked().unwrap_or(0.0)
    }

    /// Whether any energy is available.
    #[inline]
    pub fn has_energy(&self) -> bool {
        self.validate().is_ok() && self.available > 1e-10 && !self.collapsed
    }

    /// Whether this entity has collapsed (zero energy).
    #[inline]
    pub fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    /// Checked fraction of energy remaining in `[0, 1]`.
    pub fn fraction_remaining_checked(&self) -> Result<f64, EnergyBudgetError> {
        self.validate()?;
        if self.max_energy == 0.0 {
            return Ok(0.0);
        }
        let fraction = self.available / self.max_energy;
        if !fraction.is_finite() {
            return Err(EnergyBudgetError::UnrepresentableArithmetic);
        }
        Ok(fraction.clamp(0.0, 1.0))
    }

    /// Fraction of energy remaining. Invalid state fails closed to zero.
    #[inline]
    pub fn fraction_remaining(&self) -> f64 {
        self.fraction_remaining_checked().unwrap_or(0.0)
    }

    /// Checked net energy flow this tick (positive = regenerating).
    pub fn net_flow_this_tick_checked(&self) -> Result<f64, EnergyBudgetError> {
        self.validate()?;
        let flow = self.regenerated_this_tick - self.consumed_this_tick;
        if !flow.is_finite() {
            return Err(EnergyBudgetError::UnrepresentableArithmetic);
        }
        Ok(flow)
    }

    /// Net energy flow this tick. Invalid state fails closed to zero.
    pub fn net_flow_this_tick(&self) -> f64 {
        self.net_flow_this_tick_checked().unwrap_or(0.0)
    }
}

fn validate_amount(amount: f64) -> Result<(), EnergyBudgetError> {
    if !amount.is_finite() || amount < 0.0 {
        Err(EnergyBudgetError::InvalidAmount)
    } else {
        Ok(())
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
        assert_eq!(budget.validate(), Ok(()));
    }

    #[test]
    fn invalid_construction_is_recoverable_through_try_new() {
        assert_eq!(
            EnergyBudget::try_new(f64::NAN).unwrap_err(),
            EnergyBudgetError::InvalidMaxEnergy
        );
        assert_eq!(
            EnergyBudget::try_new(-1.0).unwrap_err(),
            EnergyBudgetError::InvalidMaxEnergy
        );
        let zero = EnergyBudget::try_new(0.0).unwrap();
        assert!(zero.collapsed);
        assert_eq!(zero.validate(), Ok(()));
    }

    #[test]
    fn consume_reduces_available() {
        let mut budget = EnergyBudget::new(100.0);
        let consumed = budget.consume(30.0);
        assert!((consumed - 30.0).abs() < 1e-10);
        assert!((budget.available - 70.0).abs() < 1e-10);
        assert!((budget.consumed_this_tick - 30.0).abs() < 1e-10);
    }

    #[test]
    fn consume_capped_by_available() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(80.0);
        let consumed = budget.consume(50.0);
        assert!((consumed - 20.0).abs() < 1e-10);
        assert!(budget.is_collapsed());
    }

    #[test]
    fn negative_and_non_finite_consumption_fail_closed() {
        for amount in [-1.0, f64::NAN, f64::INFINITY] {
            let mut budget = EnergyBudget::new(100.0);
            let before = budget.clone();
            assert_eq!(budget.consume(amount), 0.0);
            assert_eq!(budget.available, before.available);
            assert_eq!(budget.consumed_this_tick, before.consumed_this_tick);
            assert_eq!(budget.lifetime_consumed, before.lifetime_consumed);
            assert_eq!(budget.entropy, before.entropy);
        }
    }

    #[test]
    fn collapsed_budget_cannot_consume() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(100.0);
        assert!(budget.is_collapsed());
        assert!(!budget.has_energy());
        assert_eq!(budget.consume(10.0), 0.0);
    }

    #[test]
    fn regeneration_counts_only_energy_actually_accepted() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(30.0);
        budget.tick_reset();

        let accepted = budget.regenerate_checked(50.0).unwrap();
        assert!((accepted - 30.0).abs() < 1e-10);
        assert!((budget.available - 100.0).abs() < 1e-10);
        assert!((budget.regenerated_this_tick - 30.0).abs() < 1e-10);
    }

    #[test]
    fn full_reservoir_does_not_report_phantom_regeneration() {
        let mut budget = EnergyBudget::new(100.0);
        let accepted = budget.regenerate_checked(50.0).unwrap();
        assert_eq!(accepted, 0.0);
        assert_eq!(budget.available, 100.0);
        assert_eq!(budget.regenerated_this_tick, 0.0);
    }

    #[test]
    fn invalid_regeneration_fails_closed_without_mutation() {
        for amount in [-1.0, f64::NAN, f64::INFINITY] {
            let mut budget = EnergyBudget::new(100.0);
            budget.consume(20.0);
            budget.tick_reset();
            let before = budget.clone();
            budget.regenerate(amount);
            assert_eq!(budget.available, before.available);
            assert_eq!(budget.regenerated_this_tick, before.regenerated_this_tick);
            assert_eq!(budget.collapsed, before.collapsed);
        }
    }

    #[test]
    fn regeneration_recovers_from_collapse_only_when_energy_is_accepted() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(100.0);
        budget.tick_reset();
        assert!(budget.is_collapsed());

        let accepted = budget.regenerate_checked(10.0).unwrap();
        assert_eq!(accepted, 10.0);
        assert!(!budget.is_collapsed());
        assert!(budget.has_energy());
        assert_eq!(budget.regenerated_this_tick, 10.0);
    }

    #[test]
    fn heat_update_is_transactional_and_rejects_invalid_input() {
        let mut budget = EnergyBudget::new(100.0);
        let initial_temperature = budget.temperature;
        let initial_entropy = budget.entropy;
        assert_eq!(budget.dissipate_heat_checked(100.0).unwrap(), 100.0);
        assert!(budget.temperature > initial_temperature);
        assert!(budget.entropy > initial_entropy);

        let before = budget.clone();
        budget.dissipate_heat(f64::NAN);
        assert_eq!(budget.temperature, before.temperature);
        assert_eq!(budget.entropy, before.entropy);
    }

    #[test]
    fn public_field_corruption_is_detected_and_mutations_do_not_repair_it() {
        let mut budget = EnergyBudget::new(100.0);
        budget.available = f64::NAN;
        assert_eq!(
            budget.validate(),
            Err(EnergyBudgetError::InvalidAvailableEnergy)
        );
        let consumed_before = budget.consumed_this_tick;
        let regenerated_before = budget.regenerated_this_tick;
        assert_eq!(budget.consume(10.0), 0.0);
        budget.regenerate(10.0);
        budget.dissipate_heat(10.0);
        assert!(budget.available.is_nan());
        assert_eq!(budget.consumed_this_tick, consumed_before);
        assert_eq!(budget.regenerated_this_tick, regenerated_before);
    }

    #[test]
    fn counter_overflow_rejects_mutation_transactionally() {
        let mut budget = EnergyBudget::new(f64::MAX);
        budget.available = 0.0;
        budget.collapsed = true;
        budget.regenerated_this_tick = f64::MAX;
        let before = budget.clone();
        assert_eq!(
            budget.regenerate_checked(1.0),
            Err(EnergyBudgetError::UnrepresentableArithmetic)
        );
        assert_eq!(budget.available, before.available);
        assert_eq!(budget.regenerated_this_tick, before.regenerated_this_tick);
        assert_eq!(budget.collapsed, before.collapsed);
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
    fn net_flow_tracks_actual_transfer() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(10.0);
        budget.regenerate(15.0);
        // 10 J of headroom existed, so only 10 J can actually regenerate.
        assert!((budget.net_flow_this_tick() - 0.0).abs() < 1e-10);
        assert!((budget.regenerated_this_tick - 10.0).abs() < 1e-10);
    }

    #[test]
    fn tick_reset_clears_per_tick_but_not_persistent_energy() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(10.0);
        budget.regenerate(5.0);
        let available = budget.available;
        budget.tick_reset();
        assert_eq!(budget.consumed_this_tick, 0.0);
        assert_eq!(budget.regenerated_this_tick, 0.0);
        assert_eq!(budget.available, available);
    }

    #[test]
    fn fraction_remaining_is_bounded_and_invalid_state_fails_closed() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(25.0);
        assert!((budget.fraction_remaining() - 0.75).abs() < 1e-10);

        budget.available = f64::NAN;
        assert_eq!(budget.fraction_remaining(), 0.0);
        assert_eq!(budget.available_work(), 0.0);
        assert_eq!(budget.net_flow_this_tick(), 0.0);
    }
}
