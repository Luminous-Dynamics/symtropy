// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact small-instance feasibility oracle for interval-based WPT energy service.
//!
//! The oracle uses integer mWh capacities and a time-expanded max-flow network.
//! It answers whether every receiver's minimum energy obligation can be met
//! before its departure deadline, given already-qualified conservative
//! segment, transfer, and receiver-acceptance envelopes. It does not estimate
//! those envelopes, optimize fairness, or authorize physical operation.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use single_commodity_flow_kernel::{FlowEdge, FlowGraph, FlowNode, MAX_EDGES, MAX_NODES};

const MAX_INTERVALS: u64 = 256;
const MAX_SEGMENT_SLOTS: usize = 4096;
const MAX_RECEIVERS: usize = 128;
const MAX_INDIVIDUAL_MAXIMA: usize = 32;
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

    let modeled_node_count = 2_usize
        .checked_add(segment_slots.len())
        .and_then(|count| count.checked_add(receiver_slots.len()))
        .and_then(|count| count.checked_add(receivers.len()))
        .ok_or_else(|| fail("$", "modeled graph node count overflowed usize"))?;
    if modeled_node_count > MAX_NODES {
        return Err(fail(
            "$",
            format!(
                "modeled graph has {modeled_node_count} nodes, exceeding kernel limit {MAX_NODES}"
            ),
        ));
    }

    let modeled_edge_count = segment_slots.len()
        .checked_add(receiver_slots.len())
        .and_then(|count| count.checked_add(transfer_links.len()))
        .and_then(|count| count.checked_add(receivers.len()))
        .ok_or_else(|| fail("$", "modeled graph edge count overflowed usize"))?;
    if modeled_edge_count > MAX_EDGES {
        return Err(fail(
            "$",
            format!(
                "modeled graph has {modeled_edge_count} edges, exceeding kernel limit {MAX_EDGES}"
            ),
        ));
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndividualMaximaStatus {
    NotNeededJointServiceFeasible,
    ComputedForInfeasibleJointService,
    OmittedAboveDeclaredReceiverLimit,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReceiverAllocation {
    pub receiver_id: String,
    pub minimum_service_mwh: u64,
    /// This receiver's allocation in the one joint max-flow witness.
    /// It is not an individual maximum or a fairness guarantee.
    pub joint_witness_allocation_mwh: u64,
    pub joint_witness_shortfall_mwh: u64,
    /// Independent diagnostic; absent when not needed or above the bound.
    /// Independent maxima are not additive across receivers.
    pub individual_max_deliverable_mwh: Option<u64>,
    pub individual_max_min_cut_capacity_mwh: Option<u64>,
    pub individual_max_graph_sha256: Option<String>,
    pub individual_floor_feasible: Option<bool>,
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
    pub individual_maxima_status: IndividualMaximaStatus,
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
fn push_wpt_edge(
    edges: &mut Vec<FlowEdge>,
    edge_metadata: &mut BTreeMap<String, (String, String)>,
    id: String,
    from: String,
    to: String,
    capacity: u64,
    kind: &str,
    label: String,
) {
    let metadata_ref = format!("wpt-exact-service-oracle-v0/{kind}/{label}");
    edges.push(FlowEdge::new(
        id.clone(),
        from,
        to,
        capacity,
        Some(metadata_ref),
    ));
    edge_metadata.insert(id, (kind.to_string(), label));
}

fn required_wpt_flow(
    edge_flows: &BTreeMap<String, u64>,
    edge_id: &str,
) -> Result<u64, String> {
    edge_flows
        .get(edge_id)
        .copied()
        .ok_or_else(|| format!("flow kernel omitted required edge witness {edge_id}"))
}

/// Evaluate whether all minimum receiver-service obligations can be met.
///
/// Capacities are integer milliwatt-hours at one common *deliverable-energy*
/// boundary. The caller must supply conservative qualified bounds; this
/// adapter does not infer conversion losses or authenticate measurements.
/// The shared kernel proves only the supplied single-commodity directed graph.
pub fn evaluate_scenario(value: &Value) -> Result<ServiceOracleReport, String> {
    let scenario = parse_scenario(value)?;
    let mut nodes = vec![FlowNode::new("00-source")];
    let mut segment_node_ids = BTreeMap::new();
    let mut receiver_slot_node_ids = BTreeMap::new();
    let mut receiver_demand_node_ids = BTreeMap::new();

    for (index, key) in scenario.segment_slots.keys().enumerate() {
        let node_id = format!("10-segment-slot-{index:06}");
        nodes.push(FlowNode::new(node_id.clone()));
        segment_node_ids.insert(key.clone(), node_id);
    }
    for (index, key) in scenario.receiver_slots.keys().enumerate() {
        let node_id = format!("20-receiver-slot-{index:06}");
        nodes.push(FlowNode::new(node_id.clone()));
        receiver_slot_node_ids.insert(key.clone(), node_id);
    }
    for (index, receiver_id) in scenario.receivers.keys().enumerate() {
        let node_id = format!("30-receiver-demand-{index:06}");
        nodes.push(FlowNode::new(node_id.clone()));
        receiver_demand_node_ids.insert(receiver_id.clone(), node_id);
    }
    nodes.push(FlowNode::new("99-sink"));

    let mut edges = Vec::new();
    let mut edge_metadata = BTreeMap::new();
    let mut segment_edge_ids = BTreeMap::new();
    let mut transfer_edge_ids = BTreeMap::new();
    let mut receiver_slot_edge_ids = BTreeMap::new();
    let mut demand_edge_ids = BTreeMap::new();

    for (index, (key, slot)) in scenario.segment_slots.iter().enumerate() {
        let edge_id = format!("10-segment-supply-{index:06}");
        let node_id = segment_node_ids
            .get(key)
            .ok_or_else(|| "missing mapped WPT segment node".to_string())?
            .clone();
        push_wpt_edge(
            &mut edges,
            &mut edge_metadata,
            edge_id.clone(),
            "00-source".to_string(),
            node_id,
            slot.available_deliverable_energy_mwh,
            "segment_supply",
            format!("{}@{}", slot.segment_id, slot.interval_index),
        );
        segment_edge_ids.insert(key.clone(), edge_id);
    }

    for link in &scenario.transfer_links {
        let segment_key = (link.segment_id.clone(), link.interval_index);
        let receiver_key = (link.receiver_id.clone(), link.interval_index);
        let from = segment_node_ids
            .get(&segment_key)
            .ok_or_else(|| format!("missing mapped segment node for {}", link.link_id))?
            .clone();
        let to = receiver_slot_node_ids
            .get(&receiver_key)
            .ok_or_else(|| format!("missing mapped receiver node for {}", link.link_id))?
            .clone();
        let edge_id = format!("20-transfer-{}", link.link_id);
        push_wpt_edge(
            &mut edges,
            &mut edge_metadata,
            edge_id.clone(),
            from,
            to,
            link.max_deliverable_energy_mwh,
            "transfer_link",
            link.link_id.clone(),
        );
        transfer_edge_ids.insert(link.link_id.clone(), edge_id);
    }

    for (index, (key, slot)) in scenario.receiver_slots.iter().enumerate() {
        let edge_id = format!("30-receiver-acceptance-{index:06}");
        let from = receiver_slot_node_ids
            .get(key)
            .ok_or_else(|| "missing mapped WPT receiver-slot node".to_string())?
            .clone();
        let to = receiver_demand_node_ids
            .get(&slot.receiver_id)
            .ok_or_else(|| format!("missing mapped receiver demand node for {}", slot.receiver_id))?
            .clone();
        push_wpt_edge(
            &mut edges,
            &mut edge_metadata,
            edge_id.clone(),
            from,
            to,
            slot.max_acceptance_mwh,
            "receiver_acceptance",
            format!("{}@{}", slot.receiver_id, slot.interval_index),
        );
        receiver_slot_edge_ids.insert(key.clone(), edge_id);
    }

    for (index, (receiver_id, receiver)) in scenario.receivers.iter().enumerate() {
        let edge_id = format!("40-receiver-obligation-{index:06}");
        let from = receiver_demand_node_ids
            .get(receiver_id)
            .ok_or_else(|| format!("missing mapped receiver demand node for {receiver_id}"))?
            .clone();
        push_wpt_edge(
            &mut edges,
            &mut edge_metadata,
            edge_id.clone(),
            from,
            "99-sink".to_string(),
            receiver.minimum_service_mwh,
            "receiver_obligation",
            receiver_id.clone(),
        );
        demand_edge_ids.insert(receiver_id.clone(), edge_id);
    }

    let graph = FlowGraph {
        commodity_ref: "electrical-energy".to_string(),
        capacity_unit_ref: "mWh".to_string(),
        quantity_boundary_ref: "wpt-common-deliverable-energy-boundary-v0".to_string(),
        nodes,
        edges,
        source: "00-source".to_string(),
        sink: "99-sink".to_string(),
    };
    let flow_result = graph.solve().map_err(|error| error.to_string())?;
    let edge_flows = flow_result
        .edge_flows
        .iter()
        .map(|edge| (edge.id.clone(), edge.flow))
        .collect::<BTreeMap<_, _>>();

    let feasible = flow_result.max_flow == scenario.total_minimum_service_mwh;
    let individual_maxima_status = if feasible {
        IndividualMaximaStatus::NotNeededJointServiceFeasible
    } else if scenario.receivers.len() <= MAX_INDIVIDUAL_MAXIMA {
        IndividualMaximaStatus::ComputedForInfeasibleJointService
    } else {
        IndividualMaximaStatus::OmittedAboveDeclaredReceiverLimit
    };
    let individual_source_capacity_bound = if individual_maxima_status
        == IndividualMaximaStatus::ComputedForInfeasibleJointService
    {
        Some(scenario.segment_slots.values().try_fold(0_u64, |sum, slot| {
            sum.checked_add(slot.available_deliverable_energy_mwh)
                .ok_or_else(|| "individual WPT source-capacity bound overflowed u64".to_string())
        })?)
    } else {
        None
    };
    let obligation_edge_ids = demand_edge_ids.values()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut receiver_allocations = Vec::with_capacity(scenario.receivers.len());
    for (receiver_id, receiver) in &scenario.receivers {
        let edge_id = demand_edge_ids
            .get(receiver_id)
            .ok_or_else(|| format!("missing demand edge for {receiver_id}"))?;
        let joint_witness_allocation_mwh = required_wpt_flow(&edge_flows, edge_id)?;

        let individual_result = if individual_maxima_status
            == IndividualMaximaStatus::ComputedForInfeasibleJointService
        {
            // Remove other receiver sink obligations to compute this receiver's
            // isolated maximum. This diagnostic is not additive: shared capacity
            // may make the sum of these maxima exceed the joint maximum flow.
            let mut individual_graph = graph.clone();
            individual_graph.edges.retain(|edge| {
                !obligation_edge_ids.contains(edge.id.as_str())
                    || edge.id.as_str() == edge_id.as_str()
            });
            let target_edge = individual_graph.edges.iter_mut()
                .find(|edge| edge.id == *edge_id)
                .ok_or_else(|| format!("individual graph omitted receiver obligation edge for {receiver_id}"))?;
            target_edge.capacity = individual_source_capacity_bound
                .ok_or_else(|| "individual source-capacity bound was unexpectedly absent".to_string())?;
            Some(individual_graph.solve().map_err(|error| {
                format!("individual maximum for {receiver_id}: {error}")
            })?)
        } else {
            None
        };

        let individual_max_deliverable_mwh =
            individual_result.as_ref().map(|result| result.max_flow);
        let individual_floor_feasible = if feasible {
            Some(true)
        } else {
            individual_max_deliverable_mwh
                .map(|maximum| maximum >= receiver.minimum_service_mwh)
        };

        receiver_allocations.push(ReceiverAllocation {
            receiver_id: receiver_id.clone(),
            minimum_service_mwh: receiver.minimum_service_mwh,
            joint_witness_allocation_mwh,
            joint_witness_shortfall_mwh: receiver.minimum_service_mwh
                - joint_witness_allocation_mwh,
            individual_max_deliverable_mwh,
            individual_max_min_cut_capacity_mwh: individual_result
                .as_ref()
                .map(|result| result.min_cut.capacity),
            individual_max_graph_sha256: individual_result
                .as_ref()
                .map(|result| result.graph_sha256.clone()),
            individual_floor_feasible,
        });
    }

    let mut segment_allocations = Vec::with_capacity(scenario.segment_slots.len());
    for (key, slot) in &scenario.segment_slots {
        let edge_id = segment_edge_ids
            .get(key)
            .ok_or_else(|| format!("missing segment supply edge for {}@{}", slot.segment_id, slot.interval_index))?;
        segment_allocations.push(SegmentSlotAllocation {
            segment_id: slot.segment_id.clone(),
            interval_index: slot.interval_index,
            available_deliverable_energy_mwh: slot.available_deliverable_energy_mwh,
            allocated_mwh: required_wpt_flow(&edge_flows, edge_id)?,
        });
    }

    let mut transfer_allocations = Vec::with_capacity(scenario.transfer_links.len());
    for link in &scenario.transfer_links {
        let edge_id = transfer_edge_ids
            .get(&link.link_id)
            .ok_or_else(|| format!("missing transfer edge for {}", link.link_id))?;
        transfer_allocations.push(TransferAllocation {
            link_id: link.link_id.clone(),
            segment_id: link.segment_id.clone(),
            receiver_id: link.receiver_id.clone(),
            interval_index: link.interval_index,
            transfer_capacity_mwh: link.max_deliverable_energy_mwh,
            allocated_mwh: required_wpt_flow(&edge_flows, edge_id)?,
        });
    }

    let mut receiver_slot_allocations = Vec::with_capacity(scenario.receiver_slots.len());
    for (key, slot) in &scenario.receiver_slots {
        let edge_id = receiver_slot_edge_ids
            .get(key)
            .ok_or_else(|| format!("missing receiver acceptance edge for {}@{}", slot.receiver_id, slot.interval_index))?;
        receiver_slot_allocations.push(ReceiverSlotAllocation {
            receiver_id: slot.receiver_id.clone(),
            interval_index: slot.interval_index,
            max_acceptance_mwh: slot.max_acceptance_mwh,
            allocated_mwh: required_wpt_flow(&edge_flows, edge_id)?,
        });
    }

    let minimum_cut_edges = flow_result
        .min_cut
        .edges
        .iter()
        .filter(|edge| edge.capacity > 0)
        .map(|edge| {
            let (kind, label) = edge_metadata
                .get(&edge.id)
                .cloned()
                .ok_or_else(|| format!("flow kernel cut contains unknown edge {}", edge.id))?;
            Ok(MinCutEdge {
                kind,
                label,
                capacity_mwh: edge.capacity,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    minimum_cut_edges.sort_by_key(|edge| (edge.kind.clone(), edge.label.clone()));

    let total_shortfall_mwh = scenario.total_minimum_service_mwh - flow_result.max_flow;

    Ok(ServiceOracleReport {
        scenario_id: scenario.scenario_id,
        interval_count: scenario.interval_count,
        receiver_count: scenario.receivers.len(),
        segment_slot_count: scenario.segment_slots.len(),
        transfer_link_count: scenario.transfer_links.len(),
        total_minimum_service_mwh: scenario.total_minimum_service_mwh,
        maximum_deliverable_mwh: flow_result.max_flow,
        total_shortfall_mwh,
        feasible,
        individual_maxima_status,
        receiver_allocations,
        segment_allocations,
        transfer_allocations,
        receiver_slot_allocations,
        minimum_cut_capacity_mwh: flow_result.min_cut.capacity,
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
                .all(|row| row.joint_witness_shortfall_mwh == 0
                && row.individual_floor_feasible == Some(true)
                && row.individual_max_deliverable_mwh.is_none())
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
    fn independent_receiver_maxima_are_not_additive_under_shared_segment_capacity() {
        let scenario = serde_json::json!({
            "schema_version": "wpt-exact-service-oracle-v0",
            "scenario_id": "shared-segment-non-additive-receiver-maxima",
            "evidence_kind": "SYNTHETIC_KNOWN_TRUTH",
            "interval_count": 1,
            "segment_slots": [
                {"segment_id": "segment-a", "interval_index": 0, "available_deliverable_energy_mwh": 8}
            ],
            "receivers": [
                {"receiver_id": "vehicle-a", "arrival_interval": 0, "departure_interval": 1, "minimum_service_mwh": 5},
                {"receiver_id": "vehicle-b", "arrival_interval": 0, "departure_interval": 1, "minimum_service_mwh": 5}
            ],
            "receiver_slots": [
                {"receiver_id": "vehicle-a", "interval_index": 0, "max_acceptance_mwh": 5},
                {"receiver_id": "vehicle-b", "interval_index": 0, "max_acceptance_mwh": 5}
            ],
            "transfer_links": [
                {
                    "link_id": "segment-a-to-vehicle-a",
                    "segment_id": "segment-a",
                    "receiver_id": "vehicle-a",
                    "interval_index": 0,
                    "max_deliverable_energy_mwh": 5
                },
                {
                    "link_id": "segment-a-to-vehicle-b",
                    "segment_id": "segment-a",
                    "receiver_id": "vehicle-b",
                    "interval_index": 0,
                    "max_deliverable_energy_mwh": 5
                }
            ],
            "quality_flags": ["SYNTHETIC_ONLY"]
        });

        let report = evaluate_scenario(&scenario).unwrap();
        assert!(!report.feasible);
        assert_eq!(report.total_minimum_service_mwh, 10);
        assert_eq!(report.maximum_deliverable_mwh, 8);
        assert_eq!(
            report.receiver_allocations.iter()
                .map(|row| row.joint_witness_allocation_mwh)
                .sum::<u64>(),
            8
        );
        assert_eq!(
            report.individual_maxima_status,
            IndividualMaximaStatus::ComputedForInfeasibleJointService
        );
        assert!(report.receiver_allocations.iter()
            .all(|row| row.individual_max_deliverable_mwh == Some(5)));
        assert_eq!(
            report.receiver_allocations.iter()
                .map(|row| row.individual_max_deliverable_mwh)
                .sum::<u64>(),
            10
        );
        assert!(report.receiver_allocations.iter()
            .all(|row| row.individual_max_graph_sha256.as_deref()
                .is_some_and(|digest| digest.starts_with("sha256:"))));
    }

    #[test]
    fn individual_receiver_maximum_can_exceed_its_service_floor() {
        let mut scenario = fixture();
        scenario["receivers"][0]["minimum_service_mwh"] = Value::from(8);
        scenario["receivers"][1]["minimum_service_mwh"] = Value::from(5);
        let report = evaluate_scenario(&scenario).unwrap();
        assert!(!report.feasible);
        let vehicle_one = report.receiver_allocations.iter()
            .find(|row| row.receiver_id == "vehicle-01")
            .unwrap();
        assert_eq!(vehicle_one.minimum_service_mwh, 8);
        assert_eq!(vehicle_one.individual_max_deliverable_mwh, Some(9));
        assert_eq!(vehicle_one.individual_floor_feasible, Some(true));
        assert!(vehicle_one.joint_witness_allocation_mwh <= 8);
    }

    #[test]
    fn individual_diagnostics_are_omitted_above_the_declared_receiver_bound() {
        let receivers = (0..33).map(|index| serde_json::json!({
            "receiver_id": format!("receiver-{index:02}"),
            "arrival_interval": 0,
            "departure_interval": 1,
            "minimum_service_mwh": 1
        })).collect::<Vec<_>>();
        let receiver_slots = (0..33).map(|index| serde_json::json!({
            "receiver_id": format!("receiver-{index:02}"),
            "interval_index": 0,
            "max_acceptance_mwh": 1
        })).collect::<Vec<_>>();
        let transfer_links = (0..33).map(|index| serde_json::json!({
            "link_id": format!("segment-to-receiver-{index:02}"),
            "segment_id": "segment-a",
            "receiver_id": format!("receiver-{index:02}"),
            "interval_index": 0,
            "max_deliverable_energy_mwh": 1
        })).collect::<Vec<_>>();
        let scenario = serde_json::json!({
            "schema_version": "wpt-exact-service-oracle-v0",
            "scenario_id": "receiver-individual-maxima-bound",
            "evidence_kind": "SYNTHETIC_KNOWN_TRUTH",
            "interval_count": 1,
            "segment_slots": [{
                "segment_id": "segment-a",
                "interval_index": 0,
                "available_deliverable_energy_mwh": 1
            }],
            "receivers": receivers,
            "receiver_slots": receiver_slots,
            "transfer_links": transfer_links,
            "quality_flags": ["SYNTHETIC_ONLY"]
        });

        let report = evaluate_scenario(&scenario).unwrap();
        assert!(!report.feasible);
        assert_eq!(
            report.individual_maxima_status,
            IndividualMaximaStatus::OmittedAboveDeclaredReceiverLimit
        );
        assert!(report.receiver_allocations.iter().all(|row| {
            row.individual_max_deliverable_mwh.is_none()
                && row.individual_max_min_cut_capacity_mwh.is_none()
                && row.individual_max_graph_sha256.is_none()
                && row.individual_floor_feasible.is_none()
        }));
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
        assert_eq!(receiver.joint_witness_allocation_mwh, 0);
        assert_eq!(receiver.joint_witness_shortfall_mwh, 4);
        assert_eq!(receiver.individual_max_deliverable_mwh, Some(0));
        assert_eq!(receiver.individual_floor_feasible, Some(false));
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
    fn parser_rejects_a_scenario_larger_than_the_shared_kernel_node_limit() {
        let mut scenario = fixture();
        let segment_slots = scenario["segment_slots"].as_array_mut().unwrap();
        for index in 0..(MAX_SEGMENT_SLOTS - segment_slots.len()) {
            segment_slots.push(serde_json::json!({
                "segment_id": format!("extra-segment-{index:04}"),
                "interval_index": 0,
                "available_deliverable_energy_mwh": 1
            }));
        }
        let error = evaluate_scenario(&scenario).unwrap_err();
        assert!(error.contains("exceeding kernel limit"));
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
