// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Thermodynamic closure: unifying consciousness energy and physics energy.
//!
//! This module implements the novel Joules-per-Phi energy accounting system.
//! Every action costs physical energy from the consciousness budget. Collision
//! impulses dissipate energy as heat. The total energy is tracked and conserved.
//!
//! # Key Quantities
//!
//! - **Landauer bound**: minimum work to erase one bit in an ideal isothermal
//!   process, k_B * T * ln(2) ≈ 2.967 × 10⁻²¹ J at 310 K. This is a lower
//!   bound for a specific logically irreversible operation, not a universal
//!   energy cost for all information processing.
//! - **Joules-per-Phi**: novel metric — how many Joules does it cost to
//!   maintain a given Φ level? No one has published this.
//! - **Conservation error**: measures violation of energy conservation law.
//!   Should remain < 1% for a valid simulation.

/// Boltzmann constant in J/K; exact in the SI since the 2019 redefinition.
pub const BOLTZMANN_CONSTANT_J_PER_K: f64 = 1.380_649e-23;

/// Natural logarithm of two, for the Landauer bound.
pub const LN_2: f64 = 0.693_147_180_559_945_3;

/// Body (baseline) temperature used by the model.
pub const BODY_TEMPERATURE_K: f64 = 310.0;

/// Landauer lower bound k_B T ln(2) at the model's 310 K reference temperature.
/// This applies to ideal logically irreversible bit erasure; it is not a
/// universal per-operation energy prediction.
pub const LANDAUER_BOUND_310K: f64 =
    BOLTZMANN_CONSTANT_J_PER_K * BODY_TEMPERATURE_K * LN_2;

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

/// Legacy model parameter scaled to 16.6 times the idealized Landauer bit-erasure
/// bound at 310 K. This is **not** an empirical neural/synaptic energy estimate,
/// and "cognitive operation" is not physically defined by this constant. It is
/// retained for API compatibility; do not use it to predict biological or
/// hardware energy costs without an independently specified operation and
/// measured/calibrated energy model.
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

/// Errors returned when a thermodynamic ledger receives invalid values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermodynamicLedgerError {
    InvalidEnergyAmount,
    InvalidPhi,
    InvalidPhiChange,
    InvalidBitCount,
    InvalidState,
    UnrepresentableTransfer,
    NumericalOverflow,
}

impl std::fmt::Display for ThermodynamicLedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidEnergyAmount => write!(f, "energy amount must be finite and nonnegative"),
            Self::InvalidPhi => write!(f, "Phi must be finite and in [0, 1]"),
            Self::InvalidPhiChange => write!(f, "Phi delta must be finite and in [-1, 1]"),
            Self::InvalidBitCount => write!(f, "bit count must be finite and nonnegative"),
            Self::InvalidState => write!(f, "thermodynamic ledger state is invalid"),
            Self::UnrepresentableTransfer => {
                write!(f, "nonzero transfer is too small to represent in the ledger")
            }
            Self::NumericalOverflow => write!(f, "ledger operation produced a non-finite result"),
        }
    }
}

impl std::error::Error for ThermodynamicLedgerError {}

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

    fn state_is_valid(&self) -> bool {
        self.energy_in.is_finite()
            && self.energy_in >= 0.0
            && self.energy_out.is_finite()
            && self.energy_out >= 0.0
            && self.phi_energy_integral.is_finite()
            && self.phi_energy_integral >= 0.0
            && (self.phi_energy_integral <= self.energy_in
                || (self.phi_energy_integral - self.energy_in).abs()
                    <= 1.0e-12 * self.phi_energy_integral.max(self.energy_in))
            && self.phi_change_total.is_finite()
            && self.phi_change_total >= 0.0
            && self.lifetime_energy.is_finite()
            && self.lifetime_energy >= 0.0
            && self.lifetime_error.is_finite()
            && self.lifetime_error >= 0.0
    }

    /// Record an energy-consuming action.
    ///
    /// The legacy void method is retained for compatibility. Invalid values
    /// are rejected without mutating the ledger; callers needing the reason can
    /// use try_record_action.
    pub fn record_action(&mut self, energy_cost: f64, phi: f64) {
        let _ = self.try_record_action(energy_cost, phi);
    }

    /// Record a finite, nonnegative energy cost and a Phi value in [0, 1].
    pub fn try_record_action(
        &mut self,
        energy_cost: f64,
        phi: f64,
    ) -> Result<(), ThermodynamicLedgerError> {
        if !energy_cost.is_finite() || energy_cost < 0.0 {
            return Err(ThermodynamicLedgerError::InvalidEnergyAmount);
        }
        if !phi.is_finite() || !(0.0..=1.0).contains(&phi) {
            return Err(ThermodynamicLedgerError::InvalidPhi);
        }
        if !self.state_is_valid() {
            return Err(ThermodynamicLedgerError::InvalidState);
        }

        let new_energy_in = self.energy_in + energy_cost;
        if !new_energy_in.is_finite() {
            return Err(ThermodynamicLedgerError::NumericalOverflow);
        }
        let actual_energy_added = new_energy_in - self.energy_in;
        if energy_cost > 0.0 && actual_energy_added <= 0.0 {
            return Err(ThermodynamicLedgerError::UnrepresentableTransfer);
        }
        let phi_weighted_cost = actual_energy_added * phi;
        let new_phi_energy_integral = self.phi_energy_integral + phi_weighted_cost;
        if !phi_weighted_cost.is_finite() || !new_phi_energy_integral.is_finite() {
            return Err(ThermodynamicLedgerError::NumericalOverflow);
        }

        self.energy_in = new_energy_in;
        self.phi_energy_integral = new_phi_energy_integral;
        Ok(())
    }

    /// Record energy dissipated (friction, damping, collision heat).
    /// Invalid or negative values are rejected without changing the ledger.
    pub fn record_dissipation(&mut self, energy: f64) {
        let _ = self.try_record_dissipation(energy);
    }

    /// Record a finite, nonnegative dissipation amount.
    pub fn try_record_dissipation(
        &mut self,
        energy: f64,
    ) -> Result<(), ThermodynamicLedgerError> {
        if !energy.is_finite() || energy < 0.0 {
            return Err(ThermodynamicLedgerError::InvalidEnergyAmount);
        }
        if !self.state_is_valid() {
            return Err(ThermodynamicLedgerError::InvalidState);
        }

        let new_energy_out = self.energy_out + energy;
        if !new_energy_out.is_finite() {
            return Err(ThermodynamicLedgerError::NumericalOverflow);
        }
        if energy > 0.0 && new_energy_out <= self.energy_out {
            return Err(ThermodynamicLedgerError::UnrepresentableTransfer);
        }
        self.energy_out = new_energy_out;
        Ok(())
    }

    /// Record a change in Phi for an entity. A single delta must be in [-1, 1].
    /// Invalid changes are rejected without mutating the ledger.
    pub fn record_phi_change(&mut self, delta_phi: f64) {
        let _ = self.try_record_phi_change(delta_phi);
    }

    pub fn try_record_phi_change(
        &mut self,
        delta_phi: f64,
    ) -> Result<(), ThermodynamicLedgerError> {
        if !delta_phi.is_finite() || delta_phi.abs() > 1.0 {
            return Err(ThermodynamicLedgerError::InvalidPhiChange);
        }
        if !self.state_is_valid() {
            return Err(ThermodynamicLedgerError::InvalidState);
        }

        let new_phi_change_total = self.phi_change_total + delta_phi.abs();
        if !new_phi_change_total.is_finite() {
            return Err(ThermodynamicLedgerError::NumericalOverflow);
        }
        if delta_phi != 0.0 && new_phi_change_total <= self.phi_change_total {
            return Err(ThermodynamicLedgerError::UnrepresentableTransfer);
        }
        self.phi_change_total = new_phi_change_total;
        Ok(())
    }

    /// Finalize this tick: compute conservation error and accumulate.
    ///
    /// If direct writes to public fields have corrupted the ledger, this legacy
    /// wrapper returns non-finite sentinels and leaves the invalid state intact.
    /// Prefer try_tick_balance to receive a typed error.
    pub fn tick_balance(&mut self) -> TickBalance {
        self.try_tick_balance().unwrap_or_else(|_| TickBalance {
            energy_in: self.energy_in,
            energy_out: self.energy_out,
            balance: f64::NAN,
            conservation_error: f64::INFINITY,
            joules_per_phi: None,
        })
    }

    /// Validate and finalize a tick. The residual is normalized by the larger
    /// of input/output energy, so a zero-input/nonzero-output imbalance cannot
    /// be incorrectly reported as zero error. A genuinely empty ledger has
    /// zero error. On failure, no counters are reset or accumulated.
    pub fn try_tick_balance(&mut self) -> Result<TickBalance, ThermodynamicLedgerError> {
        if !self.state_is_valid() {
            return Err(ThermodynamicLedgerError::InvalidState);
        }

        let balance = self.energy_in - self.energy_out;
        let error = balance.abs();
        let reference_energy = self.energy_in.max(self.energy_out);
        let conservation_error = if reference_energy > 0.0 {
            error / reference_energy
        } else {
            0.0
        };
        let joules_per_phi = self.joules_per_phi();
        if !balance.is_finite()
            || !error.is_finite()
            || !conservation_error.is_finite()
            || joules_per_phi.is_some_and(|value| !value.is_finite())
        {
            return Err(ThermodynamicLedgerError::NumericalOverflow);
        }

        let new_lifetime_energy = self.lifetime_energy + self.energy_in;
        let new_lifetime_error = self.lifetime_error + error;
        let new_tick_count = self
            .tick_count
            .checked_add(1)
            .ok_or(ThermodynamicLedgerError::NumericalOverflow)?;
        if !new_lifetime_energy.is_finite() || !new_lifetime_error.is_finite() {
            return Err(ThermodynamicLedgerError::NumericalOverflow);
        }

        let result = TickBalance {
            energy_in: self.energy_in,
            energy_out: self.energy_out,
            balance,
            conservation_error,
            joules_per_phi,
        };

        self.lifetime_energy = new_lifetime_energy;
        self.lifetime_error = new_lifetime_error;
        self.tick_count = new_tick_count;
        self.energy_in = 0.0;
        self.energy_out = 0.0;
        self.phi_energy_integral = 0.0;
        self.phi_change_total = 0.0;
        Ok(result)
    }

    /// Novel metric: Joules per unit Phi.
    ///
    /// How much energy does it cost to maintain Phi integration?
    /// Returns None if no Phi change occurred, or the ledger state is invalid.
    pub fn joules_per_phi(&self) -> Option<f64> {
        if !self.state_is_valid() || self.phi_change_total < 1e-15 {
            return None;
        }
        let result = self.phi_energy_integral / self.phi_change_total;
        result.is_finite().then_some(result)
    }

    /// Minimum thermodynamic cost of processing bits of information.
    /// Invalid bit counts return NaN in this compatibility-preserving wrapper.
    pub fn landauer_floor(bits: f64) -> f64 {
        Self::try_landauer_floor(bits).unwrap_or(f64::NAN)
    }

    pub fn try_landauer_floor(bits: f64) -> Result<f64, ThermodynamicLedgerError> {
        if !bits.is_finite() || bits < 0.0 {
            return Err(ThermodynamicLedgerError::InvalidBitCount);
        }
        let floor = LANDAUER_BOUND_310K * bits;
        if !floor.is_finite() {
            return Err(ThermodynamicLedgerError::NumericalOverflow);
        }
        if bits > 0.0 && floor == 0.0 {
            return Err(ThermodynamicLedgerError::UnrepresentableTransfer);
        }
        Ok(floor)
    }

    /// Whether an energy cost meets the Landauer floor for the given bit count.
    /// Invalid energy/bit inputs fail closed and return false.
    pub fn above_landauer_floor(energy: f64, bits: f64) -> bool {
        energy.is_finite()
            && energy >= 0.0
            && Self::try_landauer_floor(bits).is_ok_and(|floor| energy >= floor)
    }

    /// Lifetime conservation error rate normalized by the larger cumulative
    /// input/error scale. This remains meaningful if inputs were zero but
    /// outputs/errors were not; a truly empty history returns zero.
    pub fn lifetime_error_rate(&self) -> f64 {
        if !self.state_is_valid() {
            return f64::INFINITY;
        }
        let reference_energy = self.lifetime_energy.max(self.lifetime_error);
        if reference_energy == 0.0 {
            return 0.0;
        }
        let rate = self.lifetime_error / reference_energy;
        if rate.is_finite() {
            rate
        } else {
            f64::INFINITY
        }
    }
}

impl Default for ThermodynamicLedger {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a single tick's energy balance.
#[derive(Debug, Clone, PartialEq)]
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
    fn zero_input_with_nonzero_dissipation_is_not_reported_as_zero_error() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.record_dissipation(100.0);
        let balance = ledger.tick_balance();

        assert_eq!(balance.energy_in, 0.0);
        assert_eq!(balance.energy_out, 100.0);
        assert_eq!(balance.balance, -100.0);
        assert!((balance.conservation_error - 1.0).abs() < 1e-12);
        assert!((ledger.lifetime_error_rate() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn tiny_zero_input_imbalance_is_not_hidden_by_absolute_epsilon() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.record_dissipation(5e-20);
        let balance = ledger.tick_balance();
        assert!((balance.conservation_error - 1.0).abs() < 1e-12);
        assert!((ledger.lifetime_error_rate() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn invalid_ledger_inputs_fail_closed_without_partial_mutation() {
        let mut ledger = ThermodynamicLedger::new();
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
            assert_eq!(
                ledger.try_record_action(bad, 0.5),
                Err(ThermodynamicLedgerError::InvalidEnergyAmount)
            );
            assert_eq!(
                ledger.try_record_dissipation(bad),
                Err(ThermodynamicLedgerError::InvalidEnergyAmount)
            );
        }
        for bad_phi in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
            assert_eq!(
                ledger.try_record_action(1.0, bad_phi),
                Err(ThermodynamicLedgerError::InvalidPhi)
            );
        }
        assert_eq!(
            ledger.try_record_phi_change(f64::NAN),
            Err(ThermodynamicLedgerError::InvalidPhiChange)
        );
        assert_eq!(ledger.energy_in, 0.0);
        assert_eq!(ledger.energy_out, 0.0);
        assert_eq!(ledger.phi_energy_integral, 0.0);
        assert_eq!(ledger.phi_change_total, 0.0);
    }

    #[test]
    fn ledger_overflow_does_not_partially_mutate() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.energy_in = f64::MAX;
        assert_eq!(
            ledger.try_record_action(f64::MAX, 0.5),
            Err(ThermodynamicLedgerError::NumericalOverflow)
        );
        assert_eq!(ledger.energy_in, f64::MAX);
        assert_eq!(ledger.phi_energy_integral, 0.0);
    }

    #[test]
    fn phi_weighted_energy_cannot_exceed_recorded_energy_input() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.energy_in = 1.0;
        ledger.phi_energy_integral = 2.0;

        assert_eq!(
            ledger.try_tick_balance(),
            Err(ThermodynamicLedgerError::InvalidState)
        );
        assert_eq!(ledger.tick_count, 0);
        assert_eq!(ledger.energy_in, 1.0);
        assert_eq!(ledger.energy_out, 0.0);
    }

    #[test]
    fn invalid_public_ledger_state_is_not_reported_as_a_pass() {
        let mut ledger = ThermodynamicLedger::new();
        ledger.energy_in = f64::NAN;

        let balance = ledger.tick_balance();
        assert!(balance.balance.is_nan());
        assert!(balance.conservation_error.is_infinite());
        assert!(ledger.energy_in.is_nan());
        assert_eq!(ledger.tick_count, 0);
    }

    #[test]
    fn sub_ulp_ledger_transfers_are_rejected_without_mutation() {
        let tiny = f64::EPSILON / 4.0;

        let mut action_ledger = ThermodynamicLedger::new();
        action_ledger.energy_in = 100.0;
        assert_eq!(
            action_ledger.try_record_action(tiny, 0.5),
            Err(ThermodynamicLedgerError::UnrepresentableTransfer)
        );
        assert_eq!(action_ledger.energy_in, 100.0);
        assert_eq!(action_ledger.phi_energy_integral, 0.0);

        let mut dissipation_ledger = ThermodynamicLedger::new();
        dissipation_ledger.energy_out = 100.0;
        assert_eq!(
            dissipation_ledger.try_record_dissipation(tiny),
            Err(ThermodynamicLedgerError::UnrepresentableTransfer)
        );
        assert_eq!(dissipation_ledger.energy_out, 100.0);

        let mut phi_ledger = ThermodynamicLedger::new();
        phi_ledger.phi_change_total = 1.0;
        assert_eq!(
            phi_ledger.try_record_phi_change(tiny),
            Err(ThermodynamicLedgerError::UnrepresentableTransfer)
        );
        assert_eq!(phi_ledger.phi_change_total, 1.0);
    }

    #[test]
    fn landauer_floor_fails_closed_when_positive_bound_underflows_to_zero() {
        assert_eq!(
            ThermodynamicLedger::try_landauer_floor(f64::MIN_POSITIVE),
            Err(ThermodynamicLedgerError::UnrepresentableTransfer)
        );
    }

    #[test]
    fn landauer_helpers_reject_invalid_inputs() {
        assert!(ThermodynamicLedger::landauer_floor(f64::NAN).is_nan());
        assert!(ThermodynamicLedger::landauer_floor(-1.0).is_nan());
        assert!(!ThermodynamicLedger::above_landauer_floor(f64::NAN, 1.0));
        assert!(!ThermodynamicLedger::above_landauer_floor(1.0, f64::INFINITY));
        assert!(ThermodynamicLedger::try_landauer_floor(1.0).is_ok());
        assert_eq!(
            ThermodynamicLedger::try_landauer_floor(-1.0),
            Err(ThermodynamicLedgerError::InvalidBitCount)
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
    fn landauer_bound_uses_the_310k_si_reference_value() {
        let floor = ThermodynamicLedger::landauer_floor(1.0);
        let independently_evaluated_reference = 2.966_678_181_248_014_7e-21;
        assert!(
            (floor - independently_evaluated_reference).abs() < 1e-32,
            "310 K Landauer reference should be {independently_evaluated_reference:e}, got {floor:e}"
        );
        assert_eq!(floor, LANDAUER_BOUND_310K);

        // 1 billion logically irreversible bit erasures at the idealized bound.
        let floor_1g = ThermodynamicLedger::landauer_floor(1e9);
        assert!(floor_1g > 2.9e-12);
        assert!(floor_1g < 3.0e-12);
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
    fn landauer_scaled_legacy_parameter_is_positive_and_finite() {
        // This test verifies only the arithmetic of a dimensionless scaling
        // against the idealized bit-erasure bound. It is not a biological
        // calibration or a claim about joules per neural/cognitive operation.
        assert!(ENERGY_PER_COGNITIVE_OP.is_finite());
        assert!(ENERGY_PER_COGNITIVE_OP > LANDAUER_BOUND_310K);
        assert_eq!(ENERGY_PER_COGNITIVE_OP, LANDAUER_BOUND_310K * 16.6);
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
