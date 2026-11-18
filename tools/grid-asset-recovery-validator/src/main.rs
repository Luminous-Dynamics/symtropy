use serde::Deserialize;
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
        depot_id: String,
        part_id: String,
        quantity: u32,
    },
    OemProcurement {
        supplier_id: String,
        part_id: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Task {
    stage: String,
    duration_hours: f64,
    resource_id: Option<String>,
    resource_units: Option<u32>,
}

#[derive(Clone, Debug)]
struct Allocation {
    start: f64,
    end: f64,
    units: u32,
    event_id: String,
}

fn unique_ids<'a>(ids: impl Iterator<Item = &'a str>, label: &str, errors: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id.to_string()) {
            errors.push(format!("duplicate {label} id: {id}"));
        }
    }
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

    let mut stock_demand: BTreeMap<(String, String), u32> = BTreeMap::new();
    let mut resource_allocations: BTreeMap<String, Vec<Allocation>> = BTreeMap::new();
    let required_stages = [
        "detect_and_isolate",
        "diagnose",
        "source_part",
        "crew_mobilise",
        "repair_or_replace",
        "test_and_commission",
    ];

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
        let mut stages = BTreeSet::new();
        let mut elapsed = 0.0;
        for task in &plan.tasks {
            stages.insert(task.stage.as_str());
            if task.duration_hours <= 0.0 || !task.duration_hours.is_finite() {
                errors.push(format!("{} has non-positive or non-finite task duration", event.event_id));
                continue;
            }
            let task_start = event.start_hour + elapsed;
            let task_end = task_start + task.duration_hours;

            match (&task.resource_id, task.resource_units) {
                (Some(resource_id), Some(units)) => {
                    if let Some(resource) = resources.get(resource_id.as_str()) {
                        if units == 0 || units > resource.available_units {
                            errors.push(format!(
                                "{} over-allocates resource {}: requested {}, available {}",
                                event.event_id, resource_id, units, resource.available_units
                            ));
                        }
                        resource_allocations
                            .entry(resource_id.clone())
                            .or_default()
                            .push(Allocation {
                                start: task_start,
                                end: task_end,
                                units,
                                event_id: event.event_id.clone(),
                            });
                    } else {
                        errors.push(format!("{} references unknown resource {}", event.event_id, resource_id));
                    }
                }
                (Some(resource_id), None) | (None, Some(_)) => {
                    errors.push(format!(
                        "{} task {} must declare resourceId and resourceUnits together",
                        event.event_id, task.stage
                    ));
                    if !resource_id.is_empty() {}
                }
                (None, None) => {}
            }
            elapsed += task.duration_hours;
        }

        for stage in required_stages {
            if !stages.contains(stage) {
                errors.push(format!("{} missing required stage {}", event.event_id, stage));
            }
        }

        if (elapsed - plan.claimed_restoration_hours).abs() > 1e-6 {
            errors.push(format!(
                "{} claimed restoration {}h but tasks sum to {}h",
                event.event_id, plan.claimed_restoration_hours, elapsed
            ));
        }

        let net_shortfall = (event.firm_capacity_lost_mw - plan.alternative_firm_capacity_mw).max(0.0);
        let expected_energy = net_shortfall * elapsed;
        if (expected_energy - plan.expected_unserved_energy_mwh).abs() > 1e-5 {
            errors.push(format!(
                "{} expected unserved energy {} MWh but arithmetic gives {} MWh",
                event.event_id, plan.expected_unserved_energy_mwh, expected_energy
            ));
        }
        if plan.alternative_firm_capacity_mw > event.firm_capacity_lost_mw {
            errors.push(format!("{} alternate capacity exceeds the declared lost capacity", event.event_id));
        }

        let part_id = match &plan.part_source {
            PartSource::DepotStock {
                depot_id,
                part_id,
                quantity,
            } => {
                let Some(depot) = depots.get(depot_id.as_str()) else {
                    errors.push(format!("{} references unknown depot {}", event.event_id, depot_id));
                    continue;
                };
                let inventory = depot.inventory.iter().find(|x| x.part_id == *part_id);
                let Some(inventory) = inventory else {
                    errors.push(format!("{} part {} is absent from depot {}", event.event_id, part_id, depot_id));
                    continue;
                };
                if inventory.quantity_reserved > inventory.quantity_serviceable
                    || inventory.quantity_serviceable > inventory.quantity_on_hand
                {
                    errors.push(format!("{} depot {} has inconsistent inventory counts", event.event_id, depot_id));
                }
                let available = inventory.quantity_serviceable.saturating_sub(inventory.quantity_reserved);
                if *quantity > available {
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
                            if mass > route.max_cargo_kg {
                                errors.push(format!(
                                    "{} cargo mass {}kg exceeds route limit {}kg",
                                    event.event_id, mass, route.max_cargo_kg
                                ));
                            }
                            let transport_stage = plan.tasks.iter().find(|x| x.stage == "transport");
                            if let Some(task) = transport_stage {
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
                part_id.as_str()
            }
            PartSource::OemProcurement { supplier_id, part_id } => {
                if supplier_id.trim().is_empty() {
                    errors.push(format!("{} OEM procurement requires a supplierId", event.event_id));
                }
                if plan.logistics_route_id.is_some() || plan.transport_mass_kg.is_some() {
                    errors.push(format!("{} OEM procurement cannot silently reuse a depot route allocation", event.event_id));
                }
                part_id.as_str()
            }
        };

        let Some(part) = parts.get(part_id) else {
            errors.push(format!("{} references unknown part {}", event.event_id, part_id));
            continue;
        };
        if !part.approved_interface_profiles.iter().any(|x| x == &asset.interface_profile_id) {
            errors.push(format!(
                "{} part {} is not approved for interface profile {}",
                event.event_id, part.part_id, asset.interface_profile_id
            ));
        }
        let Some(record) = evidence.get(part.evidence_record_id.as_str()) else {
            errors.push(format!(
                "{} part {} references missing evidence {}",
                event.event_id, part.part_id, part.evidence_record_id
            ));
            continue;
        };
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

    for (resource_id, allocations) in resource_allocations {
        if let Some(resource) = resources.get(resource_id.as_str()) {
            let mut boundaries = Vec::new();
            for allocation in allocations {
                boundaries.push((allocation.start, true, allocation.units, allocation.event_id.clone()));
                boundaries.push((allocation.end, false, allocation.units, allocation.event_id));
            }
            boundaries.sort_by(|a, b| {
                a.0.partial_cmp(&b.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.1.cmp(&b.1))
            });
            let mut used = 0u32;
            for (time, is_start, units, event_id) in boundaries {
                if is_start {
                    used = used.saturating_add(units);
                    if used > resource.available_units {
                        errors.push(format!(
                            "resource contention on {} at hour {} while serving {}: {} units demanded, {} available",
                            resource_id, time, event_id, used, resource.available_units
                        ));
                    }
                } else {
                    used = used.saturating_sub(units);
                }
            }
        }
    }

    // Keep unused catalogue fields intentionally visible in the typed contract:
    // their presence lets the schema and future validators retain asset/part class identity.
    for asset in &case.assets {
        if asset.asset_class.trim().is_empty() || asset.variant_id.trim().is_empty() {
            errors.push(format!("{} has incomplete class/variant identity", asset.asset_id));
        }
    }
    for part in &case.parts {
        if part.part_class.trim().is_empty() || part.variant_id.trim().is_empty() {
            errors.push(format!("{} has incomplete class/variant identity", part.part_id));
        }
    }
    for resource in &case.resources {
        if resource.resource_type.trim().is_empty() {
            errors.push(format!("{} has empty resourceType", resource.resource_id));
        }
    }
    for route in &case.logistics_routes {
        if route.transit_hours <= 0.0 || route.max_cargo_kg <= 0.0 {
            errors.push(format!("{} has invalid logistics parameters", route.route_id));
        }
    }

    errors.sort();
    errors
}

fn parse_case(input: &str) -> Result<Case, serde_json::Error> {
    serde_json::from_str(input)
}

fn run(input: &str) -> Result<(), Vec<String>> {
    let case = parse_case(input).map_err(|error| vec![format!("JSON decode error: {error}")])?;
    let errors = validate(&case);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn main() {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: grid-asset-recovery-validator <case.json>");
        process::exit(2);
    };
    let input = fs::read_to_string(&path).unwrap_or_else(|error| {
        eprintln!("cannot read {path}: {error}");
        process::exit(2);
    });
    match run(&input) {
        Ok(()) => println!("PASS: grid asset recovery case is semantically coherent"),
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
    use serde_json::Value;

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
    fn accepts_hand_calculable_two_event_fixture() {
        let errors = run(FIXTURE).expect("fixture should pass");
        assert_eq!(errors, ());
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
        value["events"][1]["selectedPlan"]["partSource"] = serde_json::json!({
            "kind": "depot_stock",
            "depotId": "DEPOT-CENTRAL",
            "partId": "SPARE-A",
            "quantity": 1
        });
        value["events"][1]["selectedPlan"]["logisticsRouteId"] = Value::String("ROUTE-CENTRAL-NORTH".to_string());
        value["events"][1]["selectedPlan"]["transportMassKg"] = Value::from(90000);
        value["events"][1]["selectedPlan"]["tasks"].as_array_mut().unwrap()
            .insert(3, serde_json::json!({"stage":"transport","durationHours":12}));
        assert!(check(&value).iter().any(|x| x.contains("spare double-allocation")));
    }

    #[test]
    fn rejects_reserved_inventory() {
        let mut value = fixture();
        value["depots"][0]["inventory"][0]["quantityReserved"] = Value::from(1);
        assert!(check(&value).iter().any(|x| x.contains("only 0 serviceable unreserved")));
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
        value["events"][0]["selectedPlan"]["tasks"].as_array_mut().unwrap()
            .retain(|task| task["stage"] != "test_and_commission");
        assert!(check(&value).iter().any(|x| x.contains("missing required stage test_and_commission")));
    }

    #[test]
    fn rejects_overlapping_single_crew_assignments() {
        let mut value = fixture();
        value["events"][1]["startHour"] = Value::from(15);
        value["events"][1]["selectedPlan"]["tasks"][2]["durationHours"] = Value::from(1);
        value["events"][1]["selectedPlan"]["claimedRestorationHours"] = Value::from(24);
        value["events"][1]["selectedPlan"]["expectedUnservedEnergyMwh"] = Value::from(1200);
        assert!(check(&value).iter().any(|x| x.contains("resource contention")));
    }

    #[test]
    fn rejects_synthetic_evidence_promotion() {
        let mut value = fixture();
        value["caseMaturity"] = Value::String("field_observed".to_string());
        assert!(check(&value).iter().any(|x| x.contains("lacks site-qualified")));
    }

    #[test]
    fn rejects_incorrect_unserved_energy_arithmetic() {
        let mut value = fixture();
        value["events"][0]["selectedPlan"]["expectedUnservedEnergyMwh"] = Value::from(1);
        assert!(check(&value).iter().any(|x| x.contains("expected unserved energy")));
    }
}
