// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! EV carrier module serviceability and lifecycle evidence adapter.
//! Consumes the factory module screen and adds only module-specific service observations.
//! This is not a maintenance ledger or return-to-service authority.

use std::collections::{BTreeMap, BTreeSet};
use serde::{Deserialize, Serialize};

const CLAIM_CEILING: &str = "Per-module service-burden arithmetic only. Synthetic task times and rates are not field reliability data. No whole-life cost ranking, fleet availability, product release, maintenance authorization, Battery Passport update, vehicle homologation or airworthiness approval is established.";
const MAX_PROFILES: usize = 32;
const MAX_EVENTS_PER_PROFILE: usize = 32;
const MAX_TASK_MINUTES: f64 = 1_000_000.0;
const MAX_EVENT_RATE_PER_YEAR: f64 = 1_000_000.0;
const MAX_MONEY_PER_EVENT: f64 = 1_000_000_000.0;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass { Synthetic, Measured, IndependentTest, Unknown }

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventClass { Planned, Unplanned }

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventTreatment { Inspection, Repair, Replacement, Remanufacture, Repurpose }

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ServicePhase {
    SafeIsolation,
    AccessDisassembly,
    DiagnoseInspect,
    Remove,
    RepairOrReplace,
    ReinstallConfigure,
    PostMaintenanceTest,
    ReturnToServiceDisposition,
}

const REQUIRED_PHASES: [ServicePhase; 8] = [
    ServicePhase::SafeIsolation,
    ServicePhase::AccessDisassembly,
    ServicePhase::DiagnoseInspect,
    ServicePhase::Remove,
    ServicePhase::RepairOrReplace,
    ServicePhase::ReinstallConfigure,
    ServicePhase::PostMaintenanceTest,
    ServicePhase::ReturnToServiceDisposition,
];

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PhaseOutcome { Performed, NotApplicable, Unknown, Blocked }

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TestResult { Pass, Fail, Unknown }

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReturnDisposition { ReleasedByAuthorizedPersonnel, Held, Unknown }

#[derive(Debug, Deserialize)]
pub struct FactoryModuleReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub base_factory_benchmark_id: String,
    pub results: Vec<FactoryModuleScenario>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FactoryModuleScenario {
    pub site_id: String,
    pub template_id: String,
    pub config_id: String,
    pub architecture: String,
    pub target_good_vehicles_per_year: u64,
    pub equivalent_good_vehicle_capacity_per_year: Option<u64>,
    pub good_vehicle_capacity_margin_per_year: Option<i64>,
    pub status: String,
    pub base_factory_production_release_ready: bool,
    pub base_factory_evidence_gaps: Vec<String>,
    pub module_production_release_ready: bool,
    pub missing_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleCatalog {
    pub schema_version: String,
    pub benchmark_id: String,
    pub profiles: Vec<LifecycleProfile>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleProfile {
    pub config_id: String,
    /// None is reserved for the truck-only zero-incremental-module control.
    pub factory_config_id: Option<String>,
    pub architecture: String,
    pub evidence_class: EvidenceClass,
    pub evidence_reference: String,
    pub currency: String,
    pub cost_basis_year: u16,
    pub labor_cost_per_hour: Option<f64>,
    pub annual_operating_hours_per_module: Option<f64>,
    pub base_vehicle_maintenance_included: bool,
    pub events: Vec<ServiceEvent>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceEvent {
    pub event_id: String,
    pub event_class: EventClass,
    pub treatment: EventTreatment,
    /// Explicit annual frequency. Unknown is not interpreted as zero failures.
    pub occurrences_per_module_year: Option<f64>,
    pub occurrence_evidence_class: EvidenceClass,
    pub occurrence_evidence_reference: String,
    pub occurrence_basis: String,
    /// Required for measured/independent unplanned event rates.
    pub observed_event_count: Option<u64>,
    pub exposure_module_hours: Option<f64>,
    pub parts_cost_per_occurrence: Option<f64>,
    pub tasks: Vec<ServiceTask>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceTask {
    pub sequence_index: u8,
    pub phase: ServicePhase,
    pub outcome: PhaseOutcome,
    pub elapsed_minutes: Option<f64>,
    pub technician_count: Option<u32>,
    pub skill_code: Option<String>,
    pub evidence_class: EvidenceClass,
    pub evidence_reference: String,
    pub measurement_method: Option<String>,
    pub outcome_note: Option<String>,
    pub test_result: Option<TestResult>,
    pub return_disposition: Option<ReturnDisposition>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status { PassesSyntheticScreen, PassesEvidenceBackedScreen, Blocked, Indeterminate }

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EventResult {
    pub event_id: String,
    pub event_class: EventClass,
    pub treatment: EventTreatment,
    pub occurrences_per_module_year: Option<f64>,
    pub event_downtime_minutes: Option<f64>,
    pub technician_hours_per_event: Option<f64>,
    pub labor_cost_per_event: Option<f64>,
    pub parts_cost_per_event: Option<f64>,
    pub direct_cost_per_event: Option<f64>,
    pub annual_downtime_hours_per_module: Option<f64>,
    pub annual_technician_hours_per_module: Option<f64>,
    pub annual_direct_cost_per_module: Option<f64>,
    pub post_maintenance_test: Option<TestResult>,
    pub return_disposition: Option<ReturnDisposition>,
    /// Evidence completeness only. This field does not authorize a release.
    pub return_to_service_evidence_complete: bool,
    pub release_attempt_without_passing_test: bool,
    pub missing_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FactoryContext {
    pub source_factory_config_id: Option<String>,
    pub factory_screen_status: Option<String>,
    pub equivalent_good_vehicle_capacity_per_year: Option<u64>,
    pub good_vehicle_capacity_margin_per_year: Option<i64>,
    pub base_factory_production_release_ready: bool,
    pub factory_evidence_gaps: Vec<String>,
    pub factory_missing_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProfileResult {
    pub site_id: String,
    pub template_id: String,
    pub target_good_vehicles_per_year: u64,
    pub config_id: String,
    pub architecture: String,
    pub evidence_class: EvidenceClass,
    pub currency: String,
    pub cost_basis_year: u16,
    pub annual_operating_hours_per_module: Option<f64>,
    pub base_vehicle_maintenance_included: bool,
    pub factory: FactoryContext,
    pub event_count: usize,
    pub planned_annual_downtime_hours: Option<f64>,
    pub unplanned_annual_downtime_hours: Option<f64>,
    pub total_annual_downtime_hours: Option<f64>,
    pub planned_annual_technician_hours: Option<f64>,
    pub unplanned_annual_technician_hours: Option<f64>,
    pub total_annual_technician_hours: Option<f64>,
    pub planned_annual_direct_cost: Option<f64>,
    pub unplanned_annual_direct_cost: Option<f64>,
    pub total_annual_direct_service_cost: Option<f64>,
    /// Additive single-module downtime fraction; no spare substitution or overlap.
    pub single_module_downtime_fraction: Option<f64>,
    pub event_rates_complete: bool,
    pub task_observations_complete: bool,
    pub all_return_to_service_evidence_complete: bool,
    pub post_maintenance_test_failure_count: u64,
    pub release_attempt_without_passing_test_count: u64,
    pub service_burden_comparison_complete: bool,
    /// Always false in v0.1: acquisition, warranty, EOL and field failure data are incomplete.
    pub whole_life_cost_ranking_eligible: bool,
    pub module_production_release_ready: bool,
    pub status: Status,
    pub missing_fields: Vec<String>,
    pub findings: Vec<String>,
    pub events: Vec<EventResult>,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Serialize)]
pub struct LifecycleReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub base_factory_benchmark_id: String,
    pub scenario_count: usize,
    pub result_count: usize,
    pub results: Vec<ProfileResult>,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct BaseScenarioKey {
    site_id: String,
    template_id: String,
    target_good_vehicles_per_year: u64,
}

fn finite_nonnegative(value: Option<f64>) -> bool {
    value.is_none_or(|number| number.is_finite() && number >= 0.0)
}

fn expected_sequence(task: &ServiceTask, index: usize) -> bool {
    usize::from(task.sequence_index) == index && task.phase == REQUIRED_PHASES[index]
}

fn validate_task(task: &ServiceTask, profile_index: usize, event_index: usize, task_index: usize) -> Vec<String> {
    let mut errors = Vec::new();
    let prefix = format!("profiles[{profile_index}].events[{event_index}].tasks[{task_index}]");
    if task.evidence_reference.trim().is_empty() {
        errors.push(format!("{prefix}.evidence_reference: required"));
    }
    match task.outcome {
        PhaseOutcome::Performed => {
            if !task.elapsed_minutes.is_some_and(|v| v.is_finite() && v > 0.0 && v <= MAX_TASK_MINUTES) {
                errors.push(format!("{prefix}.elapsed_minutes: performed task requires a positive finite duration"));
            }
            if !task.technician_count.is_some_and(|v| v > 0 && v <= 10_000) {
                errors.push(format!("{prefix}.technician_count: performed task requires a positive crew count"));
            }
            if task.skill_code.as_ref().is_none_or(|v| v.trim().is_empty()) {
                errors.push(format!("{prefix}.skill_code: performed task requires a skill code"));
            }
            if task.evidence_class == EvidenceClass::Unknown {
                errors.push(format!("{prefix}.evidence_class: performed task cannot be unknown"));
            }
            if task.measurement_method.as_ref().is_none_or(|v| v.trim().is_empty()) {
                errors.push(format!("{prefix}.measurement_method: performed task requires a method"));
            }
        }
        PhaseOutcome::NotApplicable => {
            if task.outcome_note.as_ref().is_none_or(|v| v.trim().is_empty()) {
                errors.push(format!("{prefix}.outcome_note: not_applicable requires a reason"));
            }
            if task.elapsed_minutes.is_some() || task.technician_count.is_some() || task.skill_code.is_some() {
                errors.push(format!("{prefix}: not_applicable task cannot carry time or labor"));
            }
        }
        PhaseOutcome::Unknown => {
            if task.elapsed_minutes.is_some() || task.technician_count.is_some() || task.skill_code.is_some() {
                errors.push(format!("{prefix}: unknown task cannot be zero-filled with time or labor"));
            }
            if task.evidence_class != EvidenceClass::Unknown {
                errors.push(format!("{prefix}.evidence_class: unknown task must be marked unknown"));
            }
        }
        PhaseOutcome::Blocked => {
            if task.outcome_note.as_ref().is_none_or(|v| v.trim().is_empty()) {
                errors.push(format!("{prefix}.outcome_note: blocked task requires a reason"));
            }
            if task.elapsed_minutes.is_some() || task.technician_count.is_some() || task.skill_code.is_some() {
                errors.push(format!("{prefix}: blocked task cannot carry completed time or labor"));
            }
        }
    }
    if task.phase == ServicePhase::PostMaintenanceTest {
        if task.outcome == PhaseOutcome::Performed && task.test_result.is_none() {
            errors.push(format!("{prefix}.test_result: performed test requires a result"));
        }
    } else if task.test_result.is_some() {
        errors.push(format!("{prefix}.test_result: only valid for post_maintenance_test"));
    }
    if task.phase == ServicePhase::ReturnToServiceDisposition {
        if task.outcome == PhaseOutcome::Performed && task.return_disposition.is_none() {
            errors.push(format!("{prefix}.return_disposition: performed disposition requires a value"));
        }
    } else if task.return_disposition.is_some() {
        errors.push(format!("{prefix}.return_disposition: only valid for return_to_service_disposition"));
    }
    errors
}

fn validate_inputs(factory: &FactoryModuleReport, catalog: &LifecycleCatalog) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if factory.schema_version != "0.1.0"
        || factory.benchmark_id != "EV-CARRIER-MODULE-MFG-001"
        || factory.base_factory_benchmark_id != "EV-FACTORY-DESIGN-001"
    {
        errors.push("unsupported_module_factory_report".into());
    }
    if catalog.schema_version != "0.1.0" || catalog.benchmark_id != "EV-CARRIER-LIFECYCLE-001" {
        errors.push("unsupported_lifecycle_schema_or_benchmark".into());
    }
    if catalog.claim_ceiling.trim().is_empty() || catalog.profiles.is_empty() || catalog.profiles.len() > MAX_PROFILES {
        errors.push(format!("profiles: expected 1..={MAX_PROFILES}"));
    }
    if factory.results.is_empty() {
        errors.push("factory_module_report_has_no_scenarios".into());
    }

    let mut factory_keys = BTreeSet::new();
    for (index, scenario) in factory.results.iter().enumerate() {
        let key = (scenario.site_id.as_str(), scenario.template_id.as_str(),
            scenario.target_good_vehicles_per_year, scenario.config_id.as_str());
        if !factory_keys.insert(key) {
            errors.push(format!("factory.results[{index}]: duplicate scenario identity"));
        }
        if scenario.site_id.trim().is_empty() || scenario.template_id.trim().is_empty()
            || scenario.config_id.trim().is_empty() || scenario.architecture.trim().is_empty()
            || scenario.target_good_vehicles_per_year == 0
        {
            errors.push(format!("factory.results[{index}]: invalid scenario identity"));
        }
    }

    let mut profile_ids = BTreeSet::new();
    let mut mapped_factory_ids = BTreeSet::new();
    let mut baseline_count = 0_usize;
    for (profile_index, profile) in catalog.profiles.iter().enumerate() {
        if profile.config_id.trim().is_empty() || !profile_ids.insert(profile.config_id.as_str()) {
            errors.push(format!("profiles[{profile_index}].config_id: nonempty unique identity required"));
        }
        if profile.architecture.trim().is_empty() || profile.evidence_reference.trim().is_empty()
            || profile.notes.is_empty() || profile.notes.iter().any(|note| note.trim().is_empty())
        {
            errors.push(format!("profiles[{profile_index}]: architecture, evidence reference and notes are required"));
        }
        if profile.currency.len() != 3 || !profile.currency.chars().all(|c| c.is_ascii_uppercase()) {
            errors.push(format!("profiles[{profile_index}].currency: expected three-letter currency code"));
        }
        if !(2000..=2100).contains(&profile.cost_basis_year) {
            errors.push(format!("profiles[{profile_index}].cost_basis_year: outside supported range"));
        }
        if !finite_nonnegative(profile.labor_cost_per_hour) || !finite_nonnegative(profile.annual_operating_hours_per_module) {
            errors.push(format!("profiles[{profile_index}]: labor rate and operating hours must be finite and nonnegative"));
        }
        if profile.annual_operating_hours_per_module.is_some_and(|v| v <= 0.0) {
            errors.push(format!("profiles[{profile_index}].annual_operating_hours_per_module: must be positive when known"));
        }
        if let Some(factory_config_id) = &profile.factory_config_id {
            if factory_config_id.trim().is_empty() || !mapped_factory_ids.insert(factory_config_id.as_str()) {
                errors.push(format!("profiles[{profile_index}].factory_config_id: nonempty unique mapping required"));
            }
        } else {
            baseline_count += 1;
            if profile.architecture != "truck_only_baseline" || !profile.events.is_empty()
                || profile.base_vehicle_maintenance_included
            {
                errors.push(format!("profiles[{profile_index}]: truck-only control must exclude base-vehicle maintenance and have no module events"));
            }
        }
        if profile.factory_config_id.is_some() && profile.base_vehicle_maintenance_included {
            errors.push(format!("profiles[{profile_index}]: module profile cannot silently include base-vehicle maintenance"));
        }
        if profile.factory_config_id.is_some() && profile.events.is_empty() {
            errors.push(format!("profiles[{profile_index}]: module profile requires planned and unplanned service records"));
        }
        if profile.events.len() > MAX_EVENTS_PER_PROFILE {
            errors.push(format!("profiles[{profile_index}].events: exceeds v0.1 event limit"));
        }
        let has_planned = profile.events.iter().any(|event| event.event_class == EventClass::Planned);
        let has_unplanned = profile.events.iter().any(|event| event.event_class == EventClass::Unplanned);
        if profile.factory_config_id.is_some() && (!has_planned || !has_unplanned) {
            errors.push(format!("profiles[{profile_index}]: module profile must represent planned and unplanned service"));
        }

        let mut event_ids = BTreeSet::new();
        for (event_index, event) in profile.events.iter().enumerate() {
            if event.event_id.trim().is_empty() || !event_ids.insert(event.event_id.as_str()) {
                errors.push(format!("profiles[{profile_index}].events[{event_index}].event_id: nonempty unique identity required"));
            }
            if event.occurrence_evidence_reference.trim().is_empty() || event.occurrence_basis.trim().is_empty() {
                errors.push(format!("profiles[{profile_index}].events[{event_index}]: occurrence reference and basis are required"));
            }
            if event.event_class == EventClass::Planned && event.occurrences_per_module_year.is_none() {
                errors.push(format!("profiles[{profile_index}].events[{event_index}]: planned frequency must be explicit"));
            }
            if !finite_nonnegative(event.occurrences_per_module_year)
                || event.occurrences_per_module_year.is_some_and(|v| v > MAX_EVENT_RATE_PER_YEAR)
                || !finite_nonnegative(event.parts_cost_per_occurrence)
                || event.parts_cost_per_occurrence.is_some_and(|v| v > MAX_MONEY_PER_EVENT)
            {
                errors.push(format!("profiles[{profile_index}].events[{event_index}]: invalid event rate or parts cost"));
            }
            if event.event_class == EventClass::Unplanned && event.occurrences_per_module_year.is_some()
                && matches!(event.occurrence_evidence_class, EvidenceClass::Measured | EvidenceClass::IndependentTest)
            {
                let rate = event.occurrences_per_module_year.unwrap_or(0.0);
                match (event.observed_event_count, event.exposure_module_hours, profile.annual_operating_hours_per_module) {
                    (Some(count), Some(hours), Some(annual_hours)) if hours.is_finite() && hours > 0.0 => {
                        let derived = count as f64 / hours * annual_hours;
                        if (derived - rate).abs() > 1e-6 {
                            errors.push(format!("profiles[{profile_index}].events[{event_index}]: event rate does not reconcile with count/exposure"));
                        }
                    }
                    _ => errors.push(format!("profiles[{profile_index}].events[{event_index}]: measured unplanned rate requires event count, module-hour exposure and annual operating hours")),
                }
            }
            if event.event_class == EventClass::Unplanned && event.occurrences_per_module_year.is_some()
                && event.occurrence_evidence_class == EvidenceClass::Unknown
            {
                errors.push(format!("profiles[{profile_index}].events[{event_index}]: known unplanned rate cannot have unknown evidence class"));
            }
            if event.tasks.len() != REQUIRED_PHASES.len() {
                errors.push(format!("profiles[{profile_index}].events[{event_index}].tasks: exactly eight ordered phases are required"));
            }
            for (task_index, task) in event.tasks.iter().enumerate() {
                if task_index < REQUIRED_PHASES.len() && !expected_sequence(task, task_index) {
                    errors.push(format!("profiles[{profile_index}].events[{event_index}].tasks[{task_index}]: phase order/sequence mismatch"));
                }
                errors.extend(validate_task(task, profile_index, event_index, task_index));
            }
        }
    }
    if baseline_count != 1 {
        errors.push("profiles: exactly one zero-incremental-module truck-only baseline is required".into());
    }
    let expected_factory_configs: BTreeSet<&str> = factory.results.iter().map(|row| row.config_id.as_str()).collect();
    for config_id in &mapped_factory_ids {
        if !expected_factory_configs.contains(config_id) {
            errors.push(format!("profiles: factory_config_id {config_id} has no matching factory result"));
        }
    }
    for missing in expected_factory_configs.difference(&mapped_factory_ids) {
        errors.push(format!("profiles: no lifecycle profile maps factory config {missing}"));
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

#[derive(Default)]
struct AnnualAccumulator {
    downtime: f64,
    technician_hours: f64,
    direct_cost: f64,
    complete_downtime: bool,
    complete_labor: bool,
    complete_cost: bool,
    seen: bool,
}

impl AnnualAccumulator {
    fn new() -> Self {
        Self { complete_downtime: true, complete_labor: true, complete_cost: true, ..Self::default() }
    }
    fn add_event(&mut self, result: &EventResult) {
        self.seen = true;
        match result.annual_downtime_hours_per_module { Some(v) => self.downtime += v, None => self.complete_downtime = false }
        match result.annual_technician_hours_per_module { Some(v) => self.technician_hours += v, None => self.complete_labor = false }
        match result.annual_direct_cost_per_module { Some(v) => self.direct_cost += v, None => self.complete_cost = false }
    }
    fn downtime(&self) -> Option<f64> { (self.seen && self.complete_downtime).then_some(self.downtime) }
    fn technician_hours(&self) -> Option<f64> { (self.seen && self.complete_labor).then_some(self.technician_hours) }
    fn direct_cost(&self) -> Option<f64> { (self.seen && self.complete_cost).then_some(self.direct_cost) }
}

fn evaluate_event(profile: &LifecycleProfile, event: &ServiceEvent) -> EventResult {
    let mut missing_fields = Vec::new();
    let mut event_downtime = 0.0;
    let mut technician_hours = 0.0;
    let mut time_complete = true;
    let mut labor_complete = true;
    let mut test_result = None;
    let mut disposition = None;

    for task in &event.tasks {
        match task.outcome {
            PhaseOutcome::Performed => match (task.elapsed_minutes, task.technician_count) {
                (Some(minutes), Some(technicians)) => {
                    event_downtime += minutes;
                    technician_hours += minutes * f64::from(technicians) / 60.0;
                }
                _ => {
                    time_complete = false;
                    labor_complete = false;
                    missing_fields.push(format!("{:?}:duration_or_crew", task.phase));
                }
            },
            PhaseOutcome::NotApplicable => {},
            PhaseOutcome::Unknown => {
                time_complete = false;
                labor_complete = false;
                missing_fields.push(format!("{:?}:unknown_task_observation", task.phase));
            },
            PhaseOutcome::Blocked => {
                time_complete = false;
                labor_complete = false;
                missing_fields.push(format!("{:?}:blocked_task", task.phase));
            }
        }
        if task.phase == ServicePhase::PostMaintenanceTest { test_result = task.test_result; }
        if task.phase == ServicePhase::ReturnToServiceDisposition { disposition = task.return_disposition; }
    }

    let event_downtime_minutes = time_complete.then_some(event_downtime);
    let technician_hours_per_event = labor_complete.then_some(technician_hours);
    let labor_cost_per_event = match (technician_hours_per_event, profile.labor_cost_per_hour) {
        (Some(hours), Some(rate)) => Some(hours * rate),
        _ => { missing_fields.push("labor_cost_per_hour".into()); None }
    };
    let parts_cost_per_event = event.parts_cost_per_occurrence;
    if parts_cost_per_event.is_none() { missing_fields.push("parts_cost_per_occurrence".into()); }
    let direct_cost_per_event = labor_cost_per_event.zip(parts_cost_per_event).map(|(labor, parts)| labor + parts);

    let annual_downtime = event_downtime_minutes.zip(event.occurrences_per_module_year)
        .map(|(minutes, count)| minutes / 60.0 * count);
    let annual_labor = technician_hours_per_event.zip(event.occurrences_per_module_year)
        .map(|(hours, count)| hours * count);
    let annual_cost = direct_cost_per_event.zip(event.occurrences_per_module_year)
        .map(|(cost, count)| cost * count);
    if annual_downtime.is_none() { missing_fields.push("annual_event_downtime_frequency_or_duration".into()); }
    if annual_labor.is_none() { missing_fields.push("annual_event_labor_frequency_or_duration".into()); }
    if annual_cost.is_none() { missing_fields.push("annual_event_cost_frequency_or_cost_inputs".into()); }

    let test_passed = test_result == Some(TestResult::Pass);
    let authorized_disposition_recorded = disposition == Some(ReturnDisposition::ReleasedByAuthorizedPersonnel);
    let return_to_service_evidence_complete = test_passed && authorized_disposition_recorded;
    let release_attempt_without_passing_test = authorized_disposition_recorded && !test_passed;
    if !test_passed { missing_fields.push("passing_post_maintenance_test_evidence".into()); }
    if !authorized_disposition_recorded { missing_fields.push("authorized_return_to_service_disposition_evidence".into()); }
    missing_fields.sort();

    EventResult {
        event_id: event.event_id.clone(),
        event_class: event.event_class,
        treatment: event.treatment,
        occurrences_per_module_year: event.occurrences_per_module_year,
        event_downtime_minutes,
        technician_hours_per_event,
        labor_cost_per_event,
        parts_cost_per_event,
        direct_cost_per_event,
        annual_downtime_hours_per_module: annual_downtime,
        annual_technician_hours_per_module: annual_labor,
        annual_direct_cost_per_module: annual_cost,
        post_maintenance_test: test_result,
        return_disposition: disposition,
        return_to_service_evidence_complete,
        release_attempt_without_passing_test,
        missing_fields,
    }
}

fn add_values(left: Option<f64>, right: Option<f64>) -> Option<f64> {
    left.zip(right).map(|(a, b)| a + b)
}

fn evaluate_profile(
    site_id: &str,
    template_id: &str,
    target_good_vehicles_per_year: u64,
    profile: &LifecycleProfile,
    factory: Option<&FactoryModuleScenario>,
) -> ProfileResult {
    let mut events: Vec<&ServiceEvent> = profile.events.iter().collect();
    events.sort_by(|left, right| left.event_id.cmp(&right.event_id));
    let event_results: Vec<EventResult> = events.iter().map(|event| evaluate_event(profile, event)).collect();
    let is_baseline = profile.factory_config_id.is_none();

    let mut planned = AnnualAccumulator::new();
    let mut unplanned = AnnualAccumulator::new();
    for result in &event_results {
        match result.event_class { EventClass::Planned => planned.add_event(result), EventClass::Unplanned => unplanned.add_event(result) }
    }

    let planned_downtime = if is_baseline { Some(0.0) } else { planned.downtime() };
    let unplanned_downtime = if is_baseline { Some(0.0) } else { unplanned.downtime() };
    let planned_labor = if is_baseline { Some(0.0) } else { planned.technician_hours() };
    let unplanned_labor = if is_baseline { Some(0.0) } else { unplanned.technician_hours() };
    let planned_cost = if is_baseline { Some(0.0) } else { planned.direct_cost() };
    let unplanned_cost = if is_baseline { Some(0.0) } else { unplanned.direct_cost() };

    let total_downtime = add_values(planned_downtime, unplanned_downtime);
    let total_labor = add_values(planned_labor, unplanned_labor);
    let total_cost = add_values(planned_cost, unplanned_cost);
    let downtime_fraction = if is_baseline { None } else {
        total_downtime.zip(profile.annual_operating_hours_per_module).map(|(down, operating)| down / operating)
    };

    let mut missing_fields = Vec::new();
    for (label, value) in [
        ("planned_annual_downtime", planned_downtime),
        ("unplanned_annual_downtime", unplanned_downtime),
        ("planned_annual_technician_hours", planned_labor),
        ("unplanned_annual_technician_hours", unplanned_labor),
        ("planned_annual_direct_cost", planned_cost),
        ("unplanned_annual_direct_cost", unplanned_cost),
        ("total_annual_direct_service_cost", total_cost),
    ] {
        if value.is_none() { missing_fields.push(label.into()); }
    }
    if !is_baseline && profile.annual_operating_hours_per_module.is_none() {
        missing_fields.push("annual_operating_hours_per_module".into());
    }
    if !is_baseline && profile.labor_cost_per_hour.is_none() { missing_fields.push("labor_cost_per_hour".into()); }
    for result in &event_results {
        for field in &result.missing_fields { missing_fields.push(format!("{}:{field}", result.event_id)); }
    }
    missing_fields.sort();
    missing_fields.dedup();

    let test_failure_count = event_results.iter().filter(|event| event.post_maintenance_test == Some(TestResult::Fail)).count() as u64;
    let release_attempt_count = event_results.iter().filter(|event| event.release_attempt_without_passing_test).count() as u64;
    let all_return_evidence = event_results.iter().all(|event| event.return_to_service_evidence_complete);
    let any_blocked = event_results.iter().any(|event| {
        event.post_maintenance_test == Some(TestResult::Fail)
            || event.return_disposition == Some(ReturnDisposition::Held)
            || event.missing_fields.iter().any(|field| field.ends_with(":blocked_task"))
            || event.release_attempt_without_passing_test
    });
    let all_synthetic = profile.evidence_class == EvidenceClass::Synthetic
        && events.iter().all(|event| event.occurrence_evidence_class == EvidenceClass::Synthetic)
        && profile.events.iter().flat_map(|event| event.tasks.iter())
            .all(|task| task.evidence_class == EvidenceClass::Synthetic || task.outcome == PhaseOutcome::Unknown);
    let any_unknown = !missing_fields.is_empty()
        || profile.evidence_class == EvidenceClass::Unknown
        || profile.events.iter().any(|event| {
            event.occurrence_evidence_class == EvidenceClass::Unknown
                || event.occurrences_per_module_year.is_none()
                || event.tasks.iter().any(|task| task.outcome == PhaseOutcome::Unknown)
        });
    let downtime_overrun = downtime_fraction.is_some_and(|fraction| fraction > 1.0 + 1e-9);
    let status = if any_blocked || downtime_overrun { Status::Blocked }
        else if any_unknown { Status::Indeterminate }
        else if all_synthetic { Status::PassesSyntheticScreen }
        else { Status::PassesEvidenceBackedScreen };

    let mut findings = Vec::new();
    if is_baseline {
        findings.push("truck_only_reports_zero_incremental_module_burden_only; base_vehicle_maintenance_is_excluded".into());
    }
    if !is_baseline && unplanned_cost.is_none() {
        findings.push("unplanned_failure_frequency_or_cost_is_unknown; whole_life_cost_ranking_withheld".into());
    }
    if !all_return_evidence && !profile.events.is_empty() {
        findings.push("one_or_more_service_events_lack_complete_test_and_authorized_disposition_evidence".into());
    }
    if test_failure_count > 0 { findings.push("post_maintenance_test_failure_prevents_return_to_service_evidence_completion".into()); }
    if release_attempt_count > 0 { findings.push("return_to_service_disposition_claimed_without_a_passing_post_maintenance_test".into()); }
    if downtime_overrun { findings.push("additive_single_module_downtime_exceeds_annual_operating_time".into()); }
    if factory.is_some_and(|row| !row.base_factory_production_release_ready) {
        findings.push("base_factory_is_not_production_release_ready".into());
    }
    if factory.is_some_and(|row| !row.module_production_release_ready) && !is_baseline {
        findings.push("module_factory_screen_does_not_grant_production_release".into());
    }
    findings.sort();

    let factory_context = FactoryContext {
        source_factory_config_id: if is_baseline { None } else { factory.map(|row| row.config_id.clone()) },
        factory_screen_status: if is_baseline { None } else { factory.map(|row| row.status.clone()) },
        equivalent_good_vehicle_capacity_per_year: if is_baseline { None } else { factory.and_then(|row| row.equivalent_good_vehicle_capacity_per_year) },
        good_vehicle_capacity_margin_per_year: if is_baseline { None } else { factory.and_then(|row| row.good_vehicle_capacity_margin_per_year) },
        base_factory_production_release_ready: factory.is_some_and(|row| row.base_factory_production_release_ready),
        factory_evidence_gaps: factory.map(|row| row.base_factory_evidence_gaps.clone()).unwrap_or_default(),
        factory_missing_fields: if is_baseline { Vec::new() } else { factory.map(|row| row.missing_fields.clone()).unwrap_or_default() },
    };

    ProfileResult {
        site_id: site_id.to_string(),
        template_id: template_id.to_string(),
        target_good_vehicles_per_year,
        config_id: profile.config_id.clone(),
        architecture: profile.architecture.clone(),
        evidence_class: profile.evidence_class,
        currency: profile.currency.clone(),
        cost_basis_year: profile.cost_basis_year,
        annual_operating_hours_per_module: profile.annual_operating_hours_per_module,
        base_vehicle_maintenance_included: profile.base_vehicle_maintenance_included,
        factory: factory_context,
        event_count: event_results.len(),
        planned_annual_downtime_hours: planned_downtime,
        unplanned_annual_downtime_hours: unplanned_downtime,
        total_annual_downtime_hours: total_downtime,
        planned_annual_technician_hours: planned_labor,
        unplanned_annual_technician_hours: unplanned_labor,
        total_annual_technician_hours: total_labor,
        planned_annual_direct_cost: planned_cost,
        unplanned_annual_direct_cost: unplanned_cost,
        total_annual_direct_service_cost: total_cost,
        single_module_downtime_fraction: downtime_fraction,
        event_rates_complete: is_baseline || event_results.iter().all(|event| event.occurrences_per_module_year.is_some()),
        task_observations_complete: is_baseline || event_results.iter().all(|event| event.event_downtime_minutes.is_some() && event.technician_hours_per_event.is_some()),
        all_return_to_service_evidence_complete: all_return_evidence,
        post_maintenance_test_failure_count: test_failure_count,
        release_attempt_without_passing_test_count: release_attempt_count,
        service_burden_comparison_complete: total_downtime.is_some() && total_labor.is_some() && total_cost.is_some(),
        whole_life_cost_ranking_eligible: false,
        module_production_release_ready: false,
        status,
        missing_fields,
        findings,
        events: event_results,
        claim_ceiling: CLAIM_CEILING,
    }
}

/// Join lifecycle-specific service data to the module factory matrix without recalculating factory yield or cost.
pub fn validate_and_evaluate(factory: &FactoryModuleReport, catalog: &LifecycleCatalog) -> Result<LifecycleReport, Vec<String>> {
    validate_inputs(factory, catalog)?;
    let mut profiles = catalog.profiles.clone();
    profiles.sort_by(|left, right| left.config_id.cmp(&right.config_id));
    let mut factory_rows = factory.results.clone();
    factory_rows.sort_by(|left, right| {
        (&left.site_id, &left.template_id, left.target_good_vehicles_per_year, &left.config_id)
            .cmp(&(&right.site_id, &right.template_id, right.target_good_vehicles_per_year, &right.config_id))
    });

    let mut results = Vec::new();
    for profile in profiles.iter().filter(|profile| profile.factory_config_id.is_some()) {
        let config_id = profile.factory_config_id.as_deref().unwrap_or_default();
        for row in factory_rows.iter().filter(|row| row.config_id == config_id) {
            results.push(evaluate_profile(&row.site_id, &row.template_id, row.target_good_vehicles_per_year, profile, Some(row)));
        }
    }

    // One truck-only control is emitted per distinct base factory scenario.
    let baseline = profiles.iter().find(|profile| profile.factory_config_id.is_none())
        .ok_or_else(|| vec!["truck-only baseline disappeared after validation".into()])?;
    let mut base_scenarios = BTreeMap::<BaseScenarioKey, &FactoryModuleScenario>::new();
    for row in &factory_rows {
        base_scenarios.entry(BaseScenarioKey {
            site_id: row.site_id.clone(),
            template_id: row.template_id.clone(),
            target_good_vehicles_per_year: row.target_good_vehicles_per_year,
        }).or_insert(row);
    }
    for (key, row) in base_scenarios {
        results.push(evaluate_profile(&key.site_id, &key.template_id, key.target_good_vehicles_per_year, baseline, Some(row)));
    }
    results.sort_by(|left, right| {
        (&left.site_id, &left.template_id, left.target_good_vehicles_per_year, &left.config_id)
            .cmp(&(&right.site_id, &right.template_id, right.target_good_vehicles_per_year, &right.config_id))
    });
    Ok(LifecycleReport {
        schema_version: catalog.schema_version.clone(),
        benchmark_id: catalog.benchmark_id.clone(),
        base_factory_benchmark_id: factory.base_factory_benchmark_id.clone(),
        scenario_count: factory.results.len(),
        result_count: results.len(),
        results,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(index: usize, phase: ServicePhase, outcome: PhaseOutcome) -> ServiceTask {
        let performed = outcome == PhaseOutcome::Performed;
        ServiceTask {
            sequence_index: index as u8,
            phase,
            outcome,
            elapsed_minutes: performed.then_some(10.0),
            technician_count: performed.then_some(1),
            skill_code: performed.then_some("ev-module-technician".into()),
            evidence_class: if outcome == PhaseOutcome::Unknown { EvidenceClass::Unknown } else { EvidenceClass::Synthetic },
            evidence_reference: format!("synthetic://task/{index}"),
            measurement_method: performed.then_some("synthetic_fixture_arithmetic".into()),
            outcome_note: (outcome != PhaseOutcome::Performed).then_some("explicit fixture outcome".into()),
            test_result: (phase == ServicePhase::PostMaintenanceTest && performed).then_some(TestResult::Pass),
            return_disposition: (phase == ServicePhase::ReturnToServiceDisposition && performed)
                .then_some(ReturnDisposition::ReleasedByAuthorizedPersonnel),
        }
    }

    fn inspection_event() -> ServiceEvent {
        let outcomes = [
            PhaseOutcome::Performed, PhaseOutcome::Performed, PhaseOutcome::Performed,
            PhaseOutcome::NotApplicable, PhaseOutcome::NotApplicable, PhaseOutcome::NotApplicable,
            PhaseOutcome::Performed, PhaseOutcome::Performed,
        ];
        ServiceEvent {
            event_id: "scheduled-inspection".into(),
            event_class: EventClass::Planned,
            treatment: EventTreatment::Inspection,
            occurrences_per_module_year: Some(2.0),
            occurrence_evidence_class: EvidenceClass::Synthetic,
            occurrence_evidence_reference: "synthetic://event/inspection-frequency".into(),
            occurrence_basis: "synthetic two inspections per module-year".into(),
            observed_event_count: None,
            exposure_module_hours: None,
            parts_cost_per_occurrence: Some(5.0),
            tasks: REQUIRED_PHASES.iter().enumerate().map(|(i, phase)| task(i, *phase, outcomes[i])).collect(),
        }
    }

    fn repair_event_unknown_rate() -> ServiceEvent {
        let mut tasks: Vec<ServiceTask> = REQUIRED_PHASES.iter().enumerate()
            .map(|(i, phase)| task(i, *phase, PhaseOutcome::Performed)).collect();
        tasks[4].elapsed_minutes = Some(45.0);
        ServiceEvent {
            event_id: "unscheduled-repair".into(),
            event_class: EventClass::Unplanned,
            treatment: EventTreatment::Repair,
            occurrences_per_module_year: None,
            occurrence_evidence_class: EvidenceClass::Unknown,
            occurrence_evidence_reference: "unknown://failure-rate-not-measured".into(),
            occurrence_basis: "annual event rate not measured".into(),
            observed_event_count: None,
            exposure_module_hours: None,
            parts_cost_per_occurrence: Some(40.0),
            tasks,
        }
    }

    fn profile(config_id: &str, architecture: &str) -> LifecycleProfile {
        LifecycleProfile {
            config_id: config_id.into(),
            factory_config_id: Some(config_id.into()),
            architecture: architecture.into(),
            evidence_class: EvidenceClass::Synthetic,
            evidence_reference: format!("synthetic://profile/{config_id}"),
            currency: "USD".into(),
            cost_basis_year: 2026,
            labor_cost_per_hour: Some(45.0),
            annual_operating_hours_per_module: Some(2_000.0),
            base_vehicle_maintenance_included: false,
            events: vec![inspection_event(), repair_event_unknown_rate()],
            notes: vec!["synthetic service arithmetic only".into()],
        }
    }

    fn factory_row(config_id: &str, site: &str, template: &str, target: u64) -> FactoryModuleScenario {
        FactoryModuleScenario {
            site_id: site.into(),
            template_id: template.into(),
            config_id: config_id.into(),
            architecture: "reference".into(),
            target_good_vehicles_per_year: target,
            equivalent_good_vehicle_capacity_per_year: Some(target),
            good_vehicle_capacity_margin_per_year: Some(0),
            status: "passes_synthetic_screen".into(),
            base_factory_production_release_ready: false,
            base_factory_evidence_gaps: vec!["synthetic base factory".into()],
            module_production_release_ready: false,
            missing_fields: Vec::new(),
        }
    }

    fn factory() -> FactoryModuleReport {
        let ids = [
            "detachable-uav-pod-synthetic-demo",
            "amr-handoff-synthetic-demo",
            "integrated-drone-bay-production-intent-unknown",
            "mobile-drone-station-trailer-production-intent-unknown",
        ];
        let mut results = Vec::new();
        for site in ["sa-example", "eu-example"] {
            for template in ["contract", "modular-ckd"] {
                for target in [1_000, 10_000] {
                    for id in ids { results.push(factory_row(id, site, template, target)); }
                }
            }
        }
        FactoryModuleReport {
            schema_version: "0.1.0".into(),
            benchmark_id: "EV-CARRIER-MODULE-MFG-001".into(),
            base_factory_benchmark_id: "EV-FACTORY-DESIGN-001".into(),
            results,
        }
    }

    fn catalog() -> LifecycleCatalog {
        let pod = profile("detachable-uav-pod-synthetic-demo", "detachable_drone_pod");
        let amr = profile("amr-handoff-synthetic-demo", "amr_cargo_handoff");
        let mut integrated = profile("integrated-drone-bay-production-intent-unknown", "integrated_enclosed_bay");
        let mut trailer = profile("mobile-drone-station-trailer-production-intent-unknown", "mobile_drone_station_trailer");
        for unknown in [&mut integrated, &mut trailer] {
            unknown.evidence_class = EvidenceClass::Unknown;
            unknown.labor_cost_per_hour = None;
            unknown.annual_operating_hours_per_module = None;
            unknown.events[0].occurrences_per_module_year = None;
            unknown.events[0].occurrence_evidence_class = EvidenceClass::Unknown;
            unknown.events[0].occurrence_evidence_reference = "unknown://inspection-frequency".into();
            unknown.events[0].parts_cost_per_occurrence = None;
            unknown.events[0].tasks = REQUIRED_PHASES.iter().enumerate().map(|(i, p)| task(i, *p, PhaseOutcome::Unknown)).collect();
            unknown.events[1].tasks = REQUIRED_PHASES.iter().enumerate().map(|(i, p)| task(i, *p, PhaseOutcome::Unknown)).collect();
        }
        LifecycleCatalog {
            schema_version: "0.1.0".into(),
            benchmark_id: "EV-CARRIER-LIFECYCLE-001".into(),
            profiles: vec![
                pod, amr, integrated, trailer,
                LifecycleProfile {
                    config_id: "truck-only-zero-incremental-module".into(),
                    factory_config_id: None,
                    architecture: "truck_only_baseline".into(),
                    evidence_class: EvidenceClass::Synthetic,
                    evidence_reference: "synthetic://baseline/no-optional-module".into(),
                    currency: "USD".into(),
                    cost_basis_year: 2026,
                    labor_cost_per_hour: None,
                    annual_operating_hours_per_module: None,
                    base_vehicle_maintenance_included: false,
                    events: Vec::new(),
                    notes: vec!["zero optional-module service burden only; base truck maintenance is excluded".into()],
                },
            ],
            claim_ceiling: "synthetic serviceability fixture".into(),
        }
    }

    #[test]
    fn synthetic_factory_matrix_joins_four_modules_and_truck_only_control() {
        let report = validate_and_evaluate(&factory(), &catalog()).expect("valid fixture");
        assert_eq!(report.scenario_count, 32);
        assert_eq!(report.result_count, 40);
        assert_eq!(report.results.iter().filter(|row| row.architecture == "truck_only_baseline").count(), 8);
        assert_eq!(report.results.iter().filter(|row| row.architecture == "mobile_drone_station_trailer").count(), 8);
        assert!(report.results.iter().all(|row| !row.whole_life_cost_ranking_eligible));
        assert!(report.results.iter().all(|row| !row.module_production_release_ready));
    }

    #[test]
    fn unknown_unplanned_failure_rate_is_not_zero_filled() {
        let p = profile("pod", "detachable_drone_pod");
        let result = evaluate_profile("sa", "contract", 1_000, &p, Some(&factory_row("pod", "sa", "contract", 1_000)));
        assert!(result.planned_annual_direct_cost.is_some());
        assert_eq!(result.unplanned_annual_direct_cost, None);
        assert_eq!(result.total_annual_direct_service_cost, None);
        assert_eq!(result.single_module_downtime_fraction, None);
        assert!(result.missing_fields.contains(&"unplanned_annual_downtime".into()));
    }

    #[test]
    fn failed_post_maintenance_test_blocks_return_to_service_evidence() {
        let mut p = profile("pod", "detachable_drone_pod");
        p.events[0].tasks[6].test_result = Some(TestResult::Fail);
        let result = evaluate_profile("sa", "contract", 1_000, &p, Some(&factory_row("pod", "sa", "contract", 1_000)));
        assert_eq!(result.status, Status::Blocked);
        assert!(!result.all_return_to_service_evidence_complete);
        assert_eq!(result.post_maintenance_test_failure_count, 1);
        assert!(!result.module_production_release_ready);
    }

    #[test]
    fn release_disposition_without_passing_test_is_a_negative_control() {
        let mut p = profile("pod", "detachable_drone_pod");
        p.events[0].tasks[6].test_result = Some(TestResult::Unknown);
        let result = evaluate_profile("sa", "contract", 1_000, &p, Some(&factory_row("pod", "sa", "contract", 1_000)));
        assert_eq!(result.status, Status::Blocked);
        assert_eq!(result.release_attempt_without_passing_test_count, 1);
    }

    #[test]
    fn unknown_task_duration_is_not_zero_filled() {
        let mut p = profile("pod", "detachable_drone_pod");
        p.events[0].tasks[2].outcome = PhaseOutcome::Unknown;
        p.events[0].tasks[2].elapsed_minutes = None;
        p.events[0].tasks[2].technician_count = None;
        p.events[0].tasks[2].skill_code = None;
        p.events[0].tasks[2].evidence_class = EvidenceClass::Unknown;
        let result = evaluate_profile("sa", "contract", 1_000, &p, Some(&factory_row("pod", "sa", "contract", 1_000)));
        assert_eq!(result.events[0].event_downtime_minutes, None);
        assert_eq!(result.planned_annual_downtime_hours, None);
    }

    #[test]
    fn missing_service_phase_is_rejected() {
        let mut c = catalog();
        c.profiles[0].events[0].tasks.pop();
        assert!(validate_inputs(&factory(), &c).is_err());
    }

    #[test]
    fn phase_order_and_sequence_are_strict() {
        let mut c = catalog();
        c.profiles[0].events[0].tasks.swap(0, 1);
        assert!(validate_inputs(&factory(), &c).is_err());
    }

    #[test]
    fn not_applicable_phase_requires_an_explicit_reason() {
        let mut c = catalog();
        c.profiles[0].events[0].tasks[3].outcome_note = None;
        assert!(validate_inputs(&factory(), &c).is_err());
    }

    #[test]
    fn measured_unplanned_rate_requires_exposure_reconciliation() {
        let mut c = catalog();
        let event = &mut c.profiles[0].events[1];
        event.occurrences_per_module_year = Some(0.1);
        event.occurrence_evidence_class = EvidenceClass::Measured;
        event.observed_event_count = None;
        event.exposure_module_hours = None;
        assert!(validate_inputs(&factory(), &c).is_err());
    }

    #[test]
    fn base_vehicle_maintenance_is_never_silently_added_to_module_profile() {
        let mut c = catalog();
        c.profiles[0].base_vehicle_maintenance_included = true;
        assert!(validate_inputs(&factory(), &c).is_err());
    }

    #[test]
    fn input_order_permutations_produce_identical_reports() {
        let first = validate_and_evaluate(&factory(), &catalog()).expect("first");
        let mut f = factory();
        f.results.reverse();
        let mut c = catalog();
        c.profiles.reverse();
        for profile in &mut c.profiles { profile.events.reverse(); }
        let second = validate_and_evaluate(&f, &c).expect("permuted");
        assert_eq!(first, second);
    }

    #[test]
    fn factory_base_release_gaps_are_carried_without_granting_module_release() {
        let mut f = factory();
        f.results[0].base_factory_evidence_gaps = vec!["evidence missing".into()];
        let report = validate_and_evaluate(&f, &catalog()).expect("valid");
        assert!(report.results.iter().all(|row| !row.module_production_release_ready));
        assert!(report.results.iter().any(|row| !row.factory.base_factory_production_release_ready));
    }

    #[test]
    fn additive_downtime_fraction_requires_known_unplanned_rate_and_operating_hours() {
        let mut p = profile("pod", "detachable_drone_pod");
        p.events[1].occurrences_per_module_year = Some(0.1);
        p.events[1].occurrence_evidence_class = EvidenceClass::Synthetic;
        p.events[1].occurrence_evidence_reference = "synthetic://repair-rate".into();
        let result = evaluate_profile("sa", "contract", 1_000, &p, Some(&factory_row("pod", "sa", "contract", 1_000)));
        assert!(result.single_module_downtime_fraction.is_some());
        assert!(result.single_module_downtime_fraction.unwrap() >= 0.0);
    }
}
