// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Declared-topology resilience and failure-domain analysis.
//!
//! Edge-disjoint routes are graph-theoretic results only. They do not prove
//! physical route independence, adequate capacity, protection coordination,
//! source availability, or electrical safety.

use crate::{
    GraphEvidence, GraphEvidenceState, ServiceEdgeKind, ServiceGraph, ServiceNodeKind,
};
use serde::{Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureDomainKind {
    UpstreamSupply,
    Enclosure,
    CableRoute,
    Protection,
    Communications,
    Cooling,
    Authority,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureDomainInventoryStatus {
    CompleteWithinDeclaredScope,
    Incomplete,
    Unknown,
}

/// Components that may be affected by one declared common-cause event.
/// Members can be service-graph nodes and/or edges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FailureDomain {
    domain_id: String,
    kind: FailureDomainKind,
    node_ids: Vec<String>,
    edge_ids: Vec<String>,
    #[serde(serialize_with = "serialize_graph_evidence")]
    evidence: GraphEvidence,
}

impl FailureDomain {
    pub fn new(
        domain_id: impl Into<String>,
        kind: FailureDomainKind,
        mut node_ids: Vec<String>,
        mut edge_ids: Vec<String>,
        evidence: GraphEvidence,
    ) -> Result<Self, ResilienceError> {
        let domain_id = domain_id.into();
        require_non_empty("domain_id", &domain_id)?;
        for id in node_ids.iter().chain(edge_ids.iter()) {
            require_non_empty("failure_domain_member_id", id)?;
        }
        node_ids.sort();
        edge_ids.sort();
        if node_ids.windows(2).any(|pair| pair[0] == pair[1])
            || edge_ids.windows(2).any(|pair| pair[0] == pair[1])
        {
            return Err(ResilienceError::DuplicateFailureDomainMember {
                domain_id,
            });
        }
        if node_ids.is_empty() && edge_ids.is_empty() {
            return Err(ResilienceError::EmptyFailureDomain { domain_id });
        }
        Ok(Self { domain_id, kind, node_ids, edge_ids, evidence })
    }

    pub fn domain_id(&self) -> &str { &self.domain_id }
    pub const fn kind(&self) -> FailureDomainKind { self.kind }
    pub fn node_ids(&self) -> &[String] { &self.node_ids }
    pub fn edge_ids(&self) -> &[String] { &self.edge_ids }
    pub fn evidence(&self) -> &GraphEvidence { &self.evidence }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DisjointPath {
    node_ids: Vec<String>,
    edge_ids: Vec<String>,
}

impl DisjointPath {
    pub fn node_ids(&self) -> &[String] { &self.node_ids }
    pub fn edge_ids(&self) -> &[String] { &self.edge_ids }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FailureDomainExposure {
    domain_id: String,
    kind: FailureDomainKind,
    disjoint_paths_affected: usize,
    #[serde(serialize_with = "serialize_evidence_state")]
    evidence_state: GraphEvidenceState,
    evidence_id: Option<String>,
}

impl FailureDomainExposure {
    pub fn domain_id(&self) -> &str { &self.domain_id }
    pub const fn kind(&self) -> FailureDomainKind { self.kind }
    pub const fn disjoint_paths_affected(&self) -> usize { self.disjoint_paths_affected }
    pub const fn evidence_state(&self) -> GraphEvidenceState { self.evidence_state }
    pub fn evidence_id(&self) -> Option<&str> { self.evidence_id.as_deref() }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IndependenceAssessment {
    FewerThanTwoEdgeDisjointPaths,
    FailureDomainInventoryIncomplete,
    SharedModeledFailureDomainAcrossPaths,
    NoSharedModeledIntermediateDependencyDetected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResilienceAssessment {
    graph_id: String,
    revision_id: String,
    source_id: String,
    load_id: String,
    edge_disjoint_path_count: usize,
    edge_disjoint_paths: Vec<DisjointPath>,
    shared_source_terminal_across_paths: bool,
    shared_intermediate_node_ids: Vec<String>,
    undirected_power_path_bridge_edge_ids: Vec<String>,
    undirected_power_path_articulation_node_ids: Vec<String>,
    failure_domain_exposures: Vec<FailureDomainExposure>,
    inventory_status: FailureDomainInventoryStatus,
    independence_assessment: IndependenceAssessment,
}

impl ResilienceAssessment {
    pub fn graph_id(&self) -> &str { &self.graph_id }
    pub fn revision_id(&self) -> &str { &self.revision_id }
    pub fn source_id(&self) -> &str { &self.source_id }
    pub fn load_id(&self) -> &str { &self.load_id }
    pub const fn edge_disjoint_path_count(&self) -> usize { self.edge_disjoint_path_count }
    pub fn edge_disjoint_paths(&self) -> &[DisjointPath] { &self.edge_disjoint_paths }
    pub const fn shared_source_terminal_across_paths(&self) -> bool { self.shared_source_terminal_across_paths }
    pub fn shared_intermediate_node_ids(&self) -> &[String] { &self.shared_intermediate_node_ids }
    pub fn undirected_power_path_bridge_edge_ids(&self) -> &[String] { &self.undirected_power_path_bridge_edge_ids }
    pub fn undirected_power_path_articulation_node_ids(&self) -> &[String] { &self.undirected_power_path_articulation_node_ids }
    pub fn failure_domain_exposures(&self) -> &[FailureDomainExposure] { &self.failure_domain_exposures }
    pub const fn inventory_status(&self) -> FailureDomainInventoryStatus { self.inventory_status }
    pub const fn independence_assessment(&self) -> IndependenceAssessment { self.independence_assessment }
}

/// Analyze one declared source/load pair. Only directed PowerPath edges are
/// used for route enumeration and max-flow. Undirected bridges/articulations
/// are reported as structural cut candidates in the PowerPath projection.
pub fn assess_resilience(
    graph: &ServiceGraph,
    source_id: &str,
    load_id: &str,
    failure_domains: &[FailureDomain],
    inventory_status: FailureDomainInventoryStatus,
) -> Result<ResilienceAssessment, ResilienceError> {
    let source = graph.node(source_id).ok_or_else(|| ResilienceError::UnknownNode {
        node_id: source_id.to_owned(),
    })?;
    let load = graph.node(load_id).ok_or_else(|| ResilienceError::UnknownNode {
        node_id: load_id.to_owned(),
    })?;
    if !matches!(
        source.kind(),
        ServiceNodeKind::UtilitySource
            | ServiceNodeKind::RenewableSource
            | ServiceNodeKind::GeneratorSource
            | ServiceNodeKind::Storage
    ) {
        return Err(ResilienceError::NotPotentialSource { node_id: source_id.to_owned() });
    }
    if load.kind() != ServiceNodeKind::Load {
        return Err(ResilienceError::NotLoad { node_id: load_id.to_owned() });
    }

    let node_ids: Vec<String> = graph.nodes().iter().map(|node| node.node_id().to_owned()).collect();
    let node_index: BTreeMap<String, usize> = node_ids.iter().enumerate()
        .map(|(index, id)| (id.clone(), index)).collect();
    let power_edges: Vec<_> = graph.edges().iter()
        .filter(|edge| edge.kind() == ServiceEdgeKind::PowerPath)
        .collect();
    let edge_ids: BTreeSet<String> = graph.edges().iter().map(|edge| edge.edge_id().to_owned()).collect();
    let graph_node_ids: BTreeSet<String> = node_ids.iter().cloned().collect();

    let mut domain_ids = BTreeSet::new();
    for domain in failure_domains {
        if !domain_ids.insert(domain.domain_id.clone()) {
            return Err(ResilienceError::DuplicateFailureDomainId { domain_id: domain.domain_id.clone() });
        }
        for id in &domain.node_ids {
            if !graph_node_ids.contains(id) {
                return Err(ResilienceError::UnknownFailureDomainMember { domain_id: domain.domain_id.clone(), member_id: id.clone() });
            }
        }
        for id in &domain.edge_ids {
            if !edge_ids.contains(id) {
                return Err(ResilienceError::UnknownFailureDomainMember { domain_id: domain.domain_id.clone(), member_id: id.clone() });
            }
        }
    }

    let source_index = *node_index.get(source_id).expect("source checked above");
    let load_index = *node_index.get(load_id).expect("load checked above");
    let (edge_disjoint_path_count, edge_disjoint_paths) =
        max_edge_disjoint_paths(&node_ids, &node_index, &power_edges, source_index, load_index)?;

    let mut shared_intermediate_nodes = BTreeSet::new();
    if edge_disjoint_paths.len() >= 2 {
        for node_id in &node_ids {
            if node_id == source_id || node_id == load_id { continue; }
            let affected = edge_disjoint_paths.iter()
                .filter(|path| path.node_ids.iter().any(|path_node| path_node == node_id))
                .count();
            if affected >= 2 { shared_intermediate_nodes.insert(node_id.clone()); }
        }
    }

    let mut exposures = Vec::new();
    for domain in failure_domains {
        let affected = edge_disjoint_paths.iter().filter(|path| {
            path.node_ids.iter().any(|id| domain.node_ids.binary_search(id).is_ok())
                || path.edge_ids.iter().any(|id| domain.edge_ids.binary_search(id).is_ok())
        }).count();
        if affected > 0 {
            exposures.push(FailureDomainExposure {
                domain_id: domain.domain_id.clone(),
                kind: domain.kind,
                disjoint_paths_affected: affected,
                evidence_state: domain.evidence.state(),
                evidence_id: domain.evidence.evidence_id().map(str::to_owned),
            });
        }
    }
    exposures.sort_by(|left, right| left.domain_id.cmp(&right.domain_id));

    let unknown_domain_evidence = failure_domains.iter()
        .any(|domain| domain.evidence.state() == GraphEvidenceState::Unknown);
    let inventory_incomplete = inventory_status != FailureDomainInventoryStatus::CompleteWithinDeclaredScope
        || unknown_domain_evidence;
    let shared_domain = exposures.iter().any(|exposure| {
        exposure.disjoint_paths_affected >= 2
            && exposure.evidence_state != GraphEvidenceState::Unknown
    });
    let shared_intermediate_dependency = !shared_intermediate_nodes.is_empty();
    let independence_assessment = if edge_disjoint_path_count < 2 {
        IndependenceAssessment::FewerThanTwoEdgeDisjointPaths
    } else if shared_domain || shared_intermediate_dependency {
        IndependenceAssessment::SharedModeledFailureDomainAcrossPaths
    } else if inventory_incomplete {
        IndependenceAssessment::FailureDomainInventoryIncomplete
    } else {
        IndependenceAssessment::NoSharedModeledIntermediateDependencyDetected
    };

    let (bridges, articulations) = undirected_cut_candidates(&node_ids, &node_index, &power_edges);

    Ok(ResilienceAssessment {
        graph_id: graph.revision().graph_id().to_owned(),
        revision_id: graph.revision().revision_id().to_owned(),
        source_id: source_id.to_owned(),
        load_id: load_id.to_owned(),
        edge_disjoint_path_count,
        edge_disjoint_paths,
        shared_source_terminal_across_paths: edge_disjoint_path_count >= 2,
        shared_intermediate_node_ids: shared_intermediate_nodes.into_iter().collect(),
        undirected_power_path_bridge_edge_ids: bridges,
        undirected_power_path_articulation_node_ids: articulations,
        failure_domain_exposures: exposures,
        inventory_status,
        independence_assessment,
    })
}

#[derive(Debug, Clone)]
struct ResidualEdge {
    to: usize,
    reverse_index: usize,
    capacity: u8,
    original_edge_index: Option<usize>,
}

fn max_edge_disjoint_paths(
    node_ids: &[String],
    node_index: &BTreeMap<String, usize>,
    power_edges: &[&crate::ServiceEdge],
    source: usize,
    sink: usize,
) -> Result<(usize, Vec<DisjointPath>), ResilienceError> {
    let mut residual: Vec<Vec<ResidualEdge>> = vec![Vec::new(); node_ids.len()];
    let mut originals = Vec::new();

    for (edge_index, edge) in power_edges.iter().enumerate() {
        let from = *node_index.get(edge.from_node()).expect("validated service graph endpoint");
        let to = *node_index.get(edge.to_node()).expect("validated service graph endpoint");
        let forward_reverse = residual[to].len();
        let reverse_reverse = residual[from].len();
        residual[from].push(ResidualEdge {
            to,
            reverse_index: forward_reverse,
            capacity: 1,
            original_edge_index: Some(edge_index),
        });
        residual[to].push(ResidualEdge {
            to: from,
            reverse_index: reverse_reverse,
            capacity: 0,
            original_edge_index: None,
        });
        originals.push((from, to, edge.edge_id().to_owned()));
    }

    let mut flow = 0usize;
    loop {
        let mut predecessor: Vec<Option<(usize, usize)>> = vec![None; node_ids.len()];
        let mut visited = vec![false; node_ids.len()];
        let mut queue = VecDeque::new();
        visited[source] = true;
        queue.push_back(source);

        while let Some(current) = queue.pop_front() {
            if current == sink { break; }
            for (edge_position, edge) in residual[current].iter().enumerate() {
                if edge.capacity > 0 && !visited[edge.to] {
                    visited[edge.to] = true;
                    predecessor[edge.to] = Some((current, edge_position));
                    queue.push_back(edge.to);
                }
            }
        }
        if !visited[sink] { break; }

        let mut cursor = sink;
        while cursor != source {
            let (prior, edge_position) = predecessor[cursor]
                .ok_or(ResilienceError::ResidualPathReconstructionFailed)?;
            let reverse = residual[prior][edge_position].reverse_index;
            residual[prior][edge_position].capacity -= 1;
            residual[cursor][reverse].capacity += 1;
            cursor = prior;
        }
        flow += 1;
    }

    let mut outgoing: Vec<Vec<(usize, usize, String)>> = vec![Vec::new(); node_ids.len()];
    for (from, edges) in residual.iter().enumerate() {
        for edge in edges {
            if let Some(original_index) = edge.original_edge_index {
                if edge.capacity == 0 {
                    let (original_from, to, edge_id) = &originals[original_index];
                    if *original_from == from {
                        outgoing[from].push((*to, original_index, edge_id.clone()));
                    }
                }
            }
        }
        outgoing[from].sort_by(|left, right| left.2.cmp(&right.2).then_with(|| left.0.cmp(&right.0)));
    }

    let mut paths = Vec::new();
    for _ in 0..flow {
        let mut nodes = vec![node_ids[source].clone()];
        let mut edges = Vec::new();
        let mut visited = BTreeSet::new();
        visited.insert(source);
        if !extract_flow_path(source, sink, &outgoing, node_ids, &mut visited, &mut nodes, &mut edges) {
            return Err(ResilienceError::FlowDecompositionFailed);
        }
        for edge_id in &edges {
            for entries in &mut outgoing {
                if let Some(position) = entries.iter().position(|entry| &entry.2 == edge_id) {
                    entries.remove(position);
                    break;
                }
            }
        }
        paths.push(DisjointPath { node_ids: nodes, edge_ids: edges });
    }

    Ok((flow, paths))
}

fn extract_flow_path(
    current: usize,
    sink: usize,
    outgoing: &[Vec<(usize, usize, String)>],
    node_ids: &[String],
    visited: &mut BTreeSet<usize>,
    path_nodes: &mut Vec<String>,
    path_edges: &mut Vec<String>,
) -> bool {
    if current == sink { return true; }
    for (next, _, edge_id) in &outgoing[current] {
        if !visited.insert(*next) { continue; }
        path_nodes.push(node_ids[*next].clone());
        path_edges.push(edge_id.clone());
        if extract_flow_path(*next, sink, outgoing, node_ids, visited, path_nodes, path_edges) {
            return true;
        }
        path_nodes.pop();
        path_edges.pop();
        visited.remove(next);
    }
    false
}

/// Tarjan bridge/articulation candidates over the undirected projection of
/// declared PowerPath edges. They are structural topology indicators only.
fn undirected_cut_candidates(
    node_ids: &[String],
    node_index: &BTreeMap<String, usize>,
    power_edges: &[&crate::ServiceEdge],
) -> (Vec<String>, Vec<String>) {
    let mut adjacency: Vec<Vec<(usize, usize)>> = vec![Vec::new(); node_ids.len()];
    let mut edge_ids = Vec::new();
    for (edge_index, edge) in power_edges.iter().enumerate() {
        let from = node_index[edge.from_node()];
        let to = node_index[edge.to_node()];
        adjacency[from].push((to, edge_index));
        adjacency[to].push((from, edge_index));
        edge_ids.push(edge.edge_id().to_owned());
    }
    for neighbours in &mut adjacency {
        neighbours.sort_by(|left, right| node_ids[left.0].cmp(&node_ids[right.0]).then_with(|| left.1.cmp(&right.1)));
    }

    let mut discovery = vec![0usize; node_ids.len()];
    let mut low = vec![0usize; node_ids.len()];
    let mut clock = 0usize;
    let mut bridges = BTreeSet::new();
    let mut articulations = BTreeSet::new();

    for root in 0..node_ids.len() {
        if discovery[root] == 0 {
            tarjan_visit(
                root, None, &adjacency, &edge_ids, &mut discovery, &mut low,
                &mut clock, &mut bridges, &mut articulations,
            );
        }
    }

    (
        bridges.into_iter().collect(),
        articulations.into_iter().map(|index| node_ids[index].clone()).collect(),
    )
}

fn tarjan_visit(
    node: usize,
    parent_edge: Option<usize>,
    adjacency: &[Vec<(usize, usize)>],
    edge_ids: &[String],
    discovery: &mut [usize],
    low: &mut [usize],
    clock: &mut usize,
    bridges: &mut BTreeSet<String>,
    articulations: &mut BTreeSet<usize>,
) {
    *clock += 1;
    discovery[node] = *clock;
    low[node] = *clock;
    let mut child_count = 0usize;

    for (next, edge_index) in &adjacency[node] {
        if Some(*edge_index) == parent_edge { continue; }
        if discovery[*next] == 0 {
            child_count += 1;
            tarjan_visit(
                *next, Some(*edge_index), adjacency, edge_ids, discovery, low,
                clock, bridges, articulations,
            );
            low[node] = low[node].min(low[*next]);
            if low[*next] > discovery[node] {
                bridges.insert(edge_ids[*edge_index].clone());
            }
            if parent_edge.is_some() && low[*next] >= discovery[node] {
                articulations.insert(node);
            }
        } else {
            low[node] = low[node].min(discovery[*next]);
        }
    }
    if parent_edge.is_none() && child_count > 1 {
        articulations.insert(node);
    }
}

#[derive(Serialize)]
struct GraphEvidenceView<'a> {
    state: &'static str,
    evidence_id: Option<&'a str>,
}

fn serialize_graph_evidence<S>(evidence: &GraphEvidence, serializer: S) -> Result<S::Ok, S::Error>
where S: Serializer {
    GraphEvidenceView {
        state: evidence_state_name(evidence.state()),
        evidence_id: evidence.evidence_id(),
    }.serialize(serializer)
}

fn serialize_evidence_state<S>(state: &GraphEvidenceState, serializer: S) -> Result<S::Ok, S::Error>
where S: Serializer {
    serializer.serialize_str(evidence_state_name(*state))
}

fn evidence_state_name(state: GraphEvidenceState) -> &'static str {
    match state {
        GraphEvidenceState::Unknown => "unknown",
        GraphEvidenceState::DesignDeclared => "design_declared",
        GraphEvidenceState::FieldObserved => "field_observed",
        GraphEvidenceState::IndependentlyReviewed => "independently_reviewed",
    }
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), ResilienceError> {
    if value.trim().is_empty() { return Err(ResilienceError::EmptyField { field }); }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResilienceError {
    EmptyField { field: &'static str },
    EmptyFailureDomain { domain_id: String },
    DuplicateFailureDomainMember { domain_id: String },
    DuplicateFailureDomainId { domain_id: String },
    UnknownFailureDomainMember { domain_id: String, member_id: String },
    UnknownNode { node_id: String },
    NotPotentialSource { node_id: String },
    NotLoad { node_id: String },
    ResidualPathReconstructionFailed,
    FlowDecompositionFailed,
}
impl core::fmt::Display for ResilienceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for ResilienceError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        GraphEvidence, GraphMaturity, GraphRevision, ServiceEdge, ServiceNode,
        ServiceNodeKind,
    };

    fn evidence() -> GraphEvidence {
        GraphEvidence::new(GraphEvidenceState::DesignDeclared, Some("synthetic:declared".into())).unwrap()
    }

    fn graph(nodes: Vec<(&str, ServiceNodeKind)>, edges: Vec<(&str, &str, &str)>) -> ServiceGraph {
        let revision = GraphRevision::new(
            "fixture:resilience", "revision:1", "sha256:synthetic",
            "Synthetic topology only", GraphMaturity::DesignOnly, evidence(), true,
        ).unwrap();
        let nodes = nodes.into_iter().map(|(id, kind)| {
            ServiceNode::new(id, kind, id, evidence()).unwrap()
        }).collect();
        let edges = edges.into_iter().map(|(id, from, to)| {
            ServiceEdge::new(id, from, to, ServiceEdgeKind::PowerPath, evidence()).unwrap()
        }).collect();
        ServiceGraph::new(revision, nodes, edges).unwrap()
    }

    fn diamond() -> ServiceGraph {
        graph(
            vec![
                ("source", ServiceNodeKind::UtilitySource),
                ("left", ServiceNodeKind::Converter),
                ("right", ServiceNodeKind::Converter),
                ("load", ServiceNodeKind::Load),
            ],
            vec![
                ("e1", "source", "left"),
                ("e2", "left", "load"),
                ("e3", "source", "right"),
                ("e4", "right", "load"),
            ],
        )
    }

    #[test]
    fn two_edge_disjoint_paths_are_found_in_diamond() {
        let result = assess_resilience(
            &diamond(), "source", "load", &[],
            FailureDomainInventoryStatus::CompleteWithinDeclaredScope,
        ).unwrap();
        assert_eq!(result.edge_disjoint_path_count(), 2);
        assert_eq!(result.edge_disjoint_paths().len(), 2);
        assert_eq!(result.independence_assessment(), IndependenceAssessment::NoSharedModeledIntermediateDependencyDetected);
        assert!(result.shared_source_terminal_across_paths());
        assert!(result.shared_intermediate_node_ids().is_empty());
    }


    #[test]
    fn shared_intermediate_bus_is_detected_without_explicit_domain_record() {
        let graph = graph(
            vec![
                ("source", ServiceNodeKind::UtilitySource),
                ("shared-bus", ServiceNodeKind::AcBus),
                ("left", ServiceNodeKind::Converter),
                ("right", ServiceNodeKind::Converter),
                ("load", ServiceNodeKind::Load),
            ],
            vec![
                ("e0", "source", "shared-bus"),
                ("e1", "shared-bus", "left"),
                ("e2", "left", "load"),
                ("e3", "shared-bus", "right"),
                ("e4", "right", "load"),
            ],
        );
        let result = assess_resilience(
            &graph, "source", "load", &[],
            FailureDomainInventoryStatus::CompleteWithinDeclaredScope,
        ).unwrap();
        assert_eq!(result.edge_disjoint_path_count(), 2);
        assert_eq!(result.shared_intermediate_node_ids(), &["shared-bus".to_owned()]);
        assert_eq!(result.independence_assessment(), IndependenceAssessment::SharedModeledFailureDomainAcrossPaths);
    }


    #[test]
    fn shared_intermediate_node_is_reported_even_when_edges_are_disjoint() {
        let graph = graph(
            vec![
                ("source", ServiceNodeKind::UtilitySource),
                ("shared-bus", ServiceNodeKind::AcBus),
                ("load", ServiceNodeKind::Load),
            ],
            vec![
                ("source-a", "source", "shared-bus"),
                ("source-b", "source", "shared-bus"),
                ("load-a", "shared-bus", "load"),
                ("load-b", "shared-bus", "load"),
            ],
        );
        let result = assess_resilience(
            &graph, "source", "load", &[],
            FailureDomainInventoryStatus::CompleteWithinDeclaredScope,
        ).unwrap();
        assert_eq!(result.edge_disjoint_path_count(), 2);
        assert_eq!(result.shared_intermediate_node_ids(), &["shared-bus".to_owned()]);
        assert_eq!(result.independence_assessment(), IndependenceAssessment::SharedModeledFailureDomainAcrossPaths);
    }

    #[test]
    fn shared_upstream_domain_is_exposed_across_both_paths() {
        let domain = FailureDomain::new(
            "domain:upstream", FailureDomainKind::UpstreamSupply,
            vec!["source".into()], Vec::new(), evidence(),
        ).unwrap();
        let result = assess_resilience(
            &diamond(), "source", "load", &[domain],
            FailureDomainInventoryStatus::CompleteWithinDeclaredScope,
        ).unwrap();
        assert_eq!(result.failure_domain_exposures()[0].disjoint_paths_affected(), 2);
        assert_eq!(result.independence_assessment(), IndependenceAssessment::SharedModeledFailureDomainAcrossPaths);
    }

    #[test]
    fn incomplete_domain_inventory_never_claims_no_shared_domain() {
        let result = assess_resilience(
            &diamond(), "source", "load", &[],
            FailureDomainInventoryStatus::Incomplete,
        ).unwrap();
        assert_eq!(result.independence_assessment(), IndependenceAssessment::FailureDomainInventoryIncomplete);
    }

    #[test]
    fn one_route_is_not_reported_as_resilient() {
        let graph = graph(
            vec![("source", ServiceNodeKind::UtilitySource), ("mid", ServiceNodeKind::Converter), ("load", ServiceNodeKind::Load)],
            vec![("e1", "source", "mid"), ("e2", "mid", "load")],
        );
        let result = assess_resilience(&graph, "source", "load", &[], FailureDomainInventoryStatus::CompleteWithinDeclaredScope).unwrap();
        assert_eq!(result.edge_disjoint_path_count(), 1);
        assert_eq!(result.independence_assessment(), IndependenceAssessment::FewerThanTwoEdgeDisjointPaths);
        assert!(result.undirected_power_path_bridge_edge_ids().contains(&"e1".to_owned()));
        assert!(result.undirected_power_path_bridge_edge_ids().contains(&"e2".to_owned()));
        assert!(result.undirected_power_path_articulation_node_ids().contains(&"mid".to_owned()));
    }

    #[test]
    fn measurement_and_control_edges_do_not_contribute_to_paths() {
        let revision = GraphRevision::new("g", "r", "d", "scope", GraphMaturity::DesignOnly, evidence(), true).unwrap();
        let nodes = vec![
            ServiceNode::new("source", ServiceNodeKind::UtilitySource, "source", evidence()).unwrap(),
            ServiceNode::new("load", ServiceNodeKind::Load, "load", evidence()).unwrap(),
            ServiceNode::new("meter", ServiceNodeKind::Meter, "meter", evidence()).unwrap(),
        ];
        let edges = vec![
            ServiceEdge::new("measure", "source", "meter", ServiceEdgeKind::MeasurementAssociation, evidence()).unwrap(),
            ServiceEdge::new("control", "meter", "load", ServiceEdgeKind::ControlAssociation, evidence()).unwrap(),
        ];
        let graph = ServiceGraph::new(revision, nodes, edges).unwrap();
        let result = assess_resilience(&graph, "source", "load", &[], FailureDomainInventoryStatus::CompleteWithinDeclaredScope).unwrap();
        assert_eq!(result.edge_disjoint_path_count(), 0);
    }

    #[test]
    fn duplicate_failure_domain_members_are_rejected() {
        assert!(matches!(
            FailureDomain::new("d", FailureDomainKind::Enclosure,
                vec!["node".into(), "node".into()], Vec::new(), evidence()),
            Err(ResilienceError::DuplicateFailureDomainMember { .. })
        ));
    }

    #[test]
    fn unknown_failure_domain_members_are_rejected() {
        let domain = FailureDomain::new("d", FailureDomainKind::CableRoute,
            vec!["missing".into()], Vec::new(), evidence()).unwrap();
        assert!(matches!(
            assess_resilience(&diamond(), "source", "load", &[domain], FailureDomainInventoryStatus::CompleteWithinDeclaredScope),
            Err(ResilienceError::UnknownFailureDomainMember { .. })
        ));
    }

    #[test]
    fn non_source_and_non_load_pair_is_rejected() {
        assert!(matches!(
            assess_resilience(&diamond(), "left", "load", &[], FailureDomainInventoryStatus::CompleteWithinDeclaredScope),
            Err(ResilienceError::NotPotentialSource { .. })
        ));
        assert!(matches!(
            assess_resilience(&diamond(), "source", "right", &[], FailureDomainInventoryStatus::CompleteWithinDeclaredScope),
            Err(ResilienceError::NotLoad { .. })
        ));
    }

    #[test]
    fn parallel_edges_count_as_distinct_edge_disjoint_paths() {
        let graph = graph(
            vec![("source", ServiceNodeKind::UtilitySource), ("load", ServiceNodeKind::Load)],
            vec![("parallel-a", "source", "load"), ("parallel-b", "source", "load")],
        );
        let result = assess_resilience(&graph, "source", "load", &[], FailureDomainInventoryStatus::CompleteWithinDeclaredScope).unwrap();
        assert_eq!(result.edge_disjoint_path_count(), 2);
    }

    #[test]
    fn a_domain_affecting_only_one_path_is_not_marked_shared_across_paths() {
        let domain = FailureDomain::new(
            "domain:left-route", FailureDomainKind::CableRoute,
            vec!["left".into()], Vec::new(), evidence(),
        ).unwrap();
        let result = assess_resilience(&diamond(), "source", "load", &[domain], FailureDomainInventoryStatus::CompleteWithinDeclaredScope).unwrap();
        assert_eq!(result.failure_domain_exposures()[0].disjoint_paths_affected(), 1);
        assert!(result.shared_intermediate_node_ids().is_empty());
        assert_eq!(result.independence_assessment(), IndependenceAssessment::NoSharedModeledIntermediateDependencyDetected);
    }

    #[test]
    fn unknown_domain_evidence_is_retained_without_inventing_a_verification() {
        let domain = FailureDomain::new(
            "domain:unknown", FailureDomainKind::Enclosure,
            vec!["source".into()], Vec::new(), GraphEvidence::unknown(),
        ).unwrap();
        let result = assess_resilience(&diamond(), "source", "load", &[domain], FailureDomainInventoryStatus::CompleteWithinDeclaredScope).unwrap();
        assert_eq!(result.failure_domain_exposures()[0].evidence_state(), GraphEvidenceState::Unknown);
        assert_eq!(result.failure_domain_exposures()[0].evidence_id(), None);
        assert_eq!(result.independence_assessment(), IndependenceAssessment::FailureDomainInventoryIncomplete);
    }

    #[test]
    fn graph_and_revision_identity_are_retained() {
        let result = assess_resilience(&diamond(), "source", "load", &[], FailureDomainInventoryStatus::CompleteWithinDeclaredScope).unwrap();
        assert_eq!(result.graph_id(), "fixture:resilience");
        assert_eq!(result.revision_id(), "revision:1");
    }
}
