// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Multi-attempt physical-effect lineage validation.
//!
//! v0.1 of the runtime projection covers one old attempt and one recovery
//! admission. This module validates a longer, linear attempt history without
//! issuing recovery authority or replacing any owner of persistence, mission,
//! queue, observer, or native permit state.

use crate::{
    EvidenceScope, ObserverQuorumProof, OwnerEvidenceDomain, OwnerEvidenceStatus,
    claim_scope_digest, observer_proofs_are_independent, observer_quorum_is_qualified,
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
pub struct EvidenceReceipt {
    pub owner_id: String,
    pub owner_revision: String,
    pub evidence_id: String,
    pub raw_artifact_id: String,
    pub raw_artifact_sha256: String,
    pub scope: EvidenceScope,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PersistenceEvidence {
    pub outcome: PersistenceOutcome,
    pub verdict: OwnerEvidenceStatus,
    pub receipt: EvidenceReceipt,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PhysicalOutcomeEvidence {
    pub outcome: PhysicalOutcome,
    pub verdict: OwnerEvidenceStatus,
    pub receipt: EvidenceReceipt,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MissionResolutionEvidence {
    pub outcome: MissionOutcome,
    pub verdict: OwnerEvidenceStatus,
    pub receipt: EvidenceReceipt,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FenceEvidence {
    pub outcome: FenceOutcome,
    pub verdict: OwnerEvidenceStatus,
    /// The new boot/controller epoch for which the old path is quiescent.
    /// It is null when no qualified fence receipt is being asserted.
    pub target_epoch: Option<EpochPair>,
    pub receipt: EvidenceReceipt,
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
    /// Required for any attempt that is used as the parent of a retry edge.
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
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_receipt(
    receipt: &EvidenceReceipt,
    logical_effect_id: &str,
    attempt: &AttemptRecord,
    evidence_ids: &mut BTreeSet<String>,
) -> Result<(), LineageError> {
    if [
        receipt.owner_id.as_str(),
        receipt.owner_revision.as_str(),
        receipt.evidence_id.as_str(),
        receipt.raw_artifact_id.as_str(),
    ]
    .iter()
    .any(|value| !nonempty(value))
        || !valid_sha256(&receipt.raw_artifact_sha256)
    {
        return Err(LineageError::InvalidEvidenceReceipt);
    }

    if !evidence_ids.insert(receipt.evidence_id.clone()) {
        return Err(LineageError::DuplicateEvidenceIdentity);
    }

    let scope = &receipt.scope;
    if scope.logical_effect_id != logical_effect_id
        || scope.attempt_id != attempt.attempt_id
        || scope.subject_boot_epoch != attempt.authorized_epoch.boot_epoch
        || scope.subject_controller_epoch != attempt.authorized_epoch.controller_epoch
    {
        return Err(LineageError::EvidenceScopeMismatch);
    }
    Ok(())
}

fn validate_attempt_evidence(
    logical_effect_id: &str,
    attempt: &AttemptRecord,
    evidence_ids: &mut BTreeSet<String>,
) -> Result<(), LineageError> {
    for receipt in [
        &attempt.persistence.receipt,
        &attempt.physical_outcome.receipt,
        &attempt.mission_resolution.receipt,
        &attempt.controller_queue_fence.receipt,
        &attempt.downstream_quiescence.receipt,
    ] {
        validate_receipt(receipt, logical_effect_id, attempt, evidence_ids)?;
    }

    match attempt.physical_outcome.outcome {
        PhysicalOutcome::Applied => {
            if attempt.physical_outcome.verdict != OwnerEvidenceStatus::Confirmed {
                return Err(LineageError::OutcomeVerdictMismatch);
            }
            if attempt.mission_resolution.outcome != MissionOutcome::EffectPredicateSatisfied
                || attempt.mission_resolution.verdict != OwnerEvidenceStatus::Confirmed
            {
                return Err(LineageError::MissionOutcomeMismatch);
            }
        }
        PhysicalOutcome::NoEffect => {
            if attempt.mission_resolution.outcome == MissionOutcome::EffectPredicateSatisfied
                && attempt.mission_resolution.verdict == OwnerEvidenceStatus::Confirmed
            {
                return Err(LineageError::MissionOutcomeMismatch);
            }
        }
        PhysicalOutcome::NotObserved
        | PhysicalOutcome::Partial
        | PhysicalOutcome::Unknown
        | PhysicalOutcome::Disputed => {
            if attempt.mission_resolution.outcome != MissionOutcome::Unresolved {
                return Err(LineageError::MissionOutcomeMismatch);
            }
        }
    }
    Ok(())
}

fn validate_attempt_observer_assurance(
    fault_profile_revision: &str,
    logical_effect_id: &str,
    attempt: &AttemptRecord,
) -> Result<(), LineageError> {
    let assurance = attempt
        .observer_assurance
        .as_ref()
        .ok_or(LineageError::MissingObserverAssurance)?;

    if !nonempty(&assurance.physical_outcome_claim_id)
        || !nonempty(&assurance.downstream_quiescence_claim_id)
        || assurance.physical_outcome_claim_id == assurance.downstream_quiescence_claim_id
        || assurance.physical_outcome_asserted_value != "no_effect"
        || assurance.downstream_quiescence_asserted_value != "quiescent"
        || assurance.physical_outcome_quorum.fault_model_revision != fault_profile_revision
        || assurance.downstream_quiescence_quorum.fault_model_revision != fault_profile_revision
    {
        return Err(LineageError::ObserverClaimMismatch);
    }

    let scope = EvidenceScope {
        logical_effect_id: logical_effect_id.to_owned(),
        attempt_id: attempt.attempt_id.clone(),
        subject_boot_epoch: attempt.authorized_epoch.boot_epoch,
        subject_controller_epoch: attempt.authorized_epoch.controller_epoch,
    };

    if assurance.physical_outcome_claim_scope_sha256
        != claim_scope_digest(
            OwnerEvidenceDomain::PhysicalOutcomeObserverQuorum,
            &assurance.physical_outcome_claim_id,
            &assurance.physical_outcome_asserted_value,
            &scope,
        )
        || assurance.downstream_quiescence_claim_scope_sha256
            != claim_scope_digest(
                OwnerEvidenceDomain::DownstreamObserverQuorum,
                &assurance.downstream_quiescence_claim_id,
                &assurance.downstream_quiescence_asserted_value,
                &scope,
            )
    {
        return Err(LineageError::ObserverClaimMismatch);
    }

    if observer_quorum_is_qualified(&assurance.physical_outcome_quorum) != Ok(true)
        || observer_quorum_is_qualified(&assurance.downstream_quiescence_quorum) != Ok(true)
        || !observer_proofs_are_independent(
            &assurance.physical_outcome_quorum,
            &assurance.downstream_quiescence_quorum,
        )
    {
        return Err(LineageError::ObserverQuorumUnqualified);
    }
    Ok(())
}

fn validate_recovery_handoff(
    fault_profile_revision: &str,
    logical_effect_id: &str,
    parent: &AttemptRecord,
    child: &AttemptRecord,
) -> Result<(), LineageError> {
    if parent.physical_outcome.outcome != PhysicalOutcome::NoEffect
        || parent.physical_outcome.verdict != OwnerEvidenceStatus::Confirmed
        || parent.mission_resolution.outcome != MissionOutcome::NoEffect
        || parent.mission_resolution.verdict != OwnerEvidenceStatus::Confirmed
    {
        return Err(LineageError::ParentOutcomeNotRetryable);
    }

    validate_attempt_observer_assurance(fault_profile_revision, logical_effect_id, parent)?;

    if !matches!(
        parent.persistence.outcome,
        PersistenceOutcome::DefinitelyNotCommitted | PersistenceOutcome::Committed
    ) || parent.persistence.verdict != OwnerEvidenceStatus::Confirmed
    {
        return Err(LineageError::PersistenceUnresolved);
    }

    let target = child.authorized_epoch;
    let controller = &parent.controller_queue_fence;
    if controller.outcome != FenceOutcome::Quiescent
        || controller.verdict != OwnerEvidenceStatus::Confirmed
    {
        return Err(LineageError::ControllerFenceUnqualified);
    }
    if controller.target_epoch != Some(target) {
        return Err(LineageError::FenceEpochMismatch);
    }

    let downstream = &parent.downstream_quiescence;
    if downstream.outcome != FenceOutcome::Quiescent
        || downstream.verdict != OwnerEvidenceStatus::Confirmed
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
    /// A successful result means only that the supplied records are internally
    /// consistent and every retry edge has the required owner-evidence shape.
    /// It does not authenticate receipts, infer missing physical observations,
    /// or authorize a native action.
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
                )?;
            }

            validate_attempt_evidence(&self.logical_effect_id, attempt, &mut evidence_ids)?;

            if attempt.physical_outcome.outcome == PhysicalOutcome::Applied {
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
        assurance.downstream_quiescence_quorum.members[0].common_cause_domains[0] =
            assurance.physical_outcome_quorum.members[0].common_cause_domains[0].clone();
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
            .physical_outcome_claim_scope_sha256 = "f".repeat(64);
        assert_eq!(
            recording.validate(),
            Err(LineageError::ObserverClaimMismatch)
        );
    }

    #[test]
    fn refuses_observer_proofs_under_another_fault_profile_revision() {
        let mut recording = valid_recording();
        recording.attempts[0]
            .observer_assurance
            .as_mut()
            .unwrap()
            .physical_outcome_quorum
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
        assert_eq!(
            recording.validate(),
            Err(LineageError::PersistenceUnresolved)
        );
    }

    #[test]
    fn accepts_resolved_committed_persistence_when_physical_and_mission_outcomes_confirm_no_effect() {
        let mut recording = valid_recording();
        recording.attempts[0].persistence.outcome = PersistenceOutcome::Committed;
        assert!(recording.validate().is_ok());
    }

    #[test]
    fn refuses_retry_when_no_effect_is_not_confirmed() {
        let mut recording = valid_recording();
        recording.attempts[0].physical_outcome.verdict = OwnerEvidenceStatus::InsufficientEvidence;
        assert_eq!(
            recording.validate(),
            Err(LineageError::ParentOutcomeNotRetryable)
        );
    }

    #[test]
    fn refuses_retry_after_applied_partial_unknown_or_disputed_outcome() {
        for outcome in [
            PhysicalOutcome::Applied,
            PhysicalOutcome::Partial,
            PhysicalOutcome::Unknown,
            PhysicalOutcome::Disputed,
            PhysicalOutcome::NotObserved,
        ] {
            let mut recording = valid_recording();
            recording.attempts[0].physical_outcome.outcome = outcome;
            recording.attempts[0].mission_resolution.outcome =
                if outcome == PhysicalOutcome::Applied {
                    MissionOutcome::EffectPredicateSatisfied
                } else {
                    MissionOutcome::Unresolved
                };
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
        wrong_fence.attempts[0].downstream_quiescence.target_epoch = Some(EpochPair {
            boot_epoch: 2,
            controller_epoch: 9,
        });
        assert_eq!(
            wrong_fence.validate(),
            Err(LineageError::FenceEpochMismatch)
        );
    }

    #[test]
    fn refuses_replayed_receipt_identity_and_cross_attempt_scope() {
        let mut duplicate = valid_recording();
        duplicate.attempts[1].mission_resolution.receipt.evidence_id = duplicate.attempts[0]
            .physical_outcome
            .receipt
            .evidence_id
            .clone();
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
        controller.attempts[0].controller_queue_fence.verdict =
            OwnerEvidenceStatus::InsufficientEvidence;
        assert_eq!(
            controller.validate(),
            Err(LineageError::ControllerFenceUnqualified)
        );

        let mut downstream = valid_recording();
        downstream.attempts[1].downstream_quiescence.outcome = FenceOutcome::Active;
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
