// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Append-only source-artifact semantics and claim-promotion firewall.
//!
//! The module separates three different times: source publication time,
//! system retrieval time, and business valid time. It also prevents a source
//! artifact from being promoted beyond its declared evidence class. A berth
//! plan may support a planned berth window; it cannot prove actual occupancy.
//! A terminal-reported vessel status is not an independent physical witness.
//! A stack window is not gate-in or loading evidence, and an inventory snapshot
//! is not custody transfer or receiver acceptance.
//!
//! This is a typed evidence-admission layer, not a signature verifier, a source
//! rights grant, or an independent physical oracle. The artifact class and
//! rights status are caller declarations that must be supported by source
//! review. The raw digest is validated as a SHA-256-shaped identifier but this
//! module does not hash or retrieve the source bytes.

use std::collections::BTreeMap;

use std::fmt::Write as FmtWrite;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceArtifactClass {
    BerthPlan,
    VesselStatusSnapshot,
    ExportStackWindow,
    ImportStorageSnapshot,
    PhysicalObservationReceipt,
    CustodyTransferReceipt,
    ReceiverAcceptanceReceipt,
}

impl SourceArtifactClass {
    pub fn label(self) -> &'static str {
        match self {
            Self::BerthPlan => "DGT_BERTH_PLAN",
            Self::VesselStatusSnapshot => "DGT_VESSEL_STATUS_SNAPSHOT",
            Self::ExportStackWindow => "DGT_EXPORT_STACK_WINDOW",
            Self::ImportStorageSnapshot => "DGT_IMPORT_STORAGE_SNAPSHOT",
            Self::PhysicalObservationReceipt => "PHYSICAL_OBSERVATION_RECEIPT",
            Self::CustodyTransferReceipt => "CUSTODY_TRANSFER_RECEIPT",
            Self::ReceiverAcceptanceReceipt => "RECEIVER_ACCEPTANCE_RECEIPT",
        }
    }

    fn supports(self, claim: OperationalClaim) -> bool {
        match self {
            Self::BerthPlan => claim == OperationalClaim::PlannedBerthWindow,
            Self::VesselStatusSnapshot => {
                claim == OperationalClaim::TerminalReportedVesselStatus
            }
            Self::ExportStackWindow => claim == OperationalClaim::ExportStackEligibilityWindow,
            Self::ImportStorageSnapshot => claim == OperationalClaim::ImportInventoryState,
            Self::PhysicalObservationReceipt => claim.is_physical_observation(),
            Self::CustodyTransferReceipt => claim == OperationalClaim::CustodyTransferred,
            Self::ReceiverAcceptanceReceipt => claim == OperationalClaim::ReceiverAccepted,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationalClaim {
    PlannedBerthWindow,
    TerminalReportedVesselStatus,
    ExportStackEligibilityWindow,
    ImportInventoryState,
    VesselArrivalObserved,
    BerthOccupancyObserved,
    VesselDepartureObserved,
    ContainerGateInObserved,
    CargoLoadedObserved,
    CustodyTransferred,
    ReceiverAccepted,
}

impl OperationalClaim {
    pub fn label(self) -> &'static str {
        match self {
            Self::PlannedBerthWindow => "PLANNED_BERTH_WINDOW",
            Self::TerminalReportedVesselStatus => "TERMINAL_REPORTED_VESSEL_STATUS",
            Self::ExportStackEligibilityWindow => "EXPORT_STACK_ELIGIBILITY_WINDOW",
            Self::ImportInventoryState => "IMPORT_INVENTORY_STATE",
            Self::VesselArrivalObserved => "VESSEL_ARRIVAL_OBSERVED",
            Self::BerthOccupancyObserved => "BERTH_OCCUPANCY_OBSERVED",
            Self::VesselDepartureObserved => "VESSEL_DEPARTURE_OBSERVED",
            Self::ContainerGateInObserved => "CONTAINER_GATE_IN_OBSERVED",
            Self::CargoLoadedObserved => "CARGO_LOADED_OBSERVED",
            Self::CustodyTransferred => "CUSTODY_TRANSFERRED",
            Self::ReceiverAccepted => "RECEIVER_ACCEPTED",
        }
    }

    fn is_physical_observation(self) -> bool {
        matches!(
            self,
            Self::VesselArrivalObserved
                | Self::BerthOccupancyObserved
                | Self::VesselDepartureObserved
                | Self::ContainerGateInObserved
                | Self::CargoLoadedObserved
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaimAdmission {
    /// The artifact supports only its own declared claim class. This is not an
    /// assertion that the source is independent or that its contents are true.
    SupportedWithinDeclaredClass,
    /// A separately qualified physical observation receipt is required.
    RequiresIndependentPhysicalObservation,
    /// A typed custody-transfer receipt is required.
    RequiresCustodyTransferReceipt,
    /// A typed receiver-acceptance receipt is required.
    RequiresReceiverAcceptanceReceipt,
    /// The artifact does not support the requested claim.
    NotSupportedBySourceClass,
}

impl ClaimAdmission {
    pub fn label(self) -> &'static str {
        match self {
            Self::SupportedWithinDeclaredClass => "SUPPORTED_WITHIN_DECLARED_CLASS",
            Self::RequiresIndependentPhysicalObservation => {
                "REQUIRES_INDEPENDENT_PHYSICAL_OBSERVATION"
            }
            Self::RequiresCustodyTransferReceipt => "REQUIRES_CUSTODY_TRANSFER_RECEIPT",
            Self::RequiresReceiverAcceptanceReceipt => {
                "REQUIRES_RECEIVER_ACCEPTANCE_RECEIPT"
            }
            Self::NotSupportedBySourceClass => "NOT_SUPPORTED_BY_SOURCE_CLASS",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RedistributionRights {
    PermissionGranted,
    PermissionRestricted,
    NotEstablished,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidTime {
    /// Inclusive Unix-epoch milliseconds. None means unknown/unbounded.
    pub from_unix_ms: Option<i64>,
    /// Inclusive Unix-epoch milliseconds. None means unknown/unbounded.
    pub to_unix_ms: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceArtifactEnvelope {
    /// Stable logical artifact identity across revisions.
    pub artifact_id: String,
    pub source_id: String,
    pub revision_id: String,
    pub class: SourceArtifactClass,
    /// The narrow source claim this artifact is declared to make.
    pub declared_claim: OperationalClaim,
    /// Publication time reported by the source, not the system retrieval time.
    pub source_published_at_unix_ms: Option<i64>,
    /// Time the system retrieved and captured the immutable bytes.
    pub retrieved_at_unix_ms: i64,
    /// Business/effective time represented by the artifact.
    pub valid_time: ValidTime,
    /// SHA-256 digest of the captured raw artifact bytes.
    pub raw_sha256: String,
    pub redistribution_rights: RedistributionRights,
    /// Exact previous source revision; a new logical artifact has None.
    pub supersedes_revision: Option<String>,
}

impl SourceArtifactEnvelope {
    pub fn validate(&self) -> Result<(), SourceArtifactError> {
        if self.artifact_id.trim().is_empty() {
            return Err(SourceArtifactError::EmptyArtifactId);
        }
        if self.source_id.trim().is_empty() {
            return Err(SourceArtifactError::EmptySourceId);
        }
        if self.revision_id.trim().is_empty() {
            return Err(SourceArtifactError::EmptyRevisionId);
        }
        if self
            .supersedes_revision
            .as_ref()
            .is_some_and(|revision| revision.trim().is_empty())
        {
            return Err(SourceArtifactError::EmptySupersededRevision);
        }
        if !is_sha256(&self.raw_sha256) {
            return Err(SourceArtifactError::InvalidRawSha256);
        }
        if self.retrieved_at_unix_ms < 0 {
            return Err(SourceArtifactError::NegativeRetrievalTime);
        }
        if let Some(published) = self.source_published_at_unix_ms {
            if published < 0 {
                return Err(SourceArtifactError::NegativePublicationTime);
            }
            if published > self.retrieved_at_unix_ms {
                return Err(SourceArtifactError::PublishedAfterRetrieved);
            }
        }
        if self
            .valid_time
            .from_unix_ms
            .is_some_and(|timestamp| timestamp < 0)
            || self
                .valid_time
                .to_unix_ms
                .is_some_and(|timestamp| timestamp < 0)
        {
            return Err(SourceArtifactError::NegativeValidTime);
        }
        if let (Some(from), Some(to)) = (
            self.valid_time.from_unix_ms,
            self.valid_time.to_unix_ms,
        ) {
            if to < from {
                return Err(SourceArtifactError::InvertedValidTime);
            }
        }
        if !self.class.supports(self.declared_claim) {
            return Err(SourceArtifactError::DeclaredClaimExceedsSourceClass);
        }
        Ok(())
    }

    /// Checks whether the artifact class can support a requested claim.
    ///
    /// A matching class is only admissible at the declared source-claim level.
    /// It does not authenticate the publisher, prove independence, or grant
    /// operational authority.
    pub fn claim_admission(&self, requested: OperationalClaim) -> ClaimAdmission {
        if requested == self.declared_claim && self.class.supports(requested) {
            return ClaimAdmission::SupportedWithinDeclaredClass;
        }

        if requested.is_physical_observation() {
            return if self.class == SourceArtifactClass::PhysicalObservationReceipt {
                ClaimAdmission::NotSupportedBySourceClass
            } else {
                ClaimAdmission::RequiresIndependentPhysicalObservation
            };
        }

        match requested {
            OperationalClaim::CustodyTransferred => {
                if self.class == SourceArtifactClass::CustodyTransferReceipt {
                    ClaimAdmission::NotSupportedBySourceClass
                } else {
                    ClaimAdmission::RequiresCustodyTransferReceipt
                }
            }
            OperationalClaim::ReceiverAccepted => {
                if self.class == SourceArtifactClass::ReceiverAcceptanceReceipt {
                    ClaimAdmission::NotSupportedBySourceClass
                } else {
                    ClaimAdmission::RequiresReceiverAcceptanceReceipt
                }
            }
            _ => ClaimAdmission::NotSupportedBySourceClass,
        }
    }

    /// Rights status must be PermissionGranted before mirroring/redistribution.
    pub fn may_redistribute(&self) -> bool {
        self.redistribution_rights == RedistributionRights::PermissionGranted
    }

    pub fn is_valid_at(&self, valid_at_unix_ms: i64) -> bool {
        self.valid_time
            .from_unix_ms
            .is_some_and(|from| from <= valid_at_unix_ms)
            && self
                .valid_time
                .to_unix_ms
                .is_none_or(|to| valid_at_unix_ms <= to)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceArtifactError {
    EmptyArtifactId,
    EmptySourceId,
    EmptyRevisionId,
    EmptySupersededRevision,
    InvalidRawSha256,
    NegativeRetrievalTime,
    NegativePublicationTime,
    PublishedAfterRetrieved,
    NegativeValidTime,
    InvertedValidTime,
    DeclaredClaimExceedsSourceClass,
    DuplicateRevision(String),
    MissingRevisionHistory(String),
    SupersedesLatestRevisionMismatch {
        expected: String,
        found: Option<String>,
    },
    SourceIdentityChanged,
    SourceClassChanged,
    PublicationTimeRegressed,
    RetrievalTimeRegressed,
}

#[derive(Clone, Debug, Default)]
pub struct SourceArtifactLedger {
    revisions_by_artifact: BTreeMap<String, Vec<SourceArtifactEnvelope>>,
}

impl SourceArtifactLedger {
    /// Appends a new immutable revision. Existing revision bytes are never
    /// replaced: every later revision must name the exact previous revision.
    pub fn append(
        &mut self,
        artifact: SourceArtifactEnvelope,
    ) -> Result<(), SourceArtifactError> {
        artifact.validate()?;
        let revisions = self
            .revisions_by_artifact
            .entry(artifact.artifact_id.clone())
            .or_default();

        if revisions
            .iter()
            .any(|existing| existing.revision_id == artifact.revision_id)
        {
            return Err(SourceArtifactError::DuplicateRevision(
                artifact.revision_id,
            ));
        }

        match revisions.last() {
            None => {
                if artifact.supersedes_revision.is_some() {
                    return Err(SourceArtifactError::MissingRevisionHistory(
                        artifact.artifact_id,
                    ));
                }
            }
            Some(previous) => {
                if artifact.supersedes_revision.as_deref()
                    != Some(previous.revision_id.as_str())
                {
                    return Err(SourceArtifactError::SupersedesLatestRevisionMismatch {
                        expected: previous.revision_id.clone(),
                        found: artifact.supersedes_revision,
                    });
                }
                if artifact.source_id != previous.source_id {
                    return Err(SourceArtifactError::SourceIdentityChanged);
                }
                if artifact.class != previous.class
                    || artifact.declared_claim != previous.declared_claim
                {
                    return Err(SourceArtifactError::SourceClassChanged);
                }
                if artifact.retrieved_at_unix_ms < previous.retrieved_at_unix_ms {
                    return Err(SourceArtifactError::RetrievalTimeRegressed);
                }
                if let (
                    Some(previous_published),
                    Some(next_published),
                ) = (
                    previous.source_published_at_unix_ms,
                    artifact.source_published_at_unix_ms,
                ) {
                    if next_published < previous_published {
                        return Err(SourceArtifactError::PublicationTimeRegressed);
                    }
                }
            }
        }

        revisions.push(artifact);
        Ok(())
    }

    pub fn revisions(&self, artifact_id: &str) -> Option<&[SourceArtifactEnvelope]> {
        self.revisions_by_artifact
            .get(artifact_id)
            .map(Vec::as_slice)
    }

    /// Returns the latest revision that the source had published and the
    /// system had retrieved by the two independent as-of cutoffs.
    ///
    /// Revisions with unknown source publication time are intentionally
    /// excluded from source-publication as-of replay.
    pub fn latest_available_as_of(
        &self,
        artifact_id: &str,
        source_publication_cutoff_unix_ms: i64,
        system_retrieval_cutoff_unix_ms: i64,
    ) -> Option<&SourceArtifactEnvelope> {
        self.revisions_by_artifact.get(artifact_id)?.iter().rev().find(|revision| {
            revision
                .source_published_at_unix_ms
                .is_some_and(|published| published <= source_publication_cutoff_unix_ms)
                && revision.retrieved_at_unix_ms <= system_retrieval_cutoff_unix_ms
        })
    }

    /// Returns the latest captured revision that applies to a requested valid
    /// time and was available by both publication and retrieval cutoffs.
    pub fn latest_valid_at_as_of(
        &self,
        artifact_id: &str,
        valid_at_unix_ms: i64,
        source_publication_cutoff_unix_ms: i64,
        system_retrieval_cutoff_unix_ms: i64,
    ) -> Option<&SourceArtifactEnvelope> {
        self.revisions_by_artifact.get(artifact_id)?.iter().rev().find(|revision| {
            revision.is_valid_at(valid_at_unix_ms)
                && revision
                    .source_published_at_unix_ms
                    .is_some_and(|published| published <= source_publication_cutoff_unix_ms)
                && revision.retrieved_at_unix_ms <= system_retrieval_cutoff_unix_ms
        })
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn render_row(
    output: &mut String,
    class: SourceArtifactClass,
    declared: OperationalClaim,
    requested: OperationalClaim,
) {
    let envelope = fixture_artifact(class, declared, 1_000, 1_100, 1_000, 2_000);
    let admission = envelope.claim_admission(requested);
    let _ = writeln!(
        output,
        "{},{},{},{}",
        class.label(),
        declared.label(),
        requested.label(),
        admission.label()
    );
}

/// Deterministic DGT-specific negative-control matrix; no real artifact bytes
/// are read or claimed by this executable.
pub fn render_dgt_claim_negative_controls() -> String {
    let mut output = String::from(
        "source_class,declared_claim,requested_claim,admission\n",
    );
    let rows = [
        (
            SourceArtifactClass::BerthPlan,
            OperationalClaim::PlannedBerthWindow,
            OperationalClaim::PlannedBerthWindow,
        ),
        (
            SourceArtifactClass::BerthPlan,
            OperationalClaim::PlannedBerthWindow,
            OperationalClaim::BerthOccupancyObserved,
        ),
        (
            SourceArtifactClass::VesselStatusSnapshot,
            OperationalClaim::TerminalReportedVesselStatus,
            OperationalClaim::TerminalReportedVesselStatus,
        ),
        (
            SourceArtifactClass::VesselStatusSnapshot,
            OperationalClaim::TerminalReportedVesselStatus,
            OperationalClaim::VesselDepartureObserved,
        ),
        (
            SourceArtifactClass::ExportStackWindow,
            OperationalClaim::ExportStackEligibilityWindow,
            OperationalClaim::ExportStackEligibilityWindow,
        ),
        (
            SourceArtifactClass::ExportStackWindow,
            OperationalClaim::ExportStackEligibilityWindow,
            OperationalClaim::ContainerGateInObserved,
        ),
        (
            SourceArtifactClass::ImportStorageSnapshot,
            OperationalClaim::ImportInventoryState,
            OperationalClaim::ImportInventoryState,
        ),
        (
            SourceArtifactClass::ImportStorageSnapshot,
            OperationalClaim::ImportInventoryState,
            OperationalClaim::ReceiverAccepted,
        ),
        (
            SourceArtifactClass::PhysicalObservationReceipt,
            OperationalClaim::BerthOccupancyObserved,
            OperationalClaim::BerthOccupancyObserved,
        ),
        (
            SourceArtifactClass::CustodyTransferReceipt,
            OperationalClaim::CustodyTransferred,
            OperationalClaim::CustodyTransferred,
        ),
        (
            SourceArtifactClass::CustodyTransferReceipt,
            OperationalClaim::CustodyTransferred,
            OperationalClaim::ReceiverAccepted,
        ),
        (
            SourceArtifactClass::ReceiverAcceptanceReceipt,
            OperationalClaim::ReceiverAccepted,
            OperationalClaim::ReceiverAccepted,
        ),
    ];

    for (class, declared, requested) in rows {
        render_row(&mut output, class, declared, requested);
    }
    output
}

fn fixture_artifact(
    class: SourceArtifactClass,
    declared_claim: OperationalClaim,
    source_published_at_unix_ms: i64,
    retrieved_at_unix_ms: i64,
    valid_from_unix_ms: i64,
    valid_to_unix_ms: i64,
) -> SourceArtifactEnvelope {
    SourceArtifactEnvelope {
        artifact_id: "fixture-artifact".to_owned(),
        source_id: "fixture-source".to_owned(),
        revision_id: "r1".to_owned(),
        class,
        declared_claim,
        source_published_at_unix_ms: Some(source_published_at_unix_ms),
        retrieved_at_unix_ms,
        valid_time: ValidTime {
            from_unix_ms: Some(valid_from_unix_ms),
            to_unix_ms: Some(valid_to_unix_ms),
        },
        raw_sha256: "a".repeat(64),
        redistribution_rights: RedistributionRights::NotEstablished,
        supersedes_revision: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(
        revision_id: &str,
        published: i64,
        retrieved: i64,
        supersedes: Option<&str>,
    ) -> SourceArtifactEnvelope {
        SourceArtifactEnvelope {
            artifact_id: "dgt-firm-berth-plan-2026-11-24".to_owned(),
            source_id: "dgt-terminalupdates".to_owned(),
            revision_id: revision_id.to_owned(),
            class: SourceArtifactClass::BerthPlan,
            declared_claim: OperationalClaim::PlannedBerthWindow,
            source_published_at_unix_ms: Some(published),
            retrieved_at_unix_ms: retrieved,
            valid_time: ValidTime {
                from_unix_ms: Some(1_000),
                to_unix_ms: Some(5_000),
            },
            raw_sha256: "b".repeat(64),
            redistribution_rights: RedistributionRights::NotEstablished,
            supersedes_revision: supersedes.map(str::to_owned),
        }
    }

    #[test]
    fn dgt_firm_berth_plan_cannot_be_promoted_to_actual_berth_occupancy() {
        let plan = fixture_artifact(
            SourceArtifactClass::BerthPlan,
            OperationalClaim::PlannedBerthWindow,
            1_000,
            1_100,
            1_000,
            2_000,
        );
        assert_eq!(
            plan.claim_admission(OperationalClaim::PlannedBerthWindow),
            ClaimAdmission::SupportedWithinDeclaredClass
        );
        assert_eq!(
            plan.claim_admission(OperationalClaim::BerthOccupancyObserved),
            ClaimAdmission::RequiresIndependentPhysicalObservation
        );
    }

    #[test]
    fn vessel_status_snapshot_cannot_be_promoted_to_departure_observation() {
        let update = fixture_artifact(
            SourceArtifactClass::VesselStatusSnapshot,
            OperationalClaim::TerminalReportedVesselStatus,
            1_000,
            1_100,
            1_000,
            2_000,
        );
        assert_eq!(
            update.claim_admission(OperationalClaim::TerminalReportedVesselStatus),
            ClaimAdmission::SupportedWithinDeclaredClass
        );
        assert_eq!(
            update.claim_admission(OperationalClaim::VesselDepartureObserved),
            ClaimAdmission::RequiresIndependentPhysicalObservation
        );
    }

    #[test]
    fn stack_window_cannot_be_promoted_to_gate_in_or_loading() {
        let stack = fixture_artifact(
            SourceArtifactClass::ExportStackWindow,
            OperationalClaim::ExportStackEligibilityWindow,
            1_000,
            1_100,
            1_000,
            2_000,
        );
        assert_eq!(
            stack.claim_admission(OperationalClaim::ExportStackEligibilityWindow),
            ClaimAdmission::SupportedWithinDeclaredClass
        );
        assert_eq!(
            stack.claim_admission(OperationalClaim::ContainerGateInObserved),
            ClaimAdmission::RequiresIndependentPhysicalObservation
        );
        assert_eq!(
            stack.claim_admission(OperationalClaim::CargoLoadedObserved),
            ClaimAdmission::RequiresIndependentPhysicalObservation
        );
    }

    #[test]
    fn import_inventory_snapshot_cannot_be_promoted_to_receiver_acceptance() {
        let inventory = fixture_artifact(
            SourceArtifactClass::ImportStorageSnapshot,
            OperationalClaim::ImportInventoryState,
            1_000,
            1_100,
            1_000,
            2_000,
        );
        assert_eq!(
            inventory.claim_admission(OperationalClaim::ImportInventoryState),
            ClaimAdmission::SupportedWithinDeclaredClass
        );
        assert_eq!(
            inventory.claim_admission(OperationalClaim::ReceiverAccepted),
            ClaimAdmission::RequiresReceiverAcceptanceReceipt
        );
    }

    #[test]
    fn physical_observation_receipt_is_scoped_to_one_exact_physical_claim() {
        let observation = fixture_artifact(
            SourceArtifactClass::PhysicalObservationReceipt,
            OperationalClaim::BerthOccupancyObserved,
            1_000,
            1_100,
            1_000,
            2_000,
        );
        assert_eq!(
            observation.claim_admission(OperationalClaim::BerthOccupancyObserved),
            ClaimAdmission::SupportedWithinDeclaredClass
        );
        assert_eq!(
            observation.claim_admission(OperationalClaim::VesselDepartureObserved),
            ClaimAdmission::NotSupportedBySourceClass
        );
        assert_eq!(
            observation.claim_admission(OperationalClaim::ReceiverAccepted),
            ClaimAdmission::RequiresReceiverAcceptanceReceipt
        );
    }

    #[test]
    fn custody_receipt_does_not_substitute_for_receiver_acceptance() {
        let custody = fixture_artifact(
            SourceArtifactClass::CustodyTransferReceipt,
            OperationalClaim::CustodyTransferred,
            1_000,
            1_100,
            1_000,
            2_000,
        );
        assert_eq!(
            custody.claim_admission(OperationalClaim::CustodyTransferred),
            ClaimAdmission::SupportedWithinDeclaredClass
        );
        assert_eq!(
            custody.claim_admission(OperationalClaim::ReceiverAccepted),
            ClaimAdmission::RequiresReceiverAcceptanceReceipt
        );
    }

    #[test]
    fn source_class_cannot_declare_a_claim_beyond_its_evidence_type() {
        let mut plan = fixture_artifact(
            SourceArtifactClass::BerthPlan,
            OperationalClaim::PlannedBerthWindow,
            1_000,
            1_100,
            1_000,
            2_000,
        );
        plan.declared_claim = OperationalClaim::BerthOccupancyObserved;
        assert_eq!(
            plan.validate(),
            Err(SourceArtifactError::DeclaredClaimExceedsSourceClass)
        );
    }

    #[test]
    fn unknown_rights_never_authorize_redistribution() {
        let artifact = fixture_artifact(
            SourceArtifactClass::BerthPlan,
            OperationalClaim::PlannedBerthWindow,
            1_000,
            1_100,
            1_000,
            2_000,
        );
        assert!(!artifact.may_redistribute());
        let mut permitted = artifact.clone();
        permitted.redistribution_rights = RedistributionRights::PermissionGranted;
        assert!(permitted.may_redistribute());
    }

    #[test]
    fn validates_digest_times_and_valid_time_interval() {
        let mut value = artifact("r1", 1_000, 1_100, None);
        assert!(value.validate().is_ok());
        value.raw_sha256 = "not-a-digest".to_owned();
        assert_eq!(value.validate(), Err(SourceArtifactError::InvalidRawSha256));

        let mut future_publication = artifact("r1", 1_200, 1_100, None);
        assert_eq!(
            future_publication.validate(),
            Err(SourceArtifactError::PublishedAfterRetrieved)
        );

        let mut inverted = artifact("r1", 1_000, 1_100, None);
        inverted.valid_time.to_unix_ms = Some(999);
        assert_eq!(inverted.validate(), Err(SourceArtifactError::InvertedValidTime));
    }

    #[test]
    fn ledger_is_append_only_and_requires_exact_revision_lineage() {
        let mut ledger = SourceArtifactLedger::default();
        ledger
            .append(artifact("r1", 1_000, 1_100, None))
            .expect("initial revision");
        ledger
            .append(artifact("r2", 1_500, 1_600, Some("r1")))
            .expect("revision two");

        assert_eq!(
            ledger.append(artifact("r2", 1_700, 1_800, Some("r1"))),
            Err(SourceArtifactError::DuplicateRevision("r2".to_owned()))
        );
        assert!(matches!(
            ledger.append(artifact("r3", 1_700, 1_800, Some("r1"))),
            Err(SourceArtifactError::SupersedesLatestRevisionMismatch { .. })
        ));
        assert_eq!(
            ledger
                .revisions("dgt-firm-berth-plan-2026-11-24")
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn ledger_keeps_source_publication_and_system_retrieval_as_of_times_distinct() {
        let mut ledger = SourceArtifactLedger::default();
        ledger
            .append(artifact("r1", 1_000, 1_100, None))
            .expect("r1");
        ledger
            .append(artifact("r2", 1_500, 1_600, Some("r1")))
            .expect("r2");

        let as_known_before_r2 = ledger
            .latest_available_as_of("dgt-firm-berth-plan-2026-11-24", 1_300, 1_700)
            .expect("source published r1 at cutoff");
        assert_eq!(as_known_before_r2.revision_id, "r1");

        let as_retrieved_before_r2 = ledger
            .latest_available_as_of("dgt-firm-berth-plan-2026-11-24", 2_000, 1_300)
            .expect("only r1 retrieved by cutoff");
        assert_eq!(as_retrieved_before_r2.revision_id, "r1");

        let as_known_after_r2 = ledger
            .latest_available_as_of("dgt-firm-berth-plan-2026-11-24", 2_000, 2_000)
            .expect("r2 available");
        assert_eq!(as_known_after_r2.revision_id, "r2");
    }

    #[test]
    fn valid_time_query_is_separate_from_publication_and_retrieval_time() {
        let mut ledger = SourceArtifactLedger::default();
        let mut first = artifact("r1", 1_000, 1_100, None);
        first.valid_time = ValidTime {
            from_unix_ms: Some(1_000),
            to_unix_ms: Some(2_000),
        };
        ledger.append(first).expect("r1");
        let mut second = artifact("r2", 1_500, 1_600, Some("r1"));
        second.valid_time = ValidTime {
            from_unix_ms: Some(2_001),
            to_unix_ms: Some(3_000),
        };
        ledger.append(second).expect("r2");

        let historical = ledger
            .latest_valid_at_as_of("dgt-firm-berth-plan-2026-11-24", 1_500, 2_000, 2_000)
            .expect("valid at 1500");
        assert_eq!(historical.revision_id, "r1");
        assert!(
            ledger
                .latest_valid_at_as_of("dgt-firm-berth-plan-2026-11-24", 4_000, 2_000, 2_000)
                .is_none()
        );
    }

    #[test]
    fn negative_control_receipt_is_deterministic_and_schema_stable() {
        let first = render_dgt_claim_negative_controls();
        assert_eq!(first, render_dgt_claim_negative_controls());
        assert_eq!(first.lines().count(), 13);
        let width = first.lines().next().unwrap().split(',').count();
        assert_eq!(width, 4);
        for row in first.lines().skip(1) {
            assert_eq!(row.split(',').count(), width);
        }
        assert!(first.contains(
            "DGT_BERTH_PLAN,PLANNED_BERTH_WINDOW,BERTH_OCCUPANCY_OBSERVED,REQUIRES_INDEPENDENT_PHYSICAL_OBSERVATION"
        ));
        assert!(first.contains(
            "DGT_VESSEL_STATUS_SNAPSHOT,TERMINAL_REPORTED_VESSEL_STATUS,VESSEL_DEPARTURE_OBSERVED,REQUIRES_INDEPENDENT_PHYSICAL_OBSERVATION"
        ));
        assert!(first.contains(
            "DGT_IMPORT_STORAGE_SNAPSHOT,IMPORT_INVENTORY_STATE,RECEIVER_ACCEPTED,REQUIRES_RECEIVER_ACCEPTANCE_RECEIPT"
        ));
    }
}
