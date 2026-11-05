//! Fail-closed, scoped change-impact evaluator for construction evidence graphs.
//!
//! This crate computes applicability/requalification decisions over a declared dependency
//! graph. It does not authenticate authorities, verify physical evidence, certify safety,
//! approve engineering, or grant robot motion/acceptance authority.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeKind {
    ApprovedDesignManifest,
    ModelElementOrComponent,
    MethodTemplate,
    TransformationRule,
    CompiledTaskGraph,
    RobotConfiguration,
    CapabilityMatchReceipt,
    SiteEpochOrOperatingDomain,
    ControlFrameOrSurveyNetwork,
    MeasurementSystemOrCalibration,
    ExecutionReceipt,
    PhysicalMeasurementOrObservation,
    InspectionOrAcceptanceReceipt,
    EnduranceOrMaintenanceEvidence,
    CostLedger,
    TransferEvaluation,
    SafetyOrPermissionReceipt,
    DerivedQualificationClaim,
    Other(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceNode {
    pub node_id: String,
    pub node_kind: NodeKind,
    pub revision_id: String,
    pub content_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactScope {
    pub project_id: String,
    pub site_id: Option<String>,
    pub work_package_id: Option<String>,
    pub frame_id: Option<String>,
    pub odd_id: Option<String>,
}

/// Global means all scopes. Exact uses explicit dimensions; an omitted optional dimension
/// is a wildcard at that hierarchy level, while conflicting populated dimensions are disjoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScopeSelector {
    Global,
    Exact(ExactScope),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChangeKind {
    DesignGeometryChanged,
    DesignSemanticsOrMaterialChanged,
    MethodOrSequenceChanged,
    TransformationRuleChanged,
    ModelSchemaOrMappingChanged,
    CoordinateFrameOrControlNetworkChanged,
    SiteEpochOrEnvironmentChanged,
    RobotSerialOrHardwareChanged,
    SensorOrToolChanged,
    FirmwareOrSoftwareOrModelChanged,
    CapabilityEvidenceExpiredOrRevised,
    CalibrationOrMeasurementChainChanged,
    AcceptancePredicateOrInspectionMethodChanged,
    SafetyOrPermissionAuthorityChanged,
    CostBasisOrRateChanged,
    SourceEvidenceRevokedOrCorrected,
    AdministrativeMetadataOnly,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImpactAction {
    Rebuild,
    Remeasure,
    Revalidate,
    Requalify,
    ReviewRequired,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImpactRule {
    pub rule_id: String,
    pub change_kind: ChangeKind,
    /// None is an explicit, reviewed non-triggering rule for this change kind.
    pub action: Option<ImpactAction>,
    /// Continue dependency propagation beyond the target node.
    pub propagate: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencyEdge {
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub dependency_kind: String,
    pub scope: ScopeSelector,
    /// Every listed kind has an explicit rule or an explicit non-triggering decision.
    pub covered_change_kinds: Vec<ChangeKind>,
    pub rules: Vec<ImpactRule>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceGraph {
    pub graph_id: String,
    pub graph_revision: String,
    pub rule_set_digest: String,
    pub closure_complete: bool,
    pub closure_graph_revision: String,
    pub closure_manifest_ref: Option<String>,
    pub closure_authority_ref: Option<String>,
    pub closure_valid_until_unix: Option<i64>,
    pub nodes: Vec<EvidenceNode>,
    /// Edges point from upstream dependency to downstream artifact/claim.
    pub edges: Vec<DependencyEdge>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeEvent {
    pub event_id: String,
    pub subject_node_id: String,
    pub prior_revision_id: String,
    pub new_revision_id: String,
    pub prior_digest: String,
    pub new_digest: String,
    pub change_kind: ChangeKind,
    pub scope: ScopeSelector,
    pub effective_at_unix: i64,
    pub authority_ref: Option<String>,
    pub source_evidence_ref: String,
    pub graph_revision: String,
    pub rule_set_digest: String,
    pub semantic_delta_known: bool,
    pub semantic_equivalence_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecisionStatus {
    ChangedSubject,
    Affected,
    ReviewRequired,
    UnknownImpact,
    Unaffected,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WitnessEffect {
    Triggered(ImpactAction),
    ExplicitlyNonTriggering,
    ScopeDisjoint,
    UnknownPolicy,
    UnknownUpstream,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImpactWitness {
    pub source_node_id: String,
    pub target_node_id: String,
    pub node_path: Vec<String>,
    pub edge_path: Vec<String>,
    pub rule_path: Vec<String>,
    pub effect: WitnessEffect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImpactDecision {
    pub node_id: String,
    pub status: DecisionStatus,
    /// Union of all required actions over every material path.
    pub actions: Vec<ImpactAction>,
    pub witnesses: Vec<ImpactWitness>,
    pub reasons: Vec<String>,
    pub unaffected_closure_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImpactReport {
    pub evaluation_id: String,
    pub graph_id: String,
    pub graph_revision: String,
    pub rule_set_digest: String,
    pub event_id: String,
    pub decisions: Vec<ImpactDecision>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImpactError {
    InvalidGraph(&'static str),
    DuplicateNodeId(String),
    DuplicateEdgeId(String),
    MissingNode(String),
    MissingEdgeEndpoint {
        edge_id: String,
        node_id: String,
    },
    DuplicateCoveredChangeKind {
        edge_id: String,
    },
    DuplicateRule {
        edge_id: String,
        change_kind: ChangeKind,
    },
    RuleNotDeclaredCovered {
        edge_id: String,
        change_kind: ChangeKind,
    },
    NonTriggeringRuleCannotPropagate {
        edge_id: String,
        rule_id: String,
    },
    DependencyCycle(Vec<String>),
    StaleInput(&'static str),
    InvalidChangeEvent(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScopeMatch {
    Match,
    Disjoint,
}

#[derive(Clone, Debug)]
struct TraversalState {
    node_id: String,
    node_path: Vec<String>,
    edge_path: Vec<String>,
    rule_path: Vec<String>,
}

#[derive(Clone, Debug, Default)]
struct DecisionAccumulator {
    actions: BTreeSet<ImpactAction>,
    witnesses: Vec<ImpactWitness>,
    reasons: BTreeSet<String>,
    unknown: bool,
}

impl EvidenceGraph {
    pub fn validate(&self) -> Result<Vec<String>, ImpactError> {
        for (field, value) in [
            ("graph_id", self.graph_id.as_str()),
            ("graph_revision", self.graph_revision.as_str()),
            ("rule_set_digest", self.rule_set_digest.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(ImpactError::InvalidGraph(field));
            }
        }

        if self.closure_graph_revision != self.graph_revision {
            return Err(ImpactError::InvalidGraph("closure_revision_mismatch"));
        }
        if self.closure_complete
            && (self
                .closure_manifest_ref
                .as_deref()
                .is_none_or(str::is_empty)
                || self
                    .closure_authority_ref
                    .as_deref()
                    .is_none_or(str::is_empty)
                || self.closure_valid_until_unix.is_none())
        {
            return Err(ImpactError::InvalidGraph(
                "complete_closure_requires_manifest_and_authority",
            ));
        }

        let mut node_ids = BTreeSet::new();
        for node in &self.nodes {
            if node.node_id.trim().is_empty()
                || node.revision_id.trim().is_empty()
                || node.content_digest.trim().is_empty()
            {
                return Err(ImpactError::InvalidGraph("incomplete_node_identity"));
            }
            if !node_ids.insert(node.node_id.as_str()) {
                return Err(ImpactError::DuplicateNodeId(node.node_id.clone()));
            }
        }
        if self.nodes.is_empty() {
            return Err(ImpactError::InvalidGraph("empty_graph"));
        }

        let mut edge_ids = BTreeSet::new();
        for edge in &self.edges {
            if edge.edge_id.trim().is_empty() || edge.dependency_kind.trim().is_empty() {
                return Err(ImpactError::InvalidGraph("incomplete_edge_identity"));
            }
            if let ScopeSelector::Exact(scope) = &edge.scope
                && (scope.project_id.trim().is_empty()
                    || [
                        scope.site_id.as_deref(),
                        scope.work_package_id.as_deref(),
                        scope.frame_id.as_deref(),
                        scope.odd_id.as_deref(),
                    ]
                    .into_iter()
                    .flatten()
                    .any(str::is_empty))
            {
                return Err(ImpactError::InvalidGraph("incomplete_edge_scope"));
            }
            if !edge_ids.insert(edge.edge_id.as_str()) {
                return Err(ImpactError::DuplicateEdgeId(edge.edge_id.clone()));
            }
            for endpoint in [&edge.from_node_id, &edge.to_node_id] {
                if !node_ids.contains(endpoint.as_str()) {
                    return Err(ImpactError::MissingEdgeEndpoint {
                        edge_id: edge.edge_id.clone(),
                        node_id: endpoint.clone(),
                    });
                }
            }
            if edge.from_node_id == edge.to_node_id {
                return Err(ImpactError::InvalidGraph("self_dependency"));
            }

            let mut covered = BTreeSet::new();
            for kind in &edge.covered_change_kinds {
                if !covered.insert(kind.clone()) {
                    return Err(ImpactError::DuplicateCoveredChangeKind {
                        edge_id: edge.edge_id.clone(),
                    });
                }
            }

            let mut rule_kinds = BTreeSet::new();
            for rule in &edge.rules {
                if rule.rule_id.trim().is_empty() {
                    return Err(ImpactError::InvalidGraph("empty_rule_id"));
                }
                if !rule_kinds.insert(&rule.change_kind) {
                    return Err(ImpactError::DuplicateRule {
                        edge_id: edge.edge_id.clone(),
                        change_kind: rule.change_kind.clone(),
                    });
                }
                if !covered.contains(&rule.change_kind) {
                    return Err(ImpactError::RuleNotDeclaredCovered {
                        edge_id: edge.edge_id.clone(),
                        change_kind: rule.change_kind.clone(),
                    });
                }
                if rule.action.is_none() && rule.propagate {
                    return Err(ImpactError::NonTriggeringRuleCannotPropagate {
                        edge_id: edge.edge_id.clone(),
                        rule_id: rule.rule_id.clone(),
                    });
                }
            }
        }

        self.topological_order()
    }

    pub fn topological_order(&self) -> Result<Vec<String>, ImpactError> {
        let mut indegree: BTreeMap<&str, usize> = self
            .nodes
            .iter()
            .map(|node| (node.node_id.as_str(), 0))
            .collect();
        let mut outgoing: BTreeMap<&str, Vec<&str>> = self
            .nodes
            .iter()
            .map(|node| (node.node_id.as_str(), Vec::new()))
            .collect();

        for edge in &self.edges {
            *indegree
                .get_mut(edge.to_node_id.as_str())
                .ok_or_else(|| ImpactError::MissingNode(edge.to_node_id.clone()))? += 1;
            outgoing
                .get_mut(edge.from_node_id.as_str())
                .ok_or_else(|| ImpactError::MissingNode(edge.from_node_id.clone()))?
                .push(edge.to_node_id.as_str());
        }
        for children in outgoing.values_mut() {
            children.sort_unstable();
        }

        let mut ready: BTreeSet<&str> = indegree
            .iter()
            .filter_map(|(node, degree)| (*degree == 0).then_some(*node))
            .collect();
        let mut order = Vec::with_capacity(self.nodes.len());

        while let Some(node_id) = ready.pop_first() {
            order.push(node_id.to_owned());
            if let Some(children) = outgoing.get(node_id) {
                for child in children {
                    let degree = indegree
                        .get_mut(child)
                        .ok_or_else(|| ImpactError::MissingNode((*child).to_owned()))?;
                    *degree -= 1;
                    if *degree == 0 {
                        ready.insert(child);
                    }
                }
            }
        }

        if order.len() != self.nodes.len() {
            let cyclic: Vec<String> = indegree
                .iter()
                .filter_map(|(node_id, degree)| (*degree > 0).then_some((*node_id).to_owned()))
                .collect();
            return Err(ImpactError::DependencyCycle(cyclic));
        }

        Ok(order)
    }
}

pub fn evaluate_change(
    graph: &EvidenceGraph,
    event: &ChangeEvent,
) -> Result<ImpactReport, ImpactError> {
    graph.validate()?;

    if event.event_id.trim().is_empty()
        || event.subject_node_id.trim().is_empty()
        || event.source_evidence_ref.trim().is_empty()
        || event.prior_revision_id.trim().is_empty()
        || event.new_revision_id.trim().is_empty()
        || event.prior_digest.trim().is_empty()
        || event.new_digest.trim().is_empty()
    {
        return Err(ImpactError::InvalidChangeEvent("incomplete_event_identity"));
    }
    if let ScopeSelector::Exact(scope) = &event.scope
        && (scope.project_id.trim().is_empty()
            || [
                scope.site_id.as_deref(),
                scope.work_package_id.as_deref(),
                scope.frame_id.as_deref(),
                scope.odd_id.as_deref(),
            ]
            .into_iter()
            .flatten()
            .any(str::is_empty))
    {
        return Err(ImpactError::InvalidChangeEvent("incomplete_event_scope"));
    }
    if event.graph_revision != graph.graph_revision {
        return Err(ImpactError::StaleInput("graph_revision_mismatch"));
    }
    if event.rule_set_digest != graph.rule_set_digest {
        return Err(ImpactError::StaleInput("rule_set_digest_mismatch"));
    }

    let nodes_by_id: BTreeMap<&str, &EvidenceNode> = graph
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect();
    let subject = nodes_by_id
        .get(event.subject_node_id.as_str())
        .ok_or_else(|| ImpactError::MissingNode(event.subject_node_id.clone()))?;

    if subject.revision_id != event.prior_revision_id
        || subject.content_digest != event.prior_digest
    {
        return Err(ImpactError::StaleInput(
            "subject_prior_revision_or_digest_mismatch",
        ));
    }
    if event.new_revision_id == event.prior_revision_id {
        return Err(ImpactError::InvalidChangeEvent("revision_did_not_advance"));
    }
    if event.new_digest == event.prior_digest
        && (event.change_kind != ChangeKind::AdministrativeMetadataOnly
            || event
                .semantic_equivalence_ref
                .as_deref()
                .is_none_or(str::is_empty))
    {
        return Err(ImpactError::InvalidChangeEvent(
            "unchanged_digest_requires_metadata_equivalence_evidence",
        ));
    }

    let mut accumulators: BTreeMap<String, DecisionAccumulator> = graph
        .nodes
        .iter()
        .map(|node| (node.node_id.clone(), DecisionAccumulator::default()))
        .collect();

    accumulators
        .get_mut(&event.subject_node_id)
        .expect("subject was validated")
        .reasons
        .insert("changed_subject".to_owned());

    let mut global_unknown_reasons = Vec::new();
    if !graph.closure_complete
        || graph
            .closure_valid_until_unix
            .is_some_and(|expiry| event.effective_at_unix > expiry)
    {
        global_unknown_reasons.push("dependency_closure_incomplete_or_expired".to_owned());
    }
    if event.authority_ref.as_deref().is_none_or(str::is_empty) {
        global_unknown_reasons.push("change_authority_reference_missing".to_owned());
    }
    if event.source_evidence_ref.trim().is_empty() {
        global_unknown_reasons.push("change_source_evidence_missing".to_owned());
    }
    if !event.semantic_delta_known {
        global_unknown_reasons.push("semantic_delta_unknown".to_owned());
    }
    if event.change_kind == ChangeKind::Unknown {
        global_unknown_reasons.push("change_kind_unknown".to_owned());
    }
    if event.change_kind == ChangeKind::AdministrativeMetadataOnly
        && event
            .semantic_equivalence_ref
            .as_deref()
            .is_none_or(str::is_empty)
    {
        global_unknown_reasons.push("metadata_equivalence_attestation_missing".to_owned());
    }

    if !global_unknown_reasons.is_empty() {
        // The event itself cannot be trusted or bounded. Without a valid authority,
        // semantic delta, current closure or equivalence proof, the declared scope
        // cannot support any downstream Unaffected decision. This is intentionally
        // more conservative than traversing only currently known dependency edges:
        // missing/invalid event semantics may be exactly what would expose a missing edge.
        for node in &graph.nodes {
            let acc = accumulators
                .get_mut(&node.node_id)
                .expect("every graph node has an accumulator");
            acc.unknown = true;
            if node.node_id == event.subject_node_id {
                acc.reasons.extend(global_unknown_reasons.iter().cloned());
            } else {
                acc.reasons
                    .insert("change_event_admissibility_or_scope_is_unknown".to_owned());
                acc.reasons.extend(global_unknown_reasons.iter().cloned());
            }
        }
    } else {
        let mut queue = VecDeque::new();
        queue.push_back(TraversalState {
            node_id: event.subject_node_id.clone(),
            node_path: vec![event.subject_node_id.clone()],
            edge_path: Vec::new(),
            rule_path: Vec::new(),
        });

        let outgoing = outgoing_edges(graph);
        let mut explored = 0_usize;
        while let Some(state) = queue.pop_front() {
            explored += 1;
            if explored > 100_000 {
                return Err(ImpactError::InvalidGraph(
                    "impact_path_exploration_limit_exceeded",
                ));
            }

            let Some(edges) = outgoing.get(state.node_id.as_str()) else {
                continue;
            };

            for edge in edges {
                let scope = scope_intersects(&event.scope, &edge.scope);
                let mut next_nodes = state.node_path.clone();
                next_nodes.push(edge.to_node_id.clone());
                let mut next_edges = state.edge_path.clone();
                next_edges.push(edge.edge_id.clone());

                if scope == ScopeMatch::Disjoint {
                    let witness = ImpactWitness {
                        source_node_id: event.subject_node_id.clone(),
                        target_node_id: edge.to_node_id.clone(),
                        node_path: next_nodes,
                        edge_path: next_edges,
                        rule_path: state.rule_path.clone(),
                        effect: WitnessEffect::ScopeDisjoint,
                    };
                    accumulators
                        .get_mut(&edge.to_node_id)
                        .expect("validated edge target")
                        .witnesses
                        .push(witness);
                    continue;
                }

                let rule = edge
                    .rules
                    .iter()
                    .find(|rule| rule.change_kind == event.change_kind);

                match rule {
                    Some(rule) => {
                        let mut next_rules = state.rule_path.clone();
                        next_rules.push(rule.rule_id.clone());
                        match &rule.action {
                            Some(action) => {
                                let witness = ImpactWitness {
                                    source_node_id: event.subject_node_id.clone(),
                                    target_node_id: edge.to_node_id.clone(),
                                    node_path: next_nodes.clone(),
                                    edge_path: next_edges.clone(),
                                    rule_path: next_rules.clone(),
                                    effect: WitnessEffect::Triggered(action.clone()),
                                };
                                let acc = accumulators
                                    .get_mut(&edge.to_node_id)
                                    .expect("validated edge target");
                                acc.actions.insert(action.clone());
                                acc.witnesses.push(witness);

                                if *action == ImpactAction::ReviewRequired {
                                    acc.reasons.insert(
                                        "authorized_review_required_by_edge_policy".to_owned(),
                                    );
                                }

                                if rule.propagate {
                                    if *action == ImpactAction::ReviewRequired {
                                        propagate_unknown(
                                            graph,
                                            TraversalState {
                                                node_id: edge.to_node_id.clone(),
                                                node_path: next_nodes,
                                                edge_path: next_edges,
                                                rule_path: next_rules,
                                            },
                                            &mut accumulators,
                                            "upstream_review_blocks_downstream_applicability",
                                        );
                                    } else {
                                        queue.push_back(TraversalState {
                                            node_id: edge.to_node_id.clone(),
                                            node_path: next_nodes,
                                            edge_path: next_edges,
                                            rule_path: next_rules,
                                        });
                                    }
                                }
                            }
                            None => {
                                let witness = ImpactWitness {
                                    source_node_id: event.subject_node_id.clone(),
                                    target_node_id: edge.to_node_id.clone(),
                                    node_path: next_nodes,
                                    edge_path: next_edges,
                                    rule_path: next_rules,
                                    effect: WitnessEffect::ExplicitlyNonTriggering,
                                };
                                accumulators
                                    .get_mut(&edge.to_node_id)
                                    .expect("validated edge target")
                                    .witnesses
                                    .push(witness);
                            }
                        }
                    }
                    None if edge.covered_change_kinds.contains(&event.change_kind) => {
                        let witness = ImpactWitness {
                            source_node_id: event.subject_node_id.clone(),
                            target_node_id: edge.to_node_id.clone(),
                            node_path: next_nodes,
                            edge_path: next_edges,
                            rule_path: state.rule_path.clone(),
                            effect: WitnessEffect::ExplicitlyNonTriggering,
                        };
                        accumulators
                            .get_mut(&edge.to_node_id)
                            .expect("validated edge target")
                            .witnesses
                            .push(witness);
                    }
                    None => {
                        let witness = ImpactWitness {
                            source_node_id: event.subject_node_id.clone(),
                            target_node_id: edge.to_node_id.clone(),
                            node_path: next_nodes.clone(),
                            edge_path: next_edges.clone(),
                            rule_path: state.rule_path.clone(),
                            effect: WitnessEffect::UnknownPolicy,
                        };
                        let acc = accumulators
                            .get_mut(&edge.to_node_id)
                            .expect("validated edge target");
                        acc.unknown = true;
                        acc.reasons
                            .insert("edge_does_not_cover_change_kind".to_owned());
                        acc.witnesses.push(witness);

                        propagate_unknown(
                            graph,
                            TraversalState {
                                node_id: edge.to_node_id.clone(),
                                node_path: next_nodes,
                                edge_path: next_edges,
                                rule_path: state.rule_path.clone(),
                            },
                            &mut accumulators,
                            "upstream_dependency_policy_unknown",
                        );
                    }
                }
            }
        }
    }

    let mut decisions = Vec::with_capacity(graph.nodes.len());
    for node in &graph.nodes {
        let acc = accumulators
            .remove(&node.node_id)
            .expect("every node has an accumulator");
        let status = if node.node_id == event.subject_node_id {
            DecisionStatus::ChangedSubject
        } else if acc.unknown {
            DecisionStatus::UnknownImpact
        } else if acc.actions.contains(&ImpactAction::ReviewRequired) {
            DecisionStatus::ReviewRequired
        } else if !acc.actions.is_empty() {
            DecisionStatus::Affected
        } else if graph.closure_complete {
            DecisionStatus::Unaffected
        } else {
            DecisionStatus::UnknownImpact
        };

        let mut witnesses = acc.witnesses;
        witnesses.sort_by(witness_order);
        witnesses.dedup();
        let reasons: Vec<String> = acc.reasons.into_iter().collect();
        let unaffected_closure_ref = if status == DecisionStatus::Unaffected {
            graph.closure_manifest_ref.clone()
        } else {
            None
        };
        decisions.push(ImpactDecision {
            node_id: node.node_id.clone(),
            status,
            actions: acc.actions.into_iter().collect(),
            witnesses,
            reasons,
            unaffected_closure_ref,
        });
    }
    decisions.sort_by(|left, right| left.node_id.cmp(&right.node_id));

    Ok(ImpactReport {
        evaluation_id: evaluation_identity(graph, event),
        graph_id: graph.graph_id.clone(),
        graph_revision: graph.graph_revision.clone(),
        rule_set_digest: graph.rule_set_digest.clone(),
        event_id: event.event_id.clone(),
        decisions,
    })
}

fn outgoing_edges(graph: &EvidenceGraph) -> BTreeMap<&str, Vec<&DependencyEdge>> {
    let mut outgoing: BTreeMap<&str, Vec<&DependencyEdge>> = BTreeMap::new();
    for edge in &graph.edges {
        outgoing
            .entry(edge.from_node_id.as_str())
            .or_default()
            .push(edge);
    }
    for edges in outgoing.values_mut() {
        edges.sort_by(|left, right| left.edge_id.cmp(&right.edge_id));
    }
    outgoing
}

fn scope_intersects(event: &ScopeSelector, edge: &ScopeSelector) -> ScopeMatch {
    match (event, edge) {
        (ScopeSelector::Global, _) | (_, ScopeSelector::Global) => ScopeMatch::Match,
        (ScopeSelector::Exact(event), ScopeSelector::Exact(edge)) => {
            if event.project_id != edge.project_id {
                return ScopeMatch::Disjoint;
            }
            for (left, right) in [
                (&event.site_id, &edge.site_id),
                (&event.work_package_id, &edge.work_package_id),
                (&event.frame_id, &edge.frame_id),
                (&event.odd_id, &edge.odd_id),
            ] {
                if let (Some(left), Some(right)) = (left, right)
                    && left != right
                {
                    return ScopeMatch::Disjoint;
                }
            }
            ScopeMatch::Match
        }
    }
}

fn propagate_unknown(
    graph: &EvidenceGraph,
    seed: TraversalState,
    accumulators: &mut BTreeMap<String, DecisionAccumulator>,
    reason: &str,
) {
    let outgoing = outgoing_edges(graph);
    let source_node_id = seed
        .node_path
        .first()
        .cloned()
        .unwrap_or_else(|| seed.node_id.clone());
    let mut queue = VecDeque::new();
    queue.push_back(seed);
    let mut explored = 0_usize;

    while let Some(state) = queue.pop_front() {
        explored += 1;
        if explored > 100_000 {
            for (node_id, accumulator) in accumulators.iter_mut() {
                if node_id != &source_node_id {
                    accumulator.unknown = true;
                    accumulator
                        .reasons
                        .insert("unknown_path_exploration_limit_exceeded".to_owned());
                }
            }
            break;
        }
        let Some(edges) = outgoing.get(state.node_id.as_str()) else {
            continue;
        };

        for edge in edges {
            // Once an upstream path is unknown, conservatively mark all graph descendants.
            // Scope narrowing cannot prove safety after a dependency rule itself is unknown.
            let mut node_path = state.node_path.clone();
            node_path.push(edge.to_node_id.clone());
            let mut edge_path = state.edge_path.clone();
            edge_path.push(edge.edge_id.clone());

            let witness = ImpactWitness {
                source_node_id: source_node_id.clone(),
                target_node_id: edge.to_node_id.clone(),
                node_path: node_path.clone(),
                edge_path: edge_path.clone(),
                rule_path: state.rule_path.clone(),
                effect: WitnessEffect::UnknownUpstream,
            };
            let acc = accumulators
                .get_mut(&edge.to_node_id)
                .expect("validated edge target");
            acc.unknown = true;
            acc.reasons.insert(reason.to_owned());
            acc.witnesses.push(witness);

            queue.push_back(TraversalState {
                node_id: edge.to_node_id.clone(),
                node_path,
                edge_path,
                rule_path: state.rule_path.clone(),
            });
        }
    }
}

fn witness_order(left: &ImpactWitness, right: &ImpactWitness) -> std::cmp::Ordering {
    (
        left.node_path.len(),
        left.node_path.as_slice(),
        left.edge_path.as_slice(),
        left.rule_path.as_slice(),
        &left.effect,
    )
        .cmp(&(
            right.node_path.len(),
            right.node_path.as_slice(),
            right.edge_path.as_slice(),
            right.rule_path.as_slice(),
            &right.effect,
        ))
}

fn evaluation_identity(graph: &EvidenceGraph, event: &ChangeEvent) -> String {
    let fields = vec![
        graph.graph_id.clone(),
        graph.graph_revision.clone(),
        graph.rule_set_digest.clone(),
        graph.closure_complete.to_string(),
        graph.closure_graph_revision.clone(),
        graph.closure_manifest_ref.clone().unwrap_or_default(),
        graph.closure_authority_ref.clone().unwrap_or_default(),
        graph
            .closure_valid_until_unix
            .map_or_else(String::new, |v| v.to_string()),
        event.event_id.clone(),
        event.subject_node_id.clone(),
        event.prior_revision_id.clone(),
        event.new_revision_id.clone(),
        event.prior_digest.clone(),
        event.new_digest.clone(),
        format!("{:?}", event.change_kind),
        format!("{:?}", event.scope),
        event.effective_at_unix.to_string(),
        event.authority_ref.clone().unwrap_or_default(),
        event.source_evidence_ref.clone(),
        event.semantic_delta_known.to_string(),
        event.semantic_equivalence_ref.clone().unwrap_or_default(),
    ];
    let mut output = String::from("impact:");
    for field in fields {
        output.push_str(&field.len().to_string());
        output.push(':');
        output.push_str(&field);
        output.push(':');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(site: Option<&str>) -> ScopeSelector {
        ScopeSelector::Exact(ExactScope {
            project_id: "project-1".to_owned(),
            site_id: site.map(str::to_owned),
            work_package_id: None,
            frame_id: None,
            odd_id: None,
        })
    }

    fn node(id: &str, kind: NodeKind) -> EvidenceNode {
        EvidenceNode {
            node_id: id.to_owned(),
            node_kind: kind,
            revision_id: "rev-1".to_owned(),
            content_digest: format!("digest-{id}-1"),
        }
    }

    fn rule(
        id: &str,
        kind: ChangeKind,
        action: Option<ImpactAction>,
        propagate: bool,
    ) -> ImpactRule {
        ImpactRule {
            rule_id: id.to_owned(),
            change_kind: kind,
            action,
            propagate,
        }
    }

    fn edge(
        id: &str,
        from: &str,
        to: &str,
        covered: Vec<ChangeKind>,
        rules: Vec<ImpactRule>,
        edge_scope: ScopeSelector,
    ) -> DependencyEdge {
        DependencyEdge {
            edge_id: id.to_owned(),
            from_node_id: from.to_owned(),
            to_node_id: to.to_owned(),
            dependency_kind: "semantic_input".to_owned(),
            scope: edge_scope,
            covered_change_kinds: covered,
            rules,
        }
    }

    fn graph(edges: Vec<DependencyEdge>) -> EvidenceGraph {
        EvidenceGraph {
            graph_id: "graph-1".to_owned(),
            graph_revision: "graph-rev-1".to_owned(),
            rule_set_digest: "rules-v1".to_owned(),
            closure_complete: true,
            closure_graph_revision: "graph-rev-1".to_owned(),
            closure_manifest_ref: Some("closure-manifest-1".to_owned()),
            closure_authority_ref: Some("closure-authority-1".to_owned()),
            closure_valid_until_unix: Some(10_000),
            nodes: vec![
                node("source", NodeKind::ApprovedDesignManifest),
                node("task", NodeKind::CompiledTaskGraph),
                node("execution", NodeKind::ExecutionReceipt),
                node("claim", NodeKind::DerivedQualificationClaim),
                node("raw", NodeKind::PhysicalMeasurementOrObservation),
                node("cost", NodeKind::CostLedger),
                node("site-b", NodeKind::PhysicalMeasurementOrObservation),
            ],
            edges,
        }
    }

    fn event(kind: ChangeKind, scope: ScopeSelector) -> ChangeEvent {
        ChangeEvent {
            event_id: "event-1".to_owned(),
            subject_node_id: "source".to_owned(),
            prior_revision_id: "rev-1".to_owned(),
            new_revision_id: "rev-2".to_owned(),
            prior_digest: "digest-source-1".to_owned(),
            new_digest: "digest-source-2".to_owned(),
            change_kind: kind,
            scope,
            effective_at_unix: 100,
            authority_ref: Some("change-authority-1".to_owned()),
            source_evidence_ref: "change-record-1".to_owned(),
            graph_revision: "graph-rev-1".to_owned(),
            rule_set_digest: "rules-v1".to_owned(),
            semantic_delta_known: true,
            semantic_equivalence_ref: None,
        }
    }

    fn status(report: &ImpactReport, id: &str) -> DecisionStatus {
        report
            .decisions
            .iter()
            .find(|decision| decision.node_id == id)
            .expect("decision exists")
            .status
            .clone()
    }

    fn decision<'a>(report: &'a ImpactReport, id: &str) -> &'a ImpactDecision {
        report
            .decisions
            .iter()
            .find(|decision| decision.node_id == id)
            .expect("decision exists")
    }

    #[test]
    fn triggered_change_rebuilds_dependent_graph_and_claim() {
        let kind = ChangeKind::DesignGeometryChanged;
        let graph = graph(vec![
            edge(
                "e1",
                "source",
                "task",
                vec![kind.clone()],
                vec![rule("r1", kind.clone(), Some(ImpactAction::Rebuild), true)],
                ScopeSelector::Global,
            ),
            edge(
                "e2",
                "task",
                "claim",
                vec![kind.clone()],
                vec![rule(
                    "r2",
                    kind.clone(),
                    Some(ImpactAction::Revalidate),
                    true,
                )],
                ScopeSelector::Global,
            ),
        ]);
        let report = evaluate_change(&graph, &event(kind, ScopeSelector::Global)).unwrap();
        assert_eq!(status(&report, "task"), DecisionStatus::Affected);
        assert_eq!(
            decision(&report, "task").actions,
            vec![ImpactAction::Rebuild]
        );
        assert_eq!(status(&report, "claim"), DecisionStatus::Affected);
        assert_eq!(
            decision(&report, "claim").actions,
            vec![ImpactAction::Revalidate]
        );
    }

    #[test]
    fn explicit_nontriggering_rule_proves_unaffected_with_closure_witness() {
        let kind = ChangeKind::CostBasisOrRateChanged;
        let graph = graph(vec![edge(
            "e1",
            "source",
            "raw",
            vec![kind.clone()],
            vec![rule("r1", kind.clone(), None, false)],
            ScopeSelector::Global,
        )]);
        let report = evaluate_change(&graph, &event(kind, ScopeSelector::Global)).unwrap();
        assert_eq!(status(&report, "raw"), DecisionStatus::Unaffected);
        assert_eq!(
            decision(&report, "raw").unaffected_closure_ref.as_deref(),
            Some("closure-manifest-1")
        );
        assert!(
            decision(&report, "raw")
                .witnesses
                .iter()
                .any(|w| { w.effect == WitnessEffect::ExplicitlyNonTriggering })
        );
    }

    #[test]
    fn missing_change_kind_coverage_is_unknown_not_unaffected() {
        let graph = graph(vec![edge(
            "e1",
            "source",
            "task",
            vec![ChangeKind::CostBasisOrRateChanged],
            Vec::new(),
            ScopeSelector::Global,
        )]);
        let report = evaluate_change(
            &graph,
            &event(ChangeKind::DesignGeometryChanged, ScopeSelector::Global),
        )
        .unwrap();
        assert_eq!(status(&report, "task"), DecisionStatus::UnknownImpact);
    }

    #[test]
    fn incomplete_dependency_closure_forbids_unaffected() {
        let mut input = graph(Vec::new());
        input.closure_complete = false;
        let report = evaluate_change(
            &input,
            &event(ChangeKind::DesignGeometryChanged, ScopeSelector::Global),
        )
        .unwrap();
        assert_eq!(status(&report, "task"), DecisionStatus::UnknownImpact);
        assert_eq!(status(&report, "claim"), DecisionStatus::UnknownImpact);
    }

    #[test]
    fn stale_prior_digest_is_rejected() {
        let graph = graph(Vec::new());
        let mut input = event(ChangeKind::DesignGeometryChanged, ScopeSelector::Global);
        input.prior_digest = "wrong-digest".to_owned();
        assert_eq!(
            evaluate_change(&graph, &input),
            Err(ImpactError::StaleInput(
                "subject_prior_revision_or_digest_mismatch"
            ))
        );
    }

    #[test]
    fn stale_graph_revision_is_rejected() {
        let graph = graph(Vec::new());
        let mut input = event(ChangeKind::DesignGeometryChanged, ScopeSelector::Global);
        input.graph_revision = "old-graph".to_owned();
        assert_eq!(
            evaluate_change(&graph, &input),
            Err(ImpactError::StaleInput("graph_revision_mismatch"))
        );
    }

    #[test]
    fn stale_rule_set_digest_is_rejected() {
        let graph = graph(Vec::new());
        let mut input = event(ChangeKind::DesignGeometryChanged, ScopeSelector::Global);
        input.rule_set_digest = "old-rules".to_owned();
        assert_eq!(
            evaluate_change(&graph, &input),
            Err(ImpactError::StaleInput("rule_set_digest_mismatch"))
        );
    }

    #[test]
    fn metadata_only_same_digest_requires_equivalence_attestation() {
        let graph = graph(Vec::new());
        let mut input = event(
            ChangeKind::AdministrativeMetadataOnly,
            ScopeSelector::Global,
        );
        input.new_digest = input.prior_digest.clone();
        assert_eq!(
            evaluate_change(&graph, &input),
            Err(ImpactError::InvalidChangeEvent(
                "unchanged_digest_requires_metadata_equivalence_evidence"
            ))
        );

        input.semantic_equivalence_ref = Some("equivalence-1".to_owned());
        let report = evaluate_change(&graph, &input).unwrap();
        assert_eq!(status(&report, "task"), DecisionStatus::Unaffected);
    }

    #[test]
    fn metadata_only_missing_attestation_is_unknown_even_with_new_digest() {
        let graph = graph(Vec::new());
        let input = event(
            ChangeKind::AdministrativeMetadataOnly,
            ScopeSelector::Global,
        );
        let report = evaluate_change(&graph, &input).unwrap();
        assert_eq!(status(&report, "task"), DecisionStatus::UnknownImpact);
    }

    #[test]
    fn global_scope_change_matches_exact_scoped_edge() {
        let kind = ChangeKind::SiteEpochOrEnvironmentChanged;
        let graph = graph(vec![edge(
            "e1",
            "source",
            "task",
            vec![kind.clone()],
            vec![rule(
                "r1",
                kind.clone(),
                Some(ImpactAction::Requalify),
                true,
            )],
            scope(Some("site-a")),
        )]);
        let report = evaluate_change(&graph, &event(kind, ScopeSelector::Global)).unwrap();
        assert_eq!(status(&report, "task"), DecisionStatus::Affected);
    }

    #[test]
    fn site_a_event_does_not_trigger_site_b_edge() {
        let kind = ChangeKind::SiteEpochOrEnvironmentChanged;
        let graph = graph(vec![edge(
            "e1",
            "source",
            "site-b",
            vec![kind.clone()],
            vec![rule(
                "r1",
                kind.clone(),
                Some(ImpactAction::Requalify),
                true,
            )],
            scope(Some("site-b")),
        )]);
        let report = evaluate_change(&graph, &event(kind, scope(Some("site-a")))).unwrap();
        assert_eq!(status(&report, "site-b"), DecisionStatus::Unaffected);
        assert!(
            decision(&report, "site-b")
                .witnesses
                .iter()
                .any(|w| w.effect == WitnessEffect::ScopeDisjoint)
        );
    }

    #[test]
    fn multiple_paths_union_all_required_actions() {
        let kind = ChangeKind::SensorOrToolChanged;
        let graph = graph(vec![
            edge(
                "e1",
                "source",
                "task",
                vec![kind.clone()],
                vec![rule(
                    "r1",
                    kind.clone(),
                    Some(ImpactAction::Revalidate),
                    true,
                )],
                ScopeSelector::Global,
            ),
            edge(
                "e2",
                "source",
                "execution",
                vec![kind.clone()],
                vec![rule(
                    "r2",
                    kind.clone(),
                    Some(ImpactAction::Remeasure),
                    true,
                )],
                ScopeSelector::Global,
            ),
            edge(
                "e3",
                "task",
                "claim",
                vec![kind.clone()],
                vec![rule(
                    "r3",
                    kind.clone(),
                    Some(ImpactAction::Requalify),
                    false,
                )],
                ScopeSelector::Global,
            ),
            edge(
                "e4",
                "execution",
                "claim",
                vec![kind.clone()],
                vec![rule("r4", kind.clone(), Some(ImpactAction::Rebuild), false)],
                ScopeSelector::Global,
            ),
        ]);
        let report = evaluate_change(&graph, &event(kind, ScopeSelector::Global)).unwrap();
        assert_eq!(
            decision(&report, "claim").actions,
            vec![ImpactAction::Rebuild, ImpactAction::Requalify]
        );
        assert_eq!(decision(&report, "claim").witnesses.len(), 2);
    }

    #[test]
    fn unknown_policy_propagates_to_descendants() {
        let kind = ChangeKind::DesignGeometryChanged;
        let graph = graph(vec![
            edge(
                "e1",
                "source",
                "task",
                Vec::new(),
                Vec::new(),
                ScopeSelector::Global,
            ),
            edge(
                "e2",
                "task",
                "claim",
                vec![kind.clone()],
                vec![rule("r2", kind.clone(), Some(ImpactAction::Rebuild), true)],
                ScopeSelector::Global,
            ),
        ]);
        let report = evaluate_change(&graph, &event(kind, ScopeSelector::Global)).unwrap();
        assert_eq!(status(&report, "task"), DecisionStatus::UnknownImpact);
        assert_eq!(status(&report, "claim"), DecisionStatus::UnknownImpact);
    }

    #[test]
    fn review_required_dominates_action_only_paths() {
        let kind = ChangeKind::FirmwareOrSoftwareOrModelChanged;
        let graph = graph(vec![
            edge(
                "e1",
                "source",
                "task",
                vec![kind.clone()],
                vec![rule(
                    "r1",
                    kind.clone(),
                    Some(ImpactAction::ReviewRequired),
                    true,
                )],
                ScopeSelector::Global,
            ),
            edge(
                "e2",
                "source",
                "execution",
                vec![kind.clone()],
                vec![rule(
                    "r2",
                    kind.clone(),
                    Some(ImpactAction::Revalidate),
                    true,
                )],
                ScopeSelector::Global,
            ),
            edge(
                "e3",
                "execution",
                "task",
                vec![kind.clone()],
                vec![rule("r3", kind.clone(), Some(ImpactAction::Rebuild), false)],
                ScopeSelector::Global,
            ),
        ]);
        let report = evaluate_change(&graph, &event(kind, ScopeSelector::Global)).unwrap();
        assert_eq!(status(&report, "task"), DecisionStatus::ReviewRequired);
        assert!(
            decision(&report, "task")
                .actions
                .contains(&ImpactAction::ReviewRequired)
        );
        assert!(
            decision(&report, "task")
                .actions
                .contains(&ImpactAction::Rebuild)
        );
    }

    #[test]
    fn rule_change_kind_must_be_declared_covered() {
        let graph = graph(vec![edge(
            "e1",
            "source",
            "task",
            vec![ChangeKind::CostBasisOrRateChanged],
            vec![rule(
                "r1",
                ChangeKind::DesignGeometryChanged,
                Some(ImpactAction::Rebuild),
                true,
            )],
            ScopeSelector::Global,
        )]);
        assert!(matches!(
            graph.validate(),
            Err(ImpactError::RuleNotDeclaredCovered { .. })
        ));
    }

    #[test]
    fn nontriggering_rule_cannot_continue_propagation() {
        let graph = graph(vec![edge(
            "e1",
            "source",
            "task",
            vec![ChangeKind::CostBasisOrRateChanged],
            vec![rule("r1", ChangeKind::CostBasisOrRateChanged, None, true)],
            ScopeSelector::Global,
        )]);
        assert!(matches!(
            graph.validate(),
            Err(ImpactError::NonTriggeringRuleCannotPropagate { .. })
        ));
    }

    #[test]
    fn cycles_are_rejected() {
        let kind = ChangeKind::DesignGeometryChanged;
        let graph = graph(vec![
            edge(
                "e1",
                "source",
                "task",
                vec![kind.clone()],
                vec![rule("r1", kind.clone(), Some(ImpactAction::Rebuild), true)],
                ScopeSelector::Global,
            ),
            edge(
                "e2",
                "task",
                "source",
                vec![kind.clone()],
                vec![rule("r2", kind.clone(), Some(ImpactAction::Rebuild), true)],
                ScopeSelector::Global,
            ),
        ]);
        assert!(matches!(
            graph.validate(),
            Err(ImpactError::DependencyCycle(_))
        ));
    }

    #[test]
    fn parallel_dependency_edges_do_not_create_a_false_cycle() {
        let kind = ChangeKind::DesignGeometryChanged;
        let graph = graph(vec![
            edge(
                "e1",
                "source",
                "task",
                vec![kind.clone()],
                vec![rule("r1", kind.clone(), Some(ImpactAction::Rebuild), false)],
                ScopeSelector::Global,
            ),
            edge(
                "e2",
                "source",
                "task",
                vec![kind.clone()],
                vec![rule(
                    "r2",
                    kind.clone(),
                    Some(ImpactAction::Revalidate),
                    false,
                )],
                ScopeSelector::Global,
            ),
        ]);
        assert!(graph.validate().is_ok());
        let report = evaluate_change(&graph, &event(kind, ScopeSelector::Global)).unwrap();
        assert_eq!(
            decision(&report, "task").actions,
            vec![ImpactAction::Rebuild, ImpactAction::Revalidate]
        );
    }

    #[test]
    fn multiple_unknown_paths_are_retained_at_convergent_claim() {
        let kind = ChangeKind::DesignGeometryChanged;
        let graph = graph(vec![
            edge(
                "e1",
                "source",
                "task",
                vec![],
                vec![],
                ScopeSelector::Global,
            ),
            edge(
                "e2",
                "source",
                "execution",
                vec![],
                vec![],
                ScopeSelector::Global,
            ),
            edge("e3", "task", "claim", vec![], vec![], ScopeSelector::Global),
            edge(
                "e4",
                "execution",
                "claim",
                vec![],
                vec![],
                ScopeSelector::Global,
            ),
        ]);
        let report = evaluate_change(&graph, &event(kind, ScopeSelector::Global)).unwrap();
        assert_eq!(status(&report, "claim"), DecisionStatus::UnknownImpact);
        assert_eq!(decision(&report, "claim").witnesses.len(), 2);
    }

    #[test]
    fn duplicate_node_ids_are_rejected() {
        let mut graph = graph(Vec::new());
        graph.nodes.push(graph.nodes[0].clone());
        assert!(matches!(
            graph.validate(),
            Err(ImpactError::DuplicateNodeId(_))
        ));
    }

    #[test]
    fn duplicate_edge_ids_are_rejected() {
        let kind = ChangeKind::DesignGeometryChanged;
        let e = edge(
            "same-id",
            "source",
            "task",
            vec![kind.clone()],
            vec![rule("r1", kind.clone(), Some(ImpactAction::Rebuild), false)],
            ScopeSelector::Global,
        );
        let graph = graph(vec![e.clone(), e]);
        assert!(matches!(
            graph.validate(),
            Err(ImpactError::DuplicateEdgeId(_))
        ));
    }

    #[test]
    fn closure_attestation_must_bind_the_graph_revision() {
        let mut graph = graph(Vec::new());
        graph.closure_graph_revision = "old-revision".to_owned();
        assert_eq!(
            graph.validate(),
            Err(ImpactError::InvalidGraph("closure_revision_mismatch"))
        );
    }

    #[test]
    fn complete_closure_requires_authority_and_manifest() {
        let mut graph = graph(Vec::new());
        graph.closure_authority_ref = None;
        assert_eq!(
            graph.validate(),
            Err(ImpactError::InvalidGraph(
                "complete_closure_requires_manifest_and_authority"
            ))
        );
    }

    #[test]
    fn expired_closure_forbids_unaffected() {
        let mut graph = graph(Vec::new());
        graph.closure_valid_until_unix = Some(50);
        let report = evaluate_change(
            &graph,
            &event(ChangeKind::DesignGeometryChanged, ScopeSelector::Global),
        )
        .unwrap();
        assert_eq!(status(&report, "task"), DecisionStatus::UnknownImpact);
    }

    #[test]
    fn missing_change_authority_makes_downstream_unknown() {
        let graph = graph(Vec::new());
        let mut input = event(ChangeKind::DesignGeometryChanged, ScopeSelector::Global);
        input.authority_ref = None;
        let report = evaluate_change(&graph, &input).unwrap();
        assert_eq!(status(&report, "task"), DecisionStatus::UnknownImpact);
    }

    #[test]
    fn unknown_semantic_delta_makes_downstream_unknown() {
        let graph = graph(Vec::new());
        let mut input = event(ChangeKind::DesignGeometryChanged, ScopeSelector::Global);
        input.semantic_delta_known = false;
        let report = evaluate_change(&graph, &input).unwrap();
        assert_eq!(status(&report, "claim"), DecisionStatus::UnknownImpact);
    }

    #[test]
    fn unknown_change_kind_makes_downstream_unknown() {
        let graph = graph(Vec::new());
        let input = event(ChangeKind::Unknown, ScopeSelector::Global);
        let report = evaluate_change(&graph, &input).unwrap();
        assert_eq!(status(&report, "task"), DecisionStatus::UnknownImpact);
    }

    #[test]
    fn unchanged_digest_for_non_metadata_change_is_invalid() {
        let graph = graph(Vec::new());
        let mut input = event(ChangeKind::DesignGeometryChanged, ScopeSelector::Global);
        input.new_digest = input.prior_digest.clone();
        assert_eq!(
            evaluate_change(&graph, &input),
            Err(ImpactError::InvalidChangeEvent(
                "unchanged_digest_requires_metadata_equivalence_evidence"
            ))
        );
    }

    #[test]
    fn deterministic_replay_produces_identical_report() {
        let kind = ChangeKind::MethodOrSequenceChanged;
        let graph = graph(vec![edge(
            "e1",
            "source",
            "task",
            vec![kind.clone()],
            vec![rule("r1", kind.clone(), Some(ImpactAction::Rebuild), true)],
            ScopeSelector::Global,
        )]);
        let event = event(kind, ScopeSelector::Global);
        assert_eq!(
            evaluate_change(&graph, &event),
            evaluate_change(&graph, &event)
        );
    }

    #[test]
    fn input_node_and_edge_permutations_preserve_report() {
        let kind = ChangeKind::DesignGeometryChanged;
        let e1 = edge(
            "e1",
            "source",
            "task",
            vec![kind.clone()],
            vec![rule("r1", kind.clone(), Some(ImpactAction::Rebuild), true)],
            ScopeSelector::Global,
        );
        let e2 = edge(
            "e2",
            "task",
            "claim",
            vec![kind.clone()],
            vec![rule(
                "r2",
                kind.clone(),
                Some(ImpactAction::Revalidate),
                false,
            )],
            ScopeSelector::Global,
        );
        let first = graph(vec![e1.clone(), e2.clone()]);
        let mut second = graph(vec![e2, e1]);
        second.nodes.reverse();
        assert_eq!(
            evaluate_change(&first, &event(kind.clone(), ScopeSelector::Global)),
            evaluate_change(&second, &event(kind, ScopeSelector::Global))
        );
    }

    #[test]
    fn indirect_path_witness_keeps_all_edges() {
        let kind = ChangeKind::MethodOrSequenceChanged;
        let graph = graph(vec![
            edge(
                "e1",
                "source",
                "task",
                vec![kind.clone()],
                vec![rule("r1", kind.clone(), Some(ImpactAction::Rebuild), true)],
                ScopeSelector::Global,
            ),
            edge(
                "e2",
                "task",
                "claim",
                vec![kind.clone()],
                vec![rule(
                    "r2",
                    kind.clone(),
                    Some(ImpactAction::Revalidate),
                    false,
                )],
                ScopeSelector::Global,
            ),
        ]);
        let report = evaluate_change(&graph, &event(kind, ScopeSelector::Global)).unwrap();
        let witness = &decision(&report, "claim").witnesses[0];
        assert_eq!(witness.node_path, vec!["source", "task", "claim"]);
        assert_eq!(witness.edge_path, vec!["e1", "e2"]);
        assert_eq!(witness.rule_path, vec!["r1", "r2"]);
    }

    #[test]
    fn unknown_path_has_unknown_witness() {
        let kind = ChangeKind::DesignGeometryChanged;
        let graph = graph(vec![edge(
            "e1",
            "source",
            "task",
            Vec::new(),
            Vec::new(),
            ScopeSelector::Global,
        )]);
        let report = evaluate_change(&graph, &event(kind, ScopeSelector::Global)).unwrap();
        assert!(
            decision(&report, "task")
                .witnesses
                .iter()
                .any(|w| { w.effect == WitnessEffect::UnknownPolicy })
        );
    }

    #[test]
    fn evaluation_identity_binds_policy_digest() {
        let kind = ChangeKind::DesignGeometryChanged;
        let graph_a = graph(Vec::new());
        let mut graph_b = graph(Vec::new());
        graph_b.rule_set_digest = "rules-v2".to_owned();
        let mut event_b = event(kind.clone(), ScopeSelector::Global);
        event_b.rule_set_digest = "rules-v2".to_owned();
        let a = evaluate_change(&graph_a, &event(kind.clone(), ScopeSelector::Global)).unwrap();
        let b = evaluate_change(&graph_b, &event_b).unwrap();
        assert_ne!(a.evaluation_id, b.evaluation_id);
    }

    #[test]
    fn event_revision_must_advance() {
        let graph = graph(Vec::new());
        let mut input = event(ChangeKind::DesignGeometryChanged, ScopeSelector::Global);
        input.new_revision_id = input.prior_revision_id.clone();
        assert_eq!(
            evaluate_change(&graph, &input),
            Err(ImpactError::InvalidChangeEvent("revision_did_not_advance"))
        );
    }

    #[test]
    fn duplicate_rules_for_same_change_kind_are_rejected() {
        let kind = ChangeKind::DesignGeometryChanged;
        let graph = graph(vec![edge(
            "e1",
            "source",
            "task",
            vec![kind.clone()],
            vec![
                rule("r1", kind.clone(), Some(ImpactAction::Rebuild), false),
                rule("r2", kind.clone(), Some(ImpactAction::Revalidate), false),
            ],
            ScopeSelector::Global,
        )]);
        assert!(matches!(
            graph.validate(),
            Err(ImpactError::DuplicateRule { .. })
        ));
    }

    #[test]
    fn complete_closure_requires_expiry() {
        let mut graph = graph(Vec::new());
        graph.closure_valid_until_unix = None;
        assert_eq!(
            graph.validate(),
            Err(ImpactError::InvalidGraph(
                "complete_closure_requires_manifest_and_authority"
            ))
        );
    }

    #[test]
    fn report_identity_binds_authority_and_event_scope() {
        let graph = graph(Vec::new());
        let first = event(ChangeKind::DesignGeometryChanged, scope(Some("site-a")));
        let mut second = first.clone();
        second.authority_ref = Some("different-authority".to_owned());
        let a = evaluate_change(&graph, &first).unwrap();
        let b = evaluate_change(&graph, &second).unwrap();
        assert_ne!(a.evaluation_id, b.evaluation_id);
    }

    #[test]
    fn evaluation_decisions_are_sorted_by_node_id() {
        let graph = graph(Vec::new());
        let report = evaluate_change(
            &graph,
            &event(ChangeKind::DesignGeometryChanged, ScopeSelector::Global),
        )
        .unwrap();
        let ids: Vec<&str> = report
            .decisions
            .iter()
            .map(|decision| decision.node_id.as_str())
            .collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted);
    }
}
