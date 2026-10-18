use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const SCHEMA_VERSION: &str = "0.1";
const EVALUATOR_VERSION: &str = "0.1.0";
const PPM: u64 = 1_000_000;
const DESIGN_BASIS: &str = include_str!("../fixtures/design_basis.txt");
const DEFAULT_CASE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/lunar_seed_utility_case_v0_1.json"
);

type Check<T> = Result<T, String>;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    schema_version: String,
    case_id: String,
    evidence_class: String,
    claim_boundary: String,
    design_basis_sha256: String,
    interval_seconds: u64,
    battery: BatterySpec,
    resource_process: ResourceProcess,
    source_references: Vec<String>,
    intervals: Vec<IntervalInput>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BatterySpec {
    capacity_wh: u64,
    initial_state_of_charge_wh: u64,
    max_charge_input_wh_per_interval: u64,
    max_discharge_withdrawal_wh_per_interval: u64,
    charge_efficiency_ppm: u64,
    discharge_efficiency_ppm: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ResourceProcess {
    process_id: String,
    batch_energy_wh: u64,
    feed_mass_g_per_batch: u64,
    contained_water_fraction_ppm: u64,
    water_recovery_fraction_ppm: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct IntervalInput {
    id: String,
    solar_generation_wh: u64,
    firm_generation_wh: u64,
    critical_load_wh: u64,
    process_batch_feed_g: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Controller {
    GreedyImmediate,
    ForecastReserve,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StepReport {
    interval_id: String,
    generation_wh: u64,
    critical_demand_wh: u64,
    critical_generation_wh: u64,
    critical_battery_delivered_wh: u64,
    critical_served_wh: u64,
    critical_unserved_wh: u64,
    process_batch_available: bool,
    process_batch_ran: bool,
    process_defer_reason: Option<String>,
    process_energy_demand_wh: u64,
    process_energy_served_wh: u64,
    process_generation_wh: u64,
    process_battery_delivered_wh: u64,
    process_feed_available_g: u64,
    process_feed_processed_g: u64,
    contained_water_g: u64,
    recovered_water_g: u64,
    tailings_g: u64,
    unrecovered_water_g: u64,
    mass_balance_residual_g: u64,
    battery_soc_before_wh: u64,
    battery_energy_withdrawn_wh: u64,
    battery_energy_delivered_wh: u64,
    battery_discharge_loss_wh: u64,
    battery_charge_input_wh: u64,
    battery_energy_stored_wh: u64,
    battery_charge_loss_wh: u64,
    curtailed_generation_wh: u64,
    battery_soc_after_wh: u64,
    future_critical_reserve_wh: Option<u64>,
    energy_balance_residual_wh: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RunReport {
    controller: Controller,
    interval_count: usize,
    total_generation_wh: u64,
    total_critical_demand_wh: u64,
    total_critical_served_wh: u64,
    total_critical_unserved_wh: u64,
    critical_unserved_intervals: usize,
    process_batches_available: usize,
    process_batches_run: usize,
    process_batches_deferred: usize,
    total_process_feed_processed_g: u64,
    total_process_feed_unprocessed_g: u64,
    total_recovered_water_g: u64,
    total_energy_balance_residual_wh: u64,
    max_energy_balance_residual_wh: u64,
    total_mass_balance_residual_g: u64,
    all_accounting_balanced: bool,
    steps: Vec<StepReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Comparison {
    critical_service_floor_met_by_greedy: bool,
    critical_service_floor_met_by_forecast_reserve: bool,
    forecast_reserve_reduces_critical_unserved_wh: bool,
    greedy_process_batches_run: usize,
    forecast_reserve_process_batches_run: usize,
    critical_energy_unserved_wh_greedy: u64,
    critical_energy_unserved_wh_forecast_reserve: u64,
    interpretation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkReceipt {
    receipt_schema_version: String,
    evaluator_version: String,
    source_commit: Option<String>,
    scenario_id: String,
    evidence_class: String,
    claim_boundary: String,
    input_sha256: String,
    design_basis_sha256: String,
    source_references: Vec<String>,
    greedy: RunReport,
    forecast_reserve: RunReport,
    comparison: Comparison,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn checked_add(a: u64, b: u64, at: &str) -> Check<u64> {
    a.checked_add(b)
        .ok_or_else(|| format!("{at}: integer overflow while adding {a} and {b}"))
}

fn mul_div_floor(a: u64, b: u64, divisor: u64, at: &str) -> Check<u64> {
    if divisor == 0 {
        return Err(format!("{at}: division by zero"));
    }
    let product = a
        .checked_mul(b)
        .ok_or_else(|| format!("{at}: integer overflow in fixed-point multiplication"))?;
    Ok(product / divisor)
}

fn mul_div_ceil(a: u64, b: u64, divisor: u64, at: &str) -> Check<u64> {
    if divisor == 0 {
        return Err(format!("{at}: division by zero"));
    }
    let product = a
        .checked_mul(b)
        .ok_or_else(|| format!("{at}: integer overflow in fixed-point multiplication"))?;
    let quotient = product / divisor;
    let remainder = product % divisor;
    if remainder == 0 {
        Ok(quotient)
    } else {
        quotient
            .checked_add(1)
            .ok_or_else(|| format!("{at}: integer overflow during ceiling division"))
    }
}

fn generation_wh(interval: &IntervalInput) -> Check<u64> {
    checked_add(
        interval.solar_generation_wh,
        interval.firm_generation_wh,
        &format!("{}.generation_wh", interval.id),
    )
}

fn validate_scenario(scenario: &Scenario, actual_design_basis_sha256: &str) -> Check<()> {
    if scenario.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "unsupported schema version: {}",
            scenario.schema_version
        ));
    }
    if scenario.evidence_class != "synthetic" {
        return Err("this fixture runner accepts only evidence_class=synthetic".to_owned());
    }
    for (name, value) in [
        ("case_id", scenario.case_id.as_str()),
        ("claim_boundary", scenario.claim_boundary.as_str()),
        (
            "resource_process.process_id",
            scenario.resource_process.process_id.as_str(),
        ),
    ] {
        if value.trim().is_empty() {
            return Err(format!("{name} must not be blank"));
        }
    }
    if scenario.design_basis_sha256 != actual_design_basis_sha256 {
        return Err(format!(
            "design-basis digest mismatch: fixture declares {}, actual is {}",
            scenario.design_basis_sha256, actual_design_basis_sha256
        ));
    }
    if scenario.design_basis_sha256.len() != 64
        || !scenario
            .design_basis_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("design_basis_sha256 must be 64 hexadecimal characters".to_owned());
    }
    if scenario.interval_seconds == 0 {
        return Err("interval_seconds must be positive".to_owned());
    }
    if scenario.source_references.is_empty()
        || scenario
            .source_references
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err("source references must be non-empty strings".to_owned());
    }
    if scenario.intervals.len() < 2 {
        return Err("at least two intervals are required to test reserve behavior".to_owned());
    }

    let battery = &scenario.battery;
    if battery.capacity_wh == 0
        || battery.initial_state_of_charge_wh > battery.capacity_wh
        || battery.max_charge_input_wh_per_interval == 0
        || battery.max_discharge_withdrawal_wh_per_interval == 0
    {
        return Err("battery capacity/rates must be positive and initial SOC must fit".to_owned());
    }
    for (name, efficiency) in [
        ("charge_efficiency_ppm", battery.charge_efficiency_ppm),
        ("discharge_efficiency_ppm", battery.discharge_efficiency_ppm),
    ] {
        if efficiency == 0 || efficiency > PPM {
            return Err(format!(
                "battery.{name} must be in the inclusive range 1..={PPM}"
            ));
        }
    }

    let process = &scenario.resource_process;
    if process.batch_energy_wh == 0 || process.feed_mass_g_per_batch == 0 {
        return Err("process batch energy and feed mass must be positive".to_owned());
    }
    for (name, fraction) in [
        (
            "contained_water_fraction_ppm",
            process.contained_water_fraction_ppm,
        ),
        (
            "water_recovery_fraction_ppm",
            process.water_recovery_fraction_ppm,
        ),
    ] {
        if fraction > PPM {
            return Err(format!("resource_process.{name} must not exceed {PPM}"));
        }
    }

    let mut interval_ids = HashSet::new();
    for interval in &scenario.intervals {
        if interval.id.trim().is_empty() {
            return Err("interval IDs must not be blank".to_owned());
        }
        if !interval_ids.insert(interval.id.as_str()) {
            return Err(format!("duplicate interval ID: {}", interval.id));
        }
        generation_wh(interval)?;
        if interval.process_batch_feed_g != 0
            && interval.process_batch_feed_g != process.feed_mass_g_per_batch
        {
            return Err(format!(
                "{}: process feed must be zero or exactly one declared batch ({} g)",
                interval.id, process.feed_mass_g_per_batch
            ));
        }
        checked_add(
            interval.critical_load_wh,
            if interval.process_batch_feed_g > 0 {
                process.batch_energy_wh
            } else {
                0
            },
            &format!("{}.potential_load_wh", interval.id),
        )?;
    }
    Ok(())
}

/// Returns the energy that must be present in the battery at the start of a
/// future interval sequence to cover its critical loads, assuming all
/// deferrable production is switched off and declared generation occurs.
fn future_critical_reserve_wh(scenario: &Scenario, start_index: usize) -> Check<Option<u64>> {
    let battery = &scenario.battery;
    let max_deliverable_wh = mul_div_floor(
        battery
            .capacity_wh
            .min(battery.max_discharge_withdrawal_wh_per_interval),
        battery.discharge_efficiency_ppm,
        PPM,
        "future reserve maximum discharge",
    )?;
    let mut reserve_wh = 0_u64;

    for interval in scenario.intervals[start_index..].iter().rev() {
        let generation = generation_wh(interval)?;
        if generation < interval.critical_load_wh {
            let deficit_wh = interval.critical_load_wh - generation;
            if deficit_wh > max_deliverable_wh {
                return Ok(None);
            }
            let required_withdrawal_wh = mul_div_ceil(
                deficit_wh,
                PPM,
                battery.discharge_efficiency_ppm,
                "future reserve discharge requirement",
            )?;
            reserve_wh = match reserve_wh.checked_add(required_withdrawal_wh) {
                Some(value) if value <= battery.capacity_wh => value,
                _ => return Ok(None),
            };
        } else {
            let surplus_wh = generation - interval.critical_load_wh;
            let charge_input_wh = surplus_wh.min(battery.max_charge_input_wh_per_interval);
            let stored_wh = mul_div_floor(
                charge_input_wh,
                battery.charge_efficiency_ppm,
                PPM,
                "future reserve charge gain",
            )?;
            reserve_wh = reserve_wh.saturating_sub(stored_wh);
        }
    }
    Ok(Some(reserve_wh))
}

/// Withdraw from stored energy to supply a requested load. The result is
/// (delivered Wh, withdrawn Wh); stored-energy conversion loss remains explicit.
fn draw_battery(
    target_delivered_wh: u64,
    state_of_charge_wh: u64,
    max_withdrawal_remaining_wh: u64,
    discharge_efficiency_ppm: u64,
    at: &str,
) -> Check<(u64, u64)> {
    if target_delivered_wh == 0 {
        return Ok((0, 0));
    }
    let available_withdrawal_wh = state_of_charge_wh.min(max_withdrawal_remaining_wh);
    let available_delivered_wh =
        mul_div_floor(available_withdrawal_wh, discharge_efficiency_ppm, PPM, at)?;
    if available_delivered_wh >= target_delivered_wh {
        let withdrawal_wh = mul_div_ceil(target_delivered_wh, PPM, discharge_efficiency_ppm, at)?
            .min(available_withdrawal_wh);
        let delivered_wh = mul_div_floor(withdrawal_wh, discharge_efficiency_ppm, PPM, at)?
            .min(target_delivered_wh);
        if delivered_wh < target_delivered_wh {
            return Err(format!(
                "{at}: fixed-point withdrawal unexpectedly under-delivered"
            ));
        }
        Ok((delivered_wh, withdrawal_wh))
    } else {
        Ok((available_delivered_wh, available_withdrawal_wh))
    }
}

fn run_controller(scenario: &Scenario, controller: Controller) -> Check<RunReport> {
    let battery = &scenario.battery;
    let process = &scenario.resource_process;
    let mut state_of_charge_wh = battery.initial_state_of_charge_wh;
    let mut steps = Vec::with_capacity(scenario.intervals.len());

    for (index, interval) in scenario.intervals.iter().enumerate() {
        let generation = generation_wh(interval)?;
        let soc_before = state_of_charge_wh;
        let reserve_after = future_critical_reserve_wh(scenario, index + 1)?;

        // 1. Serve the critical load from generation first, then the battery.
        let critical_generation_wh = generation.min(interval.critical_load_wh);
        let critical_deficit_after_generation_wh =
            interval.critical_load_wh - critical_generation_wh;
        let (critical_battery_delivered_wh, critical_battery_withdrawn_wh) = draw_battery(
            critical_deficit_after_generation_wh,
            state_of_charge_wh,
            battery.max_discharge_withdrawal_wh_per_interval,
            battery.discharge_efficiency_ppm,
            &format!("{}.critical_discharge", interval.id),
        )?;
        state_of_charge_wh = state_of_charge_wh
            .checked_sub(critical_battery_withdrawn_wh)
            .ok_or_else(|| format!("{}: battery SOC underflow after critical load", interval.id))?;
        let critical_served_wh = checked_add(
            critical_generation_wh,
            critical_battery_delivered_wh,
            &format!("{}.critical_served", interval.id),
        )?
        .min(interval.critical_load_wh);
        let critical_unserved_wh = interval.critical_load_wh - critical_served_wh;

        let mut battery_energy_withdrawn_wh = critical_battery_withdrawn_wh;
        let mut battery_energy_delivered_wh = critical_battery_delivered_wh;
        let mut battery_discharge_loss_wh =
            critical_battery_withdrawn_wh - critical_battery_delivered_wh;
        let generation_surplus_after_critical_wh = generation - critical_generation_wh;

        // 2. Production is an indivisible batch. It runs only when its full
        //    declared energy is available after all critical service is met.
        let process_batch_available = interval.process_batch_feed_g > 0;
        let mut process_batch_ran = false;
        let mut process_defer_reason = None;
        let mut process_energy_served_wh = 0;
        let mut process_generation_wh = 0;
        let mut process_battery_delivered_wh = 0;

        if process_batch_available {
            if critical_unserved_wh > 0 {
                process_defer_reason = Some("critical_service_not_fully_met".to_owned());
            } else {
                let generation_available_for_process =
                    generation_surplus_after_critical_wh.min(process.batch_energy_wh);
                let process_battery_need_wh =
                    process.batch_energy_wh - generation_available_for_process;
                let max_withdrawal_remaining_wh = battery
                    .max_discharge_withdrawal_wh_per_interval
                    .saturating_sub(battery_energy_withdrawn_wh);
                let battery_deliverable_remaining_wh = mul_div_floor(
                    state_of_charge_wh.min(max_withdrawal_remaining_wh),
                    battery.discharge_efficiency_ppm,
                    PPM,
                    &format!("{}.process_available_battery", interval.id),
                )?;
                let process_power_available_wh = checked_add(
                    generation_available_for_process,
                    battery_deliverable_remaining_wh,
                    &format!("{}.process_power_available", interval.id),
                )?;
                if process_power_available_wh < process.batch_energy_wh {
                    process_defer_reason = Some("insufficient_available_power".to_owned());
                } else {
                    let required_process_withdrawal_wh = mul_div_ceil(
                        process_battery_need_wh,
                        PPM,
                        battery.discharge_efficiency_ppm,
                        &format!("{}.process_battery_requirement", interval.id),
                    )?;
                    let soc_after_process_if_run_wh = state_of_charge_wh
                        .checked_sub(required_process_withdrawal_wh)
                        .ok_or_else(|| {
                            format!("{}: process reserve calculation underflow", interval.id)
                        })?;

                    let allowed_by_controller = match controller {
                        Controller::GreedyImmediate => true,
                        Controller::ForecastReserve => match reserve_after {
                            Some(required_reserve_wh) => {
                                soc_after_process_if_run_wh >= required_reserve_wh
                            }
                            None => false,
                        },
                    };

                    if allowed_by_controller {
                        process_batch_ran = true;
                        process_energy_served_wh = process.batch_energy_wh;
                        process_generation_wh = generation_available_for_process;
                        let (delivered_wh, withdrawal_wh) = draw_battery(
                            process_battery_need_wh,
                            state_of_charge_wh,
                            max_withdrawal_remaining_wh,
                            battery.discharge_efficiency_ppm,
                            &format!("{}.process_discharge", interval.id),
                        )?;
                        if delivered_wh != process_battery_need_wh {
                            return Err(format!(
                                "{}: process batch admitted but battery under-delivered",
                                interval.id
                            ));
                        }
                        process_battery_delivered_wh = delivered_wh;
                        state_of_charge_wh = state_of_charge_wh
                            .checked_sub(withdrawal_wh)
                            .ok_or_else(|| {
                                format!(
                                    "{}: battery SOC underflow after process batch",
                                    interval.id
                                )
                            })?;
                        battery_energy_withdrawn_wh = checked_add(
                            battery_energy_withdrawn_wh,
                            withdrawal_wh,
                            &format!("{}.battery_withdrawn", interval.id),
                        )?;
                        battery_energy_delivered_wh = checked_add(
                            battery_energy_delivered_wh,
                            delivered_wh,
                            &format!("{}.battery_delivered", interval.id),
                        )?;
                        battery_discharge_loss_wh = checked_add(
                            battery_discharge_loss_wh,
                            withdrawal_wh - delivered_wh,
                            &format!("{}.battery_discharge_loss", interval.id),
                        )?;
                    } else {
                        process_defer_reason = Some(match reserve_after {
                            Some(_) => "preserve_future_critical_reserve".to_owned(),
                            None => "future_critical_floor_infeasible".to_owned(),
                        });
                    }
                }
            }
        }

        let generation_used_for_load_wh = checked_add(
            critical_generation_wh,
            process_generation_wh,
            &format!("{}.generation_used_for_load", interval.id),
        )?;
        let generation_surplus_wh = generation - generation_used_for_load_wh;

        // 3. Charge only from actual surplus, bounded by charge power and store headroom.
        let charge_headroom_wh = battery.capacity_wh - state_of_charge_wh;
        let max_input_for_headroom_wh = mul_div_ceil(
            charge_headroom_wh,
            PPM,
            battery.charge_efficiency_ppm,
            &format!("{}.charge_headroom", interval.id),
        )?;
        let battery_charge_input_wh = generation_surplus_wh
            .min(battery.max_charge_input_wh_per_interval)
            .min(max_input_for_headroom_wh);
        let battery_energy_stored_wh = mul_div_floor(
            battery_charge_input_wh,
            battery.charge_efficiency_ppm,
            PPM,
            &format!("{}.charge_efficiency", interval.id),
        )?;
        let battery_charge_loss_wh = battery_charge_input_wh - battery_energy_stored_wh;
        state_of_charge_wh = checked_add(
            state_of_charge_wh,
            battery_energy_stored_wh,
            &format!("{}.soc_after_charge", interval.id),
        )?;
        if state_of_charge_wh > battery.capacity_wh {
            return Err(format!("{}: battery capacity exceeded", interval.id));
        }
        let curtailed_generation_wh = generation_surplus_wh - battery_charge_input_wh;

        // 4. Process mass is accounted only for a fully completed batch.
        let process_feed_processed_g = if process_batch_ran {
            interval.process_batch_feed_g
        } else {
            0
        };
        let contained_water_g = mul_div_floor(
            process_feed_processed_g,
            process.contained_water_fraction_ppm,
            PPM,
            &format!("{}.contained_water", interval.id),
        )?;
        let recovered_water_g = mul_div_floor(
            contained_water_g,
            process.water_recovery_fraction_ppm,
            PPM,
            &format!("{}.recovered_water", interval.id),
        )?;
        let tailings_g = process_feed_processed_g - contained_water_g;
        let unrecovered_water_g = contained_water_g - recovered_water_g;
        let mass_accounted_g = checked_add(
            checked_add(
                tailings_g,
                recovered_water_g,
                &format!("{}.mass_balance_a", interval.id),
            )?,
            unrecovered_water_g,
            &format!("{}.mass_balance_b", interval.id),
        )?;
        let mass_balance_residual_g = process_feed_processed_g.abs_diff(mass_accounted_g);

        // Independent balance form includes all source energy, loads, final
        // battery energy, curtailment, and charge/discharge losses.
        let source_energy_wh = checked_add(
            generation,
            soc_before,
            &format!("{}.energy_source_balance", interval.id),
        )?;
        let delivered_loads_wh = checked_add(
            critical_served_wh,
            process_energy_served_wh,
            &format!("{}.delivered_load_balance", interval.id),
        )?;
        let energy_accounted_wh = [
            delivered_loads_wh,
            state_of_charge_wh,
            curtailed_generation_wh,
            battery_charge_loss_wh,
            battery_discharge_loss_wh,
        ]
        .into_iter()
        .try_fold(0_u64, |sum, term| {
            checked_add(sum, term, &format!("{}.energy_balance_sum", interval.id))
        })?;
        let energy_balance_residual_wh = source_energy_wh.abs_diff(energy_accounted_wh);

        steps.push(StepReport {
            interval_id: interval.id.clone(),
            generation_wh: generation,
            critical_demand_wh: interval.critical_load_wh,
            critical_generation_wh,
            critical_battery_delivered_wh,
            critical_served_wh,
            critical_unserved_wh,
            process_batch_available,
            process_batch_ran,
            process_defer_reason,
            process_energy_demand_wh: if process_batch_available {
                process.batch_energy_wh
            } else {
                0
            },
            process_energy_served_wh,
            process_generation_wh,
            process_battery_delivered_wh,
            process_feed_available_g: interval.process_batch_feed_g,
            process_feed_processed_g,
            contained_water_g,
            recovered_water_g,
            tailings_g,
            unrecovered_water_g,
            mass_balance_residual_g,
            battery_soc_before_wh: soc_before,
            battery_energy_withdrawn_wh,
            battery_energy_delivered_wh,
            battery_discharge_loss_wh,
            battery_charge_input_wh,
            battery_energy_stored_wh,
            battery_charge_loss_wh,
            curtailed_generation_wh,
            battery_soc_after_wh: state_of_charge_wh,
            future_critical_reserve_wh: reserve_after,
            energy_balance_residual_wh,
        });
    }

    let total_generation_wh = sum_checked(
        steps.iter().map(|step| step.generation_wh),
        "total generation",
    )?;
    let total_critical_demand_wh = sum_checked(
        steps.iter().map(|step| step.critical_demand_wh),
        "total critical demand",
    )?;
    let total_critical_served_wh = sum_checked(
        steps.iter().map(|step| step.critical_served_wh),
        "total critical served",
    )?;
    let total_critical_unserved_wh = sum_checked(
        steps.iter().map(|step| step.critical_unserved_wh),
        "total critical unserved",
    )?;
    let critical_unserved_intervals = steps
        .iter()
        .filter(|step| step.critical_unserved_wh > 0)
        .count();
    let process_batches_available = steps
        .iter()
        .filter(|step| step.process_batch_available)
        .count();
    let process_batches_run = steps.iter().filter(|step| step.process_batch_ran).count();
    let process_batches_deferred = process_batches_available - process_batches_run;
    let total_process_feed_processed_g = sum_checked(
        steps.iter().map(|step| step.process_feed_processed_g),
        "total process feed processed",
    )?;
    let total_process_feed_available_g = sum_checked(
        steps.iter().map(|step| step.process_feed_available_g),
        "total process feed available",
    )?;
    let total_recovered_water_g = sum_checked(
        steps.iter().map(|step| step.recovered_water_g),
        "total recovered water",
    )?;
    let total_energy_balance_residual_wh = sum_checked(
        steps.iter().map(|step| step.energy_balance_residual_wh),
        "total energy residual",
    )?;
    let max_energy_balance_residual_wh = steps
        .iter()
        .map(|step| step.energy_balance_residual_wh)
        .max()
        .unwrap_or(0);
    let total_mass_balance_residual_g = sum_checked(
        steps.iter().map(|step| step.mass_balance_residual_g),
        "total mass residual",
    )?;

    Ok(RunReport {
        controller,
        interval_count: steps.len(),
        total_generation_wh,
        total_critical_demand_wh,
        total_critical_served_wh,
        total_critical_unserved_wh,
        critical_unserved_intervals,
        process_batches_available,
        process_batches_run,
        process_batches_deferred,
        total_process_feed_processed_g,
        total_process_feed_unprocessed_g: total_process_feed_available_g
            - total_process_feed_processed_g,
        total_recovered_water_g,
        total_energy_balance_residual_wh,
        max_energy_balance_residual_wh,
        total_mass_balance_residual_g,
        all_accounting_balanced: total_energy_balance_residual_wh == 0
            && total_mass_balance_residual_g == 0,
        steps,
    })
}

fn sum_checked(mut values: impl Iterator<Item = u64>, at: &str) -> Check<u64> {
    values.try_fold(0_u64, |sum, value| checked_add(sum, value, at))
}

/// This verifier recomputes interval energy balances, battery transitions,
/// charge/discharge losses, mass balances and aggregates from the emitted trace.
/// It does not trust the trace's residual or summary fields.
fn verify_run(scenario: &Scenario, report: &RunReport) -> Check<()> {
    if report.steps.len() != scenario.intervals.len()
        || report.interval_count != scenario.intervals.len()
    {
        return Err("verifier: interval count does not match scenario".to_owned());
    }

    let mut totals = BTreeTotals::default();
    let mut expected_soc_before_wh = scenario.battery.initial_state_of_charge_wh;
    for (index, (input, step)) in scenario
        .intervals
        .iter()
        .zip(report.steps.iter())
        .enumerate()
    {
        if step.battery_soc_before_wh != expected_soc_before_wh {
            return Err(format!(
                "verifier {}: SOC does not continue from prior interval",
                input.id
            ));
        }
        if step.interval_id != input.id {
            return Err(format!(
                "verifier: interval identity mismatch at index {index}"
            ));
        }
        let generation = generation_wh(input)?;
        if step.generation_wh != generation {
            return Err(format!("verifier {}: generation value mismatch", input.id));
        }
        if step.critical_demand_wh != input.critical_load_wh {
            return Err(format!("verifier {}: critical demand mismatch", input.id));
        }
        if step.process_feed_available_g != input.process_batch_feed_g {
            return Err(format!("verifier {}: batch feed mismatch", input.id));
        }
        if step.process_energy_demand_wh
            != if input.process_batch_feed_g > 0 {
                scenario.resource_process.batch_energy_wh
            } else {
                0
            }
        {
            return Err(format!(
                "verifier {}: batch energy demand mismatch",
                input.id
            ));
        }
        let critical_reconciled_wh = checked_add(
            step.critical_served_wh,
            step.critical_unserved_wh,
            &format!("verifier {}.critical_reconciliation", input.id),
        )?;
        if critical_reconciled_wh != step.critical_demand_wh
            || step.critical_served_wh > step.critical_demand_wh
        {
            return Err(format!(
                "verifier {}: critical demand does not reconcile",
                input.id
            ));
        }
        if step.critical_generation_wh != generation.min(input.critical_load_wh) {
            return Err(format!(
                "verifier {}: critical generation priority mismatch",
                input.id
            ));
        }
        let critical_source_split_wh = checked_add(
            step.critical_generation_wh,
            step.critical_battery_delivered_wh,
            &format!("verifier {}.critical_source_split", input.id),
        )?;
        if step.critical_served_wh != critical_source_split_wh {
            return Err(format!(
                "verifier {}: critical load source split mismatch",
                input.id
            ));
        }
        if step.process_batch_ran {
            if !step.process_batch_available
                || step.process_energy_served_wh != scenario.resource_process.batch_energy_wh
                || step.process_feed_processed_g != input.process_batch_feed_g
            {
                return Err(format!(
                    "verifier {}: incomplete batch credited as complete",
                    input.id
                ));
            }
        } else if step.process_energy_served_wh != 0
            || step.process_feed_processed_g != 0
            || step.contained_water_g != 0
            || step.recovered_water_g != 0
            || step.tailings_g != 0
            || step.unrecovered_water_g != 0
            || step.process_generation_wh != 0
            || step.process_battery_delivered_wh != 0
        {
            return Err(format!(
                "verifier {}: deferred batch consumed energy or feed",
                input.id
            ));
        }
        if step.process_batch_ran {
            let process_source_split_wh = checked_add(
                step.process_generation_wh,
                step.process_battery_delivered_wh,
                &format!("verifier {}.process_source_split", input.id),
            )?;
            if step.process_energy_served_wh != process_source_split_wh {
                return Err(format!(
                    "verifier {}: process energy sources do not reconcile",
                    input.id
                ));
            }
        }

        let generation_allocated_wh = checked_add(
            step.process_generation_wh,
            step.critical_generation_wh,
            &format!("verifier {}.generation_allocation", input.id),
        )?;
        if generation_allocated_wh > generation {
            return Err(format!(
                "verifier {}: generation allocated more than once",
                input.id
            ));
        }
        let expected_reserve = future_critical_reserve_wh(scenario, index + 1)?;
        if step.future_critical_reserve_wh != expected_reserve {
            return Err(format!(
                "verifier {}: future reserve claim mismatch",
                input.id
            ));
        }

        let battery = &scenario.battery;
        let expected_soc_after = step
            .battery_soc_before_wh
            .checked_sub(step.battery_energy_withdrawn_wh)
            .and_then(|value| value.checked_add(step.battery_energy_stored_wh))
            .ok_or_else(|| format!("verifier {}: invalid battery state transition", input.id))?;
        if expected_soc_after != step.battery_soc_after_wh
            || step.battery_soc_after_wh > battery.capacity_wh
            || step.battery_soc_before_wh > battery.capacity_wh
        {
            return Err(format!(
                "verifier {}: battery state transition invalid",
                input.id
            ));
        }
        expected_soc_before_wh = step.battery_soc_after_wh;
        if step.battery_energy_withdrawn_wh > battery.max_discharge_withdrawal_wh_per_interval
            || step.battery_charge_input_wh > battery.max_charge_input_wh_per_interval
        {
            return Err(format!(
                "verifier {}: battery power limit exceeded",
                input.id
            ));
        }
        if step.battery_energy_withdrawn_wh > step.battery_soc_before_wh {
            return Err(format!(
                "verifier {}: battery withdrawal exceeds SOC",
                input.id
            ));
        }
        let physical_discharge_output = mul_div_floor(
            step.battery_energy_withdrawn_wh,
            battery.discharge_efficiency_ppm,
            PPM,
            &format!("verifier {}.discharge_efficiency", input.id),
        )?;
        let expected_discharge_loss = step
            .battery_energy_withdrawn_wh
            .checked_sub(step.battery_energy_delivered_wh)
            .ok_or_else(|| {
                format!(
                    "verifier {}: battery delivered more than withdrawn",
                    input.id
                )
            })?;
        if step.battery_energy_delivered_wh > physical_discharge_output
            || step.battery_discharge_loss_wh != expected_discharge_loss
        {
            return Err(format!(
                "verifier {}: discharge accounting invalid",
                input.id
            ));
        }
        let allocated_battery_delivery_wh = checked_add(
            step.critical_battery_delivered_wh,
            step.process_battery_delivered_wh,
            &format!("verifier {}.battery_delivery_allocation", input.id),
        )?;
        if step.battery_energy_delivered_wh != allocated_battery_delivery_wh {
            return Err(format!(
                "verifier {}: battery delivery allocation mismatch",
                input.id
            ));
        }
        let physical_charge_storage = mul_div_floor(
            step.battery_charge_input_wh,
            battery.charge_efficiency_ppm,
            PPM,
            &format!("verifier {}.charge_efficiency", input.id),
        )?;
        let expected_charge_loss = step
            .battery_charge_input_wh
            .checked_sub(step.battery_energy_stored_wh)
            .ok_or_else(|| format!("verifier {}: stored energy exceeds charge input", input.id))?;
        if step.battery_energy_stored_wh != physical_charge_storage
            || step.battery_charge_loss_wh != expected_charge_loss
        {
            return Err(format!("verifier {}: charge accounting invalid", input.id));
        }
        let generation_surplus_wh =
            generation
                .checked_sub(generation_allocated_wh)
                .ok_or_else(|| {
                    format!(
                        "verifier {}: generation load allocation exceeds source",
                        input.id
                    )
                })?;
        let accounted_generation_surplus_wh = checked_add(
            step.curtailed_generation_wh,
            step.battery_charge_input_wh,
            &format!("verifier {}.generation_surplus", input.id),
        )?;
        if accounted_generation_surplus_wh != generation_surplus_wh {
            return Err(format!(
                "verifier {}: generation surplus does not reconcile",
                input.id
            ));
        }

        let contained = mul_div_floor(
            step.process_feed_processed_g,
            scenario.resource_process.contained_water_fraction_ppm,
            PPM,
            &format!("verifier {}.contained_water", input.id),
        )?;
        let recovered = mul_div_floor(
            contained,
            scenario.resource_process.water_recovery_fraction_ppm,
            PPM,
            &format!("verifier {}.recovered_water", input.id),
        )?;
        let tailings = step.process_feed_processed_g - contained;
        let unrecovered = contained - recovered;
        if (
            step.contained_water_g,
            step.recovered_water_g,
            step.tailings_g,
            step.unrecovered_water_g,
        ) != (contained, recovered, tailings, unrecovered)
        {
            return Err(format!(
                "verifier {}: resource transformation mismatch",
                input.id
            ));
        }
        let mass_accounted = checked_add(
            checked_add(tailings, recovered, "verifier mass balance")?,
            unrecovered,
            "verifier mass balance",
        )?;
        let mass_residual = step.process_feed_processed_g.abs_diff(mass_accounted);
        if mass_residual != step.mass_balance_residual_g {
            return Err(format!(
                "verifier {}: reported mass residual was tampered",
                input.id
            ));
        }

        let source_energy = checked_add(
            generation,
            step.battery_soc_before_wh,
            "verifier energy balance",
        )?;
        let delivered_load = checked_add(
            step.critical_served_wh,
            step.process_energy_served_wh,
            "verifier energy balance",
        )?;
        let energy_accounted = [
            delivered_load,
            step.battery_soc_after_wh,
            step.curtailed_generation_wh,
            step.battery_charge_loss_wh,
            step.battery_discharge_loss_wh,
        ]
        .into_iter()
        .try_fold(0_u64, |sum, value| {
            checked_add(sum, value, "verifier energy balance")
        })?;
        let energy_residual = source_energy.abs_diff(energy_accounted);
        if energy_residual != step.energy_balance_residual_wh {
            return Err(format!(
                "verifier {}: reported energy residual was tampered",
                input.id
            ));
        }

        totals.generation_wh = checked_add(totals.generation_wh, generation, "verifier totals")?;
        totals.critical_demand_wh = checked_add(
            totals.critical_demand_wh,
            step.critical_demand_wh,
            "verifier totals",
        )?;
        totals.critical_served_wh = checked_add(
            totals.critical_served_wh,
            step.critical_served_wh,
            "verifier totals",
        )?;
        totals.critical_unserved_wh = checked_add(
            totals.critical_unserved_wh,
            step.critical_unserved_wh,
            "verifier totals",
        )?;
        totals.batches_available += if step.process_batch_available { 1 } else { 0 };
        totals.batches_run += if step.process_batch_ran { 1 } else { 0 };
        totals.feed_processed_g = checked_add(
            totals.feed_processed_g,
            step.process_feed_processed_g,
            "verifier totals",
        )?;
        totals.feed_available_g = checked_add(
            totals.feed_available_g,
            step.process_feed_available_g,
            "verifier totals",
        )?;
        totals.recovered_water_g =
            checked_add(totals.recovered_water_g, recovered, "verifier totals")?;
        totals.energy_residual_wh = checked_add(
            totals.energy_residual_wh,
            energy_residual,
            "verifier totals",
        )?;
        totals.max_energy_residual_wh = totals.max_energy_residual_wh.max(energy_residual);
        totals.mass_residual_g =
            checked_add(totals.mass_residual_g, mass_residual, "verifier totals")?;
    }

    if report.total_generation_wh != totals.generation_wh
        || report.total_critical_demand_wh != totals.critical_demand_wh
        || report.total_critical_served_wh != totals.critical_served_wh
        || report.total_critical_unserved_wh != totals.critical_unserved_wh
        || report.critical_unserved_intervals
            != report
                .steps
                .iter()
                .filter(|step| step.critical_unserved_wh > 0)
                .count()
        || report.process_batches_available != totals.batches_available
        || report.process_batches_run != totals.batches_run
        || report.process_batches_deferred != totals.batches_available - totals.batches_run
        || report.total_process_feed_processed_g != totals.feed_processed_g
        || report.total_process_feed_unprocessed_g
            != totals.feed_available_g - totals.feed_processed_g
        || report.total_recovered_water_g != totals.recovered_water_g
        || report.total_energy_balance_residual_wh != totals.energy_residual_wh
        || report.max_energy_balance_residual_wh != totals.max_energy_residual_wh
        || report.total_mass_balance_residual_g != totals.mass_residual_g
        || report.all_accounting_balanced
            != (totals.energy_residual_wh == 0 && totals.mass_residual_g == 0)
    {
        return Err(
            "verifier: aggregate summary does not match independently recomputed trace".to_owned(),
        );
    }
    Ok(())
}

#[derive(Default)]
struct BTreeTotals {
    generation_wh: u64,
    critical_demand_wh: u64,
    critical_served_wh: u64,
    critical_unserved_wh: u64,
    batches_available: usize,
    batches_run: usize,
    feed_processed_g: u64,
    feed_available_g: u64,
    recovered_water_g: u64,
    energy_residual_wh: u64,
    max_energy_residual_wh: u64,
    mass_residual_g: u64,
}

fn make_receipt(
    scenario: &Scenario,
    input_sha256: String,
    source_commit: Option<String>,
) -> Check<BenchmarkReceipt> {
    if let Some(commit) = &source_commit {
        if commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(
                "--source-commit must be a full 40-character hexadecimal commit SHA".to_owned(),
            );
        }
    }
    let design_basis_sha256 = sha256_hex(DESIGN_BASIS.as_bytes());
    validate_scenario(scenario, &design_basis_sha256)?;
    let greedy = run_controller(scenario, Controller::GreedyImmediate)?;
    let forecast_reserve = run_controller(scenario, Controller::ForecastReserve)?;
    verify_run(scenario, &greedy)?;
    verify_run(scenario, &forecast_reserve)?;

    Ok(BenchmarkReceipt {
        receipt_schema_version: SCHEMA_VERSION.to_owned(),
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        source_commit,
        scenario_id: scenario.case_id.clone(),
        evidence_class: scenario.evidence_class.clone(),
        claim_boundary: scenario.claim_boundary.clone(),
        input_sha256,
        design_basis_sha256,
        source_references: scenario.source_references.clone(),
        comparison: Comparison {
            critical_service_floor_met_by_greedy: greedy.total_critical_unserved_wh == 0,
            critical_service_floor_met_by_forecast_reserve: forecast_reserve.total_critical_unserved_wh == 0,
            forecast_reserve_reduces_critical_unserved_wh:
                forecast_reserve.total_critical_unserved_wh < greedy.total_critical_unserved_wh,
            greedy_process_batches_run: greedy.process_batches_run,
            forecast_reserve_process_batches_run: forecast_reserve.process_batches_run,
            critical_energy_unserved_wh_greedy: greedy.total_critical_unserved_wh,
            critical_energy_unserved_wh_forecast_reserve: forecast_reserve.total_critical_unserved_wh,
            interpretation: "The forecast-reserve controller may defer productive batches to protect the declared critical-service floor. Results are valid only for this synthetic forecast and energy model; they are not lunar hardware qualification.".to_owned(),
        },
        greedy,
        forecast_reserve,
    })
}

fn read_cli() -> Check<(PathBuf, Option<String>)> {
    let mut args = env::args().skip(1);
    let mut fixture_path = None;
    let mut source_commit = None;
    while let Some(arg) = args.next() {
        if arg == "--source-commit" {
            source_commit = Some(
                args.next()
                    .ok_or_else(|| "--source-commit requires a value".to_owned())?,
            );
        } else if fixture_path.is_none() {
            fixture_path = Some(PathBuf::from(arg));
        } else {
            return Err(format!("unexpected argument: {arg}"));
        }
    }
    Ok((
        fixture_path.unwrap_or_else(|| PathBuf::from(DEFAULT_CASE)),
        source_commit,
    ))
}

fn run_cli() -> Check<()> {
    let (fixture_path, source_commit) = read_cli()?;
    let raw_input = fs::read(&fixture_path)
        .map_err(|error| format!("cannot read {}: {error}", fixture_path.display()))?;
    let scenario: Scenario = serde_json::from_slice(&raw_input)
        .map_err(|error| format!("cannot decode {}: {error}", fixture_path.display()))?;
    let receipt = make_receipt(&scenario, sha256_hex(&raw_input), source_commit)?;
    let output = serde_json::to_string_pretty(&receipt)
        .map_err(|error| format!("cannot encode JSON receipt: {error}"))?;
    println!("{output}");
    Ok(())
}

fn main() -> ExitCode {
    match run_cli() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("lunar-base-bench: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_scenario() -> Scenario {
        serde_json::from_str(include_str!(
            "../fixtures/lunar_seed_utility_case_v0_1.json"
        ))
        .expect("checked-in synthetic fixture should deserialize")
    }

    #[test]
    fn fixture_design_digest_and_strict_schema_validate() {
        let scenario = fixture_scenario();
        validate_scenario(&scenario, &sha256_hex(DESIGN_BASIS.as_bytes()))
            .expect("fixture and design basis digest should match");
        assert_eq!(scenario.intervals.len(), 8);
        assert_eq!(scenario.evidence_class, "synthetic");
    }

    #[test]
    fn reserve_aware_dispatch_preserves_critical_service_better_than_greedy() {
        let scenario = fixture_scenario();
        let greedy = run_controller(&scenario, Controller::GreedyImmediate)
            .expect("greedy dispatch should produce a report");
        let reserve = run_controller(&scenario, Controller::ForecastReserve)
            .expect("reserve dispatch should produce a report");
        verify_run(&scenario, &greedy).expect("greedy trace accounting");
        verify_run(&scenario, &reserve).expect("reserve trace accounting");

        assert!(greedy.total_critical_unserved_wh > 0);
        assert_eq!(reserve.total_critical_unserved_wh, 0);
        assert!(reserve.process_batches_run < greedy.process_batches_run);
        assert_eq!(greedy.total_energy_balance_residual_wh, 0);
        assert_eq!(reserve.total_energy_balance_residual_wh, 0);
        assert_eq!(greedy.total_mass_balance_residual_g, 0);
        assert_eq!(reserve.total_mass_balance_residual_g, 0);
    }

    #[test]
    fn every_interval_and_batch_has_zero_independent_balance_residual() {
        let scenario = fixture_scenario();
        for controller in [Controller::GreedyImmediate, Controller::ForecastReserve] {
            let report = run_controller(&scenario, controller).expect("run");
            verify_run(&scenario, &report).expect("independent verifier");
            assert!(
                report
                    .steps
                    .iter()
                    .all(|step| step.energy_balance_residual_wh == 0)
            );
            assert!(
                report
                    .steps
                    .iter()
                    .all(|step| step.mass_balance_residual_g == 0)
            );
            assert!(report.steps.iter().all(|step| {
                step.process_batch_ran
                    || (step.process_feed_processed_g == 0 && step.recovered_water_g == 0)
            }));
        }
    }

    #[test]
    fn unexpected_generation_loss_is_reported_not_hidden_as_resilience() {
        let mut scenario = fixture_scenario();
        scenario.intervals[6].solar_generation_wh = 0;
        let report = run_controller(&scenario, Controller::ForecastReserve).expect("faulted run");
        verify_run(&scenario, &report).expect("faulted accounting remains auditable");
        assert!(report.total_critical_unserved_wh > 0);
        assert!(report.all_accounting_balanced);
        assert_eq!(report.total_energy_balance_residual_wh, 0);
        assert_eq!(report.total_mass_balance_residual_g, 0);
        assert!(
            report
                .steps
                .iter()
                .any(|step| step.critical_unserved_wh > 0)
        );
    }

    #[test]
    fn receipt_rejects_non_exact_source_commit_identity() {
        let scenario = fixture_scenario();
        assert!(make_receipt(&scenario, "a".repeat(64), Some("deadbeef".to_owned())).is_err());
        assert!(make_receipt(&scenario, "a".repeat(64), Some("g".repeat(40))).is_err());
    }

    #[test]
    fn invalid_efficiency_digest_and_duplicate_ids_fail_closed() {
        let digest = sha256_hex(DESIGN_BASIS.as_bytes());
        let mut invalid_efficiency = fixture_scenario();
        invalid_efficiency.battery.discharge_efficiency_ppm = PPM + 1;
        assert!(validate_scenario(&invalid_efficiency, &digest).is_err());

        let mut invalid_digest = fixture_scenario();
        invalid_digest.design_basis_sha256 = "0".repeat(64);
        assert!(validate_scenario(&invalid_digest, &digest).is_err());

        let mut duplicate_ids = fixture_scenario();
        duplicate_ids.intervals[1].id = duplicate_ids.intervals[0].id.clone();
        assert!(validate_scenario(&duplicate_ids, &digest).is_err());
    }

    #[test]
    fn verifier_detects_tampered_energy_trace_and_invalid_material_fraction() {
        let scenario = fixture_scenario();
        let mut report = run_controller(&scenario, Controller::ForecastReserve).expect("run");
        report.steps[0].curtailed_generation_wh += 1;
        assert!(verify_run(&scenario, &report).is_err());

        let mut invalid_fraction = fixture_scenario();
        invalid_fraction
            .resource_process
            .contained_water_fraction_ppm = PPM + 1;
        assert!(
            validate_scenario(&invalid_fraction, &sha256_hex(DESIGN_BASIS.as_bytes())).is_err()
        );
    }

    #[test]
    fn arithmetic_overflow_fails_closed() {
        let mut scenario = fixture_scenario();
        scenario.intervals[0].firm_generation_wh = u64::MAX;
        assert!(validate_scenario(&scenario, &sha256_hex(DESIGN_BASIS.as_bytes())).is_err());
    }
}
