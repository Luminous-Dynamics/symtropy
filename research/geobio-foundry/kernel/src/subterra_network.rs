//! Evidence-preserving subterranean network and demonstrated-route reference.
//!
//! Candidate or geophysically supported links are retained as hypotheses and
//! are never silently inserted into the directly observed cave topology.
//! Robot routes use only edges with directly observed connectivity AND
//! demonstrated traversability. Directed edges allow access and retreat
//! feasibility to differ.
//!
//! This is a graph-evidence kernel, not a SLAM system, cave mapper,
//! geomechanics model, route-risk optimizer or safety certification.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionEvidence {
    Unknown,
    Candidate,
    GeophysicallySupported,
    DirectlyObserved,
    Contradicted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraversabilityEvidence {
    Unknown,
    Candidate,
    Demonstrated,
    NonTraversable,
    OutOfDomain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeDirection {
    Undirected,
    AtoB,
    BtoA,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CaveNode {
    pub node_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CaveEdge {
    pub edge_id: String,
    pub node_a: String,
    pub node_b: String,
    pub length_m: f64,
    pub connection_evidence: ConnectionEvidence,
    pub traversability: TraversabilityEvidence,
    pub direction: EdgeDirection,
    pub evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CaveNetwork {
    pub network_id: String,
    pub nodes: Vec<CaveNode>,
    pub edges: Vec<CaveEdge>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetworkError {
    EmptyNetworkId,
    EmptyNodeId(String),
    DuplicateNodeId(String),
    EmptyEdgeId(String),
    DuplicateEdgeId(String),
    UnknownEdgeEndpoint { edge_id: String, node_id: String },
    SelfLoop(String),
    InvalidEdgeLength(String),
    ConfirmedConnectionMissingEvidence(String),
    CandidateConnectionMissingEvidence(String),
    DemonstratedTraversalWithoutConfirmedConnection(String),
    DemonstratedTraversalMissingEvidence(String),
    ContradictedConnectionMarkedTraversable(String),
}

impl std::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for NetworkError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteKnowledge {
    DemonstratedRoute { node_ids: Vec<String>, edge_ids: Vec<String> },
    NoDemonstratedRoute,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CaveNetworkSummary {
    /// Components connected by directly observed edges, regardless of whether
    /// a robot has demonstrated traversal along those edges.
    pub directly_observed_components: Vec<Vec<String>>,
    /// Candidate links remain separate from confirmed topology.
    pub candidate_edge_ids: Vec<String>,
    pub geophysically_supported_edge_ids: Vec<String>,
    pub contradicted_edge_ids: Vec<String>,
    pub unknown_edge_ids: Vec<String>,
    pub directly_observed_but_not_traversable_edge_ids: Vec<String>,
    pub demonstrated_traversable_edge_ids: Vec<String>,
}

fn has_refs(refs: &[String]) -> bool {
    refs.iter().any(|value| !value.trim().is_empty())
}

impl CaveNetwork {
    pub fn validate(&self) -> Result<(), Vec<NetworkError>> {
        let mut errors = Vec::new();

        if self.network_id.trim().is_empty() {
            errors.push(NetworkError::EmptyNetworkId);
        }

        let mut node_ids = BTreeSet::new();
        for node in &self.nodes {
            if node.node_id.trim().is_empty() {
                errors.push(NetworkError::EmptyNodeId(node.node_id.clone()));
            }
            if !node_ids.insert(node.node_id.as_str()) {
                errors.push(NetworkError::DuplicateNodeId(node.node_id.clone()));
            }
        }

        let mut edge_ids = BTreeSet::new();
        for edge in &self.edges {
            if edge.edge_id.trim().is_empty() {
                errors.push(NetworkError::EmptyEdgeId(edge.edge_id.clone()));
            }
            if !edge_ids.insert(edge.edge_id.as_str()) {
                errors.push(NetworkError::DuplicateEdgeId(edge.edge_id.clone()));
            }

            if !node_ids.contains(edge.node_a.as_str()) {
                errors.push(NetworkError::UnknownEdgeEndpoint {
                    edge_id: edge.edge_id.clone(),
                    node_id: edge.node_a.clone(),
                });
            }
            if !node_ids.contains(edge.node_b.as_str()) {
                errors.push(NetworkError::UnknownEdgeEndpoint {
                    edge_id: edge.edge_id.clone(),
                    node_id: edge.node_b.clone(),
                });
            }
            if edge.node_a == edge.node_b {
                errors.push(NetworkError::SelfLoop(edge.edge_id.clone()));
            }
            if !edge.length_m.is_finite() || edge.length_m <= 0.0 {
                errors.push(NetworkError::InvalidEdgeLength(edge.edge_id.clone()));
            }

            match edge.connection_evidence {
                ConnectionEvidence::DirectlyObserved if !has_refs(&edge.evidence_refs) => {
                    errors.push(NetworkError::ConfirmedConnectionMissingEvidence(
                        edge.edge_id.clone(),
                    ));
                }
                ConnectionEvidence::Candidate
                | ConnectionEvidence::GeophysicallySupported
                    if !has_refs(&edge.evidence_refs) =>
                {
                    errors.push(NetworkError::CandidateConnectionMissingEvidence(
                        edge.edge_id.clone(),
                    ));
                }
                _ => {}
            }

            if edge.traversability == TraversabilityEvidence::Demonstrated {
                if edge.connection_evidence != ConnectionEvidence::DirectlyObserved {
                    errors.push(
                        NetworkError::DemonstratedTraversalWithoutConfirmedConnection(
                            edge.edge_id.clone(),
                        ),
                    );
                }
                if !has_refs(&edge.evidence_refs) {
                    errors.push(NetworkError::DemonstratedTraversalMissingEvidence(
                        edge.edge_id.clone(),
                    ));
                }
            }

            if edge.connection_evidence == ConnectionEvidence::Contradicted
                && edge.traversability == TraversabilityEvidence::Demonstrated
            {
                errors.push(NetworkError::ContradictedConnectionMarkedTraversable(
                    edge.edge_id.clone(),
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Summarize confirmed topology without promoting hypotheses to observed
    /// connections. Isolated nodes are retained as singleton components.
    pub fn summarize(&self) -> Result<CaveNetworkSummary, Vec<NetworkError>> {
        self.validate()?;

        let mut adjacency = BTreeMap::<String, Vec<String>>::new();
        for node in &self.nodes {
            adjacency.entry(node.node_id.clone()).or_default();
        }

        let mut candidate_edge_ids = Vec::new();
        let mut geophysically_supported_edge_ids = Vec::new();
        let mut contradicted_edge_ids = Vec::new();
        let mut unknown_edge_ids = Vec::new();
        let mut directly_observed_but_not_traversable_edge_ids = Vec::new();
        let mut demonstrated_traversable_edge_ids = Vec::new();

        for edge in &self.edges {
            match edge.connection_evidence {
                ConnectionEvidence::Unknown => {
                    unknown_edge_ids.push(edge.edge_id.clone());
                }
                ConnectionEvidence::Candidate => {
                    candidate_edge_ids.push(edge.edge_id.clone());
                }
                ConnectionEvidence::GeophysicallySupported => {
                    geophysically_supported_edge_ids.push(edge.edge_id.clone());
                }
                ConnectionEvidence::Contradicted => {
                    contradicted_edge_ids.push(edge.edge_id.clone());
                }
                ConnectionEvidence::DirectlyObserved => {
                    adjacency
                        .entry(edge.node_a.clone())
                        .or_default()
                        .push(edge.node_b.clone());
                    adjacency
                        .entry(edge.node_b.clone())
                        .or_default()
                        .push(edge.node_a.clone());

                    if edge.traversability == TraversabilityEvidence::Demonstrated {
                        demonstrated_traversable_edge_ids.push(edge.edge_id.clone());
                    } else {
                        directly_observed_but_not_traversable_edge_ids
                            .push(edge.edge_id.clone());
                    }
                }
            }
        }

        for neighbours in adjacency.values_mut() {
            neighbours.sort();
            neighbours.dedup();
        }

        let mut visited = BTreeSet::<String>::new();
        let mut components = Vec::new();

        for node in adjacency.keys() {
            if visited.contains(node) {
                continue;
            }
            let mut queue = VecDeque::from([node.clone()]);
            let mut component = Vec::new();
            visited.insert(node.clone());

            while let Some(current) = queue.pop_front() {
                component.push(current.clone());
                if let Some(neighbours) = adjacency.get(&current) {
                    for neighbour in neighbours {
                        if visited.insert(neighbour.clone()) {
                            queue.push_back(neighbour.clone());
                        }
                    }
                }
            }

            component.sort();
            components.push(component);
        }

        components.sort_by(|left, right| left.first().cmp(&right.first()));
        candidate_edge_ids.sort();
        geophysically_supported_edge_ids.sort();
        contradicted_edge_ids.sort();
        unknown_edge_ids.sort();
        directly_observed_but_not_traversable_edge_ids.sort();
        demonstrated_traversable_edge_ids.sort();

        Ok(CaveNetworkSummary {
            directly_observed_components: components,
            candidate_edge_ids,
            geophysically_supported_edge_ids,
            contradicted_edge_ids,
            unknown_edge_ids,
            directly_observed_but_not_traversable_edge_ids,
            demonstrated_traversable_edge_ids,
        })
    }

    /// Return a demonstrated traversal route using only directly observed
    /// connectivity edges whose traversability has itself been demonstrated.
    /// A missing route means only that no demonstrated route exists in the
    /// current evidence graph; it does not prove physical disconnection.
    pub fn demonstrated_route(
        &self,
        start_node: &str,
        goal_node: &str,
    ) -> Result<RouteKnowledge, Vec<NetworkError>> {
        self.validate()?;

        let node_ids = self
            .nodes
            .iter()
            .map(|node| node.node_id.as_str())
            .collect::<BTreeSet<_>>();

        if !node_ids.contains(start_node) || !node_ids.contains(goal_node) {
            return Err(vec![NetworkError::UnknownEdgeEndpoint {
                edge_id: "route-query".to_string(),
                node_id: if !node_ids.contains(start_node) {
                    start_node.to_string()
                } else {
                    goal_node.to_string()
                },
            }]);
        }

        if start_node == goal_node {
            return Ok(RouteKnowledge::DemonstratedRoute {
                node_ids: vec![start_node.to_string()],
                edge_ids: Vec::new(),
            });
        }

        let mut adjacency = BTreeMap::<String, Vec<(String, String)>>::new();

        for edge in &self.edges {
            if edge.connection_evidence != ConnectionEvidence::DirectlyObserved
                || edge.traversability != TraversabilityEvidence::Demonstrated
            {
                continue;
            }

            match edge.direction {
                EdgeDirection::Undirected => {
                    adjacency
                        .entry(edge.node_a.clone())
                        .or_default()
                        .push((edge.node_b.clone(), edge.edge_id.clone()));
                    adjacency
                        .entry(edge.node_b.clone())
                        .or_default()
                        .push((edge.node_a.clone(), edge.edge_id.clone()));
                }
                EdgeDirection::AtoB => {
                    adjacency
                        .entry(edge.node_a.clone())
                        .or_default()
                        .push((edge.node_b.clone(), edge.edge_id.clone()));
                }
                EdgeDirection::BtoA => {
                    adjacency
                        .entry(edge.node_b.clone())
                        .or_default()
                        .push((edge.node_a.clone(), edge.edge_id.clone()));
                }
            }
        }

        for neighbours in adjacency.values_mut() {
            neighbours.sort_by(|left, right| {
                left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1))
            });
        }

        let mut queue = VecDeque::from([start_node.to_string()]);
        let mut visited = BTreeSet::from([start_node.to_string()]);
        let mut predecessor = BTreeMap::<String, (String, String)>::new();

        while let Some(current) = queue.pop_front() {
            if let Some(neighbours) = adjacency.get(&current) {
                for (neighbour, edge_id) in neighbours {
                    if visited.insert(neighbour.clone()) {
                        predecessor.insert(
                            neighbour.clone(),
                            (current.clone(), edge_id.clone()),
                        );
                        if neighbour == goal_node {
                            let mut path_nodes = vec![goal_node.to_string()];
                            let mut path_edges = Vec::new();
                            let mut cursor = goal_node.to_string();

                            while cursor != start_node {
                                let Some((previous, used_edge)) = predecessor.get(&cursor) else {
                                    return Ok(RouteKnowledge::NoDemonstratedRoute);
                                };
                                path_edges.push(used_edge.clone());
                                path_nodes.push(previous.clone());
                                cursor = previous.clone();
                            }

                            path_nodes.reverse();
                            path_edges.reverse();
                            return Ok(RouteKnowledge::DemonstratedRoute {
                                node_ids: path_nodes,
                                edge_ids: path_edges,
                            });
                        }
                        queue.push_back(neighbour.clone());
                    }
                }
            }
        }

        Ok(RouteKnowledge::NoDemonstratedRoute)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(
        id: &str,
        a: &str,
        b: &str,
        connection_evidence: ConnectionEvidence,
        traversability: TraversabilityEvidence,
        direction: EdgeDirection,
    ) -> CaveEdge {
        CaveEdge {
            edge_id: id.to_string(),
            node_a: a.to_string(),
            node_b: b.to_string(),
            length_m: 10.0,
            connection_evidence,
            traversability,
            direction,
            evidence_refs: if connection_evidence == ConnectionEvidence::Unknown {
                Vec::new()
            } else {
                vec![format!("evidence-{id}")]
            },
        }
    }

    fn network(edges: Vec<CaveEdge>) -> CaveNetwork {
        CaveNetwork {
            network_id: "synthetic-network-01".to_string(),
            nodes: ["entrance", "chamber-a", "chamber-b", "chamber-c"]
                .into_iter()
                .map(|node_id| CaveNode {
                    node_id: node_id.to_string(),
                })
                .collect(),
            edges,
        }
    }

    #[test]
    fn candidate_edges_do_not_merge_observed_components() {
        let graph = network(vec![
            edge(
                "candidate-ab",
                "entrance",
                "chamber-a",
                ConnectionEvidence::Candidate,
                TraversabilityEvidence::Candidate,
                EdgeDirection::Undirected,
            ),
        ]);

        let summary = graph.summarize().unwrap();
        assert_eq!(summary.directly_observed_components.len(), 4);
        assert_eq!(summary.candidate_edge_ids, vec!["candidate-ab"]);
    }

    #[test]
    fn geophysically_supported_edge_is_not_directly_observed_topology() {
        let graph = network(vec![
            edge(
                "gpr-ab",
                "entrance",
                "chamber-a",
                ConnectionEvidence::GeophysicallySupported,
                TraversabilityEvidence::Unknown,
                EdgeDirection::Undirected,
            ),
        ]);

        let summary = graph.summarize().unwrap();
        assert_eq!(summary.directly_observed_components.len(), 4);
        assert_eq!(summary.geophysically_supported_edge_ids, vec!["gpr-ab"]);
    }

    #[test]
    fn directly_observed_connection_merges_topology_even_without_traversal() {
        let graph = network(vec![
            edge(
                "observed-ab",
                "entrance",
                "chamber-a",
                ConnectionEvidence::DirectlyObserved,
                TraversabilityEvidence::Unknown,
                EdgeDirection::Undirected,
            ),
        ]);

        let summary = graph.summarize().unwrap();
        assert_eq!(summary.directly_observed_components.len(), 3);
        assert!(summary
            .directly_observed_but_not_traversable_edge_ids
            .contains(&"observed-ab".to_string()));
    }

    #[test]
    fn robot_route_requires_demonstrated_traversability() {
        let graph = network(vec![
            edge(
                "known-connection-only",
                "entrance",
                "chamber-a",
                ConnectionEvidence::DirectlyObserved,
                TraversabilityEvidence::Unknown,
                EdgeDirection::Undirected,
            ),
        ]);

        assert_eq!(
            graph.demonstrated_route("entrance", "chamber-a").unwrap(),
            RouteKnowledge::NoDemonstratedRoute
        );
    }

    #[test]
    fn demonstrated_undirected_route_is_returnable() {
        let graph = network(vec![
            edge(
                "demonstrated-ab",
                "entrance",
                "chamber-a",
                ConnectionEvidence::DirectlyObserved,
                TraversabilityEvidence::Demonstrated,
                EdgeDirection::Undirected,
            ),
        ]);

        let outbound = graph.demonstrated_route("entrance", "chamber-a").unwrap();
        let retreat = graph.demonstrated_route("chamber-a", "entrance").unwrap();
        assert_eq!(
            outbound,
            RouteKnowledge::DemonstratedRoute {
                node_ids: vec!["entrance".to_string(), "chamber-a".to_string()],
                edge_ids: vec!["demonstrated-ab".to_string()]
            }
        );
        assert_eq!(retreat, outbound);
    }

    #[test]
    fn one_way_edge_can_demonstrate_access_without_demonstrating_retreat() {
        let graph = network(vec![
            edge(
                "one-way-slope",
                "entrance",
                "chamber-a",
                ConnectionEvidence::DirectlyObserved,
                TraversabilityEvidence::Demonstrated,
                EdgeDirection::AtoB,
            ),
        ]);

        assert!(matches!(
            graph.demonstrated_route("entrance", "chamber-a").unwrap(),
            RouteKnowledge::DemonstratedRoute { .. }
        ));
        assert_eq!(
            graph.demonstrated_route("chamber-a", "entrance").unwrap(),
            RouteKnowledge::NoDemonstratedRoute
        );
    }

    #[test]
    fn retreat_can_exist_when_outbound_access_is_not_demonstrated() {
        let graph = network(vec![
            edge(
                "one-way-descent",
                "entrance",
                "chamber-a",
                ConnectionEvidence::DirectlyObserved,
                TraversabilityEvidence::Demonstrated,
                EdgeDirection::BtoA,
            ),
        ]);

        assert_eq!(
            graph.demonstrated_route("entrance", "chamber-a").unwrap(),
            RouteKnowledge::NoDemonstratedRoute
        );
        assert!(matches!(
            graph.demonstrated_route("chamber-a", "entrance").unwrap(),
            RouteKnowledge::DemonstratedRoute { .. }
        ));
    }

    #[test]
    fn candidate_edge_cannot_claim_demonstrated_traversal() {
        let graph = network(vec![
            edge(
                "unverified-but-traversable",
                "entrance",
                "chamber-a",
                ConnectionEvidence::Candidate,
                TraversabilityEvidence::Demonstrated,
                EdgeDirection::Undirected,
            ),
        ]);

        assert!(graph.validate().unwrap_err().contains(
            &NetworkError::DemonstratedTraversalWithoutConfirmedConnection(
                "unverified-but-traversable".to_string()
            )
        ));
    }

    #[test]
    fn contradicted_edge_cannot_claim_demonstrated_traversal() {
        let graph = network(vec![
            edge(
                "contradicted-edge",
                "entrance",
                "chamber-a",
                ConnectionEvidence::Contradicted,
                TraversabilityEvidence::Demonstrated,
                EdgeDirection::Undirected,
            ),
        ]);

        assert!(graph.validate().unwrap_err().contains(
            &NetworkError::DemonstratedTraversalWithoutConfirmedConnection(
                "contradicted-edge".to_string()
            )
        ));
    }

    #[test]
    fn directly_observed_edge_requires_evidence_reference() {
        let mut graph = network(vec![edge(
            "observed-ab",
            "entrance",
            "chamber-a",
            ConnectionEvidence::DirectlyObserved,
            TraversabilityEvidence::Unknown,
            EdgeDirection::Undirected,
        )]);
        graph.edges[0].evidence_refs.clear();

        assert!(graph.validate().unwrap_err().contains(
            &NetworkError::ConfirmedConnectionMissingEvidence("observed-ab".to_string())
        ));
    }

    #[test]
    fn unknown_edge_endpoint_is_rejected() {
        let graph = network(vec![CaveEdge {
            edge_id: "bad-endpoint".to_string(),
            node_a: "entrance".to_string(),
            node_b: "not-in-network".to_string(),
            length_m: 10.0,
            connection_evidence: ConnectionEvidence::Candidate,
            traversability: TraversabilityEvidence::Candidate,
            direction: EdgeDirection::Undirected,
            evidence_refs: vec!["evidence".to_string()],
        }]);

        assert!(graph.validate().unwrap_err().iter().any(|error| matches!(
            error,
            NetworkError::UnknownEdgeEndpoint { node_id, .. } if node_id == "not-in-network"
        )));
    }

    #[test]
    fn duplicate_ids_and_invalid_lengths_are_rejected() {
        let mut graph = network(vec![
            edge(
                "same-id",
                "entrance",
                "chamber-a",
                ConnectionEvidence::Candidate,
                TraversabilityEvidence::Candidate,
                EdgeDirection::Undirected,
            ),
            edge(
                "same-id",
                "chamber-a",
                "chamber-b",
                ConnectionEvidence::Candidate,
                TraversabilityEvidence::Candidate,
                EdgeDirection::Undirected,
            ),
        ]);
        graph.edges[1].length_m = 0.0;

        let errors = graph.validate().unwrap_err();
        assert!(errors.contains(&NetworkError::DuplicateEdgeId("same-id".to_string())));
        assert!(errors.contains(&NetworkError::InvalidEdgeLength("same-id".to_string())));
    }

    #[test]
    fn absent_route_is_not_misreported_as_physical_disconnection() {
        let graph = network(vec![
            edge(
                "unknown-ab",
                "entrance",
                "chamber-a",
                ConnectionEvidence::Unknown,
                TraversabilityEvidence::Unknown,
                EdgeDirection::Undirected,
            ),
        ]);

        assert_eq!(
            graph.demonstrated_route("entrance", "chamber-a").unwrap(),
            RouteKnowledge::NoDemonstratedRoute
        );
    }
}
