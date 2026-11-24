// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fail-closed source-claim admission for maritime operational evidence.
//!
//! The module distinguishes source-reported status, plans, inventory windows,
//! API interoperability and independent physical/receiver observations.
//! DCSA conformance, terminal plans and status snapshots are never silently
//! promoted into independent physical truth.
//!
//! All fixtures and receipts in this module are synthetic. The module validates
//! the declared evidence envelope and claim scope; it does not prove that a
//! caller's "independent" witness is genuinely independent.

use std::fmt::Write as FmtWrite;

const SHA256_HEX_LENGTH: usize = 64;
const FIXTURE_AS_OF: i64 = 2_000_000_000;
const FIXTURE_PUBLICATION_TIME: i64 = 1_900_000_000;
const FIXTURE_RETRIEVAL_TIME: i64 = 1_900_000_010;
const FIXTURE_EVENT_TIME: i64 = 1_899_999_900;
const FIXTURE_DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
#[cfg(test)]
const FIXTURE_CUSTODY_DIGEST: &str =
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactClass {
    OfficialVesselUpdate,
    OfficialFirmBerthPlan,
    OfficialExportStackWindow,
    OfficialImportStorageSnapshot,
    OfficialTerminalNotice,
    ThirdPartyDerivedAggregator,
    DcsaApiConformanceReport,
    IndependentPhysicalObservation,
    IndependentReceiverAcceptance,
}

impl ArtifactClass {
    pub const fn label(self) -> &'static str {
        match self {
            Self::OfficialVesselUpdate => "OFFICIAL_VESSEL_UPDATE",
            Self::OfficialFirmBerthPlan => "OFFICIAL_FIRM_BERTH_PLAN",
            Self::OfficialExportStackWindow => "OFFICIAL_EXPORT_STACK_WINDOW",
            Self::OfficialImportStorageSnapshot => "OFFICIAL_IMPORT_STORAGE",
            Self::OfficialTerminalNotice => "OFFICIAL_TERMINAL_NOTICE",
            Self::ThirdPartyDerivedAggregator => "THIRD_PARTY_DERIVED_AGGREGATOR",
            Self::DcsaApiConformanceReport => "DCSA_API_CONFORMANCE_REPORT",
            Self::IndependentPhysicalObservation => "INDEPENDENT_PHYSICAL_OBSERVATION",
            Self::IndependentReceiverAcceptance => "INDEPENDENT_RECEIVER_ACCEPTANCE",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaimKind {
    SourceReportedVesselStatus,
    PlannedBerthWindow,
    ExportCargoGateWindow,
    ImportStorageState,
    SourceReportedTerminalNotice,
    ThirdPartyDerivedStatus,
    ApiInteroperabilityConformance,
    ObservedBerthOccupancy,
    ObservedCargoMovement,
    ReceiverAcceptedDelivery,
}

impl ClaimKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SourceReportedVesselStatus => "SOURCE_REPORTED_VESSEL_STATUS",
            Self::PlannedBerthWindow => "PLANNED_BERTH_WINDOW",
            Self::ExportCargoGateWindow => "EXPORT_CARGO_GATE_WINDOW",
            Self::ImportStorageState => "IMPORT_STORAGE_STATE",
            Self::SourceReportedTerminalNotice => "SOURCE_REPORTED_TERMINAL_NOTICE",
            Self::ThirdPartyDerivedStatus => "THIRD_PARTY_DERIVED_STATUS",
            Self::ApiInteroperabilityConformance => "API_INTEROPERABILITY_CONFORMANCE",
            Self::ObservedBerthOccupancy => "OBSERVED_BERTH_OCCUPANCY",
            Self::ObservedCargoMovement => "OBSERVED_CARGO_MOVEMENT",
            Self::ReceiverAcceptedDelivery => "RECEIVER_ACCEPTED_DELIVERY",
        }
    }

    const fn requires_physical_observation(self) -> bool {
        matches!(
            self,
            Self::ObservedBerthOccupancy
                | Self::ObservedCargoMovement
                | Self::ReceiverAcceptedDelivery
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmittedEvidenceClass {
    SourceReported,
    PlanOnly,
    CargoWindowOnly,
    InventorySnapshotOnly,
    NoticeOnly,
    ThirdPartyDerivedOnly,
    InteroperabilityOnly,
    IndependentPhysicalObservation,
    IndependentReceiverAcceptance,
}

impl AdmittedEvidenceClass {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SourceReported => "SOURCE_REPORTED_ONLY",
            Self::PlanOnly => "PLAN_ONLY",
            Self::CargoWindowOnly => "CARGO_WINDOW_ONLY",
            Self::InventorySnapshotOnly => "INVENTORY_SNAPSHOT_ONLY",
            Self::NoticeOnly => "NOTICE_ONLY",
            Self::ThirdPartyDerivedOnly => "THIRD_PARTY_DERIVED_ONLY",
            Self::InteroperabilityOnly => "INTEROPERABILITY_ONLY",
            Self::IndependentPhysicalObservation => "PHYSICAL_OBSERVATION",
            Self::IndependentReceiverAcceptance => "RECEIVER_ACCEPTANCE",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionBlock {
    SourceClassDoesNotSupportClaim,
    PublicationTimeUnknown,
    EvidenceUnavailableAtAsOfCutoff,
    EventNotYetOccurredAtAsOfCutoff,
    EventOutsideDeclaredValidity,
    IndependenceEvidenceReferenceMissing,
    ReceiverCustodyDigestMissing,
    EventTimeUnknown,
    ClaimSubjectIdentityMissing,
    SourceRecordIdentityMissing,
}

impl AdmissionBlock {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SourceClassDoesNotSupportClaim => "SOURCE_CLASS_DOES_NOT_SUPPORT_CLAIM",
            Self::PublicationTimeUnknown => "PUBLICATION_TIME_UNKNOWN",
            Self::EvidenceUnavailableAtAsOfCutoff => "EVIDENCE_UNAVAILABLE_AT_AS_OF_CUTOFF",
            Self::EventNotYetOccurredAtAsOfCutoff => "EVENT_NOT_YET_OCCURRED_AT_AS_OF_CUTOFF",
            Self::EventOutsideDeclaredValidity => "EVENT_OUTSIDE_DECLARED_VALIDITY",
            Self::IndependenceEvidenceReferenceMissing => "INDEPENDENCE_REFERENCE_MISSING",
            Self::ReceiverCustodyDigestMissing => "RECEIVER_CUSTODY_DIGEST_MISSING",
            Self::EventTimeUnknown => "EVENT_TIME_UNKNOWN",
            Self::ClaimSubjectIdentityMissing => "CLAIM_SUBJECT_IDENTITY_MISSING",
            Self::SourceRecordIdentityMissing => "SOURCE_RECORD_IDENTITY_MISSING",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactValidationError {
    EmptyArtifactIdentity,
    InvalidRawSha256,
    InvalidCustodySha256,
    NegativeTimestamp,
    PublicationAfterRetrieval,
    InvalidValidityInterval,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactEnvelope {
    /// Stable identity for the source artifact, not an event ID.
    pub artifact_id: String,
    pub source_owner: String,
    pub source_revision: String,
    /// Digest of the exact immutable raw bytes.
    pub raw_sha256: String,
    /// When this copy was retrieved by the evidence system.
    pub retrieved_at_unix: i64,
    /// When the source says it published this artifact; None means unknown.
    pub published_at_unix: Option<i64>,
    /// Source-asserted valid-time interval, distinct from publication/retrieval.
    pub valid_from_unix: Option<i64>,
    pub valid_until_unix: Option<i64>,
    /// Optional digest for a receiver-side custody/acceptance record.
    pub receiver_custody_sha256: Option<String>,
    /// Caller-supplied reference to review of source/witness independence.
    /// This is a reference, not proof that independence has been established.
    pub independence_evidence_ref: Option<String>,
}

impl ArtifactEnvelope {
    pub fn validate(&self) -> Result<(), ArtifactValidationError> {
        if self.artifact_id.trim().is_empty()
            || self.source_owner.trim().is_empty()
            || self.source_revision.trim().is_empty()
        {
            return Err(ArtifactValidationError::EmptyArtifactIdentity);
        }
        if !is_sha256(&self.raw_sha256) {
            return Err(ArtifactValidationError::InvalidRawSha256);
        }
        if self
            .receiver_custody_sha256
            .as_deref()
            .is_some_and(|digest| !is_sha256(digest))
        {
            return Err(ArtifactValidationError::InvalidCustodySha256);
        }
        if self.retrieved_at_unix < 0
            || self.published_at_unix.is_some_and(|time| time < 0)
            || self.valid_from_unix.is_some_and(|time| time < 0)
            || self.valid_until_unix.is_some_and(|time| time < 0)
        {
            return Err(ArtifactValidationError::NegativeTimestamp);
        }
        if self
            .published_at_unix
            .is_some_and(|published| published > self.retrieved_at_unix)
        {
            return Err(ArtifactValidationError::PublicationAfterRetrieval);
        }
        if self
            .valid_from_unix
            .zip(self.valid_until_unix)
            .is_some_and(|(start, end)| end < start)
        {
            return Err(ArtifactValidationError::InvalidValidityInterval);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactSnapshot {
    pub snapshot_id: String,
    /// Stable logical family ID; e.g. one day's export-stack source.
    pub artifact_family_id: String,
    /// Monotonic revision sequence within the family, beginning at one.
    pub revision_sequence: u64,
    /// The immediate prior immutable snapshot in the same source family.
    pub supersedes_snapshot_id: Option<String>,
    pub envelope: ArtifactEnvelope,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LedgerAppendError {
    EmptySnapshotIdentity,
    DuplicateSnapshotId,
    InvalidEnvelope(ArtifactValidationError),
    RootRevisionMustStartAtOne,
    SupersededSnapshotUnknown,
    SupersessionFamilyMismatch,
    SupersessionMustBeLatest,
    RevisionSequenceMustBeContiguous,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AppendOnlyArtifactLedger {
    snapshots: Vec<ArtifactSnapshot>,
}

impl AppendOnlyArtifactLedger {
    pub fn snapshots(&self) -> &[ArtifactSnapshot] {
        &self.snapshots
    }

    /// Appends a new immutable revision. There is no update or delete API.
    pub fn append(&mut self, snapshot: ArtifactSnapshot) -> Result<(), LedgerAppendError> {
        if snapshot.snapshot_id.trim().is_empty()
            || snapshot.artifact_family_id.trim().is_empty()
            || snapshot.revision_sequence == 0
        {
            return Err(LedgerAppendError::EmptySnapshotIdentity);
        }
        if self
            .snapshots
            .iter()
            .any(|existing| existing.snapshot_id == snapshot.snapshot_id)
        {
            return Err(LedgerAppendError::DuplicateSnapshotId);
        }
        if let Some(parent_id) = snapshot.supersedes_snapshot_id.as_deref() {
            let parent = self
                .snapshots
                .iter()
                .find(|existing| existing.snapshot_id == parent_id)
                .ok_or(LedgerAppendError::SupersededSnapshotUnknown)?;
            if parent.artifact_family_id != snapshot.artifact_family_id {
                return Err(LedgerAppendError::SupersessionFamilyMismatch);
            }
            if self.snapshots.iter().any(|existing| {
                existing.artifact_family_id == snapshot.artifact_family_id
                    && existing.revision_sequence > parent.revision_sequence
            }) {
                return Err(LedgerAppendError::SupersessionMustBeLatest);
            }
            if snapshot.revision_sequence != parent.revision_sequence + 1 {
                return Err(LedgerAppendError::RevisionSequenceMustBeContiguous);
            }
        } else if snapshot.revision_sequence != 1
            || self
                .snapshots
                .iter()
                .any(|existing| existing.artifact_family_id == snapshot.artifact_family_id)
        {
            return Err(LedgerAppendError::RootRevisionMustStartAtOne);
        }

        snapshot
            .envelope
            .validate()
            .map_err(LedgerAppendError::InvalidEnvelope)?;
        self.snapshots.push(snapshot);
        Ok(())
    }

    /// Returns the highest revision that was both published and retrieved by
    /// the historical cutoff. Later corrections cannot rewrite an old replay.
    pub fn latest_available_as_of(
        &self,
        artifact_family_id: &str,
        as_of_unix: i64,
    ) -> Option<&ArtifactSnapshot> {
        self.snapshots
            .iter()
            .filter(|snapshot| snapshot.artifact_family_id == artifact_family_id)
            .filter(|snapshot| {
                snapshot
                    .envelope
                    .published_at_unix
                    .is_some_and(|published| published <= as_of_unix)
                    && snapshot.envelope.retrieved_at_unix <= as_of_unix
            })
            .max_by_key(|snapshot| snapshot.revision_sequence)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceClaim {
    pub artifact_class: ArtifactClass,
    pub artifact: ArtifactEnvelope,
    pub claim_kind: ClaimKind,
    /// Canonical identity of the vessel, container, visit or other claim subject.
    pub subject_id: String,
    /// Identity of the source-native record supporting this particular assertion.
    pub source_record_id: String,
    /// The evaluation cutoff for a historical/as-of reconstruction.
    pub as_of_unix: i64,
    /// Event time being asserted. It is not publication time or retrieval time.
    pub event_time_unix: Option<i64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionDecision {
    Admitted(AdmittedEvidenceClass),
    Blocked(AdmissionBlock),
}

impl AdmissionDecision {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Admitted(_) => "ADMITTED_WITH_CLAIM_CEILING",
            Self::Blocked(_) => "BLOCKED",
        }
    }

    pub const fn evidence_class(self) -> &'static str {
        match self {
            Self::Admitted(class) => class.label(),
            Self::Blocked(_) => "NONE",
        }
    }

    pub const fn reason(self) -> &'static str {
        match self {
            Self::Admitted(_) => "CLAIM_WITHIN_SOURCE_SCOPE",
            Self::Blocked(reason) => reason.label(),
        }
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == SHA256_HEX_LENGTH && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn supported_evidence(
    artifact_class: ArtifactClass,
    claim_kind: ClaimKind,
) -> Option<AdmittedEvidenceClass> {
    match (artifact_class, claim_kind) {
        (ArtifactClass::OfficialVesselUpdate, ClaimKind::SourceReportedVesselStatus) => {
            Some(AdmittedEvidenceClass::SourceReported)
        }
        (ArtifactClass::OfficialFirmBerthPlan, ClaimKind::PlannedBerthWindow) => {
            Some(AdmittedEvidenceClass::PlanOnly)
        }
        (ArtifactClass::OfficialExportStackWindow, ClaimKind::ExportCargoGateWindow) => {
            Some(AdmittedEvidenceClass::CargoWindowOnly)
        }
        (ArtifactClass::OfficialImportStorageSnapshot, ClaimKind::ImportStorageState) => {
            Some(AdmittedEvidenceClass::InventorySnapshotOnly)
        }
        (ArtifactClass::OfficialTerminalNotice, ClaimKind::SourceReportedTerminalNotice) => {
            Some(AdmittedEvidenceClass::NoticeOnly)
        }
        (ArtifactClass::ThirdPartyDerivedAggregator, ClaimKind::ThirdPartyDerivedStatus) => {
            Some(AdmittedEvidenceClass::ThirdPartyDerivedOnly)
        }
        (ArtifactClass::DcsaApiConformanceReport, ClaimKind::ApiInteroperabilityConformance) => {
            Some(AdmittedEvidenceClass::InteroperabilityOnly)
        }
        (
            ArtifactClass::IndependentPhysicalObservation,
            ClaimKind::ObservedBerthOccupancy | ClaimKind::ObservedCargoMovement,
        ) => Some(AdmittedEvidenceClass::IndependentPhysicalObservation),
        (ArtifactClass::IndependentReceiverAcceptance, ClaimKind::ReceiverAcceptedDelivery) => {
            Some(AdmittedEvidenceClass::IndependentReceiverAcceptance)
        }
        _ => None,
    }
}

/// Determines whether a source artifact may support a narrowly typed claim.
///
/// A successful result only admits the claim at its stated evidence class.
/// It does not prove the source/witness independence reference, physical
/// completeness, event identity, cargo custody continuity or causal impact.
pub fn admit_claim(claim: &SourceClaim) -> Result<AdmissionDecision, ArtifactValidationError> {
    claim.artifact.validate()?;

    if claim.subject_id.trim().is_empty() {
        return Ok(AdmissionDecision::Blocked(
            AdmissionBlock::ClaimSubjectIdentityMissing,
        ));
    }
    if claim.source_record_id.trim().is_empty() {
        return Ok(AdmissionDecision::Blocked(
            AdmissionBlock::SourceRecordIdentityMissing,
        ));
    }
    if claim.as_of_unix < 0 {
        return Err(ArtifactValidationError::NegativeTimestamp);
    }
    let Some(published_at) = claim.artifact.published_at_unix else {
        return Ok(AdmissionDecision::Blocked(
            AdmissionBlock::PublicationTimeUnknown,
        ));
    };
    if published_at > claim.as_of_unix || claim.artifact.retrieved_at_unix > claim.as_of_unix {
        return Ok(AdmissionDecision::Blocked(
            AdmissionBlock::EvidenceUnavailableAtAsOfCutoff,
        ));
    }

    let Some(evidence_class) = supported_evidence(claim.artifact_class, claim.claim_kind) else {
        return Ok(AdmissionDecision::Blocked(
            AdmissionBlock::SourceClassDoesNotSupportClaim,
        ));
    };

    if claim.claim_kind.requires_physical_observation() && claim.event_time_unix.is_none() {
        return Ok(AdmissionDecision::Blocked(AdmissionBlock::EventTimeUnknown));
    }
    if let Some(event_time) = claim.event_time_unix {
        if event_time < 0 {
            return Err(ArtifactValidationError::NegativeTimestamp);
        }
        if claim
            .artifact
            .valid_from_unix
            .is_some_and(|valid_from| event_time < valid_from)
            || claim
                .artifact
                .valid_until_unix
                .is_some_and(|valid_until| event_time > valid_until)
        {
            return Ok(AdmissionDecision::Blocked(
                AdmissionBlock::EventOutsideDeclaredValidity,
            ));
        }
        if claim.claim_kind.requires_physical_observation() && event_time > claim.as_of_unix {
            return Ok(AdmissionDecision::Blocked(
                AdmissionBlock::EventNotYetOccurredAtAsOfCutoff,
            ));
        }
    }

    match evidence_class {
        AdmittedEvidenceClass::IndependentPhysicalObservation => {
            if claim
                .artifact
                .independence_evidence_ref
                .as_deref()
                .is_none_or(str::is_empty)
            {
                return Ok(AdmissionDecision::Blocked(
                    AdmissionBlock::IndependenceEvidenceReferenceMissing,
                ));
            }
        }
        AdmittedEvidenceClass::IndependentReceiverAcceptance => {
            if claim
                .artifact
                .independence_evidence_ref
                .as_deref()
                .is_none_or(str::is_empty)
            {
                return Ok(AdmissionDecision::Blocked(
                    AdmissionBlock::IndependenceEvidenceReferenceMissing,
                ));
            }
            if claim.artifact.receiver_custody_sha256.is_none() {
                return Ok(AdmissionDecision::Blocked(
                    AdmissionBlock::ReceiverCustodyDigestMissing,
                ));
            }
        }
        _ => {}
    }

    Ok(AdmissionDecision::Admitted(evidence_class))
}

#[derive(Clone, Copy)]
struct BenchmarkCase {
    id: &'static str,
    artifact_class: ArtifactClass,
    claim_kind: ClaimKind,
    expected: &'static str,
    publication_time: Option<i64>,
    retrieval_time: i64,
    independence_ref: Option<&'static str>,
    custody_digest: Option<&'static str>,
    event_time: Option<i64>,
}

fn benchmark_cases() -> [BenchmarkCase; 21] {
    [
        BenchmarkCase {
            id: "dgt_vessel_status_as_reported",
            artifact_class: ArtifactClass::OfficialVesselUpdate,
            claim_kind: ClaimKind::SourceReportedVesselStatus,
            expected: "SOURCE_REPORTED_ONLY",
            publication_time: Some(FIXTURE_PUBLICATION_TIME),
            retrieval_time: FIXTURE_RETRIEVAL_TIME,
            independence_ref: None,
            custody_digest: None,
            event_time: Some(FIXTURE_EVENT_TIME),
        },
        BenchmarkCase {
            id: "dgt_vessel_status_to_observed_berth_negative",
            artifact_class: ArtifactClass::OfficialVesselUpdate,
            claim_kind: ClaimKind::ObservedBerthOccupancy,
            expected: "SOURCE_CLASS_DOES_NOT_SUPPORT_CLAIM",
            publication_time: Some(FIXTURE_PUBLICATION_TIME),
            retrieval_time: FIXTURE_RETRIEVAL_TIME,
            independence_ref: None,
            custody_digest: None,
            event_time: Some(FIXTURE_EVENT_TIME),
        },
        BenchmarkCase {
            id: "dgt_firm_plan_as_plan",
            artifact_class: ArtifactClass::OfficialFirmBerthPlan,
            claim_kind: ClaimKind::PlannedBerthWindow,
            expected: "PLAN_ONLY",
            publication_time: Some(FIXTURE_PUBLICATION_TIME),
            retrieval_time: FIXTURE_RETRIEVAL_TIME,
            independence_ref: None,
            custody_digest: None,
            event_time: Some(FIXTURE_EVENT_TIME),
        },
        BenchmarkCase {
            id: "dgt_firm_plan_to_observed_berth_negative",
            artifact_class: ArtifactClass::OfficialFirmBerthPlan,
            claim_kind: ClaimKind::ObservedBerthOccupancy,
            expected: "SOURCE_CLASS_DOES_NOT_SUPPORT_CLAIM",
            publication_time: Some(FIXTURE_PUBLICATION_TIME),
            retrieval_time: FIXTURE_RETRIEVAL_TIME,
            independence_ref: None,
            custody_digest: None,
            event_time: Some(FIXTURE_EVENT_TIME),
        },
        BenchmarkCase {
            id: "dgt_stack_window_as_gate_window",
            artifact_class: ArtifactClass::OfficialExportStackWindow,
            claim_kind: ClaimKind::ExportCargoGateWindow,
            expected: "CARGO_WINDOW_ONLY",
            publication_time: Some(FIXTURE_PUBLICATION_TIME),
            retrieval_time: FIXTURE_RETRIEVAL_TIME,
            independence_ref: None,
            custody_digest: None,
            event_time: None,
        },
        BenchmarkCase {
            id: "dgt_stack_window_to_loaded_negative",
            artifact_class: ArtifactClass::OfficialExportStackWindow,
            claim_kind: ClaimKind::ObservedCargoMovement,
            expected: "SOURCE_CLASS_DOES_NOT_SUPPORT_CLAIM",
            publication_time: Some(FIXTURE_PUBLICATION_TIME),
            retrieval_time: FIXTURE_RETRIEVAL_TIME,
            independence_ref: None,
            custody_digest: None,
            event_time: Some(FIXTURE_EVENT_TIME),
        },
        BenchmarkCase {
            id: "dgt_import_storage_as_inventory",
            artifact_class: ArtifactClass::OfficialImportStorageSnapshot,
            claim_kind: ClaimKind::ImportStorageState,
            expected: "INVENTORY_SNAPSHOT_ONLY",
            publication_time: Some(FIXTURE_PUBLICATION_TIME),
            retrieval_time: FIXTURE_RETRIEVAL_TIME,
            independence_ref: None,
            custody_digest: None,
            event_time: None,
        },
        BenchmarkCase {
            id: "dgt_import_storage_to_receiver_delivery_negative",
            artifact_class: ArtifactClass::OfficialImportStorageSnapshot,
            claim_kind: ClaimKind::ReceiverAcceptedDelivery,
            expected: "SOURCE_CLASS_DOES_NOT_SUPPORT_CLAIM",
            publication_time: Some(FIXTURE_PUBLICATION_TIME),
            retrieval_time: FIXTURE_RETRIEVAL_TIME,
            independence_ref: None,
            custody_digest: None,
            event_time: None,
        },
        BenchmarkCase {
            id: "dues_derived_status_only",
            artifact_class: ArtifactClass::ThirdPartyDerivedAggregator,
            claim_kind: ClaimKind::ThirdPartyDerivedStatus,
            expected: "THIRD_PARTY_DERIVED_ONLY",
            publication_time: Some(FIXTURE_PUBLICATION_TIME),
            retrieval_time: FIXTURE_RETRIEVAL_TIME,
            independence_ref: None,
            custody_digest: None,
            event_time: None,
        },
        BenchmarkCase {
            id: "dues_to_independent_physical_negative",
            artifact_class: ArtifactClass::ThirdPartyDerivedAggregator,
            claim_kind: ClaimKind::ObservedBerthOccupancy,
            expected: "SOURCE_CLASS_DOES_NOT_SUPPORT_CLAIM",
            publication_time: Some(FIXTURE_PUBLICATION_TIME),
            retrieval_time: FIXTURE_RETRIEVAL_TIME,
            independence_ref: None,
            custody_digest: None,
            event_time: Some(FIXTURE_EVENT_TIME),
        },
        BenchmarkCase {
            id: "dcsa_conformance_as_interoperability",
            artifact_class: ArtifactClass::DcsaApiConformanceReport,
            claim_kind: ClaimKind::ApiInteroperabilityConformance,
            expected: "INTEROPERABILITY_ONLY",
            publication_time: Some(FIXTURE_PUBLICATION_TIME),
            retrieval_time: FIXTURE_RETRIEVAL_TIME,
            independence_ref: None,
            custody_digest: None,
            event_time: None,
        },
        BenchmarkCase {
            id: "dcsa_conformance_to_physical_truth_negative",
            artifact_class: ArtifactClass::DcsaApiConformanceReport,
            claim_kind: ClaimKind::ObservedCargoMovement,
            expected: "SOURCE_CLASS_DOES_NOT_SUPPORT_CLAIM",
            publication_time: Some(FIXTURE_PUBLICATION_TIME),
            retrieval_time: FIXTURE_RETRIEVAL_TIME,
            independence_ref: None,
            custody_digest: None,
            event_time: Some(FIXTURE_EVENT_TIME),
        },
        BenchmarkCase {
            id: "independent_physical_observation_admitted",
            artifact_class: ArtifactClass::IndependentPhysicalObservation,
            claim_kind: ClaimKind::ObservedCargoMovement,
            expected: "PHYSICAL_OBSERVATION",
            publication_time: Some(FIXTURE_PUBLICATION_TIME),
            retrieval_time: FIXTURE_RETRIEVAL_TIME,
            independence_ref: Some("synthetic-independent-witness-review"),
            custody_digest: None,
            event_time: Some(FIXTURE_EVENT_TIME),
        },
        BenchmarkCase {
            id: "physical_observation_without_independence_reference_negative",
            artifact_class: ArtifactClass::IndependentPhysicalObservation,
            claim_kind: ClaimKind::ObservedBerthOccupancy,
            expected: "INDEPENDENCE_REFERENCE_MISSING",
            publication_time: Some(FIXTURE_PUBLICATION_TIME),
            retrieval_time: FIXTURE_RETRIEVAL_TIME,
            independence_ref: None,
            custody_digest: None,
            event_time: Some(FIXTURE_EVENT_TIME),
        },
        BenchmarkCase {
            id: "receiver_acceptance_without_custody_digest_negative",
            artifact_class: ArtifactClass::IndependentReceiverAcceptance,
            claim_kind: ClaimKind::ReceiverAcceptedDelivery,
            expected: "RECEIVER_CUSTODY_DIGEST_MISSING",
            publication_time: Some(FIXTURE_PUBLICATION_TIME),
            retrieval_time: FIXTURE_RETRIEVAL_TIME,
            independence_ref: Some("synthetic-independent-receiver-review"),
            custody_digest: None,
            event_time: Some(FIXTURE_EVENT_TIME),
        },
        BenchmarkCase {
            id: "unknown_publication_time_asof_negative",
            artifact_class: ArtifactClass::OfficialFirmBerthPlan,
            claim_kind: ClaimKind::PlannedBerthWindow,
            expected: "PUBLICATION_TIME_UNKNOWN",
            publication_time: None,
            retrieval_time: FIXTURE_RETRIEVAL_TIME,
            independence_ref: None,
            custody_digest: None,
            event_time: None,
        },
        BenchmarkCase {
            id: "late_retrieval_historical_asof_negative",
            artifact_class: ArtifactClass::OfficialVesselUpdate,
            claim_kind: ClaimKind::SourceReportedVesselStatus,
            expected: "EVIDENCE_UNAVAILABLE_AT_AS_OF_CUTOFF",
            publication_time: Some(FIXTURE_PUBLICATION_TIME),
            retrieval_time: FIXTURE_AS_OF + 1,
            independence_ref: None,
            custody_digest: None,
            event_time: Some(FIXTURE_EVENT_TIME),
        },
        BenchmarkCase {
            id: "published_after_historical_cutoff_negative",
            artifact_class: ArtifactClass::OfficialVesselUpdate,
            claim_kind: ClaimKind::SourceReportedVesselStatus,
            expected: "EVIDENCE_UNAVAILABLE_AT_AS_OF_CUTOFF",
            publication_time: Some(FIXTURE_AS_OF + 10),
            retrieval_time: FIXTURE_AS_OF + 20,
            independence_ref: None,
            custody_digest: None,
            event_time: Some(FIXTURE_EVENT_TIME),
        },
        BenchmarkCase {
            id: "terminal_notice_as_notice_only",
            artifact_class: ArtifactClass::OfficialTerminalNotice,
            claim_kind: ClaimKind::SourceReportedTerminalNotice,
            expected: "NOTICE_ONLY",
            publication_time: Some(FIXTURE_PUBLICATION_TIME),
            retrieval_time: FIXTURE_RETRIEVAL_TIME,
            independence_ref: None,
            custody_digest: None,
            event_time: None,
        },
        BenchmarkCase {
            id: "physical_observation_without_event_time_negative",
            artifact_class: ArtifactClass::IndependentPhysicalObservation,
            claim_kind: ClaimKind::ObservedCargoMovement,
            expected: "EVENT_TIME_UNKNOWN",
            publication_time: Some(FIXTURE_PUBLICATION_TIME),
            retrieval_time: FIXTURE_RETRIEVAL_TIME,
            independence_ref: Some("synthetic-independent-witness-review"),
            custody_digest: None,
            event_time: None,
        },
        BenchmarkCase {
            id: "receiver_acceptance_with_custody_digest",
            artifact_class: ArtifactClass::IndependentReceiverAcceptance,
            claim_kind: ClaimKind::ReceiverAcceptedDelivery,
            expected: "RECEIVER_ACCEPTANCE",
            publication_time: Some(FIXTURE_PUBLICATION_TIME),
            retrieval_time: FIXTURE_RETRIEVAL_TIME,
            independence_ref: Some("synthetic-independent-receiver-review"),
            custody_digest: Some(
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            ),
            event_time: Some(FIXTURE_EVENT_TIME),
        },
    ]
}

fn evaluate_case(case: BenchmarkCase) -> Result<AdmissionDecision, ArtifactValidationError> {
    let artifact = ArtifactEnvelope {
        artifact_id: format!("artifact-{}", case.id),
        source_owner: "synthetic-source-owner".to_owned(),
        source_revision: "synthetic-revision-1".to_owned(),
        raw_sha256: FIXTURE_DIGEST.to_owned(),
        retrieved_at_unix: case.retrieval_time,
        published_at_unix: case.publication_time,
        valid_from_unix: None,
        valid_until_unix: None,
        receiver_custody_sha256: case.custody_digest.map(str::to_owned),
        independence_evidence_ref: case.independence_ref.map(str::to_owned),
    };
    admit_claim(&SourceClaim {
        artifact_class: case.artifact_class,
        artifact,
        claim_kind: case.claim_kind,
        subject_id: "container-or-vessel:synthetic-001".to_owned(),
        source_record_id: format!("source-record:{}", case.id),
        as_of_unix: FIXTURE_AS_OF,
        event_time_unix: case.event_time,
    })
}

pub fn render_csv() -> String {
    let mut output = String::from(
        "case_id,artifact_class,claim_kind,decision,evidence_class,reason,claim_ceiling\n",
    );
    for case in benchmark_cases() {
        match evaluate_case(case) {
            Ok(decision) => {
                let observed = match decision {
                    AdmissionDecision::Admitted(class) => class.label(),
                    AdmissionDecision::Blocked(reason) => reason.label(),
                };
                assert_eq!(
                    observed, case.expected,
                    "source-claim benchmark case {}",
                    case.id
                );
                let _ = writeln!(
                    output,
                    "{},{},{},{},{},{},SYNTHETIC_ONLY",
                    case.id,
                    case.artifact_class.label(),
                    case.claim_kind.label(),
                    decision.label(),
                    decision.evidence_class(),
                    decision.reason()
                );
            }
            Err(error) => {
                let _ = writeln!(
                    output,
                    "{},{},,,NONE,VALIDATION_ERROR_{error:?},SYNTHETIC_ONLY",
                    case.id,
                    case.artifact_class.label()
                );
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_artifact() -> ArtifactEnvelope {
        ArtifactEnvelope {
            artifact_id: "dgt-firm-berth-plan-2026-11-24".to_owned(),
            source_owner: "Durban Gateway Terminal / ICTSI".to_owned(),
            source_revision: "2026-11-24T11:48:45+02:00".to_owned(),
            raw_sha256: FIXTURE_DIGEST.to_owned(),
            retrieved_at_unix: FIXTURE_RETRIEVAL_TIME,
            published_at_unix: Some(FIXTURE_PUBLICATION_TIME),
            valid_from_unix: Some(FIXTURE_EVENT_TIME),
            valid_until_unix: None,
            receiver_custody_sha256: None,
            independence_evidence_ref: None,
        }
    }

    fn claim(artifact_class: ArtifactClass, claim_kind: ClaimKind) -> SourceClaim {
        SourceClaim {
            artifact_class,
            artifact: valid_artifact(),
            claim_kind,
            subject_id: "container-or-vessel:synthetic-001".to_owned(),
            source_record_id: "source-record:synthetic-001".to_owned(),
            as_of_unix: FIXTURE_AS_OF,
            event_time_unix: Some(FIXTURE_EVENT_TIME),
        }
    }

    fn snapshot(
        snapshot_id: &str,
        family_id: &str,
        sequence: u64,
        supersedes: Option<&str>,
        published_at: Option<i64>,
        retrieved_at: i64,
    ) -> ArtifactSnapshot {
        ArtifactSnapshot {
            snapshot_id: snapshot_id.to_owned(),
            artifact_family_id: family_id.to_owned(),
            revision_sequence: sequence,
            supersedes_snapshot_id: supersedes.map(str::to_owned),
            envelope: ArtifactEnvelope {
                artifact_id: snapshot_id.to_owned(),
                source_owner: "Durban Gateway Terminal / ICTSI".to_owned(),
                source_revision: format!("revision-{sequence}"),
                raw_sha256: if sequence == 1 {
                    FIXTURE_DIGEST.to_owned()
                } else {
                    FIXTURE_CUSTODY_DIGEST.to_owned()
                },
                retrieved_at_unix: retrieved_at,
                published_at_unix: published_at,
                valid_from_unix: None,
                valid_until_unix: None,
                receiver_custody_sha256: None,
                independence_evidence_ref: None,
            },
        }
    }

    #[test]
    fn append_only_ledger_preserves_revisions_and_historical_asof_view() {
        let mut ledger = AppendOnlyArtifactLedger::default();
        ledger
            .append(snapshot(
                "stack-am",
                "dgt-export-stack-2026-11-24",
                1,
                None,
                Some(1_900_000_000),
                1_900_000_010,
            ))
            .expect("append first revision");
        ledger
            .append(snapshot(
                "stack-pm",
                "dgt-export-stack-2026-11-24",
                2,
                Some("stack-am"),
                Some(1_900_100_000),
                1_900_100_010,
            ))
            .expect("append correction revision");

        assert_eq!(ledger.snapshots().len(), 2);
        let historical = ledger
            .latest_available_as_of("dgt-export-stack-2026-11-24", 1_900_050_000)
            .expect("AM revision was available");
        assert_eq!(historical.snapshot_id, "stack-am");
        let latest = ledger
            .latest_available_as_of("dgt-export-stack-2026-11-24", 1_900_200_000)
            .expect("PM revision was available");
        assert_eq!(latest.snapshot_id, "stack-pm");
    }

    #[test]
    fn append_only_ledger_rejects_duplicate_and_nonlatest_supersession() {
        let mut ledger = AppendOnlyArtifactLedger::default();
        ledger
            .append(snapshot("rev-1", "family", 1, None, Some(100), 110))
            .expect("append root");
        ledger
            .append(snapshot(
                "rev-2",
                "family",
                2,
                Some("rev-1"),
                Some(200),
                210,
            ))
            .expect("append second");
        assert_eq!(
            ledger.append(snapshot(
                "rev-2",
                "family",
                2,
                Some("rev-1"),
                Some(200),
                210
            )),
            Err(LedgerAppendError::DuplicateSnapshotId)
        );
        assert_eq!(
            ledger.append(snapshot(
                "rev-3",
                "family",
                3,
                Some("rev-1"),
                Some(300),
                310
            )),
            Err(LedgerAppendError::SupersessionMustBeLatest)
        );
        assert_eq!(ledger.snapshots().len(), 2);
    }

    #[test]
    fn append_only_ledger_rejects_broken_revision_chains() {
        let mut ledger = AppendOnlyArtifactLedger::default();
        assert_eq!(
            ledger.append(snapshot("rev-2", "family", 2, None, Some(100), 110)),
            Err(LedgerAppendError::RootRevisionMustStartAtOne)
        );
        assert_eq!(
            ledger.append(snapshot(
                "rev-1",
                "family",
                1,
                Some("missing"),
                Some(100),
                110
            )),
            Err(LedgerAppendError::SupersededSnapshotUnknown)
        );
        ledger
            .append(snapshot("rev-1", "family", 1, None, Some(100), 110))
            .expect("append root");
        assert_eq!(
            ledger.append(snapshot(
                "rev-3",
                "family",
                3,
                Some("rev-1"),
                Some(300),
                310
            )),
            Err(LedgerAppendError::RevisionSequenceMustBeContiguous)
        );
        assert_eq!(
            ledger.append(snapshot(
                "rev-2-cross-family",
                "other-family",
                2,
                Some("rev-1"),
                Some(200),
                210
            )),
            Err(LedgerAppendError::SupersessionFamilyMismatch)
        );
    }

    #[test]
    fn unknown_publication_time_is_never_selected_by_historical_ledger_query() {
        let mut ledger = AppendOnlyArtifactLedger::default();
        ledger
            .append(snapshot(
                "unknown-pub",
                "unknown-family",
                1,
                None,
                None,
                100,
            ))
            .expect("append unknown publication timestamp");
        assert!(
            ledger
                .latest_available_as_of("unknown-family", 1_000)
                .is_none()
        );
    }

    #[test]
    fn terminal_firm_plan_is_admitted_only_as_a_plan() {
        assert_eq!(
            admit_claim(&claim(
                ArtifactClass::OfficialFirmBerthPlan,
                ClaimKind::PlannedBerthWindow
            )),
            Ok(AdmissionDecision::Admitted(AdmittedEvidenceClass::PlanOnly))
        );
    }

    #[test]
    fn firm_plan_cannot_prove_observed_berth_occupancy() {
        assert_eq!(
            admit_claim(&claim(
                ArtifactClass::OfficialFirmBerthPlan,
                ClaimKind::ObservedBerthOccupancy
            )),
            Ok(AdmissionDecision::Blocked(
                AdmissionBlock::SourceClassDoesNotSupportClaim
            ))
        );
    }

    #[test]
    fn terminal_vessel_status_remains_source_reported() {
        assert_eq!(
            admit_claim(&claim(
                ArtifactClass::OfficialVesselUpdate,
                ClaimKind::SourceReportedVesselStatus
            )),
            Ok(AdmissionDecision::Admitted(
                AdmittedEvidenceClass::SourceReported
            ))
        );
    }

    #[test]
    fn vessel_status_cannot_be_upgraded_to_independent_physical_truth() {
        assert_eq!(
            admit_claim(&claim(
                ArtifactClass::OfficialVesselUpdate,
                ClaimKind::ObservedBerthOccupancy
            )),
            Ok(AdmissionDecision::Blocked(
                AdmissionBlock::SourceClassDoesNotSupportClaim
            ))
        );
    }

    #[test]
    fn stack_date_window_is_not_proof_of_cargo_loading() {
        assert_eq!(
            admit_claim(&claim(
                ArtifactClass::OfficialExportStackWindow,
                ClaimKind::ObservedCargoMovement
            )),
            Ok(AdmissionDecision::Blocked(
                AdmissionBlock::SourceClassDoesNotSupportClaim
            ))
        );
    }

    #[test]
    fn import_storage_snapshot_is_not_receiver_delivery() {
        assert_eq!(
            admit_claim(&claim(
                ArtifactClass::OfficialImportStorageSnapshot,
                ClaimKind::ReceiverAcceptedDelivery
            )),
            Ok(AdmissionDecision::Blocked(
                AdmissionBlock::SourceClassDoesNotSupportClaim
            ))
        );
    }

    #[test]
    fn third_party_aggregator_remains_derived_only() {
        assert_eq!(
            admit_claim(&claim(
                ArtifactClass::ThirdPartyDerivedAggregator,
                ClaimKind::ThirdPartyDerivedStatus
            )),
            Ok(AdmissionDecision::Admitted(
                AdmittedEvidenceClass::ThirdPartyDerivedOnly
            ))
        );
    }

    #[test]
    fn third_party_aggregator_cannot_be_an_independent_physical_witness() {
        assert_eq!(
            admit_claim(&claim(
                ArtifactClass::ThirdPartyDerivedAggregator,
                ClaimKind::ObservedBerthOccupancy
            )),
            Ok(AdmissionDecision::Blocked(
                AdmissionBlock::SourceClassDoesNotSupportClaim
            ))
        );
    }

    #[test]
    fn dcsa_api_conformance_is_interoperability_not_physical_truth() {
        assert_eq!(
            admit_claim(&claim(
                ArtifactClass::DcsaApiConformanceReport,
                ClaimKind::ApiInteroperabilityConformance
            )),
            Ok(AdmissionDecision::Admitted(
                AdmittedEvidenceClass::InteroperabilityOnly
            ))
        );
        assert_eq!(
            admit_claim(&claim(
                ArtifactClass::DcsaApiConformanceReport,
                ClaimKind::ObservedCargoMovement
            )),
            Ok(AdmissionDecision::Blocked(
                AdmissionBlock::SourceClassDoesNotSupportClaim
            ))
        );
    }

    #[test]
    fn independent_physical_observation_requires_review_reference() {
        let mut missing = claim(
            ArtifactClass::IndependentPhysicalObservation,
            ClaimKind::ObservedCargoMovement,
        );
        assert_eq!(
            admit_claim(&missing),
            Ok(AdmissionDecision::Blocked(
                AdmissionBlock::IndependenceEvidenceReferenceMissing
            ))
        );

        missing.artifact.independence_evidence_ref =
            Some("synthetic-independent-witness-review".to_owned());
        assert_eq!(
            admit_claim(&missing),
            Ok(AdmissionDecision::Admitted(
                AdmittedEvidenceClass::IndependentPhysicalObservation
            ))
        );
    }

    #[test]
    fn receiver_acceptance_requires_independence_and_custody_digest() {
        let mut receipt = claim(
            ArtifactClass::IndependentReceiverAcceptance,
            ClaimKind::ReceiverAcceptedDelivery,
        );
        receipt.artifact.independence_evidence_ref =
            Some("synthetic-independent-receiver-review".to_owned());
        assert_eq!(
            admit_claim(&receipt),
            Ok(AdmissionDecision::Blocked(
                AdmissionBlock::ReceiverCustodyDigestMissing
            ))
        );
        receipt.artifact.receiver_custody_sha256 = Some(FIXTURE_CUSTODY_DIGEST.to_owned());
        assert_eq!(
            admit_claim(&receipt),
            Ok(AdmissionDecision::Admitted(
                AdmittedEvidenceClass::IndependentReceiverAcceptance
            ))
        );
    }

    #[test]
    fn claim_without_canonical_subject_identity_is_blocked() {
        let mut candidate = claim(
            ArtifactClass::IndependentPhysicalObservation,
            ClaimKind::ObservedCargoMovement,
        );
        candidate.artifact.independence_evidence_ref =
            Some("synthetic-independent-witness-review".to_owned());
        candidate.subject_id.clear();
        assert_eq!(
            admit_claim(&candidate),
            Ok(AdmissionDecision::Blocked(
                AdmissionBlock::ClaimSubjectIdentityMissing
            ))
        );
    }

    #[test]
    fn claim_without_source_native_record_identity_is_blocked() {
        let mut candidate = claim(
            ArtifactClass::OfficialVesselUpdate,
            ClaimKind::SourceReportedVesselStatus,
        );
        candidate.source_record_id.clear();
        assert_eq!(
            admit_claim(&candidate),
            Ok(AdmissionDecision::Blocked(
                AdmissionBlock::SourceRecordIdentityMissing
            ))
        );
    }

    #[test]
    fn missing_publication_time_blocks_historical_asof_claims() {
        let mut candidate = claim(
            ArtifactClass::OfficialFirmBerthPlan,
            ClaimKind::PlannedBerthWindow,
        );
        candidate.artifact.published_at_unix = None;
        assert_eq!(
            admit_claim(&candidate),
            Ok(AdmissionDecision::Blocked(
                AdmissionBlock::PublicationTimeUnknown
            ))
        );
    }

    #[test]
    fn publication_and_retrieval_after_cutoff_are_not_historically_available() {
        let mut candidate = claim(
            ArtifactClass::OfficialVesselUpdate,
            ClaimKind::SourceReportedVesselStatus,
        );
        candidate.artifact.retrieved_at_unix = FIXTURE_AS_OF + 1;
        assert_eq!(
            admit_claim(&candidate),
            Ok(AdmissionDecision::Blocked(
                AdmissionBlock::EvidenceUnavailableAtAsOfCutoff
            ))
        );

        candidate.artifact.retrieved_at_unix = FIXTURE_AS_OF + 20;
        candidate.artifact.published_at_unix = Some(FIXTURE_AS_OF + 10);
        assert_eq!(
            admit_claim(&candidate),
            Ok(AdmissionDecision::Blocked(
                AdmissionBlock::EvidenceUnavailableAtAsOfCutoff
            ))
        );

        candidate.artifact.retrieved_at_unix = FIXTURE_RETRIEVAL_TIME;
        candidate.artifact.published_at_unix = Some(FIXTURE_AS_OF + 1);
        assert_eq!(
            admit_claim(&candidate),
            Err(ArtifactValidationError::PublicationAfterRetrieval)
        );
    }

    #[test]
    fn physical_observation_without_event_time_is_blocked() {
        let mut candidate = claim(
            ArtifactClass::IndependentPhysicalObservation,
            ClaimKind::ObservedCargoMovement,
        );
        candidate.artifact.independence_evidence_ref =
            Some("synthetic-independent-witness-review".to_owned());
        candidate.event_time_unix = None;
        assert_eq!(
            admit_claim(&candidate),
            Ok(AdmissionDecision::Blocked(AdmissionBlock::EventTimeUnknown))
        );
    }

    #[test]
    fn observed_future_event_is_not_admitted_as_of_the_past() {
        let mut candidate = claim(
            ArtifactClass::IndependentPhysicalObservation,
            ClaimKind::ObservedBerthOccupancy,
        );
        candidate.artifact.independence_evidence_ref =
            Some("synthetic-independent-witness-review".to_owned());
        candidate.event_time_unix = Some(FIXTURE_AS_OF + 1);
        assert_eq!(
            admit_claim(&candidate),
            Ok(AdmissionDecision::Blocked(
                AdmissionBlock::EventNotYetOccurredAtAsOfCutoff
            ))
        );
    }

    #[test]
    fn validity_interval_is_distinct_from_publication_and_retrieval_time() {
        let mut candidate = claim(
            ArtifactClass::IndependentPhysicalObservation,
            ClaimKind::ObservedBerthOccupancy,
        );
        candidate.artifact.independence_evidence_ref =
            Some("synthetic-independent-witness-review".to_owned());
        candidate.artifact.valid_from_unix = None;
        candidate.artifact.valid_until_unix = Some(FIXTURE_EVENT_TIME - 1);
        assert_eq!(
            admit_claim(&candidate),
            Ok(AdmissionDecision::Blocked(
                AdmissionBlock::EventOutsideDeclaredValidity
            ))
        );
    }

    #[test]
    fn malformed_digests_and_time_order_are_rejected() {
        let mut artifact = valid_artifact();
        artifact.raw_sha256 = "not-a-digest".to_owned();
        assert_eq!(
            artifact.validate(),
            Err(ArtifactValidationError::InvalidRawSha256)
        );

        let mut artifact = valid_artifact();
        artifact.retrieved_at_unix = FIXTURE_PUBLICATION_TIME - 1;
        assert_eq!(
            artifact.validate(),
            Err(ArtifactValidationError::PublicationAfterRetrieval)
        );
    }

    #[test]
    fn benchmark_receipt_is_deterministic_and_contains_all_negative_controls() {
        let first = render_csv();
        assert_eq!(first, render_csv());
        assert_eq!(first.lines().count(), 22);
        let width = first.lines().next().expect("CSV header").split(',').count();
        assert_eq!(width, 7);
        for row in first.lines().skip(1) {
            assert_eq!(row.split(',').count(), width);
        }
        assert!(first.contains("PLAN_ONLY"));
        assert!(first.contains("SOURCE_CLASS_DOES_NOT_SUPPORT_CLAIM"));
        assert!(first.contains("PUBLICATION_TIME_UNKNOWN"));
        assert!(first.contains("THIRD_PARTY_DERIVED_ONLY"));
        assert!(first.contains("INTEROPERABILITY_ONLY"));
        assert!(first.contains("RECEIVER_CUSTODY_DIGEST_MISSING"));
    }
}
