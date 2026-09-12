// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Persistent energy reservoir for consciousness-coupled entities.
//!
//! Energy is a finite, conserved quantity. It depletes through movement,
//! consciousness maintenance, and collisions. It regenerates through
//! harmony resonance, energy wells, and slow ambient recovery.
//!
//! When energy reaches zero, the entity collapses — Φ drops to 0,
//! motor output halts, and recovery requires another agent's help.

/// Persistent energy reservoir for consciousness-coupled entities.
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
    /// Total energy actually regenerated this tick (reset each tick for telemetry).
    pub regenerated_this_tick: f64,
    /// Cumulative energy spent across all ticks.
    pub lifetime_consumed: f64,
    /// Whether this entity has collapsed (energy depleted to zero).
    pub collapsed: bool,
}

impl EnergyBudget {
    /// Create a new energy budget with starting energy.
    pub fn new(max_energy: f64) -> Self {
        Self {
            max_energy,
            available: max_energy,
            temperature: 310.0,
            entropy: 0.0,
            heat_capacity: 100.0,
            consumed_this_tick: 0.0,
            regenerated_this_tick: 0.0,
            lifetime_consumed: 0.0,
            collapsed: false,
        }
    }

    /// Reset per-tick counters (call at start of each tick).
    pub fn tick_reset(&mut self) {
        self.consumed_this_tick = 0.0;
        self.regenerated_this_tick = 0.0;
    }

    /// Try to consume finite positive energy. Returns the amount actually consumed
    /// (may be less than requested if the budget is exhausted).
    ///
    /// Non-finite and non-positive requests fail closed. In particular, a negative
    /// "consumption" request must never become an energy source by subtracting a
    /// negative amount from the reservoir.
    #[inline]
    pub fn consume(&mut self, amount: f64) -> f64 {
        if self.collapsed || !amount.is_finite() || amount <= 0.0 {
            return 0.0;
        }
        let actual = amount.min(self.available);
        self.available -= actual;
        self.consumed_this_tick += actual;
        self.lifetime_consumed += actual;

        // 2nd Law: irreversible consumption increases entropy
        if self.temperature > 0.0 && actual > 0.0 {
            self.entropy += actual / self.temperature;
        }

        // Check for collapse
        if self.available <= 0.0 {
            self.available = 0.0;
            self.collapsed = true;
        }
        actual
    }

    /// Absorb finite positive heat from dissipation (damping, friction, collision).
    /// Increases temperature and entropy per 2nd Law.
    ///
    /// Invalid source energy or invalid existing thermal state fails closed rather
    /// than poisoning the persistent reservoir with NaN/Inf. The constructor/state
    /// invariant itself is intentionally a separate concern from this input boundary.
    pub fn dissipate_heat(&mut self, energy: f64) {
        if !energy.is_finite()
            || energy <= 0.0
            || !self.heat_capacity.is_finite()
            || self.heat_capacity <= 0.0
            || !self.temperature.is_finite()
            || self.temperature <= 0.0
            || !self.entropy.is_finite()
        {
            return;
        }

        let temperature_delta = energy / self.heat_capacity;
        if !temperature_delta.is_finite() {
            return;
        }
        let new_temperature = self.temperature + temperature_delta;
        if !new_temperature.is_finite() || new_temperature <= 0.0 {
            return;
        }
        let entropy_delta = energy / new_temperature;
        if !entropy_delta.is_finite() || entropy_delta < 0.0 {
            return;
        }

        self.temperature = new_temperature;
        self.entropy += entropy_delta;
    }

    /// Helmholtz free energy: maximum extractable work at constant T.
    /// F = U - TS. Always ≥ 0.
    pub fn available_work(&self) -> f64 {
        (self.available - self.temperature * self.entropy).max(0.0)
    }

    /// Add energy from a regeneration source, capped at `max_energy`.
    ///
    /// Returns the amount that actually entered the reservoir. Source-backed callers
    /// can therefore debit their finite source by the accepted transfer rather than by
    /// an offer that the reservoir may not have had capacity to receive.
    ///
    /// `regenerated_this_tick` records the same accepted amount. A full reservoir
    /// therefore returns and records zero rather than fictitious throughput.
    /// Non-finite and non-positive requests fail closed because regeneration is an
    /// energy source, not an alternate drain API.
    #[inline]
    pub fn regenerate(&mut self, amount: f64) -> f64 {
        if !amount.is_finite() || amount <= 0.0 {
            return 0.0;
        }

        let room = (self.max_energy - self.available).max(0.0);
        let actual = amount.min(room);
        self.available += actual;
        self.regenerated_this_tick += actual;

        // Recover from collapse only when energy actually enters the reservoir.
        if self.collapsed && actual > 0.0 && self.available > 0.0 {
            self.collapsed = false;
        }
        actual
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
    fn dissipate_heat_rejects_invalid_thermal_state_without_further_mutation() {
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
