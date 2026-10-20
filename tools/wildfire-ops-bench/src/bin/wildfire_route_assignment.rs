use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

#[allow(dead_code)]
#[path = "wildfire_network_flow.rs"]
mod network;

const VERSION: &str = "0.1.0";
const CASE_IDS: [&str; 4] = [
    "SHARED_SHORT_BOTTLENECK_001",
    "CLOSURE_MAKES_FASTEST_PATH_UNUSABLE_002",
    "SHELTER_TOTAL_AND_ACCESSIBILITY_CAPACITY_003",
    "SHORT_ROUTE_ALREADY_OPTIMAL_004",
];
const MAX_UNITS: usize = 16;
const MAX_ASSIGNMENTS: u64 = 100_000;

type Check<T> = Result<T, String>;

#[derive(Clone, Debug)]
struct Route {
    id: String,
    sink_id: String,
    edge_ids: Vec<String>,
    free_flow_cost: u64,
}

#[derive(Clone, Debug)]
struct Demand {
    id: String,
    origin: String,
    vehicles: u64,
    accessible: u64,
    departure_step: u64,
    routes: Vec<Route>,
}

#[derive(Clone, Debug)]
struct Unit {
    demand_index: usize,
    vehicle_index: u64,
    accessible: bool,
    options: Vec<Route>,
}

#[derive(Clone, Debug)]
struct Evaluation {
    assignment: Vec<usize>,
    objective: Objective,
    simulation: Value,
    enumeration_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Objective {
    unserved: u64,
    p95_arrival: u64,
    queue_vehicle_steps: u64,
    free_flow_cost: u64,
    route_vector: Vec<String>,
}

impl Objective {
    fn json(&self) -> Value {
        json!({
            "unserved_or_stranded_vehicles": self.unserved,
            "p95_arrival_step": if self.p95_arrival == u64::MAX { Value::Null } else { json!(self.p95_arrival) },
            "queue_vehicle_steps": self.queue_vehicle_steps,
            "free_flow_travel_steps": self.free_flow_cost,
            "lexical_route_id_vector": self.route_vector
        })
    }
}

fn compare_objective(a: &Objective, b: &Objective) -> std::cmp::Ordering {
    a.unserved
        .cmp(&b.unserved)
        .then_with(|| a.p95_arrival.cmp(&b.p95_arrival))
        .then_with(|| a.queue_vehicle_steps.cmp(&b.queue_vehicle_steps))
        .then_with(|| a.free_flow_cost.cmp(&b.free_flow_cost))
        .then_with(|| a.route_vector.cmp(&b.route_vector))
}

fn text<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{at}.{key}: expected string"))
}

fn array<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a [Value]> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{at}.{key}: expected array"))
}

fn uint(value: &Value, key: &str, at: &str) -> Check<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{at}.{key}: expected non-negative integer"))
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn route_cost(route: &[String], edge_costs: &BTreeMap<String, u64>, at: &str) -> Check<u64> {
    route.iter().try_fold(0u64, |sum, edge_id| {
        let cost = edge_costs
            .get(edge_id)
            .ok_or_else(|| format!("{at}: unknown route edge {edge_id}"))?;
        sum.checked_add(*cost)
            .ok_or_else(|| format!("{at}: route cost overflow"))
    })
}

fn parse_case(case: &Value) -> Check<(Vec<Demand>, Vec<Unit>)> {
    let case_id = text(case, "case_id", "case")?;
    let edge_values = array(case, "edges", case_id)?;
    let sink_values = array(case, "sinks", case_id)?;
    let demand_values = array(case, "demands", case_id)?;
    if edge_values.is_empty() || sink_values.is_empty() || demand_values.is_empty() {
        return Err(format!(
            "{case_id}: edges, sinks and demands must be non-empty"
        ));
    }

    let mut edge_costs = BTreeMap::<String, u64>::new();
    let mut edge_connectivity = BTreeMap::<String, (String, String)>::new();
    for edge in edge_values {
        let id = text(edge, "id", case_id)?.to_owned();
        let from = text(edge, "from", &id)?.to_owned();
        let to = text(edge, "to", &id)?.to_owned();
        let cost = uint(edge, "travel_time_steps", &id)?;
        if id.is_empty() || from == to || cost == 0 || edge_costs.insert(id.clone(), cost).is_some()
        {
            return Err(format!("{case_id}: invalid or duplicate edge {id}"));
        }
        edge_connectivity.insert(id, (from, to));
    }

    let mut sink_nodes = BTreeMap::<String, String>::new();
    for sink in sink_values {
        let id = text(sink, "id", case_id)?.to_owned();
        let node = text(sink, "node_id", &id)?.to_owned();
        if sink_nodes.insert(id.clone(), node).is_some() {
            return Err(format!("{case_id}: duplicate sink {id}"));
        }
    }

    let mut raw_demands: Vec<&Value> = demand_values.iter().collect();
    raw_demands.sort_by_key(|demand| demand.get("id").and_then(Value::as_str).unwrap_or(""));
    let mut demands = Vec::new();
    let mut ids = BTreeSet::<String>::new();
    let mut unit_count = 0usize;
    let mut combination_count = 1u64;

    for raw in raw_demands {
        let id = text(raw, "id", case_id)?.to_owned();
        let origin = text(raw, "origin_node", &id)?.to_owned();
        let vehicles = uint(raw, "vehicles", &id)?;
        let accessible = uint(raw, "accessible_vehicles", &id)?;
        let departure_step = uint(raw, "departure_step", &id)?;
        if id.is_empty()
            || !ids.insert(id.clone())
            || vehicles == 0
            || accessible > vehicles
            || departure_step == 0
        {
            return Err(format!("{case_id}: invalid demand stream {id}"));
        }
        unit_count = unit_count
            .checked_add(vehicles as usize)
            .ok_or_else(|| format!("{case_id}: unit count overflow"))?;
        if unit_count > MAX_UNITS {
            return Err(format!(
                "{case_id}: exact route enumeration limited to {MAX_UNITS} vehicles"
            ));
        }

        let raw_routes = array(raw, "candidate_routes", &id)?;
        if raw_routes.is_empty() {
            return Err(format!(
                "{case_id}.{id}: candidate_routes must be non-empty"
            ));
        }
        let mut routes = Vec::<Route>::new();
        let mut route_ids = BTreeSet::new();
        for raw_route in raw_routes {
            let route_id = text(raw_route, "id", &id)?.to_owned();
            let sink_id = text(raw_route, "destination_sink_id", &route_id)?.to_owned();
            let edge_array = array(raw_route, "route_edge_ids", &route_id)?;
            let edge_ids: Vec<String> = edge_array
                .iter()
                .map(|edge| {
                    edge.as_str().map(str::to_owned).ok_or_else(|| {
                        format!("{case_id}.{route_id}: route edge ID must be a string")
                    })
                })
                .collect::<Check<Vec<_>>>()?;
            if route_id.is_empty() || !route_ids.insert(route_id.clone()) || edge_ids.is_empty() {
                return Err(format!(
                    "{case_id}.{id}: empty/duplicate route or empty route path"
                ));
            }
            let sink_node = sink_nodes
                .get(&sink_id)
                .ok_or_else(|| format!("{case_id}.{route_id}: unknown sink {sink_id}"))?;
            if edge_connectivity
                .get(&edge_ids[0])
                .map(|pair| pair.0.as_str())
                != Some(origin.as_str())
            {
                return Err(format!(
                    "{case_id}.{route_id}: first edge does not start at origin {origin}"
                ));
            }
            for pair in edge_ids.windows(2) {
                let end = edge_connectivity
                    .get(&pair[0])
                    .map(|edge| edge.1.as_str())
                    .ok_or_else(|| format!("{case_id}.{route_id}: unknown edge {}", pair[0]))?;
                let start = edge_connectivity
                    .get(&pair[1])
                    .map(|edge| edge.0.as_str())
                    .ok_or_else(|| format!("{case_id}.{route_id}: unknown edge {}", pair[1]))?;
                if end != start {
                    return Err(format!("{case_id}.{route_id}: disconnected route"));
                }
            }
            if edge_connectivity
                .get(edge_ids.last().expect("non-empty"))
                .map(|edge| edge.1.as_str())
                != Some(sink_node.as_str())
            {
                return Err(format!(
                    "{case_id}.{route_id}: final edge does not reach sink {sink_id}"
                ));
            }
            let cost = route_cost(&edge_ids, &edge_costs, &format!("{case_id}.{route_id}"))?;
            routes.push(Route {
                id: route_id,
                sink_id,
                edge_ids,
                free_flow_cost: cost,
            });
        }
        routes.sort_by(|a, b| a.id.cmp(&b.id));
        for _ in 0..vehicles {
            combination_count = combination_count
                .checked_mul(routes.len() as u64)
                .ok_or_else(|| format!("{case_id}: candidate combination count overflow"))?;
            if combination_count > MAX_ASSIGNMENTS {
                return Err(format!(
                    "{case_id}: joint assignment search has {combination_count} combinations, limit is {MAX_ASSIGNMENTS}"
                ));
            }
        }
        demands.push(Demand {
            id,
            origin,
            vehicles,
            accessible,
            departure_step,
            routes,
        });
    }

    let mut units = Vec::with_capacity(unit_count);
    for (demand_index, demand) in demands.iter().enumerate() {
        for vehicle_index in 0..demand.vehicles {
            units.push(Unit {
                demand_index,
                vehicle_index,
                accessible: vehicle_index < demand.accessible,
                options: demand.routes.clone(),
            });
        }
    }
    Ok((demands, units))
}

fn evaluate_assignment(
    case: &Value,
    demands: &[Demand],
    units: &[Unit],
    assignment: &[usize],
) -> Check<Evaluation> {
    if assignment.len() != units.len() {
        return Err("assignment length does not match vehicle unit count".to_owned());
    }
    let mut sim_case = case.clone();
    let mut expanded = Vec::<Value>::new();
    let mut route_vector = Vec::<String>::new();
    let mut free_flow_cost = 0u64;
    for (unit, route_index) in units.iter().zip(assignment.iter()) {
        let demand = &demands[unit.demand_index];
        let route = unit
            .options
            .get(*route_index)
            .ok_or_else(|| format!("{}: assignment route index out of range", demand.id))?;
        free_flow_cost = free_flow_cost
            .checked_add(route.free_flow_cost)
            .ok_or_else(|| "free-flow cost overflow".to_owned())?;
        route_vector.push(route.id.clone());
        expanded.push(json!({
            "id": format!("{}-V{:05}", demand.id, unit.vehicle_index),
            "origin_node": demand.origin,
            "destination_sink_id": route.sink_id,
            "vehicles": 1,
            "accessible_vehicles": if unit.accessible { 1 } else { 0 },
            "departure_step": demand.departure_step,
            "route_edge_ids": route.edge_ids
        }));
    }
    expanded.sort_by(|a, b| {
        a.get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(b.get("id").and_then(Value::as_str).unwrap_or(""))
    });
    sim_case["demands"] = json!(expanded);
    let simulation = network::simulate_case(&sim_case)?;
    let unserved = uint(&simulation, "unserved_or_stranded_vehicles", "simulation")?;
    let p95 = simulation
        .get("p95_arrival_step")
        .and_then(Value::as_u64)
        .unwrap_or(u64::MAX);
    let queue_vehicle_steps = uint(&simulation, "queue_vehicle_steps", "simulation")?;
    let objective = Objective {
        unserved,
        p95_arrival: p95,
        queue_vehicle_steps,
        free_flow_cost,
        route_vector: route_vector.clone(),
    };
    Ok(Evaluation {
        assignment: assignment.to_vec(),
        objective,
        simulation,
        enumeration_count: 1,
    })
}

fn best_assignment(
    case: &Value,
    demands: &[Demand],
    units: &[Unit],
    exhaustive: bool,
) -> Check<Evaluation> {
    if !exhaustive {
        let assignment: Vec<usize> = units
            .iter()
            .map(|unit| {
                unit.options
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        a.free_flow_cost
                            .cmp(&b.free_flow_cost)
                            .then_with(|| a.id.cmp(&b.id))
                    })
                    .map(|(index, _)| index)
                    .expect("route options validated")
            })
            .collect();
        return evaluate_assignment(case, demands, units, &assignment);
    }

    let combinations = units.iter().try_fold(1u64, |total, unit| {
        total
            .checked_mul(unit.options.len() as u64)
            .ok_or_else(|| "joint assignment count overflow".to_owned())
    })?;
    if combinations > MAX_ASSIGNMENTS {
        return Err(format!(
            "exact assignment count {combinations} exceeds budget {MAX_ASSIGNMENTS}"
        ));
    }
    let mut best: Option<Evaluation> = None;
    for encoded in 0..combinations {
        let mut remainder = encoded;
        let mut assignment = Vec::with_capacity(units.len());
        for unit in units {
            let radix = unit.options.len() as u64;
            assignment.push((remainder % radix) as usize);
            remainder /= radix;
        }
        let candidate = evaluate_assignment(case, demands, units, &assignment)?;
        if best.as_ref().is_none_or(|current| {
            compare_objective(&candidate.objective, &current.objective).is_lt()
        }) {
            best = Some(candidate);
        }
    }
    let mut best =
        best.ok_or_else(|| "joint assignment enumeration returned no candidates".to_owned())?;
    best.enumeration_count = combinations;
    Ok(best)
}

fn assignments_by_demand(demands: &[Demand], units: &[Unit], assignment: &[usize]) -> Value {
    let mut per_demand = BTreeMap::<String, BTreeMap<String, u64>>::new();
    for (unit, route_index) in units.iter().zip(assignment) {
        let route_id = &unit.options[*route_index].id;
        *per_demand
            .entry(demands[unit.demand_index].id.clone())
            .or_default()
            .entry(route_id.clone())
            .or_default() += 1;
    }
    json!(
        per_demand
            .into_iter()
            .map(|(id, counts)| (id, json!(counts)))
            .collect::<BTreeMap<_, _>>()
    )
}

fn evaluation_json(evaluation: &Evaluation, demands: &[Demand], units: &[Unit]) -> Value {
    json!({
        "objective": evaluation.objective.json(),
        "assignment_by_demand": assignments_by_demand(demands, units, &evaluation.assignment),
        "assignment_route_id_vector": evaluation.objective.route_vector,
        "served_vehicles": evaluation.simulation["served_vehicles"],
        "unserved_or_stranded_vehicles": evaluation.simulation["unserved_or_stranded_vehicles"],
        "clearance_step": evaluation.simulation["clearance_step"],
        "p50_arrival_step": evaluation.simulation["p50_arrival_step"],
        "p90_arrival_step": evaluation.simulation["p90_arrival_step"],
        "p95_arrival_step": evaluation.simulation["p95_arrival_step"],
        "queue_vehicle_steps": evaluation.simulation["queue_vehicle_steps"],
        "mass_balance_ok": evaluation.simulation["mass_balance_ok"],
        "capacity_checks_ok": evaluation.simulation["capacity_checks_ok"],
        "sink_service": evaluation.simulation["served_by_sink"],
        "enumeration_count": evaluation.enumeration_count
    })
}

fn evaluate_case(case: &Value, _index: usize) -> Check<Value> {
    let case_id = text(case, "case_id", "case")?;
    let (demands, units) = parse_case(case)?;
    let baseline = best_assignment(case, &demands, &units, false)?;
    let oracle = best_assignment(case, &demands, &units, true)?;
    let baseline_json = evaluation_json(&baseline, &demands, &units);
    let oracle_json = evaluation_json(&oracle, &demands, &units);
    let p95_improvement = match (
        baseline
            .objective
            .p95_arrival
            .checked_sub(oracle.objective.p95_arrival),
        baseline.objective.p95_arrival,
        oracle.objective.p95_arrival,
    ) {
        (_, u64::MAX, value) if value != u64::MAX => true,
        (Some(delta), _, _) if delta > 0 => true,
        _ => false,
    };
    let unserved_improvement = baseline.objective.unserved > oracle.objective.unserved;
    let identical_objective = baseline.objective == oracle.objective;
    Ok(json!({
        "case_id": case_id,
        "status": "pass",
        "unit_count": units.len(),
        "joint_assignment_combinations": oracle.enumeration_count,
        "B1_independent_shortest_path": baseline_json,
        "B2_exact_joint_assignment": oracle_json,
        "baseline_strictly_worse_p95": p95_improvement,
        "baseline_strictly_worse_unserved": unserved_improvement,
        "objectives_identical": identical_objective,
        "claim_boundary": "tiny synthetic exact enumeration using the benchmark network simulator; not a scalable optimizer or matched SUMO result"
    }))
}

fn validate_fixture(fixture: &Value) -> Check<Vec<Value>> {
    if fixture.get("schema_version").and_then(Value::as_str) != Some(VERSION)
        || fixture.get("benchmark_id").and_then(Value::as_str)
            != Some("WILDFIRE-ROUTE-ASSIGNMENT-001")
        || fixture.get("classification").and_then(Value::as_str) != Some("synthetic_only")
        || fixture.get("max_units").and_then(Value::as_u64) != Some(MAX_UNITS as u64)
        || fixture.get("max_joint_assignments").and_then(Value::as_u64) != Some(MAX_ASSIGNMENTS)
    {
        return Err("route-assignment fixture metadata or search budget mismatch".to_owned());
    }
    let cases = array(fixture, "cases", "fixture")?;
    if cases.len() != CASE_IDS.len() {
        return Err(format!(
            "fixture must contain exactly {} cases",
            CASE_IDS.len()
        ));
    }
    let mut outputs = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        if text(case, "case_id", "case")? != CASE_IDS[index] {
            return Err(format!("case inventory/order mismatch at {index}"));
        }
        outputs.push(evaluate_case(case, index)?);
    }
    Ok(outputs)
}

fn scorecard(cases: &[Value]) -> Value {
    let mut baseline_worse_p95 = 0usize;
    let mut baseline_worse_unserved = 0usize;
    let mut exact_same = 0usize;
    let mut enumeration_total = 0u64;
    let mut baseline_queue_steps = 0u64;
    let mut oracle_queue_steps = 0u64;
    for case in cases {
        baseline_worse_p95 +=
            usize::from(case["baseline_strictly_worse_p95"].as_bool() == Some(true));
        baseline_worse_unserved +=
            usize::from(case["baseline_strictly_worse_unserved"].as_bool() == Some(true));
        exact_same += usize::from(case["objectives_identical"].as_bool() == Some(true));
        enumeration_total += case["joint_assignment_combinations"].as_u64().unwrap_or(0);
        baseline_queue_steps += case["B1_independent_shortest_path"]["queue_vehicle_steps"]
            .as_u64()
            .unwrap_or(0);
        oracle_queue_steps += case["B2_exact_joint_assignment"]["queue_vehicle_steps"]
            .as_u64()
            .unwrap_or(0);
    }
    json!({
        "case_count": cases.len(),
        "baseline_worse_p95_case_count": baseline_worse_p95,
        "baseline_worse_unserved_case_count": baseline_worse_unserved,
        "identical_objective_case_count": exact_same,
        "total_joint_assignments_evaluated": enumeration_total,
        "baseline_queue_vehicle_steps": baseline_queue_steps,
        "oracle_queue_vehicle_steps": oracle_queue_steps,
        "all_baselines_and_oracles_preserve_mass_balance": cases.iter().all(|case| {
            case["B1_independent_shortest_path"]["mass_balance_ok"] == true
                && case["B2_exact_joint_assignment"]["mass_balance_ok"] == true
        }),
        "all_baselines_and_oracles_respect_capacity_checks": cases.iter().all(|case| {
            case["B1_independent_shortest_path"]["capacity_checks_ok"] == true
                && case["B2_exact_joint_assignment"]["capacity_checks_ok"] == true
        }),
        "claim_boundary": "synthetic tiny-instance comparison; not evidence of real-world accuracy or SOTA superiority"
    })
}

fn run() -> Check<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 || args[2] != "--source-commit" {
        return Err(
            "usage: wildfire_route_assignment <fixture.json> --source-commit <40-hex-sha>"
                .to_owned(),
        );
    }
    if args[3].len() != 40 || !args[3].bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("source commit must be a 40-character hex SHA".to_owned());
    }
    let bytes = fs::read(&args[1]).map_err(|e| format!("read fixture: {e}"))?;
    let fixture: Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("parse fixture: {e}"))?;
    let cases = validate_fixture(&fixture)?;
    let receipt = json!({
        "result": "pass",
        "validator_version": VERSION,
        "benchmark_id": "WILDFIRE-ROUTE-ASSIGNMENT-001",
        "scenario_count": cases.len(),
        "fixture_sha256": hash(&bytes),
        "source_commit": args[3],
        "scorecard": scorecard(&cases),
        "cases": cases,
        "claim_boundary": "independent shortest-path baseline vs exhaustive joint route assignment on tiny synthetic fixed-route queue networks only"
    });
    println!(
        "{}",
        serde_json::to_string(&receipt).map_err(|e| format!("serialize receipt: {e}"))?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("joint route-assignment benchmark failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../benchmarks/fixtures/WILDFIRE_ROUTE_ASSIGNMENT_V0_1.json"
        ))
        .expect("route-assignment fixture parses")
    }

    #[test]
    fn joint_assignment_improves_at_least_one_congestion_or_capacity_case() {
        let results = validate_fixture(&fixture()).expect("frozen fixture validates");
        assert_eq!(results.len(), 4);
        assert!(
            results[0]["baseline_strictly_worse_p95"] == true
                || results[0]["B1_independent_shortest_path"]["queue_vehicle_steps"]
                    .as_u64()
                    .unwrap_or(u64::MAX)
                    > results[0]["B2_exact_joint_assignment"]["queue_vehicle_steps"]
                        .as_u64()
                        .unwrap_or(u64::MAX)
        );
        assert!(results[1]["baseline_strictly_worse_unserved"] == true);
        assert!(results[2]["baseline_strictly_worse_unserved"] == true);
    }

    #[test]
    fn shortest_path_can_remain_optimal() {
        let results = validate_fixture(&fixture()).expect("fixture validates");
        assert_eq!(results[3]["objectives_identical"], true);
        assert_eq!(
            results[3]["B1_independent_shortest_path"]["assignment_by_demand"],
            results[3]["B2_exact_joint_assignment"]["assignment_by_demand"]
        );
    }

    #[test]
    fn exact_assignment_respects_declared_search_budget() {
        let mut fixture = fixture();
        fixture["cases"][0]["demands"][0]["vehicles"] = json!(20);
        assert!(validate_fixture(&fixture).is_err());
    }

    #[test]
    fn input_order_does_not_change_selected_objectives_or_allocations() {
        let mut fixture = fixture();
        let original = validate_fixture(&fixture).expect("original");
        for case in fixture["cases"].as_array_mut().expect("cases") {
            case["edges"].as_array_mut().expect("edges").reverse();
            case["sinks"].as_array_mut().expect("sinks").reverse();
            case["demands"].as_array_mut().expect("demands").reverse();
            for demand in case["demands"].as_array_mut().expect("demands") {
                demand["candidate_routes"]
                    .as_array_mut()
                    .expect("candidate routes")
                    .reverse();
            }
        }
        let reordered = validate_fixture(&fixture).expect("reordered");
        for (a, b) in original.iter().zip(reordered.iter()) {
            assert_eq!(
                a["B1_independent_shortest_path"]["objective"],
                b["B1_independent_shortest_path"]["objective"]
            );
            assert_eq!(
                a["B2_exact_joint_assignment"]["objective"],
                b["B2_exact_joint_assignment"]["objective"]
            );
            assert_eq!(
                a["B1_independent_shortest_path"]["assignment_by_demand"],
                b["B1_independent_shortest_path"]["assignment_by_demand"]
            );
            assert_eq!(
                a["B2_exact_joint_assignment"]["assignment_by_demand"],
                b["B2_exact_joint_assignment"]["assignment_by_demand"]
            );
        }
    }

    fn next_seed(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *state
    }

    #[test]
    fn deterministic_parameter_sweep_preserves_exact_oracle_dominance() {
        let frozen = fixture();
        let cases = frozen["cases"].as_array().expect("four frozen cases");
        let mut evaluated = 0usize;

        // 32 reproducible perturbations per topology. This is deliberately a bounded
        // correctness sweep, not a claim about large-network throughput.
        for (case_index, frozen_case) in cases.iter().enumerate() {
            for sample in 0..32u64 {
                let mut case = frozen_case.clone();
                let mut state = 0x5359_4d54_524f_5059u64
                    ^ ((case_index as u64) << 32)
                    ^ sample;

                for edge in case["edges"].as_array_mut().expect("edge array") {
                    let capacity = next_seed(&mut state) % 4 + 1;
                    let storage = capacity + next_seed(&mut state) % 7;
                    let travel_time = next_seed(&mut state) % 4 + 1;
                    edge["capacity_per_step"] = json!(capacity);
                    edge["storage"] = json!(storage);
                    edge["travel_time_steps"] = json!(travel_time);
                }
                for demand in case["demands"].as_array_mut().expect("demand array") {
                    demand["departure_step"] = json!(next_seed(&mut state) % 5 + 1);
                }

                let case_id = case["case_id"].as_str().expect("case ID");
                let (demands, units) = parse_case(&case)
                    .unwrap_or_else(|error| panic!("{case_id} sample {sample}: {error}"));
                let baseline = best_assignment(&case, &demands, &units, false)
                    .unwrap_or_else(|error| panic!("baseline {case_id} sample {sample}: {error}"));
                let oracle = best_assignment(&case, &demands, &units, true)
                    .unwrap_or_else(|error| panic!("oracle {case_id} sample {sample}: {error}"));

                assert!(
                    !compare_objective(&oracle.objective, &baseline.objective).is_gt(),
                    "exact oracle worsened objective for {case_id} sample {sample}: oracle={:?}, baseline={:?}",
                    oracle.objective,
                    baseline.objective
                );
                assert_eq!(
                    oracle.simulation["mass_balance_ok"],
                    true,
                    "{case_id} sample {sample}: oracle mass balance"
                );
                assert_eq!(
                    baseline.simulation["mass_balance_ok"],
                    true,
                    "{case_id} sample {sample}: baseline mass balance"
                );
                assert_eq!(
                    oracle.simulation["capacity_checks_ok"],
                    true,
                    "{case_id} sample {sample}: oracle capacity check"
                );
                assert_eq!(
                    baseline.simulation["capacity_checks_ok"],
                    true,
                    "{case_id} sample {sample}: baseline capacity check"
                );
                assert!(
                    oracle.enumeration_count <= MAX_ASSIGNMENTS,
                    "{case_id} sample {sample}: oracle exceeded search budget"
                );
                evaluated += 1;
            }
        }
        assert_eq!(evaluated, 128);
    }

    #[test]
    fn malformed_disconnected_candidate_route_fails_closed() {
        let mut fixture = fixture();
        fixture["cases"][0]["demands"][0]["candidate_routes"][1]["route_edge_ids"] =
            json!(["A_K", "N_S"]);
        assert!(validate_fixture(&fixture).is_err());
    }
}
