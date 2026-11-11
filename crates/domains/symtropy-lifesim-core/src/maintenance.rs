// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Energy-budgeted integrity-maintenance controls for OLI causal discrimination.
//!
//! This is a deliberately minimal, synthetic mechanism model. It contrasts passive
//! relaxation, fixed-rate repair, and proportional error-feedback repair under known
//! disturbances and constrained energy supply. It is designed to test whether an
//! experiment can distinguish persistence from active maintenance and feedback.
//!
//! It is not a biological organism model, a general homeostasis theory, or evidence
//! of life, agency, learning, intelligence, sentience, or consciousness.

use std::error::Error;
use std::fmt;

/// Competing mechanisms for a bounded integrity variable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MaintenanceMode {
    /// No repair actuator; integrity only experiences decay and disturbances.
    PassiveRelaxation,
    /// Repair at a fixed rate, independent of the measured integrity error.
    FixedRateRepair { repair_rate_per_time: f64 },
    /// Repair proportionally to the current difference from the target integrity.
    ProportionalFeedback { gain_per_time: f64 },
}

/// Declared physical/economic parameters for one synthetic maintenance system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaintenanceConfig {
    pub initial_integrity: f64,
    pub target_integrity: f64,
    /// First-order passive integrity-loss rate, in inverse time units.
    pub passive_decay_rate: f64,
    pub max_repair_rate: f64,
    pub energy_capacity: f64,
    pub initial_energy: f64,
    /// Environmental energy intake at availability = 1, per time unit.
    pub energy_harvest_rate: f64,
    /// Stored energy consumed per unit of integrity restored.
    pub energy_per_integrity_unit: f64,
    pub dt: f64,
}

impl Default for MaintenanceConfig {
    fn default() -> Self {
        Self {
            initial_integrity: 1.0,
            target_integrity: 1.0,
            passive_decay_rate: 0.005,
            max_repair_rate: 0.25,
            energy_capacity: 2.0,
            initial_energy: 0.5,
            energy_harvest_rate: 0.1,
            energy_per_integrity_unit: 1.0,
            dt: 1.0,
        }
    }
}

/// Exogenous perturbation and available environmental energy for one time step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaintenanceInput {
    /// Instantaneous integrity loss applied during this step, in [0, 1].
    pub disturbance_damage: f64,
    /// Fraction of maximum environmental energy intake available, in [0, 1].
    pub energy_availability: f64,
}

impl Default for MaintenanceInput {
    fn default() -> Self {
        Self {
            disturbance_damage: 0.0,
            energy_availability: 1.0,
        }
    }
}

/// Complete accounting for one maintenance-system update.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaintenanceStepReport {
    pub tick: u64,
    pub integrity_before: f64,
    pub passive_decay_loss: f64,
    pub disturbance_damage_requested: f64,
    pub disturbance_damage_applied: f64,
    pub integrity_before_repair: f64,
    pub repair_requested: f64,
    pub repair_applied: f64,
    pub unmet_repair: f64,
    pub integrity_after: f64,
    pub energy_before: f64,
    pub gross_energy_harvested: f64,
    pub energy_spilled: f64,
    pub energy_used: f64,
    pub energy_after: f64,
}

/// Invalid configuration, disturbance, or numerical transition.
#[derive(Debug, Clone, PartialEq)]
pub enum MaintenanceError {
    InvalidParameter { name: &'static str, value: f64 },
    InvalidInput { name: &'static str, value: f64 },
    TickOverflow,
}

impl fmt::Display for MaintenanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParameter { name, value } => {
                write!(f, "invalid maintenance parameter {name}={value}")
            }
            Self::InvalidInput { name, value } => {
                write!(f, "invalid maintenance input {name}={value}")
            }
            Self::TickOverflow => write!(f, "maintenance tick counter overflow"),
        }
    }
}

impl Error for MaintenanceError {}

impl MaintenanceConfig {
    fn validate(self, mode: MaintenanceMode) -> Result<(), MaintenanceError> {
        for (name, value) in [
            ("initial_integrity", self.initial_integrity),
            ("target_integrity", self.target_integrity),
            ("passive_decay_rate", self.passive_decay_rate),
            ("max_repair_rate", self.max_repair_rate),
            ("energy_capacity", self.energy_capacity),
            ("initial_energy", self.initial_energy),
            ("energy_harvest_rate", self.energy_harvest_rate),
            ("energy_per_integrity_unit", self.energy_per_integrity_unit),
            ("dt", self.dt),
        ] {
            if !value.is_finite() {
                return Err(MaintenanceError::InvalidParameter { name, value });
            }
        }

        for (name, value) in [
            ("initial_integrity", self.initial_integrity),
            ("target_integrity", self.target_integrity),
        ] {
            if !(0.0..=1.0).contains(&value) {
                return Err(MaintenanceError::InvalidParameter { name, value });
            }
        }

        for (name, value) in [
            ("passive_decay_rate", self.passive_decay_rate),
            ("max_repair_rate", self.max_repair_rate),
            ("energy_capacity", self.energy_capacity),
            ("initial_energy", self.initial_energy),
            ("energy_harvest_rate", self.energy_harvest_rate),
        ] {
            if value < 0.0 {
                return Err(MaintenanceError::InvalidParameter { name, value });
            }
        }
        if self.initial_energy > self.energy_capacity {
            return Err(MaintenanceError::InvalidParameter {
                name: "initial_energy_exceeds_capacity",
                value: self.initial_energy,
            });
        }
        if self.energy_per_integrity_unit <= 0.0 {
            return Err(MaintenanceError::InvalidParameter {
                name: "energy_per_integrity_unit",
                value: self.energy_per_integrity_unit,
            });
        }
        if self.dt <= 0.0 {
            return Err(MaintenanceError::InvalidParameter {
                name: "dt",
                value: self.dt,
            });
        }
        let passive_decay_step = self.passive_decay_rate * self.dt;
        let max_repair_step = self.max_repair_rate * self.dt;
        let energy_harvest_step = self.energy_harvest_rate * self.dt;
        if !passive_decay_step.is_finite() {
            return Err(MaintenanceError::InvalidParameter {
                name: "passive_decay_rate_times_dt",
                value: passive_decay_step,
            });
        }
        if !max_repair_step.is_finite() {
            return Err(MaintenanceError::InvalidParameter {
                name: "max_repair_rate_times_dt",
                value: max_repair_step,
            });
        }
        if !energy_harvest_step.is_finite()
            || !(self.energy_capacity + energy_harvest_step).is_finite()
        {
            return Err(MaintenanceError::InvalidParameter {
                name: "energy_harvest_plus_capacity",
                value: energy_harvest_step,
            });
        }

        match mode {
            MaintenanceMode::PassiveRelaxation => {}
            MaintenanceMode::FixedRateRepair {
                repair_rate_per_time,
            } if repair_rate_per_time.is_finite() && repair_rate_per_time >= 0.0 => {}
            MaintenanceMode::ProportionalFeedback { gain_per_time }
                if gain_per_time.is_finite() && gain_per_time >= 0.0 => {}
            MaintenanceMode::FixedRateRepair {
                repair_rate_per_time,
            } => {
                return Err(MaintenanceError::InvalidParameter {
                    name: "repair_rate_per_time",
                    value: repair_rate_per_time,
                });
            }
            MaintenanceMode::ProportionalFeedback { gain_per_time } => {
                return Err(MaintenanceError::InvalidParameter {
                    name: "gain_per_time",
                    value: gain_per_time,
                });
            }
        }
        Ok(())
    }
}

/// Deterministic integrity-maintenance system with explicit energy accounting.
#[derive(Debug, Clone, PartialEq)]
pub struct MaintenanceSystem {
    config: MaintenanceConfig,
    mode: MaintenanceMode,
    integrity: f64,
    energy: f64,
    tick: u64,
}

impl MaintenanceSystem {
    pub fn new(
        config: MaintenanceConfig,
        mode: MaintenanceMode,
    ) -> Result<Self, MaintenanceError> {
        config.validate(mode)?;
        Ok(Self {
            config,
            mode,
            integrity: config.initial_integrity,
            energy: config.initial_energy,
            tick: 0,
        })
    }

    pub const fn config(&self) -> MaintenanceConfig {
        self.config
    }

    pub const fn mode(&self) -> MaintenanceMode {
        self.mode
    }

    pub const fn integrity(&self) -> f64 {
        self.integrity
    }

    pub const fn energy(&self) -> f64 {
        self.energy
    }

    pub const fn tick(&self) -> u64 {
        self.tick
    }

    /// Change the mechanism as an explicit intervention (e.g. controller knockout).
    pub fn set_mode(&mut self, mode: MaintenanceMode) -> Result<(), MaintenanceError> {
        self.config.validate(mode)?;
        self.mode = mode;
        Ok(())
    }

    /// Apply passive decay, disturbance, energy intake, and the selected repair mechanism.
    ///
    /// The update order is fixed and part of the replay contract:
    /// 1. exact first-order passive decay;
    /// 2. instantaneous disturbance damage;
    /// 3. environmental energy harvest and capacity spill;
    /// 4. controller request;
    /// 5. energy-limited repair and state update.
    pub fn step(
        &mut self,
        input: MaintenanceInput,
    ) -> Result<MaintenanceStepReport, MaintenanceError> {
        validate_input(input)?;
        let next_tick = self.tick.checked_add(1).ok_or(MaintenanceError::TickOverflow)?;

        let integrity_before = self.integrity;
        let energy_before = self.energy;

        let passive_decay_fraction = 1.0 - (-self.config.passive_decay_rate * self.config.dt).exp();
        let passive_decay_loss = (integrity_before * passive_decay_fraction)
            .clamp(0.0, integrity_before);
        let after_decay = integrity_before - passive_decay_loss;

        let disturbance_damage_applied = input.disturbance_damage.min(after_decay);
        let integrity_before_repair = (after_decay - disturbance_damage_applied).max(0.0);

        let gross_energy_harvested =
            self.config.energy_harvest_rate * input.energy_availability * self.config.dt;
        let energy_with_harvest = energy_before + gross_energy_harvested;
        let energy_available = energy_with_harvest.min(self.config.energy_capacity);
        let energy_spilled = (energy_with_harvest - self.config.energy_capacity).max(0.0);

        let requested_rate = match self.mode {
            MaintenanceMode::PassiveRelaxation => 0.0,
            MaintenanceMode::FixedRateRepair {
                repair_rate_per_time,
            } => repair_rate_per_time,
            MaintenanceMode::ProportionalFeedback { gain_per_time } => {
                gain_per_time * (self.config.target_integrity - integrity_before_repair).max(0.0)
            }
        };
        if !requested_rate.is_finite() {
            return Err(MaintenanceError::InvalidParameter {
                name: "requested_repair_rate",
                value: requested_rate,
            });
        }

        let rate_limited_request = requested_rate.min(self.config.max_repair_rate);
        let repair_requested = (rate_limited_request * self.config.dt)
            .min((self.config.target_integrity - integrity_before_repair).max(0.0));
        let energy_limited_repair = energy_available / self.config.energy_per_integrity_unit;
        let repair_applied = repair_requested.min(energy_limited_repair);
        let unmet_repair = (repair_requested - repair_applied).max(0.0);
        let energy_used = repair_applied * self.config.energy_per_integrity_unit;
        let integrity_after = (integrity_before_repair + repair_applied)
            .clamp(0.0, self.config.target_integrity);
        let energy_after = (energy_available - energy_used).clamp(0.0, self.config.energy_capacity);

        if !integrity_after.is_finite() || !energy_after.is_finite() {
            return Err(MaintenanceError::InvalidParameter {
                name: "non_finite_transition",
                value: f64::NAN,
            });
        }

        self.integrity = integrity_after;
        self.energy = energy_after;
        self.tick = next_tick;

        Ok(MaintenanceStepReport {
            tick: self.tick,
            integrity_before,
            passive_decay_loss,
            disturbance_damage_requested: input.disturbance_damage,
            disturbance_damage_applied,
            integrity_before_repair,
            repair_requested,
            repair_applied,
            unmet_repair,
            integrity_after,
            energy_before,
            gross_energy_harvested,
            energy_spilled,
            energy_used,
            energy_after,
        })
    }
}

fn validate_input(input: MaintenanceInput) -> Result<(), MaintenanceError> {
    for (name, value) in [
        ("disturbance_damage", input.disturbance_damage),
        ("energy_availability", input.energy_availability),
    ] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(MaintenanceError::InvalidInput { name, value });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> MaintenanceConfig {
        MaintenanceConfig {
            initial_integrity: 1.0,
            target_integrity: 1.0,
            passive_decay_rate: 0.0,
            max_repair_rate: 0.25,
            energy_capacity: 1.0,
            initial_energy: 0.5,
            energy_harvest_rate: 0.05,
            energy_per_integrity_unit: 1.0,
            dt: 1.0,
        }
    }

    fn damage(amount: f64) -> MaintenanceInput {
        MaintenanceInput {
            disturbance_damage: amount,
            energy_availability: 1.0,
        }
    }

    #[test]
    fn feedback_repair_restores_integrity_after_disturbance() {
        let mut system = MaintenanceSystem::new(
            config(),
            MaintenanceMode::ProportionalFeedback { gain_per_time: 2.0 },
        )
        .unwrap();

        let first = system.step(damage(0.4)).unwrap();
        assert!((first.integrity_before_repair - 0.6).abs() < 1.0e-12);
        assert!((first.repair_applied - 0.25).abs() < 1.0e-12);
        assert!((system.integrity() - 0.85).abs() < 1.0e-12);

        system.step(MaintenanceInput::default()).unwrap();
        assert!((system.integrity() - 1.0).abs() < 1.0e-12);
        assert!(system.energy() < config().initial_energy + 2.0 * config().energy_harvest_rate);
    }

    #[test]
    fn passive_system_does_not_restore_damaged_integrity() {
        let mut system =
            MaintenanceSystem::new(config(), MaintenanceMode::PassiveRelaxation).unwrap();
        let report = system.step(damage(0.4)).unwrap();

        assert_eq!(report.repair_requested, 0.0);
        assert_eq!(report.repair_applied, 0.0);
        assert!((system.integrity() - 0.6).abs() < 1.0e-12);
    }

    #[test]
    fn fixed_rate_and_feedback_repair_respond_differently_to_perturbation_size() {
        let fixed = |damage_amount| {
            let mut system = MaintenanceSystem::new(
                config(),
                MaintenanceMode::FixedRateRepair {
                    repair_rate_per_time: 0.1,
                },
            )
            .unwrap();
            system.step(damage(damage_amount)).unwrap().repair_requested
        };
        let feedback = |damage_amount| {
            let mut system = MaintenanceSystem::new(
                config(),
                MaintenanceMode::ProportionalFeedback { gain_per_time: 0.5 },
            )
            .unwrap();
            system.step(damage(damage_amount)).unwrap().repair_requested
        };

        assert!((fixed(0.1) - fixed(0.4)).abs() < 1.0e-12);
        assert!((feedback(0.4) - 4.0 * feedback(0.1)).abs() < 1.0e-12);
    }

    #[test]
    fn controller_knockout_stops_feedback_repair() {
        let mut system = MaintenanceSystem::new(
            config(),
            MaintenanceMode::ProportionalFeedback { gain_per_time: 1.0 },
        )
        .unwrap();
        system.step(damage(0.3)).unwrap();
        assert!(system.integrity() < 1.0);

        system
            .set_mode(MaintenanceMode::PassiveRelaxation)
            .unwrap();
        let report = system.step(MaintenanceInput::default()).unwrap();
        assert_eq!(report.repair_requested, 0.0);
        assert_eq!(report.repair_applied, 0.0);
    }

    #[test]
    fn exhausted_energy_prevents_further_repair() {
        let mut starved = config();
        starved.initial_energy = 0.0;
        starved.energy_harvest_rate = 0.0;
        let mut system = MaintenanceSystem::new(
            starved,
            MaintenanceMode::ProportionalFeedback { gain_per_time: 2.0 },
        )
        .unwrap();
        let report = system.step(damage(0.3)).unwrap();

        assert!(report.repair_requested > 0.0);
        assert_eq!(report.repair_applied, 0.0);
        assert_eq!(report.unmet_repair, report.repair_requested);
        assert_eq!(system.integrity(), 0.7);
    }

    #[test]
    fn finite_energy_reserve_supports_only_temporary_repair() {
        let mut finite_reserve = config();
        finite_reserve.initial_energy = 0.25;
        finite_reserve.energy_harvest_rate = 0.0;
        let mut system = MaintenanceSystem::new(
            finite_reserve,
            MaintenanceMode::ProportionalFeedback { gain_per_time: 2.0 },
        )
        .unwrap();

        let first = system.step(damage(0.4)).unwrap();
        assert!((first.repair_applied - 0.25).abs() < 1.0e-12);
        assert_eq!(system.energy(), 0.0);

        let second = system.step(MaintenanceInput::default()).unwrap();
        assert!(second.repair_requested > 0.0);
        assert_eq!(second.repair_applied, 0.0);
        assert!(second.unmet_repair > 0.0);
        assert!(system.integrity() < 1.0);
    }

    #[test]
    fn zero_disturbance_does_not_trigger_feedback_repair() {
        let mut system = MaintenanceSystem::new(
            config(),
            MaintenanceMode::ProportionalFeedback { gain_per_time: 2.0 },
        )
        .unwrap();
        let report = system.step(MaintenanceInput::default()).unwrap();

        assert_eq!(report.repair_requested, 0.0);
        assert_eq!(report.energy_used, 0.0);
        assert_eq!(system.integrity(), 1.0);
    }

    #[test]
    fn energy_ledger_balances_with_harvest_use_and_spill() {
        let mut full = config();
        full.initial_energy = full.energy_capacity;
        let mut system = MaintenanceSystem::new(
            full,
            MaintenanceMode::ProportionalFeedback { gain_per_time: 1.0 },
        )
        .unwrap();
        let report = system.step(damage(0.2)).unwrap();

        let accounted = report.energy_after + report.energy_used + report.energy_spilled;
        let available = report.energy_before + report.gross_energy_harvested;
        assert!((available - accounted).abs() < 1.0e-12);
    }

    #[test]
    fn rejects_energy_capacity_plus_harvest_overflow() {
        assert!(matches!(
            MaintenanceSystem::new(
                MaintenanceConfig {
                    energy_capacity: f64::MAX,
                    initial_energy: f64::MAX,
                    energy_harvest_rate: f64::MAX,
                    dt: 1.0,
                    ..config()
                },
                MaintenanceMode::PassiveRelaxation
            ),
            Err(MaintenanceError::InvalidParameter {
                name: "energy_harvest_plus_capacity",
                ..
            })
        ));
    }

    #[test]
    fn deterministic_replay_and_input_validation() {
        let mut a = MaintenanceSystem::new(
            config(),
            MaintenanceMode::ProportionalFeedback { gain_per_time: 1.2 },
        )
        .unwrap();
        let mut b = a.clone();
        let inputs = [
            damage(0.2),
            MaintenanceInput::default(),
            damage(0.1),
            MaintenanceInput {
                disturbance_damage: 0.0,
                energy_availability: 0.0,
            },
        ];
        for input in inputs {
            assert_eq!(a.step(input).unwrap(), b.step(input).unwrap());
        }
        assert_eq!(a, b);
        assert!(matches!(
            a.step(MaintenanceInput {
                disturbance_damage: f64::NAN,
                energy_availability: 0.5,
            }),
            Err(MaintenanceError::InvalidInput {
                name: "disturbance_damage",
                ..
            })
        ));
    }
}
