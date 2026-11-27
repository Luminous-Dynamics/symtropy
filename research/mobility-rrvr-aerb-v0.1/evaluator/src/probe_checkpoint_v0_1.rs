// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! AERB v0.1 checkpoint lifecycle reducer.
//!
//! This module composes upstream evidence, decision-value and physical-path
//! receipts. It does not authenticate their issuers, run a minimax planner,
//! establish physical viability, close a command path, or authorize actuation.
//! Unknown physical horizons never become infinity. Partial evidence remains
//! bound to its exact acquired prefix and cannot prove absence over an unobserved
//! suffix.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub const CHECKPOINT_SCHEMA_VERSION: &str = "aerb-probe-checkpoint-v0.1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeLifecycle {
    Acquiring,
    Interrupted,
    Completed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimSemantics {
    PositiveEvent,
    GeneralMeasurement,
    AbsenceOverInterval,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceQuality {
    Admitted,
    Stale,
    Unknown,
    Disputed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartialEvidenceDisposition {
    NoEvidence,
    AdmittedForDeclaredScope,
    AdmittedForSubscope,
    InsufficientForDeclaredScope,
    Unqualified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalPathKind {
    ContinueProbe,
    SafeStopHold,
    AbortProbe,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyQualification {
    Qualified,
    NotQualified,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointAction {
    ContinueProbe,
    StopSensingHold,
    AbortProbe,
    NoSafeContinuation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceScope {
    pub proposition_id: String,
    pub clock_domain_id: String,
    pub clock_mapping_profile_sha256: String,
    pub start_ns: u64,
    pub end_ns: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidencePrefix {
    pub evidence_id: String,
    pub artifact_sha256: String,
    pub source_id: String,
    pub source_profile_sha256: String,
    pub calibration_profile_sha256: String,
    pub proposition_id: String,
    pub claim_semantics: ClaimSemantics,
    pub quality: EvidenceQuality,
    pub clock_domain_id: String,
    pub clock_mapping_profile_sha256: String,
    pub coverage_start_ns: u64,
    pub coverage_end_ns: u64,
    /// True only when the entire declared prefix contains no unobserved gaps.
    pub coverage_complete: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportUpdateReceipt {
    pub checkpoint_sequence: u64,
    pub base_support_sha256: String,
    pub resulting_support_sha256: String,
    pub evidence_artifact_sha256: String,
    pub evidence_prefix_sha256: String,
    pub oracle_profile_sha256: String,
    pub derivation_receipt_sha256: String,
    pub derivation_complete: bool,
    pub open_world_scope_closure_receipt_sha256: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionSufficiencyReceipt {
    pub checkpoint_sequence: u64,
    pub support_sha256: String,
    pub oracle_profile_sha256: String,
    pub receipt_sha256: String,
    pub oracle_complete: bool,
    /// Supplied by the existing sequential robust-policy oracle.
    pub all_supported_worlds_share_terminal_recommendation: bool,
    /// True only when no remaining probe strictly improves the oracle's
    /// declared worst-case terminal decision loss.
    pub no_remaining_probe_strictly_reduces_worst_case_loss: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalPathReceipt {
    pub path_kind: PhysicalPathKind,
    pub attempt_id: String,
    pub probe_id: String,
    pub checkpoint_sequence: u64,
    pub physical_plan_sha256: String,
    pub path_profile_sha256: String,
    pub receipt_sha256: String,
    pub full_trajectory_covered: bool,
    /// Upstream physical supervisor mandates this path at this checkpoint.
    /// This assertion is consumed, not authenticated, by the reducer.
    pub required_by_safety_supervisor: bool,
    pub required_duration_ns: u64,
    /// None is explicitly unknown, never an unbounded safe horizon.
    pub safe_horizon_ns: Option<u64>,
    pub qualification: SafetyQualification,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeCheckpoint {
    pub schema_version: String,
    pub attempt_id: String,
    pub probe_id: String,
    pub checkpoint_id: String,
    pub sequence: u64,
    pub previous_checkpoint_sha256: Option<String>,
    pub lifecycle: ProbeLifecycle,
    pub physical_plan_sha256: String,
    pub requested_scope: EvidenceScope,
    pub evidence_prefix: Option<EvidencePrefix>,
    pub proposed_possible_world_support_ids: Vec<String>,
    pub support_update_receipt: Option<SupportUpdateReceipt>,
    pub decision_sufficiency_receipt: Option<DecisionSufficiencyReceipt>,
    pub physical_path_receipts: Vec<PhysicalPathReceipt>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointCursor {
    pub attempt_id: String,
    pub probe_id: String,
    pub last_sequence: u64,
    pub last_checkpoint_sha256: Option<String>,
    pub possible_world_support_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointResult {
    pub action: CheckpointAction,
    pub partial_evidence_disposition: PartialEvidenceDisposition,
    pub decision_sufficient: bool,
    pub possible_world_support_ids: Vec<String>,
    pub next_cursor: CheckpointCursor,
    /// This reducer is an observer-side adapter and can never admit a new
    /// physical effect. The independent action-ticket owner remains mandatory.
    pub new_effect_permitted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckpointError {
    InvalidIdentity(&'static str),
    UnsupportedSchema,
    InvalidDigest(&'static str),
    InvalidScope(&'static str),
    InvalidSupport(&'static str),
    InvalidSupportTransition(&'static str),
    InvalidSupportReceipt(&'static str),
    InvalidDecisionReceipt(&'static str),
    InvalidPathReceipt(&'static str),
    SequenceOverflow,
    SequenceMismatch,
    PreviousCheckpointMismatch,
    SerializationFailed,
}

impl std::fmt::Display for CheckpointError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIdentity(field) => {
                write!(formatter, "invalid checkpoint identity: {field}")
            }
            Self::UnsupportedSchema => formatter.write_str("unsupported checkpoint schema"),
            Self::InvalidDigest(field) => write!(formatter, "invalid digest: {field}"),
            Self::InvalidScope(field) => write!(formatter, "invalid evidence scope: {field}"),
            Self::InvalidSupport(field) => {
                write!(formatter, "invalid possible-world support: {field}")
            }
            Self::InvalidSupportTransition(field) => {
                write!(formatter, "invalid support transition: {field}")
            }
            Self::InvalidSupportReceipt(field) => {
                write!(formatter, "invalid support receipt: {field}")
            }
            Self::InvalidDecisionReceipt(field) => {
                write!(formatter, "invalid decision receipt: {field}")
            }
            Self::InvalidPathReceipt(field) => {
                write!(formatter, "invalid physical-path receipt: {field}")
            }
            Self::SequenceOverflow => formatter.write_str("checkpoint sequence overflow"),
            Self::SequenceMismatch => {
                formatter.write_str("checkpoint sequence is not the next sequence")
            }
            Self::PreviousCheckpointMismatch => {
                formatter.write_str("previous checkpoint digest mismatch")
            }
            Self::SerializationFailed => formatter.write_str("checkpoint serialization failed"),
        }
    }
}

impl std::error::Error for CheckpointError {}

/// A canonical digest for a set of possible worlds. IDs are sorted and length
/// prefixed so caller ordering cannot alter the semantic set identity.
pub fn possible_world_support_sha256(ids: &[String]) -> Result<String, CheckpointError> {
    let canonical = canonical_support(ids)?;
    let mut bytes = b"AERB-POSSIBLE-WORLD-SUPPORT-V0.1\0".to_vec();
    for id in canonical {
        let length =
            u64::try_from(id.len()).map_err(|_| CheckpointError::InvalidSupport("ID length"))?;
        bytes.extend_from_slice(&length.to_be_bytes());
        bytes.extend_from_slice(id.as_bytes());
    }
    Ok(super::sha256_hex(&bytes))
}

fn canonical_support(ids: &[String]) -> Result<Vec<String>, CheckpointError> {
    if ids.is_empty() {
        return Err(CheckpointError::InvalidSupport("empty support"));
    }
    let mut canonical = ids.to_vec();
    if canonical.iter().any(|id| id.trim().is_empty()) {
        return Err(CheckpointError::InvalidSupport("empty world ID"));
    }
    canonical.sort();
    if canonical.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(CheckpointError::InvalidSupport("duplicate world ID"));
    }
    Ok(canonical)
}

fn serialized_digest<T: Serialize>(value: &T, domain: &[u8]) -> Result<String, CheckpointError> {
    let payload = serde_json::to_vec(value).map_err(|_| CheckpointError::SerializationFailed)?;
    let mut preimage = domain.to_vec();
    preimage.extend_from_slice(&payload);
    Ok(super::sha256_hex(&preimage))
}

fn support_update_commitment(receipt: &SupportUpdateReceipt) -> Result<String, CheckpointError> {
    let mut payload = receipt.clone();
    payload.derivation_receipt_sha256.clear();
    serialized_digest(&payload, b"AERB-SUPPORT-UPDATE-RECEIPT-V0.1\0")
}

fn decision_sufficiency_commitment(
    receipt: &DecisionSufficiencyReceipt,
) -> Result<String, CheckpointError> {
    let mut payload = receipt.clone();
    payload.receipt_sha256.clear();
    serialized_digest(&payload, b"AERB-DECISION-SUFFICIENCY-RECEIPT-V0.1\0")
}

fn physical_path_commitment(receipt: &PhysicalPathReceipt) -> Result<String, CheckpointError> {
    let mut payload = receipt.clone();
    payload.receipt_sha256.clear();
    serialized_digest(&payload, b"AERB-PHYSICAL-PATH-RECEIPT-V0.1\0")
}

fn evidence_prefix_commitment(prefix: &EvidencePrefix) -> Result<String, CheckpointError> {
    serialized_digest(prefix, b"AERB-EVIDENCE-PREFIX-V0.1\0")
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_digest(value: &str, field: &'static str) -> Result<(), CheckpointError> {
    if valid_sha256(value) {
        Ok(())
    } else {
        Err(CheckpointError::InvalidDigest(field))
    }
}

fn classify_partial_evidence(
    requested: &EvidenceScope,
    prefix: Option<&EvidencePrefix>,
) -> Result<PartialEvidenceDisposition, CheckpointError> {
    let Some(prefix) = prefix else {
        return Ok(PartialEvidenceDisposition::NoEvidence);
    };

    for (value, field) in [
        (prefix.evidence_id.as_str(), "evidence_id"),
        (prefix.source_id.as_str(), "source_id"),
        (prefix.proposition_id.as_str(), "proposition_id"),
        (prefix.clock_domain_id.as_str(), "clock_domain_id"),
    ] {
        if !nonempty(value) {
            return Err(CheckpointError::InvalidIdentity(field));
        }
    }
    validate_digest(&prefix.artifact_sha256, "evidence_artifact")?;
    validate_digest(&prefix.source_profile_sha256, "evidence_source_profile")?;
    validate_digest(
        &prefix.calibration_profile_sha256,
        "evidence_calibration_profile",
    )?;
    validate_digest(
        &prefix.clock_mapping_profile_sha256,
        "evidence_clock_profile",
    )?;

    if prefix.proposition_id != requested.proposition_id
        || prefix.clock_domain_id != requested.clock_domain_id
        || prefix.clock_mapping_profile_sha256 != requested.clock_mapping_profile_sha256
    {
        return Err(CheckpointError::InvalidScope(
            "evidence identity or clock profile mismatch",
        ));
    }
    if prefix.coverage_start_ns != requested.start_ns
        || prefix.coverage_end_ns <= prefix.coverage_start_ns
        || prefix.coverage_end_ns > requested.end_ns
    {
        return Err(CheckpointError::InvalidScope(
            "observed range is not a prefix of requested scope",
        ));
    }

    if prefix.quality != EvidenceQuality::Admitted {
        return Ok(PartialEvidenceDisposition::Unqualified);
    }
    if !prefix.coverage_complete {
        return Ok(PartialEvidenceDisposition::InsufficientForDeclaredScope);
    }

    let full_scope = prefix.coverage_end_ns == requested.end_ns;
    if full_scope {
        return Ok(PartialEvidenceDisposition::AdmittedForDeclaredScope);
    }
    if prefix.claim_semantics == ClaimSemantics::AbsenceOverInterval {
        // A valid prefix may be retained as an artifact, but cannot establish
        // absence over the unobserved suffix of the declared interval.
        return Ok(PartialEvidenceDisposition::InsufficientForDeclaredScope);
    }
    Ok(PartialEvidenceDisposition::AdmittedForSubscope)
}

fn validate_support_update(
    cursor_support: &[String],
    proposed_support: &[String],
    receipt: Option<&SupportUpdateReceipt>,
    evidence: Option<&EvidencePrefix>,
    evidence_disposition: PartialEvidenceDisposition,
    checkpoint_sequence: u64,
) -> Result<Vec<String>, CheckpointError> {
    let before = canonical_support(cursor_support)?;
    let after = canonical_support(proposed_support)?;

    if before == after {
        if receipt.is_some() {
            return Err(CheckpointError::InvalidSupportReceipt(
                "receipt supplied without a support change",
            ));
        }
        return Ok(before);
    }

    if !after
        .iter()
        .all(|world| before.binary_search(world).is_ok())
    {
        return Err(CheckpointError::InvalidSupportTransition(
            "this reducer does not widen support; use the upstream invalidation owner",
        ));
    }
    if !matches!(
        evidence_disposition,
        PartialEvidenceDisposition::AdmittedForDeclaredScope
            | PartialEvidenceDisposition::AdmittedForSubscope
    ) {
        return Err(CheckpointError::InvalidSupportTransition(
            "unqualified or insufficient evidence cannot narrow support",
        ));
    }
    let evidence = evidence.ok_or(CheckpointError::InvalidSupportTransition(
        "support narrowed without evidence",
    ))?;
    let receipt = receipt.ok_or(CheckpointError::InvalidSupportReceipt(
        "support narrowing requires an upstream derivation receipt",
    ))?;
    if receipt.checkpoint_sequence != checkpoint_sequence {
        return Err(CheckpointError::InvalidSupportReceipt(
            "checkpoint sequence",
        ));
    }
    validate_digest(&receipt.base_support_sha256, "base_support")?;
    validate_digest(&receipt.resulting_support_sha256, "resulting_support")?;
    validate_digest(&receipt.evidence_artifact_sha256, "support_evidence")?;
    validate_digest(&receipt.evidence_prefix_sha256, "support_evidence_prefix")?;
    validate_digest(&receipt.oracle_profile_sha256, "support_oracle_profile")?;
    validate_digest(&receipt.derivation_receipt_sha256, "support_derivation")?;

    if !receipt.derivation_complete {
        return Err(CheckpointError::InvalidSupportReceipt(
            "derivation coverage incomplete",
        ));
    }
    if before
        .iter()
        .any(|world| world == super::OPEN_WORLD_RESIDUAL)
        && !after
            .iter()
            .any(|world| world == super::OPEN_WORLD_RESIDUAL)
    {
        let closure = receipt
            .open_world_scope_closure_receipt_sha256
            .as_deref()
            .ok_or(CheckpointError::InvalidSupportReceipt(
                "open-world residual removal requires scope-closure receipt",
            ))?;
        validate_digest(closure, "open_world_scope_closure")?;
        if evidence_disposition != PartialEvidenceDisposition::AdmittedForDeclaredScope {
            return Err(CheckpointError::InvalidSupportReceipt(
                "open-world residual cannot be removed by partial scope",
            ));
        }
    }
    if receipt.base_support_sha256 != possible_world_support_sha256(&before)?
        || receipt.resulting_support_sha256 != possible_world_support_sha256(&after)?
        || receipt.evidence_artifact_sha256 != evidence.artifact_sha256
        || receipt.evidence_prefix_sha256 != evidence_prefix_commitment(evidence)?
        || receipt.derivation_receipt_sha256 != support_update_commitment(receipt)?
    {
        return Err(CheckpointError::InvalidSupportReceipt(
            "receipt does not bind exact support and evidence",
        ));
    }
    Ok(after)
}

fn validate_decision_sufficiency(
    receipt: Option<&DecisionSufficiencyReceipt>,
    support: &[String],
    checkpoint_sequence: u64,
) -> Result<bool, CheckpointError> {
    let Some(receipt) = receipt else {
        return Ok(false);
    };
    if receipt.checkpoint_sequence != checkpoint_sequence {
        return Err(CheckpointError::InvalidDecisionReceipt(
            "checkpoint sequence",
        ));
    }
    validate_digest(&receipt.support_sha256, "decision_support")?;
    validate_digest(&receipt.oracle_profile_sha256, "decision_oracle_profile")?;
    validate_digest(&receipt.receipt_sha256, "decision_sufficiency")?;
    if receipt.support_sha256 != possible_world_support_sha256(support)?
        || receipt.receipt_sha256 != decision_sufficiency_commitment(receipt)?
    {
        return Err(CheckpointError::InvalidDecisionReceipt(
            "support digest mismatch",
        ));
    }
    Ok(receipt.oracle_complete
        && receipt.all_supported_worlds_share_terminal_recommendation
        && receipt.no_remaining_probe_strictly_reduces_worst_case_loss)
}

fn path_is_qualified(
    receipts: &[PhysicalPathReceipt],
    kind: PhysicalPathKind,
    attempt_id: &str,
    probe_id: &str,
    checkpoint_sequence: u64,
    expected_physical_plan_sha256: &str,
) -> bool {
    receipts.iter().any(|receipt| {
        receipt.path_kind == kind
            && receipt.attempt_id == attempt_id
            && receipt.probe_id == probe_id
            && receipt.checkpoint_sequence == checkpoint_sequence
            && receipt.physical_plan_sha256 == expected_physical_plan_sha256
            && receipt.qualification == SafetyQualification::Qualified
            && receipt.full_trajectory_covered
            && receipt
                .safe_horizon_ns
                .is_some_and(|horizon| horizon >= receipt.required_duration_ns)
            && receipt.required_duration_ns > 0
            && valid_sha256(&receipt.path_profile_sha256)
            && valid_sha256(&receipt.receipt_sha256)
    })
}

fn checkpoint_fingerprint(checkpoint: &ProbeCheckpoint) -> Result<String, CheckpointError> {
    let mut canonical = checkpoint.clone();
    canonical.proposed_possible_world_support_ids.sort();
    canonical
        .physical_path_receipts
        .sort_by_key(|receipt| receipt.path_kind);
    let bytes = serde_json::to_vec(&canonical).map_err(|_| CheckpointError::SerializationFailed)?;
    Ok(super::sha256_hex(&bytes))
}

/// Consume one strictly ordered checkpoint and produce a fail-closed lifecycle
/// disposition. The function only interprets upstream receipts; it does not
/// authenticate issuers or prove that their physical/evidence coverage claims
/// are true. It always leaves new-effect authorization false.
pub fn evaluate_checkpoint(
    cursor: &CheckpointCursor,
    checkpoint: &ProbeCheckpoint,
) -> Result<CheckpointResult, CheckpointError> {
    if checkpoint.schema_version != CHECKPOINT_SCHEMA_VERSION {
        return Err(CheckpointError::UnsupportedSchema);
    }
    for (value, field) in [
        (cursor.attempt_id.as_str(), "cursor.attempt_id"),
        (cursor.probe_id.as_str(), "cursor.probe_id"),
        (checkpoint.attempt_id.as_str(), "attempt_id"),
        (checkpoint.probe_id.as_str(), "probe_id"),
        (checkpoint.checkpoint_id.as_str(), "checkpoint_id"),
        (
            checkpoint.requested_scope.proposition_id.as_str(),
            "scope.proposition_id",
        ),
        (
            checkpoint.requested_scope.clock_domain_id.as_str(),
            "scope.clock_domain_id",
        ),
    ] {
        if !nonempty(value) {
            return Err(CheckpointError::InvalidIdentity(field));
        }
    }
    if cursor.attempt_id != checkpoint.attempt_id || cursor.probe_id != checkpoint.probe_id {
        return Err(CheckpointError::InvalidIdentity("attempt/probe binding"));
    }
    validate_digest(&checkpoint.physical_plan_sha256, "physical_plan")?;
    validate_digest(
        &checkpoint.requested_scope.clock_mapping_profile_sha256,
        "requested_clock_profile",
    )?;
    if checkpoint.requested_scope.end_ns <= checkpoint.requested_scope.start_ns {
        return Err(CheckpointError::InvalidScope(
            "requested interval is empty or reversed",
        ));
    }
    if cursor.last_sequence == 0 {
        if cursor.last_checkpoint_sha256.is_some() {
            return Err(CheckpointError::PreviousCheckpointMismatch);
        }
    } else if !cursor
        .last_checkpoint_sha256
        .as_deref()
        .is_some_and(valid_sha256)
    {
        return Err(CheckpointError::PreviousCheckpointMismatch);
    }
    let expected_sequence = cursor
        .last_sequence
        .checked_add(1)
        .ok_or(CheckpointError::SequenceOverflow)?;
    if checkpoint.sequence != expected_sequence {
        return Err(CheckpointError::SequenceMismatch);
    }
    if checkpoint.previous_checkpoint_sha256 != cursor.last_checkpoint_sha256 {
        return Err(CheckpointError::PreviousCheckpointMismatch);
    }

    let partial_evidence_disposition = classify_partial_evidence(
        &checkpoint.requested_scope,
        checkpoint.evidence_prefix.as_ref(),
    )?;
    let possible_world_support_ids = validate_support_update(
        &cursor.possible_world_support_ids,
        &checkpoint.proposed_possible_world_support_ids,
        checkpoint.support_update_receipt.as_ref(),
        checkpoint.evidence_prefix.as_ref(),
        partial_evidence_disposition,
        checkpoint.sequence,
    )?;
    let decision_sufficient = validate_decision_sufficiency(
        checkpoint.decision_sufficiency_receipt.as_ref(),
        &possible_world_support_ids,
        checkpoint.sequence,
    )?;

    let mut path_kinds = BTreeSet::new();
    for receipt in &checkpoint.physical_path_receipts {
        if !path_kinds.insert(receipt.path_kind) {
            return Err(CheckpointError::InvalidPathReceipt("duplicate path kind"));
        }
        if !nonempty(&receipt.attempt_id) || !nonempty(&receipt.probe_id) {
            return Err(CheckpointError::InvalidPathReceipt("empty path identity"));
        }
        validate_digest(&receipt.physical_plan_sha256, "physical_plan")?;
        validate_digest(&receipt.path_profile_sha256, "physical_path_profile")?;
        validate_digest(&receipt.receipt_sha256, "physical_path_receipt")?;
        if receipt.physical_plan_sha256 != checkpoint.physical_plan_sha256
            || receipt.receipt_sha256 != physical_path_commitment(receipt)?
        {
            return Err(CheckpointError::InvalidPathReceipt(
                "plan binding or receipt commitment mismatch",
            ));
        }
    }

    let can_continue = checkpoint.lifecycle == ProbeLifecycle::Acquiring
        && path_is_qualified(
            &checkpoint.physical_path_receipts,
            PhysicalPathKind::ContinueProbe,
            &checkpoint.attempt_id,
            &checkpoint.probe_id,
            checkpoint.sequence,
            &checkpoint.physical_plan_sha256,
        );
    let can_stop = path_is_qualified(
        &checkpoint.physical_path_receipts,
        PhysicalPathKind::SafeStopHold,
        &checkpoint.attempt_id,
        &checkpoint.probe_id,
        checkpoint.sequence,
        &checkpoint.physical_plan_sha256,
    );
    let can_abort = path_is_qualified(
        &checkpoint.physical_path_receipts,
        PhysicalPathKind::AbortProbe,
        &checkpoint.attempt_id,
        &checkpoint.probe_id,
        checkpoint.sequence,
        &checkpoint.physical_plan_sha256,
    );

    let mandatory_paths: Vec<&PhysicalPathReceipt> = checkpoint
        .physical_path_receipts
        .iter()
        .filter(|receipt| receipt.required_by_safety_supervisor)
        .collect();
    if mandatory_paths.len() > 1 {
        return Err(CheckpointError::InvalidPathReceipt(
            "conflicting mandatory supervisor paths",
        ));
    }

    let action = if let Some(mandatory) = mandatory_paths.first() {
        // An upstream physical supervisor's checkpoint-bound mandatory path
        // overrides the observer's information-value preference. If that
        // mandatory path is not fully qualified, fail closed rather than
        // silently substitute a different trajectory.
        if mandatory.path_kind == PhysicalPathKind::ContinueProbe && !can_continue {
            CheckpointAction::NoSafeContinuation
        } else if mandatory.path_kind == PhysicalPathKind::SafeStopHold && !can_stop {
            CheckpointAction::NoSafeContinuation
        } else if mandatory.path_kind == PhysicalPathKind::AbortProbe && !can_abort {
            CheckpointAction::NoSafeContinuation
        } else if checkpoint.lifecycle == ProbeLifecycle::Completed
            && mandatory.path_kind == PhysicalPathKind::ContinueProbe
        {
            CheckpointAction::NoSafeContinuation
        } else {
            match mandatory.path_kind {
                PhysicalPathKind::ContinueProbe => CheckpointAction::ContinueProbe,
                PhysicalPathKind::SafeStopHold => CheckpointAction::StopSensingHold,
                PhysicalPathKind::AbortProbe => CheckpointAction::AbortProbe,
            }
        }
    } else if checkpoint.lifecycle == ProbeLifecycle::Completed {
        // Completion closes acquisition, not the physical need for a qualified
        // hold trajectory. A completed observation is never itself a safe hold.
        if can_stop {
            CheckpointAction::StopSensingHold
        } else {
            CheckpointAction::NoSafeContinuation
        }
    } else if decision_sufficient {
        if can_stop {
            CheckpointAction::StopSensingHold
        } else if can_abort {
            CheckpointAction::AbortProbe
        } else {
            CheckpointAction::NoSafeContinuation
        }
    } else if can_continue {
        CheckpointAction::ContinueProbe
    } else if can_stop {
        // Prefer a qualified non-abort hold only when the safety supervisor
        // has not marked another physical path as mandatory.
        CheckpointAction::StopSensingHold
    } else if can_abort {
        CheckpointAction::AbortProbe
    } else {
        CheckpointAction::NoSafeContinuation
    };

    let checkpoint_sha256 = checkpoint_fingerprint(checkpoint)?;
    let next_cursor = CheckpointCursor {
        attempt_id: cursor.attempt_id.clone(),
        probe_id: cursor.probe_id.clone(),
        last_sequence: checkpoint.sequence,
        last_checkpoint_sha256: Some(checkpoint_sha256),
        possible_world_support_ids: possible_world_support_ids.clone(),
    };

    Ok(CheckpointResult {
        action,
        partial_evidence_disposition,
        decision_sufficient,
        possible_world_support_ids,
        next_cursor,
        new_effect_permitted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn initial_cursor() -> CheckpointCursor {
        CheckpointCursor {
            attempt_id: "attempt-1".to_owned(),
            probe_id: "probe-1".to_owned(),
            last_sequence: 0,
            last_checkpoint_sha256: None,
            possible_world_support_ids: vec!["world-a".to_owned(), "world-b".to_owned()],
        }
    }

    fn prefix(semantics: ClaimSemantics, quality: EvidenceQuality, end_ns: u64) -> EvidencePrefix {
        EvidencePrefix {
            evidence_id: "evidence-1".to_owned(),
            artifact_sha256: digest('a'),
            source_id: "independent-sensor-1".to_owned(),
            source_profile_sha256: digest('b'),
            calibration_profile_sha256: digest('c'),
            proposition_id: "effect-presence-over-window".to_owned(),
            claim_semantics: semantics,
            quality,
            clock_domain_id: "robot-clock".to_owned(),
            clock_mapping_profile_sha256: digest('d'),
            coverage_start_ns: 10,
            coverage_end_ns: end_ns,
            coverage_complete: true,
        }
    }

    fn path(
        kind: PhysicalPathKind,
        qualification: SafetyQualification,
        required_duration_ns: u64,
        safe_horizon_ns: Option<u64>,
    ) -> PhysicalPathReceipt {
        let mut receipt = PhysicalPathReceipt {
            path_kind: kind,
            attempt_id: "attempt-1".to_owned(),
            probe_id: "probe-1".to_owned(),
            checkpoint_sequence: 1,
            physical_plan_sha256: digest('0'),
            path_profile_sha256: digest('e'),
            receipt_sha256: String::new(),
            full_trajectory_covered: true,
            required_by_safety_supervisor: false,
            required_duration_ns,
            safe_horizon_ns,
            qualification,
        };
        receipt.receipt_sha256 = physical_path_commitment(&receipt).expect("path commitment");
        receipt
    }

    fn checkpoint() -> ProbeCheckpoint {
        ProbeCheckpoint {
            schema_version: CHECKPOINT_SCHEMA_VERSION.to_owned(),
            attempt_id: "attempt-1".to_owned(),
            probe_id: "probe-1".to_owned(),
            checkpoint_id: "checkpoint-1".to_owned(),
            sequence: 1,
            previous_checkpoint_sha256: None,
            lifecycle: ProbeLifecycle::Acquiring,
            physical_plan_sha256: digest('0'),
            requested_scope: EvidenceScope {
                proposition_id: "effect-presence-over-window".to_owned(),
                clock_domain_id: "robot-clock".to_owned(),
                clock_mapping_profile_sha256: digest('d'),
                start_ns: 10,
                end_ns: 20,
            },
            evidence_prefix: Some(prefix(
                ClaimSemantics::PositiveEvent,
                EvidenceQuality::Admitted,
                15,
            )),
            proposed_possible_world_support_ids: vec!["world-a".to_owned(), "world-b".to_owned()],
            support_update_receipt: None,
            decision_sufficiency_receipt: None,
            physical_path_receipts: vec![path(
                PhysicalPathKind::ContinueProbe,
                SafetyQualification::Qualified,
                40,
                Some(100),
            )],
        }
    }

    fn decision_receipt(support: &[String], sequence: u64) -> DecisionSufficiencyReceipt {
        let mut receipt = DecisionSufficiencyReceipt {
            checkpoint_sequence: sequence,
            support_sha256: possible_world_support_sha256(support).expect("support digest"),
            oracle_profile_sha256: digest('1'),
            receipt_sha256: String::new(),
            oracle_complete: true,
            all_supported_worlds_share_terminal_recommendation: true,
            no_remaining_probe_strictly_reduces_worst_case_loss: true,
        };
        receipt.receipt_sha256 =
            decision_sufficiency_commitment(&receipt).expect("decision receipt commitment");
        receipt
    }

    fn support_receipt(
        before: &[String],
        after: &[String],
        evidence: &EvidencePrefix,
    ) -> SupportUpdateReceipt {
        let mut receipt = SupportUpdateReceipt {
            checkpoint_sequence: 1,
            base_support_sha256: possible_world_support_sha256(before).expect("before digest"),
            resulting_support_sha256: possible_world_support_sha256(after).expect("after digest"),
            evidence_artifact_sha256: evidence.artifact_sha256.clone(),
            evidence_prefix_sha256: evidence_prefix_commitment(evidence)
                .expect("evidence prefix commitment"),
            oracle_profile_sha256: digest('3'),
            derivation_receipt_sha256: String::new(),
            derivation_complete: true,
            open_world_scope_closure_receipt_sha256: None,
        };
        receipt.derivation_receipt_sha256 =
            support_update_commitment(&receipt).expect("support receipt commitment");
        receipt
    }

    #[test]
    fn continues_only_with_exact_checkpoint_bound_physical_horizon() {
        let input = checkpoint();
        let result = evaluate_checkpoint(&initial_cursor(), &input).expect("checkpoint");
        assert_eq!(result.action, CheckpointAction::ContinueProbe);
        assert!(!result.new_effect_permitted);
    }

    #[test]
    fn unknown_safe_horizon_is_not_treated_as_infinite() {
        let mut input = checkpoint();
        input.physical_path_receipts[0].safe_horizon_ns = None;
        assert_eq!(
            evaluate_checkpoint(&initial_cursor(), &input)
                .expect("checkpoint")
                .action,
            CheckpointAction::NoSafeContinuation
        );
    }

    #[test]
    fn expired_continuation_selects_only_a_qualified_abort_path() {
        let mut input = checkpoint();
        input.physical_path_receipts = vec![
            path(
                PhysicalPathKind::ContinueProbe,
                SafetyQualification::Qualified,
                80,
                Some(20),
            ),
            path(
                PhysicalPathKind::AbortProbe,
                SafetyQualification::Qualified,
                10,
                Some(30),
            ),
        ];
        assert_eq!(
            evaluate_checkpoint(&initial_cursor(), &input)
                .expect("checkpoint")
                .action,
            CheckpointAction::AbortProbe
        );
    }

    #[test]
    fn decision_sufficiency_stops_sensing_when_safe_stop_is_qualified() {
        let mut input = checkpoint();
        let support = initial_cursor().possible_world_support_ids;
        input.decision_sufficiency_receipt = Some(decision_receipt(&support, 1));
        input.physical_path_receipts.push(path(
            PhysicalPathKind::SafeStopHold,
            SafetyQualification::Qualified,
            15,
            Some(30),
        ));
        let result = evaluate_checkpoint(&initial_cursor(), &input).expect("checkpoint");
        assert!(result.decision_sufficient);
        assert_eq!(result.action, CheckpointAction::StopSensingHold);
    }

    #[test]
    fn decision_sufficiency_uses_qualified_abort_when_safe_hold_is_unavailable() {
        let mut input = checkpoint();
        let support = initial_cursor().possible_world_support_ids;
        input.decision_sufficiency_receipt = Some(decision_receipt(&support, 1));
        input.physical_path_receipts.push(path(
            PhysicalPathKind::AbortProbe,
            SafetyQualification::Qualified,
            15,
            Some(30),
        ));
        assert_eq!(
            evaluate_checkpoint(&initial_cursor(), &input)
                .expect("checkpoint")
                .action,
            CheckpointAction::AbortProbe
        );
    }

    #[test]
    fn admitted_positive_prefix_is_labeled_only_for_its_subscope() {
        let result = evaluate_checkpoint(&initial_cursor(), &checkpoint()).expect("checkpoint");
        assert_eq!(
            result.partial_evidence_disposition,
            PartialEvidenceDisposition::AdmittedForSubscope
        );
    }

    #[test]
    fn partial_absence_cannot_be_promoted_to_full_interval_absence() {
        let mut input = checkpoint();
        input.evidence_prefix = Some(prefix(
            ClaimSemantics::AbsenceOverInterval,
            EvidenceQuality::Admitted,
            15,
        ));
        let result = evaluate_checkpoint(&initial_cursor(), &input).expect("checkpoint");
        assert_eq!(
            result.partial_evidence_disposition,
            PartialEvidenceDisposition::InsufficientForDeclaredScope
        );
        assert_eq!(
            result.possible_world_support_ids,
            initial_cursor().possible_world_support_ids
        );
    }

    #[test]
    fn insufficient_partial_absence_cannot_narrow_possible_world_support() {
        let cursor = initial_cursor();
        let mut input = checkpoint();
        let evidence = prefix(
            ClaimSemantics::AbsenceOverInterval,
            EvidenceQuality::Admitted,
            15,
        );
        input.evidence_prefix = Some(evidence);
        input.proposed_possible_world_support_ids = vec!["world-a".to_owned()];
        input.support_update_receipt = Some(support_receipt(
            &cursor.possible_world_support_ids,
            &input.proposed_possible_world_support_ids,
            input.evidence_prefix.as_ref().expect("prefix"),
        ));
        assert!(matches!(
            evaluate_checkpoint(&cursor, &input),
            Err(CheckpointError::InvalidSupportTransition(_))
        ));
    }

    #[test]
    fn stale_or_disputed_prefix_cannot_narrow_possible_world_support() {
        let cursor = initial_cursor();
        let mut input = checkpoint();
        let evidence = prefix(ClaimSemantics::PositiveEvent, EvidenceQuality::Disputed, 15);
        input.evidence_prefix = Some(evidence);
        input.proposed_possible_world_support_ids = vec!["world-a".to_owned()];
        input.support_update_receipt = Some(support_receipt(
            &cursor.possible_world_support_ids,
            &input.proposed_possible_world_support_ids,
            input.evidence_prefix.as_ref().expect("prefix"),
        ));
        assert!(matches!(
            evaluate_checkpoint(&cursor, &input),
            Err(CheckpointError::InvalidSupportTransition(_))
        ));
    }

    #[test]
    fn admitted_prefix_can_narrow_support_only_with_exact_derivation_receipt() {
        let cursor = initial_cursor();
        let mut input = checkpoint();
        let evidence = prefix(ClaimSemantics::PositiveEvent, EvidenceQuality::Admitted, 15);
        input.evidence_prefix = Some(evidence);
        input.proposed_possible_world_support_ids = vec!["world-a".to_owned()];
        input.support_update_receipt = Some(support_receipt(
            &cursor.possible_world_support_ids,
            &input.proposed_possible_world_support_ids,
            input.evidence_prefix.as_ref().expect("prefix"),
        ));
        let result = evaluate_checkpoint(&cursor, &input).expect("qualified support update");
        assert_eq!(
            result.possible_world_support_ids,
            vec!["world-a".to_owned()]
        );
    }

    #[test]
    fn support_narrowing_without_derivation_receipt_is_rejected() {
        let mut input = checkpoint();
        input.proposed_possible_world_support_ids = vec!["world-a".to_owned()];
        assert!(matches!(
            evaluate_checkpoint(&initial_cursor(), &input),
            Err(CheckpointError::InvalidSupportReceipt(_))
        ));
    }

    #[test]
    fn support_cannot_widen_inside_the_partial_observation_reducer() {
        let mut input = checkpoint();
        input.proposed_possible_world_support_ids = vec![
            "world-a".to_owned(),
            "world-b".to_owned(),
            "world-c".to_owned(),
        ];
        assert!(matches!(
            evaluate_checkpoint(&initial_cursor(), &input),
            Err(CheckpointError::InvalidSupportTransition(_))
        ));
    }

    #[test]
    fn incomplete_prefix_with_wrong_clock_profile_is_rejected() {
        let mut input = checkpoint();
        input
            .evidence_prefix
            .as_mut()
            .expect("prefix")
            .clock_mapping_profile_sha256 = digest('9');
        assert!(matches!(
            evaluate_checkpoint(&initial_cursor(), &input),
            Err(CheckpointError::InvalidScope(_))
        ));
    }

    #[test]
    fn replayed_or_out_of_order_checkpoint_cannot_advance_cursor() {
        let cursor = initial_cursor();
        let result = evaluate_checkpoint(&cursor, &checkpoint()).expect("first checkpoint");
        assert_eq!(
            evaluate_checkpoint(&result.next_cursor, &checkpoint()),
            Err(CheckpointError::SequenceMismatch)
        );
    }

    #[test]
    fn checkpoint_must_chain_to_the_exact_previous_digest() {
        let mut cursor = initial_cursor();
        cursor.last_sequence = 1;
        cursor.last_checkpoint_sha256 = Some(digest('8'));
        let mut input = checkpoint();
        input.sequence = 2;
        input.previous_checkpoint_sha256 = Some(digest('7'));
        assert_eq!(
            evaluate_checkpoint(&cursor, &input),
            Err(CheckpointError::PreviousCheckpointMismatch)
        );
    }

    #[test]
    fn path_receipt_from_another_checkpoint_cannot_authorize_continuation() {
        let mut input = checkpoint();
        input.physical_path_receipts[0].checkpoint_sequence = 2;
        assert_eq!(
            evaluate_checkpoint(&initial_cursor(), &input)
                .expect("checkpoint")
                .action,
            CheckpointAction::NoSafeContinuation
        );
    }

    #[test]
    fn completed_probe_stops_sensing_but_never_authorizes_a_new_effect() {
        let mut input = checkpoint();
        input.lifecycle = ProbeLifecycle::Completed;
        input.physical_path_receipts.push(path(
            PhysicalPathKind::SafeStopHold,
            SafetyQualification::Qualified,
            10,
            Some(30),
        ));
        let result = evaluate_checkpoint(&initial_cursor(), &input).expect("checkpoint");
        assert_eq!(result.action, CheckpointAction::StopSensingHold);
        assert!(!result.new_effect_permitted);
    }

    #[test]
    fn nonsemantic_path_receipt_order_does_not_change_checkpoint_identity() {
        let mut first = checkpoint();
        first.physical_path_receipts.push(path(
            PhysicalPathKind::AbortProbe,
            SafetyQualification::Qualified,
            10,
            Some(30),
        ));
        let mut second = first.clone();
        second.physical_path_receipts.reverse();
        let first_result = evaluate_checkpoint(&initial_cursor(), &first).expect("first");
        let second_result = evaluate_checkpoint(&initial_cursor(), &second).expect("second");
        assert_eq!(
            first_result.next_cursor.last_checkpoint_sha256,
            second_result.next_cursor.last_checkpoint_sha256
        );
    }

    #[test]
    fn duplicate_path_kinds_are_rejected_not_resolved_by_input_order() {
        let mut input = checkpoint();
        input
            .physical_path_receipts
            .push(input.physical_path_receipts[0].clone());
        assert_eq!(
            evaluate_checkpoint(&initial_cursor(), &input),
            Err(CheckpointError::InvalidPathReceipt("duplicate path kind"))
        );
    }

    #[test]
    fn decision_receipt_for_another_support_set_is_rejected() {
        let mut input = checkpoint();
        input.decision_sufficiency_receipt =
            Some(decision_receipt(&["different-world".to_owned()], 1));
        assert_eq!(
            evaluate_checkpoint(&initial_cursor(), &input),
            Err(CheckpointError::InvalidDecisionReceipt(
                "support digest mismatch"
            ))
        );
    }

    #[test]
    fn safe_stop_with_unknown_horizon_is_not_admitted() {
        let mut input = checkpoint();
        input.physical_path_receipts.push(path(
            PhysicalPathKind::SafeStopHold,
            SafetyQualification::Qualified,
            1,
            None,
        ));
        let support = initial_cursor().possible_world_support_ids;
        input.decision_sufficiency_receipt = Some(decision_receipt(&support, 1));
        assert_eq!(
            evaluate_checkpoint(&initial_cursor(), &input)
                .expect("checkpoint")
                .action,
            CheckpointAction::NoSafeContinuation
        );
    }

    #[test]
    fn complete_absence_scope_can_be_admitted_without_inventing_a_suffix() {
        let mut input = checkpoint();
        input.evidence_prefix = Some(prefix(
            ClaimSemantics::AbsenceOverInterval,
            EvidenceQuality::Admitted,
            20,
        ));
        let result = evaluate_checkpoint(&initial_cursor(), &input).expect("checkpoint");
        assert_eq!(
            result.partial_evidence_disposition,
            PartialEvidenceDisposition::AdmittedForDeclaredScope
        );
    }

    #[test]
    fn incomplete_decision_oracle_cannot_prove_decision_sufficiency() {
        let mut input = checkpoint();
        let support = initial_cursor().possible_world_support_ids;
        let mut receipt = decision_receipt(&support, 1);
        receipt.oracle_complete = false;
        receipt.receipt_sha256.clear();
        receipt.receipt_sha256 =
            decision_sufficiency_commitment(&receipt).expect("resealed receipt");
        input.decision_sufficiency_receipt = Some(receipt);
        let result = evaluate_checkpoint(&initial_cursor(), &input).expect("checkpoint");
        assert!(!result.decision_sufficient);
        assert_eq!(result.action, CheckpointAction::ContinueProbe);
    }

    #[test]
    fn incomplete_physical_trajectory_cannot_qualify_continuation() {
        let mut input = checkpoint();
        let mut receipt = input.physical_path_receipts[0].clone();
        receipt.full_trajectory_covered = false;
        receipt.receipt_sha256.clear();
        receipt.receipt_sha256 = physical_path_commitment(&receipt).expect("resealed path");
        input.physical_path_receipts[0] = receipt;
        assert_eq!(
            evaluate_checkpoint(&initial_cursor(), &input)
                .expect("checkpoint")
                .action,
            CheckpointAction::NoSafeContinuation
        );
    }

    #[test]
    fn changed_horizon_without_new_receipt_commitment_is_rejected() {
        let mut input = checkpoint();
        input.physical_path_receipts[0].safe_horizon_ns = Some(1_000_000);
        assert_eq!(
            evaluate_checkpoint(&initial_cursor(), &input),
            Err(CheckpointError::InvalidPathReceipt(
                "plan binding or receipt commitment mismatch"
            ))
        );
    }

    #[test]
    fn open_world_residual_cannot_be_removed_by_partial_prefix() {
        let mut cursor = initial_cursor();
        cursor.possible_world_support_ids = vec![
            super::super::OPEN_WORLD_RESIDUAL.to_owned(),
            "world-a".to_owned(),
        ];
        let mut input = checkpoint();
        input.proposed_possible_world_support_ids = vec!["world-a".to_owned()];
        let evidence = prefix(ClaimSemantics::PositiveEvent, EvidenceQuality::Admitted, 15);
        input.evidence_prefix = Some(evidence.clone());
        let mut receipt = support_receipt(
            &cursor.possible_world_support_ids,
            &input.proposed_possible_world_support_ids,
            &evidence,
        );
        receipt.open_world_scope_closure_receipt_sha256 = Some(digest('5'));
        receipt.derivation_receipt_sha256.clear();
        receipt.derivation_receipt_sha256 =
            support_update_commitment(&receipt).expect("resealed receipt");
        input.support_update_receipt = Some(receipt);
        assert_eq!(
            evaluate_checkpoint(&cursor, &input),
            Err(CheckpointError::InvalidSupportReceipt(
                "open-world residual cannot be removed by partial scope"
            ))
        );
    }

    #[test]
    fn open_world_residual_requires_explicit_full_scope_closure_receipt() {
        let mut cursor = initial_cursor();
        cursor.possible_world_support_ids = vec![
            super::super::OPEN_WORLD_RESIDUAL.to_owned(),
            "world-a".to_owned(),
        ];
        let mut input = checkpoint();
        input.proposed_possible_world_support_ids = vec!["world-a".to_owned()];
        let evidence = prefix(
            ClaimSemantics::GeneralMeasurement,
            EvidenceQuality::Admitted,
            20,
        );
        input.evidence_prefix = Some(evidence.clone());
        let mut receipt = support_receipt(
            &cursor.possible_world_support_ids,
            &input.proposed_possible_world_support_ids,
            &evidence,
        );
        input.support_update_receipt = Some(receipt.clone());
        assert_eq!(
            evaluate_checkpoint(&cursor, &input),
            Err(CheckpointError::InvalidSupportReceipt(
                "open-world residual removal requires scope-closure receipt"
            ))
        );

        receipt.open_world_scope_closure_receipt_sha256 = Some(digest('5'));
        receipt.derivation_receipt_sha256.clear();
        receipt.derivation_receipt_sha256 =
            support_update_commitment(&receipt).expect("resealed receipt");
        input.support_update_receipt = Some(receipt);
        let result = evaluate_checkpoint(&cursor, &input).expect("qualified full-scope closure");
        assert_eq!(
            result.possible_world_support_ids,
            vec!["world-a".to_owned()]
        );
    }

    #[test]
    fn incomplete_support_derivation_cannot_narrow_world_support() {
        let cursor = initial_cursor();
        let mut input = checkpoint();
        let evidence = prefix(ClaimSemantics::PositiveEvent, EvidenceQuality::Admitted, 15);
        input.evidence_prefix = Some(evidence.clone());
        input.proposed_possible_world_support_ids = vec!["world-a".to_owned()];
        let mut receipt = support_receipt(
            &cursor.possible_world_support_ids,
            &input.proposed_possible_world_support_ids,
            &evidence,
        );
        receipt.derivation_complete = false;
        receipt.derivation_receipt_sha256.clear();
        receipt.derivation_receipt_sha256 =
            support_update_commitment(&receipt).expect("resealed receipt");
        input.support_update_receipt = Some(receipt);
        assert_eq!(
            evaluate_checkpoint(&cursor, &input),
            Err(CheckpointError::InvalidSupportReceipt(
                "derivation coverage incomplete"
            ))
        );
    }

    #[test]
    fn incomplete_coverage_cannot_narrow_possible_world_support() {
        let cursor = initial_cursor();
        let mut input = checkpoint();
        let mut evidence = prefix(ClaimSemantics::PositiveEvent, EvidenceQuality::Admitted, 15);
        evidence.coverage_complete = false;
        input.evidence_prefix = Some(evidence.clone());
        input.proposed_possible_world_support_ids = vec!["world-a".to_owned()];
        input.support_update_receipt = Some(support_receipt(
            &cursor.possible_world_support_ids,
            &input.proposed_possible_world_support_ids,
            &evidence,
        ));
        let result = evaluate_checkpoint(&cursor, &input);
        assert!(matches!(
            result,
            Err(CheckpointError::InvalidSupportTransition(_))
        ));
    }

    #[test]
    fn evidence_prefix_metadata_is_bound_into_support_update_receipt() {
        let cursor = initial_cursor();
        let mut input = checkpoint();
        let evidence = prefix(ClaimSemantics::PositiveEvent, EvidenceQuality::Admitted, 15);
        input.evidence_prefix = Some(evidence.clone());
        input.proposed_possible_world_support_ids = vec!["world-a".to_owned()];
        input.support_update_receipt = Some(support_receipt(
            &cursor.possible_world_support_ids,
            &input.proposed_possible_world_support_ids,
            &evidence,
        ));
        input
            .evidence_prefix
            .as_mut()
            .expect("prefix")
            .coverage_end_ns = 14;
        assert_eq!(
            evaluate_checkpoint(&cursor, &input),
            Err(CheckpointError::InvalidSupportReceipt(
                "receipt does not bind exact support and evidence"
            ))
        );
    }

    #[test]
    fn completed_probe_without_qualified_hold_path_is_not_reported_as_safe_hold() {
        let mut input = checkpoint();
        input.lifecycle = ProbeLifecycle::Completed;
        let result = evaluate_checkpoint(&initial_cursor(), &input).expect("checkpoint");
        assert_eq!(result.action, CheckpointAction::NoSafeContinuation);
        assert!(!result.new_effect_permitted);
    }

    #[test]
    fn safe_hold_is_preferred_to_abort_when_continuation_is_unavailable() {
        let mut input = checkpoint();
        input.physical_path_receipts = vec![
            path(
                PhysicalPathKind::SafeStopHold,
                SafetyQualification::Qualified,
                10,
                Some(30),
            ),
            path(
                PhysicalPathKind::AbortProbe,
                SafetyQualification::Qualified,
                10,
                Some(30),
            ),
        ];
        assert_eq!(
            evaluate_checkpoint(&initial_cursor(), &input)
                .expect("checkpoint")
                .action,
            CheckpointAction::StopSensingHold
        );
    }

    #[test]
    fn finite_lifecycle_oracle_matches_all_48_path_and_decision_combinations() {
        let path_kinds = [
            PhysicalPathKind::ContinueProbe,
            PhysicalPathKind::SafeStopHold,
            PhysicalPathKind::AbortProbe,
        ];
        for lifecycle in [
            ProbeLifecycle::Acquiring,
            ProbeLifecycle::Interrupted,
            ProbeLifecycle::Completed,
        ] {
            for decision_sufficient in [false, true] {
                for mask in 0u8..8 {
                    let mut input = checkpoint();
                    input.lifecycle = lifecycle;
                    input.physical_path_receipts.clear();
                    for (bit, kind) in path_kinds.iter().enumerate() {
                        if mask & (1 << bit) != 0 {
                            input.physical_path_receipts.push(path(
                                *kind,
                                SafetyQualification::Qualified,
                                10,
                                Some(20),
                            ));
                        }
                    }
                    if decision_sufficient {
                        let support = initial_cursor().possible_world_support_ids;
                        input.decision_sufficiency_receipt = Some(decision_receipt(&support, 1));
                    }

                    let can_continue = mask & 0b001 != 0;
                    let can_stop = mask & 0b010 != 0;
                    let can_abort = mask & 0b100 != 0;
                    let expected = if lifecycle == ProbeLifecycle::Completed {
                        if can_stop {
                            CheckpointAction::StopSensingHold
                        } else {
                            CheckpointAction::NoSafeContinuation
                        }
                    } else if decision_sufficient {
                        if can_stop {
                            CheckpointAction::StopSensingHold
                        } else if can_abort {
                            CheckpointAction::AbortProbe
                        } else {
                            CheckpointAction::NoSafeContinuation
                        }
                    } else if can_continue && lifecycle == ProbeLifecycle::Acquiring {
                        CheckpointAction::ContinueProbe
                    } else if can_stop {
                        CheckpointAction::StopSensingHold
                    } else if can_abort {
                        CheckpointAction::AbortProbe
                    } else {
                        CheckpointAction::NoSafeContinuation
                    };

                    let result =
                        evaluate_checkpoint(&initial_cursor(), &input).expect("finite oracle case");
                    assert_eq!(
                        result.action, expected,
                        "lifecycle={lifecycle:?}, decision_sufficient={decision_sufficient}, path_mask={mask:03b}"
                    );
                    assert!(!result.new_effect_permitted);
                }
            }
        }
    }

    #[test]
    fn finite_partial_evidence_oracle_matches_all_48_scope_quality_combinations() {
        for semantics in [
            ClaimSemantics::PositiveEvent,
            ClaimSemantics::GeneralMeasurement,
            ClaimSemantics::AbsenceOverInterval,
        ] {
            for quality in [
                EvidenceQuality::Admitted,
                EvidenceQuality::Stale,
                EvidenceQuality::Unknown,
                EvidenceQuality::Disputed,
            ] {
                for coverage_complete in [false, true] {
                    for full_scope in [false, true] {
                        let mut evidence =
                            prefix(semantics, quality, if full_scope { 20 } else { 15 });
                        evidence.coverage_complete = coverage_complete;
                        let expected = if quality != EvidenceQuality::Admitted {
                            PartialEvidenceDisposition::Unqualified
                        } else if !coverage_complete {
                            PartialEvidenceDisposition::InsufficientForDeclaredScope
                        } else if full_scope {
                            PartialEvidenceDisposition::AdmittedForDeclaredScope
                        } else if semantics == ClaimSemantics::AbsenceOverInterval {
                            PartialEvidenceDisposition::InsufficientForDeclaredScope
                        } else {
                            PartialEvidenceDisposition::AdmittedForSubscope
                        };
                        assert_eq!(
                            classify_partial_evidence(
                                &checkpoint().requested_scope,
                                Some(&evidence)
                            )
                            .expect("finite evidence oracle"),
                            expected,
                            "semantics={semantics:?}, quality={quality:?}, complete={coverage_complete}, full={full_scope}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn support_receipt_rejects_source_and_calibration_metadata_changes() {
        for mutate in [0_u8, 1_u8] {
            let cursor = initial_cursor();
            let mut input = checkpoint();
            let evidence = prefix(ClaimSemantics::PositiveEvent, EvidenceQuality::Admitted, 15);
            input.evidence_prefix = Some(evidence.clone());
            input.proposed_possible_world_support_ids = vec!["world-a".to_owned()];
            input.support_update_receipt = Some(support_receipt(
                &cursor.possible_world_support_ids,
                &input.proposed_possible_world_support_ids,
                &evidence,
            ));
            let prefix = input.evidence_prefix.as_mut().expect("prefix");
            if mutate == 0 {
                prefix.source_id = "substituted-sensor".to_owned();
            } else {
                prefix.calibration_profile_sha256 = digest('9');
            }
            assert_eq!(
                evaluate_checkpoint(&cursor, &input),
                Err(CheckpointError::InvalidSupportReceipt(
                    "receipt does not bind exact support and evidence"
                ))
            );
        }
    }

    #[test]
    fn mandatory_abort_overrides_a_qualified_hold_path() {
        let mut input = checkpoint();
        let mut hold = path(
            PhysicalPathKind::SafeStopHold,
            SafetyQualification::Qualified,
            10,
            Some(30),
        );
        hold.required_by_safety_supervisor = false;
        hold.receipt_sha256.clear();
        hold.receipt_sha256 = physical_path_commitment(&hold).expect("hold commitment");
        let mut abort = path(
            PhysicalPathKind::AbortProbe,
            SafetyQualification::Qualified,
            10,
            Some(30),
        );
        abort.required_by_safety_supervisor = true;
        abort.receipt_sha256.clear();
        abort.receipt_sha256 = physical_path_commitment(&abort).expect("abort commitment");
        input.physical_path_receipts = vec![hold, abort];
        assert_eq!(
            evaluate_checkpoint(&initial_cursor(), &input)
                .expect("checkpoint")
                .action,
            CheckpointAction::AbortProbe
        );
    }

    #[test]
    fn unqualified_mandatory_abort_does_not_fall_back_to_hold() {
        let mut input = checkpoint();
        let mut hold = path(
            PhysicalPathKind::SafeStopHold,
            SafetyQualification::Qualified,
            10,
            Some(30),
        );
        hold.receipt_sha256.clear();
        hold.receipt_sha256 = physical_path_commitment(&hold).expect("hold commitment");
        let mut abort = path(
            PhysicalPathKind::AbortProbe,
            SafetyQualification::Unknown,
            10,
            None,
        );
        abort.required_by_safety_supervisor = true;
        abort.receipt_sha256.clear();
        abort.receipt_sha256 = physical_path_commitment(&abort).expect("abort commitment");
        input.physical_path_receipts = vec![hold, abort];
        assert_eq!(
            evaluate_checkpoint(&initial_cursor(), &input)
                .expect("checkpoint")
                .action,
            CheckpointAction::NoSafeContinuation
        );
    }

    #[test]
    fn conflicting_mandatory_supervisor_paths_are_rejected() {
        let mut input = checkpoint();
        let mut hold = path(
            PhysicalPathKind::SafeStopHold,
            SafetyQualification::Qualified,
            10,
            Some(30),
        );
        hold.required_by_safety_supervisor = true;
        hold.receipt_sha256.clear();
        hold.receipt_sha256 = physical_path_commitment(&hold).expect("hold commitment");
        let mut abort = path(
            PhysicalPathKind::AbortProbe,
            SafetyQualification::Qualified,
            10,
            Some(30),
        );
        abort.required_by_safety_supervisor = true;
        abort.receipt_sha256.clear();
        abort.receipt_sha256 = physical_path_commitment(&abort).expect("abort commitment");
        input.physical_path_receipts = vec![hold, abort];
        assert_eq!(
            evaluate_checkpoint(&initial_cursor(), &input),
            Err(CheckpointError::InvalidPathReceipt(
                "conflicting mandatory supervisor paths"
            ))
        );
    }
}
