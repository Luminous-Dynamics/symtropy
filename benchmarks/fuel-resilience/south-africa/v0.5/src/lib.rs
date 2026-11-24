#![forbid(unsafe_code)]

//! SAFRB v0.5: evidence-aware fuel-to-useful-energy service accounting.
//!
//! This crate evaluates a supplied generator run schedule. It is not a
//! combustion solver, a power-flow solver, an optimizer, or an OT controller.
//! All physical values are integer quantities with explicit units.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Grade {
    D50,
    D500,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    /// A digest reference. v0.5 checks its shape only; it does not hash bytes
    /// or authenticate the issuer or measurement process.
    pub raw_sha256: Option<String>,
}

impl Evidence {
    pub fn validate(&self) -> Result<(), String> {
        if self.source_id.trim().is_empty() {
            return Err("evidence source_id must not be empty".into());
        }
        if self.source_uri.trim().is_empty() {
            return Err("evidence source_uri must not be empty".into());
        }
        if let Some(digest) = &self.raw_sha256
            && (digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err("raw_sha256 must be exactly 64 hexadecimal characters".into());
        }
        Ok(())
    }

    fn has_sha256_shape(&self) -> bool {
        self.raw_sha256.as_ref().is_some_and(|digest| {
            digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit())
        })
    }

    fn qualifies_for_simulation(&self) -> bool {
        match self.evidence_class {
            EvidenceClass::Measured => self.has_sha256_shape(),
            EvidenceClass::Scenario => self.source_uri.starts_with("urn:safrb:fixture:"),
            EvidenceClass::Reported | EvidenceClass::Derived | EvidenceClass::Forecast => false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FuelTank {
    pub id: String,
    pub service_zone_id: String,
    pub grade: Grade,
    pub capacity_l: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FuelBatch {
    pub id: String,
    pub tank_id: String,
    pub grade: Grade,
    pub available_day: u16,
    pub volume_l: u64,
    pub custody_evidence: Evidence,
    pub quality_released: bool,
    pub physically_accessible: bool,
    /// Lower and upper LHV energy density in watt-hours per litre.
    /// Values must be source-qualified; no default fuel calorific value exists.
    pub lhv_lower_wh_per_litre: u64,
    pub lhv_upper_wh_per_litre: u64,
    pub lhv_evidence: Evidence,
}

impl FuelBatch {
    /// Physical product receipt gate. This says the batch is physically
    /// admissible stock; it does not by itself justify an energy calculation.
    fn qualifies_as_usable_input(&self) -> bool {
        self.custody_evidence.qualifies_for_simulation()
            && self.quality_released
            && self.physically_accessible
    }

    /// Conversion requires a separately qualified lower-heating-value source.
    fn qualifies_for_energy_conversion(&self) -> bool {
        self.qualifies_as_usable_input() && self.lhv_evidence.qualifies_for_simulation()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Generator {
    pub id: String,
    pub tank_id: String,
    pub service_zone_id: String,
    pub rated_power_w: u64,
    pub max_run_hours_per_day: u16,
    /// Electrical conversion efficiency in basis points (10,000 = 100%).
    pub efficiency_lower_bps: u16,
    pub efficiency_upper_bps: u16,
    /// Parasitic/auxiliary power bounds while the unit is operating.
    pub auxiliary_power_lower_w: u64,
    pub auxiliary_power_upper_w: u64,
    pub parameter_evidence: Evidence,
}

impl Generator {
    /// The conversion envelope must have measured digest-referenced evidence
    /// or be an explicitly namespaced synthetic fixture.
    fn has_qualified_parameters(&self) -> bool {
        self.parameter_evidence.qualifies_for_simulation()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchDraw {
    pub batch_id: String,
    pub volume_l: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratorRun {
    pub day: u16,
    pub generator_id: String,
    pub run_hours: u16,
    pub fuel_draws: Vec<BatchDraw>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceDemand {
    pub id: String,
    pub day: u16,
    pub service_zone_id: String,
    pub priority: u16,
    /// Useful electric service demand in watt-hours.
    pub demand_wh: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceProblem {
    pub scenario_id: String,
    pub horizon_days: u16,
    pub tanks: Vec<FuelTank>,
    pub batches: Vec<FuelBatch>,
    pub generators: Vec<Generator>,
    pub runs: Vec<GeneratorRun>,
    pub demands: Vec<ServiceDemand>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratorDayResult {
    pub day: u16,
    pub generator_id: String,
    pub service_zone_id: String,
    pub run_hours: u16,
    pub fuel_used_l: u128,
    pub chemical_input_lower_wh: u128,
    pub chemical_input_upper_wh: u128,
    pub fuel_batch_ids: Vec<String>,
    pub custody_evidence_source_ids: Vec<String>,
    pub lhv_evidence_source_ids: Vec<String>,
    pub generator_parameter_evidence_source_id: String,
    pub net_generation_lower_wh: u128,
    pub net_generation_upper_wh: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceAllocation {
    pub demand_id: String,
    pub day: u16,
    pub service_zone_id: String,
    pub priority: u16,
    pub requested_wh: u128,
    /// Conservative delivered-energy bound using lower LHV/efficiency and
    /// upper auxiliary load.
    pub served_lower_wh: u128,
    /// Optimistic delivered-energy bound using upper LHV/efficiency and
    /// lower auxiliary load.
    pub served_upper_wh: u128,
    /// Best-case lower bound on unserved demand.
    pub unserved_lower_wh: u128,
    /// Conservative upper bound on unserved demand.
    pub unserved_upper_wh: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TankDayResult {
    pub day: u16,
    pub tank_id: String,
    pub opening_l: u128,
    pub receipts_l: u128,
    pub fuel_draws_l: u128,
    pub closing_l: u128,
    pub capacity_l: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceSummary {
    pub scenario_id: String,
    pub qualified_fuel_receipts_l: u128,
    pub excluded_fuel_volume_l: u128,
    pub fuel_used_l: u128,
    pub closing_fuel_inventory_l: u128,
    pub chemical_input_lower_wh: u128,
    pub chemical_input_upper_wh: u128,
    pub net_generation_lower_wh: u128,
    pub net_generation_upper_wh: u128,
    pub total_service_demand_wh: u128,
    pub served_lower_wh: u128,
    pub served_upper_wh: u128,
    pub unserved_lower_wh: u128,
    pub unserved_upper_wh: u128,
    pub generator_days: Vec<GeneratorDayResult>,
    pub service_allocations: Vec<ServiceAllocation>,
    pub tank_days: Vec<TankDayResult>,
}

fn checked_add_u128(target: &mut u128, value: u128, label: &str) -> Result<(), String> {
    *target = target
        .checked_add(value)
        .ok_or_else(|| format!("{label} overflowed u128"))?;
    Ok(())
}

fn checked_mul_u128(a: u128, b: u128, label: &str) -> Result<u128, String> {
    a.checked_mul(b)
        .ok_or_else(|| format!("{label} overflowed u128"))
}

impl ServiceProblem {
    pub fn validate(&self) -> Result<(), String> {
        if self.scenario_id.trim().is_empty() {
            return Err("scenario_id must not be empty".into());
        }
        if self.horizon_days == 0 {
            return Err("horizon_days must be positive".into());
        }

        let mut tank_ids = BTreeSet::new();
        for tank in &self.tanks {
            if tank.id.trim().is_empty() || tank.service_zone_id.trim().is_empty() {
                return Err("tank id and service_zone_id must not be empty".into());
            }
            if !tank_ids.insert(tank.id.as_str()) {
                return Err(format!("duplicate tank id: {}", tank.id));
            }
            if tank.capacity_l == 0 {
                return Err(format!("tank {} capacity must be positive", tank.id));
            }
        }

        let tank_by_id: BTreeMap<&str, &FuelTank> = self
            .tanks
            .iter()
            .map(|tank| (tank.id.as_str(), tank))
            .collect();

        let mut batch_ids = BTreeSet::new();
        for batch in &self.batches {
            if batch.id.trim().is_empty() {
                return Err("fuel batch id must not be empty".into());
            }
            if !batch_ids.insert(batch.id.as_str()) {
                return Err(format!("duplicate fuel batch id: {}", batch.id));
            }
            let tank = tank_by_id
                .get(batch.tank_id.as_str())
                .ok_or_else(|| format!("batch {} references unknown tank", batch.id))?;
            if batch.grade != tank.grade {
                return Err(format!(
                    "batch {} grade does not match tank {} grade",
                    batch.id, tank.id
                ));
            }
            if batch.available_day >= self.horizon_days {
                return Err(format!("batch {} is outside the horizon", batch.id));
            }
            if batch.volume_l == 0 {
                return Err(format!("batch {} volume must be positive", batch.id));
            }
            if batch.lhv_lower_wh_per_litre == 0
                || batch.lhv_lower_wh_per_litre > batch.lhv_upper_wh_per_litre
            {
                return Err(format!("batch {} has invalid LHV bounds", batch.id));
            }
            batch.custody_evidence.validate()?;
            batch.lhv_evidence.validate()?;
        }

        let mut generator_ids = BTreeSet::new();
        for generator in &self.generators {
            if generator.id.trim().is_empty() || generator.service_zone_id.trim().is_empty() {
                return Err("generator id and service_zone_id must not be empty".into());
            }
            if !generator_ids.insert(generator.id.as_str()) {
                return Err(format!("duplicate generator id: {}", generator.id));
            }
            let tank = tank_by_id
                .get(generator.tank_id.as_str())
                .ok_or_else(|| format!("generator {} references unknown tank", generator.id))?;
            if tank.service_zone_id != generator.service_zone_id {
                return Err(format!(
                    "generator {} service zone does not match tank {}",
                    generator.id, tank.id
                ));
            }
            if generator.rated_power_w == 0 {
                return Err(format!(
                    "generator {} rated_power_w must be positive",
                    generator.id
                ));
            }
            if generator.max_run_hours_per_day == 0 || generator.max_run_hours_per_day > 24 {
                return Err(format!(
                    "generator {} max_run_hours_per_day must be in 1..=24",
                    generator.id
                ));
            }
            if generator.efficiency_lower_bps == 0
                || generator.efficiency_lower_bps > generator.efficiency_upper_bps
                || generator.efficiency_upper_bps > 10_000
            {
                return Err(format!(
                    "generator {} has invalid efficiency bounds",
                    generator.id
                ));
            }
            if generator.auxiliary_power_lower_w > generator.auxiliary_power_upper_w
                || generator.auxiliary_power_upper_w > generator.rated_power_w
            {
                return Err(format!(
                    "generator {} has invalid auxiliary power bounds",
                    generator.id
                ));
            }
            generator.parameter_evidence.validate()?;
        }

        let mut run_keys = BTreeSet::new();
        for run in &self.runs {
            if run.day >= self.horizon_days {
                return Err(format!("generator run day {} is outside horizon", run.day));
            }
            let generator = self
                .generators
                .iter()
                .find(|generator| generator.id == run.generator_id)
                .ok_or_else(|| format!("run references unknown generator {}", run.generator_id))?;
            if !generator.has_qualified_parameters() {
                return Err(format!(
                    "run uses generator {} with unqualified conversion parameters",
                    generator.id
                ));
            }
            if !run_keys.insert((run.day, run.generator_id.as_str())) {
                return Err(format!(
                    "duplicate run for generator {} on day {}",
                    run.generator_id, run.day
                ));
            }
            if run.run_hours == 0 || run.run_hours > generator.max_run_hours_per_day {
                return Err(format!(
                    "generator {} run hours exceed its daily operating envelope",
                    generator.id
                ));
            }
            if run.fuel_draws.is_empty() {
                return Err(format!("generator run {} has no fuel draws", generator.id));
            }
            let mut draw_ids = BTreeSet::new();
            for draw in &run.fuel_draws {
                if draw.volume_l == 0 {
                    return Err("fuel draw volume must be positive".into());
                }
                if !draw_ids.insert(draw.batch_id.as_str()) {
                    return Err(format!(
                        "run for generator {} repeats batch {}",
                        generator.id, draw.batch_id
                    ));
                }
                let batch = self
                    .batches
                    .iter()
                    .find(|batch| batch.id == draw.batch_id)
                    .ok_or_else(|| format!("run references unknown batch {}", draw.batch_id))?;
                if batch.tank_id != generator.tank_id {
                    return Err(format!(
                        "batch {} is not connected to generator {} tank",
                        batch.id, generator.id
                    ));
                }
                if !batch.qualifies_as_usable_input() {
                    return Err(format!(
                        "run attempts to consume non-usable batch {}",
                        batch.id
                    ));
                }
                if !batch.qualifies_for_energy_conversion() {
                    return Err(format!(
                        "run attempts to calculate output from batch {} with unqualified LHV evidence",
                        batch.id
                    ));
                }
            }
        }

        let mut demand_ids = BTreeSet::new();
        for demand in &self.demands {
            if demand.id.trim().is_empty() || demand.service_zone_id.trim().is_empty() {
                return Err("service demand id and service_zone_id must not be empty".into());
            }
            if !demand_ids.insert(demand.id.as_str()) {
                return Err(format!("duplicate service demand id: {}", demand.id));
            }
            if demand.day >= self.horizon_days {
                return Err(format!("service demand {} is outside horizon", demand.id));
            }
        }

        Ok(())
    }

    pub fn simulate(&self) -> Result<ServiceSummary, String> {
        self.validate()?;

        let tank_index: BTreeMap<&str, usize> = self
            .tanks
            .iter()
            .enumerate()
            .map(|(index, tank)| (tank.id.as_str(), index))
            .collect();
        let batch_index: BTreeMap<&str, usize> = self
            .batches
            .iter()
            .enumerate()
            .map(|(index, batch)| (batch.id.as_str(), index))
            .collect();
        let generator_index: BTreeMap<&str, usize> = self
            .generators
            .iter()
            .enumerate()
            .map(|(index, generator)| (generator.id.as_str(), index))
            .collect();

        let mut batch_stock = vec![0_u64; self.batches.len()];
        let mut tank_stock = vec![0_u128; self.tanks.len()];
        let mut receipt_by_day = vec![Vec::<usize>::new(); usize::from(self.horizon_days)];
        let mut runs_by_day = vec![Vec::<&GeneratorRun>::new(); usize::from(self.horizon_days)];
        for (index, batch) in self.batches.iter().enumerate() {
            if batch.qualifies_as_usable_input() {
                receipt_by_day[usize::from(batch.available_day)].push(index);
            }
        }
        for run in &self.runs {
            runs_by_day[usize::from(run.day)].push(run);
        }

        let mut qualified_fuel_receipts_l = 0_u128;
        let mut excluded_fuel_volume_l = 0_u128;
        for batch in &self.batches {
            let target = if batch.qualifies_as_usable_input() {
                &mut qualified_fuel_receipts_l
            } else {
                &mut excluded_fuel_volume_l
            };
            checked_add_u128(target, u128::from(batch.volume_l), "fuel receipt total")?;
        }

        let mut total_fuel_used_l = 0_u128;
        let mut chemical_input_lower_wh = 0_u128;
        let mut chemical_input_upper_wh = 0_u128;
        let mut net_generation_lower_wh = 0_u128;
        let mut net_generation_upper_wh = 0_u128;
        let mut generator_days = Vec::new();
        let mut tank_days = Vec::new();
        let mut daily_zone_generation = BTreeMap::<(u16, String), (u128, u128)>::new();

        for day in 0..self.horizon_days {
            let opening_tank_stock = tank_stock.clone();
            let mut receipts_by_tank = vec![0_u128; self.tanks.len()];
            let mut draws_by_tank = vec![0_u128; self.tanks.len()];

            for index in &receipt_by_day[usize::from(day)] {
                let batch = &self.batches[*index];
                let tank = tank_index[batch.tank_id.as_str()];
                batch_stock[*index] = batch.volume_l;
                checked_add_u128(
                    &mut tank_stock[tank],
                    u128::from(batch.volume_l),
                    "tank receipts",
                )?;
                checked_add_u128(
                    &mut receipts_by_tank[tank],
                    u128::from(batch.volume_l),
                    "daily tank receipts",
                )?;
            }

            for (index, tank) in self.tanks.iter().enumerate() {
                if tank_stock[index] > u128::from(tank.capacity_l) {
                    return Err(format!(
                        "tank {} exceeds capacity on day {}",
                        tank.id,
                        day + 1
                    ));
                }
            }

            for run in &runs_by_day[usize::from(day)] {
                let generator = &self.generators[generator_index[run.generator_id.as_str()]];
                let mut fuel_used_l = 0_u128;
                let mut input_low_wh = 0_u128;
                let mut input_high_wh = 0_u128;
                let mut fuel_batch_ids = Vec::with_capacity(run.fuel_draws.len());
                let mut custody_evidence_source_ids = Vec::with_capacity(run.fuel_draws.len());
                let mut lhv_evidence_source_ids = Vec::with_capacity(run.fuel_draws.len());

                for draw in &run.fuel_draws {
                    let index = batch_index[draw.batch_id.as_str()];
                    let batch = &self.batches[index];
                    fuel_batch_ids.push(batch.id.clone());
                    custody_evidence_source_ids.push(batch.custody_evidence.source_id.clone());
                    lhv_evidence_source_ids.push(batch.lhv_evidence.source_id.clone());
                    if batch_stock[index] < draw.volume_l {
                        return Err(format!(
                            "generator {} attempts to consume unavailable batch {} on day {}",
                            generator.id,
                            batch.id,
                            day + 1
                        ));
                    }
                    batch_stock[index] -= draw.volume_l;
                    let tank = tank_index[batch.tank_id.as_str()];
                    tank_stock[tank] -= u128::from(draw.volume_l);
                    checked_add_u128(
                        &mut draws_by_tank[tank],
                        u128::from(draw.volume_l),
                        "daily tank fuel draws",
                    )?;
                    checked_add_u128(
                        &mut fuel_used_l,
                        u128::from(draw.volume_l),
                        "generator fuel used",
                    )?;
                    checked_add_u128(
                        &mut input_low_wh,
                        checked_mul_u128(
                            u128::from(draw.volume_l),
                            u128::from(batch.lhv_lower_wh_per_litre),
                            "lower chemical input energy",
                        )?,
                        "lower chemical input energy total",
                    )?;
                    checked_add_u128(
                        &mut input_high_wh,
                        checked_mul_u128(
                            u128::from(draw.volume_l),
                            u128::from(batch.lhv_upper_wh_per_litre),
                            "upper chemical input energy",
                        )?,
                        "upper chemical input energy total",
                    )?;
                }

                let rated_energy_cap_wh = checked_mul_u128(
                    u128::from(generator.rated_power_w),
                    u128::from(run.run_hours),
                    "generator rated energy cap",
                )?;
                // Apply each batch's explicit LHV interval. No default
                // calorific value or implied product-grade energy is used.
                let gross_low_wh = checked_mul_u128(
                    input_low_wh,
                    u128::from(generator.efficiency_lower_bps),
                    "lower gross generation",
                )? / 10_000;
                let gross_high_wh = checked_mul_u128(
                    input_high_wh,
                    u128::from(generator.efficiency_upper_bps),
                    "upper gross generation",
                )? / 10_000;
                let gross_low_wh = gross_low_wh.min(rated_energy_cap_wh);
                let gross_high_wh = gross_high_wh.min(rated_energy_cap_wh);
                let auxiliary_upper_wh = checked_mul_u128(
                    u128::from(generator.auxiliary_power_upper_w),
                    u128::from(run.run_hours),
                    "upper auxiliary energy",
                )?;
                let auxiliary_lower_wh = checked_mul_u128(
                    u128::from(generator.auxiliary_power_lower_w),
                    u128::from(run.run_hours),
                    "lower auxiliary energy",
                )?;
                let net_low_wh = gross_low_wh.saturating_sub(auxiliary_upper_wh);
                let net_high_wh = gross_high_wh.saturating_sub(auxiliary_lower_wh);
                if net_low_wh > net_high_wh {
                    return Err(format!(
                        "generator {} produced inverted energy bounds",
                        generator.id
                    ));
                }

                checked_add_u128(&mut total_fuel_used_l, fuel_used_l, "total fuel use")?;
                checked_add_u128(
                    &mut chemical_input_lower_wh,
                    input_low_wh,
                    "total lower chemical energy",
                )?;
                checked_add_u128(
                    &mut chemical_input_upper_wh,
                    input_high_wh,
                    "total upper chemical energy",
                )?;
                checked_add_u128(
                    &mut net_generation_lower_wh,
                    net_low_wh,
                    "total lower generation",
                )?;
                checked_add_u128(
                    &mut net_generation_upper_wh,
                    net_high_wh,
                    "total upper generation",
                )?;

                let entry = daily_zone_generation
                    .entry((day, generator.service_zone_id.clone()))
                    .or_insert((0, 0));
                checked_add_u128(&mut entry.0, net_low_wh, "zone lower generation")?;
                checked_add_u128(&mut entry.1, net_high_wh, "zone upper generation")?;

                generator_days.push(GeneratorDayResult {
                    day: day + 1,
                    generator_id: generator.id.clone(),
                    service_zone_id: generator.service_zone_id.clone(),
                    run_hours: run.run_hours,
                    fuel_used_l,
                    chemical_input_lower_wh: input_low_wh,
                    chemical_input_upper_wh: input_high_wh,
                    fuel_batch_ids,
                    custody_evidence_source_ids,
                    lhv_evidence_source_ids,
                    generator_parameter_evidence_source_id: generator
                        .parameter_evidence
                        .source_id
                        .clone(),
                    net_generation_lower_wh: net_low_wh,
                    net_generation_upper_wh: net_high_wh,
                });
            }

            for (index, tank) in self.tanks.iter().enumerate() {
                tank_days.push(TankDayResult {
                    day: day + 1,
                    tank_id: tank.id.clone(),
                    opening_l: opening_tank_stock[index],
                    receipts_l: receipts_by_tank[index],
                    fuel_draws_l: draws_by_tank[index],
                    closing_l: tank_stock[index],
                    capacity_l: tank.capacity_l,
                });
            }
        }

        let mut demand_indices: Vec<usize> = (0..self.demands.len()).collect();
        demand_indices.sort_by(|a, b| {
            let left = &self.demands[*a];
            let right = &self.demands[*b];
            (
                left.day,
                left.service_zone_id.as_str(),
                left.priority,
                left.id.as_str(),
            )
                .cmp(&(
                    right.day,
                    right.service_zone_id.as_str(),
                    right.priority,
                    right.id.as_str(),
                ))
        });

        let mut zone_remaining = daily_zone_generation.clone();
        let mut service_allocations = Vec::with_capacity(self.demands.len());
        let mut total_service_demand_wh = 0_u128;
        let mut served_lower_wh = 0_u128;
        let mut served_upper_wh = 0_u128;
        let mut unserved_lower_wh = 0_u128;
        let mut unserved_upper_wh = 0_u128;

        for index in demand_indices {
            let demand = &self.demands[index];
            let remaining = zone_remaining
                .entry((demand.day, demand.service_zone_id.clone()))
                .or_insert((0, 0));
            let served_lower = remaining.0.min(demand.demand_wh);
            let served_upper = remaining.1.min(demand.demand_wh);
            remaining.0 -= served_lower;
            remaining.1 -= served_upper;
            let unserved_lower = demand.demand_wh - served_upper;
            let unserved_upper = demand.demand_wh - served_lower;

            checked_add_u128(
                &mut total_service_demand_wh,
                demand.demand_wh,
                "total service demand",
            )?;
            checked_add_u128(&mut served_lower_wh, served_lower, "lower served energy")?;
            checked_add_u128(&mut served_upper_wh, served_upper, "upper served energy")?;
            checked_add_u128(
                &mut unserved_lower_wh,
                unserved_lower,
                "lower unserved energy",
            )?;
            checked_add_u128(
                &mut unserved_upper_wh,
                unserved_upper,
                "upper unserved energy",
            )?;

            service_allocations.push(ServiceAllocation {
                demand_id: demand.id.clone(),
                day: demand.day + 1,
                service_zone_id: demand.service_zone_id.clone(),
                priority: demand.priority,
                requested_wh: demand.demand_wh,
                served_lower_wh: served_lower,
                served_upper_wh: served_upper,
                unserved_lower_wh: unserved_lower,
                unserved_upper_wh: unserved_upper,
            });
        }

        let closing_fuel_inventory_l = batch_stock.iter().try_fold(0_u128, |total, volume| {
            total
                .checked_add(u128::from(*volume))
                .ok_or_else(|| "closing fuel inventory overflowed u128".to_string())
        })?;

        Ok(ServiceSummary {
            scenario_id: self.scenario_id.clone(),
            qualified_fuel_receipts_l,
            excluded_fuel_volume_l,
            fuel_used_l: total_fuel_used_l,
            closing_fuel_inventory_l,
            chemical_input_lower_wh,
            chemical_input_upper_wh,
            net_generation_lower_wh,
            net_generation_upper_wh,
            total_service_demand_wh,
            served_lower_wh,
            served_upper_wh,
            unserved_lower_wh,
            unserved_upper_wh,
            generator_days,
            service_allocations,
            tank_days,
        })
    }
}

pub fn base_case() -> ServiceProblem {
    let evidence = Evidence {
        source_id: "synthetic-fixture".into(),
        source_uri: "urn:safrb:fixture:generator-batch-001".into(),
        evidence_class: EvidenceClass::Scenario,
        raw_sha256: None,
    };
    let mut parameter_evidence = evidence.clone();
    parameter_evidence.source_uri = "urn:safrb:fixture:generator-parameters-001".into();

    ServiceProblem {
        scenario_id: "synthetic-generator-service-boundary".into(),
        horizon_days: 2,
        tanks: vec![FuelTank {
            id: "backup-tank".into(),
            service_zone_id: "port-critical-load".into(),
            grade: Grade::D50,
            capacity_l: 1_000,
        }],
        batches: vec![FuelBatch {
            id: "batch-001".into(),
            tank_id: "backup-tank".into(),
            grade: Grade::D50,
            available_day: 0,
            volume_l: 100,
            custody_evidence: evidence.clone(),
            quality_released: true,
            physically_accessible: true,
            lhv_lower_wh_per_litre: 9_500,
            lhv_upper_wh_per_litre: 10_000,
            lhv_evidence: evidence.clone(),
        }],
        generators: vec![Generator {
            id: "generator-001".into(),
            tank_id: "backup-tank".into(),
            service_zone_id: "port-critical-load".into(),
            rated_power_w: 500_000,
            max_run_hours_per_day: 4,
            efficiency_lower_bps: 3_500,
            efficiency_upper_bps: 4_000,
            auxiliary_power_lower_w: 3_000,
            auxiliary_power_upper_w: 5_000,
            parameter_evidence,
        }],
        runs: vec![GeneratorRun {
            day: 0,
            generator_id: "generator-001".into(),
            run_hours: 2,
            fuel_draws: vec![BatchDraw {
                batch_id: "batch-001".into(),
                volume_l: 80,
            }],
        }],
        demands: vec![ServiceDemand {
            id: "port-critical-electricity".into(),
            day: 0,
            service_zone_id: "port-critical-load".into(),
            priority: 0,
            demand_wh: 300_000,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_problem() -> ServiceProblem {
        base_case()
    }

    #[test]
    fn converts_fuel_energy_to_bounded_net_service() {
        let result = base_problem().simulate().unwrap();
        assert_eq!(result.qualified_fuel_receipts_l, 100);
        assert_eq!(result.fuel_used_l, 80);
        assert_eq!(result.closing_fuel_inventory_l, 20);
        assert!(result.net_generation_lower_wh < result.net_generation_upper_wh);
        assert_eq!(result.service_allocations[0].served_lower_wh, 256_000);
        assert_eq!(result.service_allocations[0].served_upper_wh, 300_000);
        assert_eq!(result.service_allocations[0].unserved_lower_wh, 0);
        assert_eq!(result.service_allocations[0].unserved_upper_wh, 44_000);
    }

    #[test]
    fn reported_or_unreleased_fuel_is_excluded_from_usable_supply() {
        let mut problem = base_problem();
        problem.batches[0].custody_evidence.evidence_class = EvidenceClass::Reported;
        problem.batches[0].quality_released = false;
        problem.runs.clear();
        let result = problem.simulate().unwrap();
        assert_eq!(result.qualified_fuel_receipts_l, 0);
        assert_eq!(result.excluded_fuel_volume_l, 100);
        assert_eq!(result.net_generation_lower_wh, 0);
    }

    #[test]
    fn generator_cannot_consume_reported_or_unreleased_batch() {
        let mut problem = base_problem();
        problem.batches[0].quality_released = false;
        assert!(problem.simulate().is_err());
    }

    #[test]
    fn future_batch_cannot_be_consumed_early() {
        let mut problem = base_problem();
        problem.batches[0].available_day = 1;
        assert!(problem.simulate().is_err());
    }

    #[test]
    fn tank_capacity_is_checked_at_receipt_time() {
        let mut problem = base_problem();
        problem.tanks[0].capacity_l = 50;
        assert!(problem.simulate().is_err());
    }

    #[test]
    fn grade_mismatch_between_tank_and_batch_fails_closed() {
        let mut problem = base_problem();
        problem.batches[0].grade = Grade::D500;
        assert!(problem.validate().is_err());
    }

    #[test]
    fn generator_run_time_cannot_exceed_daily_operating_envelope() {
        let mut problem = base_problem();
        problem.runs[0].run_hours = 5;
        assert!(problem.simulate().is_err());
    }

    #[test]
    fn priority_allocation_serves_critical_load_first() {
        let mut problem = base_problem();
        problem.demands = vec![
            ServiceDemand {
                id: "ordinary".into(),
                day: 0,
                service_zone_id: "port-critical-load".into(),
                priority: 10,
                demand_wh: 100_000,
            },
            ServiceDemand {
                id: "critical".into(),
                day: 0,
                service_zone_id: "port-critical-load".into(),
                priority: 0,
                demand_wh: 200_000,
            },
        ];
        let result = problem.simulate().unwrap();
        let critical = result
            .service_allocations
            .iter()
            .find(|allocation| allocation.demand_id == "critical")
            .unwrap();
        let ordinary = result
            .service_allocations
            .iter()
            .find(|allocation| allocation.demand_id == "ordinary")
            .unwrap();
        assert_eq!(critical.served_lower_wh, 200_000);
        assert_eq!(ordinary.served_lower_wh, 56_000);
    }

    #[test]
    fn duplicate_batch_and_generator_run_identifiers_fail_closed() {
        let mut problem = base_problem();
        problem.batches.push(problem.batches[0].clone());
        assert!(problem.validate().is_err());

        let mut problem = base_problem();
        problem.runs.push(problem.runs[0].clone());
        assert!(problem.validate().is_err());
    }

    #[test]
    fn auxiliary_load_can_reduce_net_service_to_zero() {
        let mut problem = base_problem();
        problem.batches[0].volume_l = 1;
        problem.runs[0].fuel_draws[0].volume_l = 1;
        problem.generators[0].auxiliary_power_lower_w = 5_000;
        problem.generators[0].auxiliary_power_upper_w = 5_000;
        let result = problem.simulate().unwrap();
        assert_eq!(result.net_generation_lower_wh, 0);
        assert_eq!(result.net_generation_upper_wh, 0);
    }

    #[test]
    fn unqualified_lhv_blocks_energy_conversion_but_not_physical_receipt() {
        let mut problem = base_problem();
        problem.batches[0].lhv_evidence.evidence_class = EvidenceClass::Reported;
        assert!(problem.simulate().is_err());

        problem.runs.clear();
        let result = problem.simulate().unwrap();
        assert_eq!(result.qualified_fuel_receipts_l, 100);
        assert_eq!(result.excluded_fuel_volume_l, 0);
        assert_eq!(result.closing_fuel_inventory_l, 100);
        assert_eq!(result.net_generation_lower_wh, 0);
    }

    #[test]
    fn unqualified_generator_parameters_block_conversion() {
        let mut problem = base_problem();
        problem.generators[0].parameter_evidence.evidence_class = EvidenceClass::Reported;
        assert!(problem.simulate().is_err());

        problem.runs.clear();
        let result = problem.simulate().unwrap();
        assert_eq!(result.qualified_fuel_receipts_l, 100);
        assert_eq!(result.closing_fuel_inventory_l, 100);
        assert_eq!(result.net_generation_lower_wh, 0);
    }

    #[test]
    fn scenario_inputs_must_use_explicit_fixture_namespace() {
        let mut problem = base_problem();
        problem.batches[0].custody_evidence.source_uri = "urn:arbitrary:scenario".into();
        problem.runs.clear();
        let result = problem.simulate().unwrap();
        assert_eq!(result.qualified_fuel_receipts_l, 0);
        assert_eq!(result.excluded_fuel_volume_l, 100);
    }

    #[test]
    fn measured_batch_requires_digest_shape_but_not_claimed_as_verified_metrology() {
        let mut problem = base_problem();
        problem.batches[0].custody_evidence = Evidence {
            source_id: "meter-01".into(),
            source_uri: "urn:site:tank-gauge:001".into(),
            evidence_class: EvidenceClass::Measured,
            raw_sha256: Some("a".repeat(64)),
        };
        let result = problem.simulate().unwrap();
        assert_eq!(result.qualified_fuel_receipts_l, 100);
    }
}
