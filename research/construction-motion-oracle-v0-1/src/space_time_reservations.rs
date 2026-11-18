//! Evidence-qualified space-time resource reservations for construction sites.
//!
//! The coordinator adds a resource/traffic layer above robot transport APIs.
//! It does not compute swept geometry: an independent conflict evaluator must
//! assess the candidate envelope against the current reservation snapshot.
//! A grant is a scheduling commitment, not a safety permit or actuation command.
//!
//! Critical invariant: lease expiry never implies physical clearance. A robot
//! may be stopped, disconnected, delayed or still occupying a zone. Expired
//! reservations therefore remain blocking until an independent assessor
//! supplies a receipt that the physical work area is clear.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReservationRequest {
    pub reservation_id: String,
    pub request_sha256: String,
    pub package_sha256: String,
    pub attempt_id: String,
    pub robot_id: String,
    pub site_epoch: String,
    pub time_domain_id: String,
    pub frame_sha256: String,
    pub transform_sha256: String,
    /// Digest of the conservative robot/tool/payload space-time envelope.
    pub occupancy_envelope_sha256: String,
    /// Half-open interval [start_tick, end_tick) on the named site clock.
    /// These are logical scheduling ticks, not wall-clock timestamps.
    pub start_tick: u64,
    pub end_tick: u64,
    /// Exclusive resource claims: workfaces, lift corridors, crane cells,
    /// laydown areas, access routes or other site-defined coordination zones.
    pub resource_ids: Vec<String>,
    /// Revision of the reservation snapshot used by the independent evaluator.
    pub expected_book_revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConflictAssessmentOutcome {
    Clear,
    Conflicts { reservation_ids: Vec<String> },
    Unresolved { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictAssessment {
    pub reservation_id: String,
    pub request_sha256: String,
    pub package_sha256: String,
    pub frame_sha256: String,
    pub transform_sha256: String,
    pub occupancy_envelope_sha256: String,
    pub site_epoch: String,
    pub time_domain_id: String,
    pub start_tick: u64,
    pub end_tick: u64,
    pub reservation_book_revision: u64,
    /// Exact conflict-engine build that evaluated this snapshot.
    pub evaluator_build_sha256: String,
    /// Digest of canonical evaluator input and output bytes, verified by caller.
    pub assessment_receipt_sha256: String,
    pub outcome: ConflictAssessmentOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReservationStatus {
    Granted {
        fencing_token: u64,
        authorization_until_tick: u64,
    },
    Executing {
        fencing_token: u64,
        authorization_until_tick: u64,
    },
    /// Authorization has expired. The physical workspace is still treated as
    /// occupied until an independent clearance receipt is accepted.
    ExpiredUnreconciled {
        fencing_token: u64,
        execution_started: bool,
    },
    Cleared {
        fencing_token: u64,
        clearance_receipt_sha256: String,
        cleared_at_tick: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReservationSnapshot {
    pub reservation_id: String,
    pub request: ReservationRequest,
    pub fencing_token: u64,
    pub status: ReservationStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReservationGrant {
    pub reservation_id: String,
    pub robot_id: String,
    pub fencing_token: u64,
    pub authorization_until_tick: u64,
    pub reservation_book_revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClearanceAssessorRole {
    IndependentReferee,
    SiteSafetyAuthority,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClearanceOutcome {
    PhysicallyClear,
    Unresolved { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalClearanceAssessment {
    pub reservation_id: String,
    pub occupancy_envelope_sha256: String,
    pub frame_sha256: String,
    pub transform_sha256: String,
    pub assessor_id: String,
    pub assessor_role: ClearanceAssessorRole,
    pub clearance_receipt_sha256: String,
    pub observed_at_tick: u64,
    pub reservation_book_revision: u64,
    pub outcome: ClearanceOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReservationCoordinatorError {
    EmptyCoordinatorIdentity,
    InvalidDigest,
    EmptyReservationIdentity,
    DuplicateReservation(String),
    InvalidTimeInterval,
    InvalidResourceId,
    DuplicateResourceId(String),
    ReservationContextMismatch,
    StaleReservationSnapshot { expected: u64, actual: u64 },
    ConflictAssessmentUnresolved(String),
    ConflictDetected(Vec<String>),
    ResourceReservationConflict {
        requested_reservation_id: String,
        existing_reservation_id: String,
        resource_id: String,
    },
    LeaseDurationZero,
    LeaseTimeOverflow,
    FencingTokenExhausted,
    BookRevisionExhausted,
    UnknownReservation(String),
    RobotIdentityMismatch,
    FencingTokenMismatch,
    ReservationNotGranted,
    ReservationNotExecuting,
    AuthorizationExpired,
    OutsideReservedTimeWindow,
    ClearanceAssessorDenied,
    ClearanceUnresolved(String),
    ClearanceContextMismatch,
    ClearanceAlreadyRecorded,
}

#[derive(Clone, Debug)]
struct ReservationRecord {
    request: ReservationRequest,
    fencing_token: u64,
    status: ReservationStatus,
}

/// Single-writer deterministic reservation book.
///
/// book_revision is a monotonic compare-and-swap version for conflict
/// assessments. Any grant, execution transition, lease expiry, renewal or
/// physical clearance increments it. Assessments computed against older
/// revisions cannot grant work.
#[derive(Clone, Debug)]
pub struct SpaceTimeReservationCoordinator {
    site_epoch: String,
    time_domain_id: String,
    book_revision: u64,
    next_fencing_token: u64,
    reservations: BTreeMap<String, ReservationRecord>,
}

impl SpaceTimeReservationCoordinator {
    pub fn new(
        site_epoch: impl Into<String>,
        time_domain_id: impl Into<String>,
    ) -> Result<Self, ReservationCoordinatorError> {
        let site_epoch = site_epoch.into();
        let time_domain_id = time_domain_id.into();
        if site_epoch.trim().is_empty() || time_domain_id.trim().is_empty() {
            return Err(ReservationCoordinatorError::EmptyCoordinatorIdentity);
        }
        Ok(Self {
            site_epoch,
            time_domain_id,
            book_revision: 0,
            next_fencing_token: 1,
            reservations: BTreeMap::new(),
        })
    }

    pub fn book_revision(&self) -> u64 {
        self.book_revision
    }

    pub fn reservation(&self, reservation_id: &str) -> Option<ReservationSnapshot> {
        self.reservations
            .get(reservation_id)
            .map(|record| ReservationSnapshot {
                reservation_id: reservation_id.to_owned(),
                request: record.request.clone(),
                fencing_token: record.fencing_token,
                status: record.status.clone(),
            })
    }

    pub fn snapshots(&self) -> Vec<ReservationSnapshot> {
        self.reservations
            .iter()
            .map(|(reservation_id, record)| ReservationSnapshot {
                reservation_id: reservation_id.clone(),
                request: record.request.clone(),
                fencing_token: record.fencing_token,
                status: record.status.clone(),
            })
            .collect()
    }

    /// Grant only when the conflict assessment is bound to the exact request
    /// and current reservation-book revision.
    ///
    /// A grant is a scheduling commitment. The resource-lock check is an
    /// additional guard; geometry-level conflict checking remains external.
    pub fn grant(
        &mut self,
        request: ReservationRequest,
        assessment: ConflictAssessment,
        now_tick: u64,
        lease_duration_ticks: u64,
    ) -> Result<ReservationGrant, ReservationCoordinatorError> {
        validate_request(&request)?;

        // Expiry is itself a state change. If this changes the book, the
        // caller's assessment becomes stale and must be recomputed.
        self.expire_leases(now_tick)?;

        if self.reservations.contains_key(&request.reservation_id) {
            return Err(ReservationCoordinatorError::DuplicateReservation(
                request.reservation_id,
            ));
        }
        if request.site_epoch != self.site_epoch
            || request.time_domain_id != self.time_domain_id
        {
            return Err(ReservationCoordinatorError::ReservationContextMismatch);
        }
        if request.start_tick < now_tick || request.end_tick <= now_tick {
            return Err(ReservationCoordinatorError::InvalidTimeInterval);
        }
        if request.expected_book_revision != self.book_revision {
            return Err(ReservationCoordinatorError::StaleReservationSnapshot {
                expected: self.book_revision,
                actual: request.expected_book_revision,
            });
        }
        validate_conflict_assessment(&request, &assessment, self.book_revision)?;
        match &assessment.outcome {
            ConflictAssessmentOutcome::Clear => {}
            ConflictAssessmentOutcome::Conflicts { reservation_ids } => {
                return Err(ReservationCoordinatorError::ConflictDetected(
                    reservation_ids.clone(),
                ));
            }
            ConflictAssessmentOutcome::Unresolved { reason } => {
                return Err(ReservationCoordinatorError::ConflictAssessmentUnresolved(
                    reason.clone(),
                ));
            }
        }
        if lease_duration_ticks == 0 {
            return Err(ReservationCoordinatorError::LeaseDurationZero);
        }
        let authorization_until_tick = now_tick
            .checked_add(lease_duration_ticks)
            .ok_or(ReservationCoordinatorError::LeaseTimeOverflow)?;

        self.check_resource_conflicts(&request)?;

        let fencing_token = self.next_fencing_token;
        let next_fencing_token = self
            .next_fencing_token
            .checked_add(1)
            .ok_or(ReservationCoordinatorError::FencingTokenExhausted)?;
        self.bump_revision()?;
        self.next_fencing_token = next_fencing_token;
        self.reservations.insert(
            request.reservation_id.clone(),
            ReservationRecord {
                request: request.clone(),
                fencing_token,
                status: ReservationStatus::Granted {
                    fencing_token,
                    authorization_until_tick,
                },
            },
        );

        Ok(ReservationGrant {
            reservation_id: request.reservation_id,
            robot_id: request.robot_id,
            fencing_token,
            authorization_until_tick,
            reservation_book_revision: self.book_revision,
        })
    }

    /// Bind execution to the exact robot and fencing token. This is a
    /// coordination record, not a safety-rated actuation permit.
    pub fn begin_execution(
        &mut self,
        reservation_id: &str,
        robot_id: &str,
        fencing_token: u64,
        now_tick: u64,
    ) -> Result<u64, ReservationCoordinatorError> {
        let authorization_until_tick = {
            let record = self.reservations.get(reservation_id).ok_or_else(|| {
                ReservationCoordinatorError::UnknownReservation(reservation_id.to_owned())
            })?;
            if record.request.robot_id != robot_id {
                return Err(ReservationCoordinatorError::RobotIdentityMismatch);
            }
            if record.fencing_token != fencing_token {
                return Err(ReservationCoordinatorError::FencingTokenMismatch);
            }
            let until = match &record.status {
                ReservationStatus::Granted {
                    fencing_token: token,
                    authorization_until_tick,
                } if *token == fencing_token => *authorization_until_tick,
                _ => return Err(ReservationCoordinatorError::ReservationNotGranted),
            };
            if now_tick >= until {
                return Err(ReservationCoordinatorError::AuthorizationExpired);
            }
            if now_tick < record.request.start_tick || now_tick >= record.request.end_tick {
                return Err(ReservationCoordinatorError::OutsideReservedTimeWindow);
            }
            until
        };
        self.bump_revision()?;
        let record = self.reservations.get_mut(reservation_id).ok_or_else(|| {
            ReservationCoordinatorError::UnknownReservation(reservation_id.to_owned())
        })?;
        record.status = ReservationStatus::Executing {
            fencing_token,
            authorization_until_tick,
        };
        Ok(self.book_revision)
    }

    /// Renew command-authority lease without releasing the space-time claim.
    pub fn renew_lease(
        &mut self,
        reservation_id: &str,
        robot_id: &str,
        fencing_token: u64,
        now_tick: u64,
        extension_ticks: u64,
    ) -> Result<u64, ReservationCoordinatorError> {
        if extension_ticks == 0 {
            return Err(ReservationCoordinatorError::LeaseDurationZero);
        }
        let (old_until, executing) = {
            let record = self.reservations.get(reservation_id).ok_or_else(|| {
                ReservationCoordinatorError::UnknownReservation(reservation_id.to_owned())
            })?;
            if record.request.robot_id != robot_id {
                return Err(ReservationCoordinatorError::RobotIdentityMismatch);
            }
            if record.fencing_token != fencing_token {
                return Err(ReservationCoordinatorError::FencingTokenMismatch);
            }
            let (until, executing) = match &record.status {
                ReservationStatus::Granted {
                    fencing_token: token,
                    authorization_until_tick,
                } if *token == fencing_token => (*authorization_until_tick, false),
                ReservationStatus::Executing {
                    fencing_token: token,
                    authorization_until_tick,
                } if *token == fencing_token => (*authorization_until_tick, true),
                ReservationStatus::ExpiredUnreconciled { .. } => {
                    return Err(ReservationCoordinatorError::AuthorizationExpired);
                }
                _ => return Err(ReservationCoordinatorError::ReservationNotGranted),
            };
            if now_tick >= until {
                return Err(ReservationCoordinatorError::AuthorizationExpired);
            }
            (until, executing)
        };
        let new_until = now_tick
            .checked_add(extension_ticks)
            .ok_or(ReservationCoordinatorError::LeaseTimeOverflow)?;
        let authorization_until_tick = old_until.max(new_until);
        self.bump_revision()?;
        let record = self.reservations.get_mut(reservation_id).ok_or_else(|| {
            ReservationCoordinatorError::UnknownReservation(reservation_id.to_owned())
        })?;
        record.status = if executing {
            ReservationStatus::Executing {
                fencing_token,
                authorization_until_tick,
            }
        } else {
            ReservationStatus::Granted {
                fencing_token,
                authorization_until_tick,
            }
        };
        Ok(self.book_revision)
    }

    /// Expire command authority, but keep every affected reservation blocking.
    /// Physical clearance must be a separate, independently evidenced action.
    pub fn expire_leases(
        &mut self,
        now_tick: u64,
    ) -> Result<Vec<String>, ReservationCoordinatorError> {
        let expired: Vec<(String, u64, bool)> = self
            .reservations
            .iter()
            .filter_map(|(id, record)| {
                let (until, execution_started) = match &record.status {
                    ReservationStatus::Granted {
                        authorization_until_tick,
                        ..
                    } => (*authorization_until_tick, false),
                    ReservationStatus::Executing {
                        authorization_until_tick,
                        ..
                    } => (*authorization_until_tick, true),
                    ReservationStatus::ExpiredUnreconciled { .. }
                    | ReservationStatus::Cleared { .. } => return None,
                };
                (now_tick >= until).then_some((id.clone(), record.fencing_token, execution_started))
            })
            .collect();

        if expired.is_empty() {
            return Ok(Vec::new());
        }
        self.bump_revision()?;
        for (id, token, execution_started) in &expired {
            if let Some(record) = self.reservations.get_mut(id) {
                record.status = ReservationStatus::ExpiredUnreconciled {
                    fencing_token: *token,
                    execution_started: *execution_started,
                };
            }
        }
        Ok(expired.into_iter().map(|(id, _, _)| id).collect())
    }

    /// Reconcile a reserved physical area as clear. Expiry, robot completion
    /// flags and connection loss cannot perform this transition.
    pub fn record_physical_clearance(
        &mut self,
        assessment: PhysicalClearanceAssessment,
    ) -> Result<u64, ReservationCoordinatorError> {
        if assessment.assessor_id.trim().is_empty()
            || !is_sha256(&assessment.clearance_receipt_sha256)
        {
            return Err(ReservationCoordinatorError::InvalidDigest);
        }
        if let ClearanceOutcome::Unresolved { reason } = &assessment.outcome {
            return Err(ReservationCoordinatorError::ClearanceUnresolved(
                reason.clone(),
            ));
        }
        {
            let record = self
                .reservations
                .get(&assessment.reservation_id)
                .ok_or_else(|| {
                    ReservationCoordinatorError::UnknownReservation(
                        assessment.reservation_id.clone(),
                    )
                })?;
            if assessment.reservation_book_revision != self.book_revision {
                return Err(ReservationCoordinatorError::StaleReservationSnapshot {
                    expected: self.book_revision,
                    actual: assessment.reservation_book_revision,
                });
            }
            if assessment.occupancy_envelope_sha256 != record.request.occupancy_envelope_sha256
                || assessment.frame_sha256 != record.request.frame_sha256
                || assessment.transform_sha256 != record.request.transform_sha256
            {
                return Err(ReservationCoordinatorError::ClearanceContextMismatch);
            }
            if matches!(&record.status, ReservationStatus::Cleared { .. }) {
                return Err(ReservationCoordinatorError::ClearanceAlreadyRecorded);
            }
        }
        // Role membership must be authorized by the caller's verified identity
        // and signature. The enum keeps the permitted assessor class explicit.
        match assessment.assessor_role {
            ClearanceAssessorRole::IndependentReferee
            | ClearanceAssessorRole::SiteSafetyAuthority => {}
        }
        self.bump_revision()?;
        let record = self
            .reservations
            .get_mut(&assessment.reservation_id)
            .ok_or_else(|| {
                ReservationCoordinatorError::UnknownReservation(assessment.reservation_id.clone())
            })?;
        record.status = ReservationStatus::Cleared {
            fencing_token: record.fencing_token,
            clearance_receipt_sha256: assessment.clearance_receipt_sha256,
            cleared_at_tick: assessment.observed_at_tick,
        };
        Ok(self.book_revision)
    }

    /// Check a live command at the command gateway. A grant receipt is not a
    /// permanently reusable credential.
    pub fn authorize_command(
        &self,
        reservation_id: &str,
        robot_id: &str,
        fencing_token: u64,
        now_tick: u64,
    ) -> Result<(), ReservationCoordinatorError> {
        let record = self.reservations.get(reservation_id).ok_or_else(|| {
            ReservationCoordinatorError::UnknownReservation(reservation_id.to_owned())
        })?;
        if record.request.robot_id != robot_id {
            return Err(ReservationCoordinatorError::RobotIdentityMismatch);
        }
        if record.fencing_token != fencing_token {
            return Err(ReservationCoordinatorError::FencingTokenMismatch);
        }
        match &record.status {
            ReservationStatus::Executing {
                fencing_token: token,
                authorization_until_tick,
            } if *token == fencing_token => {
                if now_tick >= *authorization_until_tick {
                    return Err(ReservationCoordinatorError::AuthorizationExpired);
                }
                if now_tick < record.request.start_tick || now_tick >= record.request.end_tick {
                    return Err(ReservationCoordinatorError::OutsideReservedTimeWindow);
                }
                Ok(())
            }
            ReservationStatus::ExpiredUnreconciled { .. } => {
                Err(ReservationCoordinatorError::AuthorizationExpired)
            }
            _ => Err(ReservationCoordinatorError::ReservationNotExecuting),
        }
    }

    fn check_resource_conflicts(
        &self,
        request: &ReservationRequest,
    ) -> Result<(), ReservationCoordinatorError> {
        for (existing_id, existing) in &self.reservations {
            if matches!(existing.status, ReservationStatus::Cleared { .. }) {
                continue;
            }
            let may_still_be_physically_present = matches!(
                &existing.status,
                ReservationStatus::Executing { .. }
                    | ReservationStatus::ExpiredUnreconciled { .. }
            );
            let intervals_overlap = request.start_tick < existing.request.end_tick
                && existing.request.start_tick < request.end_tick;
            if !may_still_be_physically_present && !intervals_overlap {
                continue;
            }
            for requested_resource in &request.resource_ids {
                if existing
                    .request
                    .resource_ids
                    .iter()
                    .any(|resource| resource == requested_resource)
                {
                    return Err(
                        ReservationCoordinatorError::ResourceReservationConflict {
                            requested_reservation_id: request.reservation_id.clone(),
                            existing_reservation_id: existing_id.clone(),
                            resource_id: requested_resource.clone(),
                        },
                    );
                }
            }
        }
        Ok(())
    }

    fn bump_revision(&mut self) -> Result<(), ReservationCoordinatorError> {
        self.book_revision = self
            .book_revision
            .checked_add(1)
            .ok_or(ReservationCoordinatorError::BookRevisionExhausted)?;
        Ok(())
    }
}

fn validate_request(request: &ReservationRequest) -> Result<(), ReservationCoordinatorError> {
    if request.reservation_id.trim().is_empty()
        || request.robot_id.trim().is_empty()
        || request.attempt_id.trim().is_empty()
        || request.site_epoch.trim().is_empty()
        || request.time_domain_id.trim().is_empty()
    {
        return Err(ReservationCoordinatorError::EmptyReservationIdentity);
    }
    if [
        request.request_sha256.as_str(),
        request.package_sha256.as_str(),
        request.frame_sha256.as_str(),
        request.transform_sha256.as_str(),
        request.occupancy_envelope_sha256.as_str(),
    ]
    .into_iter()
    .any(|digest| !is_sha256(digest))
    {
        return Err(ReservationCoordinatorError::InvalidDigest);
    }
    if request.start_tick >= request.end_tick {
        return Err(ReservationCoordinatorError::InvalidTimeInterval);
    }
    if request.resource_ids.is_empty() {
        return Err(ReservationCoordinatorError::InvalidResourceId);
    }
    let mut resources = BTreeSet::new();
    for resource_id in &request.resource_ids {
        if resource_id.trim().is_empty() {
            return Err(ReservationCoordinatorError::InvalidResourceId);
        }
        if !resources.insert(resource_id.as_str()) {
            return Err(ReservationCoordinatorError::DuplicateResourceId(
                resource_id.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_conflict_assessment(
    request: &ReservationRequest,
    assessment: &ConflictAssessment,
    current_book_revision: u64,
) -> Result<(), ReservationCoordinatorError> {
    if !is_sha256(&assessment.evaluator_build_sha256)
        || !is_sha256(&assessment.assessment_receipt_sha256)
    {
        return Err(ReservationCoordinatorError::InvalidDigest);
    }
    if assessment.reservation_book_revision != current_book_revision {
        return Err(ReservationCoordinatorError::StaleReservationSnapshot {
            expected: current_book_revision,
            actual: assessment.reservation_book_revision,
        });
    }
    if assessment.reservation_id != request.reservation_id
        || assessment.request_sha256 != request.request_sha256
        || assessment.package_sha256 != request.package_sha256
        || assessment.frame_sha256 != request.frame_sha256
        || assessment.transform_sha256 != request.transform_sha256
        || assessment.occupancy_envelope_sha256 != request.occupancy_envelope_sha256
        || assessment.site_epoch != request.site_epoch
        || assessment.time_domain_id != request.time_domain_id
        || assessment.start_tick != request.start_tick
        || assessment.end_tick != request.end_tick
    {
        return Err(ReservationCoordinatorError::ReservationContextMismatch);
    }
    Ok(())
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

    fn request(id: &str, robot: &str, start: u64, end: u64, revision: u64) -> ReservationRequest {
        ReservationRequest {
            reservation_id: id.into(),
            request_sha256: digest('1'),
            package_sha256: digest('2'),
            attempt_id: format!("attempt-{id}"),
            robot_id: robot.into(),
            site_epoch: "site-epoch-7".into(),
            time_domain_id: "site-monotonic".into(),
            frame_sha256: digest('3'),
            transform_sha256: digest('4'),
            occupancy_envelope_sha256: digest('5'),
            start_tick: start,
            end_tick: end,
            resource_ids: vec!["workface-A".into()],
            expected_book_revision: revision,
        }
    }

    fn assessment(
        request: &ReservationRequest,
        revision: u64,
        outcome: ConflictAssessmentOutcome,
    ) -> ConflictAssessment {
        ConflictAssessment {
            reservation_id: request.reservation_id.clone(),
            request_sha256: request.request_sha256.clone(),
            package_sha256: request.package_sha256.clone(),
            frame_sha256: request.frame_sha256.clone(),
            transform_sha256: request.transform_sha256.clone(),
            occupancy_envelope_sha256: request.occupancy_envelope_sha256.clone(),
            site_epoch: request.site_epoch.clone(),
            time_domain_id: request.time_domain_id.clone(),
            start_tick: request.start_tick,
            end_tick: request.end_tick,
            reservation_book_revision: revision,
            evaluator_build_sha256: digest('6'),
            assessment_receipt_sha256: digest('7'),
            outcome,
        }
    }

    fn coordinator() -> SpaceTimeReservationCoordinator {
        SpaceTimeReservationCoordinator::new("site-epoch-7", "site-monotonic").unwrap()
    }

    fn grant_one(
        coordinator: &mut SpaceTimeReservationCoordinator,
        id: &str,
        robot: &str,
        start: u64,
        end: u64,
        now: u64,
        lease: u64,
    ) -> ReservationGrant {
        let request = request(id, robot, start, end, coordinator.book_revision());
        let assessment = assessment(
            &request,
            coordinator.book_revision(),
            ConflictAssessmentOutcome::Clear,
        );
        coordinator.grant(request, assessment, now, lease).unwrap()
    }

    #[test]
    fn current_bound_assessment_grants_with_monotonic_fencing_token() {
        let mut coordinator = coordinator();
        let grant = grant_one(&mut coordinator, "r1", "robot-1", 10, 20, 5, 10);
        assert_eq!(grant.fencing_token, 1);
        assert_eq!(grant.authorization_until_tick, 15);
        assert_eq!(grant.reservation_book_revision, 1);
        assert_eq!(
            coordinator.reservation("r1").unwrap().status,
            ReservationStatus::Granted {
                fencing_token: 1,
                authorization_until_tick: 15
            }
        );
    }

    #[test]
    fn stale_snapshot_assessment_cannot_grant_after_book_changes() {
        let mut coordinator = coordinator();
        let first = request("r1", "robot-1", 10, 20, 0);
        let first_assessment = assessment(&first, 0, ConflictAssessmentOutcome::Clear);
        coordinator.grant(first, first_assessment, 5, 10).unwrap();

        let second = request("r2", "robot-2", 20, 30, 0);
        let stale_assessment = assessment(&second, 0, ConflictAssessmentOutcome::Clear);
        assert_eq!(
            coordinator.grant(second, stale_assessment, 5, 10),
            Err(ReservationCoordinatorError::StaleReservationSnapshot {
                expected: 1,
                actual: 0
            })
        );
    }

    #[test]
    fn conflict_or_unresolved_assessment_never_grants() {
        let mut coordinator = coordinator();
        let request = request("r1", "robot-1", 10, 20, 0);
        let conflict = assessment(
            &request,
            0,
            ConflictAssessmentOutcome::Conflicts {
                reservation_ids: vec!["other".into()],
            },
        );
        assert_eq!(
            coordinator.grant(request.clone(), conflict, 5, 10),
            Err(ReservationCoordinatorError::ConflictDetected(vec!["other".into()]))
        );
        let unresolved = assessment(
            &request,
            0,
            ConflictAssessmentOutcome::Unresolved {
                reason: "localization covariance unavailable".into(),
            },
        );
        assert_eq!(
            coordinator.grant(request, unresolved, 5, 10),
            Err(ReservationCoordinatorError::ConflictAssessmentUnresolved(
                "localization covariance unavailable".into()
            ))
        );
        assert_eq!(coordinator.book_revision(), 0);
    }

    #[test]
    fn resource_exclusivity_and_half_open_schedule_intervals_are_enforced() {
        let mut coordinator = coordinator();
        grant_one(&mut coordinator, "r1", "robot-1", 10, 20, 5, 30);
        let non_overlapping = request("r2", "robot-2", 20, 30, coordinator.book_revision());
        let non_overlapping_assessment = assessment(
            &non_overlapping,
            coordinator.book_revision(),
            ConflictAssessmentOutcome::Clear,
        );
        assert!(coordinator.grant(non_overlapping, non_overlapping_assessment, 5, 30).is_ok());

        let overlapping = request("r3", "robot-3", 19, 25, coordinator.book_revision());
        let overlapping_assessment = assessment(
            &overlapping,
            coordinator.book_revision(),
            ConflictAssessmentOutcome::Clear,
        );
        assert!(matches!(
            coordinator.grant(overlapping, overlapping_assessment, 5, 30),
            Err(ReservationCoordinatorError::ResourceReservationConflict { .. })
        ));
    }

    #[test]
    fn execution_requires_exact_robot_token_and_reserved_time() {
        let mut coordinator = coordinator();
        let grant = grant_one(&mut coordinator, "r1", "robot-1", 10, 20, 5, 40);
        assert_eq!(
            coordinator.begin_execution("r1", "robot-2", grant.fencing_token, 10),
            Err(ReservationCoordinatorError::RobotIdentityMismatch)
        );
        assert_eq!(
            coordinator.begin_execution("r1", "robot-1", grant.fencing_token + 1, 10),
            Err(ReservationCoordinatorError::FencingTokenMismatch)
        );
        assert_eq!(
            coordinator.begin_execution("r1", "robot-1", grant.fencing_token, 9),
            Err(ReservationCoordinatorError::OutsideReservedTimeWindow)
        );
        coordinator
            .begin_execution("r1", "robot-1", grant.fencing_token, 10)
            .unwrap();
        assert!(coordinator
            .authorize_command("r1", "robot-1", grant.fencing_token, 11)
            .is_ok());
        assert_eq!(
            coordinator.authorize_command("r1", "robot-1", grant.fencing_token, 25),
            Err(ReservationCoordinatorError::OutsideReservedTimeWindow)
        );
    }

    #[test]
    fn lease_expiry_does_not_release_physical_resource_until_referee_clearance() {
        let mut coordinator = coordinator();
        let grant = grant_one(&mut coordinator, "r1", "robot-1", 10, 20, 5, 10);
        coordinator
            .begin_execution("r1", "robot-1", grant.fencing_token, 10)
            .unwrap();
        let expired = coordinator.expire_leases(15).unwrap();
        assert_eq!(expired, vec!["r1"]);
        assert_eq!(
            coordinator.reservation("r1").unwrap().status,
            ReservationStatus::ExpiredUnreconciled {
                fencing_token: grant.fencing_token,
                execution_started: true
            }
        );
        assert_eq!(
            coordinator.authorize_command("r1", "robot-1", grant.fencing_token, 16),
            Err(ReservationCoordinatorError::AuthorizationExpired)
        );

        let next = request("r2", "robot-2", 30, 40, coordinator.book_revision());
        let next_assessment = assessment(
            &next,
            coordinator.book_revision(),
            ConflictAssessmentOutcome::Clear,
        );
        assert!(matches!(
            coordinator.grant(next, next_assessment, 20, 10),
            Err(ReservationCoordinatorError::ResourceReservationConflict { .. })
        ));

        let clearance = PhysicalClearanceAssessment {
            reservation_id: "r1".into(),
            occupancy_envelope_sha256: digest('5'),
            frame_sha256: digest('3'),
            transform_sha256: digest('4'),
            assessor_id: "referee-1".into(),
            assessor_role: ClearanceAssessorRole::IndependentReferee,
            clearance_receipt_sha256: digest('8'),
            observed_at_tick: 21,
            reservation_book_revision: coordinator.book_revision(),
            outcome: ClearanceOutcome::PhysicallyClear,
        };
        coordinator.record_physical_clearance(clearance).unwrap();

        let next = request("r2", "robot-2", 30, 40, coordinator.book_revision());
        let next_assessment = assessment(
            &next,
            coordinator.book_revision(),
            ConflictAssessmentOutcome::Clear,
        );
        let second = coordinator.grant(next, next_assessment, 22, 10).unwrap();
        assert!(second.fencing_token > grant.fencing_token);
    }

    #[test]
    fn clearance_requires_matching_context_and_independent_role() {
        let mut coordinator = coordinator();
        let grant = grant_one(&mut coordinator, "r1", "robot-1", 10, 20, 5, 10);
        let mut invalid = PhysicalClearanceAssessment {
            reservation_id: "r1".into(),
            occupancy_envelope_sha256: digest('0'),
            frame_sha256: digest('3'),
            transform_sha256: digest('4'),
            assessor_id: "robot-1".into(),
            assessor_role: ClearanceAssessorRole::IndependentReferee,
            clearance_receipt_sha256: digest('8'),
            observed_at_tick: 12,
            reservation_book_revision: coordinator.book_revision(),
            outcome: ClearanceOutcome::PhysicallyClear,
        };
        assert_eq!(
            coordinator.record_physical_clearance(invalid.clone()),
            Err(ReservationCoordinatorError::ClearanceContextMismatch)
        );
        invalid.occupancy_envelope_sha256 = digest('5');
        coordinator.record_physical_clearance(invalid).unwrap();
        assert_eq!(
            coordinator.reservation("r1").unwrap().status,
            ReservationStatus::Cleared {
                fencing_token: grant.fencing_token,
                clearance_receipt_sha256: digest('8'),
                cleared_at_tick: 12
            }
        );
    }
}
