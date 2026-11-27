#![forbid(unsafe_code)]

//! Deterministic max-flow/min-cut for already-validated, single-commodity,
//! directed networks with non-negative integer capacities.
//!
//! The kernel knows nothing about units, time, storage, physical identity,
//! product conversion, evidence quality, legal authority, or service semantics.
//! Domain adapters must construct a valid conserved-quantity network before
//! calling it. Lower and upper capacity envelopes are separate graph instances.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

pub const KERNEL_VERSION: &str = "single-commodity-directed-flow-v0.1.0";
pub const CERTIFICATE_SCHEMA_VERSION: &str = "luminous-flow-certificate-v1";
pub const SUPPORTED_MODEL_CLASS: &str = "single_commodity_directed_integer_capacity";
pub const MAX_NODES: usize = 4_096;
pub const MAX_EDGES: usize = 65_536;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowError {
    pub message: String,
}

impl FlowError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for FlowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for FlowError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowNode {
    pub id: String,
}

impl FlowNode {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub capacity: u64,
    /// Opaque domain-owned reference. The kernel never interprets it.
    pub metadata_ref: Option<String>,
}

impl FlowEdge {
    pub fn new(
        id: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
        capacity: u64,
        metadata_ref: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            capacity,
            metadata_ref,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowGraph {
    /// Opaque domain identifier for the single conserved commodity.
    pub commodity_ref: String,
    /// Opaque unit identifier shared by every capacity in this graph.
    pub capacity_unit_ref: String,
    /// Opaque identifier for the common conservation/measurement boundary.
    pub quantity_boundary_ref: String,
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
    pub source: String,
    pub sink: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeFlow {
    pub id: String,
    pub from: String,
    pub to: String,
    pub capacity: u64,
    pub flow: u64,
    pub metadata_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CutEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub capacity: u64,
    pub metadata_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MinimumCut {
    pub capacity: u64,
    pub edges: Vec<CutEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowResult {
    /// Explicit wire-schema version, distinct from the solver algorithm version.
    pub certificate_schema: String,
    /// SHA-256 over the canonical, versioned encoding of the complete graph.
    pub graph_sha256: String,
    pub commodity_ref: String,
    pub capacity_unit_ref: String,
    pub quantity_boundary_ref: String,
    pub kernel_version: String,
    pub supported_model_class: String,
    pub source: String,
    pub sink: String,
    pub max_flow: u64,
    /// One constructive flow value for every original edge, in edge-ID order.
    pub edge_flows: Vec<EdgeFlow>,
    /// Nodes reachable from source in the final residual graph, in ID order.
    pub source_side_nodes: Vec<String>,
    pub min_cut: MinimumCut,
}

/// Portable proof-carrying artifact: the exact input graph plus the
/// constructive max-flow/min-cut result. Consumers must call verify after
/// deserializing; JSON validity alone is not a certificate check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowCertificateBundle {
    pub schema_version: String,
    pub graph: FlowGraph,
    pub result: FlowResult,
}

impl FlowCertificateBundle {
    /// Bind a result to its graph and reject mismatches before exporting.
    pub fn new(graph: FlowGraph, result: FlowResult) -> Result<Self, FlowError> {
        graph.verify_result(&result)?;
        Ok(Self {
            schema_version: CERTIFICATE_SCHEMA_VERSION.to_owned(),
            graph,
            result,
        })
    }

    /// Independently verify the graph/result pair after loading the bundle.
    pub fn verify(&self) -> Result<(), FlowError> {
        if self.schema_version != CERTIFICATE_SCHEMA_VERSION {
            return Err(FlowError::new("unsupported flow certificate bundle schema"));
        }
        self.graph.verify_result(&self.result)
    }
}

#[derive(Debug, Clone)]
struct ResidualArc {
    to: usize,
    reverse_index: usize,
    residual_capacity: u64,
}

impl FlowGraph {
    /// Validate and encode the graph using a deterministic, length-prefixed
    /// binary format. Node and edge input order does not affect the encoding.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, FlowError> {
        self.validate()?;

        let mut nodes = self.nodes.iter().collect::<Vec<_>>();
        nodes.sort_by(|a, b| a.id.cmp(&b.id));

        let mut edges = self.edges.iter().collect::<Vec<_>>();
        edges.sort_by(|a, b| a.id.cmp(&b.id));

        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"LUMINOUS-SCFK-GRAPH-V1\0");
        push_string(&mut bytes, &self.commodity_ref);
        push_string(&mut bytes, &self.capacity_unit_ref);
        push_string(&mut bytes, &self.quantity_boundary_ref);
        push_string(&mut bytes, &self.source);
        push_string(&mut bytes, &self.sink);
        push_u64(&mut bytes, nodes.len() as u64);
        for node in nodes {
            push_string(&mut bytes, &node.id);
        }
        push_u64(&mut bytes, edges.len() as u64);
        for edge in edges {
            push_string(&mut bytes, &edge.id);
            push_string(&mut bytes, &edge.from);
            push_string(&mut bytes, &edge.to);
            push_u64(&mut bytes, edge.capacity);
            match &edge.metadata_ref {
                Some(reference) => {
                    bytes.push(1);
                    push_string(&mut bytes, reference);
                }
                None => bytes.push(0),
            }
        }
        Ok(bytes)
    }

    /// Solve the supplied graph and return a constructive flow and min-cut.
    pub fn solve(&self) -> Result<FlowResult, FlowError> {
        self.validate()?;

        let canonical = self.canonical_bytes()?;
        let graph_sha256 = format!("sha256:{}", hex_lower(&Sha256::digest(canonical)));

        let mut nodes = self.nodes.iter().collect::<Vec<_>>();
        nodes.sort_by(|a, b| a.id.cmp(&b.id));
        let node_indices = nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.as_str(), index))
            .collect::<BTreeMap<_, _>>();

        let mut edges = self.edges.iter().collect::<Vec<_>>();
        edges.sort_by(|a, b| a.id.cmp(&b.id));

        let mut residual = vec![Vec::<ResidualArc>::new(); nodes.len()];
        let mut forward_locations = Vec::with_capacity(edges.len());
        let mut source_capacity = 0_u64;

        for edge in &edges {
            let from = *node_indices
                .get(edge.from.as_str())
                .ok_or_else(|| FlowError::new("validated edge source disappeared"))?;
            let to = *node_indices
                .get(edge.to.as_str())
                .ok_or_else(|| FlowError::new("validated edge target disappeared"))?;
            if edge.from == self.source {
                source_capacity = source_capacity
                    .checked_add(edge.capacity)
                    .ok_or_else(|| FlowError::new("sum of source capacities overflowed u64"))?;
            }

            let forward_index = residual[from].len();
            let reverse_index = residual[to].len();
            residual[from].push(ResidualArc {
                to,
                reverse_index,
                residual_capacity: edge.capacity,
            });
            residual[to].push(ResidualArc {
                to: from,
                reverse_index: forward_index,
                residual_capacity: 0,
            });
            forward_locations.push((from, forward_index));
        }

        let source = *node_indices
            .get(self.source.as_str())
            .ok_or_else(|| FlowError::new("validated source disappeared"))?;
        let sink = *node_indices
            .get(self.sink.as_str())
            .ok_or_else(|| FlowError::new("validated sink disappeared"))?;

        let mut max_flow = 0_u64;
        while max_flow < source_capacity {
            let levels = build_levels(&residual, source);
            if levels[sink] == usize::MAX {
                break;
            }
            let mut next_arc = vec![0_usize; residual.len()];
            loop {
                let pushed = send_one_path(
                    &mut residual,
                    &levels,
                    &mut next_arc,
                    source,
                    sink,
                    source_capacity - max_flow,
                )?;
                if pushed == 0 {
                    break;
                }
                max_flow = max_flow
                    .checked_add(pushed)
                    .ok_or_else(|| FlowError::new("maximum flow overflowed u64"))?;
            }
        }

        let reachable = residual_reachable(&residual, source);
        let source_side_nodes = nodes
            .iter()
            .enumerate()
            .filter(|(index, _)| reachable[*index])
            .map(|(_, node)| node.id.clone())
            .collect::<Vec<_>>();

        let mut edge_flows = Vec::with_capacity(edges.len());
        let mut cut_edges = Vec::new();
        let mut cut_capacity = 0_u64;

        for (edge, (from, forward_index)) in edges.iter().zip(forward_locations) {
            let residual_capacity = residual[from][forward_index].residual_capacity;
            let flow = edge
                .capacity
                .checked_sub(residual_capacity)
                .ok_or_else(|| FlowError::new("residual capacity exceeded original capacity"))?;
            edge_flows.push(EdgeFlow {
                id: edge.id.clone(),
                from: edge.from.clone(),
                to: edge.to.clone(),
                capacity: edge.capacity,
                flow,
                metadata_ref: edge.metadata_ref.clone(),
            });

            let to = *node_indices
                .get(edge.to.as_str())
                .ok_or_else(|| FlowError::new("validated edge target disappeared"))?;
            if reachable[from] && !reachable[to] {
                cut_capacity = cut_capacity
                    .checked_add(edge.capacity)
                    .ok_or_else(|| FlowError::new("minimum-cut capacity overflowed u64"))?;
                cut_edges.push(CutEdge {
                    id: edge.id.clone(),
                    from: edge.from.clone(),
                    to: edge.to.clone(),
                    capacity: edge.capacity,
                    metadata_ref: edge.metadata_ref.clone(),
                });
            }
        }

        if cut_capacity != max_flow {
            return Err(FlowError::new(format!(
                "internal max-flow/min-cut invariant failed: flow {max_flow} != cut {cut_capacity}"
            )));
        }

        let result = FlowResult {
            certificate_schema: CERTIFICATE_SCHEMA_VERSION.to_owned(),
            graph_sha256,
            commodity_ref: self.commodity_ref.clone(),
            capacity_unit_ref: self.capacity_unit_ref.clone(),
            quantity_boundary_ref: self.quantity_boundary_ref.clone(),
            kernel_version: KERNEL_VERSION.to_owned(),
            supported_model_class: SUPPORTED_MODEL_CLASS.to_owned(),
            source: self.source.clone(),
            sink: self.sink.clone(),
            max_flow,
            edge_flows,
            source_side_nodes,
            min_cut: MinimumCut {
                capacity: cut_capacity,
                edges: cut_edges,
            },
        };
        self.verify_result(&result)?;
        Ok(result)
    }

    /// Verify a previously produced certificate without running max-flow.
    /// Checks graph binding, edge identity/capacity, conservation, source/sink
    /// partition, exact cut membership and max-flow/min-cut equality.
    pub fn verify_result(&self, result: &FlowResult) -> Result<(), FlowError> {
        self.validate()?;

        let canonical = self.canonical_bytes()?;
        let expected_digest = format!("sha256:{}", hex_lower(&Sha256::digest(canonical)));
        if result.graph_sha256 != expected_digest {
            return Err(FlowError::new(
                "graph SHA-256 does not match the supplied graph",
            ));
        }
        if result.commodity_ref != self.commodity_ref
            || result.capacity_unit_ref != self.capacity_unit_ref
            || result.quantity_boundary_ref != self.quantity_boundary_ref
        {
            return Err(FlowError::new(
                "quantity contract does not match the supplied graph",
            ));
        }
        if result.certificate_schema != CERTIFICATE_SCHEMA_VERSION {
            return Err(FlowError::new("unsupported flow certificate schema"));
        }
        if result.kernel_version != KERNEL_VERSION
            || result.supported_model_class != SUPPORTED_MODEL_CLASS
        {
            return Err(FlowError::new("unsupported kernel version or model class"));
        }
        if result.source != self.source || result.sink != self.sink {
            return Err(FlowError::new(
                "certificate source/sink does not match the graph",
            ));
        }

        let edge_by_id = self
            .edges
            .iter()
            .map(|edge| (edge.id.as_str(), edge))
            .collect::<BTreeMap<_, _>>();
        if result.edge_flows.len() != self.edges.len() {
            return Err(FlowError::new(
                "certificate edge-flow count does not match graph",
            ));
        }

        let mut flow_by_id = BTreeMap::new();
        let mut balance = BTreeMap::<String, i128>::new();
        for node in &self.nodes {
            balance.insert(node.id.clone(), 0);
        }

        for witness in &result.edge_flows {
            let edge = edge_by_id
                .get(witness.id.as_str())
                .ok_or_else(|| FlowError::new("certificate contains an unknown edge ID"))?;
            if flow_by_id.insert(witness.id.as_str(), witness).is_some() {
                return Err(FlowError::new("certificate contains a duplicate edge ID"));
            }
            if witness.from != edge.from
                || witness.to != edge.to
                || witness.capacity != edge.capacity
                || witness.metadata_ref != edge.metadata_ref
            {
                return Err(FlowError::new("edge witness identity or capacity mismatch"));
            }
            if witness.flow > witness.capacity {
                return Err(FlowError::new("edge witness exceeds its capacity"));
            }
            *balance
                .get_mut(&witness.from)
                .ok_or_else(|| FlowError::new("edge witness references unknown source node"))? +=
                i128::from(witness.flow);
            *balance
                .get_mut(&witness.to)
                .ok_or_else(|| FlowError::new("edge witness references unknown target node"))? -=
                i128::from(witness.flow);
        }
        if flow_by_id.len() != edge_by_id.len() {
            return Err(FlowError::new(
                "certificate omits one or more original edges",
            ));
        }

        for (node_id, net_outflow) in &balance {
            let expected = if node_id == &self.source {
                i128::from(result.max_flow)
            } else if node_id == &self.sink {
                -i128::from(result.max_flow)
            } else {
                0
            };
            if *net_outflow != expected {
                return Err(FlowError::new(format!(
                    "flow conservation failed at node {node_id}"
                )));
            }
        }

        let known_nodes = self
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<BTreeSet<_>>();
        let source_side = result
            .source_side_nodes
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if source_side.len() != result.source_side_nodes.len()
            || !source_side.contains(self.source.as_str())
            || source_side.contains(self.sink.as_str())
            || source_side.iter().any(|node| !known_nodes.contains(*node))
        {
            return Err(FlowError::new("invalid residual source-side node set"));
        }

        // Reconstruct the residual graph from the submitted edge-flow
        // witness. The reported source-side set is specifically the set
        // reachable from source in this residual graph, not an arbitrary
        // equal-capacity minimum-cut partition.
        let mut residual_adjacency = self
            .nodes
            .iter()
            .map(|node| (node.id.clone(), Vec::<String>::new()))
            .collect::<BTreeMap<_, _>>();
        for witness in &result.edge_flows {
            if witness.flow < witness.capacity {
                residual_adjacency
                    .get_mut(&witness.from)
                    .ok_or_else(|| FlowError::new("residual witness references an unknown tail"))?
                    .push(witness.to.clone());
            }
            if witness.flow > 0 {
                residual_adjacency
                    .get_mut(&witness.to)
                    .ok_or_else(|| FlowError::new("residual witness references an unknown head"))?
                    .push(witness.from.clone());
            }
        }
        let mut residual_reachable = BTreeSet::new();
        let mut residual_queue = VecDeque::new();
        residual_reachable.insert(self.source.clone());
        residual_queue.push_back(self.source.clone());
        while let Some(node_id) = residual_queue.pop_front() {
            if let Some(neighbors) = residual_adjacency.get(&node_id) {
                for neighbor in neighbors {
                    if residual_reachable.insert(neighbor.clone()) {
                        residual_queue.push_back(neighbor.clone());
                    }
                }
            }
        }
        let mut sorted_node_ids = self
            .nodes
            .iter()
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        sorted_node_ids.sort();
        let expected_source_side = sorted_node_ids
            .into_iter()
            .filter(|node_id| residual_reachable.contains(node_id))
            .collect::<Vec<_>>();
        if result.source_side_nodes != expected_source_side {
            return Err(FlowError::new(
                "source-side node set does not match residual reachability of the edge-flow witness",
            ));
        }

        let mut expected_cut = Vec::new();
        let mut expected_cut_capacity = 0_u64;
        let mut sorted_edges = self.edges.iter().collect::<Vec<_>>();
        sorted_edges.sort_by(|a, b| a.id.cmp(&b.id));
        for edge in sorted_edges {
            if source_side.contains(edge.from.as_str()) && !source_side.contains(edge.to.as_str()) {
                expected_cut_capacity = expected_cut_capacity
                    .checked_add(edge.capacity)
                    .ok_or_else(|| FlowError::new("verified cut capacity overflowed u64"))?;
                expected_cut.push(CutEdge {
                    id: edge.id.clone(),
                    from: edge.from.clone(),
                    to: edge.to.clone(),
                    capacity: edge.capacity,
                    metadata_ref: edge.metadata_ref.clone(),
                });
            }
        }
        if result.min_cut.edges != expected_cut {
            return Err(FlowError::new(
                "minimum-cut edge set does not match source partition",
            ));
        }
        if result.min_cut.capacity != expected_cut_capacity
            || result.min_cut.capacity != result.max_flow
        {
            return Err(FlowError::new(
                "minimum-cut capacity does not equal maximum flow",
            ));
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), FlowError> {
        if self.nodes.len() < 2 || self.nodes.len() > MAX_NODES {
            return Err(FlowError::new(format!(
                "node count must be within 2..={MAX_NODES}"
            )));
        }
        if self.edges.len() > MAX_EDGES {
            return Err(FlowError::new(format!(
                "edge count must be at most {MAX_EDGES}"
            )));
        }
        if self.commodity_ref.trim().is_empty()
            || self.capacity_unit_ref.trim().is_empty()
            || self.quantity_boundary_ref.trim().is_empty()
        {
            return Err(FlowError::new(
                "commodity, capacity unit and quantity boundary references must be non-empty",
            ));
        }
        if self.source.trim().is_empty() || self.sink.trim().is_empty() {
            return Err(FlowError::new("source and sink IDs must be non-empty"));
        }
        if self.source == self.sink {
            return Err(FlowError::new("source and sink must be distinct"));
        }

        let mut node_ids = BTreeSet::new();
        for node in &self.nodes {
            if node.id.trim().is_empty() {
                return Err(FlowError::new("node IDs must be non-empty"));
            }
            if !node_ids.insert(node.id.as_str()) {
                return Err(FlowError::new(format!("duplicate node ID {}", node.id)));
            }
        }
        if !node_ids.contains(self.source.as_str()) {
            return Err(FlowError::new("source node is missing"));
        }
        if !node_ids.contains(self.sink.as_str()) {
            return Err(FlowError::new("sink node is missing"));
        }

        let mut edge_ids = BTreeSet::new();
        for edge in &self.edges {
            if edge.id.trim().is_empty() {
                return Err(FlowError::new("edge IDs must be non-empty"));
            }
            if !edge_ids.insert(edge.id.as_str()) {
                return Err(FlowError::new(format!("duplicate edge ID {}", edge.id)));
            }
            if edge.from == edge.to {
                return Err(FlowError::new(format!(
                    "self-loop edge {} is outside the supported graph profile",
                    edge.id
                )));
            }
            if !node_ids.contains(edge.from.as_str()) || !node_ids.contains(edge.to.as_str()) {
                return Err(FlowError::new(format!(
                    "edge {} has a dangling endpoint",
                    edge.id
                )));
            }
        }
        Ok(())
    }
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_string(bytes: &mut Vec<u8>, value: &str) {
    push_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn build_levels(graph: &[Vec<ResidualArc>], source: usize) -> Vec<usize> {
    let mut levels = vec![usize::MAX; graph.len()];
    let mut queue = VecDeque::new();
    levels[source] = 0;
    queue.push_back(source);
    while let Some(node) = queue.pop_front() {
        for edge in &graph[node] {
            if edge.residual_capacity > 0 && levels[edge.to] == usize::MAX {
                levels[edge.to] = levels[node] + 1;
                queue.push_back(edge.to);
            }
        }
    }
    levels
}

/// Iterative DFS for one augmenting path in a Dinic level graph. An explicit
/// stack bounds call-stack use even for long time-expanded paths.
fn send_one_path(
    graph: &mut [Vec<ResidualArc>],
    levels: &[usize],
    next_arc: &mut [usize],
    source: usize,
    sink: usize,
    flow_limit: u64,
) -> Result<u64, FlowError> {
    if flow_limit == 0 {
        return Ok(0);
    }
    let mut nodes = vec![source];
    let mut path = Vec::<(usize, usize)>::new();
    let mut bottlenecks = vec![flow_limit];

    loop {
        let node = *nodes
            .last()
            .ok_or_else(|| FlowError::new("internal path stack became empty"))?;
        if node == sink {
            let pushed = *bottlenecks
                .last()
                .ok_or_else(|| FlowError::new("internal bottleneck stack became empty"))?;
            for (from, edge_index) in &path {
                let to = graph[*from][*edge_index].to;
                let reverse_index = graph[*from][*edge_index].reverse_index;
                graph[*from][*edge_index].residual_capacity -= pushed;
                graph[to][reverse_index].residual_capacity = graph[to][reverse_index]
                    .residual_capacity
                    .checked_add(pushed)
                    .ok_or_else(|| FlowError::new("reverse residual capacity overflowed u64"))?;
            }
            return Ok(pushed);
        }

        while next_arc[node] < graph[node].len() {
            let edge_index = next_arc[node];
            let edge = &graph[node][edge_index];
            if edge.residual_capacity > 0 && levels[edge.to] == levels[node] + 1 {
                break;
            }
            next_arc[node] += 1;
        }

        if next_arc[node] == graph[node].len() {
            if node == source {
                return Ok(0);
            }
            nodes.pop();
            bottlenecks.pop();
            let (parent, _) = path
                .pop()
                .ok_or_else(|| FlowError::new("internal path edge stack became empty"))?;
            next_arc[parent] += 1;
        } else {
            let edge_index = next_arc[node];
            let edge = &graph[node][edge_index];
            let next_bottleneck = (*bottlenecks
                .last()
                .ok_or_else(|| FlowError::new("internal bottleneck stack became empty"))?)
            .min(edge.residual_capacity);
            path.push((node, edge_index));
            nodes.push(edge.to);
            bottlenecks.push(next_bottleneck);
        }
    }
}

fn residual_reachable(graph: &[Vec<ResidualArc>], source: usize) -> Vec<bool> {
    let mut reachable = vec![false; graph.len()];
    let mut queue = VecDeque::new();
    reachable[source] = true;
    queue.push_back(source);
    while let Some(node) = queue.pop_front() {
        for edge in &graph[node] {
            if edge.residual_capacity > 0 && !reachable[edge.to] {
                reachable[edge.to] = true;
                queue.push_back(edge.to);
            }
        }
    }
    reachable
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(nodes: usize, edges: &[(usize, usize, u64)]) -> FlowGraph {
        let node_rows = (0..nodes)
            .map(|index| FlowNode::new(format!("n{index}")))
            .collect::<Vec<_>>();
        let edge_rows = edges
            .iter()
            .enumerate()
            .map(|(index, (from, to, capacity))| {
                FlowEdge::new(
                    format!("e{index:03}"),
                    format!("n{from}"),
                    format!("n{to}"),
                    *capacity,
                    None,
                )
            })
            .collect::<Vec<_>>();
        FlowGraph {
            commodity_ref: "test-commodity".to_string(),
            capacity_unit_ref: "test-capacity-unit".to_string(),
            quantity_boundary_ref: "test-boundary".to_string(),
            nodes: node_rows,
            edges: edge_rows,
            source: "n0".to_string(),
            sink: format!("n{}", nodes - 1),
        }
    }

    /// Independent exhaustive s/t cut enumerator; deliberately does not use
    /// the residual network or Dinic implementation.
    fn exhaustive_min_cut(graph: &FlowGraph) -> u64 {
        let node_ids = graph
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>();
        let source = node_ids.iter().position(|id| *id == graph.source).unwrap();
        let sink = node_ids.iter().position(|id| *id == graph.sink).unwrap();
        let mut best = u64::MAX;
        for mask in 0_usize..(1_usize << node_ids.len()) {
            if mask & (1 << source) == 0 || mask & (1 << sink) != 0 {
                continue;
            }
            let mut capacity = 0_u64;
            for edge in &graph.edges {
                let from = node_ids.iter().position(|id| *id == edge.from).unwrap();
                let to = node_ids.iter().position(|id| *id == edge.to).unwrap();
                if mask & (1 << from) != 0 && mask & (1 << to) == 0 {
                    capacity += edge.capacity;
                }
            }
            best = best.min(capacity);
        }
        best
    }

    #[test]
    fn all_simple_directed_topologies_through_four_nodes_match_cut_enumerator() {
        for node_count in 2..=4 {
            let mut possible = Vec::new();
            for from in 0..node_count {
                for to in 0..node_count {
                    if from != to {
                        possible.push((from, to));
                    }
                }
            }
            let topology_count = 1_usize << possible.len();
            for mask in 0..topology_count {
                let edges = possible
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| mask & (1 << index) != 0)
                    .map(|(index, (from, to))| {
                        let capacity = ((index * 7 + mask * 3) % 5) as u64;
                        (*from, *to, capacity)
                    })
                    .collect::<Vec<_>>();
                let candidate = graph(node_count, &edges);
                let result = candidate.solve().unwrap();
                assert_eq!(
                    result.max_flow,
                    exhaustive_min_cut(&candidate),
                    "node_count={node_count}, mask={mask}"
                );
                candidate.verify_result(&result).unwrap();
            }
        }
    }

    #[test]
    fn seeded_graphs_through_eight_nodes_match_independent_cut_oracle() {
        // Fixed local PRNG: no external crate, no wall-clock seed and no
        // benchmark-only behavior in the production kernel.
        let mut state = 0x6a09_e667_f3bc_c909_u64;
        let mut next = || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            state ^ (state >> 23)
        };

        let mut checked = 0_usize;
        for node_count in 2..=8 {
            for _case in 0..160 {
                let mut edges = Vec::new();
                for from in 0..node_count {
                    for to in 0..node_count {
                        if from == to {
                            continue;
                        }
                        let sample = next();
                        if sample % 100 < 37 {
                            // Mixed small and wide values exercise capacity
                            // skew without overflowing the total source cut.
                            let capacity = match sample % 4 {
                                0 => sample & 0x0f,
                                1 => (sample & 0xffff) + 1,
                                2 => (sample & 0xffff_ffff) + 1,
                                _ => (sample & 0x0000_ffff_ffff_ffff) + 1,
                            };
                            edges.push((from, to, capacity));
                        }
                    }
                }

                let candidate = graph(node_count, &edges);
                let expected = exhaustive_min_cut(&candidate);
                let result = candidate.solve().unwrap();
                assert_eq!(
                    result.max_flow, expected,
                    "seeded differential case {checked}, nodes={node_count}"
                );
                assert_eq!(result.min_cut.capacity, expected);
                candidate.verify_result(&result).unwrap();
                checked += 1;
            }
        }

        assert_eq!(checked, 7 * 160);
    }

    #[test]
    fn parallel_antiparallel_cycles_and_zero_capacity_edges_are_supported() {
        let candidate = FlowGraph {
            commodity_ref: "test-commodity".to_string(),
            capacity_unit_ref: "test-capacity-unit".to_string(),
            quantity_boundary_ref: "test-boundary".to_string(),
            nodes: vec![
                FlowNode::new("s"),
                FlowNode::new("a"),
                FlowNode::new("b"),
                FlowNode::new("t"),
            ],
            edges: vec![
                FlowEdge::new("parallel-a", "s", "a", 2, None),
                FlowEdge::new("parallel-b", "s", "a", 3, None),
                FlowEdge::new("backward", "a", "s", 4, None),
                FlowEdge::new("cycle-ab", "a", "b", 7, None),
                FlowEdge::new("cycle-ba", "b", "a", 1, None),
                FlowEdge::new("zero", "b", "t", 0, None),
                FlowEdge::new("b-to-t", "b", "t", 4, None),
                FlowEdge::new("a-to-t", "a", "t", 1, None),
            ],
            source: "s".to_string(),
            sink: "t".to_string(),
        };
        let result = candidate.solve().unwrap();
        assert_eq!(result.max_flow, 5);
        assert_eq!(result.min_cut.capacity, 5);
        candidate.verify_result(&result).unwrap();
    }

    #[test]
    fn u64_max_capacity_and_disconnected_graphs_have_exact_certificates() {
        let maximum_capacity = graph(2, &[(0, 1, u64::MAX)]);
        let maximum = maximum_capacity.solve().unwrap();
        assert_eq!(maximum.max_flow, u64::MAX);
        assert_eq!(maximum.min_cut.capacity, u64::MAX);
        maximum_capacity.verify_result(&maximum).unwrap();

        let maximum_bundle = FlowCertificateBundle::new(maximum_capacity.clone(), maximum).unwrap();
        let encoded = serde_json::to_vec(&maximum_bundle).unwrap();
        assert!(String::from_utf8_lossy(&encoded).contains("18446744073709551615"));
        let decoded: FlowCertificateBundle = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded.result.max_flow, u64::MAX);
        decoded.verify().unwrap();

        let disconnected = graph(2, &[]);
        let zero = disconnected.solve().unwrap();
        assert_eq!(zero.max_flow, 0);
        assert_eq!(zero.min_cut.capacity, 0);
        assert_eq!(zero.source_side_nodes, vec!["n0".to_string()]);
        disconnected.verify_result(&zero).unwrap();
    }

    #[test]
    fn reverse_residual_edges_can_reroute_an_earlier_augmentation() {
        let candidate = FlowGraph {
            commodity_ref: "test-commodity".to_string(),
            capacity_unit_ref: "test-capacity-unit".to_string(),
            quantity_boundary_ref: "test-boundary".to_string(),
            nodes: vec![
                FlowNode::new("s"),
                FlowNode::new("a"),
                FlowNode::new("b"),
                FlowNode::new("x"),
                FlowNode::new("y"),
                FlowNode::new("t"),
            ],
            edges: vec![
                FlowEdge::new("00-s-a", "s", "a", 1, None),
                FlowEdge::new("01-s-b", "s", "b", 1, None),
                FlowEdge::new("02-a-x", "a", "x", 1, None),
                FlowEdge::new("03-a-y", "a", "y", 1, None),
                FlowEdge::new("04-b-x", "b", "x", 1, None),
                FlowEdge::new("05-x-t", "x", "t", 1, None),
                FlowEdge::new("06-y-t", "y", "t", 1, None),
            ],
            source: "s".to_string(),
            sink: "t".to_string(),
        };

        let result = candidate.solve().unwrap();
        assert_eq!(result.max_flow, 2);
        assert_eq!(result.min_cut.capacity, 2);
        candidate.verify_result(&result).unwrap();

        let flows = result
            .edge_flows
            .iter()
            .map(|edge| (edge.id.as_str(), edge.flow))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(flows["00-s-a"], 1);
        assert_eq!(flows["01-s-b"], 1);
        assert_eq!(
            flows["02-a-x"], 0,
            "the first augmentation must be cancelled through a reverse residual arc"
        );
        assert_eq!(flows["03-a-y"], 1);
        assert_eq!(flows["04-b-x"], 1);
        assert_eq!(flows["05-x-t"], 1);
        assert_eq!(flows["06-y-t"], 1);
    }

    #[test]
    fn graph_digest_and_witness_are_permutation_invariant() {
        let first = graph(4, &[(0, 1, 3), (0, 2, 2), (1, 3, 3), (2, 3, 2)]);
        let mut second = first.clone();
        second.nodes.reverse();
        second.edges.reverse();
        let a = first.solve().unwrap();
        let b = second.solve().unwrap();
        assert_eq!(a.graph_sha256, b.graph_sha256);
        assert_eq!(a.edge_flows, b.edge_flows);
        assert_eq!(a.source_side_nodes, b.source_side_nodes);
        assert_eq!(a.min_cut, b.min_cut);
    }

    #[test]
    fn certificate_cannot_substitute_an_equal_capacity_cut_partition() {
        let candidate = FlowGraph {
            commodity_ref: "test-commodity".to_string(),
            capacity_unit_ref: "test-capacity-unit".to_string(),
            quantity_boundary_ref: "test-boundary".to_string(),
            nodes: vec![
                FlowNode::new("s"),
                FlowNode::new("a"),
                FlowNode::new("b"),
                FlowNode::new("t"),
            ],
            edges: vec![
                FlowEdge::new("s-a", "s", "a", 1, None),
                FlowEdge::new("s-b", "s", "b", 1, None),
                FlowEdge::new("a-t", "a", "t", 1, None),
                FlowEdge::new("b-t", "b", "t", 1, None),
            ],
            source: "s".to_string(),
            sink: "t".to_string(),
        };

        let mut result = candidate.solve().unwrap();
        assert_eq!(result.max_flow, 2);
        assert_eq!(result.source_side_nodes, vec!["s".to_string()]);

        // {s,a} is a different but equal-capacity cut (2). It is not the
        // residual-reachable source side of this submitted flow witness.
        result.source_side_nodes = vec!["a".to_string(), "s".to_string()];
        result.min_cut = MinimumCut {
            capacity: 2,
            edges: vec![
                CutEdge {
                    id: "a-t".to_string(),
                    from: "a".to_string(),
                    to: "t".to_string(),
                    capacity: 1,
                    metadata_ref: None,
                },
                CutEdge {
                    id: "s-b".to_string(),
                    from: "s".to_string(),
                    to: "b".to_string(),
                    capacity: 1,
                    metadata_ref: None,
                },
            ],
        };
        assert!(candidate.verify_result(&result).is_err());
    }

    #[test]
    fn portable_bundle_binds_graph_and_certificate_after_deserialization() {
        let candidate = graph(4, &[(0, 1, 7), (0, 2, 5), (1, 2, 2), (1, 3, 4), (2, 3, 8)]);
        let result = candidate.solve().unwrap();
        let bundle = FlowCertificateBundle::new(candidate.clone(), result).unwrap();
        let encoded = serde_json::to_vec(&bundle).unwrap();
        let decoded: FlowCertificateBundle = serde_json::from_slice(&encoded).unwrap();
        decoded.verify().unwrap();

        let mut wrong_graph = decoded.clone();
        wrong_graph.graph.edges[0].capacity += 1;
        assert!(wrong_graph.verify().is_err());

        let mut wrong_bundle_schema = decoded.clone();
        wrong_bundle_schema.schema_version = "luminous-flow-bundle-v0".to_string();
        assert!(wrong_bundle_schema.verify().is_err());

        let mut unknown = serde_json::to_value(&decoded).unwrap();
        unknown["unrecognized_semantics"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<FlowCertificateBundle>(unknown).is_err());
    }

    #[test]
    fn serialized_certificate_round_trips_and_reverifies_portably() {
        let candidate = graph(4, &[(0, 1, 7), (0, 2, 5), (1, 2, 2), (1, 3, 4), (2, 3, 8)]);
        let result = candidate.solve().unwrap();
        let encoded = serde_json::to_vec(&result).unwrap();
        let decoded: FlowResult = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded, result);
        assert_eq!(decoded.certificate_schema, CERTIFICATE_SCHEMA_VERSION);
        candidate.verify_result(&decoded).unwrap();

        let mut unknown = serde_json::to_value(&decoded).unwrap();
        unknown["unrecognized_claim"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<FlowResult>(unknown).is_err());

        let mut wrong_schema = decoded.clone();
        wrong_schema.certificate_schema = "luminous-flow-certificate-v0".to_string();
        assert!(candidate.verify_result(&wrong_schema).is_err());
    }

    #[test]
    fn certificate_mutations_are_rejected() {
        let candidate = graph(3, &[(0, 1, 4), (1, 2, 3)]);
        let mut result = candidate.solve().unwrap();
        result.edge_flows[0].flow += 1;
        assert!(candidate.verify_result(&result).is_err());

        let mut result = candidate.solve().unwrap();
        result.graph_sha256.push('0');
        assert!(candidate.verify_result(&result).is_err());

        let mut result = candidate.solve().unwrap();
        result.min_cut.capacity += 1;
        assert!(candidate.verify_result(&result).is_err());

        let mut result = candidate.solve().unwrap();
        result.capacity_unit_ref = "different-unit".to_string();
        assert!(candidate.verify_result(&result).is_err());

        let mut different_contract = candidate.clone();
        different_contract.quantity_boundary_ref = "different-boundary".to_string();
        assert_ne!(
            candidate.solve().unwrap().graph_sha256,
            different_contract.solve().unwrap().graph_sha256
        );
    }

    #[test]
    fn malformed_graphs_and_integer_overflow_are_rejected() {
        let mut candidate = graph(2, &[]);
        candidate.source = "n1".to_string();
        candidate.sink = "n1".to_string();
        assert!(candidate.solve().is_err());

        let mut candidate = graph(2, &[]);
        candidate.commodity_ref.clear();
        assert!(candidate.solve().is_err());

        let mut candidate = graph(2, &[]);
        candidate.capacity_unit_ref.clear();
        assert!(candidate.solve().is_err());

        let mut candidate = graph(2, &[]);
        candidate.edges = vec![
            FlowEdge::new("a", "n0", "n1", u64::MAX, None),
            FlowEdge::new("b", "n0", "n1", 1, None),
        ];
        assert!(candidate.solve().is_err());

        let mut candidate = graph(3, &[(0, 1, 1)]);
        candidate
            .edges
            .push(FlowEdge::new("dangling", "n1", "missing", 1, None));
        assert!(candidate.solve().is_err());
    }

    #[test]
    fn known_answer_returns_constructive_flow_and_cut() {
        let candidate = graph(4, &[(0, 1, 3), (0, 2, 2), (1, 3, 2), (2, 3, 2), (1, 2, 1)]);
        let result = candidate.solve().unwrap();
        assert_eq!(result.max_flow, 4);
        assert_eq!(result.min_cut.capacity, 4);
        assert_eq!(
            result.edge_flows.iter().map(|edge| edge.flow).sum::<u64>(),
            8
        );
        candidate.verify_result(&result).unwrap();
    }
}
