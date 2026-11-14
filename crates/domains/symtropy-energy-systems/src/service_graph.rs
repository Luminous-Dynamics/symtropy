// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Typed, versioned electrical service graph and declared source-path analysis.
//!
//! This module inventories a declared topology and traces declared PowerPath
//! edges. It does not solve load flow, protection, fault current, earthing,
//! conductor sizing, switching or physical state estimation. A path in this
//! graph is not proof that a conductor is energized, de-energized or safe to
//! touch. No path in the graph is not proof that a physical source is absent.

use crate::ModuleIdentity;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Stable identity and scope metadata for one graph revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphRevision {
    graph_id: String,
    revision_id: String,
    content_digest: String,
    scope_statement: String,
    maturity: GraphMaturity,
    maturity_evidence: GraphEvidence,
    current: bool,
}

impl GraphRevision {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        graph_id: impl Into<String>,
        revision_id: impl Into<String>,
        content_digest: impl Into<String>,
        scope_statement: impl Into<String>,
        maturity: GraphMaturity,
        maturity_evidence: GraphEvidence,
        current: bool,
    ) -> Result<Self, ServiceGraphError> {
        let graph_id = graph_id.into();
        let revision_id = revision_id.into();
        let content_digest = content_digest.into();
        let scope_statement = scope_statement.into();

        for (field, value) in [
            ("graph_id", graph_id.as_str()),
            ("revision_id", revision_id.as_str()),
            ("content_digest", content_digest.as_str()),
            ("scope_statement", scope_statement.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(ServiceGraphError::MissingRevisionField { field });
            }
        }

        let maturity_evidence_is_sufficient = match maturity {
            GraphMaturity::DesignOnly => {
                maturity_evidence.state() != GraphEvidenceState::Unknown
            }
            GraphMaturity::FieldReconciled => matches!(
                maturity_evidence.state(),
                GraphEvidenceState::FieldObserved | GraphEvidenceState::IndependentlyReviewed
            ),
            GraphMaturity::IndependentlyReviewed => {
                maturity_evidence.state() == GraphEvidenceState::IndependentlyReviewed
            }
        };
        if !maturity_evidence_is_sufficient {
            return Err(ServiceGraphError::GraphMaturityEvidenceInsufficient {
                maturity,
                evidence_state: maturity_evidence.state(),
            });
        }

        Ok(Self {
            graph_id,
            revision_id,
            content_digest,
            scope_statement,
            maturity,
            maturity_evidence,
            current,
        })
    }

    pub fn graph_id(&self) -> &str {
        &self.graph_id
    }

    pub fn revision_id(&self) -> &str {
        &self.revision_id
    }

    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }

    pub fn scope_statement(&self) -> &str {
        &self.scope_statement
    }

    pub const fn maturity(&self) -> GraphMaturity {
        self.maturity
    }

    pub const fn maturity_evidence(&self) -> &GraphEvidence {
        &self.maturity_evidence
    }

    pub const fn is_current(&self) -> bool {
        self.current
    }
}

/// Maturity of the declared graph within its written scope.
///
/// These are provenance states, not a universal safety ranking or electrical
/// installation certification. IndependentlyReviewed must be backed by the
/// evidence references on the relevant nodes and edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum GraphMaturity {
    DesignOnly,
    FieldReconciled,
    IndependentlyReviewed,
}

/// Evidence associated with one graph node or edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphEvidence {
    state: GraphEvidenceState,
    evidence_id: Option<String>,
}

impl GraphEvidence {
    pub fn new(
        state: GraphEvidenceState,
        evidence_id: Option<String>,
    ) -> Result<Self, ServiceGraphError> {
        match state {
            GraphEvidenceState::Unknown => {
                if evidence_id.is_some() {
                    return Err(ServiceGraphError::UnexpectedEvidenceForUnknown);
                }
                Ok(Self {
                    state,
                    evidence_id: None,
                })
            }
            _ => {
                let Some(id) = evidence_id else {
                    return Err(ServiceGraphError::MissingEvidenceId);
                };
                if id.trim().is_empty() {
                    return Err(ServiceGraphError::MissingEvidenceId);
                }
                Ok(Self {
                    state,
                    evidence_id: Some(id),
                })
            }
        }
    }

    pub const fn unknown() -> Self {
        Self {
            state: GraphEvidenceState::Unknown,
            evidence_id: None,
        }
    }

    pub const fn state(&self) -> GraphEvidenceState {
        self.state
    }

    pub fn evidence_id(&self) -> Option<&str> {
        self.evidence_id.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum GraphEvidenceState {
    Unknown,
    DesignDeclared,
    FieldObserved,
    IndependentlyReviewed,
}

/// Node classes used by the electrical service graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ServiceNodeKind {
    UtilitySource,
    RenewableSource,
    GeneratorSource,
    Storage,
    Converter,
    AcBus,
    DcBus,
    DistributionBoard,
    Feeder,
    Circuit,
    Load,
    ProtectiveDevice,
    IsolationPoint,
    Meter,
    ThermalModule,
    Controller,
}

impl ServiceNodeKind {
    /// Sources that may originate a declared source-to-load path.
    ///
    /// Storage is included because it may discharge to a load. Charge paths
    /// should be represented separately with their own directed PowerPath.
    const fn is_potential_source(self) -> bool {
        matches!(
            self,
            Self::UtilitySource
                | Self::RenewableSource
                | Self::GeneratorSource
                | Self::Storage
        )
    }

    const fn can_appear_on_power_path(self) -> bool {
        !matches!(self, Self::Meter | Self::ThermalModule | Self::Controller)
    }

    const fn can_originate_power_path(self) -> bool {
        !matches!(self, Self::Load | Self::Meter | Self::ThermalModule | Self::Controller)
    }
}

/// One physical or logical asset/circuit/service in the declared graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServiceNode {
    node_id: String,
    kind: ServiceNodeKind,
    label: String,
    evidence: GraphEvidence,
    module_identity: Option<ModuleIdentity>,
}

impl ServiceNode {
    pub fn new(
        node_id: impl Into<String>,
        kind: ServiceNodeKind,
        label: impl Into<String>,
        evidence: GraphEvidence,
    ) -> Result<Self, ServiceGraphError> {
        let node_id = node_id.into();
        let label = label.into();

        if node_id.trim().is_empty() {
            return Err(ServiceGraphError::MissingNodeId);
        }
        if label.trim().is_empty() {
            return Err(ServiceGraphError::MissingNodeLabel);
        }

        Ok(Self {
            node_id,
            kind,
            label,
            evidence,
            module_identity: None,
        })
    }

    /// Bind this graph node to the exact module/configuration identity from
    /// the Power Fabric module contract. A stale identity is retained and
    /// surfaced by graph assessment; it is never silently refreshed.
    pub fn with_module_identity(mut self, identity: ModuleIdentity) -> Self {
        self.module_identity = Some(identity);
        self
    }

    pub fn module_identity(&self) -> Option<&ModuleIdentity> {
        self.module_identity.as_ref()
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub const fn kind(&self) -> ServiceNodeKind {
        self.kind
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn evidence(&self) -> &GraphEvidence {
        &self.evidence
    }
}

/// Relationship types. Only PowerPath participates in source reachability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ServiceEdgeKind {
    /// Directed possibility of energy delivery from from_node to to_node.
    PowerPath,
    /// A protective device is associated with a protected circuit/section.
    ProtectionAssociation,
    /// A measurement point observes an asset or service.
    MeasurementAssociation,
    /// A controller is associated with an asset or service.
    ControlAssociation,
    /// A thermal module/thermal path is associated with an asset.
    ThermalAssociation,
    /// Physical mounting or containment relationship.
    PhysicalAssociation,
    /// Authorization or governance relationship.
    AuthorityAssociation,
}

/// Directed relationship between two graph nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServiceEdge {
    edge_id: String,
    from_node: String,
    to_node: String,
    kind: ServiceEdgeKind,
    evidence: GraphEvidence,
}

impl ServiceEdge {
    pub fn new(
        edge_id: impl Into<String>,
        from_node: impl Into<String>,
        to_node: impl Into<String>,
        kind: ServiceEdgeKind,
        evidence: GraphEvidence,
    ) -> Result<Self, ServiceGraphError> {
        let edge_id = edge_id.into();
        let from_node = from_node.into();
        let to_node = to_node.into();

        for (field, value) in [
            ("edge_id", edge_id.as_str()),
            ("from_node", from_node.as_str()),
            ("to_node", to_node.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(ServiceGraphError::MissingEdgeField { field });
            }
        }
        if from_node == to_node {
            return Err(ServiceGraphError::SelfReferentialEdge {
                edge_id,
                node_id: from_node,
            });
        }

        Ok(Self {
            edge_id,
            from_node,
            to_node,
            kind,
            evidence,
        })
    }

    pub fn edge_id(&self) -> &str {
        &self.edge_id
    }

    pub fn from_node(&self) -> &str {
        &self.from_node
    }

    pub fn to_node(&self) -> &str {
        &self.to_node
    }

    pub const fn kind(&self) -> ServiceEdgeKind {
        self.kind
    }

    pub const fn evidence(&self) -> &GraphEvidence {
        &self.evidence
    }
}

/// Validation failures for a malformed graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceGraphError {
    MissingRevisionField { field: &'static str },
    MissingEvidenceId,
    UnexpectedEvidenceForUnknown,
    MissingNodeId,
    MissingNodeLabel,
    MissingEdgeField { field: &'static str },
    SelfReferentialEdge { edge_id: String, node_id: String },
    EmptyGraph,
    DuplicateNodeId { node_id: String },
    DuplicateEdgeId { edge_id: String },
    DanglingEdgeEndpoint { edge_id: String, node_id: String },
    InvalidPowerPathNodeKind {
        edge_id: String,
        node_id: String,
        kind: ServiceNodeKind,
    },
    InvalidPowerPathOriginNodeKind {
        edge_id: String,
        node_id: String,
        kind: ServiceNodeKind,
    },
    GraphMaturityEvidenceInsufficient {
        maturity: GraphMaturity,
        evidence_state: GraphEvidenceState,
    },
}

impl core::fmt::Display for ServiceGraphError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingRevisionField { field } => {
                write!(f, "graph revision field {field} must not be empty")
            }
            Self::MissingEvidenceId => write!(f, "known graph evidence requires an evidence id"),
            Self::UnexpectedEvidenceForUnknown => {
                write!(f, "unknown graph evidence must not carry an evidence id")
            }
            Self::MissingNodeId => write!(f, "node id must not be empty"),
            Self::MissingNodeLabel => write!(f, "node label must not be empty"),
            Self::MissingEdgeField { field } => {
                write!(f, "edge field {field} must not be empty")
            }
            Self::SelfReferentialEdge { edge_id, node_id } => {
                write!(f, "edge {edge_id} cannot connect node {node_id} to itself")
            }
            Self::EmptyGraph => write!(f, "service graph must contain at least one node"),
            Self::DuplicateNodeId { node_id } => {
                write!(f, "duplicate service graph node id {node_id}")
            }
            Self::DuplicateEdgeId { edge_id } => {
                write!(f, "duplicate service graph edge id {edge_id}")
            }
            Self::DanglingEdgeEndpoint { edge_id, node_id } => {
                write!(f, "edge {edge_id} references missing node {node_id}")
            }
            Self::InvalidPowerPathNodeKind {
                edge_id,
                node_id,
                kind,
            } => write!(
                f,
                "power-path edge {edge_id} uses non-power-path node {node_id} ({kind:?})"
            ),
            Self::InvalidPowerPathOriginNodeKind {
                edge_id,
                node_id,
                kind,
            } => write!(
                f,
                "power-path edge {edge_id} cannot originate at load/measurement node {node_id} ({kind:?})"
            ),
            Self::GraphMaturityEvidenceInsufficient {
                maturity,
                evidence_state,
            } => write!(
                f,
                "graph maturity {maturity:?} is not supported by revision evidence {evidence_state:?}"
            ),
        }
    }
}

impl std::error::Error for ServiceGraphError {}

/// Validated, deterministically ordered graph revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServiceGraph {
    revision: GraphRevision,
    nodes: Vec<ServiceNode>,
    edges: Vec<ServiceEdge>,
}

impl ServiceGraph {
    pub fn new(
        revision: GraphRevision,
        mut nodes: Vec<ServiceNode>,
        mut edges: Vec<ServiceEdge>,
    ) -> Result<Self, ServiceGraphError> {
        if nodes.is_empty() {
            return Err(ServiceGraphError::EmptyGraph);
        }

        nodes.sort_by(|left, right| left.node_id.cmp(&right.node_id));
        edges.sort_by(|left, right| left.edge_id.cmp(&right.edge_id));

        let mut node_ids = BTreeSet::new();
        for node in &nodes {
            if !node_ids.insert(node.node_id.as_str()) {
                return Err(ServiceGraphError::DuplicateNodeId {
                    node_id: node.node_id.clone(),
                });
            }
        }

        let mut edge_ids = BTreeSet::new();
        let node_kinds: BTreeMap<&str, ServiceNodeKind> = nodes
            .iter()
            .map(|node| (node.node_id.as_str(), node.kind))
            .collect();

        for edge in &edges {
            if !edge_ids.insert(edge.edge_id.as_str()) {
                return Err(ServiceGraphError::DuplicateEdgeId {
                    edge_id: edge.edge_id.clone(),
                });
            }

            let Some(from_kind) = node_kinds.get(edge.from_node.as_str()).copied() else {
                return Err(ServiceGraphError::DanglingEdgeEndpoint {
                    edge_id: edge.edge_id.clone(),
                    node_id: edge.from_node.clone(),
                });
            };
            let Some(to_kind) = node_kinds.get(edge.to_node.as_str()).copied() else {
                return Err(ServiceGraphError::DanglingEdgeEndpoint {
                    edge_id: edge.edge_id.clone(),
                    node_id: edge.to_node.clone(),
                });
            };

            if edge.kind == ServiceEdgeKind::PowerPath {
                if !from_kind.can_originate_power_path() {
                    return Err(ServiceGraphError::InvalidPowerPathOriginNodeKind {
                        edge_id: edge.edge_id.clone(),
                        node_id: edge.from_node.clone(),
                        kind: from_kind,
                    });
                }
                for (node_id, kind) in [
                    (edge.from_node.as_str(), from_kind),
                    (edge.to_node.as_str(), to_kind),
                ] {
                    if !kind.can_appear_on_power_path() {
                        return Err(ServiceGraphError::InvalidPowerPathNodeKind {
                            edge_id: edge.edge_id.clone(),
                            node_id: node_id.to_owned(),
                            kind,
                        });
                    }
                }
            }
        }

        Ok(Self {
            revision,
            nodes,
            edges,
        })
    }

    pub const fn revision(&self) -> &GraphRevision {
        &self.revision
    }

    pub fn nodes(&self) -> &[ServiceNode] {
        &self.nodes
    }

    pub fn edges(&self) -> &[ServiceEdge] {
        &self.edges
    }

    pub fn node(&self, node_id: &str) -> Option<&ServiceNode> {
        self.nodes.iter().find(|node| node.node_id == node_id)
    }

    /// Build a deterministic map of declared source-to-load paths.
    ///
    /// Paths are graph topology assertions only. They do not establish live
    /// power flow, complete physical source discovery, isolation or safety.
    pub fn assess_declared_source_paths(&self) -> SourceMapAssessment {
        let mut adjacency: BTreeMap<String, Vec<&ServiceEdge>> = BTreeMap::new();
        for edge in self.edges.iter().filter(|edge| edge.kind == ServiceEdgeKind::PowerPath) {
            adjacency
                .entry(edge.from_node.clone())
                .or_default()
                .push(edge);
        }
        for outgoing in adjacency.values_mut() {
            outgoing.sort_by(|left, right| {
                left.to_node
                    .cmp(&right.to_node)
                    .then_with(|| left.edge_id.cmp(&right.edge_id))
            });
        }

        let mut paths_by_load: BTreeMap<String, Vec<SourcePath>> = BTreeMap::new();
        for load in self.nodes.iter().filter(|node| node.kind == ServiceNodeKind::Load) {
            paths_by_load.insert(load.node_id.clone(), Vec::new());
        }

        for source in self.nodes.iter().filter(|node| node.kind.is_potential_source()) {
            let mut queue = VecDeque::new();
            let mut visited = BTreeSet::new();
            let mut predecessor: BTreeMap<String, (String, &ServiceEdge)> = BTreeMap::new();

            queue.push_back(source.node_id.clone());
            visited.insert(source.node_id.clone());

            while let Some(current) = queue.pop_front() {
                if let Some(outgoing) = adjacency.get(&current) {
                    for edge in outgoing {
                        let next = edge.to_node.clone();
                        if visited.insert(next.clone()) {
                            predecessor.insert(next.clone(), (current.clone(), edge));
                            queue.push_back(next);
                        }
                    }
                }
            }

            for load in self.nodes.iter().filter(|node| node.kind == ServiceNodeKind::Load) {
                let target = load.node_id.as_str();
                if !visited.contains(target) || target == source.node_id.as_str() {
                    continue;
                }

                let mut node_ids = vec![target.to_owned()];
                let mut edge_ids = Vec::new();
                let mut cursor = target.to_owned();

                while cursor != source.node_id {
                    let Some((prior, edge)) = predecessor.get(&cursor).cloned() else {
                        break;
                    };
                    node_ids.push(prior.clone());
                    edge_ids.push(edge.edge_id.clone());
                    cursor = prior;
                }

                if cursor != source.node_id {
                    continue;
                }

                node_ids.reverse();
                edge_ids.reverse();

                let node_evidence = node_ids
                    .iter()
                    .filter_map(|id| self.node(id))
                    .map(|node| node.evidence.clone())
                    .collect();
                let node_module_identities = node_ids
                    .iter()
                    .filter_map(|id| self.node(id))
                    .map(|node| node.module_identity.clone())
                    .collect();
                let edge_evidence = edge_ids
                    .iter()
                    .filter_map(|id| self.edges.iter().find(|edge| edge.edge_id == *id))
                    .map(|edge| edge.evidence.clone())
                    .collect();

                paths_by_load
                    .entry(target.to_owned())
                    .or_default()
                    .push(SourcePath {
                        source_id: source.node_id.clone(),
                        node_ids,
                        edge_ids,
                        node_evidence,
                        edge_evidence,
                        node_module_identities,
                    });
            }
        }

        let load_coverage = self
            .nodes
            .iter()
            .filter(|node| node.kind == ServiceNodeKind::Load)
            .map(|load| {
                let mut paths = paths_by_load
                    .remove(&load.node_id)
                    .unwrap_or_default();
                paths.sort_by(|left, right| left.source_id.cmp(&right.source_id));
                LoadSourceCoverage {
                    load_id: load.node_id.clone(),
                    status: if paths.is_empty() {
                        LoadCoverageStatus::NoDeclaredSourcePath
                    } else {
                        LoadCoverageStatus::DeclaredSourcePathFound
                    },
                    source_paths: paths,
                }
            })
            .collect();

        let stale_module_configurations = self
            .nodes
            .iter()
            .filter_map(|node| {
                node.module_identity
                    .as_ref()
                    .filter(|identity| !identity.configuration_current())
                    .map(|identity| StaleModuleConfiguration {
                        node_id: node.node_id.clone(),
                        module_id: identity.module_id().to_owned(),
                        configuration_digest: identity.configuration_digest().to_owned(),
                    })
            })
            .collect();

        SourceMapAssessment {
            graph_id: self.revision.graph_id.clone(),
            revision_id: self.revision.revision_id.clone(),
            content_digest: self.revision.content_digest.clone(),
            scope_statement: self.revision.scope_statement.clone(),
            graph_maturity: self.revision.maturity,
            graph_maturity_evidence: self.revision.maturity_evidence.clone(),
            revision_current: self.revision.current,
            stale_module_configurations,
            path_witness_policy: PathWitnessPolicy::OneShortestHopPerSourceLoadPair,
            load_coverage,
        }
    }
}

/// One path found through declared PowerPath edges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourcePath {
    source_id: String,
    node_ids: Vec<String>,
    edge_ids: Vec<String>,
    node_evidence: Vec<GraphEvidence>,
    edge_evidence: Vec<GraphEvidence>,
    node_module_identities: Vec<Option<ModuleIdentity>>,
}

impl SourcePath {
    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn node_ids(&self) -> &[String] {
        &self.node_ids
    }

    pub fn edge_ids(&self) -> &[String] {
        &self.edge_ids
    }

    pub fn node_evidence(&self) -> &[GraphEvidence] {
        &self.node_evidence
    }

    pub fn edge_evidence(&self) -> &[GraphEvidence] {
        &self.edge_evidence
    }

    /// Exact module/configuration records aligned with node_ids. None means
    /// that the graph node is a logical topology node, not a catalogued module.
    pub fn node_module_identities(&self) -> &[Option<ModuleIdentity>] {
        &self.node_module_identities
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LoadCoverageStatus {
    DeclaredSourcePathFound,
    NoDeclaredSourcePath,
}

/// Path witness policy used by this bounded reachability assessment.
///
/// One shortest-hop witness is retained per source/load pair. This is not an
/// enumeration of every redundant or edge-disjoint route in a meshed network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PathWitnessPolicy {
    OneShortestHopPerSourceLoadPair,
}

/// Coverage record for one declared load.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LoadSourceCoverage {
    load_id: String,
    status: LoadCoverageStatus,
    source_paths: Vec<SourcePath>,
}

impl LoadSourceCoverage {
    pub fn load_id(&self) -> &str {
        &self.load_id
    }

    pub const fn status(&self) -> LoadCoverageStatus {
        self.status
    }

    pub fn source_paths(&self) -> &[SourcePath] {
        &self.source_paths
    }
}

/// Human- and machine-readable result of declared source-path analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StaleModuleConfiguration {
    node_id: String,
    module_id: String,
    configuration_digest: String,
}

impl StaleModuleConfiguration {
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub fn module_id(&self) -> &str {
        &self.module_id
    }

    pub fn configuration_digest(&self) -> &str {
        &self.configuration_digest
    }
}

/// Human- and machine-readable result of declared source-path analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceMapAssessment {
    graph_id: String,
    revision_id: String,
    content_digest: String,
    scope_statement: String,
    graph_maturity: GraphMaturity,
    graph_maturity_evidence: GraphEvidence,
    revision_current: bool,
    stale_module_configurations: Vec<StaleModuleConfiguration>,
    path_witness_policy: PathWitnessPolicy,
    load_coverage: Vec<LoadSourceCoverage>,
}

impl SourceMapAssessment {
    pub fn graph_id(&self) -> &str {
        &self.graph_id
    }

    pub fn revision_id(&self) -> &str {
        &self.revision_id
    }

    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }

    pub fn scope_statement(&self) -> &str {
        &self.scope_statement
    }

    pub const fn graph_maturity(&self) -> GraphMaturity {
        self.graph_maturity
    }

    pub const fn graph_maturity_evidence(&self) -> &GraphEvidence {
        &self.graph_maturity_evidence
    }

    pub const fn revision_current(&self) -> bool {
        self.revision_current
    }

    pub fn stale_module_configurations(&self) -> &[StaleModuleConfiguration] {
        &self.stale_module_configurations
    }

    pub const fn path_witness_policy(&self) -> PathWitnessPolicy {
        self.path_witness_policy
    }

    pub fn load_coverage(&self) -> &[LoadSourceCoverage] {
        &self.load_coverage
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(state: GraphEvidenceState, id: &str) -> GraphEvidence {
        GraphEvidence::new(
            state,
            if state == GraphEvidenceState::Unknown {
                None
            } else {
                Some(id.to_owned())
            },
        )
        .unwrap()
    }

    fn revision(current: bool, maturity: GraphMaturity) -> GraphRevision {
        let maturity_evidence = match maturity {
            GraphMaturity::DesignOnly => evidence(
                GraphEvidenceState::DesignDeclared,
                "design-revision-evidence",
            ),
            GraphMaturity::FieldReconciled => evidence(
                GraphEvidenceState::FieldObserved,
                "field-reconciliation-record",
            ),
            GraphMaturity::IndependentlyReviewed => evidence(
                GraphEvidenceState::IndependentlyReviewed,
                "independent-review-record",
            ),
        };
        GraphRevision::new(
            "building-a",
            "rev-7",
            "sha256:graph-rev-7",
            "Declared building A distribution from utility/PV/battery inputs to listed critical loads",
            maturity,
            maturity_evidence,
            current,
        )
        .unwrap()
    }

    fn node(id: &str, kind: ServiceNodeKind) -> ServiceNode {
        ServiceNode::new(
            id,
            kind,
            format!("Label {id}"),
            evidence(GraphEvidenceState::DesignDeclared, &format!("evidence-{id}")),
        )
        .unwrap()
    }

    fn edge(id: &str, from: &str, to: &str, kind: ServiceEdgeKind) -> ServiceEdge {
        ServiceEdge::new(
            id,
            from,
            to,
            kind,
            evidence(GraphEvidenceState::DesignDeclared, &format!("evidence-{id}")),
        )
        .unwrap()
    }

    fn simple_graph() -> ServiceGraph {
        ServiceGraph::new(
            revision(true, GraphMaturity::DesignOnly),
            vec![
                node("utility", ServiceNodeKind::UtilitySource),
                node("converter", ServiceNodeKind::Converter),
                node("ac-bus", ServiceNodeKind::AcBus),
                node("circuit-1", ServiceNodeKind::Circuit),
                node("load-1", ServiceNodeKind::Load),
            ],
            vec![
                edge("e-1", "utility", "converter", ServiceEdgeKind::PowerPath),
                edge("e-2", "converter", "ac-bus", ServiceEdgeKind::PowerPath),
                edge("e-3", "ac-bus", "circuit-1", ServiceEdgeKind::PowerPath),
                edge("e-4", "circuit-1", "load-1", ServiceEdgeKind::PowerPath),
            ],
        )
        .unwrap()
    }

    #[test]
    fn finds_declared_source_path_and_retains_evidence() {
        let graph = simple_graph();
        let result = graph.assess_declared_source_paths();

        assert_eq!(result.graph_id(), "building-a");
        assert_eq!(result.graph_maturity(), GraphMaturity::DesignOnly);
        assert_eq!(
            result.path_witness_policy(),
            PathWitnessPolicy::OneShortestHopPerSourceLoadPair
        );
        assert!(result.revision_current());
        assert_eq!(result.load_coverage().len(), 1);

        let coverage = &result.load_coverage()[0];
        assert_eq!(coverage.load_id(), "load-1");
        assert_eq!(coverage.status(), LoadCoverageStatus::DeclaredSourcePathFound);
        assert_eq!(coverage.source_paths().len(), 1);

        let path = &coverage.source_paths()[0];
        assert_eq!(path.source_id(), "utility");
        assert_eq!(
            path.node_ids(),
            &[
                "utility".to_owned(),
                "converter".to_owned(),
                "ac-bus".to_owned(),
                "circuit-1".to_owned(),
                "load-1".to_owned()
            ]
        );
        assert_eq!(
            path.edge_ids(),
            &[
                "e-1".to_owned(),
                "e-2".to_owned(),
                "e-3".to_owned(),
                "e-4".to_owned()
            ]
        );
        assert_eq!(path.node_evidence().len(), 5);
        assert_eq!(path.edge_evidence().len(), 4);
    }

    #[test]
    fn measurement_and_protection_associations_do_not_create_power_paths() {
        let graph = ServiceGraph::new(
            revision(true, GraphMaturity::FieldReconciled),
            vec![
                node("utility", ServiceNodeKind::UtilitySource),
                node("meter", ServiceNodeKind::Meter),
                node("relay", ServiceNodeKind::ProtectiveDevice),
                node("load", ServiceNodeKind::Load),
            ],
            vec![
                edge("measurement", "utility", "meter", ServiceEdgeKind::MeasurementAssociation),
                edge("protection", "relay", "load", ServiceEdgeKind::ProtectionAssociation),
            ],
        )
        .unwrap();

        let result = graph.assess_declared_source_paths();
        assert_eq!(
            result.load_coverage()[0].status(),
            LoadCoverageStatus::NoDeclaredSourcePath
        );
        assert!(result.load_coverage()[0].source_paths().is_empty());
    }

    #[test]
    fn disconnected_load_is_not_claimed_to_be_deenergized() {
        let graph = ServiceGraph::new(
            revision(true, GraphMaturity::DesignOnly),
            vec![
                node("utility", ServiceNodeKind::UtilitySource),
                node("load", ServiceNodeKind::Load),
            ],
            Vec::new(),
        )
        .unwrap();

        let coverage = graph.assess_declared_source_paths().load_coverage()[0].clone();
        assert_eq!(coverage.status(), LoadCoverageStatus::NoDeclaredSourcePath);
        assert!(coverage.source_paths().is_empty());
        // This status only describes the graph; it contains no de-energized or
        // safe-to-touch assertion.
    }

    #[test]
    fn reports_multiple_declared_sources_for_one_load() {
        let graph = ServiceGraph::new(
            revision(true, GraphMaturity::DesignOnly),
            vec![
                node("utility", ServiceNodeKind::UtilitySource),
                node("battery", ServiceNodeKind::Storage),
                node("bus", ServiceNodeKind::DcBus),
                node("load", ServiceNodeKind::Load),
            ],
            vec![
                edge("a-utility", "utility", "bus", ServiceEdgeKind::PowerPath),
                edge("b-battery", "battery", "bus", ServiceEdgeKind::PowerPath),
                edge("c-load", "bus", "load", ServiceEdgeKind::PowerPath),
            ],
        )
        .unwrap();

        let result = graph.assess_declared_source_paths();
        let paths = result.load_coverage()[0].source_paths();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].source_id(), "battery");
        assert_eq!(paths[1].source_id(), "utility");
    }

    #[test]
    fn cycle_does_not_hang_or_duplicate_a_source_path() {
        let graph = ServiceGraph::new(
            revision(true, GraphMaturity::DesignOnly),
            vec![
                node("source", ServiceNodeKind::RenewableSource),
                node("bus-a", ServiceNodeKind::DcBus),
                node("bus-b", ServiceNodeKind::DcBus),
                node("load", ServiceNodeKind::Load),
            ],
            vec![
                edge("a", "source", "bus-a", ServiceEdgeKind::PowerPath),
                edge("b", "bus-a", "bus-b", ServiceEdgeKind::PowerPath),
                edge("c", "bus-b", "bus-a", ServiceEdgeKind::PowerPath),
                edge("d", "bus-b", "load", ServiceEdgeKind::PowerPath),
            ],
        )
        .unwrap();

        let result = graph.assess_declared_source_paths();
        assert_eq!(result.load_coverage()[0].source_paths().len(), 1);
        assert_eq!(
            result.load_coverage()[0].source_paths()[0].node_ids(),
            &[
                "source".to_owned(),
                "bus-a".to_owned(),
                "bus-b".to_owned(),
                "load".to_owned()
            ]
        );
    }

    #[test]
    fn stale_graph_revision_is_explicit_in_assessment() {
        let graph = ServiceGraph::new(
            revision(false, GraphMaturity::FieldReconciled),
            vec![node("source", ServiceNodeKind::UtilitySource), node("load", ServiceNodeKind::Load)],
            vec![edge("p", "source", "load", ServiceEdgeKind::PowerPath)],
        )
        .unwrap();

        let result = graph.assess_declared_source_paths();
        assert!(!result.revision_current());
        assert_eq!(result.graph_maturity(), GraphMaturity::FieldReconciled);
    }

    #[test]
    fn rejects_duplicate_node_ids() {
        let first = node("same", ServiceNodeKind::UtilitySource);
        let second = node("same", ServiceNodeKind::Load);
        assert!(matches!(
            ServiceGraph::new(revision(true, GraphMaturity::DesignOnly), vec![first, second], Vec::new()),
            Err(ServiceGraphError::DuplicateNodeId { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_edge_ids() {
        let first = edge("same", "source", "load", ServiceEdgeKind::PowerPath);
        let second = edge("same", "source", "load", ServiceEdgeKind::MeasurementAssociation);
        assert!(matches!(
            ServiceGraph::new(
                revision(true, GraphMaturity::DesignOnly),
                vec![node("source", ServiceNodeKind::UtilitySource), node("load", ServiceNodeKind::Load)],
                vec![first, second]
            ),
            Err(ServiceGraphError::DuplicateEdgeId { .. })
        ));
    }

    #[test]
    fn rejects_dangling_edge_endpoints() {
        assert!(matches!(
            ServiceGraph::new(
                revision(true, GraphMaturity::DesignOnly),
                vec![node("source", ServiceNodeKind::UtilitySource)],
                vec![edge("power", "source", "missing", ServiceEdgeKind::PowerPath)]
            ),
            Err(ServiceGraphError::DanglingEdgeEndpoint { .. })
        ));
    }

    #[test]
    fn rejects_meter_or_controller_as_power_path_node() {
        assert!(matches!(
            ServiceGraph::new(
                revision(true, GraphMaturity::DesignOnly),
                vec![
                    node("source", ServiceNodeKind::UtilitySource),
                    node("meter", ServiceNodeKind::Meter)
                ],
                vec![edge("power", "source", "meter", ServiceEdgeKind::PowerPath)]
            ),
            Err(ServiceGraphError::InvalidPowerPathNodeKind { .. })
        ));
    }

    #[test]
    fn a_load_cannot_originate_a_declared_power_path() {
        assert!(matches!(
            ServiceGraph::new(
                revision(true, GraphMaturity::DesignOnly),
                vec![
                    node("load-a", ServiceNodeKind::Load),
                    node("load-b", ServiceNodeKind::Load)
                ],
                vec![edge("bad", "load-a", "load-b", ServiceEdgeKind::PowerPath)]
            ),
            Err(ServiceGraphError::InvalidPowerPathOriginNodeKind { .. })
        ));
    }

    #[test]
    fn rejects_self_referential_edges() {
        assert!(matches!(
            ServiceEdge::new(
                "loop",
                "node-a",
                "node-a",
                ServiceEdgeKind::PowerPath,
                GraphEvidence::unknown()
            ),
            Err(ServiceGraphError::SelfReferentialEdge { .. })
        ));
    }

    #[test]
    fn graph_maturity_requires_matching_revision_evidence() {
        let declared = evidence(GraphEvidenceState::DesignDeclared, "design-only");
        assert!(matches!(
            GraphRevision::new(
                "building-a",
                "rev-1",
                "sha256:rev-1",
                "Declared scope",
                GraphMaturity::IndependentlyReviewed,
                declared,
                true
            ),
            Err(ServiceGraphError::GraphMaturityEvidenceInsufficient {
                maturity: GraphMaturity::IndependentlyReviewed,
                evidence_state: GraphEvidenceState::DesignDeclared
            })
        ));
    }

    #[test]
    fn rejects_unknown_evidence_with_an_identifier() {
        assert!(matches!(
            GraphEvidence::new(GraphEvidenceState::Unknown, Some("should-not-exist".to_owned())),
            Err(ServiceGraphError::UnexpectedEvidenceForUnknown)
        ));
    }

    #[test]
    fn unknown_graph_evidence_rejects_even_an_empty_supplied_identifier() {
        assert!(matches!(
            GraphEvidence::new(GraphEvidenceState::Unknown, Some(String::new())),
            Err(ServiceGraphError::UnexpectedEvidenceForUnknown)
        ));
    }

    #[test]
    fn graph_order_and_source_map_order_are_deterministic() {
        let first = ServiceGraph::new(
            revision(true, GraphMaturity::DesignOnly),
            vec![
                node("z-source", ServiceNodeKind::UtilitySource),
                node("a-source", ServiceNodeKind::Storage),
                node("load", ServiceNodeKind::Load),
            ],
            vec![
                edge("z-edge", "z-source", "load", ServiceEdgeKind::PowerPath),
                edge("a-edge", "a-source", "load", ServiceEdgeKind::PowerPath),
            ],
        )
        .unwrap();
        let second = ServiceGraph::new(
            revision(true, GraphMaturity::DesignOnly),
            vec![
                node("load", ServiceNodeKind::Load),
                node("a-source", ServiceNodeKind::Storage),
                node("z-source", ServiceNodeKind::UtilitySource),
            ],
            vec![
                edge("a-edge", "a-source", "load", ServiceEdgeKind::PowerPath),
                edge("z-edge", "z-source", "load", ServiceEdgeKind::PowerPath),
            ],
        )
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(
            first.assess_declared_source_paths(),
            second.assess_declared_source_paths()
        );
    }

    #[test]
    fn an_empty_graph_is_rejected() {
        assert!(matches!(
            ServiceGraph::new(revision(true, GraphMaturity::DesignOnly), Vec::new(), Vec::new()),
            Err(ServiceGraphError::EmptyGraph)
        ));
    }

    #[test]
    fn source_path_retains_exact_module_configurations() {
        let inverter_identity = ModuleIdentity::new(
            "inverter-asset-01",
            "hw-r3",
            "sha256:inverter-config-r3",
            true,
        )
        .unwrap();
        let graph = ServiceGraph::new(
            revision(true, GraphMaturity::DesignOnly),
            vec![
                node("source", ServiceNodeKind::UtilitySource),
                node("inverter", ServiceNodeKind::Converter)
                    .with_module_identity(inverter_identity.clone()),
                node("load", ServiceNodeKind::Load),
            ],
            vec![
                edge("a", "source", "inverter", ServiceEdgeKind::PowerPath),
                edge("b", "inverter", "load", ServiceEdgeKind::PowerPath),
            ],
        )
        .unwrap();

        let result = graph.assess_declared_source_paths();
        let path = &result.load_coverage()[0].source_paths()[0];
        assert_eq!(
            path.node_module_identities()[1].as_ref(),
            Some(&inverter_identity)
        );
        assert!(result.stale_module_configurations().is_empty());
    }

    #[test]
    fn stale_module_configuration_is_explicit_in_source_map_assessment() {
        let stale_identity = ModuleIdentity::new(
            "battery-asset-02",
            "hw-r1",
            "sha256:battery-old-config",
            false,
        )
        .unwrap();
        let graph = ServiceGraph::new(
            revision(true, GraphMaturity::FieldReconciled),
            vec![
                node("battery", ServiceNodeKind::Storage)
                    .with_module_identity(stale_identity),
                node("load", ServiceNodeKind::Load),
            ],
            vec![edge("battery-load", "battery", "load", ServiceEdgeKind::PowerPath)],
        )
        .unwrap();

        let result = graph.assess_declared_source_paths();
        assert_eq!(result.stale_module_configurations().len(), 1);
        assert_eq!(
            result.stale_module_configurations()[0].module_id(),
            "battery-asset-02"
        );
        assert_eq!(
            result.stale_module_configurations()[0].configuration_digest(),
            "sha256:battery-old-config"
        );
    }

    #[test]
    fn nodes_without_a_load_produce_an_empty_coverage_list() {
        let graph = ServiceGraph::new(
            revision(true, GraphMaturity::DesignOnly),
            vec![node("utility", ServiceNodeKind::UtilitySource)],
            Vec::new(),
        )
        .unwrap();

        assert!(graph.assess_declared_source_paths().load_coverage().is_empty());
    }
}
