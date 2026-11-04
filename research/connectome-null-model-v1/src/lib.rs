// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reproducible directed connectome null-model generation.
//!
//! V1 performs cell-type- and edge-class-constrained target-endpoint swaps.
//! It preserves exact directed degrees, type-block counts, source-side weight
//! distributions and per-source outgoing strength. It does not preserve
//! target-side incoming strength, spatial embedding or higher-order motifs.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const MAX_NODES: usize = 200_000;
const MAX_EDGES: usize = 5_000_000;
const MAX_SWAP_ATTEMPTS: u64 = 100_000_000;
const MAX_ACCEPTED_SWAPS: u64 = 10_000_000;

/// A neuron or other explicitly declared connectome model unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectomeNode {
    /// Stable source-data identity; never substitute a positional array index.
    pub id: String,
    /// Source-grounded cell-type identity.
    pub cell_type: String,
}

/// One aggregated directed connection between two model units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectomeEdge {
    /// Presynaptic/source node identity.
    pub source: String,
    /// Postsynaptic/target node identity.
    pub target: String,
    /// Declared edge class, such as a transmitter/sign class.
    pub edge_class: String,
    /// Positive integer weight in the graph's declared weight unit.
    pub weight_units: u64,
}

/// A directed simple graph with an explicit, externally pinned weight scale.
///
/// V1 expects one row per ordered source-target pair. Synapses must already
/// have been aggregated by a source-pinned adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectomeGraph {
    /// Stable identity for the source graph/materialization.
    pub graph_id: String,
    /// Exact unit or quantization-scale identity for weight_units.
    pub weight_unit_id: String,
    /// Nodes in stable adapter order.
    pub nodes: Vec<ConnectomeNode>,
    /// Directed aggregated connections in stable adapter order.
    pub edges: Vec<ConnectomeEdge>,
}

/// Deterministic randomization settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NullModelConfig {
    /// SplitMix64 seed. This is reproducibility metadata, not cryptographic RNG.
    pub seed: u64,
    /// Number of accepted topology switches requested.
    pub requested_accepted_swaps: u64,
    /// Maximum number of pair proposals, including rejected proposals.
    pub attempt_budget: u64,
}

/// Counts why proposed edge-pair switches were held.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SwapRejections {
    /// The two selected edges had the same source node.
    pub same_source: u64,
    /// The two selected edges had the same target node.
    pub same_target: u64,
    /// A proposed target exchange would introduce a self-loop.
    pub self_loop: u64,
    /// A proposed target exchange would introduce an existing directed edge.
    pub duplicate_edge: u64,
}

/// Declared null-family constraints for a rewired graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullModelFamily {
    /// Preserve directed degree and source-attached edge weights, but allow
    /// cell-type and edge-class mixing to change.
    DirectedDegreeOnly,
    /// Additionally preserve directed source-type × target-type × edge-class
    /// block counts and each block's weight multiset.
    CellTypeAndEdgeClassConstrained,
}

/// Explicit post-generation invariant checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvariantReport {
    /// Node IDs and their cell-type assignments are unchanged.
    pub node_identity_and_type_preserved: bool,
    /// Every node's directed in-degree and out-degree are unchanged.
    pub degree_sequences_preserved: bool,
    /// Directed source-type × target-type × edge-class counts are unchanged.
    pub cell_type_blocks_preserved: bool,
    /// Whether cell-type block preservation was required by this null family.
    pub cell_type_blocks_required: bool,
    /// Per-source weight multisets and per-source outgoing strengths are unchanged.
    pub source_weight_constraints_preserved: bool,
    /// Weight multisets within cell-type/edge-class blocks are unchanged.
    pub block_weight_distribution_preserved: bool,
    /// Whether block weight-distribution preservation was required by this family.
    pub block_weight_distribution_required: bool,
    /// The output remains a simple directed graph without self-loops.
    pub simple_graph_constraints_preserved: bool,
}

impl InvariantReport {
    /// True only when the base constraints and every family-required invariant passed.
    #[must_use]
    pub const fn all_passed(self) -> bool {
        self.node_identity_and_type_preserved
            && self.degree_sequences_preserved
            && (!self.cell_type_blocks_required || self.cell_type_blocks_preserved)
            && self.source_weight_constraints_preserved
            && (!self.block_weight_distribution_required
                || self.block_weight_distribution_preserved)
            && self.simple_graph_constraints_preserved
    }
}

/// Receipt for one generated null graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NullModelReceipt {
    /// Exact input graph identity.
    pub graph_id: String,
    /// Null family whose constraint set was applied.
    pub family: NullModelFamily,
    /// Seed used by the local deterministic generator.
    pub seed: u64,
    /// Requested accepted swaps.
    pub requested_accepted_swaps: u64,
    /// Accepted swaps performed.
    pub accepted_swaps: u64,
    /// Pair proposals attempted, including held proposals.
    pub attempted_proposals: u64,
    /// Number of eligible edge pairs in the fixed type/edge-class blocks.
    pub eligible_pair_count: u64,
    /// Per-reason rejected proposal counts.
    pub rejections: SwapRejections,
    /// Number of output directed connections absent from the input graph.
    pub changed_connections: usize,
    /// Accepted proposals per million attempted proposals.
    pub acceptance_rate_per_million: u64,
    /// Post-generation checks. A returned sample requires all checks to pass.
    pub invariants: InvariantReport,
    /// Always false in V1: finite-chain mixing is not certified here.
    pub chain_mixing_certified: bool,
}

/// One rewired graph and the receipt that qualifies its declared invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NullModelSample {
    /// Generated graph; node identities and original edge order are retained.
    pub graph: ConnectomeGraph,
    /// Auditable randomization and invariant receipt.
    pub receipt: NullModelReceipt,
}

/// Validation, resource and randomization failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NullModelError {
    /// A graph has no nodes or no connections.
    EmptyGraph,
    /// A graph or randomization request exceeds the explicit resource ceiling.
    ResourceLimitExceeded,
    /// An identifier is empty, too long or contains unsupported characters.
    InvalidIdentifier {
        /// Field that failed identifier validation.
        field: &'static str,
    },
    /// A node identity occurs more than once.
    DuplicateNode(String),
    /// An edge references a node absent from the node table.
    UnknownEndpoint(String),
    /// A directed edge connects a node to itself.
    SelfLoop(String),
    /// Two edge rows describe the same ordered source-target pair.
    DuplicateEdge {
        /// Duplicate source identity.
        source: String,
        /// Duplicate target identity.
        target: String,
    },
    /// A connection has zero aggregate weight.
    ZeroWeight,
    /// Requested swap count or attempt budget is invalid.
    InvalidConfig,
    /// No within-block edge pair exists to propose a constrained switch.
    NoEligibleEdgePairs,
    /// The attempt budget ended before enough accepted switches were found.
    SwapBudgetExhausted {
        /// Requested number of accepted switches.
        requested: u64,
        /// Number actually accepted.
        accepted: u64,
        /// Number of proposals attempted.
        attempted: u64,
        /// Rejection diagnostics at exhaustion.
        rejections: SwapRejections,
    },
    /// An output invariant failed. No sample is returned.
    InvariantViolation,
}

impl fmt::Display for NullModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGraph => write!(formatter, "connectome graph is empty"),
            Self::ResourceLimitExceeded => {
                write!(formatter, "connectome null-model resource limit exceeded")
            }
            Self::InvalidIdentifier { field } => {
                write!(formatter, "invalid identifier in field {field}")
            }
            Self::DuplicateNode(id) => write!(formatter, "duplicate connectome node {id}"),
            Self::UnknownEndpoint(id) => write!(formatter, "edge references unknown node {id}"),
            Self::SelfLoop(id) => write!(formatter, "self-loop at node {id} is not allowed"),
            Self::DuplicateEdge { source, target } => {
                write!(formatter, "duplicate directed edge {source} -> {target}")
            }
            Self::ZeroWeight => write!(formatter, "edge weight must be positive"),
            Self::InvalidConfig => write!(formatter, "invalid null-model swap configuration"),
            Self::NoEligibleEdgePairs => {
                write!(formatter, "no edge pairs share a swappable cell-type block")
            }
            Self::SwapBudgetExhausted {
                requested,
                accepted,
                attempted,
                ..
            } => write!(
                formatter,
                "swap budget exhausted: accepted {accepted}/{requested} after {attempted} proposals"
            ),
            Self::InvariantViolation => {
                write!(formatter, "generated graph failed an invariant check")
            }
        }
    }
}

impl std::error::Error for NullModelError {}

type BlockKey = (String, String, String);
type BlockWeightKey = (String, String, String, u64);

#[derive(Debug, Clone, PartialEq, Eq)]
struct GraphInvariants {
    node_identity_and_type: Vec<(String, String)>,
    in_degree: BTreeMap<String, u64>,
    out_degree: BTreeMap<String, u64>,
    block_counts: BTreeMap<BlockKey, u64>,
    block_weight_counts: BTreeMap<BlockWeightKey, u64>,
    source_weight_counts: BTreeMap<(String, u64), u64>,
    source_strength: BTreeMap<String, u128>,
}

#[derive(Debug, Clone)]
struct EdgeBucket {
    edge_indices: Vec<usize>,
    pair_count: u64,
}

#[derive(Debug, Clone, Copy)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn below(&mut self, upper_exclusive: u64) -> u64 {
        let threshold = upper_exclusive.wrapping_neg() % upper_exclusive;
        loop {
            let value = self.next_u64();
            if value >= threshold {
                return value % upper_exclusive;
            }
        }
    }
}

/// Generates one constrained rewiring sample or returns an explicit failure.
///
/// Candidate pairs are selected by a fixed, seed-reproducible rule over eligible
/// source-type/target-type/edge-class blocks. An invalid proposal is a hold.
/// V1 makes no stationary-distribution, irreducibility or finite-run mixing
/// claim; callers must qualify the ensemble before statistical use.
/// Generates a degree-only directed switch null.
pub fn generate_degree_only_null_model(
    input: &ConnectomeGraph,
    config: NullModelConfig,
) -> Result<NullModelSample, NullModelError> {
    generate_null_model_for_family(input, config, NullModelFamily::DirectedDegreeOnly)
}

/// Generates a cell-type- and edge-class-constrained directed switch null.
pub fn generate_null_model(
    input: &ConnectomeGraph,
    config: NullModelConfig,
) -> Result<NullModelSample, NullModelError> {
    generate_null_model_for_family(
        input,
        config,
        NullModelFamily::CellTypeAndEdgeClassConstrained,
    )
}

fn generate_null_model_for_family(
    input: &ConnectomeGraph,
    config: NullModelConfig,
    family: NullModelFamily,
) -> Result<NullModelSample, NullModelError> {
    let input_invariants = validate_graph(input)?;
    validate_config(input, config)?;

    let node_indices: BTreeMap<String, usize> = input
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), index))
        .collect();
    let node_types: BTreeMap<String, String> = input
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node.cell_type.clone()))
        .collect();

    let mut edge_indices_by_block: BTreeMap<BlockKey, Vec<usize>> = BTreeMap::new();
    let mut source_indices = Vec::with_capacity(input.edges.len());
    let mut target_indices = Vec::with_capacity(input.edges.len());
    let mut adjacency = BTreeSet::new();

    for (edge_index, edge) in input.edges.iter().enumerate() {
        let source_index = *node_indices
            .get(&edge.source)
            .ok_or_else(|| NullModelError::UnknownEndpoint(edge.source.clone()))?;
        let target_index = *node_indices
            .get(&edge.target)
            .ok_or_else(|| NullModelError::UnknownEndpoint(edge.target.clone()))?;
        source_indices.push(source_index);
        target_indices.push(target_index);
        adjacency.insert((source_index, target_index));

        if family == NullModelFamily::CellTypeAndEdgeClassConstrained {
            let source_type = node_types
                .get(&edge.source)
                .ok_or_else(|| NullModelError::UnknownEndpoint(edge.source.clone()))?;
            let target_type = node_types
                .get(&edge.target)
                .ok_or_else(|| NullModelError::UnknownEndpoint(edge.target.clone()))?;
            let key = (
                source_type.clone(),
                target_type.clone(),
                edge.edge_class.clone(),
            );
            edge_indices_by_block
                .entry(key)
                .or_default()
                .push(edge_index);
        }
    }

    if family == NullModelFamily::DirectedDegreeOnly {
        edge_indices_by_block.insert(
            ("__all_sources__".to_string(), "__all_targets__".to_string(), "__all_edges__".to_string()),
            (0..input.edges.len()).collect(),
        );
    }

    let mut buckets = Vec::new();
    let mut eligible_pair_count = 0_u64;
    for edge_indices in edge_indices_by_block.into_values() {
        if edge_indices.len() < 2 {
            continue;
        }
        let edge_count =
            u64::try_from(edge_indices.len()).map_err(|_| NullModelError::ResourceLimitExceeded)?;
        let pair_count = edge_count
            .checked_mul(edge_count - 1)
            .and_then(|value| value.checked_div(2))
            .ok_or(NullModelError::ResourceLimitExceeded)?;
        eligible_pair_count = eligible_pair_count
            .checked_add(pair_count)
            .ok_or(NullModelError::ResourceLimitExceeded)?;
        buckets.push(EdgeBucket {
            edge_indices,
            pair_count,
        });
    }
    if eligible_pair_count == 0 {
        return Err(NullModelError::NoEligibleEdgePairs);
    }

    let mut output = input.clone();
    let mut rng = SplitMix64::new(config.seed);
    let mut attempted_proposals = 0_u64;
    let mut accepted_swaps = 0_u64;
    let mut rejections = SwapRejections::default();

    while accepted_swaps < config.requested_accepted_swaps
        && attempted_proposals < config.attempt_budget
    {
        attempted_proposals += 1;
        let ticket = rng.below(eligible_pair_count);
        let bucket = select_bucket(&buckets, ticket)?;
        let (first_edge, second_edge) = choose_two(&bucket.edge_indices, &mut rng)?;

        let first_source = source_indices[first_edge];
        let second_source = source_indices[second_edge];
        if first_source == second_source {
            rejections.same_source += 1;
            continue;
        }

        let first_target = target_indices[first_edge];
        let second_target = target_indices[second_edge];
        if first_target == second_target {
            rejections.same_target += 1;
            continue;
        }

        let new_first = (first_source, second_target);
        let new_second = (second_source, first_target);
        if first_source == second_target || second_source == first_target {
            rejections.self_loop += 1;
            continue;
        }
        if adjacency.contains(&new_first) || adjacency.contains(&new_second) {
            rejections.duplicate_edge += 1;
            continue;
        }

        let old_first = (first_source, first_target);
        let old_second = (second_source, second_target);
        if !adjacency.remove(&old_first) || !adjacency.remove(&old_second) {
            return Err(NullModelError::InvariantViolation);
        }
        if !adjacency.insert(new_first) || !adjacency.insert(new_second) {
            return Err(NullModelError::InvariantViolation);
        }

        target_indices[first_edge] = second_target;
        target_indices[second_edge] = first_target;
        output.edges[first_edge].target = input.nodes[second_target].id.clone();
        output.edges[second_edge].target = input.nodes[first_target].id.clone();
        accepted_swaps += 1;
    }

    if accepted_swaps != config.requested_accepted_swaps {
        return Err(NullModelError::SwapBudgetExhausted {
            requested: config.requested_accepted_swaps,
            accepted: accepted_swaps,
            attempted: attempted_proposals,
            rejections,
        });
    }

    let output_invariants = validate_graph(&output)?;
    let invariant_report = compare_invariants(&input_invariants, &output_invariants, family);
    if !invariant_report.all_passed() {
        return Err(NullModelError::InvariantViolation);
    }

    let original_edges: BTreeSet<(String, String)> = input
        .edges
        .iter()
        .map(|edge| (edge.source.clone(), edge.target.clone()))
        .collect();
    let changed_connections = output
        .edges
        .iter()
        .filter(|edge| !original_edges.contains(&(edge.source.clone(), edge.target.clone())))
        .count();
    let acceptance_rate_per_million = u64::try_from(
        (u128::from(accepted_swaps) * 1_000_000_u128) / u128::from(attempted_proposals),
    )
    .map_err(|_| NullModelError::ResourceLimitExceeded)?;

    let receipt = NullModelReceipt {
        graph_id: input.graph_id.clone(),
        family,
        seed: config.seed,
        requested_accepted_swaps: config.requested_accepted_swaps,
        accepted_swaps,
        attempted_proposals,
        eligible_pair_count,
        rejections,
        changed_connections,
        acceptance_rate_per_million,
        invariants: invariant_report,
        chain_mixing_certified: false,
    };

    Ok(NullModelSample {
        graph: output,
        receipt,
    })
}

fn validate_config(input: &ConnectomeGraph, config: NullModelConfig) -> Result<(), NullModelError> {
    if config.requested_accepted_swaps == 0
        || config.requested_accepted_swaps > MAX_ACCEPTED_SWAPS
        || config.attempt_budget < config.requested_accepted_swaps
        || config.attempt_budget > MAX_SWAP_ATTEMPTS
    {
        return Err(NullModelError::InvalidConfig);
    }
    if input.edges.len() > MAX_EDGES || input.nodes.len() > MAX_NODES {
        return Err(NullModelError::ResourceLimitExceeded);
    }
    Ok(())
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

fn validate_graph(graph: &ConnectomeGraph) -> Result<GraphInvariants, NullModelError> {
    if graph.nodes.is_empty() || graph.edges.is_empty() {
        return Err(NullModelError::EmptyGraph);
    }
    if graph.nodes.len() > MAX_NODES || graph.edges.len() > MAX_EDGES {
        return Err(NullModelError::ResourceLimitExceeded);
    }
    if !valid_identifier(&graph.graph_id) {
        return Err(NullModelError::InvalidIdentifier { field: "graph_id" });
    }
    if !valid_identifier(&graph.weight_unit_id) {
        return Err(NullModelError::InvalidIdentifier {
            field: "weight_unit_id",
        });
    }

    let mut node_types = BTreeMap::new();
    for node in &graph.nodes {
        if !valid_identifier(&node.id) {
            return Err(NullModelError::InvalidIdentifier { field: "node_id" });
        }
        if !valid_identifier(&node.cell_type) {
            return Err(NullModelError::InvalidIdentifier { field: "cell_type" });
        }
        if node_types
            .insert(node.id.clone(), node.cell_type.clone())
            .is_some()
        {
            return Err(NullModelError::DuplicateNode(node.id.clone()));
        }
    }

    let mut seen_edges = BTreeSet::new();
    for edge in &graph.edges {
        if !valid_identifier(&edge.edge_class) {
            return Err(NullModelError::InvalidIdentifier {
                field: "edge_class",
            });
        }
        if !node_types.contains_key(&edge.source) {
            return Err(NullModelError::UnknownEndpoint(edge.source.clone()));
        }
        if !node_types.contains_key(&edge.target) {
            return Err(NullModelError::UnknownEndpoint(edge.target.clone()));
        }
        if edge.source == edge.target {
            return Err(NullModelError::SelfLoop(edge.source.clone()));
        }
        if edge.weight_units == 0 {
            return Err(NullModelError::ZeroWeight);
        }
        if !seen_edges.insert((edge.source.clone(), edge.target.clone())) {
            return Err(NullModelError::DuplicateEdge {
                source: edge.source.clone(),
                target: edge.target.clone(),
            });
        }
    }

    build_invariants(graph, &node_types)
}

fn build_invariants(
    graph: &ConnectomeGraph,
    node_types: &BTreeMap<String, String>,
) -> Result<GraphInvariants, NullModelError> {
    let mut in_degree = BTreeMap::new();
    let mut out_degree = BTreeMap::new();
    let mut source_strength = BTreeMap::new();
    for node in &graph.nodes {
        in_degree.insert(node.id.clone(), 0_u64);
        out_degree.insert(node.id.clone(), 0_u64);
        source_strength.insert(node.id.clone(), 0_u128);
    }

    let mut block_counts = BTreeMap::new();
    let mut block_weight_counts = BTreeMap::new();
    let mut source_weight_counts = BTreeMap::new();

    for edge in &graph.edges {
        *out_degree.entry(edge.source.clone()).or_insert(0) += 1;
        *in_degree.entry(edge.target.clone()).or_insert(0) += 1;
        *source_strength.entry(edge.source.clone()).or_insert(0) += u128::from(edge.weight_units);
        *source_weight_counts
            .entry((edge.source.clone(), edge.weight_units))
            .or_insert(0) += 1;

        let source_type = node_types
            .get(&edge.source)
            .ok_or_else(|| NullModelError::UnknownEndpoint(edge.source.clone()))?;
        let target_type = node_types
            .get(&edge.target)
            .ok_or_else(|| NullModelError::UnknownEndpoint(edge.target.clone()))?;
        let block = (
            source_type.clone(),
            target_type.clone(),
            edge.edge_class.clone(),
        );
        *block_counts.entry(block.clone()).or_insert(0) += 1;
        *block_weight_counts
            .entry((block.0, block.1, block.2, edge.weight_units))
            .or_insert(0) += 1;
    }

    Ok(GraphInvariants {
        node_identity_and_type: graph
            .nodes
            .iter()
            .map(|node| (node.id.clone(), node.cell_type.clone()))
            .collect(),
        in_degree,
        out_degree,
        block_counts,
        block_weight_counts,
        source_weight_counts,
        source_strength,
    })
}

fn compare_invariants(
    input: &GraphInvariants,
    output: &GraphInvariants,
    family: NullModelFamily,
) -> InvariantReport {
    let constrained = family == NullModelFamily::CellTypeAndEdgeClassConstrained;
    InvariantReport {
        node_identity_and_type_preserved: input.node_identity_and_type
            == output.node_identity_and_type,
        degree_sequences_preserved: input.in_degree == output.in_degree
            && input.out_degree == output.out_degree,
        cell_type_blocks_preserved: input.block_counts == output.block_counts,
        cell_type_blocks_required: constrained,
        source_weight_constraints_preserved: input.source_weight_counts
            == output.source_weight_counts
            && input.source_strength == output.source_strength,
        block_weight_distribution_preserved: input.block_weight_counts
            == output.block_weight_counts,
        block_weight_distribution_required: constrained,
        simple_graph_constraints_preserved: true,
    }
}

fn select_bucket(buckets: &[EdgeBucket], mut ticket: u64) -> Result<&EdgeBucket, NullModelError> {
    for bucket in buckets {
        if ticket < bucket.pair_count {
            return Ok(bucket);
        }
        ticket -= bucket.pair_count;
    }
    Err(NullModelError::InvariantViolation)
}

fn choose_two(
    edge_indices: &[usize],
    rng: &mut SplitMix64,
) -> Result<(usize, usize), NullModelError> {
    if edge_indices.len() < 2 {
        return Err(NullModelError::NoEligibleEdgePairs);
    }
    let length =
        u64::try_from(edge_indices.len()).map_err(|_| NullModelError::ResourceLimitExceeded)?;
    let first = rng.below(length) as usize;
    let mut second = rng.below(length - 1) as usize;
    if second >= first {
        second += 1;
    }
    Ok((edge_indices[first], edge_indices[second]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_graph() -> ConnectomeGraph {
        let nodes = ["m1", "m2", "m3", "m4", "i1", "i2", "i3", "i4"]
            .into_iter()
            .map(|id| ConnectomeNode {
                id: id.to_string(),
                cell_type: if id.starts_with('m') {
                    "motor".to_string()
                } else {
                    "interneuron".to_string()
                },
            })
            .collect();
        let edges = [
            ("m1", "i1", 3),
            ("m2", "i2", 5),
            ("m3", "i3", 7),
            ("m4", "i4", 11),
        ]
        .into_iter()
        .map(|(source, target, weight_units)| ConnectomeEdge {
            source: source.to_string(),
            target: target.to_string(),
            edge_class: "excitatory".to_string(),
            weight_units,
        })
        .collect();
        ConnectomeGraph {
            graph_id: "fixture-v1".to_string(),
            weight_unit_id: "synapse-count".to_string(),
            nodes,
            edges,
        }
    }

    fn config(seed: u64, swaps: u64, attempts: u64) -> NullModelConfig {
        NullModelConfig {
            seed,
            requested_accepted_swaps: swaps,
            attempt_budget: attempts,
        }
    }

    #[test]
    fn one_switch_changes_topology_and_preserves_all_declared_invariants() {
        let input = fixture_graph();
        let sample = generate_null_model(&input, config(7, 1, 1)).expect("one valid switch");
        assert_eq!(sample.receipt.accepted_swaps, 1);
        assert_eq!(sample.receipt.attempted_proposals, 1);
        assert!(sample.receipt.changed_connections >= 2);
        assert!(sample.receipt.invariants.all_passed());
        assert!(!sample.receipt.chain_mixing_certified);
        assert_eq!(sample.graph.nodes, input.nodes);
    }

    #[test]
    fn deterministic_seed_reproduces_graph_and_receipt() {
        let input = fixture_graph();
        let first = generate_null_model(&input, config(42, 50, 500)).expect("first sample");
        let second = generate_null_model(&input, config(42, 50, 500)).expect("second sample");
        assert_eq!(first, second);
    }

    #[test]
    fn a_different_seed_is_allowed_to_generate_a_different_graph() {
        let input = fixture_graph();
        let first = generate_null_model(&input, config(11, 1, 1)).expect("first sample");
        let second = generate_null_model(&input, config(12, 1, 1)).expect("second sample");
        assert_ne!(first.graph, second.graph);
    }

    #[test]
    fn exact_per_node_degree_and_source_weight_invariants_are_verified() {
        let input = fixture_graph();
        let original = validate_graph(&input).expect("valid input");
        let sample = generate_null_model(&input, config(99, 25, 500)).expect("sample");
        let rewired = validate_graph(&sample.graph).expect("valid output");
        assert_eq!(original.in_degree, rewired.in_degree);
        assert_eq!(original.out_degree, rewired.out_degree);
        assert_eq!(original.source_strength, rewired.source_strength);
        assert_eq!(original.source_weight_counts, rewired.source_weight_counts);
        assert_eq!(original.block_weight_counts, rewired.block_weight_counts);
    }

    #[test]
    fn degree_only_null_preserves_degree_but_can_change_cell_type_mixing() {
        let input = ConnectomeGraph {
            graph_id: "degree-only-fixture".to_string(),
            weight_unit_id: "synapse-count".to_string(),
            nodes: vec![
                ConnectomeNode {
                    id: "a".to_string(),
                    cell_type: "type-a".to_string(),
                },
                ConnectomeNode {
                    id: "b".to_string(),
                    cell_type: "type-b".to_string(),
                },
                ConnectomeNode {
                    id: "c".to_string(),
                    cell_type: "type-b".to_string(),
                },
                ConnectomeNode {
                    id: "d".to_string(),
                    cell_type: "type-a".to_string(),
                },
            ],
            edges: vec![
                ConnectomeEdge {
                    source: "a".to_string(),
                    target: "b".to_string(),
                    edge_class: "excitatory".to_string(),
                    weight_units: 2,
                },
                ConnectomeEdge {
                    source: "c".to_string(),
                    target: "d".to_string(),
                    edge_class: "inhibitory".to_string(),
                    weight_units: 5,
                },
            ],
        };

        let sample =
            generate_degree_only_null_model(&input, config(5, 1, 1)).expect("degree-only switch");
        assert_eq!(sample.receipt.family, NullModelFamily::DirectedDegreeOnly);
        assert!(sample.receipt.invariants.all_passed());
        assert!(sample.receipt.invariants.degree_sequences_preserved);
        assert!(sample.receipt.invariants.source_weight_constraints_preserved);
        assert!(!sample.receipt.invariants.cell_type_blocks_required);
        assert!(!sample.receipt.invariants.cell_type_blocks_preserved);
        assert!(!sample.receipt.invariants.block_weight_distribution_required);
        assert!(!sample.receipt.invariants.block_weight_distribution_preserved);
        assert_eq!(
            sample.graph.edges.iter().map(|edge| (edge.source.as_str(), edge.target.as_str())).collect::<BTreeSet<_>>(),
            BTreeSet::from([("a", "d"), ("c", "b")])
        );
    }

    #[test]
    fn cell_type_constrained_family_abstains_when_only_cross_block_pairs_exist() {
        let input = ConnectomeGraph {
            graph_id: "typed-degree-only-fixture".to_string(),
            weight_unit_id: "synapse-count".to_string(),
            nodes: vec![
                ConnectomeNode {
                    id: "a".to_string(),
                    cell_type: "type-a".to_string(),
                },
                ConnectomeNode {
                    id: "b".to_string(),
                    cell_type: "type-b".to_string(),
                },
                ConnectomeNode {
                    id: "c".to_string(),
                    cell_type: "type-b".to_string(),
                },
                ConnectomeNode {
                    id: "d".to_string(),
                    cell_type: "type-a".to_string(),
                },
            ],
            edges: vec![
                ConnectomeEdge {
                    source: "a".to_string(),
                    target: "b".to_string(),
                    edge_class: "excitatory".to_string(),
                    weight_units: 2,
                },
                ConnectomeEdge {
                    source: "c".to_string(),
                    target: "d".to_string(),
                    edge_class: "inhibitory".to_string(),
                    weight_units: 5,
                },
            ],
        };
        assert_eq!(
            generate_null_model(&input, config(5, 1, 10)),
            Err(NullModelError::NoEligibleEdgePairs)
        );
    }

    #[test]
    fn incompatible_cell_type_or_edge_class_pairs_are_not_swapped() {
        let mut input = fixture_graph();
        for (index, edge) in input.edges.iter_mut().enumerate() {
            edge.edge_class = format!("class-{index}");
        }
        assert_eq!(
            generate_null_model(&input, config(3, 1, 100)),
            Err(NullModelError::NoEligibleEdgePairs)
        );
    }

    #[test]
    fn same_source_pairs_are_held_not_mistaken_for_topology_changes() {
        let mut input = fixture_graph();
        input.edges = vec![
            ConnectomeEdge {
                source: "m1".to_string(),
                target: "i1".to_string(),
                edge_class: "excitatory".to_string(),
                weight_units: 2,
            },
            ConnectomeEdge {
                source: "m1".to_string(),
                target: "i2".to_string(),
                edge_class: "excitatory".to_string(),
                weight_units: 3,
            },
        ];
        let result = generate_null_model(&input, config(1, 1, 10));
        assert!(matches!(
            result,
            Err(NullModelError::SwapBudgetExhausted {
                accepted: 0,
                attempted: 10,
                ..
            })
        ));
    }

    #[test]
    fn self_loop_proposals_are_rejected() {
        let input = ConnectomeGraph {
            graph_id: "reciprocal".to_string(),
            weight_unit_id: "count".to_string(),
            nodes: vec![
                ConnectomeNode {
                    id: "a".to_string(),
                    cell_type: "same".to_string(),
                },
                ConnectomeNode {
                    id: "b".to_string(),
                    cell_type: "same".to_string(),
                },
            ],
            edges: vec![
                ConnectomeEdge {
                    source: "a".to_string(),
                    target: "b".to_string(),
                    edge_class: "same".to_string(),
                    weight_units: 1,
                },
                ConnectomeEdge {
                    source: "b".to_string(),
                    target: "a".to_string(),
                    edge_class: "same".to_string(),
                    weight_units: 1,
                },
            ],
        };
        assert!(matches!(
            generate_null_model(&input, config(2, 1, 10)),
            Err(NullModelError::SwapBudgetExhausted {
                accepted: 0,
                rejections: SwapRejections { self_loop: 10, .. },
                ..
            })
        ));
    }

    #[test]
    fn duplicate_edge_proposals_are_rejected() {
        let mut input = fixture_graph();
        input.edges = vec![
            ConnectomeEdge {
                source: "m1".to_string(),
                target: "i1".to_string(),
                edge_class: "excitatory".to_string(),
                weight_units: 1,
            },
            ConnectomeEdge {
                source: "m2".to_string(),
                target: "i2".to_string(),
                edge_class: "excitatory".to_string(),
                weight_units: 2,
            },
            ConnectomeEdge {
                source: "m1".to_string(),
                target: "i2".to_string(),
                edge_class: "excitatory".to_string(),
                weight_units: 3,
            },
        ];
        assert!(matches!(
            generate_null_model(&input, config(1, 1, 1)),
            Err(NullModelError::SwapBudgetExhausted {
                accepted: 0,
                attempted: 1,
                rejections: SwapRejections {
                    duplicate_edge: 1,
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn rejects_duplicate_nodes_and_duplicate_directed_edges() {
        let mut input = fixture_graph();
        input.nodes.push(input.nodes[0].clone());
        assert!(matches!(
            validate_graph(&input),
            Err(NullModelError::DuplicateNode(_))
        ));

        let mut input = fixture_graph();
        input.edges.push(input.edges[0].clone());
        assert!(matches!(
            validate_graph(&input),
            Err(NullModelError::DuplicateEdge { .. })
        ));
    }

    #[test]
    fn rejects_unknown_endpoints_self_loops_and_zero_weights() {
        let mut input = fixture_graph();
        input.edges[0].target = "missing".to_string();
        assert_eq!(
            validate_graph(&input),
            Err(NullModelError::UnknownEndpoint("missing".to_string()))
        );

        let mut input = fixture_graph();
        input.edges[0].target = input.edges[0].source.clone();
        assert!(matches!(
            validate_graph(&input),
            Err(NullModelError::SelfLoop(_))
        ));

        let mut input = fixture_graph();
        input.edges[0].weight_units = 0;
        assert_eq!(validate_graph(&input), Err(NullModelError::ZeroWeight));
    }

    #[test]
    fn invalid_identifiers_and_empty_graphs_fail_closed() {
        let mut input = fixture_graph();
        input.nodes[0].id = "m 1".to_string();
        assert_eq!(
            validate_graph(&input),
            Err(NullModelError::InvalidIdentifier { field: "node_id" })
        );
        assert_eq!(
            validate_graph(&ConnectomeGraph {
                graph_id: "empty".to_string(),
                weight_unit_id: "count".to_string(),
                nodes: Vec::new(),
                edges: Vec::new(),
            }),
            Err(NullModelError::EmptyGraph)
        );
    }

    #[test]
    fn swap_request_and_attempt_budgets_are_bounded() {
        let input = fixture_graph();
        assert_eq!(
            generate_null_model(&input, config(1, 0, 10)),
            Err(NullModelError::InvalidConfig)
        );
        assert_eq!(
            generate_null_model(&input, config(1, 2, 1)),
            Err(NullModelError::InvalidConfig)
        );
        assert_eq!(
            generate_null_model(&input, config(1, 1, MAX_SWAP_ATTEMPTS + 1)),
            Err(NullModelError::InvalidConfig)
        );
    }

    #[test]
    fn preserves_edge_class_and_directional_cell_type_block_counts() {
        let input = fixture_graph();
        let sample = generate_null_model(&input, config(4, 15, 300)).expect("sample");
        assert!(sample.receipt.invariants.cell_type_blocks_preserved);
        let input_invariants = validate_graph(&input).expect("input");
        let output_invariants = validate_graph(&sample.graph).expect("output");
        assert_eq!(
            input_invariants.block_counts,
            output_invariants.block_counts
        );
    }
}
