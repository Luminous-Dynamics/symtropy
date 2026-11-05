//! Renderer-neutral, dependency-free contract for synthetic maritime service-chain cases.
//!
//! This crate validates scenario structure and evidence applicability only. It
//! does not optimize a voyage, schedule a lock, model hydraulics, authorize a
//! booking, or operate physical infrastructure.

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TickWindow {
    /// Inclusive start tick.
    pub start_tick: u64,
    /// Exclusive end tick.
    pub end_tick: u64,
}

impl TickWindow {
    pub fn is_valid(self) -> bool {
        self.start_tick < self.end_tick
    }

    pub fn covers(self, other: Self) -> bool {
        self.start_tick <= other.start_tick && self.end_tick >= other.end_tick
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseClass {
    Synthetic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceOwner {
    Symthaea,
    Symtropy,
    Mycelix,
    SolAtlas,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceRole {
    VesselEngineering,
    VesselPerformance,
    ArrivalWindow,
    SpatialTopology,
    Navigability,
    LockCompatibility,
    LockCycleWater,
    ResourceCapacity,
    EnvironmentalWindow,
    ParticipantAuthorization,
    CargoCustody,
    PortCallObservation,
    ExternalMeasurement,
    DataRights,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceState {
    Current,
    Stale,
    Unknown,
    Disputed,
    Revoked,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRef {
    pub evidence_id: String,
    pub owner: EvidenceOwner,
    pub role: EvidenceRole,
    pub source_id: String,
    pub source_revision: String,
    /// Identifies the exact fixture/source snapshot; it is not authenticated here.
    pub snapshot_ref: String,
    /// Entity, node, resource, or participant to which this evidence applies.
    pub subject_id: String,
    pub observed_at_tick: u64,
    pub valid_from_tick: u64,
    pub valid_through_tick: Option<u64>,
    /// Arrival-window evidence must bind these exact bounds.
    pub covered_window: Option<TickWindow>,
    /// Resource-capacity evidence must bind the exact unit, intervals and capacities.
    pub covered_resource_unit: Option<ResourceUnit>,
    pub covered_capacity_windows: Option<Vec<CapacityWindow>>,
    pub state: EvidenceState,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Demand {
    pub demand_id: String,
    pub participant_id: String,
    pub origin_node_id: String,
    pub destination_node_id: String,
    pub requested_amount: u64,
    pub service_unit: ServiceUnit,
    pub delivery_window: TickWindow,
    /// Opaque Mycelix reference. Presence is not verified or activated here.
    pub participant_commitment_ref: Option<String>,
    /// Opaque custody/consignment reference owned by Mycelix.
    pub cargo_custody_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceUnit {
    Tonnes,
    Teu,
    Passengers,
    Vehicles,
    Containers,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub node_id: String,
    pub kind: NodeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Origin,
    LockApproach,
    LockExit,
    Berth,
    TransferTerminal,
    Destination,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentKind {
    ChannelTransit,
    Lockage,
    PortApproach,
    BerthHandling,
    InlandTransfer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceUse {
    pub resource_id: String,
    pub amount: u64,
    pub unit: ResourceUnit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteSegment {
    pub segment_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub kind: SegmentKind,
    pub evidence_ids: Vec<String>,
    pub resource_uses: Vec<ResourceUse>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourceKind {
    ChannelWindow,
    Lock,
    Berth,
    TerminalHandling,
    ShorePower,
    InlandTransfer,
    WaterQuota,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceUnit {
    TransitSlots,
    Lockages,
    BerthSlots,
    Tonnes,
    Teu,
    Passengers,
    Vehicles,
    Containers,
    WattHours,
    Millilitres,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapacityWindow {
    pub window: TickWindow,
    pub capacity: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    pub resource_id: String,
    pub kind: ResourceKind,
    pub capacity_unit: ResourceUnit,
    pub availability: Vec<CapacityWindow>,
    pub evidence_ids: Vec<String>,
}

/// Separate meter boundaries for a synthetic service-chain alternative.
///
/// These values are deliberately not summed. Shore power may be the supply for
/// the vessel auxiliary load, so adding source-side imports to end-use loads can
/// double-count the same energy. Aggregate energy is unavailable until an
/// evidence-qualified boundary cut proves the measurements are disjoint. See
/// Symtropy PHYS-ENERGY-PORTS-001 (#3615) and the typed energy-port contract
/// (#3691).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EnergyLedger {
    /// Vessel propulsion energy measured at the propulsion boundary (Wh).
    pub propulsion_wh: u64,
    /// Vessel auxiliary / hotel-load energy measured at the load boundary (Wh).
    pub auxiliary_wh: u64,
    /// Shore-grid energy imported at the shore connection boundary (Wh).
    pub shore_power_import_wh: u64,
    /// Energy measured at the inland-transfer traction boundary (Wh).
    pub inland_transfer_wh: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentOperationTiming {
    pub segment_id: String,
    pub duration_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoyageAlternative {
    pub alternative_id: String,
    pub vessel_id: String,
    pub vessel_revision: String,
    pub speed_profile_id: String,
    pub cargo_capacity_amount: u64,
    pub cargo_capacity_unit: ServiceUnit,
    pub arrival_node_id: String,
    pub arrival_window: TickWindow,
    pub arrival_evidence_id: String,
    pub evidence_ids: Vec<String>,
    /// Ordered route segment IDs; must match the frozen corridor path.
    pub route_segment_ids: Vec<String>,
    /// One positive-duration operation envelope per route segment, in route order.
    pub operation_timings: Vec<SegmentOperationTiming>,
    pub energy: EnergyLedger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConstraintKind {
    Navigability,
    LockEligibility,
    EnvironmentalFlow,
    WaterBudget,
    EnergyCapacity,
    BerthCapacity,
    ParticipantAuthorization,
    CargoCustody,
    DataRights,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintDisposition {
    Satisfied,
    Unknown,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardConstraint {
    pub constraint_id: String,
    pub kind: ConstraintKind,
    pub disposition: ConstraintDisposition,
    pub evidence_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceChainCase {
    pub schema_version: String,
    pub case_id: String,
    pub name: String,
    pub class: CaseClass,
    pub decision_tick: u64,
    pub maximum_evidence_age_ticks: u64,
    pub tick_seconds: u64,
    pub corridor_id: String,
    pub origin_node_id: String,
    pub destination_node_id: String,
    pub demand: Demand,
    pub nodes: Vec<Node>,
    pub route_segments: Vec<RouteSegment>,
    pub resources: Vec<Resource>,
    pub voyage_alternatives: Vec<VoyageAlternative>,
    pub evidence: Vec<EvidenceRef>,
    pub hard_constraints: Vec<HardConstraint>,
    /// Read-only port-call adapter reference; timestamp semantics live elsewhere.
    pub port_call_projection_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    MissingField(&'static str),
    UnsupportedSchemaVersion,
    InvalidTickResolution,
    InvalidDemand,
    InvalidWindow(&'static str),
    DuplicateIdentifier(&'static str, String),
    UnknownNode(String),
    DisconnectedRoute(String),
    EmptyRoute,
    UnknownResource(String),
    ResourceUnitMismatch(String),
    ResourceKindUnitMismatch(String),
    InvalidResourceUse(String),
    ResourceUseExceedsDeclaredCapacity(String),
    ResourceMissingEvidence(String),
    ResourceMissingCapacityEvidence(String),
    ResourceCapacityEvidenceMismatch(String),
    MissingRequiredSegmentResource(String),
    MissingSegmentEvidenceRole(String, EvidenceRole),
    SegmentEvidenceSubjectMismatch(String, String),
    SegmentNodeKindMismatch(String),
    DuplicateResourceUse(String),
    ResourceCalendarNotCanonical(String),
    SegmentResourceKindMismatch(String),
    VesselCapacityInsufficient(String),
    VesselCapacityUnitMismatch(String),
    EmptyResourceAvailability(String),
    InvalidCapacityWindow(String),
    OverlappingCapacityWindows(String),
    UnknownEvidence(String),
    EvidenceNotCurrent(String, EvidenceState),
    EvidenceFutureDated(String),
    EvidenceNotYetValid(String),
    EvidenceTooOld(String),
    EvidenceExpired(String),
    EvidenceRoleMismatch(String),
    EvidenceOwnerMismatch(String),
    EvidenceSubjectMismatch(String),
    ArrivalIntervalEvidenceMismatch(String),
    ArrivalNodeNotOnRoute(String),
    InvalidVoyageAlternative(String),
    RouteAlternativeMismatch(String),
    SegmentOperationTimingMismatch(String),
    InvalidSegmentOperationTiming(String),
    DemandRouteMismatch,
    DemandResourceUnitMismatch(String),
    DemandResourceShortfall(String),
    HardConstraintUnknown(String),
    HardConstraintBlocked(String),
    MissingConstraintEvidence(String),
    MissingRequiredConstraint(ConstraintKind),
    DuplicateConstraintKind(ConstraintKind),
    ConstraintEvidenceRoleMismatch(String, EvidenceRole),
    ConstraintEvidenceSubjectMismatch(String),
    MissingParticipantCommitment,
    MissingCargoCustody,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn require_nonempty(value: &str, field: &'static str) -> Result<(), ValidationError> {
    if nonempty(value) {
        Ok(())
    } else {
        Err(ValidationError::MissingField(field))
    }
}

fn unique_id(
    seen: &mut BTreeSet<String>,
    id: &str,
    kind: &'static str,
) -> Result<(), ValidationError> {
    require_nonempty(id, kind)?;
    if seen.insert(id.to_owned()) {
        Ok(())
    } else {
        Err(ValidationError::DuplicateIdentifier(kind, id.to_owned()))
    }
}

fn evidence_by_id<'a>(
    case: &'a ServiceChainCase,
    evidence_id: &str,
) -> Result<&'a EvidenceRef, ValidationError> {
    case.evidence
        .iter()
        .find(|evidence| evidence.evidence_id == evidence_id)
        .ok_or_else(|| ValidationError::UnknownEvidence(evidence_id.to_owned()))
}

fn validate_evidence_use<'a>(
    case: &'a ServiceChainCase,
    evidence_id: &str,
) -> Result<&'a EvidenceRef, ValidationError> {
    let evidence = evidence_by_id(case, evidence_id)?;
    require_nonempty(&evidence.source_id, "evidence.source_id")?;
    require_nonempty(&evidence.source_revision, "evidence.source_revision")?;
    require_nonempty(&evidence.snapshot_ref, "evidence.snapshot_ref")?;
    require_nonempty(&evidence.subject_id, "evidence.subject_id")?;
    require_nonempty(&evidence.claim_ceiling, "evidence.claim_ceiling")?;

    if evidence.state != EvidenceState::Current {
        return Err(ValidationError::EvidenceNotCurrent(
            evidence_id.to_owned(),
            evidence.state,
        ));
    }
    if evidence.observed_at_tick > case.decision_tick {
        return Err(ValidationError::EvidenceFutureDated(evidence_id.to_owned()));
    }
    if case.decision_tick - evidence.observed_at_tick > case.maximum_evidence_age_ticks {
        return Err(ValidationError::EvidenceTooOld(evidence_id.to_owned()));
    }
    if evidence.valid_from_tick > case.decision_tick {
        return Err(ValidationError::EvidenceNotYetValid(evidence_id.to_owned()));
    }
    if evidence
        .valid_through_tick
        .is_none_or(|valid_through| valid_through < case.decision_tick)
    {
        return Err(ValidationError::EvidenceExpired(evidence_id.to_owned()));
    }
    if let Some(owner) = expected_owner(evidence.role) {
        validate_expected_owner(evidence, owner)?;
    }
    Ok(evidence)
}

fn expected_owner(role: EvidenceRole) -> Option<EvidenceOwner> {
    match role {
        EvidenceRole::VesselEngineering
        | EvidenceRole::VesselPerformance
        | EvidenceRole::ArrivalWindow => Some(EvidenceOwner::Symthaea),
        EvidenceRole::SpatialTopology | EvidenceRole::Navigability => Some(EvidenceOwner::SolAtlas),
        EvidenceRole::LockCompatibility
        | EvidenceRole::LockCycleWater
        | EvidenceRole::ResourceCapacity => Some(EvidenceOwner::Symtropy),
        EvidenceRole::ParticipantAuthorization | EvidenceRole::CargoCustody => {
            Some(EvidenceOwner::Mycelix)
        }
        EvidenceRole::EnvironmentalWindow
        | EvidenceRole::PortCallObservation
        | EvidenceRole::ExternalMeasurement
        | EvidenceRole::DataRights => Some(EvidenceOwner::External),
    }
}

fn resource_unit_matches_kind(kind: ResourceKind, unit: ResourceUnit) -> bool {
    match kind {
        ResourceKind::ChannelWindow => unit == ResourceUnit::TransitSlots,
        ResourceKind::Lock => unit == ResourceUnit::Lockages,
        ResourceKind::Berth => unit == ResourceUnit::BerthSlots,
        ResourceKind::TerminalHandling | ResourceKind::InlandTransfer => matches!(
            unit,
            ResourceUnit::Tonnes
                | ResourceUnit::Teu
                | ResourceUnit::Passengers
                | ResourceUnit::Vehicles
                | ResourceUnit::Containers
        ),
        ResourceKind::ShorePower => unit == ResourceUnit::WattHours,
        ResourceKind::WaterQuota => unit == ResourceUnit::Millilitres,
    }
}

fn resource_kind_allowed_for_segment(segment: SegmentKind, resource: ResourceKind) -> bool {
    match segment {
        SegmentKind::ChannelTransit | SegmentKind::PortApproach => {
            resource == ResourceKind::ChannelWindow
        }
        SegmentKind::Lockage => matches!(resource, ResourceKind::Lock | ResourceKind::WaterQuota),
        SegmentKind::BerthHandling => matches!(
            resource,
            ResourceKind::Berth | ResourceKind::TerminalHandling | ResourceKind::ShorePower
        ),
        SegmentKind::InlandTransfer => resource == ResourceKind::InlandTransfer,
    }
}

fn segment_has_required_resource(segment: SegmentKind, kinds: &BTreeSet<ResourceKind>) -> bool {
    match segment {
        SegmentKind::ChannelTransit | SegmentKind::PortApproach => {
            kinds.contains(&ResourceKind::ChannelWindow)
        }
        SegmentKind::Lockage => {
            kinds.contains(&ResourceKind::Lock) && kinds.contains(&ResourceKind::WaterQuota)
        }
        SegmentKind::BerthHandling => {
            kinds.contains(&ResourceKind::Berth) && kinds.contains(&ResourceKind::TerminalHandling)
        }
        SegmentKind::InlandTransfer => kinds.contains(&ResourceKind::InlandTransfer),
    }
}

fn validate_expected_owner(
    evidence: &EvidenceRef,
    expected_owner: EvidenceOwner,
) -> Result<(), ValidationError> {
    if evidence.owner == expected_owner {
        Ok(())
    } else {
        Err(ValidationError::EvidenceOwnerMismatch(
            evidence.evidence_id.clone(),
        ))
    }
}

fn validate_unique_case_ids(case: &ServiceChainCase) -> Result<(), ValidationError> {
    let mut nodes = BTreeSet::new();
    for node in &case.nodes {
        unique_id(&mut nodes, &node.node_id, "node.node_id")?;
    }

    let mut segments = BTreeSet::new();
    for segment in &case.route_segments {
        unique_id(&mut segments, &segment.segment_id, "segment.segment_id")?;
    }

    let mut resources = BTreeSet::new();
    for resource in &case.resources {
        unique_id(
            &mut resources,
            &resource.resource_id,
            "resource.resource_id",
        )?;
    }

    let mut evidence = BTreeSet::new();
    for item in &case.evidence {
        unique_id(&mut evidence, &item.evidence_id, "evidence.evidence_id")?;
    }

    let mut alternatives = BTreeSet::new();
    for alternative in &case.voyage_alternatives {
        unique_id(
            &mut alternatives,
            &alternative.alternative_id,
            "alternative.alternative_id",
        )?;
    }

    let mut constraints = BTreeSet::new();
    for constraint in &case.hard_constraints {
        unique_id(
            &mut constraints,
            &constraint.constraint_id,
            "constraint.constraint_id",
        )?;
    }
    Ok(())
}

fn validate_evidence_records(case: &ServiceChainCase) -> Result<(), ValidationError> {
    for evidence in &case.evidence {
        require_nonempty(&evidence.source_id, "evidence.source_id")?;
        require_nonempty(&evidence.source_revision, "evidence.source_revision")?;
        require_nonempty(&evidence.snapshot_ref, "evidence.snapshot_ref")?;
        require_nonempty(&evidence.subject_id, "evidence.subject_id")?;
        require_nonempty(&evidence.claim_ceiling, "evidence.claim_ceiling")?;
        if evidence
            .covered_window
            .is_some_and(|window| !window.is_valid())
        {
            return Err(ValidationError::InvalidWindow("evidence.covered_window"));
        }
        if evidence
            .covered_capacity_windows
            .as_ref()
            .is_some_and(|windows| {
                windows
                    .iter()
                    .any(|item| !item.window.is_valid() || item.capacity == 0)
            })
        {
            return Err(ValidationError::InvalidWindow(
                "evidence.covered_capacity_windows",
            ));
        }
    }
    Ok(())
}

fn node_by_id<'a>(case: &'a ServiceChainCase, node_id: &str) -> Option<&'a Node> {
    case.nodes.iter().find(|node| node.node_id == node_id)
}

fn segment_node_kinds_are_valid(case: &ServiceChainCase, segment: &RouteSegment) -> bool {
    let Some(from) = node_by_id(case, &segment.from_node_id) else {
        return false;
    };
    let Some(to) = node_by_id(case, &segment.to_node_id) else {
        return false;
    };
    match segment.kind {
        SegmentKind::ChannelTransit => {
            matches!(from.kind, NodeKind::Origin | NodeKind::LockExit)
                && matches!(to.kind, NodeKind::LockApproach | NodeKind::LockExit)
        }
        SegmentKind::Lockage => {
            from.kind == NodeKind::LockApproach && to.kind == NodeKind::LockExit
        }
        SegmentKind::PortApproach => from.kind == NodeKind::LockExit && to.kind == NodeKind::Berth,
        SegmentKind::BerthHandling => {
            from.kind == NodeKind::Berth && to.kind == NodeKind::TransferTerminal
        }
        SegmentKind::InlandTransfer => {
            from.kind == NodeKind::TransferTerminal && to.kind == NodeKind::Destination
        }
    }
}

fn required_segment_evidence_roles(kind: SegmentKind) -> &'static [EvidenceRole] {
    match kind {
        SegmentKind::ChannelTransit => &[
            EvidenceRole::SpatialTopology,
            EvidenceRole::Navigability,
            EvidenceRole::ResourceCapacity,
        ],
        SegmentKind::Lockage => &[
            EvidenceRole::LockCompatibility,
            EvidenceRole::LockCycleWater,
            EvidenceRole::ResourceCapacity,
        ],
        SegmentKind::PortApproach => &[EvidenceRole::Navigability, EvidenceRole::ResourceCapacity],
        SegmentKind::BerthHandling | SegmentKind::InlandTransfer => {
            &[EvidenceRole::ResourceCapacity]
        }
    }
}

fn constraint_evidence_role(kind: ConstraintKind) -> EvidenceRole {
    match kind {
        ConstraintKind::Navigability => EvidenceRole::Navigability,
        ConstraintKind::LockEligibility => EvidenceRole::LockCompatibility,
        ConstraintKind::EnvironmentalFlow => EvidenceRole::EnvironmentalWindow,
        ConstraintKind::WaterBudget => EvidenceRole::LockCycleWater,
        ConstraintKind::EnergyCapacity | ConstraintKind::BerthCapacity => {
            EvidenceRole::ResourceCapacity
        }
        ConstraintKind::ParticipantAuthorization => EvidenceRole::ParticipantAuthorization,
        ConstraintKind::CargoCustody => EvidenceRole::CargoCustody,
        ConstraintKind::DataRights => EvidenceRole::DataRights,
    }
}

fn validate_route(case: &ServiceChainCase) -> Result<(), ValidationError> {
    if !nonempty(&case.corridor_id) {
        return Err(ValidationError::MissingField("corridor_id"));
    }
    if case.demand.origin_node_id != case.origin_node_id
        || case.demand.destination_node_id != case.destination_node_id
    {
        return Err(ValidationError::DemandRouteMismatch);
    }
    if case.route_segments.is_empty() {
        return Err(ValidationError::EmptyRoute);
    }
    let node_ids: BTreeSet<&str> = case
        .nodes
        .iter()
        .map(|node| node.node_id.as_str())
        .collect();
    if !node_ids.contains(case.origin_node_id.as_str()) {
        return Err(ValidationError::UnknownNode(case.origin_node_id.clone()));
    }
    if !node_ids.contains(case.destination_node_id.as_str()) {
        return Err(ValidationError::UnknownNode(
            case.destination_node_id.clone(),
        ));
    }

    let mut expected_from = case.origin_node_id.as_str();
    for segment in &case.route_segments {
        if !node_ids.contains(segment.from_node_id.as_str()) {
            return Err(ValidationError::UnknownNode(segment.from_node_id.clone()));
        }
        if !node_ids.contains(segment.to_node_id.as_str()) {
            return Err(ValidationError::UnknownNode(segment.to_node_id.clone()));
        }
        if segment.from_node_id != expected_from {
            return Err(ValidationError::DisconnectedRoute(
                segment.segment_id.clone(),
            ));
        }
        if !segment_node_kinds_are_valid(case, segment) {
            return Err(ValidationError::SegmentNodeKindMismatch(
                segment.segment_id.clone(),
            ));
        }
        expected_from = segment.to_node_id.as_str();
    }
    if expected_from != case.destination_node_id {
        return Err(ValidationError::DisconnectedRoute(
            case.route_segments
                .last()
                .map(|segment| segment.segment_id.clone())
                .unwrap_or_default(),
        ));
    }
    Ok(())
}

fn resource_unit_for_service(unit: ServiceUnit) -> ResourceUnit {
    match unit {
        ServiceUnit::Tonnes => ResourceUnit::Tonnes,
        ServiceUnit::Teu => ResourceUnit::Teu,
        ServiceUnit::Passengers => ResourceUnit::Passengers,
        ServiceUnit::Vehicles => ResourceUnit::Vehicles,
        ServiceUnit::Containers => ResourceUnit::Containers,
    }
}

fn validate_resources(case: &ServiceChainCase) -> Result<(), ValidationError> {
    for resource in &case.resources {
        if !resource_unit_matches_kind(resource.kind, resource.capacity_unit) {
            return Err(ValidationError::ResourceKindUnitMismatch(
                resource.resource_id.clone(),
            ));
        }
        if resource.evidence_ids.is_empty() {
            return Err(ValidationError::ResourceMissingEvidence(
                resource.resource_id.clone(),
            ));
        }
        if resource.availability.is_empty() {
            return Err(ValidationError::EmptyResourceAvailability(
                resource.resource_id.clone(),
            ));
        }
        if resource.availability.windows(2).any(|pair| {
            (pair[0].window.start_tick, pair[0].window.end_tick)
                > (pair[1].window.start_tick, pair[1].window.end_tick)
        }) {
            return Err(ValidationError::ResourceCalendarNotCanonical(
                resource.resource_id.clone(),
            ));
        }
        let mut windows = resource.availability.clone();
        windows.sort_by_key(|item| (item.window.start_tick, item.window.end_tick));
        for (index, item) in windows.iter().enumerate() {
            if !item.window.is_valid() || item.capacity == 0 {
                return Err(ValidationError::InvalidCapacityWindow(
                    resource.resource_id.clone(),
                ));
            }
            if index > 0 && windows[index - 1].window.end_tick > item.window.start_tick {
                return Err(ValidationError::OverlappingCapacityWindows(
                    resource.resource_id.clone(),
                ));
            }
        }
        let mut has_capacity_evidence = false;
        for evidence_id in &resource.evidence_ids {
            let evidence = validate_evidence_use(case, evidence_id)?;
            if evidence.subject_id != resource.resource_id {
                return Err(ValidationError::EvidenceSubjectMismatch(
                    evidence_id.clone(),
                ));
            }
            if evidence.role == EvidenceRole::ResourceCapacity {
                has_capacity_evidence = true;
                if evidence.covered_resource_unit != Some(resource.capacity_unit)
                    || evidence.covered_capacity_windows.as_deref()
                        != Some(resource.availability.as_slice())
                {
                    return Err(ValidationError::ResourceCapacityEvidenceMismatch(
                        resource.resource_id.clone(),
                    ));
                }
            }
        }
        if !has_capacity_evidence {
            return Err(ValidationError::ResourceMissingCapacityEvidence(
                resource.resource_id.clone(),
            ));
        }
    }

    for segment in &case.route_segments {
        let mut seen_evidence = BTreeSet::new();
        let mut roles_present = BTreeSet::new();
        for evidence_id in &segment.evidence_ids {
            if !seen_evidence.insert(evidence_id) {
                return Err(ValidationError::DuplicateIdentifier(
                    "segment.evidence_id",
                    evidence_id.clone(),
                ));
            }
            let evidence = validate_evidence_use(case, evidence_id)?;
            roles_present.insert(evidence.role);
            match evidence.role {
                EvidenceRole::SpatialTopology => {
                    if evidence.subject_id != case.corridor_id {
                        return Err(ValidationError::SegmentEvidenceSubjectMismatch(
                            segment.segment_id.clone(),
                            evidence_id.clone(),
                        ));
                    }
                }
                EvidenceRole::Navigability
                | EvidenceRole::LockCompatibility
                | EvidenceRole::LockCycleWater => {
                    if evidence.subject_id != segment.segment_id {
                        return Err(ValidationError::SegmentEvidenceSubjectMismatch(
                            segment.segment_id.clone(),
                            evidence_id.clone(),
                        ));
                    }
                }
                EvidenceRole::ResourceCapacity => {
                    let applies_to_segment_resource = segment.resource_uses.iter().any(|usage| {
                        usage.resource_id == evidence.subject_id
                            && case.resources.iter().any(|resource| {
                                resource.resource_id == usage.resource_id
                                    && resource.evidence_ids.iter().any(|id| id == evidence_id)
                            })
                    });
                    if !applies_to_segment_resource {
                        return Err(ValidationError::SegmentEvidenceSubjectMismatch(
                            segment.segment_id.clone(),
                            evidence_id.clone(),
                        ));
                    }
                }
                _ => {
                    return Err(ValidationError::EvidenceRoleMismatch(evidence_id.clone()));
                }
            }
        }
        for required_role in required_segment_evidence_roles(segment.kind) {
            if !roles_present.contains(required_role) {
                return Err(ValidationError::MissingSegmentEvidenceRole(
                    segment.segment_id.clone(),
                    *required_role,
                ));
            }
        }

        let mut used_kinds = BTreeSet::new();
        let mut used_resource_ids = BTreeSet::new();
        for usage in &segment.resource_uses {
            if !used_resource_ids.insert(usage.resource_id.as_str()) {
                return Err(ValidationError::DuplicateResourceUse(
                    usage.resource_id.clone(),
                ));
            }
            let resource = case
                .resources
                .iter()
                .find(|resource| resource.resource_id == usage.resource_id)
                .ok_or_else(|| ValidationError::UnknownResource(usage.resource_id.clone()))?;
            if usage.amount == 0 {
                return Err(ValidationError::InvalidResourceUse(
                    usage.resource_id.clone(),
                ));
            }
            if resource.capacity_unit != usage.unit {
                return Err(ValidationError::ResourceUnitMismatch(
                    usage.resource_id.clone(),
                ));
            }
            let maximum_declared_capacity = resource
                .availability
                .iter()
                .map(|window| window.capacity)
                .max()
                .unwrap_or(0);
            if usage.amount > maximum_declared_capacity {
                return Err(ValidationError::ResourceUseExceedsDeclaredCapacity(
                    usage.resource_id.clone(),
                ));
            }
            if matches!(
                resource.kind,
                ResourceKind::TerminalHandling | ResourceKind::InlandTransfer
            ) {
                let expected_unit = resource_unit_for_service(case.demand.service_unit);
                if usage.unit != expected_unit {
                    return Err(ValidationError::DemandResourceUnitMismatch(
                        usage.resource_id.clone(),
                    ));
                }
                if usage.amount < case.demand.requested_amount {
                    return Err(ValidationError::DemandResourceShortfall(
                        usage.resource_id.clone(),
                    ));
                }
            }
            if !resource_kind_allowed_for_segment(segment.kind, resource.kind) {
                return Err(ValidationError::SegmentResourceKindMismatch(
                    segment.segment_id.clone(),
                ));
            }
            used_kinds.insert(resource.kind);
        }
        if !segment_has_required_resource(segment.kind, &used_kinds) {
            return Err(ValidationError::MissingRequiredSegmentResource(
                segment.segment_id.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_voyage_alternatives(case: &ServiceChainCase) -> Result<(), ValidationError> {
    if case.voyage_alternatives.is_empty() {
        return Err(ValidationError::InvalidVoyageAlternative(
            "no voyage alternatives".to_owned(),
        ));
    }
    let expected_route: Vec<&str> = case
        .route_segments
        .iter()
        .map(|segment| segment.segment_id.as_str())
        .collect();

    for alternative in &case.voyage_alternatives {
        if !nonempty(&alternative.vessel_id)
            || !nonempty(&alternative.vessel_revision)
            || !nonempty(&alternative.speed_profile_id)
            || !alternative.arrival_window.is_valid()
        {
            return Err(ValidationError::InvalidVoyageAlternative(
                alternative.alternative_id.clone(),
            ));
        }
        if alternative.cargo_capacity_unit != case.demand.service_unit {
            return Err(ValidationError::VesselCapacityUnitMismatch(
                alternative.alternative_id.clone(),
            ));
        }
        if alternative.cargo_capacity_amount < case.demand.requested_amount {
            return Err(ValidationError::VesselCapacityInsufficient(
                alternative.alternative_id.clone(),
            ));
        }
        let arrival_node_exists = case
            .nodes
            .iter()
            .any(|node| node.node_id == alternative.arrival_node_id);
        if !arrival_node_exists {
            return Err(ValidationError::UnknownNode(
                alternative.arrival_node_id.clone(),
            ));
        }
        if !case
            .route_segments
            .iter()
            .any(|segment| segment.to_node_id == alternative.arrival_node_id)
        {
            return Err(ValidationError::ArrivalNodeNotOnRoute(
                alternative.alternative_id.clone(),
            ));
        }

        let actual_route: Vec<&str> = alternative
            .route_segment_ids
            .iter()
            .map(String::as_str)
            .collect();
        if actual_route != expected_route {
            return Err(ValidationError::RouteAlternativeMismatch(
                alternative.alternative_id.clone(),
            ));
        }
        let timed_route: Vec<&str> = alternative
            .operation_timings
            .iter()
            .map(|timing| timing.segment_id.as_str())
            .collect();
        if timed_route != expected_route {
            return Err(ValidationError::SegmentOperationTimingMismatch(
                alternative.alternative_id.clone(),
            ));
        }
        if alternative
            .operation_timings
            .iter()
            .any(|timing| timing.duration_ticks == 0)
        {
            return Err(ValidationError::InvalidSegmentOperationTiming(
                alternative.alternative_id.clone(),
            ));
        }

        for evidence_id in &alternative.evidence_ids {
            let evidence = validate_evidence_use(case, evidence_id)?;
            if !matches!(
                evidence.role,
                EvidenceRole::VesselEngineering | EvidenceRole::VesselPerformance
            ) {
                return Err(ValidationError::EvidenceRoleMismatch(evidence_id.clone()));
            }
            validate_expected_owner(evidence, EvidenceOwner::Symthaea)?;
            if evidence.subject_id != alternative.vessel_id {
                return Err(ValidationError::EvidenceSubjectMismatch(
                    evidence_id.clone(),
                ));
            }
        }

        let arrival = validate_evidence_use(case, &alternative.arrival_evidence_id)?;
        if arrival.role != EvidenceRole::ArrivalWindow {
            return Err(ValidationError::EvidenceRoleMismatch(
                alternative.arrival_evidence_id.clone(),
            ));
        }
        validate_expected_owner(arrival, EvidenceOwner::Symthaea)?;
        if arrival.subject_id != alternative.arrival_node_id {
            return Err(ValidationError::EvidenceSubjectMismatch(
                alternative.arrival_evidence_id.clone(),
            ));
        }
        if arrival.covered_window != Some(alternative.arrival_window) {
            return Err(ValidationError::ArrivalIntervalEvidenceMismatch(
                alternative.alternative_id.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_hard_constraints(case: &ServiceChainCase) -> Result<(), ValidationError> {
    let required_constraints = [
        ConstraintKind::Navigability,
        ConstraintKind::LockEligibility,
        ConstraintKind::EnvironmentalFlow,
        ConstraintKind::WaterBudget,
        ConstraintKind::EnergyCapacity,
        ConstraintKind::BerthCapacity,
        ConstraintKind::DataRights,
    ];
    let mut present_constraints = BTreeSet::<ConstraintKind>::new();
    for constraint in &case.hard_constraints {
        if !present_constraints.insert(constraint.kind) {
            return Err(ValidationError::DuplicateConstraintKind(constraint.kind));
        }
    }
    for required in required_constraints {
        if !present_constraints.contains(&required) {
            return Err(ValidationError::MissingRequiredConstraint(required));
        }
    }
    if case.demand.participant_commitment_ref.is_some()
        && !present_constraints.contains(&ConstraintKind::ParticipantAuthorization)
    {
        return Err(ValidationError::MissingRequiredConstraint(
            ConstraintKind::ParticipantAuthorization,
        ));
    }
    if case.demand.cargo_custody_ref.is_some()
        && !present_constraints.contains(&ConstraintKind::CargoCustody)
    {
        return Err(ValidationError::MissingRequiredConstraint(
            ConstraintKind::CargoCustody,
        ));
    }

    let mut has_participant_authorization = false;
    for constraint in &case.hard_constraints {
        match constraint.disposition {
            ConstraintDisposition::Unknown => {
                return Err(ValidationError::HardConstraintUnknown(
                    constraint.constraint_id.clone(),
                ));
            }
            ConstraintDisposition::Blocked => {
                return Err(ValidationError::HardConstraintBlocked(
                    constraint.constraint_id.clone(),
                ));
            }
            ConstraintDisposition::Satisfied => {}
        }

        let evidence_id = constraint.evidence_id.as_deref().ok_or_else(|| {
            ValidationError::MissingConstraintEvidence(constraint.constraint_id.clone())
        })?;
        let evidence = validate_evidence_use(case, evidence_id)?;
        let expected_role = constraint_evidence_role(constraint.kind);
        if evidence.role != expected_role {
            return Err(ValidationError::ConstraintEvidenceRoleMismatch(
                constraint.constraint_id.clone(),
                expected_role,
            ));
        }
        let subject_is_valid = match constraint.kind {
            ConstraintKind::Navigability => case.route_segments.iter().any(|segment| {
                matches!(
                    segment.kind,
                    SegmentKind::ChannelTransit | SegmentKind::PortApproach
                ) && segment.segment_id == evidence.subject_id
            }),
            ConstraintKind::LockEligibility => case.route_segments.iter().any(|segment| {
                segment.kind == SegmentKind::Lockage && segment.segment_id == evidence.subject_id
            }),
            ConstraintKind::EnvironmentalFlow => evidence.subject_id == case.corridor_id,
            ConstraintKind::WaterBudget => case.route_segments.iter().any(|segment| {
                segment.kind == SegmentKind::Lockage && segment.segment_id == evidence.subject_id
            }),
            ConstraintKind::EnergyCapacity => case.resources.iter().any(|resource| {
                resource.kind == ResourceKind::ShorePower
                    && resource.resource_id == evidence.subject_id
            }),
            ConstraintKind::BerthCapacity => case.resources.iter().any(|resource| {
                resource.kind == ResourceKind::Berth && resource.resource_id == evidence.subject_id
            }),
            ConstraintKind::ParticipantAuthorization => {
                evidence.subject_id == case.demand.participant_id
            }
            ConstraintKind::CargoCustody => evidence.subject_id == case.demand.demand_id,
            ConstraintKind::DataRights => evidence.subject_id == case.case_id,
        };
        if !subject_is_valid {
            return Err(ValidationError::ConstraintEvidenceSubjectMismatch(
                constraint.constraint_id.clone(),
            ));
        }
        if constraint.kind == ConstraintKind::ParticipantAuthorization {
            has_participant_authorization = true;
            validate_expected_owner(evidence, EvidenceOwner::Mycelix)?;
            if evidence.role != EvidenceRole::ParticipantAuthorization {
                return Err(ValidationError::EvidenceRoleMismatch(
                    evidence_id.to_owned(),
                ));
            }
            if evidence.subject_id != case.demand.participant_id {
                return Err(ValidationError::EvidenceSubjectMismatch(
                    evidence_id.to_owned(),
                ));
            }
        }
        if constraint.kind == ConstraintKind::CargoCustody {
            validate_expected_owner(evidence, EvidenceOwner::Mycelix)?;
            if evidence.role != EvidenceRole::CargoCustody {
                return Err(ValidationError::EvidenceRoleMismatch(
                    evidence_id.to_owned(),
                ));
            }
            if evidence.subject_id != case.demand.demand_id {
                return Err(ValidationError::EvidenceSubjectMismatch(
                    evidence_id.to_owned(),
                ));
            }
        }
    }

    if case.demand.participant_commitment_ref.is_some() && !has_participant_authorization {
        return Err(ValidationError::MissingParticipantCommitment);
    }
    if case.demand.cargo_custody_ref.is_some()
        && !case.hard_constraints.iter().any(|constraint| {
            constraint.kind == ConstraintKind::CargoCustody
                && constraint.disposition == ConstraintDisposition::Satisfied
        })
    {
        return Err(ValidationError::MissingCargoCustody);
    }
    Ok(())
}

/// Validate a frozen scenario without calculating or optimizing a schedule.
///
/// Current evidence, exact arrival-window binding, route continuity, resource
/// units, capacity calendars, and every non-compensable hard constraint are
/// checked before a caller can use this case in a scheduling experiment.
pub fn validate_case(case: &ServiceChainCase) -> Result<(), ValidationError> {
    require_nonempty(&case.schema_version, "schema_version")?;
    require_nonempty(&case.case_id, "case_id")?;
    require_nonempty(&case.name, "name")?;
    if case.schema_version != "0.1.0" {
        return Err(ValidationError::UnsupportedSchemaVersion);
    }
    if case.tick_seconds == 0 {
        return Err(ValidationError::InvalidTickResolution);
    }

    require_nonempty(&case.demand.demand_id, "demand.demand_id")?;
    require_nonempty(&case.demand.participant_id, "demand.participant_id")?;
    if case.demand.requested_amount == 0 || !case.demand.delivery_window.is_valid() {
        return Err(ValidationError::InvalidDemand);
    }
    if let Some(reference) = &case.demand.participant_commitment_ref {
        require_nonempty(reference, "demand.participant_commitment_ref")?;
    }
    if let Some(reference) = &case.demand.cargo_custody_ref {
        require_nonempty(reference, "demand.cargo_custody_ref")?;
    }

    validate_unique_case_ids(case)?;
    validate_evidence_records(case)?;
    validate_route(case)?;
    validate_resources(case)?;
    validate_voyage_alternatives(case)?;
    validate_hard_constraints(case)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(
        id: &str,
        owner: EvidenceOwner,
        role: EvidenceRole,
        subject: &str,
        covered_window: Option<TickWindow>,
    ) -> EvidenceRef {
        EvidenceRef {
            evidence_id: id.into(),
            owner,
            role,
            source_id: format!("fixture:{id}"),
            source_revision: "synthetic-v1".into(),
            snapshot_ref: format!("fixture-snapshot:{id}"),
            subject_id: subject.into(),
            observed_at_tick: 0,
            valid_from_tick: 0,
            valid_through_tick: Some(100),
            covered_window,
            covered_resource_unit: None,
            covered_capacity_windows: None,
            state: EvidenceState::Current,
            claim_ceiling: "synthetic benchmark fixture only".into(),
        }
    }

    fn capacity_evidence(
        id: &str,
        subject: &str,
        unit: ResourceUnit,
        availability: Vec<CapacityWindow>,
    ) -> EvidenceRef {
        let mut evidence = evidence(
            id,
            EvidenceOwner::Symtropy,
            EvidenceRole::ResourceCapacity,
            subject,
            None,
        );
        evidence.covered_resource_unit = Some(unit);
        evidence.covered_capacity_windows = Some(availability);
        evidence
    }

    fn valid_case() -> ServiceChainCase {
        let route_segment_ids = vec![
            "channel-01".to_owned(),
            "lock-01".to_owned(),
            "approach-01".to_owned(),
            "turnaround-01".to_owned(),
            "rail-01".to_owned(),
        ];
        let arrival_fast = TickWindow {
            start_tick: 10,
            end_tick: 13,
        };
        let arrival_slow = TickWindow {
            start_tick: 15,
            end_tick: 18,
        };

        let evidence = vec![
            evidence(
                "topology",
                EvidenceOwner::SolAtlas,
                EvidenceRole::SpatialTopology,
                "corridor-01",
                None,
            ),
            evidence(
                "depth",
                EvidenceOwner::SolAtlas,
                EvidenceRole::Navigability,
                "channel-01",
                None,
            ),
            evidence(
                "approach-depth",
                EvidenceOwner::SolAtlas,
                EvidenceRole::Navigability,
                "approach-01",
                None,
            ),
            evidence(
                "data-rights",
                EvidenceOwner::External,
                EvidenceRole::DataRights,
                "synthetic-lock-to-rail-001",
                None,
            ),
            evidence(
                "vessel-capability",
                EvidenceOwner::Symthaea,
                EvidenceRole::VesselEngineering,
                "vessel-01",
                None,
            ),
            evidence(
                "vessel-performance",
                EvidenceOwner::Symthaea,
                EvidenceRole::VesselPerformance,
                "vessel-01",
                None,
            ),
            evidence(
                "arrival-fast",
                EvidenceOwner::Symthaea,
                EvidenceRole::ArrivalWindow,
                "lock-approach",
                Some(arrival_fast),
            ),
            evidence(
                "arrival-slow",
                EvidenceOwner::Symthaea,
                EvidenceRole::ArrivalWindow,
                "lock-approach",
                Some(arrival_slow),
            ),
            evidence(
                "lock-compatibility",
                EvidenceOwner::Symtropy,
                EvidenceRole::LockCompatibility,
                "lock-01",
                None,
            ),
            evidence(
                "lock-water",
                EvidenceOwner::Symtropy,
                EvidenceRole::LockCycleWater,
                "lock-01",
                None,
            ),
            capacity_evidence(
                "lock-calendar",
                "lock-01",
                ResourceUnit::Lockages,
                vec![CapacityWindow {
                    window: TickWindow {
                        start_tick: 15,
                        end_tick: 35,
                    },
                    capacity: 1,
                }],
            ),
            capacity_evidence(
                "water-calendar",
                "water-budget-01",
                ResourceUnit::Millilitres,
                vec![CapacityWindow {
                    window: TickWindow {
                        start_tick: 15,
                        end_tick: 35,
                    },
                    capacity: 5_000,
                }],
            ),
            capacity_evidence(
                "berth-calendar",
                "berth-01",
                ResourceUnit::BerthSlots,
                vec![CapacityWindow {
                    window: TickWindow {
                        start_tick: 18,
                        end_tick: 40,
                    },
                    capacity: 1,
                }],
            ),
            capacity_evidence(
                "shore-calendar",
                "shore-01",
                ResourceUnit::WattHours,
                vec![CapacityWindow {
                    window: TickWindow {
                        start_tick: 18,
                        end_tick: 40,
                    },
                    capacity: 1_000,
                }],
            ),
            capacity_evidence(
                "rail-calendar",
                "rail-01",
                ResourceUnit::Tonnes,
                vec![CapacityWindow {
                    window: TickWindow {
                        start_tick: 28,
                        end_tick: 60,
                    },
                    capacity: 10,
                }],
            ),
            evidence(
                "environment",
                EvidenceOwner::External,
                EvidenceRole::EnvironmentalWindow,
                "corridor-01",
                None,
            ),
            capacity_evidence(
                "channel-calendar",
                "channel-window-01",
                ResourceUnit::TransitSlots,
                vec![CapacityWindow {
                    window: TickWindow {
                        start_tick: 5,
                        end_tick: 35,
                    },
                    capacity: 1,
                }],
            ),
            capacity_evidence(
                "handling-calendar",
                "terminal-01",
                ResourceUnit::Tonnes,
                vec![CapacityWindow {
                    window: TickWindow {
                        start_tick: 18,
                        end_tick: 40,
                    },
                    capacity: 20,
                }],
            ),
            evidence(
                "participant-auth",
                EvidenceOwner::Mycelix,
                EvidenceRole::ParticipantAuthorization,
                "participant-01",
                None,
            ),
        ];

        let nodes = vec![
            Node {
                node_id: "origin".into(),
                kind: NodeKind::Origin,
            },
            Node {
                node_id: "lock-approach".into(),
                kind: NodeKind::LockApproach,
            },
            Node {
                node_id: "lock-exit".into(),
                kind: NodeKind::LockExit,
            },
            Node {
                node_id: "berth".into(),
                kind: NodeKind::Berth,
            },
            Node {
                node_id: "railhead".into(),
                kind: NodeKind::TransferTerminal,
            },
            Node {
                node_id: "destination".into(),
                kind: NodeKind::Destination,
            },
        ];

        let resources = vec![
            Resource {
                resource_id: "channel-window-01".into(),
                kind: ResourceKind::ChannelWindow,
                capacity_unit: ResourceUnit::TransitSlots,
                availability: vec![CapacityWindow {
                    window: TickWindow {
                        start_tick: 5,
                        end_tick: 35,
                    },
                    capacity: 1,
                }],
                evidence_ids: vec!["channel-calendar".into()],
            },
            Resource {
                resource_id: "lock-01".into(),
                kind: ResourceKind::Lock,
                capacity_unit: ResourceUnit::Lockages,
                availability: vec![CapacityWindow {
                    window: TickWindow {
                        start_tick: 12,
                        end_tick: 40,
                    },
                    capacity: 1,
                }],
                evidence_ids: vec!["lock-water".into(), "lock-calendar".into()],
            },
            Resource {
                resource_id: "water-budget-01".into(),
                kind: ResourceKind::WaterQuota,
                capacity_unit: ResourceUnit::Millilitres,
                availability: vec![CapacityWindow {
                    window: TickWindow {
                        start_tick: 12,
                        end_tick: 40,
                    },
                    capacity: 5_000,
                }],
                evidence_ids: vec!["water-calendar".into()],
            },
            Resource {
                resource_id: "berth-01".into(),
                kind: ResourceKind::Berth,
                capacity_unit: ResourceUnit::BerthSlots,
                availability: vec![CapacityWindow {
                    window: TickWindow {
                        start_tick: 18,
                        end_tick: 40,
                    },
                    capacity: 1,
                }],
                evidence_ids: vec!["berth-calendar".into()],
            },
            Resource {
                resource_id: "shore-01".into(),
                kind: ResourceKind::ShorePower,
                capacity_unit: ResourceUnit::WattHours,
                availability: vec![CapacityWindow {
                    window: TickWindow {
                        start_tick: 18,
                        end_tick: 40,
                    },
                    capacity: 1_000,
                }],
                evidence_ids: vec!["shore-calendar".into()],
            },
            Resource {
                resource_id: "terminal-01".into(),
                kind: ResourceKind::TerminalHandling,
                capacity_unit: ResourceUnit::Tonnes,
                availability: vec![CapacityWindow {
                    window: TickWindow {
                        start_tick: 18,
                        end_tick: 40,
                    },
                    capacity: 20,
                }],
                evidence_ids: vec!["handling-calendar".into()],
            },
            Resource {
                resource_id: "rail-01".into(),
                kind: ResourceKind::InlandTransfer,
                capacity_unit: ResourceUnit::Tonnes,
                availability: vec![CapacityWindow {
                    window: TickWindow {
                        start_tick: 28,
                        end_tick: 60,
                    },
                    capacity: 10,
                }],
                evidence_ids: vec!["rail-calendar".into()],
            },
        ];

        let route_segments = vec![
            RouteSegment {
                segment_id: "channel-01".into(),
                from_node_id: "origin".into(),
                to_node_id: "lock-approach".into(),
                kind: SegmentKind::ChannelTransit,
                evidence_ids: vec!["topology".into(), "depth".into(), "channel-calendar".into()],
                resource_uses: vec![ResourceUse {
                    resource_id: "channel-window-01".into(),
                    amount: 1,
                    unit: ResourceUnit::TransitSlots,
                }],
            },
            RouteSegment {
                segment_id: "lock-01".into(),
                from_node_id: "lock-approach".into(),
                to_node_id: "lock-exit".into(),
                kind: SegmentKind::Lockage,
                evidence_ids: vec![
                    "lock-compatibility".into(),
                    "lock-water".into(),
                    "lock-calendar".into(),
                    "water-calendar".into(),
                ],
                resource_uses: vec![
                    ResourceUse {
                        resource_id: "lock-01".into(),
                        amount: 1,
                        unit: ResourceUnit::Lockages,
                    },
                    ResourceUse {
                        resource_id: "water-budget-01".into(),
                        amount: 4_000,
                        unit: ResourceUnit::Millilitres,
                    },
                ],
            },
            RouteSegment {
                segment_id: "approach-01".into(),
                from_node_id: "lock-exit".into(),
                to_node_id: "berth".into(),
                kind: SegmentKind::PortApproach,
                evidence_ids: vec!["approach-depth".into(), "channel-calendar".into()],
                resource_uses: vec![ResourceUse {
                    resource_id: "channel-window-01".into(),
                    amount: 1,
                    unit: ResourceUnit::TransitSlots,
                }],
            },
            RouteSegment {
                segment_id: "turnaround-01".into(),
                from_node_id: "berth".into(),
                to_node_id: "railhead".into(),
                kind: SegmentKind::BerthHandling,
                evidence_ids: vec![
                    "berth-calendar".into(),
                    "handling-calendar".into(),
                    "shore-calendar".into(),
                ],
                resource_uses: vec![
                    ResourceUse {
                        resource_id: "berth-01".into(),
                        amount: 1,
                        unit: ResourceUnit::BerthSlots,
                    },
                    ResourceUse {
                        resource_id: "terminal-01".into(),
                        amount: 10,
                        unit: ResourceUnit::Tonnes,
                    },
                    ResourceUse {
                        resource_id: "shore-01".into(),
                        amount: 800,
                        unit: ResourceUnit::WattHours,
                    },
                ],
            },
            RouteSegment {
                segment_id: "rail-01".into(),
                from_node_id: "railhead".into(),
                to_node_id: "destination".into(),
                kind: SegmentKind::InlandTransfer,
                evidence_ids: vec!["rail-calendar".into()],
                resource_uses: vec![ResourceUse {
                    resource_id: "rail-01".into(),
                    amount: 10,
                    unit: ResourceUnit::Tonnes,
                }],
            },
        ];

        let alternatives = vec![
            VoyageAlternative {
                alternative_id: "fast-high-energy".into(),
                vessel_id: "vessel-01".into(),
                vessel_revision: "vessel-r1".into(),
                speed_profile_id: "speed-fast".into(),
                cargo_capacity_amount: 12,
                cargo_capacity_unit: ServiceUnit::Tonnes,
                arrival_node_id: "lock-approach".into(),
                arrival_window: arrival_fast,
                arrival_evidence_id: "arrival-fast".into(),
                evidence_ids: vec!["vessel-capability".into(), "vessel-performance".into()],
                route_segment_ids: route_segment_ids.clone(),
                operation_timings: vec![
                    SegmentOperationTiming {
                        segment_id: "channel-01".into(),
                        duration_ticks: 2,
                    },
                    SegmentOperationTiming {
                        segment_id: "lock-01".into(),
                        duration_ticks: 2,
                    },
                    SegmentOperationTiming {
                        segment_id: "approach-01".into(),
                        duration_ticks: 1,
                    },
                    SegmentOperationTiming {
                        segment_id: "turnaround-01".into(),
                        duration_ticks: 10,
                    },
                    SegmentOperationTiming {
                        segment_id: "rail-01".into(),
                        duration_ticks: 5,
                    },
                ],
                energy: EnergyLedger {
                    propulsion_wh: 1_800,
                    auxiliary_wh: 200,
                    shore_power_import_wh: 800,
                    inland_transfer_wh: 0,
                },
            },
            VoyageAlternative {
                alternative_id: "slow-low-energy".into(),
                vessel_id: "vessel-01".into(),
                vessel_revision: "vessel-r1".into(),
                speed_profile_id: "speed-economy".into(),
                cargo_capacity_amount: 12,
                cargo_capacity_unit: ServiceUnit::Tonnes,
                arrival_node_id: "lock-approach".into(),
                arrival_window: arrival_slow,
                arrival_evidence_id: "arrival-slow".into(),
                evidence_ids: vec!["vessel-capability".into(), "vessel-performance".into()],
                route_segment_ids: route_segment_ids.clone(),
                operation_timings: vec![
                    SegmentOperationTiming {
                        segment_id: "channel-01".into(),
                        duration_ticks: 2,
                    },
                    SegmentOperationTiming {
                        segment_id: "lock-01".into(),
                        duration_ticks: 2,
                    },
                    SegmentOperationTiming {
                        segment_id: "approach-01".into(),
                        duration_ticks: 1,
                    },
                    SegmentOperationTiming {
                        segment_id: "turnaround-01".into(),
                        duration_ticks: 10,
                    },
                    SegmentOperationTiming {
                        segment_id: "rail-01".into(),
                        duration_ticks: 5,
                    },
                ],
                energy: EnergyLedger {
                    propulsion_wh: 1_300,
                    auxiliary_wh: 200,
                    shore_power_import_wh: 800,
                    inland_transfer_wh: 0,
                },
            },
        ];

        ServiceChainCase {
            schema_version: "0.1.0".into(),
            case_id: "synthetic-lock-to-rail-001".into(),
            name: "Synthetic vessel-lock-berth-to-rail handoff".into(),
            class: CaseClass::Synthetic,
            decision_tick: 0,
            maximum_evidence_age_ticks: 20,
            tick_seconds: 60,
            corridor_id: "corridor-01".into(),
            origin_node_id: "origin".into(),
            destination_node_id: "destination".into(),
            demand: Demand {
                demand_id: "demand-01".into(),
                participant_id: "participant-01".into(),
                origin_node_id: "origin".into(),
                destination_node_id: "destination".into(),
                requested_amount: 10,
                service_unit: ServiceUnit::Tonnes,
                delivery_window: TickWindow {
                    start_tick: 0,
                    end_tick: 80,
                },
                participant_commitment_ref: Some("mycelix-fixture:commitment-01".into()),
                cargo_custody_ref: None,
            },
            nodes,
            route_segments,
            resources,
            voyage_alternatives: alternatives,
            evidence,
            hard_constraints: vec![
                HardConstraint {
                    constraint_id: "constraint-navigability".into(),
                    kind: ConstraintKind::Navigability,
                    disposition: ConstraintDisposition::Satisfied,
                    evidence_id: Some("depth".into()),
                },
                HardConstraint {
                    constraint_id: "constraint-lock".into(),
                    kind: ConstraintKind::LockEligibility,
                    disposition: ConstraintDisposition::Satisfied,
                    evidence_id: Some("lock-compatibility".into()),
                },
                HardConstraint {
                    constraint_id: "constraint-environment".into(),
                    kind: ConstraintKind::EnvironmentalFlow,
                    disposition: ConstraintDisposition::Satisfied,
                    evidence_id: Some("environment".into()),
                },
                HardConstraint {
                    constraint_id: "constraint-water".into(),
                    kind: ConstraintKind::WaterBudget,
                    disposition: ConstraintDisposition::Satisfied,
                    evidence_id: Some("lock-water".into()),
                },
                HardConstraint {
                    constraint_id: "constraint-energy".into(),
                    kind: ConstraintKind::EnergyCapacity,
                    disposition: ConstraintDisposition::Satisfied,
                    evidence_id: Some("shore-calendar".into()),
                },
                HardConstraint {
                    constraint_id: "constraint-berth".into(),
                    kind: ConstraintKind::BerthCapacity,
                    disposition: ConstraintDisposition::Satisfied,
                    evidence_id: Some("berth-calendar".into()),
                },
                HardConstraint {
                    constraint_id: "constraint-data-rights".into(),
                    kind: ConstraintKind::DataRights,
                    disposition: ConstraintDisposition::Satisfied,
                    evidence_id: Some("data-rights".into()),
                },
                HardConstraint {
                    constraint_id: "constraint-participant".into(),
                    kind: ConstraintKind::ParticipantAuthorization,
                    disposition: ConstraintDisposition::Satisfied,
                    evidence_id: Some("participant-auth".into()),
                },
            ],
            port_call_projection_ref: Some("pco-fixture:port-call-01".into()),
        }
    }

    #[test]
    fn synthetic_vessel_to_rail_case_has_a_valid_composable_contract() {
        let case = valid_case();
        assert_eq!(validate_case(&case), Ok(()));
        assert_eq!(case.voyage_alternatives[0].energy.total_wh(), 2_000);
        assert_eq!(case.voyage_alternatives[1].energy.total_wh(), 1_500);
    }

    #[test]
    fn arrival_evidence_must_bind_exact_interval_bounds() {
        let mut case = valid_case();
        case.voyage_alternatives[0].arrival_window.end_tick += 1;
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::ArrivalIntervalEvidenceMismatch(
                "fast-high-energy".into()
            ))
        );
    }

    #[test]
    fn unknown_hard_constraint_fails_closed() {
        let mut case = valid_case();
        case.hard_constraints[0].disposition = ConstraintDisposition::Unknown;
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::HardConstraintUnknown(
                "constraint-navigability".into()
            ))
        );
    }

    #[test]
    fn blocked_hard_constraint_fails_closed() {
        let mut case = valid_case();
        case.hard_constraints[1].disposition = ConstraintDisposition::Blocked;
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::HardConstraintBlocked(
                "constraint-lock".into()
            ))
        );
    }

    #[test]
    fn stale_or_future_dated_evidence_cannot_be_used() {
        let mut stale = valid_case();
        stale.evidence[1].state = EvidenceState::Stale;
        assert_eq!(
            validate_case(&stale),
            Err(ValidationError::EvidenceNotCurrent(
                "depth".into(),
                EvidenceState::Stale
            ))
        );

        let mut future = valid_case();
        future.evidence[1].observed_at_tick = 1;
        assert_eq!(
            validate_case(&future),
            Err(ValidationError::EvidenceFutureDated("depth".into()))
        );
    }

    #[test]
    fn route_must_be_contiguous_from_origin_to_destination() {
        let mut case = valid_case();
        case.route_segments[2].from_node_id = "origin".into();
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::DisconnectedRoute("approach-01".into()))
        );
    }

    #[test]
    fn resource_unit_mismatch_is_not_silently_coerced() {
        let mut case = valid_case();
        case.route_segments[3].resource_uses[1].unit = ResourceUnit::Teu;
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::ResourceUnitMismatch("terminal-01".into()))
        );
    }

    #[test]
    fn required_hard_constraints_cannot_be_omitted() {
        let mut case = valid_case();
        case.hard_constraints
            .retain(|constraint| constraint.kind != ConstraintKind::EnvironmentalFlow);
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::MissingRequiredConstraint(
                ConstraintKind::EnvironmentalFlow
            ))
        );
    }

    #[test]
    fn hard_constraint_evidence_role_must_match_constraint_kind() {
        let mut case = valid_case();
        let navigability = case
            .hard_constraints
            .iter_mut()
            .find(|constraint| constraint.kind == ConstraintKind::Navigability)
            .expect("navigability constraint exists");
        navigability.evidence_id = Some("environment".into());
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::ConstraintEvidenceRoleMismatch(
                "constraint-navigability".into(),
                EvidenceRole::Navigability
            ))
        );
    }

    #[test]
    fn every_route_segment_requires_its_semantic_evidence_roles() {
        let mut case = valid_case();
        case.route_segments[1]
            .evidence_ids
            .retain(|id| id != "lock-compatibility");
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::MissingSegmentEvidenceRole(
                "lock-01".into(),
                EvidenceRole::LockCompatibility
            ))
        );
    }

    #[test]
    fn resource_use_cannot_exceed_every_declared_capacity_window() {
        let mut case = valid_case();
        let shore = case
            .resources
            .iter_mut()
            .find(|resource| resource.resource_id == "shore-01")
            .expect("shore resource exists");
        shore.availability[0].capacity = 799;
        let evidence = case
            .evidence
            .iter_mut()
            .find(|evidence| evidence.evidence_id == "shore-calendar")
            .expect("shore capacity evidence exists");
        evidence
            .covered_capacity_windows
            .as_mut()
            .expect("capacity windows are bound")[0]
            .capacity = 799;
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::ResourceUseExceedsDeclaredCapacity(
                "shore-01".into()
            ))
        );
    }

    #[test]
    fn resource_capacity_calendar_must_be_canonical() {
        let mut case = valid_case();
        case.resources[1].availability.push(CapacityWindow {
            window: TickWindow {
                start_tick: 5,
                end_tick: 10,
            },
            capacity: 1,
        });
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::ResourceCalendarNotCanonical(
                "lock-01".into()
            ))
        );
    }

    #[test]
    fn segment_cannot_allocate_the_same_resource_twice() {
        let mut case = valid_case();
        let duplicated = case.route_segments[0].resource_uses[0].clone();
        case.route_segments[0].resource_uses.push(duplicated);
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::DuplicateResourceUse(
                "channel-window-01".into()
            ))
        );
    }

    #[test]
    fn route_segment_kind_must_match_endpoint_node_kinds() {
        let mut case = valid_case();
        let approach = case
            .nodes
            .iter_mut()
            .find(|node| node.node_id == "lock-approach")
            .expect("lock approach node exists");
        approach.kind = NodeKind::Berth;
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::SegmentNodeKindMismatch(
                "channel-01".into()
            ))
        );
    }

    #[test]
    fn duplicate_hard_constraint_kinds_are_rejected() {
        let mut case = valid_case();
        let mut duplicate = case
            .hard_constraints
            .iter()
            .find(|constraint| constraint.kind == ConstraintKind::WaterBudget)
            .expect("water constraint exists")
            .clone();
        duplicate.constraint_id = "constraint-water-duplicate".into();
        case.hard_constraints.push(duplicate);
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::DuplicateConstraintKind(
                ConstraintKind::WaterBudget
            ))
        );
    }

    #[test]
    fn data_rights_gate_is_mandatory_and_source_owned() {
        let mut missing = valid_case();
        missing
            .hard_constraints
            .retain(|constraint| constraint.kind != ConstraintKind::DataRights);
        assert_eq!(
            validate_case(&missing),
            Err(ValidationError::MissingRequiredConstraint(
                ConstraintKind::DataRights
            ))
        );

        let mut wrong_owner = valid_case();
        let evidence = wrong_owner
            .evidence
            .iter_mut()
            .find(|evidence| evidence.evidence_id == "data-rights")
            .expect("data rights evidence exists");
        evidence.owner = EvidenceOwner::Symtropy;
        assert_eq!(
            validate_case(&wrong_owner),
            Err(ValidationError::EvidenceOwnerMismatch("data-rights".into()))
        );
    }

    #[test]
    fn cargo_custody_requires_mycelix_owned_evidence() {
        let mut case = valid_case();
        case.demand.cargo_custody_ref = Some("mycelix-fixture:custody-01".into());
        case.evidence.push(evidence(
            "cargo-custody",
            EvidenceOwner::Mycelix,
            EvidenceRole::CargoCustody,
            "demand-01",
            None,
        ));
        case.hard_constraints.push(HardConstraint {
            constraint_id: "constraint-cargo-custody".into(),
            kind: ConstraintKind::CargoCustody,
            disposition: ConstraintDisposition::Satisfied,
            evidence_id: Some("cargo-custody".into()),
        });
        assert_eq!(validate_case(&case), Ok(()));

        let mut revoked = case.clone();
        revoked
            .evidence
            .iter_mut()
            .find(|evidence| evidence.evidence_id == "cargo-custody")
            .expect("cargo custody evidence exists")
            .state = EvidenceState::Revoked;
        assert_eq!(
            validate_case(&revoked),
            Err(ValidationError::EvidenceNotCurrent(
                "cargo-custody".into(),
                EvidenceState::Revoked
            ))
        );
    }

    #[test]
    fn participant_authorization_revocation_invalidates_the_scenario() {
        let mut case = valid_case();
        case.evidence
            .iter_mut()
            .find(|evidence| evidence.evidence_id == "participant-auth")
            .expect("participant authorization exists")
            .state = EvidenceState::Revoked;
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::EvidenceNotCurrent(
                "participant-auth".into(),
                EvidenceState::Revoked
            ))
        );
    }

    #[test]
    fn demand_endpoints_must_match_the_frozen_route() {
        let mut case = valid_case();
        case.demand.destination_node_id = "berth".into();
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::DemandRouteMismatch)
        );
    }

    #[test]
    fn handling_and_inland_transfer_must_cover_full_demand_in_same_unit() {
        let mut short = valid_case();
        short.route_segments[3].resource_uses[1].amount = 9;
        assert_eq!(
            validate_case(&short),
            Err(ValidationError::DemandResourceShortfall(
                "terminal-01".into()
            ))
        );

        let mut mismatched = valid_case();
        let rail = mismatched
            .resources
            .iter_mut()
            .find(|resource| resource.resource_id == "rail-01")
            .expect("rail resource exists");
        rail.capacity_unit = ResourceUnit::Teu;
        mismatched.route_segments[4].resource_uses[0].unit = ResourceUnit::Teu;
        let rail_evidence = mismatched
            .evidence
            .iter_mut()
            .find(|evidence| evidence.evidence_id == "rail-calendar")
            .expect("rail capacity evidence exists");
        rail_evidence.covered_resource_unit = Some(ResourceUnit::Teu);
        assert_eq!(
            validate_case(&mismatched),
            Err(ValidationError::DemandResourceUnitMismatch(
                "rail-01".into()
            ))
        );
    }

    #[test]
    fn resource_calendar_must_match_its_exact_evidence_snapshot() {
        let mut case = valid_case();
        case.resources[0].availability[0].capacity += 1;
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::ResourceCapacityEvidenceMismatch(
                "channel-window-01".into()
            ))
        );
    }

    #[test]
    fn resource_windows_must_be_valid_non_overlapping_and_positive() {
        let mut overlap = valid_case();
        overlap.resources[1].availability.push(CapacityWindow {
            window: TickWindow {
                start_tick: 30,
                end_tick: 45,
            },
            capacity: 1,
        });
        assert_eq!(
            validate_case(&overlap),
            Err(ValidationError::OverlappingCapacityWindows(
                "lock-01".into()
            ))
        );

        let mut zero = valid_case();
        zero.resources[1].availability[0].capacity = 0;
        assert_eq!(
            validate_case(&zero),
            Err(ValidationError::InvalidCapacityWindow("lock-01".into()))
        );
    }

    #[test]
    fn participant_authority_must_come_from_mycelix() {
        let mut case = valid_case();
        let evidence = case
            .evidence
            .iter_mut()
            .find(|evidence| evidence.evidence_id == "participant-auth")
            .expect("fixture evidence exists");
        evidence.owner = EvidenceOwner::Symtropy;
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::EvidenceOwnerMismatch(
                "participant-auth".into()
            ))
        );
    }

    #[test]
    fn vessel_cargo_capacity_must_cover_the_declared_demand() {
        let mut insufficient = valid_case();
        insufficient.voyage_alternatives[0].cargo_capacity_amount = 9;
        assert_eq!(
            validate_case(&insufficient),
            Err(ValidationError::VesselCapacityInsufficient(
                "fast-high-energy".into()
            ))
        );

        let mut incomparable = valid_case();
        incomparable.voyage_alternatives[0].cargo_capacity_unit = ServiceUnit::Teu;
        assert_eq!(
            validate_case(&incomparable),
            Err(ValidationError::VesselCapacityUnitMismatch(
                "fast-high-energy".into()
            ))
        );
    }

    #[test]
    fn resource_kind_has_a_fixed_capacity_unit() {
        let mut case = valid_case();
        case.resources[0].capacity_unit = ResourceUnit::Tonnes;
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::ResourceKindUnitMismatch(
                "channel-window-01".into()
            ))
        );
    }

    #[test]
    fn frozen_30_case_adversarial_corpus_has_explicit_expected_results() {
        let mut cases: Vec<(&str, ServiceChainCase, Result<(), ValidationError>)> = Vec::new();

        cases.push(("valid corridor", valid_case(), Ok(())));

        let mut slow_only = valid_case();
        slow_only.voyage_alternatives.remove(0);
        cases.push((
            "slow voyage alternative remains admissible",
            slow_only,
            Ok(()),
        ));

        let mut arrival_changed = valid_case();
        arrival_changed.voyage_alternatives[0]
            .arrival_window
            .end_tick += 1;
        cases.push((
            "arrival window changed without new evidence",
            arrival_changed,
            Err(ValidationError::ArrivalIntervalEvidenceMismatch(
                "fast-high-energy".into(),
            )),
        ));

        let mut stale_depth = valid_case();
        stale_depth
            .evidence
            .iter_mut()
            .find(|item| item.evidence_id == "depth")
            .expect("depth evidence exists")
            .state = EvidenceState::Stale;
        cases.push((
            "stale channel-depth evidence",
            stale_depth,
            Err(ValidationError::EvidenceNotCurrent(
                "depth".into(),
                EvidenceState::Stale,
            )),
        ));

        let mut future_topology = valid_case();
        future_topology
            .evidence
            .iter_mut()
            .find(|item| item.evidence_id == "topology")
            .expect("topology evidence exists")
            .observed_at_tick = 1;
        cases.push((
            "future-dated topology evidence",
            future_topology,
            Err(ValidationError::EvidenceFutureDated("topology".into())),
        ));

        let mut expired_vessel = valid_case();
        expired_vessel
            .evidence
            .iter_mut()
            .find(|item| item.evidence_id == "vessel-performance")
            .expect("vessel performance evidence exists")
            .valid_through_tick = None;
        cases.push((
            "expired vessel performance envelope",
            expired_vessel,
            Err(ValidationError::EvidenceExpired(
                "vessel-performance".into(),
            )),
        ));

        let mut too_old = valid_case();
        too_old.decision_tick = 50;
        cases.push((
            "evidence older than declared age bound",
            too_old,
            Err(ValidationError::EvidenceTooOld("channel-calendar".into())),
        ));

        let mut short_payload = valid_case();
        short_payload.voyage_alternatives[0].cargo_capacity_amount = 9;
        cases.push((
            "vessel payload below demand",
            short_payload,
            Err(ValidationError::VesselCapacityInsufficient(
                "fast-high-energy".into(),
            )),
        ));

        let mut wrong_vessel_unit = valid_case();
        wrong_vessel_unit.voyage_alternatives[0].cargo_capacity_unit = ServiceUnit::Teu;
        cases.push((
            "vessel payload in unlike unit",
            wrong_vessel_unit,
            Err(ValidationError::VesselCapacityUnitMismatch(
                "fast-high-energy".into(),
            )),
        ));

        let mut disconnected = valid_case();
        disconnected.route_segments[2].from_node_id = "origin".into();
        cases.push((
            "disconnected route",
            disconnected,
            Err(ValidationError::DisconnectedRoute("approach-01".into())),
        ));

        let mut wrong_node_kind = valid_case();
        wrong_node_kind
            .nodes
            .iter_mut()
            .find(|node| node.node_id == "lock-approach")
            .expect("lock approach node exists")
            .kind = NodeKind::Berth;
        cases.push((
            "segment endpoint has wrong node kind",
            wrong_node_kind,
            Err(ValidationError::SegmentNodeKindMismatch(
                "channel-01".into(),
            )),
        ));

        let mut wrong_arrival_path = valid_case();
        wrong_arrival_path.voyage_alternatives[0]
            .route_segment_ids
            .reverse();
        cases.push((
            "voyage alternative uses different corridor path",
            wrong_arrival_path,
            Err(ValidationError::RouteAlternativeMismatch(
                "fast-high-energy".into(),
            )),
        ));

        let mut changed_calendar = valid_case();
        changed_calendar.resources[0].availability[0].capacity += 1;
        cases.push((
            "resource capacity differs from evidence snapshot",
            changed_calendar,
            Err(ValidationError::ResourceCapacityEvidenceMismatch(
                "channel-window-01".into(),
            )),
        ));

        let mut overlapping_calendar = valid_case();
        overlapping_calendar.resources[0]
            .availability
            .push(CapacityWindow {
                window: TickWindow {
                    start_tick: 10,
                    end_tick: 40,
                },
                capacity: 1,
            });
        cases.push((
            "overlapping resource calendar",
            overlapping_calendar,
            Err(ValidationError::OverlappingCapacityWindows(
                "channel-window-01".into(),
            )),
        ));

        let mut unordered_calendar = valid_case();
        unordered_calendar.resources[0]
            .availability
            .push(CapacityWindow {
                window: TickWindow {
                    start_tick: 0,
                    end_tick: 2,
                },
                capacity: 1,
            });
        cases.push((
            "noncanonical resource calendar order",
            unordered_calendar,
            Err(ValidationError::ResourceCalendarNotCanonical(
                "channel-window-01".into(),
            )),
        ));

        let mut zero_capacity = valid_case();
        zero_capacity.resources[0].availability[0].capacity = 0;
        cases.push((
            "zero channel capacity",
            zero_capacity,
            Err(ValidationError::InvalidCapacityWindow(
                "channel-window-01".into(),
            )),
        ));

        let mut energy_shortfall = valid_case();
        let shore = energy_shortfall
            .resources
            .iter_mut()
            .find(|resource| resource.resource_id == "shore-01")
            .expect("shore resource exists");
        shore.availability[0].capacity = 799;
        let shore_evidence = energy_shortfall
            .evidence
            .iter_mut()
            .find(|item| item.evidence_id == "shore-calendar")
            .expect("shore evidence exists");
        shore_evidence
            .covered_capacity_windows
            .as_mut()
            .expect("shore capacity snapshot exists")[0]
            .capacity = 799;
        cases.push((
            "shore-power use exceeds all capacity windows",
            energy_shortfall,
            Err(ValidationError::ResourceUseExceedsDeclaredCapacity(
                "shore-01".into(),
            )),
        ));

        let mut missing_resource_evidence = valid_case();
        missing_resource_evidence.resources[1]
            .evidence_ids
            .retain(|id| id != "lock-calendar");
        cases.push((
            "resource has no capacity evidence",
            missing_resource_evidence,
            Err(ValidationError::ResourceMissingCapacityEvidence(
                "lock-01".into(),
            )),
        ));

        let mut duplicate_use = valid_case();
        let repeated_use = duplicate_use.route_segments[0].resource_uses[0].clone();
        duplicate_use.route_segments[0]
            .resource_uses
            .push(repeated_use);
        cases.push((
            "same resource allocated twice in one segment",
            duplicate_use,
            Err(ValidationError::DuplicateResourceUse(
                "channel-window-01".into(),
            )),
        ));

        let mut unlike_service_unit = valid_case();
        let terminal = unlike_service_unit
            .resources
            .iter_mut()
            .find(|resource| resource.resource_id == "terminal-01")
            .expect("terminal resource exists");
        terminal.capacity_unit = ResourceUnit::Teu;
        unlike_service_unit.route_segments[3].resource_uses[1].unit = ResourceUnit::Teu;
        unlike_service_unit
            .evidence
            .iter_mut()
            .find(|item| item.evidence_id == "handling-calendar")
            .expect("handling evidence exists")
            .covered_resource_unit = Some(ResourceUnit::Teu);
        cases.push((
            "terminal handling unit differs from demand unit",
            unlike_service_unit,
            Err(ValidationError::DemandResourceUnitMismatch(
                "terminal-01".into(),
            )),
        ));

        let mut insufficient_handling = valid_case();
        insufficient_handling.route_segments[3].resource_uses[1].amount = 9;
        cases.push((
            "terminal handling amount below demand",
            insufficient_handling,
            Err(ValidationError::DemandResourceShortfall(
                "terminal-01".into(),
            )),
        ));

        let mut missing_lock_role = valid_case();
        missing_lock_role.route_segments[1]
            .evidence_ids
            .retain(|id| id != "lock-compatibility");
        cases.push((
            "lockage missing compatibility evidence role",
            missing_lock_role,
            Err(ValidationError::MissingSegmentEvidenceRole(
                "lock-01".into(),
                EvidenceRole::LockCompatibility,
            )),
        ));

        let mut missing_required_gate = valid_case();
        missing_required_gate
            .hard_constraints
            .retain(|constraint| constraint.kind != ConstraintKind::EnvironmentalFlow);
        cases.push((
            "environmental flow gate omitted",
            missing_required_gate,
            Err(ValidationError::MissingRequiredConstraint(
                ConstraintKind::EnvironmentalFlow,
            )),
        ));

        let mut duplicate_gate = valid_case();
        let mut duplicate_water = duplicate_gate
            .hard_constraints
            .iter()
            .find(|constraint| constraint.kind == ConstraintKind::WaterBudget)
            .expect("water constraint exists")
            .clone();
        duplicate_water.constraint_id = "constraint-water-duplicate".into();
        duplicate_gate.hard_constraints.push(duplicate_water);
        cases.push((
            "duplicate water gate",
            duplicate_gate,
            Err(ValidationError::DuplicateConstraintKind(
                ConstraintKind::WaterBudget,
            )),
        ));

        let mut unknown_environment = valid_case();
        unknown_environment
            .hard_constraints
            .iter_mut()
            .find(|constraint| constraint.kind == ConstraintKind::EnvironmentalFlow)
            .expect("environmental constraint exists")
            .disposition = ConstraintDisposition::Unknown;
        cases.push((
            "environmental flow is unknown",
            unknown_environment,
            Err(ValidationError::HardConstraintUnknown(
                "constraint-environment".into(),
            )),
        ));

        let mut blocked_berth = valid_case();
        blocked_berth
            .hard_constraints
            .iter_mut()
            .find(|constraint| constraint.kind == ConstraintKind::BerthCapacity)
            .expect("berth constraint exists")
            .disposition = ConstraintDisposition::Blocked;
        cases.push((
            "berth capacity is blocked",
            blocked_berth,
            Err(ValidationError::HardConstraintBlocked(
                "constraint-berth".into(),
            )),
        ));

        let mut wrong_gate_role = valid_case();
        wrong_gate_role
            .hard_constraints
            .iter_mut()
            .find(|constraint| constraint.kind == ConstraintKind::Navigability)
            .expect("navigability constraint exists")
            .evidence_id = Some("environment".into());
        cases.push((
            "navigability gate references environmental role",
            wrong_gate_role,
            Err(ValidationError::ConstraintEvidenceRoleMismatch(
                "constraint-navigability".into(),
                EvidenceRole::Navigability,
            )),
        ));

        let mut missing_rights = valid_case();
        missing_rights
            .hard_constraints
            .retain(|constraint| constraint.kind != ConstraintKind::DataRights);
        cases.push((
            "data rights gate omitted",
            missing_rights,
            Err(ValidationError::MissingRequiredConstraint(
                ConstraintKind::DataRights,
            )),
        ));

        let mut wrong_rights_owner = valid_case();
        wrong_rights_owner
            .evidence
            .iter_mut()
            .find(|item| item.evidence_id == "data-rights")
            .expect("data rights evidence exists")
            .owner = EvidenceOwner::Symtropy;
        cases.push((
            "data rights evidence has wrong owner",
            wrong_rights_owner,
            Err(ValidationError::EvidenceOwnerMismatch("data-rights".into())),
        ));

        let mut revoked_participant = valid_case();
        revoked_participant
            .evidence
            .iter_mut()
            .find(|item| item.evidence_id == "participant-auth")
            .expect("participant authority evidence exists")
            .state = EvidenceState::Revoked;
        cases.push((
            "participant authority revoked",
            revoked_participant,
            Err(ValidationError::EvidenceNotCurrent(
                "participant-auth".into(),
                EvidenceState::Revoked,
            )),
        ));

        let mut missing_custody = valid_case();
        missing_custody.demand.cargo_custody_ref = Some("mycelix:consignment-01".into());
        cases.push((
            "cargo custody reference without custody gate",
            missing_custody,
            Err(ValidationError::MissingRequiredConstraint(
                ConstraintKind::CargoCustody,
            )),
        ));

        let mut valid_custody = valid_case();
        valid_custody.demand.cargo_custody_ref = Some("mycelix:consignment-01".into());
        valid_custody.evidence.push(evidence(
            "cargo-custody",
            EvidenceOwner::Mycelix,
            EvidenceRole::CargoCustody,
            "demand-01",
            None,
        ));
        valid_custody.hard_constraints.push(HardConstraint {
            constraint_id: "constraint-cargo-custody".into(),
            kind: ConstraintKind::CargoCustody,
            disposition: ConstraintDisposition::Satisfied,
            evidence_id: Some("cargo-custody".into()),
        });
        cases.push(("valid Mycelix cargo custody", valid_custody, Ok(())));

        assert!(
            cases.len() >= 30,
            "frozen service-chain corpus must not silently shrink"
        );
        for (name, case, expected) in cases {
            assert_eq!(
                validate_case(&case),
                expected,
                "frozen scenario case: {name}"
            );
        }
    }

    #[test]
    fn evidence_older_than_the_declared_freshness_limit_is_rejected() {
        let mut case = valid_case();
        case.decision_tick = 10;
        case.maximum_evidence_age_ticks = 5;
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::EvidenceTooOld("channel-calendar".into()))
        );
    }

    #[test]
    fn expired_or_revoked_evidence_cannot_be_used() {
        let mut expired = valid_case();
        let evidence = expired
            .evidence
            .iter_mut()
            .find(|evidence| evidence.evidence_id == "depth")
            .expect("fixture evidence exists");
        evidence.valid_through_tick = Some(0);
        expired.decision_tick = 1;
        assert_eq!(
            validate_case(&expired),
            Err(ValidationError::EvidenceExpired("depth".into()))
        );

        let mut revoked = valid_case();
        let evidence = revoked
            .evidence
            .iter_mut()
            .find(|evidence| evidence.evidence_id == "depth")
            .expect("fixture evidence exists");
        evidence.state = EvidenceState::Revoked;
        assert_eq!(
            validate_case(&revoked),
            Err(ValidationError::EvidenceNotCurrent(
                "depth".into(),
                EvidenceState::Revoked
            ))
        );
    }

    #[test]
    fn voyage_operation_timings_must_match_every_route_segment() {
        let mut missing = valid_case();
        missing.voyage_alternatives[0].operation_timings.pop();
        assert_eq!(
            validate_case(&missing),
            Err(ValidationError::SegmentOperationTimingMismatch(
                "fast-high-energy".into()
            ))
        );

        let mut zero = valid_case();
        zero.voyage_alternatives[0].operation_timings[0].duration_ticks = 0;
        assert_eq!(
            validate_case(&zero),
            Err(ValidationError::InvalidSegmentOperationTiming(
                "fast-high-energy".into()
            ))
        );
    }

    #[test]
    fn arrival_node_must_be_a_route_node() {
        let mut case = valid_case();
        case.voyage_alternatives[0].arrival_node_id = "destination".into();
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::ArrivalIntervalEvidenceMismatch(
                "fast-high-energy".into()
            ))
        );
    }

    #[test]
    fn every_voyage_alternative_must_use_the_frozen_corridor_path() {
        let mut case = valid_case();
        case.voyage_alternatives[1].route_segment_ids.reverse();
        assert_eq!(
            validate_case(&case),
            Err(ValidationError::RouteAlternativeMismatch(
                "slow-low-energy".into()
            ))
        );
    }

    #[path = "exact_oracle.rs"]
    mod exact_oracle;
}
