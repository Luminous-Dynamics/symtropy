#![recursion_limit = "256"]

// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fail-closed projection of RRVR owner evidence into the pinned TLA+ model.
//!
//! The projected state is supplied by owners. This crate validates its exact
//! shape and selected invariants, then emits a Reflow-compatible state/event
//! JSONL trace plus a separate provenance sidecar. Missing fields are rejected;
//! no owner fact is filled with an optimistic default.

use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const CONTRACT_VERSION: &str = "0.1";
pub const REQUIRED_OWNER_DOMAINS: usize = 10;

pub const STATE_FIELDS: [&str; 43] = [
    "processStatus",
    "bootEpoch",
    "controllerEpoch",
    "stage",
    "reservationCommitted",
    "oldTicketBootEpoch",
    "oldTicketControllerEpoch",
    "oldAccepted",
    "oldAcceptedBootEpoch",
    "oldAcceptedControllerEpoch",
    "commandPending",
    "commandOwnerAttempt",
    "oldPhysicalOutcome",
    "recoveryPhysicalOutcome",
    "appliedEffectCount",
    "persistStatus",
    "storeRecord",
    "storeCallPending",
    "effectEvidence",
    "missionResolution",
    "observerIndependent",
    "queueFenceBootEpoch",
    "downstreamFenceBootEpoch",
    "recoveryPermit",
    "permitUseCount",
    "ticketStatus",
    "ticketBootEpoch",
    "ticketControllerEpoch",
    "ticketAttempt",
    "m1Current",
    "m2Current",
    "ticketBindingValid",
    "liveBindingValid",
    "recoveryAccepted",
    "admittedFreshBoot",
    "admittedFreshController",
    "admittedControllerFence",
    "admittedDownstreamFence",
    "admittedM1",
    "admittedM2",
    "admittedBinding",
    "naiveReplayCount",
    "crashCut",
];

pub const MODEL_STATE_FIELD_COUNT: usize = 43;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum OwnerEvidenceDomain {
    ProcessEpoch,
    Persistence,
    PhysicalOutcome,
    MissionResolution,
    ControllerQueue,
    DownstreamExecutionGraph,
    PhysicalOutcomeObserverQuorum,
    DownstreamObserverQuorum,
    ActionApplicability,
    RecoveryAuthority,
}

impl OwnerEvidenceDomain {
    fn as_str(self) -> &'static str {
        match self {
            Self::ProcessEpoch => "process_epoch",
            Self::Persistence => "persistence",
            Self::PhysicalOutcome => "physical_outcome",
            Self::MissionResolution => "mission_resolution",
            Self::ControllerQueue => "controller_queue",
            Self::DownstreamExecutionGraph => "downstream_execution_graph",
            Self::PhysicalOutcomeObserverQuorum => "physical_outcome_observer_quorum",
            Self::DownstreamObserverQuorum => "downstream_observer_quorum",
            Self::ActionApplicability => "action_applicability",
            Self::RecoveryAuthority => "recovery_authority",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OwnerEvidenceStatus {
    Current,
    Confirmed,
    Refuted,
    InsufficientEvidence,
    Unknown,
    Disputed,
    QuorumQualified,
    QuorumNotQualified,
    NotApplicable,
}

impl OwnerEvidenceStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Confirmed => "confirmed",
            Self::Refuted => "refuted",
            Self::InsufficientEvidence => "insufficient_evidence",
            Self::Unknown => "unknown",
            Self::Disputed => "disputed",
            Self::QuorumQualified => "quorum_qualified",
            Self::QuorumNotQualified => "quorum_not_qualified",
            Self::NotApplicable => "not_applicable",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceScope {
    /// Stable identity of the logical physical operation, independent of retries.
    pub logical_effect_id: String,
    /// Exact attempt to which this evidence applies (for this model: attempt-1 or attempt-2).
    pub attempt_id: String,
    /// Epoch in which the subject attempt was originally authorized.
    pub subject_boot_epoch: u64,
    pub subject_controller_epoch: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObserverMemberEvidence {
    pub observer_id: String,
    pub observer_revision: String,
    pub observation_id: String,
    /// Declared shared failure domains for this observer, e.g. power supply,
    /// clock source, host, firmware, calibration authority or network segment.
    pub common_cause_domains: Vec<String>,
    pub raw_artifact_id: String,
    pub raw_artifact_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObserverQuorumProof {
    pub fault_model_revision: String,
    pub required_independent_members: usize,
    pub members: Vec<ObserverMemberEvidence>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OwnerEvidenceRef {
    pub domain: OwnerEvidenceDomain,
    pub owner_id: String,
    pub owner_revision: String,
    pub evidence_id: String,
    pub status: OwnerEvidenceStatus,
    /// Exact owner-asserted value whose provenance is carried by this receipt.
    pub asserted_value: String,
    pub scope: EvidenceScope,
    /// Populated for claim-specific observer-quorum records.
    pub claim_id: Option<String>,
    /// SHA-256 of domain + claim ID + typed evidence scope.
    pub claim_scope_sha256: Option<String>,
    pub observer_quorum: Option<ObserverQuorumProof>,
    pub raw_artifact_id: String,
    pub raw_artifact_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SourceClock {
    pub clock_id: String,
    pub boot_epoch_id: String,
    pub unit: String,
    pub value: u64,
    pub resolution: u64,
    pub uncertainty: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSnapshot {
    pub event_id: String,
    pub logical_sequence: u64,
    pub model_action: Option<String>,
    pub model_arguments: Vec<Value>,
    pub predecessor_event_ids: Vec<String>,
    pub source_clock: Option<SourceClock>,
    /// Exact complete TLA variable record after this owner event.
    pub state_s: Value,
    pub owner_evidence: Vec<OwnerEvidenceRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProjectionHeader {
    pub bundle_id: String,
    pub system_id: String,
    pub implementation_revision: String,
    pub model_revision: String,
    pub model_source_sha256: String,
    pub model_config_sha256: String,
    pub tla_tool_sha256: String,
    pub fault_profile_revision: String,
    pub max_epoch: u64,
    pub naive_retry_enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeProjectionRecording {
    pub version: String,
    pub header: ProjectionHeader,
    pub snapshots: Vec<RuntimeSnapshot>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionError {
    MissingHeaderIdentity,
    MissingSnapshot,
    TooManySnapshots,
    InvalidSequence,
    InvalidEventIdentity,
    DuplicateEventIdentity,
    InvalidPredecessor,
    InvalidModelAction,
    MissingOwnerEvidence,
    DuplicateOwnerDomain,
    InvalidOwnerEvidence,
    InvalidSourceClock,
    MissingOrExtraStateField,
    InvalidStateFieldType,
    StateOutOfBounds,
    CommandOwnerMismatch,
    ReservationStageMismatch,
    OldAcceptanceLineageMismatch,
    DownstreamFenceInvariant,
    PersistenceRecordMismatch,
    DisputedObserverContradiction,
    RecoveryAdmissionInvariant,
    PermitConsumptionMismatch,
    RecoveryFromUnqualifiedOutcome,
    IndeterminatePersistenceRecovery,
    FreshTicketBindingMismatch,
    InitialStateMismatch,
    DuplicatePhysicalEffectObserved,
    ObserverQuorumMismatch,
    DuplicateObserverClaim,
    EvidenceScopeMismatch,
    ObserverQuorumMalformed,
    ObserverClaimsShareCommonCause,
    OwnerEvidenceStatusMismatch,
    OwnerAssertedValueMismatch,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}
fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn str_field<'a>(state: &'a Value, key: &'static str) -> Result<&'a str, ProjectionError> {
    state
        .get(key)
        .and_then(Value::as_str)
        .filter(|v| nonempty(v))
        .ok_or(ProjectionError::InvalidStateFieldType)
}
fn bool_field(state: &Value, key: &'static str) -> Result<bool, ProjectionError> {
    state
        .get(key)
        .and_then(Value::as_bool)
        .ok_or(ProjectionError::InvalidStateFieldType)
}
fn u64_field(state: &Value, key: &'static str) -> Result<u64, ProjectionError> {
    state
        .get(key)
        .and_then(Value::as_u64)
        .ok_or(ProjectionError::InvalidStateFieldType)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex(&digest)
}

fn claim_scope_digest(
    domain: OwnerEvidenceDomain,
    claim_id: &str,
    asserted_value: &str,
    scope: &EvidenceScope,
) -> String {
    let value = json!({
        "domain": domain.as_str(),
        "claim_id": claim_id,
        "asserted_value": asserted_value,
        "scope": {
            "logical_effect_id": scope.logical_effect_id,
            "attempt_id": scope.attempt_id,
            "subject_boot_epoch": scope.subject_boot_epoch,
            "subject_controller_epoch": scope.subject_controller_epoch
        }
    });
    let bytes = serde_json::to_vec(&value).expect("typed claim scope serializes");
    sha256_hex(&bytes)
}

fn observer_quorum_is_qualified(proof: &ObserverQuorumProof) -> Result<bool, ProjectionError> {
    if !nonempty(&proof.fault_model_revision)
        || proof.required_independent_members < 2
        || proof.required_independent_members > 32
        || proof.members.is_empty()
        || proof.members.len() > 32
    {
        return Err(ProjectionError::ObserverQuorumMalformed);
    }

    let mut observer_ids = BTreeSet::new();
    let mut observation_ids = BTreeSet::new();
    let mut artifact_ids = BTreeSet::new();
    let mut artifact_hashes = BTreeSet::new();
    let mut seen_domains = BTreeSet::new();
    let mut pairwise_independent = true;
    for member in &proof.members {
        if !nonempty(&member.observer_id)
            || !nonempty(&member.observer_revision)
            || !nonempty(&member.observation_id)
            || !nonempty(&member.raw_artifact_id)
            || !valid_sha256(&member.raw_artifact_sha256)
            || member.common_cause_domains.is_empty()
            || member
                .common_cause_domains
                .iter()
                .any(|domain| !nonempty(domain))
            || !observer_ids.insert(member.observer_id.as_str())
            || !observation_ids.insert(member.observation_id.as_str())
            || !artifact_ids.insert(member.raw_artifact_id.as_str())
            || !artifact_hashes.insert(member.raw_artifact_sha256.as_str())
        {
            return Err(ProjectionError::ObserverQuorumMalformed);
        }
        for domain in &member.common_cause_domains {
            if !seen_domains.insert(domain.as_str()) {
                pairwise_independent = false;
            }
        }
    }

    // This is independence only under the declared, versioned fault model.
    Ok(pairwise_independent && proof.members.len() >= proof.required_independent_members)
}

fn observer_proofs_are_independent(
    physical_outcome: &ObserverQuorumProof,
    downstream: &ObserverQuorumProof,
) -> bool {
    if physical_outcome.fault_model_revision != downstream.fault_model_revision {
        return false;
    }
    let physical_observers: BTreeSet<&str> = physical_outcome
        .members
        .iter()
        .map(|member| member.observer_id.as_str())
        .collect();
    let downstream_observers: BTreeSet<&str> = downstream
        .members
        .iter()
        .map(|member| member.observer_id.as_str())
        .collect();
    if !physical_observers.is_disjoint(&downstream_observers) {
        return false;
    }

    let physical_dependencies: BTreeSet<&str> = physical_outcome
        .members
        .iter()
        .flat_map(|member| member.common_cause_domains.iter().map(String::as_str))
        .collect();
    let downstream_dependencies: BTreeSet<&str> = downstream
        .members
        .iter()
        .flat_map(|member| member.common_cause_domains.iter().map(String::as_str))
        .collect();
    if !physical_dependencies.is_disjoint(&downstream_dependencies) {
        return false;
    }

    let physical_artifacts: BTreeSet<&str> = physical_outcome
        .members
        .iter()
        .map(|member| member.raw_artifact_id.as_str())
        .collect();
    let downstream_artifacts: BTreeSet<&str> = downstream
        .members
        .iter()
        .map(|member| member.raw_artifact_id.as_str())
        .collect();
    let physical_hashes: BTreeSet<&str> = physical_outcome
        .members
        .iter()
        .map(|member| member.raw_artifact_sha256.as_str())
        .collect();
    let downstream_hashes: BTreeSet<&str> = downstream
        .members
        .iter()
        .map(|member| member.raw_artifact_sha256.as_str())
        .collect();
    let physical_observations: BTreeSet<&str> = physical_outcome
        .members
        .iter()
        .map(|member| member.observation_id.as_str())
        .collect();
    let downstream_observations: BTreeSet<&str> = downstream
        .members
        .iter()
        .map(|member| member.observation_id.as_str())
        .collect();
    physical_artifacts.is_disjoint(&downstream_artifacts)
        && physical_hashes.is_disjoint(&downstream_hashes)
        && physical_observations.is_disjoint(&downstream_observations)
}

pub fn model_init_state() -> Value {
    json!({
        "processStatus":"running","bootEpoch":1,"controllerEpoch":1,"stage":"unreserved",
        "reservationCommitted":false,"oldTicketBootEpoch":1,"oldTicketControllerEpoch":1,
        "oldAccepted":false,"oldAcceptedBootEpoch":0,"oldAcceptedControllerEpoch":0,
        "commandPending":false,"commandOwnerAttempt":"none","oldPhysicalOutcome":"none",
        "recoveryPhysicalOutcome":"none","appliedEffectCount":0,"persistStatus":"definitely_not_committed",
        "storeRecord":false,"storeCallPending":false,"effectEvidence":"none","missionResolution":"none",
        "observerIndependent":true,"queueFenceBootEpoch":0,"downstreamFenceBootEpoch":0,
        "recoveryPermit":"absent","permitUseCount":0,"ticketStatus":"old","ticketBootEpoch":1,
        "ticketControllerEpoch":1,"ticketAttempt":"attempt-1","m1Current":true,"m2Current":true,
        "ticketBindingValid":true,"liveBindingValid":true,"recoveryAccepted":false,
        "admittedFreshBoot":false,"admittedFreshController":false,"admittedControllerFence":false,
        "admittedDownstreamFence":false,"admittedM1":false,"admittedM2":false,"admittedBinding":false,
        "naiveReplayCount":0,"crashCut":"none"
    })
}

fn validate_state(state: &Value, max_epoch: u64) -> Result<(), ProjectionError> {
    let object = state
        .as_object()
        .ok_or(ProjectionError::MissingOrExtraStateField)?;
    if object.len() != MODEL_STATE_FIELD_COUNT
        || STATE_FIELDS
            .iter()
            .any(|field| !object.contains_key(*field))
    {
        return Err(ProjectionError::MissingOrExtraStateField);
    }
    for key in [
        "processStatus",
        "stage",
        "commandOwnerAttempt",
        "oldPhysicalOutcome",
        "recoveryPhysicalOutcome",
        "persistStatus",
        "effectEvidence",
        "missionResolution",
        "recoveryPermit",
        "ticketStatus",
        "ticketAttempt",
        "crashCut",
    ] {
        let value = str_field(state, key)?;
        let allowed: &[&str] = match key {
            "processStatus" => &["running", "crashed"],
            "stage" => &["unreserved", "reserved", "dispatch_started", "accepted"],
            "commandOwnerAttempt" => &["none", "attempt-1", "attempt-2"],
            "oldPhysicalOutcome" | "recoveryPhysicalOutcome" => {
                &["none", "applied", "no_effect", "partial"]
            }
            "persistStatus" => &[
                "definitely_not_committed",
                "in_flight",
                "committed",
                "indeterminate_commit",
            ],
            "effectEvidence" | "missionResolution" => &[
                "none",
                "unknown",
                "applied",
                "no_effect",
                "partial",
                "disputed",
            ],
            "recoveryPermit" => &["absent", "issued", "consumed"],
            "ticketStatus" => &["old", "fresh", "consumed"],
            "ticketAttempt" => &["attempt-1", "attempt-2"],
            "crashCut" => &[
                "none",
                "before_reservation",
                "after_reservation",
                "after_dispatch_start",
                "after_native_accept",
                "after_possible_effect",
                "after_restart_before_fence",
                "after_controller_queue_fence",
                "after_downstream_execution_fence",
                "permit_consumed_before_effect",
                "after_recovery_restart",
            ],
            _ => &[],
        };
        if !allowed.contains(&value) {
            return Err(ProjectionError::InvalidStateFieldType);
        }
    }
    for key in [
        "reservationCommitted",
        "oldAccepted",
        "commandPending",
        "storeRecord",
        "storeCallPending",
        "observerIndependent",
        "m1Current",
        "m2Current",
        "ticketBindingValid",
        "liveBindingValid",
        "recoveryAccepted",
        "admittedFreshBoot",
        "admittedFreshController",
        "admittedControllerFence",
        "admittedDownstreamFence",
        "admittedM1",
        "admittedM2",
        "admittedBinding",
    ] {
        bool_field(state, key)?;
    }
    for key in [
        "bootEpoch",
        "controllerEpoch",
        "oldTicketBootEpoch",
        "oldTicketControllerEpoch",
        "oldAcceptedBootEpoch",
        "oldAcceptedControllerEpoch",
        "appliedEffectCount",
        "queueFenceBootEpoch",
        "downstreamFenceBootEpoch",
        "permitUseCount",
        "ticketBootEpoch",
        "ticketControllerEpoch",
        "naiveReplayCount",
    ] {
        u64_field(state, key)?;
    }

    let boot = u64_field(state, "bootEpoch")?;
    let controller = u64_field(state, "controllerEpoch")?;
    if max_epoch < 3
        || boot == 0
        || boot > max_epoch
        || controller == 0
        || controller > max_epoch
        || u64_field(state, "oldTicketBootEpoch")? == 0
        || u64_field(state, "oldTicketBootEpoch")? > max_epoch
        || u64_field(state, "oldTicketControllerEpoch")? == 0
        || u64_field(state, "oldTicketControllerEpoch")? > max_epoch
        || u64_field(state, "oldAcceptedBootEpoch")? > max_epoch
        || u64_field(state, "oldAcceptedControllerEpoch")? > max_epoch
        || u64_field(state, "queueFenceBootEpoch")? > max_epoch
        || u64_field(state, "downstreamFenceBootEpoch")? > max_epoch
        || u64_field(state, "ticketBootEpoch")? == 0
        || u64_field(state, "ticketBootEpoch")? > max_epoch
        || u64_field(state, "ticketControllerEpoch")? == 0
        || u64_field(state, "ticketControllerEpoch")? > max_epoch
        || u64_field(state, "naiveReplayCount")? > 2
        || u64_field(state, "permitUseCount")? > 1
    {
        return Err(ProjectionError::StateOutOfBounds);
    }

    // NoDuplicatePhysicalEffects is <= 1, stricter than the TypeOK range 0..2.
    if u64_field(state, "appliedEffectCount")? > 1 {
        return Err(ProjectionError::DuplicatePhysicalEffectObserved);
    }
    let pending = bool_field(state, "commandPending")?;
    let owner = str_field(state, "commandOwnerAttempt")?;
    if pending == (owner == "none") {
        return Err(ProjectionError::CommandOwnerMismatch);
    }
    if str_field(state, "stage")? != "unreserved" && !bool_field(state, "reservationCommitted")? {
        return Err(ProjectionError::ReservationStageMismatch);
    }
    if bool_field(state, "oldAccepted")?
        && (!bool_field(state, "reservationCommitted")?
            || u64_field(state, "oldAcceptedBootEpoch")? != 1
            || u64_field(state, "oldAcceptedControllerEpoch")? != 1)
    {
        return Err(ProjectionError::OldAcceptanceLineageMismatch);
    }
    if u64_field(state, "downstreamFenceBootEpoch")? == boot
        && (u64_field(state, "queueFenceBootEpoch")? != boot || pending)
    {
        return Err(ProjectionError::DownstreamFenceInvariant);
    }

    let persist = str_field(state, "persistStatus")?;
    if (persist == "committed" && !bool_field(state, "storeRecord")?)
        || (persist == "definitely_not_committed" && bool_field(state, "storeRecord")?)
    {
        return Err(ProjectionError::PersistenceRecordMismatch);
    }
    let evidence = str_field(state, "effectEvidence")?;
    let old_physical = str_field(state, "oldPhysicalOutcome")?;
    let mission = str_field(state, "missionResolution")?;
    let permit = str_field(state, "recoveryPermit")?;
    let accepted = bool_field(state, "recoveryAccepted")?;
    let use_count = u64_field(state, "permitUseCount")?;
    if (permit == "consumed") != (use_count == 1) || accepted != (permit == "consumed") {
        return Err(ProjectionError::PermitConsumptionMismatch);
    }
    if persist == "indeterminate_commit" && permit != "absent" {
        return Err(ProjectionError::IndeterminatePersistenceRecovery);
    }
    if permit != "absent"
        && (evidence != "no_effect"
            || old_physical != "no_effect"
            || mission != "no_effect"
            || !["committed", "definitely_not_committed"].contains(&persist)
            || bool_field(state, "storeCallPending")?
            || !bool_field(state, "m1Current")?
            || !bool_field(state, "m2Current")?)
    {
        return Err(ProjectionError::RecoveryFromUnqualifiedOutcome);
    }
    if permit == "issued"
        && (u64_field(state, "queueFenceBootEpoch")? != boot
            || u64_field(state, "downstreamFenceBootEpoch")? != boot)
    {
        return Err(ProjectionError::RecoveryFromUnqualifiedOutcome);
    }

    if str_field(state, "ticketStatus")? == "fresh"
        && (permit != "issued"
            || str_field(state, "ticketAttempt")? != "attempt-2"
            || u64_field(state, "ticketBootEpoch")? != boot
            || u64_field(state, "ticketControllerEpoch")? != controller
            || u64_field(state, "queueFenceBootEpoch")? != boot
            || u64_field(state, "downstreamFenceBootEpoch")? != boot
            || !bool_field(state, "ticketBindingValid")?
            || !bool_field(state, "liveBindingValid")?)
    {
        return Err(ProjectionError::FreshTicketBindingMismatch);
    }

    if accepted {
        // NativeAcceptRecovery captures admission-time fence truth, then clears
        // downstreamFenceBootEpoch to permit the recovery command to execute.
        if old_physical != "no_effect"
            || evidence != "no_effect"
            || mission != "no_effect"
            || persist == "indeterminate_commit"
            || !bool_field(state, "ticketBindingValid")?
            || !bool_field(state, "liveBindingValid")?
            || !bool_field(state, "admittedFreshBoot")?
            || !bool_field(state, "admittedFreshController")?
            || !bool_field(state, "admittedControllerFence")?
            || !bool_field(state, "admittedDownstreamFence")?
            || !bool_field(state, "admittedM1")?
            || !bool_field(state, "admittedM2")?
            || !bool_field(state, "admittedBinding")?
            || str_field(state, "ticketStatus")? != "consumed"
            || str_field(state, "ticketAttempt")? != "attempt-2"
            || u64_field(state, "ticketBootEpoch")? != boot
            || u64_field(state, "ticketControllerEpoch")? != controller
            || u64_field(state, "queueFenceBootEpoch")? != boot
        {
            return Err(ProjectionError::RecoveryAdmissionInvariant);
        }
    }
    if evidence == "disputed"
        && (old_physical == "none" || bool_field(state, "observerIndependent")?)
    {
        return Err(ProjectionError::DisputedObserverContradiction);
    }
    if matches!(evidence, "applied" | "no_effect" | "partial") && old_physical != evidence {
        return Err(ProjectionError::DisputedObserverContradiction);
    }
    if mission != "none" && mission != evidence {
        return Err(ProjectionError::DisputedObserverContradiction);
    }
    Ok(())
}

fn valid_sha(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn known_action(action: &str) -> bool {
    matches!(
        action,
        "ReserveAttempt"
            | "MarkDispatchStarted"
            | "NativeAcceptOld"
            | "OldDeviceApplies"
            | "OldDeviceDoesNotApply"
            | "OldDevicePartiallyApplies"
            | "BeginEffectRecordCommit"
            | "CommitEffectRecord"
            | "AbortEffectRecord"
            | "AcknowledgeStoreCommit"
            | "AcknowledgeStoreNoCommit"
            | "Crash"
            | "Restart"
            | "FenceControllerQueue"
            | "FenceDownstreamExecution"
            | "ResolvePersistence"
            | "ObserveOldEffect"
            | "ObserveUnknownEffect"
            | "ObserveDisputedEffect"
            | "ResolveMission"
            | "IssueRecoveryPermit"
            | "CreateFreshTicket"
            | "RevokeM1"
            | "RevokeM2"
            | "ChangeLiveBinding"
            | "NativeAcceptRecovery"
            | "RecoveryDeviceApplies"
            | "RecoveryDeviceDoesNotApply"
            | "RecoveryDevicePartiallyApplies"
            | "NaiveRetry"
    )
}

impl RuntimeProjectionRecording {
    pub fn validate(&self) -> Result<(), ProjectionError> {
        let h = &self.header;
        if self.version != CONTRACT_VERSION
            || [
                &h.bundle_id,
                &h.system_id,
                &h.implementation_revision,
                &h.model_revision,
                &h.fault_profile_revision,
            ]
            .iter()
            .any(|v| !nonempty(v))
            || !valid_sha(&h.model_source_sha256)
            || !valid_sha(&h.model_config_sha256)
            || !valid_sha(&h.tla_tool_sha256)
            || h.max_epoch < 3
        {
            return Err(ProjectionError::MissingHeaderIdentity);
        }
        if self.snapshots.is_empty() {
            return Err(ProjectionError::MissingSnapshot);
        }
        if self.snapshots.len() > 512 {
            return Err(ProjectionError::TooManySnapshots);
        }
        let mut ids = BTreeSet::new();
        let required = BTreeSet::from([
            OwnerEvidenceDomain::ProcessEpoch,
            OwnerEvidenceDomain::Persistence,
            OwnerEvidenceDomain::PhysicalOutcome,
            OwnerEvidenceDomain::MissionResolution,
            OwnerEvidenceDomain::ControllerQueue,
            OwnerEvidenceDomain::DownstreamExecutionGraph,
            OwnerEvidenceDomain::PhysicalOutcomeObserverQuorum,
            OwnerEvidenceDomain::DownstreamObserverQuorum,
            OwnerEvidenceDomain::ActionApplicability,
            OwnerEvidenceDomain::RecoveryAuthority,
        ]);
        for (i, s) in self.snapshots.iter().enumerate() {
            if !nonempty(&s.event_id) {
                return Err(ProjectionError::InvalidEventIdentity);
            }
            if !ids.insert(s.event_id.as_str()) {
                return Err(ProjectionError::DuplicateEventIdentity);
            }
            if s.logical_sequence != i as u64 + 1 {
                return Err(ProjectionError::InvalidSequence);
            }
            if i == 0 {
                if s.model_action.is_some()
                    || !s.predecessor_event_ids.is_empty()
                    || s.state_s != model_init_state()
                {
                    return Err(ProjectionError::InitialStateMismatch);
                }
            } else if s.predecessor_event_ids != vec![self.snapshots[i - 1].event_id.clone()] {
                return Err(ProjectionError::InvalidPredecessor);
            }
            if s.model_action
                .as_ref()
                .is_some_and(|action| !known_action(action))
            {
                return Err(ProjectionError::InvalidModelAction);
            }
            validate_state(&s.state_s, h.max_epoch)?;
            let domains: BTreeSet<OwnerEvidenceDomain> =
                s.owner_evidence.iter().map(|e| e.domain).collect();
            if domains != required || s.owner_evidence.len() != REQUIRED_OWNER_DOMAINS {
                return Err(ProjectionError::MissingOwnerEvidence);
            }
            for evidence in &s.owner_evidence {
                if !nonempty(&evidence.owner_id)
                    || !nonempty(&evidence.owner_revision)
                    || !nonempty(&evidence.evidence_id)
                    || !nonempty(&evidence.asserted_value)
                    || !nonempty(&evidence.raw_artifact_id)
                    || !valid_sha256(&evidence.raw_artifact_sha256)
                    || !nonempty(&evidence.scope.logical_effect_id)
                    || !matches!(
                        evidence.scope.attempt_id.as_str(),
                        "attempt-1" | "attempt-2"
                    )
                    || evidence.scope.subject_boot_epoch == 0
                    || evidence.scope.subject_boot_epoch > h.max_epoch
                    || evidence.scope.subject_controller_epoch == 0
                    || evidence.scope.subject_controller_epoch > h.max_epoch
                {
                    return Err(ProjectionError::InvalidOwnerEvidence);
                }

                let observer_domain = matches!(
                    evidence.domain,
                    OwnerEvidenceDomain::PhysicalOutcomeObserverQuorum
                        | OwnerEvidenceDomain::DownstreamObserverQuorum
                );
                if observer_domain {
                    if evidence.status == OwnerEvidenceStatus::NotApplicable {
                        if evidence.claim_id.is_some()
                            || evidence.claim_scope_sha256.is_some()
                            || evidence.observer_quorum.is_some()
                        {
                            return Err(ProjectionError::InvalidOwnerEvidence);
                        }
                    } else {
                        let claim_id = evidence
                            .claim_id
                            .as_deref()
                            .filter(|v| nonempty(v))
                            .ok_or(ProjectionError::InvalidOwnerEvidence)?;
                        let expected = claim_scope_digest(
                            evidence.domain,
                            claim_id,
                            &evidence.asserted_value,
                            &evidence.scope,
                        );
                        if evidence.claim_scope_sha256.as_deref() != Some(expected.as_str()) {
                            return Err(ProjectionError::InvalidOwnerEvidence);
                        }
                        match evidence.status {
                            OwnerEvidenceStatus::QuorumQualified
                            | OwnerEvidenceStatus::QuorumNotQualified => {
                                let proof = evidence
                                    .observer_quorum
                                    .as_ref()
                                    .ok_or(ProjectionError::ObserverQuorumMalformed)?;
                                let qualified = observer_quorum_is_qualified(proof)?;
                                if (evidence.status == OwnerEvidenceStatus::QuorumQualified)
                                    != qualified
                                {
                                    return Err(ProjectionError::ObserverQuorumMismatch);
                                }
                            }
                            OwnerEvidenceStatus::Unknown
                            | OwnerEvidenceStatus::InsufficientEvidence
                            | OwnerEvidenceStatus::Disputed => {
                                if let Some(proof) = evidence.observer_quorum.as_ref() {
                                    let _ = observer_quorum_is_qualified(proof)?;
                                }
                            }
                            _ => return Err(ProjectionError::InvalidOwnerEvidence),
                        }
                    }
                } else if evidence.claim_id.is_some()
                    || evidence.claim_scope_sha256.is_some()
                    || evidence.observer_quorum.is_some()
                    || matches!(
                        evidence.status,
                        OwnerEvidenceStatus::QuorumQualified
                            | OwnerEvidenceStatus::QuorumNotQualified
                    )
                {
                    return Err(ProjectionError::InvalidOwnerEvidence);
                }
            }

            let effect_scope =
                &owner_evidence_by_domain(s, OwnerEvidenceDomain::PhysicalOutcome)?.scope;
            for domain in [
                OwnerEvidenceDomain::Persistence,
                OwnerEvidenceDomain::PhysicalOutcome,
                OwnerEvidenceDomain::MissionResolution,
                OwnerEvidenceDomain::ControllerQueue,
                OwnerEvidenceDomain::DownstreamExecutionGraph,
                OwnerEvidenceDomain::PhysicalOutcomeObserverQuorum,
                OwnerEvidenceDomain::DownstreamObserverQuorum,
            ] {
                if &owner_evidence_by_domain(s, domain)?.scope != effect_scope {
                    return Err(ProjectionError::EvidenceScopeMismatch);
                }
            }
            let state_boot = u64_field(&s.state_s, "bootEpoch")?;
            let state_controller = u64_field(&s.state_s, "controllerEpoch")?;
            if effect_scope.attempt_id != "attempt-1"
                || effect_scope.subject_boot_epoch != u64_field(&s.state_s, "oldTicketBootEpoch")?
                || effect_scope.subject_controller_epoch
                    != u64_field(&s.state_s, "oldTicketControllerEpoch")?
            {
                return Err(ProjectionError::EvidenceScopeMismatch);
            }
            let applicability_scope =
                &owner_evidence_by_domain(s, OwnerEvidenceDomain::ActionApplicability)?.scope;
            let recovery_scope =
                &owner_evidence_by_domain(s, OwnerEvidenceDomain::RecoveryAuthority)?.scope;
            let process_scope =
                &owner_evidence_by_domain(s, OwnerEvidenceDomain::ProcessEpoch)?.scope;
            if applicability_scope != recovery_scope
                || recovery_scope.attempt_id != "attempt-2"
                || recovery_scope.subject_boot_epoch != state_boot
                || recovery_scope.subject_controller_epoch != state_controller
                || process_scope.subject_boot_epoch != state_boot
                || process_scope.subject_controller_epoch != state_controller
            {
                return Err(ProjectionError::EvidenceScopeMismatch);
            }

            let state_persist = str_field(&s.state_s, "persistStatus")?;
            let state_old_outcome = str_field(&s.state_s, "oldPhysicalOutcome")?;
            let state_mission = str_field(&s.state_s, "missionResolution")?;
            let state_permit = str_field(&s.state_s, "recoveryPermit")?;
            let state_queue_fence = u64_field(&s.state_s, "queueFenceBootEpoch")?;
            let state_downstream_fence = u64_field(&s.state_s, "downstreamFenceBootEpoch")?;
            let state_m1 = bool_field(&s.state_s, "m1Current")?;
            let state_m2 = bool_field(&s.state_s, "m2Current")?;

            let state_effect_evidence = str_field(&s.state_s, "effectEvidence")?;
            let expected_controller_assertion = if state_queue_fence == state_boot {
                "fenced"
            } else if state_queue_fence == 0 {
                "not_fenced"
            } else {
                "stale_fence"
            };
            let expected_graph_assertion = if state_downstream_fence == state_boot
                || bool_field(&s.state_s, "admittedDownstreamFence")?
            {
                "quiescent"
            } else if state_downstream_fence == 0 {
                "not_fenced"
            } else {
                "stale_fence"
            };
            for evidence in &s.owner_evidence {
                let expected = match evidence.domain {
                    OwnerEvidenceDomain::ProcessEpoch => str_field(&s.state_s, "processStatus")?,
                    OwnerEvidenceDomain::Persistence => state_persist,
                    OwnerEvidenceDomain::PhysicalOutcome => state_old_outcome,
                    OwnerEvidenceDomain::MissionResolution => state_mission,
                    OwnerEvidenceDomain::ControllerQueue => expected_controller_assertion,
                    OwnerEvidenceDomain::DownstreamExecutionGraph => expected_graph_assertion,
                    OwnerEvidenceDomain::PhysicalOutcomeObserverQuorum => state_effect_evidence,
                    OwnerEvidenceDomain::DownstreamObserverQuorum => match evidence.status {
                        OwnerEvidenceStatus::NotApplicable => "not_applicable",
                        OwnerEvidenceStatus::QuorumQualified => "quiescent",
                        OwnerEvidenceStatus::QuorumNotQualified => "not_quiescent",
                        OwnerEvidenceStatus::Unknown
                        | OwnerEvidenceStatus::InsufficientEvidence => "unknown",
                        OwnerEvidenceStatus::Disputed => "disputed",
                        _ => return Err(ProjectionError::OwnerAssertedValueMismatch),
                    },
                    OwnerEvidenceDomain::ActionApplicability => {
                        if state_m1 && state_m2 {
                            "current"
                        } else {
                            "revoked"
                        }
                    }
                    OwnerEvidenceDomain::RecoveryAuthority => state_permit,
                };
                if evidence.asserted_value != expected {
                    return Err(ProjectionError::OwnerAssertedValueMismatch);
                }
            }

            let persistence_status =
                owner_evidence_by_domain(s, OwnerEvidenceDomain::Persistence)?.status;
            match state_persist {
                "committed" => {
                    if persistence_status != OwnerEvidenceStatus::Confirmed {
                        return Err(ProjectionError::OwnerEvidenceStatusMismatch);
                    }
                }
                "in_flight" | "indeterminate_commit" => {
                    if !matches!(
                        persistence_status,
                        OwnerEvidenceStatus::Unknown | OwnerEvidenceStatus::InsufficientEvidence
                    ) {
                        return Err(ProjectionError::OwnerEvidenceStatusMismatch);
                    }
                }
                "definitely_not_committed" => {
                    if !matches!(
                        persistence_status,
                        OwnerEvidenceStatus::NotApplicable | OwnerEvidenceStatus::Confirmed
                    ) {
                        return Err(ProjectionError::OwnerEvidenceStatusMismatch);
                    }
                }
                _ => return Err(ProjectionError::OwnerEvidenceStatusMismatch),
            }
            if state_permit != "absent" && persistence_status != OwnerEvidenceStatus::Confirmed {
                return Err(ProjectionError::OwnerEvidenceStatusMismatch);
            }

            let physical_status =
                owner_evidence_by_domain(s, OwnerEvidenceDomain::PhysicalOutcome)?.status;
            match (state_old_outcome, state_effect_evidence) {
                ("none", "none") => {
                    if physical_status != OwnerEvidenceStatus::NotApplicable {
                        return Err(ProjectionError::OwnerEvidenceStatusMismatch);
                    }
                }
                (outcome, "none") if outcome != "none" => {
                    if physical_status != OwnerEvidenceStatus::InsufficientEvidence {
                        return Err(ProjectionError::OwnerEvidenceStatusMismatch);
                    }
                }
                (_, "unknown") => {
                    if !matches!(
                        physical_status,
                        OwnerEvidenceStatus::Unknown | OwnerEvidenceStatus::InsufficientEvidence
                    ) {
                        return Err(ProjectionError::OwnerEvidenceStatusMismatch);
                    }
                }
                (_, "disputed") => {
                    if physical_status != OwnerEvidenceStatus::Disputed {
                        return Err(ProjectionError::OwnerEvidenceStatusMismatch);
                    }
                }
                ("applied", "applied") | ("no_effect", "no_effect") | ("partial", "partial") => {
                    if physical_status != OwnerEvidenceStatus::Confirmed {
                        return Err(ProjectionError::OwnerEvidenceStatusMismatch);
                    }
                }
                _ => return Err(ProjectionError::OwnerEvidenceStatusMismatch),
            }

            let mission_status =
                owner_evidence_by_domain(s, OwnerEvidenceDomain::MissionResolution)?.status;
            match state_mission {
                "none" => {
                    if mission_status != OwnerEvidenceStatus::NotApplicable {
                        return Err(ProjectionError::OwnerEvidenceStatusMismatch);
                    }
                }
                "unknown" => {
                    if !matches!(
                        mission_status,
                        OwnerEvidenceStatus::Unknown | OwnerEvidenceStatus::InsufficientEvidence
                    ) {
                        return Err(ProjectionError::OwnerEvidenceStatusMismatch);
                    }
                }
                "disputed" => {
                    if mission_status != OwnerEvidenceStatus::Disputed {
                        return Err(ProjectionError::OwnerEvidenceStatusMismatch);
                    }
                }
                "applied" | "no_effect" | "partial" => {
                    if mission_status != OwnerEvidenceStatus::Confirmed {
                        return Err(ProjectionError::OwnerEvidenceStatusMismatch);
                    }
                }
                _ => return Err(ProjectionError::OwnerEvidenceStatusMismatch),
            }

            let controller_status =
                owner_evidence_by_domain(s, OwnerEvidenceDomain::ControllerQueue)?.status;
            let expected_controller_status = if state_queue_fence == state_boot {
                OwnerEvidenceStatus::Confirmed
            } else if state_queue_fence == 0 {
                OwnerEvidenceStatus::NotApplicable
            } else {
                OwnerEvidenceStatus::Refuted
            };
            if controller_status != expected_controller_status {
                return Err(ProjectionError::OwnerEvidenceStatusMismatch);
            }

            let graph_status =
                owner_evidence_by_domain(s, OwnerEvidenceDomain::DownstreamExecutionGraph)?.status;
            let expected_graph_status = if state_downstream_fence == state_boot
                || bool_field(&s.state_s, "admittedDownstreamFence")?
            {
                OwnerEvidenceStatus::Confirmed
            } else if state_downstream_fence == 0 {
                OwnerEvidenceStatus::NotApplicable
            } else {
                OwnerEvidenceStatus::Refuted
            };
            if graph_status != expected_graph_status {
                return Err(ProjectionError::OwnerEvidenceStatusMismatch);
            }

            let process_status =
                owner_evidence_by_domain(s, OwnerEvidenceDomain::ProcessEpoch)?.status;
            if !matches!(
                process_status,
                OwnerEvidenceStatus::Current | OwnerEvidenceStatus::Confirmed
            ) {
                return Err(ProjectionError::OwnerEvidenceStatusMismatch);
            }

            let applicability_status =
                owner_evidence_by_domain(s, OwnerEvidenceDomain::ActionApplicability)?.status;
            if state_m1 && state_m2 && applicability_status != OwnerEvidenceStatus::Current {
                return Err(ProjectionError::OwnerEvidenceStatusMismatch);
            }
            if (!state_m1 || !state_m2) && applicability_status != OwnerEvidenceStatus::Refuted {
                return Err(ProjectionError::OwnerEvidenceStatusMismatch);
            }
            let authority_status =
                owner_evidence_by_domain(s, OwnerEvidenceDomain::RecoveryAuthority)?.status;
            if state_permit == "absent"
                && !matches!(
                    authority_status,
                    OwnerEvidenceStatus::NotApplicable | OwnerEvidenceStatus::Current
                )
            {
                return Err(ProjectionError::OwnerEvidenceStatusMismatch);
            }
            if state_permit != "absent" && authority_status != OwnerEvidenceStatus::Confirmed {
                return Err(ProjectionError::OwnerEvidenceStatusMismatch);
            }

            let physical_outcome_observer =
                owner_evidence_by_domain(s, OwnerEvidenceDomain::PhysicalOutcomeObserverQuorum)?;
            let downstream_observer =
                owner_evidence_by_domain(s, OwnerEvidenceDomain::DownstreamObserverQuorum)?;
            if let (Some(a), Some(b)) = (
                physical_outcome_observer.claim_id.as_deref(),
                downstream_observer.claim_id.as_deref(),
            ) {
                if a == b {
                    return Err(ProjectionError::DuplicateObserverClaim);
                }
            }
            let independent = bool_field(&s.state_s, "observerIndependent")?;
            match state_effect_evidence {
                "none" => {
                    if physical_outcome_observer.status != OwnerEvidenceStatus::NotApplicable {
                        return Err(ProjectionError::ObserverQuorumMismatch);
                    }
                }
                "unknown" => {
                    if !matches!(
                        physical_outcome_observer.status,
                        OwnerEvidenceStatus::Unknown | OwnerEvidenceStatus::InsufficientEvidence
                    ) {
                        return Err(ProjectionError::ObserverQuorumMismatch);
                    }
                }
                "disputed" => {
                    if independent
                        || physical_outcome_observer.status != OwnerEvidenceStatus::Disputed
                    {
                        return Err(ProjectionError::ObserverQuorumMismatch);
                    }
                }
                "applied" | "no_effect" | "partial" => {
                    if !independent
                        || physical_outcome_observer.status != OwnerEvidenceStatus::QuorumQualified
                        || downstream_observer.status != OwnerEvidenceStatus::QuorumQualified
                    {
                        return Err(ProjectionError::ObserverQuorumMismatch);
                    }
                }
                _ => return Err(ProjectionError::ObserverQuorumMismatch),
            }
            let permit = str_field(&s.state_s, "recoveryPermit")?;
            let requires_quorums = permit != "absent"
                || matches!(
                    s.model_action.as_deref(),
                    Some("IssueRecoveryPermit" | "NativeAcceptRecovery")
                );
            if requires_quorums
                && (physical_outcome_observer.status != OwnerEvidenceStatus::QuorumQualified
                    || downstream_observer.status != OwnerEvidenceStatus::QuorumQualified)
            {
                return Err(ProjectionError::ObserverQuorumMismatch);
            }
            if physical_outcome_observer.status == OwnerEvidenceStatus::QuorumQualified
                && downstream_observer.status == OwnerEvidenceStatus::QuorumQualified
            {
                let physical_outcome_proof = physical_outcome_observer
                    .observer_quorum
                    .as_ref()
                    .ok_or(ProjectionError::ObserverQuorumMalformed)?;
                let downstream_proof = downstream_observer
                    .observer_quorum
                    .as_ref()
                    .ok_or(ProjectionError::ObserverQuorumMalformed)?;
                if !observer_proofs_are_independent(physical_outcome_proof, downstream_proof) {
                    return Err(ProjectionError::ObserverClaimsShareCommonCause);
                }
            }

            if let Some(clock) = &s.source_clock {
                if !nonempty(&clock.clock_id)
                    || !nonempty(&clock.boot_epoch_id)
                    || !nonempty(&clock.unit)
                    || clock.resolution == 0
                {
                    return Err(ProjectionError::InvalidSourceClock);
                }
            }
        }
        Ok(())
    }

    pub fn to_tla_trace_ndjson(&self) -> Result<String, ProjectionError> {
        self.validate()?;
        let lines: Vec<String> = self
            .snapshots
            .iter()
            .map(|s| {
                let mut v = json!({"state":{"s":s.state_s}});
                if let Some(action) = &s.model_action {
                    v["event"] = json!(action);
                    v["args"] = json!(s.model_arguments);
                }
                serde_json::to_string(&v).expect("serialize JSON value")
            })
            .collect();
        Ok(format!(
            "{}
",
            lines.join(
                "
"
            )
        ))
    }

    pub fn to_provenance_sidecar_json(&self) -> Result<Value, ProjectionError> {
        self.validate()?;
        let snapshots:Vec<Value>=self.snapshots.iter().map(|s|{
            let state_bytes=serde_json::to_vec(&s.state_s).expect("serialize state");
            json!({
                "event_id":s.event_id,"logical_sequence":s.logical_sequence,
                "model_action":s.model_action,"model_arguments":s.model_arguments,
                "predecessor_event_ids":s.predecessor_event_ids,
                "source_clock":s.source_clock.as_ref().map(|c|json!({
                    "clock_id":c.clock_id,"boot_epoch_id":c.boot_epoch_id,"unit":c.unit,
                    "value":c.value,"resolution":c.resolution,"uncertainty":c.uncertainty
                })),
                "state_sha256":hex(&Sha256::digest(state_bytes)),
                "owner_evidence":s.owner_evidence.iter().map(|e|json!({
                    "domain":e.domain.as_str(),"owner_id":e.owner_id,"owner_revision":e.owner_revision,
                    "evidence_id":e.evidence_id,"status":e.status.as_str(),
                    "asserted_value":e.asserted_value,
                    "scope": {
                        "logical_effect_id":e.scope.logical_effect_id,
                        "attempt_id":e.scope.attempt_id,
                        "subject_boot_epoch":e.scope.subject_boot_epoch,
                        "subject_controller_epoch":e.scope.subject_controller_epoch
                    },
                    "claim_id":e.claim_id,"claim_scope_sha256":e.claim_scope_sha256,
                    "observer_quorum":e.observer_quorum.as_ref().map(|proof| json!({
                        "fault_model_revision":proof.fault_model_revision,
                        "required_independent_members":proof.required_independent_members,
                        "members":proof.members.iter().map(|member| json!({
                            "observer_id":member.observer_id,
                            "observer_revision":member.observer_revision,
                            "observation_id":member.observation_id,
                            "common_cause_domains":member.common_cause_domains,
                            "raw_artifact_id":member.raw_artifact_id,
                            "raw_artifact_sha256":member.raw_artifact_sha256
                        })).collect::<Vec<_>>()
                    })),
                    "raw_artifact_id":e.raw_artifact_id,"raw_artifact_sha256":e.raw_artifact_sha256
                })).collect::<Vec<_>>()
            })
        }).collect();
        Ok(json!({
            "version":self.version,"bundle_id":self.header.bundle_id,"system_id":self.header.system_id,
            "implementation_revision":self.header.implementation_revision,"model_revision":self.header.model_revision,
            "model_source_sha256":self.header.model_source_sha256,"model_config_sha256":self.header.model_config_sha256,
            "tla_tool_sha256":self.header.tla_tool_sha256,"fault_profile_revision":self.header.fault_profile_revision,
            "max_epoch":self.header.max_epoch,"naive_retry_enabled":self.header.naive_retry_enabled,
            "snapshots":snapshots
        }))
    }
}

fn hex(bytes: &[u8]) -> String {
    const D: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(D[(b >> 4) as usize] as char);
        out.push(D[(b & 15) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha(c: char) -> String {
        std::iter::repeat_n(c, 64).collect()
    }

    fn observer_proof(label: &str) -> ObserverQuorumProof {
        ObserverQuorumProof {
            fault_model_revision: "rrvr-common-cause-model-v0.1".to_owned(),
            required_independent_members: 2,
            members: vec![
                ObserverMemberEvidence {
                    observer_id: format!("{label}-observer-a"),
                    observer_revision: "sensor-stack-a@1".to_owned(),
                    observation_id: format!("{label}-observation-a"),
                    common_cause_domains: vec![
                        format!("{label}-power-a"),
                        format!("{label}-clock-a"),
                        format!("{label}-host-a"),
                    ],
                    raw_artifact_id: format!("{label}-raw-a"),
                    raw_artifact_sha256: sha256_hex(format!("{label}-raw-a").as_bytes()),
                },
                ObserverMemberEvidence {
                    observer_id: format!("{label}-observer-b"),
                    observer_revision: "sensor-stack-b@1".to_owned(),
                    observation_id: format!("{label}-observation-b"),
                    common_cause_domains: vec![
                        format!("{label}-power-b"),
                        format!("{label}-clock-b"),
                        format!("{label}-host-b"),
                    ],
                    raw_artifact_id: format!("{label}-raw-b"),
                    raw_artifact_sha256: sha256_hex(format!("{label}-raw-b").as_bytes()),
                },
            ],
        }
    }

    fn owners() -> Vec<OwnerEvidenceRef> {
        let domains = [
            OwnerEvidenceDomain::ProcessEpoch,
            OwnerEvidenceDomain::Persistence,
            OwnerEvidenceDomain::PhysicalOutcome,
            OwnerEvidenceDomain::MissionResolution,
            OwnerEvidenceDomain::ControllerQueue,
            OwnerEvidenceDomain::DownstreamExecutionGraph,
            OwnerEvidenceDomain::PhysicalOutcomeObserverQuorum,
            OwnerEvidenceDomain::DownstreamObserverQuorum,
            OwnerEvidenceDomain::ActionApplicability,
            OwnerEvidenceDomain::RecoveryAuthority,
        ];
        domains
            .into_iter()
            .enumerate()
            .map(|(index, domain)| {
                let scope = if matches!(
                    domain,
                    OwnerEvidenceDomain::RecoveryAuthority
                        | OwnerEvidenceDomain::ActionApplicability
                ) {
                    EvidenceScope {
                        logical_effect_id: "effect-1".into(),
                        attempt_id: "attempt-2".into(),
                        subject_boot_epoch: 1,
                        subject_controller_epoch: 1,
                    }
                } else {
                    EvidenceScope {
                        logical_effect_id: "effect-1".into(),
                        attempt_id: "attempt-1".into(),
                        subject_boot_epoch: 1,
                        subject_controller_epoch: 1,
                    }
                };
                let asserted_value = match domain {
                    OwnerEvidenceDomain::ProcessEpoch => "running",
                    OwnerEvidenceDomain::Persistence => "definitely_not_committed",
                    OwnerEvidenceDomain::PhysicalOutcome => "none",
                    OwnerEvidenceDomain::MissionResolution => "none",
                    OwnerEvidenceDomain::ControllerQueue => "not_fenced",
                    OwnerEvidenceDomain::DownstreamExecutionGraph => "not_fenced",
                    OwnerEvidenceDomain::PhysicalOutcomeObserverQuorum => "none",
                    OwnerEvidenceDomain::DownstreamObserverQuorum => "not_applicable",
                    OwnerEvidenceDomain::ActionApplicability => "current",
                    OwnerEvidenceDomain::RecoveryAuthority => "absent",
                };
                OwnerEvidenceRef {
                    domain,
                    owner_id: format!("owner-{index}"),
                    owner_revision: format!("revision-{index}"),
                    evidence_id: format!("evidence-{index}"),
                    asserted_value: asserted_value.to_owned(),
                    status: if matches!(
                        domain,
                        OwnerEvidenceDomain::PhysicalOutcomeObserverQuorum
                            | OwnerEvidenceDomain::DownstreamObserverQuorum
                            | OwnerEvidenceDomain::PhysicalOutcome
                            | OwnerEvidenceDomain::MissionResolution
                            | OwnerEvidenceDomain::Persistence
                            | OwnerEvidenceDomain::ControllerQueue
                            | OwnerEvidenceDomain::DownstreamExecutionGraph
                            | OwnerEvidenceDomain::RecoveryAuthority
                    ) {
                        OwnerEvidenceStatus::NotApplicable
                    } else {
                        OwnerEvidenceStatus::Current
                    },
                    claim_id: None,
                    claim_scope_sha256: None,
                    observer_quorum: None,
                    scope,
                    raw_artifact_id: format!("artifact-{index}"),
                    raw_artifact_sha256: sha(char::from(b'a' + index as u8)),
                }
            })
            .collect()
    }

    fn set_quorum(evidence: &mut OwnerEvidenceRef, claim_id: &str, label: &str) {
        evidence.asserted_value = match evidence.domain {
            OwnerEvidenceDomain::PhysicalOutcomeObserverQuorum => "no_effect".to_owned(),
            OwnerEvidenceDomain::DownstreamObserverQuorum => "quiescent".to_owned(),
            _ => evidence.asserted_value.clone(),
        };
        evidence.status = OwnerEvidenceStatus::QuorumQualified;
        evidence.claim_id = Some(claim_id.to_owned());
        evidence.claim_scope_sha256 = Some(claim_scope_digest(
            evidence.domain,
            claim_id,
            &evidence.asserted_value,
            &evidence.scope,
        ));
        evidence.observer_quorum = Some(observer_proof(label));
    }

    fn recording() -> RuntimeProjectionRecording {
        RuntimeProjectionRecording {
            version: CONTRACT_VERSION.to_owned(),
            header: ProjectionHeader {
                bundle_id: "runtime-proj-001".into(),
                system_id: "synthetic-robot".into(),
                implementation_revision: "symtropy-test".into(),
                model_revision: "RestartSafeEffectContinuity@pinned".into(),
                model_source_sha256: sha('a'),
                model_config_sha256: sha('b'),
                tla_tool_sha256: sha('c'),
                fault_profile_revision: "fault-profile-v0.1".into(),
                max_epoch: 4,
                naive_retry_enabled: false,
            },
            snapshots: vec![RuntimeSnapshot {
                event_id: "init".into(),
                logical_sequence: 1,
                model_action: None,
                model_arguments: vec![],
                predecessor_event_ids: vec![],
                source_clock: None,
                state_s: model_init_state(),
                owner_evidence: owners(),
            }],
        }
    }

    #[test]
    fn deserializes_and_projects_the_four_state_known_answer_fixture() {
        let fixture =
            include_str!("../fixtures/runtime_projection_reservation_acceptance_v0_1.json");
        let recording: RuntimeProjectionRecording =
            serde_json::from_str(fixture).expect("strict versioned fixture deserializes");
        assert_eq!(recording.validate(), Ok(()));
        let trace = recording.to_tla_trace_ndjson().expect("valid Reflow trace");
        let lines: Vec<Value> = trace
            .lines()
            .map(|line| serde_json::from_str(line).expect("each JSONL line parses"))
            .collect();
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0]["state"]["s"]["stage"], "unreserved");
        assert_eq!(lines[1]["event"], "ReserveAttempt");
        assert_eq!(lines[1]["state"]["s"]["stage"], "reserved");
        assert_eq!(lines[2]["event"], "MarkDispatchStarted");
        assert_eq!(lines[2]["state"]["s"]["stage"], "dispatch_started");
        assert_eq!(lines[3]["event"], "NativeAcceptOld");
        assert_eq!(lines[3]["state"]["s"]["commandOwnerAttempt"], "attempt-1");
        assert_eq!(
            recording.snapshots[3].owner_evidence.len(),
            REQUIRED_OWNER_DOMAINS
        );
    }

    #[test]
    fn deserializes_and_validates_the_full_no_effect_recovery_lifecycle() {
        let fixture = include_str!("../fixtures/runtime_projection_no_effect_recovery_v0_1.json");
        let recording: RuntimeProjectionRecording =
            serde_json::from_str(fixture).expect("recovery fixture deserializes");
        assert_eq!(recording.snapshots.len(), 17);
        assert_eq!(recording.validate(), Ok(()));
        let final_snapshot = recording.snapshots.last().expect("final recovery state");
        assert_eq!(
            final_snapshot.model_action.as_deref(),
            Some("NativeAcceptRecovery")
        );
        assert_eq!(final_snapshot.state_s["recoveryPermit"], "consumed");
        assert_eq!(final_snapshot.state_s["permitUseCount"], 1);
        assert_eq!(final_snapshot.state_s["downstreamFenceBootEpoch"], 0);
        assert_eq!(final_snapshot.state_s["admittedDownstreamFence"], true);
        assert_eq!(final_snapshot.state_s["commandOwnerAttempt"], "attempt-2");
        assert_eq!(
            final_snapshot
                .owner_evidence
                .iter()
                .find(|item| item.domain == OwnerEvidenceDomain::PhysicalOutcomeObserverQuorum)
                .expect("physical quorum")
                .status,
            OwnerEvidenceStatus::QuorumQualified
        );
        assert_eq!(
            final_snapshot
                .owner_evidence
                .iter()
                .find(|item| item.domain == OwnerEvidenceDomain::DownstreamObserverQuorum)
                .expect("downstream quorum")
                .status,
            OwnerEvidenceStatus::QuorumQualified
        );
    }

    #[test]
    fn state_field_catalogue_is_the_exact_pinned_tla_init_record() {
        const EXPECTED: [&str; 43] = [
            "processStatus",
            "bootEpoch",
            "controllerEpoch",
            "stage",
            "reservationCommitted",
            "oldTicketBootEpoch",
            "oldTicketControllerEpoch",
            "oldAccepted",
            "oldAcceptedBootEpoch",
            "oldAcceptedControllerEpoch",
            "commandPending",
            "commandOwnerAttempt",
            "oldPhysicalOutcome",
            "recoveryPhysicalOutcome",
            "appliedEffectCount",
            "persistStatus",
            "storeRecord",
            "storeCallPending",
            "effectEvidence",
            "missionResolution",
            "observerIndependent",
            "queueFenceBootEpoch",
            "downstreamFenceBootEpoch",
            "recoveryPermit",
            "permitUseCount",
            "ticketStatus",
            "ticketBootEpoch",
            "ticketControllerEpoch",
            "ticketAttempt",
            "m1Current",
            "m2Current",
            "ticketBindingValid",
            "liveBindingValid",
            "recoveryAccepted",
            "admittedFreshBoot",
            "admittedFreshController",
            "admittedControllerFence",
            "admittedDownstreamFence",
            "admittedM1",
            "admittedM2",
            "admittedBinding",
            "naiveReplayCount",
            "crashCut",
        ];
        assert_eq!(STATE_FIELDS, EXPECTED);
        assert_eq!(STATE_FIELDS.len(), MODEL_STATE_FIELD_COUNT);
    }

    #[test]
    fn applied_effect_count_two_is_rejected_by_the_safety_invariant() {
        let mut state = model_init_state();
        state["appliedEffectCount"] = json!(2);
        assert_eq!(
            validate_state(&state, 4),
            Err(ProjectionError::DuplicatePhysicalEffectObserved)
        );
    }

    #[test]
    fn initial_observer_independence_is_not_mistaken_for_an_observed_quorum() {
        let record = recording();
        assert_eq!(record.validate(), Ok(()));
        assert!(bool_field(&record.snapshots[0].state_s, "observerIndependent").unwrap());
        assert_eq!(
            record.snapshots[0].owner_evidence[6].status,
            OwnerEvidenceStatus::NotApplicable
        );
        assert_eq!(
            record.snapshots[0].owner_evidence[7].status,
            OwnerEvidenceStatus::NotApplicable
        );
    }

    #[test]
    fn observer_quorum_is_qualified_only_under_declared_pairwise_disjoint_fault_domains() {
        let proof = observer_proof("isolated");
        assert_eq!(observer_quorum_is_qualified(&proof), Ok(true));

        let mut common_cause = observer_proof("shared");
        common_cause.members[1].common_cause_domains[0] =
            common_cause.members[0].common_cause_domains[0].clone();
        assert_eq!(observer_quorum_is_qualified(&common_cause), Ok(false));

        let mut insufficient = observer_proof("insufficient");
        insufficient.members.pop();
        assert_eq!(observer_quorum_is_qualified(&insufficient), Ok(false));
    }

    #[test]
    fn separate_observer_claims_cannot_share_members_artifacts_or_common_causes() {
        let no_effect = observer_proof("effect-observers");
        let mut graph = observer_proof("graph-observers");
        assert!(observer_proofs_are_independent(&no_effect, &graph));

        graph.members[1].common_cause_domains[0] =
            no_effect.members[0].common_cause_domains[0].clone();
        assert!(!observer_proofs_are_independent(&no_effect, &graph));

        graph = observer_proof("graph-observers");
        graph.members[0].observer_id = no_effect.members[0].observer_id.clone();
        assert!(!observer_proofs_are_independent(&no_effect, &graph));

        graph = observer_proof("graph-observers");
        graph.fault_model_revision = "different-fault-model".to_owned();
        assert!(!observer_proofs_are_independent(&no_effect, &graph));

        graph = observer_proof("graph-observers");
        graph.members[0].raw_artifact_sha256 = no_effect.members[0].raw_artifact_sha256.clone();
        assert!(!observer_proofs_are_independent(&no_effect, &graph));
    }

    #[test]
    fn observer_proof_rejects_reused_observation_artifacts_and_content_digests() {
        let mut proof = observer_proof("duplicate-observation");
        proof.members[1].observation_id = proof.members[0].observation_id.clone();
        assert_eq!(
            observer_quorum_is_qualified(&proof),
            Err(ProjectionError::ObserverQuorumMalformed)
        );

        let mut proof = observer_proof("duplicate-artifact");
        proof.members[1].raw_artifact_id = proof.members[0].raw_artifact_id.clone();
        assert_eq!(
            observer_quorum_is_qualified(&proof),
            Err(ProjectionError::ObserverQuorumMalformed)
        );

        let mut proof = observer_proof("duplicate-digest");
        proof.members[1].raw_artifact_sha256 = proof.members[0].raw_artifact_sha256.clone();
        assert_eq!(
            observer_quorum_is_qualified(&proof),
            Err(ProjectionError::ObserverQuorumMalformed)
        );
    }

    #[test]
    fn physical_outcome_and_downstream_quiescence_claims_have_distinct_scope_commitments() {
        let record = recording();
        let scope = record.snapshots[0].owner_evidence[6].scope.clone();
        let no_effect = claim_scope_digest(
            OwnerEvidenceDomain::PhysicalOutcomeObserverQuorum,
            "effect-1-no-effect",
            "no_effect",
            &scope,
        );
        let downstream = claim_scope_digest(
            OwnerEvidenceDomain::DownstreamObserverQuorum,
            "effect-1-downstream-quiescence",
            "quiescent",
            &scope,
        );
        assert_ne!(no_effect, downstream);
        assert_ne!(
            claim_scope_digest(
                OwnerEvidenceDomain::PhysicalOutcomeObserverQuorum,
                "effect-1-no-effect",
                "no_effect",
                &scope
            ),
            claim_scope_digest(
                OwnerEvidenceDomain::PhysicalOutcomeObserverQuorum,
                "effect-2-no-effect",
                "no_effect",
                &scope
            )
        );
    }

    #[test]
    fn owner_evidence_cannot_be_replayed_across_physical_attempts() {
        let mut record = recording();
        record.snapshots[0].owner_evidence[2].scope.attempt_id = "attempt-2".to_owned();
        assert_eq!(
            record.validate(),
            Err(ProjectionError::EvidenceScopeMismatch)
        );
    }

    #[test]
    fn qualified_claim_rejects_a_tampered_scope_commitment() {
        let mut record = recording();
        let evidence = &mut record.snapshots[0].owner_evidence[6];
        let claim = "old-attempt-physical-outcome";
        set_quorum(evidence, claim, "no-effect");
        evidence.claim_scope_sha256 = Some(sha('f'));
        assert_eq!(
            record.validate(),
            Err(ProjectionError::InvalidOwnerEvidence)
        );
    }

    #[test]
    fn recovery_acceptance_uses_captured_downstream_fence_not_current_fence_epoch() {
        let mut state = model_init_state();
        state["processStatus"] = json!("running");
        state["bootEpoch"] = json!(2);
        state["controllerEpoch"] = json!(2);
        state["stage"] = json!("accepted");
        state["reservationCommitted"] = json!(true);
        state["oldAccepted"] = json!(true);
        state["oldAcceptedBootEpoch"] = json!(1);
        state["oldAcceptedControllerEpoch"] = json!(1);
        state["commandPending"] = json!(true);
        state["commandOwnerAttempt"] = json!("attempt-2");
        state["oldPhysicalOutcome"] = json!("no_effect");
        state["effectEvidence"] = json!("no_effect");
        state["missionResolution"] = json!("no_effect");
        state["queueFenceBootEpoch"] = json!(2);
        // NativeAcceptRecovery deliberately clears this field after admission.
        state["downstreamFenceBootEpoch"] = json!(0);
        state["recoveryPermit"] = json!("consumed");
        state["permitUseCount"] = json!(1);
        state["ticketStatus"] = json!("consumed");
        state["ticketBootEpoch"] = json!(2);
        state["ticketControllerEpoch"] = json!(2);
        state["ticketAttempt"] = json!("attempt-2");
        state["recoveryAccepted"] = json!(true);
        state["admittedFreshBoot"] = json!(true);
        state["admittedFreshController"] = json!(true);
        state["admittedControllerFence"] = json!(true);
        state["admittedDownstreamFence"] = json!(true);
        state["admittedM1"] = json!(true);
        state["admittedM2"] = json!(true);
        state["admittedBinding"] = json!(true);
        state["crashCut"] = json!("permit_consumed_before_effect");
        assert_eq!(validate_state(&state, 4), Ok(()));
    }

    #[test]
    fn emits_exact_model_init_as_reflow_trace_line() {
        assert_eq!(STATE_FIELDS.len(), MODEL_STATE_FIELD_COUNT);
        let r = recording();
        let line = r.to_tla_trace_ndjson().unwrap();
        let v: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(v["state"]["s"]["processStatus"], "running");
        assert_eq!(v["state"]["s"]["stage"], "unreserved");
        assert!(v.get("event").is_none());
    }
    #[test]
    fn missing_owner_domain_is_not_filled() {
        let mut r = recording();
        r.snapshots[0].owner_evidence.pop();
        assert_eq!(r.validate(), Err(ProjectionError::MissingOwnerEvidence));
    }
    #[test]
    fn duplicate_domain_or_invalid_artifact_digest_holds() {
        let mut r = recording();
        r.snapshots[0].owner_evidence[1].domain = OwnerEvidenceDomain::ProcessEpoch;
        assert_eq!(r.validate(), Err(ProjectionError::MissingOwnerEvidence));
        let mut r = recording();
        r.snapshots[0].owner_evidence[0].raw_artifact_sha256 = "bad".into();
        assert_eq!(r.validate(), Err(ProjectionError::InvalidOwnerEvidence));
    }
    #[test]
    fn indeterminate_persistence_cannot_carry_permit() {
        let mut state = model_init_state();
        state["persistStatus"] = json!("indeterminate_commit");
        state["effectEvidence"] = json!("no_effect");
        state["oldPhysicalOutcome"] = json!("no_effect");
        state["missionResolution"] = json!("no_effect");
        state["recoveryPermit"] = json!("issued");
        assert_eq!(
            validate_state(&state, 4),
            Err(ProjectionError::IndeterminatePersistenceRecovery)
        );
    }
    #[test]
    fn applied_unknown_partial_or_disputed_cannot_carry_permit() {
        for (outcome, physical) in [
            ("applied", "applied"),
            ("unknown", "no_effect"),
            ("partial", "partial"),
            ("disputed", "partial"),
        ] {
            let mut state = model_init_state();
            state["effectEvidence"] = json!(outcome);
            state["missionResolution"] = json!(outcome);
            state["oldPhysicalOutcome"] = json!(physical);
            state["recoveryPermit"] = json!("issued");
            assert_eq!(
                validate_state(&state, 4),
                Err(ProjectionError::RecoveryFromUnqualifiedOutcome),
                "outcome={outcome}"
            );
        }
    }
    #[test]
    fn command_pending_owner_pair_must_match() {
        let mut r = recording();
        r.snapshots[0].state_s["commandPending"] = json!(true);
        assert_eq!(r.validate(), Err(ProjectionError::CommandOwnerMismatch));
        let mut r = recording();
        r.snapshots[0].state_s["commandOwnerAttempt"] = json!("attempt-1");
        assert_eq!(r.validate(), Err(ProjectionError::CommandOwnerMismatch));
    }
    #[test]
    fn downstream_fence_closes_pending_command_and_requires_controller_fence() {
        let mut r = recording();
        r.snapshots[0].state_s["downstreamFenceBootEpoch"] = json!(1);
        assert_eq!(r.validate(), Err(ProjectionError::DownstreamFenceInvariant));
    }
    #[test]
    fn recovery_acceptance_uses_captured_admission_facts_not_current_revocation_flags() {
        let mut state = model_init_state();
        state["processStatus"] = json!("running");
        state["bootEpoch"] = json!(2);
        state["controllerEpoch"] = json!(2);
        state["stage"] = json!("accepted");
        state["reservationCommitted"] = json!(true);
        state["oldAccepted"] = json!(true);
        state["oldAcceptedBootEpoch"] = json!(1);
        state["oldAcceptedControllerEpoch"] = json!(1);
        state["commandPending"] = json!(true);
        state["commandOwnerAttempt"] = json!("attempt-2");
        state["oldPhysicalOutcome"] = json!("no_effect");
        state["effectEvidence"] = json!("no_effect");
        state["missionResolution"] = json!("no_effect");
        state["queueFenceBootEpoch"] = json!(2);
        state["downstreamFenceBootEpoch"] = json!(0);
        state["recoveryAccepted"] = json!(true);
        state["recoveryPermit"] = json!("consumed");
        state["permitUseCount"] = json!(1);
        state["ticketStatus"] = json!("consumed");
        state["ticketAttempt"] = json!("attempt-2");
        state["ticketBootEpoch"] = json!(2);
        state["ticketControllerEpoch"] = json!(2);
        state["admittedFreshBoot"] = json!(true);
        state["admittedFreshController"] = json!(true);
        state["admittedControllerFence"] = json!(true);
        state["admittedDownstreamFence"] = json!(true);
        state["admittedM1"] = json!(true);
        state["admittedM2"] = json!(true);
        state["admittedBinding"] = json!(true);
        state["crashCut"] = json!("permit_consumed_before_effect");
        assert_eq!(validate_state(&state, 4), Ok(()));

        // Revocation after admission is not retroactive. A corrupted admission
        // receipt, however, must fail closed.
        state["m2Current"] = json!(false);
        assert_eq!(validate_state(&state, 4), Ok(()));
        state["admittedM2"] = json!(false);
        assert_eq!(
            validate_state(&state, 4),
            Err(ProjectionError::RecoveryAdmissionInvariant)
        );
    }
    #[test]
    fn initial_state_is_not_an_optimistic_placeholder() {
        let mut r = recording();
        r.snapshots[0].state_s["observerIndependent"] = json!(false);
        assert_eq!(r.validate(), Err(ProjectionError::InitialStateMismatch));
    }
    #[test]
    fn later_rows_require_exact_sequence_predecessor_and_known_model_action() {
        let mut r = recording();
        let mut n = r.snapshots[0].clone();
        n.event_id = "reserve".into();
        n.logical_sequence = 2;
        n.model_action = Some("ReserveAttempt".into());
        n.predecessor_event_ids = vec!["init".into()];
        n.state_s["stage"] = json!("reserved");
        n.state_s["reservationCommitted"] = json!(true);
        r.snapshots.push(n);
        assert_eq!(r.validate(), Ok(()));
        r.snapshots[1].model_action = Some("Guess".into());
        assert_eq!(r.validate(), Err(ProjectionError::InvalidModelAction));
    }
    #[test]
    fn incomparable_clock_values_are_retained_without_ordering() {
        let mut r = recording();
        r.snapshots[0].source_clock = Some(SourceClock {
            clock_id: "c1".into(),
            boot_epoch_id: "b1".into(),
            unit: "ticks".into(),
            value: 100,
            resolution: 1,
            uncertainty: 3,
        });
        assert_eq!(r.validate(), Ok(()));
        r.snapshots[0].source_clock.as_mut().unwrap().resolution = 0;
        assert_eq!(r.validate(), Err(ProjectionError::InvalidSourceClock));
    }
    #[test]
    fn sidecar_contains_owner_sources_and_state_digest() {
        let r = recording();
        let sidecar = r.to_provenance_sidecar_json().unwrap();
        assert_eq!(
            sidecar["snapshots"][0]["owner_evidence"]
                .as_array()
                .unwrap()
                .len(),
            10
        );
        assert_eq!(
            sidecar["snapshots"][0]["state_sha256"]
                .as_str()
                .unwrap()
                .len(),
            64
        );
        assert_eq!(sidecar["model_source_sha256"], sha('a'));
    }
}
