use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const BPS: u64 = 10_000;
const REQUIRED_HEADERS: [&str; 9] = [
    "schema_version",
    "scenario_id",
    "battery_capacity_wh",
    "initial_soc_wh",
    "charge_efficiency_bps",
    "discharge_efficiency_bps",
    "charge_power_limit_wh_per_interval",
    "discharge_power_limit_wh_per_interval",
    "reserve_horizon_intervals",
];

#[derive(Debug, Clone, PartialEq)]
struct Batch {
    id: String,
    energy_wh: u64,
    feed_mass_g: u64,
    product_mass_g: u64,
    residue_mass_g: u64,
}

#[derive(Debug, Clone, PartialEq)]
struct Interval {
    id: u32,
    generation_wh: u64,
    critical_load_wh: u64,
    farm_lighting_wh: u64,
    light_window: bool,
    batch: Option<Batch>,
}

#[derive(Debug, Clone, PartialEq)]
struct Scenario {
    scenario_id: String,
    battery_capacity_wh: u64,
    initial_soc_wh: u64,
    charge_efficiency_bps: u64,
    discharge_efficiency_bps: u64,
    charge_power_limit_wh_per_interval: u64,
    discharge_power_limit_wh_per_interval: u64,
    reserve_horizon_intervals: usize,
    intervals: Vec<Interval>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Policy {
    Greedy,
    ForecastReserve,
}

impl Policy {
    fn as_str(self) -> &'static str {
        match self {
            Self::Greedy => "greedy",
            Self::ForecastReserve => "forecast_reserve",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct IntervalResult {
    interval_id: u32,
    generation_wh: u64,
    critical_requested_wh: u64,
    critical_served_wh: u64,
    farm_lighting_requested_wh: u64,
    farm_lighting_served_wh: u64,
    batch_id: Option<String>,
    batch_energy_requested_wh: u64,
    batch_processed: bool,
    batch_feed_consumed_g: u64,
    batch_product_g: u64,
    batch_residue_g: u64,
    battery_start_soc_wh: u64,
    battery_internal_withdrawn_wh: u64,
    battery_bus_output_wh: u64,
    battery_charge_input_wh: u64,
    battery_energy_stored_wh: u64,
    battery_end_soc_wh: u64,
    reserve_required_wh: u64,
    reserve_violation_wh: u64,
    curtailed_wh: u64,
    critical_unserved_wh: u64,
    farm_lighting_unserved_wh: u64,
    energy_balance_residual_wh: i128,
    soc_balance_residual_wh: i128,
    batch_mass_balance_residual_g: i128,
}

#[derive(Debug, Clone, PartialEq)]
struct DispatchReport {
    policy: Policy,
    intervals: Vec<IntervalResult>,
    critical_unserved_total_wh: u64,
    farm_lighting_unserved_total_wh: u64,
    processed_batches: usize,
    deferred_batches: usize,
    reserve_violation_intervals: usize,
    reserve_violation_total_wh: u64,
    curtailed_total_wh: u64,
    max_critical_shortfall_wh: u64,
}

fn parse_u64(headers: &BTreeMap<String, String>, key: &str) -> Result<u64, String> {
    headers
        .get(key)
        .ok_or_else(|| format!("missing header {key}"))?
        .parse::<u64>()
        .map_err(|_| format!("{key} must be a non-negative integer"))
}

fn parse_interval(value: &str, line_index: usize) -> Result<Interval, String> {
    let fields: Vec<&str> = value.split(',').map(str::trim).collect();
    if fields.len() != 10 {
        return Err(format!(
            "line {}: interval requires 10 comma-separated fields",
            line_index + 1
        ));
    }

    let parse_num = |index: usize, label: &str| -> Result<u64, String> {
        fields[index].parse::<u64>().map_err(|_| {
            format!(
                "line {}: {label} must be a non-negative integer",
                line_index + 1
            )
        })
    };

    let id = fields[0]
        .parse::<u32>()
        .map_err(|_| format!("line {}: interval id must be u32", line_index + 1))?;
    let generation_wh = parse_num(1, "generation_wh")?;
    let critical_load_wh = parse_num(2, "critical_load_wh")?;
    let farm_lighting_wh = parse_num(3, "farm_lighting_wh")?;
    let light_window = match fields[4] {
        "true" => true,
        "false" => false,
        _ => {
            return Err(format!(
                "line {}: light_window must be true or false",
                line_index + 1
            ));
        }
    };
    let batch_id = fields[5];
    let batch_energy_wh = parse_num(6, "batch_energy_wh")?;
    let feed_mass_g = parse_num(7, "batch_feed_mass_g")?;
    let product_mass_g = parse_num(8, "batch_product_mass_g")?;
    let residue_mass_g = parse_num(9, "batch_residue_mass_g")?;

    let batch = if batch_id == "none" {
        if batch_energy_wh != 0 || feed_mass_g != 0 || product_mass_g != 0 || residue_mass_g != 0 {
            return Err(format!(
                "line {}: absent batch must have zero energy and mass fields",
                line_index + 1
            ));
        }
        None
    } else {
        if batch_id.is_empty() || batch_energy_wh == 0 || feed_mass_g == 0 {
            return Err(format!(
                "line {}: present batch needs id, positive energy and positive feed mass",
                line_index + 1
            ));
        }
        Some(Batch {
            id: batch_id.to_owned(),
            energy_wh: batch_energy_wh,
            feed_mass_g,
            product_mass_g,
            residue_mass_g,
        })
    };

    Ok(Interval {
        id,
        generation_wh,
        critical_load_wh,
        farm_lighting_wh,
        light_window,
        batch,
    })
}

fn parse_scenario(source: &str) -> Result<Scenario, String> {
    let mut headers = BTreeMap::<String, String>::new();
    let mut intervals = Vec::<Interval>::new();

    for (line_index, raw_line) in source.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("line {}: expected key=value", line_index + 1))?;
        let key = key.trim();
        let value = value.trim();
        if key == "interval" {
            intervals.push(parse_interval(value, line_index)?);
            continue;
        }
        if !REQUIRED_HEADERS.contains(&key) {
            return Err(format!("line {}: unknown key {key}", line_index + 1));
        }
        if headers.insert(key.to_owned(), value.to_owned()).is_some() {
            return Err(format!("line {}: duplicate header {key}", line_index + 1));
        }
    }

    for key in REQUIRED_HEADERS {
        if !headers.contains_key(key) {
            return Err(format!("missing required header {key}"));
        }
    }
    if headers.get("schema_version").map(String::as_str) != Some("0.1") {
        return Err("unsupported schema_version; expected 0.1".to_owned());
    }

    let scenario_id = headers
        .get("scenario_id")
        .cloned()
        .ok_or_else(|| "missing scenario_id".to_owned())?;
    if scenario_id.is_empty()
        || !scenario_id
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err("scenario_id must use lowercase ASCII, digits and hyphens".to_owned());
    }

    let scenario = Scenario {
        scenario_id,
        battery_capacity_wh: parse_u64(&headers, "battery_capacity_wh")?,
        initial_soc_wh: parse_u64(&headers, "initial_soc_wh")?,
        charge_efficiency_bps: parse_u64(&headers, "charge_efficiency_bps")?,
        discharge_efficiency_bps: parse_u64(&headers, "discharge_efficiency_bps")?,
        charge_power_limit_wh_per_interval: parse_u64(
            &headers,
            "charge_power_limit_wh_per_interval",
        )?,
        discharge_power_limit_wh_per_interval: parse_u64(
            &headers,
            "discharge_power_limit_wh_per_interval",
        )?,
        reserve_horizon_intervals: parse_u64(&headers, "reserve_horizon_intervals")? as usize,
        intervals,
    };
    validate(&scenario)?;
    Ok(scenario)
}

fn validate(scenario: &Scenario) -> Result<(), String> {
    if scenario.battery_capacity_wh == 0 {
        return Err("battery_capacity_wh must be positive".to_owned());
    }
    if scenario.initial_soc_wh > scenario.battery_capacity_wh {
        return Err("initial_soc_wh exceeds battery capacity".to_owned());
    }
    if !(1..=BPS).contains(&scenario.charge_efficiency_bps) {
        return Err("charge_efficiency_bps must be within 1..=10000".to_owned());
    }
    if !(1..=BPS).contains(&scenario.discharge_efficiency_bps) {
        return Err("discharge_efficiency_bps must be within 1..=10000".to_owned());
    }
    if scenario.charge_power_limit_wh_per_interval == 0
        || scenario.discharge_power_limit_wh_per_interval == 0
    {
        return Err("battery charge/discharge interval limits must be positive".to_owned());
    }
    if scenario.intervals.is_empty() {
        return Err("scenario needs at least one interval".to_owned());
    }
    if scenario.reserve_horizon_intervals > scenario.intervals.len() {
        return Err("reserve horizon exceeds scenario length".to_owned());
    }

    let mut batch_ids = BTreeSet::new();
    for (index, interval) in scenario.intervals.iter().enumerate() {
        if interval.id as usize != index {
            return Err("interval ids must be contiguous, sorted and start at zero".to_owned());
        }
        if !interval.light_window && interval.farm_lighting_wh > 0 {
            return Err(format!(
                "interval {} requests farm lighting outside its declared light window",
                interval.id
            ));
        }
        if let Some(batch) = &interval.batch {
            if !batch_ids.insert(batch.id.clone()) {
                return Err(format!("duplicate batch id {}", batch.id));
            }
            let accounted = batch
                .product_mass_g
                .checked_add(batch.residue_mass_g)
                .ok_or_else(|| "batch mass accounting overflow".to_owned())?;
            if accounted != batch.feed_mass_g {
                return Err(format!("batch {} mass does not close", batch.id));
            }
        }
    }
    Ok(())
}

fn ceil_div(numerator: u128, denominator: u128) -> Result<u64, String> {
    if denominator == 0 {
        return Err("division by zero".to_owned());
    }
    let value = numerator
        .checked_add(denominator - 1)
        .ok_or_else(|| "ceil division overflow".to_owned())?
        / denominator;
    u64::try_from(value).map_err(|_| "ceil division result exceeds u64".to_owned())
}

fn bus_output_for_internal(internal_wh: u64, efficiency_bps: u64) -> Result<u64, String> {
    let value = u128::from(internal_wh)
        .checked_mul(u128::from(efficiency_bps))
        .ok_or_else(|| "battery output multiplication overflow".to_owned())?
        / u128::from(BPS);
    u64::try_from(value).map_err(|_| "battery output exceeds u64".to_owned())
}

fn required_internal_for_bus(bus_wh: u64, efficiency_bps: u64) -> Result<u64, String> {
    ceil_div(
        u128::from(bus_wh) * u128::from(BPS),
        u128::from(efficiency_bps),
    )
}

fn serve_load(
    requested_wh: u64,
    generation_pool_wh: &mut u64,
    soc_wh: &mut u64,
    discharge_remaining_wh: &mut u64,
    discharge_efficiency_bps: u64,
    battery_internal_total_wh: &mut u64,
    battery_bus_total_wh: &mut u64,
) -> Result<u64, String> {
    let from_generation = (*generation_pool_wh).min(requested_wh);
    *generation_pool_wh -= from_generation;
    let deficit = requested_wh - from_generation;
    if deficit == 0 {
        return Ok(from_generation);
    }

    let desired_internal = required_internal_for_bus(deficit, discharge_efficiency_bps)?;
    let internal = desired_internal.min(*soc_wh).min(*discharge_remaining_wh);
    let bus_output = bus_output_for_internal(internal, discharge_efficiency_bps)?;

    *soc_wh -= internal;
    *discharge_remaining_wh -= internal;
    *battery_internal_total_wh = battery_internal_total_wh
        .checked_add(internal)
        .ok_or_else(|| "battery internal withdrawal overflow".to_owned())?;
    *battery_bus_total_wh = battery_bus_total_wh
        .checked_add(bus_output)
        .ok_or_else(|| "battery bus output overflow".to_owned())?;

    let served_from_battery = bus_output.min(deficit);
    let rounding_surplus = bus_output - served_from_battery;
    *generation_pool_wh = generation_pool_wh
        .checked_add(rounding_surplus)
        .ok_or_else(|| "generation pool overflow".to_owned())?;
    Ok(from_generation + served_from_battery)
}

fn future_critical_reserve(scenario: &Scenario, current_index: usize) -> Result<u64, String> {
    let end = current_index
        .saturating_add(1)
        .saturating_add(scenario.reserve_horizon_intervals)
        .min(scenario.intervals.len());
    let mut reserve = 0_u64;
    for interval in &scenario.intervals[current_index + 1..end] {
        let deficit = interval
            .critical_load_wh
            .saturating_sub(interval.generation_wh);
        let internal = required_internal_for_bus(deficit, scenario.discharge_efficiency_bps)?;
        reserve = reserve
            .checked_add(internal)
            .ok_or_else(|| "future reserve sum overflow".to_owned())?;
    }
    Ok(reserve)
}

fn dispatch(scenario: &Scenario, policy: Policy) -> Result<DispatchReport, String> {
    let mut soc = scenario.initial_soc_wh;
    let mut results = Vec::with_capacity(scenario.intervals.len());
    let mut critical_unserved_total = 0_u64;
    let mut lighting_unserved_total = 0_u64;
    let mut processed_batches = 0_usize;
    let mut deferred_batches = 0_usize;
    let mut reserve_violation_intervals = 0_usize;
    let mut reserve_violation_total = 0_u64;
    let mut curtailed_total = 0_u64;
    let mut max_critical_shortfall = 0_u64;

    for (index, interval) in scenario.intervals.iter().enumerate() {
        let battery_start_soc = soc;
        let mut generation_pool = interval.generation_wh;
        let mut discharge_remaining = scenario.discharge_power_limit_wh_per_interval;
        let mut battery_internal_total = 0_u64;
        let mut battery_bus_total = 0_u64;

        let critical_served = serve_load(
            interval.critical_load_wh,
            &mut generation_pool,
            &mut soc,
            &mut discharge_remaining,
            scenario.discharge_efficiency_bps,
            &mut battery_internal_total,
            &mut battery_bus_total,
        )?;
        let critical_unserved = interval.critical_load_wh - critical_served;

        let lighting_served = serve_load(
            interval.farm_lighting_wh,
            &mut generation_pool,
            &mut soc,
            &mut discharge_remaining,
            scenario.discharge_efficiency_bps,
            &mut battery_internal_total,
            &mut battery_bus_total,
        )?;
        let lighting_unserved = interval.farm_lighting_wh - lighting_served;

        let reserve_required = future_critical_reserve(scenario, index)?;
        let mut batch_processed = false;
        let mut batch_id = None;
        let mut batch_energy_requested = 0_u64;
        let mut batch_feed_consumed = 0_u64;
        let mut batch_product = 0_u64;
        let mut batch_residue = 0_u64;

        if let Some(batch) = &interval.batch {
            batch_id = Some(batch.id.clone());
            batch_energy_requested = batch.energy_wh;
            let from_generation = generation_pool.min(batch.energy_wh);
            let battery_deficit = batch.energy_wh - from_generation;
            let required_internal =
                required_internal_for_bus(battery_deficit, scenario.discharge_efficiency_bps)?;
            let energy_feasible =
                required_internal <= soc && required_internal <= discharge_remaining;
            let reserve_feasible = policy == Policy::Greedy
                || soc.saturating_sub(required_internal) >= reserve_required;

            if energy_feasible && reserve_feasible {
                generation_pool -= from_generation;
                let bus_output =
                    bus_output_for_internal(required_internal, scenario.discharge_efficiency_bps)?;
                if bus_output < battery_deficit {
                    return Err(
                        "internal battery calculation failed to cover batch deficit".to_owned()
                    );
                }
                soc -= required_internal;
                battery_internal_total = battery_internal_total
                    .checked_add(required_internal)
                    .ok_or_else(|| "battery internal withdrawal overflow".to_owned())?;
                battery_bus_total = battery_bus_total
                    .checked_add(bus_output)
                    .ok_or_else(|| "battery bus output overflow".to_owned())?;
                generation_pool = generation_pool
                    .checked_add(bus_output - battery_deficit)
                    .ok_or_else(|| "generation pool overflow".to_owned())?;

                batch_processed = true;
                batch_feed_consumed = batch.feed_mass_g;
                batch_product = batch.product_mass_g;
                batch_residue = batch.residue_mass_g;
                processed_batches += 1;
            } else {
                deferred_batches += 1;
            }
        }

        let headroom = scenario.battery_capacity_wh - soc;
        let max_charge_input_for_headroom = u64::try_from(
            u128::from(headroom) * u128::from(BPS) / u128::from(scenario.charge_efficiency_bps),
        )
        .map_err(|_| "battery charge headroom overflow".to_owned())?;
        let charge_input = generation_pool
            .min(scenario.charge_power_limit_wh_per_interval)
            .min(max_charge_input_for_headroom);
        let charge_stored = bus_output_for_internal(charge_input, scenario.charge_efficiency_bps)?;
        if charge_stored > headroom {
            return Err("battery charge calculation exceeds storage capacity".to_owned());
        }
        generation_pool -= charge_input;
        soc = soc
            .checked_add(charge_stored)
            .ok_or_else(|| "battery SOC overflow".to_owned())?;

        let curtailed = generation_pool;
        let energy_supply = interval
            .generation_wh
            .checked_add(battery_bus_total)
            .ok_or_else(|| "energy supply overflow".to_owned())?;
        let energy_sinks = critical_served
            .checked_add(lighting_served)
            .and_then(|x| {
                x.checked_add(if batch_processed {
                    batch_energy_requested
                } else {
                    0
                })
            })
            .and_then(|x| x.checked_add(charge_input))
            .and_then(|x| x.checked_add(curtailed))
            .ok_or_else(|| "energy sink sum overflow".to_owned())?;
        let energy_balance_residual = i128::from(energy_supply) - i128::from(energy_sinks);

        let expected_soc_end = i128::from(battery_start_soc) - i128::from(battery_internal_total)
            + i128::from(charge_stored);
        let soc_balance_residual = i128::from(soc) - expected_soc_end;

        let mass_balance_residual = if batch_processed {
            i128::from(batch_feed_consumed) - i128::from(batch_product) - i128::from(batch_residue)
        } else {
            0
        };

        let reserve_violation = reserve_required.saturating_sub(soc);
        if reserve_violation > 0 {
            reserve_violation_intervals += 1;
            reserve_violation_total = reserve_violation_total
                .checked_add(reserve_violation)
                .ok_or_else(|| "reserve violation sum overflow".to_owned())?;
        }

        critical_unserved_total = critical_unserved_total
            .checked_add(critical_unserved)
            .ok_or_else(|| "critical unserved sum overflow".to_owned())?;
        lighting_unserved_total = lighting_unserved_total
            .checked_add(lighting_unserved)
            .ok_or_else(|| "farm lighting unserved sum overflow".to_owned())?;
        curtailed_total = curtailed_total
            .checked_add(curtailed)
            .ok_or_else(|| "curtailment sum overflow".to_owned())?;
        max_critical_shortfall = max_critical_shortfall.max(critical_unserved);

        results.push(IntervalResult {
            interval_id: interval.id,
            generation_wh: interval.generation_wh,
            critical_requested_wh: interval.critical_load_wh,
            critical_served_wh: critical_served,
            farm_lighting_requested_wh: interval.farm_lighting_wh,
            farm_lighting_served_wh: lighting_served,
            batch_id,
            batch_energy_requested_wh: batch_energy_requested,
            batch_processed,
            batch_feed_consumed_g: batch_feed_consumed,
            batch_product_g: batch_product,
            batch_residue_g: batch_residue,
            battery_start_soc_wh: battery_start_soc,
            battery_internal_withdrawn_wh: battery_internal_total,
            battery_bus_output_wh: battery_bus_total,
            battery_charge_input_wh: charge_input,
            battery_energy_stored_wh: charge_stored,
            battery_end_soc_wh: soc,
            reserve_required_wh: reserve_required,
            reserve_violation_wh: reserve_violation,
            curtailed_wh: curtailed,
            critical_unserved_wh: critical_unserved,
            farm_lighting_unserved_wh: lighting_unserved,
            energy_balance_residual_wh: energy_balance_residual,
            soc_balance_residual_wh: soc_balance_residual,
            batch_mass_balance_residual_g: mass_balance_residual,
        });
    }

    Ok(DispatchReport {
        policy,
        intervals: results,
        critical_unserved_total_wh: critical_unserved_total,
        farm_lighting_unserved_total_wh: lighting_unserved_total,
        processed_batches,
        deferred_batches,
        reserve_violation_intervals,
        reserve_violation_total_wh: reserve_violation_total,
        curtailed_total_wh: curtailed_total,
        max_critical_shortfall_wh: max_critical_shortfall,
    })
}

fn verify_report(scenario: &Scenario, report: &DispatchReport) -> Result<(), String> {
    if report.intervals.len() != scenario.intervals.len() {
        return Err("dispatch trace interval count mismatch".to_owned());
    }

    let mut previous_soc = scenario.initial_soc_wh;
    let mut critical_sum = 0_u64;
    let mut lighting_sum = 0_u64;
    let mut processed_batches = 0_usize;
    let mut deferred_batches = 0_usize;
    let mut reserve_violation_intervals = 0_usize;
    let mut reserve_violation_total = 0_u64;
    let mut curtailed_total = 0_u64;
    let mut max_critical_shortfall = 0_u64;

    for (index, (input, actual)) in scenario
        .intervals
        .iter()
        .zip(report.intervals.iter())
        .enumerate()
    {
        if actual.interval_id != input.id {
            return Err(format!("interval trace identity mismatch at index {index}"));
        }
        if actual.generation_wh != input.generation_wh
            || actual.critical_requested_wh != input.critical_load_wh
            || actual.farm_lighting_requested_wh != input.farm_lighting_wh
        {
            return Err(format!("interval {} input binding mismatch", input.id));
        }
        if actual.battery_start_soc_wh != previous_soc {
            return Err(format!(
                "interval {} battery SOC continuity mismatch",
                input.id
            ));
        }
        if actual.battery_internal_withdrawn_wh > scenario.discharge_power_limit_wh_per_interval {
            return Err(format!(
                "interval {} exceeds battery discharge limit",
                input.id
            ));
        }
        if actual.battery_charge_input_wh > scenario.charge_power_limit_wh_per_interval {
            return Err(format!(
                "interval {} exceeds battery charge limit",
                input.id
            ));
        }
        if actual.battery_internal_withdrawn_wh > actual.battery_start_soc_wh {
            return Err(format!(
                "interval {} withdraws more energy than the starting SOC",
                input.id
            ));
        }
        if actual.battery_energy_stored_wh
            != bus_output_for_internal(
                actual.battery_charge_input_wh,
                scenario.charge_efficiency_bps,
            )?
        {
            return Err(format!(
                "interval {} battery charge efficiency receipt mismatch",
                input.id
            ));
        }

        let battery_bus_upper_bound = bus_output_for_internal(
            actual.battery_internal_withdrawn_wh,
            scenario.discharge_efficiency_bps,
        )?;
        if actual.battery_bus_output_wh > battery_bus_upper_bound {
            return Err(format!(
                "interval {} reports impossible battery bus output",
                input.id
            ));
        }

        let expected_batch_energy = input.batch.as_ref().map_or(0, |batch| batch.energy_wh);
        let expected_batch_id = input.batch.as_ref().map(|batch| batch.id.clone());
        if actual.batch_id != expected_batch_id
            || actual.batch_energy_requested_wh != expected_batch_energy
        {
            return Err(format!(
                "interval {} batch identity/energy mismatch",
                input.id
            ));
        }

        let batch_energy_served = if actual.batch_processed {
            let batch = input
                .batch
                .as_ref()
                .ok_or_else(|| format!("interval {} processed a nonexistent batch", input.id))?;
            if actual.batch_feed_consumed_g != batch.feed_mass_g
                || actual.batch_product_g != batch.product_mass_g
                || actual.batch_residue_g != batch.residue_mass_g
            {
                return Err(format!(
                    "interval {} batch receipt differs from input",
                    input.id
                ));
            }
            processed_batches += 1;
            actual.batch_energy_requested_wh
        } else {
            if actual.batch_feed_consumed_g != 0
                || actual.batch_product_g != 0
                || actual.batch_residue_g != 0
            {
                return Err(format!(
                    "interval {} deferred batch consumed resources",
                    input.id
                ));
            }
            if input.batch.is_some() {
                deferred_batches += 1;
            }
            0
        };

        let independently_recomputed_mass_residual = if actual.batch_processed {
            let feed = i128::from(actual.batch_feed_consumed_g);
            let product = i128::from(actual.batch_product_g);
            let residue = i128::from(actual.batch_residue_g);
            feed - product - residue
        } else {
            0
        };
        if independently_recomputed_mass_residual != 0
            || actual.batch_mass_balance_residual_g != independently_recomputed_mass_residual
        {
            return Err(format!(
                "interval {} batch mass verification failed",
                input.id
            ));
        }

        let requested_critical =
            u128::from(actual.critical_served_wh) + u128::from(actual.critical_unserved_wh);
        if requested_critical != u128::from(input.critical_load_wh) {
            return Err(format!(
                "interval {} critical-load accounting mismatch",
                input.id
            ));
        }
        let requested_lighting = u128::from(actual.farm_lighting_served_wh)
            + u128::from(actual.farm_lighting_unserved_wh);
        if requested_lighting != u128::from(input.farm_lighting_wh) {
            return Err(format!(
                "interval {} farm-light accounting mismatch",
                input.id
            ));
        }

        let source_wh = u128::from(input.generation_wh) + u128::from(actual.battery_bus_output_wh);
        let sink_wh = u128::from(actual.critical_served_wh)
            + u128::from(actual.farm_lighting_served_wh)
            + u128::from(batch_energy_served)
            + u128::from(actual.battery_charge_input_wh)
            + u128::from(actual.curtailed_wh);
        let independently_recomputed_energy_residual = i128::try_from(source_wh)
            .map_err(|_| "energy source exceeds i128".to_owned())?
            - i128::try_from(sink_wh).map_err(|_| "energy sinks exceed i128".to_owned())?;
        if independently_recomputed_energy_residual != 0
            || actual.energy_balance_residual_wh != independently_recomputed_energy_residual
        {
            return Err(format!(
                "interval {} independent energy balance failed",
                input.id
            ));
        }

        let expected_soc = i128::from(actual.battery_start_soc_wh)
            - i128::from(actual.battery_internal_withdrawn_wh)
            + i128::from(actual.battery_energy_stored_wh);
        let independently_recomputed_soc_residual =
            i128::from(actual.battery_end_soc_wh) - expected_soc;
        if independently_recomputed_soc_residual != 0
            || actual.soc_balance_residual_wh != independently_recomputed_soc_residual
        {
            return Err(format!(
                "interval {} independent battery SOC balance failed",
                input.id
            ));
        }
        if actual.battery_end_soc_wh > scenario.battery_capacity_wh {
            return Err(format!(
                "interval {} SOC exceeds battery capacity",
                input.id
            ));
        }

        // Recompute future critical reserve from the source scenario rather than
        // trusting the dispatcher's declared reserve field.
        let horizon_end = index
            .saturating_add(1)
            .saturating_add(scenario.reserve_horizon_intervals)
            .min(scenario.intervals.len());
        let mut independent_reserve = 0_u128;
        for future in scenario.intervals.iter().take(horizon_end).skip(index + 1) {
            let future_deficit = future.critical_load_wh.saturating_sub(future.generation_wh);
            let numerator = u128::from(future_deficit) * u128::from(BPS);
            let denominator = u128::from(scenario.discharge_efficiency_bps);
            let required = if numerator == 0 {
                0
            } else {
                numerator.div_ceil(denominator)
            };
            independent_reserve = independent_reserve
                .checked_add(required)
                .ok_or_else(|| "independent reserve calculation overflow".to_owned())?;
        }
        let independent_reserve_wh = u64::try_from(independent_reserve)
            .map_err(|_| "independent reserve exceeds u64".to_owned())?;
        if actual.reserve_required_wh != independent_reserve_wh {
            return Err(format!(
                "interval {} forecast reserve receipt mismatch",
                input.id
            ));
        }
        let independent_reserve_violation =
            independent_reserve_wh.saturating_sub(actual.battery_end_soc_wh);
        if actual.reserve_violation_wh != independent_reserve_violation {
            return Err(format!(
                "interval {} reserve violation receipt mismatch",
                input.id
            ));
        }

        if actual.critical_unserved_wh > 0 {
            max_critical_shortfall = max_critical_shortfall.max(actual.critical_unserved_wh);
        }
        critical_sum = critical_sum
            .checked_add(actual.critical_unserved_wh)
            .ok_or_else(|| "critical unserved total overflow".to_owned())?;
        lighting_sum = lighting_sum
            .checked_add(actual.farm_lighting_unserved_wh)
            .ok_or_else(|| "farm lighting unserved total overflow".to_owned())?;
        curtailed_total = curtailed_total
            .checked_add(actual.curtailed_wh)
            .ok_or_else(|| "curtailment total overflow".to_owned())?;
        if independent_reserve_violation > 0 {
            reserve_violation_intervals += 1;
            reserve_violation_total = reserve_violation_total
                .checked_add(independent_reserve_violation)
                .ok_or_else(|| "reserve violation total overflow".to_owned())?;
        }

        previous_soc = actual.battery_end_soc_wh;
    }

    if critical_sum != report.critical_unserved_total_wh
        || lighting_sum != report.farm_lighting_unserved_total_wh
        || processed_batches != report.processed_batches
        || deferred_batches != report.deferred_batches
        || reserve_violation_intervals != report.reserve_violation_intervals
        || reserve_violation_total != report.reserve_violation_total_wh
        || curtailed_total != report.curtailed_total_wh
        || max_critical_shortfall != report.max_critical_shortfall_wh
    {
        return Err(
            "dispatch aggregate receipt does not match independently recomputed trace".to_owned(),
        );
    }
    Ok(())
}

fn render_json(scenario: &Scenario, greedy: &DispatchReport, reserve: &DispatchReport) -> String {
    let render_policy = |report: &DispatchReport| {
        let rows = report
            .intervals
            .iter()
            .map(|i| {
                format!(
                    concat!(
                        "{{\"interval\":{},\"critical_unserved_wh\":{},",
                        "\"farm_light_unserved_wh\":{},\"batch_processed\":{},",
                        "\"battery_end_soc_wh\":{},\"reserve_required_wh\":{},",
                        "\"reserve_violation_wh\":{},\"energy_balance_residual_wh\":{},",
                        "\"soc_balance_residual_wh\":{},\"mass_balance_residual_g\":{}}}"
                    ),
                    i.interval_id,
                    i.critical_unserved_wh,
                    i.farm_lighting_unserved_wh,
                    i.batch_processed,
                    i.battery_end_soc_wh,
                    i.reserve_required_wh,
                    i.reserve_violation_wh,
                    i.energy_balance_residual_wh,
                    i.soc_balance_residual_wh,
                    i.batch_mass_balance_residual_g
                )
            })
            .collect::<Vec<_>>()
            .join(",\n");

        format!(
            concat!(
                "{{\n",
                "  \"policy\": \"{}\",\n",
                "  \"critical_unserved_total_wh\": {},\n",
                "  \"farm_lighting_unserved_total_wh\": {},\n",
                "  \"processed_batches\": {},\n",
                "  \"deferred_batches\": {},\n",
                "  \"reserve_violation_intervals\": {},\n",
                "  \"reserve_violation_total_wh\": {},\n",
                "  \"curtailed_total_wh\": {},\n",
                "  \"max_critical_shortfall_wh\": {},\n",
                "  \"intervals\": [{}]\n",
                "}}"
            ),
            report.policy.as_str(),
            report.critical_unserved_total_wh,
            report.farm_lighting_unserved_total_wh,
            report.processed_batches,
            report.deferred_batches,
            report.reserve_violation_intervals,
            report.reserve_violation_total_wh,
            report.curtailed_total_wh,
            report.max_critical_shortfall_wh,
            rows
        )
    };

    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"0.1\",\n",
            "  \"evidence_class\": \"synthetic\",\n",
            "  \"scenario_id\": \"{}\",\n",
            "  \"battery_capacity_wh\": {},\n",
            "  \"initial_soc_wh\": {},\n",
            "  \"charge_efficiency_bps\": {},\n",
            "  \"discharge_efficiency_bps\": {},\n",
            "  \"reserve_horizon_intervals\": {},\n",
            "  \"claim_ceiling\": \"synthetic discrete dispatch and accounting only; no reactor, battery, crop physiology, life support, lunar site or flight qualification\",\n",
            "  \"greedy\": {},\n",
            "  \"forecast_reserve\": {}\n",
            "}}"
        ),
        scenario.scenario_id,
        scenario.battery_capacity_wh,
        scenario.initial_soc_wh,
        scenario.charge_efficiency_bps,
        scenario.discharge_efficiency_bps,
        scenario.reserve_horizon_intervals,
        render_policy(greedy),
        render_policy(reserve)
    )
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let path = match args.as_slice() {
        [] => "fixtures/synthetic-lunar-seed-base-reserve-v0-1.input".to_owned(),
        [flag, path] if flag == "--input" => path.clone(),
        _ => return Err("usage: lunar-seed-dispatch-reference [--input PATH]".to_owned()),
    };

    let source =
        fs::read_to_string(&path).map_err(|error| format!("could not read {path}: {error}"))?;
    let scenario = parse_scenario(&source)?;
    let greedy = dispatch(&scenario, Policy::Greedy)?;
    let reserve = dispatch(&scenario, Policy::ForecastReserve)?;
    verify_report(&scenario, &greedy)?;
    verify_report(&scenario, &reserve)?;
    println!("{}", render_json(&scenario, &greedy, &reserve));
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../fixtures/synthetic-lunar-seed-base-reserve-v0-1.input");

    #[test]
    fn forecast_reserve_policy_avoids_critical_shortfall_and_defers_the_isru_batch() {
        let scenario = parse_scenario(FIXTURE).expect("valid synthetic scenario");
        let greedy = dispatch(&scenario, Policy::Greedy).expect("greedy dispatch");
        let reserve = dispatch(&scenario, Policy::ForecastReserve).expect("reserve dispatch");

        assert_eq!(greedy.processed_batches, 1);
        assert_eq!(reserve.processed_batches, 0);
        assert_eq!(reserve.deferred_batches, 1);
        assert_eq!(reserve.critical_unserved_total_wh, 0);
        assert!(greedy.critical_unserved_total_wh > 0);
        assert!(greedy.reserve_violation_intervals > 0);
        assert_eq!(reserve.reserve_violation_intervals, 0);
    }

    #[test]
    fn every_policy_trace_closes_energy_soc_and_batch_mass() {
        let scenario = parse_scenario(FIXTURE).expect("valid synthetic scenario");
        let greedy = dispatch(&scenario, Policy::Greedy).expect("greedy dispatch");
        let reserve = dispatch(&scenario, Policy::ForecastReserve).expect("reserve dispatch");
        verify_report(&scenario, &greedy).expect("greedy trace closes");
        verify_report(&scenario, &reserve).expect("reserve trace closes");
    }

    #[test]
    fn independent_verifier_rejects_tampered_interval_trace() {
        let scenario = parse_scenario(FIXTURE).expect("valid synthetic scenario");
        let mut reserve = dispatch(&scenario, Policy::ForecastReserve).expect("reserve dispatch");
        reserve.intervals[0].critical_served_wh += 1;
        assert!(verify_report(&scenario, &reserve).is_err());
    }

    #[test]
    fn the_crop_light_window_is_served_in_the_reference_case() {
        let scenario = parse_scenario(FIXTURE).expect("valid synthetic scenario");
        let reserve = dispatch(&scenario, Policy::ForecastReserve).expect("reserve dispatch");
        assert_eq!(reserve.farm_lighting_unserved_total_wh, 0);
    }

    #[test]
    fn a_deferred_batch_consumes_no_feedstock() {
        let scenario = parse_scenario(FIXTURE).expect("valid synthetic scenario");
        let reserve = dispatch(&scenario, Policy::ForecastReserve).expect("reserve dispatch");
        let batch_interval = reserve
            .intervals
            .iter()
            .find(|i| i.batch_id.is_some())
            .unwrap();
        assert!(!batch_interval.batch_processed);
        assert_eq!(batch_interval.batch_feed_consumed_g, 0);
        assert_eq!(batch_interval.batch_product_g, 0);
        assert_eq!(batch_interval.batch_residue_g, 0);
    }

    #[test]
    fn rejects_invalid_efficiency_duplicate_interval_and_bad_mass_balance() {
        let bad_efficiency = FIXTURE.replace(
            "discharge_efficiency_bps=9500",
            "discharge_efficiency_bps=10001",
        );
        assert!(
            parse_scenario(&bad_efficiency)
                .unwrap_err()
                .contains("efficiency")
        );

        let duplicate_header = format!("{FIXTURE}battery_capacity_wh=120000\n");
        assert!(
            parse_scenario(&duplicate_header)
                .unwrap_err()
                .contains("duplicate header")
        );

        let bad_mass = FIXTURE.replace(
            "interval=18,40000,20000,0,false,water_batch_01,60000,100000,5000,95000",
            "interval=18,40000,20000,0,false,water_batch_01,60000,100000,5000,94000",
        );
        assert!(
            parse_scenario(&bad_mass)
                .unwrap_err()
                .contains("mass does not close")
        );
    }

    #[test]
    fn rejects_light_outside_crop_window_and_soc_above_capacity() {
        let bad_light = FIXTURE.replace(
            "interval=16,40000,20000,0,false,none,0,0,0,0",
            "interval=16,40000,20000,5000,false,none,0,0,0,0",
        );
        assert!(
            parse_scenario(&bad_light)
                .unwrap_err()
                .contains("outside its declared light window")
        );

        let bad_soc = FIXTURE.replace("initial_soc_wh=120000", "initial_soc_wh=120001");
        assert!(
            parse_scenario(&bad_soc)
                .unwrap_err()
                .contains("exceeds battery capacity")
        );
    }
}
