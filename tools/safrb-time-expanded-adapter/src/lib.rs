#![forbid(unsafe_code)]

use serde_json::Value;
use sha2::{Digest, Sha256};
use single_commodity_flow_kernel::{
    FlowCertificateBundle, FlowEdge, FlowGraph, FlowNode, FlowResult,
};
use std::collections::{BTreeMap, BTreeSet};

pub mod beneficiary_floors;
pub mod temporal_profile;
pub mod temporal_verify;

const SCHEMA: &str = "safrb-time-expanded-network-v0.1";
const HYPOTHETICAL: &str = "hypothetical_counterfactual_not_admitted";
const MAX_SERVICE_OBLIGATIONS: usize = 128;
const MAX_INDIVIDUAL_MAXIMA: usize = 32;
const CLAIM_CEILING: &str = "Synthetic mathematical feasibility for the declared single-commodity time-expanded graph only; no authenticated evidence, physical asset qualification, legal access, dispatch authority or actual beneficiary-service claim.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowDisposition {
    IndividuallyFeasibleWithinDeclaredLowerEnvelope,
    IndividuallyPossibleOnlyWithinDeclaredUpperEnvelope,
    NotIndividuallyFeasibleWithinDeclaredUpperEnvelope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceDisposition {
    GuaranteedWithinDeclaredLowerEnvelope,
    IndeterminateNotGuaranteed,
    NotFeasibleWithinDeclaredUpperEnvelope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndividualMaximaStatus {
    NotNeededJointServiceFeasible,
    ComputedForInfeasibleJointService,
    OmittedAboveDeclaredObligationLimit,
    OmittedForCounterfactualSearch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowResult {
    pub obligation_id: String,
    pub time_index: u64,
    pub interval: String,
    pub required: u64,
    /// Flow allocated to this obligation by the joint all-obligation witness.
    /// This is one feasible allocation, not an individual maximum or guarantee.
    pub lower_joint_witness_allocation: u64,
    pub upper_joint_witness_allocation: u64,
    /// Maximum deliverable to this obligation when evaluated on its own.
    /// Individual maxima for different obligations need not be jointly feasible.
    pub lower_individual_max_deliverable: Option<u64>,
    pub upper_individual_max_deliverable: Option<u64>,
    /// Full primal/dual certificates for each isolated maximum. Absent when
    /// these diagnostics are not needed or deliberately omitted.
    pub lower_individual_certificate: Option<FlowCertificateBundle>,
    pub upper_individual_certificate: Option<FlowCertificateBundle>,
    pub disposition: Option<WindowDisposition>,
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
    /// Portable graph-bound certificates for simultaneous lower/upper service.
    pub lower_bundle: FlowCertificateBundle,
    pub upper_bundle: FlowCertificateBundle,
    pub windows: Vec<WindowResult>,
    pub individual_maxima_status: IndividualMaximaStatus,
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
    pub lower_certificate: FlowResult,
    pub upper_certificate: FlowResult,
    /// Portable graph-bound hypothetical certificates; never admitted as evidence.
    pub lower_bundle: FlowCertificateBundle,
    pub upper_bundle: FlowCertificateBundle,
    pub windows: Vec<WindowResult>,
    pub individual_maxima_status: IndividualMaximaStatus,
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
    node_id: String,
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
    gate_ids: BTreeSet<String>,
    scenario_sha256: String,
    hypothetical: bool,
}

fn err(path: &str, message: &str) -> String {
    format!("{path}: {message}")
}

fn get<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    value
        .as_object()
        .and_then(|o| o.get(key))
        .ok_or_else(|| err(path, &format!("missing property {key}")))
}

fn strv(value: &Value, path: &str) -> Result<String, String> {
    value
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| err(path, "must be a non-empty string"))
}

fn u64v(value: &Value, path: &str) -> Result<u64, String> {
    value
        .as_u64()
        .ok_or_else(|| err(path, "must be a non-negative integer"))
}

fn exact_keys(value: &Value, allowed: &[&str], path: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| err(path, "must be an object"))?;
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(err(path, &format!("unknown property {key}")));
        }
    }
    Ok(())
}

fn sfield(value: &Value, key: &str, path: &str) -> Result<String, String> {
    strv(get(value, key, path)?, &format!("{path}.{key}"))
}

fn nfield(value: &Value, key: &str, path: &str) -> Result<u64, String> {
    u64v(get(value, key, path)?, &format!("{path}.{key}"))
}

fn array<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a [Value], String> {
    get(value, key, path)?
        .as_array()
        .map(Vec::as_slice)
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
                parts.push(format!(
                    "{}:{}",
                    serde_json::to_string(key).map_err(|e| e.to_string())?,
                    canonical(value)?
                ));
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
    let body = value
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| err(path, "must use half-open [start,end) syntax"))?;
    let (start, end) = body
        .split_once(',')
        .ok_or_else(|| err(path, "missing interval comma"))?;
    let start = start
        .parse::<u64>()
        .map_err(|_| err(path, "invalid interval start"))?;
    let end = end
        .parse::<u64>()
        .map_err(|_| err(path, "invalid interval end"))?;
    let expected_end = index
        .checked_add(1)
        .ok_or_else(|| err(path, "one-step interval end overflows u64"))?;
    if start != index || end != expected_end {
        return Err(err(
            path,
            "interval must match the declared one-step time index",
        ));
    }
    Ok(())
}

fn path_exists_excluding(
    parsed_nodes: &BTreeMap<String, Node>,
    parsed_edges: &[Edge],
    excluded_edges: &BTreeSet<String>,
    start: &str,
    target: &str,
) -> bool {
    let mut seen = BTreeSet::new();
    let mut queue = vec![start.to_owned()];
    let mut cursor = 0_usize;
    seen.insert(start.to_owned());
    while cursor < queue.len() {
        let node_id = queue[cursor].clone();
        cursor += 1;
        if node_id == target {
            return true;
        }
        for edge in parsed_edges.iter().filter(|edge| {
            edge.from == node_id && edge.upper > 0 && !excluded_edges.contains(&edge.id)
        }) {
            if parsed_nodes.contains_key(&edge.to) && seen.insert(edge.to.clone()) {
                queue.push(edge.to.clone());
            }
        }
    }
    seen.contains(target)
}

fn verify_physical_gate_dominance(parsed: &Parsed) -> Result<(), String> {
    if parsed.gate_ids.is_empty() {
        return Err(err(
            "$.resource_aliases",
            "no physical capacity gates were declared",
        ));
    }
    // No positive-capacity source-to-service path may avoid every declared
    // physical-resource gate.
    if path_exists_excluding(
        &parsed.nodes,
        &parsed.edges,
        &parsed.gate_ids,
        &parsed.source,
        &parsed.sink,
    ) {
        return Err(err(
            "$.edges",
            "a positive-capacity source-to-service path bypasses all declared physical-resource gates",
        ));
    }
    // Each declared gate must participate in at least one feasible path. Other
    // gates remain available here, so legitimate serial infrastructure works.
    for gate_id in &parsed.gate_ids {
        let gate = parsed
            .edges
            .iter()
            .find(|edge| &edge.id == gate_id)
            .ok_or_else(|| {
                err(
                    "$.resource_aliases",
                    "declared physical gate edge is missing",
                )
            })?;
        if gate.upper == 0 {
            return Err(err(
                "$.resource_aliases",
                "declared physical gate has no positive upper capacity",
            ));
        }
        let excluded_gate = BTreeSet::from([gate_id.clone()]);
        if !path_exists_excluding(
            &parsed.nodes,
            &parsed.edges,
            &excluded_gate,
            &parsed.source,
            &gate.from,
        ) || !path_exists_excluding(
            &parsed.nodes,
            &parsed.edges,
            &excluded_gate,
            &gate.to,
            &parsed.sink,
        ) {
            return Err(err(
                "$.resource_aliases",
                "declared physical capacity gate does not lie on a source-to-service path",
            ));
        }
    }
    Ok(())
}

fn parse(value: &Value, allow_hypothetical: bool) -> Result<Parsed, String> {
    exact_keys(
        value,
        &[
            "schema_version",
            "case_id",
            "evidence_maturity",
            "claim_scope",
            "timebase",
            "commodity",
            "source_node",
            "sink_node",
            "total_opening_stock",
            "nodes",
            "edges",
            "resource_aliases",
            "capacity_evidence",
            "service_obligations",
            "route_transit",
            "adversarial_mutations",
        ],
        "$",
    )?;
    if sfield(value, "schema_version", "$")? != SCHEMA {
        return Err(err(
            "$.schema_version",
            "unsupported model/schema; multi-commodity and process-coupled graphs are not supported",
        ));
    }
    if sfield(value, "evidence_maturity", "$")? != "synthetic_known_truth" {
        return Err(err(
            "$.evidence_maturity",
            "v0.1 accepts synthetic known-truth fixtures only",
        ));
    }
    let adversarial_mutations = array(value, "adversarial_mutations", "$")?;
    for (index, mutation) in adversarial_mutations.iter().enumerate() {
        strv(mutation, &format!("$.adversarial_mutations[{index}]"))?;
    }

    let case_id = sfield(value, "case_id", "$")?;
    let scope = sfield(value, "claim_scope", "$")?;
    if scope != "FUEL-DELIVERABILITY" {
        return Err(err(
            "$.claim_scope",
            "v0.1 supports FUEL-DELIVERABILITY only",
        ));
    }
    if scope != "FUEL-DELIVERABILITY" {
        return Err(err(
            "$.claim_scope",
            "v0.1 adapter supports FUEL-DELIVERABILITY only",
        ));
    }

    let timebase = get(value, "timebase", "$")?;
    exact_keys(
        timebase,
        &["kind", "interval_seconds", "interval_semantics", "origin"],
        "$.timebase",
    )?;
    if sfield(timebase, "kind", "$.timebase")? != "relative_discrete_intervals"
        || sfield(timebase, "interval_semantics", "$.timebase")? != "half_open"
        || sfield(timebase, "origin", "$.timebase")? != "scenario_start"
    {
        return Err(err("$.timebase", "unsupported temporal profile"));
    }
    let seconds = nfield(timebase, "interval_seconds", "$.timebase")?;
    if seconds == 0 || seconds > 86_400 {
        return Err(err(
            "$.timebase.interval_seconds",
            "outside supported range",
        ));
    }

    let commodity_obj = get(value, "commodity", "$")?;
    exact_keys(
        commodity_obj,
        &["product_id", "quantity_unit", "unit_basis"],
        "$.commodity",
    )?;
    let commodity = sfield(commodity_obj, "product_id", "$.commodity")?;
    let unit = sfield(commodity_obj, "quantity_unit", "$.commodity")?;
    let unit_basis = sfield(commodity_obj, "unit_basis", "$.commodity")?;
    let source = sfield(value, "source_node", "$")?;
    let sink = sfield(value, "sink_node", "$")?;
    if source == sink {
        return Err(err("$", "source and sink must differ"));
    }
    let boundary = format!("safrb:{scope}:{unit_basis}:{source}->{sink}");

    let mut nodes = BTreeMap::new();
    for (i, row) in array(value, "nodes", "$")?.iter().enumerate() {
        let path = format!("$.nodes[{i}]");
        exact_keys(row, &["id", "kind", "time_index"], &path)?;
        let id = sfield(row, "id", &path)?;
        let node = Node {
            id: id.clone(),
            kind: sfield(row, "kind", &path)?,
            time: nfield(row, "time_index", &path)?,
        };
        if nodes.insert(id.clone(), node).is_some() {
            return Err(err(&path, "duplicate node ID"));
        }
    }
    if nodes.len() < 2 || nodes.len() > 4096 {
        return Err(err("$.nodes", "node count outside 2..=4096"));
    }
    let src = nodes
        .get(&source)
        .ok_or_else(|| err("$.source_node", "unknown source"))?;
    let snk = nodes
        .get(&sink)
        .ok_or_else(|| err("$.sink_node", "unknown sink"))?;
    if src.kind != "source" || src.time != 0 || snk.kind != "sink" || snk.time == 0 {
        return Err(err("$", "source/sink node kinds or times are invalid"));
    }

    let mut edges = Vec::new();
    let mut edge_ids = BTreeSet::new();
    for (i, row) in array(value, "edges", "$")?.iter().enumerate() {
        let path = format!("$.edges[{i}]");
        exact_keys(
            row,
            &[
                "id",
                "from",
                "to",
                "capacity_lower",
                "capacity_upper",
                "unit",
                "resource_id",
                "time_index",
            ],
            &path,
        )?;
        let id = sfield(row, "id", &path)?;
        if !edge_ids.insert(id.clone()) {
            return Err(err(&path, "duplicate edge ID"));
        }
        let from = sfield(row, "from", &path)?;
        let to = sfield(row, "to", &path)?;
        if from == to {
            return Err(err(&path, "self-loop unsupported"));
        }
        if from == sink || to == source {
            return Err(err(&path, "source and sink must be terminal graph nodes"));
        }
        let from_node = nodes
            .get(&from)
            .ok_or_else(|| err(&path, "unknown source endpoint"))?;
        let to_node = nodes
            .get(&to)
            .ok_or_else(|| err(&path, "unknown target endpoint"))?;
        let lower = nfield(row, "capacity_lower", &path)?;
        let upper = nfield(row, "capacity_upper", &path)?;
        if lower > upper {
            return Err(err(&path, "lower capacity exceeds upper capacity"));
        }
        let edge_unit = sfield(row, "unit", &path)?;
        if edge_unit != unit {
            return Err(err(&path, "unit differs from scenario commodity unit"));
        }
        let time = nfield(row, "time_index", &path)?;
        if time != from_node.time
            || to_node.time < time
            || (to_node.time > time && time.checked_add(1) != Some(to_node.time))
        {
            return Err(err(&path, "time-expanded edge chronology is invalid"));
        }
        if from_node.kind == "resource_output" && to_node.kind == "route_arrival" {
            let expected_arrival = time
                .checked_add(1)
                .ok_or_else(|| err(&path, "route arrival time overflows u64"))?;
            if to_node.time != expected_arrival {
                return Err(err(&path, "route transit must advance one interval"));
            }
        }
        edges.push(Edge {
            id,
            from,
            to,
            lower,
            upper,
            resource: sfield(row, "resource_id", &path)?,
            time,
        });
    }
    if edges.is_empty() || edges.len() > 65_536 {
        return Err(err("$.edges", "edge count outside 1..=65536"));
    }

    let opening = get(value, "total_opening_stock", "$")?;
    exact_keys(
        opening,
        &["quantity", "unit", "source_ids"],
        "$.total_opening_stock",
    )?;
    if sfield(opening, "unit", "$.total_opening_stock")? != unit {
        return Err(err("$.total_opening_stock.unit", "unit mismatch"));
    }
    let opening_quantity = nfield(opening, "quantity", "$.total_opening_stock")?;
    let source_ids = array(opening, "source_ids", "$.total_opening_stock")?
        .iter()
        .map(|v| strv(v, "$.total_opening_stock.source_ids[]"))
        .collect::<Result<Vec<_>, _>>()?;
    let expected_ids = source_ids.iter().cloned().collect::<BTreeSet<_>>();
    if source_ids.is_empty() || expected_ids.len() != source_ids.len() {
        return Err(err(
            "$.total_opening_stock.source_ids",
            "must be non-empty and unique",
        ));
    }
    let source_edges = edges
        .iter()
        .filter(|e| e.from == source)
        .collect::<Vec<_>>();
    let stock = source_edges
        .iter()
        .copied()
        .filter(|e| nodes.get(&e.to).is_some_and(|n| n.kind == "inventory"))
        .collect::<Vec<_>>();
    if source_edges.len() != stock.len() {
        return Err(err(
            "$.edges",
            "v0.1 source may connect only to opening inventory nodes",
        ));
    }
    let actual_ids = stock
        .iter()
        .map(|e| e.resource.clone())
        .collect::<BTreeSet<_>>();
    if stock.len() != source_ids.len() || actual_ids != expected_ids {
        return Err(err(
            "$.total_opening_stock",
            "source inventory edges do not reconcile to source identities",
        ));
    }
    let stock_sum = stock.iter().try_fold(0_u64, |sum, edge| {
        sum.checked_add(edge.lower)
            .ok_or_else(|| err("$.total_opening_stock", "stock overflow"))
    })?;
    let stock_upper = stock.iter().try_fold(0_u64, |sum, edge| {
        sum.checked_add(edge.upper).ok_or_else(|| {
            err(
                "$.total_opening_stock",
                "opening-stock upper bound overflow",
            )
        })
    })?;
    if stock_sum != opening_quantity || stock_upper != opening_quantity {
        return Err(err(
            "$.total_opening_stock",
            "exact opening-stock lower and upper capacities must both reconcile",
        ));
    }

    let aliases = array(value, "resource_aliases", "$")?;
    if aliases.is_empty() {
        return Err(err("$.resource_aliases", "must not be empty"));
    }
    let mut logicals = BTreeSet::new();
    let mut logical_physical = BTreeMap::<String, String>::new();
    let mut physical_gates = BTreeMap::<String, BTreeSet<String>>::new();
    let mut all_gates = BTreeSet::new();
    for (i, row) in aliases.iter().enumerate() {
        let path = format!("$.resource_aliases[{i}]");
        exact_keys(
            row,
            &[
                "logical_route_id",
                "physical_resource_id",
                "capacity_gate_edge_ids",
            ],
            &path,
        )?;
        let logical = sfield(row, "logical_route_id", &path)?;
        let physical = sfield(row, "physical_resource_id", &path)?;
        if !logicals.insert(logical.clone()) {
            return Err(err(&path, "duplicate logical route"));
        }
        logical_physical.insert(logical, physical.clone());
        let gate_ids = array(row, "capacity_gate_edge_ids", &path)?
            .iter()
            .map(|v| strv(v, &format!("{path}.capacity_gate_edge_ids[]")))
            .collect::<Result<BTreeSet<_>, _>>()?;
        if gate_ids.is_empty() {
            return Err(err(&path, "empty physical gate set"));
        }
        for gate_id in &gate_ids {
            let gate = edges
                .iter()
                .find(|e| &e.id == gate_id)
                .ok_or_else(|| err(&path, "unknown gate edge"))?;
            if gate.resource != physical {
                return Err(err(&path, "alias points to a different physical resource"));
            }
            if nodes
                .get(&gate.from)
                .is_none_or(|n| n.kind != "resource_input")
                || nodes
                    .get(&gate.to)
                    .is_none_or(|n| n.kind != "resource_output")
            {
                return Err(err(
                    &path,
                    "gate must connect resource_input to resource_output",
                ));
            }
            all_gates.insert(gate_id.clone());
        }
        if let Some(existing) = physical_gates.get(&physical) {
            if existing != &gate_ids {
                return Err(err(
                    &path,
                    "aliases for one physical resource disagree on gate set",
                ));
            }
        } else {
            physical_gates.insert(physical, gate_ids);
        }
    }
    let actual_gate_ids = edges
        .iter()
        .filter(|e| physical_gates.contains_key(&e.resource))
        .map(|e| e.id.clone())
        .collect::<BTreeSet<_>>();
    if actual_gate_ids != all_gates {
        return Err(err(
            "$.resource_aliases",
            "incomplete, duplicate or unaliased physical resource gates",
        ));
    }
    let mut gate_times = BTreeSet::new();
    for gate in edges.iter().filter(|e| all_gates.contains(&e.id)) {
        if !gate_times.insert((gate.resource.clone(), gate.time)) {
            return Err(err(
                "$.resource_aliases",
                "more than one gate for a physical resource/time",
            ));
        }
    }

    let mut route_ids = BTreeSet::new();
    for (i, row) in array(value, "route_transit", "$")?.iter().enumerate() {
        let path = format!("$.route_transit[{i}]");
        exact_keys(
            row,
            &["route_id", "transit_intervals", "physical_resources"],
            &path,
        )?;
        let id = sfield(row, "route_id", &path)?;
        if !route_ids.insert(id.clone()) {
            return Err(err(&path, "duplicate route ID"));
        }
        let declared_physical = logical_physical.get(&id).ok_or_else(|| {
            err(
                &path,
                "route has no corresponding logical-to-physical alias",
            )
        })?;
        if nfield(row, "transit_intervals", &path)? != 1 {
            return Err(err(
                &path,
                "v0.1 adapter supports exactly one transit interval",
            ));
        }
        let physical_resources = array(row, "physical_resources", &path)?;
        if physical_resources.is_empty() {
            return Err(err(&path, "physical_resources must not be empty"));
        }
        let route_resources = physical_resources
            .iter()
            .map(|resource| strv(resource, &format!("{path}.physical_resources[]")))
            .collect::<Result<BTreeSet<_>, _>>()?;
        if route_resources.len() != physical_resources.len() {
            return Err(err(&path, "physical_resources contains duplicates"));
        }
        if route_resources != BTreeSet::from([declared_physical.clone()]) {
            return Err(err(
                &path,
                "route physical_resources do not match the authoritative alias mapping",
            ));
        }
        if route_resources
            .iter()
            .any(|resource| !physical_gates.contains_key(resource))
        {
            return Err(err(&path, "route references an unknown physical resource"));
        }
    }
    if route_ids != logicals {
        return Err(err(
            "$.route_transit",
            "route IDs do not match logical aliases",
        ));
    }

    let mut evidence_ids = BTreeSet::new();
    let mut evidence = BTreeMap::<(String, u64), (u64, u64)>::new();
    let mut hypothetical = false;
    for (i, row) in array(value, "capacity_evidence", "$")?.iter().enumerate() {
        let path = format!("$.capacity_evidence[{i}]");
        exact_keys(
            row,
            &[
                "evidence_id",
                "physical_resource_id",
                "time_index",
                "lower",
                "upper",
                "unit",
                "status",
                "valid_interval",
            ],
            &path,
        )?;
        let id = sfield(row, "evidence_id", &path)?;
        if !evidence_ids.insert(id) {
            return Err(err(&path, "duplicate evidence ID"));
        }
        let physical = sfield(row, "physical_resource_id", &path)?;
        let time = nfield(row, "time_index", &path)?;
        let lower = nfield(row, "lower", &path)?;
        let upper = nfield(row, "upper", &path)?;
        if lower > upper || sfield(row, "unit", &path)? != unit {
            return Err(err(&path, "invalid capacity evidence quantity"));
        }
        let status = sfield(row, "status", &path)?;
        if status == HYPOTHETICAL && allow_hypothetical {
            hypothetical = true;
        } else if status != "synthetic_admitted" {
            return Err(err(
                &path,
                "capacity evidence is not admissible in this evaluation mode",
            ));
        }
        half_open(
            &sfield(row, "valid_interval", &path)?,
            &format!("{path}.valid_interval"),
            time,
        )?;
        if evidence.insert((physical, time), (lower, upper)).is_some() {
            return Err(err(&path, "duplicate resource/time evidence"));
        }
    }
    for gate in edges.iter().filter(|e| all_gates.contains(&e.id)) {
        if evidence.get(&(gate.resource.clone(), gate.time)) != Some(&(gate.lower, gate.upper)) {
            return Err(err(
                "$.capacity_evidence",
                "missing or inconsistent physical capacity evidence",
            ));
        }
    }
    if evidence.len() != all_gates.len() {
        return Err(err("$.capacity_evidence", "unreferenced capacity evidence"));
    }

    let obligation_rows = array(value, "service_obligations", "$")?;
    if obligation_rows.is_empty() || obligation_rows.len() > MAX_SERVICE_OBLIGATIONS {
        return Err(err(
            "$.service_obligations",
            "obligation count outside 1..=128",
        ));
    }
    let mut obligations = Vec::new();
    let mut obligation_ids = BTreeSet::new();
    let mut obligation_times = BTreeSet::new();
    for (i, row) in obligation_rows.iter().enumerate() {
        let path = format!("$.service_obligations[{i}]");
        exact_keys(
            row,
            &[
                "obligation_id",
                "node_id",
                "time_index",
                "required_lower",
                "unit",
                "interval",
            ],
            &path,
        )?;
        let id = sfield(row, "obligation_id", &path)?;
        if !obligation_ids.insert(id.clone()) {
            return Err(err(&path, "duplicate obligation ID"));
        }
        let node_id = sfield(row, "node_id", &path)?;
        let time = nfield(row, "time_index", &path)?;
        let required = nfield(row, "required_lower", &path)?;
        if required == 0 || sfield(row, "unit", &path)? != unit {
            return Err(err(&path, "invalid service floor or unit"));
        }
        let interval = sfield(row, "interval", &path)?;
        half_open(&interval, &format!("{path}.interval"), time)?;
        let node = nodes
            .get(&node_id)
            .ok_or_else(|| err(&path, "unknown service node"))?;
        if node.kind != "beneficiary_service" || node.time != time || !obligation_times.insert(time)
        {
            return Err(err(&path, "service node/time mismatch or duplicate window"));
        }
        let mut matching = edges.iter().filter(|e| e.from == node_id && e.to == sink);
        let service_edge = matching
            .next()
            .ok_or_else(|| err(&path, "missing sink obligation edge"))?;
        if matching.next().is_some()
            || service_edge.lower != required
            || service_edge.upper != required
        {
            return Err(err(
                &path,
                "sink edge must uniquely encode the declared service floor",
            ));
        }
        obligations.push(Obligation {
            id,
            node_id,
            time,
            interval,
            required,
            edge_id: service_edge.id.clone(),
        });
    }
    if obligations.is_empty() || edges.iter().filter(|e| e.to == sink).count() != obligations.len()
    {
        return Err(err(
            "$.service_obligations",
            "sink edges and declared obligations must match exactly",
        ));
    }
    let declared_service_nodes = obligations
        .iter()
        .map(|obligation| obligation.node_id.clone())
        .collect::<BTreeSet<_>>();
    let actual_service_nodes = nodes
        .values()
        .filter(|node| node.kind == "beneficiary_service")
        .map(|node| node.id.clone())
        .collect::<BTreeSet<_>>();
    if declared_service_nodes != actual_service_nodes {
        return Err(err(
            "$.service_obligations",
            "every beneficiary_service node must have exactly one declared obligation",
        ));
    }
    obligations.sort_by_key(|o| (o.time, o.id.clone()));

    let parsed = Parsed {
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
        gate_ids: all_gates,
        scenario_sha256: digest(value)?,
        hypothetical,
    };
    verify_physical_gate_dominance(&parsed)?;
    Ok(parsed)
}

fn build_graph(
    parsed: &Parsed,
    upper: bool,
    only_obligation_edge: Option<&str>,
) -> Result<FlowGraph, String> {
    let nodes = parsed
        .nodes
        .values()
        .map(|n| FlowNode::new(n.id.clone()))
        .collect::<Vec<_>>();
    let obligation_edges = parsed
        .obligations
        .iter()
        .map(|obligation| obligation.edge_id.as_str())
        .collect::<BTreeSet<_>>();
    // In an isolated-beneficiary solve, the sink arc must not remain capped
    // at the service floor. Bound it by all source capacity so the result is
    // the actual isolated maximum deliverable quantity within this graph.
    let individual_source_capacity_bound = if only_obligation_edge.is_some() {
        Some(
            parsed
                .edges
                .iter()
                .filter(|edge| edge.from == parsed.source)
                .try_fold(0_u64, |sum, edge| {
                    let capacity = if upper { edge.upper } else { edge.lower };
                    sum.checked_add(capacity)
                        .ok_or_else(|| err("$", "isolated source-capacity bound overflowed u64"))
                })?,
        )
    } else {
        None
    };
    let edges = parsed
        .edges
        .iter()
        .filter(|edge| {
            if !obligation_edges.contains(edge.id.as_str()) {
                return true;
            }
            only_obligation_edge == Some(edge.id.as_str()) || only_obligation_edge.is_none()
        })
        .map(|e| {
            let mut capacity = if upper { e.upper } else { e.lower };
            if only_obligation_edge == Some(e.id.as_str()) {
                capacity = individual_source_capacity_bound.unwrap_or(capacity);
            }
            let metadata = format!("safrb-v0.1/{}/{}/t{}", parsed.scope, e.resource, e.time);
            FlowEdge::new(
                e.id.clone(),
                e.from.clone(),
                e.to.clone(),
                capacity,
                Some(metadata),
            )
        })
        .collect::<Vec<_>>();
    let graph = FlowGraph {
        commodity_ref: parsed.commodity.clone(),
        capacity_unit_ref: parsed.unit.clone(),
        quantity_boundary_ref: parsed.boundary.clone(),
        nodes,
        edges,
        source: parsed.source.clone(),
        sink: parsed.sink.clone(),
    };
    Ok(graph)
}

fn solve_bundle(
    parsed: &Parsed,
    upper: bool,
    only_obligation_edge: Option<&str>,
) -> Result<FlowCertificateBundle, String> {
    let graph = build_graph(parsed, upper, only_obligation_edge)?;
    let result = graph.solve().map_err(|error| error.to_string())?;
    FlowCertificateBundle::new(graph, result).map_err(|error| error.to_string())
}

fn edge_flow(result: &FlowResult) -> BTreeMap<String, u64> {
    result
        .edge_flows
        .iter()
        .map(|e| (e.id.clone(), e.flow))
        .collect()
}

fn individual_verdict(lower: u64, upper: u64, required: u64) -> WindowDisposition {
    if lower >= required {
        WindowDisposition::IndividuallyFeasibleWithinDeclaredLowerEnvelope
    } else if upper >= required {
        WindowDisposition::IndividuallyPossibleOnlyWithinDeclaredUpperEnvelope
    } else {
        WindowDisposition::NotIndividuallyFeasibleWithinDeclaredUpperEnvelope
    }
}

fn individual_maxima_status(
    joint_lower_feasible: bool,
    include_individual_maxima: bool,
    obligation_count: usize,
) -> IndividualMaximaStatus {
    if joint_lower_feasible {
        IndividualMaximaStatus::NotNeededJointServiceFeasible
    } else if !include_individual_maxima {
        IndividualMaximaStatus::OmittedForCounterfactualSearch
    } else if obligation_count <= MAX_INDIVIDUAL_MAXIMA {
        IndividualMaximaStatus::ComputedForInfeasibleJointService
    } else {
        IndividualMaximaStatus::OmittedAboveDeclaredObligationLimit
    }
}

fn evaluate_internal(
    value: &Value,
    allow_hypothetical: bool,
    include_individual_maxima: bool,
) -> Result<SafrbCertificate, String> {
    let p = parse(value, allow_hypothetical)?;
    // Joint solves answer whether every beneficiary floor can be met at once.
    let lower_bundle = solve_bundle(&p, false, None)?;
    let upper_bundle = solve_bundle(&p, true, None)?;
    let lower = lower_bundle.result.clone();
    let upper = upper_bundle.result.clone();
    let lf = edge_flow(&lower);
    let uf = edge_flow(&upper);

    let total_required = p.obligations.iter().try_fold(0_u64, |sum, obligation| {
        sum.checked_add(obligation.required).ok_or_else(|| {
            err(
                "$.service_obligations",
                "sum of service floors overflows u64",
            )
        })
    })?;
    let joint_lower_feasible = lower.max_flow == total_required;

    let individual_maxima_status = individual_maxima_status(
        joint_lower_feasible,
        include_individual_maxima,
        p.obligations.len(),
    );

    let mut windows = Vec::new();
    for obligation in &p.obligations {
        let lower_joint = *lf.get(&obligation.edge_id).ok_or_else(|| {
            err(
                "$.certificate",
                "joint lower witness omitted an obligation edge",
            )
        })?;
        let upper_joint = *uf.get(&obligation.edge_id).ok_or_else(|| {
            err(
                "$.certificate",
                "joint upper witness omitted an obligation edge",
            )
        })?;

        let (lower_individual, upper_individual, window_disposition) = if individual_maxima_status
            == IndividualMaximaStatus::ComputedForInfeasibleJointService
        {
            // Each isolated solve removes other obligation edges and raises
            // the retained sink edge to the total source-capacity bound.
            // The complete graph-bound primal/dual bundles are retained.
            let lower_result = solve_bundle(&p, false, Some(&obligation.edge_id))?;
            let upper_result = solve_bundle(&p, true, Some(&obligation.edge_id))?;
            let lower_max = lower_result.result.max_flow;
            let upper_max = upper_result.result.max_flow;
            let verdict = individual_verdict(lower_max, upper_max, obligation.required);
            (Some(lower_result), Some(upper_result), Some(verdict))
        } else if individual_maxima_status == IndividualMaximaStatus::NotNeededJointServiceFeasible
        {
            // Joint lower feasibility already proves each individual
            // floor is attainable; computing separate maxima adds no
            // decision value and would multiply solver work.
            (
                None,
                None,
                Some(WindowDisposition::IndividuallyFeasibleWithinDeclaredLowerEnvelope),
            )
        } else {
            (None, None, None)
        };

        windows.push(WindowResult {
            obligation_id: obligation.id.clone(),
            time_index: obligation.time,
            interval: obligation.interval.clone(),
            required: obligation.required,
            lower_joint_witness_allocation: lower_joint,
            upper_joint_witness_allocation: upper_joint,
            lower_individual_max_deliverable: lower_individual
                .as_ref()
                .map(|bundle| bundle.result.max_flow),
            upper_individual_max_deliverable: upper_individual
                .as_ref()
                .map(|bundle| bundle.result.max_flow),
            lower_individual_certificate: lower_individual,
            upper_individual_certificate: upper_individual,
            disposition: window_disposition,
        });
    }

    // A full lower-bound max-flow saturates every sink-obligation edge because
    // their total capacity equals total_required. This is the only condition
    // under which this adapter calls all obligations simultaneously supported.
    let disposition = if joint_lower_feasible {
        ServiceDisposition::GuaranteedWithinDeclaredLowerEnvelope
    } else if upper.max_flow == total_required {
        ServiceDisposition::IndeterminateNotGuaranteed
    } else {
        ServiceDisposition::NotFeasibleWithinDeclaredUpperEnvelope
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
        lower_bundle,
        upper_bundle,
        windows,
        individual_maxima_status,
        disposition,
        counterfactual_only: p.hypothetical,
        claim_ceiling: CLAIM_CEILING,
    })
}
/// Evaluate the scenario object (the document's "scenario" property).
pub fn evaluate_scenario(value: &Value) -> Result<SafrbCertificate, String> {
    evaluate_internal(value, false, true)
}

/// Enumerate the least integer lower-bound uplift for one declared physical
/// resource gate. The input is cloned; modified evidence is never admitted.
pub fn minimum_capacity_counterfactual(
    value: &Value,
    edge_id: &str,
) -> Result<Option<RestorationThreshold>, String> {
    let base = parse(value, false)?;
    let target = base
        .edges
        .iter()
        .find(|e| e.id == edge_id)
        .ok_or_else(|| err("$.edges", "unknown counterfactual edge"))?;
    let is_gate = base
        .edges
        .iter()
        .filter(|e| {
            base.nodes
                .get(&e.from)
                .is_some_and(|n| n.kind == "resource_input")
                && base
                    .nodes
                    .get(&e.to)
                    .is_some_and(|n| n.kind == "resource_output")
        })
        .any(|e| e.id == target.id);
    if !is_gate {
        return Err(err(
            "$.edges",
            "counterfactual target is not a physical resource capacity gate",
        ));
    }

    let base_digest = base.scenario_sha256.clone();
    let resource = target.resource.clone();
    let time = target.time;
    let base_lower = target.lower;
    let maximum = target.upper;

    // Increasing a single lower-bound capacity only expands the feasible
    // region, so simultaneous feasibility is monotone. Binary search avoids
    // a linear number of full graph solves for large integer capacities.
    let make_trial = |candidate: u64| -> Result<Value, String> {
        let mut trial = value.clone();
        let edges = trial
            .get_mut("edges")
            .and_then(Value::as_array_mut)
            .ok_or_else(|| err("$.edges", "must be an array"))?;
        let target_row = edges
            .iter_mut()
            .find(|edge| edge.get("id").and_then(Value::as_str) == Some(edge_id))
            .ok_or_else(|| err("$.edges", "counterfactual edge disappeared"))?;
        target_row["capacity_lower"] = Value::from(candidate);

        let evidence = trial
            .get_mut("capacity_evidence")
            .and_then(Value::as_array_mut)
            .ok_or_else(|| err("$.capacity_evidence", "must be an array"))?;
        let evidence_row = evidence
            .iter_mut()
            .find(|row| {
                row.get("physical_resource_id").and_then(Value::as_str) == Some(resource.as_str())
                    && row.get("time_index").and_then(Value::as_u64) == Some(time)
            })
            .ok_or_else(|| err("$.capacity_evidence", "matching evidence row missing"))?;
        evidence_row["lower"] = Value::from(candidate);
        evidence_row["status"] = Value::String(HYPOTHETICAL.to_owned());
        Ok(trial)
    };

    let maximum_trial = make_trial(maximum)?;
    if evaluate_internal(&maximum_trial, true, false)?.disposition
        != ServiceDisposition::GuaranteedWithinDeclaredLowerEnvelope
    {
        return Ok(None);
    }

    let mut low = base_lower;
    let mut high = maximum;
    while low < high {
        let middle = low + (high - low) / 2;
        let trial = make_trial(middle)?;
        let report = evaluate_internal(&trial, true, false)?;
        if report.disposition == ServiceDisposition::GuaranteedWithinDeclaredLowerEnvelope {
            high = middle;
        } else {
            low = middle + 1;
        }
    }

    let final_trial = make_trial(low)?;
    let report = evaluate_internal(&final_trial, true, false)?;
    if report.disposition != ServiceDisposition::GuaranteedWithinDeclaredLowerEnvelope {
        return Err(err(
            "$.counterfactual",
            "binary-search threshold failed final joint-feasibility verification",
        ));
    }

    Ok(Some(RestorationThreshold {
        edge_id: edge_id.to_owned(),
        physical_resource_id: resource,
        time_index: time,
        base_lower,
        hypothetical_lower: low,
        additional_capacity: low - base_lower,
        lower_flow: report.lower.max_flow,
        lower_cut: report.lower.min_cut.capacity,
        lower_certificate: report.lower.clone(),
        upper_certificate: report.upper.clone(),
        lower_bundle: report.lower_bundle,
        upper_bundle: report.upper_bundle,
        windows: report.windows,
        individual_maxima_status: report.individual_maxima_status,
        evidence_status: HYPOTHETICAL,
        base_scenario_sha256: base_digest,
        hypothetical_scenario_sha256: report.scenario_sha256,
    }))
}

pub fn bundled_fixture() -> Result<Value, String> {
    serde_json::from_str(include_str!(
        "../tests/fixtures/SAFRB_TWO_WINDOW_SHARED_PUMP_V0_1.json"
    ))
    .map_err(|e| format!("shared-pump fixture JSON: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scenario() -> Value {
        bundled_fixture().unwrap()["scenario"].clone()
    }

    #[test]
    fn individual_window_maxima_do_not_masquerade_as_joint_feasibility() {
        let node_rows = [
            ("source", "source"),
            ("shared-in", "resource_input"),
            ("shared-out", "resource_output"),
            ("service-a", "beneficiary_service"),
            ("service-b", "beneficiary_service"),
            ("sink", "sink"),
        ];
        let nodes = node_rows
            .iter()
            .map(|(id, kind)| {
                (
                    (*id).to_string(),
                    Node {
                        id: (*id).to_string(),
                        kind: (*kind).to_string(),
                        time: 0,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();

        let edge = |id: &str, from: &str, to: &str, cap: u64| Edge {
            id: id.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            lower: cap,
            upper: cap,
            resource: id.to_string(),
            time: 0,
        };
        let edges = vec![
            edge("supply", "source", "shared-in", 8),
            edge("shared-resource-gate", "shared-in", "shared-out", 8),
            edge("branch-a", "shared-out", "service-a", 5),
            edge("branch-b", "shared-out", "service-b", 5),
            edge("obligation-a", "service-a", "sink", 5),
            edge("obligation-b", "service-b", "sink", 5),
        ];
        let parsed = Parsed {
            case_id: "joint-vs-individual-test".to_string(),
            scope: "FUEL-DELIVERABILITY".to_string(),
            commodity: "synthetic-commodity".to_string(),
            unit: "t".to_string(),
            boundary: "test-boundary".to_string(),
            source: "source".to_string(),
            sink: "sink".to_string(),
            nodes,
            edges,
            obligations: vec![
                Obligation {
                    id: "window-a".to_string(),
                    node_id: "service-a".to_string(),
                    time: 0,
                    interval: "[0,1)".to_string(),
                    required: 5,
                    edge_id: "obligation-a".to_string(),
                },
                Obligation {
                    id: "window-b".to_string(),
                    node_id: "service-b".to_string(),
                    time: 0,
                    interval: "[0,1)".to_string(),
                    required: 5,
                    edge_id: "obligation-b".to_string(),
                },
            ],
            gate_ids: BTreeSet::from(["shared-resource-gate".to_string()]),
            scenario_sha256: "sha256:test".to_string(),
            hypothetical: false,
        };

        let joint = build_graph(&parsed, false, None).unwrap().solve().unwrap();
        let individual_a = build_graph(&parsed, false, Some("obligation-a"))
            .unwrap()
            .solve()
            .unwrap();
        let individual_b = build_graph(&parsed, false, Some("obligation-b"))
            .unwrap()
            .solve()
            .unwrap();

        assert_eq!(joint.max_flow, 8);
        assert_eq!(individual_a.max_flow, 5);
        assert_eq!(individual_b.max_flow, 5);
        assert_eq!(
            individual_a.max_flow + individual_b.max_flow,
            10,
            "individual maxima are not additive when obligations share capacity"
        );
        assert!(
            joint.max_flow < 10,
            "the combined 10t requirement is not jointly feasible"
        );
    }

    #[test]
    fn frozen_two_window_case_has_9t_lower_and_12t_upper() {
        let document = bundled_fixture().unwrap();
        let input = document["scenario"].clone();
        let expected = &document["certificate"];
        let result = evaluate_scenario(&input).unwrap();
        assert_eq!(
            result.scenario_sha256,
            expected["scenario_sha256"].as_str().unwrap()
        );
        assert_eq!(result.lower.max_flow, 9);
        assert_eq!(result.lower.min_cut.capacity, 9);
        assert_eq!(result.upper.max_flow, 12);
        assert_eq!(result.upper.min_cut.capacity, 12);
        assert_eq!(result.windows.len(), 2);
        assert_eq!(
            (
                result.windows[0].lower_individual_max_deliverable,
                result.windows[0].upper_individual_max_deliverable
            ),
            (Some(6), Some(8))
        );
        assert_eq!(
            (
                result.windows[1].lower_individual_max_deliverable,
                result.windows[1].upper_individual_max_deliverable
            ),
            (Some(3), Some(8))
        );
        assert_eq!(
            (
                result.windows[0].lower_joint_witness_allocation,
                result.windows[1].lower_joint_witness_allocation
            ),
            (6, 3)
        );
        assert_eq!(
            result.individual_maxima_status,
            IndividualMaximaStatus::ComputedForInfeasibleJointService
        );
        assert_eq!(
            (
                result.windows[0].upper_joint_witness_allocation,
                result.windows[1].upper_joint_witness_allocation
            ),
            (6, 6)
        );
        result.lower_bundle.verify().unwrap();
        result.upper_bundle.verify().unwrap();
        assert_eq!(result.lower_bundle.result.max_flow, result.lower.max_flow);
        assert_eq!(result.upper_bundle.result.max_flow, result.upper.max_flow);
        let lower_json = serde_json::to_vec(&result.lower_bundle).unwrap();
        let upper_json = serde_json::to_vec(&result.upper_bundle).unwrap();
        let lower_round_trip: FlowCertificateBundle = serde_json::from_slice(&lower_json).unwrap();
        let upper_round_trip: FlowCertificateBundle = serde_json::from_slice(&upper_json).unwrap();
        lower_round_trip.verify().unwrap();
        upper_round_trip.verify().unwrap();
        for window in &result.windows {
            let lower_bundle = window.lower_individual_certificate.as_ref().unwrap();
            let upper_bundle = window.upper_individual_certificate.as_ref().unwrap();
            lower_bundle.verify().unwrap();
            upper_bundle.verify().unwrap();
            let lower_round_trip: FlowCertificateBundle =
                serde_json::from_slice(&serde_json::to_vec(lower_bundle).unwrap()).unwrap();
            let upper_round_trip: FlowCertificateBundle =
                serde_json::from_slice(&serde_json::to_vec(upper_bundle).unwrap()).unwrap();
            lower_round_trip.verify().unwrap();
            upper_round_trip.verify().unwrap();
            assert_eq!(
                Some(lower_bundle.result.max_flow),
                window.lower_individual_max_deliverable
            );
            assert_eq!(
                Some(upper_bundle.result.max_flow),
                window.upper_individual_max_deliverable
            );
            assert_eq!(
                lower_bundle.result.max_flow,
                lower_bundle.result.min_cut.capacity
            );
            assert_eq!(
                upper_bundle.result.max_flow,
                upper_bundle.result.min_cut.capacity
            );
            assert!(lower_bundle.result.graph_sha256.starts_with("sha256:"));
            assert!(upper_bundle.result.graph_sha256.starts_with("sha256:"));
        }
        assert_eq!(
            result.disposition,
            ServiceDisposition::IndeterminateNotGuaranteed
        );
        let expected_witness = expected["flow_witness"].as_object().unwrap();
        for edge in &result.lower.edge_flows {
            assert_eq!(
                edge.flow,
                expected_witness
                    .get(edge.id.as_str())
                    .unwrap()
                    .as_u64()
                    .unwrap(),
                "lower flow witness edge {}",
                edge.id
            );
        }
        let expected_cut = expected["cut_edge_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_string())
            .collect::<BTreeSet<_>>();
        let actual_cut = result
            .lower
            .min_cut
            .edges
            .iter()
            .map(|edge| edge.id.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(actual_cut, expected_cut);
    }

    #[test]
    fn individual_maxima_budget_dispositions_are_explicit() {
        assert_eq!(
            individual_maxima_status(true, true, 128),
            IndividualMaximaStatus::NotNeededJointServiceFeasible
        );
        assert_eq!(
            individual_maxima_status(false, true, 2),
            IndividualMaximaStatus::ComputedForInfeasibleJointService
        );
        assert_eq!(
            individual_maxima_status(false, true, 33),
            IndividualMaximaStatus::OmittedAboveDeclaredObligationLimit
        );
        assert_eq!(
            individual_maxima_status(false, false, 2),
            IndividualMaximaStatus::OmittedForCounterfactualSearch
        );
    }

    #[test]
    fn joint_lower_feasibility_skips_redundant_individual_solves() {
        let mut input = scenario();
        input["edges"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|edge| edge["id"] == "pump-physical-t1")
            .unwrap()["capacity_lower"] = Value::from(6);
        input["capacity_evidence"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["time_index"] == 1)
            .unwrap()["lower"] = Value::from(6);

        let result = evaluate_scenario(&input).unwrap();
        assert_eq!(
            result.disposition,
            ServiceDisposition::GuaranteedWithinDeclaredLowerEnvelope
        );
        assert_eq!(
            result.individual_maxima_status,
            IndividualMaximaStatus::NotNeededJointServiceFeasible
        );
        assert!(result.windows.iter().all(|window| {
            window.lower_joint_witness_allocation == window.required
                && window.lower_individual_max_deliverable.is_none()
                && window.lower_individual_certificate.is_none()
                && window.disposition
                    == Some(WindowDisposition::IndividuallyFeasibleWithinDeclaredLowerEnvelope)
        }));
    }

    #[test]
    fn individual_maximum_is_not_truncated_at_the_service_floor() {
        let mut input = scenario();
        input["service_obligations"][0]["required_lower"] = Value::from(5);
        input["edges"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|edge| edge["id"] == "service-obligation-t1")
            .unwrap()["capacity_lower"] = Value::from(5);
        input["edges"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|edge| edge["id"] == "service-obligation-t1")
            .unwrap()["capacity_upper"] = Value::from(5);

        // The second obligation remains at 6 t; the joint network is infeasible
        // under the conservative capacities. Window 1 can individually receive
        // 6 t through the shared pump although its required floor is 5 t.
        let result = evaluate_scenario(&input).unwrap();
        assert_eq!(result.windows[0].required, 5);
        assert_eq!(result.windows[0].lower_individual_max_deliverable, Some(6));
        assert_eq!(result.windows[0].upper_individual_max_deliverable, Some(8));
    }

    #[test]
    fn threshold_is_plus_three_and_explicitly_not_admitted() {
        let input = scenario();
        let before = digest(&input).unwrap();
        let result = minimum_capacity_counterfactual(&input, "pump-physical-t1")
            .unwrap()
            .unwrap();
        assert_eq!(result.base_lower, 3);
        assert_eq!(result.hypothetical_lower, 6);
        assert_eq!(result.additional_capacity, 3);
        assert_eq!(result.lower_flow, 12);
        assert_eq!(result.lower_cut, 12);
        assert_eq!(result.lower_certificate.max_flow, 12);
        assert_eq!(result.lower_certificate.min_cut.capacity, 12);
        assert_eq!(result.upper_certificate.max_flow, 12);
        result.lower_bundle.verify().unwrap();
        result.upper_bundle.verify().unwrap();
        let lower_round_trip: FlowCertificateBundle =
            serde_json::from_slice(&serde_json::to_vec(&result.lower_bundle).unwrap()).unwrap();
        let upper_round_trip: FlowCertificateBundle =
            serde_json::from_slice(&serde_json::to_vec(&result.upper_bundle).unwrap()).unwrap();
        lower_round_trip.verify().unwrap();
        upper_round_trip.verify().unwrap();
        assert_eq!(result.lower_bundle.result.max_flow, 12);
        assert_eq!(result.upper_bundle.result.max_flow, 12);
        assert_eq!(
            result.individual_maxima_status,
            IndividualMaximaStatus::NotNeededJointServiceFeasible
        );
        assert!(result.windows.iter().all(|window| {
            window.lower_individual_max_deliverable.is_none()
                && window.upper_individual_max_deliverable.is_none()
                && window.lower_individual_certificate.is_none()
                && window.upper_individual_certificate.is_none()
        }));
        assert_eq!(result.evidence_status, HYPOTHETICAL);
        assert_eq!(digest(&input).unwrap(), before);
    }

    #[test]
    fn alias_split_cannot_duplicate_shared_physical_capacity() {
        let mut input = scenario();
        input["resource_aliases"][1]["physical_resource_id"] =
            Value::String("physical:pump-02".into());
        assert!(evaluate_scenario(&input).is_err());
    }

    #[test]
    fn unit_bound_and_capacity_interval_are_fail_closed() {
        let mut input = scenario();
        input["edges"][0]["unit"] = Value::String("kg".into());
        assert!(evaluate_scenario(&input).is_err());
        let mut input = scenario();
        input["edges"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|e| e["id"] == "pump-physical-t1")
            .unwrap()["capacity_lower"] = Value::from(9);
        assert!(evaluate_scenario(&input).is_err());
    }

    #[test]
    fn half_open_interval_rejects_u64_max_time_index() {
        let maximum = u64::MAX.to_string();
        let interval = format!("[{maximum},{maximum})");
        assert!(half_open(&interval, "$.interval", u64::MAX).is_err());
    }

    #[test]
    fn stale_evidence_and_time_reordering_are_rejected() {
        let mut input = scenario();
        input["capacity_evidence"][1]["valid_interval"] = Value::String("[0,1)".into());
        assert!(evaluate_scenario(&input).is_err());
        let mut input = scenario();
        input["edges"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|e| e["id"] == "pump-physical-t1")
            .unwrap()["time_index"] = Value::from(0);
        assert!(evaluate_scenario(&input).is_err());
    }

    #[test]
    fn opening_stock_alias_duplication_is_rejected() {
        let mut input = scenario();
        input["total_opening_stock"]["source_ids"][1] = Value::String("terminal-a-stock".into());
        assert!(evaluate_scenario(&input).is_err());
    }

    #[test]
    fn obligation_must_reference_the_declared_beneficiary_service_node() {
        let mut input = scenario();
        input["service_obligations"][0]["node_id"] =
            Value::String("missing-beneficiary-service-node".into());
        assert!(evaluate_scenario(&input).is_err());
    }

    #[test]
    fn unlisted_beneficiary_service_nodes_are_rejected() {
        let mut input = scenario();
        input["nodes"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "id": "undeclared-beneficiary-window",
                "kind": "beneficiary_service",
                "time_index": 2
            }));
        assert!(evaluate_scenario(&input).is_err());
    }

    #[test]
    fn unknown_schema_properties_and_route_alias_mismatches_are_rejected() {
        let mut input = scenario();
        input["edges"][0]["unrecognized_capacity_hint"] = Value::from(999);
        assert!(evaluate_scenario(&input).is_err());

        let mut input = scenario();
        input["route_transit"][1]["physical_resources"][0] =
            Value::String("physical:pump-02".into());
        assert!(evaluate_scenario(&input).is_err());

        let mut input = scenario();
        input["unrecognized_top_level_semantics"] = Value::Bool(true);
        assert!(evaluate_scenario(&input).is_err());
    }

    #[test]
    fn serial_physical_gates_are_allowed_but_all_gate_bypass_is_not() {
        let node_rows = [
            "source",
            "pump-in",
            "pump-out",
            "treatment-in",
            "treatment-out",
            "sink",
        ]
        .iter()
        .map(|id| {
            (
                (*id).to_string(),
                Node {
                    id: (*id).to_string(),
                    kind: "stage".to_string(),
                    time: 0,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
        let make_edge = |id: &str, from: &str, to: &str| Edge {
            id: id.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            lower: 1,
            upper: 1,
            resource: id.to_string(),
            time: 0,
        };
        let mut edges = vec![
            make_edge("feed", "source", "pump-in"),
            make_edge("pump-gate", "pump-in", "pump-out"),
            make_edge("pump-to-treatment", "pump-out", "treatment-in"),
            make_edge("treatment-gate", "treatment-in", "treatment-out"),
            make_edge("distribution", "treatment-out", "sink"),
        ];
        let gates = BTreeSet::from(["pump-gate".to_string(), "treatment-gate".to_string()]);
        assert!(!path_exists_excluding(
            &node_rows, &edges, &gates, "source", "sink"
        ));

        for gate_id in &gates {
            let excluded = BTreeSet::from([gate_id.clone()]);
            let gate = edges.iter().find(|edge| &edge.id == gate_id).unwrap();
            assert!(path_exists_excluding(
                &node_rows, &edges, &excluded, "source", &gate.from
            ));
            assert!(path_exists_excluding(
                &node_rows, &edges, &excluded, &gate.to, "sink"
            ));
        }

        edges.push(make_edge("unmetered-bypass", "source", "sink"));
        assert!(path_exists_excluding(
            &node_rows, &edges, &gates, "source", "sink"
        ));
    }

    #[test]
    fn a_new_service_path_cannot_bypass_the_shared_pump() {
        let mut input = scenario();
        input["edges"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "id": "unqualified-pump-bypass",
                "from": "inventory-a-t0",
                "to": "service-t1",
                "capacity_lower": 1,
                "capacity_upper": 1,
                "unit": "t",
                "resource_id": "unqualified-bypass",
                "time_index": 0
            }));
        assert!(evaluate_scenario(&input).is_err());
    }

    #[test]
    fn unsupported_scope_and_unreferenced_capacity_evidence_are_rejected() {
        let mut input = scenario();
        input["claim_scope"] = Value::String("WHOLE-BENEFICIARY-SERVICE".into());
        assert!(evaluate_scenario(&input).is_err());

        let mut input = scenario();
        let extra = input["capacity_evidence"][0].clone();
        input["capacity_evidence"]
            .as_array_mut()
            .unwrap()
            .push(extra);
        assert!(evaluate_scenario(&input).is_err());
    }
}
