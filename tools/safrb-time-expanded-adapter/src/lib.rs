#![forbid(unsafe_code)]

use serde_json::Value;
use sha2::{Digest, Sha256};
use single_commodity_flow_kernel::{FlowEdge, FlowGraph, FlowNode, FlowResult};
use std::collections::{BTreeMap, BTreeSet};

const SCHEMA: &str = "safrb-time-expanded-network-v0.1";
const HYPOTHETICAL: &str = "hypothetical_counterfactual_not_admitted";
const MAX_COUNTERFACTUAL_STEPS: u64 = 1_000_000;
const CLAIM_CEILING: &str = "Synthetic mathematical feasibility for the declared single-commodity time-expanded graph only; no authenticated evidence, physical asset qualification, legal access, dispatch authority or actual beneficiary-service claim.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowDisposition {
    GuaranteedWithinDeclaredLowerEnvelope,
    IndeterminateNotGuaranteed,
    InfeasibleEvenAtDeclaredUpperEnvelope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceDisposition {
    GuaranteedWithinDeclaredLowerEnvelope,
    IndeterminateNotGuaranteed,
    InfeasibleEvenAtDeclaredUpperEnvelope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowResult {
    pub obligation_id: String,
    pub time_index: u64,
    pub interval: String,
    pub required: u64,
    pub lower_delivered: u64,
    pub upper_delivered: u64,
    pub disposition: WindowDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafrbCertificate {
    pub case_id: String,
    pub scenario_sha256: String,
    pub claim_scope: String,
    pub commodity_ref: String,
    pub capacity_unit_ref: String,
    pub quantity_boundary_ref: String,
    pub lower: FlowResult,
    pub upper: FlowResult,
    pub windows: Vec<WindowResult>,
    pub disposition: ServiceDisposition,
    pub counterfactual_only: bool,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestorationThreshold {
    pub edge_id: String,
    pub physical_resource_id: String,
    pub time_index: u64,
    pub base_lower: u64,
    pub hypothetical_lower: u64,
    pub additional_capacity: u64,
    pub lower_flow: u64,
    pub lower_cut: u64,
    pub windows: Vec<WindowResult>,
    pub evidence_status: &'static str,
    pub base_scenario_sha256: String,
    pub hypothetical_scenario_sha256: String,
}

#[derive(Debug, Clone)]
struct Node {
    id: String,
    kind: String,
    time: u64,
}

#[derive(Debug, Clone)]
struct Edge {
    id: String,
    from: String,
    to: String,
    lower: u64,
    upper: u64,
    resource: String,
    time: u64,
}

#[derive(Debug, Clone)]
struct Obligation {
    id: String,
    time: u64,
    interval: String,
    required: u64,
    edge_id: String,
}

#[derive(Debug, Clone)]
struct Parsed {
    case_id: String,
    scope: String,
    commodity: String,
    unit: String,
    boundary: String,
    source: String,
    sink: String,
    nodes: BTreeMap<String, Node>,
    edges: Vec<Edge>,
    obligations: Vec<Obligation>,
    scenario_sha256: String,
    hypothetical: bool,
}

fn err(path: &str, message: &str) -> String {
    format!("{path}: {message}")
}

fn get<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    value.as_object().and_then(|o| o.get(key))
        .ok_or_else(|| err(path, &format!("missing property {key}")))
}

fn strv(value: &Value, path: &str) -> Result<String, String> {
    value.as_str().filter(|s| !s.trim().is_empty()).map(str::to_owned)
        .ok_or_else(|| err(path, "must be a non-empty string"))
}

fn u64v(value: &Value, path: &str) -> Result<u64, String> {
    value.as_u64().ok_or_else(|| err(path, "must be a non-negative integer"))
}

fn sfield(value: &Value, key: &str, path: &str) -> Result<String, String> {
    strv(get(value, key, path)?, &format!("{path}.{key}"))
}

fn nfield(value: &Value, key: &str, path: &str) -> Result<u64, String> {
    u64v(get(value, key, path)?, &format!("{path}.{key}"))
}

fn array<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a [Value], String> {
    get(value, key, path)?.as_array().map(Vec::as_slice)
        .ok_or_else(|| err(&format!("{path}.{key}"), "must be an array"))
}

fn canonical(value: &Value) -> Result<String, String> {
    match value {
        Value::Null => Ok("null".into()),
        Value::Bool(v) => Ok(v.to_string()),
        Value::Number(v) => Ok(v.to_string()),
        Value::String(v) => serde_json::to_string(v).map_err(|e| e.to_string()),
        Value::Array(rows) => {
            let parts = rows.iter().map(canonical).collect::<Result<Vec<_>, _>>()?;
            Ok(format!("[{}]", parts.join(",")))
        }
        Value::Object(map) => {
            let mut rows = map.iter().collect::<Vec<_>>();
            rows.sort_by(|a, b| a.0.cmp(b.0));
            let mut parts = Vec::new();
            for (key, value) in rows {
                parts.push(format!("{}:{}", serde_json::to_string(key).map_err(|e| e.to_string())?, canonical(value)?));
            }
            Ok(format!("{{{}}}", parts.join(",")))
        }
    }
}

fn digest(value: &Value) -> Result<String, String> {
    let bytes = canonical(value)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes.as_bytes())))
}

fn half_open(value: &str, path: &str, index: u64) -> Result<(), String> {
    let body = value.strip_prefix('[').and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| err(path, "must use half-open [start,end) syntax"))?;
    let (start, end) = body.split_once(',').ok_or_else(|| err(path, "missing interval comma"))?;
    let start = start.parse::<u64>().map_err(|_| err(path, "invalid interval start"))?;
    let end = end.parse::<u64>().map_err(|_| err(path, "invalid interval end"))?;
    if start != index || end != index.saturating_add(1) {
        return Err(err(path, "interval must match the declared one-step time index"));
    }
    Ok(())
}

fn parse(value: &Value, allow_hypothetical: bool) -> Result<Parsed, String> {
    if sfield(value, "schema_version", "$")? != SCHEMA {
        return Err(err("$.schema_version", "unsupported model/schema; multi-commodity and process-coupled graphs are not supported"));
    }
    if sfield(value, "evidence_maturity", "$")? != "synthetic_known_truth" {
        return Err(err("$.evidence_maturity", "v0.1 accepts synthetic known-truth fixtures only"));
    }
    let case_id = sfield(value, "case_id", "$")?;
    let scope = sfield(value, "claim_scope", "$")?;
    if scope != "FUEL-DELIVERABILITY" {
        return Err(err("$.claim_scope", "v0.1 supports FUEL-DELIVERABILITY only"));
    }
    if scope != "FUEL-DELIVERABILITY" {
        return Err(err("$.claim_scope", "v0.1 adapter supports FUEL-DELIVERABILITY only"));
    }

    let timebase = get(value, "timebase", "$")?;
    if sfield(timebase, "kind", "$.timebase")? != "relative_discrete_intervals"
        || sfield(timebase, "interval_semantics", "$.timebase")? != "half_open"
        || sfield(timebase, "origin", "$.timebase")? != "scenario_start"
    {
        return Err(err("$.timebase", "unsupported temporal profile"));
    }
    let seconds = nfield(timebase, "interval_seconds", "$.timebase")?;
    if seconds == 0 || seconds > 86_400 {
        return Err(err("$.timebase.interval_seconds", "outside supported range"));
    }

    let commodity_obj = get(value, "commodity", "$")?;
    let commodity = sfield(commodity_obj, "product_id", "$.commodity")?;
    let unit = sfield(commodity_obj, "quantity_unit", "$.commodity")?;
    let unit_basis = sfield(commodity_obj, "unit_basis", "$.commodity")?;
    let source = sfield(value, "source_node", "$")?;
    let sink = sfield(value, "sink_node", "$")?;
    if source == sink { return Err(err("$", "source and sink must differ")); }
    let boundary = format!("safrb:{scope}:{unit_basis}:{source}->{sink}");

    let mut nodes = BTreeMap::new();
    for (i, row) in array(value, "nodes", "$")?.iter().enumerate() {
        let path = format!("$.nodes[{i}]");
        let id = sfield(row, "id", &path)?;
        let node = Node { id: id.clone(), kind: sfield(row, "kind", &path)?, time: nfield(row, "time_index", &path)? };
        if nodes.insert(id.clone(), node).is_some() { return Err(err(&path, "duplicate node ID")); }
    }
    if nodes.len() < 2 || nodes.len() > 4096 { return Err(err("$.nodes", "node count outside 2..=4096")); }
    let src = nodes.get(&source).ok_or_else(|| err("$.source_node", "unknown source"))?;
    let snk = nodes.get(&sink).ok_or_else(|| err("$.sink_node", "unknown sink"))?;
    if src.kind != "source" || src.time != 0 || snk.kind != "sink" || snk.time == 0 {
        return Err(err("$", "source/sink node kinds or times are invalid"));
    }

    let mut edges = Vec::new();
    let mut edge_ids = BTreeSet::new();
    for (i, row) in array(value, "edges", "$")?.iter().enumerate() {
        let path = format!("$.edges[{i}]");
        let id = sfield(row, "id", &path)?;
        if !edge_ids.insert(id.clone()) { return Err(err(&path, "duplicate edge ID")); }
        let from = sfield(row, "from", &path)?;
        let to = sfield(row, "to", &path)?;
        if from == to { return Err(err(&path, "self-loop unsupported")); }
        let from_node = nodes.get(&from).ok_or_else(|| err(&path, "unknown source endpoint"))?;
        let to_node = nodes.get(&to).ok_or_else(|| err(&path, "unknown target endpoint"))?;
        let lower = nfield(row, "capacity_lower", &path)?;
        let upper = nfield(row, "capacity_upper", &path)?;
        if lower > upper { return Err(err(&path, "lower capacity exceeds upper capacity")); }
        let edge_unit = sfield(row, "unit", &path)?;
        if edge_unit != unit { return Err(err(&path, "unit differs from scenario commodity unit")); }
        let time = nfield(row, "time_index", &path)?;
        if time != from_node.time || to_node.time < time || to_node.time > time.saturating_add(1) {
            return Err(err(&path, "time-expanded edge chronology is invalid"));
        }
        if from_node.kind == "resource_output" && to_node.kind == "route_arrival" && to_node.time != time.saturating_add(1) {
            return Err(err(&path, "route transit must advance one interval"));
        }
        edges.push(Edge { id, from, to, lower, upper, resource: sfield(row, "resource_id", &path)?, time });
    }
    if edges.is_empty() || edges.len() > 65_536 { return Err(err("$.edges", "edge count outside 1..=65536")); }

    let opening = get(value, "total_opening_stock", "$")?;
    if sfield(opening, "unit", "$.total_opening_stock")? != unit {
        return Err(err("$.total_opening_stock.unit", "unit mismatch"));
    }
    let opening_quantity = nfield(opening, "quantity", "$.total_opening_stock")?;
    let source_ids = array(opening, "source_ids", "$.total_opening_stock")?.iter()
        .map(|v| strv(v, "$.total_opening_stock.source_ids[]")).collect::<Result<Vec<_>, _>>()?;
    let expected_ids = source_ids.iter().cloned().collect::<BTreeSet<_>>();
    if source_ids.is_empty() || expected_ids.len() != source_ids.len() {
        return Err(err("$.total_opening_stock.source_ids", "must be non-empty and unique"));
    }
    let source_edges = edges.iter().filter(|e| e.from == source).collect::<Vec<_>>();
    let stock = source_edges.iter().copied().filter(|e| nodes.get(&e.to).is_some_and(|n| n.kind == "inventory")).collect::<Vec<_>>();
    if source_edges.len() != stock.len() {
        return Err(err("$.edges", "v0.1 source may connect only to opening inventory nodes"));
    }
    let actual_ids = stock.iter().map(|e| e.resource.clone()).collect::<BTreeSet<_>>();
    if stock.len() != source_ids.len() || actual_ids != expected_ids {
        return Err(err("$.total_opening_stock", "source inventory edges do not reconcile to source identities"));
    }
    let stock_sum = stock.iter().try_fold(0_u64, |sum, edge| sum.checked_add(edge.lower).ok_or_else(|| err("$.total_opening_stock", "stock overflow")))?;
    let stock_upper = stock.iter().try_fold(0_u64, |sum, edge| sum.checked_add(edge.upper).ok_or_else(|| err("$.total_opening_stock", "opening-stock upper bound overflow")))?;
    if stock_sum != opening_quantity || stock_upper != opening_quantity {
        return Err(err("$.total_opening_stock", "exact opening-stock lower and upper capacities must both reconcile"));
    }

    let aliases = array(value, "resource_aliases", "$")?;
    if aliases.is_empty() { return Err(err("$.resource_aliases", "must not be empty")); }
    let mut logicals = BTreeSet::new();
    let mut physical_gates = BTreeMap::<String, BTreeSet<String>>::new();
    let mut all_gates = BTreeSet::new();
    for (i, row) in aliases.iter().enumerate() {
        let path = format!("$.resource_aliases[{i}]");
        let logical = sfield(row, "logical_route_id", &path)?;
        let physical = sfield(row, "physical_resource_id", &path)?;
        if !logicals.insert(logical) { return Err(err(&path, "duplicate logical route")); }
        let gate_ids = array(row, "capacity_gate_edge_ids", &path)?.iter()
            .map(|v| strv(v, &format!("{path}.capacity_gate_edge_ids[]"))).collect::<Result<BTreeSet<_>, _>>()?;
        if gate_ids.is_empty() { return Err(err(&path, "empty physical gate set")); }
        for gate_id in &gate_ids {
            let gate = edges.iter().find(|e| &e.id == gate_id).ok_or_else(|| err(&path, "unknown gate edge"))?;
            if gate.resource != physical { return Err(err(&path, "alias points to a different physical resource")); }
            if nodes.get(&gate.from).map_or(true, |n| n.kind != "resource_input")
                || nodes.get(&gate.to).map_or(true, |n| n.kind != "resource_output") {
                return Err(err(&path, "gate must connect resource_input to resource_output"));
            }
            all_gates.insert(gate_id.clone());
        }
        if let Some(existing) = physical_gates.get(&physical) {
            if existing != &gate_ids { return Err(err(&path, "aliases for one physical resource disagree on gate set")); }
        } else {
            physical_gates.insert(physical, gate_ids);
        }
    }
    let actual_gate_ids = edges.iter().filter(|e| physical_gates.contains_key(&e.resource))
        .map(|e| e.id.clone()).collect::<BTreeSet<_>>();
    if actual_gate_ids != all_gates { return Err(err("$.resource_aliases", "incomplete, duplicate or unaliased physical resource gates")); }
    let mut gate_times = BTreeSet::new();
    for gate in edges.iter().filter(|e| all_gates.contains(&e.id)) {
        if !gate_times.insert((gate.resource.clone(), gate.time)) { return Err(err("$.resource_aliases", "more than one gate for a physical resource/time")); }
    }

    let mut route_ids = BTreeSet::new();
    for (i, row) in array(value, "route_transit", "$")?.iter().enumerate() {
        let path = format!("$.route_transit[{i}]");
        let id = sfield(row, "route_id", &path)?;
        if !route_ids.insert(id) { return Err(err(&path, "duplicate route ID")); }
        if nfield(row, "transit_intervals", &path)? != 1 {
            return Err(err(&path, "v0.1 adapter supports exactly one transit interval"));
        }
        let physical_resources = array(row, "physical_resources", &path)?;
        if physical_resources.is_empty() { return Err(err(&path, "physical_resources must not be empty")); }
        for resource in physical_resources {
            let resource = strv(resource, &format!("{path}.physical_resources[]"))?;
            if !physical_gates.contains_key(&resource) { return Err(err(&path, "route references an unknown physical resource")); }
        }
    }
    if route_ids != logicals { return Err(err("$.route_transit", "route IDs do not match logical aliases")); }

    let mut evidence_ids = BTreeSet::new();
    let mut evidence = BTreeMap::<(String, u64), (u64, u64)>::new();
    let mut hypothetical = false;
    for (i, row) in array(value, "capacity_evidence", "$")?.iter().enumerate() {
        let path = format!("$.capacity_evidence[{i}]");
        let id = sfield(row, "evidence_id", &path)?;
        if !evidence_ids.insert(id) { return Err(err(&path, "duplicate evidence ID")); }
        let physical = sfield(row, "physical_resource_id", &path)?;
        let time = nfield(row, "time_index", &path)?;
        let lower = nfield(row, "lower", &path)?;
        let upper = nfield(row, "upper", &path)?;
        if lower > upper || sfield(row, "unit", &path)? != unit { return Err(err(&path, "invalid capacity evidence quantity")); }
        let status = sfield(row, "status", &path)?;
        if status == HYPOTHETICAL && allow_hypothetical {
            hypothetical = true;
        } else if status != "synthetic_admitted" {
            return Err(err(&path, "capacity evidence is not admissible in this evaluation mode"));
        }
        half_open(&sfield(row, "valid_interval", &path)?, &format!("{path}.valid_interval"), time)?;
        if evidence.insert((physical, time), (lower, upper)).is_some() { return Err(err(&path, "duplicate resource/time evidence")); }
    }
    for gate in edges.iter().filter(|e| all_gates.contains(&e.id)) {
        if evidence.get(&(gate.resource.clone(), gate.time)) != Some(&(gate.lower, gate.upper)) {
            return Err(err("$.capacity_evidence", "missing or inconsistent physical capacity evidence"));
        }
    }
    if evidence.len() != all_gates.len() {
        return Err(err("$.capacity_evidence", "unreferenced capacity evidence"));
    }

    let mut obligations = Vec::new();
    let mut obligation_ids = BTreeSet::new();
    let mut obligation_times = BTreeSet::new();
    for (i, row) in array(value, "service_obligations", "$")?.iter().enumerate() {
        let path = format!("$.service_obligations[{i}]");
        let id = sfield(row, "obligation_id", &path)?;
        if !obligation_ids.insert(id.clone()) { return Err(err(&path, "duplicate obligation ID")); }
        let node_id = sfield(row, "node_id", &path)?;
        let time = nfield(row, "time_index", &path)?;
        let required = nfield(row, "required_lower", &path)?;
        if required == 0 || sfield(row, "unit", &path)? != unit { return Err(err(&path, "invalid service floor or unit")); }
        let interval = sfield(row, "interval", &path)?;
        half_open(&interval, &format!("{path}.interval"), time)?;
        let node = nodes.get(&node_id).ok_or_else(|| err(&path, "unknown service node"))?;
        if node.kind != "beneficiary_service" || node.time != time || !obligation_times.insert(time) {
            return Err(err(&path, "service node/time mismatch or duplicate window"));
        }
        let mut matching = edges.iter().filter(|e| e.from == node_id && e.to == sink);
        let service_edge = matching.next().ok_or_else(|| err(&path, "missing sink obligation edge"))?;
        if matching.next().is_some() || service_edge.lower != required || service_edge.upper != required {
            return Err(err(&path, "sink edge must uniquely encode the declared service floor"));
        }
        obligations.push(Obligation { id, time, interval, required, edge_id: service_edge.id.clone() });
    }
    if obligations.is_empty() || edges.iter().filter(|e| e.to == sink).count() != obligations.len() {
        return Err(err("$.service_obligations", "sink edges and declared obligations must match exactly"));
    }
    obligations.sort_by_key(|o| (o.time, o.id.clone()));

    Ok(Parsed {
        case_id,
        scope,
        commodity,
        unit,
        boundary,
        source,
        sink,
        nodes,
        edges,
        obligations,
        scenario_sha256: digest(value)?,
        hypothetical,
    })
}

fn solve(parsed: &Parsed, upper: bool) -> Result<FlowResult, String> {
    let nodes = parsed.nodes.values().map(|n| FlowNode::new(n.id.clone())).collect::<Vec<_>>();
    let edges = parsed.edges.iter().map(|e| {
        let capacity = if upper { e.upper } else { e.lower };
        let metadata = format!("safrb-v0.1/{}/{}/t{}", parsed.scope, e.resource, e.time);
        FlowEdge::new(e.id.clone(), e.from.clone(), e.to.clone(), capacity, Some(metadata))
    }).collect::<Vec<_>>();
    let graph = FlowGraph {
        commodity_ref: parsed.commodity.clone(),
        capacity_unit_ref: parsed.unit.clone(),
        quantity_boundary_ref: parsed.boundary.clone(),
        nodes,
        edges,
        source: parsed.source.clone(),
        sink: parsed.sink.clone(),
    };
    graph.solve().map_err(|e| e.to_string())
}

fn edge_flow(result: &FlowResult) -> BTreeMap<String, u64> {
    result.edge_flows.iter().map(|e| (e.id.clone(), e.flow)).collect()
}

fn verdict(lower: u64, upper: u64, required: u64) -> WindowDisposition {
    if lower >= required {
        WindowDisposition::GuaranteedWithinDeclaredLowerEnvelope
    } else if upper >= required {
        WindowDisposition::IndeterminateNotGuaranteed
    } else {
        WindowDisposition::InfeasibleEvenAtDeclaredUpperEnvelope
    }
}

fn evaluate_internal(value: &Value, allow_hypothetical: bool) -> Result<SafrbCertificate, String> {
    let p = parse(value, allow_hypothetical)?;
    let lower = solve(&p, false)?;
    let upper = solve(&p, true)?;
    let lf = edge_flow(&lower);
    let uf = edge_flow(&upper);
    let mut windows = Vec::new();
    for obligation in &p.obligations {
        let lo = *lf.get(&obligation.edge_id).ok_or_else(|| err("$.certificate", "missing lower flow witness"))?;
        let hi = *uf.get(&obligation.edge_id).ok_or_else(|| err("$.certificate", "missing upper flow witness"))?;
        windows.push(WindowResult {
            obligation_id: obligation.id.clone(),
            time_index: obligation.time,
            interval: obligation.interval.clone(),
            required: obligation.required,
            lower_delivered: lo,
            upper_delivered: hi,
            disposition: verdict(lo, hi, obligation.required),
        });
    }
    let disposition = if windows.iter().all(|w| w.disposition == WindowDisposition::GuaranteedWithinDeclaredLowerEnvelope) {
        ServiceDisposition::GuaranteedWithinDeclaredLowerEnvelope
    } else if windows.iter().any(|w| w.disposition == WindowDisposition::InfeasibleEvenAtDeclaredUpperEnvelope) {
        ServiceDisposition::InfeasibleEvenAtDeclaredUpperEnvelope
    } else {
        ServiceDisposition::IndeterminateNotGuaranteed
    };
    Ok(SafrbCertificate {
        case_id: p.case_id,
        scenario_sha256: p.scenario_sha256,
        claim_scope: p.scope,
        commodity_ref: p.commodity,
        capacity_unit_ref: p.unit,
        quantity_boundary_ref: p.boundary,
        lower,
        upper,
        windows,
        disposition,
        counterfactual_only: p.hypothetical,
        claim_ceiling: CLAIM_CEILING,
    })
}

/// Evaluate the scenario object (the document's "scenario" property).
pub fn evaluate_scenario(value: &Value) -> Result<SafrbCertificate, String> {
    evaluate_internal(value, false)
}

/// Enumerate the least integer lower-bound uplift for one declared physical
/// resource gate. The input is cloned; modified evidence is never admitted.
pub fn minimum_capacity_counterfactual(value: &Value, edge_id: &str) -> Result<Option<RestorationThreshold>, String> {
    let base = parse(value, false)?;
    let target = base.edges.iter().find(|e| e.id == edge_id)
        .ok_or_else(|| err("$.edges", "unknown counterfactual edge"))?;
    if !base.edges.iter().filter(|e| {
        base.nodes.get(&e.from).is_some_and(|n| n.kind == "resource_input")
            && base.nodes.get(&e.to).is_some_and(|n| n.kind == "resource_output")
    }).any(|e| e.id == target.id) {
        return Err(err("$.edges", "counterfactual target is not a physical resource capacity gate"));
    }
    let base_digest = base.scenario_sha256.clone();
    let resource = target.resource.clone();
    let time = target.time;
    let base_lower = target.lower;
    let max = target.upper;
    if max - base_lower > MAX_COUNTERFACTUAL_STEPS {
        return Err(err("$.edges", "counterfactual range exceeds configured search bound"));
    }
    for candidate in base_lower..=max {
        let mut trial = value.clone();
        let edges = trial.get_mut("edges").and_then(Value::as_array_mut).ok_or_else(|| err("$.edges", "must be an array"))?;
        let target_row = edges.iter_mut().find(|e| e.get("id").and_then(Value::as_str) == Some(edge_id))
            .ok_or_else(|| err("$.edges", "counterfactual edge disappeared"))?;
        target_row["capacity_lower"] = Value::from(candidate);
        let evidence = trial.get_mut("capacity_evidence").and_then(Value::as_array_mut)
            .ok_or_else(|| err("$.capacity_evidence", "must be an array"))?;
        let evidence_row = evidence.iter_mut().find(|e| {
            e.get("physical_resource_id").and_then(Value::as_str) == Some(resource.as_str())
                && e.get("time_index").and_then(Value::as_u64) == Some(time)
        }).ok_or_else(|| err("$.capacity_evidence", "matching evidence row missing"))?;
        evidence_row["lower"] = Value::from(candidate);
        evidence_row["status"] = Value::String(HYPOTHETICAL.to_owned());
        let report = evaluate_internal(&trial, true)?;
        if report.windows.iter().all(|w| w.lower_delivered >= w.required) {
            return Ok(Some(RestorationThreshold {
                edge_id: edge_id.to_owned(),
                physical_resource_id: resource,
                time_index: time,
                base_lower,
                hypothetical_lower: candidate,
                additional_capacity: candidate - base_lower,
                lower_flow: report.lower.max_flow,
                lower_cut: report.lower.min_cut.capacity,
                windows: report.windows,
                evidence_status: HYPOTHETICAL,
                base_scenario_sha256: base_digest,
                hypothetical_scenario_sha256: report.scenario_sha256,
            }));
        }
    }
    Ok(None)
}

pub fn bundled_fixture() -> Result<Value, String> {
    serde_json::from_str(include_str!("../tests/fixtures/SAFRB_TWO_WINDOW_SHARED_PUMP_V0_1.json"))
        .map_err(|e| format!("shared-pump fixture JSON: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scenario() -> Value { bundled_fixture().unwrap()["scenario"].clone() }

    #[test]
    fn frozen_two_window_case_has_9t_lower_and_12t_upper() {
        let document = bundled_fixture().unwrap();
        let input = document["scenario"].clone();
        let expected = &document["certificate"];
        let result = evaluate_scenario(&input).unwrap();
        assert_eq!(result.scenario_sha256, expected["scenario_sha256"].as_str().unwrap());
        assert_eq!(result.lower.max_flow, 9);
        assert_eq!(result.lower.min_cut.capacity, 9);
        assert_eq!(result.upper.max_flow, 12);
        assert_eq!(result.upper.min_cut.capacity, 12);
        assert_eq!(result.windows.len(), 2);
        assert_eq!((result.windows[0].lower_delivered, result.windows[0].upper_delivered), (6, 6));
        assert_eq!((result.windows[1].lower_delivered, result.windows[1].upper_delivered), (3, 6));
        assert_eq!(result.disposition, ServiceDisposition::IndeterminateNotGuaranteed);
        let expected_witness = expected["flow_witness"].as_object().unwrap();
        for edge in &result.lower.edge_flows {
            assert_eq!(
                edge.flow,
                expected_witness.get(edge.id.as_str()).unwrap().as_u64().unwrap(),
                "lower flow witness edge {}",
                edge.id
            );
        }
        let expected_cut = expected["cut_edge_ids"].as_array().unwrap()
            .iter().map(|value| value.as_str().unwrap().to_string()).collect::<BTreeSet<_>>();
        let actual_cut = result.lower.min_cut.edges.iter()
            .map(|edge| edge.id.clone()).collect::<BTreeSet<_>>();
        assert_eq!(actual_cut, expected_cut);
    }

    #[test]
    fn threshold_is_plus_three_and_explicitly_not_admitted() {
        let input = scenario();
        let before = digest(&input).unwrap();
        let result = minimum_capacity_counterfactual(&input, "pump-physical-t1").unwrap().unwrap();
        assert_eq!(result.base_lower, 3);
        assert_eq!(result.hypothetical_lower, 6);
        assert_eq!(result.additional_capacity, 3);
        assert_eq!(result.lower_flow, 12);
        assert_eq!(result.lower_cut, 12);
        assert_eq!(result.evidence_status, HYPOTHETICAL);
        assert_eq!(digest(&input).unwrap(), before);
    }

    #[test]
    fn alias_split_cannot_duplicate_shared_physical_capacity() {
        let mut input = scenario();
        input["resource_aliases"][1]["physical_resource_id"] = Value::String("physical:pump-02".into());
        assert!(evaluate_scenario(&input).is_err());
    }

    #[test]
    fn unit_bound_and_capacity_interval_are_fail_closed() {
        let mut input = scenario();
        input["edges"][0]["unit"] = Value::String("kg".into());
        assert!(evaluate_scenario(&input).is_err());
        let mut input = scenario();
        input["edges"].as_array_mut().unwrap().iter_mut()
            .find(|e| e["id"] == "pump-physical-t1").unwrap()["capacity_lower"] = Value::from(9);
        assert!(evaluate_scenario(&input).is_err());
    }

    #[test]
    fn stale_evidence_and_time_reordering_are_rejected() {
        let mut input = scenario();
        input["capacity_evidence"][1]["valid_interval"] = Value::String("[0,1)".into());
        assert!(evaluate_scenario(&input).is_err());
        let mut input = scenario();
        input["edges"].as_array_mut().unwrap().iter_mut()
            .find(|e| e["id"] == "pump-physical-t1").unwrap()["time_index"] = Value::from(0);
        assert!(evaluate_scenario(&input).is_err());
    }

    #[test]
    fn opening_stock_alias_duplication_is_rejected() {
        let mut input = scenario();
        input["total_opening_stock"]["source_ids"][1] = Value::String("terminal-a-stock".into());
        assert!(evaluate_scenario(&input).is_err());
    }

    #[test]
    fn unsupported_scope_and_unreferenced_capacity_evidence_are_rejected() {
        let mut input = scenario();
        input["claim_scope"] = Value::String("WHOLE-BENEFICIARY-SERVICE".into());
        assert!(evaluate_scenario(&input).is_err());

        let mut input = scenario();
        let extra = input["capacity_evidence"][0].clone();
        input["capacity_evidence"].as_array_mut().unwrap().push(extra);
        assert!(evaluate_scenario(&input).is_err());
    }
}
