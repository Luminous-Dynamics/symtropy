#![forbid(unsafe_code)]

//! Deterministic max-flow/min-cut for already-validated, single-commodity,
//! directed networks with non-negative integer capacities.
//!
//! The kernel knows nothing about units, time, storage, physical identity,
//! product conversion, evidence quality, legal authority, or service semantics.
//! Domain adapters must construct a valid conserved-quantity network before
//! calling it. Lower and upper capacity envelopes are separate graph instances.

use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

pub const KERNEL_VERSION: &str = "single-commodity-directed-flow-v0.1.0";
pub const SUPPORTED_MODEL_CLASS: &str = "single_commodity_directed_integer_capacity";
pub const MAX_NODES: usize = 4_096;
pub const MAX_EDGES: usize = 65_536;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowError {
    pub message: String,
}

impl FlowError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for FlowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for FlowError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowNode {
    pub id: String,
}

impl FlowNode {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowGraph {
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
    pub source: String,
    pub sink: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeFlow {
    pub id: String,
    pub from: String,
    pub to: String,
    pub capacity: u64,
    pub flow: u64,
    pub metadata_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CutEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub capacity: u64,
    pub metadata_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinimumCut {
    pub capacity: u64,
    pub edges: Vec<CutEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowResult {
    /// SHA-256 over the canonical, versioned encoding of the complete graph.
    pub graph_sha256: String,
    pub kernel_version: &'static str,
    pub supported_model_class: &'static str,
    pub source: String,
    pub sink: String,
    pub max_flow: u64,
    /// One constructive flow value for every original edge, in edge-ID order.
    pub edge_flows: Vec<EdgeFlow>,
    /// Nodes reachable from source in the final residual graph, in ID order.
    pub source_side_nodes: Vec<String>,
    pub min_cut: MinimumCut,
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
            graph_sha256,
            kernel_version: KERNEL_VERSION,
            supported_model_class: SUPPORTED_MODEL_CLASS,
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
            return Err(FlowError::new("graph SHA-256 does not match the supplied graph"));
        }
        if result.kernel_version != KERNEL_VERSION
            || result.supported_model_class != SUPPORTED_MODEL_CLASS
        {
            return Err(FlowError::new("unsupported kernel version or model class"));
        }
        if result.source != self.source || result.sink != self.sink {
            return Err(FlowError::new("certificate source/sink does not match the graph"));
        }

        let edge_by_id = self
            .edges
            .iter()
            .map(|edge| (edge.id.as_str(), edge))
            .collect::<BTreeMap<_, _>>();
        if result.edge_flows.len() != self.edges.len() {
            return Err(FlowError::new("certificate edge-flow count does not match graph"));
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
            return Err(FlowError::new("certificate omits one or more original edges"));
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
            || source_side.iter().any(|node| !known_nodes.contains(node))
        {
            return Err(FlowError::new("invalid residual source-side node set"));
        }

        let mut expected_cut = Vec::new();
        let mut expected_cut_capacity = 0_u64;
        let mut sorted_edges = self.edges.iter().collect::<Vec<_>>();
        sorted_edges.sort_by(|a, b| a.id.cmp(&b.id));
        for edge in sorted_edges {
            if source_side.contains(edge.from.as_str())
                && !source_side.contains(edge.to.as_str())
            {
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
            return Err(FlowError::new("minimum-cut edge set does not match source partition"));
        }
        if result.min_cut.capacity != expected_cut_capacity
            || result.min_cut.capacity != result.max_flow
        {
            return Err(FlowError::new("minimum-cut capacity does not equal maximum flow"));
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
        let mut source_capacity = 0_u64;
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
            if edge.from == self.source {
                source_capacity = source_capacity
                    .checked_add(edge.capacity)
                    .ok_or_else(|| FlowError::new("sum of source capacities overflowed u64"))?;
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
            nodes: node_rows,
            edges: edge_rows,
            source: "n0".to_string(),
            sink: format!("n{}", nodes - 1),
        }
    }

    /// Independent exhaustive s/t cut enumerator; deliberately does not use
    /// the residual network or Dinic implementation.
    fn exhaustive_min_cut(graph: &FlowGraph) -> u64 {
        let node_ids = graph.nodes.iter().map(|node| node.id.as_str()).collect::<Vec<_>>();
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
    fn parallel_antiparallel_cycles_and_zero_capacity_edges_are_supported() {
        let candidate = FlowGraph {
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
    }

    #[test]
    fn malformed_graphs_and_integer_overflow_are_rejected() {
        let mut candidate = graph(2, &[]);
        candidate.source = "n1".to_string();
        candidate.sink = "n1".to_string();
        assert!(candidate.solve().is_err());

        let mut candidate = graph(2, &[]);
        candidate.edges = vec![
            FlowEdge::new("a", "n0", "n1", u64::MAX, None),
            FlowEdge::new("b", "n0", "n1", 1, None),
        ];
        assert!(candidate.solve().is_err());

        let mut candidate = graph(3, &[(0, 1, 1)]);
        candidate.edges.push(FlowEdge::new("dangling", "n1", "missing", 1, None));
        assert!(candidate.solve().is_err());
    }

    #[test]
    fn known_answer_returns_constructive_flow_and_cut() {
        let candidate = graph(4, &[(0, 1, 3), (0, 2, 2), (1, 3, 2), (2, 3, 2), (1, 2, 1)]);
        let result = candidate.solve().unwrap();
        assert_eq!(result.max_flow, 4);
        assert_eq!(result.min_cut.capacity, 4);
        assert_eq!(result.edge_flows.iter().map(|edge| edge.flow).sum::<u64>(), 8);
        candidate.verify_result(&result).unwrap();
    }
}
