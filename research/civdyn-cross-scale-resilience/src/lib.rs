// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Deterministic cross-scale service-resilience trajectories built on the
//! CivDyn resource ledger. The ledger remains the sole owner of physical
//! stocks, transfers, consumption and unit-specific conservation checks.
//!
//! This v0.1 benchmark is a synthetic, hand-checkable water/energy scenario.
//! It reports separate service and resource metrics; it does not emit a
//! universal resilience score or predict any real region.

use civdyn_resource_ledger::{
    Production, Region, ResourceVector, StepInput, TransferPolicy, TransferRequest, World,
    WorldError,
};
use core::fmt;

pub const REGION_COUNT: usize = 7;
pub const BASIN: usize = 0;
pub const DISTRICT_NORTH: usize = 1;
pub const DISTRICT_SOUTH: usize = 2;
pub const FIRST_HOUSEHOLD: usize = 3;
pub const HOUSEHOLD_COUNT: usize = 4;
pub const TICKS: usize = 10;
pub const SHOCK_START_TICK: usize = 2;
pub const SHOCK_END_TICK: usize = 6;
pub const DEFAULT_RECOVERY_THRESHOLD: f64 = 0.95;
pub const DEFAULT_SUSTAINED_RECOVERY_STEPS: usize = 2;

const HOUSEHOLDS: [usize; HOUSEHOLD_COUNT] = [3, 4, 5, 6];
const DISTRICTS: [usize; 2] = [DISTRICT_NORTH, DISTRICT_SOUTH];
const HOUSEHOLD_DISTRICT: [usize; HOUSEHOLD_COUNT] = [
    DISTRICT_NORTH,
    DISTRICT_NORTH,
    DISTRICT_SOUTH,
    DISTRICT_SOUTH,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShockKind {
    WaterSupply,
    EnergySupply,
    CompoundWaterAndEnergy,
}

impl ShockKind {
    pub const ALL: [Self; 3] = [
        Self::WaterSupply,
        Self::EnergySupply,
        Self::CompoundWaterAndEnergy,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::WaterSupply => "water_supply",
            Self::EnergySupply => "energy_supply",
            Self::CompoundWaterAndEnergy => "compound_water_energy",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterventionPolicy {
    /// No inter-region transfers. External production remains at the basin.
    NoTransfer,
    /// Allocate current deficits proportionally at district and household levels.
    ProportionalSolidarity,
    /// Serve households in descending accumulated normalized service deficit.
    DeficitFirst,
    /// Proportional transfer while preserving half of each sender's opening stock.
    ReserveProtected,
}

impl InterventionPolicy {
    pub const ALL: [Self; 4] = [
        Self::NoTransfer,
        Self::ProportionalSolidarity,
        Self::DeficitFirst,
        Self::ReserveProtected,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::NoTransfer => "no_transfer",
            Self::ProportionalSolidarity => "proportional_solidarity",
            Self::DeficitFirst => "deficit_first",
            Self::ReserveProtected => "reserve_protected",
        }
    }

    fn ledger_transfer_policy(self) -> TransferPolicy {
        match self {
            Self::ReserveProtected => TransferPolicy::PreserveOpeningReserve { fraction: 0.5 },
            _ => TransferPolicy::Unrestricted,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShockPhase {
    PreShock,
    Shock,
    Recovery,
}

impl ShockPhase {
    pub const fn name(self) -> &'static str {
        match self {
            Self::PreShock => "pre_shock",
            Self::Shock => "shock",
            Self::Recovery => "recovery",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scenario {
    pub id: String,
    /// Exactly seven physical inventory nodes: basin, two districts, four households.
    /// Parent nodes have zero population; no physical stock is duplicated in an
    /// aggregate parent statistic.
    pub regions: Vec<Region>,
    /// One vector per tick, with exactly one production entry per physical node.
    pub production_by_tick: Vec<Vec<Production>>,
    pub shock_kind: ShockKind,
    pub shock_start_tick: usize,
    /// Exclusive end index: ticks in [start, end) are shocked.
    pub shock_end_tick: usize,
    pub recovery_threshold: f64,
    pub sustained_recovery_steps: usize,
}

impl Scenario {
    pub fn validate(&self) -> Result<(), ResilienceError> {
        if self.id.trim().is_empty() {
            return Err(ResilienceError::EmptyScenarioId);
        }
        if self.regions.len() != REGION_COUNT {
            return Err(ResilienceError::WrongRegionCount {
                expected: REGION_COUNT,
                actual: self.regions.len(),
            });
        }
        if self.production_by_tick.is_empty() {
            return Err(ResilienceError::EmptyTrajectory);
        }
        if self.shock_start_tick >= self.shock_end_tick
            || self.shock_end_tick >= self.production_by_tick.len()
        {
            return Err(ResilienceError::InvalidShockWindow {
                start: self.shock_start_tick,
                end: self.shock_end_tick,
                ticks: self.production_by_tick.len(),
            });
        }
        if !self.recovery_threshold.is_finite() || !(0.0..=1.0).contains(&self.recovery_threshold) {
            return Err(ResilienceError::InvalidRecoveryThreshold(
                self.recovery_threshold,
            ));
        }
        if self.sustained_recovery_steps == 0 {
            return Err(ResilienceError::ZeroSustainedRecoverySteps);
        }
        for (tick, production) in self.production_by_tick.iter().enumerate() {
            if production.len() != REGION_COUNT {
                return Err(ResilienceError::ProductionCountMismatch {
                    tick,
                    expected: REGION_COUNT,
                    actual: production.len(),
                });
            }
        }
        // World::new performs the resource-ledger's finite/non-negative checks.
        World::new(self.regions.clone())?;
        if self.regions[BASIN].population != 0.0
            || self.regions[DISTRICT_NORTH].population != 0.0
            || self.regions[DISTRICT_SOUTH].population != 0.0
        {
            return Err(ResilienceError::ParentNodeHasPopulation);
        }
        for index in HOUSEHOLDS {
            if self.regions[index].population <= 0.0 {
                return Err(ResilienceError::HouseholdHasNoPopulation {
                    region: self.regions[index].id.clone(),
                });
            }
            let demand = scaled_demand(&self.regions[index]);
            if demand.water_m3 <= 0.0 || demand.energy_kwh <= 0.0 {
                return Err(ResilienceError::HouseholdMissingWaterOrEnergyDemand {
                    region: self.regions[index].id.clone(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TickOutcome {
    pub tick: u64,
    pub phase: ShockPhase,
    pub household_water_coverage: [f64; HOUSEHOLD_COUNT],
    pub household_energy_coverage: [f64; HOUSEHOLD_COUNT],
    pub mean_water_coverage: f64,
    pub minimum_water_coverage: f64,
    pub mean_energy_coverage: f64,
    pub minimum_energy_coverage: f64,
    pub unmet_water_m3: f64,
    pub unmet_energy_kwh: f64,
    /// Gross edge throughput; the same stock may be counted once per network edge.
    pub gross_edge_transfer_water_m3: f64,
    pub gross_edge_transfer_energy_kwh: f64,
    pub transfer_receipt_count: usize,
    pub basin_closing_water_m3: f64,
    pub basin_closing_energy_kwh: f64,
    pub maximum_water_conservation_residual_m3: f64,
    pub maximum_energy_conservation_residual_kwh: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunOutcome {
    pub scenario_id: String,
    pub shock_kind: ShockKind,
    pub policy: InterventionPolicy,
    pub ticks: Vec<TickOutcome>,
    pub total_unmet_water_m3: f64,
    pub total_unmet_energy_kwh: f64,
    /// Gross edge throughput, not unique physical volume: transfers along two edges
    /// are counted twice because both edge movements are operationally relevant.
    pub gross_edge_transfer_water_m3: f64,
    pub gross_edge_transfer_energy_kwh: f64,
    /// Sum of (1 - household water coverage) across household-ticks.
    /// Dimensionless household-step coverage shortfall; not added to energy.
    pub water_deficit_area_household_steps: f64,
    /// Sum of (1 - household energy coverage) across household-ticks.
    pub energy_deficit_area_household_steps: f64,
    pub minimum_water_coverage: f64,
    pub minimum_energy_coverage: f64,
    /// Number of post-shock ticks until all households meet the service floor
    /// for the declared number of consecutive ticks.
    pub recovery_after_ticks: Option<usize>,
    pub final_basin_stock: ResourceVector,
    pub maximum_water_conservation_residual_m3: f64,
    pub maximum_energy_conservation_residual_kwh: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResilienceError {
    Ledger(WorldError),
    EmptyScenarioId,
    WrongRegionCount {
        expected: usize,
        actual: usize,
    },
    EmptyTrajectory,
    InvalidShockWindow {
        start: usize,
        end: usize,
        ticks: usize,
    },
    InvalidRecoveryThreshold(f64),
    ZeroSustainedRecoverySteps,
    ProductionCountMismatch {
        tick: usize,
        expected: usize,
        actual: usize,
    },
    ParentNodeHasPopulation,
    HouseholdHasNoPopulation {
        region: String,
    },
    HouseholdMissingWaterOrEnergyDemand {
        region: String,
    },
}

impl fmt::Display for ResilienceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ledger(error) => write!(f, "resource ledger error: {error}"),
            Self::EmptyScenarioId => write!(f, "scenario ID must not be empty"),
            Self::WrongRegionCount { expected, actual } => {
                write!(f, "expected {expected} physical nodes, got {actual}")
            }
            Self::EmptyTrajectory => write!(f, "scenario trajectory must not be empty"),
            Self::InvalidShockWindow { start, end, ticks } => write!(
                f,
                "invalid shock window [{start}, {end}) for trajectory with {ticks} ticks"
            ),
            Self::InvalidRecoveryThreshold(value) => {
                write!(
                    f,
                    "recovery threshold must be finite and in [0, 1], got {value}"
                )
            }
            Self::ZeroSustainedRecoverySteps => {
                write!(f, "sustained recovery steps must be greater than zero")
            }
            Self::ProductionCountMismatch {
                tick,
                expected,
                actual,
            } => write!(
                f,
                "tick {tick} expected {expected} production entries, got {actual}"
            ),
            Self::ParentNodeHasPopulation => {
                write!(
                    f,
                    "basin and district storage nodes must not duplicate population"
                )
            }
            Self::HouseholdHasNoPopulation { region } => {
                write!(f, "household node {region} must have positive population")
            }
            Self::HouseholdMissingWaterOrEnergyDemand { region } => {
                write!(
                    f,
                    "household node {region} needs positive water and energy demand"
                )
            }
        }
    }
}

impl std::error::Error for ResilienceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Ledger(error) => Some(error),
            _ => None,
        }
    }
}

impl From<WorldError> for ResilienceError {
    fn from(value: WorldError) -> Self {
        Self::Ledger(value)
    }
}

pub fn reference_scenarios() -> Vec<Scenario> {
    ShockKind::ALL.into_iter().map(reference_scenario).collect()
}

pub fn reference_scenario(kind: ShockKind) -> Scenario {
    let regions = vec![
        region("basin", 0.0, ResourceVector::new(80.0, 32.0, 0.0), zero()),
        region(
            "north_district",
            0.0,
            ResourceVector::new(20.0, 8.0, 0.0),
            zero(),
        ),
        region(
            "south_district",
            0.0,
            ResourceVector::new(20.0, 8.0, 0.0),
            zero(),
        ),
        region(
            "north_household_a",
            1.0,
            ResourceVector::new(5.0, 2.0, 0.0),
            demand(),
        ),
        region(
            "north_household_b",
            1.0,
            ResourceVector::new(5.0, 2.0, 0.0),
            demand(),
        ),
        region(
            "south_household_a",
            1.0,
            ResourceVector::new(5.0, 2.0, 0.0),
            demand(),
        ),
        region(
            "south_household_b",
            1.0,
            ResourceVector::new(5.0, 2.0, 0.0),
            demand(),
        ),
    ];

    let normal = ResourceVector::new(40.0, 16.0, 0.0);
    let shocked = match kind {
        ShockKind::WaterSupply => ResourceVector::new(0.0, 16.0, 0.0),
        ShockKind::EnergySupply => ResourceVector::new(40.0, 0.0, 0.0),
        ShockKind::CompoundWaterAndEnergy => ResourceVector::default(),
    };
    let production_by_tick = (0..TICKS)
        .map(|tick| {
            let root_supply = if (SHOCK_START_TICK..SHOCK_END_TICK).contains(&tick) {
                shocked
            } else {
                normal
            };
            let mut production = vec![Production::default(); REGION_COUNT];
            production[BASIN] = Production {
                resources: root_supply,
            };
            production
        })
        .collect();

    Scenario {
        id: kind.name().into(),
        regions,
        production_by_tick,
        shock_kind: kind,
        shock_start_tick: SHOCK_START_TICK,
        shock_end_tick: SHOCK_END_TICK,
        recovery_threshold: DEFAULT_RECOVERY_THRESHOLD,
        sustained_recovery_steps: DEFAULT_SUSTAINED_RECOVERY_STEPS,
    }
}

pub fn run_scenario(
    scenario: &Scenario,
    policy: InterventionPolicy,
) -> Result<RunOutcome, ResilienceError> {
    scenario.validate()?;
    let mut world = World::new(scenario.regions.clone())?;
    let mut cumulative_water_deficit = [0.0_f64; HOUSEHOLD_COUNT];
    let mut cumulative_energy_deficit = [0.0_f64; HOUSEHOLD_COUNT];
    let mut ticks = Vec::with_capacity(scenario.production_by_tick.len());
    let mut total_unmet_water_m3 = 0.0;
    let mut total_unmet_energy_kwh = 0.0;
    let mut gross_edge_transfer_water_m3 = 0.0;
    let mut gross_edge_transfer_energy_kwh = 0.0;
    let mut water_deficit_area_household_steps = 0.0;
    let mut energy_deficit_area_household_steps = 0.0;
    let mut minimum_water_coverage = 1.0_f64;
    let mut minimum_energy_coverage = 1.0_f64;
    let mut recovery_streak = 0usize;
    let mut recovery_after_ticks = None;
    let ledger_policy = policy.ledger_transfer_policy();

    for (tick_index, production) in scenario.production_by_tick.iter().enumerate() {
        let transfers = match policy {
            InterventionPolicy::NoTransfer => Vec::new(),
            InterventionPolicy::ProportionalSolidarity => {
                proportional_transfers(world.regions(), production, 0.0)
            }
            InterventionPolicy::ReserveProtected => {
                proportional_transfers(world.regions(), production, 0.5)
            }
            InterventionPolicy::DeficitFirst => deficit_first_transfers(
                world.regions(),
                production,
                &cumulative_water_deficit,
                &cumulative_energy_deficit,
            ),
        };

        let report = world.step(
            &StepInput {
                production: production.clone(),
                transfers,
            },
            ledger_policy,
        )?;

        let mut household_water_coverage = [0.0; HOUSEHOLD_COUNT];
        let mut household_energy_coverage = [0.0; HOUSEHOLD_COUNT];
        let mut unmet_water_m3 = 0.0;
        let mut unmet_energy_kwh = 0.0;
        let mut max_water_residual = 0.0_f64;
        let mut max_energy_residual = 0.0_f64;

        for (household_index, region_index) in HOUSEHOLDS.iter().enumerate() {
            let outcome = &report.regions[*region_index];
            let water_coverage = outcome.coverage.water.unwrap_or(0.0);
            let energy_coverage = outcome.coverage.energy.unwrap_or(0.0);
            household_water_coverage[household_index] = water_coverage;
            household_energy_coverage[household_index] = energy_coverage;
            unmet_water_m3 += outcome.unmet.water_m3;
            unmet_energy_kwh += outcome.unmet.energy_kwh;
            cumulative_water_deficit[household_index] += outcome.unmet.water_m3;
            cumulative_energy_deficit[household_index] += outcome.unmet.energy_kwh;
            water_deficit_area_household_steps += (1.0 - water_coverage).max(0.0);
            energy_deficit_area_household_steps += (1.0 - energy_coverage).max(0.0);
            minimum_water_coverage = minimum_water_coverage.min(water_coverage);
            minimum_energy_coverage = minimum_energy_coverage.min(energy_coverage);
        }

        for region in &report.regions {
            let residual = region.ledger.conservation_residual();
            max_water_residual = max_water_residual.max(residual.water_m3.abs());
            max_energy_residual = max_energy_residual.max(residual.energy_kwh.abs());
            if !region.ledger.is_conserved() {
                return Err(ResilienceError::Ledger(WorldError::ConservationViolation {
                    region: region.region_id.clone(),
                    residual,
                }));
            }
        }

        total_unmet_water_m3 += unmet_water_m3;
        total_unmet_energy_kwh += unmet_energy_kwh;
        let tick_gross_water = report
            .transfers
            .iter()
            .map(|receipt| receipt.delivered.water_m3)
            .sum::<f64>();
        let tick_gross_energy = report
            .transfers
            .iter()
            .map(|receipt| receipt.delivered.energy_kwh)
            .sum::<f64>();
        gross_edge_transfer_water_m3 += tick_gross_water;
        gross_edge_transfer_energy_kwh += tick_gross_energy;
        let transfer_receipt_count = report.transfers.len();
        let minimum_tick_water = household_water_coverage
            .iter()
            .copied()
            .fold(1.0_f64, f64::min);
        let minimum_tick_energy = household_energy_coverage
            .iter()
            .copied()
            .fold(1.0_f64, f64::min);
        let mean_water = household_water_coverage.iter().sum::<f64>() / HOUSEHOLD_COUNT as f64;
        let mean_energy = household_energy_coverage.iter().sum::<f64>() / HOUSEHOLD_COUNT as f64;

        let phase = if tick_index < scenario.shock_start_tick {
            ShockPhase::PreShock
        } else if tick_index < scenario.shock_end_tick {
            ShockPhase::Shock
        } else {
            ShockPhase::Recovery
        };

        if tick_index >= scenario.shock_end_tick {
            let recovered = minimum_tick_water >= scenario.recovery_threshold
                && minimum_tick_energy >= scenario.recovery_threshold;
            if recovered {
                recovery_streak += 1;
            } else {
                recovery_streak = 0;
            }
            if recovery_after_ticks.is_none()
                && recovery_streak >= scenario.sustained_recovery_steps
            {
                recovery_after_ticks = Some(tick_index + 1 - scenario.shock_end_tick);
            }
        }

        ticks.push(TickOutcome {
            tick: report.tick,
            phase,
            household_water_coverage,
            household_energy_coverage,
            mean_water_coverage: mean_water,
            minimum_water_coverage: minimum_tick_water,
            mean_energy_coverage: mean_energy,
            minimum_energy_coverage: minimum_tick_energy,
            unmet_water_m3,
            unmet_energy_kwh,
            gross_edge_transfer_water_m3: tick_gross_water,
            gross_edge_transfer_energy_kwh: tick_gross_energy,
            transfer_receipt_count,
            basin_closing_water_m3: report.regions[BASIN].closing_stock.water_m3,
            basin_closing_energy_kwh: report.regions[BASIN].closing_stock.energy_kwh,
            maximum_water_conservation_residual_m3: max_water_residual,
            maximum_energy_conservation_residual_kwh: max_energy_residual,
        });
    }

    let final_basin_stock = world.regions()[BASIN].opening_stock;
    let maximum_water_conservation_residual_m3 = ticks
        .iter()
        .map(|tick| tick.maximum_water_conservation_residual_m3)
        .fold(0.0_f64, f64::max);
    let maximum_energy_conservation_residual_kwh = ticks
        .iter()
        .map(|tick| tick.maximum_energy_conservation_residual_kwh)
        .fold(0.0_f64, f64::max);

    Ok(RunOutcome {
        scenario_id: scenario.id.clone(),
        shock_kind: scenario.shock_kind,
        policy,
        ticks,
        total_unmet_water_m3,
        total_unmet_energy_kwh,
        gross_edge_transfer_water_m3,
        gross_edge_transfer_energy_kwh,
        water_deficit_area_household_steps,
        energy_deficit_area_household_steps,
        minimum_water_coverage,
        minimum_energy_coverage,
        recovery_after_ticks,
        final_basin_stock,
        maximum_water_conservation_residual_m3,
        maximum_energy_conservation_residual_kwh,
    })
}

pub fn run_reference_portfolio() -> Result<Vec<RunOutcome>, ResilienceError> {
    let mut outcomes = Vec::new();
    for scenario in reference_scenarios() {
        for policy in InterventionPolicy::ALL {
            outcomes.push(run_scenario(&scenario, policy)?);
        }
    }
    Ok(outcomes)
}

fn region(
    id: &str,
    population: f64,
    opening_stock: ResourceVector,
    essential_demand_per_capita: ResourceVector,
) -> Region {
    Region {
        id: id.into(),
        population,
        opening_stock,
        essential_demand_per_capita,
    }
}

fn zero() -> ResourceVector {
    ResourceVector::default()
}

fn demand() -> ResourceVector {
    ResourceVector::new(10.0, 4.0, 0.0)
}

fn scaled_demand(region: &Region) -> ResourceVector {
    ResourceVector::new(
        region.essential_demand_per_capita.water_m3 * region.population,
        region.essential_demand_per_capita.energy_kwh * region.population,
        region.essential_demand_per_capita.food_kg * region.population,
    )
}

fn resource_vector(water_m3: f64, energy_kwh: f64) -> ResourceVector {
    ResourceVector::new(water_m3.max(0.0), energy_kwh.max(0.0), 0.0)
}

fn household_deficit(region: &Region) -> ResourceVector {
    let demand = scaled_demand(region);
    resource_vector(
        demand.water_m3 - region.opening_stock.water_m3,
        demand.energy_kwh - region.opening_stock.energy_kwh,
    )
}

fn proportional_transfers(
    regions: &[Region],
    production: &[Production],
    reserve_fraction: f64,
) -> Vec<TransferRequest> {
    let root_opening = regions[BASIN].opening_stock;
    let root_production = production[BASIN].resources;
    let root_available = resource_vector(
        root_opening.water_m3 + root_production.water_m3 - reserve_fraction * root_opening.water_m3,
        root_opening.energy_kwh + root_production.energy_kwh
            - reserve_fraction * root_opening.energy_kwh,
    );

    let deficits: [ResourceVector; HOUSEHOLD_COUNT] =
        core::array::from_fn(|index| household_deficit(&regions[HOUSEHOLDS[index]]));
    let mut district_available = [ResourceVector::default(); 2];
    let mut district_deficit = [ResourceVector::default(); 2];
    let mut total_district_deficit = ResourceVector::default();

    for (district_offset, district_index) in DISTRICTS.iter().enumerate() {
        let opening = regions[*district_index].opening_stock;
        district_available[district_offset] = resource_vector(
            opening.water_m3 + production[*district_index].resources.water_m3
                - reserve_fraction * opening.water_m3,
            opening.energy_kwh + production[*district_index].resources.energy_kwh
                - reserve_fraction * opening.energy_kwh,
        );

        let child_deficit = deficits
            .iter()
            .enumerate()
            .filter(|(household_index, _)| HOUSEHOLD_DISTRICT[*household_index] == *district_index)
            .fold(ResourceVector::default(), |sum, (_, value)| {
                ResourceVector::new(
                    sum.water_m3 + value.water_m3,
                    sum.energy_kwh + value.energy_kwh,
                    0.0,
                )
            });
        district_deficit[district_offset] = resource_vector(
            child_deficit.water_m3 - district_available[district_offset].water_m3,
            child_deficit.energy_kwh - district_available[district_offset].energy_kwh,
        );
        total_district_deficit.water_m3 += district_deficit[district_offset].water_m3;
        total_district_deficit.energy_kwh += district_deficit[district_offset].energy_kwh;
    }

    let water_import_fraction = if total_district_deficit.water_m3 <= 0.0 {
        0.0
    } else {
        (root_available.water_m3 / total_district_deficit.water_m3).clamp(0.0, 1.0)
    };
    let energy_import_fraction = if total_district_deficit.energy_kwh <= 0.0 {
        0.0
    } else {
        (root_available.energy_kwh / total_district_deficit.energy_kwh).clamp(0.0, 1.0)
    };

    let mut requests = Vec::new();
    for (district_offset, district_index) in DISTRICTS.iter().enumerate() {
        let deficit = district_deficit[district_offset];
        let import = resource_vector(
            deficit.water_m3 * water_import_fraction,
            deficit.energy_kwh * energy_import_fraction,
        );
        if import.water_m3 > 0.0 || import.energy_kwh > 0.0 {
            requests.push(TransferRequest {
                from_region: BASIN,
                to_region: *district_index,
                amount: import,
            });
        }
        district_available[district_offset].water_m3 += import.water_m3;
        district_available[district_offset].energy_kwh += import.energy_kwh;
    }

    for (district_offset, district_index) in DISTRICTS.iter().enumerate() {
        let child_total = deficits
            .iter()
            .enumerate()
            .filter(|(household_index, _)| HOUSEHOLD_DISTRICT[*household_index] == *district_index)
            .fold(ResourceVector::default(), |sum, (_, value)| {
                ResourceVector::new(
                    sum.water_m3 + value.water_m3,
                    sum.energy_kwh + value.energy_kwh,
                    0.0,
                )
            });
        let water_fraction = if child_total.water_m3 <= 0.0 {
            0.0
        } else {
            (district_available[district_offset].water_m3 / child_total.water_m3).clamp(0.0, 1.0)
        };
        let energy_fraction = if child_total.energy_kwh <= 0.0 {
            0.0
        } else {
            (district_available[district_offset].energy_kwh / child_total.energy_kwh)
                .clamp(0.0, 1.0)
        };
        for (household_index, household_region) in HOUSEHOLDS.iter().enumerate() {
            if HOUSEHOLD_DISTRICT[household_index] != *district_index {
                continue;
            }
            let deficit = deficits[household_index];
            let delivery = resource_vector(
                deficit.water_m3 * water_fraction,
                deficit.energy_kwh * energy_fraction,
            );
            if delivery.water_m3 > 0.0 || delivery.energy_kwh > 0.0 {
                requests.push(TransferRequest {
                    from_region: *district_index,
                    to_region: *household_region,
                    amount: delivery,
                });
            }
        }
    }
    requests
}

fn deficit_first_transfers(
    regions: &[Region],
    production: &[Production],
    cumulative_water_deficit: &[f64; HOUSEHOLD_COUNT],
    cumulative_energy_deficit: &[f64; HOUSEHOLD_COUNT],
) -> Vec<TransferRequest> {
    let mut root_remaining = resource_vector(
        regions[BASIN].opening_stock.water_m3 + production[BASIN].resources.water_m3,
        regions[BASIN].opening_stock.energy_kwh + production[BASIN].resources.energy_kwh,
    );
    let mut district_remaining: [ResourceVector; 2] = core::array::from_fn(|offset| {
        let stock = regions[DISTRICTS[offset]].opening_stock;
        let supply = production[DISTRICTS[offset]].resources;
        resource_vector(
            stock.water_m3 + supply.water_m3,
            stock.energy_kwh + supply.energy_kwh,
        )
    });

    let mut order = (0..HOUSEHOLD_COUNT).collect::<Vec<_>>();
    order.sort_by(|left, right| {
        let left_demand = scaled_demand(&regions[HOUSEHOLDS[*left]]);
        let right_demand = scaled_demand(&regions[HOUSEHOLDS[*right]]);
        let left_score = cumulative_water_deficit[*left] / left_demand.water_m3
            + cumulative_energy_deficit[*left] / left_demand.energy_kwh;
        let right_score = cumulative_water_deficit[*right] / right_demand.water_m3
            + cumulative_energy_deficit[*right] / right_demand.energy_kwh;
        right_score.total_cmp(&left_score).then_with(|| {
            regions[HOUSEHOLDS[*left]]
                .id
                .cmp(&regions[HOUSEHOLDS[*right]].id)
        })
    });

    let mut requests = Vec::new();
    for household_index in order {
        let household_region = HOUSEHOLDS[household_index];
        let district_index = HOUSEHOLD_DISTRICT[household_index];
        let district_offset = if district_index == DISTRICT_NORTH {
            0
        } else {
            1
        };
        let deficit = household_deficit(&regions[household_region]);
        let available = district_remaining[district_offset];
        let import = resource_vector(
            (deficit.water_m3 - available.water_m3)
                .max(0.0)
                .min(root_remaining.water_m3),
            (deficit.energy_kwh - available.energy_kwh)
                .max(0.0)
                .min(root_remaining.energy_kwh),
        );

        if import.water_m3 > 0.0 || import.energy_kwh > 0.0 {
            requests.push(TransferRequest {
                from_region: BASIN,
                to_region: district_index,
                amount: import,
            });
        }
        root_remaining.water_m3 -= import.water_m3;
        root_remaining.energy_kwh -= import.energy_kwh;
        district_remaining[district_offset].water_m3 += import.water_m3;
        district_remaining[district_offset].energy_kwh += import.energy_kwh;

        let delivery = resource_vector(
            deficit
                .water_m3
                .min(district_remaining[district_offset].water_m3),
            deficit
                .energy_kwh
                .min(district_remaining[district_offset].energy_kwh),
        );
        if delivery.water_m3 > 0.0 || delivery.energy_kwh > 0.0 {
            requests.push(TransferRequest {
                from_region: district_index,
                to_region: household_region,
                amount: delivery,
            });
        }
        district_remaining[district_offset].water_m3 -= delivery.water_m3;
        district_remaining[district_offset].energy_kwh -= delivery.energy_kwh;
    }
    requests
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_task_has_three_physical_scales_without_double_counted_population() {
        let scenario = reference_scenario(ShockKind::CompoundWaterAndEnergy);
        scenario.validate().unwrap();

        assert_eq!(scenario.regions.len(), 7);
        assert_eq!(scenario.regions[BASIN].population, 0.0);
        assert_eq!(scenario.regions[DISTRICT_NORTH].population, 0.0);
        assert_eq!(scenario.regions[DISTRICT_SOUTH].population, 0.0);
        assert!(
            HOUSEHOLDS
                .iter()
                .all(|index| scenario.regions[*index].population > 0.0)
        );
    }

    #[test]
    fn compound_shock_fixture_has_pre_shock_shock_and_recovery_periods() {
        let scenario = reference_scenario(ShockKind::CompoundWaterAndEnergy);
        assert_eq!(scenario.production_by_tick.len(), TICKS);
        assert_eq!(scenario.shock_start_tick, SHOCK_START_TICK);
        assert_eq!(scenario.shock_end_tick, SHOCK_END_TICK);
        assert_eq!(
            scenario.production_by_tick[SHOCK_START_TICK][BASIN].resources,
            ResourceVector::default()
        );
        assert_eq!(
            scenario.production_by_tick[SHOCK_END_TICK][BASIN].resources,
            ResourceVector::new(40.0, 16.0, 0.0)
        );
    }

    #[test]
    fn proportional_solidarity_meets_all_household_floors_without_a_supply_shock() {
        let mut scenario = reference_scenario(ShockKind::CompoundWaterAndEnergy);
        for tick in &mut scenario.production_by_tick {
            tick[BASIN] = Production {
                resources: ResourceVector::new(40.0, 16.0, 0.0),
            };
        }
        let outcome = run_scenario(&scenario, InterventionPolicy::ProportionalSolidarity).unwrap();

        assert_eq!(outcome.total_unmet_water_m3, 0.0);
        assert_eq!(outcome.total_unmet_energy_kwh, 0.0);
        assert_eq!(outcome.minimum_water_coverage, 1.0);
        assert_eq!(outcome.minimum_energy_coverage, 1.0);
        assert_eq!(outcome.maximum_water_conservation_residual_m3, 0.0);
        assert_eq!(outcome.maximum_energy_conservation_residual_kwh, 0.0);
    }

    #[test]
    fn no_transfer_is_a_negative_control_even_when_basin_production_is_available() {
        let scenario = reference_scenario(ShockKind::CompoundWaterAndEnergy);
        let outcome = run_scenario(&scenario, InterventionPolicy::NoTransfer).unwrap();

        assert!(outcome.total_unmet_water_m3 > 0.0);
        assert!(outcome.total_unmet_energy_kwh > 0.0);
        assert!(outcome.minimum_water_coverage < DEFAULT_RECOVERY_THRESHOLD);
        assert!(outcome.minimum_energy_coverage < DEFAULT_RECOVERY_THRESHOLD);
        assert_eq!(outcome.recovery_after_ticks, None);
    }

    #[test]
    fn compound_shock_keeps_water_and_energy_losses_separate() {
        let water = run_scenario(
            &reference_scenario(ShockKind::WaterSupply),
            InterventionPolicy::ProportionalSolidarity,
        )
        .unwrap();
        let energy = run_scenario(
            &reference_scenario(ShockKind::EnergySupply),
            InterventionPolicy::ProportionalSolidarity,
        )
        .unwrap();
        let compound = run_scenario(
            &reference_scenario(ShockKind::CompoundWaterAndEnergy),
            InterventionPolicy::ProportionalSolidarity,
        )
        .unwrap();

        assert!((compound.total_unmet_water_m3 - water.total_unmet_water_m3).abs() < 1e-9);
        assert!((compound.total_unmet_energy_kwh - energy.total_unmet_energy_kwh).abs() < 1e-9);
        assert!(water.total_unmet_energy_kwh < 1e-9);
        assert!(energy.total_unmet_water_m3 < 1e-9);
    }

    #[test]
    fn recovery_requires_the_declared_number_of_consecutive_post_shock_steps() {
        let scenario = reference_scenario(ShockKind::CompoundWaterAndEnergy);
        let outcome = run_scenario(&scenario, InterventionPolicy::ProportionalSolidarity).unwrap();

        assert_eq!(outcome.recovery_after_ticks, Some(2));
        assert!(outcome.ticks[SHOCK_END_TICK].minimum_water_coverage >= DEFAULT_RECOVERY_THRESHOLD);
        assert!(
            outcome.ticks[SHOCK_END_TICK + 1].minimum_energy_coverage >= DEFAULT_RECOVERY_THRESHOLD
        );
    }

    #[test]
    fn reserve_protection_and_deficit_first_are_separate_interventions() {
        let scenario = reference_scenario(ShockKind::CompoundWaterAndEnergy);
        let reserve = run_scenario(&scenario, InterventionPolicy::ReserveProtected).unwrap();
        let adaptive = run_scenario(&scenario, InterventionPolicy::DeficitFirst).unwrap();
        assert_eq!(reserve.policy, InterventionPolicy::ReserveProtected);
        assert_eq!(adaptive.policy, InterventionPolicy::DeficitFirst);
        assert!(
            reserve
                .ticks
                .iter()
                .all(|tick| tick.maximum_water_conservation_residual_m3 < 1e-9)
        );
        assert!(
            adaptive
                .ticks
                .iter()
                .all(|tick| tick.maximum_energy_conservation_residual_kwh < 1e-9)
        );
    }

    #[test]
    fn deterministic_replay_is_byte_equivalent_at_the_result_level() {
        let scenario = reference_scenario(ShockKind::CompoundWaterAndEnergy);
        let first = run_scenario(&scenario, InterventionPolicy::DeficitFirst).unwrap();
        let second = run_scenario(&scenario, InterventionPolicy::DeficitFirst).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn portfolio_covers_every_shock_and_policy_combination() {
        let outcomes = run_reference_portfolio().unwrap();
        assert_eq!(
            outcomes.len(),
            ShockKind::ALL.len() * InterventionPolicy::ALL.len()
        );
        for outcome in outcomes {
            assert_eq!(outcome.ticks.len(), TICKS);
        }
    }

    #[test]
    fn invalid_scenario_windows_fail_closed() {
        let mut scenario = reference_scenario(ShockKind::WaterSupply);
        scenario.shock_end_tick = scenario.production_by_tick.len();
        assert!(matches!(
            scenario.validate(),
            Err(ResilienceError::InvalidShockWindow { .. })
        ));
    }
}


/// Repeated-shock benchmark separating service recovery from resource reserve recovery.
pub mod recovery_capital;
