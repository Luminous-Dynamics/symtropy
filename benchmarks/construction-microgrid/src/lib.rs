//! Synthetic-first construction-site energy portfolio accounting.
//!
//! This evaluates declared external power envelopes against measured load
//! intervals. It is not a grid simulator, optimizer, nuclear plant model,
//! electrical protection model or physical dispatch controller.

use std::collections::{BTreeMap, BTreeSet};

const EFFICIENCY_SCALE_PPM: u64 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceKind {
    GridImport,
    RenewableGeneration,
    ConventionalGeneration,
    ExternalFirmSupply,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceQualification {
    VerifiedForSyntheticScenario,
    ContractOrConnectionUnverified,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceInterval {
    pub available_energy_wh: Option<u64>,
    pub available_power_w: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupplySource {
    pub source_id: String,
    pub kind: SourceKind,
    /// Explicit order; lower values are dispatched first, then source ID.
    pub dispatch_priority: u16,
    pub maximum_power_w: u64,
    pub qualification: SourceQualification,
    pub variable_cost_microcurrency_per_kwh: Option<u64>,
    pub emissions_grams_per_kwh: Option<u64>,
    pub intervals: Vec<SourceInterval>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadInterval {
    pub interval_id: String,
    pub duration_seconds: u64,
    pub total_energy_wh: u64,
    pub critical_energy_wh: u64,
    /// Productive output accepted for this interval, in caller-declared units.
    pub accepted_output_units: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatteryProfile {
    pub capacity_wh: u64,
    pub initial_state_of_charge_wh: u64,
    pub minimum_reserve_wh: u64,
    pub maximum_charge_power_w: u64,
    pub maximum_discharge_power_w: u64,
    pub charge_efficiency_ppm: u64,
    pub discharge_efficiency_ppm: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortfolioInput {
    pub portfolio_id: String,
    pub sources: Vec<SupplySource>,
    pub loads: Vec<LoadInterval>,
    pub battery: Option<BatteryProfile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortfolioError {
    InvalidIdentity,
    EmptyLoadSeries,
    ZeroIntervalDuration,
    InvalidCriticalLoad,
    SourceIntervalCountMismatch,
    DuplicateSource,
    InvalidSourcePowerEnvelope,
    SourceEnergyExceedsPowerEnvelope,
    UnknownSourceAvailability,
    ExternalSupplyNotQualified,
    InvalidBatteryState,
    InvalidBatteryEfficiency,
    ArithmeticOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortfolioDisposition {
    FullyServed,
    CriticalLoadUnserved,
    FlexibleLoadUnserved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntervalBalance {
    pub interval_id: String,
    pub source_available_wh: u64,
    pub source_to_load_wh: u64,
    pub source_to_battery_wh: u64,
    pub curtailed_wh: u64,
    pub critical_demand_wh: u64,
    pub critical_served_wh: u64,
    pub critical_unserved_wh: u64,
    pub flexible_demand_wh: u64,
    pub flexible_served_wh: u64,
    pub flexible_unserved_wh: u64,
    pub battery_start_soc_wh: u64,
    pub battery_charge_stored_wh: u64,
    pub battery_charge_loss_wh: u64,
    pub battery_discharge_delivered_wh: u64,
    pub battery_discharge_storage_draw_wh: u64,
    pub battery_discharge_loss_wh: u64,
    pub battery_end_soc_wh: u64,
    pub source_energy_by_id_wh: BTreeMap<String, u64>,
    pub variable_cost_microcurrency: Option<u64>,
    pub emissions_grams: Option<u64>,
}

/// Exact load-energy / accepted-output ratio. Consumers may display a decimal,
/// but the evaluator retains the integer numerator and denominator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcceptedOutputEnergyIntensity {
    pub load_energy_wh: u64,
    pub accepted_output_units: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortfolioEvaluation {
    pub disposition: PortfolioDisposition,
    pub intervals: Vec<IntervalBalance>,
    pub total_demand_wh: u64,
    pub total_critical_unserved_wh: u64,
    pub total_flexible_unserved_wh: u64,
    pub total_curtailed_wh: u64,
    pub total_source_energy_wh: u64,
    pub total_battery_loss_wh: u64,
    /// None means at least one used source lacked a cost factor.
    pub total_variable_cost_microcurrency: Option<u64>,
    /// None means at least one used source lacked an emissions factor.
    pub total_emissions_grams: Option<u64>,
    /// Delivered site-load energy divided by accepted output, not generator input.
    /// None means at least one interval omitted its accepted-output measurement.
    pub load_energy_intensity: Option<AcceptedOutputEnergyIntensity>,
}

#[must_use]
pub fn evaluate_portfolio(input: &PortfolioInput) -> Result<PortfolioEvaluation, PortfolioError> {
    if input.portfolio_id.trim().is_empty() {
        return Err(PortfolioError::InvalidIdentity);
    }
    if input.loads.is_empty() {
        return Err(PortfolioError::EmptyLoadSeries);
    }
    for load in &input.loads {
        if load.interval_id.trim().is_empty() {
            return Err(PortfolioError::InvalidIdentity);
        }
        if load.duration_seconds == 0 {
            return Err(PortfolioError::ZeroIntervalDuration);
        }
        if load.critical_energy_wh > load.total_energy_wh {
            return Err(PortfolioError::InvalidCriticalLoad);
        }
    }

    let mut source_ids = BTreeSet::new();
    for source in &input.sources {
        if source.source_id.trim().is_empty() || !source_ids.insert(source.source_id.clone()) {
            return Err(if source.source_id.trim().is_empty() {
                PortfolioError::InvalidIdentity
            } else {
                PortfolioError::DuplicateSource
            });
        }
        if source.intervals.len() != input.loads.len() {
            return Err(PortfolioError::SourceIntervalCountMismatch);
        }
        if source.qualification == SourceQualification::Unavailable
            && source.intervals.iter().any(|i| {
                i.available_energy_wh.unwrap_or(0) > 0 || i.available_power_w.unwrap_or(0) > 0
            })
        {
            return Err(PortfolioError::ExternalSupplyNotQualified);
        }
        for (availability, load) in source.intervals.iter().zip(input.loads.iter()) {
            let (Some(energy), Some(power)) =
                (availability.available_energy_wh, availability.available_power_w)
            else {
                return Err(PortfolioError::UnknownSourceAvailability);
            };
            if source.qualification == SourceQualification::ContractOrConnectionUnverified
                && (energy > 0 || power > 0)
            {
                return Err(PortfolioError::ExternalSupplyNotQualified);
            }
            if power > source.maximum_power_w {
                return Err(PortfolioError::InvalidSourcePowerEnvelope);
            }
            let max_energy = u128::from(power)
                .checked_mul(u128::from(load.duration_seconds))
                .ok_or(PortfolioError::ArithmeticOverflow)?
                / 3600;
            if u128::from(energy) > max_energy {
                return Err(PortfolioError::SourceEnergyExceedsPowerEnvelope);
            }
        }
    }

    if let Some(battery) = &input.battery {
        if battery.initial_state_of_charge_wh > battery.capacity_wh
            || battery.minimum_reserve_wh > battery.initial_state_of_charge_wh
        {
            return Err(PortfolioError::InvalidBatteryState);
        }
        if battery.charge_efficiency_ppm == 0
            || battery.charge_efficiency_ppm > EFFICIENCY_SCALE_PPM
            || battery.discharge_efficiency_ppm == 0
            || battery.discharge_efficiency_ppm > EFFICIENCY_SCALE_PPM
        {
            return Err(PortfolioError::InvalidBatteryEfficiency);
        }
    }

    let mut source_order: Vec<usize> = (0..input.sources.len()).collect();
    source_order.sort_by_key(|index| {
        (input.sources[*index].dispatch_priority, input.sources[*index].source_id.clone())
    });

    let mut soc = input.battery.as_ref().map_or(0, |battery| battery.initial_state_of_charge_wh);
    let mut intervals = Vec::with_capacity(input.loads.len());
    let mut total_demand = 0_u64;
    let mut total_critical_unserved = 0_u64;
    let mut total_flexible_unserved = 0_u64;
    let mut total_curtailed = 0_u64;
    let mut total_source_energy = 0_u64;
    let mut total_battery_loss = 0_u64;
    let mut total_cost = Some(0_u64);
    let mut total_emissions = Some(0_u64);
    let mut accepted_units_total = 0_u64;
    let mut accepted_units_known = true;

    for (interval_index, load) in input.loads.iter().enumerate() {
        let mut available = Vec::with_capacity(input.sources.len());
        let mut source_available_wh = 0_u64;
        for source in &input.sources {
            let value = source.intervals[interval_index].available_energy_wh
                .ok_or(PortfolioError::UnknownSourceAvailability)?;
            source_available_wh = source_available_wh
                .checked_add(value)
                .ok_or(PortfolioError::ArithmeticOverflow)?;
            available.push(value);
        }

        let battery_start_soc = soc;
        let mut critical_remaining = load.critical_energy_wh;
        let mut flexible_remaining = load.total_energy_wh - load.critical_energy_wh;
        let mut source_to_load = 0_u64;
        let mut source_to_battery = 0_u64;
        let mut source_energy_by_id = BTreeMap::new();
        let mut interval_cost = Some(0_u64);
        let mut interval_emissions = Some(0_u64);

        for &index in &source_order {
            let amount = available[index].min(critical_remaining);
            available[index] -= amount;
            critical_remaining -= amount;
            source_to_load = source_to_load.checked_add(amount).ok_or(PortfolioError::ArithmeticOverflow)?;
            let used = source_energy_by_id.entry(input.sources[index].source_id.clone()).or_insert(0_u64);
            *used = used.checked_add(amount).ok_or(PortfolioError::ArithmeticOverflow)?;
            add_optional_cost(
                &mut interval_cost,
                input.sources[index].variable_cost_microcurrency_per_kwh,
                amount,
            )?;
            add_optional_emissions(
                &mut interval_emissions,
                input.sources[index].emissions_grams_per_kwh,
                amount,
            )?;
        }

        for &index in &source_order {
            let amount = available[index].min(flexible_remaining);
            available[index] -= amount;
            flexible_remaining -= amount;
            source_to_load = source_to_load.checked_add(amount).ok_or(PortfolioError::ArithmeticOverflow)?;
            let used = source_energy_by_id.entry(input.sources[index].source_id.clone()).or_insert(0_u64);
            *used = used.checked_add(amount).ok_or(PortfolioError::ArithmeticOverflow)?;
            add_optional_cost(
                &mut interval_cost,
                input.sources[index].variable_cost_microcurrency_per_kwh,
                amount,
            )?;
            add_optional_emissions(
                &mut interval_emissions,
                input.sources[index].emissions_grams_per_kwh,
                amount,
            )?;
        }

        let mut battery_charge_stored = 0_u64;
        let mut battery_charge_loss = 0_u64;
        let mut battery_discharge_delivered = 0_u64;
        let mut battery_discharge_draw = 0_u64;
        let mut battery_discharge_loss = 0_u64;

        if let Some(battery) = &input.battery {
            let unmet = critical_remaining.checked_add(flexible_remaining)
                .ok_or(PortfolioError::ArithmeticOverflow)?;
            if unmet > 0 {
                let available_stored = soc - battery.minimum_reserve_wh;
                let by_power = u128::from(battery.maximum_discharge_power_w)
                    .checked_mul(u128::from(load.duration_seconds))
                    .ok_or(PortfolioError::ArithmeticOverflow)?
                    / 3600;
                let by_soc = u128::from(available_stored)
                    .checked_mul(u128::from(battery.discharge_efficiency_ppm))
                    .ok_or(PortfolioError::ArithmeticOverflow)?
                    / u128::from(EFFICIENCY_SCALE_PPM);
                let delivered = u128::from(unmet).min(by_power).min(by_soc) as u64;
                if delivered > 0 {
                    let numerator = u128::from(delivered)
                        .checked_mul(u128::from(EFFICIENCY_SCALE_PPM))
                        .ok_or(PortfolioError::ArithmeticOverflow)?;
                    let efficiency = u128::from(battery.discharge_efficiency_ppm);
                    let draw = numerator
                        .checked_add(efficiency - 1)
                        .ok_or(PortfolioError::ArithmeticOverflow)?
                        / efficiency;
                    battery_discharge_draw = u64::try_from(draw).map_err(|_| PortfolioError::ArithmeticOverflow)?;
                    if battery_discharge_draw > available_stored {
                        return Err(PortfolioError::ArithmeticOverflow);
                    }
                    battery_discharge_delivered = delivered;
                    battery_discharge_loss = battery_discharge_draw - delivered;
                    soc -= battery_discharge_draw;
                    let to_critical = delivered.min(critical_remaining);
                    critical_remaining -= to_critical;
                    flexible_remaining -= delivered - to_critical;
                }
            } else {
                let surplus = available.iter().try_fold(0_u64, |sum, value| {
                    sum.checked_add(*value).ok_or(PortfolioError::ArithmeticOverflow)
                })?;
                let headroom = battery.capacity_wh - soc;
                if surplus > 0 && headroom > 0 && battery.maximum_charge_power_w > 0 {
                    let by_power = u128::from(battery.maximum_charge_power_w)
                        .checked_mul(u128::from(load.duration_seconds))
                        .ok_or(PortfolioError::ArithmeticOverflow)?
                        / 3600;
                    let by_headroom = u128::from(headroom)
                        .checked_mul(u128::from(EFFICIENCY_SCALE_PPM))
                        .ok_or(PortfolioError::ArithmeticOverflow)?
                        / u128::from(battery.charge_efficiency_ppm);
                    let charge_input = u128::from(surplus).min(by_power).min(by_headroom) as u64;
                    battery_charge_stored = u64::try_from(
                        u128::from(charge_input)
                            .checked_mul(u128::from(battery.charge_efficiency_ppm))
                            .ok_or(PortfolioError::ArithmeticOverflow)?
                            / u128::from(EFFICIENCY_SCALE_PPM)
                    ).map_err(|_| PortfolioError::ArithmeticOverflow)?;
                    battery_charge_loss = charge_input - battery_charge_stored;
                    source_to_battery = charge_input;
                    soc = soc.checked_add(battery_charge_stored).ok_or(PortfolioError::ArithmeticOverflow)?;
                    let mut remaining_charge = charge_input;
                    for &index in &source_order {
                        let amount = available[index].min(remaining_charge);
                        available[index] -= amount;
                        remaining_charge -= amount;
                        if amount > 0 {
                            let used = source_energy_by_id.entry(input.sources[index].source_id.clone()).or_insert(0_u64);
                            *used = used.checked_add(amount).ok_or(PortfolioError::ArithmeticOverflow)?;
                            add_optional_cost(
                                &mut interval_cost,
                                input.sources[index].variable_cost_microcurrency_per_kwh,
                                amount,
                            )?;
                            add_optional_emissions(
                                &mut interval_emissions,
                                input.sources[index].emissions_grams_per_kwh,
                                amount,
                            )?;
                        }
                    }
                }
            }
        }

        let curtailed = available.iter().try_fold(0_u64, |sum, value| {
            sum.checked_add(*value).ok_or(PortfolioError::ArithmeticOverflow)
        })?;
        let critical_unserved = critical_remaining;
        let flexible_unserved = flexible_remaining;
        let battery_loss = battery_charge_loss
            .checked_add(battery_discharge_loss)
            .ok_or(PortfolioError::ArithmeticOverflow)?;
        let source_used = source_to_load.checked_add(source_to_battery)
            .ok_or(PortfolioError::ArithmeticOverflow)?;
        if source_available_wh != source_used.checked_add(curtailed).ok_or(PortfolioError::ArithmeticOverflow)? {
            return Err(PortfolioError::ArithmeticOverflow);
        }
        if load.total_energy_wh
            != source_to_load
                .checked_add(battery_discharge_delivered)
                .and_then(|value| value.checked_add(critical_unserved))
                .and_then(|value| value.checked_add(flexible_unserved))
                .ok_or(PortfolioError::ArithmeticOverflow)?
        {
            return Err(PortfolioError::ArithmeticOverflow);
        }
        if let Some(battery) = &input.battery {
            let expected_soc = battery_start_soc
                .checked_add(battery_charge_stored)
                .ok_or(PortfolioError::ArithmeticOverflow)?
                .checked_sub(battery_discharge_draw)
                .ok_or(PortfolioError::ArithmeticOverflow)?;
            if expected_soc != soc || soc < battery.minimum_reserve_wh || soc > battery.capacity_wh {
                return Err(PortfolioError::InvalidBatteryState);
            }
        }

        total_demand = total_demand.checked_add(load.total_energy_wh).ok_or(PortfolioError::ArithmeticOverflow)?;
        total_critical_unserved = total_critical_unserved.checked_add(critical_unserved).ok_or(PortfolioError::ArithmeticOverflow)?;
        total_flexible_unserved = total_flexible_unserved.checked_add(flexible_unserved).ok_or(PortfolioError::ArithmeticOverflow)?;
        total_curtailed = total_curtailed.checked_add(curtailed).ok_or(PortfolioError::ArithmeticOverflow)?;
        total_source_energy = total_source_energy.checked_add(source_used).ok_or(PortfolioError::ArithmeticOverflow)?;
        total_battery_loss = total_battery_loss.checked_add(battery_loss).ok_or(PortfolioError::ArithmeticOverflow)?;
        total_cost = add_optional_total(total_cost, interval_cost)?;
        total_emissions = add_optional_total(total_emissions, interval_emissions)?;
        if let Some(units) = load.accepted_output_units {
            accepted_units_total = accepted_units_total.checked_add(units).ok_or(PortfolioError::ArithmeticOverflow)?;
        } else {
            accepted_units_known = false;
        }

        intervals.push(IntervalBalance {
            interval_id: load.interval_id.clone(),
            source_available_wh,
            source_to_load_wh: source_to_load,
            source_to_battery_wh: source_to_battery,
            curtailed_wh: curtailed,
            critical_demand_wh: load.critical_energy_wh,
            critical_served_wh: load.critical_energy_wh - critical_unserved,
            critical_unserved_wh: critical_unserved,
            flexible_demand_wh: load.total_energy_wh - load.critical_energy_wh,
            flexible_served_wh: load.total_energy_wh - load.critical_energy_wh - flexible_unserved,
            flexible_unserved_wh: flexible_unserved,
            battery_start_soc_wh: battery_start_soc,
            battery_charge_stored_wh: battery_charge_stored,
            battery_charge_loss_wh: battery_charge_loss,
            battery_discharge_delivered_wh: battery_discharge_delivered,
            battery_discharge_storage_draw_wh: battery_discharge_draw,
            battery_discharge_loss_wh: battery_discharge_loss,
            battery_end_soc_wh: soc,
            source_energy_by_id_wh: source_energy_by_id,
            variable_cost_microcurrency: interval_cost,
            emissions_grams: interval_emissions,
        });
    }

    let disposition = if total_critical_unserved > 0 {
        PortfolioDisposition::CriticalLoadUnserved
    } else if total_flexible_unserved > 0 {
        PortfolioDisposition::FlexibleLoadUnserved
    } else {
        PortfolioDisposition::FullyServed
    };
    let load_energy_intensity = if accepted_units_known && accepted_units_total > 0 {
        Some(AcceptedOutputEnergyIntensity {
            load_energy_wh: total_demand,
            accepted_output_units: accepted_units_total,
        })
    } else {
        None
    };

    Ok(PortfolioEvaluation {
        disposition,
        intervals,
        total_demand_wh: total_demand,
        total_critical_unserved_wh: total_critical_unserved,
        total_flexible_unserved_wh: total_flexible_unserved,
        total_curtailed_wh: total_curtailed,
        total_source_energy_wh: total_source_energy,
        total_battery_loss_wh: total_battery_loss,
        total_variable_cost_microcurrency: total_cost,
        total_emissions_grams: total_emissions,
        load_energy_intensity,
    })
}

fn add_optional_cost(
    total: &mut Option<u64>,
    rate: Option<u64>,
    energy_wh: u64,
) -> Result<(), PortfolioError> {
    if energy_wh == 0 { return Ok(()); }
    let Some(rate) = rate else { *total = None; return Ok(()); };
    if let Some(current) = *total {
        let numerator = u128::from(energy_wh)
            .checked_mul(u128::from(rate))
            .ok_or(PortfolioError::ArithmeticOverflow)?;
        let value = numerator.checked_add(999).ok_or(PortfolioError::ArithmeticOverflow)? / 1000;
        let value = u64::try_from(value).map_err(|_| PortfolioError::ArithmeticOverflow)?;
        *total = Some(current.checked_add(value).ok_or(PortfolioError::ArithmeticOverflow)?);
    }
    Ok(())
}

fn add_optional_emissions(
    total: &mut Option<u64>,
    rate: Option<u64>,
    energy_wh: u64,
) -> Result<(), PortfolioError> {
    if energy_wh == 0 { return Ok(()); }
    let Some(rate) = rate else { *total = None; return Ok(()); };
    if let Some(current) = *total {
        let numerator = u128::from(energy_wh)
            .checked_mul(u128::from(rate))
            .ok_or(PortfolioError::ArithmeticOverflow)?;
        let value = numerator.checked_add(999).ok_or(PortfolioError::ArithmeticOverflow)? / 1000;
        let value = u64::try_from(value).map_err(|_| PortfolioError::ArithmeticOverflow)?;
        *total = Some(current.checked_add(value).ok_or(PortfolioError::ArithmeticOverflow)?);
    }
    Ok(())
}

fn add_optional_total(
    current: Option<u64>,
    next: Option<u64>,
) -> Result<Option<u64>, PortfolioError> {
    match (current, next) {
        (Some(a), Some(b)) => Ok(Some(a.checked_add(b).ok_or(PortfolioError::ArithmeticOverflow)?)),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(id: &str, total: u64, critical: u64) -> LoadInterval {
        LoadInterval {
            interval_id: id.into(),
            duration_seconds: 3_600,
            total_energy_wh: total,
            critical_energy_wh: critical,
            accepted_output_units: Some(100),
        }
    }

    fn source(id: &str, kind: SourceKind, priority: u16, energy: u64, power: u64) -> SupplySource {
        SupplySource {
            source_id: id.into(),
            kind,
            dispatch_priority: priority,
            maximum_power_w: power.max(energy),
            qualification: SourceQualification::VerifiedForSyntheticScenario,
            variable_cost_microcurrency_per_kwh: Some(100),
            emissions_grams_per_kwh: Some(50),
            intervals: vec![SourceInterval {
                available_energy_wh: Some(energy),
                available_power_w: Some(power.max(energy)),
            }],
        }
    }

    fn input(loads: Vec<LoadInterval>, sources: Vec<SupplySource>, battery: Option<BatteryProfile>) -> PortfolioInput {
        PortfolioInput { portfolio_id: "site-portfolio-1".into(), sources, loads, battery }
    }

    fn battery() -> BatteryProfile {
        BatteryProfile {
            capacity_wh: 10_000,
            initial_state_of_charge_wh: 5_000,
            minimum_reserve_wh: 1_000,
            maximum_charge_power_w: 5_000,
            maximum_discharge_power_w: 5_000,
            charge_efficiency_ppm: 900_000,
            discharge_efficiency_ppm: 900_000,
        }
    }

    #[test]
    fn grid_only_fully_serves_measured_load() {
        let r = evaluate_portfolio(&input(
            vec![load("t0", 5_000, 2_000)],
            vec![source("grid", SourceKind::GridImport, 0, 6_000, 6_000)],
            None,
        )).unwrap();
        assert_eq!(r.disposition, PortfolioDisposition::FullyServed);
        assert_eq!(r.total_critical_unserved_wh, 0);
        assert_eq!(r.total_flexible_unserved_wh, 0);
    }

    #[test]
    fn battery_discharge_serves_load_after_declared_sources() {
        let mut b = battery();
        b.initial_state_of_charge_wh = 5_000;
        let r = evaluate_portfolio(&input(
            vec![load("t0", 6_000, 3_000)],
            vec![source("grid", SourceKind::GridImport, 0, 3_000, 3_000)],
            Some(b),
        )).unwrap();
        assert_eq!(r.disposition, PortfolioDisposition::FullyServed);
        assert_eq!(r.intervals[0].battery_discharge_delivered_wh, 3_000);
        assert_eq!(r.intervals[0].battery_discharge_storage_draw_wh, 3_334);
        assert_eq!(r.intervals[0].battery_end_soc_wh, 1_666);
    }

    #[test]
    fn minimum_battery_reserve_is_never_spent() {
        let mut b = battery();
        b.initial_state_of_charge_wh = 2_000;
        b.minimum_reserve_wh = 1_500;
        let r = evaluate_portfolio(&input(
            vec![load("t0", 2_000, 2_000)],
            vec![source("grid", SourceKind::GridImport, 0, 1_000, 1_000)],
            Some(b),
        )).unwrap();
        assert_eq!(r.disposition, PortfolioDisposition::CriticalLoadUnserved);
        assert_eq!(r.intervals[0].battery_end_soc_wh, 1_500);
        assert_eq!(r.total_critical_unserved_wh, 550);
    }

    #[test]
    fn renewable_surplus_charges_battery_and_reconciles_curtailment() {
        let mut b = battery();
        b.initial_state_of_charge_wh = 8_000;
        b.maximum_charge_power_w = 2_000;
        let r = evaluate_portfolio(&input(
            vec![load("t0", 2_000, 2_000)],
            vec![source("pv", SourceKind::RenewableGeneration, 0, 6_000, 6_000)],
            Some(b),
        )).unwrap();
        assert_eq!(r.intervals[0].source_to_load_wh, 2_000);
        assert_eq!(r.intervals[0].source_to_battery_wh, 2_000);
        assert_eq!(r.intervals[0].battery_charge_stored_wh, 1_800);
        assert_eq!(r.intervals[0].battery_end_soc_wh, 9_800);
        assert_eq!(r.intervals[0].curtailed_wh, 2_000);
        assert_eq!(r.disposition, PortfolioDisposition::FullyServed);
    }

    #[test]
    fn battery_shifts_energy_across_intervals_with_exact_soc_reconciliation() {
        let mut b = battery();
        b.capacity_wh = 5_000;
        b.initial_state_of_charge_wh = 0;
        b.minimum_reserve_wh = 0;
        b.charge_efficiency_ppm = EFFICIENCY_SCALE_PPM;
        b.discharge_efficiency_ppm = EFFICIENCY_SCALE_PPM;
        b.maximum_charge_power_w = 4_000;
        b.maximum_discharge_power_w = 4_000;

        let mut grid = source("grid", SourceKind::GridImport, 0, 4_000, 4_000);
        grid.intervals = vec![
            SourceInterval {
                available_energy_wh: Some(4_000),
                available_power_w: Some(4_000),
            },
            SourceInterval {
                available_energy_wh: Some(0),
                available_power_w: Some(0),
            },
        ];
        let loads = vec![
            LoadInterval {
                interval_id: "charge".into(),
                duration_seconds: 3_600,
                total_energy_wh: 1_000,
                critical_energy_wh: 1_000,
                accepted_output_units: Some(1),
            },
            LoadInterval {
                interval_id: "discharge".into(),
                duration_seconds: 3_600,
                total_energy_wh: 2_500,
                critical_energy_wh: 2_500,
                accepted_output_units: Some(1),
            },
        ];
        let r = evaluate_portfolio(&input(loads, vec![grid], Some(b))).unwrap();
        assert_eq!(r.disposition, PortfolioDisposition::FullyServed);
        assert_eq!(r.intervals[0].battery_charge_stored_wh, 3_000);
        assert_eq!(r.intervals[0].battery_end_soc_wh, 3_000);
        assert_eq!(r.intervals[1].battery_discharge_delivered_wh, 2_500);
        assert_eq!(r.intervals[1].battery_end_soc_wh, 500);
        assert_eq!(r.total_source_energy_wh, 7_000);
        assert_eq!(r.total_battery_loss_wh, 0);
    }

    #[test]
    fn no_simultaneous_charge_and_discharge() {
        let mut b = battery();
        b.initial_state_of_charge_wh = 3_000;
        let r = evaluate_portfolio(&input(
            vec![load("t0", 1_000, 1_000)],
            vec![source("pv", SourceKind::RenewableGeneration, 0, 3_000, 3_000)],
            Some(b),
        )).unwrap();
        assert_eq!(r.intervals[0].battery_discharge_delivered_wh, 0);
        assert_eq!(r.intervals[0].battery_charge_stored_wh, 1_800);
    }

    #[test]
    fn critical_load_is_served_before_flexible_load() {
        let r = evaluate_portfolio(&input(
            vec![load("t0", 5_000, 4_000)],
            vec![source("grid", SourceKind::GridImport, 0, 3_000, 3_000)],
            None,
        )).unwrap();
        assert_eq!(r.disposition, PortfolioDisposition::CriticalLoadUnserved);
        assert_eq!(r.intervals[0].critical_served_wh, 3_000);
        assert_eq!(r.intervals[0].flexible_served_wh, 0);
        assert_eq!(r.intervals[0].critical_unserved_wh, 1_000);
    }

    #[test]
    fn source_priority_is_explicit_and_deterministic() {
        let r = evaluate_portfolio(&input(
            vec![load("t0", 4_000, 4_000)],
            vec![
                source("diesel", SourceKind::ConventionalGeneration, 2, 3_000, 3_000),
                source("grid", SourceKind::GridImport, 0, 3_000, 3_000),
            ],
            None,
        )).unwrap();
        assert_eq!(r.intervals[0].source_energy_by_id_wh.get("grid"), Some(&3_000));
        assert_eq!(r.intervals[0].source_energy_by_id_wh.get("diesel"), Some(&1_000));
    }

    #[test]
    fn unknown_source_energy_is_not_interpreted_as_zero_or_available() {
        let mut s = source("external-firm", SourceKind::ExternalFirmSupply, 0, 1_000, 1_000);
        s.intervals[0].available_energy_wh = None;
        assert_eq!(
            evaluate_portfolio(&input(vec![load("t0", 1_000, 1_000)], vec![s], None)),
            Err(PortfolioError::UnknownSourceAvailability)
        );
    }

    #[test]
    fn unverified_external_firm_supply_cannot_be_counted() {
        let mut s = source("hypothetical-smr", SourceKind::ExternalFirmSupply, 0, 10_000, 10_000);
        s.qualification = SourceQualification::ContractOrConnectionUnverified;
        assert_eq!(
            evaluate_portfolio(&input(vec![load("t0", 5_000, 5_000)], vec![s], None)),
            Err(PortfolioError::ExternalSupplyNotQualified)
        );
    }

    #[test]
    fn missing_variable_cost_remains_unknown_not_zero() {
        let mut s = source("grid", SourceKind::GridImport, 0, 1_000, 1_000);
        s.variable_cost_microcurrency_per_kwh = None;
        let r = evaluate_portfolio(&input(vec![load("t0", 1_000, 1_000)], vec![s], None)).unwrap();
        assert_eq!(r.total_variable_cost_microcurrency, None);
    }

    #[test]
    fn missing_emissions_factor_remains_unknown_not_zero() {
        let mut s = source("grid", SourceKind::GridImport, 0, 1_000, 1_000);
        s.emissions_grams_per_kwh = None;
        let r = evaluate_portfolio(&input(vec![load("t0", 1_000, 1_000)], vec![s], None)).unwrap();
        assert_eq!(r.total_emissions_grams, None);
    }

    #[test]
    fn source_energy_cannot_exceed_interval_power_capacity() {
        let mut s = source("grid", SourceKind::GridImport, 0, 2_000, 1_000);
        s.maximum_power_w = 1_000;
        s.intervals[0].available_power_w = Some(1_000);
        assert_eq!(
            evaluate_portfolio(&input(vec![load("t0", 1_000, 1_000)], vec![s], None)),
            Err(PortfolioError::SourceEnergyExceedsPowerEnvelope)
        );
    }

    #[test]
    fn source_power_cannot_exceed_declared_maximum() {
        let mut s = source("grid", SourceKind::GridImport, 0, 1_000, 1_000);
        s.maximum_power_w = 500;
        assert_eq!(
            evaluate_portfolio(&input(vec![load("t0", 1_000, 1_000)], vec![s], None)),
            Err(PortfolioError::InvalidSourcePowerEnvelope)
        );
    }

    #[test]
    fn invalid_battery_reserve_or_soc_is_rejected() {
        let mut b = battery();
        b.minimum_reserve_wh = 6_000;
        assert_eq!(
            evaluate_portfolio(&input(vec![load("t0", 1_000, 1_000)], vec![], Some(b))),
            Err(PortfolioError::InvalidBatteryState)
        );
    }

    #[test]
    fn invalid_battery_efficiency_is_rejected() {
        let mut b = battery();
        b.discharge_efficiency_ppm = 0;
        assert_eq!(
            evaluate_portfolio(&input(vec![load("t0", 1_000, 1_000)], vec![], Some(b))),
            Err(PortfolioError::InvalidBatteryEfficiency)
        );
    }

    #[test]
    fn interval_count_mismatch_is_rejected() {
        let mut s = source("grid", SourceKind::GridImport, 0, 1_000, 1_000);
        s.intervals.clear();
        assert_eq!(
            evaluate_portfolio(&input(vec![load("t0", 1_000, 1_000)], vec![s], None)),
            Err(PortfolioError::SourceIntervalCountMismatch)
        );
    }

    #[test]
    fn duplicate_source_identity_is_rejected() {
        let a = source("grid", SourceKind::GridImport, 0, 1_000, 1_000);
        let b = a.clone();
        assert_eq!(
            evaluate_portfolio(&input(vec![load("t0", 1_000, 1_000)], vec![a, b], None)),
            Err(PortfolioError::DuplicateSource)
        );
    }

    #[test]
    fn exact_load_energy_ratio_is_reported_only_with_complete_denominator() {
        let r = evaluate_portfolio(&input(
            vec![load("t0", 10_000, 5_000)],
            vec![source("grid", SourceKind::GridImport, 0, 10_000, 10_000)],
            None,
        )).unwrap();
        assert_eq!(
            r.load_energy_intensity,
            Some(AcceptedOutputEnergyIntensity {
                load_energy_wh: 10_000,
                accepted_output_units: 100,
            })
        );
    }

    #[test]
    fn missing_accepted_output_denominator_is_not_zero_filled() {
        let mut l = load("t0", 10_000, 5_000);
        l.accepted_output_units = None;
        let r = evaluate_portfolio(&input(
            vec![l],
            vec![source("grid", SourceKind::GridImport, 0, 10_000, 10_000)],
            None,
        )).unwrap();
        assert_eq!(r.load_energy_intensity, None);
    }

    #[test]
    fn unserved_load_and_source_surplus_reconcile() {
        let r = evaluate_portfolio(&input(
            vec![load("t0", 5_000, 2_000)],
            vec![source("grid", SourceKind::GridImport, 0, 3_000, 3_000)],
            None,
        )).unwrap();
        let balance = &r.intervals[0];
        assert_eq!(balance.source_available_wh, balance.source_to_load_wh
            + balance.source_to_battery_wh + balance.curtailed_wh);
        assert_eq!(balance.critical_demand_wh, balance.critical_served_wh + balance.critical_unserved_wh);
        assert_eq!(balance.flexible_demand_wh, balance.flexible_served_wh + balance.flexible_unserved_wh);
    }
}
