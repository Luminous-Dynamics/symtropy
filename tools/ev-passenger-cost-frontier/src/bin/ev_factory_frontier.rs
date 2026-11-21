use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, env, fs, process};

const CLAIM_CEILING: &str = "Synthetic factory-layout and operating-cost sensitivity only. This is not a site design, plant quotation, process safety approval, validated production plan, vehicle BOM, regulatory compliance finding, or evidence of commercial feasibility.";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    schema_version: String,
    benchmark_id: String,
    sites: Vec<SiteProfile>,
    demand_cases_good_units_per_year: Vec<u64>,
    templates: Vec<FactoryTemplate>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteProfile {
    site_id: String,
    country_or_region: String,
    local_currency: String,
    local_currency_per_usd: f64,
    fx_reference_date: String,
    fx_evidence_class: EvidenceClass,
    site_cost_evidence_class: EvidenceClass,
    process_evidence_class: EvidenceClass,
    yield_evidence_class: EvidenceClass,
    operating_days_per_year: u32,
    shifts_per_day: u32,
    paid_hours_per_shift: f64,
    unpaid_break_hours_per_shift: f64,
    availability_fraction: f64,
    performance_fraction: f64,
    good_vehicle_yield_fraction: f64,
    fully_loaded_labor_cost_per_hour_local: f64,
    electricity_cost_per_kwh_local: f64,
    useful_thermal_energy_cost_per_kwh_local: f64,
    building_lease_cost_per_sqm_year_local: f64,
    building_fitout_cost_per_sqm_local: f64,
    annual_site_overhead_local: f64,
    imported_kit_cost_per_attempt_local: f64,
    local_parts_cost_per_attempt_local: f64,
    bought_painted_body_cost_per_attempt_local: f64,
    in_house_body_material_cost_per_attempt_local: f64,
    consumables_cost_per_attempt_local: f64,
    scrap_and_rework_cost_per_failed_attempt_local: f64,
    warranty_reserve_per_good_vehicle_local: f64,
    battery_quarantine_and_separation_area_sqm: f64,
    local_code_gate: EvidenceGate,
    battery_process_safety_gate: EvidenceGate,
    note: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct FactoryTemplate {
    template_id: String,
    label: String,
    strategy: FactoryStrategy,
    layout_area_multiplier: f64,
    annual_factory_overhead_usd: f64,
    launch_validation_capex_usd: f64,
    station_processes: Vec<StationProcess>,
    note: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum FactoryStrategy {
    ContractAssembly,
    ModularCkd,
    IntegratedFlexible,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct StationProcess {
    process_id: String,
    process_label: String,
    cycle_minutes_per_attempt: f64,
    equipment_capex_usd_per_cell: f64,
    footprint_sqm_per_cell: f64,
    operators_per_cell_per_shift: u32,
    energy_kwh_per_attempt: f64,
    thermal_energy_kwh_per_attempt: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceGate {
    status: GateStatus,
    evidence_class: EvidenceClass,
    source_reference: String,
    as_of: String,
    note: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum GateStatus {
    Passed,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EvidenceClass {
    Synthetic,
    PublicMarketData,
    ManufacturerSpec,
    OfficialRecord,
    Measured,
    IndependentTest,
    AuditedCost,
    Unknown,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct StationResult {
    process_id: String,
    process_label: String,
    cycle_minutes_per_attempt: f64,
    parallel_cells_required: u64,
    planned_attempt_capacity_per_year: u64,
    equipment_capex_usd: f64,
    operators_per_shift: u64,
    total_active_positions_across_shifts: u64,
    footprint_sqm: f64,
    energy_kwh_per_attempt: f64,
    thermal_energy_kwh_per_attempt: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ScenarioResult {
    site_id: String,
    country_or_region: String,
    local_currency: String,
    template_id: String,
    template_label: String,
    strategy: FactoryStrategy,
    target_good_vehicles_per_year: u64,
    planned_attempts_per_year: u64,
    good_vehicle_yield_fraction: f64,
    productive_minutes_per_cell_year: f64,
    takt_minutes_per_attempt: f64,
    bottleneck_attempt_capacity_per_year: u64,
    bottleneck_good_capacity_per_year: u64,
    capacity_headroom_fraction: f64,
    planned_capacity_utilization_fraction: f64,
    parallel_cell_count: u64,
    total_active_positions_across_shifts: u64,
    building_area_sqm: f64,
    equipment_capex_usd: f64,
    fitout_capex_usd: f64,
    launch_validation_capex_usd: f64,
    total_installed_capex_usd: f64,
    annual_labor_cost_usd: f64,
    annual_energy_cost_usd: f64,
    annual_useful_thermal_energy_cost_usd: f64,
    annual_building_lease_usd: f64,
    annual_site_overhead_usd: f64,
    annual_equipment_maintenance_usd: f64,
    annual_capex_amortization_usd: f64,
    annual_factory_fixed_cost_usd: f64,
    variable_imported_kit_cost_usd_per_good_vehicle: f64,
    variable_local_parts_cost_usd_per_good_vehicle: f64,
    variable_body_cost_usd_per_good_vehicle: f64,
    variable_consumables_cost_usd_per_good_vehicle: f64,
    variable_scrap_rework_cost_usd_per_good_vehicle: f64,
    variable_warranty_reserve_usd_per_good_vehicle: f64,
    labor_cost_usd_per_good_vehicle: f64,
    energy_cost_usd_per_good_vehicle: f64,
    useful_thermal_energy_cost_usd_per_good_vehicle: f64,
    lease_cost_usd_per_good_vehicle: f64,
    site_overhead_usd_per_good_vehicle: f64,
    maintenance_cost_usd_per_good_vehicle: f64,
    capex_amortization_usd_per_good_vehicle: f64,
    factory_conversion_cost_usd_per_good_vehicle: f64,
    factory_incurred_cost_usd_per_good_vehicle_excluding_vehicle_inputs: f64,
    modelled_factory_gate_cost_usd_per_good_vehicle_including_declared_inputs: f64,
    modelled_factory_gate_cost_local_per_good_vehicle_including_declared_inputs: f64,
    fx_evidence_class: EvidenceClass,
    local_code_gate: GateStatus,
    battery_process_safety_gate: GateStatus,
    production_release_ready: bool,
    evidence_gaps: Vec<String>,
    stations: Vec<StationResult>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    base_currency: String,
    scenario_count: usize,
    results: Vec<ScenarioResult>,
    claim_ceiling: String,
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        "tools/ev-passenger-cost-frontier/fixtures/ev_factory_design_v0_1.json".to_string()
    });
    let input = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            process::exit(2);
        }
    };
    let catalog: Catalog = match serde_json::from_str(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid EV factory design catalog: {error}");
            process::exit(2);
        }
    };
    if let Err(error) = validate_catalog(&catalog) {
        eprintln!("invalid EV factory design catalog: {error}");
        process::exit(2);
    }
    let report = evaluate_catalog(&catalog);
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize EV factory report: {error}");
            process::exit(2);
        }
    }
}

fn finite_nonnegative(name: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || value < 0.0 {
        return Err(format!("{name}_must_be_finite_and_nonnegative"));
    }
    Ok(())
}

fn fraction(name: &str, value: f64, allow_one: bool) -> Result<(), String> {
    if !value.is_finite() || value < 0.0 || if allow_one { value > 1.0 } else { value >= 1.0 } {
        return Err(format!("{name}_must_be_a_valid_fraction"));
    }
    Ok(())
}

fn validate_gate(name: &str, gate: &EvidenceGate) -> Result<(), String> {
    if gate.source_reference.trim().is_empty()
        || gate.as_of.trim().is_empty()
        || gate.note.trim().is_empty()
    {
        return Err(format!("{name}_requires_source_date_and_note"));
    }
    Ok(())
}

fn validate_catalog(catalog: &Catalog) -> Result<(), String> {
    if catalog.schema_version != "0.1.0" || catalog.benchmark_id.trim().is_empty() {
        return Err("unsupported_schema_version_or_missing_benchmark_id".to_string());
    }
    if catalog.claim_ceiling.trim().is_empty()
        || catalog.sites.is_empty()
        || catalog.templates.is_empty()
        || catalog.demand_cases_good_units_per_year.is_empty()
    {
        return Err("catalog_requires_claim_ceiling_sites_templates_and_demand_cases".to_string());
    }
    let mut site_ids = BTreeSet::new();
    for site in &catalog.sites {
        if site.site_id.trim().is_empty() || !site_ids.insert(site.site_id.clone()) {
            return Err("site_ids_must_be_nonempty_and_unique".to_string());
        }
        if site.local_currency.trim().is_empty()
            || site.country_or_region.trim().is_empty()
            || site.fx_reference_date.trim().is_empty()
        {
            return Err(format!(
                "site_{}_requires_region_currency_and_dated_fx_basis",
                site.site_id
            ));
        }
        finite_nonnegative("fx_local_per_usd", site.local_currency_per_usd)?;
        if site.local_currency_per_usd <= 0.0 {
            return Err("fx_local_per_usd_must_be_positive".to_string());
        }
        if site.operating_days_per_year == 0 || site.shifts_per_day == 0 {
            return Err(format!(
                "site_{}_days_and_shifts_must_be_positive",
                site.site_id
            ));
        }
        finite_nonnegative("paid_hours_per_shift", site.paid_hours_per_shift)?;
        finite_nonnegative(
            "unpaid_break_hours_per_shift",
            site.unpaid_break_hours_per_shift,
        )?;
        if site.paid_hours_per_shift <= site.unpaid_break_hours_per_shift
            || site.paid_hours_per_shift > 24.0
            || site.unpaid_break_hours_per_shift >= 24.0
        {
            return Err(format!("site_{}_shift_hours_are_invalid", site.site_id));
        }
        fraction("availability_fraction", site.availability_fraction, true)?;
        fraction("performance_fraction", site.performance_fraction, true)?;
        fraction(
            "good_vehicle_yield_fraction",
            site.good_vehicle_yield_fraction,
            true,
        )?;
        if site.availability_fraction <= 0.0
            || site.performance_fraction <= 0.0
            || site.good_vehicle_yield_fraction <= 0.0
        {
            return Err(format!(
                "site_{}_availability_performance_and_yield_must_be_positive",
                site.site_id
            ));
        }
        for (name, value) in [
            (
                "fully_loaded_labor_cost_per_hour_local",
                site.fully_loaded_labor_cost_per_hour_local,
            ),
            (
                "electricity_cost_per_kwh_local",
                site.electricity_cost_per_kwh_local,
            ),
            (
                "useful_thermal_energy_cost_per_kwh_local",
                site.useful_thermal_energy_cost_per_kwh_local,
            ),
            (
                "building_lease_cost_per_sqm_year_local",
                site.building_lease_cost_per_sqm_year_local,
            ),
            (
                "building_fitout_cost_per_sqm_local",
                site.building_fitout_cost_per_sqm_local,
            ),
            (
                "annual_site_overhead_local",
                site.annual_site_overhead_local,
            ),
            (
                "imported_kit_cost_per_attempt_local",
                site.imported_kit_cost_per_attempt_local,
            ),
            (
                "local_parts_cost_per_attempt_local",
                site.local_parts_cost_per_attempt_local,
            ),
            (
                "bought_painted_body_cost_per_attempt_local",
                site.bought_painted_body_cost_per_attempt_local,
            ),
            (
                "in_house_body_material_cost_per_attempt_local",
                site.in_house_body_material_cost_per_attempt_local,
            ),
            (
                "consumables_cost_per_attempt_local",
                site.consumables_cost_per_attempt_local,
            ),
            (
                "scrap_and_rework_cost_per_failed_attempt_local",
                site.scrap_and_rework_cost_per_failed_attempt_local,
            ),
            (
                "warranty_reserve_per_good_vehicle_local",
                site.warranty_reserve_per_good_vehicle_local,
            ),
            (
                "battery_quarantine_and_separation_area_sqm",
                site.battery_quarantine_and_separation_area_sqm,
            ),
        ] {
            finite_nonnegative(name, value)?;
        }
        validate_gate("local_code_gate", &site.local_code_gate)?;
        validate_gate(
            "battery_process_safety_gate",
            &site.battery_process_safety_gate,
        )?;
    }

    let mut template_ids = BTreeSet::new();
    for template in &catalog.templates {
        if template.template_id.trim().is_empty()
            || !template_ids.insert(template.template_id.clone())
        {
            return Err("template_ids_must_be_nonempty_and_unique".to_string());
        }
        if template.station_processes.is_empty()
            || template.label.trim().is_empty()
            || template.note.trim().is_empty()
        {
            return Err(format!(
                "template_{}_requires_label_stations_and_note",
                template.template_id
            ));
        }
        finite_nonnegative("layout_area_multiplier", template.layout_area_multiplier)?;
        if template.layout_area_multiplier < 1.0 {
            return Err("layout_area_multiplier_must_be_at_least_one".to_string());
        }
        finite_nonnegative(
            "annual_factory_overhead_usd",
            template.annual_factory_overhead_usd,
        )?;
        finite_nonnegative(
            "launch_validation_capex_usd",
            template.launch_validation_capex_usd,
        )?;
        let mut process_ids = BTreeSet::new();
        for station in &template.station_processes {
            if station.process_id.trim().is_empty()
                || !process_ids.insert(station.process_id.clone())
            {
                return Err(format!(
                    "template_{}_process_ids_must_be_nonempty_and_unique",
                    template.template_id
                ));
            }
            if station.process_label.trim().is_empty() {
                return Err(format!("process_{}_requires_label", station.process_id));
            }
            finite_nonnegative(
                "cycle_minutes_per_attempt",
                station.cycle_minutes_per_attempt,
            )?;
            finite_nonnegative(
                "equipment_capex_usd_per_cell",
                station.equipment_capex_usd_per_cell,
            )?;
            finite_nonnegative("footprint_sqm_per_cell", station.footprint_sqm_per_cell)?;
            finite_nonnegative("energy_kwh_per_attempt", station.energy_kwh_per_attempt)?;
            finite_nonnegative(
                "thermal_energy_kwh_per_attempt",
                station.thermal_energy_kwh_per_attempt,
            )?;
            if station.cycle_minutes_per_attempt <= 0.0 {
                return Err(format!(
                    "process_{}_cycle_minutes_must_be_positive",
                    station.process_id
                ));
            }
        }
    }
    let mut demands = BTreeSet::new();
    for demand in &catalog.demand_cases_good_units_per_year {
        if *demand == 0 || !demands.insert(*demand) {
            return Err("demand_cases_must_be_positive_and_unique".to_string());
        }
    }
    Ok(())
}

fn ceil_count(numerator: f64, denominator: f64) -> u64 {
    (numerator / denominator).ceil().max(1.0) as u64
}

fn evaluate_pair(
    site: &SiteProfile,
    template: &FactoryTemplate,
    demand_good: u64,
) -> ScenarioResult {
    let fx = site.local_currency_per_usd;
    let productive_minutes_per_cell_year = site.operating_days_per_year as f64
        * site.shifts_per_day as f64
        * (site.paid_hours_per_shift - site.unpaid_break_hours_per_shift)
        * 60.0
        * site.availability_fraction
        * site.performance_fraction;
    let planned_attempts = (demand_good as f64 / site.good_vehicle_yield_fraction).ceil() as u64;
    let takt = productive_minutes_per_cell_year / planned_attempts as f64;

    let mut stations = Vec::new();
    for station in &template.station_processes {
        let parallel_cells = ceil_count(
            station.cycle_minutes_per_attempt * planned_attempts as f64,
            productive_minutes_per_cell_year,
        );
        let station_capacity = (productive_minutes_per_cell_year * parallel_cells as f64
            / station.cycle_minutes_per_attempt)
            .floor() as u64;
        let operators_per_shift = station.operators_per_cell_per_shift as u64 * parallel_cells;
        stations.push(StationResult {
            process_id: station.process_id.clone(),
            process_label: station.process_label.clone(),
            cycle_minutes_per_attempt: station.cycle_minutes_per_attempt,
            parallel_cells_required: parallel_cells,
            planned_attempt_capacity_per_year: station_capacity,
            equipment_capex_usd: station.equipment_capex_usd_per_cell * parallel_cells as f64,
            operators_per_shift,
            total_active_positions_across_shifts: operators_per_shift * site.shifts_per_day as u64,
            footprint_sqm: station.footprint_sqm_per_cell * parallel_cells as f64,
            energy_kwh_per_attempt: station.energy_kwh_per_attempt,
            thermal_energy_kwh_per_attempt: station.thermal_energy_kwh_per_attempt,
        });
    }
    stations.sort_by(|a, b| a.process_id.cmp(&b.process_id));

    let bottleneck_attempt_capacity = stations
        .iter()
        .map(|station| station.planned_attempt_capacity_per_year)
        .min()
        .unwrap_or(0);
    let bottleneck_good_capacity =
        (bottleneck_attempt_capacity as f64 * site.good_vehicle_yield_fraction).floor() as u64;
    let total_cells: u64 = stations
        .iter()
        .map(|station| station.parallel_cells_required)
        .sum();
    let total_positions: u64 = stations
        .iter()
        .map(|station| station.total_active_positions_across_shifts)
        .sum();
    let base_station_area: f64 = stations.iter().map(|station| station.footprint_sqm).sum();
    let building_area = base_station_area * template.layout_area_multiplier
        + site.battery_quarantine_and_separation_area_sqm;
    let equipment_capex: f64 = stations
        .iter()
        .map(|station| station.equipment_capex_usd)
        .sum();
    let fitout_capex = building_area * site.building_fitout_cost_per_sqm_local / fx;
    let total_installed_capex =
        equipment_capex + fitout_capex + template.launch_validation_capex_usd;

    // This is a planned-cost case, not a claim that demand or market sales will materialize.
    let output_good = demand_good.min(bottleneck_good_capacity).max(1);
    let attempts_at_output = (output_good as f64 / site.good_vehicle_yield_fraction).ceil();
    let annual_positions = total_positions as f64;
    let annual_labor_local = annual_positions
        * site.operating_days_per_year as f64
        * site.paid_hours_per_shift
        * site.fully_loaded_labor_cost_per_hour_local;
    let annual_labor_usd = annual_labor_local / fx;

    let energy_kwh_per_attempt: f64 = stations
        .iter()
        .map(|station| station.energy_kwh_per_attempt)
        .sum();
    let annual_energy_kwh = energy_kwh_per_attempt * attempts_at_output;
    let annual_energy_cost_usd = annual_energy_kwh * site.electricity_cost_per_kwh_local / fx;
    let thermal_energy_kwh_per_attempt: f64 = stations
        .iter()
        .map(|station| station.thermal_energy_kwh_per_attempt)
        .sum();
    let annual_thermal_energy_cost_usd = thermal_energy_kwh_per_attempt
        * attempts_at_output
        * site.useful_thermal_energy_cost_per_kwh_local
        / fx;
    let annual_lease_usd = building_area * site.building_lease_cost_per_sqm_year_local / fx;
    let annual_site_overhead_usd =
        site.annual_site_overhead_local / fx + template.annual_factory_overhead_usd;
    let annual_maintenance_usd = equipment_capex * 0.04;
    let annual_amortization_usd =
        equipment_capex / 10.0 + fitout_capex / 10.0 + template.launch_validation_capex_usd / 5.0;
    let annual_factory_fixed_cost = annual_labor_usd
        + annual_energy_cost_usd
        + annual_thermal_energy_cost_usd
        + annual_lease_usd
        + annual_site_overhead_usd
        + annual_maintenance_usd
        + annual_amortization_usd;

    let imported_kit_cost =
        site.imported_kit_cost_per_attempt_local / fx / site.good_vehicle_yield_fraction;
    let local_parts_cost =
        site.local_parts_cost_per_attempt_local / fx / site.good_vehicle_yield_fraction;
    let body_cost_local = match template.strategy {
        FactoryStrategy::IntegratedFlexible => site.in_house_body_material_cost_per_attempt_local,
        FactoryStrategy::ContractAssembly | FactoryStrategy::ModularCkd => {
            site.bought_painted_body_cost_per_attempt_local
        }
    };
    let body_cost = body_cost_local / fx / site.good_vehicle_yield_fraction;
    let consumables_cost =
        site.consumables_cost_per_attempt_local / fx / site.good_vehicle_yield_fraction;
    let scrap_rework_cost = site.scrap_and_rework_cost_per_failed_attempt_local
        * (1.0 - site.good_vehicle_yield_fraction)
        / site.good_vehicle_yield_fraction
        / fx;
    let warranty_cost = site.warranty_reserve_per_good_vehicle_local / fx;
    let variable_vehicle_inputs = imported_kit_cost
        + local_parts_cost
        + body_cost
        + consumables_cost
        + scrap_rework_cost
        + warranty_cost;

    let labor_per_good = annual_labor_usd / output_good as f64;
    let energy_per_good = annual_energy_cost_usd / output_good as f64;
    let lease_per_good = annual_lease_usd / output_good as f64;
    let overhead_per_good = annual_site_overhead_usd / output_good as f64;
    let maintenance_per_good = annual_maintenance_usd / output_good as f64;
    let amortization_per_good = annual_amortization_usd / output_good as f64;
    let factory_conversion_per_good = labor_per_good
        + energy_per_good
        + lease_per_good
        + overhead_per_good
        + maintenance_per_good
        + amortization_per_good;
    let all_in_incremental_gate_cost = factory_conversion_per_good + variable_vehicle_inputs;

    let mut gaps = Vec::new();
    if site.fx_evidence_class != EvidenceClass::PublicMarketData {
        gaps.push(
            "FX normalization is illustrative or unqualified for investment decisions".to_string(),
        );
    }
    if !matches!(
        site.site_cost_evidence_class,
        EvidenceClass::Measured | EvidenceClass::AuditedCost
    ) {
        gaps.push("wages, utilities, lease, fit-out, site overhead and component inputs are not measured or audited site data".to_string());
    }
    if !matches!(
        site.process_evidence_class,
        EvidenceClass::Measured | EvidenceClass::IndependentTest
    ) {
        gaps.push("station cycle times and equipment requirements are not supported by measured process studies or independent tests".to_string());
    }
    if !matches!(
        site.yield_evidence_class,
        EvidenceClass::Measured | EvidenceClass::IndependentTest
    ) {
        gaps.push(
            "saleable yield is not supported by measured production or independent testing"
                .to_string(),
        );
    }
    if site.local_code_gate.status != GateStatus::Passed
        || !matches!(
            site.local_code_gate.evidence_class,
            EvidenceClass::OfficialRecord | EvidenceClass::IndependentTest
        )
    {
        gaps.push("local building, fire, electrical, environmental and employment requirements are not proven satisfied".to_string());
    }
    if site.battery_process_safety_gate.status != GateStatus::Passed
        || site.battery_process_safety_gate.evidence_class != EvidenceClass::IndependentTest
    {
        gaps.push("battery handling, quarantine, HV isolation and emergency-response design are not independently verified".to_string());
    }
    if site.site_cost_evidence_class == EvidenceClass::Synthetic
        || site.process_evidence_class == EvidenceClass::Synthetic
        || site.yield_evidence_class == EvidenceClass::Synthetic
    {
        gaps.push("one or more cost/process/yield inputs are synthetic; replace them with dated site evidence before investment decisions".to_string());
    }

    ScenarioResult {
        site_id: site.site_id.clone(),
        country_or_region: site.country_or_region.clone(),
        local_currency: site.local_currency.clone(),
        template_id: template.template_id.clone(),
        template_label: template.label.clone(),
        strategy: template.strategy,
        target_good_vehicles_per_year: demand_good,
        planned_attempts_per_year: planned_attempts,
        good_vehicle_yield_fraction: site.good_vehicle_yield_fraction,
        productive_minutes_per_cell_year,
        takt_minutes_per_attempt: takt,
        bottleneck_attempt_capacity_per_year: bottleneck_attempt_capacity,
        bottleneck_good_capacity_per_year: bottleneck_good_capacity,
        capacity_headroom_fraction: if demand_good == 0 {
            0.0
        } else {
            (bottleneck_good_capacity as f64 / demand_good as f64) - 1.0
        },
        planned_capacity_utilization_fraction: output_good as f64
            / bottleneck_good_capacity.max(1) as f64,
        parallel_cell_count: total_cells,
        total_active_positions_across_shifts: total_positions,
        building_area_sqm: building_area,
        equipment_capex_usd: equipment_capex,
        fitout_capex_usd: fitout_capex,
        launch_validation_capex_usd: template.launch_validation_capex_usd,
        total_installed_capex_usd: total_installed_capex,
        annual_labor_cost_usd: annual_labor_usd,
        annual_energy_cost_usd,
        annual_useful_thermal_energy_cost_usd: annual_thermal_energy_cost_usd,
        annual_building_lease_usd: annual_lease_usd,
        annual_site_overhead_usd,
        annual_equipment_maintenance_usd: annual_maintenance_usd,
        annual_capex_amortization_usd: annual_amortization_usd,
        annual_factory_fixed_cost_usd: annual_factory_fixed_cost,
        variable_imported_kit_cost_usd_per_good_vehicle: imported_kit_cost,
        variable_local_parts_cost_usd_per_good_vehicle: local_parts_cost,
        variable_body_cost_usd_per_good_vehicle: body_cost,
        variable_consumables_cost_usd_per_good_vehicle: consumables_cost,
        variable_scrap_rework_cost_usd_per_good_vehicle: scrap_rework_cost,
        variable_warranty_reserve_usd_per_good_vehicle: warranty_cost,
        labor_cost_usd_per_good_vehicle: labor_per_good,
        energy_cost_usd_per_good_vehicle: energy_per_good,
        useful_thermal_energy_cost_usd_per_good_vehicle: annual_thermal_energy_cost_usd
            / output_good as f64,
        lease_cost_usd_per_good_vehicle: lease_per_good,
        site_overhead_usd_per_good_vehicle: overhead_per_good,
        maintenance_cost_usd_per_good_vehicle: maintenance_per_good,
        capex_amortization_usd_per_good_vehicle: amortization_per_good,
        factory_conversion_cost_usd_per_good_vehicle: factory_conversion_per_good,
        factory_incurred_cost_usd_per_good_vehicle_excluding_vehicle_inputs:
            factory_conversion_per_good,
        modelled_factory_gate_cost_usd_per_good_vehicle_including_declared_inputs:
            all_in_incremental_gate_cost,
        modelled_factory_gate_cost_local_per_good_vehicle_including_declared_inputs:
            all_in_incremental_gate_cost * fx,
        fx_evidence_class: site.fx_evidence_class,
        local_code_gate: site.local_code_gate.status,
        battery_process_safety_gate: site.battery_process_safety_gate.status,
        production_release_ready: gaps.is_empty(),
        evidence_gaps: gaps,
        stations,
        claim_ceiling: CLAIM_CEILING.to_string(),
    }
}

fn evaluate_catalog(catalog: &Catalog) -> Report {
    let mut sites: Vec<&SiteProfile> = catalog.sites.iter().collect();
    let mut templates: Vec<&FactoryTemplate> = catalog.templates.iter().collect();
    let mut demands = catalog.demand_cases_good_units_per_year.clone();
    sites.sort_by(|a, b| a.site_id.cmp(&b.site_id));
    templates.sort_by(|a, b| a.template_id.cmp(&b.template_id));
    demands.sort_unstable();
    let mut results = Vec::new();
    for site in sites {
        for demand in &demands {
            for template in &templates {
                results.push(evaluate_pair(site, template, *demand));
            }
        }
    }
    Report {
        schema_version: catalog.schema_version.clone(),
        benchmark_id: catalog.benchmark_id.clone(),
        base_currency: "USD".to_string(),
        scenario_count: results.len(),
        results,
        claim_ceiling: catalog.claim_ceiling.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cost_fixture() -> Catalog {
        let gate = || EvidenceGate {
            status: GateStatus::Unknown,
            evidence_class: EvidenceClass::Unknown,
            source_reference: "unknown://not-verified".to_string(),
            as_of: "2026-10-20".to_string(),
            note: "Not verified; test and scenario only.".to_string(),
        };
        Catalog {
            schema_version: "0.1.0".to_string(),
            benchmark_id: "TEST-FACTORY".to_string(),
            sites: vec![SiteProfile {
                site_id: "test-site".to_string(),
                country_or_region: "Test Region".to_string(),
                local_currency: "TST".to_string(),
                local_currency_per_usd: 10.0,
                fx_reference_date: "2026-10-20".to_string(),
                fx_evidence_class: EvidenceClass::PublicMarketData,
                site_cost_evidence_class: EvidenceClass::AuditedCost,
                process_evidence_class: EvidenceClass::Measured,
                yield_evidence_class: EvidenceClass::Measured,
                operating_days_per_year: 200,
                shifts_per_day: 1,
                paid_hours_per_shift: 8.0,
                unpaid_break_hours_per_shift: 1.0,
                availability_fraction: 0.9,
                performance_fraction: 0.9,
                good_vehicle_yield_fraction: 0.9,
                fully_loaded_labor_cost_per_hour_local: 100.0,
                electricity_cost_per_kwh_local: 1.0,
                useful_thermal_energy_cost_per_kwh_local: 0.5,
                building_lease_cost_per_sqm_year_local: 10.0,
                building_fitout_cost_per_sqm_local: 100.0,
                annual_site_overhead_local: 10000.0,
                imported_kit_cost_per_attempt_local: 1000.0,
                local_parts_cost_per_attempt_local: 300.0,
                bought_painted_body_cost_per_attempt_local: 500.0,
                in_house_body_material_cost_per_attempt_local: 350.0,
                consumables_cost_per_attempt_local: 20.0,
                scrap_and_rework_cost_per_failed_attempt_local: 50.0,
                warranty_reserve_per_good_vehicle_local: 60.0,
                battery_quarantine_and_separation_area_sqm: 15.0,
                local_code_gate: gate(),
                battery_process_safety_gate: gate(),
                note: "test only".to_string(),
            }],
            demand_cases_good_units_per_year: vec![100],
            templates: vec![FactoryTemplate {
                template_id: "manual".to_string(),
                label: "Manual baseline".to_string(),
                strategy: FactoryStrategy::ContractAssembly,
                layout_area_multiplier: 1.5,
                annual_factory_overhead_usd: 1000.0,
                launch_validation_capex_usd: 500.0,
                station_processes: vec![
                    StationProcess {
                        process_id: "assembly".to_string(),
                        process_label: "Assembly".to_string(),
                        cycle_minutes_per_attempt: 20.0,
                        equipment_capex_usd_per_cell: 10000.0,
                        footprint_sqm_per_cell: 20.0,
                        operators_per_cell_per_shift: 2,
                        energy_kwh_per_attempt: 10.0,
                        thermal_energy_kwh_per_attempt: 0.0,
                    },
                    StationProcess {
                        process_id: "eol".to_string(),
                        process_label: "End-of-line test".to_string(),
                        cycle_minutes_per_attempt: 30.0,
                        equipment_capex_usd_per_cell: 20000.0,
                        footprint_sqm_per_cell: 10.0,
                        operators_per_cell_per_shift: 1,
                        energy_kwh_per_attempt: 2.0,
                        thermal_energy_kwh_per_attempt: 0.0,
                    },
                ],
                note: "test only".to_string(),
            }],
            claim_ceiling: CLAIM_CEILING.to_string(),
        }
    }

    #[test]
    fn planned_capacity_covers_target_output_for_every_station() {
        let catalog = cost_fixture();
        let result = evaluate_catalog(&catalog).results.remove(0);
        assert!(result.bottleneck_good_capacity_per_year >= result.target_good_vehicles_per_year);
        assert!(result.stations.iter().all(|station| {
            station.planned_attempt_capacity_per_year >= result.planned_attempts_per_year
        }));
    }

    #[test]
    fn takt_time_falls_as_target_volume_rises() {
        let mut catalog = cost_fixture();
        catalog.demand_cases_good_units_per_year = vec![100, 1000];
        let report = evaluate_catalog(&catalog);
        assert!(
            report.results[0].takt_minutes_per_attempt > report.results[1].takt_minutes_per_attempt
        );
        assert!(report.results[1].parallel_cell_count >= report.results[0].parallel_cell_count);
    }

    #[test]
    fn low_yield_increases_attempts_and_material_cost_per_good_unit() {
        let mut good_yield = cost_fixture();
        let mut bad_yield = cost_fixture();
        good_yield.sites[0].good_vehicle_yield_fraction = 0.98;
        bad_yield.sites[0].good_vehicle_yield_fraction = 0.8;
        let a = evaluate_catalog(&good_yield).results.remove(0);
        let b = evaluate_catalog(&bad_yield).results.remove(0);
        assert!(b.planned_attempts_per_year > a.planned_attempts_per_year);
        assert!(
            b.variable_imported_kit_cost_usd_per_good_vehicle
                > a.variable_imported_kit_cost_usd_per_good_vehicle
        );
    }

    #[test]
    fn energy_and_labor_change_costs_monotonically() {
        let mut base = cost_fixture();
        let mut expensive_energy = cost_fixture();
        expensive_energy.sites[0].electricity_cost_per_kwh_local = 3.0;
        let a = evaluate_catalog(&base).results.remove(0);
        let b = evaluate_catalog(&expensive_energy).results.remove(0);
        assert!(b.energy_cost_usd_per_good_vehicle > a.energy_cost_usd_per_good_vehicle);
        let mut expensive_heat = cost_fixture();
        expensive_heat.templates[0].station_processes[0].thermal_energy_kwh_per_attempt = 30.0;
        expensive_heat.sites[0].useful_thermal_energy_cost_per_kwh_local = 2.0;
        let thermal = evaluate_catalog(&expensive_heat).results.remove(0);
        assert!(
            thermal.useful_thermal_energy_cost_usd_per_good_vehicle
                > a.useful_thermal_energy_cost_usd_per_good_vehicle
        );
        base.sites[0].fully_loaded_labor_cost_per_hour_local *= 2.0;
        let c = evaluate_catalog(&base).results.remove(0);
        assert!(c.labor_cost_usd_per_good_vehicle > a.labor_cost_usd_per_good_vehicle);
    }

    #[test]
    fn unknown_compliance_gates_prevent_release_readiness() {
        let result = evaluate_catalog(&cost_fixture()).results.remove(0);
        assert!(!result.production_release_ready);
        assert_eq!(result.local_code_gate, GateStatus::Unknown);
        assert_eq!(result.battery_process_safety_gate, GateStatus::Unknown);
        assert!(!result.evidence_gaps.is_empty());
    }

    #[test]
    fn permutations_keep_report_deterministic() {
        let mut a = cost_fixture();
        let mut b = cost_fixture();
        a.demand_cases_good_units_per_year = vec![1000, 100];
        b.demand_cases_good_units_per_year = vec![100, 1000];
        assert_eq!(evaluate_catalog(&a), evaluate_catalog(&b));
    }

    #[test]
    fn rejects_duplicate_stations_and_zero_demand() {
        let mut bad = cost_fixture();
        let duplicate = bad.templates[0].station_processes[0].clone();
        bad.templates[0].station_processes.push(duplicate);
        assert!(validate_catalog(&bad).is_err());
        let mut zero = cost_fixture();
        zero.demand_cases_good_units_per_year = vec![0];
        assert!(validate_catalog(&zero).is_err());
    }

    #[test]
    fn fixed_cost_amortization_uses_installed_cost_and_life_assumptions() {
        let r = evaluate_catalog(&cost_fixture()).results.remove(0);
        assert!(r.annual_capex_amortization_usd > 0.0);
        assert!(r.total_installed_capex_usd >= r.equipment_capex_usd);
    }
}
