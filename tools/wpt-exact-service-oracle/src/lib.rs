// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact small-instance feasibility oracle for interval-based WPT energy service.
//!
//! The oracle uses integer mWh capacities and a time-expanded max-flow network.
//! It answers whether every receiver's minimum energy obligation can be met
//! before its departure deadline, given already-qualified conservative
//! segment, transfer, and receiver-acceptance envelopes. It does not estimate
//! those envelopes, optimize fairness, or authorize physical operation.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const MAX_INTERVALS: u64 = 256;
const MAX_SEGMENT_SLOTS: usize = 4096;
const MAX_RECEIVERS: usize = 128;
const MAX_RECEIVER_SLOTS: usize = 16384;
const MAX_TRANSFER_LINKS: usize = 8192;

fn fail(path: &str, message: impl AsRef<str>) -> String {
    format!("{path}: {}", message.as_ref())
}

fn object<'a>(value: &'a Value, path: &str) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))
}

fn exact_keys(value: &Value, allowed: &[&str], path: &str) -> Result<(), String> {
    for key in object(value, path)?.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(fail(path, format!("unknown property {key}")));
        }
    }
    Ok(())
}

fn field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    value
        .as_object()
        .and_then(|map| map.get(key))
        .ok_or_else(|| fail(path, format!("missing property {key}")))
}

fn string_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    field(value, key, path)?
        .as_str()
        .filter(|item| !item.trim().is_empty())
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-empty string"))
}

fn uint_field(value: &Value, key: &str, path: &str) -> Result<u64, String> {
    field(value, key, path)?
        .as_u64()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-negative integer"))
}

fn array_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a [Value], String> {
    field(value, key, path)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be an array"))
}

#[derive(Debug, Clone)]
struct Receiver {
    minimum_service_mwh: u64,
    arrival_interval: usize,
    departure_interval: usize,
}

#[derive(Debug, Clone)]
struct SegmentSlot {
    segment_id: String,
    interval_index: usize,
    available_deliverable_energy_mwh: u64,
}

#[derive(Debug, Clone)]
struct ReceiverSlot {
    receiver_id: String,
    interval_index: usize,
    max_acceptance_mwh: u64,
}

#[derive(Debug, Clone)]
struct TransferLink {
    link_id: String,
    segment_id: String,
    receiver_id: String,
    interval_index: usize,
    max_deliverable_energy_mwh: u64,
}

#[derive(Debug, Clone)]
struct Scenario {
    scenario_id: String,
    interval_count: usize,
    receivers: BTreeMap<String, Receiver>,
    segment_slots: BTreeMap<(String, usize), SegmentSlot>,
    receiver_slots: BTreeMap<(String, usize), ReceiverSlot>,
    transfer_links: Vec<TransferLink>,
    total_minimum_service_mwh: u64,
}

fn parse_scenario(value: &Value) -> Result<Scenario, String> {
    exact_keys(
        value,
        &[
            "schema_version",
            "scenario_id",
            "evidence_kind",
            "interval_count",
            "segment_slots",
            "receivers",
            "receiver_slots",
            "transfer_links",
            "quality_flags",
        ],
        "$",
    )?;

    if string_field(value, "schema_version", "$")? != "wpt-exact-service-oracle-v0" {
        return Err(fail("$.schema_version", "unsupported schema version"));
    }
    let scenario_id = string_field(value, "scenario_id", "$")?.to_string();
    if string_field(value, "evidence_kind", "$")? != "SYNTHETIC_KNOWN_TRUTH" {
        return Err(fail(
            "$.evidence_kind",
            "v0 admits synthetic known-truth scenarios only",
        ));
    }
    let quality_flags = array_field(value, "quality_flags", "$")?;
    if quality_flags.len() != 1 || quality_flags[0].as_str() != Some("SYNTHETIC_ONLY") {
        return Err(fail(
            "$.quality_flags",
            "v0 scenarios must carry exactly SYNTHETIC_ONLY",
        ));
    }

    let interval_count_value = uint_field(value, "interval_count", "$")?;
    if !(1..=MAX_INTERVALS).contains(&interval_count_value) {
        return Err(fail(
            "$.interval_count",
            format!("must be within 1..={MAX_INTERVALS}"),
        ));
    }
    let interval_count = usize::try_from(interval_count_value)
        .map_err(|_| fail("$.interval_count", "does not fit this platform"))?;

    let receiver_rows = array_field(value, "receivers", "$")?;
    if receiver_rows.is_empty() || receiver_rows.len() > MAX_RECEIVERS {
        return Err(fail(
            "$.receivers",
            format!("must contain 1..={MAX_RECEIVERS} receivers"),
        ));
    }
    let mut receivers = BTreeMap::new();
    let mut total_minimum_service_mwh = 0_u64;
    for (index, row) in receiver_rows.iter().enumerate() {
        let path = format!("$.receivers[{index}]");
        exact_keys(
            row,
            &[
                "receiver_id",
                "arrival_interval",
                "departure_interval",
                "minimum_service_mwh",
            ],
            &path,
        )?;
        let receiver_id = string_field(row, "receiver_id", &path)?.to_string();
        let arrival = uint_field(row, "arrival_interval", &path)?;
        let departure = uint_field(row, "departure_interval", &path)?;
        let minimum = uint_field(row, "minimum_service_mwh", &path)?;
        if arrival >= departure || departure > interval_count as u64 {
            return Err(fail(
                &path,
                "receiver window must satisfy 0 <= arrival < departure <= interval_count",
            ));
        }
        if minimum == 0 {
            return Err(fail(&path, "minimum_service_mwh must be positive"));
        }
        total_minimum_service_mwh = total_minimum_service_mwh
            .checked_add(minimum)
            .ok_or_else(|| fail("$.receivers", "total minimum service energy overflowed u64"))?;
        if receivers
            .insert(
                receiver_id.clone(),
                Receiver {
                    minimum_service_mwh: minimum,
                    arrival_interval: arrival as usize,
                    departure_interval: departure as usize,
                },
            )
            .is_some()
        {
            return Err(fail(&path, format!("duplicate receiver_id {receiver_id}")));
        }
    }

    let segment_rows = array_field(value, "segment_slots", "$")?;
    if segment_rows.is_empty() || segment_rows.len() > MAX_SEGMENT_SLOTS {
        return Err(fail(
            "$.segment_slots",
            format!("must contain 1..={MAX_SEGMENT_SLOTS} segment slots"),
        ));
    }
    let mut segment_slots = BTreeMap::new();
    for (index, row) in segment_rows.iter().enumerate() {
        let path = format!("$.segment_slots[{index}]");
        exact_keys(
            row,
            &[
                "segment_id",
                "interval_index",
                "available_deliverable_energy_mwh",
            ],
            &path,
        )?;
        let segment_id = string_field(row, "segment_id", &path)?.to_string();
        let interval = uint_field(row, "interval_index", &path)?;
        if interval >= interval_count as u64 {
            return Err(fail(&path, "interval_index is outside interval_count"));
        }
        let slot = SegmentSlot {
            segment_id: segment_id.clone(),
            interval_index: interval as usize,
            available_deliverable_energy_mwh: uint_field(
                row,
                "available_deliverable_energy_mwh",
                &path,
            )?,
        };
        if segment_slots
            .insert((segment_id.clone(), interval as usize), slot)
            .is_some()
        {
            return Err(fail(
                &path,
                format!("duplicate segment slot {segment_id}@{interval}"),
            ));
        }
    }

    let receiver_slot_rows = array_field(value, "receiver_slots", "$")?;
    if receiver_slot_rows.len() > MAX_RECEIVER_SLOTS {
        return Err(fail(
            "$.receiver_slots",
            format!("must contain at most {MAX_RECEIVER_SLOTS} receiver slots"),
        ));
    }
    let mut receiver_slots = BTreeMap::new();
    for (index, row) in receiver_slot_rows.iter().enumerate() {
        let path = format!("$.receiver_slots[{index}]");
        exact_keys(
            row,
            &["receiver_id", "interval_index", "max_acceptance_mwh"],
            &path,
        )?;
        let receiver_id = string_field(row, "receiver_id", &path)?.to_string();
        let receiver = receivers
            .get(&receiver_id)
            .ok_or_else(|| fail(&path, format!("unknown receiver_id {receiver_id}")))?;
        let interval = uint_field(row, "interval_index", &path)?;
        if interval >= interval_count as u64
            || interval < receiver.arrival_interval as u64
            || interval >= receiver.departure_interval as u64
        {
            return Err(fail(
                &path,
                "receiver slot must lie inside the receiver's [arrival, departure) service window",
            ));
        }
        let slot = ReceiverSlot {
            receiver_id: receiver_id.clone(),
            interval_index: interval as usize,
            max_acceptance_mwh: uint_field(row, "max_acceptance_mwh", &path)?,
        };
        if receiver_slots
            .insert((receiver_id.clone(), interval as usize), slot)
            .is_some()
        {
            return Err(fail(
                &path,
                format!("duplicate receiver slot {receiver_id}@{interval}"),
            ));
        }
    }

    let link_rows = array_field(value, "transfer_links", "$")?;
    if link_rows.len() > MAX_TRANSFER_LINKS {
        return Err(fail(
            "$.transfer_links",
            format!("must contain at most {MAX_TRANSFER_LINKS} transfer links"),
        ));
    }
    let mut link_ids = BTreeSet::new();
    let mut transfer_links = Vec::with_capacity(link_rows.len());
    for (index, row) in link_rows.iter().enumerate() {
        let path = format!("$.transfer_links[{index}]");
        exact_keys(
            row,
            &[
                "link_id",
                "segment_id",
                "receiver_id",
                "interval_index",
                "max_deliverable_energy_mwh",
            ],
            &path,
        )?;
        let link_id = string_field(row, "link_id", &path)?.to_string();
        if !link_ids.insert(link_id.clone()) {
            return Err(fail(&path, format!("duplicate link_id {link_id}")));
        }
        let segment_id = string_field(row, "segment_id", &path)?.to_string();
        let receiver_id = string_field(row, "receiver_id", &path)?.to_string();
        let interval = uint_field(row, "interval_index", &path)?;
        if interval >= interval_count as u64 {
            return Err(fail(&path, "interval_index is outside interval_count"));
        }
        let receiver = receivers
            .get(&receiver_id)
            .ok_or_else(|| fail(&path, format!("unknown receiver_id {receiver_id}")))?;
        if interval < receiver.arrival_interval as u64
            || interval >= receiver.departure_interval as u64
        {
            return Err(fail(
                &path,
                "transfer link is outside the receiver's [arrival, departure) service window",
            ));
        }
        if !segment_slots.contains_key(&(segment_id.clone(), interval as usize)) {
            return Err(fail(
                &path,
                format!("missing segment slot {segment_id}@{interval}"),
            ));
        }
        if !receiver_slots.contains_key(&(receiver_id.clone(), interval as usize)) {
            return Err(fail(
                &path,
                format!("missing receiver slot {receiver_id}@{interval}"),
            ));
        }
        transfer_links.push(TransferLink {
            link_id,
            segment_id,
            receiver_id,
            interval_index: interval as usize,
            max_deliverable_energy_mwh: uint_field(row, "max_deliverable_energy_mwh", &path)?,
        });
    }

    transfer_links.sort_by_key(|link| link.link_id.clone());
    Ok(Scenario {
        scenario_id,
        interval_count,
        receivers,
        segment_slots,
        receiver_slots,
        transfer_links,
        total_minimum_service_mwh,
    })
}

#[derive(Debug, Clone)]
struct Edge {
    to: usize,
    reverse_index: usize,
    residual_mwh: u64,
}

#[derive(Debug, Clone)]
struct EdgeRecord {
    from: usize,
    to: usize,
    capacity_mwh: u64,
    kind: &'static str,
    label: String,
}

fn add_edge(
    graph: &mut [Vec<Edge>],
    records: &mut Vec<EdgeRecord>,
    from: usize,
    to: usize,
    capacity_mwh: u64,
    kind: &'static str,
    label: String,
) {
    let forward_index = graph[from].len();
    let reverse_index = graph[to].len();
    graph[from].push(Edge {
        to,
        reverse_index,
        residual_mwh: capacity_mwh,
    });
    graph[to].push(Edge {
        to: from,
        reverse_index: forward_index,
        residual_mwh: 0,
    });
    records.push(EdgeRecord {
        from,
        to,
        capacity_mwh,
        kind,
        label,
    });
}

fn build_level_graph(graph: &[Vec<Edge>], source: usize) -> Vec<i32> {
    let mut levels = vec![-1; graph.len()];
    let mut queue = VecDeque::new();
    levels[source] = 0;
    queue.push_back(source);
    while let Some(node) = queue.pop_front() {
        for edge in &graph[node] {
            if edge.residual_mwh > 0 && levels[edge.to] == -1 {
                levels[edge.to] = levels[node] + 1;
                queue.push_back(edge.to);
            }
        }
    }
    levels
}

fn send_flow(
    graph: &mut [Vec<Edge>],
    levels: &[i32],
    next_edge: &mut [usize],
    node: usize,
    sink: usize,
    available_mwh: u64,
) -> u64 {
    if node == sink || available_mwh == 0 {
        return available_mwh;
    }

    while next_edge[node] < graph[node].len() {
        let edge_index = next_edge[node];
        let to = graph[node][edge_index].to;
        let residual = graph[node][edge_index].residual_mwh;
        let reverse_index = graph[node][edge_index].reverse_index;
        if residual > 0 && levels[to] == levels[node] + 1 {
            let pushed = send_flow(
                graph,
                levels,
                next_edge,
                to,
                sink,
                available_mwh.min(residual),
            );
            if pushed > 0 {
                graph[node][edge_index].residual_mwh -= pushed;
                graph[to][reverse_index].residual_mwh += pushed;
                return pushed;
            }
        }
        next_edge[node] += 1;
    }
    0
}

fn max_flow(graph: &mut [Vec<Edge>], source: usize, sink: usize, flow_limit_mwh: u64) -> u64 {
    let mut total_flow = 0_u64;
    loop {
        if total_flow == flow_limit_mwh {
            return total_flow;
        }
        let levels = build_level_graph(graph, source);
        if levels[sink] == -1 {
            return total_flow;
        }
        let mut next_edge = vec![0_usize; graph.len()];
        loop {
            let pushed = send_flow(
                graph,
                &levels,
                &mut next_edge,
                source,
                sink,
                flow_limit_mwh - total_flow,
            );
            if pushed == 0 {
                break;
            }
            total_flow += pushed;
        }
    }
}

fn residual_reachable(graph: &[Vec<Edge>], source: usize) -> Vec<bool> {
    let mut reachable = vec![false; graph.len()];
    let mut queue = VecDeque::new();
    reachable[source] = true;
    queue.push_back(source);
    while let Some(node) = queue.pop_front() {
        for edge in &graph[node] {
            if edge.residual_mwh > 0 && !reachable[edge.to] {
                reachable[edge.to] = true;
                queue.push_back(edge.to);
            }
        }
    }
    reachable
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReceiverAllocation {
    pub receiver_id: String,
    pub minimum_service_mwh: u64,
    pub delivered_mwh: u64,
    pub shortfall_mwh: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SegmentSlotAllocation {
    pub segment_id: String,
    pub interval_index: usize,
    pub available_deliverable_energy_mwh: u64,
    pub allocated_mwh: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransferAllocation {
    pub link_id: String,
    pub segment_id: String,
    pub receiver_id: String,
    pub interval_index: usize,
    pub transfer_capacity_mwh: u64,
    pub allocated_mwh: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReceiverSlotAllocation {
    pub receiver_id: String,
    pub interval_index: usize,
    pub max_acceptance_mwh: u64,
    pub allocated_mwh: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MinCutEdge {
    pub kind: String,
    pub label: String,
    pub capacity_mwh: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServiceOracleReport {
    pub scenario_id: String,
    pub interval_count: usize,
    pub receiver_count: usize,
    pub segment_slot_count: usize,
    pub transfer_link_count: usize,
    pub total_minimum_service_mwh: u64,
    pub maximum_deliverable_mwh: u64,
    pub total_shortfall_mwh: u64,
    pub feasible: bool,
    pub receiver_allocations: Vec<ReceiverAllocation>,
    /// Constructive interval-level use of each segment's deliverable-energy budget.
    pub segment_allocations: Vec<SegmentSlotAllocation>,
    /// Constructive energy assignment on each qualified transfer link.
    pub transfer_allocations: Vec<TransferAllocation>,
    /// Constructive receiver-side acceptance use in each interval.
    pub receiver_slot_allocations: Vec<ReceiverSlotAllocation>,
    pub minimum_cut_capacity_mwh: u64,
    pub minimum_cut_edges: Vec<MinCutEdge>,
    pub claim_ceiling: &'static str,
}

/// Evaluate whether all minimum receiver-service obligations can be met.
///
/// Capacities are integer milliwatt-hours at one common *deliverable-energy*
/// boundary. The caller must supply conservative qualified bounds; this
/// oracle does not infer conversion losses or authenticate measurements.
pub fn evaluate_scenario(value: &Value) -> Result<ServiceOracleReport, String> {
    let scenario = parse_scenario(value)?;
    let source = 0_usize;
    let mut graph = vec![Vec::<Edge>::new()];
    let mut segment_nodes = BTreeMap::new();
    let mut receiver_slot_nodes = BTreeMap::new();
    let mut receiver_demand_nodes = BTreeMap::new();

    for key in scenario.segment_slots.keys() {
        let node = graph.len();
        graph.push(Vec::new());
        segment_nodes.insert(key.clone(), node);
    }
    for key in scenario.receiver_slots.keys() {
        let node = graph.len();
        graph.push(Vec::new());
        receiver_slot_nodes.insert(key.clone(), node);
    }
    for receiver_id in scenario.receivers.keys() {
        let node = graph.len();
        graph.push(Vec::new());
        receiver_demand_nodes.insert(receiver_id.clone(), node);
    }
    let sink = graph.len();
    graph.push(Vec::new());
    let mut records = Vec::new();
    let mut segment_edge_records = BTreeMap::new();
    let mut transfer_edge_records = BTreeMap::new();
    let mut receiver_slot_edge_records = BTreeMap::new();

    for (key, slot) in &scenario.segment_slots {
        let node = segment_nodes[key];
        let edge_index = graph[source].len();
        add_edge(
            &mut graph,
            &mut records,
            source,
            node,
            slot.available_deliverable_energy_mwh,
            "segment_supply",
            format!("{}@{}", slot.segment_id, slot.interval_index),
        );
        segment_edge_records.insert(key.clone(), (source, edge_index));
    }

    for link in &scenario.transfer_links {
        let segment_key = (link.segment_id.clone(), link.interval_index);
        let receiver_key = (link.receiver_id.clone(), link.interval_index);
        let from = segment_nodes[&segment_key];
        let edge_index = graph[from].len();
        add_edge(
            &mut graph,
            &mut records,
            from,
            receiver_slot_nodes[&receiver_key],
            link.max_deliverable_energy_mwh,
            "transfer_link",
            link.link_id.clone(),
        );
        transfer_edge_records.insert(link.link_id.clone(), (from, edge_index));
    }

    for (key, slot) in &scenario.receiver_slots {
        let from = receiver_slot_nodes[key];
        let edge_index = graph[from].len();
        add_edge(
            &mut graph,
            &mut records,
            from,
            receiver_demand_nodes[&slot.receiver_id],
            slot.max_acceptance_mwh,
            "receiver_acceptance",
            format!("{}@{}", slot.receiver_id, slot.interval_index),
        );
        receiver_slot_edge_records.insert(key.clone(), (from, edge_index));
    }

    let mut demand_edge_records = BTreeMap::new();
    for (receiver_id, receiver) in &scenario.receivers {
        let from = receiver_demand_nodes[receiver_id];
        let edge_index = graph[from].len();
        add_edge(
            &mut graph,
            &mut records,
            from,
            sink,
            receiver.minimum_service_mwh,
            "receiver_obligation",
            receiver_id.clone(),
        );
        demand_edge_records.insert(receiver_id.clone(), (from, edge_index));
    }

    let delivered = max_flow(&mut graph, source, sink, scenario.total_minimum_service_mwh);
    let mut receiver_allocations = Vec::with_capacity(scenario.receivers.len());
    for (receiver_id, receiver) in &scenario.receivers {
        let (from, edge_index) = demand_edge_records[receiver_id];
        let residual = graph[from][edge_index].residual_mwh;
        let delivered_mwh = receiver.minimum_service_mwh - residual;
        receiver_allocations.push(ReceiverAllocation {
            receiver_id: receiver_id.clone(),
            minimum_service_mwh: receiver.minimum_service_mwh,
            delivered_mwh,
            shortfall_mwh: receiver.minimum_service_mwh - delivered_mwh,
        });
    }

    let segment_allocations = scenario
        .segment_slots
        .iter()
        .map(|(key, slot)| {
            let (from, edge_index) = segment_edge_records[key];
            SegmentSlotAllocation {
                segment_id: slot.segment_id.clone(),
                interval_index: slot.interval_index,
                available_deliverable_energy_mwh: slot.available_deliverable_energy_mwh,
                allocated_mwh: slot.available_deliverable_energy_mwh
                    - graph[from][edge_index].residual_mwh,
            }
        })
        .collect::<Vec<_>>();
    let transfer_allocations = scenario
        .transfer_links
        .iter()
        .map(|link| {
            let (from, edge_index) = transfer_edge_records[&link.link_id];
            TransferAllocation {
                link_id: link.link_id.clone(),
                segment_id: link.segment_id.clone(),
                receiver_id: link.receiver_id.clone(),
                interval_index: link.interval_index,
                transfer_capacity_mwh: link.max_deliverable_energy_mwh,
                allocated_mwh: link.max_deliverable_energy_mwh
                    - graph[from][edge_index].residual_mwh,
            }
        })
        .collect::<Vec<_>>();
    let receiver_slot_allocations = scenario
        .receiver_slots
        .iter()
        .map(|(key, slot)| {
            let (from, edge_index) = receiver_slot_edge_records[key];
            ReceiverSlotAllocation {
                receiver_id: slot.receiver_id.clone(),
                interval_index: slot.interval_index,
                max_acceptance_mwh: slot.max_acceptance_mwh,
                allocated_mwh: slot.max_acceptance_mwh - graph[from][edge_index].residual_mwh,
            }
        })
        .collect::<Vec<_>>();

    let feasible = delivered == scenario.total_minimum_service_mwh;
    let total_shortfall_mwh = scenario.total_minimum_service_mwh - delivered;
    let reachable = residual_reachable(&graph, source);
    let mut minimum_cut_edges = Vec::new();
    let mut minimum_cut_capacity_mwh = 0_u64;
    for record in &records {
        if record.capacity_mwh > 0 && reachable[record.from] && !reachable[record.to] {
            minimum_cut_capacity_mwh = minimum_cut_capacity_mwh
                .checked_add(record.capacity_mwh)
                .ok_or_else(|| "minimum-cut capacity overflowed u64".to_string())?;
            minimum_cut_edges.push(MinCutEdge {
                kind: record.kind.to_string(),
                label: record.label.clone(),
                capacity_mwh: record.capacity_mwh,
            });
        }
    }
    minimum_cut_edges.sort_by_key(|edge| (edge.kind.clone(), edge.label.clone()));
    if minimum_cut_capacity_mwh != delivered {
        return Err(format!(
            "internal max-flow/min-cut invariant failed: flow {delivered} != cut {minimum_cut_capacity_mwh}"
        ));
    }

    Ok(ServiceOracleReport {
        scenario_id: scenario.scenario_id,
        interval_count: scenario.interval_count,
        receiver_count: scenario.receivers.len(),
        segment_slot_count: scenario.segment_slots.len(),
        transfer_link_count: scenario.transfer_links.len(),
        total_minimum_service_mwh: scenario.total_minimum_service_mwh,
        maximum_deliverable_mwh: delivered,
        total_shortfall_mwh,
        feasible,
        receiver_allocations,
        segment_allocations,
        transfer_allocations,
        receiver_slot_allocations,
        minimum_cut_capacity_mwh,
        minimum_cut_edges,
        claim_ceiling: "Synthetic exact feasibility oracle for supplied integer capacities only; no field accuracy, robust uncertainty, fairness optimum, safety, grid-code or energization claim.",
    })
}

pub fn bundled_fixture() -> Result<Value, String> {
    serde_json::from_str(include_str!(
        "../../../research/wpt-exact-service-oracle-v0/scenario.synthetic-example.json"
    ))
    .map_err(|error| format!("synthetic WPT service scenario JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        bundled_fixture().unwrap()
    }

    #[test]
    fn known_truth_fixture_satisfies_every_receiver_obligation() {
        let report = evaluate_scenario(&fixture()).unwrap();
        assert!(report.feasible);
        assert_eq!(report.total_minimum_service_mwh, 16);
        assert_eq!(report.maximum_deliverable_mwh, 16);
        assert_eq!(report.total_shortfall_mwh, 0);
        assert_eq!(report.minimum_cut_capacity_mwh, 16);
        assert!(
            report
                .receiver_allocations
                .iter()
                .all(|row| row.shortfall_mwh == 0)
        );
        assert_eq!(
            report
                .segment_allocations
                .iter()
                .map(|row| row.allocated_mwh)
                .sum::<u64>(),
            16
        );
        assert_eq!(
            report
                .transfer_allocations
                .iter()
                .map(|row| row.allocated_mwh)
                .sum::<u64>(),
            16
        );
        assert_eq!(
            report
                .receiver_slot_allocations
                .iter()
                .map(|row| row.allocated_mwh)
                .sum::<u64>(),
            16
        );
        assert!(
            report
                .transfer_allocations
                .iter()
                .all(|row| row.allocated_mwh <= row.transfer_capacity_mwh)
        );
    }

    #[test]
    fn shared_segment_energy_shortage_returns_exact_min_cut_certificate() {
        let mut scenario = fixture();
        scenario["segment_slots"][2]["available_deliverable_energy_mwh"] = Value::from(0);
        scenario["segment_slots"][3]["available_deliverable_energy_mwh"] = Value::from(0);
        let report = evaluate_scenario(&scenario).unwrap();
        assert!(!report.feasible);
        assert_eq!(report.maximum_deliverable_mwh, 8);
        assert_eq!(report.total_shortfall_mwh, 8);
        assert_eq!(report.minimum_cut_capacity_mwh, 8);
        assert!(!report.minimum_cut_edges.is_empty());
    }

    #[test]
    fn transfer_capacity_can_bind_before_segment_supply() {
        let mut scenario = fixture();
        for link in scenario["transfer_links"].as_array_mut().unwrap() {
            match link["link_id"].as_str() {
                Some("a0-to-v1") => {
                    link["max_deliverable_energy_mwh"] = Value::from(1);
                }
                Some("b0-to-v1") => {
                    link["max_deliverable_energy_mwh"] = Value::from(0);
                }
                _ => {}
            }
        }
        let report = evaluate_scenario(&scenario).unwrap();
        assert!(!report.feasible);
        assert_eq!(report.total_shortfall_mwh, 1);
        assert_eq!(
            report.minimum_cut_capacity_mwh,
            report.maximum_deliverable_mwh
        );
        assert!(
            report
                .minimum_cut_edges
                .iter()
                .any(|edge| edge.kind == "transfer_link")
        );
    }

    #[test]
    fn missing_segment_slot_is_rejected_instead_of_assumed_unlimited() {
        let mut scenario = fixture();
        scenario["transfer_links"][0]["segment_id"] = Value::String("unknown-segment".into());
        assert!(
            evaluate_scenario(&scenario)
                .unwrap_err()
                .contains("missing segment slot")
        );
    }

    #[test]
    fn late_supply_cannot_satisfy_an_earlier_departure() {
        let mut scenario = fixture();
        scenario["transfer_links"]
            .as_array_mut()
            .unwrap()
            .retain(|link| link["receiver_id"] != "vehicle-02");
        let report = evaluate_scenario(&scenario).unwrap();
        let receiver = report
            .receiver_allocations
            .iter()
            .find(|row| row.receiver_id == "vehicle-02")
            .unwrap();
        assert_eq!(receiver.delivered_mwh, 0);
        assert_eq!(receiver.shortfall_mwh, 4);
        assert!(!report.feasible);
    }

    #[test]
    fn duplicate_receiver_ids_and_unknown_properties_are_rejected() {
        let mut scenario = fixture();
        scenario["receivers"][1]["receiver_id"] = Value::String("vehicle-01".into());
        assert!(
            evaluate_scenario(&scenario)
                .unwrap_err()
                .contains("duplicate receiver_id")
        );

        let mut scenario = fixture();
        scenario["optimizer_shortcut"] = Value::Bool(true);
        assert!(
            evaluate_scenario(&scenario)
                .unwrap_err()
                .contains("unknown property")
        );
    }

    #[test]
    fn receiver_window_is_half_open_and_rejects_late_receiver_slot() {
        let mut scenario = fixture();
        scenario["receiver_slots"][1]["interval_index"] = Value::from(1);
        assert!(
            evaluate_scenario(&scenario)
                .unwrap_err()
                .contains("service window")
        );
    }

    #[test]
    fn zero_acceptance_and_zero_transfer_are_valid_hard_capacity_bounds() {
        let mut scenario = fixture();
        scenario["receiver_slots"][0]["max_acceptance_mwh"] = Value::from(0);
        let report = evaluate_scenario(&scenario).unwrap();
        assert!(!report.feasible);
        assert_eq!(
            report.minimum_cut_capacity_mwh,
            report.maximum_deliverable_mwh
        );
    }

    #[test]
    fn integer_energy_arithmetic_detects_total_demand_overflow() {
        let mut scenario = fixture();
        scenario["receivers"][0]["minimum_service_mwh"] = Value::from(u64::MAX);
        assert!(
            evaluate_scenario(&scenario)
                .unwrap_err()
                .contains("overflowed u64")
        );
    }
}
