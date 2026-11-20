// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Product-specific carrier module lifecycle accounting.
//! This consumes the existing factory capacity report; it does not own asset
//! identity, work-order execution, return-to-service authority or battery passports.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

const LIFECYCLE_SCHEMA_VERSION: &str = "0.1.0";
const FACTORY_BENCHMARK_ID: &str = "EV-CARRIER-MODULE-MFG-001";
const MAX_PROFILES: usize = 16;
const MAX_EVENTS_PER_PROFILE: usize = 32;
const MAX_TASKS_PER_EVENT: usize = 16;
const MAX_EXPOSURE_MODULE_YEARS: f64 = 1_000_000.0;
const MAX_ANNUAL_OCCURRENCES: f64 = 100_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    Synthetic,
    Measured,
    IndependentTest,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventClass {
    Planned,
    Unscheduled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServicePhase {
    SafeIsolation,
    AccessAndDisassembly,
    DiagnoseAndInspect,
    Remove,
    RepairOrReplace,
    ReinstallReconnectConfigure,
    PostMaintenanceTest,
    ReturnToServiceDisposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleStatus {
    SyntheticScreen,
    EvidenceBackedScreen,
    Indeterminate,
    Blocked,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleCatalog {
    pub schema_version: String,
    pub benchmark_id: String,
    pub currency: String,
    pub cost_year: u16,
    pub annual_operating_hours_per_module: Option<f64>,
    pub annual_operating_hours_evidence_class: EvidenceClass,
    pub annual_operating_hours_evidence_reference: String,
    pub labour_rate_per_technician_hour: Option<f64>,
    pub labour_rate_evidence_class: EvidenceClass,
    pub labour_rate_evidence_reference: String,
    pub profiles: Vec<ModuleLifecycleProfile>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleLifecycleProfile {
    pub profile_id: String,
    pub config_id: String,
    pub architecture: String,
    pub acquisition_cost_per_good_module: Option<f64>,
    pub acquisition_cost_evidence_class: EvidenceClass,
    pub acquisition_cost_evidence_reference: String,
    pub amortization_years: Option<f64>,
    pub events: Vec<LifecycleEvent>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleEvent {
    pub event_id: String,
    pub event_class: EventClass,
    /// Planned event frequency per module-year. Must not be used for failures.
    pub annual_occurrences_per_module: Option<f64>,
    /// For unscheduled events, use an observed count divided by explicit module-year exposure.
    pub observed_event_count: Option<u64>,
    pub exposure_module_years: Option<f64>,
    pub frequency_evidence_class: EvidenceClass,
    pub frequency_evidence_reference: String,
    pub tasks: Vec<LifecycleTask>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleTask {
    pub phase: ServicePhase,
    /// Wall-clock time during which this module is unavailable.
    pub elapsed_downtime_minutes_per_event: Option<f64>,
    /// Sum of technician time across all workers; not wall-clock duration.
    pub technician_minutes_per_event: Option<f64>,
    pub crew_size: Option<u32>,
    pub parts_cost_per_event: Option<f64>,
    pub evidence_class: EvidenceClass,
    pub evidence_reference: String,
}

#[derive(Debug, Clone, Deserialize)]
struct FactoryReport {
    schema_version: String,
    benchmark_id: String,
    base_factory_benchmark_id: String,
    results: Vec<FactoryModuleScenario>,
}

#[derive(Debug, Clone, Deserialize)]
struct FactoryModuleScenario {
    site_id: String,
    template_id: String,
    config_id: String,
    architecture: String,
    target_good_vehicles_per_year: u64,
    status: String,
    base_factory_production_release_ready: bool,
    module_production_release_ready: bool,
    missing_fields: Vec<String>,
    equivalent_good_vehicle_capacity_per_year: Option<u64>,
    good_vehicle_capacity_margin_per_year: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LifecycleEventResult {
    pub event_id: String,
    pub event_class: EventClass,
    pub annual_event_rate_per_module: Option<f64>,
    pub observed_event_count: Option<u64>,
    pub exposure_module_years: Option<f64>,
    pub downtime_hours_per_event: Option<f64>,
    pub technician_hours_per_event: Option<f64>,
    pub parts_cost_per_event: Option<f64>,
    pub scheduled_downtime_hours_per_module_year: Option<f64>,
    pub unscheduled_downtime_hours_per_module_year: Option<f64>,
    pub scheduled_technician_hours_per_module_year: Option<f64>,
    pub unscheduled_technician_hours_per_module_year: Option<f64>,
    pub scheduled_parts_cost_per_module_year: Option<f64>,
    pub unscheduled_parts_cost_per_module_year: Option<f64>,
    pub missing_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LifecycleScenarioResult {
    pub site_id: String,
    pub template_id: String,
    pub target_good_vehicles_per_year: u64,
    pub config_id: String,
    pub architecture: String,
    pub lifecycle_profile_id: String,
    pub factory_status: String,
    pub factory_equivalent_good_vehicle_capacity_per_year: Option<u64>,
    pub factory_good_vehicle_capacity_margin_per_year: Option<i64>,
    pub base_factory_production_release_ready: bool,
    pub module_production_release_ready: bool,
    pub scheduled_downtime_hours_per_module_year: Option<f64>,
    pub unscheduled_downtime_hours_per_module_year: Option<f64>,
    pub total_downtime_hours_per_module_year: Option<f64>,
    pub downtime_fraction_of_annual_operating_hours: Option<f64>,
    pub scheduled_technician_hours_per_module_year: Option<f64>,
    pub unscheduled_technician_hours_per_module_year: Option<f64>,
    pub total_technician_hours_per_module_year: Option<f64>,
    pub scheduled_parts_cost_per_module_year: Option<f64>,
    pub unscheduled_parts_cost_per_module_year: Option<f64>,
    pub total_service_parts_cost_per_module_year: Option<f64>,
    pub amortized_acquisition_cost_per_module_year: Option<f64>,
    pub labour_cost_per_module_year: Option<f64>,
    pub full_lifecycle_cost_per_module_year: Option<f64>,
    pub currency: String,
    pub cost_year: u16,
    pub status: LifecycleStatus,
    pub full_lifecycle_ranking_eligible: bool,
    pub return_to_service_authorized: bool,
    pub missing_fields: Vec<String>,
    pub findings: Vec<String>,
    pub events: Vec<LifecycleEventResult>,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LifecycleReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub factory_benchmark_id: String,
    pub scenario_count: usize,
    pub lifecycle_profile_count: usize,
    pub result_count: usize,
    pub full_lifecycle_ranking_available: bool,
    pub results: Vec<LifecycleScenarioResult>,
    pub claim_ceiling: &'static str,
}

const CLAIM_CEILING: &str = "Lifecycle task accounting and evidence-gated annualization only. Does not establish real failure rates, fleet availability, supplier pricing, warranty, production readiness, return-to-service authority, regulatory compliance or whole-life commercial superiority.";

fn valid_nonnegative(value: Option<f64>) -> bool {
    value.is_none_or(|number| number.is_finite() && (0.0..=1_000_000.0).contains(&number))
}

fn annual_event_rate(event: &LifecycleEvent) -> Option<f64> {
    match event.event_class {
        EventClass::Planned => {
            if event.frequency_evidence_class == EvidenceClass::Unknown
                || event.frequency_evidence_reference.trim().is_empty()
            {
                None
            } else {
                event.annual_occurrences_per_module
            }
        }
        EventClass::Unscheduled => {
            if !matches!(
                event.frequency_evidence_class,
                EvidenceClass::Measured | EvidenceClass::IndependentTest
            ) || event.frequency_evidence_reference.trim().is_empty()
            {
                return None;
            }
            let count = event.observed_event_count?;
            let exposure = event.exposure_module_years?;
            if !exposure.is_finite() || exposure <= 0.0 {
                return None;
            }
            Some(count as f64 / exposure)
        }
    }
}

fn validate_catalog(
    factory: &FactoryReport,
    catalog: &LifecycleCatalog,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if factory.schema_version != LIFECYCLE_SCHEMA_VERSION
        || factory.benchmark_id != FACTORY_BENCHMARK_ID
        || factory.base_factory_benchmark_id != "EV-FACTORY-DESIGN-001"
    {
        errors.push("unsupported_module_factory_report".into());
    }
    if catalog.schema_version != LIFECYCLE_SCHEMA_VERSION
        || catalog.benchmark_id != "EV-CARRIER-LIFECYCLE-001"
    {
        errors.push("unsupported_lifecycle_schema_or_benchmark".into());
    }
    if catalog.currency.trim().is_empty()
        || catalog.cost_year < 2020
        || catalog.cost_year > 2100
        || catalog.claim_ceiling.trim().is_empty()
    {
        errors.push("currency, cost_year and claim_ceiling must be declared".into());
    }
    if !valid_nonnegative(catalog.annual_operating_hours_per_module)
        || catalog
            .annual_operating_hours_per_module
            .is_some_and(|hours| hours > 8_760.0)
    {
        errors.push("annual_operating_hours_per_module: outside valid annual range".into());
    }
    if !valid_nonnegative(catalog.labour_rate_per_technician_hour) {
        errors.push("labour_rate_per_technician_hour: must be finite and nonnegative".into());
    }
    if catalog.annual_operating_hours_evidence_reference.trim().is_empty()
        || catalog.labour_rate_evidence_reference.trim().is_empty()
    {
        errors.push("annual operating-hours and labour-rate evidence references are required, including explicit unknown references".into());
    }
    if catalog.profiles.is_empty() || catalog.profiles.len() > MAX_PROFILES {
        errors.push(format!("profiles: expected 1..={MAX_PROFILES}"));
    }
    if factory.results.is_empty() {
        errors.push("factory report contains no module scenarios".into());
    }

    let factory_configs: BTreeSet<&str> = factory
        .results
        .iter()
        .map(|scenario| scenario.config_id.as_str())
        .collect();
    let mut profile_ids = BTreeSet::new();
    let mut profile_configs = BTreeSet::new();

    for (profile_index, profile) in catalog.profiles.iter().enumerate() {
        if profile.profile_id.trim().is_empty()
            || !profile_ids.insert(profile.profile_id.as_str())
            || profile.config_id.trim().is_empty()
            || !profile_configs.insert(profile.config_id.as_str())
        {
            errors.push(format!("profiles[{profile_index}]: profile/config identities must be nonempty and unique"));
        }
        if !factory_configs.contains(profile.config_id.as_str()) {
            errors.push(format!("profiles[{profile_index}]: config_id does not exist in the factory report"));
        }
        if profile.architecture.trim().is_empty()
            || profile.notes.is_empty()
            || profile.notes.iter().any(|note| note.trim().is_empty())
        {
            errors.push(format!("profiles[{profile_index}]: architecture and nonempty notes are required"));
        }
        if !valid_nonnegative(profile.acquisition_cost_per_good_module) {
            errors.push(format!("profiles[{profile_index}].acquisition_cost_per_good_module: invalid amount"));
        }
        if !valid_nonnegative(profile.amortization_years)
            || profile.amortization_years.is_some_and(|years| years <= 0.0)
        {
            errors.push(format!("profiles[{profile_index}].amortization_years: must be positive when supplied"));
        }
        if profile.acquisition_cost_per_good_module.is_some()
            && (profile.acquisition_cost_evidence_class == EvidenceClass::Unknown
                || profile.acquisition_cost_evidence_reference.trim().is_empty()
                || profile.amortization_years.is_none())
        {
            errors.push(format!("profiles[{profile_index}]: acquisition cost requires evidence and amortization horizon"));
        }
        if profile.acquisition_cost_evidence_reference.trim().is_empty() {
            errors.push(format!("profiles[{profile_index}]: acquisition cost needs an explicit source/unknown reference"));
        }
        if profile.events.is_empty() || profile.events.len() > MAX_EVENTS_PER_PROFILE {
            errors.push(format!("profiles[{profile_index}].events: expected 1..={MAX_EVENTS_PER_PROFILE}"));
        }
        if !profile.events.iter().any(|event| event.event_class == EventClass::Planned)
            || !profile.events.iter().any(|event| event.event_class == EventClass::Unscheduled)
        {
            errors.push(format!("profiles[{profile_index}]: include explicit planned and unscheduled event records; unknown is not zero"));
        }

        let mut event_ids = BTreeSet::new();
        for (event_index, event) in profile.events.iter().enumerate() {
            if event.event_id.trim().is_empty() || !event_ids.insert(event.event_id.as_str()) {
                errors.push(format!("profiles[{profile_index}].events[{event_index}]: event identity must be nonempty and unique"));
            }
            if event.frequency_evidence_reference.trim().is_empty() {
                errors.push(format!("profiles[{profile_index}].events[{event_index}]: frequency evidence reference is required"));
            }
            if !valid_nonnegative(event.annual_occurrences_per_module)
                || event.annual_occurrences_per_module.is_some_and(|count| count > MAX_ANNUAL_OCCURRENCES)
            {
                errors.push(format!("profiles[{profile_index}].events[{event_index}]: planned annual count outside v1 limits"));
            }
            if event.exposure_module_years.is_some_and(|exposure| {
                !exposure.is_finite() || exposure <= 0.0 || exposure > MAX_EXPOSURE_MODULE_YEARS
            }) {
                errors.push(format!("profiles[{profile_index}].events[{event_index}]: invalid module-year exposure"));
            }
            match event.event_class {
                EventClass::Planned => {
                    if event.observed_event_count.is_some() || event.exposure_module_years.is_some() {
                        errors.push(format!("profiles[{profile_index}].events[{event_index}]: planned event uses annual_occurrences_per_module, not failure exposure"));
                    }
                    if event.annual_occurrences_per_module.is_some()
                        && event.frequency_evidence_class == EvidenceClass::Unknown
                    {
                        errors.push(format!("profiles[{profile_index}].events[{event_index}]: unknown frequency cannot carry a numeric annual rate"));
                    }
                }
                EventClass::Unscheduled => {
                    if event.annual_occurrences_per_module.is_some() {
                        errors.push(format!("profiles[{profile_index}].events[{event_index}]: unscheduled rates require observed count and exposure"));
                    }
                    if event.observed_event_count.is_some() != event.exposure_module_years.is_some() {
                        errors.push(format!("profiles[{profile_index}].events[{event_index}]: observed count and exposure must both be supplied or both remain unknown"));
                    }
                    if event.observed_event_count.is_some()
                        && !matches!(
                            event.frequency_evidence_class,
                            EvidenceClass::Measured | EvidenceClass::IndependentTest
                        )
                    {
                        errors.push(format!("profiles[{profile_index}].events[{event_index}]: unscheduled rates require measured/independent evidence"));
                    }
                }
            }
            if event.tasks.is_empty() || event.tasks.len() > MAX_TASKS_PER_EVENT {
                errors.push(format!("profiles[{profile_index}].events[{event_index}].tasks: expected 1..={MAX_TASKS_PER_EVENT}"));
            }
            let mut phases = BTreeSet::new();
            for (task_index, task) in event.tasks.iter().enumerate() {
                if !phases.insert(task.phase) {
                    errors.push(format!("profiles[{profile_index}].events[{event_index}]: duplicate task phase"));
                }
                for (field, value) in [
                    ("elapsed_downtime_minutes_per_event", task.elapsed_downtime_minutes_per_event),
                    ("technician_minutes_per_event", task.technician_minutes_per_event),
                    ("parts_cost_per_event", task.parts_cost_per_event),
                ] {
                    if !valid_nonnegative(value) {
                        errors.push(format!("profiles[{profile_index}].events[{event_index}].tasks[{task_index}].{field}: invalid amount"));
                    }
                }
                if task.crew_size.is_some_and(|crew| crew == 0 || crew > 100_000) {
                    errors.push(format!("profiles[{profile_index}].events[{event_index}].tasks[{task_index}].crew_size: invalid crew count"));
                }
                if task.evidence_reference.trim().is_empty() {
                    errors.push(format!("profiles[{profile_index}].events[{event_index}].tasks[{task_index}]: evidence reference required, including explicit unknown"));
                }
            }
        }
    }

    for (index, scenario) in factory.results.iter().enumerate() {
        if !catalog.profiles.iter().any(|profile| {
            profile.config_id == scenario.config_id && profile.architecture == scenario.architecture
        }) {
            errors.push(format!("factory.results[{index}]: missing matching lifecycle profile/configuration"));
        }
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn sum_task_values<'a>(
    tasks: impl Iterator<Item = &'a LifecycleTask>,
    pick: impl Fn(&LifecycleTask) -> Option<f64>,
) -> Option<f64> {
    let mut total = 0.0;
    for task in tasks {
        total += pick(task)?;
    }
    total.is_finite().then_some(total)
}

fn event_result(event: &LifecycleEvent) -> LifecycleEventResult {
    let mut tasks: Vec<&LifecycleTask> = event.tasks.iter().collect();
    tasks.sort_by_key(|task| task.phase);

    let downtime_per_event = sum_task_values(tasks.iter().copied(), |task| {
        task.elapsed_downtime_minutes_per_event
    })
    .map(|minutes| minutes / 60.0);
    let technician_per_event = sum_task_values(tasks.iter().copied(), |task| {
        task.technician_minutes_per_event
    })
    .map(|minutes| minutes / 60.0);
    let parts_per_event = sum_task_values(tasks.iter().copied(), |task| task.parts_cost_per_event);
    let rate = annual_event_rate(event);

    let annualized = |per_event: Option<f64>| {
        rate.zip(per_event).map(|(count, value)| count * value)
    };
    let planned = event.event_class == EventClass::Planned;

    LifecycleEventResult {
        event_id: event.event_id.clone(),
        event_class: event.event_class,
        annual_event_rate_per_module: rate,
        observed_event_count: event.observed_event_count,
        exposure_module_years: event.exposure_module_years,
        downtime_hours_per_event: downtime_per_event,
        technician_hours_per_event: technician_per_event,
        parts_cost_per_event: parts_per_event,
        scheduled_downtime_hours_per_module_year: planned.then(|| annualized(downtime_per_event)).flatten(),
        unscheduled_downtime_hours_per_module_year: (!planned).then(|| annualized(downtime_per_event)).flatten(),
        scheduled_technician_hours_per_module_year: planned.then(|| annualized(technician_per_event)).flatten(),
        unscheduled_technician_hours_per_module_year: (!planned).then(|| annualized(technician_per_event)).flatten(),
        scheduled_parts_cost_per_module_year: planned.then(|| annualized(parts_per_event)).flatten(),
        unscheduled_parts_cost_per_module_year: (!planned).then(|| annualized(parts_per_event)).flatten(),
        missing_fields: {
            let mut fields = Vec::new();
            if rate.is_none() { fields.push("annual_event_rate_per_module".into()); }
            if downtime_per_event.is_none() { fields.push("elapsed_downtime_minutes_per_event".into()); }
            if technician_per_event.is_none() { fields.push("technician_minutes_per_event".into()); }
            if parts_per_event.is_none() { fields.push("parts_cost_per_event".into()); }
            fields
        },
    }
}

fn sum_optional(values: impl Iterator<Item = Option<f64>>) -> Option<f64> {
    let mut total = 0.0;
    for value in values {
        total += value?;
    }
    total.is_finite().then_some(total)
}

fn evaluate_profile(
    factory: &FactoryModuleScenario,
    profile: &ModuleLifecycleProfile,
    catalog: &LifecycleCatalog,
) -> LifecycleScenarioResult {
    let mut events: Vec<&LifecycleEvent> = profile.events.iter().collect();
    events.sort_by(|left, right| left.event_id.cmp(&right.event_id));
    let event_results: Vec<LifecycleEventResult> = events.into_iter().map(event_result).collect();

    let planned_downtime = sum_optional(
        event_results
            .iter()
            .filter(|event| event.event_class == EventClass::Planned)
            .map(|event| event.scheduled_downtime_hours_per_module_year),
    );
    let unscheduled_downtime = sum_optional(
        event_results
            .iter()
            .filter(|event| event.event_class == EventClass::Unscheduled)
            .map(|event| event.unscheduled_downtime_hours_per_module_year),
    );
    let planned_technician = sum_optional(
        event_results
            .iter()
            .filter(|event| event.event_class == EventClass::Planned)
            .map(|event| event.scheduled_technician_hours_per_module_year),
    );
    let unscheduled_technician = sum_optional(
        event_results
            .iter()
            .filter(|event| event.event_class == EventClass::Unscheduled)
            .map(|event| event.unscheduled_technician_hours_per_module_year),
    );
    let planned_parts = sum_optional(
        event_results
            .iter()
            .filter(|event| event.event_class == EventClass::Planned)
            .map(|event| event.scheduled_parts_cost_per_module_year),
    );
    let unscheduled_parts = sum_optional(
        event_results
            .iter()
            .filter(|event| event.event_class == EventClass::Unscheduled)
            .map(|event| event.unscheduled_parts_cost_per_module_year),
    );

    let total_downtime = planned_downtime.zip(unscheduled_downtime).map(|(planned, unplanned)| planned + unplanned);
    let total_technician = planned_technician.zip(unscheduled_technician).map(|(planned, unplanned)| planned + unplanned);
    let total_parts = planned_parts.zip(unscheduled_parts).map(|(planned, unplanned)| planned + unplanned);
    let downtime_fraction = total_downtime
        .zip(catalog.annual_operating_hours_per_module)
        .and_then(|(downtime, operating)| {
            if operating > 0.0 && downtime <= operating {
                Some(downtime / operating)
            } else {
                None
            }
        });
    let labour_cost = total_technician
        .zip(catalog.labour_rate_per_technician_hour)
        .map(|(hours, rate)| hours * rate);
    let amortized_acquisition = profile
        .acquisition_cost_per_good_module
        .zip(profile.amortization_years)
        .map(|(cost, years)| cost / years);
    let lifecycle_cost = total_parts
        .zip(labour_cost)
        .zip(amortized_acquisition)
        .map(|((parts, labour), acquisition)| parts + labour + acquisition);

    let mut missing_fields = Vec::new();
    if total_downtime.is_none() { missing_fields.push("complete_annual_downtime_requires_known_planned_and_unscheduled_event_rates_and_task_times".into()); }
    if total_technician.is_none() { missing_fields.push("complete_annual_technician_hours".into()); }
    if total_parts.is_none() { missing_fields.push("complete_annual_parts_cost".into()); }
    if catalog.annual_operating_hours_per_module.is_none() { missing_fields.push("annual_operating_hours_per_module".into()); }
    if catalog.labour_rate_per_technician_hour.is_none() { missing_fields.push("labour_rate_per_technician_hour".into()); }
    if profile.acquisition_cost_per_good_module.is_none() { missing_fields.push("acquisition_cost_per_good_module".into()); }
    if profile.amortization_years.is_none() { missing_fields.push("amortization_years".into()); }
    if profile.events.iter().any(|event| event.event_class == EventClass::Unscheduled && annual_event_rate(event).is_none()) {
        missing_fields.push("unscheduled_failure_rate_requires_measured_count_and_module_year_exposure".into());
    }
    missing_fields.extend(factory.missing_fields.iter().map(|field| format!("factory:{field}")));
    missing_fields.sort();
    missing_fields.dedup();

    let blocked = factory.status == "blocked";
    let all_evidence_backed = matches!(
        catalog.annual_operating_hours_evidence_class,
        EvidenceClass::Measured | EvidenceClass::IndependentTest
    ) && matches!(
        catalog.labour_rate_evidence_class,
        EvidenceClass::Measured | EvidenceClass::IndependentTest
    ) && matches!(
        profile.acquisition_cost_evidence_class,
        EvidenceClass::Measured | EvidenceClass::IndependentTest
    ) && profile.events.iter().all(|event| {
        matches!(
            event.frequency_evidence_class,
            EvidenceClass::Measured | EvidenceClass::IndependentTest
        ) && event.tasks.iter().all(|task| {
            matches!(
                task.evidence_class,
                EvidenceClass::Measured | EvidenceClass::IndependentTest
            )
        })
    });
    let ranking_eligible = !blocked
        && missing_fields.is_empty()
        && lifecycle_cost.is_some()
        && downtime_fraction.is_some()
        && all_evidence_backed;
    let synthetic = !all_evidence_backed
        && catalog.annual_operating_hours_evidence_class != EvidenceClass::Unknown
        && catalog.labour_rate_evidence_class != EvidenceClass::Unknown
        && profile.acquisition_cost_evidence_class != EvidenceClass::Unknown
        && profile.events.iter().all(|event| {
            event.frequency_evidence_class != EvidenceClass::Unknown
                && event.tasks.iter().all(|task| task.evidence_class != EvidenceClass::Unknown)
        });
    let status = if blocked {
        LifecycleStatus::Blocked
    } else if !missing_fields.is_empty() {
        LifecycleStatus::Indeterminate
    } else if synthetic || !all_evidence_backed {
        LifecycleStatus::SyntheticScreen
    } else {
        LifecycleStatus::EvidenceBackedScreen
    };
    let mut findings = Vec::new();
    if factory.status != "passes_synthetic_screen" && factory.status != "passes_evidence_backed_screen" {
        findings.push("factory_module_capacity_not_qualified_for_a_pass".into());
    }
    if !factory.base_factory_production_release_ready {
        findings.push("base_factory_not_production_release_ready".into());
    }
    if !factory.module_production_release_ready {
        findings.push("module_production_release_is_not_granted_by_lifecycle_accounting".into());
    }
    if !ranking_eligible {
        findings.push("full_lifecycle_cost_and_availability_ranking_withheld".into());
    }
    findings.sort();

    LifecycleScenarioResult {
        site_id: factory.site_id.clone(),
        template_id: factory.template_id.clone(),
        target_good_vehicles_per_year: factory.target_good_vehicles_per_year,
        config_id: factory.config_id.clone(),
        architecture: factory.architecture.clone(),
        lifecycle_profile_id: profile.profile_id.clone(),
        factory_status: factory.status.clone(),
        factory_equivalent_good_vehicle_capacity_per_year: factory.equivalent_good_vehicle_capacity_per_year,
        factory_good_vehicle_capacity_margin_per_year: factory.good_vehicle_capacity_margin_per_year,
        base_factory_production_release_ready: factory.base_factory_production_release_ready,
        module_production_release_ready: false,
        scheduled_downtime_hours_per_module_year: planned_downtime,
        unscheduled_downtime_hours_per_module_year: unscheduled_downtime,
        total_downtime_hours_per_module_year: total_downtime,
        downtime_fraction_of_annual_operating_hours: downtime_fraction,
        scheduled_technician_hours_per_module_year: planned_technician,
        unscheduled_technician_hours_per_module_year: unscheduled_technician,
        total_technician_hours_per_module_year: total_technician,
        scheduled_parts_cost_per_module_year: planned_parts,
        unscheduled_parts_cost_per_module_year: unscheduled_parts,
        total_service_parts_cost_per_module_year: total_parts,
        amortized_acquisition_cost_per_module_year: amortized_acquisition,
        labour_cost_per_module_year: labour_cost,
        full_lifecycle_cost_per_module_year: lifecycle_cost,
        currency: catalog.currency.clone(),
        cost_year: catalog.cost_year,
        status,
        full_lifecycle_ranking_eligible: ranking_eligible,
        return_to_service_authorized: false,
        missing_fields,
        findings,
        events: event_results,
        claim_ceiling: CLAIM_CEILING,
    }
}

/// Evaluate lifecycle task burden against the exact module configurations produced by
/// the carrier module factory screen. Unknown event rates never become zero.
pub fn validate_and_evaluate(
    factory: &FactoryReport,
    catalog: &LifecycleCatalog,
) -> Result<LifecycleReport, Vec<String>> {
    validate_catalog(factory, catalog)?;
    let mut factory_results = factory.results.clone();
    let mut profiles = catalog.profiles.clone();
    factory_results.sort_by(|left, right| {
        (&left.site_id, &left.template_id, left.target_good_vehicles_per_year, &left.config_id)
            .cmp(&(&right.site_id, &right.template_id, right.target_good_vehicles_per_year, &right.config_id))
    });
    profiles.sort_by(|left, right| left.config_id.cmp(&right.config_id));
    let mut results = Vec::new();
    for scenario in &factory_results {
        let profile = profiles.iter().find(|profile| {
            profile.config_id == scenario.config_id && profile.architecture == scenario.architecture
        }).ok_or_else(|| vec![format!("missing lifecycle profile for {}", scenario.config_id)])?;
        results.push(evaluate_profile(scenario, profile, catalog));
    }
    let full_lifecycle_ranking_available = results.iter().all(|result| result.full_lifecycle_ranking_eligible);
    Ok(LifecycleReport {
        schema_version: LIFECYCLE_SCHEMA_VERSION.into(),
        benchmark_id: "EV-CARRIER-LIFECYCLE-001".into(),
        factory_benchmark_id: factory.benchmark_id.clone(),
        scenario_count: factory.results.len(),
        lifecycle_profile_count: catalog.profiles.len(),
        result_count: results.len(),
        full_lifecycle_ranking_available,
        results,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(phase: ServicePhase) -> LifecycleTask {
        LifecycleTask {
            phase,
            elapsed_downtime_minutes_per_event: Some(30.0),
            technician_minutes_per_event: Some(45.0),
            crew_size: Some(1),
            parts_cost_per_event: Some(10.0),
            evidence_class: EvidenceClass::Synthetic,
            evidence_reference: "synthetic://lifecycle/task".into(),
        }
    }

    fn profile(config_id: &str, architecture: &str) -> ModuleLifecycleProfile {
        ModuleLifecycleProfile {
            profile_id: format!("profile-{config_id}"),
            config_id: config_id.into(),
            architecture: architecture.into(),
            acquisition_cost_per_good_module: None,
            acquisition_cost_evidence_class: EvidenceClass::Unknown,
            acquisition_cost_evidence_reference: "unknown://acquisition-cost".into(),
            amortization_years: None,
            events: vec![
                LifecycleEvent {
                    event_id: "annual-inspection".into(),
                    event_class: EventClass::Planned,
                    annual_occurrences_per_module: Some(2.0),
                    observed_event_count: None,
                    exposure_module_years: None,
                    frequency_evidence_class: EvidenceClass::Synthetic,
                    frequency_evidence_reference: "synthetic://lifecycle/inspection-frequency".into(),
                    tasks: vec![task(ServicePhase::SafeIsolation), task(ServicePhase::PostMaintenanceTest)],
                },
                LifecycleEvent {
                    event_id: "unplanned-failure".into(),
                    event_class: EventClass::Unscheduled,
                    annual_occurrences_per_module: None,
                    observed_event_count: None,
                    exposure_module_years: None,
                    frequency_evidence_class: EvidenceClass::Unknown,
                    frequency_evidence_reference: "unknown://failure-rate-not-measured".into(),
                    tasks: vec![
                        task(ServicePhase::SafeIsolation),
                        task(ServicePhase::DiagnoseAndInspect),
                        task(ServicePhase::Remove),
                        task(ServicePhase::RepairOrReplace),
                        task(ServicePhase::ReinstallReconnectConfigure),
                        task(ServicePhase::PostMaintenanceTest),
                        task(ServicePhase::ReturnToServiceDisposition),
                    ],
                },
            ],
            notes: vec!["synthetic task observations; no real failure rate".into()],
        }
    }

    fn factory() -> FactoryReport {
        FactoryReport {
            schema_version: "0.1.0".into(),
            benchmark_id: FACTORY_BENCHMARK_ID.into(),
            base_factory_benchmark_id: "EV-FACTORY-DESIGN-001".into(),
            results: vec![FactoryModuleScenario {
                site_id: "site-a".into(),
                template_id: "contract-assembly".into(),
                config_id: "pod".into(),
                architecture: "detachable_drone_pod".into(),
                target_good_vehicles_per_year: 1_000,
                status: "passes_synthetic_screen".into(),
                base_factory_production_release_ready: false,
                module_production_release_ready: false,
                missing_fields: Vec::new(),
                equivalent_good_vehicle_capacity_per_year: Some(1_000),
                good_vehicle_capacity_margin_per_year: Some(0),
            }],
        }
    }

    fn catalog() -> LifecycleCatalog {
        LifecycleCatalog {
            schema_version: "0.1.0".into(),
            benchmark_id: "EV-CARRIER-LIFECYCLE-001".into(),
            currency: "USD".into(),
            cost_year: 2026,
            annual_operating_hours_per_module: Some(2_000.0),
            annual_operating_hours_evidence_class: EvidenceClass::Synthetic,
            annual_operating_hours_evidence_reference: "synthetic://operating-hours".into(),
            labour_rate_per_technician_hour: None,
            labour_rate_evidence_class: EvidenceClass::Unknown,
            labour_rate_evidence_reference: "unknown://labour-rate".into(),
            profiles: vec![profile("pod", "detachable_drone_pod")],
            claim_ceiling: "synthetic lifecycle task screen".into(),
        }
    }

    #[test]
    fn unknown_failure_rate_keeps_planned_burden_but_withholds_total() {
        let report = validate_and_evaluate(&factory(), &catalog()).expect("valid inputs");
        let result = &report.results[0];
        assert_eq!(result.scheduled_downtime_hours_per_module_year, Some(2.0));
        assert_eq!(result.scheduled_technician_hours_per_module_year, Some(3.0));
        assert_eq!(result.unscheduled_downtime_hours_per_module_year, None);
        assert_eq!(result.total_downtime_hours_per_module_year, None);
        assert_eq!(result.downtime_fraction_of_annual_operating_hours, None);
        assert_eq!(result.full_lifecycle_cost_per_module_year, None);
        assert!(!result.full_lifecycle_ranking_eligible);
        assert!(!report.full_lifecycle_ranking_available);
        assert!(!result.return_to_service_authorized);
    }

    #[test]
    fn full_lifecycle_accounting_can_be_complete_without_granting_release() {
        let mut catalog = catalog();
        catalog.annual_operating_hours_evidence_class = EvidenceClass::Measured;
        catalog.annual_operating_hours_evidence_reference = "measurement://hours/sample".into();
        catalog.labour_rate_per_technician_hour = Some(40.0);
        catalog.labour_rate_evidence_class = EvidenceClass::Measured;
        catalog.labour_rate_evidence_reference = "measurement://labour/rate".into();
        let profile = &mut catalog.profiles[0];
        profile.acquisition_cost_per_good_module = Some(1_000.0);
        profile.acquisition_cost_evidence_class = EvidenceClass::Measured;
        profile.acquisition_cost_evidence_reference = "quote://module/unit".into();
        profile.amortization_years = Some(5.0);
        for event in &mut profile.events {
            event.frequency_evidence_class = EvidenceClass::Measured;
            event.frequency_evidence_reference = "measurement://event-frequency".into();
            for task in &mut event.tasks {
                task.evidence_class = EvidenceClass::Measured;
                task.evidence_reference = "measurement://maintenance-task".into();
            }
        }
        let unscheduled = profile.events.iter_mut()
            .find(|event| event.event_class == EventClass::Unscheduled).unwrap();
        unscheduled.observed_event_count = Some(2);
        unscheduled.exposure_module_years = Some(10.0);

        let report = validate_and_evaluate(&factory(), &catalog).expect("complete evidence input");
        let result = &report.results[0];
        assert_eq!(result.status, LifecycleStatus::EvidenceBackedScreen);
        assert!(result.full_lifecycle_ranking_eligible);
        assert!(!result.module_production_release_ready);
        assert!(!result.return_to_service_authorized);
    }

    #[test]
    fn measured_failure_rate_requires_count_exposure_and_independent_source() {
        let mut catalog = catalog();
        let unscheduled = catalog.profiles[0].events.iter_mut()
            .find(|event| event.event_class == EventClass::Unscheduled).unwrap();
        unscheduled.observed_event_count = Some(3);
        unscheduled.exposure_module_years = Some(10.0);
        unscheduled.frequency_evidence_class = EvidenceClass::Measured;
        unscheduled.frequency_evidence_reference = "measurement://fleet-exposure-batch-01".into();
        let report = validate_and_evaluate(&factory(), &catalog).expect("valid inputs");
        let event = report.results[0].events.iter()
            .find(|event| event.event_id == "unplanned-failure").unwrap();
        assert_eq!(event.annual_event_rate_per_module, Some(0.3));
        assert_eq!(event.unscheduled_downtime_hours_per_module_year, Some(1.05));
    }

    #[test]
    fn synthetic_failure_frequency_cannot_be_promoted_to_measured() {
        let mut catalog = catalog();
        let unscheduled = catalog.profiles[0].events.iter_mut()
            .find(|event| event.event_class == EventClass::Unscheduled).unwrap();
        unscheduled.observed_event_count = Some(1);
        unscheduled.exposure_module_years = Some(1.0);
        unscheduled.frequency_evidence_class = EvidenceClass::Synthetic;
        assert!(validate_and_evaluate(&factory(), &catalog).is_err());
    }

    #[test]
    fn zero_failures_over_exposure_are_reported_as_observation_not_missing() {
        let mut catalog = catalog();
        let unscheduled = catalog.profiles[0].events.iter_mut()
            .find(|event| event.event_class == EventClass::Unscheduled).unwrap();
        unscheduled.observed_event_count = Some(0);
        unscheduled.exposure_module_years = Some(10.0);
        unscheduled.frequency_evidence_class = EvidenceClass::Measured;
        unscheduled.frequency_evidence_reference = "measurement://zero-events-10-module-years".into();
        let report = validate_and_evaluate(&factory(), &catalog).expect("valid inputs");
        let event = report.results[0].events.iter()
            .find(|event| event.event_id == "unplanned-failure").unwrap();
        assert_eq!(event.annual_event_rate_per_module, Some(0.0));
        assert_eq!(event.observed_event_count, Some(0));
        assert_eq!(event.exposure_module_years, Some(10.0));
    }

    #[test]
    fn missing_acquisition_or_labour_rate_withholds_full_lifecycle_cost() {
        let mut catalog = catalog();
        let report = validate_and_evaluate(&factory(), &catalog).expect("partial inputs are valid");
        assert_eq!(report.results[0].full_lifecycle_cost_per_module_year, None);
        catalog.labour_rate_per_technician_hour = Some(45.0);
        catalog.labour_rate_evidence_class = EvidenceClass::Synthetic;
        catalog.labour_rate_evidence_reference = "synthetic://labour-rate".into();
        catalog.profiles[0].acquisition_cost_per_good_module = Some(5_000.0);
        catalog.profiles[0].acquisition_cost_evidence_class = EvidenceClass::Synthetic;
        catalog.profiles[0].amortization_years = Some(5.0);
        assert_eq!(
            validate_and_evaluate(&factory(), &catalog).expect("still missing failure rate")
                .results[0].full_lifecycle_cost_per_module_year,
            None
        );
    }

    #[test]
    fn blocked_factory_capacity_is_propagated_and_cannot_be_ranked() {
        let mut base = factory();
        base.results[0].status = "blocked".into();
        let report = validate_and_evaluate(&base, &catalog()).expect("valid inputs");
        assert_eq!(report.results[0].status, LifecycleStatus::Blocked);
        assert!(!report.results[0].full_lifecycle_ranking_eligible);
    }

    #[test]
    fn task_and_event_input_order_does_not_change_report() {
        let base = factory();
        let mut first_catalog = catalog();
        let first = validate_and_evaluate(&base, &first_catalog).expect("first result");
        first_catalog.profiles[0].events.reverse();
        first_catalog.profiles[0].events[0].tasks.reverse();
        let second = validate_and_evaluate(&base, &first_catalog).expect("permuted result");
        assert_eq!(first, second);
    }

    #[test]
    fn acquisition_cost_requires_horizon_and_provenance() {
        let mut catalog = catalog();
        catalog.profiles[0].acquisition_cost_per_good_module = Some(1_000.0);
        catalog.profiles[0].acquisition_cost_evidence_class = EvidenceClass::Measured;
        catalog.profiles[0].amortization_years = Some(5.0);
        assert!(validate_and_evaluate(&factory(), &catalog).is_err());
    }

    #[test]
    fn missing_factory_config_profile_cannot_silently_disappear() {
        let mut catalog = catalog();
        catalog.profiles[0].config_id = "unknown".into();
        assert!(validate_and_evaluate(&factory(), &catalog).is_err());
    }
}
