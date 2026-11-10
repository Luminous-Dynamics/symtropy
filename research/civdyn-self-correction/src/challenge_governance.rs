// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Challenge-set lifecycle controls for adaptive model evaluation.
//!
//! This module determines whether a model-family revision may treat a
//! challenge evaluation as confirmatory, sealed-only, exploratory or blocked.
//! It validates the supplied ledger snapshot and model lineage. It does not
//! authenticate the external ledger, prove that challenge bytes stayed secret,
//! or implement a differentially-private reusable-holdout mechanism.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeVisibility {
    SealedPrivate,
    PublicExploratory,
}

impl ChallengeVisibility {
    pub const fn name(self) -> &'static str {
        match self {
            Self::SealedPrivate => "sealed_private",
            Self::PublicExploratory => "public_exploratory",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeSetManifest {
    pub challenge_set_id: String,
    pub generation_id: String,
    pub challenge_set_digest: String,
    pub source_dataset_digest: String,
    pub split_manifest_digest: String,
    pub statistic_registry_version: String,
    pub evaluator_digest: String,
    pub visibility: ChallengeVisibility,
}

impl ChallengeSetManifest {
    fn validate(&self) -> Result<(), ChallengeGovernanceError> {
        if [
            self.challenge_set_id.as_str(),
            self.generation_id.as_str(),
            self.challenge_set_digest.as_str(),
            self.source_dataset_digest.as_str(),
            self.split_manifest_digest.as_str(),
            self.statistic_registry_version.as_str(),
            self.evaluator_digest.as_str(),
        ]
        .into_iter()
        .any(|value| value.trim().is_empty())
        {
            return Err(ChallengeGovernanceError::InvalidChallengeManifest);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRevision {
    pub model_family_id: String,
    pub revision_sequence: u64,
    pub model_digest: String,
    pub parent_model_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelLineage {
    pub revisions: Vec<ModelRevision>,
    pub candidate_model_digest: String,
}

impl ModelLineage {
    fn validate(&self) -> Result<(), ChallengeGovernanceError> {
        if self.revisions.is_empty() || self.candidate_model_digest.trim().is_empty() {
            return Err(ChallengeGovernanceError::InvalidModelLineage);
        }
        let family = self.revisions[0].model_family_id.as_str();
        if family.trim().is_empty() {
            return Err(ChallengeGovernanceError::InvalidModelLineage);
        }

        let mut seen_digests = BTreeSet::new();
        for (index, revision) in self.revisions.iter().enumerate() {
            if revision.model_family_id != family
                || revision.model_digest.trim().is_empty()
                || !seen_digests.insert(revision.model_digest.as_str())
                || revision.revision_sequence != index as u64
            {
                return Err(ChallengeGovernanceError::InvalidModelLineage);
            }
            let expected_parent = index
                .checked_sub(1)
                .map(|parent_index| self.revisions[parent_index].model_digest.as_str());
            match (expected_parent, revision.parent_model_digest.as_deref()) {
                (None, None) => {}
                (Some(expected), Some(actual)) if expected == actual => {}
                _ => return Err(ChallengeGovernanceError::InvalidModelLineage),
            }
        }

        if self.revisions.last().map(|revision| revision.model_digest.as_str())
            != Some(self.candidate_model_digest.as_str())
        {
            return Err(ChallengeGovernanceError::CandidateLineageMismatch);
        }
        Ok(())
    }

    pub fn model_family_id(&self) -> &str {
        self.revisions
            .last()
            .map(|revision| revision.model_family_id.as_str())
            .unwrap_or_default()
    }

    pub fn candidate_revision_sequence(&self) -> u64 {
        self.revisions
            .last()
            .map(|revision| revision.revision_sequence)
            .unwrap_or_default()
    }

    fn contains_revision(&self, sequence: u64, digest: &str) -> bool {
        self.revisions.iter().any(|revision| {
            revision.revision_sequence == sequence && revision.model_digest == digest
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackDisclosure {
    None,
    AggregateStatus,
    FullMetrics,
}

impl FeedbackDisclosure {
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::AggregateStatus => "aggregate_status",
            Self::FullMetrics => "full_metrics",
        }
    }

    const fn was_disclosed(self) -> bool {
        !matches!(self, Self::None)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeEvaluationEvent {
    pub ledger_sequence: u64,
    pub run_id: String,
    pub challenge_set_id: String,
    pub generation_id: String,
    pub challenge_set_digest: String,
    pub source_dataset_digest: String,
    pub split_manifest_digest: String,
    pub statistic_registry_version: String,
    pub evaluator_digest: String,
    pub model_family_id: String,
    pub revision_sequence: u64,
    pub model_digest: String,
    pub feedback_disclosure: FeedbackDisclosure,
    /// None means disclosed publicly; Some(family) means private disclosure.
    /// For FeedbackDisclosure::None this must be None.
    pub feedback_recipient_family_id: Option<String>,
    pub receipt_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeLedgerSnapshot {
    pub authority_id: String,
    /// Identifier/digest of the external authority's authenticated snapshot.
    /// The Rust validator checks presence but cannot authenticate a signature.
    pub authority_digest: String,
    pub snapshot_digest: String,
    pub snapshot_complete: bool,
    /// Highest committed ledger event sequence in this snapshot.
    pub complete_through_sequence: u64,
    pub events: Vec<ChallengeEvaluationEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeEligibility {
    FreshConfirmatory,
    SealedReuseOnly,
    BlockedDuplicateModel,
    BlockedAdaptiveReuse,
    ExploratoryOnly,
    NotAssessed,
}

impl ChallengeEligibility {
    pub const fn name(self) -> &'static str {
        match self {
            Self::FreshConfirmatory => "fresh_confirmatory",
            Self::SealedReuseOnly => "sealed_reuse_only",
            Self::BlockedDuplicateModel => "blocked_duplicate_model",
            Self::BlockedAdaptiveReuse => "blocked_adaptive_reuse",
            Self::ExploratoryOnly => "exploratory_only",
            Self::NotAssessed => "not_assessed",
        }
    }

    pub const fn confirmatory(self) -> bool {
        matches!(self, Self::FreshConfirmatory | Self::SealedReuseOnly)
    }

    pub const fn feedback_permitted(self) -> bool {
        matches!(self, Self::FreshConfirmatory)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeEligibilityReceipt {
    pub challenge_set_id: String,
    pub generation_id: String,
    pub challenge_set_digest: String,
    pub model_family_id: String,
    pub candidate_model_digest: String,
    pub candidate_revision_sequence: u64,
    pub evaluator_digest: String,
    pub split_manifest_digest: String,
    pub ledger_authority_id: String,
    pub ledger_authority_digest: String,
    pub ledger_snapshot_digest: String,
    pub evaluated_event_count: usize,
    pub eligibility: ChallengeEligibility,
    pub confirmatory_claim_permitted: bool,
    pub evaluation_feedback_permitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChallengeGovernanceError {
    InvalidChallengeManifest,
    InvalidModelLineage,
    CandidateLineageMismatch,
    InvalidLedgerAuthority,
    InvalidLedgerEvent { sequence: u64 },
    LedgerSequenceGap { expected: u64, actual: u64 },
    LedgerSequenceExceedsSnapshot,
    LedgerSnapshotSequenceMismatch,
    DuplicateRunId { run_id: String },
    DuplicateModelEvaluation { model_digest: String },
    ChallengeIdentityMismatch { sequence: u64 },
    ModelRevisionNotInLineage { sequence: u64 },
    InvalidFeedbackRecipient { sequence: u64 },
}

impl fmt::Display for ChallengeGovernanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ChallengeGovernanceError {}

fn event_matches_manifest(
    event: &ChallengeEvaluationEvent,
    manifest: &ChallengeSetManifest,
) -> bool {
    event.challenge_set_id == manifest.challenge_set_id
        && event.generation_id == manifest.generation_id
        && event.challenge_set_digest == manifest.challenge_set_digest
        && event.source_dataset_digest == manifest.source_dataset_digest
        && event.split_manifest_digest == manifest.split_manifest_digest
        && event.statistic_registry_version == manifest.statistic_registry_version
        && event.evaluator_digest == manifest.evaluator_digest
}

fn validate_ledger_snapshot(
    manifest: &ChallengeSetManifest,
    snapshot: &ChallengeLedgerSnapshot,
    lineage: &ModelLineage,
) -> Result<bool, ChallengeGovernanceError> {
    if snapshot.authority_id.trim().is_empty()
        || snapshot.authority_digest.trim().is_empty()
        || snapshot.snapshot_digest.trim().is_empty()
    {
        return Err(ChallengeGovernanceError::InvalidLedgerAuthority);
    }

    let mut run_ids = BTreeSet::new();
    let mut model_digests = BTreeSet::new();
    let mut expected_sequence = 1_u64;

    for event in &snapshot.events {
        if event.ledger_sequence > snapshot.complete_through_sequence {
            return Err(ChallengeGovernanceError::LedgerSequenceExceedsSnapshot);
        }
        if event.ledger_sequence != expected_sequence {
            return Err(ChallengeGovernanceError::LedgerSequenceGap {
                expected: expected_sequence,
                actual: event.ledger_sequence,
            });
        }
        expected_sequence = expected_sequence
            .checked_add(1)
            .ok_or(ChallengeGovernanceError::LedgerSnapshotSequenceMismatch)?;

        if event.run_id.trim().is_empty()
            || event.model_family_id.trim().is_empty()
            || event.model_digest.trim().is_empty()
            || event.receipt_digest.trim().is_empty()
        {
            return Err(ChallengeGovernanceError::InvalidLedgerEvent {
                sequence: event.ledger_sequence,
            });
        }
        if !run_ids.insert(event.run_id.as_str()) {
            return Err(ChallengeGovernanceError::DuplicateRunId {
                run_id: event.run_id.clone(),
            });
        }
        if !model_digests.insert(event.model_digest.as_str()) {
            return Err(ChallengeGovernanceError::DuplicateModelEvaluation {
                model_digest: event.model_digest.clone(),
            });
        }
        if !event_matches_manifest(event, manifest) {
            return Err(ChallengeGovernanceError::ChallengeIdentityMismatch {
                sequence: event.ledger_sequence,
            });
        }
        match (
            event.feedback_disclosure,
            event.feedback_recipient_family_id.as_deref(),
        ) {
            (FeedbackDisclosure::None, None) => {}
            (FeedbackDisclosure::None, Some(_)) => {
                return Err(ChallengeGovernanceError::InvalidFeedbackRecipient {
                    sequence: event.ledger_sequence,
                });
            }
            (FeedbackDisclosure::AggregateStatus | FeedbackDisclosure::FullMetrics, None) => {}
            (
                FeedbackDisclosure::AggregateStatus | FeedbackDisclosure::FullMetrics,
                Some(recipient),
            ) if !recipient.trim().is_empty() => {}
            _ => {
                return Err(ChallengeGovernanceError::InvalidFeedbackRecipient {
                    sequence: event.ledger_sequence,
                });
            }
        }

        if event.model_family_id == lineage.model_family_id()
            && !lineage.contains_revision(event.revision_sequence, &event.model_digest)
        {
            return Err(ChallengeGovernanceError::ModelRevisionNotInLineage {
                sequence: event.ledger_sequence,
            });
        }
    }

    if snapshot.snapshot_complete {
        if snapshot.complete_through_sequence != snapshot.events.len() as u64 {
            return Err(ChallengeGovernanceError::LedgerSnapshotSequenceMismatch);
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Determine whether a candidate model revision can receive confirmatory
/// feedback from a challenge set. A complete snapshot is a trust assertion
/// made by the external ledger authority; this function cannot authenticate it.
pub fn qualify_challenge_use(
    manifest: &ChallengeSetManifest,
    snapshot: &ChallengeLedgerSnapshot,
    lineage: &ModelLineage,
) -> Result<ChallengeEligibilityReceipt, ChallengeGovernanceError> {
    manifest.validate()?;
    lineage.validate()?;

    let snapshot_complete = validate_ledger_snapshot(manifest, snapshot, lineage)?;
    let candidate_digest = lineage.candidate_model_digest.as_str();
    let candidate_family = lineage.model_family_id();
    let candidate_revision = lineage.candidate_revision_sequence();

    let eligibility = if !snapshot_complete {
        ChallengeEligibility::NotAssessed
    } else if manifest.visibility == ChallengeVisibility::PublicExploratory {
        ChallengeEligibility::ExploratoryOnly
    } else if snapshot.events.iter().any(|event| event.model_digest == candidate_digest) {
        ChallengeEligibility::BlockedDuplicateModel
    } else if snapshot.events.iter().any(|event| {
        event.feedback_disclosure.was_disclosed()
            && (event.feedback_recipient_family_id.is_none()
                || event.feedback_recipient_family_id.as_deref() == Some(candidate_family))
    }) {
        ChallengeEligibility::BlockedAdaptiveReuse
    } else if snapshot.events.iter().any(|event| {
        event.model_family_id == candidate_family
            && event.revision_sequence < candidate_revision
    }) {
        ChallengeEligibility::SealedReuseOnly
    } else {
        ChallengeEligibility::FreshConfirmatory
    };

    Ok(ChallengeEligibilityReceipt {
        challenge_set_id: manifest.challenge_set_id.clone(),
        generation_id: manifest.generation_id.clone(),
        challenge_set_digest: manifest.challenge_set_digest.clone(),
        model_family_id: candidate_family.to_string(),
        candidate_model_digest: candidate_digest.to_string(),
        candidate_revision_sequence: candidate_revision,
        evaluator_digest: manifest.evaluator_digest.clone(),
        split_manifest_digest: manifest.split_manifest_digest.clone(),
        ledger_authority_id: snapshot.authority_id.clone(),
        ledger_authority_digest: snapshot.authority_digest.clone(),
        ledger_snapshot_digest: snapshot.snapshot_digest.clone(),
        evaluated_event_count: snapshot.events.len(),
        eligibility,
        confirmatory_claim_permitted: eligibility.confirmatory(),
        evaluation_feedback_permitted: eligibility.feedback_permitted(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAMILY: &str = "civdyn-family-alpha";

    fn manifest(visibility: ChallengeVisibility) -> ChallengeSetManifest {
        ChallengeSetManifest {
            challenge_set_id: "network-challenge".to_string(),
            generation_id: "generation-001".to_string(),
            challenge_set_digest: "sha256-challenge".to_string(),
            source_dataset_digest: "sha256-source".to_string(),
            split_manifest_digest: "sha256-split".to_string(),
            statistic_registry_version: "registry-v1".to_string(),
            evaluator_digest: "sha256-evaluator".to_string(),
            visibility,
        }
    }

    fn lineage() -> ModelLineage {
        ModelLineage {
            revisions: vec![
                ModelRevision {
                    model_family_id: FAMILY.to_string(),
                    revision_sequence: 0,
                    model_digest: "sha256-model-0".to_string(),
                    parent_model_digest: None,
                },
                ModelRevision {
                    model_family_id: FAMILY.to_string(),
                    revision_sequence: 1,
                    model_digest: "sha256-model-1".to_string(),
                    parent_model_digest: Some("sha256-model-0".to_string()),
                },
                ModelRevision {
                    model_family_id: FAMILY.to_string(),
                    revision_sequence: 2,
                    model_digest: "sha256-model-2".to_string(),
                    parent_model_digest: Some("sha256-model-1".to_string()),
                },
            ],
            candidate_model_digest: "sha256-model-2".to_string(),
        }
    }

    fn event(
        manifest: &ChallengeSetManifest,
        model_family_id: &str,
        revision_sequence: u64,
        model_digest: &str,
        feedback_disclosure: FeedbackDisclosure,
        feedback_recipient_family_id: Option<&str>,
    ) -> ChallengeEvaluationEvent {
        ChallengeEvaluationEvent {
            ledger_sequence: 1,
            run_id: "run-001".to_string(),
            challenge_set_id: manifest.challenge_set_id.clone(),
            generation_id: manifest.generation_id.clone(),
            challenge_set_digest: manifest.challenge_set_digest.clone(),
            source_dataset_digest: manifest.source_dataset_digest.clone(),
            split_manifest_digest: manifest.split_manifest_digest.clone(),
            statistic_registry_version: manifest.statistic_registry_version.clone(),
            evaluator_digest: manifest.evaluator_digest.clone(),
            model_family_id: model_family_id.to_string(),
            revision_sequence,
            model_digest: model_digest.to_string(),
            feedback_disclosure,
            feedback_recipient_family_id: feedback_recipient_family_id.map(str::to_string),
            receipt_digest: "sha256-receipt-001".to_string(),
        }
    }

    fn snapshot(events: Vec<ChallengeEvaluationEvent>) -> ChallengeLedgerSnapshot {
        ChallengeLedgerSnapshot {
            authority_id: "mycelix-challenge-ledger".to_string(),
            authority_digest: "sha256-authority-identity".to_string(),
            snapshot_digest: "sha256-ledger-snapshot".to_string(),
            snapshot_complete: true,
            complete_through_sequence: events.len() as u64,
            events,
        }
    }

    fn valid_prior(
        manifest: &ChallengeSetManifest,
        disclosure: FeedbackDisclosure,
        recipient: Option<&str>,
    ) -> ChallengeLedgerSnapshot {
        snapshot(vec![event(
            manifest,
            FAMILY,
            0,
            "sha256-model-0",
            disclosure,
            recipient,
        )])
    }

    #[test]
    fn empty_complete_ledger_qualifies_fresh_confirmatory_evaluation() {
        let challenge = manifest(ChallengeVisibility::SealedPrivate);
        let receipt = qualify_challenge_use(&challenge, &snapshot(Vec::new()), &lineage())
            .expect("fresh challenge eligibility");
        assert_eq!(receipt.eligibility, ChallengeEligibility::FreshConfirmatory);
        assert!(receipt.confirmatory_claim_permitted);
        assert!(receipt.evaluation_feedback_permitted);
    }

    #[test]
    fn no_feedback_prior_run_allows_only_sealed_reuse_for_later_family_revision() {
        let challenge = manifest(ChallengeVisibility::SealedPrivate);
        let prior = valid_prior(&challenge, FeedbackDisclosure::None, None);
        let receipt = qualify_challenge_use(&challenge, &prior, &lineage())
            .expect("sealed reuse eligibility");
        assert_eq!(receipt.eligibility, ChallengeEligibility::SealedReuseOnly);
        assert!(receipt.confirmatory_claim_permitted);
        assert!(!receipt.evaluation_feedback_permitted);
    }

    #[test]
    fn aggregate_feedback_to_same_family_blocks_adaptive_reuse() {
        let challenge = manifest(ChallengeVisibility::SealedPrivate);
        let prior = valid_prior(
            &challenge,
            FeedbackDisclosure::AggregateStatus,
            Some(FAMILY),
        );
        let receipt = qualify_challenge_use(&challenge, &prior, &lineage())
            .expect("blocked adaptive reuse receipt");
        assert_eq!(receipt.eligibility, ChallengeEligibility::BlockedAdaptiveReuse);
        assert!(!receipt.confirmatory_claim_permitted);
        assert!(!receipt.evaluation_feedback_permitted);
    }

    #[test]
    fn public_feedback_blocks_other_families_too() {
        let challenge = manifest(ChallengeVisibility::SealedPrivate);
        let mut other_family = lineage();
        for revision in &mut other_family.revisions {
            revision.model_family_id = "independent-family-beta".to_string();
        }
        let mut prior_event = event(
            &challenge,
            FAMILY,
            0,
            "sha256-other-model",
            FeedbackDisclosure::FullMetrics,
            None,
        );
        prior_event.run_id = "public-run".to_string();
        let receipt = qualify_challenge_use(&challenge, &snapshot(vec![prior_event]), &other_family)
            .expect("public disclosure blocks reuse");
        assert_eq!(receipt.eligibility, ChallengeEligibility::BlockedAdaptiveReuse);
    }

    #[test]
    fn private_feedback_to_another_family_does_not_block_candidate_family() {
        let challenge = manifest(ChallengeVisibility::SealedPrivate);
        let prior = snapshot(vec![event(
            &challenge,
            "independent-family-beta",
            0,
            "sha256-independent-model",
            FeedbackDisclosure::FullMetrics,
            Some("independent-family-beta"),
        )]);
        let receipt = qualify_challenge_use(&challenge, &prior, &lineage())
            .expect("private unrelated feedback");
        assert_eq!(receipt.eligibility, ChallengeEligibility::FreshConfirmatory);
    }

    #[test]
    fn exact_same_model_cannot_be_reused_as_a_second_confirmatory_run() {
        let challenge = manifest(ChallengeVisibility::SealedPrivate);
        let mut prior_event = event(
            &challenge,
            FAMILY,
            2,
            "sha256-model-2",
            FeedbackDisclosure::None,
            None,
        );
        prior_event.run_id = "same-model-run".to_string();
        let receipt = qualify_challenge_use(&challenge, &snapshot(vec![prior_event]), &lineage())
            .expect("duplicate model decision");
        assert_eq!(receipt.eligibility, ChallengeEligibility::BlockedDuplicateModel);
    }

    #[test]
    fn public_benchmark_is_always_exploratory() {
        let challenge = manifest(ChallengeVisibility::PublicExploratory);
        let receipt = qualify_challenge_use(&challenge, &snapshot(Vec::new()), &lineage())
            .expect("public benchmark classification");
        assert_eq!(receipt.eligibility, ChallengeEligibility::ExploratoryOnly);
        assert!(!receipt.confirmatory_claim_permitted);
        assert!(!receipt.evaluation_feedback_permitted);
    }

    #[test]
    fn incomplete_ledger_snapshot_is_not_assessed() {
        let challenge = manifest(ChallengeVisibility::SealedPrivate);
        let mut incomplete = snapshot(Vec::new());
        incomplete.snapshot_complete = false;
        incomplete.complete_through_sequence = 4;
        let receipt = qualify_challenge_use(&challenge, &incomplete, &lineage())
            .expect("incomplete snapshot remains explicit");
        assert_eq!(receipt.eligibility, ChallengeEligibility::NotAssessed);
        assert!(!receipt.confirmatory_claim_permitted);
    }

    #[test]
    fn sequence_gaps_and_duplicate_run_ids_are_rejected() {
        let challenge = manifest(ChallengeVisibility::SealedPrivate);
        let mut first = event(
            &challenge,
            FAMILY,
            0,
            "sha256-model-0",
            FeedbackDisclosure::None,
            None,
        );
        first.ledger_sequence = 1;
        let mut second = event(
            &challenge,
            FAMILY,
            1,
            "sha256-model-1",
            FeedbackDisclosure::None,
            None,
        );
        second.ledger_sequence = 3;
        assert!(matches!(
            qualify_challenge_use(&challenge, &snapshot(vec![first.clone(), second]), &lineage()),
            Err(ChallengeGovernanceError::LedgerSequenceGap { .. })
        ));

        let mut duplicate_run = event(
            &challenge,
            "independent-family-beta",
            0,
            "sha256-independent-model",
            FeedbackDisclosure::None,
            None,
        );
        duplicate_run.run_id = first.run_id.clone();
        assert!(matches!(
            qualify_challenge_use(&challenge, &snapshot(vec![first, duplicate_run]), &lineage()),
            Err(ChallengeGovernanceError::DuplicateRunId { .. })
        ));
    }

    #[test]
    fn changed_challenge_or_evaluator_identity_is_rejected() {
        let challenge = manifest(ChallengeVisibility::SealedPrivate);
        let mut prior = valid_prior(&challenge, FeedbackDisclosure::None, None);
        prior.events[0].evaluator_digest = "sha256-changed-evaluator".to_string();
        assert!(matches!(
            qualify_challenge_use(&challenge, &prior, &lineage()),
            Err(ChallengeGovernanceError::ChallengeIdentityMismatch { .. })
        ));
    }

    #[test]
    fn broken_parent_chain_and_candidate_digest_mismatch_are_rejected() {
        let challenge = manifest(ChallengeVisibility::SealedPrivate);
        let mut broken = lineage();
        broken.revisions[2].parent_model_digest = Some("sha256-wrong-parent".to_string());
        assert_eq!(
            qualify_challenge_use(&challenge, &snapshot(Vec::new()), &broken).unwrap_err(),
            ChallengeGovernanceError::InvalidModelLineage
        );

        let mut wrong_candidate = lineage();
        wrong_candidate.candidate_model_digest = "sha256-model-1".to_string();
        assert_eq!(
            qualify_challenge_use(&challenge, &snapshot(Vec::new()), &wrong_candidate).unwrap_err(),
            ChallengeGovernanceError::CandidateLineageMismatch
        );
    }

    #[test]
    fn fresh_challenge_generation_restores_confirmatory_eligibility() {
        let old_challenge = manifest(ChallengeVisibility::SealedPrivate);
        let _old_receipt = qualify_challenge_use(
            &old_challenge,
            &valid_prior(&old_challenge, FeedbackDisclosure::FullMetrics, Some(FAMILY)),
            &lineage(),
        )
        .expect("old challenge blocked");
        let mut new_challenge = old_challenge;
        new_challenge.generation_id = "generation-002".to_string();
        new_challenge.challenge_set_digest = "sha256-new-challenge".to_string();
        new_challenge.source_dataset_digest = "sha256-new-source".to_string();
        new_challenge.split_manifest_digest = "sha256-new-split".to_string();
        let receipt = qualify_challenge_use(&new_challenge, &snapshot(Vec::new()), &lineage())
            .expect("new challenge is fresh");
        assert_eq!(receipt.eligibility, ChallengeEligibility::FreshConfirmatory);
    }

    #[test]
    fn incomplete_or_invalid_authority_identity_never_qualifies() {
        let challenge = manifest(ChallengeVisibility::SealedPrivate);
        let mut prior = snapshot(Vec::new());
        prior.authority_digest.clear();
        assert_eq!(
            qualify_challenge_use(&challenge, &prior, &lineage()).unwrap_err(),
            ChallengeGovernanceError::InvalidLedgerAuthority
        );
    }
}
