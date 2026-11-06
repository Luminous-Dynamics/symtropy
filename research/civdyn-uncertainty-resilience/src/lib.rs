// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Parameter uncertainty and elementary-effects screening for the CivDyn
//! cross-scale resilience engine.
//!
//! Latin-hypercube output distributions are induced by an explicitly declared
//! independent-uniform design measure. They are not posterior predictions,
//! empirical priors, stochastic replications or real-world event probabilities.
//! Morris mu*/sigma are screening diagnostics, not Sobol indices or causal effects.

use civdyn_cross_scale_resilience::{
    InterventionPolicy, ResilienceError, RunOutcome, Scenario, ShockKind, run_scenario,
};
use core::fmt;

pub const FACTOR_COUNT: usize = 7;
pub const DEFAULT_ENSEMBLE_SIZE: usize = 30;
pub const DEFAULT_MORRIS_TRAJECTORIES: usize = 30;
pub const MORRIS_LEVELS: u8 = 4;
pub const MORRIS_DELTA_LEVELS: u8 = 2;
pub const MORRIS_DELTA: f64 = 2.0 / 3.0;
pub const SERVICE_FLOOR: f64 = 0.95;
pub const DESIGN_SEED: u64 = 0x4353_535f_5551_3031;

const SHOCKS: [ShockKind; 3] = [
    ShockKind::WaterSupply,
    ShockKind::EnergySupply,
    ShockKind::CompoundWaterAndEnergy,
];
const POLICIES: [InterventionPolicy; 4] = [
    InterventionPolicy::NoTransfer,
    InterventionPolicy::ProportionalSolidarity,
    InterventionPolicy::DeficitFirst,
    InterventionPolicy::ReserveProtected,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Factor {
    BasinWaterSupply,
    BasinEnergySupply,
    HouseholdDemand,
    InitialReserve,
    WaterShockRetention,
    EnergyShockRetention,
    ShockDuration,
}

impl Factor {
    pub const ALL: [Self; FACTOR_COUNT] = [
        Self::BasinWaterSupply,
        Self::BasinEnergySupply,
        Self::HouseholdDemand,
        Self::InitialReserve,
        Self::WaterShockRetention,
        Self::EnergyShockRetention,
        Self::ShockDuration,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::BasinWaterSupply => "basin_water_supply_multiplier",
            Self::BasinEnergySupply => "basin_energy_supply_multiplier",
            Self::HouseholdDemand => "household_demand_multiplier",
            Self::InitialReserve => "initial_reserve_multiplier",
            Self::WaterShockRetention => "water_shock_retention",
            Self::EnergyShockRetention => "energy_shock_retention",
            Self::ShockDuration => "shock_duration_ticks",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParameterSet {
    pub basin_water_supply_multiplier: f64,
    pub basin_energy_supply_multiplier: f64,
    pub household_demand_multiplier: f64,
    pub initial_reserve_multiplier: f64,
    /// Fraction of normal water supply retained during a water/compound shock.
    pub water_shock_retention: f64,
    /// Fraction of normal energy supply retained during an energy/compound shock.
    pub energy_shock_retention: f64,
    pub shock_duration_ticks: usize,
}

impl ParameterSet {
    pub const BASELINE: Self = Self {
        basin_water_supply_multiplier: 1.0,
        basin_energy_supply_multiplier: 1.0,
        household_demand_multiplier: 1.0,
        initial_reserve_multiplier: 1.0,
        water_shock_retention: 0.0,
        energy_shock_retention: 0.0,
        shock_duration_ticks: 4,
    };

    /// Maps seven [0,1] coordinates to declared uniform parameter ranges.
    ///
    /// The ranges are stress-test assumptions, not empirical prior distributions:
    /// water/energy supply [0.7,1.3], demand [0.8,1.2], reserves [0.5,1.5],
    /// shock retention [0,0.6], and shock duration [2,5] ticks.
    pub fn from_unit_cube(values: [f64; FACTOR_COUNT]) -> Result<Self, UncertaintyError> {
        for (index, value) in values.iter().enumerate() {
            if !value.is_finite() || !(0.0..=1.0).contains(value) {
                return Err(UncertaintyError::UnitCubeCoordinateOutOfRange {
                    factor: Factor::ALL[index],
                    value: *value,
                });
            }
        }

        let result = Self {
            basin_water_supply_multiplier: 0.7 + 0.6 * values[0],
            basin_energy_supply_multiplier: 0.7 + 0.6 * values[1],
            household_demand_multiplier: 0.8 + 0.4 * values[2],
            initial_reserve_multiplier: 0.5 + values[3],
            water_shock_retention: 0.6 * values[4],
            energy_shock_retention: 0.6 * values[5],
            shock_duration_ticks: 2 + (3.0 * values[6]).round() as usize,
        };
        result.validate()?;
        Ok(result)
    }

    pub fn validate(&self) -> Result<(), UncertaintyError> {
        for (factor, value, minimum, maximum) in [
            (
                Factor::BasinWaterSupply,
                self.basin_water_supply_multiplier,
                0.7,
                1.3,
            ),
            (
                Factor::BasinEnergySupply,
                self.basin_energy_supply_multiplier,
                0.7,
                1.3,
            ),
            (
                Factor::HouseholdDemand,
                self.household_demand_multiplier,
                0.8,
                1.2,
            ),
            (
                Factor::InitialReserve,
                self.initial_reserve_multiplier,
                0.5,
                1.5,
            ),
            (
                Factor::WaterShockRetention,
                self.water_shock_retention,
                0.0,
                0.6,
            ),
            (
                Factor::EnergyShockRetention,
                self.energy_shock_retention,
                0.0,
                0.6,
            ),
        ] {
            if !value.is_finite() || value < minimum || value > maximum {
                return Err(UncertaintyError::ParameterOutOfRange {
                    factor,
                    value,
                    minimum,
                    maximum,
                });
            }
        }
        if !(2..=5).contains(&self.shock_duration_ticks) {
            return Err(UncertaintyError::ShockDurationOutOfRange(
                self.shock_duration_ticks,
            ));
        }
        Ok(())
    }

    pub fn coordinate(&self, factor: Factor) -> f64 {
        match factor {
            Factor::BasinWaterSupply => (self.basin_water_supply_multiplier - 0.7) / 0.6,
            Factor::BasinEnergySupply => (self.basin_energy_supply_multiplier - 0.7) / 0.6,
            Factor::HouseholdDemand => (self.household_demand_multiplier - 0.8) / 0.4,
            Factor::InitialReserve => self.initial_reserve_multiplier - 0.5,
            Factor::WaterShockRetention => self.water_shock_retention / 0.6,
            Factor::EnergyShockRetention => self.energy_shock_retention / 0.6,
            Factor::ShockDuration => (self.shock_duration_ticks.saturating_sub(2)) as f64 / 3.0,
        }
    }
}

pub fn apply_parameters(
    reference: &Scenario,
    parameters: ParameterSet,
) -> Result<Scenario, UncertaintyError> {
    parameters.validate()?;
    let mut scenario = reference.clone();
    let shock_end = scenario
        .shock_start_tick
        .checked_add(parameters.shock_duration_ticks)
        .ok_or(UncertaintyError::InvalidShockEnd)?;
    if shock_end >= scenario.production_by_tick.len() {
        return Err(UncertaintyError::InvalidShockEnd);
    }
    scenario.shock_end_tick = shock_end;

    for region in &mut scenario.regions {
        region.opening_stock.water_m3 *= parameters.initial_reserve_multiplier;
        region.opening_stock.energy_kwh *= parameters.initial_reserve_multiplier;
        region.essential_demand_per_capita.water_m3 *= parameters.household_demand_multiplier;
        region.essential_demand_per_capita.energy_kwh *= parameters.household_demand_multiplier;
    }

    let normal_supply = reference
        .production_by_tick
        .first()
        .and_then(|tick| tick.first())
        .ok_or(UncertaintyError::InvalidReferenceScenario)?
        .resources;

    for (tick_index, production) in scenario.production_by_tick.iter_mut().enumerate() {
        let in_shock = (scenario.shock_start_tick..scenario.shock_end_tick).contains(&tick_index);
        let water_retention = if in_shock
            && matches!(
                scenario.shock_kind,
                ShockKind::WaterSupply | ShockKind::CompoundWaterAndEnergy
            ) {
            parameters.water_shock_retention
        } else {
            1.0
        };
        let energy_retention = if in_shock
            && matches!(
                scenario.shock_kind,
                ShockKind::EnergySupply | ShockKind::CompoundWaterAndEnergy
            ) {
            parameters.energy_shock_retention
        } else {
            1.0
        };
        production[0].resources.water_m3 =
            normal_supply.water_m3 * parameters.basin_water_supply_multiplier * water_retention;
        production[0].resources.energy_kwh =
            normal_supply.energy_kwh * parameters.basin_energy_supply_multiplier * energy_retention;
    }
    scenario.validate()?;
    Ok(scenario)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    pub fn index(&mut self, upper_exclusive: usize) -> usize {
        assert!(upper_exclusive > 0);
        (self.next_u64() % upper_exclusive as u64) as usize
    }

    pub fn unit_f64(&mut self) -> f64 {
        let mantissa = self.next_u64() >> 11;
        mantissa as f64 / ((1_u64 << 53) as f64)
    }

    pub fn shuffle<T>(&mut self, values: &mut [T]) {
        for index in (1..values.len()).rev() {
            let other = self.index(index + 1);
            values.swap(index, other);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LatinHypercubeDesign {
    pub seed: u64,
    pub points: Vec<[f64; FACTOR_COUNT]>,
}

pub fn latin_hypercube(
    sample_count: usize,
    seed: u64,
) -> Result<LatinHypercubeDesign, UncertaintyError> {
    if sample_count < 2 {
        return Err(UncertaintyError::TooFewDesignSamples(sample_count));
    }
    let mut rng = SplitMix64::new(seed);
    let mut strata_by_factor = Vec::with_capacity(FACTOR_COUNT);

    for _ in 0..FACTOR_COUNT {
        let mut strata = (0..sample_count).collect::<Vec<_>>();
        rng.shuffle(&mut strata);
        strata_by_factor.push(strata);
    }

    let mut points = Vec::with_capacity(sample_count);
    for sample_index in 0..sample_count {
        let mut point = [0.0; FACTOR_COUNT];
        for factor in 0..FACTOR_COUNT {
            point[factor] =
                (strata_by_factor[factor][sample_index] as f64 + 0.5) / sample_count as f64;
        }
        points.push(point);
    }
    Ok(LatinHypercubeDesign { seed, points })
}

#[derive(Debug, Clone, PartialEq)]
pub struct MorrisTrajectory {
    pub base_levels: [u8; FACTOR_COUNT],
    /// Factor indices in the order changed along the trajectory.
    pub factor_order: [usize; FACTOR_COUNT],
    /// Direction for each factor: +1 or -1 in two grid levels.
    pub directions: [i8; FACTOR_COUNT],
    pub points: Vec<[f64; FACTOR_COUNT]>,
}

impl MorrisTrajectory {
    pub fn validate(&self) -> Result<(), UncertaintyError> {
        if self.points.len() != FACTOR_COUNT + 1 {
            return Err(UncertaintyError::InvalidMorrisPointCount(self.points.len()));
        }
        let mut seen = [false; FACTOR_COUNT];
        for index in &self.factor_order {
            if *index >= FACTOR_COUNT || seen[*index] {
                return Err(UncertaintyError::InvalidMorrisFactorOrder);
            }
            seen[*index] = true;
        }
        for (index, direction) in self.directions.iter().enumerate() {
            if *direction != 1 && *direction != -1 {
                return Err(UncertaintyError::InvalidMorrisDirection {
                    factor: Factor::ALL[index],
                    direction: *direction,
                });
            }
        }
        for point in &self.points {
            for value in point {
                if !value.is_finite() || !(0.0..=1.0).contains(value) {
                    return Err(UncertaintyError::MorrisCoordinateOutOfRange(*value));
                }
            }
        }
        for step in 0..FACTOR_COUNT {
            let before = self.points[step];
            let after = self.points[step + 1];
            let changed = (0..FACTOR_COUNT)
                .filter(|factor| (before[*factor] - after[*factor]).abs() > 1e-12)
                .collect::<Vec<_>>();
            if changed != vec![self.factor_order[step]] {
                return Err(UncertaintyError::MorrisMustChangeOneFactorPerStep);
            }
            let changed_factor = changed[0];
            if ((before[changed_factor] - after[changed_factor]).abs() - MORRIS_DELTA).abs() > 1e-12
            {
                return Err(UncertaintyError::InvalidMorrisStepSize);
            }
            let observed_direction = if after[changed_factor] > before[changed_factor] {
                1
            } else {
                -1
            };
            if self.directions[changed_factor] != observed_direction {
                return Err(UncertaintyError::MorrisDirectionMismatch {
                    factor: Factor::ALL[changed_factor],
                });
            }
        }
        Ok(())
    }
}

pub fn morris_design(
    trajectory_count: usize,
    seed: u64,
) -> Result<Vec<MorrisTrajectory>, UncertaintyError> {
    if trajectory_count == 0 {
        return Err(UncertaintyError::TooFewDesignSamples(trajectory_count));
    }
    let mut rng = SplitMix64::new(seed ^ 0x4D4F_5252_4953_3031);
    let mut trajectories = Vec::with_capacity(trajectory_count);

    for _ in 0..trajectory_count {
        let mut base_levels = [0_u8; FACTOR_COUNT];
        let mut directions = [1_i8; FACTOR_COUNT];
        let mut base = [0.0; FACTOR_COUNT];
        for factor in 0..FACTOR_COUNT {
            let low_or_high = rng.index(2) as u8;
            base_levels[factor] = low_or_high;
            directions[factor] = if low_or_high == 0 { 1 } else { -1 };
            base[factor] = f64::from(low_or_high) / f64::from(MORRIS_LEVELS - 1);
        }

        let mut factor_order = core::array::from_fn(|index| index);
        rng.shuffle(&mut factor_order);
        let mut points = Vec::with_capacity(FACTOR_COUNT + 1);
        points.push(base);

        let mut current = base;
        for factor in factor_order {
            let next_level = if directions[factor] > 0 {
                base_levels[factor] + MORRIS_DELTA_LEVELS
            } else {
                base_levels[factor] - MORRIS_DELTA_LEVELS
            };
            current[factor] = f64::from(next_level) / f64::from(MORRIS_LEVELS - 1);
            points.push(current);
        }

        let trajectory = MorrisTrajectory {
            base_levels,
            factor_order,
            directions,
            points,
        };
        trajectory.validate()?;
        trajectories.push(trajectory);
    }
    Ok(trajectories)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMetric {
    TotalUnmetWater,
    TotalUnmetEnergy,
    MinimumWaterCoverage,
    MinimumEnergyCoverage,
}

impl OutputMetric {
    pub const ALL: [Self; 4] = [
        Self::TotalUnmetWater,
        Self::TotalUnmetEnergy,
        Self::MinimumWaterCoverage,
        Self::MinimumEnergyCoverage,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::TotalUnmetWater => "total_unmet_water_m3",
            Self::TotalUnmetEnergy => "total_unmet_energy_kwh",
            Self::MinimumWaterCoverage => "minimum_water_coverage",
            Self::MinimumEnergyCoverage => "minimum_energy_coverage",
        }
    }

    pub const fn is_coverage(self) -> bool {
        matches!(
            self,
            Self::MinimumWaterCoverage | Self::MinimumEnergyCoverage
        )
    }

    pub fn value(self, outcome: &RunOutcome) -> f64 {
        match self {
            Self::TotalUnmetWater => outcome.total_unmet_water_m3,
            Self::TotalUnmetEnergy => outcome.total_unmet_energy_kwh,
            Self::MinimumWaterCoverage => outcome.minimum_water_coverage,
            Self::MinimumEnergyCoverage => outcome.minimum_energy_coverage,
        }
    }

    pub fn service_floor_breached(self, outcome: &RunOutcome) -> bool {
        match self {
            Self::TotalUnmetWater | Self::MinimumWaterCoverage => {
                outcome.minimum_water_coverage < SERVICE_FLOOR
            }
            Self::TotalUnmetEnergy | Self::MinimumEnergyCoverage => {
                outcome.minimum_energy_coverage < SERVICE_FLOOR
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnsembleSummary {
    pub shock: ShockKind,
    pub policy: InterventionPolicy,
    pub metric: OutputMetric,
    pub sample_count: usize,
    pub p05: f64,
    pub p50: f64,
    pub p95: f64,
    pub mean: f64,
    /// Mean of the worst 10% of values: upper tail for unmet demand,
    /// lower tail for service coverage.
    pub worst10_mean: f64,
    pub service_floor_breach_fraction: f64,
    pub recovery_success_fraction: f64,
}

pub fn summarize_ensemble(
    outcomes: &[RunOutcome],
) -> Result<Vec<EnsembleSummary>, UncertaintyError> {
    if outcomes.is_empty() {
        return Err(UncertaintyError::EmptyOutcomeEnsemble);
    }
    let mut summaries = Vec::new();
    for shock in SHOCKS {
        for policy in POLICIES {
            let selected = outcomes
                .iter()
                .filter(|outcome| outcome.shock_kind == shock && outcome.policy == policy)
                .collect::<Vec<_>>();
            if selected.is_empty() {
                return Err(UncertaintyError::MissingShockPolicyEnsemble { shock, policy });
            }
            for metric in OutputMetric::ALL {
                let mut values = selected
                    .iter()
                    .map(|outcome| metric.value(outcome))
                    .collect::<Vec<_>>();
                if values.iter().any(|value| !value.is_finite()) {
                    return Err(UncertaintyError::NonFiniteOutputMetric(metric));
                }
                values.sort_by(f64::total_cmp);
                let count = values.len();
                let mean = values.iter().sum::<f64>() / count as f64;
                let tail_count = ((count as f64 * 0.10).ceil() as usize).max(1);
                let tail = if metric.is_coverage() {
                    &values[..tail_count]
                } else {
                    &values[count - tail_count..]
                };
                let worst10_mean = tail.iter().sum::<f64>() / tail.len() as f64;
                let service_floor_breach_fraction = selected
                    .iter()
                    .filter(|outcome| metric.service_floor_breached(outcome))
                    .count() as f64
                    / count as f64;
                let recovery_success_fraction = selected
                    .iter()
                    .filter(|outcome| outcome.recovery_after_ticks.is_some())
                    .count() as f64
                    / count as f64;
                summaries.push(EnsembleSummary {
                    shock,
                    policy,
                    metric,
                    sample_count: count,
                    p05: quantile(&values, 0.05),
                    p50: quantile(&values, 0.50),
                    p95: quantile(&values, 0.95),
                    mean,
                    worst10_mean,
                    service_floor_breach_fraction,
                    recovery_success_fraction,
                });
            }
        }
    }
    Ok(summaries)
}

#[derive(Debug, Clone, PartialEq)]
pub struct MorrisEffectSummary {
    pub shock: ShockKind,
    pub policy: InterventionPolicy,
    pub factor: Factor,
    pub metric: OutputMetric,
    pub trajectory_count: usize,
    pub mu_star: f64,
    pub sigma: f64,
}

pub fn evaluate_morris(
    reference_scenarios: &[Scenario],
    trajectories: &[MorrisTrajectory],
) -> Result<Vec<MorrisEffectSummary>, UncertaintyError> {
    if reference_scenarios.len() != SHOCKS.len() {
        return Err(UncertaintyError::ReferenceScenarioCount(
            reference_scenarios.len(),
        ));
    }
    if trajectories.is_empty() {
        return Err(UncertaintyError::TooFewDesignSamples(0));
    }
    for trajectory in trajectories {
        trajectory.validate()?;
    }

    // [shock][policy][factor][metric][trajectory effects]
    let mut effects = vec![vec![vec![vec![Vec::<f64>::new(); 4]; FACTOR_COUNT]; 4]; 3];
    for trajectory in trajectories {
        let mut point_outcomes = Vec::with_capacity(FACTOR_COUNT + 1);
        for point in &trajectory.points {
            let parameters = ParameterSet::from_unit_cube(*point)?;
            let mut group = Vec::with_capacity(SHOCKS.len() * POLICIES.len());
            for (shock_index, reference) in reference_scenarios.iter().enumerate() {
                if reference.shock_kind != SHOCKS[shock_index] {
                    return Err(UncertaintyError::ReferenceScenarioOrderMismatch);
                }
                let scenario = apply_parameters(reference, parameters)?;
                for policy in POLICIES {
                    group.push(run_scenario(&scenario, policy)?);
                }
            }
            point_outcomes.push(group);
        }

        for (step, factor_index) in trajectory.factor_order.iter().enumerate() {
            for shock_index in 0..SHOCKS.len() {
                for policy_index in 0..POLICIES.len() {
                    let flat_index = shock_index * POLICIES.len() + policy_index;
                    for metric_index in 0..OutputMetric::ALL.len() {
                        let metric = OutputMetric::ALL[metric_index];
                        let before = metric.value(&point_outcomes[step][flat_index]);
                        let after = metric.value(&point_outcomes[step + 1][flat_index]);
                        let effect = (after - before) / MORRIS_DELTA;
                        effects[shock_index][policy_index][*factor_index][metric_index]
                            .push(effect);
                    }
                }
            }
        }
    }

    let mut summaries = Vec::new();
    for shock_index in 0..SHOCKS.len() {
        for policy_index in 0..POLICIES.len() {
            for factor_index in 0..FACTOR_COUNT {
                for metric_index in 0..OutputMetric::ALL.len() {
                    let values = &effects[shock_index][policy_index][factor_index][metric_index];
                    let count = values.len();
                    let mu_star =
                        values.iter().map(|value| value.abs()).sum::<f64>() / count as f64;
                    let mean = values.iter().sum::<f64>() / count as f64;
                    let variance = values
                        .iter()
                        .map(|value| (value - mean).powi(2))
                        .sum::<f64>()
                        / count as f64;
                    summaries.push(MorrisEffectSummary {
                        shock: SHOCKS[shock_index],
                        policy: POLICIES[policy_index],
                        factor: Factor::ALL[factor_index],
                        metric: OutputMetric::ALL[metric_index],
                        trajectory_count: count,
                        mu_star,
                        sigma: variance.sqrt(),
                    });
                }
            }
        }
    }
    Ok(summaries)
}

pub fn run_latin_hypercube_ensemble(
    reference_scenarios: &[Scenario],
    sample_count: usize,
    seed: u64,
) -> Result<Vec<RunOutcome>, UncertaintyError> {
    if reference_scenarios.len() != SHOCKS.len() {
        return Err(UncertaintyError::ReferenceScenarioCount(
            reference_scenarios.len(),
        ));
    }
    let design = latin_hypercube(sample_count, seed)?;
    let mut outcomes = Vec::with_capacity(sample_count * SHOCKS.len() * POLICIES.len());

    for point in &design.points {
        let parameters = ParameterSet::from_unit_cube(*point)?;
        for (index, reference) in reference_scenarios.iter().enumerate() {
            if reference.shock_kind != SHOCKS[index] {
                return Err(UncertaintyError::ReferenceScenarioOrderMismatch);
            }
            let scenario = apply_parameters(reference, parameters)?;
            for policy in POLICIES {
                outcomes.push(run_scenario(&scenario, policy)?);
            }
        }
    }
    Ok(outcomes)
}

pub fn reference_scenarios() -> Vec<Scenario> {
    SHOCKS
        .into_iter()
        .map(civdyn_cross_scale_resilience::reference_scenario)
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub enum UncertaintyError {
    Ledger(ResilienceError),
    UnitCubeCoordinateOutOfRange {
        factor: Factor,
        value: f64,
    },
    ParameterOutOfRange {
        factor: Factor,
        value: f64,
        minimum: f64,
        maximum: f64,
    },
    ShockDurationOutOfRange(usize),
    InvalidShockEnd,
    InvalidReferenceScenario,
    TooFewDesignSamples(usize),
    MorrisCoordinateOutOfRange(f64),
    InvalidMorrisPointCount(usize),
    InvalidMorrisFactorOrder,
    InvalidMorrisDirection {
        factor: Factor,
        direction: i8,
    },
    MorrisMustChangeOneFactorPerStep,
    InvalidMorrisStepSize,
    MorrisDirectionMismatch {
        factor: Factor,
    },
    EmptyOutcomeEnsemble,
    MissingShockPolicyEnsemble {
        shock: ShockKind,
        policy: InterventionPolicy,
    },
    NonFiniteOutputMetric(OutputMetric),
    ReferenceScenarioCount(usize),
    ReferenceScenarioOrderMismatch,
}

impl fmt::Display for UncertaintyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ledger(error) => write!(f, "resilience engine error: {error}"),
            Self::UnitCubeCoordinateOutOfRange { factor, value } => {
                write!(f, "{} coordinate is outside [0,1]: {value}", factor.name())
            }
            Self::ParameterOutOfRange {
                factor,
                value,
                minimum,
                maximum,
            } => write!(
                f,
                "{} value {value} is outside [{minimum}, {maximum}]",
                factor.name()
            ),
            Self::ShockDurationOutOfRange(value) => {
                write!(f, "shock duration {value} is outside [2,5]")
            }
            Self::InvalidShockEnd => write!(f, "parameterized shock end is outside the scenario"),
            Self::InvalidReferenceScenario => {
                write!(f, "reference scenario has no basin supply entry")
            }
            Self::TooFewDesignSamples(count) => {
                write!(f, "design requires at least one/two samples, got {count}")
            }
            Self::MorrisCoordinateOutOfRange(value) => {
                write!(f, "Morris coordinate is outside [0,1]: {value}")
            }
            Self::InvalidMorrisPointCount(count) => {
                write!(
                    f,
                    "Morris trajectory has {count} points; expected {}",
                    FACTOR_COUNT + 1
                )
            }
            Self::InvalidMorrisFactorOrder => {
                write!(f, "Morris trajectory must change every factor exactly once")
            }
            Self::InvalidMorrisDirection { factor, direction } => {
                write!(f, "{} has invalid direction {direction}", factor.name())
            }
            Self::MorrisMustChangeOneFactorPerStep => {
                write!(
                    f,
                    "each Morris step must change exactly one declared factor"
                )
            }
            Self::InvalidMorrisStepSize => {
                write!(f, "Morris trajectory step differs from delta=2/3")
            }
            Self::MorrisDirectionMismatch { factor } => {
                write!(f, "Morris step direction disagrees with {}", factor.name())
            }
            Self::EmptyOutcomeEnsemble => write!(f, "parameter outcome ensemble must not be empty"),
            Self::MissingShockPolicyEnsemble { shock, policy } => {
                write!(
                    f,
                    "missing ensemble for {} / {}",
                    shock.name(),
                    policy.name()
                )
            }
            Self::NonFiniteOutputMetric(metric) => {
                write!(f, "non-finite output values for {}", metric.name())
            }
            Self::ReferenceScenarioCount(count) => {
                write!(
                    f,
                    "expected {} ordered reference scenarios, got {count}",
                    SHOCKS.len()
                )
            }
            Self::ReferenceScenarioOrderMismatch => {
                write!(
                    f,
                    "reference scenario order must match the frozen shock portfolio"
                )
            }
        }
    }
}

impl std::error::Error for UncertaintyError {}

impl From<ResilienceError> for UncertaintyError {
    fn from(value: ResilienceError) -> Self {
        Self::Ledger(value)
    }
}

fn quantile(sorted: &[f64], probability: f64) -> f64 {
    let position = probability * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        return sorted[lower];
    }
    let fraction = position - lower as f64;
    sorted[lower] + fraction * (sorted[upper] - sorted[lower])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latin_hypercube_uses_each_stratum_once_per_factor() {
        let design = latin_hypercube(30, DESIGN_SEED).unwrap();
        assert_eq!(design.points.len(), 30);
        for factor in 0..FACTOR_COUNT {
            let mut strata = design
                .points
                .iter()
                .map(|point| (point[factor] * 30.0).floor() as usize)
                .collect::<Vec<_>>();
            strata.sort_unstable();
            assert_eq!(strata, (0..30).collect::<Vec<_>>());
        }
    }

    #[test]
    fn latin_hypercube_replay_is_deterministic() {
        assert_eq!(
            latin_hypercube(30, DESIGN_SEED).unwrap(),
            latin_hypercube(30, DESIGN_SEED).unwrap()
        );
        assert_ne!(
            latin_hypercube(30, DESIGN_SEED).unwrap(),
            latin_hypercube(30, DESIGN_SEED + 1).unwrap()
        );
    }

    #[test]
    fn morris_paths_change_each_factor_once_and_stay_in_bounds() {
        let trajectories = morris_design(30, DESIGN_SEED).unwrap();
        assert_eq!(trajectories.len(), 30);
        for trajectory in &trajectories {
            trajectory.validate().unwrap();
            assert_eq!(trajectory.factor_order.len(), FACTOR_COUNT);
            assert_eq!(trajectory.points.len(), FACTOR_COUNT + 1);
        }
    }

    #[test]
    fn unit_cube_mapping_is_bounded_and_baseline_is_exact() {
        let low = ParameterSet::from_unit_cube([0.0; FACTOR_COUNT]).unwrap();
        let high = ParameterSet::from_unit_cube([1.0; FACTOR_COUNT]).unwrap();
        assert_eq!(low.basin_water_supply_multiplier, 0.7);
        assert_eq!(high.basin_energy_supply_multiplier, 1.3);
        assert_eq!(low.shock_duration_ticks, 2);
        assert_eq!(high.shock_duration_ticks, 5);
        assert_eq!(
            ParameterSet::from_unit_cube([0.5; FACTOR_COUNT])
                .unwrap()
                .household_demand_multiplier,
            1.0
        );
        ParameterSet::BASELINE.validate().unwrap();
    }

    #[test]
    fn invalid_parameters_fail_closed() {
        assert!(matches!(
            ParameterSet::from_unit_cube([1.1, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]),
            Err(UncertaintyError::UnitCubeCoordinateOutOfRange { .. })
        ));
        let mut invalid = ParameterSet::BASELINE;
        invalid.initial_reserve_multiplier = f64::NAN;
        assert!(matches!(
            invalid.validate(),
            Err(UncertaintyError::ParameterOutOfRange { .. })
        ));
    }

    #[test]
    fn parameterized_shock_retention_and_duration_are_applied() {
        let reference = reference_scenarios().pop().unwrap();
        let parameters = ParameterSet {
            water_shock_retention: 0.25,
            energy_shock_retention: 0.5,
            shock_duration_ticks: 3,
            ..ParameterSet::BASELINE
        };
        let scenario = apply_parameters(&reference, parameters).unwrap();
        assert_eq!(scenario.shock_end_tick, scenario.shock_start_tick + 3);
        assert!(
            (scenario.production_by_tick[scenario.shock_start_tick][0]
                .resources
                .water_m3
                - 10.0)
                .abs()
                < 1e-12
        );
        assert!(
            (scenario.production_by_tick[scenario.shock_start_tick][0]
                .resources
                .energy_kwh
                - 8.0)
                .abs()
                < 1e-12
        );
        assert_eq!(
            scenario.production_by_tick[scenario.shock_end_tick][0]
                .resources
                .water_m3,
            40.0
        );
        assert_eq!(
            scenario.production_by_tick[scenario.shock_end_tick][0]
                .resources
                .energy_kwh,
            16.0
        );
    }

    #[test]
    fn morris_evaluation_emits_one_effect_per_trajectory_factor_and_output() {
        let result = evaluate_morris(
            &reference_scenarios(),
            &morris_design(4, DESIGN_SEED).unwrap(),
        )
        .unwrap();
        assert_eq!(
            result.len(),
            SHOCKS.len() * POLICIES.len() * FACTOR_COUNT * OutputMetric::ALL.len()
        );
        assert!(result.iter().all(|row| row.trajectory_count == 4));
    }

    #[test]
    fn ensemble_summarization_keeps_water_and_energy_metrics_separate() {
        let outcomes =
            run_latin_hypercube_ensemble(&reference_scenarios(), 30, DESIGN_SEED).unwrap();
        assert_eq!(outcomes.len(), 30 * SHOCKS.len() * POLICIES.len());
        let summaries = summarize_ensemble(&outcomes).unwrap();
        assert_eq!(
            summaries.len(),
            SHOCKS.len() * POLICIES.len() * OutputMetric::ALL.len()
        );
        assert!(summaries.iter().all(|row| row.sample_count == 30));
        assert!(
            summaries
                .iter()
                .all(|row| row.p05 <= row.p50 && row.p50 <= row.p95)
        );
    }

    #[test]
    fn greater_shock_retention_is_no_worse_than_zero_retention() {
        let reference = reference_scenarios().remove(0);
        let partial_supply = ParameterSet {
            water_shock_retention: 0.6,
            ..ParameterSet::BASELINE
        };
        let zero_supply = ParameterSet {
            water_shock_retention: 0.0,
            ..ParameterSet::BASELINE
        };
        let partial_outcome = run_scenario(
            &apply_parameters(&reference, partial_supply).unwrap(),
            InterventionPolicy::ProportionalSolidarity,
        )
        .unwrap();
        let zero_outcome = run_scenario(
            &apply_parameters(&reference, zero_supply).unwrap(),
            InterventionPolicy::ProportionalSolidarity,
        )
        .unwrap();
        assert!(partial_outcome.total_unmet_water_m3 <= zero_outcome.total_unmet_water_m3 + 1e-9);
    }
}
