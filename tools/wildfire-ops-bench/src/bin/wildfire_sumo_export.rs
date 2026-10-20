use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const VERSION: &str = "0.1.0";
const CASE_IDS: [&str; 6] = [
    "TWO_ORIGINS_SHARED_BOTTLENECK_001",
    "FINITE_STORAGE_BACKPRESSURE_002",
    "ENTRY_CLOSURE_DURING_DEPARTURES_003",
    "ROUND_ROBIN_MERGE_FAIRNESS_004",
    "TOTAL_SHELTER_CAPACITY_HARD_GATE_005",
    "ACCESSIBLE_CAPACITY_HARD_GATE_006",
];
const MAX_EXPORTED_VEHICLES: u64 = 50_000;

type Check<T> = Result<T, String>;

struct RenderedCase {
    nodes: String,
    edges: String,
    routes: String,
    additional: String,
    manifest: Value,
}

fn string<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a str> {
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

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_commit_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn case_inventory(fixture: &Value) -> Check<&[Value]> {
    if fixture.get("schema_version").and_then(Value::as_str) != Some(VERSION)
        || fixture.get("benchmark_id").and_then(Value::as_str) != Some("WILDFIRE-NETWORK-001")
        || fixture.get("classification").and_then(Value::as_str) != Some("synthetic_only")
    {
        return Err("network fixture metadata mismatch".to_owned());
    }
    let cases = array(fixture, "cases", "fixture")?;
    if cases.len() != CASE_IDS.len() {
        return Err(format!("expected exactly {} network cases", CASE_IDS.len()));
    }
    for (index, case) in cases.iter().enumerate() {
        let id = string(case, "case_id", "case")?;
        if id != CASE_IDS[index] {
            return Err(format!("case inventory/order mismatch at {index}: {id}"));
        }
    }
    Ok(cases)
}

fn render_case(case: &Value, seconds_per_step: u64, speed_mps: f64) -> Check<RenderedCase> {
    let case_id = string(case, "case_id", "case")?;
    if seconds_per_step == 0 || !speed_mps.is_finite() || speed_mps <= 0.0 {
        return Err("time scale and reference speed must be finite and positive".to_owned());
    }
    let horizon = uint(case, "horizon_steps", case_id)?;
    let raw_edges = array(case, "edges", case_id)?;
    let raw_demands = array(case, "demands", case_id)?;
    let raw_sinks = array(case, "sinks", case_id)?;
    if horizon == 0 || raw_edges.is_empty() || raw_demands.is_empty() || raw_sinks.is_empty() {
        return Err(format!(
            "{case_id}: invalid horizon/edge/demand/sink inventory"
        ));
    }

    let mut edges: Vec<&Value> = raw_edges.iter().collect();
    edges.sort_by_key(|edge| edge.get("id").and_then(Value::as_str).unwrap_or(""));
    let mut edge_by_id = BTreeMap::<String, &Value>::new();
    let mut node_ids = BTreeSet::<String>::new();
    for edge in &edges {
        let id = string(edge, "id", case_id)?.to_owned();
        let from = string(edge, "from", &id)?.to_owned();
        let to = string(edge, "to", &id)?.to_owned();
        let capacity = uint(edge, "capacity_per_step", &id)?;
        let storage = uint(edge, "storage", &id)?;
        let travel = uint(edge, "travel_time_steps", &id)?;
        if id.is_empty()
            || from.is_empty()
            || to.is_empty()
            || from == to
            || capacity == 0
            || storage < capacity
            || travel == 0
        {
            return Err(format!("{case_id}: invalid edge {id}"));
        }
        if edge_by_id.insert(id.clone(), *edge).is_some() {
            return Err(format!("{case_id}: duplicate edge ID {id}"));
        }
        node_ids.insert(from);
        node_ids.insert(to);
    }

    let mut sinks: Vec<&Value> = raw_sinks.iter().collect();
    sinks.sort_by_key(|sink| sink.get("id").and_then(Value::as_str).unwrap_or(""));
    let mut sink_by_id = BTreeMap::<String, &Value>::new();
    for sink in &sinks {
        let id = string(sink, "id", case_id)?.to_owned();
        let node_id = string(sink, "node_id", &id)?;
        let _ = uint(sink, "ingress_capacity_per_step", &id)?;
        let total = uint(sink, "total_capacity", &id)?;
        let accessible = uint(sink, "accessible_capacity", &id)?;
        if accessible > total || !node_ids.contains(node_id) {
            return Err(format!("{case_id}: invalid sink {id}"));
        }
        if sink_by_id.insert(id.clone(), *sink).is_some() {
            return Err(format!("{case_id}: duplicate sink ID {id}"));
        }
    }

    let mut demands: Vec<&Value> = raw_demands.iter().collect();
    demands.sort_by_key(|demand| demand.get("id").and_then(Value::as_str).unwrap_or(""));
    let mut demand_ids = BTreeSet::new();
    let mut total_demand = 0u64;
    let mut total_accessible = 0u64;
    for demand in &demands {
        let id = string(demand, "id", case_id)?.to_owned();
        let origin = string(demand, "origin_node", &id)?;
        let sink_id = string(demand, "destination_sink_id", &id)?;
        let vehicles = uint(demand, "vehicles", &id)?;
        let accessible = uint(demand, "accessible_vehicles", &id)?;
        let departure = uint(demand, "departure_step", &id)?;
        let route = array(demand, "route_edge_ids", &id)?;
        if id.is_empty()
            || !demand_ids.insert(id.clone())
            || vehicles == 0
            || accessible > vehicles
            || departure == 0
            || route.is_empty()
        {
            return Err(format!("{case_id}: invalid demand stream {id}"));
        }
        if !sink_by_id.contains_key(sink_id) {
            return Err(format!(
                "{case_id}: demand {id} references unknown sink {sink_id}"
            ));
        }
        let route_ids: Vec<&str> = route
            .iter()
            .map(|item| {
                item.as_str()
                    .ok_or_else(|| format!("{case_id}: route IDs must be strings"))
            })
            .collect::<Check<Vec<_>>>()?;
        let first = edge_by_id.get(route_ids[0]).expect("validated edge map");
        if first.get("from").and_then(Value::as_str) != Some(origin) {
            return Err(format!("{case_id}: route {id} does not start at origin"));
        }
        for pair in route_ids.windows(2) {
            let current = edge_by_id.get(pair[0]).expect("validated edge map");
            let next = edge_by_id.get(pair[1]).expect("validated edge map");
            if current.get("to").and_then(Value::as_str) != next.get("from").and_then(Value::as_str)
            {
                return Err(format!("{case_id}: route {id} is disconnected"));
            }
        }
        let last = edge_by_id
            .get(*route_ids.last().expect("nonempty route"))
            .expect("edge map");
        if last.get("to").and_then(Value::as_str)
            != sink_by_id
                .get(sink_id)
                .and_then(|sink| sink.get("node_id"))
                .and_then(Value::as_str)
        {
            return Err(format!("{case_id}: route {id} does not reach its sink"));
        }
        total_demand = total_demand
            .checked_add(vehicles)
            .ok_or_else(|| format!("{case_id}: demand total overflow"))?;
        total_accessible = total_accessible
            .checked_add(accessible)
            .ok_or_else(|| format!("{case_id}: accessible total overflow"))?;
    }
    if total_demand > MAX_EXPORTED_VEHICLES {
        return Err(format!(
            "{case_id}: demand exceeds export guardrail {MAX_EXPORTED_VEHICLES}"
        ));
    }

    let node_order: Vec<String> = node_ids.into_iter().collect();
    let positions: BTreeMap<String, (u64, u64)> = node_order
        .iter()
        .enumerate()
        .map(|(i, node)| {
            (
                node.clone(),
                (((i % 8) as u64) * 100, ((i / 8) as u64) * 100),
            )
        })
        .collect();

    let mut nodes_xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<nodes>\n");
    for node in &node_order {
        let (x, y) = positions[node];
        nodes_xml.push_str(&format!(
            "  <node id=\"{}\" x=\"{}\" y=\"{}\" type=\"priority\"/>\n",
            xml_escape(node),
            x,
            y
        ));
    }
    nodes_xml.push_str("</nodes>\n");

    let mut edges_xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<edges>\n");
    let mut edge_mapping = Vec::<Value>::new();
    let mut closure_mapping = Vec::<Value>::new();
    let mut triggers_by_closure = BTreeMap::<String, BTreeSet<String>>::new();
    for demand in &demands {
        let id = string(demand, "id", case_id)?;
        let route: Vec<&str> = array(demand, "route_edge_ids", id)?
            .iter()
            .map(|item| item.as_str().expect("validated route"))
            .collect();
        for (i, edge_id) in route.iter().enumerate() {
            let edge = edge_by_id.get(*edge_id).expect("validated edge");
            if edge.get("closed_from_step").is_some_and(|v| !v.is_null()) {
                let trigger = if i > 0 { route[i - 1] } else { *edge_id };
                triggers_by_closure
                    .entry((*edge_id).to_owned())
                    .or_default()
                    .insert(trigger.to_owned());
            }
        }
    }
    for edge in &edges {
        let id = string(edge, "id", case_id)?;
        let from = string(edge, "from", id)?;
        let to = string(edge, "to", id)?;
        let capacity = uint(edge, "capacity_per_step", id)?;
        let storage = uint(edge, "storage", id)?;
        let travel = uint(edge, "travel_time_steps", id)?;
        let closed = match edge.get("closed_from_step") {
            Some(Value::Null) => None,
            Some(Value::Number(number)) => number.as_u64(),
            _ => return Err(format!("{case_id}.{id}: missing/invalid closed_from_step")),
        };
        let length_m = speed_mps * travel as f64 * seconds_per_step as f64;
        edges_xml.push_str(&format!(
            "  <edge id=\"{}\" from=\"{}\" to=\"{}\" numLanes=\"1\" speed=\"{:.3}\" length=\"{:.3}\" priority=\"1\"/>\n",
            xml_escape(id), xml_escape(from), xml_escape(to), speed_mps, length_m
        ));
        edge_mapping.push(json!({
            "id": id,
            "from": from,
            "to": to,
            "capacity_per_step": capacity,
            "storage": storage,
            "travel_time_steps": travel,
            "speed_mps": speed_mps,
            "length_m": length_m,
            "closed_from_step": closed,
            "closure_begin_seconds": closed.map(|step| (step - 1) * seconds_per_step)
        }));
        if let Some(close_step) = closed {
            closure_mapping.push(json!({
                "edge_id": id,
                "closed_from_step": close_step,
                "begin_seconds": (close_step - 1) * seconds_per_step,
                "end_seconds": (horizon + 1) * seconds_per_step,
                "rerouter_trigger_edges": triggers_by_closure.get(id)
                    .map(|set| set.iter().cloned().collect::<Vec<_>>())
                    .unwrap_or_default(),
                "semantics": "SUMO closingReroute can reroute vehicles; not equivalent to fixed-route entry blocking"
            }));
        }
    }
    edges_xml.push_str("</edges>\n");

    let mut routes_xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<routes>\n");
    routes_xml.push_str("  <vType id=\"standard\" vClass=\"passenger\"/>\n");
    routes_xml.push_str("  <vType id=\"accessible\" vClass=\"passenger\"><param key=\"symtropy.accessible\" value=\"true\"/></vType>\n");
    let mut exported_accessible = 0u64;
    for demand in &demands {
        let id = string(demand, "id", case_id)?;
        let vehicles = uint(demand, "vehicles", id)?;
        let accessible = uint(demand, "accessible_vehicles", id)?;
        let departure = uint(demand, "departure_step", id)?;
        let depart_seconds = (departure - 1) * seconds_per_step;
        let route_ids: Vec<&str> = array(demand, "route_edge_ids", id)?
            .iter()
            .map(|item| item.as_str().expect("validated route"))
            .collect();
        let route_id = format!("route_{id}");
        routes_xml.push_str(&format!(
            "  <route id=\"{}\" edges=\"{}\"/>\n",
            xml_escape(&route_id),
            xml_escape(&route_ids.join(" "))
        ));
        for number in 0..vehicles {
            let vtype = if number < accessible {
                exported_accessible += 1;
                "accessible"
            } else {
                "standard"
            };
            routes_xml.push_str(&format!(
                "  <vehicle id=\"{}_{:05}\" type=\"{}\" route=\"{}\" depart=\"{}\"/>\n",
                xml_escape(id),
                number,
                vtype,
                xml_escape(&route_id),
                depart_seconds
            ));
        }
    }
    routes_xml.push_str("</routes>\n");

    let mut additional = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<additional>\n  <!-- Closing reroutes may alter fixed routes; see manifest semantic gaps. -->\n",
    );
    for closure in &closure_mapping {
        let edge_id = closure["edge_id"].as_str().expect("closure ID");
        let trigger_edges: Vec<String> = closure["rerouter_trigger_edges"]
            .as_array()
            .expect("trigger array")
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect();
        if trigger_edges.is_empty() {
            continue;
        }
        let begin = uint(closure, "begin_seconds", case_id)?;
        let end = (horizon + 1) * seconds_per_step;
        additional.push_str(&format!(
            "  <rerouter id=\"rr_{}\" edges=\"{}\">\n    <interval begin=\"{}\" end=\"{}\">\n      <closingReroute id=\"{}\" disallow=\"all\"/>\n    </interval>\n  </rerouter>\n",
            xml_escape(edge_id), xml_escape(&trigger_edges.join(" ")), begin, end, xml_escape(edge_id)
        ));
    }
    additional.push_str("</additional>\n");

    let file_hashes = json!({
        "nodes.nod.xml": sha256(nodes_xml.as_bytes()),
        "edges.edg.xml": sha256(edges_xml.as_bytes()),
        "demand.rou.xml": sha256(routes_xml.as_bytes()),
        "closures.add.xml": sha256(additional.as_bytes())
    });
    let manifest = json!({
        "result": "pass",
        "adapter_version": VERSION,
        "benchmark_id": "WILDFIRE-NETWORK-001",
        "case_id": case_id,
        "classification": "synthetic_only",
        "sumo_files": {
            "nodes": "nodes.nod.xml",
            "edges": "edges.edg.xml",
            "demand": "demand.rou.xml",
            "closures": "closures.add.xml"
        },
        "suggested_netconvert_command": "netconvert --node-files nodes.nod.xml --edge-files edges.edg.xml --output-file network.net.xml",
        "suggested_sumo_command": "sumo -n network.net.xml -r demand.rou.xml --additional-files closures.add.xml",
        "seconds_per_step": seconds_per_step,
        "reference_speed_mps": speed_mps,
        "edge_length_formula": "travel_time_steps * seconds_per_step * reference_speed_mps",
        "exported_vehicles": total_demand,
        "exported_accessible_vehicle_labels": exported_accessible,
        "network_nodes": node_order.len(),
        "network_edges": edges.len(),
        "demand_streams": demands.len(),
        "total_demand_vehicles": total_demand,
        "accessible_demand_vehicles": total_accessible,
        "edge_mapping_sidecar": edge_mapping,
        "closure_mapping_sidecar": closure_mapping,
        "sinks": sinks.iter().map(|sink| json!({
            "id": sink["id"],
            "node_id": sink["node_id"],
            "ingress_capacity_per_step": sink["ingress_capacity_per_step"],
            "total_capacity": sink["total_capacity"],
            "accessible_capacity": sink["accessible_capacity"]
        })).collect::<Vec<_>>(),
        "files_sha256": file_hashes,
        "semantic_gaps": [
            "Abstract vehicles-per-step capacities and finite storage are sidecar metadata; SUMO lane/speed flow is not calibrated to enforce equivalent limits.",
            "Sink ingress and total/accessibility constraints are sidecar metadata, not enforced by this adapter in SUMO.",
            "Accessible vehicle type is only a parameter label; it is not an accessibility enforcement model.",
            "SUMO closingReroute may change a vehicle's route; the source benchmark blocks entry while retaining fixed routes.",
            "Coordinates and lengths are synthetic; no geospatial fidelity is implied."
        ],
        "claim_boundary": "structural export only; no matched SUMO simulation was executed"
    });
    Ok(RenderedCase {
        nodes: nodes_xml,
        edges: edges_xml,
        routes: routes_xml,
        additional,
        manifest,
    })
}

fn write_case(root: &Path, case: &Value, rendered: &RenderedCase) -> Check<Value> {
    let id = string(case, "case_id", "case")?;
    let directory = root.join(id);
    fs::create_dir_all(&directory).map_err(|error| format!("create {id}: {error}"))?;
    let files = [
        ("nodes.nod.xml", rendered.nodes.as_str()),
        ("edges.edg.xml", rendered.edges.as_str()),
        ("demand.rou.xml", rendered.routes.as_str()),
        ("closures.add.xml", rendered.additional.as_str()),
    ];
    for (name, contents) in files {
        fs::write(directory.join(name), contents)
            .map_err(|error| format!("write {id}/{name}: {error}"))?;
    }
    let manifest = serde_json::to_vec_pretty(&rendered.manifest)
        .map_err(|error| format!("serialize {id} manifest: {error}"))?;
    fs::write(directory.join("manifest.json"), &manifest)
        .map_err(|error| format!("write {id}/manifest.json: {error}"))?;
    Ok(json!({
        "case_id": id,
        "directory": id,
        "manifest_sha256": sha256(&manifest),
        "file_sha256": rendered.manifest["files_sha256"]
    }))
}

fn run() -> Check<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 6 {
        return Err(
            "usage: wildfire_sumo_export <fixture.json> <output-dir> <source-commit> <seconds-per-step> <reference-speed-mps>"
                .to_owned()
        );
    }
    if !valid_commit_sha(&args[3]) {
        return Err("source commit must be a 40-character hexadecimal SHA".to_owned());
    }
    let seconds_per_step = args[4]
        .parse::<u64>()
        .map_err(|_| "seconds-per-step must be a positive integer".to_owned())?;
    let speed_mps = args[5]
        .parse::<f64>()
        .map_err(|_| "reference-speed-mps must be a finite positive number".to_owned())?;
    if seconds_per_step == 0 || !speed_mps.is_finite() || speed_mps <= 0.0 {
        return Err("time scale and reference speed must be finite and positive".to_owned());
    }
    let bytes = fs::read(&args[1]).map_err(|error| format!("read fixture: {error}"))?;
    let fixture: Value =
        serde_json::from_slice(&bytes).map_err(|error| format!("parse fixture: {error}"))?;
    let cases = case_inventory(&fixture)?;
    let output_root = PathBuf::from(&args[2]);
    fs::create_dir_all(&output_root).map_err(|error| format!("create output root: {error}"))?;
    let mut outputs = Vec::new();
    for case in cases {
        let rendered = render_case(case, seconds_per_step, speed_mps)?;
        outputs.push(write_case(&output_root, case, &rendered)?);
    }
    let receipt = json!({
        "result": "pass",
        "adapter_version": VERSION,
        "benchmark_id": "WILDFIRE-NETWORK-001",
        "scenario_count": outputs.len(),
        "source_commit": args[3],
        "fixture_sha256": sha256(&bytes),
        "seconds_per_step": seconds_per_step,
        "reference_speed_mps": speed_mps,
        "cases": outputs,
        "claim_boundary": "structural SUMO export only; this is not a matched simulation"
    });
    let receipt_bytes = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| format!("serialize receipt: {error}"))?;
    fs::write(output_root.join("index.json"), &receipt_bytes)
        .map_err(|error| format!("write index.json: {error}"))?;
    println!("{}", String::from_utf8_lossy(&receipt_bytes));
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("SUMO structural export failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../benchmarks/fixtures/WILDFIRE_NETWORK_FLOW_V0_1.json"
        ))
        .expect("fixture parses")
    }

    #[test]
    fn renders_all_frozen_cases_and_vehicle_counts() {
        let fixture = fixture();
        let cases = case_inventory(&fixture).expect("case inventory");
        assert_eq!(cases.len(), 6);
        for case in cases {
            let rendered = render_case(case, 60, 13.9).expect("render");
            let expected: u64 = array(case, "demands", "case")
                .expect("demands")
                .iter()
                .map(|demand| uint(demand, "vehicles", "demand").expect("vehicles"))
                .sum();
            assert_eq!(
                rendered.manifest["exported_vehicles"].as_u64(),
                Some(expected)
            );
            assert!(rendered.nodes.contains("<node "));
            assert!(rendered.edges.contains("<edge "));
            assert!(rendered.routes.contains("<vehicle "));
            assert!(rendered.manifest["semantic_gaps"].as_array().is_some());
        }
    }

    #[test]
    fn edge_and_demand_order_do_not_change_export() {
        let original_fixture = fixture();
        let original_case = &original_fixture["cases"][0];
        let original = render_case(original_case, 60, 13.9).expect("original");
        let mut reordered_fixture = original_fixture.clone();
        reordered_fixture["cases"][0]["edges"]
            .as_array_mut()
            .expect("edges")
            .reverse();
        reordered_fixture["cases"][0]["demands"]
            .as_array_mut()
            .expect("demands")
            .reverse();
        let reordered = render_case(&reordered_fixture["cases"][0], 60, 13.9).expect("reordered");
        assert_eq!(original.nodes, reordered.nodes);
        assert_eq!(original.edges, reordered.edges);
        assert_eq!(original.routes, reordered.routes);
        assert_eq!(original.additional, reordered.additional);
        assert_eq!(original.manifest, reordered.manifest);
    }

    #[test]
    fn closure_export_marks_route_semantic_difference() {
        let fixture = fixture();
        let rendered = render_case(&fixture["cases"][2], 60, 13.9).expect("closure case");
        assert!(
            rendered
                .additional
                .contains("<closingReroute id=\"A_BRIDGE\"")
        );
        assert!(rendered.additional.contains("begin=\"180\""));
        assert!(
            rendered.manifest["semantic_gaps"]
                .as_array()
                .expect("gaps")
                .iter()
                .any(|gap| gap.as_str().unwrap_or("").contains("closingReroute"))
        );
    }

    #[test]
    fn accessibility_is_only_a_label_in_this_adapter() {
        let fixture = fixture();
        let rendered = render_case(&fixture["cases"][0], 60, 13.9).expect("case");
        let tagged = rendered.routes.matches("type=\"accessible\"").count() as u64;
        let expected: u64 = array(&fixture["cases"][0], "demands", "case")
            .expect("demands")
            .iter()
            .map(|d| uint(d, "accessible_vehicles", "demand").expect("count"))
            .sum();
        assert_eq!(tagged, expected);
        assert!(
            rendered.manifest["semantic_gaps"]
                .as_array()
                .expect("gaps")
                .iter()
                .any(|gap| gap
                    .as_str()
                    .unwrap_or("")
                    .contains("not an accessibility enforcement model"))
        );
    }

    #[test]
    fn xml_is_escaped_and_invalid_scales_are_rejected() {
        assert_eq!(xml_escape("a&b<c>\"d'"), "a&amp;b&lt;c&gt;&quot;d&apos;");
        let fixture = fixture();
        let case = &fixture["cases"][0];
        assert!(render_case(case, 0, 13.9).is_err());
        assert!(render_case(case, 60, f64::NAN).is_err());
        assert!(render_case(case, 60, 0.0).is_err());
    }
}
