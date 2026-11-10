// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Small, ledger-backed reference for civilizational self-correction.
//!
//! This is a controlled benchmark kernel, not a general-purpose civilization
//! simulator. It composes the shared resource ledger and deliberately keeps
//! household service, industrial service, reserve capital, maintenance energy,
//! sensor error and correction evidence as separate quantities.

pub mod causal_drift;
pub mod causal_identifiability;
pub mod causal_transport;
pub mod causal_transport_sensitivity;
pub mod interference_identifiability;
pub mod model_adequacy;
pub mod noisy_causal_drift;
pub mod sequential_methods;

use civdyn_resource_ledger::{
    Production, Region, ResourceVector, StepInput, StepReport, TransferPolicy, TransferRequest,
    World, WorldError,
};
use core::fmt;

pub const SENSOR_TICKS: usize = 5;
pub const SENSOR_DRIFT_START_TICK: usize = 2;
pub const SENSOR_TRUE_COVERAGE: f64 = 1.0;
pub const SENSOR_DRIFT_REPORTED_COVERAGE: f64 = 0.6;
pub const SENSOR_EMERGENCY_THRESHOLD: f64 = 0.95;
pub const SENSOR_HOUSEHOLD_DEMAND_M3: f64 = 10.0;
pub const SENSOR_BASE_DELIVERY_M3: f64 = 10.0;
pub const SENSOR_OPENING_RESERVE_M3: f64 = 100.0;

pub const GROWTH_TICKS: usize = 4;
pub const RECOVERY_TICKS: usize = 2;
pub const REPEATED_SHOCK_TICKS: usize = 6;
pub const DELAYED_TOTAL_TICKS: usize = GROWTH_TICKS + RECOVERY_TICKS + REPEATED_SHOCK_TICKS;

pub const BASIN: usize = 0;
pub const HOUSEHOLD: usize = 1;
pub const INDUSTRY: usize = 2;
pub const PUMP_MAINTENANCE: usize = 3;
pub const DELAYED_REGION_COUNT: usize = 4;

pub const DELAYED_OPENING_WATER_M3: f64 = 80.0;
pub const DELAYED_OPENING_ENERGY_KWH: f64 = 100.0;
pub const HOUSEHOLD_DEMAND_M3_PER_TICK: f64 = 10.0;
pub const INDUSTRIAL_DEMAND_M3_PER_TICK: f64 = 5.0;
pub const MAINTENANCE_ENERGY_KWH_PER_TICK: f64 = 5.0;
pub const BASE_RECHARGE_M3_PER_TICK: f64 = 10.0;
pub const MAINTENANCE_RECHARGE_GAIN_M3_PER_TICK: f64 = 10.0;
pub const MAINTENANCE_PUMPING_YIELD_M3_PER_KWH: f64 = 2.0;
pub const ESSENTIAL_COVERAGE_FLOOR: f64 = 0.95;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerKind {
    FrozenPolicy,
    OutcomeReactive,
    MechanismAware,
}

impl ControllerKind {
    pub const ALL: [Self; 3] = [
        Self::FrozenPolicy,
        Self::OutcomeReactive,
        Self::MechanismAware,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::FrozenPolicy => "frozen_policy",
            Self::OutcomeReactive => "outcome_reactive",
            Self::MechanismAware => "mechanism_aware",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrectionStage {
    Observed,
    Detected,
    Diagnosed,
    Discriminated,
    BeliefUpdated,
    Intervened,
    Audited,
}

impl CorrectionStage {
    const ORDERED: [Self; 7] = [
        Self::Observed,
        Self::Detected,
        Self::Diagnosed,
        Self::Discriminated,
        Self::BeliefUpdated,
        Self::Intervened,
        Self::Audited,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Detected => "detected",
            Self::Diagnosed => "diagnosed",
            Self::Discriminated => "discriminated",
            Self::BeliefUpdated => "belief_updated",
            Self::Intervened => "intervened",
            Self::Audited => "audited",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrectionEvent {
    pub stage: CorrectionStage,
    /// One-based tick in the actual episode. Counterfactual ticks are not
    /// inserted into the actual event timeline.
    pub tick: usize,
    /// Stable protocol label, not free-form model-generated text.
    pub evidence_code: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelfCorrectionError {
    Ledger(String),
    InvalidTraceLength {
        expected: usize,
        actual: usize,
    },
    InvalidTraceStage {
        index: usize,
    },
    NonMonotonicTrace {
        previous_tick: usize,
        next_tick: usize,
    },
    InvalidScenarioTrajectory {
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for SelfCorrectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ledger(message) => write!(formatter, "resource ledger error: {message}"),
            Self::InvalidTraceLength { expected, actual } => {
                write!(
                    formatter,
                    "expected {expected} correction events, got {actual}"
                )
            }
            Self::InvalidTraceStage { index } => {
                write!(
                    formatter,
                    "correction trace has an invalid stage at index {index}"
                )
            }
            Self::NonMonotonicTrace {
                previous_tick,
                next_tick,
            } => write!(
                formatter,
                "correction trace tick moved backwards from {previous_tick} to {next_tick}"
            ),
            Self::InvalidScenarioTrajectory { expected, actual } => {
                write!(
                    formatter,
                    "expected {expected} trajectory ticks, got {actual}"
                )
            }
        }
    }
}

impl std::error::Error for SelfCorrectionError {}

impl From<WorldError> for SelfCorrectionError {
    fn from(value: WorldError) -> Self {
        Self::Ledger(value.to_string())
    }
}

pub fn validate_full_correction_trace(
    events: &[CorrectionEvent],
) -> Result<(), SelfCorrectionError> {
    if events.len() != CorrectionStage::ORDERED.len() {
        return Err(SelfCorrectionError::InvalidTraceLength {
            expected: CorrectionStage::ORDERED.len(),
            actual: events.len(),
        });
    }
    for (index, (event, expected)) in events.iter().zip(CorrectionStage::ORDERED).enumerate() {
        if event.stage != expected {
            return Err(SelfCorrectionError::InvalidTraceStage { index });
        }
        if index > 0 && event.tick < events[index - 1].tick {
            return Err(SelfCorrectionError::NonMonotonicTrace {
                previous_tick: events[index - 1].tick,
                next_tick: event.tick,
            });
        }
    }
    Ok(())
}

fn event(stage: CorrectionStage, tick: usize, evidence_code: &'static str) -> CorrectionEvent {
    CorrectionEvent {
        stage,
        tick,
        evidence_code,
    }
}

fn max_ledger_residuals(reports: &[StepReport]) -> (f64, f64) {
    let mut water = 0.0_f64;
    let mut energy = 0.0_f64;
    for report in reports {
        for region in &report.regions {
            let residual = region.ledger.conservation_residual();
            water = water.max(residual.water_m3.abs());
            energy = energy.max(residual.energy_kwh.abs());
        }
    }
    (water, energy)
}

fn region(
    id: &str,
    population: f64,
    opening_stock: ResourceVector,
    essential_demand_per_capita: ResourceVector,
) -> Region {
    Region {
        id: id.to_string(),
        population,
        opening_stock,
        essential_demand_per_capita,
    }
}

fn sensor_world() -> Result<World, SelfCorrectionError> {
    Ok(World::new(vec![
        region(
            "sensor_basin",
            0.0,
            ResourceVector::new(SENSOR_OPENING_RESERVE_M3, 0.0, 0.0),
            ResourceVector::default(),
        ),
        region(
            "sensor_household",
            1.0,
            ResourceVector::default(),
            ResourceVector::new(SENSOR_HOUSEHOLD_DEMAND_M3, 0.0, 0.0),
        ),
    ])?)
}

#[derive(Debug, Clone, PartialEq)]
pub struct SensorDriftOutcome {
    pub controller: ControllerKind,
    pub minimum_true_household_coverage: f64,
    pub minimum_reported_sensor_coverage: f64,
    pub unnecessary_dispatch_m3: f64,
    pub reserve_before_m3: f64,
    pub reserve_after_m3: f64,
    pub household_unmet_m3: f64,
    pub mechanism_identified: bool,
    pub correction_latency_ticks: Option<usize>,
    pub trace_valid: bool,
    pub trace: Vec<CorrectionEvent>,
    pub maximum_water_residual_m3: f64,
    pub maximum_energy_residual_kwh: f64,
}

pub fn run_sensor_drift_sham(
    controller: ControllerKind,
) -> Result<SensorDriftOutcome, SelfCorrectionError> {
    let mut world = sensor_world()?;
    let reserve_before_m3 = world.regions()[BASIN].opening_stock.water_m3;
    let mut reports = Vec::with_capacity(SENSOR_TICKS);
    let mut trace = Vec::new();
    let mut calibrated = false;
    let mut mechanism_identified = false;
    let mut correction_latency_ticks = None;
    let mut unnecessary_dispatch_m3 = 0.0;
    let mut minimum_true_household_coverage = 1.0_f64;
    let mut minimum_reported_sensor_coverage = 1.0_f64;
    let mut household_unmet_m3 = 0.0;

    for tick_index in 0..SENSOR_TICKS {
        let tick = tick_index + 1;
        let true_meter = SENSOR_TRUE_COVERAGE;
        let raw_primary = if tick_index < SENSOR_DRIFT_START_TICK {
            SENSOR_TRUE_COVERAGE
        } else {
            SENSOR_DRIFT_REPORTED_COVERAGE
        };
        let primary = if calibrated { true_meter } else { raw_primary };
        minimum_reported_sensor_coverage = minimum_reported_sensor_coverage.min(primary);

        let mut extra_dispatch_m3 = 0.0;
        match controller {
            ControllerKind::FrozenPolicy => {}
            ControllerKind::OutcomeReactive => {
                if primary < SENSOR_EMERGENCY_THRESHOLD {
                    extra_dispatch_m3 = (SENSOR_HOUSEHOLD_DEMAND_M3 * (1.0 - primary)).max(0.0);
                }
            }
            ControllerKind::MechanismAware => {
                if !calibrated && (raw_primary - true_meter).abs() > 0.15 {
                    trace = vec![
                        event(CorrectionStage::Observed, tick, "primary_meter_disagrees"),
                        event(
                            CorrectionStage::Detected,
                            tick,
                            "cross_meter_residual_exceeds_floor",
                        ),
                        event(
                            CorrectionStage::Diagnosed,
                            tick,
                            "primary_sensor_scale_drift",
                        ),
                        event(
                            CorrectionStage::Discriminated,
                            tick,
                            "independent_meter_probe",
                        ),
                        event(
                            CorrectionStage::BeliefUpdated,
                            tick,
                            "sensor_not_physical_service",
                        ),
                        event(
                            CorrectionStage::Intervened,
                            tick,
                            "calibrate_primary_sensor",
                        ),
                        event(
                            CorrectionStage::Audited,
                            tick,
                            "ledger_and_secondary_meter_agree",
                        ),
                    ];
                    validate_full_correction_trace(&trace)?;
                    calibrated = true;
                    mechanism_identified = true;
                    correction_latency_ticks = Some(0);
                }
            }
        }

        let mut transfers = vec![TransferRequest {
            from_region: BASIN,
            to_region: HOUSEHOLD,
            amount: ResourceVector::new(SENSOR_BASE_DELIVERY_M3, 0.0, 0.0),
        }];
        if extra_dispatch_m3 > 0.0 {
            transfers.push(TransferRequest {
                from_region: BASIN,
                to_region: HOUSEHOLD,
                amount: ResourceVector::new(extra_dispatch_m3, 0.0, 0.0),
            });
        }

        let report = world.step(
            &StepInput {
                production: vec![
                    Production {
                        resources: ResourceVector::new(SENSOR_BASE_DELIVERY_M3, 0.0, 0.0),
                    },
                    Production::default(),
                ],
                transfers,
            },
            TransferPolicy::Unrestricted,
        )?;

        if extra_dispatch_m3 > 0.0
            && let Some(receipt) = report.transfers.get(1)
        {
            unnecessary_dispatch_m3 += receipt.delivered.water_m3;
        }
        let household = &report.regions[HOUSEHOLD];
        minimum_true_household_coverage =
            minimum_true_household_coverage.min(household.coverage.water.unwrap_or(0.0));
        household_unmet_m3 += household.unmet.water_m3;
        reports.push(report);
    }

    if reports.len() != SENSOR_TICKS {
        return Err(SelfCorrectionError::InvalidScenarioTrajectory {
            expected: SENSOR_TICKS,
            actual: reports.len(),
        });
    }
    let (maximum_water_residual_m3, maximum_energy_residual_kwh) = max_ledger_residuals(&reports);

    Ok(SensorDriftOutcome {
        controller,
        minimum_true_household_coverage,
        minimum_reported_sensor_coverage,
        unnecessary_dispatch_m3,
        reserve_before_m3,
        reserve_after_m3: world.regions()[BASIN].opening_stock.water_m3,
        household_unmet_m3,
        mechanism_identified,
        correction_latency_ticks,
        trace_valid: !trace.is_empty() && validate_full_correction_trace(&trace).is_ok(),
        trace,
        maximum_water_residual_m3,
        maximum_energy_residual_kwh,
    })
}

pub fn run_sensor_drift_portfolio() -> Result<Vec<SensorDriftOutcome>, SelfCorrectionError> {
    ControllerKind::ALL
        .into_iter()
        .map(run_sensor_drift_sham)
        .collect()
}

fn delayed_world() -> Result<World, SelfCorrectionError> {
    Ok(World::new(vec![
        region(
            "source_basin",
            0.0,
            ResourceVector::new(DELAYED_OPENING_WATER_M3, DELAYED_OPENING_ENERGY_KWH, 0.0),
            ResourceVector::default(),
        ),
        region(
            "households",
            1.0,
            ResourceVector::default(),
            ResourceVector::new(HOUSEHOLD_DEMAND_M3_PER_TICK, 0.0, 0.0),
        ),
        region(
            "industry",
            1.0,
            ResourceVector::default(),
            ResourceVector::new(INDUSTRIAL_DEMAND_M3_PER_TICK, 0.0, 0.0),
        ),
        region(
            "pump_maintenance",
            1.0,
            ResourceVector::default(),
            ResourceVector::new(0.0, MAINTENANCE_ENERGY_KWH_PER_TICK, 0.0),
        ),
    ])?)
}

fn delayed_step(
    world: &mut World,
    water_recharge_m3: f64,
    industry_enabled: bool,
    maintenance_enabled: bool,
) -> Result<StepReport, SelfCorrectionError> {
    let mut transfers = vec![TransferRequest {
        from_region: BASIN,
        to_region: HOUSEHOLD,
        amount: ResourceVector::new(HOUSEHOLD_DEMAND_M3_PER_TICK, 0.0, 0.0),
    }];
    if industry_enabled {
        transfers.push(TransferRequest {
            from_region: BASIN,
            to_region: INDUSTRY,
            amount: ResourceVector::new(INDUSTRIAL_DEMAND_M3_PER_TICK, 0.0, 0.0),
        });
    }
    if maintenance_enabled {
        transfers.push(TransferRequest {
            from_region: BASIN,
            to_region: PUMP_MAINTENANCE,
            amount: ResourceVector::new(0.0, MAINTENANCE_ENERGY_KWH_PER_TICK, 0.0),
        });
    }
    let mut production = vec![Production::default(); DELAYED_REGION_COUNT];
    production[BASIN].resources.water_m3 = water_recharge_m3;

    Ok(world.step(
        &StepInput {
            production,
            transfers,
        },
        TransferPolicy::Unrestricted,
    )?)
}

fn run_recovery_interval(
    world: &mut World,
    maintenance_enabled: bool,
    reports: &mut Vec<StepReport>,
) -> Result<(), SelfCorrectionError> {
    for _ in 0..RECOVERY_TICKS {
        // Pump output is bounded by energy actually available in the source
        // basin. The following ledger step transfers and consumes that same
        // energy quantity at the maintenance node.
        let pump_energy_kwh = if maintenance_enabled {
            world.regions()[BASIN]
                .opening_stock
                .energy_kwh
                .min(MAINTENANCE_ENERGY_KWH_PER_TICK)
        } else {
            0.0
        };
        let extra_recharge_m3 = pump_energy_kwh * MAINTENANCE_PUMPING_YIELD_M3_PER_KWH;
        reports.push(delayed_step(
            world,
            BASE_RECHARGE_M3_PER_TICK + extra_recharge_m3,
            true,
            maintenance_enabled,
        )?);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShockCounterfactual {
    pub household_unmet_m3: f64,
    pub industry_unmet_m3: f64,
    pub final_reserve_m3: f64,
    pub maximum_water_residual_m3: f64,
    pub maximum_energy_residual_kwh: f64,
}

fn simulate_repeated_shock(
    mut world: World,
    industry_enabled: bool,
) -> Result<ShockCounterfactual, SelfCorrectionError> {
    let mut reports = Vec::with_capacity(REPEATED_SHOCK_TICKS);
    let mut household_unmet_m3 = 0.0;
    let mut industry_unmet_m3 = 0.0;
    for _ in 0..REPEATED_SHOCK_TICKS {
        let report = delayed_step(&mut world, 0.0, industry_enabled, false)?;
        household_unmet_m3 += report.regions[HOUSEHOLD].unmet.water_m3;
        industry_unmet_m3 += report.regions[INDUSTRY].unmet.water_m3;
        reports.push(report);
    }
    let (maximum_water_residual_m3, maximum_energy_residual_kwh) = max_ledger_residuals(&reports);
    Ok(ShockCounterfactual {
        household_unmet_m3,
        industry_unmet_m3,
        final_reserve_m3: world.regions()[BASIN].opening_stock.water_m3,
        maximum_water_residual_m3,
        maximum_energy_residual_kwh,
    })
}

fn sum_unmet_water(reports: &[StepReport], region_index: usize) -> f64 {
    reports
        .iter()
        .map(|report| report.regions[region_index].unmet.water_m3)
        .sum()
}

fn minimum_coverage(reports: &[StepReport], region_index: usize) -> f64 {
    reports
        .iter()
        .map(|report| report.regions[region_index].coverage.water.unwrap_or(0.0))
        .fold(1.0_f64, f64::min)
}

fn sum_maintenance_energy_served(reports: &[StepReport]) -> f64 {
    reports
        .iter()
        .map(|report| {
            report.regions[PUMP_MAINTENANCE]
                .ledger
                .served_essential_demand
                .energy_kwh
        })
        .sum()
}

fn sum_maintenance_energy_unmet(reports: &[StepReport]) -> f64 {
    reports
        .iter()
        .map(|report| report.regions[PUMP_MAINTENANCE].unmet.energy_kwh)
        .sum()
}

#[derive(Debug, Clone, PartialEq)]
pub struct DelayedExternalityOutcome {
    pub controller: ControllerKind,
    pub reserve_before_m3: f64,
    pub reserve_after_growth_m3: f64,
    pub reserve_after_recovery_m3: f64,
    pub reserve_after_repeated_shock_m3: f64,
    pub household_unmet_m3: f64,
    pub industry_unmet_m3: f64,
    pub minimum_household_coverage: f64,
    pub maintenance_energy_kwh: f64,
    /// Cumulative maintenance-energy obligation not delivered across the
    /// complete trajectory. This is reported separately from energy consumed.
    pub maintenance_energy_unmet_kwh: f64,
    /// Household unmet demand in the industry-on shock branch from the exact
    /// post-recovery snapshot.
    pub counterfactual_household_unmet_industry_on_m3: f64,
    /// Household unmet demand in the industry-off shock branch from that same
    /// post-recovery snapshot.
    pub counterfactual_household_unmet_industry_off_m3: f64,
    /// Difference between the matched industry-on and industry-off branches.
    pub counterfactual_industry_attributable_household_unmet_m3: f64,
    pub counterfactual_without_maintenance_household_unmet_m3: f64,
    pub mechanism_identified: bool,
    pub correction_latency_ticks: Option<usize>,
    pub trace_valid: bool,
    pub trace: Vec<CorrectionEvent>,
    pub maximum_water_residual_m3: f64,
    pub maximum_energy_residual_kwh: f64,
}

pub fn run_delayed_externality(
    controller: ControllerKind,
) -> Result<DelayedExternalityOutcome, SelfCorrectionError> {
    let mut world = delayed_world()?;
    let reserve_before_m3 = world.regions()[BASIN].opening_stock.water_m3;
    let mut reports = Vec::with_capacity(DELAYED_TOTAL_TICKS);

    // Shared calibration trajectory: every controller sees the same four
    // pre-shock ticks and the same post-growth world state.
    for _ in 0..GROWTH_TICKS {
        reports.push(delayed_step(
            &mut world,
            BASE_RECHARGE_M3_PER_TICK,
            true,
            false,
        )?);
    }
    let reserve_after_growth_m3 = world.regions()[BASIN].opening_stock.water_m3;
    let growth_snapshot = world.clone();

    // Matched recovery ablation: the maintenance action adds pump output only
    // when 5 kWh is delivered to the maintenance node. This is a declared
    // synthetic production relation; it is not a hydrological forecast.
    let mut no_maintenance_world = growth_snapshot.clone();
    let mut no_maintenance_reports = Vec::new();
    run_recovery_interval(
        &mut no_maintenance_world,
        false,
        &mut no_maintenance_reports,
    )?;
    let counterfactual_without_maintenance = simulate_repeated_shock(no_maintenance_world, false)?;

    let maintenance_enabled = controller == ControllerKind::MechanismAware;
    run_recovery_interval(&mut world, maintenance_enabled, &mut reports)?;
    let reserve_after_recovery_m3 = world.regions()[BASIN].opening_stock.water_m3;
    let post_recovery_snapshot = world.clone();

    // A paired, deterministic mechanism-discrimination test from one exact
    // state snapshot. Only the industrial water request changes.
    let industry_on = simulate_repeated_shock(post_recovery_snapshot.clone(), true)?;
    let industry_off = simulate_repeated_shock(post_recovery_snapshot.clone(), false)?;
    let counterfactual_industry_attributable_household_unmet_m3 =
        (industry_on.household_unmet_m3 - industry_off.household_unmet_m3).max(0.0);

    let mechanism_identified = controller == ControllerKind::MechanismAware
        && counterfactual_industry_attributable_household_unmet_m3 > 0.0
        && counterfactual_without_maintenance.household_unmet_m3 > 0.0;
    let proactive_curtailment =
        controller == ControllerKind::MechanismAware && mechanism_identified;

    let mut trace = Vec::new();
    let mut correction_latency_ticks = None;
    if proactive_curtailment {
        trace = vec![
            event(
                CorrectionStage::Observed,
                GROWTH_TICKS,
                "reserve_trend_below_declared_buffer",
            ),
            event(
                CorrectionStage::Detected,
                GROWTH_TICKS,
                "forecasted_household_floor_breach",
            ),
            event(
                CorrectionStage::Diagnosed,
                GROWTH_TICKS,
                "maintenance_and_competing_demand_mechanisms",
            ),
            event(
                CorrectionStage::Discriminated,
                GROWTH_TICKS,
                "matched_maintenance_and_industry_ablations",
            ),
            event(
                CorrectionStage::BeliefUpdated,
                GROWTH_TICKS,
                "industrial_withdrawal_consumes_essential_buffer",
            ),
            event(
                CorrectionStage::Intervened,
                GROWTH_TICKS + 1,
                "invest_in_pumping_then_curtail_industry_during_shock",
            ),
            event(
                CorrectionStage::Audited,
                DELAYED_TOTAL_TICKS,
                "household_floor_and_resource_ledger_replayed",
            ),
        ];
        validate_full_correction_trace(&trace)?;
        correction_latency_ticks = Some(1);
    }

    let mut last_household_coverage = 1.0_f64;
    let mut reactive_intervention_recorded = false;
    for shock_tick in 0..REPEATED_SHOCK_TICKS {
        let tick = GROWTH_TICKS + RECOVERY_TICKS + shock_tick + 1;
        let industry_enabled = match controller {
            ControllerKind::FrozenPolicy => true,
            ControllerKind::OutcomeReactive => {
                let enabled = last_household_coverage >= ESSENTIAL_COVERAGE_FLOOR;
                if !enabled && !reactive_intervention_recorded {
                    trace = vec![
                        event(
                            CorrectionStage::Observed,
                            tick - 1,
                            "household_service_floor_breached",
                        ),
                        event(
                            CorrectionStage::Detected,
                            tick - 1,
                            "reactive_service_threshold",
                        ),
                        event(
                            CorrectionStage::Intervened,
                            tick,
                            "curtail_industry_after_breach",
                        ),
                        event(
                            CorrectionStage::Audited,
                            DELAYED_TOTAL_TICKS,
                            "late_curtailment_replayed",
                        ),
                    ];
                    correction_latency_ticks = Some(1);
                    reactive_intervention_recorded = true;
                }
                enabled
            }
            ControllerKind::MechanismAware => !proactive_curtailment,
        };
        let report = delayed_step(&mut world, 0.0, industry_enabled, false)?;
        last_household_coverage = report.regions[HOUSEHOLD].coverage.water.unwrap_or(0.0);
        reports.push(report);
    }

    if reports.len() != DELAYED_TOTAL_TICKS {
        return Err(SelfCorrectionError::InvalidScenarioTrajectory {
            expected: DELAYED_TOTAL_TICKS,
            actual: reports.len(),
        });
    }
    let (maximum_water_residual_m3, maximum_energy_residual_kwh) = max_ledger_residuals(&reports);

    Ok(DelayedExternalityOutcome {
        controller,
        reserve_before_m3,
        reserve_after_growth_m3,
        reserve_after_recovery_m3,
        reserve_after_repeated_shock_m3: world.regions()[BASIN].opening_stock.water_m3,
        household_unmet_m3: sum_unmet_water(&reports, HOUSEHOLD),
        industry_unmet_m3: sum_unmet_water(&reports, INDUSTRY),
        minimum_household_coverage: minimum_coverage(&reports, HOUSEHOLD),
        maintenance_energy_kwh: sum_maintenance_energy_served(&reports),
        maintenance_energy_unmet_kwh: sum_maintenance_energy_unmet(&reports),
        counterfactual_household_unmet_industry_on_m3: industry_on.household_unmet_m3,
        counterfactual_household_unmet_industry_off_m3: industry_off.household_unmet_m3,
        counterfactual_industry_attributable_household_unmet_m3,
        counterfactual_without_maintenance_household_unmet_m3: counterfactual_without_maintenance
            .household_unmet_m3,
        mechanism_identified,
        correction_latency_ticks,
        trace_valid: !trace.is_empty() && validate_full_correction_trace(&trace).is_ok(),
        trace,
        maximum_water_residual_m3,
        maximum_energy_residual_kwh,
    })
}

pub fn run_delayed_externality_portfolio()
-> Result<Vec<DelayedExternalityOutcome>, SelfCorrectionError> {
    ControllerKind::ALL
        .into_iter()
        .map(run_delayed_externality)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sensor(controller: ControllerKind) -> SensorDriftOutcome {
        run_sensor_drift_sham(controller).expect("sensor drift fixture must run")
    }

    fn delayed(controller: ControllerKind) -> DelayedExternalityOutcome {
        run_delayed_externality(controller).expect("delayed externality fixture must run")
    }

    #[test]
    fn sensor_drift_sham_never_changes_true_household_service() {
        for controller in ControllerKind::ALL {
            let outcome = sensor(controller);
            assert_eq!(outcome.minimum_true_household_coverage, 1.0);
            assert_eq!(outcome.household_unmet_m3, 0.0);
            assert_eq!(outcome.minimum_reported_sensor_coverage, 0.6);
            assert_eq!(outcome.maximum_water_residual_m3, 0.0);
            assert_eq!(outcome.maximum_energy_residual_kwh, 0.0);
        }
    }

    #[test]
    fn outcome_reactive_sensor_following_creates_unnecessary_dispatch() {
        let outcome = sensor(ControllerKind::OutcomeReactive);
        assert_eq!(outcome.unnecessary_dispatch_m3, 12.0);
        assert_eq!(outcome.reserve_before_m3 - outcome.reserve_after_m3, 12.0);
        assert!(!outcome.mechanism_identified);
    }

    #[test]
    fn mechanism_aware_sensor_correction_avoids_physical_overreaction() {
        let outcome = sensor(ControllerKind::MechanismAware);
        assert_eq!(outcome.unnecessary_dispatch_m3, 0.0);
        assert_eq!(outcome.reserve_after_m3, outcome.reserve_before_m3);
        assert!(outcome.mechanism_identified);
        assert_eq!(outcome.correction_latency_ticks, Some(0));
        assert!(outcome.trace_valid);
        assert_eq!(outcome.trace.len(), CorrectionStage::ORDERED.len());
    }

    #[test]
    fn short_term_full_service_hides_the_delayed_reserve_externality() {
        let frozen = delayed(ControllerKind::FrozenPolicy);
        assert_eq!(frozen.reserve_before_m3, 80.0);
        assert_eq!(frozen.reserve_after_growth_m3, 60.0);
        assert_eq!(frozen.reserve_after_recovery_m3, 50.0);
        assert_eq!(frozen.minimum_household_coverage, 0.0);
        assert_eq!(frozen.household_unmet_m3, 25.0);
        assert_eq!(frozen.industry_unmet_m3, 15.0);
        assert_eq!(frozen.reserve_after_repeated_shock_m3, 0.0);
    }

    #[test]
    fn outcome_reactive_correction_after_service_failure_is_too_late() {
        let reactive = delayed(ControllerKind::OutcomeReactive);
        let frozen = delayed(ControllerKind::FrozenPolicy);
        assert_eq!(reactive.household_unmet_m3, frozen.household_unmet_m3);
        assert_eq!(reactive.household_unmet_m3, 25.0);
        assert!(!reactive.mechanism_identified);
        assert_eq!(reactive.minimum_household_coverage, 0.0);
    }

    #[test]
    fn mechanism_aware_controller_uses_maintenance_and_competing_demand_ablation() {
        let outcome = delayed(ControllerKind::MechanismAware);
        assert_eq!(outcome.reserve_after_recovery_m3, 70.0);
        assert_eq!(outcome.maintenance_energy_kwh, 10.0);
        assert_eq!(outcome.maintenance_energy_unmet_kwh, 50.0);
        assert_eq!(
            outcome.counterfactual_industry_attributable_household_unmet_m3,
            10.0
        );
        assert_eq!(
            outcome.counterfactual_without_maintenance_household_unmet_m3,
            10.0
        );
        assert!(outcome.mechanism_identified);
        assert!(outcome.trace_valid);
        assert_eq!(outcome.correction_latency_ticks, Some(1));
        assert_eq!(outcome.household_unmet_m3, 0.0);
        assert_eq!(outcome.minimum_household_coverage, 1.0);
        assert_eq!(outcome.reserve_after_repeated_shock_m3, 10.0);
        // The policy's household-service protection is not a free universal
        // improvement: industry receives less water and has more unmet demand.
        assert_eq!(outcome.industry_unmet_m3, 30.0);
    }

    #[test]
    fn ledger_conserves_every_resource_in_every_actual_scenario() {
        for outcome in run_sensor_drift_portfolio().expect("sensor portfolio") {
            assert_eq!(outcome.maximum_water_residual_m3, 0.0);
            assert_eq!(outcome.maximum_energy_residual_kwh, 0.0);
        }
        for outcome in run_delayed_externality_portfolio().expect("delayed portfolio") {
            assert_eq!(outcome.maximum_water_residual_m3, 0.0);
            assert_eq!(outcome.maximum_energy_residual_kwh, 0.0);
        }
    }

    #[test]
    fn paired_counterfactuals_share_the_same_post_recovery_snapshot() {
        let outcome = delayed(ControllerKind::MechanismAware);
        assert_eq!(
            outcome.counterfactual_industry_attributable_household_unmet_m3,
            10.0
        );
        assert_eq!(
            outcome.counterfactual_without_maintenance_household_unmet_m3,
            10.0
        );
    }

    #[test]
    fn correction_trace_rejects_missing_or_reordered_evidence() {
        let valid = vec![
            event(CorrectionStage::Observed, 1, "e1"),
            event(CorrectionStage::Detected, 1, "e2"),
            event(CorrectionStage::Diagnosed, 1, "e3"),
            event(CorrectionStage::Discriminated, 1, "e4"),
            event(CorrectionStage::BeliefUpdated, 1, "e5"),
            event(CorrectionStage::Intervened, 2, "e6"),
            event(CorrectionStage::Audited, 3, "e7"),
        ];
        assert!(validate_full_correction_trace(&valid).is_ok());

        let mut reordered = valid.clone();
        reordered.swap(2, 3);
        assert!(validate_full_correction_trace(&reordered).is_err());
        assert!(validate_full_correction_trace(&valid[..6]).is_err());
    }

    #[test]
    fn both_portfolios_are_deterministic_and_complete() {
        let sensor_first = run_sensor_drift_portfolio().expect("sensor portfolio");
        let sensor_second = run_sensor_drift_portfolio().expect("sensor portfolio");
        assert_eq!(sensor_first, sensor_second);
        assert_eq!(sensor_first.len(), ControllerKind::ALL.len());

        let delayed_first = run_delayed_externality_portfolio().expect("delayed portfolio");
        let delayed_second = run_delayed_externality_portfolio().expect("delayed portfolio");
        assert_eq!(delayed_first, delayed_second);
        assert_eq!(delayed_first.len(), ControllerKind::ALL.len());
    }
}
