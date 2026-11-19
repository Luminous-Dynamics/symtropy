mod scheduler;

use scheduler::{AvailabilityWindow, ResourceProfile, ResourceRequirement, ScheduledTask, TaskSpec, exact_small_schedule, schedule};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Case {
    schema_version: String,
    case_id: String,
    case_maturity: String,
    service_boundary: ServiceBoundary,
    assets: Vec<Asset>,
    parts: Vec<Part>,
    depots: Vec<Depot>,
    resources: Vec<Resource>,
    logistics_routes: Vec<LogisticsRoute>,
    evidence_records: Vec<EvidenceRecord>,
    events: Vec<Event>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServiceBoundary {
    service_id: String,
    service_unit: String,
    energy_unit: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Asset {
    asset_id: String,
    asset_class: String,
    variant_id: String,
    interface_profile_id: String,
    location_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Part {
    part_id: String,
    part_class: String,
    variant_id: String,
    approved_interface_profiles: Vec<String>,
    evidence_record_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Depot {
    depot_id: String,
    location_id: String,
    inventory: Vec<Inventory>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Inventory {
    part_id: String,
    quantity_on_hand: u32,
    quantity_reserved: u32,
    quantity_serviceable: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Resource {
    resource_id: String,
    resource_type: String,
    available_units: u32,
    calendar_period_hours: f64,
    availability_windows: Vec<AvailabilityWindow>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LogisticsRoute {
    route_id: String,
    from_location_id: String,
    to_location_id: String,
    transit_hours: f64,
    max_cargo_kg: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EvidenceRecord {
    evidence_record_id: String,
    evidence_class: String,
    description: String,
    source_uri: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Event {
    event_id: String,
    asset_id: String,
    start_hour: f64,
    failure_mode: String,
    firm_capacity_lost_mw: f64,
    selected_plan: RecoveryPlan,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecoveryPlan {
    plan_id: String,
    part_source: PartSource,
    logistics_route_id: Option<String>,
    transport_mass_kg: Option<f64>,
    alternative_firm_capacity_mw: f64,
    claimed_restoration_hours: f64,
    expected_unserved_energy_mwh: f64,
    tasks: Vec<Task>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum PartSource {
    DepotStock {
        #[serde(rename = "depotId")]
        depot_id: String,
        #[serde(rename = "partId")]
        part_id: String,
        quantity: u32,
    },
    OemProcurement {
        #[serde(rename = "supplierId")]
        supplier_id: String,
        #[serde(rename = "partId")]
        part_id: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Task {
    task_id: String,
    stage: String,
    #[serde(default)]
    depends_on: Vec<String>,
    duration_hours: f64,
    #[serde(default)]
    resource_requirements: Vec<ResourceRequirement>,
}

fn unique_ids<'a>(ids: impl Iterator<Item = &'a str>, label: &str, errors: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id.to_string()) {
            errors.push(format!("duplicate {label} id: {id}"));
        }
    }
}

fn dependency_reaches_stage(
    task: &Task,
    required_stage: &str,
    tasks_by_id: &BTreeMap<String, &Task>,
) -> bool {
    fn visit(
        dependencies: &[String],
        required_stage: &str,
        tasks_by_id: &BTreeMap<String, &Task>,
        visited: &mut BTreeSet<String>,
    ) -> bool {
        for dependency in dependencies {
            if !visited.insert(dependency.clone()) {
                continue;
            }
            if let Some(predecessor) = tasks_by_id.get(dependency) {
                if predecessor.stage == required_stage
                    || visit(&predecessor.depends_on, required_stage, tasks_by_id, visited)
                {
                    return true;
                }
            }
        }
        false
    }

    visit(
        &task.depends_on,
        required_stage,
        tasks_by_id,
        &mut BTreeSet::new(),
    )
}

fn validate(case: &Case) -> Vec<String> {
    let mut errors = Vec::new();

    if case.schema_version != "0.1.0" {
        errors.push("unsupported schemaVersion".to_string());
    }
    if case.case_id.trim().is_empty() {
        errors.push("caseId must not be empty".to_string());
    }
    if case.service_boundary.service_id.trim().is_empty()
        || case.service_boundary.service_unit != "MW"
        || case.service_boundary.energy_unit != "MWh"
    {
        errors.push("serviceBoundary must identify MW and MWh".to_string());
    }
    if !matches!(
        case.case_maturity.as_str(),
        "synthetic_fixture" | "engineering_estimate" | "field_observed" | "independently_reviewed"
    ) {
        errors.push("unsupported caseMaturity".to_string());
    }

    unique_ids(case.assets.iter().map(|x| x.asset_id.as_str()), "asset", &mut errors);
    unique_ids(case.parts.iter().map(|x| x.part_id.as_str()), "part", &mut errors);
    unique_ids(case.depots.iter().map(|x| x.depot_id.as_str()), "depot", &mut errors);
    unique_ids(case.resources.iter().map(|x| x.resource_id.as_str()), "resource", &mut errors);
    unique_ids(case.logistics_routes.iter().map(|x| x.route_id.as_str()), "route", &mut errors);
    unique_ids(
        case.evidence_records.iter().map(|x| x.evidence_record_id.as_str()),
        "evidence",
        &mut errors,
    );
    unique_ids(case.events.iter().map(|x| x.event_id.as_str()), "event", &mut errors);

    let assets: BTreeMap<_, _> = case.assets.iter().map(|x| (x.asset_id.as_str(), x)).collect();
    let parts: BTreeMap<_, _> = case.parts.iter().map(|x| (x.part_id.as_str(), x)).collect();
    let depots: BTreeMap<_, _> = case.depots.iter().map(|x| (x.depot_id.as_str(), x)).collect();
    let resources: BTreeMap<_, _> = case.resources.iter().map(|x| (x.resource_id.as_str(), x)).collect();
    let routes: BTreeMap<_, _> = case.logistics_routes.iter().map(|x| (x.route_id.as_str(), x)).collect();
    let evidence: BTreeMap<_, _> = case.evidence_records.iter().map(|x| (x.evidence_record_id.as_str(), x)).collect();

    for depot in &case.depots {
        let mut inventory_part_ids = BTreeSet::new();
        if depot.depot_id.trim().is_empty() || depot.location_id.trim().is_empty() {
            errors.push("depotId and depot locationId must not be empty".to_string());
        }
        for item in &depot.inventory {
            if !inventory_part_ids.insert(item.part_id.as_str()) {
                errors.push(format!(
                    "duplicate partId {} in depot {} inventory",
                    item.part_id, depot.depot_id
                ));
            }
            if !parts.contains_key(item.part_id.as_str()) {
                errors.push(format!(
                    "depot {} inventory references unknown part {}",
                    depot.depot_id, item.part_id
                ));
            }
            if item.quantity_reserved > item.quantity_serviceable
                || item.quantity_serviceable > item.quantity_on_hand
            {
                errors.push(format!(
                    "depot {} part {} has inconsistent inventory counts",
                    depot.depot_id, item.part_id
                ));
            }
        }
    }

    let mut stock_demand: BTreeMap<(String, String), u32> = BTreeMap::new();
    let mut schedule_tasks = Vec::new();
    let capacities: BTreeMap<String, u32> = case
        .resources
        .iter()
        .map(|x| (x.resource_id.clone(), x.available_units))
        .collect();
    let required_stages = [
        "detect_and_isolate",
        "diagnose",
        "source_part",
        "crew_mobilise",
        "repair_or_replace",
        "test_and_commission",
    ];
    let allowed_stages: BTreeSet<&str> = required_stages
        .into_iter()
        .chain(["transport"])
        .collect();

    for event in &case.events {
        let Some(asset) = assets.get(event.asset_id.as_str()) else {
            errors.push(format!("{} references unknown asset {}", event.event_id, event.asset_id));
            continue;
        };

        if event.start_hour < 0.0 || !event.start_hour.is_finite() {
            errors.push(format!("{} has invalid startHour", event.event_id));
        }
        if event.firm_capacity_lost_mw < 0.0 || !event.firm_capacity_lost_mw.is_finite() {
            errors.push(format!("{} has invalid firmCapacityLostMw", event.event_id));
        }

        let plan = &event.selected_plan;
        if event.failure_mode.trim().is_empty() {
            errors.push(format!("{} has empty failureMode", event.event_id));
        }
        if plan.plan_id.trim().is_empty() {
            errors.push(format!("{} has empty planId", event.event_id));
        }
        if plan.alternative_firm_capacity_mw < 0.0
            || !plan.alternative_firm_capacity_mw.is_finite()
            || plan.alternative_firm_capacity_mw > event.firm_capacity_lost_mw
        {
            errors.push(format!("{} has invalid alternative firm capacity", event.event_id));
        }

        let mut stages = BTreeSet::new();
        let mut task_ids = BTreeSet::new();
        for task in &plan.tasks {
            stages.insert(task.stage.as_str());
            if !allowed_stages.contains(task.stage.as_str()) {
                errors.push(format!("{} has unsupported task stage {}", event.event_id, task.stage));
            }
            if task.task_id.trim().is_empty() {
                errors.push(format!("{} has empty taskId", event.event_id));
            }
            if !task_ids.insert(task.task_id.clone()) {
                errors.push(format!("{} has duplicate taskId {}", event.event_id, task.task_id));
            }
            let mut resource_ids = BTreeSet::new();
            for requirement in &task.resource_requirements {
                if requirement.resource_id.trim().is_empty()
                    || requirement.units == 0
                    || !resource_ids.insert(requirement.resource_id.as_str())
                {
                    errors.push(format!(
                        "{} task {} has an empty, zero-unit or duplicate resource requirement",
                        event.event_id, task.task_id
                    ));
                }
            }

            schedule_tasks.push(TaskSpec {
                event_id: event.event_id.clone(),
                task_id: task.task_id.clone(),
                depends_on: task.depends_on.clone(),
                release_hour: event.start_hour,
                duration_hours: task.duration_hours,
                resource_requirements: task.resource_requirements.clone(),
            });
        }

        for stage in required_stages {
            if !stages.contains(stage) {
                errors.push(format!("{} missing required stage {}", event.event_id, stage));
            }
        }

        let is_depot_stock = matches!(&plan.part_source, PartSource::DepotStock { .. });
        let tasks_by_id: BTreeMap<String, &Task> = plan
            .tasks
            .iter()
            .map(|task| (task.task_id.clone(), task))
            .collect();
        for task in &plan.tasks {
            let dependency_has_stage = |required_stage: &str| {
                dependency_reaches_stage(task, required_stage, &tasks_by_id)
            };
            let required_predecessors: &[&str] = match task.stage.as_str() {
                "diagnose" => &["detect_and_isolate"],
                "source_part" | "crew_mobilise" => &["diagnose"],
                "transport" => &["source_part"],
                "repair_or_replace" if is_depot_stock => {
                    &["source_part", "transport", "crew_mobilise"]
                }
                "repair_or_replace" => &["source_part", "crew_mobilise"],
                "test_and_commission" => &["repair_or_replace"],
                _ => &[],
            };
            for predecessor in required_predecessors {
                if !dependency_has_stage(predecessor) {
                    errors.push(format!(
                        "{} task {} must depend on stage {}",
                        event.event_id, task.task_id, predecessor
                    ));
                }
            }
        }

        match &plan.part_source {
            PartSource::DepotStock {
                depot_id,
                part_id,
                quantity,
            } => {
                let Some(depot) = depots.get(depot_id.as_str()) else {
                    errors.push(format!("{} references unknown depot {}", event.event_id, depot_id));
                    continue;
                };
                let Some(inventory) = depot.inventory.iter().find(|x| x.part_id == *part_id) else {
                    errors.push(format!("{} part {} is absent from depot {}", event.event_id, part_id, depot_id));
                    continue;
                };
                if inventory.quantity_reserved > inventory.quantity_serviceable
                    || inventory.quantity_serviceable > inventory.quantity_on_hand
                {
                    errors.push(format!("{} depot {} has inconsistent inventory counts", event.event_id, depot_id));
                }
                let available = inventory.quantity_serviceable.saturating_sub(inventory.quantity_reserved);
                if *quantity == 0 || *quantity > available {
                    errors.push(format!(
                        "{} requests {} of {}, but only {} serviceable unreserved unit(s) exist",
                        event.event_id, quantity, part_id, available
                    ));
                }
                *stock_demand
                    .entry((depot_id.clone(), part_id.clone()))
                    .or_default() += *quantity;

                let route_id = plan.logistics_route_id.as_deref();
                let mass = plan.transport_mass_kg;
                match (route_id, mass) {
                    (Some(route_id), Some(mass)) => {
                        if let Some(route) = routes.get(route_id) {
                            if route.from_location_id != depot.location_id
                                || route.to_location_id != asset.location_id
                            {
                                errors.push(format!("{} logistics route does not connect depot to asset", event.event_id));
                            }
                            if !mass.is_finite() || mass <= 0.0 || mass > route.max_cargo_kg {
                                errors.push(format!(
                                    "{} cargo mass {}kg is invalid or exceeds route limit {}kg",
                                    event.event_id, mass, route.max_cargo_kg
                                ));
                            }
                            let transport_task = plan.tasks.iter().find(|x| x.stage == "transport");
                            if let Some(task) = transport_task {
                                if task.duration_hours + 1e-6 < route.transit_hours {
                                    errors.push(format!("{} transport task is shorter than route transit time", event.event_id));
                                }
                            } else {
                                errors.push(format!("{} depot-stock recovery is missing transport stage", event.event_id));
                            }
                        } else {
                            errors.push(format!("{} references unknown logistics route {}", event.event_id, route_id));
                        }
                    }
                    _ => errors.push(format!("{} depot-stock recovery requires route and cargo mass", event.event_id)),
                }
            }
            PartSource::OemProcurement { supplier_id, part_id } => {
                if supplier_id.trim().is_empty() {
                    errors.push(format!("{} OEM procurement requires a supplierId", event.event_id));
                }
                if plan.logistics_route_id.is_some() || plan.transport_mass_kg.is_some() {
                    errors.push(format!("{} OEM procurement cannot silently reuse a depot route allocation", event.event_id));
                }
                if !stages.contains("source_part") {
                    errors.push(format!("{} OEM procurement is missing source_part duration", event.event_id));
                }
                if !parts.contains_key(part_id.as_str()) {
                    errors.push(format!("{} references unknown procured part {}", event.event_id, part_id));
                }
            }
        }

        let part_id = match &plan.part_source {
            PartSource::DepotStock { part_id, .. } => part_id,
            PartSource::OemProcurement { part_id, .. } => part_id,
        };
        if let Some(part) = parts.get(part_id.as_str()) {
            if part.part_class != asset.asset_class {
                errors.push(format!(
                    "{} part {} class {} does not match asset class {}",
                    event.event_id, part.part_id, part.part_class, asset.asset_class
                ));
            }
            if !part.approved_interface_profiles.iter().any(|x| x == &asset.interface_profile_id) {
                errors.push(format!(
                    "{} part {} is not approved for interface profile {}",
                    event.event_id, part.part_id, asset.interface_profile_id
                ));
            }
            if let Some(record) = evidence.get(part.evidence_record_id.as_str()) {
                if case.case_maturity == "synthetic_fixture" {
                    if record.evidence_class != "synthetic_fixture" {
                        errors.push(format!(
                            "{} synthetic case has non-synthetic evidence class {}",
                            event.event_id, record.evidence_class
                        ));
                    }
                } else if !matches!(record.evidence_class.as_str(), "site_qualified" | "independent_field_verified") {
                    errors.push(format!(
                        "{} operational case lacks site-qualified or independently field-verified part evidence",
                        event.event_id
                    ));
                }
                if record.description.trim().is_empty() {
                    errors.push(format!("{} evidence description is empty", event.event_id));
                }
                if let Some(uri) = &record.source_uri {
                    if !(uri.starts_with("https://") || uri.starts_with("http://")) {
                        errors.push(format!("{} evidence sourceUri is not an HTTP(S) URI", event.event_id));
                    }
                }
                if case.case_maturity != "synthetic_fixture"
                    && record.source_uri.as_deref().is_none_or(str::is_empty)
                {
                    errors.push(format!("{} non-synthetic evidence requires sourceUri", event.event_id));
                }
            } else {
                errors.push(format!(
                    "{} part {} references missing evidence {}",
                    event.event_id, part.part_id, part.evidence_record_id
                ));
            }
        }
    }

    for ((depot_id, part_id), demand) in stock_demand {
        if let Some(depot) = depots.get(depot_id.as_str()) {
            if let Some(item) = depot.inventory.iter().find(|x| x.part_id == part_id) {
                let available = item.quantity_serviceable.saturating_sub(item.quantity_reserved);
                if demand > available {
                    errors.push(format!(
                        "spare double-allocation: {demand} units of {part_id} demanded from {depot_id}, only {available} available"
                    ));
                }
            }
        }
    }

    match schedule(&schedule_tasks, &capacities) {
        Ok(scheduled) => {
            for event in &case.events {
                let completion = scheduled
                    .iter()
                    .filter(|task| task.event_id == event.event_id)
                    .map(|task| task.end_hour)
                    .fold(event.start_hour, f64::max);
                let elapsed = completion - event.start_hour;
                let plan = &event.selected_plan;
                if (elapsed - plan.claimed_restoration_hours).abs() > 1e-6 {
                    errors.push(format!(
                        "{} claimed restoration {}h but dependency/resource schedule gives {}h",
                        event.event_id, plan.claimed_restoration_hours, elapsed
                    ));
                }
                let net_shortfall =
                    (event.firm_capacity_lost_mw - plan.alternative_firm_capacity_mw).max(0.0);
                let expected_energy = net_shortfall * elapsed;
                if (expected_energy - plan.expected_unserved_energy_mwh).abs() > 1e-5 {
                    errors.push(format!(
                        "{} expected unserved energy {} MWh but scheduled arithmetic gives {} MWh",
                        event.event_id, plan.expected_unserved_energy_mwh, expected_energy
                    ));
                }
            }
        }
        Err(schedule_errors) => errors.extend(schedule_errors),
    }

    for asset in &case.assets {
        if asset.asset_class.trim().is_empty()
            || asset.variant_id.trim().is_empty()
            || asset.interface_profile_id.trim().is_empty()
        {
            errors.push(format!("{} has incomplete class, variant or interface identity", asset.asset_id));
        }
    }
    for part in &case.parts {
        if part.part_class.trim().is_empty() || part.variant_id.trim().is_empty() {
            errors.push(format!("{} has incomplete class/variant identity", part.part_id));
        }
        if part.approved_interface_profiles.is_empty() {
            errors.push(format!("{} has no approved interface profiles", part.part_id));
        }
    }
    for resource in &case.resources {
        if resource.resource_type.trim().is_empty()
            || resource.available_units == 0
            || !resource.calendar_period_hours.is_finite()
            || resource.calendar_period_hours <= 0.0
            || resource.availability_windows.is_empty()
        {
            errors.push(format!("{} has invalid resource type, capacity or calendar", resource.resource_id));
        }
    }
    for route in &case.logistics_routes {
        if !route.transit_hours.is_finite()
            || route.transit_hours <= 0.0
            || !route.max_cargo_kg.is_finite()
            || route.max_cargo_kg <= 0.0
        {
            errors.push(format!("{} has invalid logistics parameters", route.route_id));
        }
    }

    errors.sort();
    errors
}


#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EventScheduleReport {
    event_id: String,
    completion_hour: f64,
    restoration_hours: f64,
    claimed_restoration_hours: f64,
    firm_capacity_lost_mw: f64,
    alternative_firm_capacity_mw: f64,
    net_firm_shortfall_mw: f64,
    calculated_unserved_energy_mwh: f64,
    declared_unserved_energy_mwh: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExactSmallComparison {
    greedy_makespan_hours: f64,
    exact_makespan_hours: f64,
    absolute_gap_hours: f64,
    relative_gap: f64,
    exact_schedule: Vec<ScheduledTask>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScheduleReport {
    case_id: String,
    case_maturity: String,
    scheduler: String,
    optimality_status: String,
    exact_small_comparison: Option<ExactSmallComparison>,
    events: Vec<EventScheduleReport>,
    scheduled_tasks: Vec<ScheduledTask>,
}

fn resource_capacities(case: &Case) -> BTreeMap<String, ResourceProfile> {
    case.resources
        .iter()
        .map(|resource| {
            (
                resource.resource_id.clone(),
                ResourceProfile {
                    available_units: resource.available_units,
                    calendar_period_hours: resource.calendar_period_hours,
                    availability_windows: resource.availability_windows.clone(),
                },
            )
        })
        .collect()
}

fn task_specs_for_case(case: &Case) -> Vec<TaskSpec> {
    case.events
        .iter()
        .flat_map(|event| {
            event.selected_plan.tasks.iter().map(move |task| TaskSpec {
                event_id: event.event_id.clone(),
                task_id: task.task_id.clone(),
                depends_on: task.depends_on.clone(),
                release_hour: event.start_hour,
                duration_hours: task.duration_hours,
                resource_requirements: task.resource_requirements.clone(),
            })
        })
        .collect()
}

fn schedule_for_case(case: &Case) -> Result<Vec<ScheduledTask>, Vec<String>> {
    schedule(&task_specs_for_case(case), &resource_capacities(case))
}

fn build_report(
    case: &Case,
    mut scheduled_tasks: Vec<ScheduledTask>,
    exact_small_comparison: Option<ExactSmallComparison>,
) -> ScheduleReport {
    scheduled_tasks.sort_by(|left, right| {
        left.start_hour
            .total_cmp(&right.start_hour)
            .then_with(|| left.event_id.cmp(&right.event_id))
            .then_with(|| left.task_id.cmp(&right.task_id))
    });
    let events = case
        .events
        .iter()
        .map(|event| {
            let completion_hour = scheduled_tasks
                .iter()
                .filter(|task| task.event_id == event.event_id)
                .map(|task| task.end_hour)
                .fold(event.start_hour, f64::max);
            let restoration_hours = completion_hour - event.start_hour;
            let net_firm_shortfall_mw = (event.firm_capacity_lost_mw
                - event.selected_plan.alternative_firm_capacity_mw)
                .max(0.0);
            EventScheduleReport {
                event_id: event.event_id.clone(),
                completion_hour,
                restoration_hours,
                claimed_restoration_hours: event.selected_plan.claimed_restoration_hours,
                firm_capacity_lost_mw: event.firm_capacity_lost_mw,
                alternative_firm_capacity_mw: event.selected_plan.alternative_firm_capacity_mw,
                net_firm_shortfall_mw,
                calculated_unserved_energy_mwh: net_firm_shortfall_mw * restoration_hours,
                declared_unserved_energy_mwh: event.selected_plan.expected_unserved_energy_mwh,
            }
        })
        .collect();
    ScheduleReport {
        case_id: case.case_id.clone(),
        case_maturity: case.case_maturity.clone(),
        scheduler: "earliest_feasible_start_list_v0.1".to_string(),
        optimality_status: if exact_small_comparison.is_some() {
            "exhaustively_optimal_for_bounded_case".to_string()
        } else {
            "heuristic_not_proven_optimal".to_string()
        },
        exact_small_comparison,
        events,
        scheduled_tasks,
    }
}

fn run(input: &str) -> Result<(Case, Vec<ScheduledTask>), Vec<String>> {
    let case: Case =
        serde_json::from_str(input).map_err(|error| vec![format!("JSON decode error: {error}")])?;
    let errors = validate(&case);
    if !errors.is_empty() {
        return Err(errors);
    }
    let scheduled = schedule_for_case(&case)?;
    Ok((case, scheduled))
}

fn main() {
    let mut path = None;
    let mut exact_small = false;
    for argument in env::args().skip(1) {
        if argument == "--exact-small" {
            exact_small = true;
        } else if path.is_none() {
            path = Some(argument);
        } else {
            eprintln!("usage: grid-asset-recovery-validator [--exact-small] <case.json>");
            process::exit(2);
        }
    }
    let Some(path) = path else {
        eprintln!("usage: grid-asset-recovery-validator [--exact-small] <case.json>");
        process::exit(2);
    };
    let input = fs::read_to_string(&path).unwrap_or_else(|error| {
        eprintln!("cannot read {path}: {error}");
        process::exit(2);
    });
    match run(&input) {
        Ok((case, scheduled)) => {
            let exact_comparison = if exact_small {
                let tasks = task_specs_for_case(&case);
                let capacities = resource_capacities(&case);
                match exact_small_schedule(&tasks, &capacities) {
                    Ok(exact_schedule) => {
                        let greedy_makespan = scheduled
                            .iter()
                            .map(|task| task.end_hour)
                            .fold(0.0, f64::max);
                        let exact_makespan = exact_schedule
                            .iter()
                            .map(|task| task.end_hour)
                            .fold(0.0, f64::max);
                        Some(ExactSmallComparison {
                            greedy_makespan_hours: greedy_makespan,
                            exact_makespan_hours: exact_makespan,
                            absolute_gap_hours: (greedy_makespan - exact_makespan).max(0.0),
                            relative_gap: if exact_makespan > 0.0 {
                                (greedy_makespan - exact_makespan).max(0.0) / exact_makespan
                            } else {
                                0.0
                            },
                            exact_schedule,
                        })
                    }
                    Err(errors) => {
                        for error in errors {
                            eprintln!("EXACT-REFERENCE-FAIL: {error}");
                        }
                        process::exit(1);
                    }
                }
            } else {
                None
            };
            let report = build_report(&case, scheduled, exact_comparison);
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("schedule report serializes")
            );
        }
        Err(errors) => {
            for error in errors {
                eprintln!("FAIL: {error}");
            }
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const FIXTURE: &str =
        include_str!("../../../docs/research/grid-asset-recovery/data/synthetic-asset-recovery-v0.1.json");

    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).expect("fixture JSON")
    }

    fn check(value: &Value) -> Vec<String> {
        let case: Case = serde_json::from_value(value.clone()).expect("typed fixture");
        validate(&case)
    }

    #[test]
    fn accepts_three_event_dag_recovery_fixture() {
        run(FIXTURE).expect("fixture should pass");
    }

    #[test]
    fn rejects_unqualified_interface_substitution() {
        let mut value = fixture();
        value["events"][0]["assetId"] = Value::String("TX-B1".to_string());
        assert!(check(&value).iter().any(|x| x.contains("not approved")));
    }

    #[test]
    fn rejects_double_allocation_of_one_physical_spare() {
        let mut value = fixture();
        value["events"][1]["selectedPlan"]["partSource"] = json!({
            "kind": "depot_stock",
            "depotId": "DEPOT-CENTRAL",
            "partId": "SPARE-A",
            "quantity": 1
        });
        value["events"][1]["selectedPlan"]["logisticsRouteId"] = Value::String("ROUTE-CENTRAL-NORTH".to_string());
        value["events"][1]["selectedPlan"]["transportMassKg"] = Value::from(90000);
        value["events"][1]["selectedPlan"]["tasks"].as_array_mut().unwrap().insert(
            3,
            json!({
                "taskId": "transport",
                "stage": "transport",
                "dependsOn": ["source-part"],
                "durationHours": 12
            }),
        );
        assert!(check(&value).iter().any(|x| x.contains("spare double-allocation")));
    }

    #[test]
    fn rejects_reserved_inventory() {
        let mut value = fixture();
        value["depots"][0]["inventory"][0]["quantityReserved"] = Value::from(1);
        assert!(check(&value).iter().any(|x| x.contains("only 0 serviceable unreserved")));
    }

    #[test]
    fn rejects_duplicate_inventory_rows() {
        let mut value = fixture();
        let duplicate = value["depots"][0]["inventory"][0].clone();
        value["depots"][0]["inventory"].as_array_mut().unwrap().push(duplicate);
        assert!(check(&value).iter().any(|x| x.contains("duplicate partId SPARE-A in depot DEPOT-CENTRAL")));
    }

    #[test]
    fn rejects_impossible_transport_mass() {
        let mut value = fixture();
        value["logisticsRoutes"][0]["maxCargoKg"] = Value::from(1000);
        assert!(check(&value).iter().any(|x| x.contains("exceeds route limit")));
    }

    #[test]
    fn rejects_missing_commissioning() {
        let mut value = fixture();
        value["events"][0]["selectedPlan"]["tasks"]
            .as_array_mut()
            .unwrap()
            .retain(|task| task["stage"] != "test_and_commission");
        assert!(check(&value)
            .iter()
            .any(|x| x.contains("missing required stage test_and_commission")));
    }

    #[test]
    fn detects_resource_delay_in_claimed_restoration_time() {
        let mut value = fixture();
        value["events"][2]["startHour"] = Value::from(10);
        assert!(check(&value)
            .iter()
            .any(|x| x.contains("dependency/resource schedule gives")));
    }

    #[test]
    fn rejects_synthetic_evidence_promotion() {
        let mut value = fixture();
        value["caseMaturity"] = Value::String("field_observed".to_string());
        assert!(check(&value)
            .iter()
            .any(|x| x.contains("lacks site-qualified")));
    }

    #[test]
    fn rejects_incorrect_unserved_energy_arithmetic() {
        let mut value = fixture();
        value["events"][0]["selectedPlan"]["expectedUnservedEnergyMwh"] = Value::from(1);
        assert!(check(&value)
            .iter()
            .any(|x| x.contains("scheduled arithmetic")));
    }

    #[test]
    fn rejects_dependency_cycle() {
        let mut value = fixture();
        value["events"][0]["selectedPlan"]["tasks"][0]["dependsOn"] =
            json!(["test-commission"]);
        assert!(check(&value).iter().any(|x| x.contains("cycle")));
    }

    #[test]
    fn rejects_missing_essential_stage_dependency() {
        let mut value = fixture();
        value["events"][0]["selectedPlan"]["tasks"][1]["dependsOn"] = json!([]);
        assert!(check(&value).iter().any(|x| x.contains("must depend on stage detect_and_isolate")));
    }
}
