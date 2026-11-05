// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Repeated-shock resource-capital reference experiment.
//!
//! This module composes the same CivDyn physical ledger to distinguish service
//! recovery from recovery of the reserve stock needed for future shocks. It is
//! a deterministic synthetic water-only experiment, not a regional forecast.

use civdyn_resource_ledger::{
    Production, Region, ResourceVector, StepInput, TransferPolicy, TransferRequest, World,
    WorldError,
};
use core::fmt;

pub const REGION_COUNT: usize = 3;
pub const BASIN: usize = 0;
pub const HOUSEHOLD_A: usize = 1;
pub const HOUSEHOLD_B: usize = 2;
pub const HOUSEHOLD_DEMAND_M3: f64 = 12.5;
pub const TOTAL_DEMAND_M3: f64 = HOUSEHOLD_DEMAND_M3 * 2.0;
pub const INITIAL_RESERVE_M3: f64 = 200.0;
pub const PRE_SHOCK_TICKS: usize = 2;
pub const FIRST_SHOCK_TICKS: usize = 4;
pub const RECOVERY_TICKS: usize = 2;
pub const REPEAT_SHOCK_TICKS: usize = 6;
pub const TOTAL_TICKS: usize =
    PRE_SHOCK_TICKS + FIRST_SHOCK_TICKS + RECOVERY_TICKS + REPEAT_SHOCK_TICKS;
pub const RECOVERY_THRESHOLD: f64 = 0.95;
pub const SUSTAINED_RECOVERY_STEPS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryPath {
    /// Recovery inflow only meets contemporaneous household demand.
    ServiceOnly,
    /// Recovery inflow meets demand and replenishes the basin reserve.
    CapitalRebuilding,
}

impl RecoveryPath {
    pub const ALL: [Self; 2] = [Self::ServiceOnly, Self::CapitalRebuilding];

    pub const fn name(self) -> &'static str {
        match self {
            Self::ServiceOnly => "service_only",
            Self::CapitalRebuilding => "capital_rebuilding",
        }
    }

    fn recovery_inflow_m3(self) -> f64 {
        match self {
            Self::ServiceOnly => TOTAL_DEMAND_M3,
            Self::CapitalRebuilding => TOTAL_DEMAND_M3 * 2.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryPhase {
    PreShock,
    FirstShock,
    ServiceRecovery,
    RepeatShock,
}

impl RecoveryPhase {
    pub const fn name(self) -> &'static str {
        match self {
            Self::PreShock => "pre_shock",
            Self::FirstShock => "first_shock",
            Self::ServiceRecovery => "service_recovery",
            Self::RepeatShock => "repeat_shock",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecoveryCapitalTick {
    pub tick: u64,
    pub phase: RecoveryPhase,
    pub household_a_coverage: f64,
    pub household_b_coverage: f64,
    pub unmet_water_m3: f64,
    pub basin_closing_stock_m3: f64,
    pub maximum_conservation_residual_m3: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecoveryCapitalOutcome {
    pub path: RecoveryPath,
    pub ticks: Vec<RecoveryCapitalTick>,
    pub first_shock_unmet_water_m3: f64,
    pub service_recovery_unmet_water_m3: f64,
    pub repeat_shock_unmet_water_m3: f64,
    pub reserve_before_first_shock_m3: f64,
    pub reserve_after_first_shock_m3: f64,
    pub reserve_at_service_recovery_m3: f64,
    /// Net reserve increase between the end of the first shock and the end of
    /// the declared service-recovery interval.
    pub reserve_replenishment_m3: f64,
    /// Reserve at service recovery divided by pre-shock reserve.
    pub reserve_recovery_ratio: f64,
    /// First post-shock tick of the first sustained interval meeting the
    /// threshold, measured from the end of the first shock.
    pub service_recovery_after_ticks: Option<usize>,
    pub maximum_conservation_residual_m3: f64,
    pub final_reserve_m3: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryCapitalError {
    Ledger(WorldError),
    WrongRegionCount { expected: usize, actual: usize },
    InvalidOutcomeTrajectory { expected: usize, actual: usize },
    MissingRecoveryInterval,
}

impl fmt::Display for RecoveryCapitalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ledger(error) => write!(f, "resource ledger error: {error}"),
            Self::WrongRegionCount { expected, actual } => {
                write!(f, "expected {expected} resource nodes, got {actual}")
            }
            Self::InvalidOutcomeTrajectory { expected, actual } => {
                write!(f, "expected {expected} ticks, got {actual}")
            }
            Self::MissingRecoveryInterval => {
                write!(f, "service-recovery interval was not present")
            }
        }
    }
}

impl std::error::Error for RecoveryCapitalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Ledger(error) => Some(error),
            _ => None,
        }
    }
}

impl From<WorldError> for RecoveryCapitalError {
    fn from(value: WorldError) -> Self {
        Self::Ledger(value)
    }
}

pub fn run_recovery_capital_reference(
    path: RecoveryPath,
) -> Result<RecoveryCapitalOutcome, RecoveryCapitalError> {
    let regions = vec![
        Region {
            id: "basin".into(),
            population: 0.0,
            opening_stock: ResourceVector::new(INITIAL_RESERVE_M3, 0.0, 0.0),
            essential_demand_per_capita: ResourceVector::default(),
        },
        Region {
            id: "household_a".into(),
            population: 1.0,
            opening_stock: ResourceVector::default(),
            essential_demand_per_capita: ResourceVector::new(HOUSEHOLD_DEMAND_M3, 0.0, 0.0),
        },
        Region {
            id: "household_b".into(),
            population: 1.0,
            opening_stock: ResourceVector::default(),
            essential_demand_per_capita: ResourceVector::new(HOUSEHOLD_DEMAND_M3, 0.0, 0.0),
        },
    ];
    let mut world = World::new(regions)?;
    if world.regions().len() != REGION_COUNT {
        return Err(RecoveryCapitalError::WrongRegionCount {
            expected: REGION_COUNT,
            actual: world.regions().len(),
        });
    }

    let mut ticks = Vec::with_capacity(TOTAL_TICKS);
    let mut first_shock_unmet_water_m3 = 0.0;
    let mut service_recovery_unmet_water_m3 = 0.0;
    let mut repeat_shock_unmet_water_m3 = 0.0;
    let reserve_before_first_shock_m3 = INITIAL_RESERVE_M3;
    let mut reserve_after_first_shock_m3 = None;
    let mut reserve_at_service_recovery_m3 = None;
    let mut recovery_streak = 0usize;
    let mut service_recovery_after_ticks = None;

    for tick_index in 0..TOTAL_TICKS {
        let phase = if tick_index < PRE_SHOCK_TICKS {
            RecoveryPhase::PreShock
        } else if tick_index < PRE_SHOCK_TICKS + FIRST_SHOCK_TICKS {
            RecoveryPhase::FirstShock
        } else if tick_index < PRE_SHOCK_TICKS + FIRST_SHOCK_TICKS + RECOVERY_TICKS {
            RecoveryPhase::ServiceRecovery
        } else {
            RecoveryPhase::RepeatShock
        };

        let production_water_m3 = match phase {
            RecoveryPhase::PreShock => TOTAL_DEMAND_M3,
            RecoveryPhase::FirstShock | RecoveryPhase::RepeatShock => 0.0,
            RecoveryPhase::ServiceRecovery => path.recovery_inflow_m3(),
        };
        let available_before_transfers =
            world.regions()[BASIN].opening_stock.water_m3 + production_water_m3;
        let transfer_total = available_before_transfers.min(TOTAL_DEMAND_M3);
        let transfer_a = transfer_total * 0.5;
        let transfer_b = transfer_total - transfer_a;

        let report = world.step(
            &StepInput {
                production: vec![
                    Production {
                        resources: ResourceVector::new(production_water_m3, 0.0, 0.0),
                    },
                    Production::default(),
                    Production::default(),
                ],
                transfers: vec![
                    TransferRequest {
                        from_region: BASIN,
                        to_region: HOUSEHOLD_A,
                        amount: ResourceVector::new(transfer_a, 0.0, 0.0),
                    },
                    TransferRequest {
                        from_region: BASIN,
                        to_region: HOUSEHOLD_B,
                        amount: ResourceVector::new(transfer_b, 0.0, 0.0),
                    },
                ],
            },
            TransferPolicy::Unrestricted,
        )?;

        let unmet_water_m3 = report.regions[HOUSEHOLD_A].unmet.water_m3
            + report.regions[HOUSEHOLD_B].unmet.water_m3;
        match phase {
            RecoveryPhase::FirstShock => first_shock_unmet_water_m3 += unmet_water_m3,
            RecoveryPhase::ServiceRecovery => {
                service_recovery_unmet_water_m3 += unmet_water_m3;
                let all_households_recovered = report.regions[HOUSEHOLD_A]
                    .coverage
                    .water
                    .unwrap_or(0.0)
                    >= RECOVERY_THRESHOLD
                    && report.regions[HOUSEHOLD_B]
                        .coverage
                        .water
                        .unwrap_or(0.0)
                        >= RECOVERY_THRESHOLD;
                if all_households_recovered {
                    recovery_streak += 1;
                } else {
                    recovery_streak = 0;
                }
                if service_recovery_after_ticks.is_none()
                    && recovery_streak >= SUSTAINED_RECOVERY_STEPS
                {
                    service_recovery_after_ticks = Some(
                        tick_index + 1 - (PRE_SHOCK_TICKS + FIRST_SHOCK_TICKS),
                    );
                }
            }
            RecoveryPhase::RepeatShock => repeat_shock_unmet_water_m3 += unmet_water_m3,
            RecoveryPhase::PreShock => {}
        }

        let basin_stock = report.regions[BASIN].closing_stock.water_m3;
        if tick_index + 1 == PRE_SHOCK_TICKS + FIRST_SHOCK_TICKS {
            reserve_after_first_shock_m3 = Some(basin_stock);
        }
        if tick_index + 1 == PRE_SHOCK_TICKS + FIRST_SHOCK_TICKS + RECOVERY_TICKS {
            reserve_at_service_recovery_m3 = Some(basin_stock);
        }

        let mut maximum_residual = 0.0_f64;
        for region in &report.regions {
            if !region.ledger.is_conserved() {
                return Err(RecoveryCapitalError::Ledger(
                    WorldError::ConservationViolation {
                        region: region.region_id.clone(),
                        residual: region.ledger.conservation_residual(),
                    },
                ));
            }
            maximum_residual = maximum_residual
                .max(region.ledger.conservation_residual().water_m3.abs());
        }
        ticks.push(RecoveryCapitalTick {
            tick: report.tick,
            phase,
            household_a_coverage: report.regions[HOUSEHOLD_A]
                .coverage
                .water
                .unwrap_or(0.0),
            household_b_coverage: report.regions[HOUSEHOLD_B]
                .coverage
                .water
                .unwrap_or(0.0),
            unmet_water_m3,
            basin_closing_stock_m3: basin_stock,
            maximum_conservation_residual_m3: maximum_residual,
        });
    }

    if ticks.len() != TOTAL_TICKS {
        return Err(RecoveryCapitalError::InvalidOutcomeTrajectory {
            expected: TOTAL_TICKS,
            actual: ticks.len(),
        });
    }
    let reserve_after_first_shock_m3 =
        reserve_after_first_shock_m3.ok_or(RecoveryCapitalError::MissingRecoveryInterval)?;
    let reserve_at_service_recovery_m3 =
        reserve_at_service_recovery_m3.ok_or(RecoveryCapitalError::MissingRecoveryInterval)?;
    let reserve_replenishment_m3 =
        reserve_at_service_recovery_m3 - reserve_after_first_shock_m3;
    let maximum_conservation_residual_m3 = ticks
        .iter()
        .map(|tick| tick.maximum_conservation_residual_m3)
        .fold(0.0_f64, f64::max);

    Ok(RecoveryCapitalOutcome {
        path,
        ticks,
        first_shock_unmet_water_m3,
        service_recovery_unmet_water_m3,
        repeat_shock_unmet_water_m3,
        reserve_before_first_shock_m3,
        reserve_after_first_shock_m3,
        reserve_at_service_recovery_m3,
        reserve_replenishment_m3,
        reserve_recovery_ratio: reserve_at_service_recovery_m3
            / reserve_before_first_shock_m3,
        service_recovery_after_ticks,
        maximum_conservation_residual_m3,
        final_reserve_m3: world.regions()[BASIN].opening_stock.water_m3,
    })
}

pub fn run_recovery_capital_portfolio(
) -> Result<Vec<RecoveryCapitalOutcome>, RecoveryCapitalError> {
    RecoveryPath::ALL
        .into_iter()
        .map(run_recovery_capital_reference)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_paths_meet_first_shock_service_and_sustained_service_recovery() {
        for outcome in run_recovery_capital_portfolio().unwrap() {
            assert_eq!(outcome.first_shock_unmet_water_m3, 0.0);
            assert_eq!(outcome.service_recovery_unmet_water_m3, 0.0);
            assert_eq!(outcome.service_recovery_after_ticks, Some(2));
            let recovery_ticks = outcome
                .ticks
                .iter()
                .filter(|tick| tick.phase == RecoveryPhase::ServiceRecovery)
                .collect::<Vec<_>>();
            assert_eq!(recovery_ticks.len(), RECOVERY_TICKS);
            assert!(recovery_ticks.iter().all(|tick| {
                tick.household_a_coverage >= RECOVERY_THRESHOLD
                    && tick.household_b_coverage >= RECOVERY_THRESHOLD
            }));
        }
    }

    #[test]
    fn service_only_recovery_does_not_replenish_reserve() {
        let outcome = run_recovery_capital_reference(RecoveryPath::ServiceOnly).unwrap();
        assert_eq!(outcome.reserve_after_first_shock_m3, 100.0);
        assert_eq!(outcome.reserve_at_service_recovery_m3, 100.0);
        assert_eq!(outcome.reserve_replenishment_m3, 0.0);
        assert_eq!(outcome.reserve_recovery_ratio, 0.5);
    }

    #[test]
    fn capital_rebuilding_restores_fifty_cubic_metres_of_reserve() {
        let outcome = run_recovery_capital_reference(RecoveryPath::CapitalRebuilding).unwrap();
        assert_eq!(outcome.reserve_after_first_shock_m3, 100.0);
        assert_eq!(outcome.reserve_at_service_recovery_m3, 150.0);
        assert_eq!(outcome.reserve_replenishment_m3, 50.0);
        assert_eq!(outcome.reserve_recovery_ratio, 0.75);
    }

    #[test]
    fn repeat_shock_exposes_unreplenished_reserve() {
        let service_only = run_recovery_capital_reference(RecoveryPath::ServiceOnly).unwrap();
        let rebuilding = run_recovery_capital_reference(RecoveryPath::CapitalRebuilding).unwrap();
        assert_eq!(service_only.repeat_shock_unmet_water_m3, 50.0);
        assert_eq!(rebuilding.repeat_shock_unmet_water_m3, 0.0);
    }

    #[test]
    fn every_node_and_tick_preserves_water_balance() {
        for outcome in run_recovery_capital_portfolio().unwrap() {
            assert!(outcome.maximum_conservation_residual_m3 < 1e-9);
            assert!(
                outcome
                    .ticks
                    .iter()
                    .all(|tick| tick.maximum_conservation_residual_m3 < 1e-9)
            );
        }
    }

    #[test]
    fn repeated_execution_is_deterministic() {
        assert_eq!(
            run_recovery_capital_portfolio().unwrap(),
            run_recovery_capital_portfolio().unwrap()
        );
    }
}
