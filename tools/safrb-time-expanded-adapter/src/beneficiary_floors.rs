#![forbid(unsafe_code)]

//! Joint beneficiary-floor feasibility for the temporal single-commodity model.
//!
//! This module proves whether all declared service floors can be satisfied
//! simultaneously. It does not optimize priority tiers or operational cost.
//! Mandatory floors are represented as lower bounds on the service edges and
//! reduced to a circulation with super-source/super-sink arcs.
//!
//! The producer exports the exact base graph, exact lower-bound reduction and
//! max-flow/min-cut certificate. The verifier reconstructs the reduction
//! separately, verifies the transformed certificate, and checks the recovered
//! original-network allocation and every beneficiary floor.

use crate::temporal_profile::{TEMPORAL_PROFILE_SCHEMA, TemporalProfile, compile_temporal_profile};
use crate::temporal_verify::verify_temporal_certificate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use single_commodity_flow_kernel::{
    EdgeFlow, FlowCertificateBundle, FlowEdge, FlowGraph, FlowNode, MAX_EDGES, MAX_NODES,
};
use std::collections::{BTreeMap, BTreeSet};

pub const FLOOR_FEASIBILITY_SCHEMA: &str = "safrb-beneficiary-floor-feasibility-v0.1";

const AUX_SOURCE: &str = "__safrb-floor-feasibility/super-source-v0.1";
const AUX_SINK: &str = "__safrb-floor-feasibility/super-sink-v0.1";
const RETURN_EDGE: &str = "__safrb-floor-feasibility/return-v0.1";
const RETURN_METADATA: &str = "safrb-beneficiary-floor-feasibility-v0.1/return";
const SUPER_SOURCE_PREFIX: &str = "__safrb-floor-feasibility/ss/";
const SUPER_SINK_PREFIX: &str = "__safrb-floor-feasibility/tt/";

pub const FLOOR_CLAIM_CEILING: &str = "Synthetic joint feasibility of the declared beneficiary minimum floors in the supplied temporal single-commodity graph only; no priority optimization, source authentication, legal authority, physical qualification, dispatch authorization or actual service claim.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeneficiaryFloor {
    pub service_id: String,
    pub minimum_required_quantity: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeneficiaryFloorInput {
    pub profile: TemporalProfile,
    pub floors: Vec<BeneficiaryFloor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeneficiaryFloorAllocation {
    pub service_id: String,
    pub requested_quantity: u64,
    pub minimum_required_quantity: u64,
    pub allocated_quantity: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeneficiaryFloorReport {
    pub schema_version: String,
    pub case_id: String,
    /// Digest of the canonical original temporal graph.
    pub base_graph_sha256: String,
    /// Sorted by service_id.
    pub obligations: Vec<BeneficiaryFloor>,
    pub required_auxiliary_flow: u64,
    pub achieved_auxiliary_flow: u64,
    pub floors_jointly_feasible: bool,
    /// Exact original temporal graph used by the lower-bound reduction.
    pub base_graph: FlowGraph,
    /// Exact circulation-reduction graph and max-flow/min-cut certificate.
    pub feasibility_certificate: FlowCertificateBundle,
    /// Present only when every mandatory floor is jointly feasible.
    pub original_edge_flows: Option<Vec<EdgeFlow>>,
    /// Present only when every mandatory floor is jointly feasible.
    pub beneficiary_allocations: Option<Vec<BeneficiaryFloorAllocation>>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone)]
struct Reduction {
    graph: FlowGraph,
    required_flow: u64,
    lower_by_edge: BTreeMap<String, u64>,
}

fn service_edge_id(service_id: &str) -> String {
    format!("service/{}/{service_id}", service_id.len())
}

fn require_nonempty(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    Ok(())
}

fn canonical_graph_digest(graph: &FlowGraph) -> Result<String, String> {
    let bytes = graph
        .canonical_bytes()
        .map_err(|error| format!("invalid base graph: {error}"))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn validate_floor_inputs(
    profile: &TemporalProfile,
    base_graph: &FlowGraph,
    floors: &[BeneficiaryFloor],
) -> Result<(Vec<BeneficiaryFloor>, BTreeMap<String, u64>), String> {
    if profile.schema_version != TEMPORAL_PROFILE_SCHEMA {
        return Err("unsupported temporal profile schema for beneficiary floors".to_string());
    }
    require_nonempty(&profile.case_id, "case_id")?;
    if floors.is_empty() {
        return Err("at least one mandatory beneficiary floor is required".to_string());
    }

    let services = profile
        .services
        .iter()
        .map(|service| (service.service_id.as_str(), service))
        .collect::<BTreeMap<_, _>>();
    let graph_edges = base_graph
        .edges
        .iter()
        .map(|edge| (edge.id.as_str(), edge))
        .collect::<BTreeMap<_, _>>();

    let mut seen = BTreeSet::new();
    let mut normalized = floors.to_vec();
    normalized.sort_by(|left, right| left.service_id.cmp(&right.service_id));
    let mut lower_by_edge = BTreeMap::new();

    for floor in &normalized {
        require_nonempty(&floor.service_id, "service_id")?;
        if !seen.insert(floor.service_id.as_str()) {
            return Err(format!(
                "duplicate beneficiary floor for service {}",
                floor.service_id
            ));
        }
        let service = services
            .get(floor.service_id.as_str())
            .ok_or_else(|| format!("floor references unknown service {}", floor.service_id))?;
        if floor.minimum_required_quantity > service.requested_quantity {
            return Err(format!(
                "mandatory floor for service {} exceeds its requested quantity",
                floor.service_id
            ));
        }

        let edge_id = service_edge_id(&floor.service_id);
        let edge = graph_edges
            .get(edge_id.as_str())
            .ok_or_else(|| format!("base temporal graph omitted service edge {edge_id}"))?;
        if edge.capacity != service.requested_quantity {
            return Err(format!(
                "base graph service ceiling does not match profile for {}",
                floor.service_id
            ));
        }
        lower_by_edge.insert(edge_id, floor.minimum_required_quantity);
    }

    Ok((normalized, lower_by_edge))
}

fn add_balance(
    balances: &mut BTreeMap<String, i128>,
    node_id: &str,
    delta: i128,
) -> Result<(), String> {
    let current = balances
        .get(node_id)
        .copied()
        .ok_or_else(|| format!("lower-bound reduction references unknown node {node_id}"))?;
    let next = current
        .checked_add(delta)
        .ok_or_else(|| "lower-bound node balance overflowed i128".to_string())?;
    balances.insert(node_id.to_string(), next);
    Ok(())
}

fn build_reduction(
    base_graph: &FlowGraph,
    lower_by_edge: &BTreeMap<String, u64>,
) -> Result<Reduction, String> {
    base_graph
        .canonical_bytes()
        .map_err(|error| format!("invalid temporal base graph: {error}"))?;

    if base_graph
        .nodes
        .iter()
        .any(|node| node.id == AUX_SOURCE || node.id == AUX_SINK)
    {
        return Err("base graph collides with reserved lower-bound reduction node".to_string());
    }
    if base_graph.edges.iter().any(|edge| edge.id == RETURN_EDGE) {
        return Err("base graph collides with reserved lower-bound return edge".to_string());
    }

    let graph_node_count = base_graph
        .nodes
        .len()
        .checked_add(2)
        .ok_or_else(|| "lower-bound reduction node count overflowed usize".to_string())?;
    let graph_edge_ceiling = base_graph
        .edges
        .len()
        .checked_add(base_graph.nodes.len())
        .and_then(|count| count.checked_add(1))
        .ok_or_else(|| "lower-bound reduction edge count overflowed usize".to_string())?;
    if graph_node_count > MAX_NODES || graph_edge_ceiling > MAX_EDGES {
        return Err("lower-bound feasibility reduction exceeds kernel graph limits".to_string());
    }

    let mut balances = base_graph
        .nodes
        .iter()
        .map(|node| (node.id.clone(), 0_i128))
        .collect::<BTreeMap<_, _>>();
    let mut residual_edges = Vec::with_capacity(graph_edge_ceiling);
    let mut source_capacity_bound = 0_u64;

    for edge in &base_graph.edges {
        let lower = lower_by_edge.get(&edge.id).copied().unwrap_or(0);
        if lower > edge.capacity {
            return Err(format!(
                "lower bound exceeds upper capacity for edge {}",
                edge.id
            ));
        }
        add_balance(&mut balances, &edge.from, -(i128::from(lower)))?;
        add_balance(&mut balances, &edge.to, i128::from(lower))?;

        residual_edges.push(FlowEdge::new(
            edge.id.clone(),
            edge.from.clone(),
            edge.to.clone(),
            edge.capacity - lower,
            edge.metadata_ref.clone(),
        ));

        if edge.from == base_graph.source {
            source_capacity_bound = source_capacity_bound
                .checked_add(edge.capacity)
                .ok_or_else(|| "source capacity bound overflowed u64".to_string())?;
        }
    }

    residual_edges.push(FlowEdge::new(
        RETURN_EDGE,
        base_graph.sink.clone(),
        base_graph.source.clone(),
        source_capacity_bound,
        Some(RETURN_METADATA.to_string()),
    ));

    let mut nodes = base_graph
        .nodes
        .iter()
        .map(|node| FlowNode::new(node.id.clone()))
        .collect::<Vec<_>>();
    nodes.push(FlowNode::new(AUX_SOURCE));
    nodes.push(FlowNode::new(AUX_SINK));

    let mut required_flow = 0_u64;
    let mut positive_balance = 0_i128;
    let mut negative_balance = 0_i128;
    for (node_id, balance) in &balances {
        if *balance > 0 {
            let capacity = u64::try_from(*balance)
                .map_err(|_| format!("positive lower-bound balance exceeds u64 at {node_id}"))?;
            positive_balance = positive_balance
                .checked_add(*balance)
                .ok_or_else(|| "positive lower-bound total overflowed i128".to_string())?;
            required_flow = required_flow.checked_add(capacity).ok_or_else(|| {
                "required lower-bound feasibility flow overflowed u64".to_string()
            })?;
            residual_edges.push(FlowEdge::new(
                format!("{SUPER_SOURCE_PREFIX}{}{node_id}", node_id.len()),
                AUX_SOURCE,
                node_id.clone(),
                capacity,
                Some(format!("{FLOOR_FEASIBILITY_SCHEMA}/super-source/{node_id}")),
            ));
        } else if *balance < 0 {
            let deficit = balance
                .checked_neg()
                .ok_or_else(|| "negative lower-bound balance overflowed i128".to_string())?;
            let capacity = u64::try_from(deficit)
                .map_err(|_| format!("negative lower-bound balance exceeds u64 at {node_id}"))?;
            negative_balance = negative_balance
                .checked_add(deficit)
                .ok_or_else(|| "negative lower-bound total overflowed i128".to_string())?;
            residual_edges.push(FlowEdge::new(
                format!("{SUPER_SINK_PREFIX}{}{node_id}", node_id.len()),
                node_id.clone(),
                AUX_SINK,
                capacity,
                Some(format!("{FLOOR_FEASIBILITY_SCHEMA}/super-sink/{node_id}")),
            ));
        }
    }

    if positive_balance != negative_balance {
        return Err("lower-bound node balances do not sum to zero".to_string());
    }

    let graph = FlowGraph {
        commodity_ref: base_graph.commodity_ref.clone(),
        capacity_unit_ref: base_graph.capacity_unit_ref.clone(),
        quantity_boundary_ref: format!(
            "{}::{FLOOR_FEASIBILITY_SCHEMA}",
            base_graph.quantity_boundary_ref
        ),
        nodes,
        edges: residual_edges,
        source: AUX_SOURCE.to_string(),
        sink: AUX_SINK.to_string(),
    };
    graph
        .canonical_bytes()
        .map_err(|error| format!("invalid lower-bound reduction graph: {error}"))?;

    Ok(Reduction {
        graph,
        required_flow,
        lower_by_edge: lower_by_edge.clone(),
    })
}

fn reconstruct_expected_reduction(
    base_graph: &FlowGraph,
    lower_by_edge: &BTreeMap<String, u64>,
) -> Result<(FlowGraph, u64), String> {
    // Independent reconstruction used by the verifier. It deliberately
    // rebuilds the auxiliary graph from the base graph and lower-bound table
    // rather than trusting the producer's serialized reduction graph.
    base_graph
        .canonical_bytes()
        .map_err(|error| format!("verifier rejected invalid base graph: {error}"))?;

    let expected_node_count = base_graph
        .nodes
        .len()
        .checked_add(2)
        .ok_or_else(|| "verifier lower-bound node count overflowed usize".to_string())?;
    let expected_edge_ceiling = base_graph
        .edges
        .len()
        .checked_add(base_graph.nodes.len())
        .and_then(|count| count.checked_add(1))
        .ok_or_else(|| "verifier lower-bound edge count overflowed usize".to_string())?;
    if expected_node_count > MAX_NODES || expected_edge_ceiling > MAX_EDGES {
        return Err("verifier lower-bound reduction exceeds kernel graph limits".to_string());
    }
    if base_graph
        .nodes
        .iter()
        .any(|node| node.id == AUX_SOURCE || node.id == AUX_SINK)
    {
        return Err("verifier detected reserved node identity collision".to_string());
    }
    if base_graph.edges.iter().any(|edge| edge.id == RETURN_EDGE) {
        return Err("verifier detected reserved return edge collision".to_string());
    }

    let mut node_balance = BTreeMap::new();
    for node in &base_graph.nodes {
        node_balance.insert(node.id.clone(), 0_i128);
    }

    let mut edges = Vec::with_capacity(expected_edge_ceiling);
    let mut return_capacity = 0_u64;
    for edge in &base_graph.edges {
        let lower = lower_by_edge.get(&edge.id).copied().unwrap_or(0);
        if lower > edge.capacity {
            return Err(format!(
                "verifier found lower bound above capacity on {}",
                edge.id
            ));
        }
        let from_balance = node_balance
            .get(&edge.from)
            .copied()
            .ok_or_else(|| "verifier found unknown edge tail".to_string())?;
        let to_balance = node_balance
            .get(&edge.to)
            .copied()
            .ok_or_else(|| "verifier found unknown edge head".to_string())?;
        node_balance.insert(
            edge.from.clone(),
            from_balance
                .checked_sub(i128::from(lower))
                .ok_or_else(|| "verifier lower-bound tail balance overflowed".to_string())?,
        );
        node_balance.insert(
            edge.to.clone(),
            to_balance
                .checked_add(i128::from(lower))
                .ok_or_else(|| "verifier lower-bound head balance overflowed".to_string())?,
        );

        edges.push(FlowEdge {
            id: edge.id.clone(),
            from: edge.from.clone(),
            to: edge.to.clone(),
            capacity: edge.capacity - lower,
            metadata_ref: edge.metadata_ref.clone(),
        });
        if edge.from == base_graph.source {
            return_capacity = return_capacity
                .checked_add(edge.capacity)
                .ok_or_else(|| "verifier source capacity bound overflowed".to_string())?;
        }
    }

    edges.push(FlowEdge {
        id: RETURN_EDGE.to_string(),
        from: base_graph.sink.clone(),
        to: base_graph.source.clone(),
        capacity: return_capacity,
        metadata_ref: Some(RETURN_METADATA.to_string()),
    });

    let mut nodes = base_graph
        .nodes
        .iter()
        .map(|node| FlowNode::new(node.id.clone()))
        .collect::<Vec<_>>();
    nodes.push(FlowNode::new(AUX_SOURCE));
    nodes.push(FlowNode::new(AUX_SINK));

    let mut demand = 0_u64;
    let mut positive_sum = 0_i128;
    let mut negative_sum = 0_i128;
    for (node_id, balance) in node_balance {
        match balance.cmp(&0) {
            std::cmp::Ordering::Greater => {
                let amount = u64::try_from(balance)
                    .map_err(|_| "verifier positive balance exceeds u64".to_string())?;
                positive_sum = positive_sum
                    .checked_add(balance)
                    .ok_or_else(|| "verifier positive balance sum overflowed".to_string())?;
                demand = demand
                    .checked_add(amount)
                    .ok_or_else(|| "verifier required flow overflowed".to_string())?;
                edges.push(FlowEdge {
                    id: format!("{SUPER_SOURCE_PREFIX}{}{node_id}", node_id.len()),
                    from: AUX_SOURCE.to_string(),
                    to: node_id.clone(),
                    capacity: amount,
                    metadata_ref: Some(format!(
                        "{FLOOR_FEASIBILITY_SCHEMA}/super-source/{node_id}"
                    )),
                });
            }
            std::cmp::Ordering::Less => {
                let amount_i128 = balance
                    .checked_neg()
                    .ok_or_else(|| "verifier negative balance overflowed".to_string())?;
                let amount = u64::try_from(amount_i128)
                    .map_err(|_| "verifier negative balance exceeds u64".to_string())?;
                negative_sum = negative_sum
                    .checked_add(amount_i128)
                    .ok_or_else(|| "verifier negative balance sum overflowed".to_string())?;
                edges.push(FlowEdge {
                    id: format!("{SUPER_SINK_PREFIX}{}{node_id}", node_id.len()),
                    from: node_id.clone(),
                    to: AUX_SINK.to_string(),
                    capacity: amount,
                    metadata_ref: Some(format!("{FLOOR_FEASIBILITY_SCHEMA}/super-sink/{node_id}")),
                });
            }
            std::cmp::Ordering::Equal => {}
        }
    }
    if positive_sum != negative_sum {
        return Err("verifier lower-bound balances do not reconcile".to_string());
    }

    let graph = FlowGraph {
        commodity_ref: base_graph.commodity_ref.clone(),
        capacity_unit_ref: base_graph.capacity_unit_ref.clone(),
        quantity_boundary_ref: format!(
            "{}::{FLOOR_FEASIBILITY_SCHEMA}",
            base_graph.quantity_boundary_ref
        ),
        nodes,
        edges,
        source: AUX_SOURCE.to_string(),
        sink: AUX_SINK.to_string(),
    };
    graph
        .canonical_bytes()
        .map_err(|error| format!("verifier reconstructed invalid reduction: {error}"))?;

    Ok((graph, demand))
}

fn reconstruct_original_flows(
    base_graph: &FlowGraph,
    lower_by_edge: &BTreeMap<String, u64>,
    auxiliary_bundle: &FlowCertificateBundle,
) -> Result<Vec<EdgeFlow>, String> {
    let residual_flow = auxiliary_bundle
        .result
        .edge_flows
        .iter()
        .map(|edge| (edge.id.as_str(), edge.flow))
        .collect::<BTreeMap<_, _>>();

    let mut result = Vec::with_capacity(base_graph.edges.len());
    for edge in &base_graph.edges {
        let lower = lower_by_edge.get(&edge.id).copied().unwrap_or(0);
        let residual = residual_flow
            .get(edge.id.as_str())
            .copied()
            .ok_or_else(|| format!("feasibility certificate omitted residual edge {}", edge.id))?;
        let flow = lower
            .checked_add(residual)
            .ok_or_else(|| format!("recovered original flow overflowed for {}", edge.id))?;
        if flow > edge.capacity {
            return Err(format!(
                "recovered original flow exceeds capacity on {}",
                edge.id
            ));
        }
        result.push(EdgeFlow {
            id: edge.id.clone(),
            from: edge.from.clone(),
            to: edge.to.clone(),
            capacity: edge.capacity,
            flow,
            metadata_ref: edge.metadata_ref.clone(),
        });
    }
    result.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(result)
}

fn verify_original_allocation(
    profile: &TemporalProfile,
    base_graph: &FlowGraph,
    floors: &[BeneficiaryFloor],
    flows: &[EdgeFlow],
) -> Result<Vec<BeneficiaryFloorAllocation>, String> {
    let flow_by_id = flows
        .iter()
        .map(|edge| (edge.id.as_str(), edge))
        .collect::<BTreeMap<_, _>>();
    if flow_by_id.len() != base_graph.edges.len() || flows.len() != base_graph.edges.len() {
        return Err(
            "original allocation does not cover every base graph edge exactly once".to_string(),
        );
    }

    let graph_edges = base_graph
        .edges
        .iter()
        .map(|edge| (edge.id.as_str(), edge))
        .collect::<BTreeMap<_, _>>();
    let mut node_balance = base_graph
        .nodes
        .iter()
        .map(|node| (node.id.clone(), 0_i128))
        .collect::<BTreeMap<_, _>>();

    for edge_flow in flows {
        let graph_edge = graph_edges
            .get(edge_flow.id.as_str())
            .ok_or_else(|| format!("allocation contains unknown edge {}", edge_flow.id))?;
        if edge_flow.from != graph_edge.from
            || edge_flow.to != graph_edge.to
            || edge_flow.capacity != graph_edge.capacity
            || edge_flow.metadata_ref != graph_edge.metadata_ref
            || edge_flow.flow > edge_flow.capacity
        {
            return Err(format!(
                "allocation edge {} does not match base graph",
                edge_flow.id
            ));
        }

        let from = node_balance
            .get(&edge_flow.from)
            .copied()
            .ok_or_else(|| format!("allocation references unknown node {}", edge_flow.from))?;
        let to = node_balance
            .get(&edge_flow.to)
            .copied()
            .ok_or_else(|| format!("allocation references unknown node {}", edge_flow.to))?;
        node_balance.insert(
            edge_flow.from.clone(),
            from.checked_add(i128::from(edge_flow.flow))
                .ok_or_else(|| "original flow source balance overflowed".to_string())?,
        );
        node_balance.insert(
            edge_flow.to.clone(),
            to.checked_sub(i128::from(edge_flow.flow))
                .ok_or_else(|| "original flow destination balance overflowed".to_string())?,
        );
    }

    let source_balance = node_balance
        .get(&base_graph.source)
        .copied()
        .ok_or_else(|| "base graph source missing from allocation".to_string())?;
    let sink_balance = node_balance
        .get(&base_graph.sink)
        .copied()
        .ok_or_else(|| "base graph sink missing from allocation".to_string())?;
    if source_balance < 0 || sink_balance != -source_balance {
        return Err("original allocation source/sink balances do not reconcile".to_string());
    }
    for (node_id, balance) in &node_balance {
        if node_id != &base_graph.source && node_id != &base_graph.sink && *balance != 0 {
            return Err(format!(
                "original allocation violates conservation at {node_id}"
            ));
        }
    }

    let services = profile
        .services
        .iter()
        .map(|service| (service.service_id.as_str(), service))
        .collect::<BTreeMap<_, _>>();
    let mut allocations = Vec::with_capacity(floors.len());
    for floor in floors {
        let service = services.get(floor.service_id.as_str()).ok_or_else(|| {
            format!(
                "allocation floor references unknown service {}",
                floor.service_id
            )
        })?;
        let edge_id = service_edge_id(&floor.service_id);
        let amount = flow_by_id
            .get(edge_id.as_str())
            .ok_or_else(|| format!("allocation omitted service edge {edge_id}"))?
            .flow;
        if amount < floor.minimum_required_quantity || amount > service.requested_quantity {
            return Err(format!(
                "allocation does not satisfy the mandatory floor for {}",
                floor.service_id
            ));
        }
        allocations.push(BeneficiaryFloorAllocation {
            service_id: floor.service_id.clone(),
            requested_quantity: service.requested_quantity,
            minimum_required_quantity: floor.minimum_required_quantity,
            allocated_quantity: amount,
        });
    }
    allocations.sort_by(|left, right| left.service_id.cmp(&right.service_id));
    Ok(allocations)
}

/// Prove joint feasibility of all beneficiary floors using an exact lower-bound
/// circulation reduction. This proves feasibility only; it does not optimize
/// priority tiers or operational cost.
pub fn evaluate_beneficiary_floors(
    profile: &TemporalProfile,
    floors: &[BeneficiaryFloor],
) -> Result<BeneficiaryFloorReport, String> {
    let base_graph = compile_temporal_profile(profile)?;
    let (obligations, lower_by_edge) = validate_floor_inputs(profile, &base_graph, floors)?;
    let reduction = build_reduction(&base_graph, &lower_by_edge)?;
    let result = reduction
        .graph
        .solve()
        .map_err(|error| format!("lower-bound feasibility solve failed: {error}"))?;
    let feasibility_certificate = FlowCertificateBundle::new(reduction.graph, result)
        .map_err(|error| format!("lower-bound feasibility certificate failed: {error}"))?;
    let achieved = feasibility_certificate.result.max_flow;
    let feasible = achieved == reduction.required_flow;

    let (original_edge_flows, beneficiary_allocations) = if feasible {
        let flows = reconstruct_original_flows(
            &base_graph,
            &reduction.lower_by_edge,
            &feasibility_certificate,
        )?;
        let allocations = verify_original_allocation(profile, &base_graph, &obligations, &flows)?;
        (Some(flows), Some(allocations))
    } else {
        (None, None)
    };

    let report = BeneficiaryFloorReport {
        schema_version: FLOOR_FEASIBILITY_SCHEMA.to_string(),
        case_id: profile.case_id.clone(),
        base_graph_sha256: canonical_graph_digest(&base_graph)?,
        obligations,
        required_auxiliary_flow: reduction.required_flow,
        achieved_auxiliary_flow: achieved,
        floors_jointly_feasible: feasible,
        base_graph,
        feasibility_certificate,
        original_edge_flows,
        beneficiary_allocations,
        claim_ceiling: FLOOR_CLAIM_CEILING.to_string(),
    };
    verify_beneficiary_floor_report(profile, floors, &report)?;
    Ok(report)
}

/// Evaluate the portable profile-plus-floors request envelope.
pub fn evaluate_beneficiary_floor_input(
    input: &BeneficiaryFloorInput,
) -> Result<BeneficiaryFloorReport, String> {
    evaluate_beneficiary_floors(&input.profile, &input.floors)
}

/// Independently verify a floor-feasibility report. This function does not call
/// the producer's temporal compiler or lower-bound reduction builder.
pub fn verify_beneficiary_floor_report(
    profile: &TemporalProfile,
    floors: &[BeneficiaryFloor],
    report: &BeneficiaryFloorReport,
) -> Result<(), String> {
    if report.schema_version != FLOOR_FEASIBILITY_SCHEMA {
        return Err("unsupported beneficiary-floor report schema".to_string());
    }
    if report.claim_ceiling != FLOOR_CLAIM_CEILING {
        return Err("beneficiary-floor claim ceiling mismatch".to_string());
    }
    if report.case_id != profile.case_id {
        return Err("beneficiary-floor report case identity mismatch".to_string());
    }

    let (obligations, lower_by_edge) = validate_floor_inputs(profile, &report.base_graph, floors)?;
    if report.obligations != obligations {
        return Err("beneficiary-floor obligations do not match verifier inputs".to_string());
    }
    if report.base_graph_sha256 != canonical_graph_digest(&report.base_graph)? {
        return Err("base temporal graph digest mismatch".to_string());
    }

    let base_result = report
        .base_graph
        .solve()
        .map_err(|error| format!("base graph verification solve failed: {error}"))?;
    let base_bundle = FlowCertificateBundle::new(report.base_graph.clone(), base_result)
        .map_err(|error| format!("base graph verification bundle failed: {error}"))?;
    verify_temporal_certificate(profile, &base_bundle)
        .map_err(|error| format!("base temporal graph failed independent verification: {error}"))?;

    let (expected_auxiliary_graph, expected_required_flow) =
        reconstruct_expected_reduction(&report.base_graph, &lower_by_edge)?;
    let expected_bytes = expected_auxiliary_graph
        .canonical_bytes()
        .map_err(|error| format!("expected feasibility graph is invalid: {error}"))?;
    let actual_bytes = report
        .feasibility_certificate
        .graph
        .canonical_bytes()
        .map_err(|error| format!("reported feasibility graph is invalid: {error}"))?;
    if expected_bytes != actual_bytes {
        return Err(
            "feasibility certificate graph does not match reconstructed lower-bound reduction"
                .to_string(),
        );
    }

    report
        .feasibility_certificate
        .verify()
        .map_err(|error| format!("lower-bound feasibility certificate invalid: {error}"))?;
    if report.required_auxiliary_flow != expected_required_flow
        || report.achieved_auxiliary_flow != report.feasibility_certificate.result.max_flow
    {
        return Err("beneficiary-floor auxiliary flow totals do not match certificate".to_string());
    }
    let expected_feasible = expected_required_flow == report.achieved_auxiliary_flow;
    if report.floors_jointly_feasible != expected_feasible {
        return Err(
            "beneficiary-floor disposition does not match auxiliary certificate".to_string(),
        );
    }

    if expected_feasible {
        let expected_flows = reconstruct_original_flows(
            &report.base_graph,
            &lower_by_edge,
            &report.feasibility_certificate,
        )?;
        let expected_allocations =
            verify_original_allocation(profile, &report.base_graph, &obligations, &expected_flows)?;
        if report.original_edge_flows.as_ref() != Some(&expected_flows)
            || report.beneficiary_allocations.as_ref() != Some(&expected_allocations)
        {
            return Err(
                "beneficiary-floor allocation does not match reconstructed feasible flow"
                    .to_string(),
            );
        }
    } else if report.original_edge_flows.is_some() || report.beneficiary_allocations.is_some() {
        return Err("infeasible floor report must not expose an original allocation".to_string());
    }

    Ok(())
}

/// Verify a floor report against the typed portable request envelope.
pub fn verify_beneficiary_floor_input(
    input: &BeneficiaryFloorInput,
    report: &BeneficiaryFloorReport,
) -> Result<(), String> {
    verify_beneficiary_floor_report(&input.profile, &input.floors, report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal_profile::{
        TEMPORAL_PROFILE_SCHEMA, TemporalNode, TemporalReceipt, TemporalService, TemporalTransfer,
    };

    fn node(
        id: &str,
        storage_capacity: u64,
        handling_capacity: u64,
        service_issue_capacity: u64,
    ) -> TemporalNode {
        TemporalNode {
            node_id: id.to_string(),
            storage_capacity,
            handling_capacity,
            service_issue_capacity,
            opening_inventory: 0,
        }
    }

    fn profile(handling_capacity: u64, route_a: u64, route_b: u64) -> TemporalProfile {
        TemporalProfile {
            schema_version: TEMPORAL_PROFILE_SCHEMA.to_string(),
            case_id: "floor-feasibility-test".to_string(),
            commodity_ref: "diesel".to_string(),
            capacity_unit_ref: "litre".to_string(),
            quantity_boundary_ref: "beneficiary-deliverable-litres".to_string(),
            period_timebase_ref: "synthetic:daily-periods".to_string(),
            horizon_last_period: 1,
            nodes: vec![
                node("origin-yard", 0, handling_capacity, 0),
                node("beneficiary-a", 0, 0, 15),
                node("beneficiary-b", 0, 0, 15),
            ],
            receipts: vec![TemporalReceipt {
                receipt_id: "receipt-20".to_string(),
                node_id: "origin-yard".to_string(),
                available_period: 0,
                quantity: 20,
            }],
            transfers: vec![
                TemporalTransfer {
                    transfer_id: "route-a".to_string(),
                    from_node_id: "origin-yard".to_string(),
                    to_node_id: "beneficiary-a".to_string(),
                    departure_period: 0,
                    transit_periods: 1,
                    route_capacity: route_a,
                },
                TemporalTransfer {
                    transfer_id: "route-b".to_string(),
                    from_node_id: "origin-yard".to_string(),
                    to_node_id: "beneficiary-b".to_string(),
                    departure_period: 0,
                    transit_periods: 1,
                    route_capacity: route_b,
                },
            ],
            services: vec![
                TemporalService {
                    service_id: "service-a".to_string(),
                    node_id: "beneficiary-a".to_string(),
                    period: 1,
                    requested_quantity: 15,
                },
                TemporalService {
                    service_id: "service-b".to_string(),
                    node_id: "beneficiary-b".to_string(),
                    period: 1,
                    requested_quantity: 15,
                },
            ],
        }
    }

    fn next_small(state: &mut u64, bound: u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (*state >> 32) % bound
    }

    fn physical_floor_oracle(
        floors: &[BeneficiaryFloor],
        route_limits: &[u64],
        issue_limits: &[u64],
        requests: &[u64],
        supply: u64,
        handling: u64,
    ) -> bool {
        if floors.len() != route_limits.len()
            || floors.len() != issue_limits.len()
            || floors.len() != requests.len()
        {
            return false;
        }
        let mut total_floor = 0_u64;
        for (((floor, route_limit), issue_limit), request) in floors
            .iter()
            .zip(route_limits.iter())
            .zip(issue_limits.iter())
            .zip(requests.iter())
        {
            let per_service_limit = (*route_limit).min(*issue_limit).min(*request);
            if floor.minimum_required_quantity > per_service_limit {
                return false;
            }
            let Some(next_total) = total_floor.checked_add(floor.minimum_required_quantity) else {
                return false;
            };
            total_floor = next_total;
        }
        total_floor <= supply.min(handling)
    }

    #[test]
    fn seeded_floor_sets_match_independent_physical_feasibility_oracle() {
        for seed in 0..250_u64 {
            let mut rng = 0x05AF_B202_6D15_CA11_u64 ^ seed;
            let count = 2 + next_small(&mut rng, 3) as usize;
            let supply = next_small(&mut rng, 21);
            let handling = next_small(&mut rng, 21);
            let mut nodes = vec![node("origin-yard", 0, handling, 0)];
            let receipts = vec![TemporalReceipt {
                receipt_id: format!("receipt-{seed}"),
                node_id: "origin-yard".to_string(),
                available_period: 0,
                quantity: supply,
            }];
            let mut transfers = Vec::new();
            let mut services = Vec::new();
            let mut floors = Vec::new();
            let mut route_limits = Vec::new();
            let mut issue_limits = Vec::new();
            let mut requests = Vec::new();

            for index in 0..count {
                let destination = format!("beneficiary-{index}");
                let route_capacity = next_small(&mut rng, 13);
                let issue_capacity = next_small(&mut rng, 13);
                let request = next_small(&mut rng, 13);
                let floor = next_small(&mut rng, request + 1);
                nodes.push(node(&destination, 0, 0, issue_capacity));
                transfers.push(TemporalTransfer {
                    transfer_id: format!("route-{seed}-{index}"),
                    from_node_id: "origin-yard".to_string(),
                    to_node_id: destination.clone(),
                    departure_period: 0,
                    transit_periods: 1,
                    route_capacity,
                });
                services.push(TemporalService {
                    service_id: format!("service-{seed}-{index}"),
                    node_id: destination,
                    period: 1,
                    requested_quantity: request,
                });
                floors.push(BeneficiaryFloor {
                    service_id: format!("service-{seed}-{index}"),
                    minimum_required_quantity: floor,
                });
                route_limits.push(route_capacity);
                issue_limits.push(issue_capacity);
                requests.push(request);
            }

            let profile = TemporalProfile {
                schema_version: TEMPORAL_PROFILE_SCHEMA.to_string(),
                case_id: format!("seeded-floor-feasibility-{seed}"),
                commodity_ref: "diesel".to_string(),
                capacity_unit_ref: "litre".to_string(),
                quantity_boundary_ref: "beneficiary-deliverable-litres".to_string(),
                period_timebase_ref: "synthetic:daily-periods".to_string(),
                horizon_last_period: 1,
                nodes,
                receipts,
                transfers,
                services,
            };
            let expected = physical_floor_oracle(
                &floors,
                &route_limits,
                &issue_limits,
                &requests,
                supply,
                handling,
            );
            let report = evaluate_beneficiary_floors(&profile, &floors).unwrap();
            assert_eq!(
                report.floors_jointly_feasible, expected,
                "seed {seed}: independent physical floor oracle disagreed"
            );
            verify_beneficiary_floor_report(&profile, &floors, &report).unwrap();
            if expected {
                assert!(
                    report
                        .beneficiary_allocations
                        .as_ref()
                        .is_some_and(|allocations| {
                            allocations.iter().all(|allocation| {
                                allocation.allocated_quantity
                                    >= allocation.minimum_required_quantity
                            })
                        })
                );
            } else {
                assert!(report.original_edge_flows.is_none());
                assert!(report.beneficiary_allocations.is_none());
            }
        }
    }

    #[test]
    fn joint_floors_are_feasible_with_one_constructive_allocation() {
        let model = profile(15, 10, 10);
        let floors = vec![
            BeneficiaryFloor {
                service_id: "service-a".to_string(),
                minimum_required_quantity: 5,
            },
            BeneficiaryFloor {
                service_id: "service-b".to_string(),
                minimum_required_quantity: 5,
            },
        ];
        let report = evaluate_beneficiary_floors(&model, &floors).unwrap();
        assert!(report.floors_jointly_feasible);
        let allocations = report.beneficiary_allocations.as_ref().unwrap();
        assert_eq!(allocations.len(), 2);
        assert!(allocations.iter().all(|item| item.allocated_quantity >= 5));
        verify_beneficiary_floor_report(&model, &floors, &report).unwrap();
    }

    #[test]
    fn individually_possible_floors_can_be_jointly_infeasible() {
        let model = profile(15, 10, 10);
        let floors = vec![
            BeneficiaryFloor {
                service_id: "service-a".to_string(),
                minimum_required_quantity: 8,
            },
            BeneficiaryFloor {
                service_id: "service-b".to_string(),
                minimum_required_quantity: 8,
            },
        ];
        let report = evaluate_beneficiary_floors(&model, &floors).unwrap();
        assert!(!report.floors_jointly_feasible);
        assert!(report.achieved_auxiliary_flow < report.required_auxiliary_flow);
        assert!(report.original_edge_flows.is_none());
        assert!(report.beneficiary_allocations.is_none());
        verify_beneficiary_floor_report(&model, &floors, &report).unwrap();
    }

    #[test]
    fn floor_above_request_is_rejected_before_solving() {
        let model = profile(15, 10, 10);
        let floors = vec![BeneficiaryFloor {
            service_id: "service-a".to_string(),
            minimum_required_quantity: 16,
        }];
        assert!(
            evaluate_beneficiary_floors(&model, &floors)
                .unwrap_err()
                .contains("exceeds its requested quantity")
        );
    }

    #[test]
    fn unknown_and_duplicate_service_floors_are_rejected() {
        let model = profile(15, 10, 10);
        let unknown = vec![BeneficiaryFloor {
            service_id: "service-missing".to_string(),
            minimum_required_quantity: 1,
        }];
        assert!(
            evaluate_beneficiary_floors(&model, &unknown)
                .unwrap_err()
                .contains("unknown service")
        );

        let duplicate = vec![
            BeneficiaryFloor {
                service_id: "service-a".to_string(),
                minimum_required_quantity: 1,
            },
            BeneficiaryFloor {
                service_id: "service-a".to_string(),
                minimum_required_quantity: 2,
            },
        ];
        assert!(
            evaluate_beneficiary_floors(&model, &duplicate)
                .unwrap_err()
                .contains("duplicate beneficiary floor")
        );
    }

    #[test]
    fn verifier_rejects_changed_floor_and_forged_allocation() {
        let model = profile(15, 10, 10);
        let floors = vec![
            BeneficiaryFloor {
                service_id: "service-a".to_string(),
                minimum_required_quantity: 5,
            },
            BeneficiaryFloor {
                service_id: "service-b".to_string(),
                minimum_required_quantity: 5,
            },
        ];
        let report = evaluate_beneficiary_floors(&model, &floors).unwrap();

        let changed_floor = vec![
            BeneficiaryFloor {
                service_id: "service-a".to_string(),
                minimum_required_quantity: 6,
            },
            floors[1].clone(),
        ];
        assert!(
            verify_beneficiary_floor_report(&model, &changed_floor, &report)
                .unwrap_err()
                .contains("obligations do not match")
        );

        let mut forged = report;
        forged.beneficiary_allocations.as_mut().unwrap()[0].allocated_quantity = 0;
        assert!(
            verify_beneficiary_floor_report(&model, &floors, &forged)
                .unwrap_err()
                .contains("allocation does not match")
        );
    }

    #[test]
    fn published_floor_schemas_are_valid_json_documents() {
        let temporal_schema: serde_json::Value =
            serde_json::from_str(include_str!("../schemas/temporal-profile-v0.1.schema.json"))
                .unwrap();
        let report_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/beneficiary-floor-feasibility-v0.1.schema.json"
        ))
        .unwrap();
        let input_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/beneficiary-floor-feasibility-input-v0.1.schema.json"
        ))
        .unwrap();
        assert_eq!(
            temporal_schema["properties"]["schema_version"]["const"],
            TEMPORAL_PROFILE_SCHEMA
        );
        assert_eq!(
            report_schema["properties"]["schema_version"]["const"],
            FLOOR_FEASIBILITY_SCHEMA
        );
        assert_eq!(input_schema["required"][0], "profile");
        assert_eq!(input_schema["required"][1], "floors");
    }

    #[test]
    fn beneficiary_floor_schemas_expose_strict_top_level_contracts() {
        let report_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/beneficiary-floor-feasibility-v0.1.schema.json"
        ))
        .unwrap();
        let input_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/beneficiary-floor-feasibility-input-v0.1.schema.json"
        ))
        .unwrap();
        for schema in [&report_schema, &input_schema] {
            assert_eq!(schema["additionalProperties"], false);
            assert_eq!(schema["type"], "object");
        }
        assert_eq!(
            report_schema["properties"]["schema_version"]["const"],
            FLOOR_FEASIBILITY_SCHEMA
        );
        assert_eq!(input_schema["required"][0], "profile");
        assert_eq!(input_schema["required"][1], "floors");
    }

    #[test]
    fn floor_input_envelope_round_trips_and_verifies() {
        let input = BeneficiaryFloorInput {
            profile: profile(15, 10, 10),
            floors: vec![
                BeneficiaryFloor {
                    service_id: "service-a".to_string(),
                    minimum_required_quantity: 5,
                },
                BeneficiaryFloor {
                    service_id: "service-b".to_string(),
                    minimum_required_quantity: 5,
                },
            ],
        };
        let bytes = serde_json::to_vec(&input).unwrap();
        let decoded: BeneficiaryFloorInput = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded, input);
        let report = evaluate_beneficiary_floor_input(&decoded).unwrap();
        verify_beneficiary_floor_input(&decoded, &report).unwrap();

        let mut unknown = serde_json::to_value(&input).unwrap();
        unknown["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<BeneficiaryFloorInput>(unknown).is_err());
    }

    #[test]
    fn floor_report_json_round_trip_is_strict() {
        let model = profile(15, 10, 10);
        let floors = vec![BeneficiaryFloor {
            service_id: "service-a".to_string(),
            minimum_required_quantity: 5,
        }];
        let report = evaluate_beneficiary_floors(&model, &floors).unwrap();
        let encoded = serde_json::to_vec(&report).unwrap();
        let decoded: BeneficiaryFloorReport = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, report);
        verify_beneficiary_floor_report(&model, &floors, &decoded).unwrap();

        let mut unknown = serde_json::to_value(&report).unwrap();
        unknown["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<BeneficiaryFloorReport>(unknown).is_err());
    }
}
