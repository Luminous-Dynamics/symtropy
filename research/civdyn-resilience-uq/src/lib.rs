// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Parameter-design and global sensitivity reference for the CivDyn
//! cross-scale resilience benchmark.
//!
//! The design measures consequences under an explicitly declared parameter
//! design. It does not treat Latin-hypercube points as stochastic simulation
//! replications, empirical priors or calibrated posterior samples.

use civdyn_cross_scale_resilience::{
    BASIN, DEFAULT_RECOVERY_THRESHOLD, FIRST_HOUSEHOLD, HOUSEHOLD_COUNT, InterventionPolicy,
    REGION_COUNT, ResilienceError, RunOutcome, SHOCK_START_TICK, Scenario, ShockKind,
    reference_scenario, run_scenario,
};
use civdyn_resource_ledger::{Production, ResourceVector};
use core::fmt;

pub const FACTOR_COUNT: usize = 7;
pub const LHS_DEFAULT_POINTS: usize = 30;
pub const MORRIS_DEFAULT_TRAJECTORIES: usize = 30;
pub const MORRIS_LEVELS: usize = 4;
pub const MORRIS_DELTA: f64 = 2.0 / 3.0;
pub const TAIL_FRACTION: f64 = 0.10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterFactor {
    WaterSupplyMultiplier,
    EnergySupplyMultiplier,
    DemandMultiplier,
    DistributedReserveMultiplier,
    WaterShockRetention,
    EnergyShockRetention,
    ShockDurationTicks,
}

impl ParameterFactor {
    pub const ALL: [Self; FACTOR_COUNT] = [
        Self::WaterSupplyMultiplier,
        Self::EnergySupplyMultiplier,
        Self::DemandMultiplier,
        Self::DistributedReserveMultiplier,
        Self::WaterShockRetention,
        Self::EnergyShockRetention,
        Self::ShockDurationTicks,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::WaterSupplyMultiplier => "water_supply_multiplier",
            Self::EnergySupplyMultiplier => "energy_supply_multiplier",
            Self::DemandMultiplier => "demand_multiplier",
            Self::DistributedReserveMultiplier => "distributed_reserve_multiplier",
            Self::WaterShockRetention => "water_shock_retention",
            Self::EnergyShockRetention => "energy_shock_retention",
            Self::ShockDurationTicks => "shock_duration_ticks",
        }
    }

    pub const fn index(self) -> usize {
        match self {
            Self::WaterSupplyMultiplier => 0,
            Self::EnergySupplyMultiplier => 1,
            Self::DemandMultiplier => 2,
            Self::DistributedReserveMultiplier => 3,
            Self::WaterShockRetention => 4,
            Self::EnergyShockRetention => 5,
            Self::ShockDurationTicks => 6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParameterSet {
    pub water_supply_multiplier: f64,
    pub energy_supply_multiplier: f64,
    pub demand_multiplier: f64,
    pub distributed_reserve_multiplier: f64,
    /// Fraction of normal basin water production retained during the water shock.
    pub water_shock_retention: f64,
    /// Fraction of normal basin energy production retained during the energy shock.
    pub energy_shock_retention: f64,
    pub shock_duration_ticks: usize,
}

impl ParameterSet {
    pub fn from_unit_coordinates(unit: [f64; FACTOR_COUNT]) -> Result<Self, UqError> {
        for (index, value) in unit.iter().enumerate() {
            if !value.is_finite() || !(0.0..=1.0).contains(value) {
                return Err(UqError::InvalidUnitCoordinate {
                    factor: ParameterFactor::ALL[index],
                    value: *value,
                });
            }
        }
        let parameters = Self {
            water_supply_multiplier: lerp(0.75, 1.25, unit[0]),
            energy_supply_multiplier: lerp(0.75, 1.25, unit[1]),
            demand_multiplier: lerp(0.85, 1.35, unit[2]),
            distributed_reserve_multiplier: lerp(0.50, 1.50, unit[3]),
            water_shock_retention: lerp(0.0, 0.50, unit[4]),
            energy_shock_retention: lerp(0.0, 0.50, unit[5]),
            // Discrete duration is deliberately mapped to {2, 3, 4} ticks.
            shock_duration_ticks: 2 + (unit[6] * 2.0).round() as usize,
        };
        parameters.validate()?;
        Ok(parameters)
    }

    pub fn validate(self) -> Result<(), UqError> {
        for (factor, value) in [
            (
                ParameterFactor::WaterSupplyMultiplier,
                self.water_supply_multiplier,
            ),
            (
                ParameterFactor::EnergySupplyMultiplier,
                self.energy_supply_multiplier,
            ),
            (ParameterFactor::DemandMultiplier, self.demand_multiplier),
            (
                ParameterFactor::DistributedReserveMultiplier,
                self.distributed_reserve_multiplier,
            ),
            (
                ParameterFactor::WaterShockRetention,
                self.water_shock_retention,
            ),
            (
                ParameterFactor::EnergyShockRetention,
                self.energy_shock_retention,
            ),
        ] {
            if !value.is_finite() {
                return Err(UqError::NonFiniteParameter { factor, value });
            }
        }
        if self.water_supply_multiplier <= 0.0
            || self.energy_supply_multiplier <= 0.0
            || self.demand_multiplier <= 0.0
            || self.distributed_reserve_multiplier < 0.0
            || !(0.0..=1.0).contains(&self.water_shock_retention)
            || !(0.0..=1.0).contains(&self.energy_shock_retention)
            || !(2..=4).contains(&self.shock_duration_ticks)
        {
            return Err(UqError::ParameterOutOfRange);
        }
        Ok(())
    }

    pub fn values(self) -> [f64; FACTOR_COUNT] {
        [
            self.water_supply_multiplier,
            self.energy_supply_multiplier,
            self.demand_multiplier,
            self.distributed_reserve_multiplier,
            self.water_shock_retention,
            self.energy_shock_retention,
            self.shock_duration_ticks as f64,
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DesignPoint {
    pub design_id: String,
    pub unit_coordinates: [f64; FACTOR_COUNT],
    pub parameters: ParameterSet,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MorrisTrajectory {
    pub trajectory_index: usize,
    /// Eight points for seven factors: base point plus one change per factor.
    pub points: Vec<[f64; FACTOR_COUNT]>,
    /// Factor changed between points[i] and points[i + 1].
    pub changed_factors: Vec<ParameterFactor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeMetric {
    TotalUnmetWaterM3,
    TotalUnmetEnergyKwh,
    WaterDeficitAreaHouseholdSteps,
    EnergyDeficitAreaHouseholdSteps,
    MinimumWaterCoverage,
    MinimumEnergyCoverage,
    WaterFloorBreachFraction,
    EnergyFloorBreachFraction,
    RecoverySuccess,
}

impl OutcomeMetric {
    pub const ALL: [Self; 9] = [
        Self::TotalUnmetWaterM3,
        Self::TotalUnmetEnergyKwh,
        Self::WaterDeficitAreaHouseholdSteps,
        Self::EnergyDeficitAreaHouseholdSteps,
        Self::MinimumWaterCoverage,
        Self::MinimumEnergyCoverage,
        Self::WaterFloorBreachFraction,
        Self::EnergyFloorBreachFraction,
        Self::RecoverySuccess,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::TotalUnmetWaterM3 => "total_unmet_water_m3",
            Self::TotalUnmetEnergyKwh => "total_unmet_energy_kwh",
            Self::WaterDeficitAreaHouseholdSteps => "water_deficit_area_household_steps",
            Self::EnergyDeficitAreaHouseholdSteps => "energy_deficit_area_household_steps",
            Self::MinimumWaterCoverage => "minimum_water_coverage",
            Self::MinimumEnergyCoverage => "minimum_energy_coverage",
            Self::WaterFloorBreachFraction => "water_floor_breach_fraction",
            Self::EnergyFloorBreachFraction => "energy_floor_breach_fraction",
            Self::RecoverySuccess => "recovery_success",
        }
    }

    pub const fn index(self) -> usize {
        match self {
            Self::TotalUnmetWaterM3 => 0,
            Self::TotalUnmetEnergyKwh => 1,
            Self::WaterDeficitAreaHouseholdSteps => 2,
            Self::EnergyDeficitAreaHouseholdSteps => 3,
            Self::MinimumWaterCoverage => 4,
            Self::MinimumEnergyCoverage => 5,
            Self::WaterFloorBreachFraction => 6,
            Self::EnergyFloorBreachFraction => 7,
            Self::RecoverySuccess => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunMetrics {
    pub total_unmet_water_m3: f64,
    pub total_unmet_energy_kwh: f64,
    pub water_deficit_area_household_steps: f64,
    pub energy_deficit_area_household_steps: f64,
    pub minimum_water_coverage: f64,
    pub minimum_energy_coverage: f64,
    pub water_floor_breach_fraction: f64,
    pub energy_floor_breach_fraction: f64,
    pub recovery_success: f64,
}

impl RunMetrics {
    pub fn value(self, metric: OutcomeMetric) -> f64 {
        match metric {
            OutcomeMetric::TotalUnmetWaterM3 => self.total_unmet_water_m3,
            OutcomeMetric::TotalUnmetEnergyKwh => self.total_unmet_energy_kwh,
            OutcomeMetric::WaterDeficitAreaHouseholdSteps => {
                self.water_deficit_area_household_steps
            }
            OutcomeMetric::EnergyDeficitAreaHouseholdSteps => {
                self.energy_deficit_area_household_steps
            }
            OutcomeMetric::MinimumWaterCoverage => self.minimum_water_coverage,
            OutcomeMetric::MinimumEnergyCoverage => self.minimum_energy_coverage,
            OutcomeMetric::WaterFloorBreachFraction => self.water_floor_breach_fraction,
            OutcomeMetric::EnergyFloorBreachFraction => self.energy_floor_breach_fraction,
            OutcomeMetric::RecoverySuccess => self.recovery_success,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LhsRunRecord {
    pub design_point: DesignPoint,
    pub shock_kind: ShockKind,
    pub policy: InterventionPolicy,
    pub metrics: RunMetrics,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LhsSummary {
    pub shock_kind: ShockKind,
    pub policy: InterventionPolicy,
    pub metric: OutcomeMetric,
    pub sample_count: usize,
    pub mean: f64,
    pub p05: f64,
    pub p50: f64,
    pub p95: f64,
    pub lower_tail_mean: f64,
    pub upper_tail_mean: f64,
    /// Fraction of parameter-design runs violating the metric's declared floor/zero condition.
    pub run_breach_fraction: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MorrisPointRecord {
    pub trajectory_index: usize,
    pub point_index: usize,
    pub unit_coordinates: [f64; FACTOR_COUNT],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MorrisEffectSummary {
    pub factor: ParameterFactor,
    pub shock_kind: ShockKind,
    pub policy: InterventionPolicy,
    pub metric: OutcomeMetric,
    pub trajectory_count: usize,
    /// Mean signed elementary effect.
    pub mu: f64,
    /// Mean absolute elementary effect; primary Morris screening statistic.
    pub mu_star: f64,
    /// Sample standard deviation of elementary effects.
    pub sigma: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkReport {
    pub seed: u64,
    pub lhs_runs: Vec<LhsRunRecord>,
    pub lhs_summaries: Vec<LhsSummary>,
    pub morris_points: Vec<MorrisPointRecord>,
    pub morris_effects: Vec<MorrisEffectSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UqError {
    EmptyDesign,
    InvalidUnitCoordinate { factor: ParameterFactor, value: f64 },
    NonFiniteParameter { factor: ParameterFactor, value: f64 },
    ParameterOutOfRange,
    InvalidTrajectoryCount,
    Resilience(ResilienceError),
    NonFiniteOutcome { metric: OutcomeMetric, value: f64 },
}

impl fmt::Display for UqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDesign => write!(f, "design must contain at least one sample"),
            Self::InvalidUnitCoordinate { factor, value } => {
                write!(
                    f,
                    "{} unit coordinate is outside [0, 1]: {value}",
                    factor.name()
                )
            }
            Self::NonFiniteParameter { factor, value } => {
                write!(f, "{} parameter is non-finite: {value}", factor.name())
            }
            Self::ParameterOutOfRange => write!(f, "parameter set is outside its declared range"),
            Self::InvalidTrajectoryCount => {
                write!(f, "Morris trajectory count must be greater than zero")
            }
            Self::Resilience(error) => write!(f, "resilience benchmark error: {error}"),
            Self::NonFiniteOutcome { metric, value } => {
                write!(f, "{} outcome is non-finite: {value}", metric.name())
            }
        }
    }
}

impl std::error::Error for UqError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Resilience(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ResilienceError> for UqError {
    fn from(value: ResilienceError) -> Self {
        Self::Resilience(value)
    }
}

pub fn latin_hypercube_design(seed: u64, sample_count: usize) -> Result<Vec<DesignPoint>, UqError> {
    if sample_count == 0 {
        return Err(UqError::EmptyDesign);
    }
    let mut rng = SplitMix64::new(seed);
    let mut coordinates = vec![[0.0; FACTOR_COUNT]; sample_count];

    for factor_index in 0..FACTOR_COUNT {
        let mut strata = (0..sample_count).collect::<Vec<_>>();
        shuffle(&mut strata, &mut rng);
        for (sample_index, stratum) in strata.into_iter().enumerate() {
            coordinates[sample_index][factor_index] =
                (stratum as f64 + rng.next_unit()) / sample_count as f64;
        }
    }

    coordinates
        .into_iter()
        .enumerate()
        .map(|(index, unit_coordinates)| {
            Ok(DesignPoint {
                design_id: format!("lhs_{index:02}"),
                parameters: ParameterSet::from_unit_coordinates(unit_coordinates)?,
                unit_coordinates,
            })
        })
        .collect()
}

pub fn morris_design(seed: u64, trajectory_count: usize) -> Result<Vec<MorrisTrajectory>, UqError> {
    if trajectory_count == 0 {
        return Err(UqError::InvalidTrajectoryCount);
    }
    let mut rng = SplitMix64::new(seed);
    let mut trajectories = Vec::with_capacity(trajectory_count);
    for trajectory_index in 0..trajectory_count {
        let mut directions = [1.0; FACTOR_COUNT];
        let mut base = [0.0; FACTOR_COUNT];
        for factor in 0..FACTOR_COUNT {
            let positive = rng.next_u64() & 1 == 0;
            directions[factor] = if positive { 1.0 } else { -1.0 };
            base[factor] = if positive {
                if rng.next_u64() & 1 == 0 {
                    0.0
                } else {
                    1.0 / 3.0
                }
            } else if rng.next_u64() & 1 == 0 {
                2.0 / 3.0
            } else {
                1.0
            };
        }

        let mut order = (0..FACTOR_COUNT).collect::<Vec<_>>();
        shuffle(&mut order, &mut rng);
        let mut points = Vec::with_capacity(FACTOR_COUNT + 1);
        points.push(base);
        let mut current = base;
        for factor_index in &order {
            current[*factor_index] += directions[*factor_index] * MORRIS_DELTA;
            points.push(current);
        }

        trajectories.push(MorrisTrajectory {
            trajectory_index,
            points,
            changed_factors: order
                .into_iter()
                .map(|index| ParameterFactor::ALL[index])
                .collect(),
        });
    }
    Ok(trajectories)
}

pub fn parameterized_scenario(
    shock_kind: ShockKind,
    parameters: ParameterSet,
) -> Result<Scenario, UqError> {
    parameters.validate()?;
    let mut scenario = reference_scenario(shock_kind);
    scenario.shock_end_tick = SHOCK_START_TICK + parameters.shock_duration_ticks;

    for region_index in 1..REGION_COUNT {
        let stock = scenario.regions[region_index].opening_stock;
        scenario.regions[region_index].opening_stock = ResourceVector::new(
            stock.water_m3 * parameters.distributed_reserve_multiplier,
            stock.energy_kwh * parameters.distributed_reserve_multiplier,
            stock.food_kg,
        );
    }
    for region_index in FIRST_HOUSEHOLD..REGION_COUNT {
        let demand = scenario.regions[region_index].essential_demand_per_capita;
        scenario.regions[region_index].essential_demand_per_capita = ResourceVector::new(
            demand.water_m3 * parameters.demand_multiplier,
            demand.energy_kwh * parameters.demand_multiplier,
            demand.food_kg,
        );
    }

    for (tick_index, production_by_region) in scenario.production_by_tick.iter_mut().enumerate() {
        let in_shock = (scenario.shock_start_tick..scenario.shock_end_tick).contains(&tick_index);
        let water_retention = if in_shock
            && matches!(
                shock_kind,
                ShockKind::WaterSupply | ShockKind::CompoundWaterAndEnergy
            ) {
            parameters.water_shock_retention
        } else {
            1.0
        };
        let energy_retention = if in_shock
            && matches!(
                shock_kind,
                ShockKind::EnergySupply | ShockKind::CompoundWaterAndEnergy
            ) {
            parameters.energy_shock_retention
        } else {
            1.0
        };
        production_by_region[BASIN] = Production {
            resources: ResourceVector::new(
                40.0 * parameters.water_supply_multiplier * water_retention,
                16.0 * parameters.energy_supply_multiplier * energy_retention,
                0.0,
            ),
        };
    }
    scenario.validate()?;
    Ok(scenario)
}

pub fn run_parameterized_scenario(
    shock_kind: ShockKind,
    policy: InterventionPolicy,
    parameters: ParameterSet,
) -> Result<RunMetrics, UqError> {
    let scenario = parameterized_scenario(shock_kind, parameters)?;
    let outcome: RunOutcome = run_scenario(&scenario, policy)?;
    let household_tick_count = (outcome.ticks.len() * HOUSEHOLD_COUNT) as f64;
    let mut water_breaches = 0usize;
    let mut energy_breaches = 0usize;
    for tick in &outcome.ticks {
        water_breaches += tick
            .household_water_coverage
            .iter()
            .filter(|coverage| **coverage < DEFAULT_RECOVERY_THRESHOLD)
            .count();
        energy_breaches += tick
            .household_energy_coverage
            .iter()
            .filter(|coverage| **coverage < DEFAULT_RECOVERY_THRESHOLD)
            .count();
    }
    let metrics = RunMetrics {
        total_unmet_water_m3: outcome.total_unmet_water_m3,
        total_unmet_energy_kwh: outcome.total_unmet_energy_kwh,
        water_deficit_area_household_steps: outcome.water_deficit_area_household_steps,
        energy_deficit_area_household_steps: outcome.energy_deficit_area_household_steps,
        minimum_water_coverage: outcome.minimum_water_coverage,
        minimum_energy_coverage: outcome.minimum_energy_coverage,
        water_floor_breach_fraction: water_breaches as f64 / household_tick_count,
        energy_floor_breach_fraction: energy_breaches as f64 / household_tick_count,
        recovery_success: if outcome.recovery_after_ticks.is_some() {
            1.0
        } else {
            0.0
        },
    };
    for metric in OutcomeMetric::ALL {
        let value = metrics.value(metric);
        if !value.is_finite() {
            return Err(UqError::NonFiniteOutcome { metric, value });
        }
    }
    Ok(metrics)
}

pub fn execute_benchmark(
    seed: u64,
    lhs_sample_count: usize,
    morris_trajectory_count: usize,
) -> Result<BenchmarkReport, UqError> {
    let lhs_design = latin_hypercube_design(seed, lhs_sample_count)?;
    let mut lhs_runs =
        Vec::with_capacity(lhs_sample_count * ShockKind::ALL.len() * InterventionPolicy::ALL.len());
    for design_point in lhs_design {
        for shock_kind in ShockKind::ALL {
            for policy in InterventionPolicy::ALL {
                let metrics =
                    run_parameterized_scenario(shock_kind, policy, design_point.parameters)?;
                lhs_runs.push(LhsRunRecord {
                    design_point: design_point.clone(),
                    shock_kind,
                    policy,
                    metrics,
                });
            }
        }
    }
    let lhs_summaries = summarize_lhs(&lhs_runs);

    let trajectories = morris_design(seed ^ 0xA5A5_5A5A_D3C4_B2E1, morris_trajectory_count)?;
    let mut morris_points = Vec::with_capacity(morris_trajectory_count * (FACTOR_COUNT + 1));
    let factor_count = FACTOR_COUNT;
    let shock_count = ShockKind::ALL.len();
    let policy_count = InterventionPolicy::ALL.len();
    let metric_count = OutcomeMetric::ALL.len();
    let group_count = factor_count * shock_count * policy_count * metric_count;
    let mut effects_by_group =
        vec![Vec::<f64>::with_capacity(morris_trajectory_count); group_count];

    for trajectory in trajectories {
        for (point_index, unit_coordinates) in trajectory.points.iter().enumerate() {
            morris_points.push(MorrisPointRecord {
                trajectory_index: trajectory.trajectory_index,
                point_index,
                unit_coordinates: *unit_coordinates,
            });
        }

        let mut point_outputs = Vec::with_capacity(trajectory.points.len());
        for unit_coordinates in &trajectory.points {
            let parameters = ParameterSet::from_unit_coordinates(*unit_coordinates)?;
            let mut outputs = Vec::with_capacity(shock_count * policy_count);
            for shock_kind in ShockKind::ALL {
                for policy in InterventionPolicy::ALL {
                    outputs.push(run_parameterized_scenario(shock_kind, policy, parameters)?);
                }
            }
            point_outputs.push(outputs);
        }

        for (step_index, factor) in trajectory.changed_factors.iter().enumerate() {
            let previous = trajectory.points[step_index][factor.index()];
            let current = trajectory.points[step_index + 1][factor.index()];
            let signed_delta = current - previous;
            if (signed_delta.abs() - MORRIS_DELTA).abs() > 1e-12 {
                return Err(UqError::ParameterOutOfRange);
            }
            for shock_index in 0..shock_count {
                for policy_index in 0..policy_count {
                    let scenario_index = shock_index * policy_count + policy_index;
                    for metric in OutcomeMetric::ALL {
                        let before = point_outputs[step_index][scenario_index].value(metric);
                        let after = point_outputs[step_index + 1][scenario_index].value(metric);
                        let effect = (after - before) / signed_delta;
                        let group_index = (((factor.index() * shock_count + shock_index)
                            * policy_count
                            + policy_index)
                            * metric_count)
                            + metric.index();
                        effects_by_group[group_index].push(effect);
                    }
                }
            }
        }
    }

    let mut morris_effects = Vec::with_capacity(group_count);
    for factor in ParameterFactor::ALL {
        for shock_kind in ShockKind::ALL {
            for policy in InterventionPolicy::ALL {
                for metric in OutcomeMetric::ALL {
                    let group_index = (((factor.index() * shock_count + shock_index(shock_kind))
                        * policy_count
                        + policy_index(policy))
                        * metric_count)
                        + metric.index();
                    let effects = &effects_by_group[group_index];
                    let mean = average(effects);
                    let mu_star =
                        effects.iter().map(|value| value.abs()).sum::<f64>() / effects.len() as f64;
                    let sigma = sample_stddev(effects, mean);
                    morris_effects.push(MorrisEffectSummary {
                        factor,
                        shock_kind,
                        policy,
                        metric,
                        trajectory_count: effects.len(),
                        mu: mean,
                        mu_star,
                        sigma,
                    });
                }
            }
        }
    }

    Ok(BenchmarkReport {
        seed,
        lhs_runs,
        lhs_summaries,
        morris_points,
        morris_effects,
    })
}

fn shock_index(shock: ShockKind) -> usize {
    ShockKind::ALL
        .iter()
        .position(|candidate| *candidate == shock)
        .unwrap_or(0)
}

fn policy_index(policy: InterventionPolicy) -> usize {
    InterventionPolicy::ALL
        .iter()
        .position(|candidate| *candidate == policy)
        .unwrap_or(0)
}

fn summarize_lhs(runs: &[LhsRunRecord]) -> Vec<LhsSummary> {
    let mut summaries = Vec::with_capacity(
        ShockKind::ALL.len() * InterventionPolicy::ALL.len() * OutcomeMetric::ALL.len(),
    );
    for shock_kind in ShockKind::ALL {
        for policy in InterventionPolicy::ALL {
            for metric in OutcomeMetric::ALL {
                let mut values = runs
                    .iter()
                    .filter(|run| run.shock_kind == shock_kind && run.policy == policy)
                    .map(|run| run.metrics.value(metric))
                    .collect::<Vec<_>>();
                values.sort_by(f64::total_cmp);
                let mean = average(&values);
                let lower_count = ((values.len() as f64 * TAIL_FRACTION).ceil() as usize).max(1);
                let upper_start = values.len().saturating_sub(lower_count);
                let breach_count = runs
                    .iter()
                    .filter(|run| run.shock_kind == shock_kind && run.policy == policy)
                    .filter(|run| metric_breach(metric, run.metrics.value(metric)))
                    .count();
                summaries.push(LhsSummary {
                    shock_kind,
                    policy,
                    metric,
                    sample_count: values.len(),
                    mean,
                    p05: quantile(&values, 0.05),
                    p50: quantile(&values, 0.50),
                    p95: quantile(&values, 0.95),
                    lower_tail_mean: average(&values[..lower_count]),
                    upper_tail_mean: average(&values[upper_start..]),
                    run_breach_fraction: breach_count as f64 / values.len() as f64,
                });
            }
        }
    }
    summaries
}

fn metric_breach(metric: OutcomeMetric, value: f64) -> bool {
    match metric {
        OutcomeMetric::MinimumWaterCoverage | OutcomeMetric::MinimumEnergyCoverage => {
            value < DEFAULT_RECOVERY_THRESHOLD
        }
        OutcomeMetric::WaterFloorBreachFraction | OutcomeMetric::EnergyFloorBreachFraction => {
            value > 0.0
        }
        OutcomeMetric::RecoverySuccess => value < 1.0,
        OutcomeMetric::TotalUnmetWaterM3
        | OutcomeMetric::TotalUnmetEnergyKwh
        | OutcomeMetric::WaterDeficitAreaHouseholdSteps
        | OutcomeMetric::EnergyDeficitAreaHouseholdSteps => value > 0.0,
    }
}

fn quantile(sorted: &[f64], probability: f64) -> f64 {
    if sorted.len() == 1 {
        return sorted[0];
    }
    let position = probability.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let weight = position - lower as f64;
        sorted[lower] * (1.0 - weight) + sorted[upper] * weight
    }
}

fn average(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn sample_stddev(values: &[f64], mean: f64) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let sum = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>();
    (sum / (values.len() - 1) as f64).sqrt()
}

fn lerp(low: f64, high: f64, coordinate: f64) -> f64 {
    low + (high - low) * coordinate
}

#[derive(Debug, Clone)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn next_unit(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 + 0.5) / 9_007_199_254_740_992.0
    }
}

fn shuffle<T>(items: &mut [T], rng: &mut SplitMix64) {
    for index in (1..items.len()).rev() {
        let other = (rng.next_u64() % (index as u64 + 1)) as usize;
        items.swap(index, other);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latin_hypercube_uses_each_stratum_once_per_factor_and_replays_exactly() {
        let first = latin_hypercube_design(12345, LHS_DEFAULT_POINTS).unwrap();
        let second = latin_hypercube_design(12345, LHS_DEFAULT_POINTS).unwrap();
        assert_eq!(first, second);
        for factor in 0..FACTOR_COUNT {
            let mut strata = first
                .iter()
                .map(|point| {
                    (point.unit_coordinates[factor] * LHS_DEFAULT_POINTS as f64).floor() as usize
                })
                .collect::<Vec<_>>();
            strata.sort_unstable();
            assert_eq!(strata, (0..LHS_DEFAULT_POINTS).collect::<Vec<_>>());
        }
    }

    #[test]
    fn morris_trajectories_change_each_factor_once_by_two_thirds() {
        let trajectories = morris_design(991, MORRIS_DEFAULT_TRAJECTORIES).unwrap();
        assert_eq!(trajectories.len(), MORRIS_DEFAULT_TRAJECTORIES);
        for trajectory in trajectories {
            assert_eq!(trajectory.points.len(), FACTOR_COUNT + 1);
            assert_eq!(trajectory.changed_factors.len(), FACTOR_COUNT);
            let mut seen = trajectory
                .changed_factors
                .iter()
                .map(|factor| factor.index())
                .collect::<Vec<_>>();
            seen.sort_unstable();
            assert_eq!(seen, (0..FACTOR_COUNT).collect::<Vec<_>>());
            for point in &trajectory.points {
                assert!(point.iter().all(|value| (0.0..=1.0).contains(value)));
            }
            for (step, factor) in trajectory.changed_factors.iter().enumerate() {
                let delta = trajectory.points[step + 1][factor.index()]
                    - trajectory.points[step][factor.index()];
                assert!((delta.abs() - MORRIS_DELTA).abs() < 1e-12);
                for other in 0..FACTOR_COUNT {
                    if other != factor.index() {
                        assert_eq!(
                            trajectory.points[step + 1][other],
                            trajectory.points[step][other]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn supply_and_reserve_parameters_are_separate_from_demand_and_retention() {
        let parameters = ParameterSet::from_unit_coordinates([0.5; FACTOR_COUNT]).unwrap();
        assert_eq!(parameters.water_supply_multiplier, 1.0);
        assert_eq!(parameters.energy_supply_multiplier, 1.0);
        assert_eq!(parameters.demand_multiplier, 1.1);
        assert_eq!(parameters.distributed_reserve_multiplier, 1.0);
        assert_eq!(parameters.water_shock_retention, 0.25);
        assert_eq!(parameters.energy_shock_retention, 0.25);
        assert_eq!(parameters.shock_duration_ticks, 3);
    }

    #[test]
    fn parameterized_scenarios_preserve_resource_specific_shock_controls() {
        let parameters = ParameterSet::from_unit_coordinates([0.5; FACTOR_COUNT]).unwrap();
        let water = parameterized_scenario(ShockKind::WaterSupply, parameters).unwrap();
        let water_shock = water.production_by_tick[water.shock_start_tick][BASIN].resources;
        assert_eq!(water_shock.water_m3, 10.0);
        assert_eq!(water_shock.energy_kwh, 16.0);

        let energy = parameterized_scenario(ShockKind::EnergySupply, parameters).unwrap();
        let energy_shock = energy.production_by_tick[energy.shock_start_tick][BASIN].resources;
        assert_eq!(energy_shock.water_m3, 40.0);
        assert_eq!(energy_shock.energy_kwh, 4.0);
    }

    #[test]
    fn full_recovery_is_not_confused_with_parameter_sampling_repetitions() {
        let report = execute_benchmark(7, 3, 2).unwrap();
        assert_eq!(
            report.lhs_runs.len(),
            3 * ShockKind::ALL.len() * InterventionPolicy::ALL.len()
        );
        assert_eq!(
            report.lhs_summaries.len(),
            ShockKind::ALL.len() * InterventionPolicy::ALL.len() * OutcomeMetric::ALL.len()
        );
        assert_eq!(report.morris_points.len(), 2 * (FACTOR_COUNT + 1));
        assert_eq!(
            report.morris_effects.len(),
            FACTOR_COUNT
                * ShockKind::ALL.len()
                * InterventionPolicy::ALL.len()
                * OutcomeMetric::ALL.len()
        );
        assert!(
            report
                .morris_effects
                .iter()
                .all(|effect| effect.trajectory_count == 2)
        );
    }

    #[test]
    fn complete_retention_is_no_more_harmful_than_zero_retention_for_water_shock() {
        let mut no_retention = ParameterSet::from_unit_coordinates([0.5; FACTOR_COUNT]).unwrap();
        no_retention.water_shock_retention = 0.0;
        let mut full_retention = no_retention;
        full_retention.water_shock_retention = 1.0;
        let severe = run_parameterized_scenario(
            ShockKind::WaterSupply,
            InterventionPolicy::ProportionalSolidarity,
            no_retention,
        )
        .unwrap();
        let mild = run_parameterized_scenario(
            ShockKind::WaterSupply,
            InterventionPolicy::ProportionalSolidarity,
            full_retention,
        )
        .unwrap();
        assert!(mild.total_unmet_water_m3 <= severe.total_unmet_water_m3);
    }
}
