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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteActorRole {
    DesignAuthority,
    Surveyor,
    Robot,
    Referee,
    SiteQualityAuthority,
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
    pub design_revision_sha256: String,
    pub frame_id: String,
    pub frame_sha256: String,
    pub transform_sha256: String,
    pub entity_manifest_sha256: String,
    pub acceptance_profile_sha256: String,
    pub entities: Vec<DesignIntentEntity>,
    pub acceptance_profile: FieldAcceptanceProfile,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SiteEventPayload {
    WorkPackagePublished {
        package: SiteWorkPackage,
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
    ActorRoleDenied,
    EmptyPackageIdentity,
    InvalidPackageDigest,
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteApplyOutcome {
    Applied,
    DuplicateIgnored,
}

#[derive(Clone, Debug)]
struct StationQualification {
    package_sha256: String,
    station_id: String,
    calibration_sha256: String,
}

#[derive(Clone, Debug)]
struct ExecutionAttempt {
    package_sha256: String,
    entity_id: String,
    robot_id: String,
    robot_disposition: Option<RobotExecutionDisposition>,
    referee_assessed: bool,
}

#[derive(Clone, Debug)]
struct PackageProjection {
    package: SiteWorkPackage,
    superseded_by: Option<String>,
    entity_states: BTreeMap<String, SiteEntityState>,
    station_qualifications: BTreeMap<String, StationQualification>,
}

#[derive(Clone, Debug, Default)]
pub struct SiteStateLedger {
    last_sequence: u64,
    last_event_sha256: Option<String>,
    events: Vec<SiteEventEnvelope>,
    events_by_id: BTreeMap<String, usize>,
    event_ids_by_digest: BTreeMap<String, String>,
    packages: BTreeMap<String, PackageProjection>,
    active_package_sha256: Option<String>,
    attempts: BTreeMap<String, ExecutionAttempt>,
}

impl SiteStateLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a caller-preverified event atomically.
    ///
    /// An exact duplicate envelope is idempotently ignored. A reused event ID
    /// with changed content, a sequence gap, or a broken previous-digest link
    /// is rejected. Any payload/state-transition failure leaves the ledger
    /// unchanged.
    pub fn apply(&mut self, event: SiteEventEnvelope) -> Result<SiteApplyOutcome, SiteLedgerError> {
        validate_event_envelope(&event)?;

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
                validate_attempt_owner(attempt, package_sha256, entity_id, &event.actor_id)?;
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
                validate_attempt_owner(attempt, package_sha256, entity_id, &event.actor_id)?;
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
    actor_id: &str,
) -> Result<(), SiteLedgerError> {
    if attempt.package_sha256 != package_sha256 || attempt.entity_id != entity_id {
        return Err(SiteLedgerError::AttemptContextMismatch(
            attempt.entity_id.clone(),
        ));
    }
    if attempt.robot_id != actor_id {
        return Err(SiteLedgerError::AttemptActorMismatch(entity_id.to_owned()));
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
        let mut ledger = SiteStateLedger::new();
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
        let mut ledger = SiteStateLedger::new();
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
        let mut ledger = SiteStateLedger::new();
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
        let mut ledger = SiteStateLedger::new();
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
        let mut ledger = SiteStateLedger::new();
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
        let mut ledger = SiteStateLedger::new();
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
        let mut ledger = SiteStateLedger::new();
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
            let station = event(
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
            let mut station = station;
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

}
