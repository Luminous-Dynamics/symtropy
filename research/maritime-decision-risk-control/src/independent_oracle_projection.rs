// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Typed per-schedule / per-policy projection of independent physical and
//! authority oracle receipts.
//!
//! Physical feasibility and participant authority have distinct oracle
//! identities, implementation digests, input references, and receipt digests.
//! Participant service outcomes remain a per-participant vector.
//!
//! This module validates caller-supplied identities, references, and digests.
//! A SHA-256 field proves neither source truth nor oracle independence; the
//! independence-review reference is still a caller declaration. This module
//! does not implement physics, infer authorization, authenticate signatures,
//! grant operational authority, or compile an operational deployment policy.

use std::collections::{BTreeMap, BTreeSet};

use crate::oracle_adapter::{
    AuthorityDisposition, DatasetRole, FrozenPolicyEntry, PhysicalDisposition, PolicyActionClass,
};

const EXPECTED_SCHEMA: &str = "maritime.independent-oracle-projection.v1";
const MAX_LOSS_MILLI: u16 = 1_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleIdentity {
    pub oracle_id: String,
    pub oracle_revision: String,
    /// Digest of the exact executable/container/source build declared by caller.
    pub implementation_sha256: String,
    /// Caller-supplied review record for structural independence.
    pub independence_review_ref: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceEvidenceKind {
    ScheduleInput,
    PhysicalObservation,
    AuthorityRecord,
    ParticipantService,
    OperationalObservation,
    ModelParameter,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceArtifactEvidence {
    pub source_id: String,
    pub source_revision: String,
    pub source_sha256: String,
    pub kind: SourceEvidenceKind,
    pub retrieved_at_utc: String,
    /// None means that the source did not provide an explicit publication time.
    pub publication_time_utc: Option<String>,
    /// None means that the source did not provide an explicit valid-from time.
    pub valid_from_time_utc: Option<String>,
    /// None means that the source did not provide an explicit valid-to time.
    pub valid_to_time_utc: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParticipantFloorContract {
    pub participant_id: String,
    pub minimum_service_units: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleProjectionContract {
    pub schema_version: String,
    pub dataset_id: String,
    pub dataset_revision: String,
    pub physical_oracle: OracleIdentity,
    pub authority_oracle: OracleIdentity,
    pub evaluator_revision: String,
    pub loss_contract_revision: String,
    pub operating_regime_id: String,
    /// Digest of the canonical, ordered policy-grid encoding.
    pub policy_grid_sha256: String,
    /// Exact frozen policy order; parameter_index is the matrix column.
    pub policy_grid: Vec<FrozenPolicyEntry>,
    /// Every participant in this contract must appear in each policy receipt.
    pub participant_floors: Vec<ParticipantFloorContract>,
    /// Immutable source registry referenced by physical, authority and service receipts.
    pub source_artifacts: Vec<SourceArtifactEvidence>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleReceiptBinding {
    pub episode_id: String,
    pub policy_id: String,
    pub policy_revision: String,
    pub evaluator_revision: String,
    pub loss_contract_revision: String,
    pub operating_regime_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhysicalOracleReceipt {
    pub binding: OracleReceiptBinding,
    pub oracle_id: String,
    pub oracle_revision: String,
    pub disposition: PhysicalDisposition,
    pub source_evidence_ids: Vec<String>,
    pub receipt_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorityOracleReceipt {
    pub binding: OracleReceiptBinding,
    pub oracle_id: String,
    pub oracle_revision: String,
    pub action_class: PolicyActionClass,
    pub disposition: AuthorityDisposition,
    /// Required for Authorized / Denied; absent for NotRequired / Unknown.
    pub authority_record_sha256: Option<String>,
    pub source_evidence_ids: Vec<String>,
    pub receipt_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceLossOracleReceipt {
    pub binding: OracleReceiptBinding,
    pub loss_milli: u16,
    pub source_evidence_ids: Vec<String>,
    pub receipt_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParticipantServiceReceipt {
    pub participant_id: String,
    pub delivered_service_units: u32,
    pub source_evidence_ids: Vec<String>,
    pub receipt_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyOracleProjection {
    pub policy_id: String,
    pub policy_revision: String,
    pub physical: PhysicalOracleReceipt,
    pub authority: AuthorityOracleReceipt,
    pub service_loss: ServiceLossOracleReceipt,
    pub participant_service: Vec<ParticipantServiceReceipt>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleOracleProjection {
    /// One complete schedule / episode is one statistical unit.
    pub episode_id: String,
    /// Shared weather, outage, TOS incident or other dependence cluster.
    pub dependence_cluster_id: String,
    pub scenario_family: String,
    pub operating_regime_id: String,
    pub role: DatasetRole,
    /// Exactly one projection for every frozen policy.
    pub policy_outcomes: Vec<PolicyOracleProjection>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndependentOracleProjection {
    pub contract: OracleProjectionContract,
    pub schedules: Vec<ScheduleOracleProjection>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleProjectionError {
    MissingField(&'static str),
    UnsupportedSchema(String),
    InvalidSha256 { field: &'static str, value: String },
    OracleIdentitiesNotDistinct,
    DuplicatePolicyId(String),
    InvalidPolicyGridOrder { position: usize, found: usize },
    DuplicateParticipantId(String),
    InvalidParticipantFloor(String),
    DuplicateSourceId(String),
    EmptyDataset,
    MissingCalibrationSchedules,
    MissingEvaluationSchedules,
    DuplicateEpisodeId(String),
    ReusedDependenceCluster(String),
    MixedOperatingRegimes { expected: String, found: String },
    DuplicatePolicyReceipt { episode_id: String, policy_id: String },
    PolicyGridDrift {
        episode_id: String,
        missing_policy_ids: Vec<String>,
        unexpected_policy_ids: Vec<String>,
    },
    PolicyIdentityMismatch { episode_id: String, policy_id: String },
    ReceiptBindingMismatch { episode_id: String, policy_id: String, axis: &'static str },
    OracleIdentityMismatch { oracle_id: String, axis: &'static str },
    AuthorityActionMismatch {
        episode_id: String,
        policy_id: String,
        action_class: PolicyActionClass,
        disposition: AuthorityDisposition,
    },
    AuthorityRecordDigestMismatch { episode_id: String, policy_id: String },
    MissingEvidenceReference { episode_id: String, policy_id: String, axis: &'static str },
    DuplicateEvidenceReference { source_id: String },
    UnknownEvidenceReference { source_id: String },
    AuthorityEvidenceKindMissing { episode_id: String, policy_id: String },
    ServiceEvidenceKindMissing { episode_id: String, policy_id: String },
    InvalidServiceLoss { episode_id: String, policy_id: String, value_milli: u16 },
    DuplicateParticipantReceipt { episode_id: String, policy_id: String, participant_id: String },
    MissingParticipantReceipts { episode_id: String, policy_id: String, participant_ids: Vec<String> },
    UnexpectedParticipantReceipt { episode_id: String, policy_id: String, participant_id: String },
    InvalidSourceTime { source_id: String, field: &'static str },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleProjectionAudit {
    pub dataset_id: String,
    pub dataset_revision: String,
    pub physical_oracle_id: String,
    pub physical_oracle_revision: String,
    pub authority_oracle_id: String,
    pub authority_oracle_revision: String,
    pub operating_regime_id: String,
    pub schedule_count: usize,
    pub calibration_schedule_count: usize,
    pub evaluation_schedule_count: usize,
    pub policy_receipt_count: usize,
    pub source_artifact_count: usize,
    pub physical_failure_count: usize,
    pub physical_unknown_count: usize,
    pub authority_denied_count: usize,
    pub authority_unknown_count: usize,
    pub participant_floor_violation_count: usize,
    pub minimum_observed_participant_service_units: Option<u32>,
    pub maximum_service_loss_milli: Option<u16>,
}

impl OracleProjectionAudit {
    /// True only if every physical and authority disposition is resolved.
    /// A known physical failure or denied action is resolved evidence, not a pass.
    pub fn all_dispositions_resolved(&self) -> bool {
        self.physical_unknown_count == 0 && self.authority_unknown_count == 0
    }

    pub fn all_participant_floors_met(&self) -> bool {
        self.participant_floor_violation_count == 0
    }

    /// True only when no candidate in this projection has a hard blocker.
    pub fn all_candidates_admissible(&self) -> bool {
        self.physical_failure_count == 0
            && self.physical_unknown_count == 0
            && self.authority_denied_count == 0
            && self.authority_unknown_count == 0
            && self.participant_floor_violation_count == 0
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn require_nonempty(value: &str, field: &'static str) -> Result<(), OracleProjectionError> {
    if value.trim().is_empty() {
        return Err(OracleProjectionError::MissingField(field));
    }
    Ok(())
}

fn validate_digest(value: &str, field: &'static str) -> Result<(), OracleProjectionError> {
    if !is_sha256(value) {
        return Err(OracleProjectionError::InvalidSha256 {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_oracle_identity(
    identity: &OracleIdentity,
    axis: &'static str,
) -> Result<(), OracleProjectionError> {
    for (value, field) in [
        (identity.oracle_id.as_str(), "oracle_id"),
        (identity.oracle_revision.as_str(), "oracle_revision"),
        (identity.independence_review_ref.as_str(), "independence_review_ref"),
    ] {
        require_nonempty(value, field)?;
    }
    let field = if axis == "physical" {
        "physical_oracle_implementation_sha256"
    } else {
        "authority_oracle_implementation_sha256"
    };
    validate_digest(&identity.implementation_sha256, field)
}

fn validate_contract(contract: &OracleProjectionContract) -> Result<(), OracleProjectionError> {
    if contract.schema_version != EXPECTED_SCHEMA {
        return Err(OracleProjectionError::UnsupportedSchema(contract.schema_version.clone()));
    }
    for (value, field) in [
        (contract.dataset_id.as_str(), "dataset_id"),
        (contract.dataset_revision.as_str(), "dataset_revision"),
        (contract.evaluator_revision.as_str(), "evaluator_revision"),
        (contract.loss_contract_revision.as_str(), "loss_contract_revision"),
        (contract.operating_regime_id.as_str(), "operating_regime_id"),
    ] {
        require_nonempty(value, field)?;
    }
    validate_oracle_identity(&contract.physical_oracle, "physical")?;
    validate_oracle_identity(&contract.authority_oracle, "authority")?;
    if contract.physical_oracle.oracle_id == contract.authority_oracle.oracle_id
        || contract.physical_oracle.implementation_sha256
            == contract.authority_oracle.implementation_sha256
    {
        return Err(OracleProjectionError::OracleIdentitiesNotDistinct);
    }
    validate_digest(&contract.policy_grid_sha256, "policy_grid_sha256")?;

    if contract.policy_grid.is_empty() {
        return Err(OracleProjectionError::MissingField("policy_grid"));
    }
    let mut policy_ids = BTreeSet::new();
    for (position, policy) in contract.policy_grid.iter().enumerate() {
        if policy.parameter_index != position {
            return Err(OracleProjectionError::InvalidPolicyGridOrder {
                position,
                found: policy.parameter_index,
            });
        }
        require_nonempty(&policy.policy_id, "policy_id")?;
        require_nonempty(&policy.policy_revision, "policy_revision")?;
        if !policy_ids.insert(policy.policy_id.clone()) {
            return Err(OracleProjectionError::DuplicatePolicyId(policy.policy_id.clone()));
        }
    }

    if contract.participant_floors.is_empty() {
        return Err(OracleProjectionError::MissingField("participant_floors"));
    }
    let mut participant_ids = BTreeSet::new();
    for participant in &contract.participant_floors {
        require_nonempty(&participant.participant_id, "participant_id")?;
        if participant.minimum_service_units == 0 {
            return Err(OracleProjectionError::InvalidParticipantFloor(
                participant.participant_id.clone(),
            ));
        }
        if !participant_ids.insert(participant.participant_id.clone()) {
            return Err(OracleProjectionError::DuplicateParticipantId(
                participant.participant_id.clone(),
            ));
        }
    }

    if contract.source_artifacts.is_empty() {
        return Err(OracleProjectionError::MissingField("source_artifacts"));
    }
    let mut source_ids = BTreeSet::new();
    for source in &contract.source_artifacts {
        require_nonempty(&source.source_id, "source_id")?;
        require_nonempty(&source.source_revision, "source_revision")?;
        require_nonempty(&source.retrieved_at_utc, "retrieved_at_utc")?;
        validate_digest(&source.source_sha256, "source_sha256")?;
        for (value, field) in [
            (source.publication_time_utc.as_deref(), "publication_time_utc"),
            (source.valid_from_time_utc.as_deref(), "valid_from_time_utc"),
            (source.valid_to_time_utc.as_deref(), "valid_to_time_utc"),
        ] {
            if value.is_some_and(|time| time.trim().is_empty()) {
                return Err(OracleProjectionError::InvalidSourceTime {
                    source_id: source.source_id.clone(),
                    field,
                });
            }
        }
        if !source_ids.insert(source.source_id.clone()) {
            return Err(OracleProjectionError::DuplicateSourceId(source.source_id.clone()));
        }
    }

    Ok(())
}

fn validate_source_refs(
    refs: &[String],
    registry: &BTreeMap<&str, &SourceArtifactEvidence>,
    episode_id: &str,
    policy_id: &str,
    axis: &'static str,
    require_at_least_one: bool,
) -> Result<Vec<SourceEvidenceKind>, OracleProjectionError> {
    if require_at_least_one && refs.is_empty() {
        return Err(OracleProjectionError::MissingEvidenceReference {
            episode_id: episode_id.to_owned(),
            policy_id: policy_id.to_owned(),
            axis,
        });
    }
    let mut seen = BTreeSet::new();
    let mut kinds = Vec::new();
    for source_id in refs {
        if !seen.insert(source_id.as_str()) {
            return Err(OracleProjectionError::DuplicateEvidenceReference {
                source_id: source_id.clone(),
            });
        }
        let source = registry
            .get(source_id.as_str())
            .ok_or_else(|| OracleProjectionError::UnknownEvidenceReference {
                source_id: source_id.clone(),
            })?;
        kinds.push(source.kind);
    }
    Ok(kinds)
}

fn expected_binding(
    schedule: &ScheduleOracleProjection,
    policy: &FrozenPolicyEntry,
    contract: &OracleProjectionContract,
) -> OracleReceiptBinding {
    OracleReceiptBinding {
        episode_id: schedule.episode_id.clone(),
        policy_id: policy.policy_id.clone(),
        policy_revision: policy.policy_revision.clone(),
        evaluator_revision: contract.evaluator_revision.clone(),
        loss_contract_revision: contract.loss_contract_revision.clone(),
        operating_regime_id: contract.operating_regime_id.clone(),
    }
}

fn validate_policy_projection(
    schedule: &ScheduleOracleProjection,
    outcome: &PolicyOracleProjection,
    policy: &FrozenPolicyEntry,
    contract: &OracleProjectionContract,
    sources: &BTreeMap<&str, &SourceArtifactEvidence>,
) -> Result<(), OracleProjectionError> {
    let episode_id = schedule.episode_id.as_str();
    let policy_id = policy.policy_id.as_str();
    if outcome.policy_id != policy.policy_id || outcome.policy_revision != policy.policy_revision {
        return Err(OracleProjectionError::PolicyIdentityMismatch {
            episode_id: episode_id.to_owned(),
            policy_id: outcome.policy_id.clone(),
        });
    }
    let binding = expected_binding(schedule, policy, contract);
    for (receipt_binding, axis) in [
        (&outcome.physical.binding, "physical"),
        (&outcome.authority.binding, "authority"),
        (&outcome.service_loss.binding, "service_loss"),
    ] {
        if receipt_binding != &binding {
            return Err(OracleProjectionError::ReceiptBindingMismatch {
                episode_id: episode_id.to_owned(),
                policy_id: policy_id.to_owned(),
                axis,
            });
        }
    }

    if outcome.physical.oracle_id != contract.physical_oracle.oracle_id
        || outcome.physical.oracle_revision != contract.physical_oracle.oracle_revision
    {
        return Err(OracleProjectionError::OracleIdentityMismatch {
            oracle_id: outcome.physical.oracle_id.clone(),
            axis: "physical",
        });
    }
    if outcome.authority.oracle_id != contract.authority_oracle.oracle_id
        || outcome.authority.oracle_revision != contract.authority_oracle.oracle_revision
    {
        return Err(OracleProjectionError::OracleIdentityMismatch {
            oracle_id: outcome.authority.oracle_id.clone(),
            axis: "authority",
        });
    }

    validate_digest(&outcome.physical.receipt_sha256, "physical_receipt_sha256")?;
    validate_digest(&outcome.authority.receipt_sha256, "authority_receipt_sha256")?;
    validate_digest(&outcome.service_loss.receipt_sha256, "service_loss_receipt_sha256")?;

    validate_source_refs(
        &outcome.physical.source_evidence_ids,
        sources,
        episode_id,
        policy_id,
        "physical",
        true,
    )?;
    let authority_kinds = validate_source_refs(
        &outcome.authority.source_evidence_ids,
        sources,
        episode_id,
        policy_id,
        "authority",
        outcome.authority.disposition != AuthorityDisposition::Unknown
            && outcome.authority.action_class == PolicyActionClass::Mutating,
    )?;
    let service_kinds = validate_source_refs(
        &outcome.service_loss.source_evidence_ids,
        sources,
        episode_id,
        policy_id,
        "service_loss",
        true,
    )?;

    match (outcome.authority.action_class, outcome.authority.disposition) {
        (PolicyActionClass::NoMutation, AuthorityDisposition::NotRequired) => {
            if outcome.authority.authority_record_sha256.is_some() {
                return Err(OracleProjectionError::AuthorityRecordDigestMismatch {
                    episode_id: episode_id.to_owned(),
                    policy_id: policy_id.to_owned(),
                });
            }
        }
        (PolicyActionClass::Mutating, AuthorityDisposition::Authorized)
        | (PolicyActionClass::Mutating, AuthorityDisposition::Denied) => {
            let digest = outcome.authority.authority_record_sha256.as_deref().ok_or_else(|| {
                OracleProjectionError::AuthorityRecordDigestMismatch {
                    episode_id: episode_id.to_owned(),
                    policy_id: policy_id.to_owned(),
                }
            })?;
            validate_digest(digest, "authority_record_sha256")?;
            if !authority_kinds.contains(&SourceEvidenceKind::AuthorityRecord) {
                return Err(OracleProjectionError::AuthorityEvidenceKindMissing {
                    episode_id: episode_id.to_owned(),
                    policy_id: policy_id.to_owned(),
                });
            }
        }
        (PolicyActionClass::Mutating, AuthorityDisposition::Unknown) => {
            if outcome.authority.authority_record_sha256.is_some() {
                return Err(OracleProjectionError::AuthorityRecordDigestMismatch {
                    episode_id: episode_id.to_owned(),
                    policy_id: policy_id.to_owned(),
                });
            }
        }
        (action_class, disposition) => {
            return Err(OracleProjectionError::AuthorityActionMismatch {
                episode_id: episode_id.to_owned(),
                policy_id: policy_id.to_owned(),
                action_class,
                disposition,
            });
        }
    }

    if outcome.service_loss.loss_milli > MAX_LOSS_MILLI {
        return Err(OracleProjectionError::InvalidServiceLoss {
            episode_id: episode_id.to_owned(),
            policy_id: policy_id.to_owned(),
            value_milli: outcome.service_loss.loss_milli,
        });
    }
    if !service_kinds.iter().any(|kind| {
        matches!(
            kind,
            SourceEvidenceKind::ParticipantService | SourceEvidenceKind::OperationalObservation
        )
    }) {
        return Err(OracleProjectionError::ServiceEvidenceKindMissing {
            episode_id: episode_id.to_owned(),
            policy_id: policy_id.to_owned(),
        });
    }

    let expected_participants = contract
        .participant_floors
        .iter()
        .map(|participant| participant.participant_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut observed_participants = BTreeSet::new();
    for participant_receipt in &outcome.participant_service {
        require_nonempty(&participant_receipt.participant_id, "participant_id")?;
        if !observed_participants.insert(participant_receipt.participant_id.as_str()) {
            return Err(OracleProjectionError::DuplicateParticipantReceipt {
                episode_id: episode_id.to_owned(),
                policy_id: policy_id.to_owned(),
                participant_id: participant_receipt.participant_id.clone(),
            });
        }
        if !expected_participants.contains(participant_receipt.participant_id.as_str()) {
            return Err(OracleProjectionError::UnexpectedParticipantReceipt {
                episode_id: episode_id.to_owned(),
                policy_id: policy_id.to_owned(),
                participant_id: participant_receipt.participant_id.clone(),
            });
        }
        validate_digest(
            &participant_receipt.receipt_sha256,
            "participant_service_receipt_sha256",
        )?;
        let kinds = validate_source_refs(
            &participant_receipt.source_evidence_ids,
            sources,
            episode_id,
            policy_id,
            "participant_service",
            true,
        )?;
        if !kinds.iter().any(|kind| {
            matches!(
                kind,
                SourceEvidenceKind::ParticipantService | SourceEvidenceKind::OperationalObservation
            )
        }) {
            return Err(OracleProjectionError::ServiceEvidenceKindMissing {
                episode_id: episode_id.to_owned(),
                policy_id: policy_id.to_owned(),
            });
        }
    }

    if expected_participants != observed_participants {
        return Err(OracleProjectionError::MissingParticipantReceipts {
            episode_id: episode_id.to_owned(),
            policy_id: policy_id.to_owned(),
            participant_ids: expected_participants
                .difference(&observed_participants)
                .map(|id| (*id).to_owned())
                .collect(),
        });
    }

    Ok(())
}

/// Validates independently typed oracle receipts and returns a projection audit.
///
/// Explicit physical Infeasible and authority Denied outcomes are retained as
/// observed failures. Unknown outcomes are retained and counted as blockers.
/// Participant-floor violations are reported per participant-policy-episode;
/// they are never averaged away across participants.
pub fn audit_independent_oracle_projection(
    dataset: &IndependentOracleProjection,
) -> Result<OracleProjectionAudit, OracleProjectionError> {
    if dataset.schedules.is_empty() {
        return Err(OracleProjectionError::EmptyDataset);
    }
    validate_contract(&dataset.contract)?;

    let contract = &dataset.contract;
    let expected_policy_ids = contract
        .policy_grid
        .iter()
        .map(|policy| policy.policy_id.as_str())
        .collect::<BTreeSet<_>>();
    let source_registry = contract
        .source_artifacts
        .iter()
        .map(|source| (source.source_id.as_str(), source))
        .collect::<BTreeMap<_, _>>();

    let mut episode_ids = BTreeSet::new();
    let mut dependence_clusters = BTreeSet::new();
    let mut calibration_count = 0_usize;
    let mut evaluation_count = 0_usize;
    let mut physical_failure_count = 0_usize;
    let mut physical_unknown_count = 0_usize;
    let mut authority_denied_count = 0_usize;
    let mut authority_unknown_count = 0_usize;
    let mut participant_floor_violation_count = 0_usize;
    let mut minimum_service = None;
    let mut maximum_loss = None;
    let mut policy_receipt_count = 0_usize;

    for schedule in &dataset.schedules {
        for (value, field) in [
            (schedule.episode_id.as_str(), "episode_id"),
            (schedule.dependence_cluster_id.as_str(), "dependence_cluster_id"),
            (schedule.scenario_family.as_str(), "scenario_family"),
            (schedule.operating_regime_id.as_str(), "operating_regime_id"),
        ] {
            require_nonempty(value, field)?;
        }
        if schedule.operating_regime_id != contract.operating_regime_id {
            return Err(OracleProjectionError::MixedOperatingRegimes {
                expected: contract.operating_regime_id.clone(),
                found: schedule.operating_regime_id.clone(),
            });
        }
        if !episode_ids.insert(schedule.episode_id.clone()) {
            return Err(OracleProjectionError::DuplicateEpisodeId(schedule.episode_id.clone()));
        }
        if !dependence_clusters.insert(schedule.dependence_cluster_id.clone()) {
            return Err(OracleProjectionError::ReusedDependenceCluster(
                schedule.dependence_cluster_id.clone(),
            ));
        }
        match schedule.role {
            DatasetRole::Calibration => calibration_count += 1,
            DatasetRole::Evaluation => evaluation_count += 1,
        }

        let mut policy_ids = BTreeSet::new();
        for outcome in &schedule.policy_outcomes {
            if !policy_ids.insert(outcome.policy_id.as_str()) {
                return Err(OracleProjectionError::DuplicatePolicyReceipt {
                    episode_id: schedule.episode_id.clone(),
                    policy_id: outcome.policy_id.clone(),
                });
            }
            let frozen_policy = contract
                .policy_grid
                .iter()
                .find(|policy| policy.policy_id == outcome.policy_id)
                .ok_or_else(|| OracleProjectionError::PolicyGridDrift {
                    episode_id: schedule.episode_id.clone(),
                    missing_policy_ids: expected_policy_ids
                        .difference(&policy_ids)
                        .map(|id| (*id).to_owned())
                        .collect(),
                    unexpected_policy_ids: vec![outcome.policy_id.clone()],
                })?;
            validate_policy_projection(
                schedule,
                outcome,
                frozen_policy,
                contract,
                &source_registry,
            )?;

            for participant in &contract.participant_floors {
                let receipt = outcome
                    .participant_service
                    .iter()
                    .find(|entry| entry.participant_id == participant.participant_id)
                    .expect("participant completeness validated above");
                if receipt.delivered_service_units < participant.minimum_service_units {
                    participant_floor_violation_count += 1;
                }
                minimum_service = Some(minimum_service.map_or(
                    receipt.delivered_service_units,
                    |current: u32| current.min(receipt.delivered_service_units),
                ));
            }
            physical_failure_count +=
                (outcome.physical.disposition == PhysicalDisposition::Infeasible) as usize;
            physical_unknown_count +=
                (outcome.physical.disposition == PhysicalDisposition::Unknown) as usize;
            authority_denied_count +=
                (outcome.authority.disposition == AuthorityDisposition::Denied) as usize;
            authority_unknown_count +=
                (outcome.authority.disposition == AuthorityDisposition::Unknown) as usize;
            maximum_loss = Some(maximum_loss.map_or(
                outcome.service_loss.loss_milli,
                |current: u16| current.max(outcome.service_loss.loss_milli),
            ));
            policy_receipt_count += 1;
        }
        if expected_policy_ids != policy_ids {
            return Err(OracleProjectionError::PolicyGridDrift {
                episode_id: schedule.episode_id.clone(),
                missing_policy_ids: expected_policy_ids
                    .difference(&policy_ids)
                    .map(|id| (*id).to_owned())
                    .collect(),
                unexpected_policy_ids: policy_ids
                    .difference(&expected_policy_ids)
                    .map(|id| (*id).to_owned())
                    .collect(),
            });
        }
    }

    if calibration_count == 0 {
        return Err(OracleProjectionError::MissingCalibrationSchedules);
    }
    if evaluation_count == 0 {
        return Err(OracleProjectionError::MissingEvaluationSchedules);
    }

    Ok(OracleProjectionAudit {
        dataset_id: contract.dataset_id.clone(),
        dataset_revision: contract.dataset_revision.clone(),
        physical_oracle_id: contract.physical_oracle.oracle_id.clone(),
        physical_oracle_revision: contract.physical_oracle.oracle_revision.clone(),
        authority_oracle_id: contract.authority_oracle.oracle_id.clone(),
        authority_oracle_revision: contract.authority_oracle.oracle_revision.clone(),
        operating_regime_id: contract.operating_regime_id.clone(),
        schedule_count: dataset.schedules.len(),
        calibration_schedule_count: calibration_count,
        evaluation_schedule_count: evaluation_count,
        policy_receipt_count,
        source_artifact_count: contract.source_artifacts.len(),
        physical_failure_count,
        physical_unknown_count,
        authority_denied_count,
        authority_unknown_count,
        participant_floor_violation_count,
        minimum_observed_participant_service_units: minimum_service,
        maximum_service_loss_milli: maximum_loss,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

    fn binding(episode: &str, policy: &str, policy_revision: &str) -> OracleReceiptBinding {
        OracleReceiptBinding {
            episode_id: episode.to_owned(),
            policy_id: policy.to_owned(),
            policy_revision: policy_revision.to_owned(),
            evaluator_revision: "evaluator-v1".to_owned(),
            loss_contract_revision: "loss-v1".to_owned(),
            operating_regime_id: "regime-A".to_owned(),
        }
    }

    fn contract() -> OracleProjectionContract {
        OracleProjectionContract {
            schema_version: EXPECTED_SCHEMA.to_owned(),
            dataset_id: "maritime-oracle-fixture".to_owned(),
            dataset_revision: "dataset-v1".to_owned(),
            physical_oracle: OracleIdentity {
                oracle_id: "physical-oracle".to_owned(),
                oracle_revision: "physical-v1".to_owned(),
                implementation_sha256: A.to_owned(),
                independence_review_ref: "physical-code-review".to_owned(),
            },
            authority_oracle: OracleIdentity {
                oracle_id: "authority-oracle".to_owned(),
                oracle_revision: "authority-v1".to_owned(),
                implementation_sha256: B.to_owned(),
                independence_review_ref: "authority-code-review".to_owned(),
            },
            evaluator_revision: "evaluator-v1".to_owned(),
            loss_contract_revision: "loss-v1".to_owned(),
            operating_regime_id: "regime-A".to_owned(),
            policy_grid_sha256: D.to_owned(),
            policy_grid: vec![
                FrozenPolicyEntry {
                    parameter_index: 0,
                    policy_id: "policy-safe".to_owned(),
                    policy_revision: "policy-safe-v1".to_owned(),
                },
                FrozenPolicyEntry {
                    parameter_index: 1,
                    policy_id: "policy-mutating".to_owned(),
                    policy_revision: "policy-mutating-v1".to_owned(),
                },
            ],
            participant_floors: vec![
                ParticipantFloorContract {
                    participant_id: "receiver-A".to_owned(),
                    minimum_service_units: 10,
                },
                ParticipantFloorContract {
                    participant_id: "receiver-B".to_owned(),
                    minimum_service_units: 20,
                },
            ],
            source_artifacts: vec![
                SourceArtifactEvidence {
                    source_id: "schedule-input".to_owned(),
                    source_revision: "input-v1".to_owned(),
                    source_sha256: C.to_owned(),
                    kind: SourceEvidenceKind::ScheduleInput,
                    retrieved_at_utc: "2026-11-24T01:00:00Z".to_owned(),
                    publication_time_utc: Some("2026-11-24T00:00:00Z".to_owned()),
                    valid_from_time_utc: Some("2026-11-24T00:00:00Z".to_owned()),
                    valid_to_time_utc: None,
                },
                SourceArtifactEvidence {
                    source_id: "authority-record".to_owned(),
                    source_revision: "grant-v1".to_owned(),
                    source_sha256: D.to_owned(),
                    kind: SourceEvidenceKind::AuthorityRecord,
                    retrieved_at_utc: "2026-11-24T01:00:00Z".to_owned(),
                    publication_time_utc: None,
                    valid_from_time_utc: Some("2026-11-24T00:00:00Z".to_owned()),
                    valid_to_time_utc: None,
                },
                SourceArtifactEvidence {
                    source_id: "service-evidence".to_owned(),
                    source_revision: "service-v1".to_owned(),
                    source_sha256: A.to_owned(),
                    kind: SourceEvidenceKind::ParticipantService,
                    retrieved_at_utc: "2026-11-24T01:00:00Z".to_owned(),
                    publication_time_utc: Some("2026-11-24T01:00:00Z".to_owned()),
                    valid_from_time_utc: None,
                    valid_to_time_utc: None,
                },
            ],
        }
    }

    fn participant(
        participant_id: &str,
        delivered_service_units: u32,
    ) -> ParticipantServiceReceipt {
        ParticipantServiceReceipt {
            participant_id: participant_id.to_owned(),
            delivered_service_units,
            source_evidence_ids: vec!["service-evidence".to_owned()],
            receipt_sha256: C.to_owned(),
        }
    }

    fn policy(
        episode_id: &str,
        policy_id: &str,
        policy_revision: &str,
        physical: PhysicalDisposition,
        authority: AuthorityDisposition,
        action_class: PolicyActionClass,
        delivered_a: u32,
        delivered_b: u32,
    ) -> PolicyOracleProjection {
        let binding = binding(episode_id, policy_id, policy_revision);
        let authority_source = if authority == AuthorityDisposition::NotRequired {
            Vec::new()
        } else if authority == AuthorityDisposition::Unknown {
            Vec::new()
        } else {
            vec!["authority-record".to_owned()]
        };
        PolicyOracleProjection {
            policy_id: policy_id.to_owned(),
            policy_revision: policy_revision.to_owned(),
            physical: PhysicalOracleReceipt {
                binding: binding.clone(),
                oracle_id: "physical-oracle".to_owned(),
                oracle_revision: "physical-v1".to_owned(),
                disposition: physical,
                source_evidence_ids: vec!["schedule-input".to_owned()],
                receipt_sha256: A.to_owned(),
            },
            authority: AuthorityOracleReceipt {
                binding: binding.clone(),
                oracle_id: "authority-oracle".to_owned(),
                oracle_revision: "authority-v1".to_owned(),
                action_class,
                disposition: authority,
                authority_record_sha256: match authority {
                    AuthorityDisposition::Authorized | AuthorityDisposition::Denied => Some(D.to_owned()),
                    AuthorityDisposition::NotRequired | AuthorityDisposition::Unknown => None,
                },
                source_evidence_ids: authority_source,
                receipt_sha256: B.to_owned(),
            },
            service_loss: ServiceLossOracleReceipt {
                binding,
                loss_milli: 100,
                source_evidence_ids: vec!["service-evidence".to_owned()],
                receipt_sha256: C.to_owned(),
            },
            participant_service: vec![
                participant("receiver-A", delivered_a),
                participant("receiver-B", delivered_b),
            ],
        }
    }

    fn schedule(
        episode_id: &str,
        cluster: &str,
        role: DatasetRole,
        policy_outcomes: Vec<PolicyOracleProjection>,
    ) -> ScheduleOracleProjection {
        ScheduleOracleProjection {
            episode_id: episode_id.to_owned(),
            dependence_cluster_id: cluster.to_owned(),
            scenario_family: "berth-disruption".to_owned(),
            operating_regime_id: "regime-A".to_owned(),
            role,
            policy_outcomes,
        }
    }

    fn valid_projection() -> IndependentOracleProjection {
        IndependentOracleProjection {
            contract: contract(),
            schedules: vec![
                schedule(
                    "cal-1",
                    "weather-1",
                    DatasetRole::Calibration,
                    vec![
                        policy(
                            "cal-1",
                            "policy-safe",
                            "policy-safe-v1",
                            PhysicalDisposition::Feasible,
                            AuthorityDisposition::NotRequired,
                            PolicyActionClass::NoMutation,
                            12,
                            25,
                        ),
                        policy(
                            "cal-1",
                            "policy-mutating",
                            "policy-mutating-v1",
                            PhysicalDisposition::Infeasible,
                            AuthorityDisposition::Authorized,
                            PolicyActionClass::Mutating,
                            11,
                            22,
                        ),
                    ],
                ),
                schedule(
                    "eval-1",
                    "outage-1",
                    DatasetRole::Evaluation,
                    vec![
                        policy(
                            "eval-1",
                            "policy-safe",
                            "policy-safe-v1",
                            PhysicalDisposition::Feasible,
                            AuthorityDisposition::NotRequired,
                            PolicyActionClass::NoMutation,
                            14,
                            26,
                        ),
                        policy(
                            "eval-1",
                            "policy-mutating",
                            "policy-mutating-v1",
                            PhysicalDisposition::Feasible,
                            AuthorityDisposition::Authorized,
                            PolicyActionClass::Mutating,
                            12,
                            23,
                        ),
                    ],
                ),
            ],
        }
    }

    #[test]
    fn projection_preserves_distinct_physical_authority_and_participant_axes() {
        let audit = audit_independent_oracle_projection(&valid_projection()).unwrap();
        assert_eq!(audit.schedule_count, 2);
        assert_eq!(audit.calibration_schedule_count, 1);
        assert_eq!(audit.evaluation_schedule_count, 1);
        assert_eq!(audit.policy_receipt_count, 4);
        assert_eq!(audit.physical_failure_count, 1);
        assert_eq!(audit.physical_unknown_count, 0);
        assert_eq!(audit.authority_denied_count, 0);
        assert_eq!(audit.participant_floor_violation_count, 0);
        assert_eq!(audit.minimum_observed_participant_service_units, Some(11));
        assert!(!audit.all_candidates_admissible());
    }

    #[test]
    fn unknown_physical_outcome_is_retained_as_a_blocker() {
        let mut data = valid_projection();
        data.schedules[0].policy_outcomes[0].physical.disposition = PhysicalDisposition::Unknown;
        let audit = audit_independent_oracle_projection(&data).unwrap();
        assert_eq!(audit.physical_unknown_count, 1);
        assert!(!audit.all_dispositions_resolved());
    }

    #[test]
    fn denied_authority_is_retained_and_never_promoted_to_authorized() {
        let mut data = valid_projection();
        data.schedules[0].policy_outcomes[1].authority.disposition = AuthorityDisposition::Denied;
        let audit = audit_independent_oracle_projection(&data).unwrap();
        assert_eq!(audit.authority_denied_count, 1);
        assert!(!audit.all_candidates_admissible());
    }

    #[test]
    fn unknown_authority_is_retained_without_fabricating_an_authority_record() {
        let mut data = valid_projection();
        let authority = &mut data.schedules[0].policy_outcomes[1].authority;
        authority.disposition = AuthorityDisposition::Unknown;
        authority.authority_record_sha256 = None;
        authority.source_evidence_ids.clear();
        let audit = audit_independent_oracle_projection(&data).unwrap();
        assert_eq!(audit.authority_unknown_count, 1);
        assert!(!audit.all_dispositions_resolved());
    }

    #[test]
    fn participant_floor_breaches_are_counted_per_participant_not_averaged() {
        let mut data = valid_projection();
        data.schedules[0].policy_outcomes[0].participant_service[0].delivered_service_units = 0;
        let audit = audit_independent_oracle_projection(&data).unwrap();
        assert_eq!(audit.participant_floor_violation_count, 1);
        assert!(!audit.all_participant_floors_met());
    }

    #[test]
    fn participant_universe_must_be_complete_for_each_policy_episode() {
        let mut data = valid_projection();
        data.schedules[0].policy_outcomes[0].participant_service.pop();
        assert!(matches!(
            audit_independent_oracle_projection(&data),
            Err(OracleProjectionError::MissingParticipantReceipts { .. })
        ));
    }

    #[test]
    fn mismatched_physical_and_authority_bindings_are_rejected() {
        let mut data = valid_projection();
        data.schedules[0].policy_outcomes[0].authority.binding.episode_id = "other-episode".to_owned();
        assert!(matches!(
            audit_independent_oracle_projection(&data),
            Err(OracleProjectionError::ReceiptBindingMismatch { axis: "authority", .. })
        ));
    }

    #[test]
    fn oracle_implementations_must_have_distinct_declared_identities() {
        let mut data = valid_projection();
        data.contract.authority_oracle.implementation_sha256 =
            data.contract.physical_oracle.implementation_sha256.clone();
        assert_eq!(
            audit_independent_oracle_projection(&data),
            Err(OracleProjectionError::OracleIdentitiesNotDistinct)
        );
    }

    #[test]
    fn receipts_must_reference_registered_source_artifacts() {
        let mut data = valid_projection();
        data.schedules[0].policy_outcomes[0].physical.source_evidence_ids[0] =
            "unregistered-source".to_owned();
        assert!(matches!(
            audit_independent_oracle_projection(&data),
            Err(OracleProjectionError::UnknownEvidenceReference { .. })
        ));
    }

    #[test]
    fn authorized_mutation_requires_an_authority_record_digest_and_source() {
        let mut data = valid_projection();
        let authority = &mut data.schedules[0].policy_outcomes[1].authority;
        authority.authority_record_sha256 = None;
        assert!(matches!(
            audit_independent_oracle_projection(&data),
            Err(OracleProjectionError::AuthorityRecordDigestMismatch { .. })
        ));
    }

    #[test]
    fn source_times_cannot_be_present_but_empty() {
        let mut data = valid_projection();
        data.contract.source_artifacts[0].publication_time_utc = Some(" ".to_owned());
        assert!(matches!(
            audit_independent_oracle_projection(&data),
            Err(OracleProjectionError::InvalidSourceTime { .. })
        ));
    }

    #[test]
    fn reused_dependence_clusters_are_rejected() {
        let mut data = valid_projection();
        data.schedules[1].dependence_cluster_id = data.schedules[0].dependence_cluster_id.clone();
        assert!(matches!(
            audit_independent_oracle_projection(&data),
            Err(OracleProjectionError::ReusedDependenceCluster(_))
        ));
    }
}
