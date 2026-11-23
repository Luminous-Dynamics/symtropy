#![forbid(unsafe_code)]
//! Maritime profile adapter for owner-supplied transitive dependency closures.
//!
//! This crate does not traverse dependency graphs, discover infrastructure,
//! authenticate inventories, or compute cut sets. It consumes a bounded,
//! read-only closure projection and evaluates maritime role-pair obligations.
//! Digest identities are opaque equality keys, not proof of factual or
//! organizational independence.

use std::collections::{BTreeMap, BTreeSet};

pub const PROFILE_SCHEMA: &str = "maritime-channel-dependency-binding-v0.1";
pub const CLOSURE_SCHEMA: &str = "mycelix-closure-projection-consumer-contract-v0.1";
pub const MYCELIX_RECEIVER_PR: u32 = 6121;
pub const MYCELIX_RECEIVER_HEAD: &str = "92948ecc24392a25cec77dd646130f9d98e8c790";
pub const PROFILE_ID_V0_1: &str = "maritime-two-channel-v0.1";
pub const PROFILE_VERSION_V0_1: &str = "0.1.0";

fn frozen_v0_1_obligations() -> BTreeSet<StageDimension> {
    let mut obligations = BTreeSet::new();
    for (stage, dimensions) in [
        (
            MaritimeStage::Detection,
            vec![
                FaultDomainDimension::InputAcquisition,
                FaultDomainDimension::SourceEvidenceLineage,
                FaultDomainDimension::OperatorControl,
            ],
        ),
        (
            MaritimeStage::Transport,
            vec![
                FaultDomainDimension::ExecutionProvider,
                FaultDomainDimension::Runtime,
                FaultDomainDimension::TrustRoot,
            ],
        ),
        (
            MaritimeStage::PersistenceRetrieval,
            vec![
                FaultDomainDimension::SourceEvidenceLineage,
                FaultDomainDimension::ExecutionProvider,
                FaultDomainDimension::Runtime,
                FaultDomainDimension::RepositoryControl,
            ],
        ),
        (
            MaritimeStage::Interpretation,
            vec![
                FaultDomainDimension::ImplementationLineage,
                FaultDomainDimension::SpecificationLineage,
                FaultDomainDimension::SourceEvidenceLineage,
                FaultDomainDimension::TrustRoot,
            ],
        ),
        (
            MaritimeStage::GovernanceControl,
            vec![
                FaultDomainDimension::OperatorControl,
                FaultDomainDimension::RepositoryControl,
                FaultDomainDimension::TrustRoot,
                FaultDomainDimension::ExecutionProvider,
            ],
        ),
    ] {
        for dimension in dimensions {
            obligations.insert(StageDimension { stage, dimension });
        }
    }
    obligations
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Digest(pub [u8; 32]);

impl Digest {
    pub const fn from_byte(byte: u8) -> Self {
        Self([byte; 32])
    }
}

/// Exact dimension names in Mycelix ASSURE's currently proposed receiver.
/// Keep this adapter synchronized with the pinned owner contract; do not add
/// maritime-only dimensions here.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FaultDomainDimension {
    EvaluatorIdentity,
    EvaluatorProfileIdentity,
    ImplementationLineage,
    SpecificationLineage,
    ExpectedAnswerLineage,
    TestCorpusLineage,
    SourceEvidenceLineage,
    ToolchainDistribution,
    RepositoryControl,
    ExecutionProvider,
    RunnerImage,
    Runtime,
    InputAcquisition,
    OperatorControl,
    TrustRoot,
}

/// Maritime component classes are mapped into owner-native generic dimensions.
/// Multiple maritime classes may map to one owner dimension; the exact class
/// remains attached to every witness so the adapter never erases that detail.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MaritimeDependencyClass {
    /// Aggregate compatibility labels; prefer the finer-grained variants below.
    PhysicalAcquisition,
    MeasurementQualification,
    TransportFabric,
    GatewayRuntime,
    PersistenceRetrieval,
    UpstreamSourceRegistry,
    ResolverOrTransformation,
    SchemaOrCodeList,
    VerifierImplementation,
    RepositoryAuthority,
    OperatorOrTenantControl,
    SigningAuthority,
    ExecutionEnvironment,
    /// Specific maritime acquisition / metrology dependencies.
    PhysicalSensorOrReader,
    InstallationOrReadPoint,
    PowerSupplyOrUps,
    ClockOrTimeRoot,
    CalibrationOrMetrologyAuthority,
    FirmwareOrConfiguration,
    /// Specific transport and persistence dependencies.
    NetworkOrBackhaul,
    BrokerOrIntermediary,
    DatabaseOrEventStore,
    ObjectStoreArchiveIndex,
    RetrievalOrExportPath,
}

impl MaritimeDependencyClass {
    pub fn applies_to_stage(self, stage: MaritimeStage) -> bool {
        match self {
            Self::PhysicalAcquisition
            | Self::PhysicalSensorOrReader
            | Self::InstallationOrReadPoint => stage == MaritimeStage::Detection,
            Self::MeasurementQualification
            | Self::ClockOrTimeRoot
            | Self::CalibrationOrMetrologyAuthority => {
                matches!(
                    stage,
                    MaritimeStage::Detection | MaritimeStage::Interpretation
                )
            }
            Self::PowerSupplyOrUps => matches!(
                stage,
                MaritimeStage::Detection
                    | MaritimeStage::Transport
                    | MaritimeStage::PersistenceRetrieval
            ),
            Self::FirmwareOrConfiguration => {
                matches!(stage, MaritimeStage::Detection | MaritimeStage::Transport)
            }
            Self::TransportFabric
            | Self::GatewayRuntime
            | Self::NetworkOrBackhaul
            | Self::BrokerOrIntermediary => matches!(
                stage,
                MaritimeStage::Transport | MaritimeStage::PersistenceRetrieval
            ),
            Self::PersistenceRetrieval
            | Self::DatabaseOrEventStore
            | Self::ObjectStoreArchiveIndex
            | Self::RetrievalOrExportPath => stage == MaritimeStage::PersistenceRetrieval,
            Self::UpstreamSourceRegistry
            | Self::ResolverOrTransformation
            | Self::SchemaOrCodeList
            | Self::VerifierImplementation => stage == MaritimeStage::Interpretation,
            Self::RepositoryAuthority => matches!(
                stage,
                MaritimeStage::PersistenceRetrieval | MaritimeStage::GovernanceControl
            ),
            Self::ExecutionEnvironment => matches!(
                stage,
                MaritimeStage::Transport
                    | MaritimeStage::PersistenceRetrieval
                    | MaritimeStage::Interpretation
                    | MaritimeStage::GovernanceControl
            ),
            Self::OperatorOrTenantControl | Self::SigningAuthority => true,
        }
    }

    pub const fn canonical_dimension(self) -> FaultDomainDimension {
        match self {
            Self::PhysicalAcquisition
            | Self::PhysicalSensorOrReader
            | Self::InstallationOrReadPoint
            | Self::PowerSupplyOrUps => FaultDomainDimension::InputAcquisition,
            Self::MeasurementQualification
            | Self::ClockOrTimeRoot
            | Self::CalibrationOrMetrologyAuthority
            | Self::PersistenceRetrieval
            | Self::DatabaseOrEventStore
            | Self::ObjectStoreArchiveIndex
            | Self::RetrievalOrExportPath
            | Self::UpstreamSourceRegistry => FaultDomainDimension::SourceEvidenceLineage,
            Self::TransportFabric | Self::NetworkOrBackhaul | Self::BrokerOrIntermediary => {
                FaultDomainDimension::ExecutionProvider
            }
            Self::GatewayRuntime | Self::ExecutionEnvironment | Self::FirmwareOrConfiguration => {
                FaultDomainDimension::Runtime
            }
            Self::ResolverOrTransformation | Self::VerifierImplementation => {
                FaultDomainDimension::ImplementationLineage
            }
            Self::SchemaOrCodeList => FaultDomainDimension::SpecificationLineage,
            Self::RepositoryAuthority => FaultDomainDimension::RepositoryControl,
            Self::OperatorOrTenantControl => FaultDomainDimension::OperatorControl,
            Self::SigningAuthority => FaultDomainDimension::TrustRoot,
        }
    }
}

/// LEX-NET-030 dependency/corroboration relation vocabulary. These are
/// provenance relations, not causal proof or an independence score.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DependencyRelation {
    SameArtifact,
    SameIssuer,
    SameTrustRoot,
    SameOperator,
    SameUpstreamData,
    DerivedFrom,
    MirrorOrRepublish,
    PotentiallyCorrelated,
    DependencyUnknown,
    IndependenceEstablishedForProfile,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EvidenceCurrentness {
    Current,
    Stale,
    Revoked,
    Superseded,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PathWitness {
    /// Starts at the bound role node and ends at DependencyWitness.node_id.
    pub node_ids: Vec<String>,
    pub edge_ids: Vec<String>,
    pub relations: Vec<DependencyRelation>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DependencyWitness {
    /// Exact owner-native identity for (dimension, identity). This is not a
    /// locally invented ID and must already be canonicalized upstream.
    pub identity: Digest,
    pub class: MaritimeDependencyClass,
    pub node_id: String,
    pub currentness: EvidenceCurrentness,
    pub path: PathWitness,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DimensionInventory {
    /// All claim-critical dependencies for this role/dimension are present.
    Complete(Vec<DependencyWitness>),
    /// Known dependencies are retained; an unknown remainder still exists.
    Partial(Vec<DependencyWitness>),
    /// No defensible inventory is available.
    Unknown,
    /// Exclusion must carry an explicit rationale and remains visible.
    NotApplicable { rationale: String, owner: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleClosure {
    pub role_id: String,
    pub role_binding_node_id: String,
    pub dimensions: BTreeMap<StageDimension, DimensionInventory>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProjectionCurrentness {
    Current,
    Stale,
    Revoked,
    Superseded,
    Contradictory,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClosureProjection {
    pub schema: String,
    pub graph_schema_version: String,
    pub source_registry_version: String,
    pub snapshot_id: String,
    pub claim_id: String,
    pub subject_digest: Digest,
    pub profile_digest: Digest,
    pub graph_digest: Digest,
    pub closure_digest: Digest,
    pub as_of_epoch_seconds: u64,
    pub currentness: ProjectionCurrentness,
    pub roles: BTreeMap<String, RoleClosure>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MaritimeStage {
    Detection,
    Transport,
    PersistenceRetrieval,
    Interpretation,
    GovernanceControl,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct StageDimension {
    pub stage: MaritimeStage,
    pub dimension: FaultDomainDimension,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptedSharedDependency {
    pub stage: MaritimeStage,
    pub dimension: FaultDomainDimension,
    pub identity: Digest,
    pub rationale: String,
    pub impact: String,
    pub owner: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaritimeProfile {
    pub schema: String,
    pub profile_id: String,
    pub profile_version: String,
    pub claim_id: String,
    pub subject_digest: Digest,
    pub profile_digest: Digest,
    pub as_of_epoch_seconds: u64,
    pub source_role: String,
    pub witness_role: String,
    pub obligations: BTreeSet<StageDimension>,
    /// Explicitly accepted common dependencies remain visible and never turn
    /// a Shared finding into a SeparatedForProfile finding.
    pub accepted_shared_dependencies: Vec<AcceptedSharedDependency>,
}

impl MaritimeProfile {
    /// Frozen v0.1 minimum obligations. A site-specific profile may add
    /// obligations, but may not silently remove one of these in a later
    /// version or reinterpret an owner-native dimension.
    pub fn v0_1(
        claim_id: impl Into<String>,
        subject_digest: Digest,
        profile_digest: Digest,
        as_of_epoch_seconds: u64,
    ) -> Self {
        let obligations = frozen_v0_1_obligations();
        Self {
            schema: PROFILE_SCHEMA.to_owned(),
            profile_id: PROFILE_ID_V0_1.to_owned(),
            profile_version: PROFILE_VERSION_V0_1.to_owned(),
            claim_id: claim_id.into(),
            subject_digest,
            profile_digest,
            as_of_epoch_seconds,
            source_role: "source".to_owned(),
            witness_role: "witness".to_owned(),
            obligations,
            accepted_shared_dependencies: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SeparationDisposition {
    SeparatedForProfile,
    Shared,
    Incomplete,
    Indeterminate,
    NotApplicable,
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProfileDisposition {
    ProfileObligationsMet,
    ProfileObligationsNotMet,
    ProfileIncomplete,
    ProfileIndeterminate,
    ProfileInvalid,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SharedPathWitness {
    pub identity: Digest,
    pub left_node_id: String,
    pub right_node_id: String,
    pub left_path: PathWitness,
    pub right_path: PathWitness,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DimensionFinding {
    pub stage: MaritimeStage,
    pub dimension: FaultDomainDimension,
    pub disposition: SeparationDisposition,
    /// Known exact overlaps are retained even when the inventory is partial.
    pub shared_identities: BTreeSet<Digest>,
    /// Exact path witnesses are retained; duplicate paths do not multiply
    /// identities or improve the disposition.
    pub shared_paths: Vec<SharedPathWitness>,
    /// Explicitly accepted common dependencies with rationale, impact and owner.
    /// This annotation never removes or weakens the underlying Shared result.
    pub accepted_shared_dependencies: Vec<AcceptedSharedDependency>,
    pub left_known_count: usize,
    pub right_known_count: usize,
    pub left_partial: bool,
    pub right_partial: bool,
    pub left_not_applicable: bool,
    pub right_not_applicable: bool,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ValidationError {
    EmptyProfileBinding,
    UnsupportedProfileBinding,
    MissingFrozenObligations,
    InvalidProfileSchema,
    InvalidClosureSchema,
    ProfileProjectionMismatch,
    InvalidRoleBinding,
    UnexpectedRole,
    MissingRole,
    EmptyInventory,
    EmptyPartialInventory,
    EmptyNotApplicableRationale,
    InvalidSharedAcceptance,
    DuplicateSharedAcceptance,
    UnobservedSharedAcceptance,
    DimensionClassMismatch,
    DependencyClassStageMismatch,
    InvalidPathWitness,
    NonCurrentProjectionRevoked,
    NonCurrentProjectionContradictory,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Assessment {
    pub claim_id: String,
    pub profile_digest: Digest,
    pub graph_digest: Digest,
    pub closure_digest: Digest,
    pub as_of_epoch_seconds: u64,
    pub disposition: ProfileDisposition,
    pub validation_errors: Vec<ValidationError>,
    pub findings: Vec<DimensionFinding>,
}

fn inventory_parts(inventory: &DimensionInventory) -> (Vec<&DependencyWitness>, bool, bool) {
    match inventory {
        DimensionInventory::Complete(witnesses) => (witnesses.iter().collect(), false, false),
        DimensionInventory::Partial(witnesses) => (witnesses.iter().collect(), true, false),
        DimensionInventory::Unknown => (Vec::new(), false, true),
        DimensionInventory::NotApplicable { .. } => (Vec::new(), false, false),
    }
}

fn validate_path(role_binding_node_id: &str, witness: &DependencyWitness) -> bool {
    let path = &witness.path;
    if path.node_ids.len() < 2
        || path.edge_ids.len() + 1 != path.node_ids.len()
        || path.relations.len() != path.edge_ids.len()
        || path.node_ids.first().map(String::as_str) != Some(role_binding_node_id)
        || path.node_ids.last().map(String::as_str) != Some(witness.node_id.as_str())
        || witness.node_id.trim().is_empty()
    {
        return false;
    }
    if path.node_ids.iter().any(|value| value.trim().is_empty())
        || path.edge_ids.iter().any(|value| value.trim().is_empty())
    {
        return false;
    }
    let unique_nodes: BTreeSet<_> = path.node_ids.iter().collect();
    if unique_nodes.len() != path.node_ids.len() {
        return false;
    }
    !path
        .relations
        .contains(&DependencyRelation::IndependenceEstablishedForProfile)
}

fn path_has_unresolved_dependency(path: &PathWitness) -> bool {
    path.relations.iter().any(|relation| {
        matches!(
            relation,
            DependencyRelation::PotentiallyCorrelated | DependencyRelation::DependencyUnknown
        )
    })
}

fn validate_profile_and_projection(
    profile: &MaritimeProfile,
    projection: &ClosureProjection,
) -> Vec<ValidationError> {
    let mut errors = BTreeSet::new();
    if profile.schema != PROFILE_SCHEMA {
        errors.insert(ValidationError::InvalidProfileSchema);
    }
    let is_frozen_v0_1 =
        profile.profile_id == PROFILE_ID_V0_1 && profile.profile_version == PROFILE_VERSION_V0_1;
    if is_frozen_v0_1 {
        if !profile.obligations.is_superset(&frozen_v0_1_obligations()) {
            errors.insert(ValidationError::MissingFrozenObligations);
        }
    } else {
        #[cfg(not(test))]
        errors.insert(ValidationError::UnsupportedProfileBinding);
    }
    if projection.schema != CLOSURE_SCHEMA {
        errors.insert(ValidationError::InvalidClosureSchema);
    }
    if profile.profile_id.trim().is_empty()
        || profile.profile_version.trim().is_empty()
        || profile.claim_id.trim().is_empty()
        || profile.source_role.trim().is_empty()
        || profile.witness_role.trim().is_empty()
        || profile.source_role == profile.witness_role
        || profile.obligations.is_empty()
    {
        errors.insert(ValidationError::EmptyProfileBinding);
    }
    if projection.graph_schema_version.trim().is_empty()
        || projection.source_registry_version.trim().is_empty()
        || projection.snapshot_id.trim().is_empty()
        || projection.claim_id.trim().is_empty()
    {
        errors.insert(ValidationError::EmptyProfileBinding);
    }
    if profile.claim_id != projection.claim_id
        || profile.subject_digest != projection.subject_digest
        || profile.profile_digest != projection.profile_digest
        || profile.as_of_epoch_seconds != projection.as_of_epoch_seconds
    {
        errors.insert(ValidationError::ProfileProjectionMismatch);
    }
    if !projection.roles.contains_key(&profile.source_role)
        || !projection.roles.contains_key(&profile.witness_role)
    {
        errors.insert(ValidationError::MissingRole);
    }
    if projection
        .roles
        .keys()
        .any(|role| role != &profile.source_role && role != &profile.witness_role)
    {
        errors.insert(ValidationError::UnexpectedRole);
    }
    for role_id in [&profile.source_role, &profile.witness_role] {
        if let Some(role) = projection.roles.get(role_id) {
            if role.role_id != *role_id || role.role_binding_node_id.trim().is_empty() {
                errors.insert(ValidationError::InvalidRoleBinding);
            }
            for (stage_dimension, inventory) in &role.dimensions {
                match inventory {
                    DimensionInventory::Complete(witnesses) if witnesses.is_empty() => {
                        errors.insert(ValidationError::EmptyInventory);
                    }
                    DimensionInventory::Partial(witnesses) if witnesses.is_empty() => {
                        errors.insert(ValidationError::EmptyPartialInventory);
                    }
                    DimensionInventory::Partial(_) | DimensionInventory::Unknown => {}
                    DimensionInventory::NotApplicable { rationale, owner }
                        if rationale.trim().is_empty() || owner.trim().is_empty() =>
                    {
                        errors.insert(ValidationError::EmptyNotApplicableRationale);
                    }
                    _ => {}
                }
                let witnesses = match inventory {
                    DimensionInventory::Complete(items) | DimensionInventory::Partial(items) => {
                        items
                    }
                    DimensionInventory::Unknown | DimensionInventory::NotApplicable { .. } => {
                        continue;
                    }
                };
                for witness in witnesses {
                    if witness.class.canonical_dimension() != stage_dimension.dimension {
                        errors.insert(ValidationError::DimensionClassMismatch);
                    }
                    if !witness.class.applies_to_stage(stage_dimension.stage) {
                        errors.insert(ValidationError::DependencyClassStageMismatch);
                    }
                    if !validate_path(&role.role_binding_node_id, witness) {
                        errors.insert(ValidationError::InvalidPathWitness);
                    }
                }
            }
        }
    }
    match projection.currentness {
        ProjectionCurrentness::Current
        | ProjectionCurrentness::Stale
        | ProjectionCurrentness::Superseded => {}
        ProjectionCurrentness::Revoked => {
            errors.insert(ValidationError::NonCurrentProjectionRevoked);
        }
        ProjectionCurrentness::Contradictory => {
            errors.insert(ValidationError::NonCurrentProjectionContradictory);
        }
    }
    errors.into_iter().collect()
}

fn witnesses_for(
    role: &RoleClosure,
    stage_dimension: StageDimension,
) -> Option<&DimensionInventory> {
    role.dimensions.get(&stage_dimension)
}

fn assess_obligation(
    obligation: StageDimension,
    profile: &MaritimeProfile,
    projection: &ClosureProjection,
) -> DimensionFinding {
    let left = projection.roles.get(&profile.source_role);
    let right = projection.roles.get(&profile.witness_role);
    let mut finding = DimensionFinding {
        stage: obligation.stage,
        dimension: obligation.dimension,
        disposition: SeparationDisposition::Incomplete,
        shared_identities: BTreeSet::new(),
        shared_paths: Vec::new(),
        accepted_shared_dependencies: Vec::new(),
        left_known_count: 0,
        right_known_count: 0,
        left_partial: false,
        right_partial: false,
        left_not_applicable: false,
        right_not_applicable: false,
        notes: Vec::new(),
    };
    let (Some(left), Some(right)) = (left, right) else {
        finding.disposition = SeparationDisposition::Invalid;
        finding
            .notes
            .push("required role binding is absent".to_owned());
        return finding;
    };
    let (Some(left_inventory), Some(right_inventory)) = (
        witnesses_for(left, obligation),
        witnesses_for(right, obligation),
    ) else {
        finding.disposition = SeparationDisposition::Incomplete;
        finding
            .notes
            .push("required dimension is absent from closure projection".to_owned());
        return finding;
    };

    if let DimensionInventory::NotApplicable { rationale, owner } = left_inventory {
        finding.left_not_applicable = true;
        finding.notes.push(format!(
            "source role excludes dimension: {rationale}; owner: {owner}"
        ));
    }
    if let DimensionInventory::NotApplicable { rationale, owner } = right_inventory {
        finding.right_not_applicable = true;
        finding.notes.push(format!(
            "witness role excludes dimension: {rationale}; owner: {owner}"
        ));
    }

    let (left_items, left_partial, left_unknown) = inventory_parts(left_inventory);
    let (right_items, right_partial, right_unknown) = inventory_parts(right_inventory);
    finding.left_known_count = left_items.len();
    finding.right_known_count = right_items.len();
    finding.left_partial = left_partial;
    finding.right_partial = right_partial;

    let mut right_by_identity = BTreeMap::<Digest, Vec<&DependencyWitness>>::new();
    for witness in &right_items {
        right_by_identity
            .entry(witness.identity)
            .or_default()
            .push(witness);
    }
    for left_witness in &left_items {
        if let Some(right_witnesses) = right_by_identity.get(&left_witness.identity) {
            finding.shared_identities.insert(left_witness.identity);
            for right_witness in right_witnesses {
                finding.shared_paths.push(SharedPathWitness {
                    identity: left_witness.identity,
                    left_node_id: left_witness.node_id.clone(),
                    right_node_id: right_witness.node_id.clone(),
                    left_path: left_witness.path.clone(),
                    right_path: right_witness.path.clone(),
                });
            }
        }
    }
    finding.shared_paths.sort();

    let path_has_unknown_relation = left_items
        .iter()
        .chain(right_items.iter())
        .any(|item| path_has_unresolved_dependency(&item.path));
    let common_current = finding.shared_paths.iter().any(|shared| {
        left_items.iter().any(|item| {
            item.identity == shared.identity && item.currentness == EvidenceCurrentness::Current
        }) && right_items.iter().any(|item| {
            item.identity == shared.identity && item.currentness == EvidenceCurrentness::Current
        })
    });
    let any_non_current = left_items
        .iter()
        .chain(right_items.iter())
        .any(|item| item.currentness != EvidenceCurrentness::Current);
    let projection_current = projection.currentness == ProjectionCurrentness::Current;
    let any_not_applicable = finding.left_not_applicable || finding.right_not_applicable;

    if common_current && projection_current {
        finding.disposition = SeparationDisposition::Shared;
        finding
            .notes
            .push("a required canonical dependency is shared".to_owned());
    } else if path_has_unknown_relation {
        finding.disposition = SeparationDisposition::Indeterminate;
        finding.notes.push(
            "dependency path contains an unknown or potentially correlated relation".to_owned(),
        );
    } else if left_unknown || right_unknown || any_non_current || !projection_current {
        finding.disposition = SeparationDisposition::Indeterminate;
        finding
            .notes
            .push("dependency closure or currentness is unresolved".to_owned());
    } else if any_not_applicable {
        finding.disposition = SeparationDisposition::NotApplicable;
        finding.notes.push(
            "a role excludes this dimension; known dependencies on the other role remain visible"
                .to_owned(),
        );
    } else if left_partial || right_partial {
        finding.disposition = SeparationDisposition::Incomplete;
        finding
            .notes
            .push("known dependencies have an unknown remainder".to_owned());
    } else if !finding.shared_identities.is_empty() {
        finding.disposition = SeparationDisposition::Indeterminate;
        finding
            .notes
            .push("overlap is present but its witnesses are not current".to_owned());
    } else {
        finding.disposition = SeparationDisposition::SeparatedForProfile;
        finding
            .notes
            .push("complete current dependency sets are disjoint for this dimension".to_owned());
    }
    finding
}

/// Evaluate only the exact obligations in a frozen maritime profile.
/// No graph traversal, trust score, source discovery, cut-set calculation or
/// operational side effect occurs here.
pub fn assess(profile: &MaritimeProfile, projection: &ClosureProjection) -> Assessment {
    let validation_errors = validate_profile_and_projection(profile, projection);
    if !validation_errors.is_empty() {
        return Assessment {
            claim_id: profile.claim_id.clone(),
            profile_digest: profile.profile_digest,
            graph_digest: projection.graph_digest,
            closure_digest: projection.closure_digest,
            as_of_epoch_seconds: profile.as_of_epoch_seconds,
            disposition: ProfileDisposition::ProfileInvalid,
            validation_errors,
            findings: Vec::new(),
        };
    }

    let mut findings: Vec<_> = profile
        .obligations
        .iter()
        .map(|obligation| assess_obligation(*obligation, profile, projection))
        .collect();
    let mut post_validation_errors = BTreeSet::new();
    let mut seen_acceptances = BTreeSet::new();
    for acceptance in &profile.accepted_shared_dependencies {
        if acceptance.rationale.trim().is_empty()
            || acceptance.impact.trim().is_empty()
            || acceptance.owner.trim().is_empty()
            || !profile.obligations.contains(&StageDimension {
                stage: acceptance.stage,
                dimension: acceptance.dimension,
            })
        {
            post_validation_errors.insert(ValidationError::InvalidSharedAcceptance);
            continue;
        }
        if !seen_acceptances.insert((acceptance.stage, acceptance.dimension, acceptance.identity)) {
            post_validation_errors.insert(ValidationError::DuplicateSharedAcceptance);
            continue;
        }
        let Some(finding) = findings.iter_mut().find(|finding| {
            finding.stage == acceptance.stage && finding.dimension == acceptance.dimension
        }) else {
            post_validation_errors.insert(ValidationError::InvalidSharedAcceptance);
            continue;
        };
        if finding.disposition != SeparationDisposition::Shared
            || !finding.shared_identities.contains(&acceptance.identity)
        {
            post_validation_errors.insert(ValidationError::UnobservedSharedAcceptance);
            continue;
        }
        finding
            .accepted_shared_dependencies
            .push(acceptance.clone());
    }
    let validation_errors = post_validation_errors.into_iter().collect::<Vec<_>>();
    let disposition = if !validation_errors.is_empty()
        || findings
            .iter()
            .any(|finding| finding.disposition == SeparationDisposition::Invalid)
    {
        ProfileDisposition::ProfileInvalid
    } else if findings
        .iter()
        .any(|finding| finding.disposition == SeparationDisposition::Shared)
    {
        ProfileDisposition::ProfileObligationsNotMet
    } else if findings
        .iter()
        .any(|finding| finding.disposition == SeparationDisposition::Indeterminate)
    {
        ProfileDisposition::ProfileIndeterminate
    } else if findings.iter().any(|finding| {
        matches!(
            finding.disposition,
            SeparationDisposition::Incomplete | SeparationDisposition::NotApplicable
        )
    }) {
        // A NOT_APPLICABLE dimension does not silently discharge a mandatory
        // obligation in v0.1. A future profile may explicitly authorize one.
        ProfileDisposition::ProfileIncomplete
    } else {
        ProfileDisposition::ProfileObligationsMet
    };
    Assessment {
        claim_id: profile.claim_id.clone(),
        profile_digest: profile.profile_digest,
        graph_digest: projection.graph_digest,
        closure_digest: projection.closure_digest,
        as_of_epoch_seconds: profile.as_of_epoch_seconds,
        disposition,
        validation_errors,
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(role_binding: &str, dependency: &str, edge: &str) -> PathWitness {
        PathWitness {
            node_ids: vec![role_binding.to_owned(), dependency.to_owned()],
            edge_ids: vec![edge.to_owned()],
            relations: vec![DependencyRelation::DerivedFrom],
        }
    }

    fn witness(
        identity: u8,
        class: MaritimeDependencyClass,
        role_binding: &str,
        node: &str,
        edge: &str,
    ) -> DependencyWitness {
        DependencyWitness {
            identity: Digest::from_byte(identity),
            class,
            node_id: node.to_owned(),
            currentness: EvidenceCurrentness::Current,
            path: path(role_binding, node, edge),
        }
    }

    fn projection(
        left: DimensionInventory,
        right: DimensionInventory,
        dimension: FaultDomainDimension,
    ) -> (MaritimeProfile, ClosureProjection) {
        let subject = Digest::from_byte(1);
        let profile_digest = Digest::from_byte(2);
        let mut profile =
            MaritimeProfile::v0_1("claim-gate-crossing-42", subject, profile_digest, 500);
        // Unit tests isolate one role × stage × dimension. The release build
        // accepts only the named frozen v0.1 profile and requires its full set.
        profile.profile_id = "maritime-two-channel-test-subset".to_owned();
        profile.obligations = BTreeSet::from([StageDimension {
            stage: MaritimeStage::Detection,
            dimension,
        }]);
        let mut left_dimensions = BTreeMap::new();
        left_dimensions.insert(
            StageDimension {
                stage: MaritimeStage::Detection,
                dimension,
            },
            left,
        );
        let mut right_dimensions = BTreeMap::new();
        right_dimensions.insert(
            StageDimension {
                stage: MaritimeStage::Detection,
                dimension,
            },
            right,
        );
        let projection = ClosureProjection {
            schema: CLOSURE_SCHEMA.to_owned(),
            graph_schema_version: "lex-net-030-v1".to_owned(),
            source_registry_version: "registry-7".to_owned(),
            snapshot_id: "snapshot-500".to_owned(),
            claim_id: profile.claim_id.clone(),
            subject_digest: subject,
            profile_digest,
            graph_digest: Digest::from_byte(3),
            closure_digest: Digest::from_byte(4),
            as_of_epoch_seconds: 500,
            currentness: ProjectionCurrentness::Current,
            roles: BTreeMap::from([
                (
                    "source".to_owned(),
                    RoleClosure {
                        role_id: "source".to_owned(),
                        role_binding_node_id: "source-binding".to_owned(),
                        dimensions: left_dimensions,
                    },
                ),
                (
                    "witness".to_owned(),
                    RoleClosure {
                        role_id: "witness".to_owned(),
                        role_binding_node_id: "witness-binding".to_owned(),
                        dimensions: right_dimensions,
                    },
                ),
            ]),
        };
        (profile, projection)
    }

    #[test]
    fn finite_identity_set_panel_matches_independent_profile_oracle() {
        // Independent finite oracle over all non-empty subsets of a four-ID
        // universe and all Complete/Partial combinations. The oracle is
        // expressed directly as set intersection/completeness, not by calling
        // the production assessment helpers.
        let dimension = FaultDomainDimension::InputAcquisition;
        let mut checked_cases = 0usize;
        for left_mask in 1u8..16 {
            for right_mask in 1u8..16 {
                for left_partial in [false, true] {
                    for right_partial in [false, true] {
                        let ids = |mask: u8, role_binding: &str, prefix: &str| {
                            (0u8..4)
                                .filter(|index| mask & (1u8 << *index) != 0)
                                .map(|index| {
                                    witness(
                                        index + 1,
                                        MaritimeDependencyClass::PhysicalAcquisition,
                                        role_binding,
                                        &format!("{prefix}-node-{index}"),
                                        &format!("{prefix}-edge-{index}"),
                                    )
                                })
                                .collect::<Vec<_>>()
                        };
                        let left_ids = ids(left_mask, "source-binding", "source");
                        let right_ids = ids(right_mask, "witness-binding", "witness");
                        let left_set: BTreeSet<_> =
                            left_ids.iter().map(|item| item.identity).collect();
                        let right_set: BTreeSet<_> =
                            right_ids.iter().map(|item| item.identity).collect();
                        let expected = if !left_set.is_disjoint(&right_set) {
                            SeparationDisposition::Shared
                        } else if left_partial || right_partial {
                            SeparationDisposition::Incomplete
                        } else {
                            SeparationDisposition::SeparatedForProfile
                        };
                        let left_inventory = if left_partial {
                            DimensionInventory::Partial(left_ids)
                        } else {
                            DimensionInventory::Complete(left_ids)
                        };
                        let right_inventory = if right_partial {
                            DimensionInventory::Partial(right_ids)
                        } else {
                            DimensionInventory::Complete(right_ids)
                        };
                        let (profile, projection) =
                            projection(left_inventory, right_inventory, dimension);
                        let actual = assess(&profile, &projection);
                        assert_eq!(
                            actual.findings[0].disposition, expected,
                            "left={left_mask:04b} right={right_mask:04b} left_partial={left_partial} right_partial={right_partial}"
                        );
                        checked_cases += 1;
                    }
                }
            }
        }
        assert_eq!(checked_cases, 900);
    }

    #[test]
    fn named_frozen_v0_1_profile_cannot_remove_mandatory_stage_obligations() {
        let dimension = FaultDomainDimension::InputAcquisition;
        let (mut profile, projection) = projection(
            DimensionInventory::Complete(vec![witness(
                1,
                MaritimeDependencyClass::PhysicalSensorOrReader,
                "source-binding",
                "source-reader",
                "edge-source",
            )]),
            DimensionInventory::Complete(vec![witness(
                2,
                MaritimeDependencyClass::PhysicalSensorOrReader,
                "witness-binding",
                "witness-reader",
                "edge-witness",
            )]),
            dimension,
        );
        profile.profile_id = PROFILE_ID_V0_1.to_owned();
        profile.profile_version = PROFILE_VERSION_V0_1.to_owned();

        let result = assess(&profile, &projection);
        assert_eq!(result.disposition, ProfileDisposition::ProfileInvalid);
        assert!(
            result
                .validation_errors
                .contains(&ValidationError::MissingFrozenObligations)
        );
        assert!(result.findings.is_empty());
    }

    #[test]
    fn physical_acquisition_maps_to_owner_input_acquisition_dimension() {
        assert_eq!(
            MaritimeDependencyClass::PhysicalAcquisition.canonical_dimension(),
            FaultDomainDimension::InputAcquisition
        );
    }

    #[test]
    fn complete_disjoint_dependencies_meet_one_declared_obligation() {
        let dimension = FaultDomainDimension::InputAcquisition;
        let (profile, projection) = projection(
            DimensionInventory::Complete(vec![witness(
                1,
                MaritimeDependencyClass::PhysicalAcquisition,
                "source-binding",
                "source-reader",
                "edge-s1",
            )]),
            DimensionInventory::Complete(vec![witness(
                2,
                MaritimeDependencyClass::PhysicalAcquisition,
                "witness-binding",
                "witness-camera",
                "edge-w1",
            )]),
            dimension,
        );
        let result = assess(&profile, &projection);
        assert_eq!(
            result.disposition,
            ProfileDisposition::ProfileObligationsMet
        );
        assert_eq!(
            result.findings[0].disposition,
            SeparationDisposition::SeparatedForProfile
        );
    }

    #[test]
    fn different_device_ids_do_not_hide_one_shared_physical_reader() {
        let dimension = FaultDomainDimension::InputAcquisition;
        let (profile, projection) = projection(
            DimensionInventory::Complete(vec![witness(
                7,
                MaritimeDependencyClass::PhysicalAcquisition,
                "source-binding",
                "reader-alias-a",
                "edge-a",
            )]),
            DimensionInventory::Complete(vec![witness(
                7,
                MaritimeDependencyClass::PhysicalAcquisition,
                "witness-binding",
                "reader-alias-b",
                "edge-b",
            )]),
            dimension,
        );
        let result = assess(&profile, &projection);
        assert_eq!(
            result.disposition,
            ProfileDisposition::ProfileObligationsNotMet
        );
        assert_eq!(
            result.findings[0].disposition,
            SeparationDisposition::Shared
        );
    }

    #[test]
    fn transitive_common_ancestor_is_detected_from_owner_closure_witnesses() {
        let dimension = FaultDomainDimension::ExecutionProvider;
        let mut left = witness(
            9,
            MaritimeDependencyClass::TransportFabric,
            "source-binding",
            "shared-upstream-broker",
            "source-gateway-to-broker",
        );
        left.path.node_ids = vec![
            "source-binding".to_owned(),
            "source-gateway".to_owned(),
            "shared-upstream-broker".to_owned(),
        ];
        left.path.edge_ids = vec![
            "source-to-gateway".to_owned(),
            "gateway-to-broker".to_owned(),
        ];
        left.path.relations = vec![
            DependencyRelation::DerivedFrom,
            DependencyRelation::SameUpstreamData,
        ];
        let mut right = witness(
            9,
            MaritimeDependencyClass::TransportFabric,
            "witness-binding",
            "shared-upstream-broker",
            "witness-gateway-to-broker",
        );
        right.path.node_ids = vec![
            "witness-binding".to_owned(),
            "witness-gateway".to_owned(),
            "shared-upstream-broker".to_owned(),
        ];
        right.path.edge_ids = vec![
            "witness-to-gateway".to_owned(),
            "gateway-to-broker-2".to_owned(),
        ];
        right.path.relations = vec![
            DependencyRelation::DerivedFrom,
            DependencyRelation::MirrorOrRepublish,
        ];
        let (mut profile, mut projection) = projection(
            DimensionInventory::Complete(vec![left]),
            DimensionInventory::Complete(vec![right]),
            dimension,
        );
        let transport = StageDimension {
            stage: MaritimeStage::Transport,
            dimension,
        };
        profile.obligations = BTreeSet::from([transport]);
        for role in projection.roles.values_mut() {
            let inventory = role
                .dimensions
                .remove(&StageDimension {
                    stage: MaritimeStage::Detection,
                    dimension,
                })
                .unwrap();
            role.dimensions.insert(transport, inventory);
        }
        let result = assess(&profile, &projection);
        assert_eq!(
            result.findings[0].disposition,
            SeparationDisposition::Shared
        );
        assert_eq!(result.findings[0].shared_paths.len(), 1);
        assert_eq!(
            result.findings[0].shared_identities,
            BTreeSet::from([Digest::from_byte(9)])
        );
    }

    #[test]
    fn accepted_shared_dependency_keeps_shared_disposition_and_risk_owner() {
        let dimension = FaultDomainDimension::InputAcquisition;
        let (mut profile, projection) = projection(
            DimensionInventory::Complete(vec![witness(
                7,
                MaritimeDependencyClass::PhysicalSensorOrReader,
                "source-binding",
                "shared-reader-source",
                "edge-source",
            )]),
            DimensionInventory::Complete(vec![witness(
                7,
                MaritimeDependencyClass::PhysicalSensorOrReader,
                "witness-binding",
                "shared-reader-witness",
                "edge-witness",
            )]),
            dimension,
        );
        let acceptance = AcceptedSharedDependency {
            stage: MaritimeStage::Detection,
            dimension,
            identity: Digest::from_byte(7),
            rationale: "Temporary shared read-point is contractually unavoidable".to_owned(),
            impact: "A common reader fault can suppress both event channels".to_owned(),
            owner: "terminal-assurance-owner".to_owned(),
        };
        profile
            .accepted_shared_dependencies
            .push(acceptance.clone());
        let result = assess(&profile, &projection);
        assert_eq!(
            result.disposition,
            ProfileDisposition::ProfileObligationsNotMet
        );
        assert_eq!(
            result.findings[0].disposition,
            SeparationDisposition::Shared
        );
        assert!(
            result.findings[0]
                .shared_identities
                .contains(&Digest::from_byte(7))
        );
        assert_eq!(
            result.findings[0].accepted_shared_dependencies,
            vec![acceptance]
        );
    }

    #[test]
    fn accepted_shared_dependency_must_match_an_observed_overlap() {
        let dimension = FaultDomainDimension::InputAcquisition;
        let (mut profile, projection) = projection(
            DimensionInventory::Complete(vec![witness(
                1,
                MaritimeDependencyClass::PhysicalSensorOrReader,
                "source-binding",
                "source-reader",
                "edge-source",
            )]),
            DimensionInventory::Complete(vec![witness(
                2,
                MaritimeDependencyClass::PhysicalSensorOrReader,
                "witness-binding",
                "witness-reader",
                "edge-witness",
            )]),
            dimension,
        );
        profile
            .accepted_shared_dependencies
            .push(AcceptedSharedDependency {
                stage: MaritimeStage::Detection,
                dimension,
                identity: Digest::from_byte(7),
                rationale: "Not actually observed".to_owned(),
                impact: "No known impact".to_owned(),
                owner: "assurance-owner".to_owned(),
            });
        let result = assess(&profile, &projection);
        assert_eq!(result.disposition, ProfileDisposition::ProfileInvalid);
        assert!(
            result
                .validation_errors
                .contains(&ValidationError::UnobservedSharedAcceptance)
        );
    }

    #[test]
    fn duplicate_shared_acceptance_is_invalid() {
        let dimension = FaultDomainDimension::InputAcquisition;
        let (mut profile, projection) = projection(
            DimensionInventory::Complete(vec![witness(
                7,
                MaritimeDependencyClass::PhysicalSensorOrReader,
                "source-binding",
                "shared-source",
                "edge-source",
            )]),
            DimensionInventory::Complete(vec![witness(
                7,
                MaritimeDependencyClass::PhysicalSensorOrReader,
                "witness-binding",
                "shared-witness",
                "edge-witness",
            )]),
            dimension,
        );
        let acceptance = AcceptedSharedDependency {
            stage: MaritimeStage::Detection,
            dimension,
            identity: Digest::from_byte(7),
            rationale: "Accepted for test".to_owned(),
            impact: "Shared reader risk retained".to_owned(),
            owner: "assurance-owner".to_owned(),
        };
        profile.accepted_shared_dependencies = vec![acceptance.clone(), acceptance];
        let result = assess(&profile, &projection);
        assert_eq!(result.disposition, ProfileDisposition::ProfileInvalid);
        assert!(
            result
                .validation_errors
                .contains(&ValidationError::DuplicateSharedAcceptance)
        );
    }

    #[test]
    fn partial_known_overlap_is_preserved_and_blocks_separation() {
        let dimension = FaultDomainDimension::SourceEvidenceLineage;
        let (profile, projection) = projection(
            DimensionInventory::Partial(vec![witness(
                3,
                MaritimeDependencyClass::MeasurementQualification,
                "source-binding",
                "shared-clock-authority",
                "edge-s-clock",
            )]),
            DimensionInventory::Complete(vec![witness(
                3,
                MaritimeDependencyClass::MeasurementQualification,
                "witness-binding",
                "shared-clock-authority",
                "edge-w-clock",
            )]),
            dimension,
        );
        let result = assess(&profile, &projection);
        assert_eq!(
            result.findings[0].disposition,
            SeparationDisposition::Shared
        );
        assert_eq!(result.findings[0].shared_identities.len(), 1);
        assert!(result.findings[0].left_partial);
    }

    #[test]
    fn partial_disjoint_known_sets_remain_incomplete_not_separated() {
        let dimension = FaultDomainDimension::InputAcquisition;
        let (profile, projection) = projection(
            DimensionInventory::Partial(vec![witness(
                1,
                MaritimeDependencyClass::PhysicalAcquisition,
                "source-binding",
                "source-reader",
                "edge-a",
            )]),
            DimensionInventory::Complete(vec![witness(
                2,
                MaritimeDependencyClass::PhysicalAcquisition,
                "witness-binding",
                "witness-reader",
                "edge-b",
            )]),
            dimension,
        );
        let result = assess(&profile, &projection);
        assert_eq!(result.disposition, ProfileDisposition::ProfileIncomplete);
        assert_eq!(
            result.findings[0].disposition,
            SeparationDisposition::Incomplete
        );
    }

    #[test]
    fn unknown_inventory_is_indeterminate_not_disjoint() {
        let dimension = FaultDomainDimension::InputAcquisition;
        let (profile, projection) = projection(
            DimensionInventory::Unknown,
            DimensionInventory::Complete(vec![witness(
                2,
                MaritimeDependencyClass::PhysicalAcquisition,
                "witness-binding",
                "witness-reader",
                "edge-b",
            )]),
            dimension,
        );
        assert_eq!(
            assess(&profile, &projection).findings[0].disposition,
            SeparationDisposition::Indeterminate
        );
    }

    #[test]
    fn stale_dependency_evidence_never_establishes_separation() {
        let dimension = FaultDomainDimension::InputAcquisition;
        let mut stale = witness(
            1,
            MaritimeDependencyClass::PhysicalAcquisition,
            "source-binding",
            "source-reader",
            "edge-a",
        );
        stale.currentness = EvidenceCurrentness::Stale;
        let (profile, projection) = projection(
            DimensionInventory::Complete(vec![stale]),
            DimensionInventory::Complete(vec![witness(
                2,
                MaritimeDependencyClass::PhysicalAcquisition,
                "witness-binding",
                "witness-reader",
                "edge-b",
            )]),
            dimension,
        );
        assert_eq!(
            assess(&profile, &projection).findings[0].disposition,
            SeparationDisposition::Indeterminate
        );
    }

    #[test]
    fn revoked_projection_is_invalid() {
        let dimension = FaultDomainDimension::InputAcquisition;
        let (profile, mut projection) = projection(
            DimensionInventory::Complete(vec![witness(
                1,
                MaritimeDependencyClass::PhysicalAcquisition,
                "source-binding",
                "source-reader",
                "edge-a",
            )]),
            DimensionInventory::Complete(vec![witness(
                2,
                MaritimeDependencyClass::PhysicalAcquisition,
                "witness-binding",
                "witness-reader",
                "edge-b",
            )]),
            dimension,
        );
        projection.currentness = ProjectionCurrentness::Revoked;
        assert_eq!(
            assess(&profile, &projection).disposition,
            ProfileDisposition::ProfileInvalid
        );
    }

    #[test]
    fn cyclic_or_dangling_path_shape_is_invalid() {
        let dimension = FaultDomainDimension::InputAcquisition;
        for cyclic in [true, false] {
            let mut bad = witness(
                1,
                MaritimeDependencyClass::PhysicalAcquisition,
                "source-binding",
                "source-reader",
                "edge-a",
            );
            if cyclic {
                bad.path.node_ids = vec![
                    "source-binding".to_owned(),
                    "middle".to_owned(),
                    "source-binding".to_owned(),
                    "source-reader".to_owned(),
                ];
                bad.path.edge_ids = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
                bad.path.relations = vec![
                    DependencyRelation::DerivedFrom,
                    DependencyRelation::DerivedFrom,
                    DependencyRelation::DerivedFrom,
                ];
            } else {
                // One dependency edge is missing for this declared path.
                bad.path.node_ids = vec![
                    "source-binding".to_owned(),
                    "intermediate".to_owned(),
                    "source-reader".to_owned(),
                ];
                bad.path.edge_ids = vec!["a".to_owned()];
                bad.path.relations = vec![DependencyRelation::DerivedFrom];
            }
            let (profile, projection) = projection(
                DimensionInventory::Complete(vec![bad]),
                DimensionInventory::Complete(vec![witness(
                    2,
                    MaritimeDependencyClass::PhysicalAcquisition,
                    "witness-binding",
                    "witness-reader",
                    "edge-b",
                )]),
                dimension,
            );
            assert_eq!(
                assess(&profile, &projection).disposition,
                ProfileDisposition::ProfileInvalid
            );
        }
    }

    #[test]
    fn identical_digest_bytes_in_different_dimensions_do_not_overlap() {
        let input = FaultDomainDimension::InputAcquisition;
        let runtime = FaultDomainDimension::Runtime;
        let (mut profile, mut projection) = projection(
            DimensionInventory::Complete(vec![witness(
                5,
                MaritimeDependencyClass::PhysicalAcquisition,
                "source-binding",
                "source-reader",
                "edge-a",
            )]),
            DimensionInventory::Complete(vec![witness(
                6,
                MaritimeDependencyClass::PhysicalAcquisition,
                "witness-binding",
                "witness-reader",
                "edge-b",
            )]),
            input,
        );
        profile.obligations.insert(StageDimension {
            stage: MaritimeStage::Transport,
            dimension: runtime,
        });
        for role in projection.roles.values_mut() {
            role.dimensions.insert(
                StageDimension {
                    stage: MaritimeStage::Transport,
                    dimension: runtime,
                },
                DimensionInventory::Complete(vec![witness(
                    if role.role_id == "source" { 5 } else { 6 },
                    MaritimeDependencyClass::GatewayRuntime,
                    if role.role_id == "source" {
                        "source-binding"
                    } else {
                        "witness-binding"
                    },
                    if role.role_id == "source" {
                        "source-gateway"
                    } else {
                        "witness-gateway"
                    },
                    if role.role_id == "source" {
                        "edge-r1"
                    } else {
                        "edge-r2"
                    },
                )]),
            );
        }
        let result = assess(&profile, &projection);
        assert_eq!(result.findings.len(), 2);
        assert_eq!(
            result.findings[0].disposition,
            SeparationDisposition::SeparatedForProfile
        );
        assert_eq!(
            result.findings[1].disposition,
            SeparationDisposition::SeparatedForProfile
        );
    }

    #[test]
    fn dependency_class_cannot_be_assigned_to_an_incompatible_stage() {
        let dimension = FaultDomainDimension::SourceEvidenceLineage;
        let (profile, projection) = projection(
            DimensionInventory::Complete(vec![witness(
                1,
                MaritimeDependencyClass::DatabaseOrEventStore,
                "source-binding",
                "source-database",
                "edge-source-db",
            )]),
            DimensionInventory::Complete(vec![witness(
                2,
                MaritimeDependencyClass::DatabaseOrEventStore,
                "witness-binding",
                "witness-database",
                "edge-witness-db",
            )]),
            dimension,
        );
        let result = assess(&profile, &projection);
        assert_eq!(result.disposition, ProfileDisposition::ProfileInvalid);
        assert!(
            result
                .validation_errors
                .contains(&ValidationError::DependencyClassStageMismatch)
        );
    }

    #[test]
    fn same_dimension_is_evaluated_against_distinct_stage_inventories() {
        let dimension = FaultDomainDimension::SourceEvidenceLineage;
        let (mut profile, mut projection) = projection(
            DimensionInventory::Complete(vec![witness(
                1,
                MaritimeDependencyClass::MeasurementQualification,
                "source-binding",
                "shared-clock-detection",
                "edge-detection-source",
            )]),
            DimensionInventory::Complete(vec![witness(
                1,
                MaritimeDependencyClass::MeasurementQualification,
                "witness-binding",
                "shared-clock-detection",
                "edge-detection-witness",
            )]),
            dimension,
        );
        let persistence = StageDimension {
            stage: MaritimeStage::PersistenceRetrieval,
            dimension,
        };
        profile.obligations.insert(persistence);
        projection
            .roles
            .get_mut("source")
            .unwrap()
            .dimensions
            .insert(
                persistence,
                DimensionInventory::Complete(vec![witness(
                    2,
                    MaritimeDependencyClass::PersistenceRetrieval,
                    "source-binding",
                    "source-object-store",
                    "edge-persistence-source",
                )]),
            );
        projection
            .roles
            .get_mut("witness")
            .unwrap()
            .dimensions
            .insert(
                persistence,
                DimensionInventory::Complete(vec![witness(
                    3,
                    MaritimeDependencyClass::PersistenceRetrieval,
                    "witness-binding",
                    "witness-archive",
                    "edge-persistence-witness",
                )]),
            );
        let result = assess(&profile, &projection);
        assert_eq!(result.findings.len(), 2);
        assert_eq!(
            result.findings[0].disposition,
            SeparationDisposition::Shared
        );
        assert_eq!(
            result.findings[1].disposition,
            SeparationDisposition::SeparatedForProfile
        );
        assert_eq!(
            result.disposition,
            ProfileDisposition::ProfileObligationsNotMet
        );
    }

    #[test]
    fn potentially_correlated_or_unknown_path_relations_never_establish_separation() {
        let dimension = FaultDomainDimension::InputAcquisition;
        for relation in [
            DependencyRelation::PotentiallyCorrelated,
            DependencyRelation::DependencyUnknown,
        ] {
            let mut left = witness(
                1,
                MaritimeDependencyClass::PhysicalAcquisition,
                "source-binding",
                "source-reader",
                "edge-a",
            );
            left.path.relations = vec![relation];
            let (profile, projection) = projection(
                DimensionInventory::Complete(vec![left]),
                DimensionInventory::Complete(vec![witness(
                    2,
                    MaritimeDependencyClass::PhysicalAcquisition,
                    "witness-binding",
                    "witness-reader",
                    "edge-b",
                )]),
                dimension,
            );
            let result = assess(&profile, &projection);
            assert_eq!(
                result.findings[0].disposition,
                SeparationDisposition::Indeterminate,
                "{relation:?}"
            );
            assert_eq!(result.disposition, ProfileDisposition::ProfileIndeterminate);
        }
    }

    #[test]
    fn empty_partial_inventory_is_invalid_not_an_empty_known_set() {
        let dimension = FaultDomainDimension::InputAcquisition;
        let (profile, projection) = projection(
            DimensionInventory::Partial(Vec::new()),
            DimensionInventory::Complete(vec![witness(
                2,
                MaritimeDependencyClass::PhysicalAcquisition,
                "witness-binding",
                "witness-reader",
                "edge-b",
            )]),
            dimension,
        );
        let result = assess(&profile, &projection);
        assert_eq!(result.disposition, ProfileDisposition::ProfileInvalid);
        assert!(
            result
                .validation_errors
                .contains(&ValidationError::EmptyPartialInventory)
        );
    }

    #[test]
    fn duplicate_paths_do_not_inflate_shared_identity_count() {
        let dimension = FaultDomainDimension::InputAcquisition;
        let shared = witness(
            4,
            MaritimeDependencyClass::PhysicalAcquisition,
            "source-binding",
            "shared-reader",
            "edge-a",
        );
        let right = witness(
            4,
            MaritimeDependencyClass::PhysicalAcquisition,
            "witness-binding",
            "shared-reader",
            "edge-b",
        );
        let (profile, projection) = projection(
            DimensionInventory::Complete(vec![shared.clone(), shared]),
            DimensionInventory::Complete(vec![right.clone(), right]),
            dimension,
        );
        let result = assess(&profile, &projection);
        assert_eq!(result.findings[0].shared_identities.len(), 1);
        assert_eq!(result.findings[0].shared_paths.len(), 4);
    }

    #[test]
    fn profile_snapshot_mismatch_is_invalid() {
        let dimension = FaultDomainDimension::InputAcquisition;
        let (profile, mut projection) = projection(
            DimensionInventory::Complete(vec![witness(
                1,
                MaritimeDependencyClass::PhysicalAcquisition,
                "source-binding",
                "source-reader",
                "edge-a",
            )]),
            DimensionInventory::Complete(vec![witness(
                2,
                MaritimeDependencyClass::PhysicalAcquisition,
                "witness-binding",
                "witness-reader",
                "edge-b",
            )]),
            dimension,
        );
        projection.as_of_epoch_seconds += 1;
        assert_eq!(
            assess(&profile, &projection).disposition,
            ProfileDisposition::ProfileInvalid
        );
    }

    #[test]
    fn missing_required_dimension_is_incomplete() {
        let dimension = FaultDomainDimension::InputAcquisition;
        let (profile, mut projection) = projection(
            DimensionInventory::Complete(vec![witness(
                1,
                MaritimeDependencyClass::PhysicalAcquisition,
                "source-binding",
                "source-reader",
                "edge-a",
            )]),
            DimensionInventory::Complete(vec![witness(
                2,
                MaritimeDependencyClass::PhysicalAcquisition,
                "witness-binding",
                "witness-reader",
                "edge-b",
            )]),
            dimension,
        );
        projection
            .roles
            .get_mut("source")
            .unwrap()
            .dimensions
            .clear();
        assert_eq!(
            assess(&profile, &projection).findings[0].disposition,
            SeparationDisposition::Incomplete
        );
    }

    #[test]
    fn not_applicable_dimension_is_retained_but_does_not_discharge_mandatory_obligation() {
        let dimension = FaultDomainDimension::TrustRoot;
        let (profile, projection) = projection(
            DimensionInventory::NotApplicable {
                rationale: "no trust-root proof supplied in this synthetic profile".to_owned(),
                owner: "profile-review-owner".to_owned(),
            },
            DimensionInventory::Complete(vec![witness(
                2,
                MaritimeDependencyClass::SigningAuthority,
                "witness-binding",
                "witness-signing-root",
                "edge-b",
            )]),
            dimension,
        );
        let result = assess(&profile, &projection);
        assert_eq!(result.disposition, ProfileDisposition::ProfileIncomplete);
        assert_eq!(
            result.findings[0].disposition,
            SeparationDisposition::NotApplicable
        );
        assert!(result.findings[0].left_not_applicable);
        assert_eq!(result.findings[0].right_known_count, 1);
    }

    #[test]
    fn stale_counterpart_is_not_hidden_by_not_applicable_role() {
        let dimension = FaultDomainDimension::TrustRoot;
        let mut stale = witness(
            2,
            MaritimeDependencyClass::SigningAuthority,
            "witness-binding",
            "witness-signing-root",
            "edge-witness",
        );
        stale.currentness = EvidenceCurrentness::Stale;
        let (profile, projection) = projection(
            DimensionInventory::NotApplicable {
                rationale: "Source channel has no declared signing authority".to_owned(),
                owner: "source-assurance-owner".to_owned(),
            },
            DimensionInventory::Complete(vec![stale]),
            dimension,
        );
        let result = assess(&profile, &projection);
        assert_eq!(
            result.findings[0].disposition,
            SeparationDisposition::Indeterminate
        );
        assert_eq!(result.disposition, ProfileDisposition::ProfileIndeterminate);
        assert!(result.findings[0].left_not_applicable);
        assert_eq!(result.findings[0].right_known_count, 1);
    }

    #[test]
    fn fine_grained_maritime_fault_domains_map_into_owner_dimensions() {
        let expected = [
            (
                MaritimeDependencyClass::PhysicalSensorOrReader,
                FaultDomainDimension::InputAcquisition,
            ),
            (
                MaritimeDependencyClass::InstallationOrReadPoint,
                FaultDomainDimension::InputAcquisition,
            ),
            (
                MaritimeDependencyClass::PowerSupplyOrUps,
                FaultDomainDimension::InputAcquisition,
            ),
            (
                MaritimeDependencyClass::ClockOrTimeRoot,
                FaultDomainDimension::SourceEvidenceLineage,
            ),
            (
                MaritimeDependencyClass::CalibrationOrMetrologyAuthority,
                FaultDomainDimension::SourceEvidenceLineage,
            ),
            (
                MaritimeDependencyClass::FirmwareOrConfiguration,
                FaultDomainDimension::Runtime,
            ),
            (
                MaritimeDependencyClass::NetworkOrBackhaul,
                FaultDomainDimension::ExecutionProvider,
            ),
            (
                MaritimeDependencyClass::BrokerOrIntermediary,
                FaultDomainDimension::ExecutionProvider,
            ),
            (
                MaritimeDependencyClass::DatabaseOrEventStore,
                FaultDomainDimension::SourceEvidenceLineage,
            ),
            (
                MaritimeDependencyClass::ObjectStoreArchiveIndex,
                FaultDomainDimension::SourceEvidenceLineage,
            ),
            (
                MaritimeDependencyClass::RetrievalOrExportPath,
                FaultDomainDimension::SourceEvidenceLineage,
            ),
        ];
        for (class, dimension) in expected {
            assert_eq!(class.canonical_dimension(), dimension, "{class:?}");
        }
    }

    #[test]
    fn mapping_classes_remain_distinct_even_when_owner_dimension_is_shared() {
        assert_eq!(
            MaritimeDependencyClass::MeasurementQualification.canonical_dimension(),
            FaultDomainDimension::SourceEvidenceLineage
        );
        assert_eq!(
            MaritimeDependencyClass::PersistenceRetrieval.canonical_dimension(),
            FaultDomainDimension::SourceEvidenceLineage
        );
        assert_ne!(
            MaritimeDependencyClass::MeasurementQualification,
            MaritimeDependencyClass::PersistenceRetrieval
        );
    }
}
