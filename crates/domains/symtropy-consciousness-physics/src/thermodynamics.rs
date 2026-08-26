// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Legacy operational-energy telemetry for the consciousness/physics coupling.
//!
//! This module predates the typed physical `EnergyTransferLedger` in
//! `symtropy-physics`. Its `ThermodynamicLedger` is retained for gameplay and
//! historical diagnostics, but its `balance` / `conservation_error` values are
//! **not first-law evidence**. Physical conservation claims require explicit
//! physical reservoirs, typed source/destination transfers, and measured
//! reservoir reconciliation in the core physics crate.
//!
//! In particular, impulse magnitudes, prediction error, harmony, and other
//! semantic signals must not be converted into Joules by arbitrary scalars.
//! Boundary inflow/outflow is tracked separately from dissipation so signed
//! "negative dissipation" cannot make legacy telemetry look more conserved.
//!
//! # Key Quantities
//!
//! - **Landauer bound**: minimum energy per bit of information processing
//!   (k_B * T * ln(2) ≈ 2.87 × 10⁻²¹ J at body temperature 310K)
//! - **Joules-per-Phi**: experimental operational metric for the modeled budget
//! - **Legacy balance**: diagnostic mismatch between recorded operational cost
//!   and legacy dissipation-like telemetry; not a physical conservation proof

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
/// Returns a multiplier in [TEMP_CLARITY_FLOOR, 1.0] for physically valid,
/// finite positive Kelvin inputs:
/// - At body temperature (310K): ≈ 0.994 (essentially no penalty)
/// - At midpoint (360K): 0.55 (half capacity)
/// - At fever peak (410K+): ≈ 0.106 (near floor)
///
/// Non-finite or non-positive absolute temperature is invalid authoritative
/// evidence and fails closed to [`TEMP_CLARITY_FLOOR`]. It must never become
/// favorable near-unity motor/cognitive authority merely because floating-point
/// comparisons or a logistic expression happen to produce a benign value.
///
/// Uses a logistic sigmoid centred at TEMP_SIGMOID_MIDPOINT_K so the valid-input
/// penalty is continuous and differentiable.
///
/// Formula: `FLOOR + (1 - FLOOR) / (1 + exp(k * (T - T_mid)))`
///
/// # References
/// - Somjen et al. (2001) — cortical activity suppressed above 42°C
/// - Kiyatkin (2010) — brain temperature and neural activity review
#[inline]
pub fn smooth_temperature_penalty(temperature_k: f64) -> f64 {
    if !temperature_k.is_finite() || temperature_k <= 0.0 {
        return TEMP_CLARITY_FLOOR;
    }

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
/// These values determine how quickly the operational budget depletes and regenerates.
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
    /// Legacy collision-drain coefficient applied to impulse magnitude.
    ///
    /// This coefficient is an operational gameplay mapping, not a physical
    /// impulse→Joule conversion. It must not feed the physical heat ledger.
    pub collision_energy_drain: f64,
    /// Legacy operational budget credit associated with harmony resonance.
    pub harmony_resonance_regen_rate: f64,
    /// Joules per tick gained when standing in an energy well.
    pub energy_well_regen_rate: f64,
    /// Slow background operational regeneration (too slow alone).
    pub ambient_regen_rate: f64,
    /// Legacy recovery threshold retained for API compatibility.
    pub collapse_recovery_harmony_threshold: f64,
    /// Range within which harmony interactions are evaluated.
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

/// Rejection reason for a legacy telemetry recording operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyLedgerRecordError {
    NonFiniteValue,
    NonPositiveEnergy,
    NonFinitePhi,
    UnrepresentableAggregate,
}

/// Legacy operational telemetry ledger.
///
/// `energy_in`, `energy_out`, `balance`, and `conservation_error` preserve the
/// historical API but must not be cited as first-law physics evidence. Explicit
/// boundary flows are tracked separately, and strict physical accounting belongs
/// to `symtropy_physics::EnergyTransferLedger` plus reservoir reconciliation.
#[derive(Debug, Clone)]
pub struct ThermodynamicLedger {
    /// Historical operational-action throughput this tick.
    pub energy_in: f64,
    /// Historical dissipation-like throughput this tick. Never signed.
    pub energy_out: f64,
    /// Explicit positive energy entering across a legacy operational boundary.
    pub boundary_in: f64,
    /// Explicit positive energy leaving across a legacy operational boundary.
    pub boundary_out: f64,
    /// Phi-weighted operational energy integral: Σ(energy_i × phi_i).
    pub phi_energy_integral: f64,
    /// Total Phi change this tick: Σ|Δphi_i|.
    pub phi_change_total: f64,
    /// Cumulative operational energy recorded by `record_action`.
    pub lifetime_energy: f64,
    /// Cumulative historical balance mismatch. Not physical conservation error.
    pub lifetime_error: f64,
    /// Cumulative explicit boundary inflow telemetry.
    pub lifetime_boundary_in: f64,
    /// Cumulative explicit boundary outflow telemetry.
    pub lifetime_boundary_out: f64,
    /// Number of rejected invalid/unrepresentable telemetry events.
    pub rejected_event_count: u64,
    /// Number of ticks processed.
    pub tick_count: u64,
}

impl ThermodynamicLedger {
    pub fn new() -> Self {
        Self {
            energy_in: 0.0,
            energy_out: 0.0,
            boundary_in: 0.0,
            boundary_out: 0.0,
            phi_energy_integral: 0.0,
            phi_change_total: 0.0,
            lifetime_energy: 0.0,
            lifetime_error: 0.0,
            lifetime_boundary_in: 0.0,
            lifetime_boundary_out: 0.0,
            rejected_event_count: 0,
            tick_count: 0,
        }
    }

    fn reject<T>(&mut self, error: LegacyLedgerRecordError) -> Result<T, LegacyLedgerRecordError> {
        self.rejected_event_count = self.rejected_event_count.saturating_add(1);
        Err(error)
    }

    /// Checked operational-action recording.
    pub fn record_action_checked(
        &mut self,
        energy_cost: f64,
        phi: f64,
    ) -> Result<(), LegacyLedgerRecordError> {
        if !energy_cost.is_finite() {
            return self.reject(LegacyLedgerRecordError::NonFiniteValue);
        }
        if energy_cost <= 0.0 {
            return self.reject(LegacyLedgerRecordError::NonPositiveEnergy);
        }
        if !phi.is_finite() {
            return self.reject(LegacyLedgerRecordError::NonFinitePhi);
        }

        let weighted = energy_cost * phi;
        let next_energy = self.energy_in + energy_cost;
        let next_weighted = self.phi_energy_integral + weighted;
        if !weighted.is_finite() || !next_energy.is_finite() || !next_weighted.is_finite() {
            return self.reject(LegacyLedgerRecordError::UnrepresentableAggregate);
        }

        self.energy_in = next_energy;
        self.phi_energy_integral = next_weighted;
        Ok(())
    }

    /// Compatibility wrapper for legacy callers.
    pub fn record_action(&mut self, energy_cost: f64, phi: f64) {
        let _ = self.record_action_checked(energy_cost, phi);
    }

    /// Checked dissipation-like telemetry.
    ///
    /// Signed negative values are forbidden. Incoming energy must use
    /// [`Self::record_boundary_inflow_checked`] instead.
    pub fn record_dissipation_checked(
        &mut self,
        energy: f64,
    ) -> Result<(), LegacyLedgerRecordError> {
        if !energy.is_finite() {
            return self.reject(LegacyLedgerRecordError::NonFiniteValue);
        }
        if energy <= 0.0 {
            return self.reject(LegacyLedgerRecordError::NonPositiveEnergy);
        }
        let next = self.energy_out + energy;
        if !next.is_finite() {
            return self.reject(LegacyLedgerRecordError::UnrepresentableAggregate);
        }
        self.energy_out = next;
        Ok(())
    }

    /// Compatibility wrapper. Invalid/signed values are rejected and counted.
    pub fn record_dissipation(&mut self, energy: f64) {
        let _ = self.record_dissipation_checked(energy);
    }

    pub fn record_boundary_inflow_checked(
        &mut self,
        energy: f64,
    ) -> Result<(), LegacyLedgerRecordError> {
        if !energy.is_finite() {
            return self.reject(LegacyLedgerRecordError::NonFiniteValue);
        }
        if energy <= 0.0 {
            return self.reject(LegacyLedgerRecordError::NonPositiveEnergy);
        }
        let next = self.boundary_in + energy;
        if !next.is_finite() {
            return self.reject(LegacyLedgerRecordError::UnrepresentableAggregate);
        }
        self.boundary_in = next;
        Ok(())
    }

    pub fn record_boundary_outflow_checked(
        &mut self,
        energy: f64,
    ) -> Result<(), LegacyLedgerRecordError> {
        if !energy.is_finite() {
            return self.reject(LegacyLedgerRecordError::NonFiniteValue);
        }
        if energy <= 0.0 {
            return self.reject(LegacyLedgerRecordError::NonPositiveEnergy);
        }
        let next = self.boundary_out + energy;
        if !next.is_finite() {
            return self.reject(LegacyLedgerRecordError::UnrepresentableAggregate);
        }
        self.boundary_out = next;
        Ok(())
    }

    pub fn record_boundary_inflow(&mut self, energy: f64) {
        let _ = self.record_boundary_inflow_checked(energy);
    }

    pub fn record_boundary_outflow(&mut self, energy: f64) {
        let _ = self.record_boundary_outflow_checked(energy);
    }

    /// Record a change in Phi for an entity, rejecting malformed evidence.
    pub fn record_phi_change(&mut self, delta_phi: f64) {
        if !delta_phi.is_finite() {
            self.rejected_event_count = self.rejected_event_count.saturating_add(1);
            return;
        }
        let next = self.phi_change_total + delta_phi.abs();
        if next.is_finite() {
            self.phi_change_total = next;
        } else {
            self.rejected_event_count = self.rejected_event_count.saturating_add(1);
        }
    }

    /// Finalize this tick's **legacy operational diagnostic**.
    ///
    /// The returned `conservation_error` name is retained for API compatibility;
    /// it is not a first-law conservation metric. Boundary flow is deliberately
    /// excluded because this legacy balance does not represent complete physical
    /// endpoint reservoirs.
    pub fn tick_balance(&mut self) -> TickBalance {
        let balance = self.energy_in - self.energy_out;
        let error = balance.abs();

        if self.energy_in.is_finite() {
            let next = self.lifetime_energy + self.energy_in;
            if next.is_finite() {
                self.lifetime_energy = next;
            } else {
                self.rejected_event_count = self.rejected_event_count.saturating_add(1);
            }
        }
        if error.is_finite() {
            let next = self.lifetime_error + error;
            if next.is_finite() {
                self.lifetime_error = next;
            } else {
                self.rejected_event_count = self.rejected_event_count.saturating_add(1);
            }
        }
        for (value, lifetime) in [
            (self.boundary_in, &mut self.lifetime_boundary_in),
            (self.boundary_out, &mut self.lifetime_boundary_out),
        ] {
            let next = *lifetime + value;
            if value.is_finite() && next.is_finite() {
                *lifetime = next;
            } else {
                self.rejected_event_count = self.rejected_event_count.saturating_add(1);
            }
        }
        self.tick_count = self.tick_count.saturating_add(1);

        let result = TickBalance {
            energy_in: self.energy_in,
            energy_out: self.energy_out,
            balance,
            conservation_error: if self.energy_in > 1e-15 && error.is_finite() {
                error / self.energy_in
            } else if error == 0.0 {
                0.0
            } else {
                f64::INFINITY
            },
            joules_per_phi: self.joules_per_phi(),
        };

        self.energy_in = 0.0;
        self.energy_out = 0.0;
        self.boundary_in = 0.0;
        self.boundary_out = 0.0;
        self.phi_energy_integral = 0.0;
        self.phi_change_total = 0.0;

        result
    }

    /// Experimental operational metric: weighted Joules per unit Phi change.
    pub fn joules_per_phi(&self) -> Option<f64> {
        if !self.phi_change_total.is_finite() || self.phi_change_total < 1e-15 {
            return None;
        }
        let value = self.phi_energy_integral / self.phi_change_total;
        value.is_finite().then_some(value)
    }

    /// Minimum thermodynamic cost of processing `bits` of information.
    /// Based on the Landauer bound at body temperature.
    pub fn landauer_floor(bits: f64) -> f64 {
        LANDAUER_BOUND_310K * bits
    }

    /// Whether an energy cost exceeds the Landauer floor for given bits processed.
    pub fn above_landauer_floor(energy: f64, bits: f64) -> bool {
        energy.is_finite()
            && bits.is_finite()
            && bits >= 0.0
            && energy >= Self::landauer_floor(bits)
    }

    /// Historical lifetime diagnostic mismatch rate. Not first-law evidence.
    pub fn lifetime_error_rate(&self) -> f64 {
        if !self.lifetime_energy.is_finite()
            || !self.lifetime_error.is_finite()
            || self.lifetime_energy < 1e-15
        {
            return 0.0;
        }
        let rate = self.lifetime_error / self.lifetime_energy;
        if rate.is_finite() { rate } else { 0.0 }
    }
}

impl Default for ThermodynamicLedger {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a single tick's historical operational balance.
#[derive(Debug, Clone)]
pub struct TickBalance {
    /// Operational action throughput recorded this tick.
    pub energy_in: f64,
    /// Legacy dissipation-like throughput recorded this tick.
    pub energy_out: f64,
    /// `energy_in - energy_out`; legacy diagnostic only.
    pub balance: f64,
    /// Historical fractional mismatch. Name retained for compatibility; not first-law evidence.
    pub conservation_error: f64,
    /// Experimental Joules-per-Phi metric.
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
    fn balanced_legacy_telemetry_zero_error() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.record_action(100.0, 0.5);
        ledger.record_dissipation(100.0);
        let balance = ledger.tick_balance();
        assert!(balance.conservation_error < 1e-10);
    }

    #[test]
    fn negative_dissipation_cannot_masquerade_as_boundary_inflow() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.record_dissipation(-50.0);
        assert_eq!(ledger.energy_out, 0.0);
        assert_eq!(ledger.boundary_in, 0.0);
        assert_eq!(ledger.rejected_event_count, 1);
    }

    #[test]
    fn explicit_boundary_flow_is_separate_from_legacy_balance() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.record_action(100.0, 0.5);
        ledger.record_dissipation(80.0);
        ledger.record_boundary_inflow(20.0);
        ledger.record_boundary_outflow(5.0);
        assert_eq!(ledger.boundary_in, 20.0);
        assert_eq!(ledger.boundary_out, 5.0);

        let balance = ledger.tick_balance();
        assert_eq!(balance.balance, 20.0);
        assert!((ledger.lifetime_boundary_in - 20.0).abs() < 1e-12);
        assert!((ledger.lifetime_boundary_out - 5.0).abs() < 1e-12);
    }

    #[test]
    fn non_finite_and_aggregate_overflow_are_rejected_without_commit() {
        let mut ledger = ThermodynamicLedger::new();
        assert_eq!(
            ledger.record_dissipation_checked(f64::NAN),
            Err(LegacyLedgerRecordError::NonFiniteValue)
        );
        assert_eq!(
            ledger.record_dissipation_checked(f64::MAX),
            Ok(())
        );
        let before = ledger.energy_out;
        assert_eq!(
            ledger.record_dissipation_checked(f64::MAX),
            Err(LegacyLedgerRecordError::UnrepresentableAggregate)
        );
        assert_eq!(ledger.energy_out, before);
    }

    #[test]
    fn invalid_phi_cannot_enter_weighted_action_telemetry() {
        let mut ledger = ThermodynamicLedger::new();
        assert_eq!(
            ledger.record_action_checked(10.0, f64::NAN),
            Err(LegacyLedgerRecordError::NonFinitePhi)
        );
        assert_eq!(ledger.energy_in, 0.0);
        assert_eq!(ledger.phi_energy_integral, 0.0);
    }

    #[test]
    fn unbalanced_energy_nonzero_error() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.record_action(100.0, 0.5);
        ledger.record_dissipation(80.0);
        let balance = ledger.tick_balance();
        assert!((balance.conservation_error - 0.2).abs() < 1e-10);
    }

    #[test]
    fn joules_per_phi() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.record_action(100.0, 0.8);
        ledger.record_phi_change(0.2);
        let j_per_phi = ledger.joules_per_phi().unwrap();
        assert!((j_per_phi - 400.0).abs() < 1e-10);
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
        let floor_1g = ThermodynamicLedger::landauer_floor(1e9);
        assert!(floor_1g > 1e-12);
    }

    #[test]
    fn above_landauer_floor_check() {
        assert!(ThermodynamicLedger::above_landauer_floor(1e-20, 1.0));
        assert!(!ThermodynamicLedger::above_landauer_floor(1e-22, 1.0));
        assert!(!ThermodynamicLedger::above_landauer_floor(f64::NAN, 1.0));
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
        ledger.record_boundary_inflow(5.0);
        ledger.tick_balance();
        assert!(ledger.energy_in < 1e-15);
        assert!(ledger.energy_out < 1e-15);
        assert_eq!(ledger.boundary_in, 0.0);
        assert_eq!(ledger.boundary_out, 0.0);
    }

    #[test]
    fn lifetime_error_rate() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.record_action(100.0, 0.5);
        ledger.record_dissipation(90.0);
        ledger.tick_balance();
        let rate = ledger.lifetime_error_rate();
        assert!((rate - 0.1).abs() < 1e-10);
    }

    #[test]
    fn constants_default_survival_times() {
        let c = ThermodynamicConstants::default();
        let ticks_per_sec = 64.0;

        let idle_ticks = c.initial_energy / c.consciousness_maintenance_per_tick;
        let idle_seconds = idle_ticks / ticks_per_sec;
        assert!(idle_seconds > 150.0);
        assert!(idle_seconds < 250.0);

        let walk_speed = 100.0;
        let walk_displacement_per_tick = walk_speed / ticks_per_sec;
        let walk_cost_per_tick = c.movement_cost_per_unit * walk_displacement_per_tick;
        let walk_ticks =
            c.initial_energy / (c.consciousness_maintenance_per_tick + walk_cost_per_tick);
        let walk_seconds = walk_ticks / ticks_per_sec;
        assert!(walk_seconds > 100.0);
        assert!(walk_seconds < 200.0);

        let sprint_cost_per_tick =
            c.movement_cost_per_unit * (200.0 / ticks_per_sec) * c.sprint_cost_multiplier;
        let sprint_ticks =
            c.initial_energy / (c.consciousness_maintenance_per_tick + sprint_cost_per_tick);
        let sprint_seconds = sprint_ticks / ticks_per_sec;
        assert!(sprint_seconds < walk_seconds);

        let harmony_net = c.harmony_resonance_regen_rate - c.consciousness_maintenance_per_tick;
        assert!(harmony_net > 0.0);
    }

    #[test]
    fn energy_per_cognitive_op() {
        const { assert!(ENERGY_PER_COGNITIVE_OP > 0.0) };
        const { assert!(ENERGY_PER_COGNITIVE_OP < 1e-15) };
    }

    #[test]
    fn penalty_at_body_temp_is_near_one() {
        let p = smooth_temperature_penalty(BODY_TEMPERATURE_K);
        assert!(p > 0.98);
    }

    #[test]
    fn penalty_at_midpoint_is_half() {
        let p = smooth_temperature_penalty(TEMP_SIGMOID_MIDPOINT_K);
        assert!((p - 0.55).abs() < 1e-10);
    }

    #[test]
    fn penalty_at_410k_near_floor() {
        let p = smooth_temperature_penalty(410.0);
        assert!(p < 0.15);
        assert!(p >= TEMP_CLARITY_FLOOR);
    }

    #[test]
    fn penalty_is_strictly_decreasing() {
        let mut prev = smooth_temperature_penalty(310.0);
        for t in (320..=420).step_by(10) {
            let curr = smooth_temperature_penalty(t as f64);
            assert!(curr < prev);
            prev = curr;
        }
    }

    #[test]
    fn penalty_is_smooth_no_kink_at_body_temp() {
        let eps = 1.0;
        let t = BODY_TEMPERATURE_K;
        let d_before = smooth_temperature_penalty(t) - smooth_temperature_penalty(t - eps);
        let d_after = smooth_temperature_penalty(t + eps) - smooth_temperature_penalty(t);
        assert!((d_before - d_after).abs() < 0.01);
    }

    #[test]
    fn penalty_clamps_at_floor() {
        let p = smooth_temperature_penalty(1000.0);
        assert!((p - TEMP_CLARITY_FLOOR).abs() < 1e-6);
    }

    #[test]
    fn invalid_absolute_temperature_fails_closed() {
        for temperature in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            0.0,
            -1.0,
            -273.15,
        ] {
            let penalty = smooth_temperature_penalty(temperature);
            assert_eq!(penalty, TEMP_CLARITY_FLOOR);
            assert!(penalty.is_finite());
        }
    }

    #[test]
    fn extreme_positive_finite_temperatures_remain_bounded() {
        for temperature in [f64::MIN_POSITIVE, BODY_TEMPERATURE_K, 1.0e6, f64::MAX] {
            let penalty = smooth_temperature_penalty(temperature);
            assert!(penalty.is_finite());
            assert!((TEMP_CLARITY_FLOOR..=1.0).contains(&penalty));
        }
    }

    #[test]
    fn penalty_monotonic_cold() {
        let p_cold = smooth_temperature_penalty(280.0);
        let p_body = smooth_temperature_penalty(BODY_TEMPERATURE_K);
        assert!(p_cold >= p_body);
    }
}
