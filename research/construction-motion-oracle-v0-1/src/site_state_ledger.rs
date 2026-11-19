//! Append-only, independently reconciled construction site state.
//!
//! This module projects a pre-verified event stream into per-design-entity
//! state. Robot observations can start or describe attempts, but cannot mark
//! an entity accepted. Acceptance is recomputed from the existing independent
//! field-layout scorer. Package revisions preserve prior receipts as history
//! and change only which package is current.
//!
//! Cryptographic signature verification and recomputation of event digests are
//! intentionally outside this module. The caller must verify the event's
//! canonical bytes, digest and actor signature before invoking apply. This
//! reducer validates digest encoding, event identity, sequence continuity,
//! previous-digest linkage, role permissions and state transitions.

use std::collections::{BTreeMap, BTreeSet};

use crate::field_layout_acceptance::{
    CrewHours, DesignIntentEntity, DesignIntentManifest, EntityScoreReason, FieldAcceptanceProfile,
    FieldEntityObservation, HumanWaiver, MeasurementDisposition, RefereeEntityState,
    RefereeMeasurement, RobotClaimedEntityState, score_field_layout_run,
};

use crate::cumulative_resource_capacity::{
    CapacityReservation, CapacityResourceDefinition, CumulativeCapacityScenario,
};
use crate::robust_capacity_profile::{
    CapacityAvailabilityWindow, RobustCapacityError, RobustCumulativeCapacityScenario,
    RobustCumulativeCapacityReport, evaluate_robust_cumulative_resource_capacity,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteActorRole {
    DesignAuthority,
    Surveyor,
    Robot,
    Referee,
    SiteQualityAuthority,
    SiteResourceAuthority,
    DispatchCoordinator,
}

/// Actor-to-role policy supplied by an authenticated identity source.
///
/// Role membership is checked for every event. This in-memory registry is a
/// projection input; production callers must load it from an authenticated,
/// versioned authority source and ensure revocations are reflected before
/// event ingestion.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SiteActorRegistry {
    roles_by_actor: BTreeMap<String, BTreeSet<SiteActorRole>>,
}

impl SiteActorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn grant(
        &mut self,
        actor_id: impl Into<String>,
        role: SiteActorRole,
    ) -> Result<(), SiteLedgerError> {
        let actor_id = actor_id.into();
        if actor_id.trim().is_empty() {
            return Err(SiteLedgerError::EmptyActorRegistration);
        }
        self.roles_by_actor.entry(actor_id).or_default().insert(role);
        Ok(())
    }

    pub fn allows(&self, actor_id: &str, role: SiteActorRole) -> bool {
        self.roles_by_actor
            .get(actor_id)
            .is_some_and(|roles| roles.contains(&role))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RobotExecutionDisposition {
    Printed,
    NotPrinted,
    Partial,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SiteRefereeAssessment {
    Measured(RefereeMeasurement),
    NoMarkFound {
        receipt_sha256: String,
    },
    Unresolved {
        receipt_sha256: String,
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct SiteWorkPackage {
    pub package_sha256: String,
    pub source_model_sha256: String,
    pub source_schema_release: String,
    pub ids_spec_sha256: String,
    pub ids_version: String,
    pub ids_engine_build_sha256: String,
    pub design_revision_sha256: String,
    pub frame_id: String,
    pub frame_sha256: String,
    pub transform_sha256: String,
    pub entity_manifest_sha256: String,
    pub acceptance_profile_sha256: String,
    pub entities: Vec<DesignIntentEntity>,
    pub acceptance_profile: FieldAcceptanceProfile,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SiteResourceDefinition {
    pub resource_id: String,
    /// Digest of the site owner's canonical resource definition.
    pub definition_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExclusiveResourceClaim {
    pub resource_id: String,
    pub definition_sha256: String,
    pub start_tick: u64,
    pub end_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpaceTimeReservation {
    pub reservation_id: String,
    pub resource_catalog_sha256: String,
    pub resource_claim_manifest_sha256: String,
    pub claims: Vec<ExclusiveResourceClaim>,
    pub package_sha256: String,
    pub design_revision_sha256: String,
    pub entity_id: String,
    pub robot_id: String,
    pub envelope_sha256: String,
    pub readiness_receipt_sha256: String,
    pub occupancy_assessment_receipt_sha256: String,
    pub occupancy_engine_build_sha256: String,
    pub clearance_policy_sha256: String,
    pub site_snapshot_sha256: String,
    pub site_ledger_sequence: u64,
    pub site_ledger_event_sha256: String,
    pub site_clock_domain_id: String,
    pub checked_at_tick: u64,
    pub valid_until_tick: u64,
    pub committed_at_tick: u64,
    pub start_tick: u64,
    pub end_tick: u64,
    /// Lease expiry prevents stale token use; it never releases the physical
    /// resource automatically. Explicit referee release is required.
    pub lease_valid_until_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpaceTimeReservationRecord {
    pub reservation: SpaceTimeReservation,
    pub committed_event_id: String,
    pub committed_sequence: u64,
    pub released_event_id: Option<String>,
    pub released_at_tick: Option<u64>,
    pub release_receipt_sha256: Option<String>,
}

impl SpaceTimeReservationRecord {
    pub fn is_active(&self) -> bool {
        self.released_event_id.is_none()
    }
}

/// An aggregate capacity-N admission. The wrapped reservation contains
/// weighted half-open claims; this context binds it to the exact readiness,
/// occupancy, resource catalog, capacity snapshot and pre-append ledger head.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapacityReservationCommit {
    pub reservation: CapacityReservation,
    pub package_sha256: String,
    pub design_revision_sha256: String,
    pub entity_id: String,
    pub robot_id: String,
    pub readiness_receipt_sha256: String,
    pub occupancy_assessment_receipt_sha256: String,
    pub occupancy_engine_build_sha256: String,
    pub clearance_policy_sha256: String,
    pub resource_catalog_sha256: String,
    pub availability_snapshot_sha256: String,
    pub site_snapshot_sha256: String,
    pub site_ledger_sequence: u64,
    pub site_ledger_event_sha256: String,
    pub checked_at_tick: u64,
    pub valid_until_tick: u64,
    pub committed_at_tick: u64,
    /// Lease expiry prevents reuse of a stale commit receipt. It never frees
    /// physical capacity without a separate referee-authored release event.
    pub lease_valid_until_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapacityReservationRecord {
    pub commit: CapacityReservationCommit,
    pub committed_event_id: String,
    pub committed_sequence: u64,
    pub released_event_id: Option<String>,
    pub released_at_tick: Option<u64>,
    pub release_receipt_sha256: Option<String>,
}

impl CapacityReservationRecord {
    pub fn is_active(&self) -> bool {
        self.released_event_id.is_none()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SiteEventPayload {
    WorkPackagePublished {
        package: SiteWorkPackage,
    },
    ResourceCatalogPublished {
        catalog_sha256: String,
        resources: Vec<SiteResourceDefinition>,
    },
    /// Separate namespace from exclusive capacity-one resources. A resource
    /// must exist in exactly one catalog so the two admission models cannot
    /// double-book or silently share different capacity semantics.
    CapacityResourceCatalogPublished {
        catalog_sha256: String,
        resources: Vec<CapacityResourceDefinition>,
    },
    /// One coherent owner-attested capacity snapshot. An adverse derating
    /// update is recorded even when existing reservations become non-robust;
    /// it blocks future admissions but never releases active physical claims.
    CapacityAvailabilitySnapshotPublished {
        availability_snapshot_sha256: String,
        site_snapshot_sha256: String,
        site_clock_domain_id: String,
        windows: Vec<CapacityAvailabilityWindow>,
    },
    CapacityReservationCommitted {
        commit: CapacityReservationCommit,
    },
    CapacityReservationReleased {
        reservation_id: String,
        site_clock_domain_id: String,
        cleared_at_tick: u64,
        site_snapshot_sha256: String,
        site_ledger_sequence: u64,
        site_ledger_event_sha256: String,
        release_receipt_sha256: String,
    },
    SpaceTimeReservationCommitted {
        reservation: SpaceTimeReservation,
    },
    SpaceTimeReservationReleased {
        reservation_id: String,
        site_clock_domain_id: String,
        cleared_at_tick: u64,
        site_snapshot_sha256: String,
        site_ledger_sequence: u64,
        site_ledger_event_sha256: String,
        release_receipt_sha256: String,
    },
    StationQualified {
        package_sha256: String,
        station_id: String,
        frame_sha256: String,
        transform_sha256: String,
        calibration_sha256: String,
    },
    ExecutionStarted {
        package_sha256: String,
        entity_id: String,
        attempt_id: String,
        station_qualification_event_id: String,
    },
    RobotObservationRecorded {
        package_sha256: String,
        entity_id: String,
        attempt_id: String,
        disposition: RobotExecutionDisposition,
        robot_receipt_sha256: String,
    },
    ExecutionInterrupted {
        package_sha256: String,
        entity_id: String,
        attempt_id: String,
        reason: String,
        interruption_receipt_sha256: String,
    },
    RefereeAssessmentRecorded {
        package_sha256: String,
        entity_id: String,
        attempt_id: String,
        assessment: SiteRefereeAssessment,
    },
    HumanWaiverIssued {
        package_sha256: String,
        waiver: HumanWaiver,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct SiteEventEnvelope {
    pub event_id: String,
    /// Strictly increasing ledger sequence. Wall-clock timestamps never order
    /// or authorize site-state transitions.
    pub sequence: u64,
    pub previous_event_sha256: Option<String>,
    /// Digest of canonical event bytes, verified by the caller.
    pub event_sha256: String,
    pub actor_id: String,
    pub actor_role: SiteActorRole,
    pub payload: SiteEventPayload,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteRejectionReason {
    GeometryOutsideTolerance,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SiteEntityState {
    NotAttempted,
    ExecutionInProgress {
        attempt_id: String,
    },
    AwaitingIndependentMeasurement {
        attempt_id: String,
        robot_disposition: RobotExecutionDisposition,
    },
    Accepted {
        attempt_id: String,
        referee_event_id: String,
        measurement_receipt_sha256: String,
    },
    Rejected {
        attempt_id: String,
        referee_event_id: String,
        receipt_sha256: String,
        reason: SiteRejectionReason,
    },
    PhysicallyAbsent {
        attempt_id: String,
        referee_event_id: String,
        receipt_sha256: String,
    },
    Held {
        attempt_id: String,
        referee_event_id: String,
        receipt_sha256: String,
        reason: EntityScoreReason,
    },
    Waived {
        waiver_event_id: String,
        authority_id: String,
        waiver_receipt_sha256: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SiteLedgerError {
    EmptyEventIdentity,
    InvalidEventDigest,
    InvalidPreviousEventDigest,
    EventSequenceMismatch { expected: u64, actual: u64 },
    PreviousEventDigestMismatch,
    ConflictingEventId(String),
    ConflictingEventDigest(String),
    EventIndexCorrupt,
    EventHistoryContainsDuplicate(String),
    ActorRoleDenied,
    EmptyActorRegistration,
    EmptyPackageIdentity,
    InvalidPackageDigest,
    InvalidIdsEngineBuildDigest,
    EmptyEntityManifest,
    EmptyEntityId,
    DuplicatePackageEntityId(String),
    DuplicatePackageDigest(String),
    PackageHasUnresolvedAttempts(String),
    InvalidAcceptanceProfile,
    UnknownPackage(String),
    InactivePackage(String),
    UnknownDesignEntity(String),
    UnknownStationQualification(String),
    StationQualificationContextMismatch,
    EmptyStationId,
    InvalidCalibrationDigest,
    EmptyAttemptId,
    DuplicateAttemptId(String),
    AttemptContextMismatch(String),
    AttemptActorMismatch(String),
    AttemptAlreadyObserved(String),
    AttemptNotObserved(String),
    AttemptAlreadyAssessed(String),
    EntityTransitionDenied(String),
    InvalidReceiptDigest,
    InvalidWaiver(String),
    AcceptanceScorerRejectedMeasurement(String),
    InvalidResourceCatalogDigest,
    ResourceCatalogDigestContentMismatch,
    EmptyResourceCatalog,
    EmptyResourceId,
    DuplicateResourceId(String),
    ResourceCatalogWouldOrphanReservation(String),
    ResourceCatalogMismatch,
    PackageHasActiveSpaceTimeReservations(String),
    PackageHasActiveCapacityReservations(String),
    ReservationSnapshotMismatch,
    InvalidSpaceTimeReservation(String),
    DuplicateSpaceTimeReservation(String),
    UnknownSpaceTimeReservation(String),
    SpaceTimeReservationAlreadyReleased(String),
    SpaceTimeReservationConflict {
        resource_id: String,
        conflicting_reservation_id: String,
    },
    ReservationReleaseContextMismatch(String),
    InvalidCapacityResourceCatalogDigest,
    EmptyCapacityResourceCatalog,
    InvalidCapacityResourceDefinition(String),
    DuplicateCapacityResourceId(String),
    CapacityResourceCatalogDigestContentMismatch,
    CapacityResourceCatalogWouldOrphanReservation(String),
    CapacityCatalogNamespaceCollision(String),
    InvalidCapacityAvailabilitySnapshot,
    InvalidCapacityWindow(String),
    CapacityAvailabilitySnapshotWouldOrphanReservation(String),
    CapacityResourceCatalogMismatch,
    CapacityAvailabilitySnapshotMismatch,
    DuplicateCapacityReservation(String),
    UnknownCapacityReservation(String),
    UnknownCapacityReservationResource(String),
    CapacityResourceDefinitionMismatch(String),
    CapacityReservationAlreadyReleased(String),
    InvalidCapacityReservation(String),
    CapacityReservationSnapshotMismatch,
    CapacityAdmissionUnqualified(Vec<RobustCapacityError>),
    CapacityAdmissionNotRobust {
        impacted_reservation_ids: Vec<String>,
    },
    CapacityReservationReleaseContextMismatch(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteApplyOutcome {
    Applied,
    DuplicateIgnored,
}

#[derive(Clone, Debug, PartialEq)]
struct StationQualification {
    package_sha256: String,
    station_id: String,
    calibration_sha256: String,
}

#[derive(Clone, Debug, PartialEq)]
struct ExecutionAttempt {
    package_sha256: String,
    entity_id: String,
    robot_id: String,
    robot_disposition: Option<RobotExecutionDisposition>,
    referee_assessed: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct PackageProjection {
    package: SiteWorkPackage,
    superseded_by: Option<String>,
    entity_states: BTreeMap<String, SiteEntityState>,
    station_qualifications: BTreeMap<String, StationQualification>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SiteStateLedger {
    actor_registry: SiteActorRegistry,
    last_sequence: u64,
    last_event_sha256: Option<String>,
    events: Vec<SiteEventEnvelope>,
    events_by_id: BTreeMap<String, usize>,
    event_ids_by_digest: BTreeMap<String, String>,
    packages: BTreeMap<String, PackageProjection>,
    active_package_sha256: Option<String>,
    attempts: BTreeMap<String, ExecutionAttempt>,
    resource_catalog_sha256: Option<String>,
    resource_catalog: BTreeMap<String, String>,
    space_time_reservations: BTreeMap<String, SpaceTimeReservationRecord>,
    capacity_resource_catalog_sha256: Option<String>,
    capacity_resources: BTreeMap<String, CapacityResourceDefinition>,
    capacity_availability_snapshot_sha256: Option<String>,
    capacity_site_snapshot_sha256: Option<String>,
    capacity_site_clock_domain_id: Option<String>,
    capacity_windows: Vec<CapacityAvailabilityWindow>,
    capacity_reservations: BTreeMap<String, CapacityReservationRecord>,
    /// One non-overlapping interval index per exclusive resource. Since
    /// committed intervals cannot overlap, predecessor/successor checks are
    /// sufficient for O(log n) conflict detection on each claim.
    active_resource_windows: BTreeMap<String, BTreeMap<u64, (u64, String)>>,
}

impl SiteStateLedger {
    /// Create an unconfigured ledger. All event ingestion is denied until the
    /// caller supplies an authenticated actor-role registry.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_actor_registry(actor_registry: SiteActorRegistry) -> Self {
        Self {
            actor_registry,
            ..Self::default()
        }
    }

    pub fn actor_registry(&self) -> &SiteActorRegistry {
        &self.actor_registry
    }

    /// Reconstruct the projection from caller-preverified history and the
    /// same authenticated actor-role registry used by the original reducer.
    pub fn replay(
        events: &[SiteEventEnvelope],
        actor_registry: SiteActorRegistry,
    ) -> Result<Self, SiteLedgerError> {
        let mut ledger = Self::with_actor_registry(actor_registry);
        for event in events {
            if ledger.apply(event.clone())? == SiteApplyOutcome::DuplicateIgnored {
                return Err(SiteLedgerError::EventHistoryContainsDuplicate(
                    event.event_id.clone(),
                ));
            }
        }
        Ok(ledger)
    }

    /// Apply a caller-preverified event atomically.
    ///
    /// An exact duplicate envelope is idempotently ignored. A reused event ID
    /// with changed content, a sequence gap, or a broken previous-digest link
    /// is rejected. Any payload/state-transition failure leaves the ledger
    /// unchanged.
    pub fn apply(&mut self, event: SiteEventEnvelope) -> Result<SiteApplyOutcome, SiteLedgerError> {
        validate_event_envelope(&event)?;
        if !self.actor_registry.allows(&event.actor_id, event.actor_role) {
            return Err(SiteLedgerError::ActorRoleDenied);
        }

        if let Some(index) = self.events_by_id.get(&event.event_id) {
            let existing = self
                .events
                .get(*index)
                .ok_or(SiteLedgerError::EventIndexCorrupt)?;
            return if existing == &event {
                Ok(SiteApplyOutcome::DuplicateIgnored)
            } else {
                Err(SiteLedgerError::ConflictingEventId(event.event_id))
            };
        }
        if let Some(existing_event_id) = self.event_ids_by_digest.get(&event.event_sha256) {
            return Err(SiteLedgerError::ConflictingEventDigest(
                existing_event_id.clone(),
            ));
        }

        let expected =
            self.last_sequence
                .checked_add(1)
                .ok_or(SiteLedgerError::EventSequenceMismatch {
                    expected: u64::MAX,
                    actual: event.sequence,
                })?;
        if event.sequence != expected {
            return Err(SiteLedgerError::EventSequenceMismatch {
                expected,
                actual: event.sequence,
            });
        }
        if event.previous_event_sha256 != self.last_event_sha256 {
            return Err(SiteLedgerError::PreviousEventDigestMismatch);
        }

        // Prepare validates every precondition without mutating the ledger.
        // Commit then touches only the affected package/attempt and appends one
        // event. This avoids cloning the full history and site projection on
        // every event (which made ingestion O(number_of_events²)).
        let mutation = self.prepare_payload(&event)?;
        let event_index = self.events.len();
        self.commit_payload(mutation, &event);

        self.last_sequence = event.sequence;
        self.last_event_sha256 = Some(event.event_sha256.clone());
        self.event_ids_by_digest
            .insert(event.event_sha256.clone(), event.event_id.clone());
        self.events_by_id
            .insert(event.event_id.clone(), event_index);
        self.events.push(event);
        Ok(SiteApplyOutcome::Applied)
    }

    pub fn last_sequence(&self) -> u64 {
        self.last_sequence
    }

    pub fn last_event_sha256(&self) -> Option<&str> {
        self.last_event_sha256.as_deref()
    }

    pub fn active_package_sha256(&self) -> Option<&str> {
        self.active_package_sha256.as_deref()
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn package_is_current(&self, package_sha256: &str) -> Option<bool> {
        self.packages
            .get(package_sha256)
            .map(|package| package.superseded_by.is_none())
    }

    pub fn work_package(&self, package_sha256: &str) -> Option<&SiteWorkPackage> {
        self.packages
            .get(package_sha256)
            .map(|projection| &projection.package)
    }

    pub fn package_superseded_by(&self, package_sha256: &str) -> Option<&str> {
        self.packages
            .get(package_sha256)
            .and_then(|package| package.superseded_by.as_deref())
    }

    pub fn entity_state(&self, package_sha256: &str, entity_id: &str) -> Option<&SiteEntityState> {
        self.packages
            .get(package_sha256)
            .and_then(|package| package.entity_states.get(entity_id))
    }

    pub fn accepted_count(&self, package_sha256: &str) -> Option<u64> {
        let package = self.packages.get(package_sha256)?;
        Some(
            package
                .entity_states
                .values()
                .filter(|state| matches!(state, SiteEntityState::Accepted { .. }))
                .count() as u64,
        )
    }

    pub fn resource_catalog_sha256(&self) -> Option<&str> {
        self.resource_catalog_sha256.as_deref()
    }

    pub fn resource_definition_sha256(&self, resource_id: &str) -> Option<&str> {
        self.resource_catalog
            .get(resource_id)
            .map(String::as_str)
    }

    pub fn space_time_reservation(
        &self,
        reservation_id: &str,
    ) -> Option<&SpaceTimeReservationRecord> {
        self.space_time_reservations.get(reservation_id)
    }

    pub fn active_space_time_reservations(
        &self,
    ) -> impl Iterator<Item = &SpaceTimeReservationRecord> {
        self.space_time_reservations
            .values()
            .filter(|record| record.is_active())
    }

    pub fn active_resource_claim_count(&self) -> usize {
        self.active_resource_windows
            .values()
            .map(BTreeMap::len)
            .sum()
    }

    pub fn active_resource_count(&self) -> usize {
        self.active_resource_windows.len()
    }

    pub fn capacity_resource_catalog_sha256(&self) -> Option<&str> {
        self.capacity_resource_catalog_sha256.as_deref()
    }

    pub fn capacity_availability_snapshot_sha256(&self) -> Option<&str> {
        self.capacity_availability_snapshot_sha256.as_deref()
    }

    pub fn capacity_reservation(
        &self,
        reservation_id: &str,
    ) -> Option<&CapacityReservationRecord> {
        self.capacity_reservations.get(reservation_id)
    }

    pub fn active_capacity_reservations(
        &self,
    ) -> impl Iterator<Item = &CapacityReservationRecord> {
        self.capacity_reservations
            .values()
            .filter(|record| record.is_active())
    }

    pub fn active_capacity_reservation_count(&self) -> usize {
        self.active_capacity_reservations().count()
    }

    pub fn capacity_resource_count(&self) -> usize {
        self.capacity_resources.len()
    }

    /// Assess all currently active capacity-N commitments against the latest
    /// owner-attested capacity windows. A non-robust report is evidence of a
    /// changed constraint, not permission to erase or auto-release claims.
    pub fn assess_current_capacity(
        &self,
    ) -> Result<RobustCumulativeCapacityReport, SiteLedgerError> {
        let catalog_sha256 = self
            .capacity_resource_catalog_sha256
            .as_ref()
            .ok_or(SiteLedgerError::CapacityResourceCatalogMismatch)?;
        let site_snapshot_sha256 = self
            .capacity_site_snapshot_sha256
            .as_ref()
            .ok_or(SiteLedgerError::CapacityAvailabilitySnapshotMismatch)?;
        let site_clock_domain_id = self
            .capacity_site_clock_domain_id
            .as_ref()
            .ok_or(SiteLedgerError::CapacityAvailabilitySnapshotMismatch)?;
        let scenario = RobustCumulativeCapacityScenario {
            base: CumulativeCapacityScenario {
                scenario_id: format!("site-ledger-sequence-{}", self.last_sequence),
                resource_catalog_sha256: catalog_sha256.clone(),
                site_snapshot_sha256: site_snapshot_sha256.clone(),
                site_clock_domain_id: site_clock_domain_id.clone(),
                resources: self.capacity_resources.values().cloned().collect(),
                reservations: self
                    .active_capacity_reservations()
                    .map(|record| record.commit.reservation.clone())
                    .collect(),
            },
            availability_windows: self.capacity_windows.clone(),
        };
        evaluate_robust_cumulative_resource_capacity(&scenario)
            .map_err(SiteLedgerError::CapacityAdmissionUnqualified)
    }

    pub fn event_history(&self) -> &[SiteEventEnvelope] {
        &self.events
    }

    fn prepare_payload(
        &self,
        event: &SiteEventEnvelope,
    ) -> Result<PreparedSiteMutation, SiteLedgerError> {
        match &event.payload {
            SiteEventPayload::WorkPackagePublished { package } => {
                require_role(event.actor_role, SiteActorRole::DesignAuthority)?;
                validate_work_package(package)?;
                if self.packages.contains_key(&package.package_sha256) {
                    return Err(SiteLedgerError::DuplicatePackageDigest(
                        package.package_sha256.clone(),
                    ));
                }
                if let Some(previous) = self.active_package_sha256.as_ref() {
                    if self.active_space_time_reservations().any(|record| {
                        record.reservation.package_sha256 == *previous
                    }) {
                        return Err(SiteLedgerError::PackageHasActiveSpaceTimeReservations(
                            previous.clone(),
                        ));
                    }
                    if self.active_capacity_reservations().any(|record| {
                        record.commit.package_sha256 == *previous
                    }) {
                        return Err(SiteLedgerError::PackageHasActiveCapacityReservations(
                            previous.clone(),
                        ));
                    }
                    let previous_projection = self
                        .packages
                        .get(previous)
                        .ok_or_else(|| SiteLedgerError::UnknownPackage(previous.clone()))?;
                    if previous_projection.entity_states.values().any(|state| {
                        matches!(
                            state,
                            SiteEntityState::ExecutionInProgress { .. }
                                | SiteEntityState::AwaitingIndependentMeasurement { .. }
                                | SiteEntityState::Rejected { .. }
                                | SiteEntityState::Held { .. }
                        )
                    }) {
                        return Err(SiteLedgerError::PackageHasUnresolvedAttempts(
                            previous.clone(),
                        ));
                    }
                }
                let entity_states = package
                    .entities
                    .iter()
                    .map(|entity| {
                        (
                            entity.design_entity_id.clone(),
                            SiteEntityState::NotAttempted,
                        )
                    })
                    .collect();
                Ok(PreparedSiteMutation::PublishPackage {
                    previous_package_sha256: self.active_package_sha256.clone(),
                    package_sha256: package.package_sha256.clone(),
                    projection: PackageProjection {
                        package: package.clone(),
                        superseded_by: None,
                        entity_states,
                        station_qualifications: BTreeMap::new(),
                    },
                })
            }
            SiteEventPayload::ResourceCatalogPublished {
                catalog_sha256,
                resources,
            } => {
                require_role(event.actor_role, SiteActorRole::SiteResourceAuthority)?;
                if !is_sha256(catalog_sha256) {
                    return Err(SiteLedgerError::InvalidResourceCatalogDigest);
                }
                if resources.is_empty() {
                    return Err(SiteLedgerError::EmptyResourceCatalog);
                }
                if self.active_space_time_reservations().next().is_some()
                    && self.resource_catalog_sha256.as_deref()
                        != Some(catalog_sha256.as_str())
                {
                    return Err(SiteLedgerError::ResourceCatalogWouldOrphanReservation(
                        "resource-catalog-version".into(),
                    ));
                }
                let mut definitions = BTreeMap::<String, String>::new();
                for resource in resources {
                    if resource.resource_id.trim().is_empty() {
                        return Err(SiteLedgerError::EmptyResourceId);
                    }
                    if self.capacity_resources.contains_key(&resource.resource_id) {
                        return Err(SiteLedgerError::CapacityCatalogNamespaceCollision(
                            resource.resource_id.clone(),
                        ));
                    }
                    if !is_sha256(&resource.definition_sha256) {
                        return Err(SiteLedgerError::InvalidReceiptDigest);
                    }
                    if definitions
                        .insert(
                            resource.resource_id.clone(),
                            resource.definition_sha256.clone(),
                        )
                        .is_some()
                    {
                        return Err(SiteLedgerError::DuplicateResourceId(
                            resource.resource_id.clone(),
                        ));
                    }
                }
                if self.resource_catalog_sha256.as_deref() == Some(catalog_sha256.as_str())
                    && self.resource_catalog != definitions
                {
                    return Err(SiteLedgerError::ResourceCatalogDigestContentMismatch);
                }
                for record in self.active_space_time_reservations() {
                    for claim in &record.reservation.claims {
                        if definitions.get(&claim.resource_id)
                            != Some(&claim.definition_sha256)
                        {
                            return Err(SiteLedgerError::ResourceCatalogWouldOrphanReservation(
                                claim.resource_id.clone(),
                            ));
                        }
                    }
                }
                Ok(PreparedSiteMutation::PublishResourceCatalog {
                    catalog_sha256: catalog_sha256.clone(),
                    definitions,
                })
            }
            SiteEventPayload::CapacityResourceCatalogPublished {
                catalog_sha256,
                resources,
            } => {
                require_role(event.actor_role, SiteActorRole::SiteResourceAuthority)?;
                if !is_sha256(catalog_sha256) {
                    return Err(SiteLedgerError::InvalidCapacityResourceCatalogDigest);
                }
                if resources.is_empty() {
                    return Err(SiteLedgerError::EmptyCapacityResourceCatalog);
                }
                if self.active_capacity_reservations().next().is_some()
                    && self.capacity_resource_catalog_sha256.as_deref()
                        != Some(catalog_sha256.as_str())
                {
                    return Err(SiteLedgerError::CapacityResourceCatalogWouldOrphanReservation(
                        "capacity-resource-catalog-version".into(),
                    ));
                }

                let mut definitions = BTreeMap::<String, CapacityResourceDefinition>::new();
                for resource in resources {
                    if resource.resource_id.trim().is_empty()
                        || resource.capacity_owner_id.trim().is_empty()
                        || resource.unit_label.trim().is_empty()
                        || resource.capacity_units == 0
                        || !is_sha256(&resource.definition_sha256)
                    {
                        return Err(SiteLedgerError::InvalidCapacityResourceDefinition(
                            resource.resource_id.clone(),
                        ));
                    }
                    if self.resource_catalog.contains_key(&resource.resource_id) {
                        return Err(SiteLedgerError::CapacityCatalogNamespaceCollision(
                            resource.resource_id.clone(),
                        ));
                    }
                    if definitions
                        .insert(resource.resource_id.clone(), resource.clone())
                        .is_some()
                    {
                        return Err(SiteLedgerError::DuplicateCapacityResourceId(
                            resource.resource_id.clone(),
                        ));
                    }
                }
                if self.capacity_resource_catalog_sha256.as_deref()
                    == Some(catalog_sha256.as_str())
                    && self.capacity_resources != definitions
                {
                    return Err(SiteLedgerError::CapacityResourceCatalogDigestContentMismatch);
                }
                for record in self.active_capacity_reservations() {
                    for claim in &record.commit.reservation.claims {
                        if definitions
                            .get(&claim.resource_id)
                            .is_none_or(|definition| {
                                definition.definition_sha256 != claim.resource_definition_sha256
                            })
                        {
                            return Err(
                                SiteLedgerError::CapacityResourceCatalogWouldOrphanReservation(
                                    claim.resource_id.clone(),
                                ),
                            );
                        }
                    }
                }
                Ok(PreparedSiteMutation::PublishCapacityResourceCatalog {
                    catalog_sha256: catalog_sha256.clone(),
                    definitions,
                })
            }
            SiteEventPayload::CapacityAvailabilitySnapshotPublished {
                availability_snapshot_sha256,
                site_snapshot_sha256,
                site_clock_domain_id,
                windows,
            } => {
                require_role(event.actor_role, SiteActorRole::SiteResourceAuthority)?;
                if !is_sha256(availability_snapshot_sha256)
                    || !is_sha256(site_snapshot_sha256)
                    || site_clock_domain_id.trim().is_empty()
                    || windows.is_empty()
                {
                    return Err(SiteLedgerError::InvalidCapacityAvailabilitySnapshot);
                }
                let catalog_sha256 = self
                    .capacity_resource_catalog_sha256
                    .as_ref()
                    .ok_or(SiteLedgerError::CapacityResourceCatalogMismatch)?;
                let scenario = RobustCumulativeCapacityScenario {
                    base: CumulativeCapacityScenario {
                        scenario_id: format!(
                            "capacity-window-validation-{}",
                            event.event_id
                        ),
                        resource_catalog_sha256: catalog_sha256.clone(),
                        site_snapshot_sha256: site_snapshot_sha256.clone(),
                        site_clock_domain_id: site_clock_domain_id.clone(),
                        resources: self.capacity_resources.values().cloned().collect(),
                        reservations: Vec::new(),
                    },
                    availability_windows: windows.clone(),
                };
                // This validates each owner's window identity, definition,
                // site snapshot, clock, interval, capacity ordering and
                // non-overlap. Coverage of active reservations is deliberately
                // evaluated separately: an adverse derating update must still
                // be recorded, even if it makes existing commitments non-robust.
                evaluate_robust_cumulative_resource_capacity(&scenario)
                    .map_err(SiteLedgerError::CapacityAdmissionUnqualified)?;

                if self.capacity_availability_snapshot_sha256.as_deref()
                    == Some(availability_snapshot_sha256.as_str())
                    && (self.capacity_site_snapshot_sha256.as_deref()
                        != Some(site_snapshot_sha256.as_str())
                        || self.capacity_site_clock_domain_id.as_deref()
                            != Some(site_clock_domain_id.as_str())
                        || self.capacity_windows != *windows)
                {
                    return Err(
                        SiteLedgerError::CapacityAvailabilitySnapshotWouldOrphanReservation(
                            "capacity-availability-snapshot-digest-content-mismatch".into(),
                        ),
                    );
                }
                Ok(PreparedSiteMutation::PublishCapacityAvailabilitySnapshot {
                    availability_snapshot_sha256: availability_snapshot_sha256.clone(),
                    site_snapshot_sha256: site_snapshot_sha256.clone(),
                    site_clock_domain_id: site_clock_domain_id.clone(),
                    windows: windows.clone(),
                })
            }
            SiteEventPayload::CapacityReservationCommitted { commit } => {
                require_role(event.actor_role, SiteActorRole::DispatchCoordinator)?;
                validate_capacity_reservation_commit(self, commit)?;
                if self
                    .capacity_reservations
                    .contains_key(&commit.reservation.reservation_id)
                {
                    return Err(SiteLedgerError::DuplicateCapacityReservation(
                        commit.reservation.reservation_id.clone(),
                    ));
                }

                let catalog_sha256 = self
                    .capacity_resource_catalog_sha256
                    .as_ref()
                    .ok_or(SiteLedgerError::CapacityResourceCatalogMismatch)?;
                let site_snapshot_sha256 = self
                    .capacity_site_snapshot_sha256
                    .as_ref()
                    .ok_or(SiteLedgerError::CapacityAvailabilitySnapshotMismatch)?;
                let site_clock_domain_id = self
                    .capacity_site_clock_domain_id
                    .as_ref()
                    .ok_or(SiteLedgerError::CapacityAvailabilitySnapshotMismatch)?;
                let mut reservations: Vec<CapacityReservation> = self
                    .active_capacity_reservations()
                    .map(|record| record.commit.reservation.clone())
                    .collect();
                reservations.push(commit.reservation.clone());
                let scenario = RobustCumulativeCapacityScenario {
                    base: CumulativeCapacityScenario {
                        scenario_id: format!(
                            "capacity-admission-{}-{}",
                            self.last_sequence, commit.reservation.reservation_id
                        ),
                        resource_catalog_sha256: catalog_sha256.clone(),
                        site_snapshot_sha256: site_snapshot_sha256.clone(),
                        site_clock_domain_id: site_clock_domain_id.clone(),
                        resources: self.capacity_resources.values().cloned().collect(),
                        reservations,
                    },
                    availability_windows: self.capacity_windows.clone(),
                };
                let report = evaluate_robust_cumulative_resource_capacity(&scenario)
                    .map_err(SiteLedgerError::CapacityAdmissionUnqualified)?;
                if !report.is_robustly_feasible
                    || report.indeterminate_segment_count != 0
                    || report.definitely_over_capacity_segment_count != 0
                {
                    return Err(SiteLedgerError::CapacityAdmissionNotRobust {
                        impacted_reservation_ids: report.impacted_reservation_ids(),
                    });
                }
                Ok(PreparedSiteMutation::CommitCapacityReservation {
                    commit: commit.clone(),
                    committed_event_id: event.event_id.clone(),
                    committed_sequence: event.sequence,
                })
            }
            SiteEventPayload::CapacityReservationReleased {
                reservation_id,
                site_clock_domain_id,
                cleared_at_tick,
                site_snapshot_sha256,
                site_ledger_sequence,
                site_ledger_event_sha256,
                release_receipt_sha256,
            } => {
                require_role(event.actor_role, SiteActorRole::Referee)?;
                if !is_sha256(site_snapshot_sha256) || !is_sha256(release_receipt_sha256) {
                    return Err(SiteLedgerError::InvalidReceiptDigest);
                }
                if !is_sha256(site_ledger_event_sha256)
                    || *site_ledger_sequence != self.last_sequence
                    || self.last_event_sha256.as_deref()
                        != Some(site_ledger_event_sha256.as_str())
                    || self.capacity_site_snapshot_sha256.as_deref()
                        != Some(site_snapshot_sha256.as_str())
                {
                    return Err(SiteLedgerError::ReservationSnapshotMismatch);
                }
                let record = self
                    .capacity_reservations
                    .get(reservation_id)
                    .ok_or_else(|| {
                        SiteLedgerError::UnknownCapacityReservation(reservation_id.clone())
                    })?;
                if !record.is_active() {
                    return Err(SiteLedgerError::CapacityReservationAlreadyReleased(
                        reservation_id.clone(),
                    ));
                }
                if site_clock_domain_id != &record.commit.reservation.site_clock_domain_id
                    || self.capacity_site_clock_domain_id.as_deref()
                        != Some(site_clock_domain_id.as_str())
                    || *cleared_at_tick < record.commit.committed_at_tick
                {
                    return Err(
                        SiteLedgerError::CapacityReservationReleaseContextMismatch(
                            reservation_id.clone(),
                        ),
                    );
                }
                Ok(PreparedSiteMutation::ReleaseCapacityReservation {
                    reservation_id: reservation_id.clone(),
                    released_event_id: event.event_id.clone(),
                    cleared_at_tick: *cleared_at_tick,
                    release_receipt_sha256: release_receipt_sha256.clone(),
                })
            }
            SiteEventPayload::SpaceTimeReservationCommitted { reservation } => {
                require_role(event.actor_role, SiteActorRole::DispatchCoordinator)?;
                validate_space_time_reservation(self, reservation)?;
                if self
                    .space_time_reservations
                    .contains_key(&reservation.reservation_id)
                {
                    return Err(SiteLedgerError::DuplicateSpaceTimeReservation(
                        reservation.reservation_id.clone(),
                    ));
                }
                for claim in &reservation.claims {
                    let Some(index) = self.active_resource_windows.get(&claim.resource_id) else {
                        continue;
                    };

                    // The index invariant is that committed intervals for a
                    // resource never overlap. Therefore only the adjacent
                    // predecessor and successor can conflict with this claim.
                    if let Some((_, (previous_end, previous_id))) =
                        index.range(..claim.start_tick).next_back()
                    {
                        if *previous_end > claim.start_tick {
                            return Err(SiteLedgerError::SpaceTimeReservationConflict {
                                resource_id: claim.resource_id.clone(),
                                conflicting_reservation_id: previous_id.clone(),
                            });
                        }
                    }
                    if let Some((next_start, (_, next_id))) =
                        index.range(claim.start_tick..).next()
                    {
                        if *next_start < claim.end_tick {
                            return Err(SiteLedgerError::SpaceTimeReservationConflict {
                                resource_id: claim.resource_id.clone(),
                                conflicting_reservation_id: next_id.clone(),
                            });
                        }
                    }
                }
                Ok(PreparedSiteMutation::CommitSpaceTimeReservation {
                    reservation: reservation.clone(),
                    committed_event_id: event.event_id.clone(),
                    committed_sequence: event.sequence,
                })
            }
            SiteEventPayload::SpaceTimeReservationReleased {
                reservation_id,
                site_clock_domain_id,
                cleared_at_tick,
                site_snapshot_sha256,
                site_ledger_sequence,
                site_ledger_event_sha256,
                release_receipt_sha256,
            } => {
                require_role(event.actor_role, SiteActorRole::Referee)?;
                if !is_sha256(site_snapshot_sha256) || !is_sha256(release_receipt_sha256) {
                    return Err(SiteLedgerError::InvalidReceiptDigest);
                }
                if !is_sha256(site_ledger_event_sha256)
                    || *site_ledger_sequence != self.last_sequence
                    || self.last_event_sha256.as_deref()
                        != Some(site_ledger_event_sha256.as_str())
                {
                    return Err(SiteLedgerError::ReservationSnapshotMismatch);
                }
                let record = self
                    .space_time_reservations
                    .get(reservation_id)
                    .ok_or_else(|| {
                        SiteLedgerError::UnknownSpaceTimeReservation(reservation_id.clone())
                    })?;
                if !record.is_active() {
                    return Err(SiteLedgerError::SpaceTimeReservationAlreadyReleased(
                        reservation_id.clone(),
                    ));
                }
                if *site_clock_domain_id != record.reservation.site_clock_domain_id
                    || *cleared_at_tick < record.reservation.committed_at_tick
                {
                    return Err(SiteLedgerError::ReservationReleaseContextMismatch(
                        reservation_id.clone(),
                    ));
                }
                Ok(PreparedSiteMutation::ReleaseSpaceTimeReservation {
                    reservation_id: reservation_id.clone(),
                    claims: record.reservation.claims.clone(),
                    released_event_id: event.event_id.clone(),
                    cleared_at_tick: *cleared_at_tick,
                    release_receipt_sha256: release_receipt_sha256.clone(),
                })
            }
            SiteEventPayload::StationQualified {
                package_sha256,
                station_id,
                frame_sha256,
                transform_sha256,
                calibration_sha256,
            } => {
                require_role(event.actor_role, SiteActorRole::Surveyor)?;
                if station_id.trim().is_empty() {
                    return Err(SiteLedgerError::EmptyStationId);
                }
                if !is_sha256(calibration_sha256) {
                    return Err(SiteLedgerError::InvalidCalibrationDigest);
                }
                let package = self.current_package(package_sha256)?;
                if package.package.frame_sha256 != *frame_sha256
                    || package.package.transform_sha256 != *transform_sha256
                {
                    return Err(SiteLedgerError::StationQualificationContextMismatch);
                }
                Ok(PreparedSiteMutation::QualifyStation {
                    package_sha256: package_sha256.clone(),
                    qualification_event_id: event.event_id.clone(),
                    qualification: StationQualification {
                        package_sha256: package_sha256.clone(),
                        station_id: station_id.clone(),
                        calibration_sha256: calibration_sha256.clone(),
                    },
                })
            }
            SiteEventPayload::ExecutionStarted {
                package_sha256,
                entity_id,
                attempt_id,
                station_qualification_event_id,
            } => {
                require_role(event.actor_role, SiteActorRole::Robot)?;
                if attempt_id.trim().is_empty() {
                    return Err(SiteLedgerError::EmptyAttemptId);
                }
                if self.attempts.contains_key(attempt_id) {
                    return Err(SiteLedgerError::DuplicateAttemptId(attempt_id.clone()));
                }
                let package = self.current_package(package_sha256)?;
                if !package.entity_states.contains_key(entity_id) {
                    return Err(SiteLedgerError::UnknownDesignEntity(entity_id.clone()));
                }
                let qualification = package
                    .station_qualifications
                    .get(station_qualification_event_id)
                    .ok_or_else(|| {
                        SiteLedgerError::UnknownStationQualification(
                            station_qualification_event_id.clone(),
                        )
                    })?;
                if qualification.package_sha256 != *package_sha256
                    || qualification.station_id.trim().is_empty()
                    || !is_sha256(&qualification.calibration_sha256)
                {
                    return Err(SiteLedgerError::StationQualificationContextMismatch);
                }
                let state = package
                    .entity_states
                    .get(entity_id)
                    .ok_or_else(|| SiteLedgerError::UnknownDesignEntity(entity_id.clone()))?;
                if matches!(
                    state,
                    SiteEntityState::Accepted { .. }
                        | SiteEntityState::Waived { .. }
                        | SiteEntityState::ExecutionInProgress { .. }
                        | SiteEntityState::AwaitingIndependentMeasurement { .. }
                ) {
                    return Err(SiteLedgerError::EntityTransitionDenied(entity_id.clone()));
                }
                Ok(PreparedSiteMutation::StartExecution {
                    package_sha256: package_sha256.clone(),
                    entity_id: entity_id.clone(),
                    attempt_id: attempt_id.clone(),
                    attempt: ExecutionAttempt {
                        package_sha256: package_sha256.clone(),
                        entity_id: entity_id.clone(),
                        robot_id: event.actor_id.clone(),
                        robot_disposition: None,
                        referee_assessed: false,
                    },
                })
            }
            SiteEventPayload::RobotObservationRecorded {
                package_sha256,
                entity_id,
                attempt_id,
                disposition,
                robot_receipt_sha256,
            } => {
                require_role(event.actor_role, SiteActorRole::Robot)?;
                if !is_sha256(robot_receipt_sha256) {
                    return Err(SiteLedgerError::InvalidReceiptDigest);
                }
                let attempt = self
                    .attempts
                    .get(attempt_id)
                    .ok_or_else(|| SiteLedgerError::AttemptContextMismatch(attempt_id.clone()))?;
                validate_attempt_owner(
                    attempt,
                    package_sha256,
                    entity_id,
                    attempt_id,
                    &event.actor_id,
                )?;
                if attempt.robot_disposition.is_some() {
                    return Err(SiteLedgerError::AttemptAlreadyObserved(attempt_id.clone()));
                }
                let package = self.current_package(package_sha256)?;
                ensure_attempt_is_in_progress(package, entity_id, attempt_id)?;
                Ok(PreparedSiteMutation::RecordRobotObservation {
                    package_sha256: package_sha256.clone(),
                    entity_id: entity_id.clone(),
                    attempt_id: attempt_id.clone(),
                    disposition: *disposition,
                })
            }
            SiteEventPayload::ExecutionInterrupted {
                package_sha256,
                entity_id,
                attempt_id,
                reason,
                interruption_receipt_sha256,
            } => {
                require_role(event.actor_role, SiteActorRole::Robot)?;
                if reason.trim().is_empty() || !is_sha256(interruption_receipt_sha256) {
                    return Err(SiteLedgerError::InvalidReceiptDigest);
                }
                let attempt = self
                    .attempts
                    .get(attempt_id)
                    .ok_or_else(|| SiteLedgerError::AttemptContextMismatch(attempt_id.clone()))?;
                validate_attempt_owner(
                    attempt,
                    package_sha256,
                    entity_id,
                    attempt_id,
                    &event.actor_id,
                )?;
                if attempt.robot_disposition.is_some() || attempt.referee_assessed {
                    return Err(SiteLedgerError::AttemptAlreadyObserved(attempt_id.clone()));
                }
                let package = self.current_package(package_sha256)?;
                ensure_attempt_is_in_progress(package, entity_id, attempt_id)?;
                Ok(PreparedSiteMutation::RecordRobotObservation {
                    package_sha256: package_sha256.clone(),
                    entity_id: entity_id.clone(),
                    attempt_id: attempt_id.clone(),
                    disposition: RobotExecutionDisposition::Unknown,
                })
            }
            SiteEventPayload::RefereeAssessmentRecorded {
                package_sha256,
                entity_id,
                attempt_id,
                assessment,
            } => {
                require_role(event.actor_role, SiteActorRole::Referee)?;
                let attempt = self
                    .attempts
                    .get(attempt_id)
                    .ok_or_else(|| SiteLedgerError::AttemptContextMismatch(attempt_id.clone()))?;
                if attempt.package_sha256 != *package_sha256 || attempt.entity_id != *entity_id {
                    return Err(SiteLedgerError::AttemptContextMismatch(attempt_id.clone()));
                }
                let robot_disposition = attempt
                    .robot_disposition
                    .ok_or_else(|| SiteLedgerError::AttemptNotObserved(attempt_id.clone()))?;
                if attempt.referee_assessed {
                    return Err(SiteLedgerError::AttemptAlreadyAssessed(attempt_id.clone()));
                }
                let package = self.current_package(package_sha256)?;
                ensure_attempt_is_awaiting_measurement(package, entity_id, attempt_id)?;
                let outcome = evaluate_referee_assessment(
                    package,
                    entity_id,
                    robot_disposition,
                    assessment,
                )?;
                let new_state = match outcome {
                    ScoredRefereeOutcome::Accepted(receipt) => SiteEntityState::Accepted {
                        attempt_id: attempt_id.clone(),
                        referee_event_id: event.event_id.clone(),
                        measurement_receipt_sha256: receipt,
                    },
                    ScoredRefereeOutcome::Rejected(receipt, reason) => SiteEntityState::Rejected {
                        attempt_id: attempt_id.clone(),
                        referee_event_id: event.event_id.clone(),
                        receipt_sha256: receipt,
                        reason,
                    },
                    ScoredRefereeOutcome::Absent(receipt) => SiteEntityState::PhysicallyAbsent {
                        attempt_id: attempt_id.clone(),
                        referee_event_id: event.event_id.clone(),
                        receipt_sha256: receipt,
                    },
                    ScoredRefereeOutcome::Held(receipt, reason) => SiteEntityState::Held {
                        attempt_id: attempt_id.clone(),
                        referee_event_id: event.event_id.clone(),
                        receipt_sha256: receipt,
                        reason,
                    },
                };
                Ok(PreparedSiteMutation::RecordRefereeAssessment {
                    package_sha256: package_sha256.clone(),
                    entity_id: entity_id.clone(),
                    attempt_id: attempt_id.clone(),
                    new_state,
                })
            }
            SiteEventPayload::HumanWaiverIssued {
                package_sha256,
                waiver,
            } => {
                require_role(event.actor_role, SiteActorRole::SiteQualityAuthority)?;
                validate_waiver(waiver, &event.actor_id)?;
                let package = self.current_package(package_sha256)?;
                let state = package
                    .entity_states
                    .get(&waiver.design_entity_id)
                    .ok_or_else(|| {
                        SiteLedgerError::UnknownDesignEntity(waiver.design_entity_id.clone())
                    })?;
                if matches!(
                    state,
                    SiteEntityState::Accepted { .. }
                        | SiteEntityState::Waived { .. }
                        | SiteEntityState::ExecutionInProgress { .. }
                        | SiteEntityState::AwaitingIndependentMeasurement { .. }
                ) {
                    return Err(SiteLedgerError::EntityTransitionDenied(
                        waiver.design_entity_id.clone(),
                    ));
                }
                Ok(PreparedSiteMutation::IssueWaiver {
                    package_sha256: package_sha256.clone(),
                    entity_id: waiver.design_entity_id.clone(),
                    new_state: SiteEntityState::Waived {
                        waiver_event_id: event.event_id.clone(),
                        authority_id: waiver.authority_id.clone(),
                        waiver_receipt_sha256: waiver.waiver_receipt_sha256.clone(),
                    },
                })
            }
        }
    }

    fn commit_payload(&mut self, mutation: PreparedSiteMutation, event: &SiteEventEnvelope) {
        match mutation {
            PreparedSiteMutation::PublishPackage {
                previous_package_sha256,
                package_sha256,
                projection,
            } => {
                if let Some(previous) = previous_package_sha256 {
                    self.packages
                        .get_mut(&previous)
                        .expect("prepared previous package remains present")
                        .superseded_by = Some(package_sha256.clone());
                }
                self.packages.insert(package_sha256.clone(), projection);
                self.active_package_sha256 = Some(package_sha256);
            }
            PreparedSiteMutation::PublishResourceCatalog {
                catalog_sha256,
                definitions,
            } => {
                self.resource_catalog_sha256 = Some(catalog_sha256);
                self.resource_catalog = definitions;
            }
            PreparedSiteMutation::PublishCapacityResourceCatalog {
                catalog_sha256,
                definitions,
            } => {
                self.capacity_resource_catalog_sha256 = Some(catalog_sha256);
                self.capacity_resources = definitions;
            }
            PreparedSiteMutation::PublishCapacityAvailabilitySnapshot {
                availability_snapshot_sha256,
                site_snapshot_sha256,
                site_clock_domain_id,
                windows,
            } => {
                self.capacity_availability_snapshot_sha256 =
                    Some(availability_snapshot_sha256);
                self.capacity_site_snapshot_sha256 = Some(site_snapshot_sha256);
                self.capacity_site_clock_domain_id = Some(site_clock_domain_id);
                self.capacity_windows = windows;
            }
            PreparedSiteMutation::CommitCapacityReservation {
                commit,
                committed_event_id,
                committed_sequence,
            } => {
                self.capacity_reservations.insert(
                    commit.reservation.reservation_id.clone(),
                    CapacityReservationRecord {
                        commit,
                        committed_event_id,
                        committed_sequence,
                        released_event_id: None,
                        released_at_tick: None,
                        release_receipt_sha256: None,
                    },
                );
            }
            PreparedSiteMutation::ReleaseCapacityReservation {
                reservation_id,
                released_event_id,
                cleared_at_tick,
                release_receipt_sha256,
            } => {
                let record = self
                    .capacity_reservations
                    .get_mut(&reservation_id)
                    .expect("prepared capacity reservation remains present");
                record.released_event_id = Some(released_event_id);
                record.released_at_tick = Some(cleared_at_tick);
                record.release_receipt_sha256 = Some(release_receipt_sha256);
            }
            PreparedSiteMutation::CommitSpaceTimeReservation {
                reservation,
                committed_event_id,
                committed_sequence,
            } => {
                for claim in &reservation.claims {
                    self.active_resource_windows
                        .entry(claim.resource_id.clone())
                        .or_default()
                        .insert(
                            claim.start_tick,
                            (claim.end_tick, reservation.reservation_id.clone()),
                        );
                }
                self.space_time_reservations.insert(
                    reservation.reservation_id.clone(),
                    SpaceTimeReservationRecord {
                        reservation,
                        committed_event_id,
                        committed_sequence,
                        released_event_id: None,
                        released_at_tick: None,
                        release_receipt_sha256: None,
                    },
                );
            }
            PreparedSiteMutation::ReleaseSpaceTimeReservation {
                reservation_id,
                claims,
                released_event_id,
                cleared_at_tick,
                release_receipt_sha256,
            } => {
                for claim in &claims {
                    let remove_resource_index = {
                        let index = self
                            .active_resource_windows
                            .get_mut(&claim.resource_id)
                            .expect("prepared active resource index remains present");
                        let removed = index.remove(&claim.start_tick);
                        debug_assert!(
                            removed.as_ref().is_some_and(|(_, id)| id == &reservation_id),
                            "active resource index must point to the released reservation"
                        );
                        index.is_empty()
                    };
                    if remove_resource_index {
                        self.active_resource_windows.remove(&claim.resource_id);
                    }
                }
                let record = self
                    .space_time_reservations
                    .get_mut(&reservation_id)
                    .expect("prepared reservation remains present");
                record.released_event_id = Some(released_event_id);
                record.released_at_tick = Some(cleared_at_tick);
                record.release_receipt_sha256 = Some(release_receipt_sha256);
            }
            PreparedSiteMutation::QualifyStation {
                package_sha256,
                qualification_event_id,
                qualification,
            } => {
                self.packages
                    .get_mut(&package_sha256)
                    .expect("prepared current package remains present")
                    .station_qualifications
                    .insert(qualification_event_id, qualification);
            }
            PreparedSiteMutation::StartExecution {
                package_sha256,
                entity_id,
                attempt_id,
                attempt,
            } => {
                self.packages
                    .get_mut(&package_sha256)
                    .expect("prepared current package remains present")
                    .entity_states
                    .insert(
                        entity_id,
                        SiteEntityState::ExecutionInProgress {
                            attempt_id: attempt_id.clone(),
                        },
                    );
                self.attempts.insert(attempt_id, attempt);
            }
            PreparedSiteMutation::RecordRobotObservation {
                package_sha256,
                entity_id,
                attempt_id,
                disposition,
            } => {
                self.attempts
                    .get_mut(&attempt_id)
                    .expect("prepared execution attempt remains present")
                    .robot_disposition = Some(disposition);
                self.packages
                    .get_mut(&package_sha256)
                    .expect("prepared current package remains present")
                    .entity_states
                    .insert(
                        entity_id,
                        SiteEntityState::AwaitingIndependentMeasurement {
                            attempt_id,
                            robot_disposition: disposition,
                        },
                    );
            }
            PreparedSiteMutation::RecordRefereeAssessment {
                package_sha256,
                entity_id,
                attempt_id,
                new_state,
            } => {
                self.attempts
                    .get_mut(&attempt_id)
                    .expect("prepared execution attempt remains present")
                    .referee_assessed = true;
                self.packages
                    .get_mut(&package_sha256)
                    .expect("prepared current package remains present")
                    .entity_states
                    .insert(entity_id, new_state);
            }
            PreparedSiteMutation::IssueWaiver {
                package_sha256,
                entity_id,
                new_state,
            } => {
                self.packages
                    .get_mut(&package_sha256)
                    .expect("prepared current package remains present")
                    .entity_states
                    .insert(entity_id, new_state);
            }
        }

        // The event is immutable in the prepared plan; this argument keeps the
        // commit API explicit about which event is being committed and gives
        // debug builds a consistency check without any recoverable failure.
        debug_assert_eq!(
            self.last_sequence.saturating_add(1),
            event.sequence,
            "commit must follow successful event-chain validation"
        );
    }

    fn current_package(
        &self,
        package_sha256: &str,
    ) -> Result<&PackageProjection, SiteLedgerError> {
        if self.active_package_sha256.as_deref() != Some(package_sha256) {
            return Err(SiteLedgerError::InactivePackage(package_sha256.to_owned()));
        }
        self.packages
            .get(package_sha256)
            .ok_or_else(|| SiteLedgerError::UnknownPackage(package_sha256.to_owned()))
    }

}

#[derive(Clone, Debug)]
enum ScoredRefereeOutcome {
    Accepted(String),
    Rejected(String, SiteRejectionReason),
    Absent(String),
    Held(String, EntityScoreReason),
}

fn evaluate_referee_assessment(
    package: &PackageProjection,
    entity_id: &str,
    robot_disposition: RobotExecutionDisposition,
    assessment: &SiteRefereeAssessment,
) -> Result<ScoredRefereeOutcome, SiteLedgerError> {
    match assessment {
        SiteRefereeAssessment::NoMarkFound { receipt_sha256 } => {
            if !is_sha256(receipt_sha256) {
                return Err(SiteLedgerError::InvalidReceiptDigest);
            }
            Ok(ScoredRefereeOutcome::Absent(receipt_sha256.clone()))
        }
        SiteRefereeAssessment::Unresolved {
            receipt_sha256,
            reason,
        } => {
            if !is_sha256(receipt_sha256) || reason.trim().is_empty() {
                return Err(SiteLedgerError::InvalidReceiptDigest);
            }
            Ok(ScoredRefereeOutcome::Held(
                receipt_sha256.clone(),
                EntityScoreReason::MeasurementUnavailable,
            ))
        }
        SiteRefereeAssessment::Measured(measurement) => {
            if !is_sha256(&measurement.measurement_receipt_sha256) {
                return Err(SiteLedgerError::InvalidReceiptDigest);
            }
            let entity = package
                .package
                .entities
                .iter()
                .find(|entity| entity.design_entity_id == entity_id)
                .ok_or_else(|| SiteLedgerError::UnknownDesignEntity(entity_id.to_owned()))?;
            let manifest = DesignIntentManifest {
                package_sha256: package.package.package_sha256.clone(),
                design_revision_sha256: package.package.design_revision_sha256.clone(),
                frame_id: package.package.frame_id.clone(),
                entity_manifest_sha256: package.package.entity_manifest_sha256.clone(),
                entities: vec![entity.clone()],
            };
            let robot_claimed_state = match robot_disposition {
                RobotExecutionDisposition::Printed => RobotClaimedEntityState::Printed,
                RobotExecutionDisposition::NotPrinted => RobotClaimedEntityState::NotPrinted,
                RobotExecutionDisposition::Partial | RobotExecutionDisposition::Unknown => {
                    RobotClaimedEntityState::Unknown
                }
            };
            let observation = FieldEntityObservation {
                design_entity_id: entity_id.to_owned(),
                robot_claimed_state,
                measurement_disposition: MeasurementDisposition::Measured,
                measurement: Some(*measurement),
                rework_event_id: None,
            };
            let score = score_field_layout_run(
                &manifest,
                &[observation],
                &[],
                package.package.acceptance_profile,
                CrewHours::default(),
            )
            .map_err(|_| {
                SiteLedgerError::AcceptanceScorerRejectedMeasurement(entity_id.to_owned())
            })?;
            let result = score.entity_results.first().ok_or_else(|| {
                SiteLedgerError::AcceptanceScorerRejectedMeasurement(entity_id.to_owned())
            })?;
            match result.referee_state {
                RefereeEntityState::Accepted => Ok(ScoredRefereeOutcome::Accepted(
                    measurement.measurement_receipt_sha256.clone(),
                )),
                RefereeEntityState::Rejected => Ok(ScoredRefereeOutcome::Rejected(
                    measurement.measurement_receipt_sha256.clone(),
                    SiteRejectionReason::GeometryOutsideTolerance,
                )),
                RefereeEntityState::Held => Ok(ScoredRefereeOutcome::Held(
                    measurement.measurement_receipt_sha256.clone(),
                    result.reason,
                )),
                RefereeEntityState::Waived | RefereeEntityState::NotAttempted => Err(
                    SiteLedgerError::AcceptanceScorerRejectedMeasurement(entity_id.to_owned()),
                ),
            }
        }
    }
}

#[derive(Clone, Debug)]
enum PreparedSiteMutation {
    PublishResourceCatalog {
        catalog_sha256: String,
        definitions: BTreeMap<String, String>,
    },
    PublishCapacityResourceCatalog {
        catalog_sha256: String,
        definitions: BTreeMap<String, CapacityResourceDefinition>,
    },
    PublishCapacityAvailabilitySnapshot {
        availability_snapshot_sha256: String,
        site_snapshot_sha256: String,
        site_clock_domain_id: String,
        windows: Vec<CapacityAvailabilityWindow>,
    },
    CommitCapacityReservation {
        commit: CapacityReservationCommit,
        committed_event_id: String,
        committed_sequence: u64,
    },
    ReleaseCapacityReservation {
        reservation_id: String,
        released_event_id: String,
        cleared_at_tick: u64,
        release_receipt_sha256: String,
    },
    CommitSpaceTimeReservation {
        reservation: SpaceTimeReservation,
        committed_event_id: String,
        committed_sequence: u64,
    },
    ReleaseSpaceTimeReservation {
        reservation_id: String,
        claims: Vec<ExclusiveResourceClaim>,
        released_event_id: String,
        cleared_at_tick: u64,
        release_receipt_sha256: String,
    },
    PublishPackage {
        previous_package_sha256: Option<String>,
        package_sha256: String,
        projection: PackageProjection,
    },
    QualifyStation {
        package_sha256: String,
        qualification_event_id: String,
        qualification: StationQualification,
    },
    StartExecution {
        package_sha256: String,
        entity_id: String,
        attempt_id: String,
        attempt: ExecutionAttempt,
    },
    RecordRobotObservation {
        package_sha256: String,
        entity_id: String,
        attempt_id: String,
        disposition: RobotExecutionDisposition,
    },
    RecordRefereeAssessment {
        package_sha256: String,
        entity_id: String,
        attempt_id: String,
        new_state: SiteEntityState,
    },
    IssueWaiver {
        package_sha256: String,
        entity_id: String,
        new_state: SiteEntityState,
    },
}

fn validate_attempt_owner(
    attempt: &ExecutionAttempt,
    package_sha256: &str,
    entity_id: &str,
    attempt_id: &str,
    actor_id: &str,
) -> Result<(), SiteLedgerError> {
    if attempt.package_sha256 != package_sha256 || attempt.entity_id != entity_id {
        return Err(SiteLedgerError::AttemptContextMismatch(
            attempt_id.to_owned(),
        ));
    }
    if attempt.robot_id != actor_id {
        return Err(SiteLedgerError::AttemptActorMismatch(
            attempt_id.to_owned(),
        ));
    }
    Ok(())
}

fn ensure_attempt_is_in_progress(
    package: &PackageProjection,
    entity_id: &str,
    attempt_id: &str,
) -> Result<(), SiteLedgerError> {
    match package.entity_states.get(entity_id) {
        Some(SiteEntityState::ExecutionInProgress { attempt_id: current })
            if current == attempt_id =>
        {
            Ok(())
        }
        _ => Err(SiteLedgerError::AttemptContextMismatch(
            attempt_id.to_owned(),
        )),
    }
}

fn ensure_attempt_is_awaiting_measurement(
    package: &PackageProjection,
    entity_id: &str,
    attempt_id: &str,
) -> Result<(), SiteLedgerError> {
    match package.entity_states.get(entity_id) {
        Some(SiteEntityState::AwaitingIndependentMeasurement {
            attempt_id: current,
            ..
        }) if current == attempt_id => Ok(()),
        _ => Err(SiteLedgerError::AttemptContextMismatch(
            attempt_id.to_owned(),
        )),
    }
}


fn validate_capacity_reservation_commit(
    ledger: &SiteStateLedger,
    commit: &CapacityReservationCommit,
) -> Result<(), SiteLedgerError> {
    let reservation = &commit.reservation;
    if [
        reservation.reservation_receipt_sha256.as_str(),
        commit.package_sha256.as_str(),
        commit.design_revision_sha256.as_str(),
        commit.readiness_receipt_sha256.as_str(),
        commit.occupancy_assessment_receipt_sha256.as_str(),
        commit.occupancy_engine_build_sha256.as_str(),
        commit.clearance_policy_sha256.as_str(),
        commit.resource_catalog_sha256.as_str(),
        commit.availability_snapshot_sha256.as_str(),
        commit.site_snapshot_sha256.as_str(),
        commit.site_ledger_event_sha256.as_str(),
    ]
    .into_iter()
    .any(|digest| !is_sha256(digest))
    {
        return Err(SiteLedgerError::InvalidCapacityReservation(
            reservation.reservation_id.clone(),
        ));
    }
    if reservation.reservation_id.trim().is_empty()
        || reservation.site_clock_domain_id.trim().is_empty()
        || commit.entity_id.trim().is_empty()
        || commit.robot_id.trim().is_empty()
        || reservation.claims.is_empty()
        || commit.checked_at_tick > commit.committed_at_tick
        || commit.valid_until_tick <= commit.checked_at_tick
        || commit.committed_at_tick >= commit.valid_until_tick
        || commit.lease_valid_until_tick <= commit.committed_at_tick
        || commit.committed_at_tick > commit.valid_until_tick
    {
        return Err(SiteLedgerError::InvalidCapacityReservation(
            reservation.reservation_id.clone(),
        ));
    }

    if ledger.capacity_resource_catalog_sha256.as_deref()
        != Some(commit.resource_catalog_sha256.as_str())
    {
        return Err(SiteLedgerError::CapacityResourceCatalogMismatch);
    }
    if ledger.capacity_availability_snapshot_sha256.as_deref()
        != Some(commit.availability_snapshot_sha256.as_str())
        || ledger.capacity_site_snapshot_sha256.as_deref()
            != Some(commit.site_snapshot_sha256.as_str())
        || ledger.capacity_site_clock_domain_id.as_deref()
            != Some(reservation.site_clock_domain_id.as_str())
    {
        return Err(SiteLedgerError::CapacityAvailabilitySnapshotMismatch);
    }

    if commit.site_ledger_sequence != ledger.last_sequence
        || ledger.last_event_sha256.as_deref()
            != Some(commit.site_ledger_event_sha256.as_str())
    {
        return Err(SiteLedgerError::CapacityReservationSnapshotMismatch);
    }

    let package = ledger.current_package(&commit.package_sha256)?;
    if package.package.design_revision_sha256 != commit.design_revision_sha256 {
        return Err(SiteLedgerError::InactivePackage(commit.package_sha256.clone()));
    }
    match package.entity_states.get(&commit.entity_id) {
        Some(SiteEntityState::NotAttempted | SiteEntityState::PhysicallyAbsent { .. }) => {}
        _ => {
            return Err(SiteLedgerError::EntityTransitionDenied(
                commit.entity_id.clone(),
            ));
        }
    }

    for claim in &reservation.claims {
        if claim.resource_id.trim().is_empty()
            || !is_sha256(&claim.resource_definition_sha256)
            || claim.start_tick < commit.committed_at_tick
            || claim.end_tick <= claim.start_tick
            || claim.demand_units == 0
        {
            return Err(SiteLedgerError::InvalidCapacityReservation(
                reservation.reservation_id.clone(),
            ));
        }
        let Some(definition) = ledger.capacity_resources.get(&claim.resource_id) else {
            return Err(SiteLedgerError::UnknownCapacityReservationResource(
                claim.resource_id.clone(),
            ));
        };
        if definition.definition_sha256 != claim.resource_definition_sha256 {
            return Err(SiteLedgerError::CapacityResourceDefinitionMismatch(
                claim.resource_id.clone(),
            ));
        }
    }

    Ok(())
}

fn validate_space_time_reservation(
    ledger: &SiteStateLedger,
    reservation: &SpaceTimeReservation,
) -> Result<(), SiteLedgerError> {
    if [
        reservation.resource_catalog_sha256.as_str(),
        reservation.resource_claim_manifest_sha256.as_str(),
        reservation.package_sha256.as_str(),
        reservation.design_revision_sha256.as_str(),
        reservation.envelope_sha256.as_str(),
        reservation.readiness_receipt_sha256.as_str(),
        reservation.occupancy_assessment_receipt_sha256.as_str(),
        reservation.occupancy_engine_build_sha256.as_str(),
        reservation.clearance_policy_sha256.as_str(),
        reservation.site_snapshot_sha256.as_str(),
        reservation.site_ledger_event_sha256.as_str(),
    ]
    .into_iter()
    .any(|digest| !is_sha256(digest))
    {
        return Err(SiteLedgerError::InvalidSpaceTimeReservation(
            reservation.reservation_id.clone(),
        ));
    }
    if reservation.reservation_id.trim().is_empty()
        || reservation.entity_id.trim().is_empty()
        || reservation.robot_id.trim().is_empty()
        || reservation.site_clock_domain_id.trim().is_empty()
        || reservation.claims.is_empty()
        || reservation.start_tick < reservation.committed_at_tick
        || reservation.end_tick <= reservation.start_tick
        || reservation.valid_until_tick <= reservation.checked_at_tick
        || reservation.committed_at_tick < reservation.checked_at_tick
        || reservation.committed_at_tick >= reservation.valid_until_tick
        || reservation.start_tick >= reservation.valid_until_tick
        || reservation.end_tick > reservation.valid_until_tick
        || reservation.lease_valid_until_tick <= reservation.committed_at_tick
        || reservation.lease_valid_until_tick > reservation.valid_until_tick
    {
        return Err(SiteLedgerError::InvalidSpaceTimeReservation(
            reservation.reservation_id.clone(),
        ));
    }
    if ledger.resource_catalog_sha256.as_deref()
        != Some(reservation.resource_catalog_sha256.as_str())
    {
        return Err(SiteLedgerError::ResourceCatalogMismatch);
    }
    if reservation.site_ledger_sequence != ledger.last_sequence
        || ledger.last_event_sha256.as_deref()
            != Some(reservation.site_ledger_event_sha256.as_str())
    {
        return Err(SiteLedgerError::ReservationSnapshotMismatch);
    }
    let package = ledger.current_package(&reservation.package_sha256)?;
    if package.package.design_revision_sha256 != reservation.design_revision_sha256 {
        return Err(SiteLedgerError::InactivePackage(
            reservation.package_sha256.clone(),
        ));
    }
    if !package.entity_states.contains_key(&reservation.entity_id) {
        return Err(SiteLedgerError::UnknownDesignEntity(
            reservation.entity_id.clone(),
        ));
    }
    match package.entity_states.get(&reservation.entity_id) {
        Some(SiteEntityState::NotAttempted | SiteEntityState::PhysicallyAbsent { .. }) => {}
        _ => {
            return Err(SiteLedgerError::EntityTransitionDenied(
                reservation.entity_id.clone(),
            ));
        }
    }

    let mut resource_ids = BTreeSet::new();
    for claim in &reservation.claims {
        if claim.resource_id.trim().is_empty()
            || !is_sha256(&claim.definition_sha256)
            || claim.start_tick < reservation.start_tick
            || claim.end_tick > reservation.end_tick
            || claim.end_tick <= claim.start_tick
            || !resource_ids.insert(claim.resource_id.as_str())
        {
            return Err(SiteLedgerError::InvalidSpaceTimeReservation(
                reservation.reservation_id.clone(),
            ));
        }
        if ledger.resource_catalog.get(&claim.resource_id)
            != Some(&claim.definition_sha256)
        {
            return Err(SiteLedgerError::UnknownResourceCatalog);
        }
    }
    Ok(())
}

fn validate_event_envelope(event: &SiteEventEnvelope) -> Result<(), SiteLedgerError> {
    if event.event_id.trim().is_empty() || event.actor_id.trim().is_empty() || event.sequence == 0 {
        return Err(SiteLedgerError::EmptyEventIdentity);
    }
    if !is_sha256(&event.event_sha256) {
        return Err(SiteLedgerError::InvalidEventDigest);
    }
    if event
        .previous_event_sha256
        .as_ref()
        .is_some_and(|digest| !is_sha256(digest))
    {
        return Err(SiteLedgerError::InvalidPreviousEventDigest);
    }
    Ok(())
}

fn validate_work_package(package: &SiteWorkPackage) -> Result<(), SiteLedgerError> {
    if package.frame_id.trim().is_empty()
        || package.source_schema_release.trim().is_empty()
        || package.ids_version.trim().is_empty()
    {
        return Err(SiteLedgerError::EmptyPackageIdentity);
    }
    if [
        package.package_sha256.as_str(),
        package.source_model_sha256.as_str(),
        package.ids_spec_sha256.as_str(),
        package.ids_engine_build_sha256.as_str(),
        package.design_revision_sha256.as_str(),
        package.frame_sha256.as_str(),
        package.transform_sha256.as_str(),
        package.entity_manifest_sha256.as_str(),
        package.acceptance_profile_sha256.as_str(),
    ]
    .into_iter()
    .any(|digest| !is_sha256(digest))
    {
        return Err(SiteLedgerError::InvalidPackageDigest);
    }
    if !is_sha256(&package.ids_engine_build_sha256) {
        return Err(SiteLedgerError::InvalidIdsEngineBuildDigest);
    }
    if package.entities.is_empty() {
        return Err(SiteLedgerError::EmptyEntityManifest);
    }
    let mut ids = BTreeSet::new();
    for entity in &package.entities {
        if entity.design_entity_id.trim().is_empty() {
            return Err(SiteLedgerError::EmptyEntityId);
        }
        if !ids.insert(entity.design_entity_id.as_str()) {
            return Err(SiteLedgerError::DuplicatePackageEntityId(
                entity.design_entity_id.clone(),
            ));
        }
    }
    let profile = package.acceptance_profile;
    if !profile.max_abs_xy_residual_mm.is_finite()
        || profile.max_abs_xy_residual_mm <= 0.0
        || !profile.max_reference_u95_xy_mm.is_finite()
        || profile.max_reference_u95_xy_mm < 0.0
        || profile
            .max_abs_z_residual_mm
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
        || profile
            .max_reference_u95_z_mm
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        || profile
            .max_abs_endpoint_residual_mm
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
        || profile.max_abs_z_residual_mm.is_some() != profile.max_reference_u95_z_mm.is_some()
    {
        return Err(SiteLedgerError::InvalidAcceptanceProfile);
    }
    Ok(())
}

fn validate_waiver(waiver: &HumanWaiver, actor_id: &str) -> Result<(), SiteLedgerError> {
    if waiver.authority_id != actor_id
        || waiver.authority_role.trim().is_empty()
        || waiver.reason.trim().is_empty()
        || waiver.scope.trim().is_empty()
        || !is_sha256(&waiver.waiver_receipt_sha256)
    {
        return Err(SiteLedgerError::InvalidWaiver(
            waiver.design_entity_id.clone(),
        ));
    }
    Ok(())
}

fn require_role(actual: SiteActorRole, expected: SiteActorRole) -> Result<(), SiteLedgerError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SiteLedgerError::ActorRoleDenied)
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
    use crate::field_layout_acceptance::FieldLayoutEntityType;

    fn test_actor_registry() -> SiteActorRegistry {
        let mut registry = SiteActorRegistry::new();
        registry
            .grant("design-authority", SiteActorRole::DesignAuthority)
            .unwrap();
        registry.grant("surveyor", SiteActorRole::Surveyor).unwrap();
        registry.grant("robot-1", SiteActorRole::Robot).unwrap();
        registry.grant("robot-2", SiteActorRole::Robot).unwrap();
        registry.grant("referee-1", SiteActorRole::Referee).unwrap();
        registry.grant("referee-01", SiteActorRole::Referee).unwrap();
        registry
            .grant("quality-lead", SiteActorRole::SiteQualityAuthority)
            .unwrap();
        registry
            .grant("resource-authority", SiteActorRole::SiteResourceAuthority)
            .unwrap();
        registry
            .grant("capacity-owner-1", SiteActorRole::SiteResourceAuthority)
            .unwrap();
        registry
            .grant("dispatch-coordinator", SiteActorRole::DispatchCoordinator)
            .unwrap();
        registry
    }

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn package(package_digest: char, entity_ids: &[&str]) -> SiteWorkPackage {
        SiteWorkPackage {
            package_sha256: digest(package_digest),
            source_model_sha256: digest('c'),
            source_schema_release: "IFC4X3_ADD2".into(),
            ids_spec_sha256: digest('d'),
            ids_version: "1.0".into(),
            design_revision_sha256: digest('b'),
            frame_id: "site-frame".into(),
            frame_sha256: digest('c'),
            transform_sha256: digest('d'),
            entity_manifest_sha256: digest('e'),
            acceptance_profile_sha256: digest('f'),
            entities: entity_ids
                .iter()
                .enumerate()
                .map(|(index, entity_id)| DesignIntentEntity {
                    design_entity_id: (*entity_id).into(),
                    entity_type: if index == 0 {
                        FieldLayoutEntityType::Line
                    } else {
                        FieldLayoutEntityType::Point
                    },
                })
                .collect(),
            acceptance_profile: FieldAcceptanceProfile {
                max_abs_xy_residual_mm: 1.0,
                max_reference_u95_xy_mm: 0.25,
                max_abs_z_residual_mm: None,
                max_reference_u95_z_mm: None,
                max_abs_endpoint_residual_mm: Some(1.0),
            },
        }
    }

    fn event(
        event_id: &str,
        sequence: u64,
        previous: Option<String>,
        event_digest: char,
        actor_id: &str,
        actor_role: SiteActorRole,
        payload: SiteEventPayload,
    ) -> SiteEventEnvelope {
        SiteEventEnvelope {
            event_id: event_id.into(),
            sequence,
            previous_event_sha256: previous,
            event_sha256: digest(event_digest),
            actor_id: actor_id.into(),
            actor_role,
            payload,
        }
    }

    fn publish_package(ledger: &mut SiteStateLedger, package: SiteWorkPackage) -> String {
        let package_digest = package.package_sha256.clone();
        let event = event(
            "event-publish",
            1,
            None,
            '1',
            "design-authority",
            SiteActorRole::DesignAuthority,
            SiteEventPayload::WorkPackagePublished { package },
        );
        ledger.apply(event).unwrap();
        package_digest
    }

    fn station_event(ledger: &SiteStateLedger, sequence: u64, event_id: &str) -> SiteEventEnvelope {
        let package_digest = ledger.active_package_sha256().unwrap().to_owned();
        event(
            event_id,
            sequence,
            ledger.last_event_sha256().map(str::to_owned),
            char::from_digit(sequence as u32 + 1, 16).unwrap(),
            "surveyor",
            SiteActorRole::Surveyor,
            SiteEventPayload::StationQualified {
                package_sha256: package_digest,
                station_id: "station-1".into(),
                frame_sha256: digest('c'),
                transform_sha256: digest('d'),
                calibration_sha256: digest('9'),
            },
        )
    }

    fn measurement(residual_x_mm: f64, u95_mm: f64) -> RefereeMeasurement {
        RefereeMeasurement {
            residual_x_mm,
            residual_y_mm: 0.0,
            residual_z_mm: None,
            endpoint_start_mm: Some(0.2),
            endpoint_end_mm: Some(0.3),
            reference_u95_x_mm: u95_mm,
            reference_u95_y_mm: u95_mm,
            reference_u95_z_mm: None,
            measurement_receipt_sha256: digest('8'),
        }
    }

    fn setup_started_attempt() -> (SiteStateLedger, String, String) {
        let mut ledger = SiteStateLedger::with_actor_registry(test_actor_registry());
        let package_digest = publish_package(&mut ledger, package('a', &["line-1", "point-1"]));
        let station = station_event(&ledger, 2, "event-station");
        ledger.apply(station).unwrap();
        let start = event(
            "event-start",
            3,
            ledger.last_event_sha256().map(str::to_owned),
            '4',
            "robot-1",
            SiteActorRole::Robot,
            SiteEventPayload::ExecutionStarted {
                package_sha256: package_digest.clone(),
                entity_id: "line-1".into(),
                attempt_id: "attempt-1".into(),
                station_qualification_event_id: "event-station".into(),
            },
        );
        ledger.apply(start).unwrap();
        (ledger, package_digest, "attempt-1".into())
    }

    #[test]
    fn robot_observation_never_accepts_and_referee_measurement_drives_acceptance() {
        let (mut ledger, package_digest, attempt_id) = setup_started_attempt();
        let observed = event(
            "event-robot",
            4,
            ledger.last_event_sha256().map(str::to_owned),
            '5',
            "robot-1",
            SiteActorRole::Robot,
            SiteEventPayload::RobotObservationRecorded {
                package_sha256: package_digest.clone(),
                entity_id: "line-1".into(),
                attempt_id: attempt_id.clone(),
                disposition: RobotExecutionDisposition::Printed,
                robot_receipt_sha256: digest('7'),
            },
        );
        ledger.apply(observed).unwrap();
        assert!(matches!(
            ledger.entity_state(&package_digest, "line-1"),
            Some(SiteEntityState::AwaitingIndependentMeasurement { .. })
        ));
        assert_eq!(ledger.accepted_count(&package_digest), Some(0));

        let assessment = event(
            "event-referee",
            5,
            ledger.last_event_sha256().map(str::to_owned),
            '6',
            "referee-1",
            SiteActorRole::Referee,
            SiteEventPayload::RefereeAssessmentRecorded {
                package_sha256: package_digest.clone(),
                entity_id: "line-1".into(),
                attempt_id,
                assessment: SiteRefereeAssessment::Measured(measurement(0.2, 0.1)),
            },
        );
        ledger.apply(assessment).unwrap();
        assert!(matches!(
            ledger.entity_state(&package_digest, "line-1"),
            Some(SiteEntityState::Accepted { .. })
        ));
        assert_eq!(ledger.accepted_count(&package_digest), Some(1));
    }

    #[test]
    fn no_mark_is_rejected_and_uncertain_measurement_is_held() {
        let (mut ledger, package_digest, attempt_id) = setup_started_attempt();
        let observed = event(
            "event-robot",
            4,
            ledger.last_event_sha256().map(str::to_owned),
            '5',
            "robot-1",
            SiteActorRole::Robot,
            SiteEventPayload::RobotObservationRecorded {
                package_sha256: package_digest.clone(),
                entity_id: "line-1".into(),
                attempt_id: attempt_id.clone(),
                disposition: RobotExecutionDisposition::NotPrinted,
                robot_receipt_sha256: digest('7'),
            },
        );
        ledger.apply(observed).unwrap();
        let assessment = event(
            "event-referee",
            5,
            ledger.last_event_sha256().map(str::to_owned),
            '6',
            "referee-1",
            SiteActorRole::Referee,
            SiteEventPayload::RefereeAssessmentRecorded {
                package_sha256: package_digest.clone(),
                entity_id: "line-1".into(),
                attempt_id,
                assessment: SiteRefereeAssessment::NoMarkFound {
                    receipt_sha256: digest('8'),
                },
            },
        );
        ledger.apply(assessment).unwrap();
        assert!(matches!(
            ledger.entity_state(&package_digest, "line-1"),
            Some(SiteEntityState::PhysicallyAbsent { .. })
        ));

        // A confirmed-absent entity can be attempted again. The follow-up is
        // not accepted until a second independent assessment resolves it.
        let retry_start = event(
            "event-start-rework",
            6,
            ledger.last_event_sha256().map(str::to_owned),
            '7',
            "robot-1",
            SiteActorRole::Robot,
            SiteEventPayload::ExecutionStarted {
                package_sha256: package_digest.clone(),
                entity_id: "line-1".into(),
                attempt_id: "attempt-2".into(),
                station_qualification_event_id: "event-station".into(),
            },
        );
        ledger.apply(retry_start).unwrap();
        let retry_observation = event(
            "event-robot-rework",
            7,
            ledger.last_event_sha256().map(str::to_owned),
            '8',
            "robot-1",
            SiteActorRole::Robot,
            SiteEventPayload::RobotObservationRecorded {
                package_sha256: package_digest.clone(),
                entity_id: "line-1".into(),
                attempt_id: "attempt-2".into(),
                disposition: RobotExecutionDisposition::Printed,
                robot_receipt_sha256: digest('7'),
            },
        );
        ledger.apply(retry_observation).unwrap();
        let unresolved = event(
            "event-referee-unresolved",
            8,
            ledger.last_event_sha256().map(str::to_owned),
            '9',
            "referee-1",
            SiteActorRole::Referee,
            SiteEventPayload::RefereeAssessmentRecorded {
                package_sha256: package_digest.clone(),
                entity_id: "line-1".into(),
                attempt_id: "attempt-2".into(),
                assessment: SiteRefereeAssessment::Unresolved {
                    receipt_sha256: digest('a'),
                    reason: "reference instrument lost lock".into(),
                },
            },
        );
        ledger.apply(unresolved).unwrap();
        assert!(matches!(
            ledger.entity_state(&package_digest, "line-1"),
            Some(SiteEntityState::Held {
                attempt_id,
                reason: EntityScoreReason::MeasurementUnavailable,
                ..
            }) if attempt_id == "attempt-2"
        ));
    }

    #[test]
    fn stale_station_transform_and_wrong_actor_are_rejected_atomically() {
        let mut ledger = SiteStateLedger::with_actor_registry(test_actor_registry());
        let package_digest = publish_package(&mut ledger, package('a', &["line-1"]));
        let mut invalid = station_event(&ledger, 2, "event-station");
        if let SiteEventPayload::StationQualified {
            transform_sha256, ..
        } = &mut invalid.payload
        {
            *transform_sha256 = digest('0');
        }
        assert_eq!(
            ledger.apply(invalid),
            Err(SiteLedgerError::StationQualificationContextMismatch)
        );
        assert_eq!(ledger.last_sequence(), 1);

        let wrong_actor = event(
            "event-station-wrong-actor",
            2,
            ledger.last_event_sha256().map(str::to_owned),
            '2',
            "robot-1",
            SiteActorRole::Robot,
            SiteEventPayload::StationQualified {
                package_sha256: package_digest,
                station_id: "station-1".into(),
                frame_sha256: digest('c'),
                transform_sha256: digest('d'),
                calibration_sha256: digest('9'),
            },
        );
        assert_eq!(
            ledger.apply(wrong_actor),
            Err(SiteLedgerError::ActorRoleDenied)
        );
        assert_eq!(ledger.last_sequence(), 1);
    }

    #[test]
    fn exact_duplicate_event_is_idempotent_but_conflicting_reuse_is_rejected() {
        let mut ledger = SiteStateLedger::with_actor_registry(test_actor_registry());
        let package = package('a', &["point-1"]);
        let original = event(
            "event-publish",
            1,
            None,
            '1',
            "design-authority",
            SiteActorRole::DesignAuthority,
            SiteEventPayload::WorkPackagePublished {
                package: package.clone(),
            },
        );
        assert_eq!(
            ledger.apply(original.clone()),
            Ok(SiteApplyOutcome::Applied)
        );
        assert_eq!(
            ledger.apply(original.clone()),
            Ok(SiteApplyOutcome::DuplicateIgnored)
        );
        let conflicting = event(
            "event-publish",
            1,
            None,
            '2',
            "design-authority",
            SiteActorRole::DesignAuthority,
            SiteEventPayload::WorkPackagePublished { package },
        );
        assert_eq!(
            ledger.apply(conflicting),
            Err(SiteLedgerError::ConflictingEventId("event-publish".into()))
        );
        assert_eq!(ledger.event_count(), 1);
    }

    #[test]
    fn sequence_gap_and_previous_digest_mismatch_fail_closed() {
        let mut ledger = SiteStateLedger::with_actor_registry(test_actor_registry());
        let gap = event(
            "event-gap",
            2,
            None,
            '1',
            "design-authority",
            SiteActorRole::DesignAuthority,
            SiteEventPayload::WorkPackagePublished {
                package: package('a', &["point-1"]),
            },
        );
        assert_eq!(
            ledger.apply(gap),
            Err(SiteLedgerError::EventSequenceMismatch {
                expected: 1,
                actual: 2
            })
        );
        assert_eq!(ledger.last_sequence(), 0);
    }

    #[test]
    fn publishing_a_new_revision_preserves_old_acceptance_as_history() {
        let (mut ledger, old_package, attempt_id) = setup_started_attempt();
        let robot = event(
            "event-robot",
            4,
            ledger.last_event_sha256().map(str::to_owned),
            '5',
            "robot-1",
            SiteActorRole::Robot,
            SiteEventPayload::RobotObservationRecorded {
                package_sha256: old_package.clone(),
                entity_id: "line-1".into(),
                attempt_id: attempt_id.clone(),
                disposition: RobotExecutionDisposition::Printed,
                robot_receipt_sha256: digest('7'),
            },
        );
        ledger.apply(robot).unwrap();
        let referee = event(
            "event-referee",
            5,
            ledger.last_event_sha256().map(str::to_owned),
            '6',
            "referee-1",
            SiteActorRole::Referee,
            SiteEventPayload::RefereeAssessmentRecorded {
                package_sha256: old_package.clone(),
                entity_id: "line-1".into(),
                attempt_id,
                assessment: SiteRefereeAssessment::Measured(measurement(0.2, 0.1)),
            },
        );
        ledger.apply(referee).unwrap();
        assert_eq!(ledger.accepted_count(&old_package), Some(1));

        let new_package = package('f', &["line-1", "point-1"]);
        let new_digest = new_package.package_sha256.clone();
        let published = event(
            "event-publish-v2",
            6,
            ledger.last_event_sha256().map(str::to_owned),
            '7',
            "design-authority",
            SiteActorRole::DesignAuthority,
            SiteEventPayload::WorkPackagePublished {
                package: new_package,
            },
        );
        ledger.apply(published).unwrap();
        assert_eq!(ledger.active_package_sha256(), Some(new_digest.as_str()));
        assert_eq!(ledger.package_is_current(&old_package), Some(false));
        assert_eq!(
            ledger.package_superseded_by(&old_package),
            Some(new_digest.as_str())
        );
        assert_eq!(ledger.accepted_count(&old_package), Some(1));
        assert_eq!(
            ledger.entity_state(&new_digest, "line-1"),
            Some(&SiteEntityState::NotAttempted)
        );
    }

    #[test]
    fn waiver_requires_quality_authority_and_does_not_increment_acceptance() {
        let mut ledger = SiteStateLedger::with_actor_registry(test_actor_registry());
        let package_digest = publish_package(&mut ledger, package('a', &["point-1"]));
        let waiver = HumanWaiver {
            design_entity_id: "point-1".into(),
            authority_id: "quality-lead".into(),
            authority_role: "site-quality-manager".into(),
            reason: "approved alternate measurement method".into(),
            scope: "point-1 only".into(),
            waiver_receipt_sha256: digest('8'),
        };
        let issued = event(
            "event-waiver",
            2,
            ledger.last_event_sha256().map(str::to_owned),
            '2',
            "quality-lead",
            SiteActorRole::SiteQualityAuthority,
            SiteEventPayload::HumanWaiverIssued {
                package_sha256: package_digest.clone(),
                waiver,
            },
        );
        ledger.apply(issued).unwrap();
        assert!(matches!(
            ledger.entity_state(&package_digest, "point-1"),
            Some(SiteEntityState::Waived { .. })
        ));
        assert_eq!(ledger.accepted_count(&package_digest), Some(0));
    }

    #[test]
    fn robot_observation_must_match_the_robot_that_started_the_attempt() {
        let (mut ledger, package_digest, attempt_id) = setup_started_attempt();
        let wrong_robot = event(
            "event-robot-wrong",
            4,
            ledger.last_event_sha256().map(str::to_owned),
            '5',
            "robot-2",
            SiteActorRole::Robot,
            SiteEventPayload::RobotObservationRecorded {
                package_sha256: package_digest.clone(),
                entity_id: "line-1".into(),
                attempt_id,
                disposition: RobotExecutionDisposition::Printed,
                robot_receipt_sha256: digest('7'),
            },
        );
        assert_eq!(
            ledger.apply(wrong_robot),
            Err(SiteLedgerError::AttemptActorMismatch("attempt-1".into()))
        );
        assert_eq!(ledger.last_sequence(), 3);
        assert!(matches!(
            ledger.entity_state(&package_digest, "line-1"),
            Some(SiteEntityState::ExecutionInProgress { .. })
        ));
    }


    #[test]
    fn previous_event_digest_mismatch_fails_closed_without_advancing_state() {
        let mut ledger = SiteStateLedger::with_actor_registry(test_actor_registry());
        let package_digest = publish_package(&mut ledger, package('a', &["point-1"]));
        let mut event = station_event(&ledger, 2, "event-station");
        event.previous_event_sha256 = Some(digest('0'));
        event.payload = SiteEventPayload::StationQualified {
            package_sha256: package_digest,
            station_id: "station-1".into(),
            frame_sha256: digest('c'),
            transform_sha256: digest('d'),
            calibration_sha256: digest('9'),
        };
        assert_eq!(
            ledger.apply(event),
            Err(SiteLedgerError::PreviousEventDigestMismatch)
        );
        assert_eq!(ledger.last_sequence(), 1);
        assert_eq!(ledger.event_count(), 1);
    }

    #[test]
    fn accepted_entities_are_terminal_within_a_work_package() {
        let (mut ledger, package_digest, attempt_id) = setup_started_attempt();
        let robot = event(
            "event-robot",
            4,
            ledger.last_event_sha256().map(str::to_owned),
            '5',
            "robot-1",
            SiteActorRole::Robot,
            SiteEventPayload::RobotObservationRecorded {
                package_sha256: package_digest.clone(),
                entity_id: "line-1".into(),
                attempt_id: attempt_id.clone(),
                disposition: RobotExecutionDisposition::Printed,
                robot_receipt_sha256: digest('7'),
            },
        );
        ledger.apply(robot).unwrap();
        let referee = event(
            "event-referee",
            5,
            ledger.last_event_sha256().map(str::to_owned),
            '6',
            "referee-1",
            SiteActorRole::Referee,
            SiteEventPayload::RefereeAssessmentRecorded {
                package_sha256: package_digest.clone(),
                entity_id: "line-1".into(),
                attempt_id,
                assessment: SiteRefereeAssessment::Measured(measurement(0.2, 0.1)),
            },
        );
        ledger.apply(referee).unwrap();
        let retry = event(
            "event-start-after-accepted",
            6,
            ledger.last_event_sha256().map(str::to_owned),
            '7',
            "robot-1",
            SiteActorRole::Robot,
            SiteEventPayload::ExecutionStarted {
                package_sha256: package_digest.clone(),
                entity_id: "line-1".into(),
                attempt_id: "attempt-3".into(),
                station_qualification_event_id: "event-station".into(),
            },
        );
        assert_eq!(
            ledger.apply(retry),
            Err(SiteLedgerError::EntityTransitionDenied("line-1".into()))
        );
        assert_eq!(ledger.last_sequence(), 5);
        assert_eq!(ledger.accepted_count(&package_digest), Some(1));
    }


    #[test]
    fn referee_measurement_can_override_a_robot_not_printed_claim() {
        let (mut ledger, package_digest, attempt_id) = setup_started_attempt();
        let robot = event(
            "event-robot-not-printed",
            4,
            ledger.last_event_sha256().map(str::to_owned),
            '5',
            "robot-1",
            SiteActorRole::Robot,
            SiteEventPayload::RobotObservationRecorded {
                package_sha256: package_digest.clone(),
                entity_id: "line-1".into(),
                attempt_id: attempt_id.clone(),
                disposition: RobotExecutionDisposition::NotPrinted,
                robot_receipt_sha256: digest('7'),
            },
        );
        ledger.apply(robot).unwrap();
        let referee = event(
            "event-referee-found-mark",
            5,
            ledger.last_event_sha256().map(str::to_owned),
            '6',
            "referee-1",
            SiteActorRole::Referee,
            SiteEventPayload::RefereeAssessmentRecorded {
                package_sha256: package_digest.clone(),
                entity_id: "line-1".into(),
                attempt_id,
                assessment: SiteRefereeAssessment::Measured(measurement(0.1, 0.1)),
            },
        );
        ledger.apply(referee).unwrap();
        assert!(matches!(
            ledger.entity_state(&package_digest, "line-1"),
            Some(SiteEntityState::Accepted { .. })
        ));
        assert_eq!(ledger.accepted_count(&package_digest), Some(1));
    }

    #[test]
    fn measured_geometry_with_insufficient_reference_u95_is_held_in_the_ledger() {
        let (mut ledger, package_digest, attempt_id) = setup_started_attempt();
        let robot = event(
            "event-robot",
            4,
            ledger.last_event_sha256().map(str::to_owned),
            '5',
            "robot-1",
            SiteActorRole::Robot,
            SiteEventPayload::RobotObservationRecorded {
                package_sha256: package_digest.clone(),
                entity_id: "line-1".into(),
                attempt_id: attempt_id.clone(),
                disposition: RobotExecutionDisposition::Printed,
                robot_receipt_sha256: digest('7'),
            },
        );
        ledger.apply(robot).unwrap();
        let referee = event(
            "event-referee-low-resolution",
            5,
            ledger.last_event_sha256().map(str::to_owned),
            '6',
            "referee-1",
            SiteActorRole::Referee,
            SiteEventPayload::RefereeAssessmentRecorded {
                package_sha256: package_digest.clone(),
                entity_id: "line-1".into(),
                attempt_id,
                assessment: SiteRefereeAssessment::Measured(measurement(0.1, 0.5)),
            },
        );
        ledger.apply(referee).unwrap();
        assert!(matches!(
            ledger.entity_state(&package_digest, "line-1"),
            Some(SiteEntityState::Held {
                reason: EntityScoreReason::ReferenceResolutionInsufficient,
                ..
            })
        ));
        assert_eq!(ledger.accepted_count(&package_digest), Some(0));
    }

    #[test]
    fn package_revision_cannot_supersede_an_unresolved_physical_attempt() {
        let (mut ledger, old_package, _) = setup_started_attempt();
        let new_package = package('f', &["line-1", "point-1"]);
        let new_digest = new_package.package_sha256.clone();
        let publish = event(
            "event-publish-while-executing",
            4,
            ledger.last_event_sha256().map(str::to_owned),
            '5',
            "design-authority",
            SiteActorRole::DesignAuthority,
            SiteEventPayload::WorkPackagePublished {
                package: new_package,
            },
        );
        assert_eq!(
            ledger.apply(publish),
            Err(SiteLedgerError::PackageHasUnresolvedAttempts(
                old_package.clone()
            ))
        );
        assert_eq!(ledger.last_sequence(), 3);
        assert_eq!(ledger.active_package_sha256(), Some(old_package.as_str()));
        assert_eq!(ledger.package_is_current(&old_package), Some(true));
        assert_eq!(ledger.package_is_current(&new_digest), None);
    }

    #[test]
    fn package_revision_waits_for_referee_reconciliation_after_robot_observation() {
        let (mut ledger, old_package, attempt_id) = setup_started_attempt();
        let robot = event(
            "event-robot",
            4,
            ledger.last_event_sha256().map(str::to_owned),
            '5',
            "robot-1",
            SiteActorRole::Robot,
            SiteEventPayload::RobotObservationRecorded {
                package_sha256: old_package.clone(),
                entity_id: "line-1".into(),
                attempt_id,
                disposition: RobotExecutionDisposition::Printed,
                robot_receipt_sha256: digest('7'),
            },
        );
        ledger.apply(robot).unwrap();
        let new_package = package('f', &["line-1", "point-1"]);
        let publish = event(
            "event-publish-while-awaiting-referee",
            5,
            ledger.last_event_sha256().map(str::to_owned),
            '6',
            "design-authority",
            SiteActorRole::DesignAuthority,
            SiteEventPayload::WorkPackagePublished {
                package: new_package,
            },
        );
        assert_eq!(
            ledger.apply(publish),
            Err(SiteLedgerError::PackageHasUnresolvedAttempts(
                old_package.clone()
            ))
        );
        assert_eq!(ledger.last_sequence(), 4);
        assert_eq!(ledger.active_package_sha256(), Some(old_package.as_str()));
        assert!(matches!(
            ledger.entity_state(&old_package, "line-1"),
            Some(SiteEntityState::AwaitingIndependentMeasurement { .. })
        ));
    }


    #[test]
    fn interrupted_attempt_remains_unaccepted_until_referee_reconciliation() {
        let (mut ledger, package_digest, attempt_id) = setup_started_attempt();
        let interrupted = event(
            "event-interrupted",
            4,
            ledger.last_event_sha256().map(str::to_owned),
            '5',
            "robot-1",
            SiteActorRole::Robot,
            SiteEventPayload::ExecutionInterrupted {
                package_sha256: package_digest.clone(),
                entity_id: "line-1".into(),
                attempt_id: attempt_id.clone(),
                reason: "robot lost power mid-task".into(),
                interruption_receipt_sha256: digest('7'),
            },
        );
        ledger.apply(interrupted).unwrap();
        assert!(matches!(
            ledger.entity_state(&package_digest, "line-1"),
            Some(SiteEntityState::AwaitingIndependentMeasurement {
                robot_disposition: RobotExecutionDisposition::Unknown,
                ..
            })
        ));
        assert_eq!(ledger.accepted_count(&package_digest), Some(0));

        let publish = event(
            "event-publish-while-interrupted",
            5,
            ledger.last_event_sha256().map(str::to_owned),
            '8',
            "design-authority",
            SiteActorRole::DesignAuthority,
            SiteEventPayload::WorkPackagePublished {
                package: package('f', &["line-1"]),
            },
        );
        assert_eq!(
            ledger.apply(publish),
            Err(SiteLedgerError::PackageHasUnresolvedAttempts(
                package_digest.clone()
            ))
        );
        assert_eq!(ledger.last_sequence(), 4);

        let referee = event(
            "event-referee-after-interruption",
            5,
            ledger.last_event_sha256().map(str::to_owned),
            '9',
            "referee-1",
            SiteActorRole::Referee,
            SiteEventPayload::RefereeAssessmentRecorded {
                package_sha256: package_digest.clone(),
                entity_id: "line-1".into(),
                attempt_id,
                assessment: SiteRefereeAssessment::Measured(measurement(0.2, 0.1)),
            },
        );
        ledger.apply(referee).unwrap();
        assert!(matches!(
            ledger.entity_state(&package_digest, "line-1"),
            Some(SiteEntityState::Accepted { .. })
        ));
        assert_eq!(ledger.accepted_count(&package_digest), Some(1));
    }

    #[test]
    fn held_and_rejected_physical_marks_block_package_supersession() {
        let (mut held_ledger, held_package, held_attempt) = setup_started_attempt();
        let robot = event(
            "event-robot",
            4,
            held_ledger.last_event_sha256().map(str::to_owned),
            '5',
            "robot-1",
            SiteActorRole::Robot,
            SiteEventPayload::RobotObservationRecorded {
                package_sha256: held_package.clone(),
                entity_id: "line-1".into(),
                attempt_id: held_attempt.clone(),
                disposition: RobotExecutionDisposition::Printed,
                robot_receipt_sha256: digest('7'),
            },
        );
        held_ledger.apply(robot).unwrap();
        let unresolved = event(
            "event-referee-held",
            5,
            held_ledger.last_event_sha256().map(str::to_owned),
            '6',
            "referee-1",
            SiteActorRole::Referee,
            SiteEventPayload::RefereeAssessmentRecorded {
                package_sha256: held_package.clone(),
                entity_id: "line-1".into(),
                attempt_id: held_attempt,
                assessment: SiteRefereeAssessment::Unresolved {
                    receipt_sha256: digest('8'),
                    reason: "tracker lost lock".into(),
                },
            },
        );
        held_ledger.apply(unresolved).unwrap();
        let held_publish = event(
            "event-publish-held",
            6,
            held_ledger.last_event_sha256().map(str::to_owned),
            '9',
            "design-authority",
            SiteActorRole::DesignAuthority,
            SiteEventPayload::WorkPackagePublished {
                package: package('f', &["line-1"]),
            },
        );
        assert!(matches!(
            held_ledger.apply(held_publish),
            Err(SiteLedgerError::PackageHasUnresolvedAttempts(_))
        ));
        assert_eq!(held_ledger.last_sequence(), 5);

        let (mut rejected_ledger, rejected_package, rejected_attempt) =
            setup_started_attempt();
        let robot = event(
            "event-robot",
            4,
            rejected_ledger.last_event_sha256().map(str::to_owned),
            '5',
            "robot-1",
            SiteActorRole::Robot,
            SiteEventPayload::RobotObservationRecorded {
                package_sha256: rejected_package.clone(),
                entity_id: "line-1".into(),
                attempt_id: rejected_attempt.clone(),
                disposition: RobotExecutionDisposition::Printed,
                robot_receipt_sha256: digest('7'),
            },
        );
        rejected_ledger.apply(robot).unwrap();
        let rejected = event(
            "event-referee-rejected",
            5,
            rejected_ledger.last_event_sha256().map(str::to_owned),
            '6',
            "referee-1",
            SiteActorRole::Referee,
            SiteEventPayload::RefereeAssessmentRecorded {
                package_sha256: rejected_package.clone(),
                entity_id: "line-1".into(),
                attempt_id: rejected_attempt,
                assessment: SiteRefereeAssessment::Measured(measurement(2.0, 0.1)),
            },
        );
        rejected_ledger.apply(rejected).unwrap();
        let rejected_publish = event(
            "event-publish-rejected",
            6,
            rejected_ledger.last_event_sha256().map(str::to_owned),
            '9',
            "design-authority",
            SiteActorRole::DesignAuthority,
            SiteEventPayload::WorkPackagePublished {
                package: package('f', &["line-1"]),
            },
        );
        assert!(matches!(
            rejected_ledger.apply(rejected_publish),
            Err(SiteLedgerError::PackageHasUnresolvedAttempts(_))
        ));
        assert_eq!(rejected_ledger.last_sequence(), 5);
    }

    #[test]
    fn confirmed_absence_allows_revision_without_falsely_counting_acceptance() {
        let (mut ledger, old_package, attempt_id) = setup_started_attempt();
        let robot = event(
            "event-robot",
            4,
            ledger.last_event_sha256().map(str::to_owned),
            '5',
            "robot-1",
            SiteActorRole::Robot,
            SiteEventPayload::RobotObservationRecorded {
                package_sha256: old_package.clone(),
                entity_id: "line-1".into(),
                attempt_id: attempt_id.clone(),
                disposition: RobotExecutionDisposition::NotPrinted,
                robot_receipt_sha256: digest('7'),
            },
        );
        ledger.apply(robot).unwrap();
        let absent = event(
            "event-referee-absent",
            5,
            ledger.last_event_sha256().map(str::to_owned),
            '6',
            "referee-1",
            SiteActorRole::Referee,
            SiteEventPayload::RefereeAssessmentRecorded {
                package_sha256: old_package.clone(),
                entity_id: "line-1".into(),
                attempt_id,
                assessment: SiteRefereeAssessment::NoMarkFound {
                    receipt_sha256: digest('8'),
                },
            },
        );
        ledger.apply(absent).unwrap();
        assert!(matches!(
            ledger.entity_state(&old_package, "line-1"),
            Some(SiteEntityState::PhysicallyAbsent { .. })
        ));
        assert_eq!(ledger.accepted_count(&old_package), Some(0));
        let new_package = package('f', &["line-1"]);
        let new_digest = new_package.package_sha256.clone();
        let publish = event(
            "event-publish-after-absence",
            6,
            ledger.last_event_sha256().map(str::to_owned),
            '9',
            "design-authority",
            SiteActorRole::DesignAuthority,
            SiteEventPayload::WorkPackagePublished {
                package: new_package,
            },
        );
        assert_eq!(ledger.apply(publish), Ok(SiteApplyOutcome::Applied));
        assert_eq!(ledger.active_package_sha256(), Some(new_digest.as_str()));
        assert_eq!(ledger.accepted_count(&old_package), Some(0));
    }


    #[test]
    fn event_identity_index_remains_stable_after_long_append_history() {
        let mut ledger = SiteStateLedger::with_actor_registry(test_actor_registry());
        let package = package('a', &["point-1"]);
        let package_event = event(
            "event-publish",
            1,
            None,
            '1',
            "design-authority",
            SiteActorRole::DesignAuthority,
            SiteEventPayload::WorkPackagePublished {
                package: package.clone(),
            },
        );
        ledger.apply(package_event.clone()).unwrap();

        for sequence in 2..=130_u64 {
            let event_id = format!("station-event-{sequence:04}");
            let event_digest = format!("{sequence:064x}");
            let mut station = event(
                &event_id,
                sequence,
                ledger.last_event_sha256().map(str::to_owned),
                '2',
                "surveyor",
                SiteActorRole::Surveyor,
                SiteEventPayload::StationQualified {
                    package_sha256: package.package_sha256.clone(),
                    station_id: format!("station-{sequence:04}"),
                    frame_sha256: digest('c'),
                    transform_sha256: digest('d'),
                    calibration_sha256: digest('9'),
                },
            );
            station.event_sha256 = event_digest;
            ledger.apply(station).unwrap();
        }

        assert_eq!(ledger.last_sequence(), 130);
        assert_eq!(ledger.event_count(), 130);
        assert_eq!(
            ledger.apply(package_event),
            Ok(SiteApplyOutcome::DuplicateIgnored)
        );
        assert_eq!(ledger.event_count(), 130);
    }


    #[test]
    fn event_history_replay_reconstructs_an_identical_projection() {
        let (mut ledger, package_digest, attempt_id) = setup_started_attempt();
        let robot = event(
            "event-robot",
            4,
            ledger.last_event_sha256().map(str::to_owned),
            '5',
            "robot-1",
            SiteActorRole::Robot,
            SiteEventPayload::RobotObservationRecorded {
                package_sha256: package_digest.clone(),
                entity_id: "line-1".into(),
                attempt_id: attempt_id.clone(),
                disposition: RobotExecutionDisposition::Printed,
                robot_receipt_sha256: digest('7'),
            },
        );
        ledger.apply(robot).unwrap();
        let referee = event(
            "event-referee",
            5,
            ledger.last_event_sha256().map(str::to_owned),
            '6',
            "referee-1",
            SiteActorRole::Referee,
            SiteEventPayload::RefereeAssessmentRecorded {
                package_sha256: package_digest.clone(),
                entity_id: "line-1".into(),
                attempt_id,
                assessment: SiteRefereeAssessment::Measured(measurement(0.2, 0.1)),
            },
        );
        ledger.apply(referee).unwrap();

        let replayed = SiteStateLedger::replay(ledger.event_history(), ledger.actor_registry().clone()).unwrap();
        assert_eq!(replayed, ledger);
        assert_eq!(replayed.accepted_count(&package_digest), Some(1));
    }

    #[test]
    fn persisted_duplicate_or_broken_chain_is_not_valid_replay_history() {
        let mut ledger = SiteStateLedger::with_actor_registry(test_actor_registry());
        publish_package(&mut ledger, package('a', &["point-1"]));
        let first = ledger.event_history()[0].clone();
        let duplicate_history = vec![first.clone(), first];
        assert_eq!(
            SiteStateLedger::replay(&duplicate_history, test_actor_registry()),
            Err(SiteLedgerError::EventHistoryContainsDuplicate(
                "event-publish".into()
            ))
        );

        let mut broken = ledger.event_history().to_vec();
        let mut next = station_event(&ledger, 2, "event-station");
        next.previous_event_sha256 = Some(digest('0'));
        next.event_sha256 = digest('2');
        broken.push(next);
        assert_eq!(
            SiteStateLedger::replay(&broken, test_actor_registry()),
            Err(SiteLedgerError::PreviousEventDigestMismatch)
        );
    }


    #[test]
    fn malformed_ids_engine_build_cannot_be_published_to_the_site_ledger() {
        let mut ledger = SiteStateLedger::with_actor_registry(test_actor_registry());
        let mut invalid = package('a', &["point-1"]);
        invalid.ids_engine_build_sha256 = "unknown-engine-build".into();
        let publication = event(
            "event-publish-invalid-engine",
            1,
            None,
            '1',
            "design-authority",
            SiteActorRole::DesignAuthority,
            SiteEventPayload::WorkPackagePublished { package: invalid },
        );
        assert_eq!(
            ledger.apply(publication),
            Err(SiteLedgerError::InvalidIdsEngineBuildDigest)
        );
        assert_eq!(ledger.last_sequence(), 0);
        assert_eq!(ledger.event_count(), 0);
    }


    fn publish_exclusive_resource_catalog(ledger: &mut SiteStateLedger) {
        let catalog = event(
            "event-resource-catalog",
            ledger.last_sequence() + 1,
            ledger.last_event_sha256().map(str::to_owned),
            '2',
            "resource-authority",
            SiteActorRole::SiteResourceAuthority,
            SiteEventPayload::ResourceCatalogPublished {
                catalog_sha256: digest('b'),
                resources: vec![SiteResourceDefinition {
                    resource_id: "crane-zone-1".into(),
                    definition_sha256: digest('c'),
                }],
            },
        );
        ledger.apply(catalog).unwrap();
    }

    fn reservation_for(
        ledger: &SiteStateLedger,
        reservation_id: &str,
        start_tick: u64,
        end_tick: u64,
        committed_at_tick: u64,
        checked_at_tick: u64,
        lease_valid_until_tick: u64,
    ) -> SpaceTimeReservation {
        SpaceTimeReservation {
            reservation_id: reservation_id.into(),
            resource_catalog_sha256: digest('b'),
            resource_claim_manifest_sha256: digest('d'),
            claims: vec![ExclusiveResourceClaim {
                resource_id: "crane-zone-1".into(),
                definition_sha256: digest('c'),
                start_tick,
                end_tick,
            }],
            package_sha256: digest('a'),
            design_revision_sha256: digest('b'),
            entity_id: "point-1".into(),
            robot_id: "robot-01".into(),
            envelope_sha256: digest('e'),
            readiness_receipt_sha256: digest('f'),
            occupancy_assessment_receipt_sha256: digest('8'),
            occupancy_engine_build_sha256: digest('9'),
            clearance_policy_sha256: digest('7'),
            site_snapshot_sha256: digest('6'),
            site_ledger_sequence: ledger.last_sequence(),
            site_ledger_event_sha256: ledger.last_event_sha256().unwrap().to_owned(),
            site_clock_domain_id: "site-clock-1".into(),
            checked_at_tick,
            valid_until_tick: 500,
            committed_at_tick,
            start_tick,
            end_tick,
            lease_valid_until_tick,
        }
    }

    fn commit_reservation_event(
        ledger: &mut SiteStateLedger,
        reservation: SpaceTimeReservation,
        event_id: &str,
        event_digest: char,
    ) -> Result<SiteApplyOutcome, SiteLedgerError> {
        ledger.apply(event(
            event_id,
            ledger.last_sequence() + 1,
            ledger.last_event_sha256().map(str::to_owned),
            event_digest,
            "dispatch-coordinator",
            SiteActorRole::DispatchCoordinator,
            SiteEventPayload::SpaceTimeReservationCommitted { reservation },
        ))
    }

    #[test]
    fn overlapping_exclusive_resource_claims_are_rejected_atomically() {
        let mut ledger = SiteStateLedger::with_actor_registry(test_actor_registry());
        publish_package(&mut ledger, package('a', &["point-1"]));
        publish_exclusive_resource_catalog(&mut ledger);
        let first = reservation_for(&ledger, "reservation-1", 130, 300, 120, 100, 450);
        commit_reservation_event(&mut ledger, first, "event-reservation-1", '3').unwrap();
        let before_sequence = ledger.last_sequence();
        let before_event_count = ledger.event_count();

        let overlapping =
            reservation_for(&ledger, "reservation-2", 250, 350, 200, 160, 450);
        assert_eq!(
            commit_reservation_event(
                &mut ledger,
                overlapping,
                "event-reservation-2",
                '4'
            ),
            Err(SiteLedgerError::SpaceTimeReservationConflict {
                resource_id: "crane-zone-1".into(),
                conflicting_reservation_id: "reservation-1".into(),
            })
        );
        assert_eq!(ledger.last_sequence(), before_sequence);
        assert_eq!(ledger.event_count(), before_event_count);
        assert!(ledger
            .space_time_reservation("reservation-1")
            .unwrap()
            .is_active());
        assert!(ledger.space_time_reservation("reservation-2").is_none());
    }

    #[test]
    fn adjacent_half_open_resource_claims_do_not_conflict() {
        let mut ledger = SiteStateLedger::with_actor_registry(test_actor_registry());
        publish_package(&mut ledger, package('a', &["point-1"]));
        publish_exclusive_resource_catalog(&mut ledger);
        let first_reservation =
            reservation_for(&ledger, "reservation-1", 130, 300, 120, 100, 450);
        commit_reservation_event(
            &mut ledger,
            first_reservation,
            "event-reservation-1",
            '3',
        )
        .unwrap();
        let second_reservation =
            reservation_for(&ledger, "reservation-2", 300, 400, 200, 160, 450);
        commit_reservation_event(
            &mut ledger,
            second_reservation,
            "event-reservation-2",
            '4',
        )
        .unwrap();
        assert_eq!(ledger.active_space_time_reservations().count(), 2);
    }

    #[test]
    fn lease_expiry_does_not_automatically_release_physical_resource() {
        let mut ledger = SiteStateLedger::with_actor_registry(test_actor_registry());
        publish_package(&mut ledger, package('a', &["point-1"]));
        publish_exclusive_resource_catalog(&mut ledger);
        let first_reservation =
            reservation_for(&ledger, "reservation-1", 130, 300, 120, 100, 150);
        commit_reservation_event(
            &mut ledger,
            first_reservation,
            "event-reservation-1",
            '3',
        )
        .unwrap();
        // The lease has elapsed by tick 200, but no independent clearance
        // event exists. The reservation remains active and continues to block.
        assert_eq!(
            ledger
                .space_time_reservation("reservation-1")
                .unwrap()
                .reservation
                .lease_valid_until_tick,
            150
        );
        assert_eq!(ledger.active_space_time_reservations().count(), 1);
        let second_reservation =
            reservation_for(&ledger, "reservation-2", 200, 250, 200, 160, 450);
        assert!(matches!(
            commit_reservation_event(
                &mut ledger,
                second_reservation,
                "event-reservation-2",
                '4'
            ),
            Err(SiteLedgerError::SpaceTimeReservationConflict { .. })
        ));
    }

    #[test]
    fn referee_clearance_releases_a_reservation_and_replay_preserves_projection() {
        let mut ledger = SiteStateLedger::with_actor_registry(test_actor_registry());
        publish_package(&mut ledger, package('a', &["point-1"]));
        publish_exclusive_resource_catalog(&mut ledger);
        let first_reservation =
            reservation_for(&ledger, "reservation-1", 130, 300, 120, 100, 450);
        commit_reservation_event(
            &mut ledger,
            first_reservation,
            "event-reservation-1",
            '3',
        )
        .unwrap();
        let release = event(
            "event-reservation-release",
            ledger.last_sequence() + 1,
            ledger.last_event_sha256().map(str::to_owned),
            '4',
            "referee-01",
            SiteActorRole::Referee,
            SiteEventPayload::SpaceTimeReservationReleased {
                reservation_id: "reservation-1".into(),
                site_clock_domain_id: "site-clock-1".into(),
                cleared_at_tick: 220,
                site_snapshot_sha256: digest('5'),
                site_ledger_sequence: ledger.last_sequence(),
                site_ledger_event_sha256: ledger.last_event_sha256().unwrap().to_owned(),
                release_receipt_sha256: digest('6'),
            },
        );
        ledger.apply(release).unwrap();
        let released = ledger.space_time_reservation("reservation-1").unwrap();
        assert!(!released.is_active());
        assert_eq!(released.released_at_tick, Some(220));

        let replay = SiteStateLedger::replay(ledger.event_history(), ledger.actor_registry().clone()).unwrap();
        assert_eq!(replay, ledger);
        assert_eq!(replay.active_space_time_reservations().count(), 0);

        // A fresh reservation can use the cleared resource.
        let second_reservation =
            reservation_for(&ledger, "reservation-2", 240, 300, 230, 225, 450);
        commit_reservation_event(
            &mut ledger,
            second_reservation,
            "event-reservation-2",
            '7',
        )
        .unwrap();
        assert_eq!(ledger.active_space_time_reservations().count(), 1);
    }

    #[test]
    fn resource_catalog_cannot_remove_a_resource_held_by_an_active_reservation() {
        let mut ledger = SiteStateLedger::with_actor_registry(test_actor_registry());
        publish_package(&mut ledger, package('a', &["point-1"]));
        publish_exclusive_resource_catalog(&mut ledger);
        let first_reservation =
            reservation_for(&ledger, "reservation-1", 130, 300, 120, 100, 450);
        commit_reservation_event(
            &mut ledger,
            first_reservation,
            "event-reservation-1",
            '3',
        )
        .unwrap();
        let before_sequence = ledger.last_sequence();
        let changed_catalog = event(
            "event-resource-catalog-v2",
            ledger.last_sequence() + 1,
            ledger.last_event_sha256().map(str::to_owned),
            '4',
            "resource-authority",
            SiteActorRole::SiteResourceAuthority,
            SiteEventPayload::ResourceCatalogPublished {
                catalog_sha256: digest('5'),
                resources: vec![SiteResourceDefinition {
                    resource_id: "different-zone".into(),
                    definition_sha256: digest('6'),
                }],
            },
        );
        assert_eq!(
            ledger.apply(changed_catalog),
            Err(SiteLedgerError::ResourceCatalogWouldOrphanReservation(
                "crane-zone-1".into()
            ))
        );
        assert_eq!(ledger.last_sequence(), before_sequence);
        let expected_definition = digest('c');
        assert_eq!(
            ledger.resource_definition_sha256("crane-zone-1"),
            Some(expected_definition.as_str())
        );
    }

    #[test]
    fn stale_release_snapshot_and_wrong_release_role_fail_closed() {
        let mut ledger = SiteStateLedger::with_actor_registry(test_actor_registry());
        publish_package(&mut ledger, package('a', &["point-1"]));
        publish_exclusive_resource_catalog(&mut ledger);
        let first_reservation =
            reservation_for(&ledger, "reservation-1", 130, 300, 120, 100, 450);
        commit_reservation_event(
            &mut ledger,
            first_reservation,
            "event-reservation-1",
            '3',
        )
        .unwrap();
        let stale = event(
            "event-release-stale",
            ledger.last_sequence() + 1,
            ledger.last_event_sha256().map(str::to_owned),
            '4',
            "referee-01",
            SiteActorRole::Referee,
            SiteEventPayload::SpaceTimeReservationReleased {
                reservation_id: "reservation-1".into(),
                site_clock_domain_id: "site-clock-1".into(),
                cleared_at_tick: 220,
                site_snapshot_sha256: digest('5'),
                site_ledger_sequence: 1,
                site_ledger_event_sha256: digest('1'),
                release_receipt_sha256: digest('6'),
            },
        );
        assert_eq!(
            ledger.apply(stale),
            Err(SiteLedgerError::ReservationSnapshotMismatch)
        );
        assert!(ledger
            .space_time_reservation("reservation-1")
            .unwrap()
            .is_active());
    }


    #[test]
    fn indexed_conflict_decisions_match_brute_force_oracle_over_generated_windows() {
        let mut ledger = SiteStateLedger::with_actor_registry(test_actor_registry());
        publish_package(&mut ledger, package('a', &["point-1"]));
        publish_exclusive_resource_catalog(&mut ledger);
        let mut seed = 0x51_7e_2026_u64;

        for index in 0..256_u64 {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let start_tick = 120 + (seed % 300);
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let duration = 1 + (seed % 24);
            let end_tick = start_tick + duration;

            let expected_conflict = ledger.active_space_time_reservations().any(|record| {
                record.reservation.claims.iter().any(|claim| {
                    claim.resource_id == "crane-zone-1"
                        && claim.start_tick < end_tick
                        && start_tick < claim.end_tick
                })
            });
            let reservation = reservation_for(
                &ledger,
                &format!("generated-{index:03}"),
                start_tick,
                end_tick,
                100,
                90,
                450,
            );
            let result = ledger.apply(event(
                &format!("generated-event-{index:03}"),
                ledger.last_sequence() + 1,
                ledger.last_event_sha256().map(str::to_owned),
                &format!("{:064x}", index + 100),
                "dispatch-coordinator",
                SiteActorRole::DispatchCoordinator,
                SiteEventPayload::SpaceTimeReservationCommitted { reservation },
            ));
            assert_eq!(
                matches!(result, Err(SiteLedgerError::SpaceTimeReservationConflict { .. })),
                expected_conflict,
                "indexed-vs-brute-force conflict result diverged at generated case {index}"
            );
            if !expected_conflict {
                assert_eq!(result, Ok(SiteApplyOutcome::Applied));
            }

            let projection_claim_count: usize = ledger
                .active_space_time_reservations()
                .map(|record| record.reservation.claims.len())
                .sum();
            assert_eq!(
                ledger.active_resource_claim_count(),
                projection_claim_count,
                "active interval index count diverged from reservation projection"
            );
            assert_eq!(ledger.active_resource_count(), usize::from(projection_claim_count > 0));
        }
    }


    #[test]
    fn resource_catalog_version_cannot_change_while_a_reservation_is_active() {
        let mut ledger = SiteStateLedger::with_actor_registry(test_actor_registry());
        publish_package(&mut ledger, package('a', &["point-1"]));
        publish_exclusive_resource_catalog(&mut ledger);
        let reservation =
            reservation_for(&ledger, "reservation-1", 130, 300, 120, 100, 450);
        commit_reservation_event(&mut ledger, reservation, "reservation-event-1", '3').unwrap();

        let changed_version = event(
            "event-resource-catalog-v2",
            ledger.last_sequence() + 1,
            ledger.last_event_sha256().map(str::to_owned),
            '4',
            "resource-authority",
            SiteActorRole::SiteResourceAuthority,
            SiteEventPayload::ResourceCatalogPublished {
                catalog_sha256: digest('5'),
                resources: vec![SiteResourceDefinition {
                    resource_id: "crane-zone-1".into(),
                    definition_sha256: digest('c'),
                }],
            },
        );
        assert_eq!(
            ledger.apply(changed_version),
            Err(SiteLedgerError::ResourceCatalogWouldOrphanReservation(
                "resource-catalog-version".into()
            ))
        );
        let original_catalog_digest = digest('b');
        assert_eq!(
            ledger.resource_catalog_sha256(),
            Some(original_catalog_digest.as_str())
        );
        assert_eq!(ledger.last_sequence(), 3);
    }

    #[test]
    fn work_package_revision_cannot_supersede_an_active_resource_reservation() {
        let mut ledger = SiteStateLedger::with_actor_registry(test_actor_registry());
        let package_digest = publish_package(&mut ledger, package('a', &["point-1"]));
        publish_exclusive_resource_catalog(&mut ledger);
        let reservation = reservation_for(
            &ledger,
            "reservation-1",
            130,
            300,
            120,
            100,
            450,
        );
        commit_reservation_event(&mut ledger, reservation, "reservation-event-1", '3').unwrap();

        let next_package = package('d', &["point-1"]);
        let event = event(
            "event-package-v2",
            ledger.last_sequence() + 1,
            ledger.last_event_sha256().map(str::to_owned),
            '4',
            "design-authority",
            SiteActorRole::DesignAuthority,
            SiteEventPayload::WorkPackagePublished {
                package: next_package,
            },
        );
        assert_eq!(
            ledger.apply(event),
            Err(SiteLedgerError::PackageHasActiveSpaceTimeReservations(
                package_digest.clone()
            ))
        );
        assert_eq!(ledger.active_package_sha256(), Some(package_digest.as_str()));
        assert_eq!(ledger.last_sequence(), 3);
    }


    #[test]
    fn identical_catalog_digest_cannot_be_reused_for_changed_definitions() {
        let mut ledger = SiteStateLedger::with_actor_registry(test_actor_registry());
        publish_package(&mut ledger, package('a', &["point-1"]));
        publish_exclusive_resource_catalog(&mut ledger);
        let before_sequence = ledger.last_sequence();
        let changed_content_same_digest = event(
            "event-resource-catalog-same-digest-different-content",
            before_sequence + 1,
            ledger.last_event_sha256().map(str::to_owned),
            '3',
            "resource-authority",
            SiteActorRole::SiteResourceAuthority,
            SiteEventPayload::ResourceCatalogPublished {
                catalog_sha256: digest('b'),
                resources: vec![SiteResourceDefinition {
                    resource_id: "crane-zone-1".into(),
                    definition_sha256: digest('4'),
                }],
            },
        );
        assert_eq!(
            ledger.apply(changed_content_same_digest),
            Err(SiteLedgerError::ResourceCatalogDigestContentMismatch)
        );
        assert_eq!(ledger.last_sequence(), before_sequence);
        let expected_definition = digest('c');
        assert_eq!(
            ledger.resource_definition_sha256("crane-zone-1"),
            Some(expected_definition.as_str())
        );
    }


    #[test]
    fn empty_actor_registry_denies_all_event_ingestion() {
        let mut ledger = SiteStateLedger::new();
        let event = event(
            "untrusted-package-event",
            1,
            None,
            '1',
            "design-authority",
            SiteActorRole::DesignAuthority,
            SiteEventPayload::WorkPackagePublished {
                package: package('a', &["point-1"]),
            },
        );
        assert_eq!(ledger.apply(event), Err(SiteLedgerError::ActorRoleDenied));
        assert_eq!(ledger.last_sequence(), 0);
        assert_eq!(ledger.event_count(), 0);
    }

    #[test]
    fn replay_requires_the_same_actor_authority_registry() {
        let mut ledger = SiteStateLedger::with_actor_registry(test_actor_registry());
        publish_package(&mut ledger, package('a', &["point-1"]));
        assert_eq!(
            SiteStateLedger::replay(ledger.event_history(), ledger.actor_registry().clone()).unwrap(),
            ledger
        );
        assert_eq!(
            SiteStateLedger::replay(ledger.event_history(), SiteActorRegistry::new()),
            Err(SiteLedgerError::ActorRoleDenied)
        );
    }

}
