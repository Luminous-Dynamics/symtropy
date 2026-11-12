// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic, energy-accounted feedback-maintenance control for OLI.
//!
//! This reference models an engineered compartment whose scalar integrity is driven
//! toward a target by a fixed feedback controller. Repair consumes a finite energy
//! reservoir; energy can be harvested, stored, spilled, or consumed by idle operation.
//! Damage, sensing availability, and controller enablement are explicit interventions.
//!
//! It is a deliberately simple abiotic counterexample: fixed-rule feedback can produce
//! homeostasis-like recovery without reproduction, heredity, task learning, generalization,
//! agency, or biological life. A recovery curve alone is therefore not a life detector.
//!
//! This is an abstract nondimensional model, not a validated biochemical, cellular,
//! ecological, or spacecraft thermal model. Its energy ledger is exact by construction
//! within floating-point tolerance, but its chosen dynamics are a benchmark mechanism.

use std::error::Error;
use std::fmt;

/// Fixed parameters for the engineered maintenance controller.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaintenanceConfig {
    /// Integrity setpoint in [0, 1].
    pub target_integrity: f64,
    /// Energy required per unit of integrity restored before efficiency losses.
    pub energy_per_integrity_unit: f64,
    /// Fraction of repair energy converted into integrity restoration, in (0, 1].
    pub repair_efficiency: f64,
    /// Maximum repair power (energy units per time unit).
    pub max_repair_power: f64,
    /// Idle controller/sensor power (energy units per time unit).
    pub idle_power: f64,
    /// Energy reservoir capacity.
    pub max_stored_energy: f64,
    /// Fixed integration interval.
    pub dt: f64,
}

impl Default for MaintenanceConfig {
    fn default() -> Self {
        Self {
            target_integrity: 1.0,
            energy_per_integrity_unit: 1.0,
            repair_efficiency: 0.8,
            max_repair_power: 0.5,
            idle_power: 0.02,
            max_stored_energy: 10.0,
            dt: 1.0,
        }
    }
}

impl MaintenanceConfig {
    fn validate(self) -> Result<(), MaintenanceError> {
        for (name, value) in [
            ("target_integrity", self.target_integrity),
            ("energy_per_integrity_unit", self.energy_per_integrity_unit),
            ("repair_efficiency", self.repair_efficiency),
            ("max_repair_power", self.max_repair_power),
            ("idle_power", self.idle_power),
            ("max_stored_energy", self.max_stored_energy),
            ("dt", self.dt),
        ] {
            if !value.is_finite() {
                return Err(MaintenanceError::NonFiniteParameter { name, value });
            }
        }
        if !(0.0..=1.0).contains(&self.target_integrity) {
            return Err(MaintenanceError::ParameterOutOfRange {
                name: "target_integrity",
                value: self.target_integrity,
                minimum: 0.0,
                maximum: 1.0,
            });
        }
        if self.energy_per_integrity_unit <= 0.0 {
            return Err(MaintenanceError::NonPositiveParameter {
                name: "energy_per_integrity_unit",
                value: self.energy_per_integrity_unit,
            });
        }
        if !(0.0..=1.0).contains(&self.repair_efficiency) || self.repair_efficiency == 0.0 {
            return Err(MaintenanceError::ParameterOutOfRange {
                name: "repair_efficiency",
                value: self.repair_efficiency,
                minimum: f64::MIN_POSITIVE,
                maximum: 1.0,
            });
        }
        for (name, value) in [
            ("max_repair_power", self.max_repair_power),
            ("idle_power", self.idle_power),
        ] {
            if value < 0.0 {
                return Err(MaintenanceError::NegativeParameter { name, value });
            }
        }
        if self.max_stored_energy <= 0.0 {
            return Err(MaintenanceError::NonPositiveParameter {
                name: "max_stored_energy",
                value: self.max_stored_energy,
            });
        }
        if self.dt <= 0.0 {
            return Err(MaintenanceError::NonPositiveParameter {
                name: "dt",
                value: self.dt,
            });
        }
        if !(self.max_repair_power * self.dt).is_finite()
            || !(self.idle_power * self.dt).is_finite()
        {
            return Err(MaintenanceError::NonFiniteParameter {
                name: "power_times_dt",
                value: self.max_repair_power.max(self.idle_power) * self.dt,
            });
        }
        Ok(())
    }
}

/// One externally supplied intervention for a maintenance step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaintenanceIntervention {
    /// Damage applied to integrity before feedback repair, in [0, +∞).
    pub damage_units: f64,
    /// Energy entering the store during this step.
    pub harvested_energy: f64,
    /// Whether the fixed controller is enabled.
    pub controller_enabled: bool,
    /// Whether the controller can observe the current integrity error.
    pub sensor_available: bool,
}

impl Default for MaintenanceIntervention {
    fn default() -> Self {
        Self {
            damage_units: 0.0,
            harvested_energy: 0.0,
            controller_enabled: true,
            sensor_available: true,
        }
    }
}

/// One-step observable and energy-accounting receipt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaintenanceReport {
    pub integrity_before: f64,
    pub integrity_after: f64,
    pub damage_requested: f64,
    pub damage_applied: f64,
    pub repair_applied: f64,
    pub energy_before: f64,
    pub energy_harvested: f64,
    pub energy_spilled: f64,
    pub idle_energy_consumed: f64,
    pub repair_energy_consumed: f64,
    pub energy_after: f64,
    /// (energy before + harvested) - (spilled + idle + repair + energy after).
    pub energy_balance_residual: f64,
    pub controller_enabled: bool,
    pub sensor_available: bool,
}

/// Current state of the fixed-rule maintenance system.
#[derive(Debug, Clone, PartialEq)]
pub struct MaintenanceSystem {
    config: MaintenanceConfig,
    integrity: f64,
    stored_energy: f64,
    tick: u64,
}

impl MaintenanceSystem {
    pub fn new(
        config: MaintenanceConfig,
        integrity: f64,
        stored_energy: f64,
    ) -> Result<Self, MaintenanceError> {
        config.validate()?;
        if !integrity.is_finite() || !(0.0..=1.0).contains(&integrity) {
            return Err(MaintenanceError::StateOutOfRange {
                name: "integrity",
                value: integrity,
                minimum: 0.0,
                maximum: 1.0,
            });
        }
        if !stored_energy.is_finite()
            || stored_energy < 0.0
            || stored_energy > config.max_stored_energy
        {
            return Err(MaintenanceError::StateOutOfRange {
                name: "stored_energy",
                value: stored_energy,
                minimum: 0.0,
                maximum: config.max_stored_energy,
            });
        }
        Ok(Self {
            config,
            integrity,
            stored_energy,
            tick: 0,
        })
    }

    pub const fn config(&self) -> MaintenanceConfig {
        self.config
    }

    pub const fn integrity(&self) -> f64 {
        self.integrity
    }

    pub const fn stored_energy(&self) -> f64 {
        self.stored_energy
    }

    pub const fn tick(&self) -> u64 {
        self.tick
    }

    /// Apply damage, receive energy, pay idle cost, then perform bounded feedback repair.
    ///
    /// Energy conservation is:
    /// E_before + E_harvested = E_spilled + E_idle + E_repair + E_after.
    ///
    /// Integrity accounting is:
    /// I_after = I_before - damage_applied + repair_applied.
    ///
    /// Repair is permitted only when both controller and sensor are enabled, the integrity
    /// is below target, and sufficient stored/harvested energy remains after idle cost.
    pub fn step(
        &mut self,
        intervention: MaintenanceIntervention,
    ) -> Result<MaintenanceReport, MaintenanceError> {
        let next_tick = self.tick.checked_add(1).ok_or(MaintenanceError::TickOverflow)?;
        for (name, value) in [
            ("damage_units", intervention.damage_units),
            ("harvested_energy", intervention.harvested_energy),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(MaintenanceError::InvalidIntervention { name, value });
            }
        }

        let integrity_before = self.integrity;
        let energy_before = self.stored_energy;

        let damage_applied = intervention.damage_units.min(integrity_before);
        let integrity_after_damage = integrity_before - damage_applied;

        let energy_including_harvest = energy_before + intervention.harvested_energy;
        if !energy_including_harvest.is_finite() {
            return Err(MaintenanceError::EnergyOverflow);
        }
        let energy_spilled =
            (energy_including_harvest - self.config.max_stored_energy).max(0.0);
        let energy_available = energy_including_harvest - energy_spilled;

        let idle_energy_requested = self.config.idle_power * self.config.dt;
        let idle_energy_consumed = idle_energy_requested.min(energy_available);
        let energy_after_idle = energy_available - idle_energy_consumed;

        let repair_enabled = intervention.controller_enabled
            && intervention.sensor_available
            && integrity_after_damage < self.config.target_integrity;
        let integrity_deficit = (self.config.target_integrity - integrity_after_damage).max(0.0);
        let repair_energy_demand =
            integrity_deficit * self.config.energy_per_integrity_unit / self.config.repair_efficiency;
        let repair_energy_power_limited = self.config.max_repair_power * self.config.dt;
        let repair_energy_consumed = if repair_enabled {
            repair_energy_demand
                .min(repair_energy_power_limited)
                .min(energy_after_idle)
        } else {
            0.0
        };
        let repair_applied = (repair_energy_consumed * self.config.repair_efficiency
            / self.config.energy_per_integrity_unit)
            .min(integrity_deficit);
        let integrity_after = integrity_after_damage + repair_applied;
        let energy_after = energy_after_idle - repair_energy_consumed;

        if !integrity_after.is_finite()
            || !energy_after.is_finite()
            || integrity_after < 0.0
            || integrity_after > 1.0
            || energy_after < 0.0
        {
            return Err(MaintenanceError::NumericalInvariantViolation);
        }

        let energy_balance_residual = energy_before + intervention.harvested_energy
            - (energy_spilled + idle_energy_consumed + repair_energy_consumed + energy_after);

        self.integrity = integrity_after;
        self.stored_energy = energy_after;
        self.tick = next_tick;

        Ok(MaintenanceReport {
            integrity_before,
            integrity_after,
            damage_requested: intervention.damage_units,
            damage_applied,
            repair_applied,
            energy_before,
            energy_harvested: intervention.harvested_energy,
            energy_spilled,
            idle_energy_consumed,
            repair_energy_consumed,
            energy_after,
            energy_balance_residual,
            controller_enabled: intervention.controller_enabled,
            sensor_available: intervention.sensor_available,
        })
    }
}

/// Invalid system configuration, state, intervention, or numerical invariant.
#[derive(Debug, Clone, PartialEq)]
pub enum MaintenanceError {
    NonFiniteParameter {
        name: &'static str,
        value: f64,
    },
    NegativeParameter {
        name: &'static str,
        value: f64,
    },
    NonPositiveParameter {
        name: &'static str,
        value: f64,
    },
    ParameterOutOfRange {
        name: &'static str,
        value: f64,
        minimum: f64,
        maximum: f64,
    },
    StateOutOfRange {
        name: &'static str,
        value: f64,
        minimum: f64,
        maximum: f64,
    },
    InvalidIntervention {
        name: &'static str,
        value: f64,
    },
    EnergyOverflow,
    TickOverflow,
    NumericalInvariantViolation,
}

impl fmt::Display for MaintenanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteParameter { name, value } => {
                write!(f, "{name} must be finite, got {value}")
            }
            Self::NegativeParameter { name, value } => {
                write!(f, "{name} must be non-negative, got {value}")
            }
            Self::NonPositiveParameter { name, value } => {
                write!(f, "{name} must be positive, got {value}")
            }
            Self::ParameterOutOfRange {
                name,
                value,
                minimum,
                maximum,
            } => write!(
                f,
                "{name}={value} must be in [{minimum}, {maximum}]"
            ),
            Self::StateOutOfRange {
                name,
                value,
                minimum,
                maximum,
            } => write!(
                f,
                "{name}={value} must be in [{minimum}, {maximum}]"
            ),
            Self::InvalidIntervention { name, value } => {
                write!(f, "intervention {name} must be finite and >= 0, got {value}")
            }
            Self::EnergyOverflow => write!(f, "stored energy plus harvested energy overflowed"),
            Self::TickOverflow => write!(f, "maintenance tick counter overflow"),
            Self::NumericalInvariantViolation => {
                write!(f, "maintenance system violated an integrity or energy invariant")
            }
        }
    }
}

impl Error for MaintenanceError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> MaintenanceConfig {
        MaintenanceConfig {
            target_integrity: 1.0,
            energy_per_integrity_unit: 1.0,
            repair_efficiency: 1.0,
            max_repair_power: 1.0,
            idle_power: 0.0,
            max_stored_energy: 10.0,
            dt: 1.0,
        }
    }

    #[test]
    fn feedback_repair_restores_integrity_using_a_finite_energy_budget() {
        let mut system = MaintenanceSystem::new(config(), 0.5, 10.0).unwrap();
        let report = system
            .step(MaintenanceIntervention {
                damage_units: 0.2,
                ..MaintenanceIntervention::default()
            })
            .unwrap();

        assert!((report.integrity_after - 1.0).abs() < 1.0e-12);
        assert!((report.damage_applied - 0.2).abs() < 1.0e-12);
        assert!((report.repair_applied - 0.7).abs() < 1.0e-12);
        assert!((report.repair_energy_consumed - 0.7).abs() < 1.0e-12);
        assert!((system.stored_energy() - 9.3).abs() < 1.0e-12);
    }

    #[test]
    fn disabling_controller_prevents_repair_after_identical_damage() {
        let mut system = MaintenanceSystem::new(config(), 0.5, 10.0).unwrap();
        let report = system
            .step(MaintenanceIntervention {
                damage_units: 0.2,
                controller_enabled: false,
                ..MaintenanceIntervention::default()
            })
            .unwrap();

        assert!((report.integrity_after - 0.3).abs() < 1.0e-12);
        assert_eq!(report.repair_applied, 0.0);
        assert_eq!(report.repair_energy_consumed, 0.0);
    }

    #[test]
    fn disabling_sensor_prevents_feedback_repair() {
        let mut system = MaintenanceSystem::new(config(), 0.5, 10.0).unwrap();
        let report = system
            .step(MaintenanceIntervention {
                damage_units: 0.2,
                sensor_available: false,
                ..MaintenanceIntervention::default()
            })
            .unwrap();

        assert!((report.integrity_after - 0.3).abs() < 1.0e-12);
        assert_eq!(report.repair_applied, 0.0);
    }

    #[test]
    fn depleted_energy_prevents_maintenance_without_creating_energy() {
        let mut system = MaintenanceSystem::new(config(), 0.5, 0.0).unwrap();
        let report = system
            .step(MaintenanceIntervention {
                damage_units: 0.2,
                ..MaintenanceIntervention::default()
            })
            .unwrap();

        assert!((report.integrity_after - 0.3).abs() < 1.0e-12);
        assert_eq!(report.repair_applied, 0.0);
        assert_eq!(report.energy_after, 0.0);
    }

    #[test]
    fn energy_ledger_accounts_for_harvest_spill_idle_and_repair() {
        let mut params = config();
        params.max_stored_energy = 10.0;
        params.max_repair_power = 0.4;
        params.idle_power = 0.5;
        let mut system = MaintenanceSystem::new(params, 0.5, 9.0).unwrap();
        let report = system
            .step(MaintenanceIntervention {
                damage_units: 0.1,
                harvested_energy: 2.0,
                ..MaintenanceIntervention::default()
            })
            .unwrap();

        assert!((report.energy_spilled - 1.0).abs() < 1.0e-12);
        assert!((report.idle_energy_consumed - 0.5).abs() < 1.0e-12);
        assert!((report.repair_energy_consumed - 0.4).abs() < 1.0e-12);
        assert!((report.energy_after - 9.1).abs() < 1.0e-12);
        assert!(report.energy_balance_residual.abs() < 1.0e-12);
    }

    #[test]
    fn harvest_can_refill_energy_and_allow_recovery_after_depletion() {
        let mut system = MaintenanceSystem::new(config(), 0.2, 0.0).unwrap();
        let first = system
            .step(MaintenanceIntervention {
                damage_units: 0.1,
                ..MaintenanceIntervention::default()
            })
            .unwrap();
        assert_eq!(first.repair_applied, 0.0);

        let second = system
            .step(MaintenanceIntervention {
                harvested_energy: 0.8,
                ..MaintenanceIntervention::default()
            })
            .unwrap();
        assert!((second.repair_applied - 0.8).abs() < 1.0e-12);
        assert!((second.integrity_after - 0.9).abs() < 1.0e-12);
    }

    #[test]
    fn no_damage_means_no_unnecessary_repair_or_energy_use() {
        let mut system = MaintenanceSystem::new(config(), 1.0, 5.0).unwrap();
        let report = system.step(MaintenanceIntervention::default()).unwrap();
        assert_eq!(report.repair_applied, 0.0);
        assert_eq!(report.repair_energy_consumed, 0.0);
        assert_eq!(report.energy_after, 5.0);
    }

    #[test]
    fn damage_cannot_reduce_integrity_below_zero() {
        let mut system = MaintenanceSystem::new(config(), 0.1, 0.0).unwrap();
        let report = system
            .step(MaintenanceIntervention {
                damage_units: 2.0,
                controller_enabled: false,
                ..MaintenanceIntervention::default()
            })
            .unwrap();
        assert_eq!(report.damage_applied, 0.1);
        assert_eq!(report.integrity_after, 0.0);
    }

    #[test]
    fn rejects_invalid_configuration_state_and_interventions() {
        assert!(matches!(
            MaintenanceSystem::new(
                MaintenanceConfig {
                    repair_efficiency: 0.0,
                    ..config()
                },
                0.5,
                1.0
            ),
            Err(MaintenanceError::ParameterOutOfRange {
                name: "repair_efficiency",
                ..
            })
        ));
        assert!(matches!(
            MaintenanceSystem::new(config(), 1.1, 0.0),
            Err(MaintenanceError::StateOutOfRange {
                name: "integrity",
                ..
            })
        ));
        let mut system = MaintenanceSystem::new(config(), 0.5, 1.0).unwrap();
        assert!(matches!(
            system.step(MaintenanceIntervention {
                damage_units: f64::NAN,
                ..MaintenanceIntervention::default()
            }),
            Err(MaintenanceError::InvalidIntervention {
                name: "damage_units",
                ..
            })
        ));
    }
}
