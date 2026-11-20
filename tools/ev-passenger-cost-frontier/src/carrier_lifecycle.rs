// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Carrier module lifecycle cost and serviceability adapter.
//! Consumes the module factory screen; does not create a maintenance ledger or
//! grant return-to-service authority.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

const CLAIM_CEILING: &str = "Synthetic/evidence-bounded per-module lifecycle screen only; not fleet availability, a physical failure-rate estimate, approved maintenance data, production release, vehicle homologation, airworthiness or commercial TCO.";
const MAX_PROFILES: usize = 32;
const MAX_EVENTS_PER_YEAR: f64 = 1_000.0;
const MAX_INPUT_VALUE: f64 = 1_000_000_000.0;
const MAX_HORIZON_YEARS: f64 = 50.0;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EvidenceClass {
    Synthetic,
    Measured,
    IndependentTest,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum FactoryStatus {
    PassesSyntheticScreen,
    PassesEvidenceBackedScreen,
    Blocked,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum PostTestStatus {
    PassedFresh,
    Failed,
    Unknown,
    Stale,
    Mismatched,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum PhaseKind {
    SafeIsolation,
    AccessDisassembly,
    DiagnoseInspect,
    RemoveModule,
    RepairModule,
    ReplaceModule,
    ReinstallReconfigure,
    PostMaintenanceTest,
    AuthorizedSignoff,
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
pub struct ModuleFactoryReport {
    schema_version: String,
    benchmark_id: String,
    results: Vec<FactoryScenario>,
}

#[derive(Debug, Clone, Deserialize)]
struct FactoryScenario {
    site_id: String,
    template_id: String,
    config_id: String,
    architecture: String,
    evidence_class: EvidenceClass,
    target_good_vehicles_per_year: u64,
    target_good_modules_per_year: Option<u64>,
    equivalent_good_vehicle_capacity_per_year: Option<u64>,
    good_vehicle_capacity_margin_per_year: Option<i64>,
    additional_equipment_capex_usd: Option<f64>,
    module_energy_kwh_per_good_module: Option<f64>,
    module_thermal_energy_kwh_per_good_module: Option<f64>,
    status: FactoryStatus,
    module_production_release_ready: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleCatalog {
    pub schema_version: String,
    pub benchmark_id: String,
    profiles: Vec<LifecycleProfile>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct LifecycleProfile {
    config_id: String,
    architecture: String,
    evidence_class: EvidenceClass,
    evidence_reference: String,
    currency: String,
    currency_year: u16,
    horizon_years: f64,
    annual_operating_hours_per_module: f64,
    material_cost_usd_per_good_module: Option<f64>,
    direct_labor_minutes_per_good_module: Option<f64>,
    manufacturing_labor_rate_usd_per_hour: Option<f64>,
    electricity_price_usd_per_kwh: Option<f64>,
    thermal_energy_price_usd_per_kwh: Option<f64>,
    equipment_amortization_years: Option<f64>,
    technician_labor_rate_usd_per_hour: Option<f64>,
    downtime_cost_usd_per_hour: Option<f64>,
    scheduled_inspections_per_module_year: Option<f64>,
    scheduled_inspection_evidence_class: EvidenceClass,
    scheduled_inspection_evidence_reference: String,
    scheduled_inspection_path: ServicePath,
    unscheduled_fault_count: Option<f64>,
    fault_exposure_module_years: Option<f64>,
    fault_rate_evidence_class: EvidenceClass,
    fault_rate_evidence_reference: String,
    repair_path: ServicePath,
    replacement_path: ServicePath,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ServicePath {
    path_id: String,
    evidence_class: EvidenceClass,
    evidence_reference: String,
    phases: Vec<ServicePhase>,
    parts_and_consumables_cost_usd: Option<f64>,
    replacement_module_cost_usd: Option<f64>,
    parts_wait_hours: Option<f64>,
    post_test_status: PostTestStatus,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ServicePhase {
    phase: PhaseKind,
    elapsed_hours: Option<f64>,
    technician_hours: Option<f64>,
    evidence_reference: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct PathResult {
    path_id: String,
    eligible_for_return_to_service: bool,
    elapsed_hours_per_event: Option<f64>,
    technician_hours_per_event: Option<f64>,
    direct_cost_usd_per_event: Option<f64>,
    downtime_cost_usd_per_event: Option<f64>,
    total_cost_usd_per_event: Option<f64>,
    missing_fields: Vec<String>,
    blocked_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct LifecycleResult {
    site_id: String,
    template_id: String,
    config_id: String,
    architecture: String,
    evidence_class: EvidenceClass,
    currency: String,
    currency_year: u16,
    horizon_years: f64,
    annual_operating_hours_per_module: f64,
    target_good_vehicles_per_year: u64,
    target_good_modules_per_year: Option<u64>,
    equivalent_good_vehicle_capacity_per_year: Option<u64>,
    good_vehicle_capacity_margin_per_year: Option<i64>,
    module_production_release_ready: bool,
    factory_status: FactoryStatus,
    initial_module_build_cost_usd: Option<f64>,
    allocated_factory_equipment_cost_usd_per_module: Option<f64>,
    scheduled_inspection_events_per_module_year: Option<f64>,
    scheduled_maintenance_cost_usd_per_module_year: Option<f64>,
    scheduled_downtime_hours_per_module_year: Option<f64>,
    scheduled_technician_hours_per_module_year: Option<f64>,
    unscheduled_fault_events_per_module_year: Option<f64>,
    selected_fault_path: Option<String>,
    repair_path: PathResult,
    replacement_path: PathResult,
    fault_intervention_cost_usd_per_module_year: Option<f64>,
    fault_downtime_hours_per_module_year: Option<f64>,
    fault_technician_hours_per_module_year: Option<f64>,
    total_maintenance_cost_usd_per_module_year: Option<f64>,
    total_maintenance_downtime_hours_per_module_year: Option<f64>,
    single_module_maintenance_time_fraction: Option<f64>,
    undiscounted_lifecycle_cost_usd_per_module: Option<f64>,
    status: Status,
    return_to_service_authority_granted: bool,
    production_release_granted: bool,
    missing_fields: Vec<String>,
    findings: Vec<String>,
    claim_ceiling: &'static str,
}

#[derive(Debug, Serialize)]
pub struct LifecycleReport {
    pub schema_version: String,
    pub benchmark_id: String,
    base_factory_module_benchmark_id: String,
    pub scenario_count: usize,
    results: Vec<LifecycleResult>,
    claim_ceiling: &'static str,
}

fn valid_nonnegative(value: Option<f64>) -> bool {
    value.is_none_or(|number| number.is_finite() && number >= 0.0)
}

fn sum_path(
    path: &ServicePath,
    expected_phases: &[PhaseKind],
    technician_rate: Option<f64>,
    downtime_rate: Option<f64>,
) -> PathResult {
    let mut missing = Vec::new();
    let mut blocked = Vec::new();
    let mut elapsed = 0.0;
    if path.evidence_reference.trim().is_empty() || path.evidence_class == EvidenceClass::Unknown {
        missing.push("service_path_evidence".into());
    }
    let mut technician_hours = 0.0;
    let mut phases_seen = BTreeSet::new();

    for phase in &path.phases {
        if !phases_seen.insert(phase.phase) {
            blocked.push("duplicate_service_phase".into());
        }
        if phase.evidence_reference.trim().is_empty() {
            missing.push(format!("{:?}:evidence_reference", phase.phase));
        }
        match phase.elapsed_hours {
            Some(value) if value.is_finite() && value >= 0.0 && value <= MAX_INPUT_VALUE => elapsed += value,
            _ => missing.push(format!("{:?}:elapsed_hours", phase.phase)),
        }
        match phase.technician_hours {
            Some(value) if value.is_finite() && value >= 0.0 && value <= MAX_INPUT_VALUE => {
                technician_hours += value;
            }
            _ => missing.push(format!("{:?}:technician_hours", phase.phase)),
        }
    }

    let observed_phases: Vec<PhaseKind> = path.phases.iter().map(|phase| phase.phase).collect();
    if observed_phases != expected_phases {
        blocked.push("service_phase_order_or_identity_mismatch".into());
    }

    let parts_wait = match path.parts_wait_hours {
        Some(value) if value.is_finite() && value >= 0.0 && value <= MAX_INPUT_VALUE => value,
        _ => {
            missing.push("parts_wait_hours".into());
            0.0
        }
    };
    elapsed += parts_wait;

    if path.parts_and_consumables_cost_usd.is_none_or(|value| {
        !value.is_finite() || value < 0.0 || value > MAX_INPUT_VALUE
    }) {
        missing.push("parts_and_consumables_cost_usd".into());
    }
    if path.replacement_module_cost_usd.is_none_or(|value| {
        !value.is_finite() || value < 0.0 || value > MAX_INPUT_VALUE
    }) {
        missing.push("replacement_module_cost_usd".into());
    }
    if !valid_nonnegative(technician_rate) {
        missing.push("technician_labor_rate_usd_per_hour".into());
    }
    if !valid_nonnegative(downtime_rate) {
        missing.push("downtime_cost_usd_per_hour".into());
    }

    let direct_cost = match (
        path.parts_and_consumables_cost_usd,
        path.replacement_module_cost_usd,
        technician_rate,
    ) {
        (Some(parts), Some(module), Some(rate))
            if [parts, module, rate].iter().all(|v| v.is_finite() && *v >= 0.0) =>
        {
            Some(parts + module + technician_hours * rate)
        }
        _ => None,
    };
    let downtime_cost = downtime_rate
        .filter(|rate| rate.is_finite() && *rate >= 0.0)
        .map(|rate| elapsed * rate);
    let total_cost = direct_cost.zip(downtime_cost).map(|(direct, downtime)| direct + downtime);

    match path.post_test_status {
        PostTestStatus::PassedFresh => {}
        PostTestStatus::Failed => blocked.push("post_maintenance_test_failed".into()),
        PostTestStatus::Unknown => missing.push("post_maintenance_test_status".into()),
        PostTestStatus::Stale => missing.push("post_maintenance_test_evidence_stale".into()),
        PostTestStatus::Mismatched => blocked.push("post_maintenance_test_configuration_mismatch".into()),
    }

    let eligible = blocked.is_empty()
        && missing.is_empty()
        && path.post_test_status == PostTestStatus::PassedFresh;
    missing.sort();
    missing.dedup();
    blocked.sort();
    blocked.dedup();

    PathResult {
        path_id: path.path_id.clone(),
        eligible_for_return_to_service: eligible,
        elapsed_hours_per_event: (!missing.iter().any(|field| field.ends_with("elapsed_hours")
            || field == "parts_wait_hours")).then_some(elapsed),
        technician_hours_per_event: (!missing.iter().any(|field| field.ends_with("technician_hours")))
            .then_some(technician_hours),
        direct_cost_usd_per_event: direct_cost,
        downtime_cost_usd_per_event: downtime_cost,
        total_cost_usd_per_event: if eligible { total_cost } else { None },
        missing_fields: missing,
        blocked_reasons: blocked,
    }
}

fn required_phase_sets() -> (Vec<PhaseKind>, Vec<PhaseKind>, Vec<PhaseKind>) {
    (
        vec![
            PhaseKind::SafeIsolation,
            PhaseKind::AccessDisassembly,
            PhaseKind::DiagnoseInspect,
            PhaseKind::RemoveModule,
            PhaseKind::RepairModule,
            PhaseKind::ReinstallReconfigure,
            PhaseKind::PostMaintenanceTest,
            PhaseKind::AuthorizedSignoff,
        ],
        vec![
            PhaseKind::SafeIsolation,
            PhaseKind::AccessDisassembly,
            PhaseKind::DiagnoseInspect,
            PhaseKind::RemoveModule,
            PhaseKind::ReplaceModule,
            PhaseKind::ReinstallReconfigure,
            PhaseKind::PostMaintenanceTest,
            PhaseKind::AuthorizedSignoff,
        ],
        vec![
            PhaseKind::SafeIsolation,
            PhaseKind::AccessDisassembly,
            PhaseKind::DiagnoseInspect,
            PhaseKind::ReinstallReconfigure,
            PhaseKind::PostMaintenanceTest,
            PhaseKind::AuthorizedSignoff,
        ],
    )
}

fn build_cost(
    factory: &FactoryScenario,
    profile: &LifecycleProfile,
) -> (Option<f64>, Option<f64>, Vec<String>) {
    let mut missing = Vec::new();
    let material = profile.material_cost_usd_per_good_module;
    let labor_minutes = profile.direct_labor_minutes_per_good_module;
    let labor_rate = profile.manufacturing_labor_rate_usd_per_hour;
    let electricity_rate = profile.electricity_price_usd_per_kwh;
    let thermal_rate = profile.thermal_energy_price_usd_per_kwh;
    let amortization = profile.equipment_amortization_years;
    let module_energy = factory.module_energy_kwh_per_good_module;
    let thermal_energy = factory.module_thermal_energy_kwh_per_good_module;
    let annual_module_target = factory.target_good_modules_per_year;
    let equipment_capex = factory.additional_equipment_capex_usd;

    for (name, value) in [
        ("material_cost_usd_per_good_module", material),
        ("direct_labor_minutes_per_good_module", labor_minutes),
        ("manufacturing_labor_rate_usd_per_hour", labor_rate),
        ("electricity_price_usd_per_kwh", electricity_rate),
        ("equipment_amortization_years", amortization),
        ("module_energy_kwh_per_good_module", module_energy),
        ("annual_module_target", annual_module_target.map(|v| v as f64)),
        ("additional_equipment_capex_usd", equipment_capex),
    ] {
        if value.is_none() {
            missing.push(name.to_string());
        }
    }
    if thermal_energy.is_some_and(|value| value > 0.0) && thermal_rate.is_none() {
        missing.push("thermal_energy_price_usd_per_kwh".into());
    }

    let allocated_equipment = match (equipment_capex, annual_module_target, amortization) {
        (Some(capex), Some(target), Some(years))
            if target > 0 && years.is_finite() && years > 0.0 =>
        {
            Some(capex / target as f64 / years)
        }
        _ => None,
    };
    let build_cost = match (
        material,
        labor_minutes,
        labor_rate,
        electricity_rate,
        module_energy,
        thermal_energy,
        thermal_rate,
        allocated_equipment,
    ) {
        (
            Some(material),
            Some(minutes),
            Some(labor_rate),
            Some(electricity_rate),
            Some(energy),
            Some(thermal),
            thermal_rate,
            Some(equipment),
        ) if [material, minutes, labor_rate, electricity_rate, energy, thermal, equipment]
            .iter()
            .all(|v| v.is_finite() && *v >= 0.0)
            && (thermal == 0.0 || thermal_rate.is_some()) =>
        {
            let thermal_cost = thermal * thermal_rate.unwrap_or(0.0);
            Some(
                material
                    + minutes / 60.0 * labor_rate
                    + energy * electricity_rate
                    + thermal_cost
                    + equipment,
            )
        }
        _ => None,
    };
    missing.sort();
    missing.dedup();
    (build_cost, allocated_equipment, missing)
}

fn validate_catalog(
    factory: &ModuleFactoryReport,
    catalog: &LifecycleCatalog,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if factory.schema_version != "0.1.0"
        || factory.benchmark_id != "EV-CARRIER-MODULE-MFG-001"
    {
        errors.push("unsupported_module_factory_report".into());
    }
    if catalog.schema_version != "0.1.0"
        || catalog.benchmark_id != "EV-CARRIER-LIFECYCLE-001"
        || catalog.claim_ceiling.trim().is_empty()
        || catalog.profiles.is_empty()
        || catalog.profiles.len() > MAX_PROFILES
    {
        errors.push("unsupported_lifecycle_catalog_or_profile_count".into());
    }

    let mut config_ids = BTreeSet::new();
    for (index, profile) in catalog.profiles.iter().enumerate() {
        if profile.config_id.trim().is_empty()
            || !config_ids.insert(profile.config_id.as_str())
            || profile.architecture.trim().is_empty()
            || profile.evidence_reference.trim().is_empty()
        {
            errors.push(format!("profiles[{index}]: identity and evidence reference must be present and unique"));
        }
        if profile.currency != "USD"
            || profile.currency_year < 2024
            || profile.currency_year > 2035
            || !profile.horizon_years.is_finite()
            || profile.horizon_years <= 0.0
            || profile.horizon_years > MAX_HORIZON_YEARS
            || !profile.annual_operating_hours_per_module.is_finite()
            || profile.annual_operating_hours_per_module <= 0.0
            || profile.annual_operating_hours_per_module > 8_760.0
        {
            errors.push(format!("profiles[{index}]: invalid v0.1 currency, year, horizon or annual exposure"));
        }
        for (name, value) in [
            ("material_cost_usd_per_good_module", profile.material_cost_usd_per_good_module),
            ("direct_labor_minutes_per_good_module", profile.direct_labor_minutes_per_good_module),
            ("manufacturing_labor_rate_usd_per_hour", profile.manufacturing_labor_rate_usd_per_hour),
            ("electricity_price_usd_per_kwh", profile.electricity_price_usd_per_kwh),
            ("thermal_energy_price_usd_per_kwh", profile.thermal_energy_price_usd_per_kwh),
            ("equipment_amortization_years", profile.equipment_amortization_years),
            ("technician_labor_rate_usd_per_hour", profile.technician_labor_rate_usd_per_hour),
            ("downtime_cost_usd_per_hour", profile.downtime_cost_usd_per_hour),
            ("scheduled_inspections_per_module_year", profile.scheduled_inspections_per_module_year),
            ("unscheduled_fault_count", profile.unscheduled_fault_count),
            ("fault_exposure_module_years", profile.fault_exposure_module_years),
        ] {
            if value.is_some_and(|number| !number.is_finite() || number < 0.0 || number > MAX_INPUT_VALUE) {
                errors.push(format!("profiles[{index}].{name}: must be finite, nonnegative and within v0.1 numeric limits"));
            }
        }
        if profile.scheduled_inspections_per_module_year.is_some_and(|value| value > MAX_EVENTS_PER_YEAR) {
            errors.push(format!("profiles[{index}].scheduled_inspections_per_module_year: exceeds v0.1 event-rate limit"));
        }
        if profile.equipment_amortization_years.is_some_and(|value| value <= 0.0) {
            errors.push(format!("profiles[{index}].equipment_amortization_years: must be positive"));
        }
        if profile.fault_exposure_module_years.is_some_and(|value| value <= 0.0 || value < 0.000001) {
            errors.push(format!("profiles[{index}].fault_exposure_module_years: must be positive"));
        }
        if profile.unscheduled_fault_count.is_some() != profile.fault_exposure_module_years.is_some() {
            errors.push(format!("profiles[{index}]: fault count and exposure denominator must both be present or both unknown"));
        }
        if profile.scheduled_inspection_evidence_reference.trim().is_empty()
            || profile.fault_rate_evidence_reference.trim().is_empty()
        {
            errors.push(format!("profiles[{index}]: scheduled/fault evidence references are required even when a metric is unknown"));
        }
        for path in [
            &profile.scheduled_inspection_path,
            &profile.repair_path,
            &profile.replacement_path,
        ] {
            if path.path_id.trim().is_empty() || path.evidence_reference.trim().is_empty() {
                errors.push(format!("profiles[{index}]: service path identity and evidence reference are required"));
            }
            if path.phases.is_empty() || path.phases.len() > 16 {
                errors.push(format!("profiles[{index}].{}: expected 1..=16 phases", path.path_id));
            }
        }
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn evaluate_pair(factory: &FactoryScenario, profile: &LifecycleProfile) -> LifecycleResult {
    let (repair_phases, replacement_phases, inspection_phases) = required_phase_sets();
    let repair = sum_path(
        &profile.repair_path,
        &repair_phases,
        profile.technician_labor_rate_usd_per_hour,
        profile.downtime_cost_usd_per_hour,
    );
    let replacement = sum_path(
        &profile.replacement_path,
        &replacement_phases,
        profile.technician_labor_rate_usd_per_hour,
        profile.downtime_cost_usd_per_hour,
    );
    let inspection = sum_path(
        &profile.scheduled_inspection_path,
        &inspection_phases,
        profile.technician_labor_rate_usd_per_hour,
        profile.downtime_cost_usd_per_hour,
    );
    let (build_cost, allocated_equipment, mut missing) = build_cost(factory, profile);
    let mut findings = Vec::new();

    if factory.status == FactoryStatus::Blocked {
        findings.push("module_factory_capacity_screen_blocked".into());
    }
    if factory.status == FactoryStatus::Indeterminate {
        findings.push("module_factory_capacity_or_process_evidence_indeterminate".into());
    }
    if !factory.module_production_release_ready {
        findings.push("module_factory_does_not_grant_production_release".into());
    }
    if profile.evidence_class == EvidenceClass::Unknown {
        missing.push("profile_evidence_class".into());
    }
    if profile.scheduled_inspection_evidence_reference.trim().is_empty() {
        missing.push("scheduled_inspection_evidence_reference".into());
    }
    if profile.fault_rate_evidence_reference.trim().is_empty() {
        missing.push("fault_rate_evidence_reference".into());
    }
    if profile.scheduled_inspection_evidence_class == EvidenceClass::Unknown {
        missing.push("scheduled_inspection_evidence_class".into());
    }
    if profile.fault_rate_evidence_class == EvidenceClass::Unknown {
        missing.push("fault_rate_evidence_class".into());
    }

    let scheduled_events = profile.scheduled_inspections_per_module_year;
    if scheduled_events.is_none() {
        missing.push("scheduled_inspections_per_module_year".into());
    }
    let scheduled_cost = inspection.total_cost_usd_per_event.zip(scheduled_events)
        .map(|(per_event, count)| per_event * count);
    let scheduled_down = inspection.elapsed_hours_per_event.zip(scheduled_events)
        .map(|(per_event, count)| per_event * count);
    let scheduled_labor = inspection.technician_hours_per_event.zip(scheduled_events)
        .map(|(per_event, count)| per_event * count);

    let fault_rate = match (profile.unscheduled_fault_count, profile.fault_exposure_module_years) {
        (Some(events), Some(exposure)) if exposure > 0.0 => Some(events / exposure),
        _ => {
            missing.push("unscheduled_fault_count_and_exposure".into());
            None
        }
    };

    if fault_rate.is_some_and(|rate| !rate.is_finite() || rate > MAX_EVENTS_PER_YEAR) {
        missing.push("unscheduled_fault_rate_exceeds_v0_1_limit".into());
    }

    let repair_valid = repair.eligible_for_return_to_service;
    let replacement_valid = replacement.eligible_for_return_to_service;
    let chosen_fault_path = match (repair_valid, replacement_valid, repair.total_cost_usd_per_event, replacement.total_cost_usd_per_event) {
        (true, true, Some(repair_cost), Some(replacement_cost)) => {
            Some(if repair_cost <= replacement_cost { "repair" } else { "replacement" }.to_string())
        }
        (true, false, Some(_), _) if replacement.blocked_reasons.len() > 0 => Some("repair".into()),
        (false, true, _, Some(_)) if repair.blocked_reasons.len() > 0 => Some("replacement".into()),
        _ => {
            if !repair.missing_fields.is_empty() || !replacement.missing_fields.is_empty() {
                missing.push("repair_vs_replacement_evidence_incomplete".into());
            } else if !repair_valid && !replacement_valid {
                findings.push("no_fault_path_has_current_post_test_and_signoff_evidence".into());
            }
            None
        }
    };
    let selected_path = match chosen_fault_path.as_deref() {
        Some("repair") => Some(&repair),
        Some("replacement") => Some(&replacement),
        _ => None,
    };
    let fault_cost = selected_path.and_then(|path| path.total_cost_usd_per_event.zip(fault_rate)
        .map(|(cost, rate)| cost * rate));
    let fault_down = selected_path.and_then(|path| path.elapsed_hours_per_event.zip(fault_rate)
        .map(|(hours, rate)| hours * rate));
    let fault_labor = selected_path.and_then(|path| path.technician_hours_per_event.zip(fault_rate)
        .map(|(hours, rate)| hours * rate));

    let annual_maintenance = scheduled_cost.zip(fault_cost).map(|(a, b)| a + b);
    let annual_downtime = scheduled_down.zip(fault_down).map(|(a, b)| a + b);
    let annual_labor = scheduled_labor.zip(fault_labor).map(|(a, b)| a + b);
    let time_fraction = annual_downtime
        .map(|hours| hours / profile.annual_operating_hours_per_module);
    if time_fraction.is_some_and(|fraction| fraction > 1.0 + 1e-9) {
        findings.push("single_module_maintenance_downtime_exceeds_annual_operating_exposure".into());
    }
    let lifecycle_cost = build_cost.zip(annual_maintenance)
        .map(|(initial, annual)| initial + annual * profile.horizon_years);

    for field in repair.missing_fields.iter()
        .chain(replacement.missing_fields.iter())
        .chain(inspection.missing_fields.iter())
    {
        missing.push(field.clone());
    }
    missing.sort();
    missing.dedup();
    findings.sort();
    findings.dedup();

    let factory_blocked = factory.status == FactoryStatus::Blocked;
    let path_blocked = !repair.blocked_reasons.is_empty()
        && !replacement.blocked_reasons.is_empty();
    let scheduled_path_blocked = !inspection.blocked_reasons.is_empty();
    let missing_any = !missing.is_empty()
        || build_cost.is_none()
        || annual_maintenance.is_none()
        || annual_downtime.is_none()
        || annual_labor.is_none()
        || lifecycle_cost.is_none()
        || chosen_fault_path.is_none();

    let status = if factory_blocked || path_blocked || scheduled_path_blocked
        || time_fraction.is_some_and(|fraction| fraction > 1.0 + 1e-9)
    {
        Status::Blocked
    } else if missing_any || factory.status == FactoryStatus::Indeterminate
        || profile.evidence_class == EvidenceClass::Unknown
    {
        Status::Indeterminate
    } else if factory.evidence_class == EvidenceClass::Synthetic
        || profile.evidence_class == EvidenceClass::Synthetic
        || profile.scheduled_inspection_evidence_class == EvidenceClass::Synthetic
        || profile.fault_rate_evidence_class == EvidenceClass::Synthetic
    {
        Status::PassesSyntheticScreen
    } else {
        Status::PassesEvidenceBackedScreen
    };

    LifecycleResult {
        site_id: factory.site_id.clone(),
        template_id: factory.template_id.clone(),
        config_id: factory.config_id.clone(),
        architecture: factory.architecture.clone(),
        evidence_class: profile.evidence_class,
        currency: profile.currency.clone(),
        currency_year: profile.currency_year,
        horizon_years: profile.horizon_years,
        annual_operating_hours_per_module: profile.annual_operating_hours_per_module,
        target_good_vehicles_per_year: factory.target_good_vehicles_per_year,
        target_good_modules_per_year: factory.target_good_modules_per_year,
        equivalent_good_vehicle_capacity_per_year: factory.equivalent_good_vehicle_capacity_per_year,
        good_vehicle_capacity_margin_per_year: factory.good_vehicle_capacity_margin_per_year,
        module_production_release_ready: factory.module_production_release_ready,
        factory_status: factory.status,
        initial_module_build_cost_usd: build_cost,
        allocated_factory_equipment_cost_usd_per_module: allocated_equipment,
        scheduled_inspection_events_per_module_year: scheduled_events,
        scheduled_maintenance_cost_usd_per_module_year: scheduled_cost,
        scheduled_downtime_hours_per_module_year: scheduled_down,
        scheduled_technician_hours_per_module_year: scheduled_labor,
        unscheduled_fault_events_per_module_year: fault_rate,
        selected_fault_path: chosen_fault_path,
        repair_path: repair,
        replacement_path: replacement,
        fault_intervention_cost_usd_per_module_year: fault_cost,
        fault_downtime_hours_per_module_year: fault_down,
        fault_technician_hours_per_module_year: fault_labor,
        total_maintenance_cost_usd_per_module_year: annual_maintenance,
        total_maintenance_downtime_hours_per_module_year: annual_downtime,
        single_module_maintenance_time_fraction: time_fraction,
        undiscounted_lifecycle_cost_usd_per_module: lifecycle_cost,
        status,
        return_to_service_authority_granted: false,
        production_release_granted: false,
        missing_fields: missing,
        findings,
        claim_ceiling: CLAIM_CEILING,
    }
}

/// Compose a carrier module factory report with module-specific service evidence.
/// Unknown event rates or return-to-service evidence never become zero or PASS.
pub fn evaluate_lifecycle(
    factory: &ModuleFactoryReport,
    catalog: &LifecycleCatalog,
) -> Result<LifecycleReport, Vec<String>> {
    validate_catalog(factory, catalog)?;
    let mut profiles = catalog.profiles.clone();
    profiles.sort_by(|left, right| left.config_id.cmp(&right.config_id));
    let mut factories = factory.results.clone();
    factories.sort_by(|left, right| {
        (&left.site_id, &left.template_id, &left.config_id)
            .cmp(&(&right.site_id, &right.template_id, &right.config_id))
    });

    let profile_index: BTreeMap<&str, &LifecycleProfile> = profiles
        .iter()
        .map(|profile| (profile.config_id.as_str(), profile))
        .collect();
    let mut results = Vec::new();
    for row in &factories {
        let profile = profile_index.get(row.config_id.as_str())
            .ok_or_else(|| vec![format!("missing lifecycle profile for {}", row.config_id)])?;
        if row.architecture != profile.architecture {
            return Err(vec![format!("architecture mismatch for {}", row.config_id)]);
        }
        results.push(evaluate_pair(row, profile));
    }

    Ok(LifecycleReport {
        schema_version: catalog.schema_version.clone(),
        benchmark_id: catalog.benchmark_id.clone(),
        base_factory_module_benchmark_id: factory.benchmark_id.clone(),
        scenario_count: results.len(),
        results,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phase(kind: PhaseKind, hours: f64, labor: f64) -> ServicePhase {
        ServicePhase {
            phase: kind,
            elapsed_hours: Some(hours),
            technician_hours: Some(labor),
            evidence_reference: format!("synthetic://phase/{kind:?}"),
        }
    }

    fn path(path_id: &str, replace: bool, elapsed: f64, parts: f64, module: f64) -> ServicePath {
        let action = if replace { PhaseKind::ReplaceModule } else { PhaseKind::RepairModule };
        let phases = vec![
            phase(PhaseKind::SafeIsolation, 0.5, 0.5),
            phase(PhaseKind::AccessDisassembly, 0.5, 0.5),
            phase(PhaseKind::DiagnoseInspect, 1.0, 1.0),
            phase(PhaseKind::RemoveModule, 0.5, 0.5),
            phase(action, elapsed, elapsed),
            phase(PhaseKind::ReinstallReconfigure, 0.5, 0.5),
            phase(PhaseKind::PostMaintenanceTest, 0.5, 0.5),
            phase(PhaseKind::AuthorizedSignoff, 0.25, 0.25),
        ];
        ServicePath {
            path_id: path_id.into(),
            evidence_class: EvidenceClass::Synthetic,
            evidence_reference: format!("synthetic://path/{path_id}"),
            phases,
            parts_and_consumables_cost_usd: Some(parts),
            replacement_module_cost_usd: Some(module),
            parts_wait_hours: Some(0.0),
            post_test_status: PostTestStatus::PassedFresh,
        }
    }

    fn inspection_path() -> ServicePath {
        ServicePath {
            path_id: "scheduled-inspection".into(),
            evidence_class: EvidenceClass::Synthetic,
            evidence_reference: "synthetic://inspection/path".into(),
            phases: vec![
                phase(PhaseKind::SafeIsolation, 0.25, 0.25),
                phase(PhaseKind::AccessDisassembly, 0.5, 0.5),
                phase(PhaseKind::DiagnoseInspect, 0.75, 0.75),
                phase(PhaseKind::ReinstallReconfigure, 0.25, 0.25),
                phase(PhaseKind::PostMaintenanceTest, 0.25, 0.25),
                phase(PhaseKind::AuthorizedSignoff, 0.1, 0.1),
            ],
            parts_and_consumables_cost_usd: Some(10.0),
            replacement_module_cost_usd: Some(0.0),
            parts_wait_hours: Some(0.0),
            post_test_status: PostTestStatus::PassedFresh,
        }
    }

    fn factory_row() -> FactoryScenario {
        FactoryScenario {
            site_id: "site-a".into(),
            template_id: "contract-assembly-flex".into(),
            config_id: "pod".into(),
            architecture: "detachable_drone_pod".into(),
            evidence_class: EvidenceClass::Synthetic,
            target_good_vehicles_per_year: 1_000,
            target_good_modules_per_year: Some(1_000),
            equivalent_good_vehicle_capacity_per_year: Some(1_000),
            good_vehicle_capacity_margin_per_year: Some(0),
            additional_equipment_capex_usd: Some(100_000.0),
            module_energy_kwh_per_good_module: Some(17.0),
            module_thermal_energy_kwh_per_good_module: Some(0.0),
            status: FactoryStatus::PassesSyntheticScreen,
            module_production_release_ready: false,
        }
    }

    fn profile() -> LifecycleProfile {
        LifecycleProfile {
            config_id: "pod".into(),
            architecture: "detachable_drone_pod".into(),
            evidence_class: EvidenceClass::Synthetic,
            evidence_reference: "synthetic://profile/pod".into(),
            currency: "USD".into(),
            currency_year: 2026,
            horizon_years: 8.0,
            annual_operating_hours_per_module: 2_000.0,
            material_cost_usd_per_good_module: Some(8_000.0),
            direct_labor_minutes_per_good_module: Some(180.0),
            manufacturing_labor_rate_usd_per_hour: Some(35.0),
            electricity_price_usd_per_kwh: Some(0.12),
            thermal_energy_price_usd_per_kwh: Some(0.12),
            equipment_amortization_years: Some(10.0),
            technician_labor_rate_usd_per_hour: Some(45.0),
            downtime_cost_usd_per_hour: Some(60.0),
            scheduled_inspections_per_module_year: Some(1.0),
            scheduled_inspection_evidence_class: EvidenceClass::Synthetic,
            scheduled_inspection_evidence_reference: "synthetic://inspection-rate/pod".into(),
            scheduled_inspection_path: inspection_path(),
            unscheduled_fault_count: Some(3.0),
            fault_exposure_module_years: Some(10.0),
            fault_rate_evidence_class: EvidenceClass::Synthetic,
            fault_rate_evidence_reference: "synthetic://fault-exposure/pod-10-module-years".into(),
            repair_path: path("repair", false, 2.0, 250.0, 0.0),
            replacement_path: path("replacement", true, 0.5, 300.0, 18_000.0),
        }
    }

    #[test]
    fn computes_synthetic_module_cost_and_service_burden_without_release() {
        let result = evaluate_pair(&factory_row(), &profile());
        assert_eq!(result.status, Status::PassesSyntheticScreen);
        assert_eq!(result.initial_module_build_cost_usd, Some(8_117.04));
        assert_eq!(result.unscheduled_fault_events_per_module_year, Some(0.3));
        assert!(result.undiscounted_lifecycle_cost_usd_per_module.is_some());
        assert!(!result.production_release_granted);
        assert!(!result.return_to_service_authority_granted);
    }

    #[test]
    fn repair_and_replace_are_compared_with_downtime_value() {
        let result = evaluate_pair(&factory_row(), &profile());
        assert_eq!(result.selected_fault_path.as_deref(), Some("repair"));
    }

    #[test]
    fn unknown_failure_exposure_prevents_lifecycle_total() {
        let mut test = profile();
        test.unscheduled_fault_count = None;
        test.fault_exposure_module_years = None;
        let result = evaluate_pair(&factory_row(), &test);
        assert_eq!(result.status, Status::Indeterminate);
        assert_eq!(result.unscheduled_fault_events_per_module_year, None);
        assert_eq!(result.undiscounted_lifecycle_cost_usd_per_module, None);
    }

    #[test]
    fn stale_or_failed_post_test_cannot_authorize_return_to_service() {
        let mut test = profile();
        test.repair_path.post_test_status = PostTestStatus::Failed;
        test.replacement_path.post_test_status = PostTestStatus::Stale;
        let result = evaluate_pair(&factory_row(), &test);
        assert_eq!(result.status, Status::Blocked);
        assert_eq!(result.selected_fault_path, None);
        assert!(!result.return_to_service_authority_granted);
    }

    #[test]
    fn service_phase_reordering_blocks_the_path() {
        let mut test = profile();
        test.repair_path.phases.swap(0, 1);
        let result = evaluate_pair(&factory_row(), &test);
        assert!(result
            .repair_path
            .blocked_reasons
            .contains(&"service_phase_order_or_identity_mismatch".into()));
        assert_eq!(result.selected_fault_path, None);
    }

    #[test]
    fn unknown_service_path_evidence_remains_indeterminate() {
        let mut test = profile();
        test.repair_path.evidence_class = EvidenceClass::Unknown;
        let result = evaluate_pair(&factory_row(), &test);
        assert!(result
            .repair_path
            .missing_fields
            .contains(&"service_path_evidence".into()));
        assert_eq!(result.status, Status::Indeterminate);
    }

    #[test]
    fn unknown_path_phase_time_does_not_become_zero() {
        let mut test = profile();
        test.repair_path.phases[0].elapsed_hours = None;
        let result = evaluate_pair(&factory_row(), &test);
        assert_eq!(result.repair_path.elapsed_hours_per_event, None);
        assert_eq!(result.undiscounted_lifecycle_cost_usd_per_module, None);
        assert_eq!(result.status, Status::Indeterminate);
    }

    #[test]
    fn failed_factory_capacity_screen_blocks_lifecycle_claim() {
        let mut factory = factory_row();
        factory.status = FactoryStatus::Blocked;
        let result = evaluate_pair(&factory, &profile());
        assert_eq!(result.status, Status::Blocked);
    }

    #[test]
    fn factory_equipment_capex_is_amortized_per_good_module() {
        let result = evaluate_pair(&factory_row(), &profile());
        assert_eq!(result.allocated_factory_equipment_cost_usd_per_module, Some(10.0));
    }

    #[test]
    fn zero_fault_exposure_is_rejected_not_divided_by_zero() {
        let mut test = profile();
        test.fault_exposure_module_years = Some(0.0);
        assert_eq!(test.fault_exposure_module_years.unwrap(), 0.0);
        let input = ModuleFactoryReport {
            schema_version: "0.1.0".into(),
            benchmark_id: "EV-CARRIER-MODULE-MFG-001".into(),
            results: vec![factory_row()],
        };
        let catalog = LifecycleCatalog {
            schema_version: "0.1.0".into(),
            benchmark_id: "EV-CARRIER-LIFECYCLE-001".into(),
            profiles: vec![test],
            claim_ceiling: "synthetic".into(),
        };
        assert!(validate_catalog(&input, &catalog).is_err());
    }

    #[test]
    fn deterministic_profile_and_factory_ordering_is_preserved() {
        let input = ModuleFactoryReport {
            schema_version: "0.1.0".into(),
            benchmark_id: "EV-CARRIER-MODULE-MFG-001".into(),
            results: vec![factory_row()],
        };
        let catalog = LifecycleCatalog {
            schema_version: "0.1.0".into(),
            benchmark_id: "EV-CARRIER-LIFECYCLE-001".into(),
            profiles: vec![profile()],
            claim_ceiling: "synthetic".into(),
        };
        let report = evaluate_lifecycle(&input, &catalog).expect("valid synthetic lifecycle");
        assert_eq!(report.scenario_count, 1);
        assert_eq!(report.results[0].config_id, "pod");
    }

    #[test]
    fn replacement_path_cost_includes_new_module_and_down_time() {
        let mut test = profile();
        test.downtime_cost_usd_per_hour = Some(10_000.0);
        test.repair_path.parts_and_consumables_cost_usd = Some(20_000.0);
        let result = evaluate_pair(&factory_row(), &test);
        assert_eq!(result.selected_fault_path.as_deref(), Some("replacement"));
        assert!(result.replacement_path.total_cost_usd_per_event.unwrap()
            > result.repair_path.direct_cost_usd_per_event.unwrap());
    }

    #[test]
    fn missing_thermal_tariff_with_nonzero_process_heat_is_indeterminate() {
        let mut factory = factory_row();
        factory.module_thermal_energy_kwh_per_good_module = Some(2.0);
        let mut test = profile();
        test.thermal_energy_price_usd_per_kwh = None;
        let (cost, _, missing) = build_cost(&factory, &test);
        assert_eq!(cost, None);
        assert!(missing.contains(&"thermal_energy_price_usd_per_kwh".into()));
    }
}
