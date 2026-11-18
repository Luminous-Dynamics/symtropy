//! Fencing and replay protection for commands issued against a committed
//! construction space-time reservation.
//!
//! The fencing token is the reservation commit's monotonic site-ledger
//! sequence. No second token authority is introduced. The gate verifies the
//! readiness receipt consumed by that exact commit, the active reservation,
//! robot/work-package/envelope bindings, site clock, lease and current ledger.
//!
//! Only later reservation commits/releases under the same occupancy engine,
//! clearance policy, resource catalog, package and clock are considered
//! compatible. Other site mutations invalidate this command authorization.
//! This deliberately conservative event rule can be relaxed only with a
//! separately qualified, resource-scoped invalidation model.
//!
//! This is a software command-eligibility reference, not an actuator API,
//! safety permit or protective-stop system. The caller must authenticate
//! identities and receipts, and persist CommandReplaySnapshot atomically with
//! its command gateway state.

use std::collections::BTreeMap;

use crate::dispatch_readiness::{
    DISPATCH_READINESS_RECEIPT_SCHEMA_VERSION, DispatchReadinessReceipt,
};
use crate::site_state_ledger::{
    SiteEntityState, SiteEventPayload, SiteStateLedger, SpaceTimeReservationRecord,
};

pub const RESERVATION_COMMAND_SCHEMA_VERSION: &str =
    "construction-space-time-reservation-command/0.1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReservationCommandRequest {
    pub schema_version: String,
    pub reservation_id: String,
    /// The immutable reservation commit sequence. This is the fencing token.
    pub fencing_token: u64,
    pub robot_id: String,
    pub attempt_id: String,
    pub package_sha256: String,
    pub design_revision_sha256: String,
    pub entity_id: String,
    pub envelope_sha256: String,
    pub resource_catalog_sha256: String,
    pub resource_claim_manifest_sha256: String,
    pub readiness_receipt_sha256: String,
    pub readiness_receipt: DispatchReadinessReceipt,
    /// Current site-ledger snapshot observed by the command gateway.
    pub current_ledger_sequence: u64,
    pub current_ledger_event_sha256: String,
    pub site_clock_domain_id: String,
    pub command_id: String,
    pub command_sha256: String,
    /// Strictly increasing per reservation; first command is sequence 1.
    pub command_sequence: u64,
    pub issued_at_tick: u64,
    pub now_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReservationCommandReceipt {
    pub schema_version: String,
    pub reservation_id: String,
    pub fencing_token: u64,
    pub robot_id: String,
    pub attempt_id: String,
    pub package_sha256: String,
    pub entity_id: String,
    pub envelope_sha256: String,
    pub command_id: String,
    pub command_sha256: String,
    pub command_sequence: u64,
    pub ledger_sequence: u64,
    pub ledger_event_sha256: String,
    pub valid_until_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReservationCommandFailure {
    UnsupportedSchemaVersion,
    InvalidDigest(&'static str),
    EmptyCommandIdentity,
    UnknownReservation,
    ReservationReleased,
    ReservationFencingTokenMismatch,
    RobotIdentityMismatch,
    ReservationBindingMismatch,
    ReadinessReceiptSchemaMismatch,
    ReadinessReceiptBindingMismatch,
    ReadinessReceiptExpired,
    ReadinessReceiptNotYetValid,
    ReservationLeaseExpired,
    OutsideReservationTimeWindow,
    SiteClockDomainMismatch,
    StaleLedgerSequence,
    LedgerHeadMismatch,
    ReservationCommitEventMissing,
    UnrelatedLedgerMutation(String),
    MixedOccupancyEngineOrPolicy,
    CurrentEntityStateInvalid,
    ExecutionAttemptMismatch,
    DuplicateExecutionStart,
    CommandSequenceZero,
    CommandSequenceGap { expected: u64, actual: u64 },
    DuplicateCommandId(String),
    CommandIdDigestMismatch(String),
    ReplaySnapshotInvalid,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommandReplaySnapshot {
    /// reservation ID -> (fencing token, last accepted command sequence)
    pub last_sequence_by_reservation: BTreeMap<String, (u64, u64)>,
    /// command ID -> command digest, retained to reject ID reuse across
    /// reservations even after a reservation has been cleared.
    pub command_digests_by_id: BTreeMap<String, String>,
}

/// Reference replay guard. Persist snapshot() atomically with the command
/// gateway's durable command acceptance record. Constructing a fresh empty
/// guard after restart discards replay history and is not production-safe.
#[derive(Clone, Debug, Default)]
pub struct CommandReplayGuard {
    snapshot: CommandReplaySnapshot,
}

impl CommandReplayGuard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_snapshot(
        snapshot: CommandReplaySnapshot,
    ) -> Result<Self, ReservationCommandFailure> {
        for (reservation_id, (fencing_token, sequence)) in
            &snapshot.last_sequence_by_reservation
        {
            if reservation_id.trim().is_empty() || *fencing_token == 0 || *sequence == 0 {
                return Err(ReservationCommandFailure::ReplaySnapshotInvalid);
            }
        }
        for (command_id, digest) in &snapshot.command_digests_by_id {
            if command_id.trim().is_empty() || !is_sha256(digest) {
                return Err(ReservationCommandFailure::ReplaySnapshotInvalid);
            }
        }
        Ok(Self { snapshot })
    }

    pub fn snapshot(&self) -> &CommandReplaySnapshot {
        &self.snapshot
    }

    pub fn last_sequence(&self, reservation_id: &str) -> Option<u64> {
        self.snapshot
            .last_sequence_by_reservation
            .get(reservation_id)
            .map(|(_, sequence)| *sequence)
    }

    /// Verify all ledger/reservation bindings, then atomically consume the
    /// next per-reservation command sequence in this replay-guard projection.
    pub fn authorize(
        &mut self,
        ledger: &SiteStateLedger,
        request: &ReservationCommandRequest,
    ) -> Result<ReservationCommandReceipt, ReservationCommandFailure> {
        let receipt = check_reservation_command_current(ledger, request)?;

        if let Some(previous_digest) = self
            .snapshot
            .command_digests_by_id
            .get(&request.command_id)
        {
            return if previous_digest == &request.command_sha256 {
                Err(ReservationCommandFailure::DuplicateCommandId(
                    request.command_id.clone(),
                ))
            } else {
                Err(ReservationCommandFailure::CommandIdDigestMismatch(
                    request.command_id.clone(),
                ))
            };
        }

        let expected_sequence = self
            .snapshot
            .last_sequence_by_reservation
            .get(&request.reservation_id)
            .map_or(1, |(token, sequence)| {
                if *token == request.fencing_token {
                    sequence.saturating_add(1)
                } else {
                    0
                }
            });
        if expected_sequence == 0 {
            return Err(ReservationCommandFailure::ReservationFencingTokenMismatch);
        }
        if request.command_sequence != expected_sequence {
            return Err(ReservationCommandFailure::CommandSequenceGap {
                expected: expected_sequence,
                actual: request.command_sequence,
            });
        }

        // All checks above are read-only. Mutate replay state only after the
        // complete command authorization predicate has passed.
        self.snapshot.last_sequence_by_reservation.insert(
            request.reservation_id.clone(),
            (request.fencing_token, request.command_sequence),
        );
        self.snapshot.command_digests_by_id.insert(
            request.command_id.clone(),
            request.command_sha256.clone(),
        );
        Ok(receipt)
    }
}

/// Check a reservation command against the current append-only site ledger.
///
/// The original readiness receipt is intentionally compared to the snapshot
/// recorded inside the reservation. It is not re-run against the post-commit
/// ledger because the reservation commit itself advances that ledger.
pub fn check_reservation_command_current(
    ledger: &SiteStateLedger,
    request: &ReservationCommandRequest,
) -> Result<ReservationCommandReceipt, ReservationCommandFailure> {
    if request.schema_version != RESERVATION_COMMAND_SCHEMA_VERSION {
        return Err(ReservationCommandFailure::UnsupportedSchemaVersion);
    }
    for (label, digest) in [
        ("package", request.package_sha256.as_str()),
        ("design revision", request.design_revision_sha256.as_str()),
        ("envelope", request.envelope_sha256.as_str()),
        ("resource catalog", request.resource_catalog_sha256.as_str()),
        ("resource claim manifest", request.resource_claim_manifest_sha256.as_str()),
        ("readiness receipt", request.readiness_receipt_sha256.as_str()),
        ("current ledger head", request.current_ledger_event_sha256.as_str()),
        ("command", request.command_sha256.as_str()),
    ] {
        if !is_sha256(digest) {
            return Err(ReservationCommandFailure::InvalidDigest(label));
        }
    }
    if request.reservation_id.trim().is_empty()
        || request.robot_id.trim().is_empty()
        || request.attempt_id.trim().is_empty()
        || request.entity_id.trim().is_empty()
        || request.site_clock_domain_id.trim().is_empty()
        || request.command_id.trim().is_empty()
    {
        return Err(ReservationCommandFailure::EmptyCommandIdentity);
    }
    if request.command_sequence == 0 {
        return Err(ReservationCommandFailure::CommandSequenceZero);
    }

    let record = ledger
        .space_time_reservation(&request.reservation_id)
        .ok_or(ReservationCommandFailure::UnknownReservation)?;
    if !record.is_active() {
        return Err(ReservationCommandFailure::ReservationReleased);
    }
    let reservation = &record.reservation;

    if request.fencing_token != record.committed_sequence {
        return Err(ReservationCommandFailure::ReservationFencingTokenMismatch);
    }
    if request.robot_id != reservation.robot_id {
        return Err(ReservationCommandFailure::RobotIdentityMismatch);
    }
    if request.package_sha256 != reservation.package_sha256
        || request.design_revision_sha256 != reservation.design_revision_sha256
        || request.entity_id != reservation.entity_id
        || request.envelope_sha256 != reservation.envelope_sha256
        || request.resource_catalog_sha256 != reservation.resource_catalog_sha256
        || request.resource_claim_manifest_sha256 != reservation.resource_claim_manifest_sha256
        || request.readiness_receipt_sha256 != reservation.readiness_receipt_sha256
        || request.site_clock_domain_id != reservation.site_clock_domain_id
    {
        return Err(ReservationCommandFailure::ReservationBindingMismatch);
    }

    validate_readiness_receipt_binding(request, record)?;

    if ledger.active_package_sha256() != Some(reservation.package_sha256.as_str())
        || ledger.package_is_current(&reservation.package_sha256) != Some(true)
    {
        return Err(ReservationCommandFailure::ReservationBindingMismatch);
    }
    if ledger.resource_catalog_sha256() != Some(reservation.resource_catalog_sha256.as_str()) {
        return Err(ReservationCommandFailure::ReservationBindingMismatch);
    }

    if request.now_tick < reservation.committed_at_tick
        || request.issued_at_tick < reservation.committed_at_tick
        || request.issued_at_tick > request.now_tick
    {
        return Err(ReservationCommandFailure::ReadinessReceiptNotYetValid);
    }
    if request.now_tick >= reservation.valid_until_tick
        || request.now_tick >= reservation.lease_valid_until_tick
    {
        return Err(ReservationCommandFailure::ReservationLeaseExpired);
    }
    if request.now_tick < reservation.start_tick || request.now_tick >= reservation.end_tick {
        return Err(ReservationCommandFailure::OutsideReservationTimeWindow);
    }
    if request.readiness_receipt.checked_at_tick > request.now_tick {
        return Err(ReservationCommandFailure::ReadinessReceiptNotYetValid);
    }
    if request.now_tick >= request.readiness_receipt.valid_until_tick {
        return Err(ReservationCommandFailure::ReadinessReceiptExpired);
    }
    if request.current_ledger_sequence != ledger.last_sequence() {
        return Err(ReservationCommandFailure::StaleLedgerSequence);
    }
    if ledger.last_event_sha256() != Some(request.current_ledger_event_sha256.as_str()) {
        return Err(ReservationCommandFailure::LedgerHeadMismatch);
    }

    validate_commit_history_compatibility(ledger, record)?;

    let package_state = ledger.entity_state(&reservation.package_sha256, &reservation.entity_id);
    let mut started_attempt: Option<String> = None;
    if let Some(state) = package_state {
        match state {
            SiteEntityState::NotAttempted | SiteEntityState::PhysicallyAbsent { .. } => {}
            SiteEntityState::ExecutionInProgress { attempt_id } => {
                started_attempt = Some(attempt_id.clone());
            }
            _ => return Err(ReservationCommandFailure::CurrentEntityStateInvalid),
        }
    } else {
        return Err(ReservationCommandFailure::CurrentEntityStateInvalid);
    }
    if let Some(attempt_id) = started_attempt {
        if attempt_id != request.attempt_id {
            return Err(ReservationCommandFailure::ExecutionAttemptMismatch);
        }
    }

    let valid_until_tick = reservation
        .valid_until_tick
        .min(reservation.lease_valid_until_tick)
        .min(reservation.end_tick)
        .min(request.readiness_receipt.valid_until_tick);

    Ok(ReservationCommandReceipt {
        schema_version: RESERVATION_COMMAND_SCHEMA_VERSION.into(),
        reservation_id: request.reservation_id.clone(),
        fencing_token: record.committed_sequence,
        robot_id: request.robot_id.clone(),
        attempt_id: request.attempt_id.clone(),
        package_sha256: reservation.package_sha256.clone(),
        entity_id: reservation.entity_id.clone(),
        envelope_sha256: reservation.envelope_sha256.clone(),
        command_id: request.command_id.clone(),
        command_sha256: request.command_sha256.clone(),
        command_sequence: request.command_sequence,
        ledger_sequence: ledger.last_sequence(),
        ledger_event_sha256: request.current_ledger_event_sha256.clone(),
        valid_until_tick,
    })
}

fn validate_readiness_receipt_binding(
    request: &ReservationCommandRequest,
    record: &SpaceTimeReservationRecord,
) -> Result<(), ReservationCommandFailure> {
    let reservation = &record.reservation;
    let receipt = &request.readiness_receipt;
    if receipt.schema_version != DISPATCH_READINESS_RECEIPT_SCHEMA_VERSION {
        return Err(ReservationCommandFailure::ReadinessReceiptSchemaMismatch);
    }
    for digest in [
        receipt.package_sha256.as_str(),
        receipt.design_revision_sha256.as_str(),
        receipt.envelope_sha256.as_str(),
        receipt.site_snapshot_sha256.as_str(),
        receipt.occupancy_assessment_receipt_sha256.as_str(),
        receipt.occupancy_engine_build_sha256.as_str(),
        receipt.clearance_policy_sha256.as_str(),
        receipt.site_ledger_event_sha256.as_str(),
    ] {
        if !is_sha256(digest) {
            return Err(ReservationCommandFailure::ReadinessReceiptBindingMismatch);
        }
    }
    if receipt.valid_until_tick <= receipt.checked_at_tick
        || receipt.site_ledger_sequence == 0
        || receipt.package_sha256 != reservation.package_sha256
        || receipt.design_revision_sha256 != reservation.design_revision_sha256
        || receipt.entity_id != reservation.entity_id
        || receipt.robot_id != reservation.robot_id
        || receipt.envelope_sha256 != reservation.envelope_sha256
        || receipt.site_snapshot_sha256 != reservation.site_snapshot_sha256
        || receipt.occupancy_assessment_receipt_sha256
            != reservation.occupancy_assessment_receipt_sha256
        || receipt.occupancy_engine_build_sha256 != reservation.occupancy_engine_build_sha256
        || receipt.clearance_policy_sha256 != reservation.clearance_policy_sha256
        || receipt.site_ledger_sequence != reservation.site_ledger_sequence
        || receipt.site_ledger_event_sha256 != reservation.site_ledger_event_sha256
        || receipt.site_clock_domain_id != reservation.site_clock_domain_id
        || receipt.checked_at_tick != reservation.checked_at_tick
        || receipt.valid_until_tick != reservation.valid_until_tick
        || request.readiness_receipt_sha256 != reservation.readiness_receipt_sha256
    {
        return Err(ReservationCommandFailure::ReadinessReceiptBindingMismatch);
    }
    let expected_commit_sequence = reservation
        .site_ledger_sequence
        .checked_add(1)
        .ok_or(ReservationCommandFailure::ReservationFencingTokenMismatch)?;
    if record.committed_sequence != expected_commit_sequence
        || record.committed_event_id.trim().is_empty()
    {
        return Err(ReservationCommandFailure::ReservationFencingTokenMismatch);
    }
    Ok(())
}

/// Ensure every ledger mutation after this reservation is either another
/// reservation commit validated against the then-current ledger, or a
/// referee-authored release that only removes occupancy. Any other event is
/// treated as potentially invalidating the command's original physical context.
fn validate_commit_history_compatibility(
    ledger: &SiteStateLedger,
    record: &SpaceTimeReservationRecord,
) -> Result<(), ReservationCommandFailure> {
    let reservation = &record.reservation;
    let commit_index = usize::try_from(record.committed_sequence)
        .ok()
        .and_then(|sequence| sequence.checked_sub(1))
        .ok_or(ReservationCommandFailure::ReservationCommitEventMissing)?;
    let commit_event = ledger
        .event_history()
        .get(commit_index)
        .ok_or(ReservationCommandFailure::ReservationCommitEventMissing)?;
    if commit_event.event_id != record.committed_event_id
        || !is_sha256(&commit_event.event_sha256)
        || commit_event.sequence != record.committed_sequence
    {
        return Err(ReservationCommandFailure::ReservationCommitEventMissing);
    }
    match &commit_event.payload {
        SiteEventPayload::SpaceTimeReservationCommitted { reservation: committed }
            if committed == reservation => {}
        _ => return Err(ReservationCommandFailure::ReservationCommitEventMissing),
    }

    for event in ledger.event_history().iter().skip(commit_index + 1) {
        match &event.payload {
            SiteEventPayload::SpaceTimeReservationCommitted { reservation: later } => {
                if later.package_sha256 != reservation.package_sha256
                    || later.design_revision_sha256 != reservation.design_revision_sha256
                    || later.resource_catalog_sha256 != reservation.resource_catalog_sha256
                    || later.site_clock_domain_id != reservation.site_clock_domain_id
                    || later.occupancy_engine_build_sha256
                        != reservation.occupancy_engine_build_sha256
                    || later.clearance_policy_sha256 != reservation.clearance_policy_sha256
                {
                    return Err(ReservationCommandFailure::MixedOccupancyEngineOrPolicy);
                }
            }
            SiteEventPayload::SpaceTimeReservationReleased {
                reservation_id: released_id,
                site_clock_domain_id,
                ..
            } => {
                if released_id == &record.reservation.reservation_id {
                    return Err(ReservationCommandFailure::ReservationReleased);
                }
                if event.actor_role != crate::site_state_ledger::SiteActorRole::Referee
                    || site_clock_domain_id != &reservation.site_clock_domain_id
                {
                    return Err(ReservationCommandFailure::UnrelatedLedgerMutation(
                        event.event_id.clone(),
                    ));
                }
            }
            SiteEventPayload::ExecutionStarted {
                package_sha256,
                entity_id,
                ..
            } if package_sha256 == &reservation.package_sha256
                && entity_id == &reservation.entity_id
                && event.actor_id == reservation.robot_id => {}
            _ => {
                return Err(ReservationCommandFailure::UnrelatedLedgerMutation(
                    event.event_id.clone(),
                ));
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandReplayGuardError {
    InvalidSnapshot,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommandReplaySnapshot {
    /// reservation ID -> (fencing token, last accepted command sequence)
    pub last_sequence_by_reservation: BTreeMap<String, (u64, u64)>,
    /// command ID -> command SHA-256; retained across reservation release.
    pub command_digests_by_id: BTreeMap<String, String>,
}

/// Stateful reference replay guard. The snapshot must be persisted atomically
/// by the consuming command gateway; a volatile reset is not replay-safe.
#[derive(Clone, Debug, Default)]
pub struct CommandReplayGuard {
    snapshot: CommandReplaySnapshot,
}

impl CommandReplayGuard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_snapshot(snapshot: CommandReplaySnapshot) -> Result<Self, CommandReplayGuardError> {
        for (reservation_id, (fencing_token, sequence)) in
            &snapshot.last_sequence_by_reservation
        {
            if reservation_id.trim().is_empty() || *fencing_token == 0 || *sequence == 0 {
                return Err(CommandReplayGuardError::InvalidSnapshot);
            }
        }
        for (command_id, digest) in &snapshot.command_digests_by_id {
            if command_id.trim().is_empty() || !is_sha256(digest) {
                return Err(CommandReplayGuardError::InvalidSnapshot);
            }
        }
        Ok(Self { snapshot })
    }

    pub fn snapshot(&self) -> &CommandReplaySnapshot {
        &self.snapshot
    }

    pub fn authorize(
        &mut self,
        ledger: &SiteStateLedger,
        request: &ReservationCommandRequest,
    ) -> Result<ReservationCommandReceipt, ReservationCommandFailure> {
        let receipt = check_reservation_command_current(ledger, request)?;

        if let Some(existing_digest) = self
            .snapshot
            .command_digests_by_id
            .get(&request.command_id)
        {
            if existing_digest == &request.command_sha256 {
                return Err(ReservationCommandFailure::DuplicateCommandId(
                    request.command_id.clone(),
                ));
            }
            return Err(ReservationCommandFailure::CommandIdDigestMismatch(
                request.command_id.clone(),
            ));
        }

        let expected = match self
            .snapshot
            .last_sequence_by_reservation
            .get(&request.reservation_id)
        {
            None => 1,
            Some((token, last_sequence)) if *token == request.fencing_token => last_sequence
                .checked_add(1)
                .ok_or(ReservationCommandFailure::CommandSequenceGap {
                    expected: u64::MAX,
                    actual: request.command_sequence,
                })?,
            Some(_) => {
                return Err(ReservationCommandFailure::ReservationFencingTokenMismatch);
            }
        };
        if request.command_sequence != expected {
            return Err(ReservationCommandFailure::CommandSequenceGap {
                expected,
                actual: request.command_sequence,
            });
        }

        // Mutate replay state only after every currentness and replay check.
        self.snapshot.last_sequence_by_reservation.insert(
            request.reservation_id.clone(),
            (request.fencing_token, request.command_sequence),
        );
        self.snapshot.command_digests_by_id.insert(
            request.command_id.clone(),
            request.command_sha256.clone(),
        );
        Ok(receipt)
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
    use crate::dispatch_readiness::{
        check_dispatch_readiness, commit_dispatch_reservation, DispatchEnvelopeBinding,
        DispatchReadinessRequest, OccupancyAssessmentReceipt, DISPATCH_READINESS_REQUEST_SCHEMA_VERSION,
    };
    use crate::field_layout_acceptance::{
        DesignIntentEntity, FieldAcceptanceProfile, FieldLayoutEntityType,
    };
    use crate::site_state_ledger::{
        ExclusiveResourceClaim, SiteActorRole, SiteEventEnvelope, SiteEventPayload,
        SiteResourceDefinition, SiteWorkPackage,
    };

    fn digest(value: char) -> String {
        std::iter::repeat_n(value, 64).collect()
    }

    fn initial_ledger() -> SiteStateLedger {
        let mut ledger = SiteStateLedger::new();
        let package = SiteWorkPackage {
            package_sha256: digest('a'),
            source_model_sha256: digest('b'),
            source_schema_release: "IFC4X3_ADD2".into(),
            ids_spec_sha256: digest('c'),
            ids_version: "1.0".into(),
            ids_engine_build_sha256: digest('d'),
            design_revision_sha256: digest('e'),
            frame_id: "site-frame".into(),
            frame_sha256: digest('f'),
            transform_sha256: digest('1'),
            entity_manifest_sha256: digest('2'),
            acceptance_profile_sha256: digest('3'),
            entities: vec![
                DesignIntentEntity {
                    design_entity_id: "ifc4x3:0000000000000000000001".into(),
                    entity_type: FieldLayoutEntityType::Point,
                },
                DesignIntentEntity {
                    design_entity_id: "ifc4x3:0000000000000000000002".into(),
                    entity_type: FieldLayoutEntityType::Point,
                },
            ],
            acceptance_profile: FieldAcceptanceProfile {
                max_abs_xy_residual_mm: 1.0,
                max_reference_u95_xy_mm: 0.25,
                max_abs_z_residual_mm: None,
                max_reference_u95_z_mm: None,
                max_abs_endpoint_residual_mm: None,
            },
        };
        ledger
            .apply(SiteEventEnvelope {
                event_id: "package-publish".into(),
                sequence: 1,
                previous_event_sha256: None,
                event_sha256: digest('4'),
                actor_id: "design-authority".into(),
                actor_role: SiteActorRole::DesignAuthority,
                payload: SiteEventPayload::WorkPackagePublished { package },
            })
            .unwrap();
        ledger
    }

    fn readiness_request(
        ledger: &SiteStateLedger,
        robot_id: &str,
        entity_id: &str,
        now_tick: u64,
        snapshot_digest: char,
        assessment_digest: char,
    ) -> DispatchReadinessRequest {
        DispatchReadinessRequest {
            schema_version: DISPATCH_READINESS_REQUEST_SCHEMA_VERSION.into(),
            envelope: DispatchEnvelopeBinding {
                package_sha256: ledger.active_package_sha256().unwrap().to_owned(),
                design_revision_sha256: digest('e'),
                entity_id: entity_id.into(),
                frame_sha256: digest('f'),
                transform_sha256: digest('1'),
                robot_id: robot_id.into(),
                robot_configuration_sha256: digest('5'),
                tool_configuration_sha256: digest('6'),
                payload_configuration_sha256: digest('7'),
                primitive_manifest_sha256: digest('8'),
                trajectory_sha256: digest('9'),
                envelope_sha256: digest('a'),
                site_clock_domain_id: "site-clock-01".into(),
                generated_at_tick: now_tick.saturating_sub(20),
                valid_until_tick: 500,
            },
            trusted_occupancy_engine_build_sha256: digest('e'),
            required_clearance_policy_sha256: digest('f'),
            occupancy: OccupancyAssessmentReceipt {
                assessed_envelope_sha256: digest('a'),
                primitive_manifest_sha256: digest('8'),
                site_snapshot_sha256: digest(snapshot_digest),
                site_ledger_sequence: ledger.last_sequence(),
                site_ledger_event_sha256: ledger.last_event_sha256().unwrap().to_owned(),
                dynamic_occupancy_manifest_sha256: digest('c'),
                actor_manifest_sha256: digest('d'),
                occupancy_engine_build_sha256: digest('e'),
                clearance_policy_sha256: digest('f'),
                site_clock_domain_id: "site-clock-01".into(),
                checked_at_tick: now_tick.saturating_sub(10),
                valid_until_tick: 400,
                primitive_coverage_complete: true,
                dynamic_actor_coverage_complete: true,
                broadphase_pair_coverage_complete: true,
                narrowphase_coverage_complete: true,
                candidate_pair_count: 0,
                narrowphase_evaluated_pair_count: 0,
                conflict_count: 0,
                unresolved_pair_count: 0,
                invalid_bound_count: 0,
                unqualified_bound_count: 0,
                stale_actor_count: 0,
                unmodelled_occupant_count: 0,
                assessment_receipt_sha256: digest(assessment_digest),
            },
            now_tick,
        }
    }

    fn commit_one_reservation(
        ledger: &mut SiteStateLedger,
    ) -> (DispatchReadinessReceipt, SpaceTimeReservationRecord) {
        ledger
            .apply(SiteEventEnvelope {
                event_id: "resource-catalog".into(),
                sequence: ledger.last_sequence() + 1,
                previous_event_sha256: ledger.last_event_sha256().map(str::to_owned),
                event_sha256: digest('5'),
                actor_id: "resource-authority".into(),
                actor_role: SiteActorRole::SiteResourceAuthority,
                payload: SiteEventPayload::ResourceCatalogPublished {
                    catalog_sha256: digest('6'),
                    resources: vec![
                        SiteResourceDefinition {
                            resource_id: "workface-A".into(),
                            definition_sha256: digest('7'),
                        },
                        SiteResourceDefinition {
                            resource_id: "workface-B".into(),
                            definition_sha256: digest('8'),
                        },
                    ],
                },
            })
            .unwrap();

        let request = readiness_request(
            ledger,
            "robot-01",
            "ifc4x3:0000000000000000000001",
            120,
            'b',
            '9',
        );
        let receipt = check_dispatch_readiness(ledger, &request).unwrap();
        let reservation = crate::site_state_ledger::SpaceTimeReservation {
            reservation_id: "reservation-1".into(),
            resource_catalog_sha256: digest('6'),
            resource_claim_manifest_sha256: digest('8'),
            claims: vec![ExclusiveResourceClaim {
                resource_id: "workface-A".into(),
                definition_sha256: digest('7'),
                start_tick: 130,
                end_tick: 300,
            }],
            package_sha256: receipt.package_sha256.clone(),
            design_revision_sha256: receipt.design_revision_sha256.clone(),
            entity_id: receipt.entity_id.clone(),
            robot_id: receipt.robot_id.clone(),
            envelope_sha256: receipt.envelope_sha256.clone(),
            readiness_receipt_sha256: digest('d'),
            occupancy_assessment_receipt_sha256: receipt
                .occupancy_assessment_receipt_sha256
                .clone(),
            occupancy_engine_build_sha256: receipt.occupancy_engine_build_sha256.clone(),
            clearance_policy_sha256: receipt.clearance_policy_sha256.clone(),
            site_snapshot_sha256: receipt.site_snapshot_sha256.clone(),
            site_ledger_sequence: receipt.site_ledger_sequence,
            site_ledger_event_sha256: receipt.site_ledger_event_sha256.clone(),
            site_clock_domain_id: receipt.site_clock_domain_id.clone(),
            checked_at_tick: receipt.checked_at_tick,
            valid_until_tick: receipt.valid_until_tick,
            committed_at_tick: 120,
            start_tick: 130,
            end_tick: 300,
            lease_valid_until_tick: 350,
        };
        let commit_event = SiteEventEnvelope {
            event_id: "reservation-commit".into(),
            sequence: ledger.last_sequence() + 1,
            previous_event_sha256: ledger.last_event_sha256().map(str::to_owned),
            event_sha256: digest('9'),
            actor_id: "dispatch-coordinator".into(),
            actor_role: SiteActorRole::DispatchCoordinator,
            payload: SiteEventPayload::SpaceTimeReservationCommitted {
                reservation: reservation.clone(),
            },
        };
        commit_dispatch_reservation(
            ledger,
            &receipt,
            &digest('d'),
            &digest('e'),
            &digest('f'),
            120,
            commit_event,
        )
        .unwrap();
        (
            receipt,
            ledger.space_time_reservation("reservation-1").unwrap().clone(),
        )
    }

    fn command_request(
        ledger: &SiteStateLedger,
        readiness: &DispatchReadinessReceipt,
        reservation: &SpaceTimeReservationRecord,
        command_id: &str,
        command_sequence: u64,
    ) -> ReservationCommandRequest {
        ReservationCommandRequest {
            schema_version: RESERVATION_COMMAND_SCHEMA_VERSION.into(),
            reservation_id: "reservation-1".into(),
            fencing_token: reservation.committed_sequence,
            robot_id: readiness.robot_id.clone(),
            attempt_id: "attempt-1".into(),
            package_sha256: readiness.package_sha256.clone(),
            design_revision_sha256: readiness.design_revision_sha256.clone(),
            entity_id: readiness.entity_id.clone(),
            envelope_sha256: readiness.envelope_sha256.clone(),
            resource_catalog_sha256: reservation.reservation.resource_catalog_sha256.clone(),
            resource_claim_manifest_sha256: reservation
                .reservation
                .resource_claim_manifest_sha256
                .clone(),
            readiness_receipt_sha256: digest('d'),
            readiness_receipt: readiness.clone(),
            current_ledger_sequence: ledger.last_sequence(),
            current_ledger_event_sha256: ledger.last_event_sha256().unwrap().to_owned(),
            site_clock_domain_id: readiness.site_clock_domain_id.clone(),
            command_id: command_id.into(),
            command_sha256: digest('c'),
            command_sequence,
            issued_at_tick: 130,
            now_tick: 130,
        }
    }

    #[test]
    fn exact_active_reservation_issues_a_sequence_fenced_command_receipt() {
        let mut ledger = initial_ledger();
        let (readiness, reservation) = commit_one_reservation(&mut ledger);
        let request = command_request(&ledger, &readiness, &reservation, "cmd-1", 1);
        let receipt = check_reservation_command_current(&ledger, &request).unwrap();
        assert_eq!(receipt.fencing_token, reservation.committed_sequence);
        assert_eq!(receipt.valid_until_tick, 300);
        assert_eq!(receipt.ledger_sequence, ledger.last_sequence());
    }

    #[test]
    fn stale_token_robot_lease_or_release_cannot_authorize_a_command() {
        let mut ledger = initial_ledger();
        let (readiness, reservation) = commit_one_reservation(&mut ledger);

        let mut stale = command_request(&ledger, &readiness, &reservation, "cmd-stale", 1);
        stale.fencing_token -= 1;
        assert_eq!(
            check_reservation_command_current(&ledger, &stale),
            Err(ReservationCommandFailure::ReservationFencingTokenMismatch)
        );

        let mut wrong_robot = command_request(&ledger, &readiness, &reservation, "cmd-wrong-robot", 1);
        wrong_robot.robot_id = "robot-02".into();
        assert_eq!(
            check_reservation_command_current(&ledger, &wrong_robot),
            Err(ReservationCommandFailure::RobotIdentityMismatch)
        );

        let mut expired = command_request(&ledger, &readiness, &reservation, "cmd-expired", 1);
        expired.now_tick = reservation.reservation.lease_valid_until_tick;
        expired.issued_at_tick = expired.now_tick;
        assert_eq!(
            check_reservation_command_current(&ledger, &expired),
            Err(ReservationCommandFailure::ReservationLeaseExpired)
        );
    }

    #[test]
    fn later_compatible_reservations_can_coexist_but_unrelated_site_events_invalidate() {
        let mut ledger = initial_ledger();
        let (readiness, reservation) = commit_one_reservation(&mut ledger);

        // A second robot obtains a fresh occupancy assessment against the
        // current ledger, which already contains reservation-1. It reserves a
        // different exclusive workface under the same engine and policy.
        let second_request = readiness_request(
            &ledger,
            "robot-02",
            "ifc4x3:0000000000000000000002",
            125,
            'c',
            'd',
        );
        let second_readiness = check_dispatch_readiness(&ledger, &second_request).unwrap();
        let second_reservation = crate::site_state_ledger::SpaceTimeReservation {
            reservation_id: "reservation-2".into(),
            resource_catalog_sha256: digest('6'),
            resource_claim_manifest_sha256: digest('e'),
            claims: vec![ExclusiveResourceClaim {
                resource_id: "workface-B".into(),
                definition_sha256: digest('8'),
                start_tick: 135,
                end_tick: 300,
            }],
            package_sha256: second_readiness.package_sha256.clone(),
            design_revision_sha256: second_readiness.design_revision_sha256.clone(),
            entity_id: second_readiness.entity_id.clone(),
            robot_id: second_readiness.robot_id.clone(),
            envelope_sha256: second_readiness.envelope_sha256.clone(),
            readiness_receipt_sha256: digest('f'),
            occupancy_assessment_receipt_sha256: second_readiness
                .occupancy_assessment_receipt_sha256
                .clone(),
            occupancy_engine_build_sha256: second_readiness
                .occupancy_engine_build_sha256
                .clone(),
            clearance_policy_sha256: second_readiness.clearance_policy_sha256.clone(),
            site_snapshot_sha256: second_readiness.site_snapshot_sha256.clone(),
            site_ledger_sequence: second_readiness.site_ledger_sequence,
            site_ledger_event_sha256: second_readiness.site_ledger_event_sha256.clone(),
            site_clock_domain_id: second_readiness.site_clock_domain_id.clone(),
            checked_at_tick: second_readiness.checked_at_tick,
            valid_until_tick: second_readiness.valid_until_tick,
            committed_at_tick: 125,
            start_tick: 135,
            end_tick: 300,
            lease_valid_until_tick: 350,
        };
        let second_commit = SiteEventEnvelope {
            event_id: "reservation-2-commit".into(),
            sequence: ledger.last_sequence() + 1,
            previous_event_sha256: ledger.last_event_sha256().map(str::to_owned),
            event_sha256: digest('a'),
            actor_id: "dispatch-coordinator".into(),
            actor_role: SiteActorRole::DispatchCoordinator,
            payload: SiteEventPayload::SpaceTimeReservationCommitted {
                reservation: second_reservation,
            },
        };
        commit_dispatch_reservation(
            &mut ledger,
            &second_readiness,
            &digest('f'),
            &digest('e'),
            &digest('f'),
            125,
            second_commit,
        )
        .unwrap();

        // The original token remains valid: the later reservation was
        // checked against a snapshot containing the first reservation, and
        // it uses the same occupancy engine, policy, package and site clock.
        let request = command_request(&ledger, &readiness, &reservation, "cmd-1", 1);
        assert!(check_reservation_command_current(&ledger, &request).is_ok());

        // A station/transform change is not in the compatible-event
        // allowlist, so it invalidates the old command authorization.
        let station = SiteEventEnvelope {
            event_id: "station-after-reservation".into(),
            sequence: ledger.last_sequence() + 1,
            previous_event_sha256: ledger.last_event_sha256().map(str::to_owned),
            event_sha256: digest('b'),
            actor_id: "surveyor".into(),
            actor_role: SiteActorRole::Surveyor,
            payload: SiteEventPayload::StationQualified {
                package_sha256: readiness.package_sha256.clone(),
                station_id: "station-2".into(),
                frame_sha256: digest('f'),
                transform_sha256: digest('1'),
                calibration_sha256: digest('6'),
            },
        };
        ledger.apply(station).unwrap();
        let stale_request = command_request(&ledger, &readiness, &reservation, "cmd-2", 2);
        assert!(matches!(
            check_reservation_command_current(&ledger, &stale_request),
            Err(ReservationCommandFailure::UnrelatedLedgerMutation(event_id))
                if event_id == "station-after-reservation"
        ));
    }

    #[test]
    fn replay_guard_enforces_monotonic_commands_and_restores_durable_snapshot() {
        let mut ledger = initial_ledger();
        let (readiness, reservation) = commit_one_reservation(&mut ledger);
        let mut guard = CommandReplayGuard::new();
        let first = command_request(&ledger, &readiness, &reservation, "cmd-1", 1);
        guard.authorize(&ledger, &first).unwrap();
        assert_eq!(guard.last_sequence("reservation-1"), Some(1));

        assert_eq!(
            guard.authorize(&ledger, &first),
            Err(ReservationCommandFailure::DuplicateCommandId("cmd-1".into()))
        );
        let gap = command_request(&ledger, &readiness, &reservation, "cmd-3", 3);
        assert_eq!(
            guard.authorize(&ledger, &gap),
            Err(ReservationCommandFailure::CommandSequenceGap {
                expected: 2,
                actual: 3
            })
        );

        let persisted = guard.snapshot().clone();
        let mut restored = CommandReplayGuard::from_snapshot(persisted).unwrap();
        let second = command_request(&ledger, &readiness, &reservation, "cmd-2", 2);
        restored.authorize(&ledger, &second).unwrap();
        assert_eq!(restored.last_sequence("reservation-1"), Some(2));
    }

    #[test]
    fn command_id_reuse_with_a_different_digest_is_rejected() {
        let mut ledger = initial_ledger();
        let (readiness, reservation) = commit_one_reservation(&mut ledger);
        let mut guard = CommandReplayGuard::new();
        let first = command_request(&ledger, &readiness, &reservation, "cmd-1", 1);
        guard.authorize(&ledger, &first).unwrap();

        let mut changed = command_request(&ledger, &readiness, &reservation, "cmd-1", 2);
        changed.command_sha256 = digest('9');
        assert_eq!(
            guard.authorize(&ledger, &changed),
            Err(ReservationCommandFailure::CommandIdDigestMismatch(
                "cmd-1".into()
            ))
        );
        assert_eq!(guard.last_sequence("reservation-1"), Some(1));
    }

    #[test]
    fn malformed_persisted_replay_snapshot_is_rejected() {
        let snapshot = CommandReplaySnapshot {
            last_sequence_by_reservation: BTreeMap::from([(
                "reservation-1".into(),
                (0, 1),
            )]),
            command_digests_by_id: BTreeMap::new(),
        };
        assert_eq!(
            CommandReplayGuard::from_snapshot(snapshot).unwrap_err(),
            CommandReplayGuardError::InvalidSnapshot
        );
    }

}
