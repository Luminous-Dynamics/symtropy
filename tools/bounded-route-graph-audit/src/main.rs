use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process};

const CORPUS: &str =
    include_str!("../../../fixtures/transport/bounded_route_graph_scenarios_v0_1.json");
const CLAIM_CEILING: &str =
    "synthetic_route_enumeration_is_not_route_authorization_or_real_world_transport_validation";
const ENVIRONMENTS: [&str; 5] = [
    "amazon_basin",
    "congo_basin",
    "antarctica",
    "sahara",
    "afghanistan",
];
const CAPABILITY_FIELDS: [&str; 6] = [
    "condition_control",
    "securement",
    "accessible_transfer",
    "patient_support",
    "operator_present",
    "recovery_available",
];
const GATE_FIELDS: [&str; 6] = [
    "route_authorization",
    "environmental_protection",
    "hazard_status",
    "recovery_status",
    "operator_status",
    "condition_control_status",
];
const VALID_STATUSES: [&str; 3] = ["passed", "unknown", "failed"];
const MAX_MISSIONS: usize = 50;
const MAX_NODES: usize = 1000;
const MAX_EDGES: usize = 5000;
const MAX_PATHS_PER_MISSION: usize = 10_000;
const MAX_EXPANSIONS_PER_MISSION: usize = 100_000;

#[derive(Clone, Debug)]
struct Route {
    node_ids: Vec<String>,
    edge_ids: Vec<String>,
}

fn fail(path: &str, message: &str) -> String {
    format!("{path}: {message}")
}

fn exact_keys(value: &Value, expected: &[&str], path: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))?;
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let expected: BTreeSet<&str> = expected.iter().copied().collect();
    if actual != expected {
        return Err(fail(
            path,
            &format!(
                "keys differ; expected {:?}, received {:?}",
                expected, actual
            ),
        ));
    }
    Ok(())
}

fn text<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-empty string"))
}

fn number(value: &Value, key: &str, path: &str) -> Result<f64, String> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite())
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a finite number"))
}

fn integer(value: &Value, key: &str, path: &str) -> Result<i64, String> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be an integer"))
}

fn boolean(value: &Value, key: &str, path: &str) -> Result<bool, String> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a boolean"))
}

fn string_array(value: &Value, path: &str) -> Result<Vec<String>, String> {
    let items = value
        .as_array()
        .filter(|items| !items.is_empty())
        .ok_or_else(|| fail(path, "must be a non-empty array"))?;
    let mut seen = BTreeSet::new();
    let mut output = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let item = item
            .as_str()
            .filter(|item| !item.trim().is_empty())
            .ok_or_else(|| fail(&format!("{path}[{index}]"), "must be a non-empty string"))?;
        if !seen.insert(item.to_string()) {
            return Err(fail(path, &format!("duplicate array entry {item}")));
        }
        output.push(item.to_string());
    }
    Ok(output)
}

fn validate_capabilities(value: &Value, path: &str) -> Result<(), String> {
    exact_keys(value, &CAPABILITY_FIELDS, path)?;
    for field in CAPABILITY_FIELDS {
        let _ = boolean(value, field, path)?;
    }
    Ok(())
}

fn validate_gates(value: &Value, path: &str) -> Result<(), String> {
    exact_keys(value, &GATE_FIELDS, path)?;
    for field in GATE_FIELDS {
        let status = text(value, field, path)?;
        if !VALID_STATUSES.contains(&status) {
            return Err(fail(
                &format!("{path}.{field}"),
                "status must be passed, unknown or failed",
            ));
        }
    }
    Ok(())
}

fn validate_fixture(input: &Value) -> Result<(), String> {
    exact_keys(
        input,
        &[
            "schema_version",
            "benchmark_id",
            "provenance",
            "claim_ceiling",
            "missions",
            "nodes",
            "edges",
            "expectations",
        ],
        "$",
    )?;
    if text(input, "schema_version", "$")? != "0.1"
        || text(input, "benchmark_id", "$")? != "bounded-route-graph-v0.1"
        || text(input, "claim_ceiling", "$")? != CLAIM_CEILING
    {
        return Err(fail("$", "benchmark identity or claim ceiling mismatch"));
    }

    let provenance = &input["provenance"];
    exact_keys(
        provenance,
        &["kind", "external_real_world_data_included", "notes"],
        "$.provenance",
    )?;
    if text(provenance, "kind", "$.provenance")? != "synthetic_only"
        || boolean(provenance, "external_real_world_data_included", "$.provenance")?
    {
        return Err(fail(
            "$.provenance",
            "only synthetic fixtures without real-world data are accepted",
        ));
    }
    let _ = text(provenance, "notes", "$.provenance")?;

    let missions = input["missions"]
        .as_array()
        .filter(|items| !items.is_empty() && items.len() <= MAX_MISSIONS)
        .ok_or_else(|| fail("$.missions", "must be a non-empty array within the mission limit"))?;
    let mut mission_ids = BTreeSet::new();
    for (index, mission) in missions.iter().enumerate() {
        let path = format!("$.missions[{index}]");
        exact_keys(
            mission,
            &[
                "mission_id",
                "environment_id",
                "payload_class",
                "origin_node",
                "destination_node",
                "deadline_max_minutes",
                "initial_energy_reserve_pct",
                "minimum_energy_reserve_pct",
                "minimum_condition_margin_c",
                "max_hops",
                "requirements",
            ],
            &path,
        )?;
        let id = text(mission, "mission_id", &path)?;
        if !mission_ids.insert(id.to_string()) {
            return Err(fail(&path, "duplicate mission_id"));
        }
        let environment = text(mission, "environment_id", &path)?;
        if !ENVIRONMENTS.contains(&environment) {
            return Err(fail(&path, "unsupported environment_id"));
        }
        for field in ["payload_class", "origin_node", "destination_node"] {
            let _ = text(mission, field, &path)?;
        }
        let deadline = integer(mission, "deadline_max_minutes", &path)?;
        let max_hops = integer(mission, "max_hops", &path)?;
        if !(1..=1_000_000).contains(&deadline) {
            return Err(fail(&path, "deadline must be between 1 and 1000000 minutes"));
        }
        if !(1..=8).contains(&max_hops) {
            return Err(fail(&path, "max_hops must be between 1 and 8"));
        }
        for field in ["initial_energy_reserve_pct", "minimum_energy_reserve_pct"] {
            let amount = number(mission, field, &path)?;
            if !(0.0..=100.0).contains(&amount) {
                return Err(fail(&path, &format!("{field} must be between 0 and 100")));
            }
        }
        if number(mission, "minimum_condition_margin_c", &path)?.abs() > 1000.0 {
            return Err(fail(&path, "minimum condition margin exceeds declared bounds"));
        }
        validate_capabilities(
            &mission["requirements"],
            &format!("{path}.requirements"),
        )?;
    }

    let nodes = input["nodes"]
        .as_array()
        .filter(|items| !items.is_empty() && items.len() <= MAX_NODES)
        .ok_or_else(|| fail("$.nodes", "must be a non-empty array within the node limit"))?;
    let mut node_ids = BTreeSet::new();
    for (index, node) in nodes.iter().enumerate() {
        let path = format!("$.nodes[{index}]");
        exact_keys(
            node,
            &[
                "node_id",
                "environment_ids",
                "payload_classes",
                "transfer_duration_min_minutes",
                "transfer_duration_max_minutes",
                "transfer_energy_consumption_pct_max",
                "condition_margin_c",
                "capabilities",
                "gates",
            ],
            &path,
        )?;
        let id = text(node, "node_id", &path)?;
        if !node_ids.insert(id.to_string()) {
            return Err(fail(&path, "duplicate node_id"));
        }
        let environments = string_array(&node["environment_ids"], &format!("{path}.environment_ids"))?;
        if environments.iter().any(|value| !ENVIRONMENTS.contains(&value.as_str())) {
            return Err(fail(&path, "node contains an unsupported environment"));
        }
        let _ = string_array(&node["payload_classes"], &format!("{path}.payload_classes"))?;
        let duration_min = integer(node, "transfer_duration_min_minutes", &path)?;
        let duration_max = integer(node, "transfer_duration_max_minutes", &path)?;
        if !(0..=1_000_000).contains(&duration_min)
            || !(duration_min..=1_000_000).contains(&duration_max)
        {
            return Err(fail(&path, "invalid node transfer duration interval"));
        }
        let transfer_energy = number(node, "transfer_energy_consumption_pct_max", &path)?;
        if !(0.0..=100.0).contains(&transfer_energy) {
            return Err(fail(&path, "node transfer energy must be between 0 and 100"));
        }
        if number(node, "condition_margin_c", &path)?.abs() > 1000.0 {
            return Err(fail(&path, "node condition margin exceeds declared bounds"));
        }
        validate_capabilities(&node["capabilities"], &format!("{path}.capabilities"))?;
        validate_gates(&node["gates"], &format!("{path}.gates"))?;
    }

    let edges = input["edges"]
        .as_array()
        .filter(|items| items.len() <= MAX_EDGES)
        .ok_or_else(|| fail("$.edges", "must be an array within the edge limit"))?;
    let mut edge_ids = BTreeSet::new();
    for (index, edge) in edges.iter().enumerate() {
        let path = format!("$.edges[{index}]");
        exact_keys(
            edge,
            &[
                "edge_id",
                "from_node",
                "to_node",
                "mode",
                "environment_ids",
                "payload_classes",
                "duration_min_minutes",
                "duration_max_minutes",
                "energy_consumption_pct_max",
                "condition_margin_c",
                "capabilities",
                "gates",
                "available",
            ],
            &path,
        )?;
        let id = text(edge, "edge_id", &path)?;
        if !edge_ids.insert(id.to_string()) {
            return Err(fail(&path, "duplicate edge_id"));
        }
        let from = text(edge, "from_node", &path)?;
        let to = text(edge, "to_node", &path)?;
        if from == to || !node_ids.contains(from) || !node_ids.contains(to) {
            return Err(fail(&path, "edge endpoints must be distinct known nodes"));
        }
        let _ = text(edge, "mode", &path)?;
        let environments = string_array(&edge["environment_ids"], &format!("{path}.environment_ids"))?;
        if environments.iter().any(|value| !ENVIRONMENTS.contains(&value.as_str())) {
            return Err(fail(&path, "edge contains an unsupported environment"));
        }
        let _ = string_array(&edge["payload_classes"], &format!("{path}.payload_classes"))?;
        let duration_min = integer(edge, "duration_min_minutes", &path)?;
        let duration_max = integer(edge, "duration_max_minutes", &path)?;
        if !(0..=1_000_000).contains(&duration_min)
            || !(duration_min..=1_000_000).contains(&duration_max)
        {
            return Err(fail(&path, "invalid edge duration interval"));
        }
        let energy = number(edge, "energy_consumption_pct_max", &path)?;
        if !(0.0..=100.0).contains(&energy) {
            return Err(fail(&path, "edge energy consumption must be between 0 and 100"));
        }
        if number(edge, "condition_margin_c", &path)?.abs() > 1000.0 {
            return Err(fail(&path, "edge condition margin exceeds declared bounds"));
        }
        let _ = boolean(edge, "available", &path)?;
        validate_capabilities(&edge["capabilities"], &format!("{path}.capabilities"))?;
        validate_gates(&edge["gates"], &format!("{path}.gates"))?;
    }

    for mission in missions {
        let path = format!("$.missions[{}]", mission["mission_id"].as_str().unwrap());
        for field in ["origin_node", "destination_node"] {
            let node_id = mission[field].as_str().unwrap();
            if !node_ids.contains(node_id) {
                return Err(fail(&path, &format!("{field} references an unknown node")));
            }
        }
        if mission["origin_node"] == mission["destination_node"] {
            return Err(fail(&path, "origin and destination must differ"));
        }
    }

    let expectations = input["expectations"]
        .as_array()
        .filter(|items| !items.is_empty())
        .ok_or_else(|| fail("$.expectations", "must be a non-empty array"))?;
    if expectations.len() != missions.len() {
        return Err(fail(
            "$.expectations",
            "must contain exactly one expectation for every mission",
        ));
    }
    let mut expectation_ids = BTreeSet::new();
    for (index, expected) in expectations.iter().enumerate() {
        let path = format!("$.expectations[{index}]");
        exact_keys(
            expected,
            &[
                "mission_id",
                "expected_topological_route_count",
                "expected_eligible_route_count",
            ],
            &path,
        )?;
        let id = text(expected, "mission_id", &path)?;
        if !mission_ids.contains(id) || !expectation_ids.insert(id.to_string()) {
            return Err(fail(&path, "expectation mission is unknown or duplicated"));
        }
        for field in ["expected_topological_route_count", "expected_eligible_route_count"] {
            if !(0..=10_000).contains(&integer(expected, field, &path)?) {
                return Err(fail(&path, &format!("{field} exceeds route count bounds")));
            }
        }
    }
    if expectation_ids != mission_ids {
        return Err(fail("$.expectations", "expectation coverage must match mission IDs exactly"));
    }
    Ok(())
}

fn enumerate_from(
    current: &str,
    destination: &str,
    max_hops: usize,
    adjacency: &BTreeMap<String, Vec<Value>>,
    visited: &mut BTreeSet<String>,
    node_path: &mut Vec<String>,
    edge_path: &mut Vec<String>,
    routes: &mut Vec<Route>,
    expansions: &mut usize,
) -> Result<(), String> {
    if current == destination {
        if routes.len() >= MAX_PATHS_PER_MISSION {
            return Err("route enumeration exceeded the per-mission route limit".to_string());
        }
        routes.push(Route {
            node_ids: node_path.clone(),
            edge_ids: edge_path.clone(),
        });
        return Ok(());
    }
    if edge_path.len() >= max_hops {
        return Ok(());
    }
    let Some(outgoing) = adjacency.get(current) else {
        return Ok(());
    };
    for edge in outgoing {
        *expansions += 1;
        if *expansions > MAX_EXPANSIONS_PER_MISSION {
            return Err("route enumeration exceeded the per-mission expansion limit".to_string());
        }
        let edge_id = edge["edge_id"].as_str().unwrap();
        let next_node = edge["to_node"].as_str().unwrap();
        if visited.contains(next_node) {
            continue;
        }
        visited.insert(next_node.to_string());
        node_path.push(next_node.to_string());
        edge_path.push(edge_id.to_string());
        enumerate_from(
            next_node,
            destination,
            max_hops,
            adjacency,
            visited,
            node_path,
            edge_path,
            routes,
            expansions,
        )?;
        edge_path.pop();
        node_path.pop();
        visited.remove(next_node);
    }
    Ok(())
}

fn enumerate_routes(
    mission: &Value,
    adjacency: &BTreeMap<String, Vec<Value>>,
) -> Result<Vec<Route>, String> {
    let origin = mission["origin_node"].as_str().unwrap();
    let destination = mission["destination_node"].as_str().unwrap();
    let mut visited = BTreeSet::from([origin.to_string()]);
    let mut node_path = vec![origin.to_string()];
    let mut edge_path = Vec::new();
    let mut routes = Vec::new();
    let mut expansions = 0usize;
    enumerate_from(
        origin,
        destination,
        mission["max_hops"].as_u64().unwrap() as usize,
        adjacency,
        &mut visited,
        &mut node_path,
        &mut edge_path,
        &mut routes,
        &mut expansions,
    )?;
    routes.sort_by(|left, right| left.edge_ids.cmp(&right.edge_ids));
    Ok(routes)
}

fn eligible_capability(
    requirements: &Value,
    capabilities: &Value,
    capability: &str,
) -> bool {
    requirements[capability].as_bool() != Some(true)
        || capabilities[capability].as_bool() == Some(true)
}

fn assess_route(
    mission: &Value,
    route: &Route,
    nodes: &BTreeMap<String, Value>,
    edges: &BTreeMap<String, Value>,
) -> Result<Value, String> {
    let mission_id = mission["mission_id"].as_str().unwrap();
    let environment = mission["environment_id"].as_str().unwrap();
    let payload_class = mission["payload_class"].as_str().unwrap();
    let requirements = &mission["requirements"];
    let route_id = route.edge_ids.join("+");
    let mut failed_gates = Vec::new();
    let mut unknown_gates = Vec::new();
    let mut constraint_violations = Vec::new();
    let mut unavailable_edges = Vec::new();
    let mut duration_min = 0i64;
    let mut duration_max = 0i64;
    let mut energy_consumption_max = 0.0f64;
    let mut condition_margin = f64::INFINITY;
    let mut mode_chain = Vec::new();

    for (node_index, node_id) in route.node_ids.iter().enumerate() {
        let node = nodes
            .get(node_id)
            .ok_or_else(|| fail("route", "enumerated path references missing node"))?;
        let node_label = format!("node:{node_id}");
        if !node["environment_ids"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some(environment))
        {
            constraint_violations.push(format!("{node_label}:environment_mismatch"));
        }
        if !node["payload_classes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some(payload_class))
        {
            constraint_violations.push(format!("{node_label}:payload_class_unsupported"));
        }
        for capability in CAPABILITY_FIELDS {
            if !eligible_capability(requirements, &node["capabilities"], capability) {
                constraint_violations.push(format!("{node_label}:required_capability_missing:{capability}"));
            }
        }
        for gate in GATE_FIELDS {
            match node["gates"][gate].as_str().unwrap() {
                "failed" => failed_gates.push(format!("{node_label}:{gate}")),
                "unknown" => unknown_gates.push(format!("{node_label}:{gate}")),
                _ => {}
            }
        }
        condition_margin = condition_margin.min(node["condition_margin_c"].as_f64().unwrap());

        // The payload already sits at the origin and is delivered at the destination;
        // only intermediate hubs contribute transfer time and energy.
        if node_index > 0 && node_index + 1 < route.node_ids.len() {
            duration_min += node["transfer_duration_min_minutes"].as_i64().unwrap();
            duration_max += node["transfer_duration_max_minutes"].as_i64().unwrap();
            energy_consumption_max +=
                node["transfer_energy_consumption_pct_max"].as_f64().unwrap();
        }
    }

    for edge_id in &route.edge_ids {
        let edge = edges
            .get(edge_id)
            .ok_or_else(|| fail("route", "enumerated path references missing edge"))?;
        let edge_label = format!("edge:{edge_id}");
        mode_chain.push(edge["mode"].as_str().unwrap().to_string());
        if !edge["available"].as_bool().unwrap() {
            unavailable_edges.push(edge_id.clone());
        }
        if !edge["environment_ids"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some(environment))
        {
            constraint_violations.push(format!("{edge_label}:environment_mismatch"));
        }
        if !edge["payload_classes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some(payload_class))
        {
            constraint_violations.push(format!("{edge_label}:payload_class_unsupported"));
        }
        for capability in CAPABILITY_FIELDS {
            if !eligible_capability(requirements, &edge["capabilities"], capability) {
                constraint_violations.push(format!("{edge_label}:required_capability_missing:{capability}"));
            }
        }
        for gate in GATE_FIELDS {
            match edge["gates"][gate].as_str().unwrap() {
                "failed" => failed_gates.push(format!("{edge_label}:{gate}")),
                "unknown" => unknown_gates.push(format!("{edge_label}:{gate}")),
                _ => {}
            }
        }
        duration_min += edge["duration_min_minutes"].as_i64().unwrap();
        duration_max += edge["duration_max_minutes"].as_i64().unwrap();
        energy_consumption_max += edge["energy_consumption_pct_max"].as_f64().unwrap();
        condition_margin = condition_margin.min(edge["condition_margin_c"].as_f64().unwrap());
    }

    if duration_max > mission["deadline_max_minutes"].as_i64().unwrap() {
        constraint_violations.push("worst_case_duration_exceeds_deadline".to_string());
    }
    let energy_remaining = mission["initial_energy_reserve_pct"].as_f64().unwrap()
        - energy_consumption_max;
    if energy_remaining < mission["minimum_energy_reserve_pct"].as_f64().unwrap() {
        constraint_violations.push("worst_case_energy_reserve_below_minimum".to_string());
    }
    if condition_margin < mission["minimum_condition_margin_c"].as_f64().unwrap() {
        constraint_violations.push("minimum_condition_margin_below_requirement".to_string());
    }
    failed_gates.sort();
    unknown_gates.sort();
    constraint_violations.sort();
    unavailable_edges.sort();

    let disposition = if !unavailable_edges.is_empty() {
        "blocked_unavailable"
    } else if !failed_gates.is_empty() {
        "blocked_hard_gate"
    } else if !constraint_violations.is_empty() {
        "blocked_constraints"
    } else if !unknown_gates.is_empty() {
        "needs_gate_evidence"
    } else {
        "eligible_for_human_review"
    };
    Ok(json!({
        "mission_id": mission_id,
        "route_id": route_id,
        "node_chain": route.node_ids,
        "edge_chain": route.edge_ids,
        "mode_chain": mode_chain,
        "lower_bound_duration_minutes": duration_min,
        "upper_bound_duration_minutes": duration_max,
        "worst_case_energy_consumption_pct": energy_consumption_max,
        "remaining_energy_reserve_pct": energy_remaining,
        "minimum_condition_margin_c": condition_margin,
        "failed_gates": failed_gates,
        "unknown_gates": unknown_gates,
        "unavailable_edges": unavailable_edges,
        "constraint_violations": constraint_violations,
        "disposition": disposition
    }))
}

fn metric(assessment: &Value, field: &str) -> f64 {
    assessment[field].as_f64().unwrap()
}

fn dominates(left: &Value, right: &Value) -> bool {
    let duration_left = metric(left, "upper_bound_duration_minutes");
    let duration_right = metric(right, "upper_bound_duration_minutes");
    let reserve_left = metric(left, "remaining_energy_reserve_pct");
    let reserve_right = metric(right, "remaining_energy_reserve_pct");
    let margin_left = metric(left, "minimum_condition_margin_c");
    let margin_right = metric(right, "minimum_condition_margin_c");
    let epsilon = 1e-9;
    let no_worse = duration_left <= duration_right + epsilon
        && reserve_left + epsilon >= reserve_right
        && margin_left + epsilon >= margin_right;
    let strictly_better = duration_left < duration_right - epsilon
        || reserve_left > reserve_right + epsilon
        || margin_left > margin_right + epsilon;
    no_worse && strictly_better
}

fn pareto_route_ids(assessments: &[Value]) -> Vec<String> {
    let eligible: Vec<&Value> = assessments
        .iter()
        .filter(|assessment| {
            assessment["disposition"].as_str() == Some("eligible_for_human_review")
        })
        .collect();
    let mut ids: Vec<String> = eligible
        .iter()
        .filter(|candidate| {
            !eligible.iter().any(|other| {
                other["route_id"] != candidate["route_id"] && dominates(other, candidate)
            })
        })
        .map(|candidate| candidate["route_id"].as_str().unwrap().to_string())
        .collect();
    ids.sort();
    ids
}

fn audit(input: &Value) -> Result<Value, String> {
    validate_fixture(input)?;
    let mut mission_map: BTreeMap<String, Value> = BTreeMap::new();
    for mission in input["missions"].as_array().unwrap() {
        mission_map.insert(mission["mission_id"].as_str().unwrap().to_string(), mission.clone());
    }
    let mut node_map: BTreeMap<String, Value> = BTreeMap::new();
    for node in input["nodes"].as_array().unwrap() {
        node_map.insert(node["node_id"].as_str().unwrap().to_string(), node.clone());
    }
    let mut edge_map: BTreeMap<String, Value> = BTreeMap::new();
    let mut adjacency: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for edge in input["edges"].as_array().unwrap() {
        let from = edge["from_node"].as_str().unwrap().to_string();
        let edge_id = edge["edge_id"].as_str().unwrap().to_string();
        adjacency.entry(from).or_default().push(edge.clone());
        edge_map.insert(edge_id, edge.clone());
    }
    for outgoing in adjacency.values_mut() {
        outgoing.sort_by(|left, right| left["edge_id"].as_str().cmp(&right["edge_id"].as_str()));
    }
    let mut expectations: BTreeMap<String, Value> = BTreeMap::new();
    for expected in input["expectations"].as_array().unwrap() {
        expectations.insert(
            expected["mission_id"].as_str().unwrap().to_string(),
            expected.clone(),
        );
    }

    let mut sorted_missions = input["missions"].as_array().unwrap().clone();
    sorted_missions.sort_by(|left, right| left["mission_id"].as_str().cmp(&right["mission_id"].as_str()));
    let mut mission_summaries = Vec::new();
    let mut all_assessments = Vec::new();
    let mut total_topological_route_count = 0usize;
    let mut total_eligible_route_count = 0usize;
    let mut expected_match_count = 0usize;
    let mut environment_ids = BTreeSet::new();

    for mission in &sorted_missions {
        let mission_id = mission["mission_id"].as_str().unwrap();
        let environment_id = mission["environment_id"].as_str().unwrap();
        environment_ids.insert(environment_id.to_string());
        let routes = enumerate_routes(mission, &adjacency)?;
        let mut assessments = Vec::new();
        for route in &routes {
            assessments.push(assess_route(mission, route, &node_map, &edge_map)?);
        }
        assessments.sort_by(|left, right| left["route_id"].as_str().cmp(&right["route_id"].as_str()));
        let eligible: Vec<&Value> = assessments
            .iter()
            .filter(|assessment| {
                assessment["disposition"].as_str() == Some("eligible_for_human_review")
            })
            .collect();
        let topological_count = routes.len();
        let eligible_count = eligible.len();
        total_topological_route_count += topological_count;
        total_eligible_route_count += eligible_count;
        let expected = expectations.get(mission_id).unwrap();
        let expected_topological = expected["expected_topological_route_count"].as_u64().unwrap() as usize;
        let expected_eligible = expected["expected_eligible_route_count"].as_u64().unwrap() as usize;
        let oracle_matches = topological_count == expected_topological && eligible_count == expected_eligible;
        if oracle_matches {
            expected_match_count += 1;
        }
        let mut eligible_ids: Vec<String> = eligible
            .iter()
            .map(|assessment| assessment["route_id"].as_str().unwrap().to_string())
            .collect();
        eligible_ids.sort();
        let disposition = if topological_count == 0 {
            "no_topological_route"
        } else if eligible_count == 0 {
            "no_admissible_route"
        } else {
            "admissible_routes_available"
        };
        mission_summaries.push(json!({
            "mission_id": mission_id,
            "environment_id": environment_id,
            "payload_class": mission["payload_class"],
            "origin_node": mission["origin_node"],
            "destination_node": mission["destination_node"],
            "topological_route_count": topological_count,
            "eligible_route_count": eligible_count,
            "eligible_route_ids": eligible_ids,
            "pareto_route_ids": pareto_route_ids(&assessments),
            "mission_disposition": disposition,
            "expected_topological_route_count": expected_topological,
            "expected_eligible_route_count": expected_eligible,
            "expectation_matches": oracle_matches,
            "route_assessments": assessments
        }));
        all_assessments.extend(assessments);
    }

    let mut environment_summaries = Vec::new();
    let environment_count = environment_ids.len();
    for environment_id in &environment_ids {
        let summaries: Vec<&Value> = mission_summaries
            .iter()
            .filter(|summary| summary["environment_id"].as_str() == Some(environment_id.as_str()))
            .collect();
        let mission_count = summaries.len();
        let topological_routes: usize = summaries
            .iter()
            .map(|summary| summary["topological_route_count"].as_u64().unwrap() as usize)
            .sum();
        let eligible_routes: usize = summaries
            .iter()
            .map(|summary| summary["eligible_route_count"].as_u64().unwrap() as usize)
            .sum();
        let no_topological_missions = summaries
            .iter()
            .filter(|summary| summary["mission_disposition"] == "no_topological_route")
            .count();
        let no_admissible_missions = summaries
            .iter()
            .filter(|summary| summary["mission_disposition"] == "no_admissible_route")
            .count();
        environment_summaries.push(json!({
            "environment_id": environment_id,
            "mission_count": mission_count,
            "topological_route_count": topological_routes,
            "eligible_route_count": eligible_routes,
            "no_topological_route_mission_count": no_topological_missions,
            "no_admissible_route_mission_count": no_admissible_missions
        }));
    }

    let mission_count = mission_map.len();
    let expectation_match_percent = if mission_count == 0 {
        0.0
    } else {
        ((expected_match_count as f64 / mission_count as f64) * 10_000.0).round() / 100.0
    };
    Ok(json!({
        "benchmark_id": "bounded-route-graph-v0.1",
        "claim_ceiling": CLAIM_CEILING,
        "provenance_kind": "synthetic_only",
        "mission_count": mission_count,
        "environment_count": environment_count,
        "node_count": node_map.len(),
        "edge_count": edge_map.len(),
        "topological_route_count": total_topological_route_count,
        "eligible_route_count": total_eligible_route_count,
        "expected_oracle_match_count": expected_match_count,
        "expected_oracle_match_percent": expectation_match_percent,
        "weighted_utility_score_produced": false,
        "universal_winner_produced": false,
        "route_or_deployment_authorized": false,
        "patient_care_authorized": false,
        "autonomous_hazard_handling_authorized": false,
        "real_world_validated": false,
        "mission_summaries": mission_summaries,
        "environment_summaries": environment_summaries,
        "route_assessments": all_assessments
    }))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() > 1 {
        return Err("usage: bounded-route-graph-audit [fixture.json]".to_string());
    }
    let source = if let Some(path) = args.first() {
        fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?
    } else {
        CORPUS.to_string()
    };
    let input: Value =
        serde_json::from_str(&source).map_err(|error| format!("invalid JSON: {error}"))?;
    let report = audit(&input)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("cannot serialize report: {error}"))?
    );
    let expected = report["mission_count"].as_u64().unwrap();
    let matched = report["expected_oracle_match_count"].as_u64().unwrap();
    if expected != matched {
        return Err(format!(
            "mission oracle mismatch: {matched} of {expected} mission expectations matched"
        ));
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("bounded-route-graph-audit: {error}");
        process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(CORPUS).unwrap()
    }

    fn report() -> Value {
        audit(&fixture()).unwrap()
    }

    fn mission<'a>(input: &'a Value, id: &str) -> &'a Value {
        input["missions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["mission_id"].as_str() == Some(id))
            .unwrap()
    }

    fn route<'a>(report: &'a Value, id: &str) -> &'a Value {
        report["route_assessments"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["route_id"].as_str() == Some(id))
            .unwrap()
    }

    fn mission_result<'a>(report: &'a Value, id: &str) -> &'a Value {
        report["mission_summaries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["mission_id"].as_str() == Some(id))
            .unwrap()
    }

    #[test]
    fn five_environment_graph_counts_and_all_synthetic_oracles_match() {
        let output = report();
        assert_eq!(output["mission_count"], 6);
        assert_eq!(output["environment_count"], 5);
        assert_eq!(output["node_count"], 17);
        assert_eq!(output["edge_count"], 19);
        assert_eq!(output["topological_route_count"], 14);
        assert_eq!(output["eligible_route_count"], 9);
        assert_eq!(output["expected_oracle_match_count"], 6);
        assert_eq!(output["expected_oracle_match_percent"], 100.0);
        assert_eq!(output["weighted_utility_score_produced"], false);
        assert_eq!(output["universal_winner_produced"], false);
        assert_eq!(output["route_or_deployment_authorized"], false);
        assert_eq!(output["real_world_validated"], false);
    }

    #[test]
    fn disconnected_graph_and_unknown_authorization_are_distinguished() {
        let output = report();
        let disconnected = mission_result(&output, "sahara-disconnected-sensor");
        assert_eq!(disconnected["topological_route_count"], 0);
        assert_eq!(disconnected["mission_disposition"], "no_topological_route");

        let unknown = mission_result(&output, "afghanistan-civilian-aid-payload");
        assert_eq!(unknown["topological_route_count"], 2);
        assert_eq!(unknown["eligible_route_count"], 0);
        assert_eq!(unknown["mission_disposition"], "no_admissible_route");
        assert!(unknown["route_assessments"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["disposition"] == "needs_gate_evidence"));
    }

    #[test]
    fn the_route_with_missing_securement_is_not_pareto_eligible() {
        let output = report();
        assert_eq!(
            route(&output, "congo-unsecure-direct")["disposition"],
            "blocked_constraints"
        );
        assert!(!mission_result(&output, "congo-fragile-sample")["pareto_route_ids"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "congo-unsecure-direct"));
    }

    #[test]
    fn all_tradeoff_preserving_frontiers_match_the_reference() {
        let output = report();
        assert_eq!(
            mission_result(&output, "amazon-medical-cold-chain")["pareto_route_ids"],
            json!([
                "amz-aerial-direct",
                "amz-ground-direct",
                "amz-rivercraft+amz-ground-transfer"
            ])
        );
        assert_eq!(
            mission_result(&output, "sahara-water-medical-kit")["pareto_route_ids"],
            json!(["sahara-dune-direct", "sahara-night-first-leg+sahara-night-final-leg"])
        );
        assert_eq!(
            mission_result(&output, "afghanistan-civilian-aid-payload")["pareto_route_ids"],
            json!([])
        );
    }

    #[test]
    fn unknown_route_or_hazard_evidence_never_becomes_authorization() {
        let output = report();
        let assessment = route(&output, "afg-primary-direct");
        assert_eq!(assessment["disposition"], "needs_gate_evidence");
        assert!(assessment["unknown_gates"].as_array().unwrap().iter().any(|value| value == "edge:afg-primary-direct:route_authorization"));
        assert_eq!(output["route_or_deployment_authorized"], false);
        assert_eq!(output["autonomous_hazard_handling_authorized"], false);
    }

    #[test]
    fn collection_order_does_not_change_normalized_output() {
        let input = fixture();
        let baseline = serde_json::to_string(&audit(&input).unwrap()).unwrap();
        let mut reordered = input.clone();
        reordered["missions"].as_array_mut().unwrap().reverse();
        reordered["nodes"].as_array_mut().unwrap().reverse();
        reordered["edges"].as_array_mut().unwrap().reverse();
        reordered["expectations"].as_array_mut().unwrap().reverse();
        assert_eq!(
            baseline,
            serde_json::to_string(&audit(&reordered).unwrap()).unwrap()
        );
    }

    #[test]
    fn intermediate_hub_time_energy_and_condition_margins_are_counted_once() {
        let output = report();
        let assessment = route(&output, "amz-rivercraft+amz-ground-transfer");
        assert_eq!(assessment["lower_bound_duration_minutes"].as_i64().unwrap(), 90);
        assert_eq!(assessment["upper_bound_duration_minutes"].as_i64().unwrap(), 130);
        assert!((assessment["worst_case_energy_consumption_pct"].as_f64().unwrap() - 40.0).abs() < 1e-9);
        assert!((assessment["remaining_energy_reserve_pct"].as_f64().unwrap() - 45.0).abs() < 1e-9);
        assert!((assessment["minimum_condition_margin_c"].as_f64().unwrap() - 2.4).abs() < 1e-9);
    }

    #[test]
    fn route_enumeration_limit_fails_instead_of_returning_a_partial_frontier() {
        let mut adjacency: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        for from in 0..10 {
            for to in 0..10 {
                if from == to {
                    continue;
                }
                let from_id = format!("n{from}");
                let to_id = format!("n{to}");
                adjacency.entry(from_id.clone()).or_default().push(json!({
                    "edge_id": format!("{from_id}-to-{to_id}"),
                    "to_node": to_id
                }));
            }
        }
        for outgoing in adjacency.values_mut() {
            outgoing.sort_by(|left, right| left["edge_id"].as_str().cmp(&right["edge_id"].as_str()));
        }
        let origin = "n0".to_string();
        let mut visited = BTreeSet::from([origin.clone()]);
        let mut node_path = vec![origin.clone()];
        let mut edge_path = Vec::new();
        let mut routes = Vec::new();
        let mut expansions = 0usize;
        let error = enumerate_from(
            &origin,
            "n9",
            8,
            &adjacency,
            &mut visited,
            &mut node_path,
            &mut edge_path,
            &mut routes,
            &mut expansions,
        )
        .unwrap_err();
        assert!(error.contains("route enumeration exceeded the per-mission route limit"));
        assert_eq!(routes.len(), MAX_PATHS_PER_MISSION);
    }

    #[test]
    fn unknown_fields_duplicate_identifiers_and_dangling_edges_are_rejected() {
        let mut extra = fixture();
        extra["nodes"][0]["optimistic_score"] = json!(999);
        assert!(audit(&extra).unwrap_err().contains("keys differ"));

        let mut duplicate = fixture();
        duplicate["edges"][1]["edge_id"] = duplicate["edges"][0]["edge_id"].clone();
        assert!(audit(&duplicate).unwrap_err().contains("duplicate edge_id"));

        let mut dangling = fixture();
        dangling["edges"][0]["to_node"] = json!("missing-node");
        assert!(audit(&dangling).unwrap_err().contains("edge endpoints"));
    }

    #[test]
    fn cycles_are_not_repeated_inside_candidate_routes() {
        let mut input = fixture();
        let mut back_edge = input["edges"][0].clone();
        back_edge["edge_id"] = json!("amz-return-cycle");
        back_edge["from_node"] = json!("amz-river-hub");
        back_edge["to_node"] = json!("amz-origin");
        input["edges"].as_array_mut().unwrap().push(back_edge);
        let output = audit(&input).unwrap();
        assert_eq!(mission_result(&output, "amazon-medical-cold-chain")["topological_route_count"], 3);
    }

    #[test]
    fn all_729_gate_combinations_follow_fail_closed_precedence() {
        let input = fixture();
        let mission = mission(&input, "amazon-medical-cold-chain");
        let nodes: BTreeMap<String, Value> = input["nodes"].as_array().unwrap().iter()
            .map(|node| (node["node_id"].as_str().unwrap().to_string(), node.clone())).collect();
        let mut edges: BTreeMap<String, Value> = input["edges"].as_array().unwrap().iter()
            .map(|edge| (edge["edge_id"].as_str().unwrap().to_string(), edge.clone())).collect();
        let route = Route {
            node_ids: vec!["amz-origin".to_string(), "amz-destination".to_string()],
            edge_ids: vec!["amz-aerial-direct".to_string()],
        };
        let states = ["passed", "unknown", "failed"];
        for mut encoded in 0usize..729 {
            let mut edge = edges.get("amz-aerial-direct").unwrap().clone();
            let mut any_failed = false;
            let mut any_unknown = false;
            for field in GATE_FIELDS {
                let status = states[encoded % 3];
                encoded /= 3;
                edge["gates"][field] = json!(status);
                any_failed |= status == "failed";
                any_unknown |= status == "unknown";
            }
            edges.insert("amz-aerial-direct".to_string(), edge);
            let output = assess_route(mission, &route, &nodes, &edges).unwrap();
            let expected = if any_failed {
                "blocked_hard_gate"
            } else if any_unknown {
                "needs_gate_evidence"
            } else {
                "eligible_for_human_review"
            };
            assert_eq!(output["disposition"], expected);
        }
    }

    #[test]
    fn all_64_capability_masks_enforce_mission_requirements() {
        let mut input = fixture();
        let mission_index = input["missions"].as_array().unwrap().iter()
            .position(|item| item["mission_id"] == "amazon-medical-cold-chain").unwrap();
        for field in CAPABILITY_FIELDS {
            input["missions"][mission_index]["requirements"][field] = json!(true);
        }
        let mission = &input["missions"][mission_index];
        let nodes: BTreeMap<String, Value> = input["nodes"].as_array().unwrap().iter()
            .map(|node| (node["node_id"].as_str().unwrap().to_string(), node.clone())).collect();
        let mut edges: BTreeMap<String, Value> = input["edges"].as_array().unwrap().iter()
            .map(|edge| (edge["edge_id"].as_str().unwrap().to_string(), edge.clone())).collect();
        let route = Route {
            node_ids: vec!["amz-origin".to_string(), "amz-destination".to_string()],
            edge_ids: vec!["amz-ground-direct".to_string()],
        };
        for mask in 0usize..64 {
            let mut edge = edges.get("amz-ground-direct").unwrap().clone();
            let mut all_present = true;
            for (index, field) in CAPABILITY_FIELDS.iter().enumerate() {
                let enabled = (mask & (1 << index)) != 0;
                edge["capabilities"][*field] = json!(enabled);
                all_present &= enabled;
            }
            edges.insert("amz-ground-direct".to_string(), edge);
            let output = assess_route(mission, &route, &nodes, &edges).unwrap();
            let expected = if all_present {
                "eligible_for_human_review"
            } else {
                "blocked_constraints"
            };
            assert_eq!(output["disposition"], expected, "capability mask {mask}");
        }
    }

    #[test]
    fn false_oracle_expectations_are_detected() {
        let mut input = fixture();
        input["expectations"][0]["expected_eligible_route_count"] = json!(2);
        let output = audit(&input).unwrap();
        assert_eq!(output["expected_oracle_match_count"], 5);
        let mismatch = mission_result(&output, "amazon-medical-cold-chain");
        assert_eq!(mismatch["expectation_matches"], false);
    }
}

