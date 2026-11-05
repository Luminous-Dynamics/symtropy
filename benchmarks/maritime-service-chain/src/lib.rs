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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Resource-capacity evidence must bind the exact intervals and capacities.
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EnergyLedger {
    /// Ship propulsion boundary, expressed in watt-hours.
    pub propulsion_wh: u64,
    /// Ship auxiliary / hotel-load boundary, expressed in watt-hours.
    pub auxiliary_wh: u64,
    /// Shore-power supply boundary, expressed in watt-hours.
    pub shore_power_wh: u64,
    /// Onward-transfer energy boundary, expressed in watt-hours.
    pub onward_transfer_wh: u64,
}

impl EnergyLedger {
    pub fn total_wh(self) -> u128 {
        u128::from(self.propulsion_wh)
            + u128::from(self.auxiliary_wh)
            + u128::from(self.shore_power_wh)
            + u128::from(self.onward_transfer_wh)
    }
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
    pub energy: EnergyLedger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub tick_seconds: u64,
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
    ResourceMissingEvidence(String),
    ResourceMissingCapacityEvidence(String),
    ResourceCapacityEvidenceMismatch(String),
    MissingRequiredSegmentResource(String),
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
    EvidenceExpired(String),
    EvidenceRoleMismatch(String),
    EvidenceOwnerMismatch(String),
    EvidenceSubjectMismatch(String),
    ArrivalIntervalEvidenceMismatch(String),
    InvalidVoyageAlternative(String),
    RouteAlternativeMismatch(String),
    DemandRouteMismatch,
    DemandResourceUnitMismatch(String),
    DemandResourceShortfall(String),
    HardConstraintUnknown(String),
    HardConstraintBlocked(String),
    MissingConstraintEvidence(String),
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
        | EvidenceRole::ExternalMeasurement => None,
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
    }
}

fn resource_kind_allowed_for_segment(segment: SegmentKind, resource: ResourceKind) -> bool {
    match segment {
        SegmentKind::ChannelTransit | SegmentKind::PortApproach => {
            resource == ResourceKind::ChannelWindow
        }
        SegmentKind::Lockage => resource == ResourceKind::Lock,
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
        SegmentKind::Lockage => kinds.contains(&ResourceKind::Lock),
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
        if evidence.covered_capacity_windows.as_ref().is_some_and(|windows| {
            windows
                .iter()
                .any(|item| !item.window.is_valid() || item.capacity == 0)
        }) {
            return Err(ValidationError::InvalidWindow(
                "evidence.covered_capacity_windows",
            ));
        }
    }
    Ok(())
}

fn validate_route(case: &ServiceChainCase) -> Result<(), ValidationError> {
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
                if evidence.covered_capacity_windows.as_deref()
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
        for evidence_id in &segment.evidence_ids {
            validate_evidence_use(case, evidence_id)?;
        }
        let mut used_kinds = BTreeSet::new();
        for usage in &segment.resource_uses {
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
            covered_capacity_windows: None,
            state: EvidenceState::Current,
            claim_ceiling: "synthetic benchmark fixture only".into(),
        }
    }

    fn capacity_evidence(
        id: &str,
        subject: &str,
        availability: Vec<CapacityWindow>,
    ) -> EvidenceRef {
        let mut evidence = evidence(
            id,
            EvidenceOwner::Symtropy,
            EvidenceRole::ResourceCapacity,
            subject,
            None,
        );
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
            start_tick: 14,
            end_tick: 17,
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
                vec![CapacityWindow {
                    window: TickWindow {
                        start_tick: 15,
                        end_tick: 35,
                    },
                    capacity: 1,
                }],
            ),
            capacity_evidence(
                "berth-calendar",
                "berth-01",
                vec![CapacityWindow {
                    window: TickWindow {
                        start_tick: 25,
                        end_tick: 50,
                    },
                    capacity: 1,
                }],
            ),
            capacity_evidence(
                "shore-calendar",
                "shore-01",
                vec![CapacityWindow {
                    window: TickWindow {
                        start_tick: 25,
                        end_tick: 40,
                    },
                    capacity: 1_000,
                }],
            ),
            capacity_evidence(
                "rail-calendar",
                "rail-01",
                vec![CapacityWindow {
                    window: TickWindow {
                        start_tick: 40,
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
                vec![CapacityWindow {
                    window: TickWindow {
                        start_tick: 25,
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
                        start_tick: 15,
                        end_tick: 35,
                    },
                    capacity: 1,
                }],
                evidence_ids: vec!["lock-water".into(), "lock-calendar".into()],
            },
            Resource {
                resource_id: "berth-01".into(),
                kind: ResourceKind::Berth,
                capacity_unit: ResourceUnit::BerthSlots,
                availability: vec![CapacityWindow {
                    window: TickWindow {
                        start_tick: 25,
                        end_tick: 50,
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
                        start_tick: 25,
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
                        start_tick: 25,
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
                        start_tick: 40,
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
                evidence_ids: vec!["topology".into(), "depth".into()],
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
                evidence_ids: vec!["lock-compatibility".into(), "lock-water".into()],
                resource_uses: vec![ResourceUse {
                    resource_id: "lock-01".into(),
                    amount: 1,
                    unit: ResourceUnit::Lockages,
                }],
            },
            RouteSegment {
                segment_id: "approach-01".into(),
                from_node_id: "lock-exit".into(),
                to_node_id: "berth".into(),
                kind: SegmentKind::PortApproach,
                evidence_ids: vec!["depth".into()],
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
                evidence_ids: vec!["berth-calendar".into(), "shore-calendar".into()],
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
                energy: EnergyLedger {
                    propulsion_wh: 1_800,
                    auxiliary_wh: 200,
                    shore_power_wh: 0,
                    onward_transfer_wh: 0,
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
                energy: EnergyLedger {
                    propulsion_wh: 1_300,
                    auxiliary_wh: 200,
                    shore_power_wh: 0,
                    onward_transfer_wh: 0,
                },
            },
        ];

        ServiceChainCase {
            schema_version: "0.1.0".into(),
            case_id: "synthetic-lock-to-rail-001".into(),
            name: "Synthetic vessel-lock-berth-to-rail handoff".into(),
            class: CaseClass::Synthetic,
            decision_tick: 0,
            tick_seconds: 60,
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
    fn demand_endpoints_must_match_the_frozen_route() {
        let mut case = valid_case();
        case.demand.destination_node_id = "berth".into();
        assert_eq!(validate_case(&case), Err(ValidationError::DemandRouteMismatch));
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
        mismatched.route_segments[4].resource_uses[0].unit = ResourceUnit::Teu;
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
}
