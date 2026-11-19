//! Dependency-scoped invalidation for construction evidence.
//!
//! Graph edges point from a derived artifact to its prerequisites:
//! motion-envelope -> task-plan -> work-package -> source model.
//! A revision to a prerequisite invalidates only its transitive dependents.
//! Historical receipts are immutable; this module plans which current
//! evidence must be regenerated or requalified.
//!
//! This is a deterministic dependency model, not a cryptographic hashing
//! implementation, distributed consensus service, or safety authorization.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceNodeKind {
    SourceModel,
    SourceEntity,
    IdsSpecification,
    IdsEvaluation,
    CoordinateFrame,
    SurveyTransform,
    SurveyCalibration,
    StationQualification,
    RobotConfiguration,
    ToolConfiguration,
    PayloadConfiguration,
    MaterialBatch,
    WorkPackage,
    TaskPlan,
    MotionEnvelope,
    ExecutionEvidence,
    InspectionEvidence,
    AcceptedOutput,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceNode {
    pub node_id: String,
    pub kind: EvidenceNodeKind,
    /// Digest of exact node content/version; caller computes and verifies it.
    pub version_sha256: String,
}

/// dependent_node_id consumes or was derived from prerequisite_node_id.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EvidenceDependency {
    pub dependent_node_id: String,
    pub prerequisite_node_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DependencyGraphError {
    EmptyNodeId,
    DuplicateNodeId(String),
    InvalidNodeVersionDigest(String),
    UnknownDependentNode(String),
    UnknownPrerequisiteNode(String),
    SelfDependency(String),
    DuplicateDependency {
        dependent_node_id: String,
        prerequisite_node_id: String,
    },
    DependencyCycle(Vec<String>),
    UnknownNode(String),
    EmptyChangeSet,
    DuplicateChangedNode(String),
    InvalidChangedVersionDigest(String),
    UnchangedVersion(String),
    EmptyChangeReason(String),
    InvalidSnapshot(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeVersion {
    pub node_id: String,
    pub version_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceSnapshot {
    pub subject_node_id: String,
    pub subject_version_sha256: String,
    /// All transitive prerequisite versions, sorted by node ID.
    pub transitive_dependency_versions: Vec<NodeVersion>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRevisionChange {
    pub node_id: String,
    pub new_version_sha256: String,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequalificationAction {
    RequalifySourceEvidence,
    RevalidateIds,
    RequalifySurvey,
    RequalifyStation,
    RequalifyRobotOrTool,
    RecompileWorkPackage,
    RecompileTaskPlan,
    RecomputeMotionEnvelope,
    ReconcileExecutionEvidence,
    ReinspectPhysicalOutput,
    ReevaluateAcceptance,
    RequalifyMaterialTraceability,
    RequalifyOther,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidatedEvidenceNode {
    pub node_id: String,
    pub kind: EvidenceNodeKind,
    pub action: RequalificationAction,
    /// Direct prerequisites that changed or were themselves invalidated.
    pub affected_prerequisites: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencyInvalidationPlan {
    pub changed_nodes: Vec<EvidenceRevisionChange>,
    /// Dependents in deterministic prerequisite-before-dependent order.
    pub requalification_order: Vec<InvalidatedEvidenceNode>,
    /// Not in the changed set or its transitive dependent closure.
    pub unaffected_nodes: Vec<String>,
    /// Planning never deletes, rewrites or revokes historical receipts.
    pub historical_evidence_preserved: bool,
}

#[derive(Clone, Debug, Default)]
pub struct EvidenceDependencyGraph {
    nodes: BTreeMap<String, EvidenceNode>,
    prerequisites: BTreeMap<String, BTreeSet<String>>,
    dependents: BTreeMap<String, BTreeSet<String>>,
    topological_order: Vec<String>,
}

impl EvidenceDependencyGraph {
    /// Build a graph, rejecting malformed references, duplicate edges and
    /// cycles. Topological ordering is deterministic by node ID.
    pub fn new(
        nodes: Vec<EvidenceNode>,
        dependencies: Vec<EvidenceDependency>,
    ) -> Result<Self, DependencyGraphError> {
        let mut graph = Self::default();

        for node in nodes {
            if node.node_id.trim().is_empty() {
                return Err(DependencyGraphError::EmptyNodeId);
            }
            if !is_sha256(&node.version_sha256) {
                return Err(DependencyGraphError::InvalidNodeVersionDigest(
                    node.node_id,
                ));
            }
            let node_id = node.node_id.clone();
            if graph.nodes.insert(node_id.clone(), node).is_some() {
                return Err(DependencyGraphError::DuplicateNodeId(node_id));
            }
            graph.prerequisites.insert(node_id.clone(), BTreeSet::new());
            graph.dependents.insert(node_id, BTreeSet::new());
        }

        let mut seen_edges = BTreeSet::new();
        for edge in dependencies {
            if !graph.nodes.contains_key(&edge.dependent_node_id) {
                return Err(DependencyGraphError::UnknownDependentNode(
                    edge.dependent_node_id,
                ));
            }
            if !graph.nodes.contains_key(&edge.prerequisite_node_id) {
                return Err(DependencyGraphError::UnknownPrerequisiteNode(
                    edge.prerequisite_node_id,
                ));
            }
            if edge.dependent_node_id == edge.prerequisite_node_id {
                return Err(DependencyGraphError::SelfDependency(
                    edge.dependent_node_id,
                ));
            }
            if !seen_edges.insert(edge.clone()) {
                return Err(DependencyGraphError::DuplicateDependency {
                    dependent_node_id: edge.dependent_node_id,
                    prerequisite_node_id: edge.prerequisite_node_id,
                });
            }
            graph
                .prerequisites
                .get_mut(&edge.dependent_node_id)
                .expect("dependent node was checked above")
                .insert(edge.prerequisite_node_id.clone());
            graph
                .dependents
                .get_mut(&edge.prerequisite_node_id)
                .expect("prerequisite node was checked above")
                .insert(edge.dependent_node_id);
        }

        graph.topological_order = graph.compute_topological_order()?;
        Ok(graph)
    }

    pub fn node(&self, node_id: &str) -> Option<&EvidenceNode> {
        self.nodes.get(node_id)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn topological_order(&self) -> &[String] {
        &self.topological_order
    }

    /// Capture the subject version and every transitive prerequisite version.
    pub fn capture_snapshot(
        &self,
        subject_node_id: &str,
    ) -> Result<EvidenceSnapshot, DependencyGraphError> {
        let subject = self
            .nodes
            .get(subject_node_id)
            .ok_or_else(|| DependencyGraphError::UnknownNode(subject_node_id.to_owned()))?;
        let mut closure = BTreeSet::new();
        let mut pending = vec![subject_node_id.to_owned()];

        while let Some(node_id) = pending.pop() {
            let dependencies = self
                .prerequisites
                .get(&node_id)
                .ok_or_else(|| DependencyGraphError::UnknownNode(node_id.clone()))?;
            for dependency_id in dependencies {
                if closure.insert(dependency_id.clone()) {
                    pending.push(dependency_id.clone());
                }
            }
        }

        let transitive_dependency_versions = closure
            .into_iter()
            .map(|node_id| {
                let node = self
                    .nodes
                    .get(&node_id)
                    .expect("graph edges only reference known nodes");
                NodeVersion {
                    node_id,
                    version_sha256: node.version_sha256.clone(),
                }
            })
            .collect();

        Ok(EvidenceSnapshot {
            subject_node_id: subject.node_id.clone(),
            subject_version_sha256: subject.version_sha256.clone(),
            transitive_dependency_versions,
        })
    }

    /// Historical snapshots remain valid records but may no longer describe
    /// the current evidence basis after any transitive prerequisite changes.
    pub fn snapshot_is_current(
        &self,
        snapshot: &EvidenceSnapshot,
    ) -> Result<bool, DependencyGraphError> {
        if !is_sha256(&snapshot.subject_version_sha256) {
            return Err(DependencyGraphError::InvalidSnapshot(
                snapshot.subject_node_id.clone(),
            ));
        }
        let current = self
            .capture_snapshot(&snapshot.subject_node_id)
            .map_err(|_| {
                DependencyGraphError::InvalidSnapshot(snapshot.subject_node_id.clone())
            })?;
        if current.subject_version_sha256 != snapshot.subject_version_sha256 {
            return Ok(false);
        }
        if snapshot
            .transitive_dependency_versions
            .iter()
            .any(|version| !is_sha256(&version.version_sha256))
        {
            return Err(DependencyGraphError::InvalidSnapshot(
                snapshot.subject_node_id.clone(),
            ));
        }
        Ok(current.transitive_dependency_versions == snapshot.transitive_dependency_versions)
    }

    /// Plan a revision without changing graph state.
    pub fn plan_revision(
        &self,
        changes: &[EvidenceRevisionChange],
    ) -> Result<DependencyInvalidationPlan, DependencyGraphError> {
        let changed_nodes = self.validate_changes(changes)?;
        let changed_ids: BTreeSet<String> = changed_nodes
            .iter()
            .map(|change| change.node_id.clone())
            .collect();
        let mut invalidated_ids = BTreeSet::new();
        let mut pending: Vec<String> = changed_ids.iter().cloned().collect();

        while let Some(node_id) = pending.pop() {
            let downstream = self
                .dependents
                .get(&node_id)
                .ok_or_else(|| DependencyGraphError::UnknownNode(node_id.clone()))?;
            for dependent_id in downstream {
                if !changed_ids.contains(dependent_id)
                    && invalidated_ids.insert(dependent_id.clone())
                {
                    pending.push(dependent_id.clone());
                }
            }
        }

        let mut requalification_order = Vec::with_capacity(invalidated_ids.len());
        for node_id in &self.topological_order {
            if !invalidated_ids.contains(node_id) {
                continue;
            }
            let node = self
                .nodes
                .get(node_id)
                .expect("topological order contains known nodes");
            let affected_prerequisites = self
                .prerequisites
                .get(node_id)
                .expect("every node has a prerequisite set")
                .iter()
                .filter(|dependency_id| {
                    changed_ids.contains(*dependency_id) || invalidated_ids.contains(*dependency_id)
                })
                .cloned()
                .collect();
            requalification_order.push(InvalidatedEvidenceNode {
                node_id: node_id.clone(),
                kind: node.kind,
                action: action_for_kind(node.kind),
                affected_prerequisites,
            });
        }

        let unaffected_nodes = self
            .nodes
            .keys()
            .filter(|node_id| {
                !changed_ids.contains(*node_id) && !invalidated_ids.contains(*node_id)
            })
            .cloned()
            .collect();

        Ok(DependencyInvalidationPlan {
            changed_nodes,
            requalification_order,
            unaffected_nodes,
            historical_evidence_preserved: true,
        })
    }

    /// Apply upstream or derived-artifact version changes and return the
    /// invalidation plan. Dependent versions remain unchanged until regenerated.
    pub fn apply_revision(
        &mut self,
        changes: &[EvidenceRevisionChange],
    ) -> Result<DependencyInvalidationPlan, DependencyGraphError> {
        let plan = self.plan_revision(changes)?;
        for change in &plan.changed_nodes {
            self.nodes
                .get_mut(&change.node_id)
                .expect("change validation checked node existence")
                .version_sha256 = change.new_version_sha256.clone();
        }
        Ok(plan)
    }

    fn validate_changes(
        &self,
        changes: &[EvidenceRevisionChange],
    ) -> Result<Vec<EvidenceRevisionChange>, DependencyGraphError> {
        if changes.is_empty() {
            return Err(DependencyGraphError::EmptyChangeSet);
        }
        let mut seen = BTreeSet::new();
        let mut validated = Vec::with_capacity(changes.len());
        for change in changes {
            if !seen.insert(change.node_id.as_str()) {
                return Err(DependencyGraphError::DuplicateChangedNode(
                    change.node_id.clone(),
                ));
            }
            let current = self
                .nodes
                .get(&change.node_id)
                .ok_or_else(|| DependencyGraphError::UnknownNode(change.node_id.clone()))?;
            if !is_sha256(&change.new_version_sha256) {
                return Err(DependencyGraphError::InvalidChangedVersionDigest(
                    change.node_id.clone(),
                ));
            }
            if change.reason.trim().is_empty() {
                return Err(DependencyGraphError::EmptyChangeReason(
                    change.node_id.clone(),
                ));
            }
            if current.version_sha256 == change.new_version_sha256 {
                return Err(DependencyGraphError::UnchangedVersion(
                    change.node_id.clone(),
                ));
            }
            validated.push(change.clone());
        }
        validated.sort_by(|left, right| left.node_id.cmp(&right.node_id));
        Ok(validated)
    }

    fn compute_topological_order(&self) -> Result<Vec<String>, DependencyGraphError> {
        let mut remaining_dependencies: BTreeMap<String, usize> = self
            .prerequisites
            .iter()
            .map(|(node_id, dependencies)| (node_id.clone(), dependencies.len()))
            .collect();
        let mut ready: BTreeSet<String> = remaining_dependencies
            .iter()
            .filter(|(_, count)| **count == 0)
            .map(|(node_id, _)| node_id.clone())
            .collect();
        let mut order = Vec::with_capacity(self.nodes.len());

        while let Some(node_id) = ready.iter().next().cloned() {
            ready.remove(&node_id);
            order.push(node_id.clone());
            if let Some(dependents) = self.dependents.get(&node_id) {
                for dependent_id in dependents {
                    let count = remaining_dependencies
                        .get_mut(dependent_id)
                        .expect("all dependent nodes are known");
                    *count -= 1;
                    if *count == 0 {
                        ready.insert(dependent_id.clone());
                    }
                }
            }
        }

        if order.len() != self.nodes.len() {
            let cycle_nodes = self
                .nodes
                .keys()
                .filter(|node_id| !order.contains(node_id))
                .cloned()
                .collect();
            return Err(DependencyGraphError::DependencyCycle(cycle_nodes));
        }
        Ok(order)
    }
}

fn action_for_kind(kind: EvidenceNodeKind) -> RequalificationAction {
    match kind {
        EvidenceNodeKind::SourceModel | EvidenceNodeKind::SourceEntity => {
            RequalificationAction::RequalifySourceEvidence
        }
        EvidenceNodeKind::IdsSpecification | EvidenceNodeKind::IdsEvaluation => {
            RequalificationAction::RevalidateIds
        }
        EvidenceNodeKind::CoordinateFrame
        | EvidenceNodeKind::SurveyTransform
        | EvidenceNodeKind::SurveyCalibration => RequalificationAction::RequalifySurvey,
        EvidenceNodeKind::StationQualification => RequalificationAction::RequalifyStation,
        EvidenceNodeKind::RobotConfiguration
        | EvidenceNodeKind::ToolConfiguration
        | EvidenceNodeKind::PayloadConfiguration => {
            RequalificationAction::RequalifyRobotOrTool
        }
        EvidenceNodeKind::WorkPackage => RequalificationAction::RecompileWorkPackage,
        EvidenceNodeKind::TaskPlan => RequalificationAction::RecompileTaskPlan,
        EvidenceNodeKind::MotionEnvelope => RequalificationAction::RecomputeMotionEnvelope,
        EvidenceNodeKind::ExecutionEvidence => {
            RequalificationAction::ReconcileExecutionEvidence
        }
        EvidenceNodeKind::InspectionEvidence => {
            RequalificationAction::ReinspectPhysicalOutput
        }
        EvidenceNodeKind::AcceptedOutput => RequalificationAction::ReevaluateAcceptance,
        EvidenceNodeKind::MaterialBatch => {
            RequalificationAction::RequalifyMaterialTraceability
        }
        EvidenceNodeKind::Other => RequalificationAction::RequalifyOther,
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn node(id: &str, kind: EvidenceNodeKind, version: char) -> EvidenceNode {
        EvidenceNode {
            node_id: id.into(),
            kind,
            version_sha256: digest(version),
        }
    }

    fn edge(dependent: &str, prerequisite: &str) -> EvidenceDependency {
        EvidenceDependency {
            dependent_node_id: dependent.into(),
            prerequisite_node_id: prerequisite.into(),
        }
    }

    fn site_graph() -> EvidenceDependencyGraph {
        EvidenceDependencyGraph::new(
            vec![
                node("ifc-model", EvidenceNodeKind::SourceModel, '1'),
                node("wall-geometry", EvidenceNodeKind::SourceEntity, '2'),
                node("ids-spec", EvidenceNodeKind::IdsSpecification, '3'),
                node("ids-eval", EvidenceNodeKind::IdsEvaluation, '4'),
                node("site-frame", EvidenceNodeKind::CoordinateFrame, '5'),
                node("survey-transform", EvidenceNodeKind::SurveyTransform, '6'),
                node("station", EvidenceNodeKind::StationQualification, '7'),
                node("robot", EvidenceNodeKind::RobotConfiguration, '8'),
                node("tool", EvidenceNodeKind::ToolConfiguration, '9'),
                node("work-package", EvidenceNodeKind::WorkPackage, 'a'),
                node("task-plan", EvidenceNodeKind::TaskPlan, 'b'),
                node("motion-envelope", EvidenceNodeKind::MotionEnvelope, 'c'),
                node("execution", EvidenceNodeKind::ExecutionEvidence, 'd'),
                node("inspection", EvidenceNodeKind::InspectionEvidence, 'e'),
                node("accepted-panel", EvidenceNodeKind::AcceptedOutput, 'f'),
                node("unrelated-water-system", EvidenceNodeKind::MaterialBatch, 'a'),
            ],
            vec![
                edge("wall-geometry", "ifc-model"),
                edge("ids-eval", "ids-spec"),
                edge("ids-eval", "wall-geometry"),
                edge("work-package", "wall-geometry"),
                edge("work-package", "ids-eval"),
                edge("work-package", "site-frame"),
                edge("work-package", "survey-transform"),
                edge("task-plan", "work-package"),
                edge("task-plan", "robot"),
                edge("task-plan", "tool"),
                edge("task-plan", "station"),
                edge("motion-envelope", "task-plan"),
                edge("execution", "motion-envelope"),
                edge("inspection", "execution"),
                edge("accepted-panel", "inspection"),
            ],
        )
        .unwrap()
    }

    #[test]
    fn source_model_revision_invalidates_only_its_transitive_dependents() {
        let mut graph = site_graph();
        let old_acceptance = graph.capture_snapshot("accepted-panel").unwrap();
        let unrelated = graph.capture_snapshot("unrelated-water-system").unwrap();
        let plan = graph
            .apply_revision(&[EvidenceRevisionChange {
                node_id: "ifc-model".into(),
                new_version_sha256: digest('0'),
                reason: "new coordinated design revision".into(),
            }])
            .unwrap();

        let affected: BTreeSet<_> = plan
            .requalification_order
            .iter()
            .map(|item| item.node_id.as_str())
            .collect();
        assert_eq!(
            affected,
            BTreeSet::from([
                "wall-geometry",
                "ids-eval",
                "work-package",
                "task-plan",
                "motion-envelope",
                "execution",
                "inspection",
                "accepted-panel",
            ])
        );
        assert_eq!(
            plan.unaffected_nodes,
            vec![
                "ids-spec".to_owned(),
                "robot".to_owned(),
                "site-frame".to_owned(),
                "station".to_owned(),
                "survey-transform".to_owned(),
                "tool".to_owned(),
                "unrelated-water-system".to_owned(),
            ]
        );
        assert!(plan.historical_evidence_preserved);
        assert!(!graph.snapshot_is_current(&old_acceptance).unwrap());
        assert!(graph.snapshot_is_current(&unrelated).unwrap());
    }

    #[test]
    fn requalification_order_is_prerequisite_before_dependent() {
        let graph = site_graph();
        let plan = graph
            .plan_revision(&[EvidenceRevisionChange {
                node_id: "site-frame".into(),
                new_version_sha256: digest('0'),
                reason: "survey frame corrected".into(),
            }])
            .unwrap();
        let ids: Vec<_> = plan
            .requalification_order
            .iter()
            .map(|item| item.node_id.as_str())
            .collect();
        let positions: BTreeMap<_, _> = ids
            .iter()
            .enumerate()
            .map(|(index, id)| (*id, index))
            .collect();
        assert!(positions["work-package"] < positions["task-plan"]);
        assert!(positions["task-plan"] < positions["motion-envelope"]);
        assert!(positions["motion-envelope"] < positions["execution"]);
        assert!(positions["execution"] < positions["inspection"]);
        assert!(positions["inspection"] < positions["accepted-panel"]);
        assert_eq!(
            plan.requalification_order[0].action,
            RequalificationAction::RecompileWorkPackage
        );
    }

    #[test]
    fn two_simultaneous_revisions_share_one_invalidation_closure() {
        let graph = site_graph();
        let plan = graph
            .plan_revision(&[
                EvidenceRevisionChange {
                    node_id: "tool".into(),
                    new_version_sha256: digest('0'),
                    reason: "tool calibration correction".into(),
                },
                EvidenceRevisionChange {
                    node_id: "site-frame".into(),
                    new_version_sha256: digest('1'),
                    reason: "frame correction".into(),
                },
            ])
            .unwrap();
        let affected: BTreeSet<_> = plan
            .requalification_order
            .iter()
            .map(|item| item.node_id.as_str())
            .collect();
        assert!(affected.contains("work-package"));
        assert!(affected.contains("task-plan"));
        assert!(affected.contains("accepted-panel"));
        assert_eq!(
            plan.requalification_order
                .iter()
                .filter(|item| item.node_id == "task-plan")
                .count(),
            1
        );
    }

    #[test]
    fn stale_snapshot_is_current_only_after_dependent_artifact_is_reissued() {
        let mut graph = site_graph();
        let before = graph.capture_snapshot("motion-envelope").unwrap();
        graph
            .apply_revision(&[EvidenceRevisionChange {
                node_id: "tool".into(),
                new_version_sha256: digest('0'),
                reason: "new end effector".into(),
            }])
            .unwrap();
        assert!(!graph.snapshot_is_current(&before).unwrap());

        graph
            .apply_revision(&[EvidenceRevisionChange {
                node_id: "motion-envelope".into(),
                new_version_sha256: digest('1'),
                reason: "recomputed envelope".into(),
            }])
            .unwrap();
        let after_recompute = graph.capture_snapshot("motion-envelope").unwrap();
        assert!(graph.snapshot_is_current(&after_recompute).unwrap());
        assert!(!graph.snapshot_is_current(&before).unwrap());
    }

    #[test]
    fn dependency_cycles_and_duplicate_edges_are_rejected() {
        let cycle = EvidenceDependencyGraph::new(
            vec![
                node("a", EvidenceNodeKind::Other, '1'),
                node("b", EvidenceNodeKind::Other, '2'),
            ],
            vec![edge("a", "b"), edge("b", "a")],
        );
        assert!(matches!(
            cycle,
            Err(DependencyGraphError::DependencyCycle(_))
        ));

        let duplicate = EvidenceDependencyGraph::new(
            vec![
                node("a", EvidenceNodeKind::Other, '1'),
                node("b", EvidenceNodeKind::Other, '2'),
            ],
            vec![edge("b", "a"), edge("b", "a")],
        );
        assert_eq!(
            duplicate,
            Err(DependencyGraphError::DuplicateDependency {
                dependent_node_id: "b".into(),
                prerequisite_node_id: "a".into(),
            })
        );
    }

    #[test]
    fn revision_changes_fail_closed_on_unknown_duplicate_noop_or_malformed_digest() {
        let graph = site_graph();
        assert_eq!(
            graph.plan_revision(&[]),
            Err(DependencyGraphError::EmptyChangeSet)
        );
        assert_eq!(
            graph.plan_revision(&[
                EvidenceRevisionChange {
                    node_id: "tool".into(),
                    new_version_sha256: digest('0'),
                    reason: "first change".into(),
                },
                EvidenceRevisionChange {
                    node_id: "tool".into(),
                    new_version_sha256: digest('1'),
                    reason: "duplicate".into(),
                },
            ]),
            Err(DependencyGraphError::DuplicateChangedNode("tool".into()))
        );
        assert_eq!(
            graph.plan_revision(&[EvidenceRevisionChange {
                node_id: "missing".into(),
                new_version_sha256: digest('0'),
                reason: "unknown".into(),
            }]),
            Err(DependencyGraphError::UnknownNode("missing".into()))
        );
        assert_eq!(
            graph.plan_revision(&[EvidenceRevisionChange {
                node_id: "tool".into(),
                new_version_sha256: digest('9'),
                reason: "no-op".into(),
            }]),
            Err(DependencyGraphError::UnchangedVersion("tool".into()))
        );
        assert_eq!(
            graph.plan_revision(&[EvidenceRevisionChange {
                node_id: "tool".into(),
                new_version_sha256: "not-a-digest".into(),
                reason: "malformed".into(),
            }]),
            Err(DependencyGraphError::InvalidChangedVersionDigest(
                "tool".into()
            ))
        );
    }

    #[test]
    fn revision_requires_a_nonempty_auditable_reason() {
        let graph = site_graph();
        assert_eq!(
            graph.plan_revision(&[EvidenceRevisionChange {
                node_id: "tool".into(),
                new_version_sha256: digest('0'),
                reason: "   ".into(),
            }]),
            Err(DependencyGraphError::EmptyChangeReason("tool".into()))
        );
    }

    #[test]
    fn graph_rejects_unknown_edges_and_invalid_node_digests() {
        assert_eq!(
            EvidenceDependencyGraph::new(
                vec![node("known", EvidenceNodeKind::Other, '1')],
                vec![edge("missing", "known")],
            ),
            Err(DependencyGraphError::UnknownDependentNode("missing".into()))
        );
        assert_eq!(
            EvidenceDependencyGraph::new(
                vec![EvidenceNode {
                    node_id: "bad".into(),
                    kind: EvidenceNodeKind::Other,
                    version_sha256: "not-a-digest".into(),
                }],
                Vec::new(),
            ),
            Err(DependencyGraphError::InvalidNodeVersionDigest("bad".into()))
        );
    }

}
