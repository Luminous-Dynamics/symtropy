//! Evidence-gated admission to exclusive construction resources.
//!
//! This module coordinates ownership of workfaces, lifts, staging areas,
//! access corridors and temporary works over half-open site-time intervals.
//! It does NOT perform geometry collision checking. A caller supplies a
//! context-bound receipt from the existing site-time occupancy evaluator;
//! this book additionally enforces resource exclusivity and stale-snapshot
//! rejection. Neither a clear occupancy receipt nor a reservation authorizes
//! robot actuation or replaces machinery/site safety controls.
//!
//! Time is represented by an explicit site epoch, timebase digest and integer
//! ticks. Wall-clock timestamps are not ordering authority. Reservations are
//! never implicitly expired: release/revocation is an explicit evidence event.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourceActorRole {
    RobotOperator,
    CrewLead,
    LiftOperator,
    MaterialCustodian,
    SiteCoordinator,
    SafetyAuthority,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResourceActorRegistry {
    roles_by_actor: BTreeMap<String, BTreeSet<ResourceActorRole>>,
}

impl ResourceActorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn grant(
        &mut self,
        actor_id: impl Into<String>,
        role: ResourceActorRole,
    ) -> Result<(), ReservationError> {
        let actor_id = actor_id.into();
        if actor_id.trim().is_empty() {
            return Err(ReservationError::EmptyActorIdentity);
        }
        self.roles_by_actor.entry(actor_id).or_default().insert(role);
        Ok(())
    }

    pub fn allows(&self, actor_id: &str, role: ResourceActorRole) -> bool {
        self.roles_by_actor
            .get(actor_id)
            .is_some_and(|roles| roles.contains(&role))
    }

    fn is_override_authority(&self, actor_id: &str) -> bool {
        self.allows(actor_id, ResourceActorRole::SiteCoordinator)
            || self.allows(actor_id, ResourceActorRole::SafetyAuthority)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstructionResourceKind {
    Workface,
    Lift,
    MaterialStagingArea,
    AccessCorridor,
    TemporaryWorks,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourcePolicy {
    /// At most one active claimant may own the resource during any overlapping
    /// site-time interval. v0.1 deliberately does not infer safe sharing.
    Exclusive,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceDefinition {
    pub resource_id: String,
    pub kind: ConstructionResourceKind,
    pub resource_configuration_sha256: String,
    pub resource_geometry_sha256: String,
    pub site_epoch_id: String,
    pub timebase_sha256: String,
    pub frame_sha256: String,
    pub transform_sha256: String,
    pub policy: ResourcePolicy,
    pub allowed_actor_roles: BTreeSet<ResourceActorRole>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceCatalog {
    resources: BTreeMap<String, ResourceDefinition>,
}

impl ResourceCatalog {
    pub fn new(
        definitions: impl IntoIterator<Item = ResourceDefinition>,
    ) -> Result<Self, ReservationError> {
        let mut resources = BTreeMap::new();
        for definition in definitions {
            validate_resource_definition(&definition)?;
            let resource_id = definition.resource_id.clone();
            if resources.insert(resource_id.clone(), definition).is_some() {
                return Err(ReservationError::DuplicateResource(resource_id));
            }
        }
        Ok(Self { resources })
    }

    pub fn get(&self, resource_id: &str) -> Option<&ResourceDefinition> {
        self.resources.get(resource_id)
    }

    pub fn len(&self) -> usize {
        self.resources.len()
    }

    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SiteTimeInterval {
    pub start_tick: u64,
    pub end_tick: u64,
}

impl SiteTimeInterval {
    pub fn is_valid(self) -> bool {
        self.start_tick < self.end_tick
    }

    /// Half-open interval overlap: [a.start, a.end) intersects [b.start, b.end).
    /// Exactly adjacent reservations do not overlap.
    pub fn overlaps(self, other: Self) -> bool {
        self.start_tick < other.end_tick && other.start_tick < self.end_tick
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpaceTimeClearanceDisposition {
    /// The upstream evaluator completed its declared check and found no
    /// occupancy conflict for this candidate and site snapshot.
    ClearFromSnapshot,
    Conflict,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpaceTimeClearanceReceipt {
    pub receipt_sha256: String,
    pub evaluator_build_sha256: String,
    pub occupancy_snapshot_sha256: String,
    pub work_package_sha256: String,
    pub candidate_occupancy_sha256: String,
    pub resource_id: String,
    pub resource_geometry_sha256: String,
    pub site_epoch_id: String,
    pub timebase_sha256: String,
    pub interval: SiteTimeInterval,
    pub frame_sha256: String,
    pub transform_sha256: String,
    pub disposition: SpaceTimeClearanceDisposition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceReservationRequest {
    pub reservation_id: String,
    pub resource_id: String,
    pub owner_actor_id: String,
    pub owner_role: ResourceActorRole,
    pub work_package_sha256: String,
    pub candidate_occupancy_sha256: String,
    pub site_epoch_id: String,
    pub timebase_sha256: String,
    pub interval: SiteTimeInterval,
    pub frame_sha256: String,
    pub transform_sha256: String,
    pub resource_geometry_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReservationHoldReason {
    OccupancyConflict,
    OccupancyUnresolved,
    StaleOccupancySnapshot,
    ResourceContextChanged,
    IncompatibleTimeDomain,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReservationAdmission {
    Reserved,
    ExactDuplicate,
    Held {
        reason: ReservationHoldReason,
        conflicting_reservation_id: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReservationTerminalState {
    Released,
    Revoked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReservationClosure {
    pub terminal_state: ReservationTerminalState,
    pub actor_id: String,
    pub reason: String,
    pub receipt_sha256: String,
    pub event_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceReservationRecord {
    pub request: ResourceReservationRequest,
    pub clearance_receipt: SpaceTimeClearanceReceipt,
    pub closure: Option<ReservationClosure>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReservationError {
    EmptyActorIdentity,
    ActorRoleDenied,
    EmptyResourceIdentity,
    DuplicateResource(String),
    InvalidResourceDigest,
    EmptySiteEpoch,
    EmptyAllowedRoleSet(String),
    InvalidSiteTimeInterval,
    EmptyReservationIdentity,
    InvalidReservationDigest,
    UnknownResource(String),
    ResourceRoleDenied(String),
    ResourceContextDigestMismatch(String),
    InvalidClearanceReceipt,
    ClearanceContextMismatch,
    EmptyOccupancySnapshotIdentity,
    ConflictingReservationIdentity(String),
    ReservationIndexCorrupt,
    UnknownReservation(String),
    ReservationAlreadyClosed(String),
    ReleaseActorDenied(String),
    ClosureEventIdentityConflict(String),
    InvalidClosureReceipt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceReservationBook {
    catalog: ResourceCatalog,
    actor_registry: ResourceActorRegistry,
    reservations: Vec<ResourceReservationRecord>,
    index_by_reservation_id: BTreeMap<String, usize>,
    closure_event_ids: BTreeSet<String>,
}

impl ResourceReservationBook {
    pub fn new(catalog: ResourceCatalog, actor_registry: ResourceActorRegistry) -> Self {
        Self {
            catalog,
            actor_registry,
            reservations: Vec::new(),
            index_by_reservation_id: BTreeMap::new(),
            closure_event_ids: BTreeSet::new(),
        }
    }

    pub fn catalog(&self) -> &ResourceCatalog {
        &self.catalog
    }

    pub fn reservations(&self) -> &[ResourceReservationRecord] {
        &self.reservations
    }

    pub fn active_reservations(&self) -> impl Iterator<Item = &ResourceReservationRecord> {
        self.reservations
            .iter()
            .filter(|reservation| reservation.closure.is_none())
    }

    pub fn reserve(
        &mut self,
        request: ResourceReservationRequest,
        clearance: SpaceTimeClearanceReceipt,
        current_occupancy_snapshot_sha256: &str,
    ) -> Result<ReservationAdmission, ReservationError> {
        validate_request(&request)?;
        validate_clearance_receipt(&clearance)?;

        if !self
            .actor_registry
            .allows(&request.owner_actor_id, request.owner_role)
        {
            return Err(ReservationError::ActorRoleDenied);
        }

        if let Some(index) = self.index_by_reservation_id.get(&request.reservation_id) {
            let existing = self
                .reservations
                .get(*index)
                .ok_or(ReservationError::ReservationIndexCorrupt)?;
            return if existing.request == request && existing.clearance_receipt == clearance {
                if existing.closure.is_none() {
                    Ok(ReservationAdmission::ExactDuplicate)
                } else {
                    Err(ReservationError::ReservationAlreadyClosed(
                        request.reservation_id,
                    ))
                }
            } else {
                Err(ReservationError::ConflictingReservationIdentity(
                    request.reservation_id,
                ))
            };
        }

        let resource = self
            .catalog
            .get(&request.resource_id)
            .ok_or_else(|| ReservationError::UnknownResource(request.resource_id.clone()))?;
        if !resource
            .allowed_actor_roles
            .contains(&request.owner_role)
        {
            return Err(ReservationError::ResourceRoleDenied(
                request.resource_id.clone(),
            ));
        }
        if !resource_context_matches(resource, &request) {
            return Ok(ReservationAdmission::Held {
                reason: if resource.site_epoch_id != request.site_epoch_id
                    || resource.timebase_sha256 != request.timebase_sha256
                {
                    ReservationHoldReason::IncompatibleTimeDomain
                } else {
                    ReservationHoldReason::ResourceContextChanged
                },
                conflicting_reservation_id: None,
            });
        }
        if !clearance_matches_request(&clearance, &request) {
            return Err(ReservationError::ClearanceContextMismatch);
        }

        if current_occupancy_snapshot_sha256.trim().is_empty()
            || !is_sha256(current_occupancy_snapshot_sha256)
        {
            return Err(ReservationError::EmptyOccupancySnapshotIdentity);
        }
        if clearance.occupancy_snapshot_sha256 != current_occupancy_snapshot_sha256 {
            return Ok(ReservationAdmission::Held {
                reason: ReservationHoldReason::StaleOccupancySnapshot,
                conflicting_reservation_id: None,
            });
        }

        match clearance.disposition {
            SpaceTimeClearanceDisposition::Conflict => {
                return Ok(ReservationAdmission::Held {
                    reason: ReservationHoldReason::OccupancyConflict,
                    conflicting_reservation_id: None,
                });
            }
            SpaceTimeClearanceDisposition::Unresolved => {
                return Ok(ReservationAdmission::Held {
                    reason: ReservationHoldReason::OccupancyUnresolved,
                    conflicting_reservation_id: None,
                });
            }
            SpaceTimeClearanceDisposition::ClearFromSnapshot => {}
        }

        for existing in self.active_reservations() {
            if existing.request.resource_id != request.resource_id {
                continue;
            }
            if existing.request.site_epoch_id != request.site_epoch_id
                || existing.request.timebase_sha256 != request.timebase_sha256
            {
                return Ok(ReservationAdmission::Held {
                    reason: ReservationHoldReason::IncompatibleTimeDomain,
                    conflicting_reservation_id: Some(existing.request.reservation_id.clone()),
                });
            }
            if existing.request.interval.overlaps(request.interval) {
                return Ok(ReservationAdmission::Held {
                    reason: ReservationHoldReason::OccupancyConflict,
                    conflicting_reservation_id: Some(existing.request.reservation_id.clone()),
                });
            }
        }

        let index = self.reservations.len();
        self.index_by_reservation_id
            .insert(request.reservation_id.clone(), index);
        self.reservations.push(ResourceReservationRecord {
            request,
            clearance_receipt: clearance,
            closure: None,
        });
        Ok(ReservationAdmission::Reserved)
    }

    /// Explicitly close a reservation. The owner may release their own claim;
    /// a trusted coordinator or safety authority may also release it. No
    /// reservation expires merely because a wall-clock deadline has elapsed.
    pub fn release(
        &mut self,
        reservation_id: &str,
        actor_id: &str,
        actor_role: ResourceActorRole,
        reason: &str,
        event_id: &str,
        receipt_sha256: &str,
    ) -> Result<(), ReservationError> {
        self.close(
            reservation_id,
            actor_id,
            actor_role,
            reason,
            event_id,
            receipt_sha256,
            ReservationTerminalState::Released,
        )
    }

    /// Revoke an active reservation. Only a site coordinator or safety
    /// authority can revoke another actor's reservation.
    pub fn revoke(
        &mut self,
        reservation_id: &str,
        actor_id: &str,
        actor_role: ResourceActorRole,
        reason: &str,
        event_id: &str,
        receipt_sha256: &str,
    ) -> Result<(), ReservationError> {
        if !matches!(
            actor_role,
            ResourceActorRole::SiteCoordinator | ResourceActorRole::SafetyAuthority
        ) || !self.actor_registry.allows(actor_id, actor_role)
        {
            return Err(ReservationError::ReleaseActorDenied(
                reservation_id.to_owned(),
            ));
        }
        self.close(
            reservation_id,
            actor_id,
            actor_role,
            reason,
            event_id,
            receipt_sha256,
            ReservationTerminalState::Revoked,
        )
    }

    fn close(
        &mut self,
        reservation_id: &str,
        actor_id: &str,
        actor_role: ResourceActorRole,
        reason: &str,
        event_id: &str,
        receipt_sha256: &str,
        terminal_state: ReservationTerminalState,
    ) -> Result<(), ReservationError> {
        if !self.actor_registry.allows(actor_id, actor_role) {
            return Err(ReservationError::ActorRoleDenied);
        }
        if reason.trim().is_empty()
            || event_id.trim().is_empty()
            || !is_sha256(receipt_sha256)
        {
            return Err(ReservationError::InvalidClosureReceipt);
        }
        if self.closure_event_ids.contains(event_id) {
            return Err(ReservationError::ClosureEventIdentityConflict(
                event_id.to_owned(),
            ));
        }
        let index = *self
            .index_by_reservation_id
            .get(reservation_id)
            .ok_or_else(|| ReservationError::UnknownReservation(reservation_id.to_owned()))?;
        let record = self
            .reservations
            .get_mut(index)
            .ok_or(ReservationError::ReservationIndexCorrupt)?;
        if record.closure.is_some() {
            return Err(ReservationError::ReservationAlreadyClosed(
                reservation_id.to_owned(),
            ));
        }
        let owner_release = record.request.owner_actor_id == actor_id;
        let authority_release = self.actor_registry.is_override_authority(actor_id);
        if !owner_release && !authority_release {
            return Err(ReservationError::ReleaseActorDenied(
                reservation_id.to_owned(),
            ));
        }
        record.closure = Some(ReservationClosure {
            terminal_state,
            actor_id: actor_id.to_owned(),
            reason: reason.to_owned(),
            receipt_sha256: receipt_sha256.to_owned(),
            event_id: event_id.to_owned(),
        });
        self.closure_event_ids.insert(event_id.to_owned());
        Ok(())
    }
}

fn validate_resource_definition(resource: &ResourceDefinition) -> Result<(), ReservationError> {
    if resource.resource_id.trim().is_empty() || resource.site_epoch_id.trim().is_empty() {
        return Err(ReservationError::EmptyResourceIdentity);
    }
    if [
        resource.resource_configuration_sha256.as_str(),
        resource.resource_geometry_sha256.as_str(),
        resource.timebase_sha256.as_str(),
        resource.frame_sha256.as_str(),
        resource.transform_sha256.as_str(),
    ]
    .into_iter()
    .any(|digest| !is_sha256(digest))
    {
        return Err(ReservationError::InvalidResourceDigest);
    }
    if resource.allowed_actor_roles.is_empty() {
        return Err(ReservationError::EmptyAllowedRoleSet(
            resource.resource_id.clone(),
        ));
    }
    Ok(())
}

fn validate_request(request: &ResourceReservationRequest) -> Result<(), ReservationError> {
    if request.reservation_id.trim().is_empty() {
        return Err(ReservationError::EmptyReservationIdentity);
    }
    if request.resource_id.trim().is_empty()
        || request.owner_actor_id.trim().is_empty()
        || request.site_epoch_id.trim().is_empty()
    {
        return Err(ReservationError::EmptyResourceIdentity);
    }
    if !request.interval.is_valid() {
        return Err(ReservationError::InvalidSiteTimeInterval);
    }
    if [
        request.work_package_sha256.as_str(),
        request.candidate_occupancy_sha256.as_str(),
        request.timebase_sha256.as_str(),
        request.frame_sha256.as_str(),
        request.transform_sha256.as_str(),
        request.resource_geometry_sha256.as_str(),
    ]
    .into_iter()
    .any(|digest| !is_sha256(digest))
    {
        return Err(ReservationError::InvalidReservationDigest);
    }
    Ok(())
}

fn validate_clearance_receipt(
    receipt: &SpaceTimeClearanceReceipt,
) -> Result<(), ReservationError> {
    if [
        receipt.receipt_sha256.as_str(),
        receipt.evaluator_build_sha256.as_str(),
        receipt.occupancy_snapshot_sha256.as_str(),
        receipt.work_package_sha256.as_str(),
        receipt.candidate_occupancy_sha256.as_str(),
        receipt.resource_geometry_sha256.as_str(),
        receipt.timebase_sha256.as_str(),
        receipt.frame_sha256.as_str(),
        receipt.transform_sha256.as_str(),
    ]
    .into_iter()
    .any(|digest| !is_sha256(digest))
        || receipt.resource_id.trim().is_empty()
        || receipt.site_epoch_id.trim().is_empty()
        || !receipt.interval.is_valid()
    {
        return Err(ReservationError::InvalidClearanceReceipt);
    }
    Ok(())
}

fn clearance_matches_request(
    receipt: &SpaceTimeClearanceReceipt,
    request: &ResourceReservationRequest,
) -> bool {
    receipt.work_package_sha256 == request.work_package_sha256
        && receipt.candidate_occupancy_sha256 == request.candidate_occupancy_sha256
        && receipt.resource_id == request.resource_id
        && receipt.resource_geometry_sha256 == request.resource_geometry_sha256
        && receipt.site_epoch_id == request.site_epoch_id
        && receipt.timebase_sha256 == request.timebase_sha256
        && receipt.interval == request.interval
        && receipt.frame_sha256 == request.frame_sha256
        && receipt.transform_sha256 == request.transform_sha256
}

fn resource_context_matches(
    resource: &ResourceDefinition,
    request: &ResourceReservationRequest,
) -> bool {
    resource.site_epoch_id == request.site_epoch_id
        && resource.timebase_sha256 == request.timebase_sha256
        && resource.frame_sha256 == request.frame_sha256
        && resource.transform_sha256 == request.transform_sha256
        && resource.resource_geometry_sha256 == request.resource_geometry_sha256
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

    fn actor_registry() -> ResourceActorRegistry {
        let mut registry = ResourceActorRegistry::new();
        registry.grant("robot-1", ResourceActorRole::RobotOperator).unwrap();
        registry.grant("robot-2", ResourceActorRole::RobotOperator).unwrap();
        registry.grant("crew-1", ResourceActorRole::CrewLead).unwrap();
        registry.grant("coordinator", ResourceActorRole::SiteCoordinator).unwrap();
        registry.grant("safety", ResourceActorRole::SafetyAuthority).unwrap();
        registry
    }

    fn catalog() -> ResourceCatalog {
        let mut roles = BTreeSet::new();
        roles.insert(ResourceActorRole::RobotOperator);
        roles.insert(ResourceActorRole::CrewLead);
        ResourceCatalog::new([ResourceDefinition {
            resource_id: "workface-level-02-east".into(),
            kind: ConstructionResourceKind::Workface,
            resource_configuration_sha256: digest('1'),
            resource_geometry_sha256: digest('2'),
            site_epoch_id: "site-epoch-02".into(),
            timebase_sha256: digest('3'),
            frame_sha256: digest('4'),
            transform_sha256: digest('5'),
            policy: ResourcePolicy::Exclusive,
            allowed_actor_roles: roles,
        }])
        .unwrap()
    }

    fn request(
        reservation_id: &str,
        actor_id: &str,
        start_tick: u64,
        end_tick: u64,
    ) -> ResourceReservationRequest {
        ResourceReservationRequest {
            reservation_id: reservation_id.into(),
            resource_id: "workface-level-02-east".into(),
            owner_actor_id: actor_id.into(),
            owner_role: if actor_id == "crew-1" {
                ResourceActorRole::CrewLead
            } else {
                ResourceActorRole::RobotOperator
            },
            work_package_sha256: digest('6'),
            candidate_occupancy_sha256: digest('7'),
            site_epoch_id: "site-epoch-02".into(),
            timebase_sha256: digest('3'),
            interval: SiteTimeInterval {
                start_tick,
                end_tick,
            },
            frame_sha256: digest('4'),
            transform_sha256: digest('5'),
            resource_geometry_sha256: digest('2'),
        }
    }

    fn clearance(
        request: &ResourceReservationRequest,
        snapshot: &str,
        disposition: SpaceTimeClearanceDisposition,
    ) -> SpaceTimeClearanceReceipt {
        SpaceTimeClearanceReceipt {
            receipt_sha256: digest('8'),
            evaluator_build_sha256: digest('9'),
            occupancy_snapshot_sha256: snapshot.into(),
            work_package_sha256: request.work_package_sha256.clone(),
            candidate_occupancy_sha256: request.candidate_occupancy_sha256.clone(),
            resource_id: request.resource_id.clone(),
            resource_geometry_sha256: request.resource_geometry_sha256.clone(),
            site_epoch_id: request.site_epoch_id.clone(),
            timebase_sha256: request.timebase_sha256.clone(),
            interval: request.interval,
            frame_sha256: request.frame_sha256.clone(),
            transform_sha256: request.transform_sha256.clone(),
            disposition,
        }
    }

    fn book() -> ResourceReservationBook {
        ResourceReservationBook::new(catalog(), actor_registry())
    }

    #[test]
    fn clear_occupancy_and_free_resource_are_both_required_for_admission() {
        let mut book = book();
        let request = request("reservation-1", "robot-1", 100, 200);
        assert_eq!(
            book.reserve(
                request.clone(),
                clearance(
                    &request,
                    &digest('a'),
                    SpaceTimeClearanceDisposition::ClearFromSnapshot
                ),
                &digest('a'),
            ),
            Ok(ReservationAdmission::Reserved)
        );
        assert_eq!(book.active_reservations().count(), 1);
    }

    #[test]
    fn geometry_conflict_or_unresolved_evaluation_never_creates_a_reservation() {
        for disposition in [
            SpaceTimeClearanceDisposition::Conflict,
            SpaceTimeClearanceDisposition::Unresolved,
        ] {
            let mut book = book();
            let request = request("reservation-1", "robot-1", 100, 200);
            let result = book
                .reserve(
                    request,
                    clearance(&request, &digest('a'), disposition),
                    &digest('a'),
                )
                .unwrap();
            assert!(matches!(result, ReservationAdmission::Held { .. }));
            assert_eq!(book.active_reservations().count(), 0);
        }
    }

    #[test]
    fn stale_occupancy_snapshot_is_held_without_mutating_the_book() {
        let mut book = book();
        let request = request("reservation-1", "robot-1", 100, 200);
        let result = book
            .reserve(
                request.clone(),
                clearance(
                    &request,
                    &digest('a'),
                    SpaceTimeClearanceDisposition::ClearFromSnapshot,
                ),
                &digest('b'),
            )
            .unwrap();
        assert_eq!(
            result,
            ReservationAdmission::Held {
                reason: ReservationHoldReason::StaleOccupancySnapshot,
                conflicting_reservation_id: None
            }
        );
        assert!(book.reservations().is_empty());
    }

    #[test]
    fn exclusive_resource_rejects_overlap_but_allows_half_open_adjacency() {
        let mut book = book();
        let first = request("reservation-1", "robot-1", 100, 200);
        assert_eq!(
            book.reserve(
                first.clone(),
                clearance(
                    &first,
                    &digest('a'),
                    SpaceTimeClearanceDisposition::ClearFromSnapshot
                ),
                &digest('a'),
            ),
            Ok(ReservationAdmission::Reserved)
        );

        let overlapping = request("reservation-2", "robot-2", 199, 300);
        assert!(matches!(
            book.reserve(
                overlapping.clone(),
                clearance(
                    &overlapping,
                    &digest('b'),
                    SpaceTimeClearanceDisposition::ClearFromSnapshot
                ),
                &digest('b'),
            ),
            Ok(ReservationAdmission::Held {
                reason: ReservationHoldReason::OccupancyConflict,
                conflicting_reservation_id: Some(id)
            }) if id == "reservation-1"
        ));

        let adjacent = request("reservation-3", "robot-2", 200, 300);
        assert_eq!(
            book.reserve(
                adjacent.clone(),
                clearance(
                    &adjacent,
                    &digest('b'),
                    SpaceTimeClearanceDisposition::ClearFromSnapshot
                ),
                &digest('b'),
            ),
            Ok(ReservationAdmission::Reserved)
        );
    }

    #[test]
    fn incompatible_site_epoch_is_held_not_treated_as_disjoint() {
        let mut book = book();
        let mut first = request("reservation-1", "robot-1", 100, 200);
        first.site_epoch_id = "site-epoch-old".into();
        let mut book2 = ResourceReservationBook::new(
            ResourceCatalog::new([ResourceDefinition {
                site_epoch_id: "site-epoch-old".into(),
                ..catalog().get("workface-level-02-east").unwrap().clone()
            }])
            .unwrap(),
            actor_registry(),
        );
        assert_eq!(
            book2.reserve(
                first.clone(),
                clearance(
                    &first,
                    &digest('a'),
                    SpaceTimeClearanceDisposition::ClearFromSnapshot
                ),
                &digest('a'),
            ),
            Ok(ReservationAdmission::Reserved)
        );
        let next_epoch = request("reservation-2", "robot-2", 150, 250);
        assert!(matches!(
            book2.reserve(
                next_epoch.clone(),
                clearance(
                    &next_epoch,
                    &digest('b'),
                    SpaceTimeClearanceDisposition::ClearFromSnapshot
                ),
                &digest('b'),
            ),
            Ok(ReservationAdmission::Held {
                reason: ReservationHoldReason::ResourceContextChanged
                    | ReservationHoldReason::IncompatibleTimeDomain,
                ..
            })
        ));
        // The original catalog book remains empty; reservations are per book.
        assert_eq!(book.active_reservations().count(), 0);
    }

    #[test]
    fn same_owner_exact_duplicate_is_idempotent_and_changed_identity_is_rejected() {
        let mut book = book();
        let request = request("reservation-1", "robot-1", 100, 200);
        let receipt = clearance(
            &request,
            &digest('a'),
            SpaceTimeClearanceDisposition::ClearFromSnapshot,
        );
        assert_eq!(
            book.reserve(request.clone(), receipt.clone(), &digest('a')),
            Ok(ReservationAdmission::Reserved)
        );
        assert_eq!(
            book.reserve(request.clone(), receipt, &digest('a')),
            Ok(ReservationAdmission::ExactDuplicate)
        );
        let mut changed = request;
        changed.interval.end_tick = 250;
        assert_eq!(
            book.reserve(
                changed.clone(),
                clearance(
                    &changed,
                    &digest('a'),
                    SpaceTimeClearanceDisposition::ClearFromSnapshot
                ),
                &digest('a')
            ),
            Err(ReservationError::ConflictingReservationIdentity(
                "reservation-1".into()
            ))
        );
        assert_eq!(book.reservations().len(), 1);
    }

    #[test]
    fn only_owner_or_trusted_override_can_release_and_release_is_explicit() {
        let mut book = book();
        let first = request("reservation-1", "robot-1", 100, 200);
        book.reserve(
            first.clone(),
            clearance(
                &first,
                &digest('a'),
                SpaceTimeClearanceDisposition::ClearFromSnapshot,
            ),
            &digest('a'),
        )
        .unwrap();

        assert_eq!(
            book.release(
                "reservation-1",
                "robot-2",
                ResourceActorRole::RobotOperator,
                "unrelated actor",
                "release-event-1",
                &digest('b'),
            ),
            Err(ReservationError::ReleaseActorDenied(
                "reservation-1".into()
            ))
        );
        assert!(book.active_reservations().next().is_some());

        book.release(
            "reservation-1",
            "robot-1",
            ResourceActorRole::RobotOperator,
            "task independently verified complete",
            "release-event-2",
            &digest('b'),
        )
        .unwrap();
        assert_eq!(book.active_reservations().count(), 0);

        let next = request("reservation-2", "robot-2", 100, 200);
        assert_eq!(
            book.reserve(
                next.clone(),
                clearance(
                    &next,
                    &digest('c'),
                    SpaceTimeClearanceDisposition::ClearFromSnapshot
                ),
                &digest('c')
            ),
            Ok(ReservationAdmission::Reserved)
        );
    }

    #[test]
    fn coordinator_can_revoke_but_no_reservation_implicitly_expires() {
        let mut book = book();
        let request = request("reservation-1", "robot-1", 100, 200);
        book.reserve(
            request.clone(),
            clearance(
                &request,
                &digest('a'),
                SpaceTimeClearanceDisposition::ClearFromSnapshot,
            ),
            &digest('a'),
        )
        .unwrap();
        // Advancing a logical clock does not mutate this book.
        assert_eq!(book.active_reservations().count(), 1);
        book.revoke(
            "reservation-1",
            "coordinator",
            ResourceActorRole::SiteCoordinator,
            "workface configuration changed",
            "revoke-event-1",
            &digest('c'),
        )
        .unwrap();
        assert_eq!(
            book.reservations()[0]
                .closure
                .as_ref()
                .unwrap()
                .terminal_state,
            ReservationTerminalState::Revoked
        );
    }

    #[test]
    fn unregistered_actor_and_resource_role_mismatch_are_rejected() {
        let mut book = book();
        let mut request = request("reservation-1", "unregistered", 100, 200);
        request.owner_role = ResourceActorRole::RobotOperator;
        assert_eq!(
            book.reserve(
                request.clone(),
                clearance(
                    &request,
                    &digest('a'),
                    SpaceTimeClearanceDisposition::ClearFromSnapshot
                ),
                &digest('a'),
            ),
            Err(ReservationError::ActorRoleDenied)
        );

        let mut request = request("reservation-2", "crew-1", 100, 200);
        request.owner_role = ResourceActorRole::CrewLead;
        assert_eq!(
            book.reserve(
                request.clone(),
                clearance(
                    &request,
                    &digest('a'),
                    SpaceTimeClearanceDisposition::ClearFromSnapshot
                ),
                &digest('a'),
            ),
            Ok(ReservationAdmission::Reserved)
        );
    }
}
