//! Dependency-aware construction change impact and requalification compiler.
//!
//! This pure reducer calculates which derived site artifacts no longer match
//! the current input versions after a source/site/equipment/policy change. It
//! preserves accepted historical evidence and reports it separately from
//! current artifacts that must be rebuilt or requalified.
//!
//! The graph is supplied by an upstream owner and must be complete for the
//! declared site/work-package snapshot. This module validates its structure,
//! digest syntax and version transitions; it does not verify signatures,
//! recompute source digests, mutate the SiteStateLedger, or authorize work.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DependencyDomain {
    SourceModel,
    IfcSchema,
    IdsSpecification,
    IdsEngineBuild,
    DesignRevision,
    EntityGeometry,
    CoordinateFrame,
    CoordinateTransform,
    AcceptanceProfile,
    SurveyControl,
    StationCalibration,
    RobotConfiguration,
    ToolConfiguration,
    PayloadConfiguration,
    TrajectoryConfiguration,
    ObstacleMap,
    DynamicActorManifest,
    SiteClockMap,
    OccupancyEngineBuild,
    ClearancePolicy,
    MaterialLot,
    WorkPackageSchedule,
    AuthorityPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DependencyKey {
    pub domain: DependencyDomain,
    /// Explicit scope, for example project:site-4, floor:02, or
    /// entity:ifc4x3:... . Localized scopes prevent global invalidation when
    /// only one design entity or site region changed.
    pub scope_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionedDependency {
    pub key: DependencyKey,
    pub observed_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencyTransition {
    pub key: DependencyKey,
    pub previous_sha256: String,
    pub current_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstructionChangeSet {
    pub change_id: String,
    /// Digest of the caller-verified change event / change package.
    pub source_event_sha256: String,
    pub changes: Vec<DependencyTransition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstructionArtifactKind {
    IfcIdsPreflight,
    DesignWorkPackage,
    StationQualification,
    MaterialReadiness,
    ProductionTask,
    RobotTrajectory,
    MotionEnvelope,
    SpaceTimeOccupancyAssessment,
    DispatchReadiness,
    InspectionPlan,
    AsBuiltMeasurement,
    AcceptedConstructionRecord,
    HandoverRecord,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactPersistence {
    /// Derived evidence that must be rebuilt/requalified when an input changes.
    Recomputable,
    /// Historical evidence is immutable. Its truth is not erased by a later
    /// revision, but the report records that its context has changed.
    HistoricalImmutable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstructionArtifact {
    pub artifact_id: String,
    pub artifact_sha256: String,
    pub kind: ConstructionArtifactKind,
    pub persistence: ArtifactPersistence,
    pub dependencies: Vec<VersionedDependency>,
    /// Other artifacts used to produce or qualify this artifact.
    pub depends_on_artifact_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstructionArtifactGraph {
    pub graph_snapshot_sha256: String,
    pub artifacts: Vec<ConstructionArtifact>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangeImpactError {
    EmptyChangeIdentity,
    InvalidDigest(&'static str),
    EmptyChangeSet,
    EmptyDependencyScope,
    DuplicateChangedDependency(DependencyKey),
    NoOpDependencyTransition(DependencyKey),
    EmptyArtifactId,
    DuplicateArtifactId(String),
    InvalidArtifactDigest(String),
    DuplicateArtifactDependency {
        artifact_id: String,
        key: DependencyKey,
    },
    InvalidArtifactDependencyDigest {
        artifact_id: String,
        key: DependencyKey,
    },
    DuplicateArtifactParent {
        artifact_id: String,
        parent_artifact_id: String,
    },
    SelfDependency(String),
    UnknownArtifactParent {
        artifact_id: String,
        parent_artifact_id: String,
    },
    DependencyCycle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactImpact {
    pub artifact_id: String,
    pub artifact_sha256: String,
    pub kind: ConstructionArtifactKind,
    /// Directly stale input versions. Empty for purely transitive invalidation.
    pub changed_dependencies: Vec<DependencyKey>,
    /// Derived or historical artifacts whose changed context propagated here.
    pub impacted_upstream_artifact_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeImpactReport {
    pub change_id: String,
    pub source_event_sha256: String,
    pub graph_snapshot_sha256: String,
    pub direct_invalidations: Vec<ArtifactImpact>,
    pub transitive_invalidations: Vec<ArtifactImpact>,
    /// Immutable accepted/inspection/handover records are retained, not
    /// rewritten or deleted. They are listed because their context changed.
    pub preserved_historical_records: Vec<ArtifactImpact>,
    /// Change keys that no artifact in the supplied graph declares. They
    /// require explicit reconciliation; their lack of dependents is not
    /// silently interpreted as proof of irrelevance.
    pub unmatched_change_keys: Vec<DependencyKey>,
    pub unaffected_artifact_ids: Vec<String>,
    /// Rebuild/requalification order: dependencies before their dependents.
    pub requalification_order: Vec<String>,
    pub source_records_mutated: bool,
}

impl ChangeImpactReport {
    pub fn has_unmatched_changes(&self) -> bool {
        !self.unmatched_change_keys.is_empty()
    }

    /// A graph with no unbound change keys has reconciled every change against
    /// its declared artifacts. This is not proof that the upstream graph is
    /// complete or that its evidence digests/signatures are authentic.
    pub fn change_keys_reconciled(&self) -> bool {
        self.unmatched_change_keys.is_empty()
    }

    pub fn invalidated_artifact_count(&self) -> usize {
        self.direct_invalidations.len() + self.transitive_invalidations.len()
    }
}

/// Calculate the minimal explicit invalidation closure for a change set.
///
/// Direct invalidation occurs when an artifact's recorded dependency version
/// differs from the change set's new/current version. The dependency's old
/// version is retained in the transition to make the change event auditable.
/// Downstream recomputable artifacts are transitively invalidated. Historical
/// records are never modified; they are emitted in
/// preserved_historical_records, and their downstream dependents are still
/// considered for impact because a current decision may rely on historical
/// evidence under a changed context.
///
/// No graph or site record is mutated.
pub fn compile_change_impact(
    graph: &ConstructionArtifactGraph,
    changes: &ConstructionChangeSet,
) -> Result<ChangeImpactReport, ChangeImpactError> {
    validate_change_set(changes)?;
    let topological_order = validate_and_topologically_sort(graph)?;

    let transition_by_key: BTreeMap<DependencyKey, &DependencyTransition> = changes
        .changes
        .iter()
        .map(|transition| (transition.key.clone(), transition))
        .collect();

    let mut direct_dependencies_by_artifact =
        BTreeMap::<&str, Vec<DependencyKey>>::new();
    let mut declared_change_keys = BTreeSet::new();

    for artifact in &graph.artifacts {
        let mut stale_dependencies = Vec::new();
        for dependency in &artifact.dependencies {
            if let Some(transition) = transition_by_key.get(&dependency.key) {
                declared_change_keys.insert(dependency.key.clone());
                if dependency.observed_sha256 != transition.current_sha256 {
                    stale_dependencies.push(dependency.key.clone());
                }
            }
        }
        if !stale_dependencies.is_empty() {
            stale_dependencies.sort();
            direct_dependencies_by_artifact.insert(
                artifact.artifact_id.as_str(),
                stale_dependencies,
            );
        }
    }

    let unmatched_change_keys: Vec<DependencyKey> = changes
        .changes
        .iter()
        .map(|transition| transition.key.clone())
        .filter(|key| !declared_change_keys.contains(key))
        .collect();

    let artifact_by_id: BTreeMap<&str, &ConstructionArtifact> = graph
        .artifacts
        .iter()
        .map(|artifact| (artifact.artifact_id.as_str(), artifact))
        .collect();

    let mut dependents_by_artifact = BTreeMap::<&str, Vec<&str>>::new();
    for artifact in &graph.artifacts {
        for parent_id in &artifact.depends_on_artifact_ids {
            dependents_by_artifact
                .entry(parent_id.as_str())
                .or_default()
                .push(artifact.artifact_id.as_str());
        }
    }
    for dependents in dependents_by_artifact.values_mut() {
        dependents.sort_unstable();
    }

    let mut impacted_ids = BTreeSet::<String>::new();
    let mut direct_ids = BTreeSet::<String>::new();
    let mut historical_ids = BTreeSet::<String>::new();
    let mut upstream_reasons = BTreeMap::<String, BTreeSet<String>>::new();

    let mut queue = VecDeque::<String>::new();
    for artifact_id in direct_dependencies_by_artifact.keys() {
        let artifact = artifact_by_id[*artifact_id];
        impacted_ids.insert((*artifact_id).to_owned());
        queue.push_back((*artifact_id).to_owned());
        match artifact.persistence {
            ArtifactPersistence::Recomputable => {
                direct_ids.insert((*artifact_id).to_owned());
            }
            ArtifactPersistence::HistoricalImmutable => {
                historical_ids.insert((*artifact_id).to_owned());
            }
        }
    }

    while let Some(upstream_id) = queue.pop_front() {
        if let Some(dependents) = dependents_by_artifact.get(upstream_id.as_str()) {
            for dependent_id in dependents {
                let dependent = artifact_by_id[*dependent_id];
                upstream_reasons
                    .entry((*dependent_id).to_owned())
                    .or_default()
                    .insert(upstream_id.clone());

                if impacted_ids.insert((*dependent_id).to_owned()) {
                    if dependent.persistence == ArtifactPersistence::HistoricalImmutable {
                        historical_ids.insert((*dependent_id).to_owned());
                    }
                    queue.push_back((*dependent_id).to_owned());
                }
            }
        }
    }

    let mut direct_invalidations = Vec::new();
    let mut transitive_invalidations = Vec::new();
    let mut preserved_historical_records = Vec::new();
    let mut unaffected_artifact_ids = Vec::new();
    let mut requalification_order = Vec::new();

    // Use a deterministic topological order for reporting and requalification.
    for artifact_id in &topological_order {
        let artifact = artifact_by_id[artifact_id.as_str()];
        if historical_ids.contains(artifact_id) {
            preserved_historical_records.push(build_artifact_impact(
                artifact,
                direct_dependencies_by_artifact.get(artifact_id.as_str()),
                upstream_reasons.get(artifact_id),
            ));
        } else if direct_ids.contains(artifact_id) {
            direct_invalidations.push(build_artifact_impact(
                artifact,
                direct_dependencies_by_artifact.get(artifact_id.as_str()),
                upstream_reasons.get(artifact_id),
            ));
            requalification_order.push(artifact_id.clone());
        } else if impacted_ids.contains(artifact_id) {
            transitive_invalidations.push(build_artifact_impact(
                artifact,
                None,
                upstream_reasons.get(artifact_id),
            ));
            requalification_order.push(artifact_id.clone());
        } else {
            unaffected_artifact_ids.push(artifact_id.clone());
        }
    }

    Ok(ChangeImpactReport {
        change_id: changes.change_id.clone(),
        source_event_sha256: changes.source_event_sha256.clone(),
        graph_snapshot_sha256: graph.graph_snapshot_sha256.clone(),
        direct_invalidations,
        transitive_invalidations,
        preserved_historical_records,
        unmatched_change_keys,
        unaffected_artifact_ids,
        requalification_order,
        source_records_mutated: false,
    })
}

fn build_artifact_impact(
    artifact: &ConstructionArtifact,
    changed_dependencies: Option<&Vec<DependencyKey>>,
    upstream_artifact_ids: Option<&BTreeSet<String>>,
) -> ArtifactImpact {
    ArtifactImpact {
        artifact_id: artifact.artifact_id.clone(),
        artifact_sha256: artifact.artifact_sha256.clone(),
        kind: artifact.kind,
        changed_dependencies: changed_dependencies.cloned().unwrap_or_default(),
        impacted_upstream_artifact_ids: upstream_artifact_ids
            .map(|ids| ids.iter().cloned().collect())
            .unwrap_or_default(),
    }
}

fn validate_change_set(changes: &ConstructionChangeSet) -> Result<(), ChangeImpactError> {
    if changes.change_id.trim().is_empty() {
        return Err(ChangeImpactError::EmptyChangeIdentity);
    }
    if !is_sha256(&changes.source_event_sha256) {
        return Err(ChangeImpactError::InvalidDigest("source change event"));
    }
    if changes.changes.is_empty() {
        return Err(ChangeImpactError::EmptyChangeSet);
    }

    let mut keys = BTreeSet::new();
    for transition in &changes.changes {
        if transition.key.scope_id.trim().is_empty() {
            return Err(ChangeImpactError::EmptyDependencyScope);
        }
        if !keys.insert(transition.key.clone()) {
            return Err(ChangeImpactError::DuplicateChangedDependency(
                transition.key.clone(),
            ));
        }
        if !is_sha256(&transition.previous_sha256) {
            return Err(ChangeImpactError::InvalidDigest("previous dependency version"));
        }
        if !is_sha256(&transition.current_sha256) {
            return Err(ChangeImpactError::InvalidDigest("current dependency version"));
        }
        if transition.previous_sha256 == transition.current_sha256 {
            return Err(ChangeImpactError::NoOpDependencyTransition(
                transition.key.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_and_topologically_sort(
    graph: &ConstructionArtifactGraph,
) -> Result<Vec<String>, ChangeImpactError> {
    if !is_sha256(&graph.graph_snapshot_sha256) {
        return Err(ChangeImpactError::InvalidDigest("artifact graph snapshot"));
    }

    let mut artifact_by_id = BTreeMap::<&str, &ConstructionArtifact>::new();
    for artifact in &graph.artifacts {
        if artifact.artifact_id.trim().is_empty() {
            return Err(ChangeImpactError::EmptyArtifactId);
        }
        if !is_sha256(&artifact.artifact_sha256) {
            return Err(ChangeImpactError::InvalidArtifactDigest(
                artifact.artifact_id.clone(),
            ));
        }
        if artifact_by_id
            .insert(artifact.artifact_id.as_str(), artifact)
            .is_some()
        {
            return Err(ChangeImpactError::DuplicateArtifactId(
                artifact.artifact_id.clone(),
            ));
        }

        let mut dependency_keys = BTreeSet::new();
        for dependency in &artifact.dependencies {
            if dependency.key.scope_id.trim().is_empty() {
                return Err(ChangeImpactError::EmptyDependencyScope);
            }
            if !dependency_keys.insert(dependency.key.clone()) {
                return Err(ChangeImpactError::DuplicateArtifactDependency {
                    artifact_id: artifact.artifact_id.clone(),
                    key: dependency.key.clone(),
                });
            }
            if !is_sha256(&dependency.observed_sha256) {
                return Err(ChangeImpactError::InvalidArtifactDependencyDigest {
                    artifact_id: artifact.artifact_id.clone(),
                    key: dependency.key.clone(),
                });
            }
        }

        let mut parents = BTreeSet::new();
        for parent_id in &artifact.depends_on_artifact_ids {
            if parent_id == &artifact.artifact_id {
                return Err(ChangeImpactError::SelfDependency(
                    artifact.artifact_id.clone(),
                ));
            }
            if !parents.insert(parent_id.as_str()) {
                return Err(ChangeImpactError::DuplicateArtifactParent {
                    artifact_id: artifact.artifact_id.clone(),
                    parent_artifact_id: parent_id.clone(),
                });
            }
        }
    }

    let mut indegree = BTreeMap::<&str, usize>::new();
    let mut dependents = BTreeMap::<&str, Vec<&str>>::new();
    for artifact in &graph.artifacts {
        indegree.insert(artifact.artifact_id.as_str(), artifact.depends_on_artifact_ids.len());
        for parent_id in &artifact.depends_on_artifact_ids {
            if !artifact_by_id.contains_key(parent_id.as_str()) {
                return Err(ChangeImpactError::UnknownArtifactParent {
                    artifact_id: artifact.artifact_id.clone(),
                    parent_artifact_id: parent_id.clone(),
                });
            }
            dependents
                .entry(parent_id.as_str())
                .or_default()
                .push(artifact.artifact_id.as_str());
        }
    }
    for children in dependents.values_mut() {
        children.sort_unstable();
    }

    let mut ready = BTreeSet::<&str>::new();
    for (artifact_id, degree) in &indegree {
        if *degree == 0 {
            ready.insert(*artifact_id);
        }
    }

    let mut order = Vec::with_capacity(graph.artifacts.len());
    while let Some(artifact_id) = ready.pop_first() {
        order.push(artifact_id.to_owned());
        if let Some(children) = dependents.get(artifact_id) {
            for child in children {
                let degree = indegree
                    .get_mut(child)
                    .expect("child was inserted into the artifact index");
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(*child);
                }
            }
        }
    }

    if order.len() != graph.artifacts.len() {
        return Err(ChangeImpactError::DependencyCycle);
    }
    Ok(order)
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

    fn digest(value: char) -> String {
        std::iter::repeat_n(value, 64).collect()
    }

    fn key(domain: DependencyDomain, scope: &str) -> DependencyKey {
        DependencyKey {
            domain,
            scope_id: scope.into(),
        }
    }

    fn dependency(domain: DependencyDomain, scope: &str, version: char) -> VersionedDependency {
        VersionedDependency {
            key: key(domain, scope),
            observed_sha256: digest(version),
        }
    }

    fn artifact(
        id: &str,
        kind: ConstructionArtifactKind,
        persistence: ArtifactPersistence,
        dependencies: Vec<VersionedDependency>,
        depends_on: &[&str],
        digest_char: char,
    ) -> ConstructionArtifact {
        ConstructionArtifact {
            artifact_id: id.into(),
            artifact_sha256: digest(digest_char),
            kind,
            persistence,
            dependencies,
            depends_on_artifact_ids: depends_on.iter().map(|id| (*id).into()).collect(),
        }
    }

    fn graph(artifacts: Vec<ConstructionArtifact>) -> ConstructionArtifactGraph {
        ConstructionArtifactGraph {
            graph_snapshot_sha256: digest('f'),
            artifacts,
        }
    }

    fn change(domain: DependencyDomain, scope: &str, previous: char, current: char) -> DependencyTransition {
        DependencyTransition {
            key: key(domain, scope),
            previous_sha256: digest(previous),
            current_sha256: digest(current),
        }
    }

    fn change_set(changes: Vec<DependencyTransition>) -> ConstructionChangeSet {
        ConstructionChangeSet {
            change_id: "site-change-001".into(),
            source_event_sha256: digest('e'),
            changes,
        }
    }

    #[test]
    fn localized_entity_change_invalidates_only_its_derived_closure() {
        let graph = graph(vec![
            artifact(
                "motion:A",
                ConstructionArtifactKind::MotionEnvelope,
                ArtifactPersistence::Recomputable,
                vec![dependency(DependencyDomain::EntityGeometry, "entity:A", '1')],
                &[],
                'a',
            ),
            artifact(
                "motion:B",
                ConstructionArtifactKind::MotionEnvelope,
                ArtifactPersistence::Recomputable,
                vec![dependency(DependencyDomain::EntityGeometry, "entity:B", '2')],
                &[],
                'b',
            ),
            artifact(
                "occupancy:A",
                ConstructionArtifactKind::SpaceTimeOccupancyAssessment,
                ArtifactPersistence::Recomputable,
                vec![],
                &["motion:A"],
                'c',
            ),
            artifact(
                "readiness:A",
                ConstructionArtifactKind::DispatchReadiness,
                ArtifactPersistence::Recomputable,
                vec![],
                &["occupancy:A"],
                'd',
            ),
            artifact(
                "accepted:A",
                ConstructionArtifactKind::AcceptedConstructionRecord,
                ArtifactPersistence::HistoricalImmutable,
                vec![dependency(DependencyDomain::EntityGeometry, "entity:A", '1')],
                &[],
                '9',
            ),
        ]);
        let changes = change_set(vec![change(
            DependencyDomain::EntityGeometry,
            "entity:A",
            '1',
            '3',
        )]);

        let report = compile_change_impact(&graph, &changes).unwrap();
        assert_eq!(
            report
                .direct_invalidations
                .iter()
                .map(|item| item.artifact_id.as_str())
                .collect::<Vec<_>>(),
            vec!["motion:A"]
        );
        assert_eq!(
            report
                .transitive_invalidations
                .iter()
                .map(|item| item.artifact_id.as_str())
                .collect::<Vec<_>>(),
            vec!["occupancy:A", "readiness:A"]
        );
        assert_eq!(
            report
                .preserved_historical_records
                .iter()
                .map(|item| item.artifact_id.as_str())
                .collect::<Vec<_>>(),
            vec!["accepted:A"]
        );
        assert_eq!(report.unaffected_artifact_ids, vec!["motion:B"]);
        assert_eq!(
            report.requalification_order,
            vec!["motion:A", "occupancy:A", "readiness:A"]
        );
        assert!(!report.source_records_mutated);
    }

    #[test]
    fn shared_transform_change_invalidates_every_dependent_robot_branch() {
        let graph = graph(vec![
            artifact(
                "transform-qualification",
                ConstructionArtifactKind::StationQualification,
                ArtifactPersistence::Recomputable,
                vec![dependency(DependencyDomain::CoordinateTransform, "floor:02", '1')],
                &[],
                'a',
            ),
            artifact(
                "robot-a-envelope",
                ConstructionArtifactKind::MotionEnvelope,
                ArtifactPersistence::Recomputable,
                vec![],
                &["transform-qualification"],
                'b',
            ),
            artifact(
                "robot-b-envelope",
                ConstructionArtifactKind::MotionEnvelope,
                ArtifactPersistence::Recomputable,
                vec![],
                &["transform-qualification"],
                'c',
            ),
            artifact(
                "robot-a-readiness",
                ConstructionArtifactKind::DispatchReadiness,
                ArtifactPersistence::Recomputable,
                vec![],
                &["robot-a-envelope"],
                'd',
            ),
        ]);
        let report = compile_change_impact(
            &graph,
            &change_set(vec![change(
                DependencyDomain::CoordinateTransform,
                "floor:02",
                '1',
                '2',
            )]),
        )
        .unwrap();

        assert_eq!(report.direct_invalidations.len(), 1);
        assert_eq!(report.transitive_invalidations.len(), 3);
        assert_eq!(
            report.requalification_order,
            vec![
                "transform-qualification",
                "robot-a-envelope",
                "robot-a-readiness",
                "robot-b-envelope"
            ]
        );
    }

    #[test]
    fn historical_evidence_is_preserved_but_current_dependents_are_invalidated() {
        let graph = graph(vec![
            artifact(
                "accepted-floor-02",
                ConstructionArtifactKind::AcceptedConstructionRecord,
                ArtifactPersistence::HistoricalImmutable,
                vec![dependency(DependencyDomain::DesignRevision, "floor:02", '1')],
                &[],
                'a',
            ),
            artifact(
                "current-install-plan",
                ConstructionArtifactKind::ProductionTask,
                ArtifactPersistence::Recomputable,
                vec![],
                &["accepted-floor-02"],
                'b',
            ),
        ]);
        let report = compile_change_impact(
            &graph,
            &change_set(vec![change(
                DependencyDomain::DesignRevision,
                "floor:02",
                '1',
                '2',
            )]),
        )
        .unwrap();

        assert_eq!(report.preserved_historical_records.len(), 1);
        assert_eq!(
            report.transitive_invalidations[0].artifact_id,
            "current-install-plan"
        );
        assert_eq!(
            report.transitive_invalidations[0].impacted_upstream_artifact_ids,
            vec!["accepted-floor-02"]
        );
        assert_eq!(report.requalification_order, vec!["current-install-plan"]);
    }

    #[test]
    fn any_old_dependency_version_is_stale_even_when_not_the_immediate_previous_version() {
        let graph = graph(vec![artifact(
            "old-trajectory",
            ConstructionArtifactKind::RobotTrajectory,
            ArtifactPersistence::Recomputable,
            vec![dependency(DependencyDomain::DesignRevision, "floor:02", '0')],
            &[],
            'a',
        )]);
        let report = compile_change_impact(
            &graph,
            &change_set(vec![change(
                DependencyDomain::DesignRevision,
                "floor:02",
                '1',
                '2',
            )]),
        )
        .unwrap();

        assert_eq!(report.direct_invalidations.len(), 1);
        assert_eq!(report.unaffected_artifact_ids.len(), 0);
    }

    #[test]
    fn unmatched_change_keys_are_reported_not_silently_ignored() {
        let graph = graph(vec![artifact(
            "motion:A",
            ConstructionArtifactKind::MotionEnvelope,
            ArtifactPersistence::Recomputable,
            vec![dependency(DependencyDomain::EntityGeometry, "entity:A", '1')],
            &[],
            'a',
        )]);
        let report = compile_change_impact(
            &graph,
            &change_set(vec![change(
                DependencyDomain::MaterialLot,
                "lot:LOT-44",
                '1',
                '2',
            )]),
        )
        .unwrap();

        assert_eq!(
            report.unmatched_change_keys,
            vec![key(DependencyDomain::MaterialLot, "lot:LOT-44")]
        );
        assert!(!report.change_keys_reconciled());
    }

    #[test]
    fn duplicate_change_keys_and_noop_transitions_fail_closed() {
        let duplicate = change_set(vec![
            change(DependencyDomain::EntityGeometry, "entity:A", '1', '2'),
            change(DependencyDomain::EntityGeometry, "entity:A", '2', '3'),
        ]);
        assert_eq!(
            compile_change_impact(&graph(Vec::new()), &duplicate),
            Err(ChangeImpactError::DuplicateChangedDependency(key(
                DependencyDomain::EntityGeometry,
                "entity:A"
            )))
        );

        let noop = change_set(vec![change(
            DependencyDomain::EntityGeometry,
            "entity:A",
            '1',
            '1',
        )]);
        assert_eq!(
            compile_change_impact(&graph(Vec::new()), &noop),
            Err(ChangeImpactError::NoOpDependencyTransition(key(
                DependencyDomain::EntityGeometry,
                "entity:A"
            )))
        );
    }

    #[test]
    fn duplicate_artifacts_unknown_parents_and_cycles_fail_closed() {
        let duplicate = graph(vec![
            artifact(
                "same",
                ConstructionArtifactKind::RobotTrajectory,
                ArtifactPersistence::Recomputable,
                vec![],
                &[],
                'a',
            ),
            artifact(
                "same",
                ConstructionArtifactKind::MotionEnvelope,
                ArtifactPersistence::Recomputable,
                vec![],
                &[],
                'b',
            ),
        ]);
        assert_eq!(
            compile_change_impact(
                &duplicate,
                &change_set(vec![change(
                    DependencyDomain::EntityGeometry,
                    "entity:A",
                    '1',
                    '2'
                )])
            ),
            Err(ChangeImpactError::DuplicateArtifactId("same".into()))
        );

        let unknown_parent = graph(vec![artifact(
            "child",
            ConstructionArtifactKind::DispatchReadiness,
            ArtifactPersistence::Recomputable,
            vec![],
            &["missing"],
            'a',
        )]);
        assert!(matches!(
            compile_change_impact(
                &unknown_parent,
                &change_set(vec![change(
                    DependencyDomain::EntityGeometry,
                    "entity:A",
                    '1',
                    '2'
                )])
            ),
            Err(ChangeImpactError::UnknownArtifactParent { .. })
        ));

        let cycle = graph(vec![
            artifact(
                "a",
                ConstructionArtifactKind::RobotTrajectory,
                ArtifactPersistence::Recomputable,
                vec![],
                &["b"],
                'a',
            ),
            artifact(
                "b",
                ConstructionArtifactKind::MotionEnvelope,
                ArtifactPersistence::Recomputable,
                vec![],
                &["a"],
                'b',
            ),
        ]);
        assert_eq!(
            compile_change_impact(
                &cycle,
                &change_set(vec![change(
                    DependencyDomain::EntityGeometry,
                    "entity:A",
                    '1',
                    '2'
                )])
            ),
            Err(ChangeImpactError::DependencyCycle)
        );
    }

    #[test]
    fn deterministic_order_is_independent_of_input_artifact_order() {
        let a = artifact(
            "a",
            ConstructionArtifactKind::StationQualification,
            ArtifactPersistence::Recomputable,
            vec![dependency(DependencyDomain::SurveyControl, "floor:02", '1')],
            &[],
            'a',
        );
        let b = artifact(
            "b",
            ConstructionArtifactKind::MotionEnvelope,
            ArtifactPersistence::Recomputable,
            vec![],
            &["a"],
            'b',
        );
        let c = artifact(
            "c",
            ConstructionArtifactKind::DispatchReadiness,
            ArtifactPersistence::Recomputable,
            vec![],
            &["b"],
            'c',
        );
        let change = change_set(vec![change(
            DependencyDomain::SurveyControl,
            "floor:02",
            '1',
            '2',
        )]);
        let first = compile_change_impact(&graph(vec![a.clone(), b.clone(), c.clone()]), &change).unwrap();
        let second = compile_change_impact(&graph(vec![c, a, b]), &change).unwrap();
        assert_eq!(first.requalification_order, second.requalification_order);
        assert_eq!(first.requalification_order, vec!["a", "b", "c"]);
    }
}
