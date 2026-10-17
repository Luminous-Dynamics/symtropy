// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Thermodynamic closure and local energy-flow diagnostics for the consciousness-physics system.
//!
//! The ledger distinguishes action costs, regenerative inflows, heat dissipation,
//! and explicit sinks. Its residual is a local flow diagnostic only: without a
//! declared system boundary, storage deltas, and every external input/output, it
//! must not be interpreted as proof of whole-system energy conservation.
//!
//! # Key quantities
//!
//! - **Landauer bound**: minimum energy per bit of information processing.
//! - **Joules-per-Phi**: an exploratory ratio of Phi-weighted action cost to Phi change.
//! - **Local flow residual**: a bookkeeping diagnostic over the flow categories
//!   recorded here, not a validated physical conservation error.
//!
//! These software invariants do not establish that the simulation's physical
//! model or coefficients are empirically calibrated.

/// Landauer bound at body temperature (310K): k_B * T * ln(2).
/// Minimum energy to erase one bit of information.
/// Ref: Landauer (1961), "Irreversibility and Heat Generation"
pub const LANDAUER_BOUND_310K: f64 = 2.87e-21; // Joules per bit

/// Body (baseline) temperature for consciousness clarity calculations.
/// At or below this temperature, no penalty applies.
pub const BODY_TEMPERATURE_K: f64 = 310.0;

/// Sigmoid midpoint for temperature-consciousness coupling.
/// Penalty reaches 0.55 at this temperature (halfway between baseline and max reduction).
pub const TEMP_SIGMOID_MIDPOINT_K: f64 = 360.0;

/// Sigmoid steepness for temperature-consciousness coupling.
/// At 0.1/K the transition spans ~100K (310K → 410K).
pub const TEMP_SIGMOID_STEEPNESS: f64 = 0.1;

/// Minimum consciousness clarity under extreme heat (asymptotic floor).
pub const TEMP_CLARITY_FLOOR: f64 = 0.1;

/// Smooth temperature-to-consciousness clarity penalty.
///
/// Returns a multiplier in [TEMP_CLARITY_FLOOR, 1.0]:
/// - At body temperature (310K): ≈ 0.994 (essentially no penalty)
/// - At midpoint (360K): 0.55 (half capacity)
/// - At fever peak (410K+): ≈ 0.106 (near floor)
///
/// Uses a logistic sigmoid centred at TEMP_SIGMOID_MIDPOINT_K so the
/// penalty is **continuous and differentiable** — no binary step at any
/// temperature, which previously caused a Dirac-like jump at 311K.
///
/// Formula: `FLOOR + (1 - FLOOR) / (1 + exp(k * (T - T_mid)))`
///
/// # References
/// - Somjen et al. (2001) — cortical activity suppressed above 42°C
/// - Kiyatkin (2010) — brain temperature and neural activity review
#[inline]
pub fn smooth_temperature_penalty(temperature_k: f64) -> f64 {
    let exponent = TEMP_SIGMOID_STEEPNESS * (temperature_k - TEMP_SIGMOID_MIDPOINT_K);
    let sigmoid = 1.0 / (1.0 + exponent.exp());
    (TEMP_CLARITY_FLOOR + (1.0 - TEMP_CLARITY_FLOOR) * sigmoid).clamp(TEMP_CLARITY_FLOOR, 1.0)
}

/// Default energy per cognitive operation (empirical estimate).
/// Cortical synapses operate at ~16.6× Landauer limit.
/// Ref: Laughlin & Sejnowski (2003)
pub const ENERGY_PER_COGNITIVE_OP: f64 = LANDAUER_BOUND_310K * 16.6;

/// Tunable thermodynamic constants for the enforcement layer.
///
/// These values determine how quickly energy depletes and regenerates.
/// Calibrated so that solo play survives ~4 minutes, cooperation sustains indefinitely.
#[derive(Debug, Clone)]
pub struct ThermodynamicConstants {
    /// Starting Joules for a new entity.
    pub initial_energy: f64,
    /// Maximum energy an entity can hold.
    pub max_energy: f64,
    /// Joules per physics-unit of displacement (walking).
    /// At WALK_SPEED=100 u/s, 64Hz: ~0.0078 J/tick for movement.
    pub movement_cost_per_unit: f64,
    /// Sprint cost multiplier (applied on top of movement_cost_per_unit).
    pub sprint_cost_multiplier: f64,
    /// Joules per tick to maintain consciousness (Φ > 0).
    /// Higher Φ costs more: actual = base * (1.0 + phi * 0.5).
    pub consciousness_maintenance_per_tick: f64,
    /// Fraction of collision impulse magnitude drained as energy.
    pub collision_energy_drain: f64,
    /// Joules per tick gained when harmony resonance > 0.5 with a nearby agent.
    pub harmony_resonance_regen_rate: f64,
    /// Joules per tick gained when standing in an energy well.
    pub energy_well_regen_rate: f64,
    /// Slow background regeneration (too slow alone).
    pub ambient_regen_rate: f64,
    /// Harmony resonance threshold to rescue a collapsed entity.
    pub collapse_recovery_harmony_threshold: f64,
    /// Range within which harmony resonance transfers energy.
    pub harmony_range: f64,
}

impl ThermodynamicConstants {
    /// Research-grade constants tuned for scarcity-driven dynamics.
    /// Solo agent ~1000 ticks. Cooperation extends to 5000+. Wells sustain but don't trivialize.
    pub fn research() -> Self {
        Self {
            initial_energy: 200.0,
            max_energy: 200.0,
            movement_cost_per_unit: 0.008,
            sprint_cost_multiplier: 2.5,
            consciousness_maintenance_per_tick: 0.20,
            collision_energy_drain: 0.05,
            harmony_resonance_regen_rate: 0.06,
            energy_well_regen_rate: 0.12,
            ambient_regen_rate: 0.005,
            collapse_recovery_harmony_threshold: 0.5,
            harmony_range: 40.0,
        }
    }
}

impl Default for ThermodynamicConstants {
    fn default() -> Self {
        Self {
            initial_energy: 1000.0,
            max_energy: 1000.0,
            movement_cost_per_unit: 0.005,
            sprint_cost_multiplier: 2.5,
            consciousness_maintenance_per_tick: 0.08,
            collision_energy_drain: 0.05,
            harmony_resonance_regen_rate: 0.15,
            energy_well_regen_rate: 0.25,
            ambient_regen_rate: 0.02,
            collapse_recovery_harmony_threshold: 0.5,
            harmony_range: 50.0,
        }
    }
}

/// Errors from checked energy-ledger updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerInputError {
    /// A value or computed accumulator was NaN or infinite.
    NonFiniteValue,
    /// An energy flow was negative; flow directions have separate methods.
    NegativeFlow,
    /// Phi was outside its normalized [0, 1] domain.
    PhiOutOfRange,
    /// Adding this update would overflow a finite accumulator.
    ArithmeticOverflow,
}

/// Ledger for local energy-flow diagnostics.
///
/// energy_in retains its historical meaning as recorded action/consumption cost.
/// Regeneration and external sinks are separate from heat dissipation. Public flow
/// fields remain for compatibility; tick_balance checks numeric domains before
/// reporting a valid local residual.
#[derive(Debug, Clone)]
pub struct ThermodynamicLedger {
    /// Recorded action/consumption costs this tick (legacy field name).
    pub energy_in: f64,
    /// Energy converted to heat through friction, damping, collision, etc.
    pub energy_out: f64,
    /// Positive energy accepted from explicit regenerative sources this tick.
    pub energy_regenerated: f64,
    /// Positive energy removed through explicit sinks (not necessarily heat).
    pub energy_removed: f64,
    /// Phi-weighted action-cost integral: Σ(action_cost_i × phi_i).
    pub phi_energy_integral: f64,
    /// Total Phi change this tick: Σ|Δphi_i|.
    pub phi_change_total: f64,
    /// Cumulative action/consumption cost across ticks.
    pub lifetime_energy: f64,
    /// Cumulative absolute local flow residual; infinity after any invalid tick.
    pub lifetime_error: f64,
    /// Cumulative normalization scale across valid ticks; includes every flow category.
    pub lifetime_flow_scale: f64,
    /// Number of ticks processed.
    pub tick_count: u64,
    /// Number of rejected or invalid flow updates in the current tick.
    pub invalid_flow_count: u64,
    /// Number of rejected or invalid flow updates across finalized ticks.
    pub lifetime_invalid_flow_count: u64,
}

impl ThermodynamicLedger {
    pub fn new() -> Self {
        Self {
            energy_in: 0.0,
            energy_out: 0.0,
            energy_regenerated: 0.0,
            energy_removed: 0.0,
            phi_energy_integral: 0.0,
            phi_change_total: 0.0,
            lifetime_energy: 0.0,
            lifetime_error: 0.0,
            lifetime_flow_scale: 0.0,
            tick_count: 0,
            invalid_flow_count: 0,
            lifetime_invalid_flow_count: 0,
        }
    }

    fn flow_fields_valid(&self) -> bool {
        [
            self.energy_in,
            self.energy_out,
            self.energy_regenerated,
            self.energy_removed,
            self.phi_energy_integral,
            self.phi_change_total,
        ]
        .into_iter()
        .all(|value| value.is_finite() && value >= 0.0)
    }

    fn reject<T>(&mut self, error: LedgerInputError) -> Result<T, LedgerInputError> {
        self.invalid_flow_count = self.invalid_flow_count.saturating_add(1);
        Err(error)
    }

    /// Compatibility wrapper: invalid input is rejected and invalidates this tick.
    /// Use try_record_action when the caller needs an error value.
    pub fn record_action(&mut self, energy_cost: f64, phi: f64) {
        let _ = self.try_record_action(energy_cost, phi);
    }

    pub fn try_record_action(
        &mut self,
        energy_cost: f64,
        phi: f64,
    ) -> Result<(), LedgerInputError> {
        if !energy_cost.is_finite() || !phi.is_finite() {
            return self.reject(LedgerInputError::NonFiniteValue);
        }
        if energy_cost < 0.0 {
            return self.reject(LedgerInputError::NegativeFlow);
        }
        if !(0.0..=1.0).contains(&phi) {
            return self.reject(LedgerInputError::PhiOutOfRange);
        }
        let next_in = self.energy_in + energy_cost;
        let weighted = energy_cost * phi;
        let next_phi_integral = self.phi_energy_integral + weighted;
        if !weighted.is_finite() || !next_in.is_finite() || !next_phi_integral.is_finite() {
            return self.reject(LedgerInputError::ArithmeticOverflow);
        }
        self.energy_in = next_in;
        self.phi_energy_integral = next_phi_integral;
        Ok(())
    }

    /// Record non-negative energy dissipated as heat.
    pub fn record_dissipation(&mut self, energy: f64) {
        let _ = self.try_record_dissipation(energy);
    }

    pub fn try_record_dissipation(&mut self, energy: f64) -> Result<(), LedgerInputError> {
        if !energy.is_finite() {
            return self.reject(LedgerInputError::NonFiniteValue);
        }
        if energy < 0.0 {
            return self.reject(LedgerInputError::NegativeFlow);
        }
        let next = self.energy_out + energy;
        if !next.is_finite() {
            return self.reject(LedgerInputError::ArithmeticOverflow);
        }
        self.energy_out = next;
        Ok(())
    }

    /// Record positive inflow from a regeneration/source mechanism.
    pub fn record_regeneration(&mut self, energy: f64) {
        let _ = self.try_record_regeneration(energy);
    }

    pub fn try_record_regeneration(&mut self, energy: f64) -> Result<(), LedgerInputError> {
        if !energy.is_finite() {
            return self.reject(LedgerInputError::NonFiniteValue);
        }
        if energy < 0.0 {
            return self.reject(LedgerInputError::NegativeFlow);
        }
        let next = self.energy_regenerated + energy;
        if !next.is_finite() {
            return self.reject(LedgerInputError::ArithmeticOverflow);
        }
        self.energy_regenerated = next;
        Ok(())
    }

    /// Record a positive flow explicitly removed through a sink or system boundary.
    /// This is not automatically classified as heat dissipation.
    pub fn record_external_sink(&mut self, energy: f64) {
        let _ = self.try_record_external_sink(energy);
    }

    pub fn try_record_external_sink(&mut self, energy: f64) -> Result<(), LedgerInputError> {
        if !energy.is_finite() {
            return self.reject(LedgerInputError::NonFiniteValue);
        }
        if energy < 0.0 {
            return self.reject(LedgerInputError::NegativeFlow);
        }
        let next = self.energy_removed + energy;
        if !next.is_finite() {
            return self.reject(LedgerInputError::ArithmeticOverflow);
        }
        self.energy_removed = next;
        Ok(())
    }

    /// Record a change in Phi for an entity.
    pub fn record_phi_change(&mut self, delta_phi: f64) {
        let _ = self.try_record_phi_change(delta_phi);
    }

    pub fn try_record_phi_change(&mut self, delta_phi: f64) -> Result<(), LedgerInputError> {
        if !delta_phi.is_finite() {
            return self.reject(LedgerInputError::NonFiniteValue);
        }
        let next = self.phi_change_total + delta_phi.abs();
        if !next.is_finite() {
            return self.reject(LedgerInputError::ArithmeticOverflow);
        }
        self.phi_change_total = next;
        Ok(())
    }

    /// Finalize the tick and calculate a local flow residual.
    ///
    /// The residual is not proof of whole-system conservation. Any rejected or
    /// invalid numeric flow marks the result invalid and returns an infinite
    /// residual rather than a zero/error-free result.
    pub fn tick_balance(&mut self) -> TickBalance {
        let fields_valid = self.flow_fields_valid();

        if !fields_valid && self.invalid_flow_count == 0 {
            self.invalid_flow_count = 1;
        }

        let raw_balance =
            self.energy_in + self.energy_regenerated - self.energy_out - self.energy_removed;
        let raw_scale = (self.energy_in + self.energy_regenerated)
            .max(self.energy_out + self.energy_removed)
            .max(1e-15);
        if (!raw_balance.is_finite() || !raw_scale.is_finite()) && self.invalid_flow_count == 0 {
            self.invalid_flow_count = 1;
        }
        let inputs_valid = fields_valid
            && raw_balance.is_finite()
            && raw_scale.is_finite()
            && self.invalid_flow_count == 0;
        let reported_balance = if raw_balance.is_finite() {
            raw_balance
        } else {
            f64::INFINITY
        };
        let error = reported_balance.abs();
        let residual = if inputs_valid {
            error / raw_scale
        } else {
            f64::INFINITY
        };

        self.lifetime_energy += self.energy_in;
        if inputs_valid {
            self.lifetime_error += error;
            self.lifetime_flow_scale += raw_scale;
            if !self.lifetime_error.is_finite() || !self.lifetime_flow_scale.is_finite() {
                self.lifetime_error = f64::INFINITY;
                self.lifetime_flow_scale = f64::INFINITY;
            }
        } else {
            self.lifetime_error = f64::INFINITY;
            self.lifetime_flow_scale = f64::INFINITY;
        }
        self.lifetime_invalid_flow_count = self
            .lifetime_invalid_flow_count
            .saturating_add(self.invalid_flow_count);
        self.tick_count = self.tick_count.saturating_add(1);

        let result = TickBalance {
            energy_in: self.energy_in,
            energy_out: self.energy_out,
            energy_regenerated: self.energy_regenerated,
            energy_removed: self.energy_removed,
            balance: reported_balance,
            conservation_error: residual,
            inputs_valid,
            invalid_flow_count: self.invalid_flow_count,
            joules_per_phi: self.joules_per_phi(),
        };

        self.energy_in = 0.0;
        self.energy_out = 0.0;
        self.energy_regenerated = 0.0;
        self.energy_removed = 0.0;
        self.phi_energy_integral = 0.0;
        self.phi_change_total = 0.0;
        self.invalid_flow_count = 0;

        result
    }

    /// Novel metric: Joules per unit Phi.
    ///
    /// How much energy does it cost to maintain consciousness?
    /// Returns None if no Phi change this tick.
    pub fn joules_per_phi(&self) -> Option<f64> {
        if self.invalid_flow_count > 0 || !self.flow_fields_valid() || self.phi_change_total < 1e-15
        {
            return None;
        }
        let ratio = self.phi_energy_integral / self.phi_change_total;
        ratio.is_finite().then_some(ratio)
    }

    /// Minimum thermodynamic cost of processing `bits` of information.
    /// Based on the Landauer bound at body temperature.
    pub fn landauer_floor(bits: f64) -> f64 {
        LANDAUER_BOUND_310K * bits
    }

    /// Whether an energy cost exceeds the Landauer floor for given bits processed.
    /// If the energy is below the floor, the computation is thermodynamically impossible.
    pub fn above_landauer_floor(energy: f64, bits: f64) -> bool {
        energy >= Self::landauer_floor(bits)
    }

    /// Lifetime local-flow residual rate. Returns infinity after invalid input;
    /// it does not certify conservation of the whole simulated system.
    pub fn lifetime_error_rate(&self) -> f64 {
        if !self.flow_fields_valid()
            || self.invalid_flow_count > 0
            || self.lifetime_invalid_flow_count > 0
            || !self.lifetime_energy.is_finite()
            || !self.lifetime_error.is_finite()
            || !self.lifetime_flow_scale.is_finite()
            || self.lifetime_flow_scale < 0.0
        {
            return f64::INFINITY;
        }
        if self.lifetime_flow_scale < 1e-15 {
            return 0.0;
        }
        self.lifetime_error / self.lifetime_flow_scale
    }
}

impl Default for ThermodynamicLedger {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a single tick's energy-flow bookkeeping.
#[derive(Debug, Clone)]
pub struct TickBalance {
    /// Action/consumption costs this tick (legacy field name).
    pub energy_in: f64,
    /// Energy converted to heat.
    pub energy_out: f64,
    /// Regenerative inflow.
    pub energy_regenerated: f64,
    /// Energy explicitly removed through sinks.
    pub energy_removed: f64,
    /// Local residual: action cost + regeneration - heat - explicit sinks.
    /// This is not a whole-system conservation proof.
    pub balance: f64,
    /// Legacy-named local residual fraction. Infinity means invalid bookkeeping.
    pub conservation_error: f64,
    /// True only when this tick's numeric flow inputs are valid; not a physics proof.
    pub inputs_valid: bool,
    /// Number of invalid/rejected numeric flow updates this tick.
    pub invalid_flow_count: u64,
    /// Joules per unit Phi this tick (None if no Phi change).
    pub joules_per_phi: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_ledger_zero_error() {
        let mut ledger = ThermodynamicLedger::new();
        let balance = ledger.tick_balance();
        assert_eq!(balance.conservation_error, 0.0);
    }

    #[test]
    fn balanced_energy_zero_error() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.record_action(100.0, 0.5);
        ledger.record_dissipation(100.0);
        let balance = ledger.tick_balance();
        assert!(
            balance.conservation_error < 1e-10,
            "error = {}",
            balance.conservation_error
        );
    }

    #[test]
    fn unbalanced_energy_nonzero_error() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.record_action(100.0, 0.5);
        ledger.record_dissipation(80.0); // 20% lost
        let balance = ledger.tick_balance();
        assert!(
            (balance.conservation_error - 0.2).abs() < 1e-10,
            "error = {}",
            balance.conservation_error
        );
    }

    #[test]
    fn joules_per_phi() {
        let mut ledger = ThermodynamicLedger::new();
        // Entity with phi=0.8 consumes 100J
        ledger.record_action(100.0, 0.8);
        ledger.record_phi_change(0.2); // Phi changed by 0.2
        let j_per_phi = ledger.joules_per_phi().unwrap();
        // phi_energy_integral = 100 * 0.8 = 80
        // joules_per_phi = 80 / 0.2 = 400
        assert!((j_per_phi - 400.0).abs() < 1e-10, "j/phi = {}", j_per_phi);
    }

    #[test]
    fn no_phi_change_returns_none() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.record_action(100.0, 0.5);
        assert!(ledger.joules_per_phi().is_none());
    }

    #[test]
    fn landauer_bound() {
        let floor = ThermodynamicLedger::landauer_floor(1.0);
        assert!((floor - LANDAUER_BOUND_310K).abs() < 1e-30);

        // 1 billion bits
        let floor_1g = ThermodynamicLedger::landauer_floor(1e9);
        assert!(floor_1g > 1e-12); // ~2.87e-12 J for 1 Gbit
    }

    #[test]
    fn above_landauer_floor_check() {
        assert!(ThermodynamicLedger::above_landauer_floor(1e-20, 1.0)); // Way above
        assert!(!ThermodynamicLedger::above_landauer_floor(1e-22, 1.0)); // Below
    }

    #[test]
    fn lifetime_accumulates() {
        let mut ledger = ThermodynamicLedger::new();

        ledger.record_action(100.0, 0.5);
        ledger.record_dissipation(100.0);
        ledger.tick_balance();

        ledger.record_action(200.0, 0.7);
        ledger.record_dissipation(200.0);
        ledger.tick_balance();

        assert!((ledger.lifetime_energy - 300.0).abs() < 1e-10);
        assert_eq!(ledger.tick_count, 2);
    }

    #[test]
    fn tick_resets_accumulators() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.record_action(100.0, 0.5);
        ledger.tick_balance();

        // After tick, per-tick values should be reset
        assert!(ledger.energy_in < 1e-15);
        assert!(ledger.energy_out < 1e-15);
    }

    #[test]
    fn lifetime_error_rate() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.record_action(100.0, 0.5);
        ledger.record_dissipation(90.0); // 10% error
        ledger.tick_balance();

        let rate = ledger.lifetime_error_rate();
        assert!((rate - 0.1).abs() < 1e-10, "error rate = {}", rate);
    }

    #[test]
    fn regeneration_and_external_sinks_are_not_heat_dissipation() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.record_action(100.0, 0.5);
        ledger.record_regeneration(20.0);
        ledger.record_dissipation(110.0);
        ledger.record_external_sink(10.0);
        let balance = ledger.tick_balance();

        assert!(balance.inputs_valid);
        assert_eq!(balance.energy_in, 100.0);
        assert_eq!(balance.energy_regenerated, 20.0);
        assert_eq!(balance.energy_out, 110.0);
        assert_eq!(balance.energy_removed, 10.0);
        assert_eq!(balance.balance, 0.0);
        assert_eq!(balance.conservation_error, 0.0);
    }

    #[test]
    fn rejected_negative_flow_fails_tick_closed_without_mutation() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.record_dissipation(4.0);
        assert_eq!(
            ledger.try_record_dissipation(-1.0),
            Err(LedgerInputError::NegativeFlow)
        );
        assert_eq!(ledger.energy_out, 4.0);
        // The lifetime accessor must fail closed before the tick is finalized too.
        assert!(ledger.lifetime_error_rate().is_infinite());

        let balance = ledger.tick_balance();
        assert!(!balance.inputs_valid);
        assert_eq!(balance.invalid_flow_count, 1);
        assert!(balance.conservation_error.is_infinite());
        assert!(ledger.lifetime_error_rate().is_infinite());
    }

    #[test]
    fn every_flow_category_rejects_negative_or_non_finite_values() {
        let mut ledger = ThermodynamicLedger::new();

        assert_eq!(
            ledger.try_record_action(-1.0, 0.5),
            Err(LedgerInputError::NegativeFlow)
        );
        assert_eq!(
            ledger.try_record_dissipation(-1.0),
            Err(LedgerInputError::NegativeFlow)
        );
        assert_eq!(
            ledger.try_record_regeneration(-1.0),
            Err(LedgerInputError::NegativeFlow)
        );
        assert_eq!(
            ledger.try_record_external_sink(-1.0),
            Err(LedgerInputError::NegativeFlow)
        );
        assert_eq!(
            ledger.try_record_regeneration(f64::NAN),
            Err(LedgerInputError::NonFiniteValue)
        );
        assert_eq!(
            ledger.try_record_external_sink(f64::INFINITY),
            Err(LedgerInputError::NonFiniteValue)
        );
        assert_eq!(
            ledger.try_record_phi_change(f64::NEG_INFINITY),
            Err(LedgerInputError::NonFiniteValue)
        );

        assert_eq!(ledger.energy_in, 0.0);
        assert_eq!(ledger.energy_out, 0.0);
        assert_eq!(ledger.energy_regenerated, 0.0);
        assert_eq!(ledger.energy_removed, 0.0);
        assert_eq!(ledger.phi_change_total, 0.0);
        assert_eq!(ledger.invalid_flow_count, 7);
        let balance = ledger.tick_balance();
        assert!(!balance.inputs_valid);
        assert_eq!(balance.invalid_flow_count, 7);
        assert!(balance.conservation_error.is_infinite());
    }

    #[test]
    fn non_finite_and_out_of_range_phi_are_rejected() {
        let mut ledger = ThermodynamicLedger::new();
        assert_eq!(
            ledger.try_record_action(1.0, f64::NAN),
            Err(LedgerInputError::NonFiniteValue)
        );
        assert_eq!(
            ledger.try_record_action(1.0, 1.01),
            Err(LedgerInputError::PhiOutOfRange)
        );
        assert_eq!(ledger.energy_in, 0.0);
        assert_eq!(ledger.phi_energy_integral, 0.0);
        assert_eq!(ledger.invalid_flow_count, 2);
        let result = ledger.tick_balance();
        assert!(!result.inputs_valid);
        assert!(result.conservation_error.is_infinite());
    }

    #[test]
    fn accumulator_overflow_is_rejected_transactionally() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.energy_in = f64::MAX;
        assert_eq!(
            ledger.try_record_action(f64::MAX, 0.5),
            Err(LedgerInputError::ArithmeticOverflow)
        );
        assert_eq!(ledger.energy_in, f64::MAX);
        let result = ledger.tick_balance();
        assert!(!result.inputs_valid);
        assert!(result.conservation_error.is_infinite());
    }

    #[test]
    fn joules_per_phi_is_unavailable_after_invalid_flow() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.record_action(2.0, 0.5);
        ledger.record_phi_change(0.1);
        assert!(ledger.joules_per_phi().is_some());

        assert_eq!(
            ledger.try_record_regeneration(-1.0),
            Err(LedgerInputError::NegativeFlow)
        );
        assert!(ledger.joules_per_phi().is_none());
    }

    #[test]
    fn zero_flows_remain_a_valid_empty_diagnostic() {
        let mut ledger = ThermodynamicLedger::new();
        let result = ledger.tick_balance();
        assert!(result.inputs_valid);
        assert_eq!(result.invalid_flow_count, 0);
        assert_eq!(result.balance, 0.0);
        assert_eq!(result.conservation_error, 0.0);
    }

    #[test]
    fn direct_invalid_public_state_fails_closed_in_derived_metrics() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.record_action(2.0, 0.5);
        ledger.record_phi_change(0.1);
        assert!(ledger.joules_per_phi().is_some());

        // Public fields are retained for compatibility, so direct mutation must
        // be detected too; the derived metrics must not silently look healthy.
        ledger.energy_out = f64::NAN;
        assert!(ledger.joules_per_phi().is_none());
        assert!(ledger.lifetime_error_rate().is_infinite());
        assert!(!ledger.tick_balance().inputs_valid);
    }

    #[test]
    fn lifetime_error_rate_includes_flows_without_action_cost() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.record_regeneration(2.0);
        ledger.record_external_sink(1.0);
        ledger.tick_balance();

        // The local residual is 1J over a 2J gross-flow scale, even without action cost.
        assert!((ledger.lifetime_error_rate() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn constants_default_survival_times() {
        // Verify the calibration math from the plan
        let c = ThermodynamicConstants::default();
        let ticks_per_sec = 64.0;

        // Idle survival: energy / maintenance_per_tick
        let idle_ticks = c.initial_energy / c.consciousness_maintenance_per_tick;
        let idle_seconds = idle_ticks / ticks_per_sec;
        assert!(
            idle_seconds > 150.0,
            "idle survival {idle_seconds}s should be > 150s"
        );
        assert!(
            idle_seconds < 250.0,
            "idle survival {idle_seconds}s should be < 250s"
        );

        // Walking survival: energy / (maintenance + movement_cost * speed/tick_rate)
        let walk_speed = 100.0; // physics units/sec
        let walk_displacement_per_tick = walk_speed / ticks_per_sec;
        let walk_cost_per_tick = c.movement_cost_per_unit * walk_displacement_per_tick;
        let walk_ticks =
            c.initial_energy / (c.consciousness_maintenance_per_tick + walk_cost_per_tick);
        let walk_seconds = walk_ticks / ticks_per_sec;
        assert!(
            walk_seconds > 100.0,
            "walk survival {walk_seconds}s should be > 100s"
        );
        assert!(
            walk_seconds < 200.0,
            "walk survival {walk_seconds}s should be < 200s"
        );

        // Sprinting: faster depletion
        let sprint_cost_per_tick =
            c.movement_cost_per_unit * (200.0 / ticks_per_sec) * c.sprint_cost_multiplier;
        let sprint_ticks =
            c.initial_energy / (c.consciousness_maintenance_per_tick + sprint_cost_per_tick);
        let sprint_seconds = sprint_ticks / ticks_per_sec;
        assert!(
            sprint_seconds < walk_seconds,
            "sprint should deplete faster than walk"
        );

        // With harmony partner: should regenerate (net positive)
        let harmony_net = c.harmony_resonance_regen_rate - c.consciousness_maintenance_per_tick;
        assert!(
            harmony_net > 0.0,
            "harmony regen should exceed maintenance: {harmony_net}"
        );
    }

    #[test]
    fn energy_per_cognitive_op() {
        // Verify the constant is physically reasonable
        // Brain uses ~20W for ~10^11 synapses at ~10Hz = 10^12 ops/sec
        // 20W / 10^12 = 2×10^-11 J/op
        // Our constant: ~4.8×10^-20 J/op (much lower — Landauer-based, not metabolic)
        const { assert!(ENERGY_PER_COGNITIVE_OP > 0.0) };
        const { assert!(ENERGY_PER_COGNITIVE_OP < 1e-15) }; // Not absurdly large
    }

    // ── smooth_temperature_penalty ────────────────────────────────────────────

    #[test]
    fn penalty_at_body_temp_is_near_one() {
        let p = smooth_temperature_penalty(BODY_TEMPERATURE_K);
        assert!(
            p > 0.98,
            "penalty at body temp ({BODY_TEMPERATURE_K}K) should be ≈1.0, got {p}"
        );
    }

    #[test]
    fn penalty_at_midpoint_is_half() {
        let p = smooth_temperature_penalty(TEMP_SIGMOID_MIDPOINT_K);
        // sigmoid(0) = 0.5 → penalty = FLOOR + (1-FLOOR)*0.5 = 0.1 + 0.45 = 0.55
        assert!(
            (p - 0.55).abs() < 1e-10,
            "penalty at midpoint should be 0.55, got {p}"
        );
    }

    #[test]
    fn penalty_at_410k_near_floor() {
        let p = smooth_temperature_penalty(410.0);
        assert!(
            p < 0.15,
            "penalty at 410K should be near floor ({TEMP_CLARITY_FLOOR}), got {p}"
        );
        assert!(p >= TEMP_CLARITY_FLOOR, "penalty should not go below floor");
    }

    #[test]
    fn penalty_is_strictly_decreasing() {
        // Sample 10 points from 310K to 420K — each must be less than the previous.
        let mut prev = smooth_temperature_penalty(310.0);
        for t in (320..=420).step_by(10) {
            let curr = smooth_temperature_penalty(t as f64);
            assert!(
                curr < prev,
                "penalty should decrease: penalty({t}K)={curr} ≥ prev={prev}"
            );
            prev = curr;
        }
    }

    #[test]
    fn penalty_is_smooth_no_kink_at_body_temp() {
        // Finite-difference test: derivative should be continuous around 310K.
        // If there were a kink (binary switch), the derivative would jump.
        let eps = 1.0;
        let t = BODY_TEMPERATURE_K;
        let d_before = smooth_temperature_penalty(t) - smooth_temperature_penalty(t - eps);
        let d_after = smooth_temperature_penalty(t + eps) - smooth_temperature_penalty(t);
        assert!(
            (d_before - d_after).abs() < 0.01,
            "derivative should be continuous at body temp: d_before={d_before}, d_after={d_after}"
        );
    }

    #[test]
    fn penalty_clamps_at_floor() {
        // At extreme temperatures (1000K), penalty must not go below floor.
        let p = smooth_temperature_penalty(1000.0);
        assert!(
            (p - TEMP_CLARITY_FLOOR).abs() < 1e-6,
            "extreme temp should clamp to floor {TEMP_CLARITY_FLOOR}, got {p}"
        );
    }

    #[test]
    fn penalty_monotonic_cold() {
        // Below body temperature, penalty should be ≥ body_temp penalty (no super-clarity).
        let p_cold = smooth_temperature_penalty(280.0);
        let p_body = smooth_temperature_penalty(BODY_TEMPERATURE_K);
        assert!(
            p_cold >= p_body,
            "cold temp penalty {p_cold} should be ≥ body temp penalty {p_body}"
        );
    }
}
