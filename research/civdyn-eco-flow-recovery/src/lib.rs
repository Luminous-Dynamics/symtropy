// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Repeated-shock experiment separating household service, environmental-flow
//! delivery and source reserve recovery. The resource ledger is the sole
//! physical stock-flow authority. The environmental-flow node is an explicit
//! obligation sink, not a model of ecosystem health.

use civdyn_resource_ledger::{
    Production, Region, ResourceVector, StepInput, TransferPolicy, TransferRequest, World,
    WorldError,
};
use core::fmt;

pub const REGION_COUNT: usize = 4;
pub const BASIN: usize = 0;
pub const HOUSEHOLD_A: usize = 1;
pub const HOUSEHOLD_B: usize = 2;
pub const WETLAND_FLOW_OBLIGATION: usize = 3;

pub const INITIAL_RESERVE_M3: f64 = 200.0;
pub const HOUSEHOLD_DEMAND_PER_TICK_M3: f64 = 12.5;
pub const HOUSEHOLD_COUNT: usize = 2;
pub const TOTAL_HOUSEHOLD_DEMAND_M3: f64 =
    HOUSEHOLD_DEMAND_PER_TICK_M3 * HOUSEHOLD_COUNT as f64;
pub const ENVIRONMENTAL_FLOW_DEMAND_M3: f64 = 10.0;
pub const TOTAL_SYSTEM_DEMAND_M3: f64 =
    TOTAL_HOUSEHOLD_DEMAND_M3 + ENVIRONMENTAL_FLOW_DEMAND_M3;

pub const PRE_SHOCK_TICKS: usize = 2;
pub const FIRST_SHOCK_TICKS: usize = 4;
pub const RECOVERY_TICKS: usize = 2;
pub const REPEATED_SHOCK_TICKS: usize = 6;
pub const TOTAL_TICKS: usize =
    PRE_SHOCK_TICKS + FIRST_SHOCK_TICKS + RECOVERY_TICKS + REPEATED_SHOCK_TICKS;

pub const RECOVERY_THRESHOLD: f64 = 0.95;
pub const SUSTAINED_RECOVERY_TICKS: usize = 2;
pub const CONSERVATION_TOLERANCE_M3: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryPath {
    /// Recharge only meets household plus environmental-flow demand.
    ServiceOnly,
    /// Recharge meets current demand and retains an additional 20 m³/tick.
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

    fn recovery_recharge_m3(self) -> f64 {
        match self {
            Self::ServiceOnly => TOTAL_SYSTEM_DEMAND_M3,
            Self::CapitalRebuilding => TOTAL_SYSTEM_DEMAND_M3 + 20.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationPriority {
    /// Deliver household requirements first; the environmental-flow obligation
    /// receives only the stock remaining after the household allocation.
    HouseholdPriority,
    /// Deliver the environmental-flow obligation first; households share the
    /// stock remaining after that allocation.
    EcologicalFlowPriority,
}

impl AllocationPriority {
    pub const ALL: [Self; 2] = [Self::HouseholdPriority, Self::EcologicalFlowPriority];

    pub const fn name(self) -> &'static str {
        match self {
            Self::HouseholdPriority => "household_priority",
            Self::EcologicalFlowPriority => "ecological_flow_priority",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    PreShock,
    FirstShock,
    ServiceRecovery,
    RepeatedShock,
}

impl Phase {
    pub const fn name(self) -> &'static str {
        match self {
            Self::PreShock => "pre_shock",
            Self::FirstShock => "first_shock",
            Self::ServiceRecovery => "service_recovery",
            Self::RepeatedShock => "repeated_shock",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Allocation {
    household_a_m3: f64,
    household_b_m3: f64,
    environmental_flow_m3: f64,
}

impl Allocation {
    fn household_total_m3(self) -> f64 {
        self.household_a_m3 + self.household_b_m3
    }

    fn total_m3(self) -> f64 {
        self.household_total_m3() + self.environmental_flow_m3
    }
}

fn allocate(available_m3: f64, priority: AllocationPriority) -> Allocation {
    let available_m3 = available_m3.max(0.0);
    match priority {
        AllocationPriority::HouseholdPriority => {
            let household_total = available_m3.min(TOTAL_HOUSEHOLD_DEMAND_M3);
            let remaining = (available_m3 - household_total).max(0.0);
            let environmental = remaining.min(ENVIRONMENTAL_FLOW_DEMAND_M3);
            Allocation {
                household_a_m3: household_total * 0.5,
                household_b_m3: household_total - household_total * 0.5,
                environmental_flow_m3: environmental,
            }
        }
        AllocationPriority::EcologicalFlowPriority => {
            let environmental = available_m3.min(ENVIRONMENTAL_FLOW_DEMAND_M3);
            let remaining = (available_m3 - environmental).max(0.0);
            let household_total = remaining.min(TOTAL_HOUSEHOLD_DEMAND_M3);
            Allocation {
                household_a_m3: household_total * 0.5,
                household_b_m3: household_total - household_total * 0.5,
                environmental_flow_m3: environmental,
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecoveryTick {
    pub tick: u64,
    pub phase: Phase,
    pub recharge_m3: f64,
    pub household_a_coverage: f64,
    pub household_b_coverage: f64,
    pub environmental_flow_coverage: f64,
    pub household_unmet_m3: f64,
    pub environmental_flow_unmet_m3: f64,
    pub requested_household_delivery_m3: f64,
    pub requested_environmental_flow_m3: f64,
    pub basin_closing_stock_m3: f64,
    pub maximum_conservation_residual_m3: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecoveryOutcome {
    pub path: RecoveryPath,
    pub priority: AllocationPriority,
    pub ticks: Vec<RecoveryTick>,
    pub first_shock_household_unmet_m3: f64,
    pub first_shock_environmental_flow_unmet_m3: f64,
    pub recovery_household_unmet_m3: f64,
    pub recovery_environmental_flow_unmet_m3: f64,
    pub repeat_shock_household_unmet_m3: f64,
    pub repeat_shock_environmental_flow_unmet_m3: f64,
    pub reserve_before_first_shock_m3: f64,
    pub reserve_after_first_shock_m3: f64,
    pub reserve_after_service_recovery_m3: f64,
    pub reserve_rebuilding_m3: f64,
    pub reserve_recovery_ratio: f64,
    pub household_recovery_after_ticks: Option<usize>,
    pub environmental_flow_recovery_after_ticks: Option<usize>,
    pub minimum_repeat_household_coverage: f64,
    pub minimum_repeat_environmental_flow_coverage: f64,
    pub final_reserve_m3: f64,
    pub maximum_conservation_residual_m3: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EcoFlowError {
    Ledger(WorldError),
    WrongRegionCount { expected: usize, actual: usize },
    InvalidTrajectory { expected: usize, actual: usize },
    MissingReserveSnapshot,
    InvalidAllocation { available_m3: f64, allocated_m3: f64 },
    ConservationViolation { region: String, residual_m3: f64 },
}

impl fmt::Display for EcoFlowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ledger(error) => write!(formatter, "resource ledger error: {error}"),
            Self::WrongRegionCount { expected, actual } => {
                write!(formatter, "expected {expected} resource nodes, got {actual}")
            }
            Self::InvalidTrajectory { expected, actual } => {
                write!(formatter, "expected {expected} ticks, got {actual}")
            }
            Self::MissingReserveSnapshot => write!(formatter, "required reserve snapshot is missing"),
            Self::InvalidAllocation { available_m3, allocated_m3 } => write!(
                formatter,
                "allocation {allocated_m3} m³ exceeds available stock {available_m3} m³"
            ),
            Self::ConservationViolation { region, residual_m3 } => write!(
                formatter,
                "water conservation violation at {region}: {residual_m3} m³"
            ),
        }
    }
}

impl std::error::Error for EcoFlowError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Ledger(error) => Some(error),
            _ => None,
        }
    }
}

impl From<WorldError> for EcoFlowError {
    fn from(value: WorldError) -> Self {
        Self::Ledger(value)
    }
}

fn reference_world() -> Result<World, EcoFlowError> {
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
            essential_demand_per_capita: ResourceVector::new(
                HOUSEHOLD_DEMAND_PER_TICK_M3,
                0.0,
                0.0,
            ),
        },
        Region {
            id: "household_b".into(),
            population: 1.0,
            opening_stock: ResourceVector::default(),
            essential_demand_per_capita: ResourceVector::new(
                HOUSEHOLD_DEMAND_PER_TICK_M3,
                0.0,
                0.0,
            ),
        },
        Region {
            id: "wetland_flow_obligation".into(),
            // One synthetic obligation unit, not a human or ecological population.
            population: 1.0,
            opening_stock: ResourceVector::default(),
            essential_demand_per_capita: ResourceVector::new(
                ENVIRONMENTAL_FLOW_DEMAND_M3,
                0.0,
                0.0,
            ),
        },
    ];
    let world = World::new(regions)?;
    if world.regions().len() != REGION_COUNT {
        return Err(EcoFlowError::WrongRegionCount {
            expected: REGION_COUNT,
            actual: world.regions().len(),
        });
    }
    Ok(world)
}

fn phase_at(tick_index: usize) -> Phase {
    if tick_index < PRE_SHOCK_TICKS {
        Phase::PreShock
    } else if tick_index < PRE_SHOCK_TICKS + FIRST_SHOCK_TICKS {
        Phase::FirstShock
    } else if tick_index < PRE_SHOCK_TICKS + FIRST_SHOCK_TICKS + RECOVERY_TICKS {
        Phase::ServiceRecovery
    } else {
        Phase::RepeatedShock
    }
}

fn recharge_for(phase: Phase, path: RecoveryPath) -> f64 {
    match phase {
        Phase::PreShock => TOTAL_SYSTEM_DEMAND_M3,
        Phase::FirstShock | Phase::RepeatedShock => 0.0,
        Phase::ServiceRecovery => path.recovery_recharge_m3(),
    }
}

fn recovery_latency(
    ticks: &[RecoveryTick],
    environmental_flow: bool,
) -> Option<usize> {
    let first_recovery_index = PRE_SHOCK_TICKS + FIRST_SHOCK_TICKS;
    let mut streak = 0usize;

    for (index, tick) in ticks.iter().enumerate().skip(first_recovery_index) {
        let recovered = if environmental_flow {
            tick.environmental_flow_coverage >= RECOVERY_THRESHOLD
        } else {
            tick.household_a_coverage >= RECOVERY_THRESHOLD
                && tick.household_b_coverage >= RECOVERY_THRESHOLD
        };
        if recovered {
            streak += 1;
        } else {
            streak = 0;
        }
        if streak >= SUSTAINED_RECOVERY_TICKS {
            return Some(index + 1 - first_recovery_index);
        }
    }
    None
}

pub fn run_reference(
    path: RecoveryPath,
    priority: AllocationPriority,
) -> Result<RecoveryOutcome, EcoFlowError> {
    let mut world = reference_world()?;
    let mut ticks = Vec::with_capacity(TOTAL_TICKS);
    let mut first_shock_household_unmet_m3 = 0.0;
    let mut first_shock_environmental_flow_unmet_m3 = 0.0;
    let mut recovery_household_unmet_m3 = 0.0;
    let mut recovery_environmental_flow_unmet_m3 = 0.0;
    let mut repeat_shock_household_unmet_m3 = 0.0;
    let mut repeat_shock_environmental_flow_unmet_m3 = 0.0;
    let mut reserve_after_first_shock_m3 = None;
    let mut reserve_after_service_recovery_m3 = None;

    for tick_index in 0..TOTAL_TICKS {
        let phase = phase_at(tick_index);
        let recharge_m3 = recharge_for(phase, path);
        let opening_reserve_m3 = world.regions()[BASIN].opening_stock.water_m3;
        let available_m3 = opening_reserve_m3 + recharge_m3;
        let allocation = allocate(available_m3, priority);

        if allocation.total_m3() > available_m3 + CONSERVATION_TOLERANCE_M3 {
            return Err(EcoFlowError::InvalidAllocation {
                available_m3,
                allocated_m3: allocation.total_m3(),
            });
        }

        let mut transfers = Vec::with_capacity(3);
        for (destination, amount_m3) in [
            (HOUSEHOLD_A, allocation.household_a_m3),
            (HOUSEHOLD_B, allocation.household_b_m3),
            (WETLAND_FLOW_OBLIGATION, allocation.environmental_flow_m3),
        ] {
            if amount_m3 > 0.0 {
                transfers.push(TransferRequest {
                    from_region: BASIN,
                    to_region: destination,
                    amount: ResourceVector::new(amount_m3, 0.0, 0.0),
                });
            }
        }

        let report = world.step(
            &StepInput {
                production: vec![
                    Production {
                        resources: ResourceVector::new(recharge_m3, 0.0, 0.0),
                    },
                    Production::default(),
                    Production::default(),
                    Production::default(),
                ],
                transfers,
            },
            TransferPolicy::Unrestricted,
        )?;

        let household_a = &report.regions[HOUSEHOLD_A];
        let household_b = &report.regions[HOUSEHOLD_B];
        let wetland = &report.regions[WETLAND_FLOW_OBLIGATION];
        let household_unmet_m3 = household_a.unmet.water_m3 + household_b.unmet.water_m3;
        let environmental_flow_unmet_m3 = wetland.unmet.water_m3;
        let household_a_coverage = household_a.coverage.water.unwrap_or(0.0);
        let household_b_coverage = household_b.coverage.water.unwrap_or(0.0);
        let environmental_flow_coverage = wetland.coverage.water.unwrap_or(0.0);
        let closing_reserve_m3 = report.regions[BASIN].closing_stock.water_m3;

        match phase {
            Phase::FirstShock => {
                first_shock_household_unmet_m3 += household_unmet_m3;
                first_shock_environmental_flow_unmet_m3 += environmental_flow_unmet_m3;
            }
            Phase::ServiceRecovery => {
                recovery_household_unmet_m3 += household_unmet_m3;
                recovery_environmental_flow_unmet_m3 += environmental_flow_unmet_m3;
            }
            Phase::RepeatedShock => {
                repeat_shock_household_unmet_m3 += household_unmet_m3;
                repeat_shock_environmental_flow_unmet_m3 += environmental_flow_unmet_m3;
            }
            Phase::PreShock => {}
        }

        if tick_index + 1 == PRE_SHOCK_TICKS + FIRST_SHOCK_TICKS {
            reserve_after_first_shock_m3 = Some(closing_reserve_m3);
        }
        if tick_index + 1 == PRE_SHOCK_TICKS + FIRST_SHOCK_TICKS + RECOVERY_TICKS {
            reserve_after_service_recovery_m3 = Some(closing_reserve_m3);
        }

        let mut maximum_residual_m3 = 0.0_f64;
        for region in &report.regions {
            let residual_m3 = region.ledger.conservation_residual().water_m3.abs();
            if !region.ledger.is_conserved() || residual_m3 > CONSERVATION_TOLERANCE_M3 {
                return Err(EcoFlowError::ConservationViolation {
                    region: region.region_id.clone(),
                    residual_m3,
                });
            }
            maximum_residual_m3 = maximum_residual_m3.max(residual_m3);
        }

        ticks.push(RecoveryTick {
            tick: report.tick,
            phase,
            recharge_m3,
            household_a_coverage,
            household_b_coverage,
            environmental_flow_coverage,
            household_unmet_m3,
            environmental_flow_unmet_m3,
            requested_household_delivery_m3: allocation.household_total_m3(),
            requested_environmental_flow_m3: allocation.environmental_flow_m3,
            basin_closing_stock_m3: closing_reserve_m3,
            maximum_conservation_residual_m3: maximum_residual_m3,
        });
    }

    if ticks.len() != TOTAL_TICKS {
        return Err(EcoFlowError::InvalidTrajectory {
            expected: TOTAL_TICKS,
            actual: ticks.len(),
        });
    }
    let reserve_after_first_shock_m3 =
        reserve_after_first_shock_m3.ok_or(EcoFlowError::MissingReserveSnapshot)?;
    let reserve_after_service_recovery_m3 =
        reserve_after_service_recovery_m3.ok_or(EcoFlowError::MissingReserveSnapshot)?;
    let minimum_repeat_household_coverage = ticks
        .iter()
        .filter(|tick| tick.phase == Phase::RepeatedShock)
        .map(|tick| tick.household_a_coverage.min(tick.household_b_coverage))
        .fold(1.0_f64, f64::min);
    let minimum_repeat_environmental_flow_coverage = ticks
        .iter()
        .filter(|tick| tick.phase == Phase::RepeatedShock)
        .map(|tick| tick.environmental_flow_coverage)
        .fold(1.0_f64, f64::min);
    let maximum_conservation_residual_m3 = ticks
        .iter()
        .map(|tick| tick.maximum_conservation_residual_m3)
        .fold(0.0_f64, f64::max);
    let final_reserve_m3 = world.regions()[BASIN].opening_stock.water_m3;
    let reserve_rebuilding_m3 = reserve_after_service_recovery_m3 - reserve_after_first_shock_m3;

    Ok(RecoveryOutcome {
        path,
        priority,
        ticks: ticks.clone(),
        first_shock_household_unmet_m3,
        first_shock_environmental_flow_unmet_m3,
        recovery_household_unmet_m3,
        recovery_environmental_flow_unmet_m3,
        repeat_shock_household_unmet_m3,
        repeat_shock_environmental_flow_unmet_m3,
        reserve_before_first_shock_m3: INITIAL_RESERVE_M3,
        reserve_after_first_shock_m3,
        reserve_after_service_recovery_m3,
        reserve_rebuilding_m3,
        reserve_recovery_ratio: reserve_after_service_recovery_m3 / INITIAL_RESERVE_M3,
        household_recovery_after_ticks: recovery_latency(&ticks, false),
        environmental_flow_recovery_after_ticks: recovery_latency(&ticks, true),
        minimum_repeat_household_coverage,
        minimum_repeat_environmental_flow_coverage,
        final_reserve_m3,
        maximum_conservation_residual_m3,
    })
}

pub fn run_portfolio() -> Result<Vec<RecoveryOutcome>, EcoFlowError> {
    let mut outcomes = Vec::with_capacity(RecoveryPath::ALL.len() * AllocationPriority::ALL.len());
    for path in RecoveryPath::ALL {
        for priority in AllocationPriority::ALL {
            outcomes.push(run_reference(path, priority)?);
        }
    }
    Ok(outcomes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_recovery_does_not_prove_reserve_recovery() {
        let service = run_reference(RecoveryPath::ServiceOnly, AllocationPriority::HouseholdPriority)
            .expect("valid service-only fixture");
        let rebuilding = run_reference(
            RecoveryPath::CapitalRebuilding,
            AllocationPriority::HouseholdPriority,
        )
        .expect("valid capital-rebuilding fixture");

        assert_eq!(service.recovery_household_unmet_m3, 0.0);
        assert_eq!(rebuilding.recovery_household_unmet_m3, 0.0);
        assert_eq!(service.recovery_environmental_flow_unmet_m3, 0.0);
        assert_eq!(rebuilding.recovery_environmental_flow_unmet_m3, 0.0);
        assert_eq!(service.household_recovery_after_ticks, Some(2));
        assert_eq!(service.environmental_flow_recovery_after_ticks, Some(2));
        assert_eq!(service.reserve_after_first_shock_m3, 60.0);
        assert_eq!(service.reserve_after_service_recovery_m3, 60.0);
        assert_eq!(service.reserve_rebuilding_m3, 0.0);
        assert_eq!(rebuilding.reserve_after_service_recovery_m3, 100.0);
        assert_eq!(rebuilding.reserve_rebuilding_m3, 40.0);
    }

    #[test]
    fn allocation_priority_exposes_household_environmental_flow_tradeoff() {
        let household_first = run_reference(
            RecoveryPath::CapitalRebuilding,
            AllocationPriority::HouseholdPriority,
        )
        .expect("valid household-first fixture");
        let ecological_first = run_reference(
            RecoveryPath::CapitalRebuilding,
            AllocationPriority::EcologicalFlowPriority,
        )
        .expect("valid ecology-first fixture");

        assert_eq!(household_first.repeat_shock_household_unmet_m3, 75.0);
        assert_eq!(household_first.repeat_shock_environmental_flow_unmet_m3, 35.0);
        assert_eq!(ecological_first.repeat_shock_household_unmet_m3, 80.0);
        assert_eq!(ecological_first.repeat_shock_environmental_flow_unmet_m3, 30.0);
        assert_eq!(
            household_first.repeat_shock_household_unmet_m3
                + household_first.repeat_shock_environmental_flow_unmet_m3,
            ecological_first.repeat_shock_household_unmet_m3
                + ecological_first.repeat_shock_environmental_flow_unmet_m3
        );
    }

    #[test]
    fn household_recovery_can_coexist_with_an_environmental_flow_breach() {
        let outcome = run_reference(
            RecoveryPath::CapitalRebuilding,
            AllocationPriority::HouseholdPriority,
        )
        .expect("valid fixture");
        let counterexample = outcome.ticks.iter().find(|tick| {
            tick.phase == Phase::RepeatedShock
                && tick.household_a_coverage >= RECOVERY_THRESHOLD
                && tick.household_b_coverage >= RECOVERY_THRESHOLD
                && tick.environmental_flow_coverage < RECOVERY_THRESHOLD
        });
        assert!(counterexample.is_some());
    }

    #[test]
    fn service_only_path_has_less_repeat_shock_resilience() {
        let service = run_reference(
            RecoveryPath::ServiceOnly,
            AllocationPriority::HouseholdPriority,
        )
        .expect("valid service path");
        let rebuilding = run_reference(
            RecoveryPath::CapitalRebuilding,
            AllocationPriority::HouseholdPriority,
        )
        .expect("valid rebuilding path");

        assert!(service.repeat_shock_household_unmet_m3 > rebuilding.repeat_shock_household_unmet_m3);
        assert!(service.repeat_shock_environmental_flow_unmet_m3 > rebuilding.repeat_shock_environmental_flow_unmet_m3);
    }

    #[test]
    fn all_nodes_and_ticks_conserve_water() {
        for outcome in run_portfolio().expect("portfolio valid") {
            assert!(outcome.maximum_conservation_residual_m3 <= CONSERVATION_TOLERANCE_M3);
            assert!(outcome.ticks.iter().all(|tick| {
                tick.maximum_conservation_residual_m3 <= CONSERVATION_TOLERANCE_M3
            }));
        }
    }

    #[test]
    fn portfolio_is_complete_and_deterministic() {
        let first = run_portfolio().expect("portfolio valid");
        let second = run_portfolio().expect("portfolio valid");
        assert_eq!(first, second);
        assert_eq!(first.len(), RecoveryPath::ALL.len() * AllocationPriority::ALL.len());
        assert!(first.iter().all(|run| run.ticks.len() == TOTAL_TICKS));
    }
}
