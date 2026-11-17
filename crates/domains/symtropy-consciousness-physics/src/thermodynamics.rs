// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Experimental energy accounting for Φ-coupled simulation dynamics.
//!
//! The resource-budget and coupling rules in this module are model policies.
//! They are not, by themselves, a closed thermodynamic model or evidence that
//! Φ has a measured physical energy cost. A physical conservation claim requires
//! a declared system boundary, stored-energy changes, external work/heat,
//! inter-reservoir transfers, and named dissipation destinations.
//!
//! # Key quantities
//!
//! - **Landauer erasure bound**: minimum heat for a logically irreversible
//!   one-bit erasure under the stated thermal-bath assumptions:
//!   k_B * T * ln(2) (about 2.97e-21 J at 310 K). This is not a universal
//!   lower bound for arbitrary information processing.
//! - **Joules-per-Φ**: a descriptive model statistic; not a measured
//!   thermodynamic cost of consciousness without independent calibration.
//! - **Energy-balance residual**: a diagnostic for the terms actually entered
//!   into the ledger; it is not a conservation proof unless the system boundary
//!   and every energy store/flow are represented.

/// Exact SI Boltzmann constant, in joules per kelvin.
pub const BOLTZMANN_CONSTANT_J_PER_K: f64 = 1.380_649e-23;

/// Landauer erasure bound at 310 K: k_B * T * ln(2).
/// Minimum heat dissipated to erase one bit in a logically irreversible
/// operation under the standard thermal-bath assumptions. This does not bound
/// arbitrary reversible or information-processing operations.
pub const LANDAUER_BOUND_310K: f64 =
    BOLTZMANN_CONSTANT_J_PER_K * BODY_TEMPERATURE_K * std::f64::consts::LN_2;

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

/// Legacy model estimate for energy per cognitive operation.
/// Its relationship to physical bit erasure and its empirical calibration
/// remain to be independently qualified; do not interpret it as a universal
/// biological or computational energy constant.
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

/// Thermodynamic ledger: tracks energy flow through the consciousness-physics system.
#[derive(Debug, Clone)]
pub struct ThermodynamicLedger {
    /// Total energy input this tick (forces applied, work done).
    pub energy_in: f64,
    /// Total energy output this tick (dissipated as heat via damping/friction).
    pub energy_out: f64,
    /// Phi-weighted energy integral: Σ(energy_i × phi_i) across all entities.
    /// Used to compute Joules-per-Phi.
    pub phi_energy_integral: f64,
    /// Total Phi change this tick: Σ|Δphi_i|.
    pub phi_change_total: f64,
    /// Cumulative energy consumed across all ticks.
    pub lifetime_energy: f64,
    /// Cumulative conservation error across all ticks.
    pub lifetime_error: f64,
    /// Number of ticks processed.
    pub tick_count: u64,
}

impl ThermodynamicLedger {
    pub fn new() -> Self {
        Self {
            energy_in: 0.0,
            energy_out: 0.0,
            phi_energy_integral: 0.0,
            phi_change_total: 0.0,
            lifetime_energy: 0.0,
            lifetime_error: 0.0,
            tick_count: 0,
        }
    }

    /// Record an energy-consuming action.
    ///
    /// `energy_cost` is the Joules consumed.
    /// `phi` is the entity's current consciousness level.
    pub fn record_action(&mut self, energy_cost: f64, phi: f64) {
        self.energy_in += energy_cost;
        self.phi_energy_integral += energy_cost * phi;
    }

    /// Record energy dissipated (friction, damping, collision heat).
    pub fn record_dissipation(&mut self, energy: f64) {
        self.energy_out += energy;
    }

    /// Record a change in Phi for an entity.
    pub fn record_phi_change(&mut self, delta_phi: f64) {
        self.phi_change_total += delta_phi.abs();
    }

    /// Finalize this tick: compute conservation error and accumulate.
    pub fn tick_balance(&mut self) -> TickBalance {
        let balance = self.energy_in - self.energy_out;
        let error = balance.abs();
        self.lifetime_energy += self.energy_in;
        self.lifetime_error += error;
        self.tick_count += 1;

        let result = TickBalance {
            energy_in: self.energy_in,
            energy_out: self.energy_out,
            balance,
            conservation_error: if self.energy_in > 1e-15 {
                error / self.energy_in
            } else {
                0.0
            },
            joules_per_phi: self.joules_per_phi(),
        };

        // Reset per-tick accumulators
        self.energy_in = 0.0;
        self.energy_out = 0.0;
        self.phi_energy_integral = 0.0;
        self.phi_change_total = 0.0;

        result
    }

    /// Novel metric: Joules per unit Phi.
    ///
    /// How much energy does it cost to maintain consciousness?
    /// Returns None if no Phi change this tick.
    pub fn joules_per_phi(&self) -> Option<f64> {
        if self.phi_change_total < 1e-15 {
            return None;
        }
        Some(self.phi_energy_integral / self.phi_change_total)
    }

    /// Minimum heat for logically irreversible erasure of bits_erased bits
    /// at temperature_k, assuming a thermal bath at that temperature.
    ///
    /// Returns None for negative or non-finite bit counts or non-positive /
    /// non-finite temperatures. This is an erasure bound, not a feasibility
    /// test for arbitrary information processing.
    pub fn landauer_erasure_floor(bits_erased: f64, temperature_k: f64) -> Option<f64> {
        if !bits_erased.is_finite()
            || bits_erased < 0.0
            || !temperature_k.is_finite()
            || temperature_k <= 0.0
        {
            return None;
        }

        let floor =
            BOLTZMANN_CONSTANT_J_PER_K * temperature_k * std::f64::consts::LN_2 * bits_erased;
        floor.is_finite().then_some(floor)
    }

    /// Compatibility helper: 310 K Landauer heat bound for bits_erased
    /// logically irreversible bit erasures. Prefer landauer_erasure_floor
    /// with an explicit temperature for new code.
    pub fn landauer_floor(bits_erased: f64) -> f64 {
        LANDAUER_BOUND_310K * bits_erased
    }

    /// Whether heat dissipated meets the 310 K Landauer bound for the
    /// specified number of logically irreversible bit erasures. The energy
    /// argument must represent heat from those erasures, not total energy used
    /// by an arbitrary computation. Passing this comparison does not establish
    /// general computational feasibility or thermodynamic efficiency.
    pub fn above_landauer_floor(energy: f64, bits_erased: f64) -> bool {
        if !energy.is_finite() || energy < 0.0 {
            return false;
        }

        Self::landauer_erasure_floor(bits_erased, BODY_TEMPERATURE_K)
            .is_some_and(|floor| energy >= floor)
    }

    /// Lifetime conservation error rate.
    pub fn lifetime_error_rate(&self) -> f64 {
        if self.lifetime_energy < 1e-15 {
            return 0.0;
        }
        self.lifetime_error / self.lifetime_energy
    }
}

impl Default for ThermodynamicLedger {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a single tick's energy balance.
#[derive(Debug, Clone)]
pub struct TickBalance {
    /// Total energy consumed this tick.
    pub energy_in: f64,
    /// Total energy dissipated this tick.
    pub energy_out: f64,
    /// Net energy balance (should be near zero for conservation).
    pub balance: f64,
    /// Fractional conservation error [0, 1].
    pub conservation_error: f64,
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
        assert!(floor_1g > 1e-12); // ~2.97e-12 J for 1 Gbit
    }

    #[test]
    fn landauer_erasure_floor_is_temperature_and_bit_scaled() {
        let one_bit_310k = ThermodynamicLedger::landauer_erasure_floor(1.0, 310.0).unwrap();
        let two_bits_310k = ThermodynamicLedger::landauer_erasure_floor(2.0, 310.0).unwrap();
        let one_bit_300k = ThermodynamicLedger::landauer_erasure_floor(1.0, 300.0).unwrap();

        assert!((one_bit_310k - LANDAUER_BOUND_310K).abs() < 1e-35);
        assert!((one_bit_310k - 2.966_678_181_248_014_7e-21).abs() < 1e-35);
        assert!((two_bits_310k - 2.0 * one_bit_310k).abs() < 1e-35);
        assert!((one_bit_310k / one_bit_300k - 310.0 / 300.0).abs() < 1e-14);
    }

    #[test]
    fn landauer_erasure_floor_rejects_invalid_inputs() {
        assert!(ThermodynamicLedger::landauer_erasure_floor(-1.0, 310.0).is_none());
        assert!(ThermodynamicLedger::landauer_erasure_floor(f64::NAN, 310.0).is_none());
        assert!(ThermodynamicLedger::landauer_erasure_floor(1.0, 0.0).is_none());
        assert!(ThermodynamicLedger::landauer_erasure_floor(1.0, f64::INFINITY).is_none());
        assert!(ThermodynamicLedger::landauer_erasure_floor(f64::MAX, f64::MAX).is_none());
        assert!(!ThermodynamicLedger::above_landauer_floor(f64::NAN, 1.0));
        assert!(!ThermodynamicLedger::above_landauer_floor(-1.0, 1.0));
        assert!(!ThermodynamicLedger::above_landauer_floor(1.0, -1.0));
    }

    #[test]
    fn above_landauer_floor_check() {
        assert!(ThermodynamicLedger::above_landauer_floor(1e-20, 1.0)); // Meets bound
        assert!(!ThermodynamicLedger::above_landauer_floor(1e-22, 1.0)); // Below bound
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
