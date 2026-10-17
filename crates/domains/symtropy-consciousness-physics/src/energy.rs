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

/// Why an energy budget capacity could not be initialized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyBudgetConfigError {
    NonFiniteMaxEnergy,
    NegativeMaxEnergy,
}

impl EnergyBudget {
    /// Create a budget only when its capacity is finite and non-negative.
    pub fn try_new(max_energy: f64) -> Result<Self, EnergyBudgetConfigError> {
        if !max_energy.is_finite() {
            return Err(EnergyBudgetConfigError::NonFiniteMaxEnergy);
        }
        if max_energy < 0.0 {
            return Err(EnergyBudgetConfigError::NegativeMaxEnergy);
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
            collapsed: false,
        })
    }

    /// Create a new energy budget with starting energy.
    ///
    /// Compatibility wrapper for older callers. Invalid capacities fail closed
    /// to an inert, zero-capacity, collapsed budget. New code that needs to
    /// distinguish invalid configuration should use [`try_new`].
    pub fn new(max_energy: f64) -> Self {
        Self::try_new(max_energy).unwrap_or(Self {
            max_energy: 0.0,
            available: 0.0,
            temperature: 310.0,
            entropy: 0.0,
            heat_capacity: 100.0,
            consumed_this_tick: 0.0,
            regenerated_this_tick: 0.0,
            lifetime_consumed: 0.0,
            collapsed: true,
        })
    }

    /// Reset per-tick counters (call at start of each tick).
    pub fn tick_reset(&mut self) {
        self.consumed_this_tick = 0.0;
        self.regenerated_this_tick = 0.0;
    }

    /// Try to consume energy. Returns the amount actually consumed
    /// (may be less than requested if budget is exhausted).
    #[inline]
    pub fn consume(&mut self, amount: f64) -> f64 {
        // This public boundary must not let NaN bypass comparisons, or let a
        // negative/non-finite request create energy or poison telemetry.
        if self.collapsed
            || !amount.is_finite()
            || amount <= 0.0
            || !self.available.is_finite()
            || self.available <= 0.0
            || !self.max_energy.is_finite()
            || self.max_energy < self.available
            || !self.temperature.is_finite()
            || self.temperature <= 0.0
            || !self.entropy.is_finite()
            || self.entropy < 0.0
            || !self.consumed_this_tick.is_finite()
            || self.consumed_this_tick < 0.0
            || !self.lifetime_consumed.is_finite()
            || self.lifetime_consumed < 0.0
        {
            return 0.0;
        }

        let actual = amount.min(self.available);
        let next_available = (self.available - actual).max(0.0);
        let next_consumed = self.consumed_this_tick + actual;
        let next_lifetime = self.lifetime_consumed + actual;

        // 2nd Law: irreversible consumption increases entropy. Calculate all
        // outputs before mutation so overflow cannot partially poison state.
        let next_entropy = if self.temperature > 0.0 {
            self.entropy + actual / self.temperature
        } else {
            self.entropy
        };

        if !next_available.is_finite()
            || !next_consumed.is_finite()
            || !next_lifetime.is_finite()
            || !next_entropy.is_finite()
        {
            return 0.0;
        }

        self.available = next_available;
        self.consumed_this_tick = next_consumed;
        self.lifetime_consumed = next_lifetime;
        self.entropy = next_entropy;

        if self.available <= 0.0 {
            self.available = 0.0;
            self.collapsed = true;
        }
        actual
    }

    /// Absorb heat from dissipation (damping, friction, collision).
    /// Increases temperature and entropy per 2nd Law.
    pub fn dissipate_heat(&mut self, energy: f64) {
        if !energy.is_finite()
            || energy <= 0.0
            || !self.heat_capacity.is_finite()
            || self.heat_capacity <= 0.0
            || !self.temperature.is_finite()
            || self.temperature <= 0.0
            || !self.entropy.is_finite()
            || self.entropy < 0.0
        {
            return;
        }

        let next_temperature = self.temperature + energy / self.heat_capacity;
        if !next_temperature.is_finite() || next_temperature <= 0.0 {
            return;
        }
        let next_entropy = self.entropy + energy / next_temperature;
        if !next_entropy.is_finite() || next_entropy < self.entropy {
            return;
        }

        // Commit only after the complete update is finite and physically
        // admissible; an overflowing heat input must not corrupt the budget.
        self.temperature = next_temperature;
        self.entropy = next_entropy;
    }

    /// Helmholtz free energy: maximum extractable work at constant T.
    /// F = U - TS. Always ≥ 0.
    pub fn available_work(&self) -> f64 {
        (self.available - self.temperature * self.entropy).max(0.0)
    }

    /// Add energy (from regeneration sources). Capped at max_energy.
    #[inline]
    pub fn regenerate(&mut self, amount: f64) {
        // Regeneration is an energy source, not a debit. Reject invalid public
        // inputs and invalid reservoir state without changing any counters.
        if !amount.is_finite()
            || amount <= 0.0
            || !self.max_energy.is_finite()
            || self.max_energy < 0.0
            || !self.available.is_finite()
            || self.available < 0.0
            || self.available > self.max_energy
            || !self.regenerated_this_tick.is_finite()
            || self.regenerated_this_tick < 0.0
        {
            return;
        }

        // Subtract available headroom first, avoiding overflow in the sum
        // even when amount is a very large finite value.
        let headroom = self.max_energy - self.available;
        let added = amount.min(headroom);
        let next_available = self.available + added;
        let next_regenerated = self.regenerated_this_tick + added;
        if !next_available.is_finite() || !next_regenerated.is_finite() {
            return;
        }

        self.available = next_available;
        // Telemetry reports actual energy accepted by the reservoir, not the
        // requested amount that may have been truncated by max_energy.
        self.regenerated_this_tick = next_regenerated;

        // Recover from collapse if energy is restored
        if self.collapsed && self.available > 0.0 {
            self.collapsed = false;
        }
    }

    /// Whether any energy is available.
    #[inline]
    pub fn has_energy(&self) -> bool {
        self.available > 1e-10 && !self.collapsed
    }

    /// Whether this entity has collapsed (zero energy).
    #[inline]
    pub fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    /// Fraction of energy remaining [0.0, 1.0].
    #[inline]
    pub fn fraction_remaining(&self) -> f64 {
        if self.max_energy < 1e-10 {
            return 0.0;
        }
        self.available / self.max_energy
    }

    /// Net energy flow this tick (positive = regenerating, negative = depleting).
    pub fn net_flow_this_tick(&self) -> f64 {
        self.regenerated_this_tick - self.consumed_this_tick
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
    }

    #[test]
    fn checked_constructor_rejects_invalid_capacity() {
        assert_eq!(
            EnergyBudget::try_new(f64::NAN).unwrap_err(),
            EnergyBudgetConfigError::NonFiniteMaxEnergy
        );
        assert_eq!(
            EnergyBudget::try_new(f64::INFINITY).unwrap_err(),
            EnergyBudgetConfigError::NonFiniteMaxEnergy
        );
        assert_eq!(
            EnergyBudget::try_new(f64::NEG_INFINITY).unwrap_err(),
            EnergyBudgetConfigError::NonFiniteMaxEnergy
        );
        assert_eq!(
            EnergyBudget::try_new(-f64::EPSILON).unwrap_err(),
            EnergyBudgetConfigError::NegativeMaxEnergy
        );
        assert_eq!(EnergyBudget::try_new(0.0).unwrap().available, 0.0);
        assert_eq!(EnergyBudget::try_new(100.0).unwrap().available, 100.0);
    }

    #[test]
    fn legacy_constructor_fails_closed_for_invalid_capacity() {
        for capacity in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let budget = EnergyBudget::new(capacity);
            assert_eq!(budget.max_energy, 0.0, "capacity={capacity:?}");
            assert_eq!(budget.available, 0.0, "capacity={capacity:?}");
            assert!(budget.collapsed, "capacity={capacity:?}");
            assert_eq!(budget.fraction_remaining(), 0.0, "capacity={capacity:?}");
        }
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
    fn regenerate_adds_energy() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(50.0);
        budget.regenerate(20.0);
        assert!((budget.available - 70.0).abs() < 1e-10);
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
        budget.consume(20.0);
        budget.tick_reset();
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
    fn consume_rejects_negative_and_nonfinite_requests_without_mutation() {
        for amount in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut budget = EnergyBudget::new(100.0);
            let consumed = budget.consume(amount);
            assert_eq!(consumed, 0.0, "amount={amount:?}");
            assert_eq!(budget.available, 100.0, "amount={amount:?}");
            assert_eq!(budget.consumed_this_tick, 0.0, "amount={amount:?}");
            assert_eq!(budget.lifetime_consumed, 0.0, "amount={amount:?}");
            assert_eq!(budget.entropy, 0.0, "amount={amount:?}");
            assert!(!budget.collapsed, "amount={amount:?}");
        }
    }

    #[test]
    fn regenerate_rejects_negative_and_nonfinite_requests_without_mutation() {
        for amount in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut budget = EnergyBudget::new(100.0);
            budget.consume(25.0);
            let before_available = budget.available;
            let before_regenerated = budget.regenerated_this_tick;
            budget.regenerate(amount);
            assert_eq!(budget.available, before_available, "amount={amount:?}");
            assert_eq!(
                budget.regenerated_this_tick, before_regenerated,
                "amount={amount:?}"
            );
        }
    }

    #[test]
    fn regeneration_counter_records_only_energy_accepted() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(80.0);
        budget.regenerate(200.0);
        assert_eq!(budget.available, 100.0);
        assert_eq!(budget.regenerated_this_tick, 80.0);
        assert_eq!(budget.net_flow_this_tick(), 0.0);
    }

    #[test]
    fn dissipate_heat_rejects_nonfinite_or_overflowing_input_atomically() {
        for energy in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut budget = EnergyBudget::new(100.0);
            budget.dissipate_heat(energy);
            assert_eq!(budget.temperature, 310.0, "energy={energy:?}");
            assert_eq!(budget.entropy, 0.0, "energy={energy:?}");
        }

        let mut budget = EnergyBudget::new(100.0);
        budget.heat_capacity = f64::MIN_POSITIVE;
        budget.dissipate_heat(f64::MAX);
        assert_eq!(budget.temperature, 310.0);
        assert_eq!(budget.entropy, 0.0);
    }

    #[test]
    fn fraction_remaining() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(25.0);
        assert!((budget.fraction_remaining() - 0.75).abs() < 1e-10);
    }
}
