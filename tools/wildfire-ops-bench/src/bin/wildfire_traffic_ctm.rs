use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::process::ExitCode;

const VERSION: &str = "0.1.0";
const CASE_IDS: [&str; 4] = [
    "FREEFLOW_MISRANKS_BOTTLENECK_001",
    "CLOSURE_DURING_DEPARTURE_WAVE_002",
    "SHELTER_RECEIVING_BOTTLENECK_003",
    "ACCESSIBLE_SHELTER_HARD_GATE_004",
];

type Check<T> = Result<T, String>;

fn unsigned(value: &Value, key: &str, at: &str) -> Check<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{at}.{key}: expected non-negative integer"))
}

fn list<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a [Value]> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{at}.{key}: expected array"))
}

fn text<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{at}.{key}: expected string"))
}

fn path_id(path: Option<&Value>) -> Option<&str> {
    path.and_then(|value| value.get("id").and_then(Value::as_str))
}

fn static_choice<'a>(paths: &'a [Value]) -> Option<&'a Value> {
    paths.iter().min_by(|left, right| {
        left.get("cells")
            .and_then(Value::as_u64)
            .unwrap_or(u64::MAX)
            .cmp(
                &right
                    .get("cells")
                    .and_then(Value::as_u64)
                    .unwrap_or(u64::MAX),
            )
            .then_with(|| {
                left.get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .cmp(right.get("id").and_then(Value::as_str).unwrap_or(""))
            })
    })
}

fn percentile(steps: &[u64], numerator: u64, denominator: u64) -> Option<u64> {
    if steps.is_empty() {
        return None;
    }
    let rank = (steps.len() as u64 * numerator)
        .div_ceil(denominator)
        .max(1);
    steps.get((rank - 1) as usize).copied()
}

fn simulate_path(demand: u64, path: &Value, max_steps: u64) -> Check<Value> {
    let at = text(path, "id", "path")?;
    let cells = usize::try_from(unsigned(path, "cells", at)?)
        .map_err(|_| format!("{at}.cells: exceeds addressable size"))?;
    let capacity = unsigned(path, "capacity_per_step", at)?;
    let jam = unsigned(path, "jam_occupancy", at)?;
    let sink_capacity = unsigned(path, "sink_capacity_per_step", at)?;
    let closed_from_step = path.get("closed_from_step").and_then(Value::as_u64);

    if cells == 0 || capacity == 0 || jam == 0 || sink_capacity == 0 {
        return Err(format!(
            "{at}: cells, capacity, jam occupancy, and sink capacity must be positive"
        ));
    }
    if capacity > jam {
        return Err(format!(
            "{at}: cell capacity cannot exceed jam occupancy in this discretization"
        ));
    }
    if closed_from_step == Some(0) {
        return Err(format!(
            "{at}: closed_from_step is 1-based; use null for open"
        ));
    }

    let mut occupancy = vec![0u64; cells];
    let mut remaining = demand;
    let mut arrived = 0u64;
    let mut arrival_steps = Vec::<u64>::new();
    let mut queue_vehicle_steps = 0u64;
    let mut completion_step = None;
    let mut max_cell_occupancy = 0u64;
    let mut max_observed_flow = 0u64;
    let mut mass_balance_ok = true;
    let mut closed = false;

    for step in 1..=max_steps {
        if closed_from_step.is_some_and(|closed_step| step >= closed_step) {
            closed = true;
            break;
        }

        let mut outflow = vec![0u64; cells];
        for cell in 0..cells {
            let sending = occupancy[cell].min(capacity);
            let receiving = if cell + 1 < cells {
                jam.saturating_sub(occupancy[cell + 1]).min(capacity)
            } else {
                sink_capacity
            };
            outflow[cell] = sending.min(receiving);
            max_observed_flow = max_observed_flow.max(outflow[cell]);
        }

        let source_flow = remaining
            .min(capacity)
            .min(jam.saturating_sub(occupancy[0]));
        max_observed_flow = max_observed_flow.max(source_flow);
        let mut next = occupancy.clone();
        next[0] = occupancy[0] + source_flow - outflow[0];
        for cell in 1..cells {
            next[cell] = occupancy[cell] + outflow[cell - 1] - outflow[cell];
        }

        let next_remaining = remaining - source_flow;
        let exits = outflow[cells - 1];
        let next_arrived = arrived + exits;
        for _ in 0..exits {
            arrival_steps.push(step);
        }

        let next_system_total = next_remaining + next.iter().sum::<u64>() + next_arrived;
        mass_balance_ok &= next_system_total == demand;
        if next.iter().any(|value| *value > jam) {
            return Err(format!(
                "{at}: cell occupancy exceeded jam storage at step {step}"
            ));
        }
        max_cell_occupancy = max_cell_occupancy.max(next.iter().copied().max().unwrap_or(0));
        queue_vehicle_steps += next_remaining + next.iter().sum::<u64>();
        remaining = next_remaining;
        arrived = next_arrived;
        occupancy = next;

        if remaining == 0 && occupancy.iter().all(|value| *value == 0) {
            completion_step = Some(step);
            break;
        }
    }

    let stranded = if completion_step.is_some() {
        0
    } else {
        remaining + occupancy.iter().sum::<u64>()
    };
    if !mass_balance_ok {
        return Err(format!("{at}: vehicle mass conservation failed"));
    }

    Ok(json!({
        "demand_vehicles": demand,
        "arrived_vehicles": arrived,
        "unserved_or_stranded_vehicles": stranded,
        "completed": completion_step.is_some() && arrived == demand,
        "clearance_step": completion_step,
        "p50_arrival_step": percentile(&arrival_steps, 50, 100),
        "p90_arrival_step": percentile(&arrival_steps, 90, 100),
        "p95_arrival_step": percentile(&arrival_steps, 95, 100),
        "queue_vehicle_steps": queue_vehicle_steps,
        "max_cell_occupancy": max_cell_occupancy,
        "jam_occupancy_limit": jam,
        "max_observed_flow_per_step": max_observed_flow,
        "capacity_per_step": capacity,
        "mass_balance_ok": mass_balance_ok,
        "closure_enforced": closed,
        "closed_from_step": closed_from_step
    }))
}

fn simulate_case(case: &Value) -> Check<Value> {
    let case_id = text(case, "case_id", "case")?;
    let demand = unsigned(case, "demand_vehicles", case_id)?;
    let assisted = unsigned(case, "assisted_vehicle_demand", case_id)?;
    let max_steps = unsigned(case, "max_steps", case_id)?;
    let paths = list(case, "paths", case_id)?;
    if demand == 0 || max_steps == 0 || paths.is_empty() {
        return Err(format!(
            "{case_id}: demand, max_steps, and path count must be positive"
        ));
    }
    if assisted > demand {
        return Err(format!(
            "{case_id}: assisted demand cannot exceed total demand"
        ));
    }

    let mut ids = std::collections::BTreeSet::new();
    let mut details = Vec::<Value>::new();
    for path in paths {
        let id = text(path, "id", case_id)?;
        if !ids.insert(id.to_owned()) {
            return Err(format!("{case_id}: duplicate path id {id}"));
        }
        let cells = unsigned(path, "cells", id)?;
        let simulation = simulate_path(demand, path, max_steps)?;
        let total_capacity = unsigned(path, "shelter_capacity", id)?;
        let accessible_capacity = unsigned(path, "accessible_capacity", id)?;
        let shelter_ok = total_capacity >= demand && accessible_capacity >= assisted;
        let operationally_feasible = simulation["completed"].as_bool() == Some(true) && shelter_ok;
        details.push(json!({
            "path_id": id,
            "free_flow_steps": cells,
            "shelter_capacity_feasible": total_capacity >= demand,
            "accessible_capacity_feasible": accessible_capacity >= assisted,
            "operationally_feasible": operationally_feasible,
            "simulation": simulation
        }));
    }

    let static_path = static_choice(paths);
    let static_id = path_id(static_path);
    let dynamic = details
        .iter()
        .filter(|detail| detail["operationally_feasible"].as_bool() == Some(true))
        .min_by(|left, right| {
            left["simulation"]["clearance_step"]
                .as_u64()
                .unwrap_or(u64::MAX)
                .cmp(
                    &right["simulation"]["clearance_step"]
                        .as_u64()
                        .unwrap_or(u64::MAX),
                )
                .then_with(|| {
                    left["path_id"]
                        .as_str()
                        .unwrap_or("")
                        .cmp(right["path_id"].as_str().unwrap_or(""))
                })
        });
    let dynamic_id = dynamic.and_then(|detail| detail["path_id"].as_str());
    let expected_id = case
        .get("oracle_truth")
        .and_then(|oracle| oracle.get("expected_dynamic_path_id"))
        .and_then(Value::as_str);
    let static_detail = details
        .iter()
        .find(|detail| detail["path_id"].as_str() == static_id);
    let dynamic_clearance =
        dynamic.and_then(|detail| detail["simulation"]["clearance_step"].as_u64());
    let static_feasible = static_detail
        .is_some_and(|detail| detail["operationally_feasible"].as_bool() == Some(true));

    Ok(json!({
        "case_id": case_id,
        "status": "pass",
        "demand_vehicles": demand,
        "assisted_vehicle_demand": assisted,
        "B1_static_free_flow": {
            "selected_path_id": static_id,
            "free_flow_steps": static_path.and_then(|path| path["cells"].as_u64()),
            "selected_operationally_feasible": static_feasible,
            "matches_expected_dynamic_path": static_id == expected_id
        },
        "B2_capacity_aware_cell_model": {
            "selected_path_id": dynamic_id,
            "clearance_step": dynamic_clearance,
            "abstained": dynamic.is_none(),
            "matches_expected_dynamic_path": dynamic_id == expected_id
        },
        "paths": details
    }))
}

fn validate_fixture(fixture: &Value) -> Check<Vec<Value>> {
    if fixture.get("schema_version").and_then(Value::as_str) != Some(VERSION)
        || fixture.get("benchmark_id").and_then(Value::as_str) != Some("WILDFIRE-TRAFFIC-001")
        || fixture.get("classification").and_then(Value::as_str) != Some("synthetic_only")
    {
        return Err("fixture metadata mismatch".to_owned());
    }
    let cases = list(fixture, "cases", "fixture")?;
    if cases.len() != CASE_IDS.len() {
        return Err(format!(
            "fixture must contain exactly {} cases",
            CASE_IDS.len()
        ));
    }
    let mut results = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let actual_id = text(case, "case_id", "case")?;
        if actual_id != CASE_IDS[index] {
            return Err(format!(
                "case inventory/order mismatch at index {index}: {actual_id}"
            ));
        }
        results.push(simulate_case(case)?);
    }
    Ok(results)
}

fn scorecard(cases: &[Value]) -> Value {
    let mut static_selected = 0usize;
    let mut static_infeasible = 0usize;
    let mut static_matches = 0usize;
    let mut dynamic_selected = 0usize;
    let mut dynamic_abstentions = 0usize;
    let mut dynamic_matches = 0usize;
    for case in cases {
        if !case["B1_static_free_flow"]["selected_path_id"].is_null() {
            static_selected += 1;
        }
        if case["B1_static_free_flow"]["selected_operationally_feasible"].as_bool() != Some(true) {
            static_infeasible += 1;
        }
        if case["B1_static_free_flow"]["matches_expected_dynamic_path"].as_bool() == Some(true) {
            static_matches += 1;
        }
        if case["B2_capacity_aware_cell_model"]["selected_path_id"].is_null() {
            dynamic_abstentions += 1;
        } else {
            dynamic_selected += 1;
        }
        if case["B2_capacity_aware_cell_model"]["matches_expected_dynamic_path"].as_bool()
            == Some(true)
        {
            dynamic_matches += 1;
        }
    }
    json!({
        "case_count": cases.len(),
        "B1_static_free_flow": {
            "selected_count": static_selected,
            "selected_infeasible_count": static_infeasible,
            "expected_dynamic_choice_matches": static_matches,
            "expected_dynamic_choice_mismatches": cases.len() - static_matches
        },
        "B2_capacity_aware_cell_model": {
            "selected_count": dynamic_selected,
            "abstention_count": dynamic_abstentions,
            "expected_dynamic_choice_matches": dynamic_matches,
            "expected_dynamic_choice_mismatches": cases.len() - dynamic_matches
        },
        "claim_boundary": "synthetic route-path microbenchmarks only; not field validated"
    })
}

fn run() -> Check<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 || args[2] != "--source-commit" {
        return Err(
            "usage: wildfire-traffic-ctm <fixture.json> --source-commit <40-hex-sha>".to_owned(),
        );
    }
    let source_commit = &args[3];
    if source_commit.len() != 40 || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character hex SHA".to_owned());
    }
    let bytes = fs::read(&args[1]).map_err(|error| format!("read fixture: {error}"))?;
    let fixture: Value =
        serde_json::from_slice(&bytes).map_err(|error| format!("parse fixture: {error}"))?;
    let cases = validate_fixture(&fixture)?;
    let summary = scorecard(&cases);
    let receipt = json!({
        "result": "pass",
        "validator_version": VERSION,
        "benchmark_id": "WILDFIRE-TRAFFIC-001",
        "scenario_count": cases.len(),
        "fixture_sha256": format!("{:x}", Sha256::digest(&bytes)),
        "source_commit": source_commit,
        "scorecard": summary,
        "cases": cases
    });
    println!(
        "{}",
        serde_json::to_string(&receipt).map_err(|error| format!("serialize receipt: {error}"))?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("wildfire traffic benchmark failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../benchmarks/fixtures/WILDFIRE_TRAFFIC_CTM_V0_1.json"
        ))
        .expect("synthetic fixture parses")
    }

    #[test]
    fn all_paths_conserve_vehicle_mass_and_respect_capacity() {
        let results = validate_fixture(&fixture()).expect("fixture validates");
        assert_eq!(results.len(), 3);
        for case in results {
            for path in case["paths"].as_array().expect("paths") {
                let sim = &path["simulation"];
                assert_eq!(sim["mass_balance_ok"], true);
                assert_eq!(
                    sim["unserved_or_stranded_vehicles"].as_u64().unwrap_or(0)
                        + sim["arrived_vehicles"].as_u64().unwrap_or(0),
                    sim["demand_vehicles"].as_u64().unwrap_or(0)
                );
                assert!(
                    sim["max_cell_occupancy"].as_u64().unwrap_or(0)
                        <= sim["jam_occupancy_limit"].as_u64().unwrap_or(0)
                );
                assert!(
                    sim["max_observed_flow_per_step"]
                        .as_u64()
                        .unwrap_or(u64::MAX)
                        <= sim["capacity_per_step"].as_u64().unwrap_or(0)
                );
            }
        }
    }

    #[test]
    fn congestion_changes_all_three_static_choices() {
        let results = validate_fixture(&fixture()).expect("fixture validates");
        assert_eq!(
            results[0]["B1_static_free_flow"]["selected_path_id"],
            "FAST_LOW_CAPACITY"
        );
        assert_eq!(
            results[0]["B2_capacity_aware_cell_model"]["selected_path_id"],
            "SLOW_HIGH_CAPACITY"
        );
        assert_eq!(
            results[1]["B1_static_free_flow"]["selected_path_id"],
            "SHORT_PATH_CLOSES"
        );
        assert_eq!(
            results[1]["B2_capacity_aware_cell_model"]["selected_path_id"],
            "OPEN_ALTERNATIVE"
        );
        assert_eq!(
            results[2]["B1_static_free_flow"]["selected_path_id"],
            "SHORT_SHELTER_LIMITED"
        );
        assert_eq!(
            results[2]["B2_capacity_aware_cell_model"]["selected_path_id"],
            "THROUGHPUT_SHELTER"
        );
        assert_eq!(
            results[3]["B1_static_free_flow"]["selected_path_id"],
            "FAST_INACCESSIBLE_SHELTER"
        );
        assert_eq!(
            results[3]["B1_static_free_flow"]["selected_operationally_feasible"],
            false
        );
        assert_eq!(
            results[3]["B2_capacity_aware_cell_model"]["selected_path_id"],
            "ACCESSIBLE_ALTERNATIVE"
        );
    }

    #[test]
    fn closure_stops_flow_and_explicitly_strands_unserved_vehicles() {
        let results = validate_fixture(&fixture()).expect("fixture validates");
        let closing = &results[1]["paths"][0]["simulation"];
        assert_eq!(closing["closure_enforced"], true);
        assert_eq!(closing["completed"], false);
        assert!(
            closing["unserved_or_stranded_vehicles"]
                .as_u64()
                .unwrap_or(0)
                > 0
        );
    }

    #[test]
    fn accessible_shelter_capacity_is_a_hard_gate() {
        let results = validate_fixture(&fixture()).expect("fixture validates");
        assert_eq!(
            results[3]["paths"][0]["accessible_capacity_feasible"],
            false
        );
        assert_eq!(
            results[3]["B2_capacity_aware_cell_model"]["selected_path_id"],
            "ACCESSIBLE_ALTERNATIVE"
        );
    }

    #[test]
    fn no_accessible_capacity_causes_explicit_abstention() {
        let mut fixture = fixture();
        for path in fixture["cases"][3]["paths"]
            .as_array_mut()
            .expect("paths")
        {
            path["accessible_capacity"] = serde_json::json!(1);
        }
        let results = validate_fixture(&fixture).expect("synthetic constraints remain valid");
        assert_eq!(
            results[3]["B2_capacity_aware_cell_model"]["selected_path_id"],
            Value::Null
        );
        assert_eq!(
            results[3]["B2_capacity_aware_cell_model"]["abstained"],
            true
        );
    }

    #[test]
    fn zero_road_capacity_is_rejected() {
        let mut fixture = fixture();
        fixture["cases"][0]["paths"][0]["capacity_per_step"] = serde_json::json!(0);
        assert!(validate_fixture(&fixture).is_err());
    }

    #[test]
    fn input_path_order_does_not_change_winner() {
        let mut fixture = fixture();
        let original = validate_fixture(&fixture).expect("original validates");
        for case in fixture["cases"].as_array_mut().expect("cases") {
            case["paths"].as_array_mut().expect("paths").reverse();
        }
        let reordered = validate_fixture(&fixture).expect("reordered validates");
        for (before, after) in original.iter().zip(reordered.iter()) {
            assert_eq!(
                before["B1_static_free_flow"]["selected_path_id"],
                after["B1_static_free_flow"]["selected_path_id"]
            );
            assert_eq!(
                before["B2_capacity_aware_cell_model"]["selected_path_id"],
                after["B2_capacity_aware_cell_model"]["selected_path_id"]
            );
        }
    }
}
