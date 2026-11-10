// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Challenge-set custody for model repair and independent post-repair audits.
//!
//! This module verifies a declared immutable revision/evidence ledger. It can
//! detect recorded challenge leakage and retirement after result disclosure;
//! it cannot prove that unrecorded human access did not occur.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRevision {
    pub revision_id: String,
    pub model_digest: String,
    pub parent_model_digest: Option<String>,
    pub update_event_digest: String,
    /// All data/evidence digests used for training, calibration, fitting,
    /// diagnostic selection, hyperparameter tuning or repair in this revision.
    pub consumed_evidence_digests: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeEvaluationReceipt {
    pub receipt_id: String,
    pub model_digest: String,
    pub challenge_data_digest: String,
    pub challenge_manifest_digest: String,
    pub evaluation_protocol_digest: String,
    pub result_digest: String,
    pub result_disclosed_to_developers: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeRequest {
    pub candidate_model_digest: String,
    pub challenge_data_digest: String,
    pub challenge_manifest_digest: String,
    pub evaluation_protocol_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeEligibilityDecision {
    EligibleForSealedChallenge,
    ExactIdempotentReplay,
}

impl ChallengeEligibilityDecision {
    pub const fn name(self) -> &'static str {
        match self {
            Self::EligibleForSealedChallenge => "eligible_for_sealed_challenge",
            Self::ExactIdempotentReplay => "exact_idempotent_replay",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeEligibilityReport {
    pub decision: ChallengeEligibilityDecision,
    pub candidate_model_digest: String,
    pub challenge_data_digest: String,
    pub challenge_manifest_digest: String,
    pub evaluation_protocol_digest: String,
    pub revision_count: usize,
    pub unique_consumed_evidence_count: usize,
    pub prior_receipt_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChallengeCustodyError {
    EmptyRevisionChain,
    EmptyIdentity { field: &'static str },
    InvalidRootParent,
    ParentDigestMismatch { revision_id: String },
    DuplicateRevisionId { revision_id: String },
    DuplicateModelDigest { model_digest: String },
    DuplicateUpdateEventDigest { update_event_digest: String },
    DuplicateConsumedEvidence { revision_id: String, evidence_digest: String },
    CandidateNotChainHead,
    ChallengeDataConsumedByRevision { revision_id: String },
    DuplicateReceiptId { receipt_id: String },
    DisclosedChallengeRetired,
}

impl fmt::Display for ChallengeCustodyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ChallengeCustodyError {}

fn nonempty(value: &str, field: &'static str) -> Result<(), ChallengeCustodyError> {
    if value.trim().is_empty() {
        return Err(ChallengeCustodyError::EmptyIdentity { field });
    }
    Ok(())
}

fn validate_revision_chain(
    revisions: &[ModelRevision],
) -> Result<BTreeSet<String>, ChallengeCustodyError> {
    if revisions.is_empty() {
        return Err(ChallengeCustodyError::EmptyRevisionChain);
    }

    let mut revision_ids = BTreeSet::new();
    let mut model_digests = BTreeSet::new();
    let mut update_event_digests = BTreeSet::new();
    let mut consumed_evidence_digests = BTreeSet::new();

    for (index, revision) in revisions.iter().enumerate() {
        nonempty(&revision.revision_id, "revision_id")?;
        nonempty(&revision.model_digest, "model_digest")?;
        nonempty(&revision.update_event_digest, "update_event_digest")?;

        if !revision_ids.insert(revision.revision_id.as_str()) {
            return Err(ChallengeCustodyError::DuplicateRevisionId {
                revision_id: revision.revision_id.clone(),
            });
        }
        if !model_digests.insert(revision.model_digest.as_str()) {
            return Err(ChallengeCustodyError::DuplicateModelDigest {
                model_digest: revision.model_digest.clone(),
            });
        }
        if !update_event_digests.insert(revision.update_event_digest.as_str()) {
            return Err(ChallengeCustodyError::DuplicateUpdateEventDigest {
                update_event_digest: revision.update_event_digest.clone(),
            });
        }

        if index == 0 {
            if revision.parent_model_digest.is_some() {
                return Err(ChallengeCustodyError::InvalidRootParent);
            }
        } else if revision.parent_model_digest.as_deref()
            != Some(revisions[index - 1].model_digest.as_str())
        {
            return Err(ChallengeCustodyError::ParentDigestMismatch {
                revision_id: revision.revision_id.clone(),
            });
        }

        let mut seen_in_revision = BTreeSet::new();
        for digest in &revision.consumed_evidence_digests {
            nonempty(digest, "consumed_evidence_digest")?;
            if !seen_in_revision.insert(digest.as_str()) {
                return Err(ChallengeCustodyError::DuplicateConsumedEvidence {
                    revision_id: revision.revision_id.clone(),
                    evidence_digest: digest.clone(),
                });
            }
            consumed_evidence_digests.insert(digest.clone());
        }
    }

    Ok(consumed_evidence_digests)
}

/// Validate that a declared challenge dataset remains independent of all
/// recorded model-development evidence and has not been retired by a disclosed
/// challenge result for a changed model or evaluator protocol.
pub fn validate_challenge_eligibility(
    revisions: &[ModelRevision],
    prior_receipts: &[ChallengeEvaluationReceipt],
    request: &ChallengeRequest,
) -> Result<ChallengeEligibilityReport, ChallengeCustodyError> {
    nonempty(&request.candidate_model_digest, "candidate_model_digest")?;
    nonempty(&request.challenge_data_digest, "challenge_data_digest")?;
    nonempty(&request.challenge_manifest_digest, "challenge_manifest_digest")?;
    nonempty(&request.evaluation_protocol_digest, "evaluation_protocol_digest")?;

    let consumed_evidence_digests = validate_revision_chain(revisions)?;
    if revisions.last().map(|revision| revision.model_digest.as_str())
        != Some(request.candidate_model_digest.as_str())
    {
        return Err(ChallengeCustodyError::CandidateNotChainHead);
    }

    if consumed_evidence_digests.contains(&request.challenge_data_digest) {
        let revision_id = revisions
            .iter()
            .find(|revision| {
                revision
                    .consumed_evidence_digests
                    .iter()
                    .any(|digest| digest == &request.challenge_data_digest)
            })
            .map(|revision| revision.revision_id.clone())
            .unwrap_or_default();
        return Err(ChallengeCustodyError::ChallengeDataConsumedByRevision { revision_id });
    }

    let mut receipt_ids = BTreeSet::new();
    let mut exact_disclosed_replay = false;
    for receipt in prior_receipts {
        nonempty(&receipt.receipt_id, "receipt_id")?;
        nonempty(&receipt.model_digest, "receipt_model_digest")?;
        nonempty(&receipt.challenge_data_digest, "receipt_challenge_data_digest")?;
        nonempty(&receipt.challenge_manifest_digest, "receipt_challenge_manifest_digest")?;
        nonempty(&receipt.evaluation_protocol_digest, "receipt_evaluation_protocol_digest")?;
        nonempty(&receipt.result_digest, "receipt_result_digest")?;

        if !receipt_ids.insert(receipt.receipt_id.as_str()) {
            return Err(ChallengeCustodyError::DuplicateReceiptId {
                receipt_id: receipt.receipt_id.clone(),
            });
        }
        let same_data = receipt.challenge_data_digest == request.challenge_data_digest;
        let same_manifest = receipt.challenge_manifest_digest == request.challenge_manifest_digest;
        if receipt.result_disclosed_to_developers && (same_data || same_manifest) {
            let exact_replay = receipt.model_digest == request.candidate_model_digest
                && same_data
                && same_manifest
                && receipt.evaluation_protocol_digest == request.evaluation_protocol_digest;
            if exact_replay {
                exact_disclosed_replay = true;
            } else {
                return Err(ChallengeCustodyError::DisclosedChallengeRetired);
            }
        }
    }

    Ok(ChallengeEligibilityReport {
        decision: if exact_disclosed_replay {
            ChallengeEligibilityDecision::ExactIdempotentReplay
        } else {
            ChallengeEligibilityDecision::EligibleForSealedChallenge
        },
        candidate_model_digest: request.candidate_model_digest.clone(),
        challenge_data_digest: request.challenge_data_digest.clone(),
        challenge_manifest_digest: request.challenge_manifest_digest.clone(),
        evaluation_protocol_digest: request.evaluation_protocol_digest.clone(),
        revision_count: revisions.len(),
        unique_consumed_evidence_count: consumed_evidence_digests.len(),
        prior_receipt_count: prior_receipts.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn revision(
        revision_id: &str,
        model: &str,
        parent: Option<&str>,
        consumed: &[&str],
    ) -> ModelRevision {
        ModelRevision {
            revision_id: revision_id.to_string(),
            model_digest: model.to_string(),
            parent_model_digest: parent.map(str::to_string),
            update_event_digest: format!("sha256-update-{revision_id}"),
            consumed_evidence_digests: consumed.iter().map(|value| value.to_string()).collect(),
        }
    }

    fn request(model: &str, data: &str, manifest: &str, protocol: &str) -> ChallengeRequest {
        ChallengeRequest {
            candidate_model_digest: model.to_string(),
            challenge_data_digest: data.to_string(),
            challenge_manifest_digest: manifest.to_string(),
            evaluation_protocol_digest: protocol.to_string(),
        }
    }

    fn disclosed_receipt(
        receipt_id: &str,
        model: &str,
        data: &str,
        manifest: &str,
        protocol: &str,
    ) -> ChallengeEvaluationReceipt {
        ChallengeEvaluationReceipt {
            receipt_id: receipt_id.to_string(),
            model_digest: model.to_string(),
            challenge_data_digest: data.to_string(),
            challenge_manifest_digest: manifest.to_string(),
            evaluation_protocol_digest: protocol.to_string(),
            result_digest: format!("sha256-result-{receipt_id}"),
            result_disclosed_to_developers: true,
        }
    }

    fn root_lineage() -> Vec<ModelRevision> {
        vec![revision(
            "revision-0",
            "sha256-model-0",
            None,
            &["sha256-development-data-0", "sha256-calibration-data-0"],
        )]
    }

    #[test]
    fn fresh_challenge_is_eligible_for_exact_model_head() {
        let report = validate_challenge_eligibility(
            &root_lineage(),
            &[],
            &request(
                "sha256-model-0",
                "sha256-challenge-0",
                "sha256-challenge-manifest-0",
                "sha256-evaluator-v1",
            ),
        )
        .expect("fresh challenge should be eligible");
        assert_eq!(
            report.decision,
            ChallengeEligibilityDecision::EligibleForSealedChallenge
        );
        assert_eq!(report.revision_count, 1);
        assert_eq!(report.unique_consumed_evidence_count, 2);
    }

    #[test]
    fn challenge_used_in_ancestor_training_or_repair_is_rejected() {
        let revisions = vec![
            revision("revision-0", "sha256-model-0", None, &["sha256-challenge-0"]),
            revision(
                "revision-1",
                "sha256-model-1",
                Some("sha256-model-0"),
                &["sha256-repair-data-1"],
            ),
        ];
        assert!(matches!(
            validate_challenge_eligibility(
                &revisions,
                &[],
                &request(
                    "sha256-model-1",
                    "sha256-challenge-0",
                    "sha256-challenge-manifest-0",
                    "sha256-evaluator-v1"
                )
            ),
            Err(ChallengeCustodyError::ChallengeDataConsumedByRevision { .. })
        ));
    }

    #[test]
    fn disclosed_challenge_is_retired_after_model_repair_but_fresh_holdout_is_eligible() {
        let revisions = vec![
            revision(
                "revision-0",
                "sha256-model-0",
                None,
                &["sha256-development-data-0", "sha256-calibration-data-0"],
            ),
            revision(
                "revision-1",
                "sha256-model-1",
                Some("sha256-model-0"),
                &["sha256-development-data-0", "sha256-c0-diagnostic-receipt"],
            ),
        ];
        let receipts = vec![disclosed_receipt(
            "receipt-c0-model-0",
            "sha256-model-0",
            "sha256-challenge-c0",
            "sha256-manifest-c0",
            "sha256-evaluator-v1",
        )];

        assert_eq!(
            validate_challenge_eligibility(
                &revisions,
                &receipts,
                &request(
                    "sha256-model-1",
                    "sha256-challenge-c0",
                    "sha256-manifest-c0",
                    "sha256-evaluator-v1"
                )
            )
            .unwrap_err(),
            ChallengeCustodyError::DisclosedChallengeRetired
        );

        let fresh = validate_challenge_eligibility(
            &revisions,
            &receipts,
            &request(
                "sha256-model-1",
                "sha256-challenge-c1",
                "sha256-manifest-c1",
                "sha256-evaluator-v1",
            ),
        )
        .expect("fresh challenge remains eligible after repair");
        assert_eq!(
            fresh.decision,
            ChallengeEligibilityDecision::EligibleForSealedChallenge
        );
        assert_eq!(fresh.revision_count, 2);
        assert_eq!(fresh.prior_receipt_count, 1);
    }

    #[test]
    fn exact_same_model_same_protocol_replay_is_idempotent() {
        let receipts = vec![disclosed_receipt(
            "receipt-c0-model-0",
            "sha256-model-0",
            "sha256-challenge-c0",
            "sha256-manifest-c0",
            "sha256-evaluator-v1",
        )];
        let report = validate_challenge_eligibility(
            &root_lineage(),
            &receipts,
            &request(
                "sha256-model-0",
                "sha256-challenge-c0",
                "sha256-manifest-c0",
                "sha256-evaluator-v1",
            ),
        )
        .expect("exact replay is permitted");
        assert_eq!(
            report.decision,
            ChallengeEligibilityDecision::ExactIdempotentReplay
        );
    }

    #[test]
    fn disclosed_challenge_cannot_be_reused_under_changed_protocol_or_manifest() {
        let receipts = vec![disclosed_receipt(
            "receipt-c0-model-0",
            "sha256-model-0",
            "sha256-challenge-c0",
            "sha256-manifest-c0",
            "sha256-evaluator-v1",
        )];
        assert_eq!(
            validate_challenge_eligibility(
                &root_lineage(),
                &receipts,
                &request(
                    "sha256-model-0",
                    "sha256-challenge-c0",
                    "sha256-manifest-c0",
                    "sha256-evaluator-v2"
                )
            )
            .unwrap_err(),
            ChallengeCustodyError::DisclosedChallengeRetired
        );
        assert_eq!(
            validate_challenge_eligibility(
                &root_lineage(),
                &receipts,
                &request(
                    "sha256-model-0",
                    "sha256-challenge-c0",
                    "sha256-manifest-c0-revised",
                    "sha256-evaluator-v1"
                )
            )
            .unwrap_err(),
            ChallengeCustodyError::DisclosedChallengeRetired
        );
    }

    #[test]
    fn rejects_wrong_head_and_malformed_parent_chain() {
        let root = root_lineage();
        assert_eq!(
            validate_challenge_eligibility(
                &root,
                &[],
                &request(
                    "sha256-not-the-head",
                    "sha256-challenge-0",
                    "sha256-manifest-0",
                    "sha256-evaluator-v1"
                )
            )
            .unwrap_err(),
            ChallengeCustodyError::CandidateNotChainHead
        );

        let broken = vec![
            revision("revision-0", "sha256-model-0", None, &[]),
            revision(
                "revision-1",
                "sha256-model-1",
                Some("sha256-unrelated-model"),
                &[],
            ),
        ];
        assert!(matches!(
            validate_challenge_eligibility(
                &broken,
                &[],
                &request(
                    "sha256-model-1",
                    "sha256-challenge-0",
                    "sha256-manifest-0",
                    "sha256-evaluator-v1"
                )
            ),
            Err(ChallengeCustodyError::ParentDigestMismatch { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_revision_receipt_and_consumed_evidence_identities() {
        let duplicate_revision = vec![
            revision("revision-0", "sha256-model-0", None, &[]),
            revision(
                "revision-0",
                "sha256-model-1",
                Some("sha256-model-0"),
                &[],
            ),
        ];
        assert!(matches!(
            validate_challenge_eligibility(
                &duplicate_revision,
                &[],
                &request(
                    "sha256-model-1",
                    "sha256-challenge-0",
                    "sha256-manifest-0",
                    "sha256-evaluator-v1"
                )
            ),
            Err(ChallengeCustodyError::DuplicateRevisionId { .. })
        ));

        let duplicate_evidence = vec![revision(
            "revision-0",
            "sha256-model-0",
            None,
            &["sha256-same-data", "sha256-same-data"],
        )];
        assert!(matches!(
            validate_challenge_eligibility(
                &duplicate_evidence,
                &[],
                &request(
                    "sha256-model-0",
                    "sha256-challenge-0",
                    "sha256-manifest-0",
                    "sha256-evaluator-v1"
                )
            ),
            Err(ChallengeCustodyError::DuplicateConsumedEvidence { .. })
        ));

        let duplicate_receipt = vec![
            disclosed_receipt(
                "receipt-duplicate",
                "sha256-model-0",
                "sha256-challenge-a",
                "sha256-manifest-a",
                "sha256-evaluator-v1",
            ),
            disclosed_receipt(
                "receipt-duplicate",
                "sha256-model-0",
                "sha256-challenge-b",
                "sha256-manifest-b",
                "sha256-evaluator-v1",
            ),
        ];
        assert!(matches!(
            validate_challenge_eligibility(
                &root_lineage(),
                &duplicate_receipt,
                &request(
                    "sha256-model-0",
                    "sha256-challenge-0",
                    "sha256-manifest-0",
                    "sha256-evaluator-v1"
                )
            ),
            Err(ChallengeCustodyError::DuplicateReceiptId { .. })
        ));
    }

    #[test]
    fn rejects_empty_data_manifest_protocol_and_receipt_digests() {
        let mut candidate = request(
            "sha256-model-0",
            "sha256-challenge-0",
            "sha256-manifest-0",
            "sha256-evaluator-v1",
        );
        candidate.challenge_data_digest.clear();
        assert_eq!(
            validate_challenge_eligibility(&root_lineage(), &[], &candidate).unwrap_err(),
            ChallengeCustodyError::EmptyIdentity {
                field: "challenge_data_digest"
            }
        );

        let mut receipt = disclosed_receipt(
            "receipt-c0",
            "sha256-model-0",
            "sha256-c0",
            "sha256-manifest-c0",
            "sha256-evaluator-v1",
        );
        receipt.result_digest.clear();
        assert_eq!(
            validate_challenge_eligibility(
                &root_lineage(),
                &[receipt],
                &request(
                    "sha256-model-0",
                    "sha256-challenge-0",
                    "sha256-manifest-0",
                    "sha256-evaluator-v1"
                )
            )
            .unwrap_err(),
            ChallengeCustodyError::EmptyIdentity {
                field: "receipt_result_digest"
            }
        );
    }
}
