// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Multi-attempt physical-effect lineage validation.
//!
//! The runtime projection v0.1 covers one original attempt and one recovery
//! admission. This module validates a longer, linear attempt history without
//! issuing recovery authority or replacing any owner of persistence, mission,
//! queue, observer, or native permit state. Every typed outcome is bound to
//! the same existing OwnerEvidenceRef that carries its owner, verdict, scope,
//! asserted value and raw-artifact commitment.

use crate::{
    EvidenceScope, ObserverQuorumProof, OwnerEvidenceDomain, OwnerEvidenceRef,
    OwnerEvidenceStatus, claim_scope_digest, observer_proofs_are_independent,
    observer_quorum_is_qualified,
};
use serde::Deserialize;
use std::collections::BTreeSet;

pub const ATTEMPT_LINEAGE_VERSION: &str = "0.1";
pub const MAX_ATTEMPTS: usize = 64;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceOutcome {
    DefinitelyNotCommitted,
    Committed,
    InFlight,
    IndeterminateCommit,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalOutcome {
    NotObserved,
    Applied,
    NoEffect,
    Partial,
    Unknown,
    Disputed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MissionOutcome {
    NoEffect,
    EffectPredicateSatisfied,
    Unresolved,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FenceOutcome {
    Quiescent,
    Active,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EpochPair {
    pub boot_epoch: u64,
    pub controller_epoch: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PersistenceEvidence {
    pub outcome: PersistenceOutcome,
    pub receipt: OwnerEvidenceRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PhysicalOutcomeEvidence {
    pub outcome: PhysicalOutcome,
    pub receipt: OwnerEvidenceRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MissionResolutionEvidence {
    pub outcome: MissionOutcome,
    pub receipt: OwnerEvidenceRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FenceEvidence {
    pub outcome: FenceOutcome,
    /// The new boot/controller epoch for which the old path is quiescent.
    /// It is null when the evidence does not assert quiescence for a child.
    pub target_epoch: Option<EpochPair>,
    pub receipt: OwnerEvidenceRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AttemptObserverAssurance {
    /// Existing owner-issued observer-quorum evidence for the typed physical outcome.
    pub physical_outcome: OwnerEvidenceRef,
    /// Required when this attempt is a parent of a retry edge.
    pub downstream_quiescence: Option<OwnerEvidenceRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AttemptRecord {
    pub attempt_id: String,
    pub ordinal: u32,
    pub parent_attempt_id: Option<String>,
    pub authorized_epoch: EpochPair,
    pub persistence: PersistenceEvidence,
    pub physical_outcome: PhysicalOutcomeEvidence,
    pub mission_resolution: MissionResolutionEvidence,
    pub controller_queue_fence: FenceEvidence,
    pub downstream_quiescence: FenceEvidence,
    /// Required when this attempt is used as the parent of a retry edge.
    pub observer_assurance: Option<AttemptObserverAssurance>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AttemptLineageRecording {
    pub version: String,
    pub logical_effect_id: String,
    pub fault_profile_revision: String,
    pub attempts: Vec<AttemptRecord>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineageError {
    InvalidHeader,
    EmptyAttemptChain,
    TooManyAttempts,
    InvalidAttemptIdentity,
    DuplicateAttemptIdentity,
    OrdinalDiscontinuity,
    ParentLinkMismatch,
    EpochNotMonotonic,
    InvalidEvidenceReceipt,
    DuplicateEvidenceIdentity,
    EvidenceScopeMismatch,
    OutcomeVerdictMismatch,
    MissionOutcomeMismatch,
    ParentOutcomeNotRetryable,
    PersistenceUnresolved,
    ControllerFenceUnqualified,
    DownstreamFenceUnqualified,
    FenceEpochMismatch,
    MissingObserverAssurance,
    ObserverQuorumUnqualified,
    ObserverClaimMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineageSummary {
    pub attempt_count: usize,
    pub recovery_edge_count: usize,
    pub applied_attempt_count: usize,
    pub head_attempt_id: String,
    pub head_outcome: PhysicalOutcome,
    pub head_outcome_status: OwnerEvidenceStatus,
    pub head_physical_observer_qualified: bool,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn persistence_value(outcome: PersistenceOutcome) -> &'static str {
    match outcome {
        PersistenceOutcome::DefinitelyNotCommitted => "definitely_not_committed",
        PersistenceOutcome::Committed => "committed",
        PersistenceOutcome::InFlight => "in_flight",
        PersistenceOutcome::IndeterminateCommit => "indeterminate_commit",
    }
}

fn physical_value(outcome: PhysicalOutcome) -> &'static str {
    match outcome {
        PhysicalOutcome::NotObserved => "not_observed",
        PhysicalOutcome::Applied => "applied",
        PhysicalOutcome::NoEffect => "no_effect",
        PhysicalOutcome::Partial => "partial",
        PhysicalOutcome::Unknown => "unknown",
        PhysicalOutcome::Disputed => "disputed",
    }
}

fn mission_value(outcome: MissionOutcome) -> &'static str {
    match outcome {
        MissionOutcome::NoEffect => "no_effect",
        MissionOutcome::EffectPredicateSatisfied => "effect_predicate_satisfied",
        MissionOutcome::Unresolved => "unresolved",
    }
}

fn fence_value(outcome: FenceOutcome, target_epoch: Option<EpochPair>) -> String {
    match target_epoch {
        Some(target) => format!(
            "{};target_boot_epoch={};target_controller_epoch={}",
            match outcome {
                FenceOutcome::Quiescent => "quiescent",
                FenceOutcome::Active => "active",
                FenceOutcome::Unknown => "unknown",
            },
            target.boot_epoch,
            target.controller_epoch
        ),
        None => format!(
            "{};target_epoch=none",
            match outcome {
                FenceOutcome::Quiescent => "quiescent",
                FenceOutcome::Active => "active",
                FenceOutcome::Unknown => "unknown",
            }
        ),
    }
}

fn evidence_scope(logical_effect_id: &str, attempt: &AttemptRecord) -> EvidenceScope {
    EvidenceScope {
        logical_effect_id: logical_effect_id.to_owned(),
        attempt_id: attempt.attempt_id.clone(),
        subject_boot_epoch: attempt.authorized_epoch.boot_epoch,
        subject_controller_epoch: attempt.authorized_epoch.controller_epoch,
    }
}

fn validate_owner_evidence(
    evidence: &OwnerEvidenceRef,
    expected_domain: OwnerEvidenceDomain,
    expected_asserted_value: &str,
    expected_scope: &EvidenceScope,
    evidence_ids: &mut BTreeSet<String>,
) -> Result<(), LineageError> {
    if evidence.domain != expected_domain
        || evidence.asserted_value != expected_asserted_value
        || evidence.scope != *expected_scope
        || !nonempty(&evidence.owner_id)
        || !nonempty(&evidence.owner_revision)
        || !nonempty(&evidence.evidence_id)
        || !nonempty(&evidence.raw_artifact_id)
        || !valid_sha256(&evidence.raw_artifact_sha256)
        || evidence.claim_id.is_some()
        || evidence.claim_scope_sha256.is_some()
        || evidence.observer_quorum.is_some()
        || matches!(
            evidence.status,
            OwnerEvidenceStatus::NotApplicable
                | OwnerEvidenceStatus::QuorumQualified
                | OwnerEvidenceStatus::QuorumNotQualified
        )
    {
        return Err(LineageError::InvalidEvidenceReceipt);
    }

    if !evidence_ids.insert(evidence.evidence_id.clone()) {
        return Err(LineageError::DuplicateEvidenceIdentity);
    }
    Ok(())
}

fn validate_attempt_evidence(
    logical_effect_id: &str,
    attempt: &AttemptRecord,
    evidence_ids: &mut BTreeSet<String>,
) -> Result<(), LineageError> {
    let scope = evidence_scope(logical_effect_id, attempt);

    validate_owner_evidence(
        &attempt.persistence.receipt,
        OwnerEvidenceDomain::Persistence,
        persistence_value(attempt.persistence.outcome),
        &scope,
        evidence_ids,
    )?;
    validate_owner_evidence(
        &attempt.physical_outcome.receipt,
        OwnerEvidenceDomain::PhysicalOutcome,
        physical_value(attempt.physical_outcome.outcome),
        &scope,
        evidence_ids,
    )?;
    validate_owner_evidence(
        &attempt.mission_resolution.receipt,
        OwnerEvidenceDomain::MissionResolution,
        mission_value(attempt.mission_resolution.outcome),
        &scope,
        evidence_ids,
    )?;

    for fence in [
        &attempt.controller_queue_fence,
        &attempt.downstream_quiescence,
    ] {
        if (fence.outcome == FenceOutcome::Quiescent) != fence.target_epoch.is_some() {
            return Err(LineageError::FenceEpochMismatch);
        }
    }
    validate_owner_evidence(
        &attempt.controller_queue_fence.receipt,
        OwnerEvidenceDomain::ControllerQueue,
        &fence_value(
            attempt.controller_queue_fence.outcome,
            attempt.controller_queue_fence.target_epoch,
        ),
        &scope,
        evidence_ids,
    )?;
    validate_owner_evidence(
        &attempt.downstream_quiescence.receipt,
        OwnerEvidenceDomain::DownstreamExecutionGraph,
        &fence_value(
            attempt.downstream_quiescence.outcome,
            attempt.downstream_quiescence.target_epoch,
        ),
        &scope,
        evidence_ids,
    )?;

    if attempt.physical_outcome.outcome == PhysicalOutcome::Applied
        && attempt.physical_outcome.receipt.status != OwnerEvidenceStatus::Confirmed
    {
        return Err(LineageError::OutcomeVerdictMismatch);
    }
    if attempt.physical_outcome.outcome == PhysicalOutcome::NoEffect
        && attempt.mission_resolution.outcome == MissionOutcome::EffectPredicateSatisfied
        && attempt.mission_resolution.receipt.status == OwnerEvidenceStatus::Confirmed
    {
        return Err(LineageError::MissionOutcomeMismatch);
    }

    Ok(())
}

fn validate_observer_claim<'a>(
    evidence: &'a OwnerEvidenceRef,
    expected_domain: OwnerEvidenceDomain,
    expected_asserted_value: &str,
    expected_scope: &EvidenceScope,
    fault_profile_revision: &str,
    evidence_ids: &mut BTreeSet<String>,
    claim_ids: &mut BTreeSet<String>,
    artifact_ids: &mut BTreeSet<String>,
    artifact_hashes: &mut BTreeSet<String>,
    observation_ids: &mut BTreeSet<String>,
) -> Result<&'a ObserverQuorumProof, LineageError> {
    let claim_id = evidence
        .claim_id
        .as_deref()
        .filter(|value| nonempty(value))
        .ok_or(LineageError::ObserverClaimMismatch)?;
    let claim_scope_sha256 = evidence
        .claim_scope_sha256
        .as_deref()
        .ok_or(LineageError::ObserverClaimMismatch)?;
    let quorum = evidence
        .observer_quorum
        .as_ref()
        .ok_or(LineageError::ObserverQuorumUnqualified)?;

    let expected_claim_scope_sha256 = claim_scope_digest(
        expected_domain,
        claim_id,
        expected_asserted_value,
        expected_scope,
    );
    if evidence.domain != expected_domain
        || evidence.status != OwnerEvidenceStatus::QuorumQualified
        || evidence.asserted_value != expected_asserted_value
        || evidence.scope != *expected_scope
        || !nonempty(&evidence.owner_id)
        || !nonempty(&evidence.owner_revision)
        || !nonempty(&evidence.evidence_id)
        || !nonempty(&evidence.raw_artifact_id)
        || !valid_sha256(&evidence.raw_artifact_sha256)
        || quorum.fault_model_revision != fault_profile_revision
        || claim_scope_sha256 != expected_claim_scope_sha256.as_str()
    {
        return Err(LineageError::ObserverClaimMismatch);
    }

    if !evidence_ids.insert(evidence.evidence_id.clone())
        || !claim_ids.insert(claim_id.to_owned())
        || !artifact_ids.insert(evidence.raw_artifact_id.clone())
        || !artifact_hashes.insert(evidence.raw_artifact_sha256.clone())
    {
        return Err(LineageError::ObserverClaimMismatch);
    }

    if !matches!(observer_quorum_is_qualified(quorum), Ok(true)) {
        return Err(LineageError::ObserverQuorumUnqualified);
    }

    for member in &quorum.members {
        if !artifact_ids.insert(member.raw_artifact_id.clone())
            || !artifact_hashes.insert(member.raw_artifact_sha256.clone())
            || !observation_ids.insert(member.observation_id.clone())
        {
            return Err(LineageError::ObserverQuorumUnqualified);
        }
    }
    Ok(quorum)
}

fn validate_attempt_observer_assurance(
    fault_profile_revision: &str,
    logical_effect_id: &str,
    attempt: &AttemptRecord,
    require_downstream_quiescence: bool,
    evidence_ids: &mut BTreeSet<String>,
    claim_ids: &mut BTreeSet<String>,
    artifact_ids: &mut BTreeSet<String>,
    artifact_hashes: &mut BTreeSet<String>,
    observation_ids: &mut BTreeSet<String>,
) -> Result<(), LineageError> {
    let assurance = attempt
        .observer_assurance
        .as_ref()
        .ok_or(LineageError::MissingObserverAssurance)?;
    let scope = evidence_scope(logical_effect_id, attempt);

    let physical = validate_observer_claim(
        &assurance.physical_outcome,
        OwnerEvidenceDomain::PhysicalOutcomeObserverQuorum,
        physical_value(attempt.physical_outcome.outcome),
        &scope,
        fault_profile_revision,
        evidence_ids,
        claim_ids,
        artifact_ids,
        artifact_hashes,
        observation_ids,
    )?;

    match &assurance.downstream_quiescence {
        Some(downstream_evidence) => {
            let downstream = validate_observer_claim(
                downstream_evidence,
                OwnerEvidenceDomain::DownstreamObserverQuorum,
                "quiescent",
                &scope,
                fault_profile_revision,
                evidence_ids,
                claim_ids,
                artifact_ids,
                artifact_hashes,
                observation_ids,
            )?;
            if !observer_proofs_are_independent(physical, downstream) {
                return Err(LineageError::ObserverQuorumUnqualified);
            }
        }
        None if require_downstream_quiescence => {
            return Err(LineageError::MissingObserverAssurance);
        }
        None => {}
    }
    Ok(())
}

fn validate_recovery_handoff(
    fault_profile_revision: &str,
    logical_effect_id: &str,
    parent: &AttemptRecord,
    child: &AttemptRecord,
    evidence_ids: &mut BTreeSet<String>,
    claim_ids: &mut BTreeSet<String>,
    artifact_ids: &mut BTreeSet<String>,
    artifact_hashes: &mut BTreeSet<String>,
    observation_ids: &mut BTreeSet<String>,
) -> Result<(), LineageError> {
    if parent.physical_outcome.outcome != PhysicalOutcome::NoEffect
        || parent.physical_outcome.receipt.status != OwnerEvidenceStatus::Confirmed
        || parent.mission_resolution.outcome != MissionOutcome::NoEffect
        || parent.mission_resolution.receipt.status != OwnerEvidenceStatus::Confirmed
    {
        return Err(LineageError::ParentOutcomeNotRetryable);
    }

    validate_attempt_observer_assurance(
        fault_profile_revision,
        logical_effect_id,
        parent,
        true,
        evidence_ids,
        claim_ids,
        artifact_ids,
        artifact_hashes,
        observation_ids,
    )?;

    if !matches!(
        parent.persistence.outcome,
        PersistenceOutcome::DefinitelyNotCommitted | PersistenceOutcome::Committed
    ) || parent.persistence.receipt.status != OwnerEvidenceStatus::Confirmed
    {
        return Err(LineageError::PersistenceUnresolved);
    }

    let target = child.authorized_epoch;
    let controller = &parent.controller_queue_fence;
    if controller.outcome != FenceOutcome::Quiescent
        || controller.receipt.status != OwnerEvidenceStatus::Confirmed
    {
        return Err(LineageError::ControllerFenceUnqualified);
    }
    if controller.target_epoch != Some(target) {
        return Err(LineageError::FenceEpochMismatch);
    }

    let downstream = &parent.downstream_quiescence;
    if downstream.outcome != FenceOutcome::Quiescent
        || downstream.receipt.status != OwnerEvidenceStatus::Confirmed
    {
        return Err(LineageError::DownstreamFenceUnqualified);
    }
    if downstream.target_epoch != Some(target) {
        return Err(LineageError::FenceEpochMismatch);
    }
    Ok(())
}

impl AttemptLineageRecording {
    /// Validate a complete, ordered attempt chain.
    ///
    /// A successful result means only that supplied owner records are
    /// internally consistent and every retry edge has the required evidence
    /// shape. It does not authenticate owners/artifacts, infer missing physical
    /// observations, or authorize a native action.
    pub fn validate(&self) -> Result<LineageSummary, LineageError> {
        if self.version != ATTEMPT_LINEAGE_VERSION
            || !nonempty(&self.logical_effect_id)
            || !nonempty(&self.fault_profile_revision)
        {
            return Err(LineageError::InvalidHeader);
        }
        if self.attempts.is_empty() {
            return Err(LineageError::EmptyAttemptChain);
        }
        if self.attempts.len() > MAX_ATTEMPTS {
            return Err(LineageError::TooManyAttempts);
        }

        let mut attempt_ids = BTreeSet::new();
        let mut evidence_ids = BTreeSet::new();
        let mut claim_ids = BTreeSet::new();
        let mut observer_artifact_ids = BTreeSet::new();
        let mut observer_artifact_hashes = BTreeSet::new();
        let mut observer_observation_ids = BTreeSet::new();
        let mut applied_attempt_count = 0usize;

        for (index, attempt) in self.attempts.iter().enumerate() {
            if !nonempty(&attempt.attempt_id)
                || attempt.authorized_epoch.boot_epoch == 0
                || attempt.authorized_epoch.controller_epoch == 0
            {
                return Err(LineageError::InvalidAttemptIdentity);
            }
            if !attempt_ids.insert(attempt.attempt_id.clone()) {
                return Err(LineageError::DuplicateAttemptIdentity);
            }
            if attempt.ordinal != index as u32 + 1 {
                return Err(LineageError::OrdinalDiscontinuity);
            }

            let expected_parent = index
                .checked_sub(1)
                .map(|parent_index| self.attempts[parent_index].attempt_id.as_str());
            if attempt.parent_attempt_id.as_deref() != expected_parent {
                return Err(LineageError::ParentLinkMismatch);
            }

            if let Some(parent_index) = index.checked_sub(1) {
                let parent = &self.attempts[parent_index];
                if attempt.authorized_epoch.boot_epoch <= parent.authorized_epoch.boot_epoch
                    || attempt.authorized_epoch.controller_epoch
                        <= parent.authorized_epoch.controller_epoch
                {
                    return Err(LineageError::EpochNotMonotonic);
                }
                validate_recovery_handoff(
                    &self.fault_profile_revision,
                    &self.logical_effect_id,
                    parent,
                    attempt,
                    &mut evidence_ids,
                    &mut claim_ids,
                    &mut observer_artifact_ids,
                    &mut observer_artifact_hashes,
                    &mut observer_observation_ids,
                )?;
            }

            validate_attempt_evidence(&self.logical_effect_id, attempt, &mut evidence_ids)?;

            if index + 1 == self.attempts.len()
                && (attempt.physical_outcome.receipt.status == OwnerEvidenceStatus::Confirmed
                    || attempt.observer_assurance.is_some())
            {
                validate_attempt_observer_assurance(
                    &self.fault_profile_revision,
                    &self.logical_effect_id,
                    attempt,
                    false,
                    &mut evidence_ids,
                    &mut claim_ids,
                    &mut observer_artifact_ids,
                    &mut observer_artifact_hashes,
                    &mut observer_observation_ids,
                )?;
            }

            if attempt.physical_outcome.outcome == PhysicalOutcome::Applied
                && attempt.physical_outcome.receipt.status == OwnerEvidenceStatus::Confirmed
            {
                applied_attempt_count += 1;
            }
        }

        if applied_attempt_count > 1 {
            return Err(LineageError::OutcomeVerdictMismatch);
        }

        let head = self.attempts.last().expect("non-empty chain checked");
        Ok(LineageSummary {
            attempt_count: self.attempts.len(),
            recovery_edge_count: self.attempts.len() - 1,
            applied_attempt_count,
            head_attempt_id: head.attempt_id.clone(),
            head_outcome: head.physical_outcome.outcome,
            head_outcome_status: head.physical_outcome.receipt.status,
            head_physical_observer_qualified: head
                .observer_assurance
                .as_ref()
                .is_some_and(|assurance| {
                    assurance.physical_outcome.status == OwnerEvidenceStatus::QuorumQualified
                }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn valid_recording() -> AttemptLineageRecording {
        serde_json::from_str(include_str!(
            "../fixtures/attempt_lineage_three_attempts_v0_1.json"
        ))
        .expect("known-answer fixture deserializes")
    }

    #[test]
    fn accepts_three_attempt_lineage_with_two_independently_scoped_recovery_edges() {
        let summary = valid_recording().validate().unwrap();
        assert_eq!(summary.attempt_count, 3);
        assert_eq!(summary.recovery_edge_count, 2);
        assert_eq!(summary.applied_attempt_count, 1);
        assert_eq!(summary.head_attempt_id, "attempt-3");
        assert_eq!(summary.head_outcome, PhysicalOutcome::Applied);
        assert_eq!(summary.head_outcome_status, OwnerEvidenceStatus::Confirmed);
        assert!(summary.head_physical_observer_qualified);
    }

    #[test]
    fn refuses_confirmed_terminal_applied_without_physical_observer_quorum() {
        let mut recording = valid_recording();
        recording.attempts[2].observer_assurance = None;
        assert_eq!(
            recording.validate(),
            Err(LineageError::MissingObserverAssurance)
        );
    }

    #[test]
    fn rejects_typed_outcome_changed_without_matching_owner_assertion() {
        let mut recording = valid_recording();
        recording.attempts[0].physical_outcome.outcome = PhysicalOutcome::Applied;
        assert_eq!(
            recording.validate(),
            Err(LineageError::InvalidEvidenceReceipt)
        );
    }

    #[test]
    fn rejects_owner_assertion_from_another_evidence_domain() {
        let mut recording = valid_recording();
        recording.attempts[0].physical_outcome.receipt.domain =
            OwnerEvidenceDomain::Persistence;
        assert_eq!(
            recording.validate(),
            Err(LineageError::InvalidEvidenceReceipt)
        );
    }

    #[test]
    fn refuses_retry_without_claim_specific_independent_observer_proofs() {
        let mut recording = valid_recording();
        recording.attempts[0].observer_assurance = None;
        assert_eq!(
            recording.validate(),
            Err(LineageError::MissingObserverAssurance)
        );
    }

    #[test]
    fn refuses_shared_common_cause_across_physical_and_downstream_claims() {
        let mut recording = valid_recording();
        let assurance = recording.attempts[0].observer_assurance.as_mut().unwrap();
        let shared_domain = assurance
            .physical_outcome
            .observer_quorum
            .as_ref()
            .unwrap()
            .members[0]
            .common_cause_domains[0]
            .clone();
        assurance
            .downstream_quiescence
            .as_mut()
            .unwrap()
            .observer_quorum
            .as_mut()
            .unwrap()
            .members[0]
            .common_cause_domains[0] = shared_domain;
        assert_eq!(
            recording.validate(),
            Err(LineageError::ObserverQuorumUnqualified)
        );
    }

    #[test]
    fn refuses_tampered_observer_claim_scope_commitment() {
        let mut recording = valid_recording();
        recording.attempts[1]
            .observer_assurance
            .as_mut()
            .unwrap()
            .physical_outcome
            .claim_scope_sha256 = Some("f".repeat(64));
        assert_eq!(
            recording.validate(),
            Err(LineageError::ObserverClaimMismatch)
        );
    }

    #[test]
    fn refuses_replayed_observer_owner_receipt_from_another_attempt() {
        let mut recording = valid_recording();
        let old_attempt_receipt = recording.attempts[0]
            .observer_assurance
            .as_ref()
            .unwrap()
            .physical_outcome
            .clone();
        recording.attempts[1]
            .observer_assurance
            .as_mut()
            .unwrap()
            .physical_outcome = old_attempt_receipt;
        assert_eq!(
            recording.validate(),
            Err(LineageError::ObserverClaimMismatch)
        );
    }

    #[test]
    fn refuses_replayed_observer_artifact_across_attempts() {
        let mut recording = valid_recording();
        let old_artifact = recording.attempts[0]
            .observer_assurance
            .as_ref()
            .unwrap()
            .physical_outcome
            .observer_quorum
            .as_ref()
            .unwrap()
            .members[0]
            .raw_artifact_id
            .clone();
        recording.attempts[1]
            .observer_assurance
            .as_mut()
            .unwrap()
            .physical_outcome
            .observer_quorum
            .as_mut()
            .unwrap()
            .members[0]
            .raw_artifact_id = old_artifact;
        assert_eq!(
            recording.validate(),
            Err(LineageError::ObserverQuorumUnqualified)
        );
    }

    #[test]
    fn refuses_replayed_observation_id_across_attempts() {
        let mut recording = valid_recording();
        let old_observation = recording.attempts[0]
            .observer_assurance
            .as_ref()
            .unwrap()
            .physical_outcome
            .observer_quorum
            .as_ref()
            .unwrap()
            .members[0]
            .observation_id
            .clone();
        recording.attempts[1]
            .observer_assurance
            .as_mut()
            .unwrap()
            .physical_outcome
            .observer_quorum
            .as_mut()
            .unwrap()
            .members[0]
            .observation_id = old_observation;
        assert_eq!(
            recording.validate(),
            Err(LineageError::ObserverQuorumUnqualified)
        );
    }

    #[test]
    fn permits_same_physical_sensor_on_a_later_attempt_with_a_new_observation() {
        let mut recording = valid_recording();
        let prior_observer_id = recording.attempts[0]
            .observer_assurance
            .as_ref()
            .unwrap()
            .physical_outcome
            .observer_quorum
            .as_ref()
            .unwrap()
            .members[0]
            .observer_id
            .clone();
        recording.attempts[1]
            .observer_assurance
            .as_mut()
            .unwrap()
            .physical_outcome
            .observer_quorum
            .as_mut()
            .unwrap()
            .members[0]
            .observer_id = prior_observer_id;
        assert!(recording.validate().is_ok());
    }

    #[test]
    fn refuses_observer_proofs_under_another_fault_profile_revision() {
        let mut recording = valid_recording();
        recording.attempts[0]
            .observer_assurance
            .as_mut()
            .unwrap()
            .physical_outcome
            .observer_quorum
            .as_mut()
            .unwrap()
            .fault_model_revision = "other-fault-model".into();
        assert_eq!(
            recording.validate(),
            Err(LineageError::ObserverClaimMismatch)
        );
    }

    #[test]
    fn refuses_retry_after_indeterminate_persistence() {
        let mut recording = valid_recording();
        recording.attempts[0].persistence.outcome = PersistenceOutcome::IndeterminateCommit;
        recording.attempts[0].persistence.receipt.asserted_value =
            "indeterminate_commit".into();
        assert_eq!(
            recording.validate(),
            Err(LineageError::PersistenceUnresolved)
        );
    }

    #[test]
    fn accepts_committed_when_independent_no_effect_is_confirmed() {
        let mut recording = valid_recording();
        recording.attempts[0].persistence.outcome = PersistenceOutcome::Committed;
        recording.attempts[0].persistence.receipt.asserted_value = "committed".into();
        assert!(recording.validate().is_ok());
    }

    #[test]
    fn refuses_retry_when_no_effect_is_not_confirmed() {
        let mut recording = valid_recording();
        recording.attempts[0].physical_outcome.receipt.status =
            OwnerEvidenceStatus::InsufficientEvidence;
        assert_eq!(
            recording.validate(),
            Err(LineageError::ParentOutcomeNotRetryable)
        );
    }

    #[test]
    fn refuses_retry_after_applied_partial_unknown_disputed_or_not_observed_outcome() {
        for outcome in [
            PhysicalOutcome::Applied,
            PhysicalOutcome::Partial,
            PhysicalOutcome::Unknown,
            PhysicalOutcome::Disputed,
            PhysicalOutcome::NotObserved,
        ] {
            let mut recording = valid_recording();
            recording.attempts[0].physical_outcome.outcome = outcome;
            recording.attempts[0].physical_outcome.receipt.asserted_value =
                physical_value(outcome).into();
            assert!(
                recording.validate().is_err(),
                "outcome {outcome:?} must not have a descendant"
            );
        }
    }

    #[test]
    fn refuses_wrong_parent_attempt_and_skipped_ordinals() {
        let mut wrong_parent = valid_recording();
        wrong_parent.attempts[1].parent_attempt_id = Some("attempt-3".into());
        assert_eq!(
            wrong_parent.validate(),
            Err(LineageError::ParentLinkMismatch)
        );

        let mut skipped = valid_recording();
        skipped.attempts[1].ordinal = 3;
        assert_eq!(skipped.validate(), Err(LineageError::OrdinalDiscontinuity));
    }

    #[test]
    fn refuses_epoch_regression_and_fence_for_the_wrong_child_epoch() {
        let mut regressed = valid_recording();
        regressed.attempts[1].authorized_epoch.boot_epoch = 1;
        assert_eq!(regressed.validate(), Err(LineageError::EpochNotMonotonic));

        let mut wrong_fence = valid_recording();
        let wrong_target = EpochPair {
            boot_epoch: 2,
            controller_epoch: 9,
        };
        wrong_fence.attempts[0].downstream_quiescence.target_epoch = Some(wrong_target);
        wrong_fence.attempts[0].downstream_quiescence.receipt.asserted_value =
            fence_value(FenceOutcome::Quiescent, Some(wrong_target));
        assert_eq!(
            wrong_fence.validate(),
            Err(LineageError::FenceEpochMismatch)
        );
    }

    #[test]
    fn refuses_replayed_receipt_identity_and_cross_attempt_scope() {
        let mut duplicate = valid_recording();
        let prior_evidence_id = duplicate.attempts[0]
            .physical_outcome
            .receipt
            .evidence_id
            .clone();
        duplicate.attempts[1]
            .mission_resolution
            .receipt
            .evidence_id = prior_evidence_id;
        assert_eq!(
            duplicate.validate(),
            Err(LineageError::DuplicateEvidenceIdentity)
        );

        let mut cross_attempt = valid_recording();
        cross_attempt.attempts[1]
            .physical_outcome
            .receipt
            .scope
            .attempt_id = "attempt-1".into();
        assert_eq!(
            cross_attempt.validate(),
            Err(LineageError::EvidenceScopeMismatch)
        );
    }

    #[test]
    fn refuses_unqualified_controller_or_downstream_fence() {
        let mut controller = valid_recording();
        controller.attempts[0].controller_queue_fence.receipt.status =
            OwnerEvidenceStatus::InsufficientEvidence;
        assert_eq!(
            controller.validate(),
            Err(LineageError::ControllerFenceUnqualified)
        );

        let mut downstream = valid_recording();
        downstream.attempts[1].downstream_quiescence.outcome = FenceOutcome::Active;
        downstream.attempts[1].downstream_quiescence.target_epoch = None;
        downstream.attempts[1].downstream_quiescence.receipt.asserted_value =
            fence_value(FenceOutcome::Active, None);
        assert_eq!(
            downstream.validate(),
            Err(LineageError::DownstreamFenceUnqualified)
        );
    }

    #[test]
    fn rejects_empty_or_oversized_history() {
        let mut empty = valid_recording();
        empty.attempts.clear();
        assert_eq!(empty.validate(), Err(LineageError::EmptyAttemptChain));

        let mut oversized = valid_recording();
        let template = oversized.attempts[2].clone();
        oversized.attempts.resize(MAX_ATTEMPTS + 1, template);
        assert_eq!(oversized.validate(), Err(LineageError::TooManyAttempts));
    }

    #[test]
    fn strict_deserializer_rejects_unknown_fields() {
        let mut value: Value = serde_json::from_str(include_str!(
            "../fixtures/attempt_lineage_three_attempts_v0_1.json"
        ))
        .unwrap();
        value["unexpected_authority"] = serde_json::json!(true);
        assert!(serde_json::from_value::<AttemptLineageRecording>(value).is_err());
    }
}
