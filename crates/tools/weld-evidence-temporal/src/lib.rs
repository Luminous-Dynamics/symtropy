//! Bitemporal, purpose-scoped evidence snapshots for robotic welding decisions.
//!
//! V0 accepts synthetic contract events and integer clock ticks only. It does
//! not validate physical source clocks, sensor calibration, rights or welding
//! quality. The canonical snapshot transcript is returned for the host to
//! SHA-256 and retain; this crate does not claim to hash source bytes.

use std::collections::{HashMap, HashSet};

const TRANSCRIPT_VERSION: &[u8] = b"WELD-EVIDENCE-TEMPORAL-V0.1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DecisionPurpose {
    FastProcess,
    MediumNextPass,
    SlowQualityRequalification,
}

impl DecisionPurpose {
    fn code(self) -> u8 {
        match self {
            Self::FastProcess => 1,
            Self::MediumNextPass => 2,
            Self::SlowQualityRequalification => 3,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::FastProcess => "fast_process",
            Self::MediumNextPass => "medium_next_pass",
            Self::SlowQualityRequalification => "slow_quality_requalification",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    SyntheticContractOnly,
    MeasuredPhysical,
    Simulation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceKind {
    ProcessObservation,
    RobotTrace,
    Calibration,
    ScanMeasurement,
    RegistrationReceipt,
    ScanQualification,
    GeometryComparison,
    NdtDisposition,
    RepairRecord,
    ReinspectionResult,
    ModelTrainingArtifact,
}

impl EvidenceKind {
    fn code(self) -> u8 {
        match self {
            Self::ProcessObservation => 1,
            Self::RobotTrace => 2,
            Self::Calibration => 3,
            Self::ScanMeasurement => 4,
            Self::RegistrationReceipt => 5,
            Self::ScanQualification => 6,
            Self::GeometryComparison => 7,
            Self::NdtDisposition => 8,
            Self::RepairRecord => 9,
            Self::ReinspectionResult => 10,
            Self::ModelTrainingArtifact => 11,
        }
    }

    fn is_slow_outcome(self) -> bool {
        matches!(
            self,
            Self::NdtDisposition
                | Self::RepairRecord
                | Self::ReinspectionResult
                | Self::ModelTrainingArtifact
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceScope {
    pub workpiece_id: String,
    pub joint_revision: String,
    pub layer_id: String,
    pub predecessor_pass_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionRequest {
    pub purpose: DecisionPurpose,
    pub scope: EvidenceScope,
    pub cutoff_tick: i64,
    pub policy_revision: String,
    pub maximum_clock_uncertainty_ticks: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClockMapping {
    pub source_clock_id: String,
    pub mapping_revision: String,
    pub mapping_sha256: [u8; 32],
    pub ledger_offset_ticks: i64,
    pub offset_uncertainty_ticks: u64,
    pub maximum_source_precision_ticks: u64,
    pub valid_from_tick: i64,
    pub valid_until_tick: Option<i64>,
    pub first_received_at_tick: i64,
    pub authorized: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceEvent {
    pub event_id: String,
    pub kind: EvidenceKind,
    pub evidence_class: EvidenceClass,
    pub scope: EvidenceScope,
    pub source_clock_id: String,
    pub source_tick: i64,
    /// Conservative half-width of the source timestamp interval.
    pub timestamp_precision_ticks: u64,
    /// First time the governed ledger could use this record.
    pub first_received_at_tick: i64,
    /// Validity is in governed ledger ticks, with an exclusive upper bound.
    pub valid_from_tick: i64,
    pub valid_until_tick: Option<i64>,
    pub source_sha256: [u8; 32],
    pub transform_receipt_sha256: [u8; 32],
    pub authorized_purposes: Vec<DecisionPurpose>,
    /// For a model artifact, the latest source record used for training.
    pub model_training_cutoff_tick: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleKind {
    Supersede,
    Revoke,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleEvent {
    pub event_id: String,
    pub kind: LifecycleKind,
    pub target_event_id: String,
    /// A supersession may name its replacement. Revoke has no replacement.
    pub replacement_event_id: Option<String>,
    pub effective_at_tick: i64,
    pub first_received_at_tick: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceLedger {
    pub clock_mappings: Vec<ClockMapping>,
    pub events: Vec<EvidenceEvent>,
    pub lifecycle_events: Vec<LifecycleEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgerError {
    EmptyIdentity,
    DuplicateIdentity,
    MissingLifecycleTarget,
    MissingSupersessionReplacement,
    InvalidClockMappingInterval,
    InvalidClockMappingDigest,
    InvalidLifecycleTime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExclusionReason {
    LateArrival,
    FutureEvent,
    ClockMappingUnavailable,
    ClockMappingUnauthorized,
    ClockPrecisionTooCoarse,
    ClockBoundaryIndeterminate,
    ClockMappingOverflow,
    ClockUncertaintyAboveDecisionLimit,
    InvalidSourceDigest,
    InvalidTransformDigest,
    PhysicalEvidenceNotAdmitted,
    SimulationEvidenceNotAdmitted,
    WrongWorkpiece,
    WrongJointRevision,
    WrongLayer,
    WrongPredecessorPass,
    NotAuthorizedForPurpose,
    OutcomeLabelLeakage,
    EventNotYetValid,
    EventExpired,
    SupersededByKnownEvent,
    RevokedByKnownEvent,
    ModelTrainedOnFutureEvidence,
}

impl ExclusionReason {
    fn code(self) -> u8 {
        match self {
            Self::LateArrival => 1,
            Self::FutureEvent => 2,
            Self::ClockMappingUnavailable => 3,
            Self::ClockMappingUnauthorized => 4,
            Self::ClockPrecisionTooCoarse => 5,
            Self::ClockBoundaryIndeterminate => 6,
            Self::ClockMappingOverflow => 7,
            Self::ClockUncertaintyAboveDecisionLimit => 8,
            Self::InvalidSourceDigest => 9,
            Self::InvalidTransformDigest => 10,
            Self::PhysicalEvidenceNotAdmitted => 11,
            Self::SimulationEvidenceNotAdmitted => 12,
            Self::WrongWorkpiece => 13,
            Self::WrongJointRevision => 14,
            Self::WrongLayer => 15,
            Self::WrongPredecessorPass => 16,
            Self::NotAuthorizedForPurpose => 17,
            Self::OutcomeLabelLeakage => 18,
            Self::EventNotYetValid => 19,
            Self::EventExpired => 20,
            Self::SupersededByKnownEvent => 21,
            Self::RevokedByKnownEvent => 22,
            Self::ModelTrainedOnFutureEvidence => 23,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncludedEvidence {
    pub event_id: String,
    pub kind: EvidenceKind,
    pub mapped_earliest_tick: i64,
    pub mapped_latest_tick: i64,
    pub source_sha256: [u8; 32],
    pub transform_receipt_sha256: [u8; 32],
    pub clock_mapping_sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExcludedEvidence {
    pub event_id: String,
    pub reason: ExclusionReason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub purpose: DecisionPurpose,
    pub scope: EvidenceScope,
    pub cutoff_tick: i64,
    pub policy_revision: String,
    pub included: Vec<IncludedEvidence>,
    pub excluded: Vec<ExcludedEvidence>,
    pub pending_lifecycle_event_ids: Vec<String>,
    pub applied_lifecycle_event_ids: Vec<String>,
    pub maximum_clock_uncertainty_ticks: u64,
    /// Canonical deterministic payload. Host must SHA-256 and persist it.
    pub canonical_transcript: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SnapshotResult {
    InvalidLedger(Vec<LedgerError>),
    Snapshot(Snapshot),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MappedTime {
    earliest_tick: i64,
    latest_tick: i64,
    mapping_sha256: [u8; 32],
}

fn empty(value: &str) -> bool {
    value.trim().is_empty()
}

fn nonzero_digest(value: &[u8; 32]) -> bool {
    value.iter().any(|byte| *byte != 0)
}

fn map_event_time(
    event: &EvidenceEvent,
    mapping: &ClockMapping,
    request: &DecisionRequest,
) -> Result<MappedTime, ExclusionReason> {
    if !mapping.authorized {
        return Err(ExclusionReason::ClockMappingUnauthorized);
    }
    if mapping.first_received_at_tick > request.cutoff_tick {
        return Err(ExclusionReason::ClockMappingUnavailable);
    }
    if !nonzero_digest(&mapping.mapping_sha256) {
        return Err(ExclusionReason::ClockMappingUnavailable);
    }
    if event.timestamp_precision_ticks > mapping.maximum_source_precision_ticks {
        return Err(ExclusionReason::ClockPrecisionTooCoarse);
    }

    let uncertainty = event
        .timestamp_precision_ticks
        .checked_add(mapping.offset_uncertainty_ticks)
        .ok_or(ExclusionReason::ClockMappingOverflow)?;
    if uncertainty > request.maximum_clock_uncertainty_ticks {
        return Err(ExclusionReason::ClockUncertaintyAboveDecisionLimit);
    }

    let center = event
        .source_tick
        .checked_add(mapping.ledger_offset_ticks)
        .ok_or(ExclusionReason::ClockMappingOverflow)?;
    let uncertainty_i64 =
        i64::try_from(uncertainty).map_err(|_| ExclusionReason::ClockMappingOverflow)?;
    let earliest_tick = center
        .checked_sub(uncertainty_i64)
        .ok_or(ExclusionReason::ClockMappingOverflow)?;
    let latest_tick = center
        .checked_add(uncertainty_i64)
        .ok_or(ExclusionReason::ClockMappingOverflow)?;

    if earliest_tick < mapping.valid_from_tick
        || mapping
            .valid_until_tick
            .is_some_and(|until| latest_tick >= until)
    {
        return Err(ExclusionReason::ClockMappingUnavailable);
    }
    if earliest_tick > request.cutoff_tick {
        return Err(ExclusionReason::FutureEvent);
    }
    if latest_tick > request.cutoff_tick {
        return Err(ExclusionReason::ClockBoundaryIndeterminate);
    }

    Ok(MappedTime {
        earliest_tick,
        latest_tick,
        mapping_sha256: mapping.mapping_sha256,
    })
}

fn validate_ledger(ledger: &EvidenceLedger) -> Result<(), Vec<LedgerError>> {
    let mut errors = Vec::new();
    let mut identities = HashSet::new();
    let mut clocks = HashSet::new();
    let event_ids: HashSet<&str> = ledger
        .events
        .iter()
        .map(|event| event.event_id.as_str())
        .collect();

    for mapping in &ledger.clock_mappings {
        if empty(&mapping.source_clock_id) || empty(&mapping.mapping_revision) {
            errors.push(LedgerError::EmptyIdentity);
        }
        if !clocks.insert(mapping.source_clock_id.as_str()) {
            errors.push(LedgerError::DuplicateIdentity);
        }
        if mapping
            .valid_until_tick
            .is_some_and(|until| until <= mapping.valid_from_tick)
        {
            errors.push(LedgerError::InvalidClockMappingInterval);
        }
        if mapping.first_received_at_tick < 0 || !nonzero_digest(&mapping.mapping_sha256) {
            errors.push(LedgerError::InvalidClockMappingDigest);
        }
    }

    for event in &ledger.events {
        if empty(&event.event_id)
            || empty(&event.scope.workpiece_id)
            || empty(&event.scope.joint_revision)
            || empty(&event.scope.layer_id)
            || empty(&event.scope.predecessor_pass_id)
            || empty(&event.source_clock_id)
        {
            errors.push(LedgerError::EmptyIdentity);
        }
        if !identities.insert(event.event_id.as_str()) {
            errors.push(LedgerError::DuplicateIdentity);
        }
        if event.first_received_at_tick < 0
            || event.source_tick < 0
            || event.valid_from_tick < 0
            || event
                .valid_until_tick
                .is_some_and(|until| until <= event.valid_from_tick)
            || event
                .model_training_cutoff_tick
                .is_some_and(|tick| tick < 0)
        {
            errors.push(LedgerError::InvalidLifecycleTime);
        }
    }

    for lifecycle in &ledger.lifecycle_events {
        if empty(&lifecycle.event_id) || empty(&lifecycle.target_event_id) {
            errors.push(LedgerError::EmptyIdentity);
        }
        if !identities.insert(lifecycle.event_id.as_str()) {
            errors.push(LedgerError::DuplicateIdentity);
        }
        if !event_ids.contains(lifecycle.target_event_id.as_str()) {
            errors.push(LedgerError::MissingLifecycleTarget);
        }
        if lifecycle.first_received_at_tick < 0 || lifecycle.effective_at_tick < 0 {
            errors.push(LedgerError::InvalidLifecycleTime);
        }
        if lifecycle.kind == LifecycleKind::Supersede {
            let Some(replacement_id) = lifecycle.replacement_event_id.as_deref() else {
                errors.push(LedgerError::MissingSupersessionReplacement);
                continue;
            };
            if !event_ids.contains(replacement_id) || replacement_id == lifecycle.target_event_id {
                errors.push(LedgerError::MissingSupersessionReplacement);
            }
        } else if lifecycle.replacement_event_id.is_some() {
            errors.push(LedgerError::MissingSupersessionReplacement);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        errors.sort_by_key(|error| *error as u8);
        errors.dedup();
        Err(errors)
    }
}

fn scope_exclusion(event: &EvidenceEvent, scope: &EvidenceScope) -> Option<ExclusionReason> {
    if event.scope.workpiece_id != scope.workpiece_id {
        return Some(ExclusionReason::WrongWorkpiece);
    }
    if event.scope.joint_revision != scope.joint_revision {
        return Some(ExclusionReason::WrongJointRevision);
    }
    if event.scope.layer_id != scope.layer_id {
        return Some(ExclusionReason::WrongLayer);
    }
    if event.scope.predecessor_pass_id != scope.predecessor_pass_id {
        return Some(ExclusionReason::WrongPredecessorPass);
    }
    None
}

fn purpose_exclusion(event: &EvidenceEvent, purpose: DecisionPurpose) -> Option<ExclusionReason> {
    if !event.authorized_purposes.contains(&purpose) {
        return Some(ExclusionReason::NotAuthorizedForPurpose);
    }
    if purpose != DecisionPurpose::SlowQualityRequalification && event.kind.is_slow_outcome() {
        return Some(ExclusionReason::OutcomeLabelLeakage);
    }
    None
}

fn lifecycle_effect(
    event_id: &str,
    ledger: &EvidenceLedger,
    cutoff_tick: i64,
    pending: &mut Vec<String>,
    applied: &mut Vec<String>,
) -> Option<ExclusionReason> {
    let mut active_effects = Vec::new();
    for lifecycle in &ledger.lifecycle_events {
        if lifecycle.target_event_id != event_id {
            continue;
        }
        if lifecycle.first_received_at_tick > cutoff_tick {
            continue;
        }
        if lifecycle.effective_at_tick > cutoff_tick {
            pending.push(lifecycle.event_id.clone());
            continue;
        }
        active_effects.push(lifecycle);
    }
    active_effects.sort_by(|a, b| {
        a.effective_at_tick
            .cmp(&b.effective_at_tick)
            .then_with(|| a.first_received_at_tick.cmp(&b.first_received_at_tick))
            .then_with(|| a.event_id.cmp(&b.event_id))
    });
    if active_effects.is_empty() {
        return None;
    }
    applied.extend(active_effects.iter().map(|item| item.event_id.clone()));
    let latest = active_effects.last()?;
    Some(match latest.kind {
        LifecycleKind::Supersede => ExclusionReason::SupersededByKnownEvent,
        LifecycleKind::Revoke => ExclusionReason::RevokedByKnownEvent,
    })
}

fn put_bytes(out: &mut Vec<u8>, value: &[u8]) {
    let length = value.len() as u64;
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(value);
}

fn put_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn canonical_transcript(snapshot: &Snapshot) -> Vec<u8> {
    let mut out = Vec::new();
    put_bytes(&mut out, TRANSCRIPT_VERSION);
    out.push(snapshot.purpose.code());
    out.extend_from_slice(&snapshot.maximum_clock_uncertainty_ticks.to_be_bytes());
    put_bytes(&mut out, snapshot.scope.workpiece_id.as_bytes());
    put_bytes(&mut out, snapshot.scope.joint_revision.as_bytes());
    put_bytes(&mut out, snapshot.scope.layer_id.as_bytes());
    put_bytes(&mut out, snapshot.scope.predecessor_pass_id.as_bytes());
    put_i64(&mut out, snapshot.cutoff_tick);
    put_bytes(&mut out, snapshot.policy_revision.as_bytes());

    out.extend_from_slice(&(snapshot.included.len() as u64).to_be_bytes());
    for item in &snapshot.included {
        put_bytes(&mut out, item.event_id.as_bytes());
        out.push(item.kind.code());
        put_i64(&mut out, item.mapped_earliest_tick);
        put_i64(&mut out, item.mapped_latest_tick);
        out.extend_from_slice(&item.source_sha256);
        out.extend_from_slice(&item.transform_receipt_sha256);
        out.extend_from_slice(&item.clock_mapping_sha256);
    }

    out.extend_from_slice(&(snapshot.excluded.len() as u64).to_be_bytes());
    for item in &snapshot.excluded {
        put_bytes(&mut out, item.event_id.as_bytes());
        out.push(item.reason.code());
    }

    out.extend_from_slice(&(snapshot.pending_lifecycle_event_ids.len() as u64).to_be_bytes());
    for event_id in &snapshot.pending_lifecycle_event_ids {
        put_bytes(&mut out, event_id.as_bytes());
    }
    out.extend_from_slice(&(snapshot.applied_lifecycle_event_ids.len() as u64).to_be_bytes());
    for event_id in &snapshot.applied_lifecycle_event_ids {
        put_bytes(&mut out, event_id.as_bytes());
    }
    out
}

/// Select evidence that was measured, received, valid and authorized by T.
///
/// The returned transcript is canonical but not hashed. The caller must hash
/// these exact bytes with SHA-256 and retain the resulting digest as a receipt.
pub fn snapshot_as_of(ledger: &EvidenceLedger, request: &DecisionRequest) -> SnapshotResult {
    if let Err(errors) = validate_ledger(ledger) {
        return SnapshotResult::InvalidLedger(errors);
    }
    if request.cutoff_tick < 0
        || empty(&request.scope.workpiece_id)
        || empty(&request.scope.joint_revision)
        || empty(&request.scope.layer_id)
        || empty(&request.scope.predecessor_pass_id)
        || empty(&request.policy_revision)
    {
        return SnapshotResult::InvalidLedger(vec![LedgerError::EmptyIdentity]);
    }

    let clock_mappings: HashMap<&str, &ClockMapping> = ledger
        .clock_mappings
        .iter()
        .map(|mapping| (mapping.source_clock_id.as_str(), mapping))
        .collect();

    let mut included = Vec::new();
    let mut excluded = Vec::new();
    let mut pending_lifecycle_event_ids = Vec::new();
    let mut applied_lifecycle_event_ids = Vec::new();

    for event in &ledger.events {
        let result = (|| {
            if event.first_received_at_tick > request.cutoff_tick {
                return Err(ExclusionReason::LateArrival);
            }
            if let Some(reason) = lifecycle_effect(
                &event.event_id,
                ledger,
                request.cutoff_tick,
                &mut pending_lifecycle_event_ids,
                &mut applied_lifecycle_event_ids,
            ) {
                return Err(reason);
            }
            if event.evidence_class != EvidenceClass::SyntheticContractOnly {
                return Err(match event.evidence_class {
                    EvidenceClass::MeasuredPhysical => ExclusionReason::PhysicalEvidenceNotAdmitted,
                    EvidenceClass::Simulation => ExclusionReason::SimulationEvidenceNotAdmitted,
                    EvidenceClass::SyntheticContractOnly => unreachable!(),
                });
            }
            if let Some(reason) = scope_exclusion(event, &request.scope) {
                return Err(reason);
            }
            if let Some(reason) = purpose_exclusion(event, request.purpose) {
                return Err(reason);
            }
            if !nonzero_digest(&event.source_sha256) {
                return Err(ExclusionReason::InvalidSourceDigest);
            }
            if !nonzero_digest(&event.transform_receipt_sha256) {
                return Err(ExclusionReason::InvalidTransformDigest);
            }
            if event.valid_from_tick > request.cutoff_tick {
                return Err(ExclusionReason::EventNotYetValid);
            }
            if event
                .valid_until_tick
                .is_some_and(|until| request.cutoff_tick >= until)
            {
                return Err(ExclusionReason::EventExpired);
            }
            if event.kind == EvidenceKind::ModelTrainingArtifact
                && event
                    .model_training_cutoff_tick
                    .is_none_or(|training_cutoff| training_cutoff > request.cutoff_tick)
            {
                return Err(ExclusionReason::ModelTrainedOnFutureEvidence);
            }
            let Some(mapping) = clock_mappings.get(event.source_clock_id.as_str()) else {
                return Err(ExclusionReason::ClockMappingUnavailable);
            };
            let mapped = map_event_time(event, mapping, request)?;
            Ok(mapped)
        })();

        match result {
            Ok(mapped) => included.push(IncludedEvidence {
                event_id: event.event_id.clone(),
                kind: event.kind,
                mapped_earliest_tick: mapped.earliest_tick,
                mapped_latest_tick: mapped.latest_tick,
                source_sha256: event.source_sha256,
                transform_receipt_sha256: event.transform_receipt_sha256,
                clock_mapping_sha256: mapped.mapping_sha256,
            }),
            Err(reason) => excluded.push(ExcludedEvidence {
                event_id: event.event_id.clone(),
                reason,
            }),
        }
    }

    included.sort_by(|a, b| a.event_id.cmp(&b.event_id));
    excluded.sort_by(|a, b| a.event_id.cmp(&b.event_id));
    pending_lifecycle_event_ids.sort();
    pending_lifecycle_event_ids.dedup();
    applied_lifecycle_event_ids.sort();
    applied_lifecycle_event_ids.dedup();

    let mut snapshot = Snapshot {
        purpose: request.purpose,
        scope: request.scope.clone(),
        cutoff_tick: request.cutoff_tick,
        policy_revision: request.policy_revision.clone(),
        included,
        excluded,
        pending_lifecycle_event_ids,
        applied_lifecycle_event_ids,
        maximum_clock_uncertainty_ticks: request.maximum_clock_uncertainty_ticks,
        canonical_transcript: Vec::new(),
    };
    snapshot.canonical_transcript = canonical_transcript(&snapshot);
    SnapshotResult::Snapshot(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn scope() -> EvidenceScope {
        EvidenceScope {
            workpiece_id: "WID-SYNTH-01".to_owned(),
            joint_revision: "JOINT-R1".to_owned(),
            layer_id: "LAYER-03".to_owned(),
            predecessor_pass_id: "PASS-02".to_owned(),
        }
    }

    fn request(purpose: DecisionPurpose, cutoff: i64) -> DecisionRequest {
        DecisionRequest {
            purpose,
            scope: scope(),
            cutoff_tick: cutoff,
            policy_revision: "TEMPORAL-POLICY-R1".to_owned(),
            maximum_clock_uncertainty_ticks: 2,
        }
    }

    fn clock() -> ClockMapping {
        ClockMapping {
            source_clock_id: "PLC-A".to_owned(),
            mapping_revision: "CLOCK-MAP-R1".to_owned(),
            mapping_sha256: digest(0x91),
            ledger_offset_ticks: 0,
            offset_uncertainty_ticks: 0,
            maximum_source_precision_ticks: 2,
            valid_from_tick: 0,
            valid_until_tick: None,
            first_received_at_tick: 0,
            authorized: true,
        }
    }

    fn event(id: &str, kind: EvidenceKind) -> EvidenceEvent {
        EvidenceEvent {
            event_id: id.to_owned(),
            kind,
            evidence_class: EvidenceClass::SyntheticContractOnly,
            scope: scope(),
            source_clock_id: "PLC-A".to_owned(),
            source_tick: 10,
            timestamp_precision_ticks: 0,
            first_received_at_tick: 10,
            valid_from_tick: 0,
            valid_until_tick: None,
            source_sha256: digest(0x11),
            transform_receipt_sha256: digest(0x22),
            authorized_purposes: vec![
                DecisionPurpose::FastProcess,
                DecisionPurpose::MediumNextPass,
                DecisionPurpose::SlowQualityRequalification,
            ],
            model_training_cutoff_tick: None,
        }
    }

    fn ledger(events: Vec<EvidenceEvent>) -> EvidenceLedger {
        EvidenceLedger {
            clock_mappings: vec![clock()],
            events,
            lifecycle_events: Vec::new(),
        }
    }

    fn snapshot(ledger: &EvidenceLedger, purpose: DecisionPurpose, cutoff: i64) -> Snapshot {
        match snapshot_as_of(ledger, &request(purpose, cutoff)) {
            SnapshotResult::Snapshot(value) => value,
            SnapshotResult::InvalidLedger(errors) => {
                panic!("valid ledger rejected: {errors:?}");
            }
        }
    }

    #[test]
    fn measurement_and_receipt_at_cutoff_are_eligible() {
        let mut item = event("E1", EvidenceKind::ProcessObservation);
        item.source_tick = 10;
        item.first_received_at_tick = 10;
        let result = snapshot(&ledger(vec![item]), DecisionPurpose::FastProcess, 10);
        assert_eq!(result.included.len(), 1);
    }

    #[test]
    fn measurement_before_cutoff_but_received_after_is_excluded() {
        let mut item = event("E1", EvidenceKind::ProcessObservation);
        item.source_tick = 5;
        item.first_received_at_tick = 11;
        let result = snapshot(&ledger(vec![item]), DecisionPurpose::FastProcess, 10);
        assert_eq!(result.excluded[0].reason, ExclusionReason::LateArrival);
    }

    #[test]
    fn future_event_time_is_excluded() {
        let mut item = event("E1", EvidenceKind::ProcessObservation);
        item.source_tick = 11;
        item.first_received_at_tick = 10;
        let result = snapshot(&ledger(vec![item]), DecisionPurpose::FastProcess, 10);
        assert_eq!(result.excluded[0].reason, ExclusionReason::FutureEvent);
    }

    #[test]
    fn clock_mapping_must_be_valid_at_measurement_time_not_merely_at_decision_time() {
        let mut mapping = clock();
        mapping.valid_from_tick = 11;
        let result = snapshot(
            &EvidenceLedger {
                clock_mappings: vec![mapping],
                events: vec![event("E1", EvidenceKind::ProcessObservation)],
                lifecycle_events: Vec::new(),
            },
            DecisionPurpose::FastProcess,
            12,
        );
        assert_eq!(
            result.excluded[0].reason,
            ExclusionReason::ClockMappingUnavailable
        );
    }

    #[test]
    fn clock_mapping_received_after_cutoff_is_unavailable() {
        let mut mapping = clock();
        mapping.first_received_at_tick = 11;
        let result = snapshot(
            &EvidenceLedger {
                clock_mappings: vec![mapping],
                events: vec![event("E1", EvidenceKind::ProcessObservation)],
                lifecycle_events: Vec::new(),
            },
            DecisionPurpose::FastProcess,
            10,
        );
        assert_eq!(
            result.excluded[0].reason,
            ExclusionReason::ClockMappingUnavailable
        );
    }

    #[test]
    fn ndt_outcome_is_not_a_medium_loop_feature() {
        let result = snapshot(
            &ledger(vec![event("NDT1", EvidenceKind::NdtDisposition)]),
            DecisionPurpose::MediumNextPass,
            10,
        );
        assert_eq!(
            result.excluded[0].reason,
            ExclusionReason::OutcomeLabelLeakage
        );
    }

    #[test]
    fn ndt_outcome_is_eligible_for_later_slow_requalification() {
        let result = snapshot(
            &ledger(vec![event("NDT1", EvidenceKind::NdtDisposition)]),
            DecisionPurpose::SlowQualityRequalification,
            10,
        );
        assert_eq!(result.included.len(), 1);
    }

    #[test]
    fn supersession_known_and_effective_before_cutoff_excludes_old_event() {
        let lifecycle = LifecycleEvent {
            event_id: "L1".to_owned(),
            kind: LifecycleKind::Supersede,
            target_event_id: "SCAN-OLD".to_owned(),
            replacement_event_id: Some("SCAN-NEW".to_owned()),
            effective_at_tick: 10,
            first_received_at_tick: 10,
        };
        let result = snapshot(
            &EvidenceLedger {
                clock_mappings: vec![clock()],
                events: vec![
                    event("SCAN-OLD", EvidenceKind::ScanMeasurement),
                    event("SCAN-NEW", EvidenceKind::ScanMeasurement),
                ],
                lifecycle_events: vec![lifecycle],
            },
            DecisionPurpose::MediumNextPass,
            10,
        );
        assert_eq!(
            result.excluded[0].reason,
            ExclusionReason::SupersededByKnownEvent
        );
        assert!(
            result
                .included
                .iter()
                .any(|item| item.event_id == "SCAN-NEW")
        );
    }

    #[test]
    fn supersession_received_after_cutoff_does_not_rewrite_old_snapshot() {
        let lifecycle = LifecycleEvent {
            event_id: "L1".to_owned(),
            kind: LifecycleKind::Supersede,
            target_event_id: "SCAN-OLD".to_owned(),
            replacement_event_id: Some("SCAN-NEW".to_owned()),
            effective_at_tick: 9,
            first_received_at_tick: 11,
        };
        let result = snapshot(
            &EvidenceLedger {
                clock_mappings: vec![clock()],
                events: vec![
                    event("SCAN-OLD", EvidenceKind::ScanMeasurement),
                    event("SCAN-NEW", EvidenceKind::ScanMeasurement),
                ],
                lifecycle_events: vec![lifecycle],
            },
            DecisionPurpose::MediumNextPass,
            10,
        );
        assert!(
            result
                .included
                .iter()
                .any(|item| item.event_id == "SCAN-OLD")
        );
    }

    #[test]
    fn revocation_received_after_cutoff_does_not_rewrite_history() {
        let lifecycle = LifecycleEvent {
            event_id: "L1".to_owned(),
            kind: LifecycleKind::Revoke,
            target_event_id: "SCAN-1".to_owned(),
            replacement_event_id: None,
            effective_at_tick: 9,
            first_received_at_tick: 11,
        };
        let result = snapshot(
            &EvidenceLedger {
                clock_mappings: vec![clock()],
                events: vec![event("SCAN-1", EvidenceKind::ScanMeasurement)],
                lifecycle_events: vec![lifecycle],
            },
            DecisionPurpose::MediumNextPass,
            10,
        );
        assert_eq!(result.included.len(), 1);
    }

    #[test]
    fn revocation_known_and_effective_at_cutoff_excludes_event() {
        let lifecycle = LifecycleEvent {
            event_id: "L1".to_owned(),
            kind: LifecycleKind::Revoke,
            target_event_id: "SCAN-1".to_owned(),
            replacement_event_id: None,
            effective_at_tick: 9,
            first_received_at_tick: 10,
        };
        let result = snapshot(
            &EvidenceLedger {
                clock_mappings: vec![clock()],
                events: vec![event("SCAN-1", EvidenceKind::ScanMeasurement)],
                lifecycle_events: vec![lifecycle],
            },
            DecisionPurpose::MediumNextPass,
            10,
        );
        assert_eq!(
            result.excluded[0].reason,
            ExclusionReason::RevokedByKnownEvent
        );
    }

    #[test]
    fn event_expired_before_cutoff_is_excluded() {
        let mut item = event("CAL-1", EvidenceKind::Calibration);
        item.valid_until_tick = Some(10);
        let result = snapshot(&ledger(vec![item]), DecisionPurpose::MediumNextPass, 10);
        assert_eq!(result.excluded[0].reason, ExclusionReason::EventExpired);
    }

    #[test]
    fn calibration_not_yet_valid_is_excluded() {
        let mut item = event("CAL-1", EvidenceKind::Calibration);
        item.valid_from_tick = 11;
        let result = snapshot(&ledger(vec![item]), DecisionPurpose::MediumNextPass, 10);
        assert_eq!(result.excluded[0].reason, ExclusionReason::EventNotYetValid);
    }

    #[test]
    fn duplicate_event_identity_rejects_ledger() {
        let result = snapshot_as_of(
            &ledger(vec![
                event("DUP", EvidenceKind::ProcessObservation),
                event("DUP", EvidenceKind::RobotTrace),
            ]),
            &request(DecisionPurpose::FastProcess, 10),
        );
        assert_eq!(
            result,
            SnapshotResult::InvalidLedger(vec![LedgerError::DuplicateIdentity])
        );
    }

    #[test]
    fn missing_source_digest_is_excluded() {
        let mut item = event("E1", EvidenceKind::ProcessObservation);
        item.source_sha256 = [0; 32];
        let result = snapshot(&ledger(vec![item]), DecisionPurpose::FastProcess, 10);
        assert_eq!(
            result.excluded[0].reason,
            ExclusionReason::InvalidSourceDigest
        );
    }

    #[test]
    fn missing_transform_digest_is_excluded() {
        let mut item = event("E1", EvidenceKind::ProcessObservation);
        item.transform_receipt_sha256 = [0; 32];
        let result = snapshot(&ledger(vec![item]), DecisionPurpose::FastProcess, 10);
        assert_eq!(
            result.excluded[0].reason,
            ExclusionReason::InvalidTransformDigest
        );
    }

    #[test]
    fn wrong_workpiece_is_excluded() {
        let mut item = event("E1", EvidenceKind::ProcessObservation);
        item.scope.workpiece_id = "WID-OTHER".to_owned();
        let result = snapshot(&ledger(vec![item]), DecisionPurpose::FastProcess, 10);
        assert_eq!(result.excluded[0].reason, ExclusionReason::WrongWorkpiece);
    }

    #[test]
    fn wrong_joint_revision_is_excluded() {
        let mut item = event("E1", EvidenceKind::ProcessObservation);
        item.scope.joint_revision = "JOINT-R2".to_owned();
        let result = snapshot(&ledger(vec![item]), DecisionPurpose::FastProcess, 10);
        assert_eq!(
            result.excluded[0].reason,
            ExclusionReason::WrongJointRevision
        );
    }

    #[test]
    fn wrong_layer_is_excluded() {
        let mut item = event("E1", EvidenceKind::ProcessObservation);
        item.scope.layer_id = "LAYER-04".to_owned();
        let result = snapshot(&ledger(vec![item]), DecisionPurpose::FastProcess, 10);
        assert_eq!(result.excluded[0].reason, ExclusionReason::WrongLayer);
    }

    #[test]
    fn wrong_predecessor_pass_is_excluded() {
        let mut item = event("E1", EvidenceKind::ProcessObservation);
        item.scope.predecessor_pass_id = "PASS-01".to_owned();
        let result = snapshot(&ledger(vec![item]), DecisionPurpose::MediumNextPass, 10);
        assert_eq!(
            result.excluded[0].reason,
            ExclusionReason::WrongPredecessorPass
        );
    }

    #[test]
    fn unmapped_clock_is_excluded() {
        let mut item = event("E1", EvidenceKind::ProcessObservation);
        item.source_clock_id = "UNKNOWN-CLOCK".to_owned();
        let result = snapshot(&ledger(vec![item]), DecisionPurpose::FastProcess, 10);
        assert_eq!(
            result.excluded[0].reason,
            ExclusionReason::ClockMappingUnavailable
        );
    }

    #[test]
    fn input_permutation_preserves_membership_and_transcript() {
        let first = event("B", EvidenceKind::ProcessObservation);
        let second = event("A", EvidenceKind::RobotTrace);
        let left = snapshot(
            &ledger(vec![first.clone(), second.clone()]),
            DecisionPurpose::FastProcess,
            10,
        );
        let right = snapshot(
            &ledger(vec![second, first]),
            DecisionPurpose::FastProcess,
            10,
        );
        assert_eq!(left.included, right.included);
        assert_eq!(left.canonical_transcript, right.canonical_transcript);
    }

    #[test]
    fn model_training_artifact_is_not_a_medium_loop_feature() {
        let mut item = event("MODEL-1", EvidenceKind::ModelTrainingArtifact);
        item.model_training_cutoff_tick = Some(10);
        let result = snapshot(&ledger(vec![item]), DecisionPurpose::MediumNextPass, 10);
        assert_eq!(
            result.excluded[0].reason,
            ExclusionReason::OutcomeLabelLeakage
        );
    }

    #[test]
    fn model_trained_on_future_evidence_is_excluded_from_slow_replay() {
        let mut item = event("MODEL-1", EvidenceKind::ModelTrainingArtifact);
        item.model_training_cutoff_tick = Some(11);
        let result = snapshot(
            &ledger(vec![item]),
            DecisionPurpose::SlowQualityRequalification,
            10,
        );
        assert_eq!(
            result.excluded[0].reason,
            ExclusionReason::ModelTrainedOnFutureEvidence
        );
    }

    #[test]
    fn receipt_time_distinguishes_equal_measurement_timestamps() {
        let mut early = event("EARLY", EvidenceKind::ProcessObservation);
        early.source_tick = 5;
        early.first_received_at_tick = 9;
        let mut late = event("LATE", EvidenceKind::ProcessObservation);
        late.source_tick = 5;
        late.first_received_at_tick = 11;
        let result = snapshot(&ledger(vec![early, late]), DecisionPurpose::FastProcess, 10);
        assert_eq!(result.included.len(), 1);
        assert_eq!(result.included[0].event_id, "EARLY");
        assert_eq!(result.excluded[0].reason, ExclusionReason::LateArrival);
    }

    #[test]
    fn timestamp_precision_coarser_than_mapping_limit_is_excluded() {
        let mut item = event("E1", EvidenceKind::ProcessObservation);
        item.timestamp_precision_ticks = 3;
        let result = snapshot(&ledger(vec![item]), DecisionPurpose::FastProcess, 10);
        assert_eq!(
            result.excluded[0].reason,
            ExclusionReason::ClockPrecisionTooCoarse
        );
    }

    #[test]
    fn timestamp_interval_straddling_cutoff_is_indeterminate() {
        let mut item = event("E1", EvidenceKind::ProcessObservation);
        item.source_tick = 10;
        item.timestamp_precision_ticks = 1;
        let result = snapshot(&ledger(vec![item]), DecisionPurpose::FastProcess, 10);
        assert_eq!(
            result.excluded[0].reason,
            ExclusionReason::ClockBoundaryIndeterminate
        );
    }

    #[test]
    fn late_event_insertion_does_not_change_prior_snapshot() {
        let earlier = event("EARLY", EvidenceKind::ProcessObservation);
        let mut late = event("LATE", EvidenceKind::ProcessObservation);
        late.source_tick = 4;
        late.first_received_at_tick = 11;
        let baseline = snapshot(
            &ledger(vec![earlier.clone()]),
            DecisionPurpose::FastProcess,
            10,
        );
        let with_late = snapshot(
            &ledger(vec![late, earlier]),
            DecisionPurpose::FastProcess,
            10,
        );
        assert_eq!(baseline.included, with_late.included);
        assert_eq!(
            baseline.canonical_transcript,
            with_late.canonical_transcript
        );
    }

    #[test]
    fn applied_lifecycle_ids_are_bound_into_the_canonical_transcript() {
        let lifecycle = LifecycleEvent {
            event_id: "L1".to_owned(),
            kind: LifecycleKind::Revoke,
            target_event_id: "SCAN-1".to_owned(),
            replacement_event_id: None,
            effective_at_tick: 9,
            first_received_at_tick: 10,
        };
        let result = snapshot(
            &EvidenceLedger {
                clock_mappings: vec![clock()],
                events: vec![event("SCAN-1", EvidenceKind::ScanMeasurement)],
                lifecycle_events: vec![lifecycle.clone()],
            },
            DecisionPurpose::MediumNextPass,
            10,
        );
        assert_eq!(result.applied_lifecycle_event_ids, vec!["L1"]);

        let mut changed = lifecycle;
        changed.event_id = "L2".to_owned();
        let other = snapshot(
            &EvidenceLedger {
                clock_mappings: vec![clock()],
                events: vec![event("SCAN-1", EvidenceKind::ScanMeasurement)],
                lifecycle_events: vec![changed],
            },
            DecisionPurpose::MediumNextPass,
            10,
        );
        assert_ne!(result.canonical_transcript, other.canonical_transcript);
    }

    #[test]
    fn model_training_artifact_without_training_cutoff_is_excluded() {
        let item = event("MODEL-1", EvidenceKind::ModelTrainingArtifact);
        let result = snapshot(
            &ledger(vec![item]),
            DecisionPurpose::SlowQualityRequalification,
            10,
        );
        assert_eq!(
            result.excluded[0].reason,
            ExclusionReason::ModelTrainedOnFutureEvidence
        );
    }

    #[test]
    fn unauthorized_purpose_is_excluded() {
        let mut item = event("E1", EvidenceKind::ProcessObservation);
        item.authorized_purposes = vec![DecisionPurpose::SlowQualityRequalification];
        let result = snapshot(&ledger(vec![item]), DecisionPurpose::MediumNextPass, 10);
        assert_eq!(
            result.excluded[0].reason,
            ExclusionReason::NotAuthorizedForPurpose
        );
    }

    #[test]
    fn known_but_not_yet_effective_lifecycle_is_recorded_not_applied() {
        let lifecycle = LifecycleEvent {
            event_id: "L1".to_owned(),
            kind: LifecycleKind::Supersede,
            target_event_id: "SCAN-OLD".to_owned(),
            replacement_event_id: Some("SCAN-NEW".to_owned()),
            effective_at_tick: 12,
            first_received_at_tick: 10,
        };
        let result = snapshot(
            &EvidenceLedger {
                clock_mappings: vec![clock()],
                events: vec![
                    event("SCAN-OLD", EvidenceKind::ScanMeasurement),
                    event("SCAN-NEW", EvidenceKind::ScanMeasurement),
                ],
                lifecycle_events: vec![lifecycle],
            },
            DecisionPurpose::MediumNextPass,
            10,
        );
        assert!(
            result
                .included
                .iter()
                .any(|item| item.event_id == "SCAN-OLD")
        );
        assert_eq!(result.pending_lifecycle_event_ids, vec!["L1"]);
    }

    #[test]
    fn clock_mapping_overflow_is_excluded_not_wrapped() {
        let mut item = event("E1", EvidenceKind::ProcessObservation);
        item.source_tick = i64::MAX;
        let result = snapshot(&ledger(vec![item]), DecisionPurpose::FastProcess, 10);
        assert_eq!(
            result.excluded[0].reason,
            ExclusionReason::ClockMappingOverflow
        );
    }
}
