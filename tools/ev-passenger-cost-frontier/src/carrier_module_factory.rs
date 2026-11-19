// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const CLAIM_CEILING: &str = "Synthetic module-process capacity screen only; not a validated production plan, supplier quotation, plant layout, process capability study, product release, vehicle homologation, airworthiness or commercial feasibility.";
const MAX_CONFIGS: usize = 32;
const MAX_STATIONS: usize = 32;
const MAX_MODULES_PER_VEHICLE: u32 = 16;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EvidenceClass {
    Synthetic,
    Measured,
    IndependentTest,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Status {
    PassesSyntheticScreen,
    PassesEvidenceBackedScreen,
    Blocked,
    Indeterminate,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactoryReport {
    pub schema_version: String,
    pub benchmark_id: String,
    results: Vec<BaseFactoryScenario>,
}

#[derive(Debug, Clone, Deserialize)]
struct BaseFactoryScenario {
    site_id: String,
    template_id: String,
    target_good_vehicles_per_year: u64,
    shifts_per_day: u32,
    productive_minutes_per_cell_year: f64,
    good_vehicle_yield_fraction: f64,
    production_release_ready: bool,
    evidence_gaps: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleProcessCatalog {
    pub schema_version: String,
    pub benchmark_id: String,
    configurations: Vec<ModuleConfiguration>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModuleConfiguration {
    config_id: String,
    architecture: String,
    evidence_class: EvidenceClass,
    evidence_reference: String,
    modules_per_good_vehicle: u32,
    good_module_yield_fraction: Option<f64>,
    station_processes: Vec<ModuleStation>,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModuleStation {
    process_id: String,
    process_label: String,
    cycle_minutes_per_attempt: Option<f64>,
    equipment_capex_usd_per_cell: Option<f64>,
    footprint_sqm_per_cell: Option<f64>,
    operators_per_cell_per_shift: Option<u32>,
    energy_kwh_per_attempt: Option<f64>,
    thermal_energy_kwh_per_attempt: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct StationResult {
    process_id: String,
    process_label: String,
    cycle_minutes_per_attempt: Option<f64>,
    parallel_cells_required: Option<u64>,
    planned_attempt_capacity_per_year: Option<u64>,
    good_module_capacity_per_year: Option<u64>,
    equipment_capex_usd: Option<f64>,
    operators_per_shift: Option<u64>,
    active_positions_across_shifts: Option<u64>,
    footprint_sqm: Option<f64>,
    energy_kwh_per_attempt: Option<f64>,
    thermal_energy_kwh_per_attempt: Option<f64>,
    missing_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ScenarioResult {
    site_id: String,
    template_id: String,
    config_id: String,
    architecture: String,
    evidence_class: EvidenceClass,
    target_good_vehicles_per_year: u64,
    modules_per_good_vehicle: u32,
    target_good_modules_per_year: Option<u64>,
    planned_module_attempts_per_year: Option<u64>,
    module_bottleneck_attempt_capacity_per_year: Option<u64>,
    module_bottleneck_good_capacity_per_year: Option<u64>,
    equivalent_good_vehicle_capacity_per_year: Option<u64>,
    good_vehicle_capacity_margin_per_year: Option<i64>,
    additional_parallel_cells: Option<u64>,
    additional_operators_per_shift: Option<u64>,
    additional_active_positions_across_shifts: Option<u64>,
    additional_footprint_sqm: Option<f64>,
    additional_equipment_capex_usd: Option<f64>,
    module_energy_kwh_per_good_module: Option<f64>,
    module_thermal_energy_kwh_per_good_module: Option<f64>,
    status: Status,
    base_factory_production_release_ready: bool,
    base_factory_evidence_gaps: Vec<String>,
    module_production_release_ready: bool,
    missing_fields: Vec<String>,
    findings: Vec<String>,
    stations: Vec<StationResult>,
    claim_ceiling: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub schema_version: String,
    pub benchmark_id: String,
    base_factory_benchmark_id: String,
    scenario_count: usize,
    results: Vec<ScenarioResult>,
    claim_ceiling: &'static str,
}

pub fn validate_and_evaluate(
    base: &FactoryReport,
    modules: &ModuleProcessCatalog,
) -> Result<Report, Vec<String>> {
    validate_inputs(base, modules)?;
    let mut base_scenarios = base.results.clone();
    let mut configs = modules.configurations.clone();
    base_scenarios.sort_by(|left, right| {
        (&left.site_id, &left.template_id, left.target_good_vehicles_per_year)
            .cmp(&(&right.site_id, &right.template_id, right.target_good_vehicles_per_year))
    });
    configs.sort_by(|left, right| left.config_id.cmp(&right.config_id));
    let mut results = Vec::new();
    for scenario in &base_scenarios {
        for config in &configs {
            results.push(evaluate_pair(scenario, config));
        }
    }
    Ok(Report {
        schema_version: modules.schema_version.clone(),
        benchmark_id: modules.benchmark_id.clone(),
        base_factory_benchmark_id: base.benchmark_id.clone(),
        scenario_count: results.len(),
        results,
        claim_ceiling: CLAIM_CEILING,
    })
}

fn validate_inputs(
    base: &FactoryReport,
    modules: &ModuleProcessCatalog,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if base.schema_version != "0.1.0" || base.benchmark_id != "EV-FACTORY-DESIGN-001" {
        errors.push("unsupported_base_factory_report".into());
    }
    if modules.schema_version != "0.1.0"
        || modules.benchmark_id != "EV-CARRIER-MODULE-MFG-001"
    {
        errors.push("unsupported_module_process_schema_or_benchmark".into());
    }
    if modules.claim_ceiling.trim().is_empty()
        || modules.configurations.is_empty()
        || modules.configurations.len() > MAX_CONFIGS
    {
        errors.push(format!("configurations: expected 1..={MAX_CONFIGS}"));
    }
    if base.results.is_empty() {
        errors.push("base_factory_report_has_no_scenarios".into());
    }
    let mut base_keys = BTreeSet::new();
    for (index, scenario) in base.results.iter().enumerate() {
        let key = (
            scenario.site_id.as_str(),
            scenario.template_id.as_str(),
            scenario.target_good_vehicles_per_year,
        );
        if !base_keys.insert(key) {
            errors.push(format!("base.results[{index}]: duplicate scenario identity"));
        }
        if scenario.site_id.trim().is_empty()
            || scenario.template_id.trim().is_empty()
            || scenario.target_good_vehicles_per_year == 0
            || scenario.shifts_per_day == 0
            || !scenario.productive_minutes_per_cell_year.is_finite()
            || scenario.productive_minutes_per_cell_year <= 0.0
            || !scenario.good_vehicle_yield_fraction.is_finite()
            || scenario.good_vehicle_yield_fraction <= 0.0
            || scenario.good_vehicle_yield_fraction > 1.0
        {
            errors.push(format!("base.results[{index}]: invalid production basis"));
        }
    }

    let mut config_ids = BTreeSet::new();
    for (index, config) in modules.configurations.iter().enumerate() {
        if config.config_id.trim().is_empty() || !config_ids.insert(config.config_id.as_str()) {
            errors.push(format!("configurations[{index}].config_id: nonempty unique identity required"));
        }
        if config.architecture.trim().is_empty()
            || config.evidence_reference.trim().is_empty()
            || config.notes.is_empty()
            || config.notes.iter().any(|note| note.trim().is_empty())
        {
            errors.push(format!("configurations[{index}]: architecture, reference and notes are required"));
        }
        if config.modules_per_good_vehicle == 0
            || config.modules_per_good_vehicle > MAX_MODULES_PER_VEHICLE
        {
            errors.push(format!(
                "configurations[{index}].modules_per_good_vehicle: expected 1..={MAX_MODULES_PER_VEHICLE}"
            ));
        }
        if let Some(yield_fraction) = config.good_module_yield_fraction {
            if !yield_fraction.is_finite() || yield_fraction <= 0.0 || yield_fraction > 1.0 {
                errors.push(format!("configurations[{index}].good_module_yield_fraction: invalid fraction"));
            }
        }
        if config.station_processes.is_empty() || config.station_processes.len() > MAX_STATIONS {
            errors.push(format!("configurations[{index}].station_processes: expected 1..={MAX_STATIONS}"));
        }
        let mut process_ids = BTreeSet::new();
        for (station_index, station) in config.station_processes.iter().enumerate() {
            if station.process_id.trim().is_empty()
                || station.process_label.trim().is_empty()
                || !process_ids.insert(station.process_id.as_str())
            {
                errors.push(format!("configurations[{index}].station_processes[{station_index}]: process identity/label must be nonempty and unique"));
            }
            for (field, value) in [
                ("cycle_minutes_per_attempt", station.cycle_minutes_per_attempt),
                ("equipment_capex_usd_per_cell", station.equipment_capex_usd_per_cell),
                ("footprint_sqm_per_cell", station.footprint_sqm_per_cell),
                ("energy_kwh_per_attempt", station.energy_kwh_per_attempt),
                ("thermal_energy_kwh_per_attempt", station.thermal_energy_kwh_per_attempt),
            ] {
                if let Some(value) = value {
                    if !value.is_finite() || value < 0.0 || value > 1_000_000.0 {
                        errors.push(format!("configurations[{index}].{field}: must be finite and nonnegative"));
                    }
                }
            }
            if station.cycle_minutes_per_attempt.is_some_and(|value| value <= 0.0) {
                errors.push(format!("configurations[{index}].cycle_minutes_per_attempt: must be positive"));
            }
        }
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn ceil_div_u64(numerator: u64, denominator: u64) -> Option<u64> {
    if denominator == 0 {
        return None;
    }
    numerator.checked_add(denominator - 1).map(|value| value / denominator)
}

fn required_cells(
    cycle_minutes: f64,
    attempts: u64,
    productive_minutes: f64,
) -> Option<u64> {
    let ratio = cycle_minutes * attempts as f64 / productive_minutes;
    if !ratio.is_finite() || ratio > u64::MAX as f64 {
        return None;
    }
    Some(ratio.ceil().max(1.0) as u64)
}

fn evaluate_pair(base: &BaseFactoryScenario, config: &ModuleConfiguration) -> ScenarioResult {
    let target_good_modules = u64::from(config.modules_per_good_vehicle)
        .checked_mul(base.target_good_vehicles_per_year);
    let attempts = match (target_good_modules, config.good_module_yield_fraction) {
        (Some(target), Some(yield_fraction)) => {
            let value = (target as f64 / yield_fraction).ceil();
            if value.is_finite() && value <= u64::MAX as f64 { Some(value as u64) } else { None }
        }
        _ => None,
    };

    let mut missing_fields = Vec::new();
    if target_good_modules.is_none() {
        missing_fields.push("target_good_module_count_overflow".into());
    }
    if config.good_module_yield_fraction.is_none() {
        missing_fields.push("good_module_yield_fraction".into());
    }

    let mut station_results = Vec::new();
    let mut station_capacities = Vec::new();
    let mut total_cells = 0_u64;
    let mut total_operators = 0_u64;
    let mut total_positions = 0_u64;
    let mut total_area = 0.0_f64;
    let mut total_capex = 0.0_f64;
    let mut total_energy_per_attempt = 0.0_f64;
    let mut total_thermal_per_attempt = 0.0_f64;

    let mut stations: Vec<&ModuleStation> = config.station_processes.iter().collect();
    stations.sort_by(|left, right| left.process_id.cmp(&right.process_id));
    for station in stations {
        let mut missing = Vec::new();
        let cells = match (station.cycle_minutes_per_attempt, attempts) {
            (Some(cycle), Some(attempts)) => required_cells(
                cycle,
                attempts,
                base.productive_minutes_per_cell_year,
            ),
            _ => None,
        };
        if station.cycle_minutes_per_attempt.is_none() { missing.push("cycle_minutes_per_attempt".into()); }
        if station.equipment_capex_usd_per_cell.is_none() { missing.push("equipment_capex_usd_per_cell".into()); }
        if station.footprint_sqm_per_cell.is_none() { missing.push("footprint_sqm_per_cell".into()); }
        if station.operators_per_cell_per_shift.is_none() { missing.push("operators_per_cell_per_shift".into()); }
        if station.energy_kwh_per_attempt.is_none() { missing.push("energy_kwh_per_attempt".into()); }
        if station.thermal_energy_kwh_per_attempt.is_none() { missing.push("thermal_energy_kwh_per_attempt".into()); }

        let capacity = match (cells, station.cycle_minutes_per_attempt) {
            (Some(cells), Some(cycle)) => {
                let raw = base.productive_minutes_per_cell_year * cells as f64 / cycle;
                if raw.is_finite() && raw <= u64::MAX as f64 { Some(raw.floor() as u64) } else { None }
            }
            _ => None,
        };
        if let Some(value) = capacity { station_capacities.push(value); }
        if let Some(value) = cells {
            total_cells = total_cells.saturating_add(value);
        }
        let station_operators = match (cells, station.operators_per_cell_per_shift) {
            (Some(cells), Some(per_cell)) => Some(cells.saturating_mul(u64::from(per_cell))),
            _ => None,
        };
        if let Some(value) = station_operators {
            total_operators = total_operators.saturating_add(value);
            total_positions = total_positions.saturating_add(value * u64::from(base.shifts_per_day));
        }
        let area = match (cells, station.footprint_sqm_per_cell) {
            (Some(cells), Some(area)) => Some(area * cells as f64),
            _ => None,
        };
        if let Some(value) = area { total_area += value; }
        let capex = match (cells, station.equipment_capex_usd_per_cell) {
            (Some(cells), Some(capex)) => Some(capex * cells as f64),
            _ => None,
        };
        if let Some(value) = capex { total_capex += value; }
        if let Some(value) = station.energy_kwh_per_attempt { total_energy_per_attempt += value; }
        if let Some(value) = station.thermal_energy_kwh_per_attempt { total_thermal_per_attempt += value; }

        station_results.push(StationResult {
            process_id: station.process_id.clone(),
            process_label: station.process_label.clone(),
            cycle_minutes_per_attempt: station.cycle_minutes_per_attempt,
            parallel_cells_required: cells,
            planned_attempt_capacity_per_year: capacity,
            good_module_capacity_per_year: capacity.zip(config.good_module_yield_fraction)
                .map(|(capacity, yield_fraction)| (capacity as f64 * yield_fraction).floor() as u64),
            equipment_capex_usd: capex,
            operators_per_shift: station_operators,
            active_positions_across_shifts: station_operators,
            footprint_sqm: area,
            energy_kwh_per_attempt: station.energy_kwh_per_attempt,
            thermal_energy_kwh_per_attempt: station.thermal_energy_kwh_per_attempt,
            missing_fields: missing.clone(),
        });
        for field in missing {
            missing_fields.push(format!("{}:{field}", station.process_id));
        }
    }

    let all_station_capacities_known = station_results
        .iter()
        .all(|station| station.planned_attempt_capacity_per_year.is_some());
    let bottleneck_attempt_capacity = all_station_capacities_known
        .then(|| station_capacities.iter().copied().min())
        .flatten();
    let bottleneck_good_capacity = bottleneck_attempt_capacity
        .zip(config.good_module_yield_fraction)
        .map(|(capacity, yield_fraction)| (capacity as f64 * yield_fraction).floor() as u64);
    let carrier_capacity = bottleneck_good_capacity
        .map(|modules| modules / u64::from(config.modules_per_good_vehicle));
    let capacity_margin = carrier_capacity.and_then(|capacity| {
        i64::try_from(capacity).ok()?.checked_sub(i64::try_from(base.target_good_vehicles_per_year).ok()?)
    });
    let capacity_blocker = capacity_margin.is_some_and(|margin| margin < 0);

    let missing_any = !missing_fields.is_empty() || station_results.iter().any(|station| !station.missing_fields.is_empty());
    let mut findings = Vec::new();
    if capacity_blocker {
        findings.push("module_process_capacity_cannot_support_base_vehicle_output".into());
    }
    if !base.production_release_ready {
        findings.push("base_factory_scenario_is_not_production_release_ready".into());
    }
    if config.evidence_class == EvidenceClass::Synthetic {
        findings.push("process_values_are_synthetic_and_only_validate_screen_arithmetic".into());
    }
    if missing_any {
        findings.push("missing_module_process_or_yield_inputs_prevent_capacity_ranking".into());
    }
    findings.sort();

    let status = if capacity_blocker {
        Status::Blocked
    } else if missing_any || config.evidence_class == EvidenceClass::Unknown {
        Status::Indeterminate
    } else if config.evidence_class == EvidenceClass::Synthetic {
        Status::PassesSyntheticScreen
    } else {
        Status::PassesEvidenceBackedScreen
    };

    let target_good_modules = target_good_modules;
    let attempts = attempts;
    let all_cells_known = station_results.iter().all(|station| station.parallel_cells_required.is_some());
    let cells_total = all_cells_known.then_some(total_cells);
    let operators_total = station_results.iter().all(|station| station.operators_per_shift.is_some())
        .then_some(total_operators);
    let positions_total = station_results.iter().all(|station| station.active_positions_across_shifts.is_some())
        .then_some(total_positions);
    let area_total = station_results.iter().all(|station| station.footprint_sqm.is_some()).then_some(total_area);
    let capex_total = station_results.iter().all(|station| station.equipment_capex_usd.is_some()).then_some(total_capex);
    let all_energy_known = station_results
        .iter()
        .all(|station| station.energy_kwh_per_attempt.is_some());
    let all_thermal_known = station_results
        .iter()
        .all(|station| station.thermal_energy_kwh_per_attempt.is_some());
    let energy_per_module = match (attempts, target_good_modules, all_energy_known) {
        (Some(attempts), Some(good_modules), true) if good_modules > 0 => {
            Some(total_energy_per_attempt * attempts as f64 / good_modules as f64)
        }
        _ => None,
    };
    let thermal_per_module = match (attempts, target_good_modules, all_thermal_known) {
        (Some(attempts), Some(good_modules), true) if good_modules > 0 => {
            Some(total_thermal_per_attempt * attempts as f64 / good_modules as f64)
        }
        _ => None,
    };

    ScenarioResult {
        site_id: base.site_id.clone(),
        template_id: base.template_id.clone(),
        config_id: config.config_id.clone(),
        architecture: config.architecture.clone(),
        evidence_class: config.evidence_class,
        target_good_vehicles_per_year: base.target_good_vehicles_per_year,
        modules_per_good_vehicle: config.modules_per_good_vehicle,
        target_good_modules_per_year: target_good_modules,
        planned_module_attempts_per_year: attempts,
        module_bottleneck_attempt_capacity_per_year: bottleneck_attempt_capacity,
        module_bottleneck_good_capacity_per_year: bottleneck_good_capacity,
        equivalent_good_vehicle_capacity_per_year: carrier_capacity,
        good_vehicle_capacity_margin_per_year: capacity_margin,
        additional_parallel_cells: cells_total,
        additional_operators_per_shift: operators_total,
        additional_active_positions_across_shifts: positions_total,
        additional_footprint_sqm: area_total,
        additional_equipment_capex_usd: capex_total,
        module_energy_kwh_per_good_module: energy_per_module,
        module_thermal_energy_kwh_per_good_module: thermal_per_module,
        status,
        base_factory_production_release_ready: base.production_release_ready,
        base_factory_evidence_gaps: base.evidence_gaps.clone(),
        module_production_release_ready: false,
        missing_fields,
        findings,
        stations: station_results,
        claim_ceiling: CLAIM_CEILING,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> BaseFactoryScenario {
        BaseFactoryScenario {
            site_id: "south-africa-example".into(),
            template_id: "contract-assembly-flex".into(),
            target_good_vehicles_per_year: 1_000,
            shifts_per_day: 2,
            productive_minutes_per_cell_year: 150_000.0,
            good_vehicle_yield_fraction: 0.93,
            production_release_ready: false,
            evidence_gaps: vec!["synthetic".into()],
        }
    }

    fn config() -> ModuleConfiguration {
        ModuleConfiguration {
            config_id: "synthetic-pod".into(),
            architecture: "detachable_drone_pod".into(),
            evidence_class: EvidenceClass::Synthetic,
            evidence_reference: "synthetic fixture".into(),
            modules_per_good_vehicle: 1,
            good_module_yield_fraction: Some(0.95),
            station_processes: vec![
                ModuleStation {
                    process_id: "assembly".into(),
                    process_label: "Pod assembly".into(),
                    cycle_minutes_per_attempt: Some(30.0),
                    equipment_capex_usd_per_cell: Some(20_000.0),
                    footprint_sqm_per_cell: Some(24.0),
                    operators_per_cell_per_shift: Some(2),
                    energy_kwh_per_attempt: Some(4.0),
                    thermal_energy_kwh_per_attempt: Some(0.0),
                },
                ModuleStation {
                    process_id: "eol-test".into(),
                    process_label: "Electrical / EOL test".into(),
                    cycle_minutes_per_attempt: Some(20.0),
                    equipment_capex_usd_per_cell: Some(35_000.0),
                    footprint_sqm_per_cell: Some(18.0),
                    operators_per_cell_per_shift: Some(1),
                    energy_kwh_per_attempt: Some(3.0),
                    thermal_energy_kwh_per_attempt: Some(0.0),
                },
            ],
            notes: vec!["synthetic".into()],
        }
    }

    #[test]
    fn synthetic_pod_route_covers_base_vehicle_factory_throughput() {
        let result = evaluate_pair(&base(), &config());
        assert_eq!(result.status, Status::PassesSyntheticScreen);
        assert_eq!(result.target_good_modules_per_year, Some(1_000));
        assert_eq!(result.planned_module_attempts_per_year, Some(1_053));
        assert!(result.equivalent_good_vehicle_capacity_per_year.unwrap() >= 1_000);
        assert_eq!(result.module_production_release_ready, false);
    }

    #[test]
    fn reduced_module_yield_increases_attempts_and_does_not_increase_capacity() {
        let mut high = config();
        let mut low = config();
        high.good_module_yield_fraction = Some(0.99);
        low.good_module_yield_fraction = Some(0.80);
        let high_result = evaluate_pair(&base(), &high);
        let low_result = evaluate_pair(&base(), &low);
        assert!(low_result.planned_module_attempts_per_year.unwrap()
            > high_result.planned_module_attempts_per_year.unwrap());
    }

    #[test]
    fn unknown_yield_or_cycle_time_is_indeterminate_not_zero_filled() {
        let mut test = config();
        test.good_module_yield_fraction = None;
        test.station_processes[0].cycle_minutes_per_attempt = None;
        let result = evaluate_pair(&base(), &test);
        assert_eq!(result.status, Status::Indeterminate);
        assert!(result.missing_fields.iter().any(|field| field == "good_module_yield_fraction"));
        assert!(result.equivalent_good_vehicle_capacity_per_year.is_none());
    }

    #[test]
    fn known_module_capacity_shortfall_blocks() {
        let mut test = config();
        test.station_processes[0].cycle_minutes_per_attempt = Some(500.0);
        let result = evaluate_pair(&base(), &test);
        assert_eq!(result.status, Status::Blocked);
        assert!(result.good_vehicle_capacity_margin_per_year.unwrap() < 0);
    }

    #[test]
    fn higher_volume_never_reduces_required_parallel_cells() {
        let mut low = base();
        let mut high = base();
        high.target_good_vehicles_per_year = 10_000;
        let low_result = evaluate_pair(&low, &config());
        let high_result = evaluate_pair(&high, &config());
        assert!(high_result.additional_parallel_cells.unwrap()
            >= low_result.additional_parallel_cells.unwrap());
    }

    #[test]
    fn reordered_station_inputs_produce_identical_reports() {
        let first = evaluate_pair(&base(), &config());
        let mut reordered = config();
        reordered.station_processes.reverse();
        let second = evaluate_pair(&base(), &reordered);
        assert_eq!(first, second);
    }

    #[test]
    fn module_process_screen_never_grants_production_release() {
        let factory = base();
        let result = evaluate_pair(&factory, &config());
        assert!(!result.base_factory_production_release_ready);
        assert!(!result.module_production_release_ready);
        assert!(!result.base_factory_evidence_gaps.is_empty());
    }

    #[test]
    fn a_missing_station_cycle_time_prevents_bottleneck_capacity_claim() {
        let mut test = config();
        test.station_processes[0].cycle_minutes_per_attempt = None;
        let result = evaluate_pair(&base(), &test);
        assert_eq!(result.module_bottleneck_attempt_capacity_per_year, None);
        assert_eq!(result.equivalent_good_vehicle_capacity_per_year, None);
    }

    #[test]
    fn missing_station_energy_prevents_partial_energy_total() {
        let mut test = config();
        test.station_processes[0].energy_kwh_per_attempt = None;
        let result = evaluate_pair(&base(), &test);
        assert_eq!(result.module_energy_kwh_per_good_module, None);
    }

    #[test]
    fn unknown_process_fields_are_reported_per_station() {
        let mut test = config();
        test.station_processes[0].equipment_capex_usd_per_cell = None;
        let result = evaluate_pair(&base(), &test);
        assert_eq!(result.status, Status::Indeterminate);
        assert!(result.stations.iter().any(|station| station.missing_fields.contains(&"equipment_capex_usd_per_cell".into())));
        assert!(result.additional_equipment_capex_usd.is_none());
    }

    #[test]
    fn rejects_unknown_json_fields() {
        let mut value = serde_json::to_value(config()).expect("serialize");
        value.as_object_mut().unwrap().insert("pretend_release".into(), serde_json::json!(true));
        assert!(serde_json::from_value::<ModuleConfiguration>(value).is_err());
    }
}
