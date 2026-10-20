use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, env, fs, process};

const CLAIM_CEILING: &str = "Station-level capital-allocation sensitivity only. Synthetic options are not vendor prices, production-time observations, labor study results, safety approvals or a guarantee of financial return.";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    schema_version: String,
    benchmark_id: String,
    evaluation_horizon_years: u32,
    asset_life_years: u32,
    discount_rate_fraction: f64,
    demand_cases_good_units_per_year: Vec<u64>,
    sites: Vec<SiteProfile>,
    families: Vec<StationFamily>,
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
    operating_days_per_year: u32,
    shifts_per_day: u32,
    paid_hours_per_shift: f64,
    unpaid_break_hours_per_shift: f64,
    availability_fraction: f64,
    performance_fraction: f64,
    fully_loaded_labor_cost_per_hour_local: f64,
    electricity_cost_per_kwh_local: f64,
    useful_thermal_energy_cost_per_kwh_local: f64,
    building_lease_cost_per_sqm_year_local: f64,
    site_cost_evidence_class: EvidenceClass,
    local_compliance_gate: Gate,
    battery_hv_safety_gate: Gate,
    note: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct StationFamily {
    family_id: String,
    label: String,
    options: Vec<StationOption>,
    note: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct StationOption {
    option_id: String,
    label: String,
    mode: AutomationMode,
    cycle_minutes_per_attempt: f64,
    equipment_capex_usd_per_cell: f64,
    integration_capex_usd: f64,
    operators_per_cell_per_shift: u32,
    footprint_sqm_per_cell: f64,
    electrical_kwh_per_attempt: f64,
    useful_thermal_kwh_per_attempt: f64,
    good_yield_fraction: f64,
    failed_attempt_cost_local: f64,
    rework_fraction: f64,
    rework_cost_per_attempt_local: f64,
    annual_maintenance_fraction_of_capex: f64,
    evidence_class: EvidenceClass,
    technical_gate: Gate,
    safety_gate: Gate,
    note: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AutomationMode {
    Manual,
    Assisted,
    Automated,
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Gate {
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

#[derive(Debug, Clone, Serialize, PartialEq)]
struct OptionResult {
    site_id: String,
    country_or_region: String,
    local_currency: String,
    family_id: String,
    family_label: String,
    option_id: String,
    option_label: String,
    automation_mode: AutomationMode,
    target_good_units_per_year: u64,
    planned_attempts_per_year: u64,
    productive_minutes_per_cell_year: f64,
    required_cells: u64,
    planned_attempt_capacity_per_year: u64,
    good_capacity_per_year: u64,
    capacity_headroom_fraction: f64,
    planned_capacity_utilization_fraction: f64,
    operators_per_shift: u64,
    annual_paid_operator_hours: f64,
    installed_equipment_capex_usd: f64,
    integration_capex_usd: f64,
    total_installed_capex_usd: f64,
    station_footprint_sqm: f64,
    annual_labor_cost_usd: f64,
    annual_electricity_cost_usd: f64,
    annual_useful_thermal_cost_usd: f64,
    annual_lease_cost_usd: f64,
    annual_maintenance_cost_usd: f64,
    expected_failed_attempts_per_year: f64,
    annual_failed_attempt_cost_usd: f64,
    expected_rework_attempts_per_year: f64,
    annual_rework_cost_usd: f64,
    annual_operating_cost_usd_excluding_capex: f64,
    annual_capex_amortization_usd: f64,
    annualized_total_station_cost_usd: f64,
    total_station_cost_usd_per_good_unit: f64,
    is_manual_baseline: bool,
    manual_baseline_annual_operating_cost_usd: f64,
    manual_baseline_installed_capex_usd: f64,
    annual_operating_savings_vs_manual_usd: f64,
    incremental_installed_capex_vs_manual_usd: f64,
    simple_payback_years: Option<f64>,
    discounted_npv_vs_manual_usd: f64,
    annualized_cost_delta_vs_manual_usd_per_good_unit: f64,
    evidence_release_grade: bool,
    local_compliance_gate: GateStatus,
    battery_hv_safety_gate: GateStatus,
    safety_and_technical_gates_passed: bool,
    release_ready: bool,
    evidence_gaps: Vec<String>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    base_currency: String,
    evaluation_horizon_years: u32,
    asset_life_years: u32,
    discount_rate_fraction: f64,
    result_count: usize,
    results: Vec<OptionResult>,
    claim_ceiling: String,
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        "tools/ev-passenger-cost-frontier/fixtures/ev_automation_breakeven_v0_1.json".to_string()
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
            eprintln!("invalid EV automation catalog: {error}");
            process::exit(2);
        }
    };
    if let Err(error) = validate_catalog(&catalog) {
        eprintln!("invalid EV automation catalog: {error}");
        process::exit(2);
    }
    let report = evaluate_catalog(&catalog);
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize EV automation report: {error}");
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

fn validate_gate(name: &str, gate: &Gate) -> Result<(), String> {
    if gate.source_reference.trim().is_empty()
        || gate.as_of.trim().is_empty()
        || gate.note.trim().is_empty()
    {
        return Err(format!("{name}_requires_source_date_and_note"));
    }
    Ok(())
}

fn validate_catalog(catalog: &Catalog) -> Result<(), String> {
    if catalog.schema_version != "0.1.0"
        || catalog.benchmark_id.trim().is_empty()
        || catalog.claim_ceiling.trim().is_empty()
    {
        return Err("invalid_version_benchmark_id_or_claim_ceiling".to_string());
    }
    if catalog.evaluation_horizon_years == 0 || catalog.asset_life_years == 0 {
        return Err("evaluation_horizon_and_asset_life_must_be_positive".to_string());
    }
    fraction(
        "discount_rate_fraction",
        catalog.discount_rate_fraction,
        true,
    )?;
    if catalog.sites.is_empty()
        || catalog.families.is_empty()
        || catalog.demand_cases_good_units_per_year.is_empty()
    {
        return Err("catalog_requires_sites_families_and_demand_cases".to_string());
    }
    let mut demands = BTreeSet::new();
    for demand in &catalog.demand_cases_good_units_per_year {
        if *demand == 0 || !demands.insert(*demand) {
            return Err("demand_cases_must_be_positive_and_unique".to_string());
        }
    }
    let mut site_ids = BTreeSet::new();
    for site in &catalog.sites {
        if site.site_id.trim().is_empty() || !site_ids.insert(site.site_id.clone()) {
            return Err("site_ids_must_be_nonempty_and_unique".to_string());
        }
        if site.country_or_region.trim().is_empty()
            || site.local_currency.trim().is_empty()
            || site.fx_reference_date.trim().is_empty()
            || site.note.trim().is_empty()
        {
            return Err(format!(
                "site_{}_requires_region_currency_fx_date_and_note",
                site.site_id
            ));
        }
        finite_nonnegative("local_currency_per_usd", site.local_currency_per_usd)?;
        if site.local_currency_per_usd <= 0.0 {
            return Err("local_currency_per_usd_must_be_positive".to_string());
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
        if site.availability_fraction <= 0.0 || site.performance_fraction <= 0.0 {
            return Err(format!(
                "site_{}_availability_and_performance_must_be_positive",
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
        ] {
            finite_nonnegative(name, value)?;
        }
        validate_gate("local_compliance_gate", &site.local_compliance_gate)?;
        validate_gate("battery_hv_safety_gate", &site.battery_hv_safety_gate)?;
    }

    let mut family_ids = BTreeSet::new();
    for family in &catalog.families {
        if family.family_id.trim().is_empty() || !family_ids.insert(family.family_id.clone()) {
            return Err("family_ids_must_be_nonempty_and_unique".to_string());
        }
        if family.label.trim().is_empty()
            || family.note.trim().is_empty()
            || family.options.is_empty()
        {
            return Err(format!(
                "family_{}_requires_label_options_and_note",
                family.family_id
            ));
        }
        let mut option_ids = BTreeSet::new();
        let mut manual_count = 0;
        for option in &family.options {
            if option.option_id.trim().is_empty() || !option_ids.insert(option.option_id.clone()) {
                return Err(format!(
                    "family_{}_option_ids_must_be_nonempty_and_unique",
                    family.family_id
                ));
            }
            if option.label.trim().is_empty() || option.note.trim().is_empty() {
                return Err(format!(
                    "option_{}_requires_label_and_note",
                    option.option_id
                ));
            }
            if option.option_id == "manual" && option.mode == AutomationMode::Manual {
                manual_count += 1;
            }
            if option.mode == AutomationMode::Manual && option.option_id != "manual" {
                return Err("manual_mode_option_must_use_option_id_manual".to_string());
            }
            if option.option_id == "manual" && option.mode != AutomationMode::Manual {
                return Err("manual_option_id_requires_manual_mode".to_string());
            }
            finite_nonnegative(
                "cycle_minutes_per_attempt",
                option.cycle_minutes_per_attempt,
            )?;
            if option.cycle_minutes_per_attempt <= 0.0 {
                return Err(format!(
                    "option_{}_cycle_time_must_be_positive",
                    option.option_id
                ));
            }
            for (name, value) in [
                (
                    "equipment_capex_usd_per_cell",
                    option.equipment_capex_usd_per_cell,
                ),
                ("integration_capex_usd", option.integration_capex_usd),
                ("footprint_sqm_per_cell", option.footprint_sqm_per_cell),
                (
                    "electrical_kwh_per_attempt",
                    option.electrical_kwh_per_attempt,
                ),
                (
                    "useful_thermal_kwh_per_attempt",
                    option.useful_thermal_kwh_per_attempt,
                ),
                (
                    "failed_attempt_cost_local",
                    option.failed_attempt_cost_local,
                ),
                (
                    "rework_cost_per_attempt_local",
                    option.rework_cost_per_attempt_local,
                ),
            ] {
                finite_nonnegative(name, value)?;
            }
            fraction("good_yield_fraction", option.good_yield_fraction, true)?;
            fraction("rework_fraction", option.rework_fraction, true)?;
            fraction(
                "annual_maintenance_fraction_of_capex",
                option.annual_maintenance_fraction_of_capex,
                true,
            )?;
            if option.good_yield_fraction <= 0.0 {
                return Err(format!(
                    "option_{}_good_yield_must_be_positive",
                    option.option_id
                ));
            }
            validate_gate("technical_gate", &option.technical_gate)?;
            validate_gate("safety_gate", &option.safety_gate)?;
        }
        if manual_count != 1 {
            return Err(format!(
                "family_{}_must_have_exactly_one_manual_baseline",
                family.family_id
            ));
        }
    }
    Ok(())
}

fn annuity_factor(rate: f64, years: u32) -> f64 {
    if rate.abs() < 1e-12 {
        return years as f64;
    }
    (1.0 - (1.0 + rate).powi(-(years as i32))) / rate
}

#[derive(Debug, Clone)]
struct Computed {
    attempts: u64,
    productive_minutes_per_cell_year: f64,
    cells: u64,
    capacity_attempts: u64,
    good_capacity: u64,
    operators_per_shift: u64,
    annual_paid_operator_hours: f64,
    equipment_capex: f64,
    total_capex: f64,
    footprint: f64,
    labor: f64,
    electric: f64,
    thermal: f64,
    lease: f64,
    maintenance: f64,
    failed_count: f64,
    failed_cost: f64,
    rework_count: f64,
    rework_cost: f64,
    annual_opex: f64,
    amortization: f64,
    annual_total: f64,
    unit_cost: f64,
    capacity_utilization: f64,
    headroom: f64,
}

fn compute(
    site: &SiteProfile,
    option: &StationOption,
    demand: u64,
    asset_life_years: u32,
) -> Computed {
    let available = site.operating_days_per_year as f64
        * site.shifts_per_day as f64
        * (site.paid_hours_per_shift - site.unpaid_break_hours_per_shift)
        * 60.0
        * site.availability_fraction
        * site.performance_fraction;
    let attempts = (demand as f64 / option.good_yield_fraction).ceil() as u64;
    let cells = ((option.cycle_minutes_per_attempt * attempts as f64) / available)
        .ceil()
        .max(1.0) as u64;
    let capacity_attempts =
        ((available * cells as f64) / option.cycle_minutes_per_attempt + 1e-9).floor() as u64;
    let good_capacity =
        (capacity_attempts as f64 * option.good_yield_fraction + 1e-9).floor() as u64;
    let operators_per_shift = cells * option.operators_per_cell_per_shift as u64;
    let paid_hours = operators_per_shift as f64
        * site.shifts_per_day as f64
        * site.operating_days_per_year as f64
        * site.paid_hours_per_shift;
    let fx = site.local_currency_per_usd;
    let labor = paid_hours * site.fully_loaded_labor_cost_per_hour_local / fx;
    let electric =
        attempts as f64 * option.electrical_kwh_per_attempt * site.electricity_cost_per_kwh_local
            / fx;
    let thermal = attempts as f64
        * option.useful_thermal_kwh_per_attempt
        * site.useful_thermal_energy_cost_per_kwh_local
        / fx;
    let equipment_capex = cells as f64 * option.equipment_capex_usd_per_cell;
    let total_capex = equipment_capex + option.integration_capex_usd;
    let footprint = cells as f64 * option.footprint_sqm_per_cell;
    let lease = footprint * site.building_lease_cost_per_sqm_year_local / fx;
    let maintenance = total_capex * option.annual_maintenance_fraction_of_capex;
    let failed_count = attempts as f64 * (1.0 - option.good_yield_fraction);
    let failed_cost = failed_count * option.failed_attempt_cost_local / fx;
    let rework_count = attempts as f64 * option.rework_fraction;
    let rework_cost = rework_count * option.rework_cost_per_attempt_local / fx;
    let annual_opex = labor + electric + thermal + lease + maintenance + failed_cost + rework_cost;
    let amortization = total_capex / asset_life_years as f64;
    let annual_total = annual_opex + amortization;
    Computed {
        attempts,
        productive_minutes_per_cell_year: available,
        cells,
        capacity_attempts,
        good_capacity,
        operators_per_shift,
        annual_paid_operator_hours: paid_hours,
        equipment_capex,
        total_capex,
        footprint,
        labor,
        electric,
        thermal,
        lease,
        maintenance,
        failed_count,
        failed_cost,
        rework_count,
        rework_cost,
        annual_opex,
        amortization,
        annual_total,
        unit_cost: annual_total / demand as f64,
        capacity_utilization: demand as f64 / good_capacity.max(1) as f64,
        headroom: good_capacity as f64 / demand as f64 - 1.0,
    }
}

fn evaluate_catalog(catalog: &Catalog) -> Report {
    let mut sites: Vec<&SiteProfile> = catalog.sites.iter().collect();
    let mut families: Vec<&StationFamily> = catalog.families.iter().collect();
    let mut demands = catalog.demand_cases_good_units_per_year.clone();
    sites.sort_by(|a, b| a.site_id.cmp(&b.site_id));
    families.sort_by(|a, b| a.family_id.cmp(&b.family_id));
    demands.sort_unstable();
    let annuity = annuity_factor(
        catalog.discount_rate_fraction,
        catalog.evaluation_horizon_years,
    );
    let mut results = Vec::new();

    for site in sites {
        for family in &families {
            let baseline = family
                .options
                .iter()
                .find(|o| o.option_id == "manual")
                .unwrap();
            for demand in &demands {
                let base = compute(site, baseline, *demand, catalog.asset_life_years);
                for option in &family.options {
                    let c = compute(site, option, *demand, catalog.asset_life_years);
                    let operating_savings = base.annual_opex - c.annual_opex;
                    let capex_delta = c.total_capex - base.total_capex;
                    let payback = if capex_delta <= 0.0 && operating_savings > 0.0 {
                        Some(0.0)
                    } else if capex_delta > 0.0 && operating_savings > 0.0 {
                        Some(capex_delta / operating_savings)
                    } else {
                        None
                    };
                    let npv = operating_savings * annuity - capex_delta;
                    let mut gaps = Vec::new();
                    let evidence_release = option.evidence_class == EvidenceClass::Measured
                        || option.evidence_class == EvidenceClass::AuditedCost;
                    if site.fx_evidence_class != EvidenceClass::PublicMarketData {
                        gaps.push("FX input is not qualified public market data".to_string());
                    }
                    if site.site_cost_evidence_class != EvidenceClass::Measured
                        && site.site_cost_evidence_class != EvidenceClass::AuditedCost
                    {
                        gaps.push(
                            "regional wage, utility and lease inputs are not measured or audited"
                                .to_string(),
                        );
                    }
                    if !evidence_release {
                        gaps.push(
                            "station cost/cycle/yield inputs are not measured or audited"
                                .to_string(),
                        );
                    }
                    if option.technical_gate.status != GateStatus::Passed
                        || !matches!(
                            option.technical_gate.evidence_class,
                            EvidenceClass::Measured | EvidenceClass::IndependentTest
                        )
                    {
                        gaps.push("technical cycle/quality/maintainability gate is not independently qualified".to_string());
                    }
                    let safety_ok = option.safety_gate.status == GateStatus::Passed
                        && option.safety_gate.evidence_class == EvidenceClass::IndependentTest;
                    if !safety_ok {
                        gaps.push("station safety gate is not independently qualified".to_string());
                    }
                    if site.local_compliance_gate.status != GateStatus::Passed
                        || !matches!(
                            site.local_compliance_gate.evidence_class,
                            EvidenceClass::OfficialRecord | EvidenceClass::IndependentTest
                        )
                    {
                        gaps.push("local code and compliance gate is unresolved".to_string());
                    }
                    if site.battery_hv_safety_gate.status != GateStatus::Passed
                        || site.battery_hv_safety_gate.evidence_class
                            != EvidenceClass::IndependentTest
                    {
                        gaps.push("site battery/HV handling gate is unresolved".to_string());
                    }
                    let local_ok = site.local_compliance_gate.status == GateStatus::Passed
                        && matches!(
                            site.local_compliance_gate.evidence_class,
                            EvidenceClass::OfficialRecord | EvidenceClass::IndependentTest
                        );
                    let battery_site_ok = site.battery_hv_safety_gate.status == GateStatus::Passed
                        && site.battery_hv_safety_gate.evidence_class
                            == EvidenceClass::IndependentTest;
                    let safety_and_technical_gates_passed = safety_ok
                        && local_ok
                        && battery_site_ok
                        && option.technical_gate.status == GateStatus::Passed
                        && matches!(
                            option.technical_gate.evidence_class,
                            EvidenceClass::Measured | EvidenceClass::IndependentTest
                        );

                    results.push(OptionResult {
                        site_id: site.site_id.clone(),
                        country_or_region: site.country_or_region.clone(),
                        local_currency: site.local_currency.clone(),
                        family_id: family.family_id.clone(),
                        family_label: family.label.clone(),
                        option_id: option.option_id.clone(),
                        option_label: option.label.clone(),
                        automation_mode: option.mode,
                        target_good_units_per_year: *demand,
                        planned_attempts_per_year: c.attempts,
                        productive_minutes_per_cell_year: c.productive_minutes_per_cell_year,
                        required_cells: c.cells,
                        planned_attempt_capacity_per_year: c.capacity_attempts,
                        good_capacity_per_year: c.good_capacity,
                        capacity_headroom_fraction: c.headroom,
                        planned_capacity_utilization_fraction: c.capacity_utilization,
                        operators_per_shift: c.operators_per_shift,
                        annual_paid_operator_hours: c.annual_paid_operator_hours,
                        installed_equipment_capex_usd: c.equipment_capex,
                        integration_capex_usd: option.integration_capex_usd,
                        total_installed_capex_usd: c.total_capex,
                        station_footprint_sqm: c.footprint,
                        annual_labor_cost_usd: c.labor,
                        annual_electricity_cost_usd: c.electric,
                        annual_useful_thermal_cost_usd: c.thermal,
                        annual_lease_cost_usd: c.lease,
                        annual_maintenance_cost_usd: c.maintenance,
                        expected_failed_attempts_per_year: c.failed_count,
                        annual_failed_attempt_cost_usd: c.failed_cost,
                        expected_rework_attempts_per_year: c.rework_count,
                        annual_rework_cost_usd: c.rework_cost,
                        annual_operating_cost_usd_excluding_capex: c.annual_opex,
                        annual_capex_amortization_usd: c.amortization,
                        annualized_total_station_cost_usd: c.annual_total,
                        total_station_cost_usd_per_good_unit: c.unit_cost,
                        is_manual_baseline: option.option_id == "manual",
                        manual_baseline_annual_operating_cost_usd: base.annual_opex,
                        manual_baseline_installed_capex_usd: base.total_capex,
                        annual_operating_savings_vs_manual_usd: operating_savings,
                        incremental_installed_capex_vs_manual_usd: capex_delta,
                        simple_payback_years: payback,
                        discounted_npv_vs_manual_usd: npv,
                        annualized_cost_delta_vs_manual_usd_per_good_unit: base.unit_cost
                            - c.unit_cost,
                        evidence_release_grade: evidence_release
                            && site.site_cost_evidence_class == EvidenceClass::AuditedCost,
                        local_compliance_gate: site.local_compliance_gate.status,
                        battery_hv_safety_gate: site.battery_hv_safety_gate.status,
                        safety_and_technical_gates_passed,
                        release_ready: gaps.is_empty() && c.good_capacity >= *demand,
                        evidence_gaps: gaps,
                        claim_ceiling: CLAIM_CEILING.to_string(),
                    });
                }
            }
        }
    }
    results.sort_by(|a, b| {
        (
            &a.site_id,
            &a.family_id,
            a.target_good_units_per_year,
            &a.option_id,
        )
            .cmp(&(
                &b.site_id,
                &b.family_id,
                b.target_good_units_per_year,
                &b.option_id,
            ))
    });
    Report {
        schema_version: catalog.schema_version.clone(),
        benchmark_id: catalog.benchmark_id.clone(),
        base_currency: "USD".to_string(),
        evaluation_horizon_years: catalog.evaluation_horizon_years,
        asset_life_years: catalog.asset_life_years,
        discount_rate_fraction: catalog.discount_rate_fraction,
        result_count: results.len(),
        results,
        claim_ceiling: catalog.claim_ceiling.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate() -> Gate {
        Gate {
            status: GateStatus::Unknown,
            evidence_class: EvidenceClass::Unknown,
            source_reference: "unknown://not-verified".to_string(),
            as_of: "2026-10-20".to_string(),
            note: "Test only / not verified".to_string(),
        }
    }

    fn option(
        id: &str,
        mode: AutomationMode,
        cycle: f64,
        capex: f64,
        ops: u32,
        yield_rate: f64,
        kwh: f64,
    ) -> StationOption {
        StationOption {
            option_id: id.to_string(),
            label: id.to_string(),
            mode,
            cycle_minutes_per_attempt: cycle,
            equipment_capex_usd_per_cell: capex,
            integration_capex_usd: if id == "manual" { 0.0 } else { 5000.0 },
            operators_per_cell_per_shift: ops,
            footprint_sqm_per_cell: 20.0,
            electrical_kwh_per_attempt: kwh,
            useful_thermal_kwh_per_attempt: 0.0,
            good_yield_fraction: yield_rate,
            failed_attempt_cost_local: 50.0,
            rework_fraction: if id == "manual" { 0.1 } else { 0.04 },
            rework_cost_per_attempt_local: 20.0,
            annual_maintenance_fraction_of_capex: if id == "manual" { 0.02 } else { 0.04 },
            evidence_class: EvidenceClass::Synthetic,
            technical_gate: gate(),
            safety_gate: gate(),
            note: "Test-only sensitivity".to_string(),
        }
    }

    fn fixture() -> Catalog {
        Catalog {
            schema_version: "0.1.0".to_string(),
            benchmark_id: "TEST-AUTOMATION".to_string(),
            evaluation_horizon_years: 7,
            asset_life_years: 10,
            discount_rate_fraction: 0.1,
            demand_cases_good_units_per_year: vec![100],
            sites: vec![SiteProfile {
                site_id: "test-site".to_string(),
                country_or_region: "test".to_string(),
                local_currency: "TST".to_string(),
                local_currency_per_usd: 10.0,
                fx_reference_date: "2026-10-20".to_string(),
                fx_evidence_class: EvidenceClass::PublicMarketData,
                operating_days_per_year: 200,
                shifts_per_day: 1,
                paid_hours_per_shift: 8.0,
                unpaid_break_hours_per_shift: 1.0,
                availability_fraction: 0.9,
                performance_fraction: 0.9,
                fully_loaded_labor_cost_per_hour_local: 100.0,
                electricity_cost_per_kwh_local: 1.0,
                useful_thermal_energy_cost_per_kwh_local: 0.5,
                building_lease_cost_per_sqm_year_local: 10.0,
                site_cost_evidence_class: EvidenceClass::AuditedCost,
                local_compliance_gate: gate(),
                battery_hv_safety_gate: gate(),
                note: "test only".to_string(),
            }],
            families: vec![StationFamily {
                family_id: "kit".to_string(),
                label: "Kitting".to_string(),
                options: vec![
                    option(
                        "manual",
                        AutomationMode::Manual,
                        30.0,
                        10000.0,
                        2,
                        0.95,
                        10.0,
                    ),
                    option(
                        "assisted",
                        AutomationMode::Assisted,
                        25.0,
                        20000.0,
                        1,
                        0.97,
                        14.0,
                    ),
                    option(
                        "automated",
                        AutomationMode::Automated,
                        20.0,
                        30000.0,
                        1,
                        0.99,
                        20.0,
                    ),
                ],
                note: "test only".to_string(),
            }],
            claim_ceiling: CLAIM_CEILING.to_string(),
        }
    }

    #[test]
    fn manual_baseline_has_zero_relative_capex_npv_and_savings() {
        let report = evaluate_catalog(&fixture());
        let baseline = report
            .results
            .iter()
            .find(|r| r.option_id == "manual")
            .unwrap();
        assert!(baseline.annual_operating_savings_vs_manual_usd.abs() < 1e-8);
        assert!(baseline.incremental_installed_capex_vs_manual_usd.abs() < 1e-8);
        assert!(baseline.discounted_npv_vs_manual_usd.abs() < 1e-8);
    }

    #[test]
    fn more_expensive_energy_increases_energy_intensive_option_cost() {
        let mut cheap = fixture();
        let mut expensive = fixture();
        expensive.sites[0].electricity_cost_per_kwh_local *= 3.0;
        let a = evaluate_catalog(&cheap)
            .results
            .into_iter()
            .find(|r| r.option_id == "automated")
            .unwrap();
        let b = evaluate_catalog(&expensive)
            .results
            .into_iter()
            .find(|r| r.option_id == "automated")
            .unwrap();
        assert!(b.annual_electricity_cost_usd > a.annual_electricity_cost_usd);
        cheap.sites[0].fully_loaded_labor_cost_per_hour_local *= 2.0;
    }

    #[test]
    fn higher_wages_increase_labor_heavy_baseline_cost() {
        let mut cheap = fixture();
        let mut expensive = fixture();
        expensive.sites[0].fully_loaded_labor_cost_per_hour_local *= 2.0;
        let a = evaluate_catalog(&cheap)
            .results
            .into_iter()
            .find(|r| r.option_id == "manual")
            .unwrap();
        let b = evaluate_catalog(&expensive)
            .results
            .into_iter()
            .find(|r| r.option_id == "manual")
            .unwrap();
        assert!(b.annual_labor_cost_usd > a.annual_labor_cost_usd);
        cheap.sites[0].electricity_cost_per_kwh_local *= 2.0;
    }

    #[test]
    fn lower_yield_increases_attempts_and_failure_cost() {
        let mut good = fixture();
        let mut poor = fixture();
        good.families[0].options[0].good_yield_fraction = 0.99;
        poor.families[0].options[0].good_yield_fraction = 0.8;
        let a = evaluate_catalog(&good)
            .results
            .into_iter()
            .find(|r| r.option_id == "manual")
            .unwrap();
        let b = evaluate_catalog(&poor)
            .results
            .into_iter()
            .find(|r| r.option_id == "manual")
            .unwrap();
        assert!(b.planned_attempts_per_year > a.planned_attempts_per_year);
        assert!(b.annual_failed_attempt_cost_usd > a.annual_failed_attempt_cost_usd);
    }

    #[test]
    fn each_option_meets_target_capacity_after_yield() {
        let report = evaluate_catalog(&fixture());
        assert!(
            report
                .results
                .iter()
                .all(|r| r.good_capacity_per_year >= r.target_good_units_per_year)
        );
        assert!(report.results.iter().all(|r| r.required_cells > 0));
    }

    #[test]
    fn more_demand_never_reduces_required_parallel_cells() {
        let mut catalog = fixture();
        catalog.demand_cases_good_units_per_year = vec![100, 1000, 10000];
        let results = evaluate_catalog(&catalog);
        for option_id in ["manual", "assisted", "automated"] {
            let rows: Vec<_> = results
                .iter()
                .filter(|r| r.option_id == option_id)
                .collect();
            assert!(rows[0].required_cells <= rows[1].required_cells);
            assert!(rows[1].required_cells <= rows[2].required_cells);
        }
    }

    #[test]
    fn unknown_safety_and_local_code_never_release_option() {
        let report = evaluate_catalog(&fixture());
        assert!(report.results.iter().all(|r| !r.release_ready));
        assert!(
            report
                .results
                .iter()
                .all(|r| !r.safety_and_technical_gates_passed)
        );
    }

    #[test]
    fn deterministic_under_option_demand_and_site_order_permutation() {
        let mut a = fixture();
        let mut b = fixture();
        a.demand_cases_good_units_per_year = vec![1000, 100];
        b.demand_cases_good_units_per_year = vec![100, 1000];
        a.families[0].options.reverse();
        assert_eq!(evaluate_catalog(&a), evaluate_catalog(&b));
    }

    #[test]
    fn reject_missing_manual_baseline_and_bad_yield() {
        let mut missing = fixture();
        missing.families[0].options.remove(0);
        assert!(validate_catalog(&missing).is_err());
        let mut bad_yield = fixture();
        bad_yield.families[0].options[0].good_yield_fraction = 0.0;
        assert!(validate_catalog(&bad_yield).is_err());
    }
}
