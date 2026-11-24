#![forbid(unsafe_code)]

//! SAFRB v0.6: multi-carrier, storage-aware service-boundary accounting.
//!
//! This evaluates a supplied per-period energy schedule for one or more
//! service zones. It is not a power-flow solver, an optimizer, a grid model,
//! or an operational controller. Electricity quantities are net Wh at a
//! named service boundary; fuel-to-electricity conversion is upstream.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EvidenceClass {
    Measured,
    Reported,
    Derived,
    Forecast,
    Scenario,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Evidence {
    pub source_id: String,
    pub source_uri: String,
    pub evidence_class: EvidenceClass,
    /// A SHA-256 reference whose shape is checked; raw bytes are not hashed here.
    pub raw_sha256: Option<String>,
}

impl Evidence {
    pub fn validate(&self) -> Result<(), String> {
        if self.source_id.trim().is_empty() || self.source_uri.trim().is_empty() {
            return Err("evidence source_id and source_uri must not be empty".into());
        }
        if let Some(digest) = &self.raw_sha256
            && (digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
        {
            return Err("raw_sha256 must be exactly 64 hexadecimal characters".into());
        }
        Ok(())
    }

    fn qualifies_for_simulation(&self) -> bool {
        match self.evidence_class {
            EvidenceClass::Measured => self.raw_sha256.as_ref().is_some_and(|digest| {
                digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            }),
            EvidenceClass::Scenario => self.source_uri.starts_with("urn:safrb:fixture:"),
            EvidenceClass::Reported | EvidenceClass::Derived | EvidenceClass::Forecast => false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnergyCarrier {
    GridImport,
    ShorePower,
    DieselGenerator,
    SolarPv,
    WindTurbine,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElectricSupply {
    pub id: String,
    pub period: u16,
    pub service_zone_id: String,
    pub carrier: EnergyCarrier,
    /// Net electrical energy available at the named bus boundary.
    pub available_lower_wh: u128,
    pub available_upper_wh: u128,
    pub maximum_power_w: u64,
    /// Links to a metering receipt, v0.5 generator result, or other upstream artifact.
    pub upstream_artifact_id: String,
    pub evidence: Evidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Battery {
    pub id: String,
    pub service_zone_id: String,
    pub capacity_wh: u128,
    pub opening_soc_lower_wh: u128,
    pub opening_soc_upper_wh: u128,
    pub maximum_charge_power_w: u64,
    pub maximum_discharge_power_w: u64,
    pub charge_efficiency_lower_bps: u16,
    pub charge_efficiency_upper_bps: u16,
    pub discharge_efficiency_lower_bps: u16,
    pub discharge_efficiency_upper_bps: u16,
    pub state_evidence: Evidence,
    pub parameter_evidence: Evidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BatteryMode {
    Idle,
    /// Electrical input consumed at the service bus before charge losses.
    Charge {
        input_wh: u128,
    },
    /// Chemical/electrical energy removed from the battery's stored-energy state.
    Discharge {
        battery_draw_wh: u128,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatteryAction {
    pub period: u16,
    pub battery_id: String,
    pub mode: BatteryMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceDemand {
    pub id: String,
    pub period: u16,
    pub service_zone_id: String,
    /// Lower value means higher service priority.
    pub priority: u16,
    pub demand_wh: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Problem {
    pub scenario_id: String,
    pub periods: u16,
    pub period_duration_minutes: u16,
    pub supplies: Vec<ElectricSupply>,
    pub batteries: Vec<Battery>,
    pub battery_actions: Vec<BatteryAction>,
    pub demands: Vec<ServiceDemand>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceRecord {
    pub source_id: String,
    pub period: u16,
    pub service_zone_id: String,
    pub carrier: EnergyCarrier,
    pub available_lower_wh: u128,
    pub available_upper_wh: u128,
    pub upstream_artifact_id: String,
    pub evidence: Evidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatteryPeriodResult {
    pub period: u16,
    pub battery_id: String,
    pub mode: BatteryMode,
    pub opening_soc_lower_wh: u128,
    pub opening_soc_upper_wh: u128,
    pub charge_input_wh: u128,
    pub stored_lower_wh: u128,
    pub stored_upper_wh: u128,
    pub battery_draw_wh: u128,
    pub bus_output_lower_wh: u128,
    pub bus_output_upper_wh: u128,
    pub closing_soc_lower_wh: u128,
    pub closing_soc_upper_wh: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceAllocation {
    pub demand_id: String,
    pub period: u16,
    pub service_zone_id: String,
    pub priority: u16,
    pub requested_wh: u128,
    pub served_lower_wh: u128,
    pub served_upper_wh: u128,
    pub unserved_lower_wh: u128,
    pub unserved_upper_wh: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZonePeriodResult {
    pub period: u16,
    pub service_zone_id: String,
    pub source_lower_wh: u128,
    pub source_upper_wh: u128,
    pub battery_output_lower_wh: u128,
    pub battery_output_upper_wh: u128,
    pub battery_charge_input_wh: u128,
    pub served_lower_wh: u128,
    pub served_upper_wh: u128,
    pub unserved_lower_wh: u128,
    pub unserved_upper_wh: u128,
    pub curtailed_lower_wh: u128,
    pub curtailed_upper_wh: u128,
    pub bus_balance_residual_lower_wh: i128,
    pub bus_balance_residual_upper_wh: i128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Summary {
    pub scenario_id: String,
    pub qualified_source_lower_wh: u128,
    pub qualified_source_upper_wh: u128,
    pub excluded_source_upper_wh: u128,
    pub total_demand_wh: u128,
    pub served_lower_wh: u128,
    pub served_upper_wh: u128,
    pub unserved_lower_wh: u128,
    pub unserved_upper_wh: u128,
    pub curtailed_lower_wh: u128,
    pub curtailed_upper_wh: u128,
    pub source_records: Vec<SourceRecord>,
    pub battery_periods: Vec<BatteryPeriodResult>,
    pub service_allocations: Vec<ServiceAllocation>,
    pub zone_periods: Vec<ZonePeriodResult>,
}

fn add_u128(target: &mut u128, amount: u128, label: &str) -> Result<(), String> {
    *target = target
        .checked_add(amount)
        .ok_or_else(|| format!("{label} overflowed u128"))?;
    Ok(())
}

fn multiply_u128(left: u128, right: u128, label: &str) -> Result<u128, String> {
    left.checked_mul(right)
        .ok_or_else(|| format!("{label} overflowed u128"))
}

fn energy_for_power(power_w: u64, period_minutes: u16) -> Result<u128, String> {
    multiply_u128(
        u128::from(power_w),
        u128::from(period_minutes),
        "power-to-energy conversion",
    )
    .map(|value| value / 60)
}

fn efficiency_output(input_wh: u128, efficiency_bps: u16, label: &str) -> Result<u128, String> {
    multiply_u128(input_wh, u128::from(efficiency_bps), label).map(|value| value / 10_000)
}

impl Problem {
    pub fn validate(&self) -> Result<(), String> {
        if self.scenario_id.trim().is_empty() {
            return Err("scenario_id must not be empty".into());
        }
        if self.periods == 0 {
            return Err("periods must be positive".into());
        }
        if self.period_duration_minutes == 0 || self.period_duration_minutes > 1_440 {
            return Err("period_duration_minutes must be in 1..=1440".into());
        }

        let mut source_ids = BTreeSet::new();
        for source in &self.supplies {
            if source.id.trim().is_empty()
                || source.service_zone_id.trim().is_empty()
                || source.upstream_artifact_id.trim().is_empty()
            {
                return Err("source id, zone and upstream artifact must not be empty".into());
            }
            if !source_ids.insert(source.id.as_str()) {
                return Err(format!("duplicate electric supply id: {}", source.id));
            }
            if source.period >= self.periods {
                return Err(format!("source {} is outside the horizon", source.id));
            }
            if source.available_lower_wh > source.available_upper_wh {
                return Err(format!("source {} has inverted energy bounds", source.id));
            }
            let maximum_wh =
                energy_for_power(source.maximum_power_w, self.period_duration_minutes)?;
            if source.available_upper_wh > maximum_wh {
                return Err(format!(
                    "source {} exceeds its maximum power envelope",
                    source.id
                ));
            }
            source.evidence.validate()?;
        }

        let mut battery_ids = BTreeSet::new();
        for battery in &self.batteries {
            if battery.id.trim().is_empty() || battery.service_zone_id.trim().is_empty() {
                return Err("battery id and service zone must not be empty".into());
            }
            if !battery_ids.insert(battery.id.as_str()) {
                return Err(format!("duplicate battery id: {}", battery.id));
            }
            if battery.capacity_wh == 0
                || battery.opening_soc_lower_wh > battery.opening_soc_upper_wh
                || battery.opening_soc_upper_wh > battery.capacity_wh
            {
                return Err(format!(
                    "battery {} has invalid capacity or opening SOC",
                    battery.id
                ));
            }
            if battery.maximum_charge_power_w == 0 || battery.maximum_discharge_power_w == 0 {
                return Err(format!(
                    "battery {} power limits must be positive",
                    battery.id
                ));
            }
            if battery.charge_efficiency_lower_bps == 0
                || battery.charge_efficiency_lower_bps > battery.charge_efficiency_upper_bps
                || battery.charge_efficiency_upper_bps > 10_000
                || battery.discharge_efficiency_lower_bps == 0
                || battery.discharge_efficiency_lower_bps > battery.discharge_efficiency_upper_bps
                || battery.discharge_efficiency_upper_bps > 10_000
            {
                return Err(format!(
                    "battery {} has invalid efficiency bounds",
                    battery.id
                ));
            }
            battery.state_evidence.validate()?;
            battery.parameter_evidence.validate()?;
            if !battery.state_evidence.qualifies_for_simulation()
                || !battery.parameter_evidence.qualifies_for_simulation()
            {
                return Err(format!(
                    "battery {} has unqualified state or parameter evidence",
                    battery.id
                ));
            }
        }

        let battery_by_id: BTreeMap<&str, &Battery> = self
            .batteries
            .iter()
            .map(|battery| (battery.id.as_str(), battery))
            .collect();
        let mut action_keys = BTreeSet::new();
        for action in &self.battery_actions {
            if action.period >= self.periods {
                return Err(format!(
                    "battery action for {} is outside the horizon",
                    action.battery_id
                ));
            }
            let battery = battery_by_id
                .get(action.battery_id.as_str())
                .ok_or_else(|| {
                    format!("action references unknown battery {}", action.battery_id)
                })?;
            if !action_keys.insert((action.period, action.battery_id.as_str())) {
                return Err(format!(
                    "duplicate battery action for {} period {}",
                    action.battery_id, action.period
                ));
            }
            match &action.mode {
                BatteryMode::Idle => {}
                BatteryMode::Charge { input_wh } => {
                    if *input_wh == 0 {
                        return Err(format!(
                            "battery {} charge input must be positive",
                            battery.id
                        ));
                    }
                    if !battery.state_evidence.qualifies_for_simulation()
                        || !battery.parameter_evidence.qualifies_for_simulation()
                    {
                        return Err(format!(
                            "battery {} has unqualified state or parameter evidence",
                            battery.id
                        ));
                    }
                    let maximum = energy_for_power(
                        battery.maximum_charge_power_w,
                        self.period_duration_minutes,
                    )?;
                    if *input_wh > maximum {
                        return Err(format!("battery {} charge exceeds power limit", battery.id));
                    }
                }
                BatteryMode::Discharge { battery_draw_wh } => {
                    if *battery_draw_wh == 0 {
                        return Err(format!(
                            "battery {} discharge draw must be positive",
                            battery.id
                        ));
                    }
                    if !battery.state_evidence.qualifies_for_simulation()
                        || !battery.parameter_evidence.qualifies_for_simulation()
                    {
                        return Err(format!(
                            "battery {} has unqualified state or parameter evidence",
                            battery.id
                        ));
                    }
                    let maximum = energy_for_power(
                        battery.maximum_discharge_power_w,
                        self.period_duration_minutes,
                    )?;
                    if *battery_draw_wh > maximum {
                        return Err(format!(
                            "battery {} discharge exceeds power limit",
                            battery.id
                        ));
                    }
                }
            }
        }

        let mut demand_ids = BTreeSet::new();
        for demand in &self.demands {
            if demand.id.trim().is_empty() || demand.service_zone_id.trim().is_empty() {
                return Err("demand id and service zone must not be empty".into());
            }
            if !demand_ids.insert(demand.id.as_str()) {
                return Err(format!("duplicate service demand id: {}", demand.id));
            }
            if demand.period >= self.periods {
                return Err(format!("demand {} is outside the horizon", demand.id));
            }
            if demand.demand_wh == 0 {
                return Err(format!("demand {} must be positive", demand.id));
            }
        }

        Ok(())
    }

    pub fn evaluate(&self) -> Result<Summary, String> {
        self.validate()?;

        let mut zone_ids = BTreeSet::<String>::new();
        for source in &self.supplies {
            zone_ids.insert(source.service_zone_id.clone());
        }
        for battery in &self.batteries {
            zone_ids.insert(battery.service_zone_id.clone());
        }
        for demand in &self.demands {
            zone_ids.insert(demand.service_zone_id.clone());
        }

        let mut battery_soc = self
            .batteries
            .iter()
            .map(|battery| {
                (
                    battery.id.clone(),
                    (battery.opening_soc_lower_wh, battery.opening_soc_upper_wh),
                )
            })
            .collect::<BTreeMap<_, _>>();

        let mut qualified_source_lower_wh = 0_u128;
        let mut qualified_source_upper_wh = 0_u128;
        let mut excluded_source_upper_wh = 0_u128;
        let mut total_demand_wh = 0_u128;
        let mut served_lower_wh = 0_u128;
        let mut served_upper_wh = 0_u128;
        let mut unserved_lower_wh = 0_u128;
        let mut unserved_upper_wh = 0_u128;
        let mut curtailed_lower_wh = 0_u128;
        let mut curtailed_upper_wh = 0_u128;
        let mut source_records = Vec::new();
        let mut battery_periods = Vec::new();
        let mut service_allocations = Vec::new();
        let mut zone_periods = Vec::new();

        for source in &self.supplies {
            let record = SourceRecord {
                source_id: source.id.clone(),
                period: source.period,
                service_zone_id: source.service_zone_id.clone(),
                carrier: source.carrier,
                available_lower_wh: source.available_lower_wh,
                available_upper_wh: source.available_upper_wh,
                upstream_artifact_id: source.upstream_artifact_id.clone(),
                evidence: source.evidence.clone(),
            };
            if source.evidence.qualifies_for_simulation() {
                add_u128(
                    &mut qualified_source_lower_wh,
                    source.available_lower_wh,
                    "qualified source lower",
                )?;
                add_u128(
                    &mut qualified_source_upper_wh,
                    source.available_upper_wh,
                    "qualified source upper",
                )?;
            } else {
                add_u128(
                    &mut excluded_source_upper_wh,
                    source.available_upper_wh,
                    "excluded source upper",
                )?;
            }
            source_records.push(record);
        }

        for period in 0..self.periods {
            for zone_id in &zone_ids {
                let mut source_lower_wh = 0_u128;
                let mut source_upper_wh = 0_u128;
                for source in self.supplies.iter().filter(|source| {
                    source.period == period
                        && source.service_zone_id == *zone_id
                        && source.evidence.qualifies_for_simulation()
                }) {
                    add_u128(
                        &mut source_lower_wh,
                        source.available_lower_wh,
                        "period source lower",
                    )?;
                    add_u128(
                        &mut source_upper_wh,
                        source.available_upper_wh,
                        "period source upper",
                    )?;
                }

                let mut battery_output_lower_wh = 0_u128;
                let mut battery_output_upper_wh = 0_u128;
                let mut battery_charge_input_wh = 0_u128;

                for battery in self
                    .batteries
                    .iter()
                    .filter(|battery| battery.service_zone_id == *zone_id)
                {
                    let (opening_lower, opening_upper) = battery_soc[&battery.id];
                    let action = self
                        .battery_actions
                        .iter()
                        .find(|action| action.period == period && action.battery_id == battery.id)
                        .map(|action| action.mode.clone())
                        .unwrap_or(BatteryMode::Idle);
                    let mut charge_input_wh = 0_u128;
                    let mut stored_lower_wh = 0_u128;
                    let mut stored_upper_wh = 0_u128;
                    let mut battery_draw_wh = 0_u128;
                    let mut bus_output_lower_wh = 0_u128;
                    let mut bus_output_upper_wh = 0_u128;
                    let mut closing_lower = opening_lower;
                    let mut closing_upper = opening_upper;

                    match action {
                        BatteryMode::Idle => {}
                        BatteryMode::Charge { input_wh } => {
                            charge_input_wh = input_wh;
                            stored_lower_wh = efficiency_output(
                                input_wh,
                                battery.charge_efficiency_lower_bps,
                                "battery stored lower",
                            )?;
                            stored_upper_wh = efficiency_output(
                                input_wh,
                                battery.charge_efficiency_upper_bps,
                                "battery stored upper",
                            )?;
                            closing_lower = opening_lower
                                .checked_add(stored_lower_wh)
                                .ok_or_else(|| "battery lower SOC overflowed u128".to_string())?;
                            closing_upper = opening_upper
                                .checked_add(stored_upper_wh)
                                .ok_or_else(|| "battery upper SOC overflowed u128".to_string())?;
                            if closing_upper > battery.capacity_wh {
                                return Err(format!(
                                    "battery {} exceeds energy capacity",
                                    battery.id
                                ));
                            }
                            add_u128(
                                &mut battery_charge_input_wh,
                                input_wh,
                                "battery charge input",
                            )?;
                        }
                        BatteryMode::Discharge {
                            battery_draw_wh: draw,
                        } => {
                            if draw > opening_lower {
                                return Err(format!(
                                    "battery {} discharge exceeds conservative opening SOC",
                                    battery.id
                                ));
                            }
                            battery_draw_wh = draw;
                            bus_output_lower_wh = efficiency_output(
                                draw,
                                battery.discharge_efficiency_lower_bps,
                                "battery bus output lower",
                            )?;
                            bus_output_upper_wh = efficiency_output(
                                draw,
                                battery.discharge_efficiency_upper_bps,
                                "battery bus output upper",
                            )?;
                            closing_lower = opening_lower - draw;
                            closing_upper = opening_upper - draw;
                            add_u128(
                                &mut battery_output_lower_wh,
                                bus_output_lower_wh,
                                "battery output lower",
                            )?;
                            add_u128(
                                &mut battery_output_upper_wh,
                                bus_output_upper_wh,
                                "battery output upper",
                            )?;
                        }
                    }

                    battery_soc.insert(battery.id.clone(), (closing_lower, closing_upper));
                    battery_periods.push(BatteryPeriodResult {
                        period,
                        battery_id: battery.id.clone(),
                        mode: action,
                        opening_soc_lower_wh: opening_lower,
                        opening_soc_upper_wh: opening_upper,
                        charge_input_wh,
                        stored_lower_wh,
                        stored_upper_wh,
                        battery_draw_wh,
                        bus_output_lower_wh,
                        bus_output_upper_wh,
                        closing_soc_lower_wh: closing_lower,
                        closing_soc_upper_wh: closing_upper,
                    });
                }

                let gross_bus_lower = source_lower_wh
                    .checked_add(battery_output_lower_wh)
                    .ok_or_else(|| "lower bus energy overflowed u128".to_string())?;
                let gross_bus_upper = source_upper_wh
                    .checked_add(battery_output_upper_wh)
                    .ok_or_else(|| "upper bus energy overflowed u128".to_string())?;
                if battery_charge_input_wh > gross_bus_lower {
                    return Err(format!(
                        "zone {} period {} battery charge exceeds guaranteed bus supply",
                        zone_id, period
                    ));
                }
                let mut remaining_lower = gross_bus_lower - battery_charge_input_wh;
                let mut remaining_upper = gross_bus_upper
                    .checked_sub(battery_charge_input_wh)
                    .ok_or_else(|| "battery charge exceeds upper bus supply".to_string())?;

                let mut day_demands: Vec<&ServiceDemand> = self
                    .demands
                    .iter()
                    .filter(|demand| demand.period == period && demand.service_zone_id == *zone_id)
                    .collect();
                day_demands.sort_by(|left, right| {
                    (left.priority, left.id.as_str()).cmp(&(right.priority, right.id.as_str()))
                });

                let mut zone_served_lower = 0_u128;
                let mut zone_served_upper = 0_u128;
                let mut zone_unserved_lower = 0_u128;
                let mut zone_unserved_upper = 0_u128;

                for demand in day_demands {
                    let lower = demand.demand_wh.min(remaining_lower);
                    let upper = demand.demand_wh.min(remaining_upper);
                    remaining_lower -= lower;
                    remaining_upper -= upper;
                    let unserved_lower = demand.demand_wh - upper;
                    let unserved_upper = demand.demand_wh - lower;

                    add_u128(
                        &mut total_demand_wh,
                        demand.demand_wh,
                        "total service demand",
                    )?;
                    add_u128(&mut served_lower_wh, lower, "total served lower")?;
                    add_u128(&mut served_upper_wh, upper, "total served upper")?;
                    add_u128(
                        &mut unserved_lower_wh,
                        unserved_lower,
                        "total unserved lower",
                    )?;
                    add_u128(
                        &mut unserved_upper_wh,
                        unserved_upper,
                        "total unserved upper",
                    )?;
                    add_u128(&mut zone_served_lower, lower, "zone served lower")?;
                    add_u128(&mut zone_served_upper, upper, "zone served upper")?;
                    add_u128(
                        &mut zone_unserved_lower,
                        unserved_lower,
                        "zone unserved lower",
                    )?;
                    add_u128(
                        &mut zone_unserved_upper,
                        unserved_upper,
                        "zone unserved upper",
                    )?;

                    service_allocations.push(ServiceAllocation {
                        demand_id: demand.id.clone(),
                        period,
                        service_zone_id: demand.service_zone_id.clone(),
                        priority: demand.priority,
                        requested_wh: demand.demand_wh,
                        served_lower_wh: lower,
                        served_upper_wh: upper,
                        unserved_lower_wh: unserved_lower,
                        unserved_upper_wh: unserved_upper,
                    });
                }

                let curtailed_zone_lower = remaining_lower;
                let curtailed_zone_upper = remaining_upper;
                add_u128(
                    &mut curtailed_lower_wh,
                    curtailed_zone_lower,
                    "total curtailed lower",
                )?;
                add_u128(
                    &mut curtailed_upper_wh,
                    curtailed_zone_upper,
                    "total curtailed upper",
                )?;

                let residual_lower = i128::try_from(gross_bus_lower)
                    .map_err(|_| "lower bus total exceeds i128".to_string())?
                    - i128::try_from(battery_charge_input_wh)
                        .map_err(|_| "battery charge exceeds i128".to_string())?
                    - i128::try_from(zone_served_lower)
                        .map_err(|_| "served lower exceeds i128".to_string())?
                    - i128::try_from(curtailed_zone_lower)
                        .map_err(|_| "curtailed lower exceeds i128".to_string())?;
                let residual_upper = i128::try_from(gross_bus_upper)
                    .map_err(|_| "upper bus total exceeds i128".to_string())?
                    - i128::try_from(battery_charge_input_wh)
                        .map_err(|_| "battery charge exceeds i128".to_string())?
                    - i128::try_from(zone_served_upper)
                        .map_err(|_| "served upper exceeds i128".to_string())?
                    - i128::try_from(curtailed_zone_upper)
                        .map_err(|_| "curtailed upper exceeds i128".to_string())?;
                if residual_lower != 0 || residual_upper != 0 {
                    return Err(format!(
                        "zone {} period {} bus energy does not conserve",
                        zone_id, period
                    ));
                }

                zone_periods.push(ZonePeriodResult {
                    period,
                    service_zone_id: zone_id.clone(),
                    source_lower_wh,
                    source_upper_wh,
                    battery_output_lower_wh,
                    battery_output_upper_wh,
                    battery_charge_input_wh,
                    served_lower_wh: zone_served_lower,
                    served_upper_wh: zone_served_upper,
                    unserved_lower_wh: zone_unserved_lower,
                    unserved_upper_wh: zone_unserved_upper,
                    curtailed_lower_wh: curtailed_zone_lower,
                    curtailed_upper_wh: curtailed_zone_upper,
                    bus_balance_residual_lower_wh: residual_lower,
                    bus_balance_residual_upper_wh: residual_upper,
                });
            }
        }

        Ok(Summary {
            scenario_id: self.scenario_id.clone(),
            qualified_source_lower_wh,
            qualified_source_upper_wh,
            excluded_source_upper_wh,
            total_demand_wh,
            served_lower_wh,
            served_upper_wh,
            unserved_lower_wh,
            unserved_upper_wh,
            curtailed_lower_wh,
            curtailed_upper_wh,
            source_records,
            battery_periods,
            service_allocations,
            zone_periods,
        })
    }
}

fn fixture_evidence(uri: &str) -> Evidence {
    Evidence {
        source_id: "synthetic-fixture".into(),
        source_uri: uri.into(),
        evidence_class: EvidenceClass::Scenario,
        raw_sha256: None,
    }
}

pub fn base_case() -> Problem {
    Problem {
        scenario_id: "synthetic-port-multicarrier-service".into(),
        periods: 2,
        period_duration_minutes: 60,
        supplies: vec![ElectricSupply {
            id: "solar-period-0".into(),
            period: 0,
            service_zone_id: "port-critical-load".into(),
            carrier: EnergyCarrier::SolarPv,
            available_lower_wh: 1_000,
            available_upper_wh: 1_200,
            maximum_power_w: 2_000,
            upstream_artifact_id: "pv-profile-fixture-001".into(),
            evidence: fixture_evidence("urn:safrb:fixture:solar-period-0"),
        }],
        batteries: vec![Battery {
            id: "battery-001".into(),
            service_zone_id: "port-critical-load".into(),
            capacity_wh: 1_000,
            opening_soc_lower_wh: 100,
            opening_soc_upper_wh: 100,
            maximum_charge_power_w: 2_000,
            maximum_discharge_power_w: 2_000,
            charge_efficiency_lower_bps: 9_000,
            charge_efficiency_upper_bps: 9_500,
            discharge_efficiency_lower_bps: 9_000,
            discharge_efficiency_upper_bps: 9_500,
            state_evidence: fixture_evidence("urn:safrb:fixture:battery-state-001"),
            parameter_evidence: fixture_evidence("urn:safrb:fixture:battery-parameters-001"),
        }],
        battery_actions: vec![
            BatteryAction {
                period: 0,
                battery_id: "battery-001".into(),
                mode: BatteryMode::Charge { input_wh: 500 },
            },
            BatteryAction {
                period: 1,
                battery_id: "battery-001".into(),
                mode: BatteryMode::Discharge {
                    battery_draw_wh: 400,
                },
            },
        ],
        demands: vec![
            ServiceDemand {
                id: "port-critical-day-0".into(),
                period: 0,
                service_zone_id: "port-critical-load".into(),
                priority: 0,
                demand_wh: 100,
            },
            ServiceDemand {
                id: "port-critical-day-1".into(),
                period: 1,
                service_zone_id: "port-critical-load".into(),
                priority: 0,
                demand_wh: 400,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_problem() -> Problem {
        base_case()
    }

    #[test]
    fn charges_from_renewable_surplus_then_discharge_supports_outage() {
        let result = base_problem().evaluate().unwrap();
        let day_zero = result
            .zone_periods
            .iter()
            .find(|row| row.period == 0)
            .unwrap();
        let day_one = result
            .zone_periods
            .iter()
            .find(|row| row.period == 1)
            .unwrap();
        assert_eq!(day_zero.served_lower_wh, 100);
        assert_eq!(day_zero.curtailed_lower_wh, 400);
        assert_eq!(day_zero.bus_balance_residual_lower_wh, 0);
        assert_eq!(day_one.battery_output_lower_wh, 360);
        assert_eq!(day_one.battery_output_upper_wh, 380);
        assert_eq!(day_one.served_lower_wh, 360);
        assert_eq!(day_one.served_upper_wh, 380);
        assert_eq!(day_one.unserved_lower_wh, 20);
        assert_eq!(day_one.unserved_upper_wh, 40);
        let battery_day_one = result
            .battery_periods
            .iter()
            .find(|row| row.period == 1)
            .unwrap();
        assert_eq!(battery_day_one.closing_soc_lower_wh, 150);
        assert_eq!(battery_day_one.closing_soc_upper_wh, 175);
    }

    #[test]
    fn prioritizes_critical_service_across_multiple_carriers() {
        let mut problem = base_problem();
        problem.battery_actions.clear();
        problem.supplies.push(ElectricSupply {
            id: "grid-period-0".into(),
            period: 0,
            service_zone_id: "port-critical-load".into(),
            carrier: EnergyCarrier::GridImport,
            available_lower_wh: 150,
            available_upper_wh: 150,
            maximum_power_w: 500,
            upstream_artifact_id: "grid-meter-fixture-001".into(),
            evidence: fixture_evidence("urn:safrb:fixture:grid-period-0"),
        });
        problem.demands = vec![
            ServiceDemand {
                id: "ordinary".into(),
                period: 0,
                service_zone_id: "port-critical-load".into(),
                priority: 10,
                demand_wh: 100,
            },
            ServiceDemand {
                id: "critical".into(),
                period: 0,
                service_zone_id: "port-critical-load".into(),
                priority: 0,
                demand_wh: 200,
            },
        ];
        let result = problem.evaluate().unwrap();
        let critical = result
            .service_allocations
            .iter()
            .find(|row| row.demand_id == "critical")
            .unwrap();
        let ordinary = result
            .service_allocations
            .iter()
            .find(|row| row.demand_id == "ordinary")
            .unwrap();
        assert_eq!(critical.served_lower_wh, 200);
        assert_eq!(ordinary.served_lower_wh, 100);
        assert_eq!(result.qualified_source_lower_wh, 1_150);
    }

    #[test]
    fn report_and_forecast_sources_are_excluded() {
        let mut problem = base_problem();
        problem.supplies[0].evidence.evidence_class = EvidenceClass::Reported;
        problem.supplies[0].available_upper_wh = 1_000;
        problem.battery_actions.clear();
        problem.batteries.clear();
        let result = problem.evaluate().unwrap();
        assert_eq!(result.qualified_source_lower_wh, 0);
        assert_eq!(result.excluded_source_upper_wh, 1_000);
        assert_eq!(result.served_lower_wh, 0);
        assert_eq!(result.unserved_upper_wh, 500);
    }

    #[test]
    fn scenario_source_requires_fixture_namespace() {
        let mut problem = base_problem();
        problem.supplies[0].evidence.source_uri = "urn:scenario:untrusted".into();
        problem.battery_actions.clear();
        problem.batteries.clear();
        let result = problem.evaluate().unwrap();
        assert_eq!(result.qualified_source_lower_wh, 0);
        assert_eq!(result.excluded_source_upper_wh, 1_200);
    }

    #[test]
    fn battery_cannot_discharge_more_than_conservative_soc() {
        let mut problem = base_problem();
        problem.battery_actions[1].mode = BatteryMode::Discharge {
            battery_draw_wh: 551,
        };
        assert!(problem.evaluate().is_err());
    }

    #[test]
    fn battery_charge_is_limited_by_power_envelope() {
        let mut problem = base_problem();
        problem.battery_actions[0].mode = BatteryMode::Charge { input_wh: 2_001 };
        assert!(problem.evaluate().is_err());
    }

    #[test]
    fn battery_cannot_charge_above_energy_capacity() {
        let mut problem = base_problem();
        problem.batteries[0].capacity_wh = 500;
        assert!(problem.evaluate().is_err());
    }

    #[test]
    fn battery_cannot_act_without_qualified_state_and_parameter_evidence() {
        let mut problem = base_problem();
        problem.batteries[0].state_evidence.evidence_class = EvidenceClass::Reported;
        assert!(problem.evaluate().is_err());
    }

    #[test]
    fn idle_battery_state_must_be_qualified_when_reported_in_portfolio() {
        let mut problem = base_problem();
        problem.battery_actions.clear();
        problem.batteries[0].state_evidence.evidence_class = EvidenceClass::Reported;
        assert!(problem.evaluate().is_err());
    }

    #[test]
    fn source_cannot_exceed_its_power_energy_envelope() {
        let mut problem = base_problem();
        problem.supplies[0].maximum_power_w = 100;
        assert!(problem.validate().is_err());
    }

    #[test]
    fn duplicate_battery_actions_fail_closed() {
        let mut problem = base_problem();
        problem
            .battery_actions
            .push(problem.battery_actions[0].clone());
        assert!(problem.validate().is_err());
    }

    #[test]
    fn inverted_supply_bounds_fail_closed() {
        let mut problem = base_problem();
        problem.supplies[0].available_lower_wh = 1_201;
        assert!(problem.validate().is_err());
    }

    #[test]
    fn bus_balance_is_zero_for_every_zone_period() {
        let result = base_problem().evaluate().unwrap();
        assert!(result.zone_periods.iter().all(|row| {
            row.bus_balance_residual_lower_wh == 0 && row.bus_balance_residual_upper_wh == 0
        }));
    }

    #[test]
    fn no_service_overdelivery_under_uncertainty() {
        let result = base_problem().evaluate().unwrap();
        assert!(result.service_allocations.iter().all(|allocation| {
            allocation.served_lower_wh <= allocation.requested_wh
                && allocation.served_upper_wh <= allocation.requested_wh
                && allocation.served_lower_wh <= allocation.served_upper_wh
                && allocation.unserved_lower_wh <= allocation.unserved_upper_wh
        }));
    }
}
