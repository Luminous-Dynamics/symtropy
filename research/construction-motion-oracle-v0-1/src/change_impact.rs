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
    InspectionSpecification,
    MetrologyReference,
    SiteLedgerState,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
    /// Full versioned inputs actually observed by this artifact.
    pub dependencies: Vec<VersionedDependency>,
    /// Exact complete set of scoped dependency keys the artifact claims it
    /// required. This must be independently derived from its source contract,
    /// included in artifact_sha256, and reconciled exactly with dependencies.
    /// Domain-only coverage is insufficient: EntityGeometry/entity:B must not
    /// satisfy a MotionEnvelope for entity:A.
    pub expected_dependency_keys: Vec<DependencyKey>,
    /// Other artifacts used to produce or qualify this artifact.
    pub depends_on_artifact_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstructionArtifactGraph {
    pub graph_snapshot_sha256: String,
    /// Authoritative dependency values in the frozen pre-change snapshot.
    /// Artifact dependencies are compared to this registry to detect already
    /// stale artifacts; change.previous_sha256 must match this registry before
    /// a transition can be applied.
    pub current_dependencies: Vec<VersionedDependency>,
    pub artifacts: Vec<ConstructionArtifact>,
}

/// Owner-approved minimum dependency domains for one artifact class.
/// The contract is a coverage floor; artifact-specific scope identities and
/// all additional dependencies remain explicit on each artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactDependencyContract {
    pub artifact_kind: ConstructionArtifactKind,
    pub required_domains: Vec<DependencyDomain>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencyContractSet {
    pub contract_sha256: String,
    pub contracts: Vec<ArtifactDependencyContract>,
}

/// Built-in minimum dependency domains for every supported construction
/// artifact class. Project/site contracts may add stricter domains, but must
/// include every domain returned here. Hash the canonical returned contract
/// list in the owner system and bind that digest in DependencyContractSet.
fn minimum_required_domains(
    artifact_kind: ConstructionArtifactKind,
) -> &'static [DependencyDomain] {
    use ConstructionArtifactKind::*;
    use DependencyDomain::*;

    match artifact_kind {
        IfcIdsPreflight => &[SourceModel, IfcSchema, IdsSpecification, IdsEngineBuild],
        DesignWorkPackage => &[
            SourceModel,
            IfcSchema,
            IdsSpecification,
            DesignRevision,
            CoordinateFrame,
            CoordinateTransform,
            AcceptanceProfile,
        ],
        StationQualification => &[
            CoordinateFrame,
            CoordinateTransform,
            SurveyControl,
            StationCalibration,
            SiteClockMap,
        ],
        MaterialReadiness => &[
            MaterialLot,
            DesignRevision,
            WorkPackageSchedule,
            AuthorityPolicy,
        ],
        ProductionTask => &[
            DesignRevision,
            CoordinateFrame,
            CoordinateTransform,
            WorkPackageSchedule,
            MaterialLot,
            AuthorityPolicy,
        ],
        RobotTrajectory | MotionEnvelope => &[
            DesignRevision,
            EntityGeometry,
            CoordinateFrame,
            CoordinateTransform,
            RobotConfiguration,
            ToolConfiguration,
            PayloadConfiguration,
            TrajectoryConfiguration,
            SiteClockMap,
        ],
        SpaceTimeOccupancyAssessment => &[
            CoordinateFrame,
            CoordinateTransform,
            ObstacleMap,
            DynamicActorManifest,
            SiteClockMap,
            OccupancyEngineBuild,
            ClearancePolicy,
        ],
        DispatchReadiness => &[
            DesignRevision,
            CoordinateFrame,
            CoordinateTransform,
            RobotConfiguration,
            ToolConfiguration,
            PayloadConfiguration,
            TrajectoryConfiguration,
            ObstacleMap,
            DynamicActorManifest,
            SiteClockMap,
            OccupancyEngineBuild,
            ClearancePolicy,
            SiteLedgerState,
            WorkPackageSchedule,
            AuthorityPolicy,
        ],
        InspectionPlan => &[
            DesignRevision,
            EntityGeometry,
            CoordinateFrame,
            CoordinateTransform,
            AcceptanceProfile,
            InspectionSpecification,
            AuthorityPolicy,
            WorkPackageSchedule,
        ],
        AsBuiltMeasurement => &[
            DesignRevision,
            EntityGeometry,
            CoordinateFrame,
            CoordinateTransform,
            SurveyControl,
            StationCalibration,
            MetrologyReference,
        ],
        AcceptedConstructionRecord => &[
            DesignRevision,
            EntityGeometry,
            CoordinateFrame,
            CoordinateTransform,
            AcceptanceProfile,
            MetrologyReference,
            MaterialLot,
            AuthorityPolicy,
            SiteLedgerState,
        ],
        HandoverRecord => &[
            SourceModel,
            DesignRevision,
            AcceptanceProfile,
            InspectionSpecification,
            MetrologyReference,
            AuthorityPolicy,
            SiteLedgerState,
        ],
    }
}

pub fn minimum_construction_dependency_contracts() -> Vec<ArtifactDependencyContract> {
    use ConstructionArtifactKind::*;
    use DependencyDomain::*;

    vec![
        ArtifactDependencyContract {
            artifact_kind: IfcIdsPreflight,
            required_domains: vec![SourceModel, IfcSchema, IdsSpecification, IdsEngineBuild],
        },
        ArtifactDependencyContract {
            artifact_kind: DesignWorkPackage,
            required_domains: vec![
                SourceModel,
                IfcSchema,
                IdsSpecification,
                DesignRevision,
                CoordinateFrame,
                CoordinateTransform,
                AcceptanceProfile,
            ],
        },
        ArtifactDependencyContract {
            artifact_kind: StationQualification,
            required_domains: vec![
                CoordinateFrame,
                CoordinateTransform,
                SurveyControl,
                StationCalibration,
                SiteClockMap,
            ],
        },
        ArtifactDependencyContract {
            artifact_kind: MaterialReadiness,
            required_domains: vec![
                MaterialLot,
                DesignRevision,
                WorkPackageSchedule,
                AuthorityPolicy,
            ],
        },
        ArtifactDependencyContract {
            artifact_kind: ProductionTask,
            required_domains: vec![
                DesignRevision,
                CoordinateFrame,
                CoordinateTransform,
                WorkPackageSchedule,
                MaterialLot,
                AuthorityPolicy,
            ],
        },
        ArtifactDependencyContract {
            artifact_kind: RobotTrajectory,
            required_domains: vec![
                DesignRevision,
                EntityGeometry,
                CoordinateFrame,
                CoordinateTransform,
                RobotConfiguration,
                ToolConfiguration,
                PayloadConfiguration,
                TrajectoryConfiguration,
                SiteClockMap,
            ],
        },
        ArtifactDependencyContract {
            artifact_kind: MotionEnvelope,
            required_domains: vec![
                DesignRevision,
                EntityGeometry,
                CoordinateFrame,
                CoordinateTransform,
                RobotConfiguration,
                ToolConfiguration,
                PayloadConfiguration,
                TrajectoryConfiguration,
                SiteClockMap,
            ],
        },
        ArtifactDependencyContract {
            artifact_kind: SpaceTimeOccupancyAssessment,
            required_domains: vec![
                CoordinateFrame,
                CoordinateTransform,
                ObstacleMap,
                DynamicActorManifest,
                SiteClockMap,
                OccupancyEngineBuild,
                ClearancePolicy,
            ],
        },
        ArtifactDependencyContract {
            artifact_kind: DispatchReadiness,
            required_domains: vec![
                DesignRevision,
                CoordinateFrame,
                CoordinateTransform,
                RobotConfiguration,
                ToolConfiguration,
                PayloadConfiguration,
                TrajectoryConfiguration,
                ObstacleMap,
                DynamicActorManifest,
                SiteClockMap,
                OccupancyEngineBuild,
                ClearancePolicy,
                SiteLedgerState,
                WorkPackageSchedule,
                AuthorityPolicy,
            ],
        },
        ArtifactDependencyContract {
            artifact_kind: InspectionPlan,
            required_domains: vec![
                DesignRevision,
                EntityGeometry,
                CoordinateFrame,
                CoordinateTransform,
                AcceptanceProfile,
                InspectionSpecification,
                AuthorityPolicy,
                WorkPackageSchedule,
            ],
        },
        ArtifactDependencyContract {
            artifact_kind: AsBuiltMeasurement,
            required_domains: vec![
                DesignRevision,
                EntityGeometry,
                CoordinateFrame,
                CoordinateTransform,
                SurveyControl,
                StationCalibration,
                MetrologyReference,
            ],
        },
        ArtifactDependencyContract {
            artifact_kind: AcceptedConstructionRecord,
            required_domains: vec![
                DesignRevision,
                EntityGeometry,
                CoordinateFrame,
                CoordinateTransform,
                AcceptanceProfile,
                MetrologyReference,
                MaterialLot,
                AuthorityPolicy,
                SiteLedgerState,
            ],
        },
        ArtifactDependencyContract {
            artifact_kind: HandoverRecord,
            required_domains: vec![
                SourceModel,
                DesignRevision,
                AcceptanceProfile,
                InspectionSpecification,
                MetrologyReference,
                AuthorityPolicy,
                SiteLedgerState,
            ],
        },
    ]
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencyCoverageGap {
    pub artifact_id: String,
    pub artifact_kind: ConstructionArtifactKind,
    pub missing_domains: Vec<DependencyDomain>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencyScopeCoverageGap {
    pub artifact_id: String,
    pub missing_expected_keys: Vec<DependencyKey>,
    pub unexpected_observed_keys: Vec<DependencyKey>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencyCoverageAudit {
    pub graph_snapshot_sha256: String,
    pub contract_sha256: String,
    pub artifacts_checked: u64,
    pub complete_artifacts: u64,
    pub gaps: Vec<DependencyCoverageGap>,
    pub scope_gaps: Vec<DependencyScopeCoverageGap>,
    pub source_records_mutated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangeImpactError {
    EmptyChangeIdentity,
    InvalidDigest(&'static str),
    EmptyChangeSet,
    EmptyDependencyScope,
    DuplicateChangedDependency(DependencyKey),
    NoOpDependencyTransition(DependencyKey),
    MissingCurrentDependencyVersion(DependencyKey),
    ChangeTargetMissingCurrentState(DependencyKey),
    ChangePreviousVersionMismatch(DependencyKey),
    DuplicateCurrentDependencyVersion(DependencyKey),
    EmptyArtifactId,
    DuplicateArtifactId(String),
    InvalidArtifactDigest(String),
    HistoricalArtifactMustBeImmutable(String),
    DerivedArtifactMustBeRecomputable(String),
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
    InvalidDependencyContractDigest,
    EmptyDependencyContractSet,
    DuplicateArtifactKindContract(ConstructionArtifactKind),
    EmptyRequiredDependencyDomains(ConstructionArtifactKind),
    ContractOmitsMinimumDependencyDomain {
        artifact_kind: ConstructionArtifactKind,
        domain: DependencyDomain,
    },
    DuplicateRequiredDependencyDomain {
        artifact_kind: ConstructionArtifactKind,
        domain: DependencyDomain,
    },
    DuplicateExpectedArtifactDependency {
        artifact_id: String,
        key: DependencyKey,
    },
    MissingArtifactKindContract(ConstructionArtifactKind),
    DependencyCoverageIncomplete(usize),
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
    /// None/false means only graph-internal consistency was checked. A
    /// readiness path must use compile_change_impact_with_contract.
    pub dependency_contract_sha256: Option<String>,
    pub dependency_coverage_complete: bool,
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

    /// Selective requalification is eligible only when owner-declared
    /// dependency coverage passed and every changed key mapped to the graph.
    pub fn selective_requalification_gate_passed(&self) -> bool {
        self.dependency_coverage_complete && self.unmatched_change_keys.is_empty()
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

    let current_by_key: BTreeMap<DependencyKey, &str> = graph
        .current_dependencies
        .iter()
        .map(|dependency| (dependency.key.clone(), dependency.observed_sha256.as_str()))
        .collect();
    let transition_by_key: BTreeMap<DependencyKey, &DependencyTransition> = changes
        .changes
        .iter()
        .map(|transition| (transition.key.clone(), transition))
        .collect();

    // Change events are transitions from the authoritative pre-change site
    // snapshot. A caller cannot invent a previous version or apply a change
    // against a different snapshot.
    for transition in &changes.changes {
        let Some(snapshot_version) = current_by_key.get(&transition.key) else {
            return Err(ChangeImpactError::ChangeTargetMissingCurrentState(
                transition.key.clone(),
            ));
        };
        if *snapshot_version != transition.previous_sha256 {
            return Err(ChangeImpactError::ChangePreviousVersionMismatch(
                transition.key.clone(),
            ));
        }
    }

    let mut direct_dependencies_by_artifact = BTreeMap::<&str, Vec<DependencyKey>>::new();
    let mut declared_change_keys = BTreeSet::new();

    for artifact in &graph.artifacts {
        let mut stale_dependencies = Vec::new();
        for dependency in &artifact.dependencies {
            let snapshot_version = current_by_key.get(&dependency.key).ok_or_else(|| {
                ChangeImpactError::MissingCurrentDependencyVersion(dependency.key.clone())
            })?;
            let effective_version = transition_by_key
                .get(&dependency.key)
                .map_or(*snapshot_version, |transition| {
                    transition.current_sha256.as_str()
                });
            if transition_by_key.contains_key(&dependency.key) {
                declared_change_keys.insert(dependency.key.clone());
            }
            // Check every dependency against the post-change version, not
            // only dependencies named in this event. This also discovers
            // pre-existing stale artifacts in the frozen graph snapshot.
            if dependency.observed_sha256 != effective_version {
                stale_dependencies.push(dependency.key.clone());
            }
        }
        if !stale_dependencies.is_empty() {
            stale_dependencies.sort();
            direct_dependencies_by_artifact
                .insert(artifact.artifact_id.as_str(), stale_dependencies);
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
        dependency_contract_sha256: None,
        dependency_coverage_complete: false,
    })
}

/// Audit every artifact against an owner-approved minimum dependency contract.
/// Missing kind contracts or domains are explicit gaps; a structurally valid
/// graph is not considered dependency-complete by default.
pub fn audit_dependency_coverage(
    graph: &ConstructionArtifactGraph,
    contracts: &DependencyContractSet,
) -> Result<DependencyCoverageAudit, ChangeImpactError> {
    validate_and_topologically_sort(graph)?;
    if !is_sha256(&contracts.contract_sha256) {
        return Err(ChangeImpactError::InvalidDependencyContractDigest);
    }
    if contracts.contracts.is_empty() {
        return Err(ChangeImpactError::EmptyDependencyContractSet);
    }

    let mut contract_by_kind = BTreeMap::new();
    for contract in &contracts.contracts {
        if contract.required_domains.is_empty() {
            return Err(ChangeImpactError::EmptyRequiredDependencyDomains(
                contract.artifact_kind,
            ));
        }
        let mut domains = BTreeSet::new();
        for domain in &contract.required_domains {
            if !domains.insert(*domain) {
                return Err(ChangeImpactError::DuplicateRequiredDependencyDomain {
                    artifact_kind: contract.artifact_kind,
                    domain: *domain,
                });
            }
        }
        if contract_by_kind
            .insert(contract.artifact_kind, &contract.required_domains)
            .is_some()
        {
            return Err(ChangeImpactError::DuplicateArtifactKindContract(
                contract.artifact_kind,
            ));
        }
        let supplied_domains: BTreeSet<DependencyDomain> =
            contract.required_domains.iter().copied().collect();
        for minimum_domain in minimum_required_domains(contract.artifact_kind) {
            if !supplied_domains.contains(minimum_domain) {
                return Err(ChangeImpactError::ContractOmitsMinimumDependencyDomain {
                    artifact_kind: contract.artifact_kind,
                    domain: *minimum_domain,
                });
            }
        }
    }

    let mut gaps = Vec::new();
    let mut scope_gaps = Vec::new();
    let mut complete_artifacts = 0_u64;
    for artifact in &graph.artifacts {
        let required_domains = contract_by_kind.get(&artifact.kind).ok_or(
            ChangeImpactError::MissingArtifactKindContract(artifact.kind),
        )?;

        let mut expected_keys = BTreeSet::new();
        for key in &artifact.expected_dependency_keys {
            if key.scope_id.trim().is_empty() {
                return Err(ChangeImpactError::EmptyDependencyScope);
            }
            if !expected_keys.insert(key.clone()) {
                return Err(ChangeImpactError::DuplicateExpectedArtifactDependency {
                    artifact_id: artifact.artifact_id.clone(),
                    key: key.clone(),
                });
            }
        }

        // Evaluate the domain floor from the artifact's expected manifest,
        // never from whichever same-domain keys happened to be observed.
        let expected_domains: BTreeSet<DependencyDomain> =
            expected_keys.iter().map(|key| key.domain).collect();
        let mut missing_domains: Vec<DependencyDomain> = required_domains
            .iter()
            .copied()
            .filter(|domain| !expected_domains.contains(domain))
            .collect();
        missing_domains.sort();

        let observed_keys: BTreeSet<DependencyKey> = artifact
            .dependencies
            .iter()
            .map(|dependency| dependency.key.clone())
            .collect();
        let mut missing_expected_keys: Vec<DependencyKey> = expected_keys
            .difference(&observed_keys)
            .cloned()
            .collect();
        let mut unexpected_observed_keys: Vec<DependencyKey> = observed_keys
            .difference(&expected_keys)
            .cloned()
            .collect();
        missing_expected_keys.sort();
        unexpected_observed_keys.sort();

        let has_scope_gap =
            !missing_expected_keys.is_empty() || !unexpected_observed_keys.is_empty();
        if !missing_domains.is_empty() {
            gaps.push(DependencyCoverageGap {
                artifact_id: artifact.artifact_id.clone(),
                artifact_kind: artifact.kind,
                missing_domains,
            });
        }
        if has_scope_gap {
            scope_gaps.push(DependencyScopeCoverageGap {
                artifact_id: artifact.artifact_id.clone(),
                missing_expected_keys,
                unexpected_observed_keys,
            });
        }
        if missing_domains.is_empty() && !has_scope_gap {
            complete_artifacts += 1;
        }
    }

    Ok(DependencyCoverageAudit {
        graph_snapshot_sha256: graph.graph_snapshot_sha256.clone(),
        contract_sha256: contracts.contract_sha256.clone(),
        artifacts_checked: graph.artifacts.len() as u64,
        complete_artifacts,
        gaps,
        scope_gaps,
        source_records_mutated: false,
    })
}

/// High-assurance entry point for selective requalification. It first validates
/// graph structure and owner-declared minimum dependency coverage, then
/// computes the exact invalidation closure. The returned report is marked
/// coverage-complete only after every artifact kind and required domain passes.
pub fn compile_change_impact_with_contract(
    graph: &ConstructionArtifactGraph,
    contracts: &DependencyContractSet,
    changes: &ConstructionChangeSet,
) -> Result<ChangeImpactReport, ChangeImpactError> {
    let audit = audit_dependency_coverage(graph, contracts)?;
    if !audit.gaps.is_empty() || !audit.scope_gaps.is_empty() {
        return Err(ChangeImpactError::DependencyCoverageIncomplete(
            audit.gaps.len() + audit.scope_gaps.len(),
        ));
    }
    let mut report = compile_change_impact(graph, changes)?;
    report.dependency_contract_sha256 = Some(audit.contract_sha256);
    report.dependency_coverage_complete = true;
    Ok(report)
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
            return Err(ChangeImpactError::InvalidDigest(
                "previous dependency version",
            ));
        }
        if !is_sha256(&transition.current_sha256) {
            return Err(ChangeImpactError::InvalidDigest(
                "current dependency version",
            ));
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

    let mut current_dependency_by_key = BTreeMap::<&DependencyKey, &str>::new();
    for dependency in &graph.current_dependencies {
        if dependency.key.scope_id.trim().is_empty() {
            return Err(ChangeImpactError::EmptyDependencyScope);
        }
        if !is_sha256(&dependency.observed_sha256) {
            return Err(ChangeImpactError::InvalidDigest(
                "current dependency version",
            ));
        }
        if current_dependency_by_key
            .insert(&dependency.key, dependency.observed_sha256.as_str())
            .is_some()
        {
            return Err(ChangeImpactError::DuplicateCurrentDependencyVersion(
                dependency.key.clone(),
            ));
        }
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
        let requires_historical_persistence = matches!(
            artifact.kind,
            ConstructionArtifactKind::AsBuiltMeasurement
                | ConstructionArtifactKind::AcceptedConstructionRecord
                | ConstructionArtifactKind::HandoverRecord
        );
        match (requires_historical_persistence, artifact.persistence) {
            (true, ArtifactPersistence::Recomputable) => {
                return Err(ChangeImpactError::HistoricalArtifactMustBeImmutable(
                    artifact.artifact_id.clone(),
                ));
            }
            (false, ArtifactPersistence::HistoricalImmutable) => {
                return Err(ChangeImpactError::DerivedArtifactMustBeRecomputable(
                    artifact.artifact_id.clone(),
                ));
            }
            _ => {}
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
            if !current_dependency_by_key.contains_key(&dependency.key) {
                return Err(ChangeImpactError::MissingCurrentDependencyVersion(
                    dependency.key.clone(),
                ));
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
        indegree.insert(
            artifact.artifact_id.as_str(),
            artifact.depends_on_artifact_ids.len(),
        );
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
            expected_dependency_keys: dependencies
                .iter()
                .map(|dependency| dependency.key.clone())
                .collect(),
            dependencies,
            depends_on_artifact_ids: depends_on.iter().map(|id| (*id).into()).collect(),
        }
    }

    fn graph(artifacts: Vec<ConstructionArtifact>) -> ConstructionArtifactGraph {
        let mut current_by_key = BTreeMap::<DependencyKey, String>::new();
        for artifact in &artifacts {
            for dependency in &artifact.dependencies {
                current_by_key
                    .entry(dependency.key.clone())
                    .or_insert_with(|| dependency.observed_sha256.clone());
            }
        }
        ConstructionArtifactGraph {
            graph_snapshot_sha256: digest('f'),
            current_dependencies: current_by_key
                .into_iter()
                .map(|(key, observed_sha256)| VersionedDependency {
                    key,
                    observed_sha256,
                })
                .collect(),
            artifacts,
        }
    }

    fn change(
        domain: DependencyDomain,
        scope: &str,
        previous: char,
        current: char,
    ) -> DependencyTransition {
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
                vec![dependency(
                    DependencyDomain::EntityGeometry,
                    "entity:A",
                    '1',
                )],
                &[],
                'a',
            ),
            artifact(
                "motion:B",
                ConstructionArtifactKind::MotionEnvelope,
                ArtifactPersistence::Recomputable,
                vec![dependency(
                    DependencyDomain::EntityGeometry,
                    "entity:B",
                    '2',
                )],
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
                vec![dependency(
                    DependencyDomain::EntityGeometry,
                    "entity:A",
                    '1',
                )],
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
                vec![dependency(
                    DependencyDomain::CoordinateTransform,
                    "floor:02",
                    '1',
                )],
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
                vec![dependency(
                    DependencyDomain::DesignRevision,
                    "floor:02",
                    '1',
                )],
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
        let mut graph = graph(vec![artifact(
            "old-trajectory",
            ConstructionArtifactKind::RobotTrajectory,
            ArtifactPersistence::Recomputable,
            vec![dependency(
                DependencyDomain::DesignRevision,
                "floor:02",
                '0',
            )],
            &[],
            'a',
        )]);
        graph.current_dependencies[0].observed_sha256 = digest('1');
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
        let mut graph = graph(vec![artifact(
            "motion:A",
            ConstructionArtifactKind::MotionEnvelope,
            ArtifactPersistence::Recomputable,
            vec![dependency(
                DependencyDomain::EntityGeometry,
                "entity:A",
                '1',
            )],
            &[],
            'a',
        )]);
        graph.current_dependencies.push(VersionedDependency {
            key: key(DependencyDomain::MaterialLot, "lot:LOT-44"),
            observed_sha256: digest('1'),
        });
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
        let first =
            compile_change_impact(&graph(vec![a.clone(), b.clone(), c.clone()]), &change).unwrap();
        let second = compile_change_impact(&graph(vec![c, a, b]), &change).unwrap();
        assert_eq!(first.requalification_order, second.requalification_order);
        assert_eq!(first.requalification_order, vec!["a", "b", "c"]);
    }

    #[test]
    fn accepted_output_and_as_built_measurements_cannot_be_marked_recomputable() {
        for (id, kind) in [
            (
                "accepted",
                ConstructionArtifactKind::AcceptedConstructionRecord,
            ),
            ("as-built", ConstructionArtifactKind::AsBuiltMeasurement),
            ("handover", ConstructionArtifactKind::HandoverRecord),
        ] {
            let graph = graph(vec![artifact(
                id,
                kind,
                ArtifactPersistence::Recomputable,
                vec![],
                &[],
                'a',
            )]);
            assert_eq!(
                compile_change_impact(
                    &graph,
                    &change_set(vec![change(
                        DependencyDomain::DesignRevision,
                        "floor:02",
                        '1',
                        '2'
                    )])
                ),
                Err(ChangeImpactError::HistoricalArtifactMustBeImmutable(
                    id.into()
                ))
            );
        }
    }

    #[test]
    fn derived_artifacts_cannot_be_misclassified_as_immutable_history() {
        let graph = graph(vec![artifact(
            "dispatch",
            ConstructionArtifactKind::DispatchReadiness,
            ArtifactPersistence::HistoricalImmutable,
            vec![],
            &[],
            'a',
        )]);
        assert_eq!(
            compile_change_impact(
                &graph,
                &change_set(vec![change(
                    DependencyDomain::DesignRevision,
                    "floor:02",
                    '1',
                    '2'
                )])
            ),
            Err(ChangeImpactError::DerivedArtifactMustBeRecomputable(
                "dispatch".into()
            ))
        );
    }

    #[test]
    fn dependency_contract_blocks_motion_envelopes_missing_required_tool_bindings() {
        let motion_dependencies: Vec<VersionedDependency> =
            minimum_required_domains(ConstructionArtifactKind::MotionEnvelope)
                .iter()
                .filter(|domain| **domain != DependencyDomain::ToolConfiguration)
                .map(|domain| dependency(*domain, &format!("motion:A:{domain:?}"), '1'))
                .collect();
        let graph = graph(vec![artifact(
            "motion:A",
            ConstructionArtifactKind::MotionEnvelope,
            ArtifactPersistence::Recomputable,
            motion_dependencies,
            &[],
            'a',
        )]);
        let contracts = DependencyContractSet {
            contract_sha256: digest('c'),
            contracts: vec![ArtifactDependencyContract {
                artifact_kind: ConstructionArtifactKind::MotionEnvelope,
                required_domains: minimum_required_domains(
                    ConstructionArtifactKind::MotionEnvelope,
                )
                .to_vec(),
            }],
        };
        let audit = audit_dependency_coverage(&graph, &contracts).unwrap();
        assert_eq!(audit.artifacts_checked, 1);
        assert_eq!(audit.complete_artifacts, 0);
        assert_eq!(
            audit.gaps[0].missing_domains,
            vec![DependencyDomain::ToolConfiguration]
        );
        assert!(matches!(
            compile_change_impact_with_contract(
                &graph,
                &contracts,
                &change_set(vec![change(
                    DependencyDomain::EntityGeometry,
                    "motion:A:EntityGeometry",
                    '1',
                    '3'
                )])
            ),
            Err(ChangeImpactError::DependencyCoverageIncomplete(1))
        ));
    }

    #[test]
    fn dependency_contract_marks_a_change_report_ready_only_after_full_coverage() {
        let motion_dependencies: Vec<VersionedDependency> =
            minimum_required_domains(ConstructionArtifactKind::MotionEnvelope)
                .iter()
                .map(|domain| dependency(*domain, &format!("motion:A:{domain:?}"), '1'))
                .collect();
        let readiness_dependencies: Vec<VersionedDependency> =
            minimum_required_domains(ConstructionArtifactKind::DispatchReadiness)
                .iter()
                .map(|domain| dependency(*domain, &format!("readiness:A:{domain:?}"), '2'))
                .collect();
        let graph = graph(vec![
            artifact(
                "motion:A",
                ConstructionArtifactKind::MotionEnvelope,
                ArtifactPersistence::Recomputable,
                motion_dependencies,
                &[],
                'a',
            ),
            artifact(
                "readiness:A",
                ConstructionArtifactKind::DispatchReadiness,
                ArtifactPersistence::Recomputable,
                readiness_dependencies,
                &["motion:A"],
                'b',
            ),
        ]);
        let contracts = DependencyContractSet {
            contract_sha256: digest('c'),
            contracts: vec![
                ArtifactDependencyContract {
                    artifact_kind: ConstructionArtifactKind::MotionEnvelope,
                    required_domains: minimum_required_domains(
                        ConstructionArtifactKind::MotionEnvelope,
                    )
                    .to_vec(),
                },
                ArtifactDependencyContract {
                    artifact_kind: ConstructionArtifactKind::DispatchReadiness,
                    required_domains: minimum_required_domains(
                        ConstructionArtifactKind::DispatchReadiness,
                    )
                    .to_vec(),
                },
            ],
        };
        let report = compile_change_impact_with_contract(
            &graph,
            &contracts,
            &change_set(vec![change(
                DependencyDomain::EntityGeometry,
                "motion:A:EntityGeometry",
                '1',
                '4',
            )]),
        )
        .unwrap();
        assert!(report.dependency_coverage_complete);
        assert_eq!(report.dependency_contract_sha256, Some(digest('c')));
        assert!(report.selective_requalification_gate_passed());
    }

    #[test]
    fn missing_artifact_kind_contract_is_not_assumed_complete() {
        let graph = graph(vec![artifact(
            "motion:A",
            ConstructionArtifactKind::MotionEnvelope,
            ArtifactPersistence::Recomputable,
            vec![dependency(
                DependencyDomain::EntityGeometry,
                "entity:A",
                '1',
            )],
            &[],
            'a',
        )]);
        let contracts = DependencyContractSet {
            contract_sha256: digest('c'),
            contracts: vec![ArtifactDependencyContract {
                artifact_kind: ConstructionArtifactKind::DispatchReadiness,
                required_domains: minimum_required_domains(
                    ConstructionArtifactKind::DispatchReadiness,
                )
                .to_vec(),
            }],
        };
        assert_eq!(
            audit_dependency_coverage(&graph, &contracts),
            Err(ChangeImpactError::MissingArtifactKindContract(
                ConstructionArtifactKind::MotionEnvelope
            ))
        );
    }

    #[test]
    fn change_previous_version_must_match_authoritative_graph_snapshot() {
        let graph = graph(vec![artifact(
            "motion:A",
            ConstructionArtifactKind::MotionEnvelope,
            ArtifactPersistence::Recomputable,
            vec![dependency(
                DependencyDomain::EntityGeometry,
                "entity:A",
                '1',
            )],
            &[],
            'a',
        )]);
        let wrong_previous = change_set(vec![change(
            DependencyDomain::EntityGeometry,
            "entity:A",
            '0',
            '2',
        )]);
        assert_eq!(
            compile_change_impact(&graph, &wrong_previous),
            Err(ChangeImpactError::ChangePreviousVersionMismatch(key(
                DependencyDomain::EntityGeometry,
                "entity:A"
            )))
        );
    }

    #[test]
    fn artifacts_stale_before_the_current_change_are_detected_too() {
        let mut graph = graph(vec![
            artifact(
                "motion:A",
                ConstructionArtifactKind::MotionEnvelope,
                ArtifactPersistence::Recomputable,
                vec![dependency(
                    DependencyDomain::EntityGeometry,
                    "entity:A",
                    '0',
                )],
                &[],
                'a',
            ),
            artifact(
                "motion:B",
                ConstructionArtifactKind::MotionEnvelope,
                ArtifactPersistence::Recomputable,
                vec![dependency(
                    DependencyDomain::EntityGeometry,
                    "entity:B",
                    '1',
                )],
                &[],
                'b',
            ),
        ]);
        graph.current_dependencies[0].observed_sha256 = digest('1');
        let report = compile_change_impact(
            &graph,
            &change_set(vec![change(
                DependencyDomain::EntityGeometry,
                "entity:B",
                '1',
                '2',
            )]),
        )
        .unwrap();

        assert_eq!(
            report
                .direct_invalidations
                .iter()
                .map(|item| item.artifact_id.as_str())
                .collect::<Vec<_>>(),
            vec!["motion:A", "motion:B"]
        );
        assert!(report.unmatched_change_keys.is_empty());
    }

    #[test]
    fn artifact_dependencies_without_authoritative_current_versions_fail_closed() {
        let mut graph = graph(vec![artifact(
            "motion:A",
            ConstructionArtifactKind::MotionEnvelope,
            ArtifactPersistence::Recomputable,
            vec![dependency(
                DependencyDomain::EntityGeometry,
                "entity:A",
                '1',
            )],
            &[],
            'a',
        )]);
        graph.current_dependencies.clear();
        assert_eq!(
            compile_change_impact(
                &graph,
                &change_set(vec![change(
                    DependencyDomain::EntityGeometry,
                    "entity:A",
                    '1',
                    '2'
                )])
            ),
            Err(ChangeImpactError::MissingCurrentDependencyVersion(key(
                DependencyDomain::EntityGeometry,
                "entity:A"
            )))
        );
    }

    #[test]
    fn unmatched_change_key_must_still_exist_in_authoritative_snapshot() {
        let graph = graph(vec![artifact(
            "motion:A",
            ConstructionArtifactKind::MotionEnvelope,
            ArtifactPersistence::Recomputable,
            vec![dependency(
                DependencyDomain::EntityGeometry,
                "entity:A",
                '1',
            )],
            &[],
            'a',
        )]);
        let unknown = change_set(vec![change(
            DependencyDomain::MaterialLot,
            "lot:LOT-44",
            '1',
            '2',
        )]);
        assert_eq!(
            compile_change_impact(&graph, &unknown),
            Err(ChangeImpactError::ChangeTargetMissingCurrentState(key(
                DependencyDomain::MaterialLot,
                "lot:LOT-44"
            )))
        );
    }

    #[test]
    fn duplicate_authoritative_current_dependency_keys_fail_closed() {
        let mut graph = graph(vec![artifact(
            "motion:A",
            ConstructionArtifactKind::MotionEnvelope,
            ArtifactPersistence::Recomputable,
            vec![dependency(
                DependencyDomain::EntityGeometry,
                "entity:A",
                '1',
            )],
            &[],
            'a',
        )]);
        graph
            .current_dependencies
            .push(graph.current_dependencies[0].clone());
        assert_eq!(
            compile_change_impact(
                &graph,
                &change_set(vec![change(
                    DependencyDomain::EntityGeometry,
                    "entity:A",
                    '1',
                    '2'
                )])
            ),
            Err(ChangeImpactError::DuplicateCurrentDependencyVersion(key(
                DependencyDomain::EntityGeometry,
                "entity:A"
            )))
        );
    }

    #[test]
    fn generated_dependency_dags_match_independent_invalidation_closure_oracle() {
        for seed in 0..8_usize {
            let mut artifacts = Vec::new();
            let mut current_by_key = BTreeMap::<DependencyKey, String>::new();

            for index in 0..48_usize {
                let artifact_id = format!("artifact-{index:03}");
                let (kind, persistence, domain) = match index % 4 {
                    0 => (
                        ConstructionArtifactKind::MotionEnvelope,
                        ArtifactPersistence::Recomputable,
                        DependencyDomain::EntityGeometry,
                    ),
                    1 => (
                        ConstructionArtifactKind::SpaceTimeOccupancyAssessment,
                        ArtifactPersistence::Recomputable,
                        DependencyDomain::ObstacleMap,
                    ),
                    2 => (
                        ConstructionArtifactKind::DispatchReadiness,
                        ArtifactPersistence::Recomputable,
                        DependencyDomain::AuthorityPolicy,
                    ),
                    _ => (
                        ConstructionArtifactKind::AcceptedConstructionRecord,
                        ArtifactPersistence::HistoricalImmutable,
                        DependencyDomain::DesignRevision,
                    ),
                };
                let primary_key = key(domain, format!("scope:{seed}:{index}"));
                current_by_key
                    .entry(primary_key.clone())
                    .or_insert_with(|| digest('1'));
                let primary_observed = if (index + seed) % 9 == 0 {
                    digest('0')
                } else {
                    digest('1')
                };
                let mut dependencies = vec![VersionedDependency {
                    key: primary_key,
                    observed_sha256: primary_observed,
                }];

                if index % 4 == 0 {
                    let transform_key = key(DependencyDomain::CoordinateTransform, "floor:02");
                    current_by_key
                        .entry(transform_key.clone())
                        .or_insert_with(|| digest('2'));
                    dependencies.push(VersionedDependency {
                        key: transform_key,
                        observed_sha256: digest('2'),
                    });
                }

                let mut parents = Vec::new();
                if index > 0 && (index + seed) % 2 == 0 {
                    parents.push(format!("artifact-{:03}", index - 1));
                }
                if index > 2 && (index + seed) % 3 == 0 {
                    let parent = index / 2;
                    let parent_id = format!("artifact-{parent:03}");
                    if parent < index && !parents.contains(&parent_id) {
                        parents.push(parent_id);
                    }
                }

                artifacts.push(ConstructionArtifact {
                    artifact_id,
                    artifact_sha256: digest(char::from(b'a' + (index % 20) as u8)),
                    kind,
                    persistence,
                    dependencies,
                    depends_on_artifact_ids: parents,
                });
            }

            let current_dependencies: Vec<VersionedDependency> = current_by_key
                .iter()
                .map(|(key, observed_sha256)| VersionedDependency {
                    key: key.clone(),
                    observed_sha256: observed_sha256.clone(),
                })
                .collect();
            let graph = ConstructionArtifactGraph {
                graph_snapshot_sha256: digest('f'),
                current_dependencies,
                artifacts: artifacts.clone(),
            };

            let changes: Vec<DependencyTransition> = current_by_key
                .iter()
                .enumerate()
                .filter(|(position, _)| (position + seed) % 11 == 0)
                .map(|(_, (key, previous_sha256))| DependencyTransition {
                    key: key.clone(),
                    previous_sha256: previous_sha256.clone(),
                    current_sha256: digest('3'),
                })
                .collect();
            let changes = if changes.is_empty() {
                let (key, previous_sha256) = current_by_key.first_key_value().unwrap();
                vec![DependencyTransition {
                    key: key.clone(),
                    previous_sha256: previous_sha256.clone(),
                    current_sha256: digest('3'),
                }]
            } else {
                changes
            };
            let change_set = ConstructionChangeSet {
                change_id: format!("generated-change-{seed}"),
                source_event_sha256: digest('e'),
                changes,
            };

            // Independent oracle: compare every artifact input with the
            // post-change snapshot, then walk graph edges to form the closure.
            let mut effective_versions = current_by_key.clone();
            for transition in &change_set.changes {
                effective_versions
                    .insert(transition.key.clone(), transition.current_sha256.clone());
            }
            let direct: BTreeSet<String> = artifacts
                .iter()
                .filter(|artifact| {
                    artifact.dependencies.iter().any(|dependency| {
                        effective_versions.get(&dependency.key) != Some(&dependency.observed_sha256)
                    })
                })
                .map(|artifact| artifact.artifact_id.clone())
                .collect();
            let mut impacted = direct.clone();
            let mut progressed = true;
            while progressed {
                progressed = false;
                for artifact in &artifacts {
                    if !impacted.contains(&artifact.artifact_id)
                        && artifact
                            .depends_on_artifact_ids
                            .iter()
                            .any(|parent| impacted.contains(parent))
                    {
                        impacted.insert(artifact.artifact_id.clone());
                        progressed = true;
                    }
                }
            }

            let expected_historical: BTreeSet<String> = artifacts
                .iter()
                .filter(|artifact| {
                    impacted.contains(&artifact.artifact_id)
                        && artifact.persistence == ArtifactPersistence::HistoricalImmutable
                })
                .map(|artifact| artifact.artifact_id.clone())
                .collect();
            let expected_direct: BTreeSet<String> = artifacts
                .iter()
                .filter(|artifact| {
                    direct.contains(&artifact.artifact_id)
                        && artifact.persistence == ArtifactPersistence::Recomputable
                })
                .map(|artifact| artifact.artifact_id.clone())
                .collect();
            let expected_transitive: BTreeSet<String> = artifacts
                .iter()
                .filter(|artifact| {
                    impacted.contains(&artifact.artifact_id)
                        && !direct.contains(&artifact.artifact_id)
                        && artifact.persistence == ArtifactPersistence::Recomputable
                })
                .map(|artifact| artifact.artifact_id.clone())
                .collect();
            let expected_unaffected: BTreeSet<String> = artifacts
                .iter()
                .filter(|artifact| !impacted.contains(&artifact.artifact_id))
                .map(|artifact| artifact.artifact_id.clone())
                .collect();

            let report = compile_change_impact(&graph, &change_set).unwrap();
            let actual_direct: BTreeSet<String> = report
                .direct_invalidations
                .iter()
                .map(|artifact| artifact.artifact_id.clone())
                .collect();
            let actual_transitive: BTreeSet<String> = report
                .transitive_invalidations
                .iter()
                .map(|artifact| artifact.artifact_id.clone())
                .collect();
            let actual_historical: BTreeSet<String> = report
                .preserved_historical_records
                .iter()
                .map(|artifact| artifact.artifact_id.clone())
                .collect();
            let actual_unaffected: BTreeSet<String> =
                report.unaffected_artifact_ids.iter().cloned().collect();

            assert_eq!(actual_direct, expected_direct, "seed {seed}: direct set");
            assert_eq!(
                actual_transitive, expected_transitive,
                "seed {seed}: transitive set"
            );
            assert_eq!(
                actual_historical, expected_historical,
                "seed {seed}: historical set"
            );
            assert_eq!(
                actual_unaffected, expected_unaffected,
                "seed {seed}: unaffected set"
            );

            let requalification_positions: BTreeMap<&str, usize> = report
                .requalification_order
                .iter()
                .enumerate()
                .map(|(position, id)| (id.as_str(), position))
                .collect();
            for artifact in &artifacts {
                for parent in &artifact.depends_on_artifact_ids {
                    if let (Some(parent_position), Some(child_position)) = (
                        requalification_positions.get(parent.as_str()),
                        requalification_positions.get(artifact.artifact_id.as_str()),
                    ) {
                        assert!(
                            parent_position < child_position,
                            "seed {seed}: parent {parent} must precede child {}",
                            artifact.artifact_id
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn same_dependency_domain_at_the_wrong_scope_does_not_satisfy_coverage() {
        use ConstructionArtifactKind::*;
        use DependencyDomain::*;

        let dependencies = vec![
            dependency(DesignRevision, "project:alpha", '1'),
            dependency(EntityGeometry, "entity:B", '2'),
            dependency(CoordinateFrame, "floor:02", '3'),
            dependency(CoordinateTransform, "floor:02", '4'),
            dependency(RobotConfiguration, "robot:1", '5'),
            dependency(ToolConfiguration, "tool:gripper", '6'),
            dependency(PayloadConfiguration, "payload:panel", '7'),
            dependency(TrajectoryConfiguration, "trajectory:panel-A", '8'),
            dependency(SiteClockMap, "site:alpha", '9'),
        ];
        let mut motion = artifact(
            "motion:panel-A",
            MotionEnvelope,
            ArtifactPersistence::Recomputable,
            dependencies,
            &[],
            'a',
        );
        motion
            .expected_dependency_keys
            .retain(|key| key.domain != EntityGeometry);
        motion
            .expected_dependency_keys
            .push(key(EntityGeometry, "entity:A"));

        let graph = graph(vec![motion]);
        let contracts = DependencyContractSet {
            contract_sha256: digest('c'),
            contracts: vec![ArtifactDependencyContract {
                artifact_kind: MotionEnvelope,
                required_domains: minimum_required_domains(MotionEnvelope).to_vec(),
            }],
        };

        let audit = audit_dependency_coverage(&graph, &contracts).unwrap();
        assert!(audit.gaps.is_empty(), "all required domains are represented");
        assert_eq!(audit.scope_gaps.len(), 1);
        assert_eq!(
            audit.scope_gaps[0].missing_expected_keys,
            vec![key(EntityGeometry, "entity:A")]
        );
        assert_eq!(
            audit.scope_gaps[0].unexpected_observed_keys,
            vec![key(EntityGeometry, "entity:B")]
        );
        assert_eq!(audit.complete_artifacts, 0);
        assert_eq!(
            compile_change_impact_with_contract(
                &graph,
                &contracts,
                &change_set(vec![change(EntityGeometry, "entity:A", '2', 'f')]),
            ),
            Err(ChangeImpactError::DependencyCoverageIncomplete(1))
        );
    }

    #[test]
    fn owner_contract_cannot_remove_a_builtin_minimum_dependency_domain() {
        let graph = graph(vec![artifact(
            "motion:A",
            ConstructionArtifactKind::MotionEnvelope,
            ArtifactPersistence::Recomputable,
            vec![
                dependency(DependencyDomain::EntityGeometry, "entity:A", '1'),
                dependency(DependencyDomain::CoordinateTransform, "floor:02", '2'),
                dependency(DependencyDomain::RobotConfiguration, "robot:A", '3'),
                dependency(
                    DependencyDomain::PayloadConfiguration,
                    "robot:A:payload",
                    '4',
                ),
                dependency(
                    DependencyDomain::TrajectoryConfiguration,
                    "robot:A:trajectory",
                    '5',
                ),
                dependency(DependencyDomain::SiteClockMap, "site", '6'),
            ],
            &[],
            'a',
        )]);
        let contracts = DependencyContractSet {
            contract_sha256: digest('c'),
            contracts: vec![ArtifactDependencyContract {
                artifact_kind: ConstructionArtifactKind::MotionEnvelope,
                required_domains: vec![
                    DependencyDomain::EntityGeometry,
                    DependencyDomain::CoordinateTransform,
                    DependencyDomain::RobotConfiguration,
                    DependencyDomain::PayloadConfiguration,
                    DependencyDomain::TrajectoryConfiguration,
                    DependencyDomain::SiteClockMap,
                ],
            }],
        };
        assert_eq!(
            audit_dependency_coverage(&graph, &contracts),
            Err(ChangeImpactError::ContractOmitsMinimumDependencyDomain {
                artifact_kind: ConstructionArtifactKind::MotionEnvelope,
                domain: DependencyDomain::ToolConfiguration,
            })
        );
    }
}
