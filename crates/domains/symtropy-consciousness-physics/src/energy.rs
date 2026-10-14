// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Persistent energy reservoir for consciousness-coupled entities.
//!
//! Energy is a finite, conserved quantity. It depletes through movement,
//! consciousness maintenance, and collisions. It regenerates through
//! harmony resonance, energy wells, and slow ambient recovery.
//!
//! When energy reaches zero, the entity collapses — Φ drops to 0,
//! motor output halts, and recovery requires another agent's help.

/// Validation failures for energy-budget capacity, heat, or numeric state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyBudgetError {
    InvalidCapacity,
    InvalidEnergyAmount,
    InvalidState,
    NumericalOverflow,
}

impl std::fmt::Display for EnergyBudgetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCapacity => write!(f, "energy capacity must be finite and nonnegative"),
            Self::InvalidEnergyAmount => write!(f, "energy input must be finite and positive"),
            Self::InvalidState => write!(f, "energy budget state is outside its valid domain"),
            Self::NumericalOverflow => write!(f, "energy transfer produced a non-finite result"),
        }
    }
}

impl std::error::Error for EnergyBudgetError {}

/// Persistent energy reservoir for a consciousness-coupled entity.
///
/// Tracks internal energy (U), temperature (T), and entropy (S)
/// for 2nd Law compliance. Available work = Helmholtz free energy F = U - TS.
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
    /// Total energy regenerated this tick (reset each tick for telemetry).
    pub regenerated_this_tick: f64,
    /// Cumulative energy spent across all ticks.
    pub lifetime_consumed: f64,
    /// Whether this entity has collapsed (energy depleted to zero).
    pub collapsed: bool,
}

impl EnergyBudget {
    /// Create a new energy budget with starting energy.
    ///
    /// Invalid capacities produce a safe inert budget rather than propagating
    /// NaN, infinity, or negative energy into later calculations. Call try_new
    /// when the caller needs an explicit validation error.
    pub fn new(max_energy: f64) -> Self {
        Self::try_new(max_energy).unwrap_or_else(|_| Self::inert())
    }

    /// Create an energy budget after validating its capacity.
    ///
    /// A finite zero capacity is permitted and creates an initially collapsed
    /// budget. Negative or non-finite capacities are rejected.
    pub fn try_new(max_energy: f64) -> Result<Self, EnergyBudgetError> {
        if !max_energy.is_finite() || max_energy < 0.0 {
            return Err(EnergyBudgetError::InvalidCapacity);
        }
        Ok(Self {
            max_energy,
            available: max_energy,
            temperature: 310.0,
            entropy: 0.0,
            heat_capacity: 100.0,
            consumed_this_tick: 0.0,
            regenerated_this_tick: 0.0,
            lifetime_consumed: 0.0,
            collapsed: max_energy == 0.0,
        })
    }

    fn inert() -> Self {
        Self {
            max_energy: 0.0,
            available: 0.0,
            temperature: 310.0,
            entropy: 0.0,
            heat_capacity: 100.0,
            consumed_this_tick: 0.0,
            regenerated_this_tick: 0.0,
            lifetime_consumed: 0.0,
            collapsed: true,
        }
    }

    /// Whether the public state remains inside its physical/numeric domain.
    ///
    /// Fields remain public for API compatibility, so every mutation method
    /// validates the state before changing it.
    fn state_is_valid(&self) -> bool {
        self.max_energy.is_finite()
            && self.max_energy >= 0.0
            && self.available.is_finite()
            && self.available >= 0.0
            && self.available <= self.max_energy
            && self.temperature.is_finite()
            && self.temperature > 0.0
            && self.heat_capacity.is_finite()
            && self.heat_capacity > 0.0
            && self.entropy.is_finite()
            && self.entropy >= 0.0
            && self.consumed_this_tick.is_finite()
            && self.consumed_this_tick >= 0.0
            && self.regenerated_this_tick.is_finite()
            && self.regenerated_this_tick >= 0.0
            && self.lifetime_consumed.is_finite()
            && self.lifetime_consumed >= 0.0
            && ((self.available == 0.0 && self.collapsed)
                || (self.available > 0.0 && !self.collapsed))
    }

    /// Reset per-tick counters (call at start of each tick).
    pub fn tick_reset(&mut self) {
        self.consumed_this_tick = 0.0;
        self.regenerated_this_tick = 0.0;
    }

    /// Try to consume energy. Returns the amount actually consumed
    /// (may be less than requested if budget is exhausted).
    ///
    /// Invalid, negative, and non-finite requests fail closed: no state changes
    /// occur and the returned amount is zero.
    #[inline]
    pub fn consume(&mut self, amount: f64) -> f64 {
        if !self.state_is_valid()
            || self.collapsed
            || !amount.is_finite()
            || amount <= 0.0
        {
            return 0.0;
        }

        let actual = amount.min(self.available);
        if actual <= 0.0 || !actual.is_finite() {
            return 0.0;
        }
        let new_available = (self.available - actual).max(0.0);
        let new_consumed_this_tick = self.consumed_this_tick + actual;
        let new_lifetime_consumed = self.lifetime_consumed + actual;
        let entropy_increment = actual / self.temperature;
        let new_entropy = self.entropy + entropy_increment;

        if !new_available.is_finite()
            || !new_consumed_this_tick.is_finite()
            || !new_lifetime_consumed.is_finite()
            || !entropy_increment.is_finite()
            || !new_entropy.is_finite()
        {
            return 0.0;
        }

        self.available = new_available;
        self.consumed_this_tick = new_consumed_this_tick;
        self.lifetime_consumed = new_lifetime_consumed;
        self.entropy = new_entropy;
        if self.available == 0.0 {
            self.collapsed = true;
        }
        actual
    }

    /// Absorb heat from dissipation (damping, friction, collision).
    ///
    /// Invalid inputs or invalid thermal state are ignored for compatibility;
    /// callers needing explicit rejection should use try_dissipate_heat.
    pub fn dissipate_heat(&mut self, energy: f64) {
        let _ = self.try_dissipate_heat(energy);
    }

    /// Absorb finite positive heat, returning an error rather than silently
    /// accepting values that could corrupt temperature or entropy.
    pub fn try_dissipate_heat(&mut self, energy: f64) -> Result<(), EnergyBudgetError> {
        if !energy.is_finite() || energy <= 0.0 {
            return Err(EnergyBudgetError::InvalidEnergyAmount);
        }
        if !self.state_is_valid() {
            return Err(EnergyBudgetError::InvalidState);
        }

        let temperature_rise = energy / self.heat_capacity;
        let new_temperature = self.temperature + temperature_rise;
        let entropy_increment = energy / new_temperature;
        let new_entropy = self.entropy + entropy_increment;
        if !temperature_rise.is_finite()
            || !new_temperature.is_finite()
            || new_temperature <= 0.0
            || !entropy_increment.is_finite()
            || !new_entropy.is_finite()
        {
            return Err(EnergyBudgetError::NumericalOverflow);
        }

        self.temperature = new_temperature;
        self.entropy = new_entropy;
        Ok(())
    }

    /// Helmholtz free energy: maximum extractable work at constant T.
    /// F = U - TS. Invalid numeric state reports zero usable work.
    pub fn available_work(&self) -> f64 {
        if !self.state_is_valid() {
            return 0.0;
        }
        let work = self.available - self.temperature * self.entropy;
        if work.is_finite() {
            work.max(0.0)
        } else {
            0.0
        }
    }

    /// Add energy (from regeneration sources), capped at remaining capacity.
    ///
    /// Only energy actually accepted by the reservoir is recorded in
    /// regenerated_this_tick; a request while already full records no gain.
    /// Invalid, negative, and non-finite requests fail closed.
    #[inline]
    pub fn regenerate(&mut self, amount: f64) {
        if !self.state_is_valid() || !amount.is_finite() || amount <= 0.0 {
            return;
        }

        let headroom = self.max_energy - self.available;
        let actual = amount.min(headroom);
        if !actual.is_finite() || actual <= 0.0 {
            return;
        }
        let new_available = self.available + actual;
        let new_regenerated_this_tick = self.regenerated_this_tick + actual;
        if !new_available.is_finite()
            || new_available > self.max_energy
            || !new_regenerated_this_tick.is_finite()
        {
            return;
        }

        self.available = new_available;
        self.regenerated_this_tick = new_regenerated_this_tick;
        if self.available > 0.0 {
            self.collapsed = false;
        }
    }

    /// Whether any finite, valid energy is available.
    #[inline]
    pub fn has_energy(&self) -> bool {
        self.state_is_valid() && self.available > 1e-10 && !self.collapsed
    }

    /// Whether this entity has collapsed (zero energy).
    #[inline]
    pub fn is_collapsed(&self) -> bool {
        self.collapsed || !self.available.is_finite() || self.available <= 0.0
    }

    /// Fraction of energy remaining [0.0, 1.0].
    #[inline]
    pub fn fraction_remaining(&self) -> f64 {
        if !self.state_is_valid() || self.max_energy < 1e-10 {
            return 0.0;
        }
        (self.available / self.max_energy).clamp(0.0, 1.0)
    }

    /// Net energy flow this tick (positive = regenerating, negative = depleting).
    pub fn net_flow_this_tick(&self) -> f64 {
        if !self.state_is_valid() {
            return 0.0;
        }
        self.regenerated_this_tick - self.consumed_this_tick
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validated_constructor_rejects_invalid_capacity() {
        assert_eq!(
            EnergyBudget::try_new(f64::NAN).unwrap_err(),
            EnergyBudgetError::InvalidCapacity
        );
        assert_eq!(
            EnergyBudget::try_new(f64::INFINITY).unwrap_err(),
            EnergyBudgetError::InvalidCapacity
        );
        assert_eq!(
            EnergyBudget::try_new(-1.0).unwrap_err(),
            EnergyBudgetError::InvalidCapacity
        );

        let empty = EnergyBudget::try_new(0.0).unwrap();
        assert!(empty.is_collapsed());
        assert!(!empty.has_energy());
    }

    #[test]
    fn legacy_constructor_fails_closed_for_invalid_capacity() {
        let budget = EnergyBudget::new(f64::NAN);
        assert_eq!(budget.max_energy, 0.0);
        assert_eq!(budget.available, 0.0);
        assert!(budget.is_collapsed());
        assert!(!budget.has_energy());
    }

    #[test]
    fn new_budget_has_full_energy() {
        let budget = EnergyBudget::new(100.0);
        assert!((budget.available - 100.0).abs() < 1e-10);
        assert!(!budget.collapsed);
    }

    #[test]
    fn invalid_consumption_requests_leave_state_unchanged() {
        for amount in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 0.0] {
            let mut budget = EnergyBudget::new(100.0);
            let initial_available = budget.available;
            let initial_entropy = budget.entropy;
            let initial_consumed = budget.consumed_this_tick;
            let initial_lifetime = budget.lifetime_consumed;

            assert_eq!(budget.consume(amount), 0.0);
            assert_eq!(budget.available, initial_available);
            assert_eq!(budget.entropy, initial_entropy);
            assert_eq!(budget.consumed_this_tick, initial_consumed);
            assert_eq!(budget.lifetime_consumed, initial_lifetime);
            assert!(!budget.is_collapsed());
        }
    }

    #[test]
    fn consume_fails_closed_if_public_state_has_become_invalid() {
        let mut budget = EnergyBudget::new(100.0);
        budget.available = f64::NAN;
        assert_eq!(budget.consume(10.0), 0.0);
        assert!(budget.available.is_nan());
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
    fn collapse_on_zero_energy() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(100.0);
        assert!(budget.is_collapsed());
        assert!(!budget.has_energy());

        // Collapsed entity cannot consume
        let consumed = budget.consume(10.0);
        assert!((consumed - 0.0).abs() < 1e-10);
    }

    #[test]
    fn regeneration_records_only_energy_actually_accepted() {
        let mut budget = EnergyBudget::new(100.0);
        budget.regenerate(50.0);
        assert_eq!(budget.available, 100.0);
        assert_eq!(budget.regenerated_this_tick, 0.0);

        budget.consume(80.0);
        budget.tick_reset();
        budget.regenerate(500.0);
        assert_eq!(budget.available, 100.0);
        assert_eq!(budget.regenerated_this_tick, 80.0);
        assert_eq!(budget.net_flow_this_tick(), 80.0);
    }

    #[test]
    fn invalid_regeneration_requests_do_not_mutate_state() {
        for amount in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 0.0] {
            let mut budget = EnergyBudget::new(100.0);
            budget.consume(20.0);
            let initial_available = budget.available;
            let initial_regenerated = budget.regenerated_this_tick;
            budget.regenerate(amount);
            assert_eq!(budget.available, initial_available);
            assert_eq!(budget.regenerated_this_tick, initial_regenerated);
        }
    }

    #[test]
    fn regenerate_adds_energy() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(50.0);
        budget.regenerate(20.0);
        assert!((budget.available - 70.0).abs() < 1e-10);
    }

    #[test]
    fn invalid_heat_transfer_is_rejected_without_state_mutation() {
        for energy in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 0.0] {
            let mut budget = EnergyBudget::new(100.0);
            let temperature = budget.temperature;
            let entropy = budget.entropy;
            assert_eq!(
                budget.try_dissipate_heat(energy),
                Err(EnergyBudgetError::InvalidEnergyAmount)
            );
            assert_eq!(budget.temperature, temperature);
            assert_eq!(budget.entropy, entropy);
        }
    }

    #[test]
    fn heat_transfer_updates_temperature_and_entropy() {
        let mut budget = EnergyBudget::new(100.0);
        budget.try_dissipate_heat(100.0).unwrap();
        assert!((budget.temperature - 311.0).abs() < 1e-12);
        assert!(budget.entropy > 0.0);
    }

    #[test]
    fn heat_transfer_rejects_overflow_without_partial_mutation() {
        let mut budget = EnergyBudget::new(100.0);
        budget.heat_capacity = f64::MIN_POSITIVE;
        let temperature = budget.temperature;
        let entropy = budget.entropy;

        assert_eq!(
            budget.try_dissipate_heat(f64::MAX),
            Err(EnergyBudgetError::NumericalOverflow)
        );
        assert_eq!(budget.temperature, temperature);
        assert_eq!(budget.entropy, entropy);
    }

    #[test]
    fn regenerate_capped_at_max() {
        let mut budget = EnergyBudget::new(100.0);
        budget.regenerate(50.0); // already at max
        assert!((budget.available - 100.0).abs() < 1e-10);
    }

    #[test]
    fn regenerate_recovers_from_collapse() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(100.0);
        assert!(budget.is_collapsed());

        budget.regenerate(10.0);
        assert!(!budget.is_collapsed());
        assert!(budget.has_energy());
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
        budget.consume(10.0);
        budget.regenerate(15.0);
        assert!((budget.net_flow_this_tick() - 5.0).abs() < 1e-10); // +5 net
    }

    #[test]
    fn tick_reset_clears_per_tick() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(10.0);
        budget.regenerate(5.0);
        budget.tick_reset();
        assert!((budget.consumed_this_tick - 0.0).abs() < 1e-10);
        assert!((budget.regenerated_this_tick - 0.0).abs() < 1e-10);
        // But available is unchanged
        assert!((budget.available - 95.0).abs() < 1e-10);
    }

    #[test]
    fn fraction_remaining() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(25.0);
        assert!((budget.fraction_remaining() - 0.75).abs() < 1e-10);
    }
}
