// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Budget and audit developer-visible feedback from a sealed scientific benchmark.
//!
//! This module treats evaluation feedback as a data release. It controls only
//! the declared, append-only feedback ledger; it cannot prove that a recipient
//! did not infer information or receive it through an unrecorded channel.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FeedbackDisclosureTier {
    SealedNoFeedback,
    BinaryDecision,
    AggregateScore,
    MultiMetricProfile,
    PerStatisticResults,
    RawDiagnostics,
}

impl FeedbackDisclosureTier {
    pub const fn name(self) -> &'static str {
        match self {
            Self::SealedNoFeedback => "sealed_no_feedback",
            Self::BinaryDecision => "binary_decision",
            Self::AggregateScore => "aggregate_score",
            Self::MultiMetricProfile => "multi_metric_profile",
            Self::PerStatisticResults => "per_statistic_results",
            Self::RawDiagnostics => "raw_diagnostics",
        }
    }

    const fn is_developer_visible(self) -> bool {
        !matches!(self, Self::SealedNoFeedback)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackBudgetStatus {
    EligibleWithinFrozenFeedbackBudget,
    FeedbackBudgetExhausted,
    DisclosureTierExceeded,
    NotAssessed,
}

impl FeedbackBudgetStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::EligibleWithinFrozenFeedbackBudget => "eligible_within_frozen_feedback_budget",
            Self::FeedbackBudgetExhausted => "feedback_budget_exhausted",
            Self::DisclosureTierExceeded => "disclosure_tier_exceeded",
            Self::NotAssessed => "not_assessed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackBudgetReason {
    WithinFrozenBudget,
    ExactIdempotentReplay,
    CountBudgetExhausted,
    DisclosureTierExceeded,
    HistoricalDisclosureTierViolation,
    HistoricalCountBudgetViolation,
    InvalidOrIncompleteCustodyEvidence,
}

impl FeedbackBudgetReason {
    pub const fn name(self) -> &'static str {
        match self {
            Self::WithinFrozenBudget => "within_frozen_budget",
            Self::ExactIdempotentReplay => "exact_idempotent_replay",
            Self::CountBudgetExhausted => "count_budget_exhausted",
            Self::DisclosureTierExceeded => "disclosure_tier_exceeded",
            Self::HistoricalDisclosureTierViolation => "historical_disclosure_tier_violation",
            Self::HistoricalCountBudgetViolation => "historical_count_budget_violation",
            Self::InvalidOrIncompleteCustodyEvidence => "invalid_or_incomplete_custody_evidence",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackBudgetPolicy {
    pub policy_id: String,
    pub policy_digest: String,
    pub challenge_data_digest: String,
    pub challenge_manifest_digest: String,
    pub evaluator_protocol_digest: String,
    pub metric_registry_digest: String,
    pub maximum_nonsealed_releases: usize,
    pub maximum_disclosure_tier: FeedbackDisclosureTier,
    pub preregistration_receipt_id: String,
    pub freeze_event_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackReleaseEvent {
    pub event_id: String,
    pub receipt_id: String,
    pub policy_digest: String,
    pub model_digest: String,
    pub model_revision_digest: String,
    pub challenge_data_digest: String,
    pub challenge_manifest_digest: String,
    pub evaluator_protocol_digest: String,
    pub metric_registry_digest: String,
    pub disclosure_tier: FeedbackDisclosureTier,
    /// Exact digest of the developer-visible artifact. Absent only when sealed.
    pub release_artifact_digest: Option<String>,
    /// Stable recipient/audience identity. Absent only when sealed.
    pub recipient_id: Option<String>,
    /// Monotonic logical order assigned by the trusted custody ledger.
    pub sequence: u64,
    pub event_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackBudgetReport {
    pub policy_id: String,
    pub policy_digest: String,
    pub challenge_data_digest: String,
    pub challenge_manifest_digest: String,
    pub historical_nonsealed_releases: usize,
    pub projected_nonsealed_releases: usize,
    pub releases_remaining_after_request: usize,
    pub highest_historical_disclosure_tier: FeedbackDisclosureTier,
    pub requested_disclosure_tier: FeedbackDisclosureTier,
    pub maximum_disclosure_tier: FeedbackDisclosureTier,
    pub idempotent_replay: bool,
    pub status: FeedbackBudgetStatus,
    pub reason: FeedbackBudgetReason,
    /// Present when validation failed and the decision must remain NotAssessed.
    pub validation_error: Option<FeedbackBudgetError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedbackBudgetError {
    MissingPolicyIdentity,
    InvalidPolicyBudget,
    MissingEventIdentity,
    PolicyDigestMismatch,
    ChallengeIdentityMismatch,
    EvaluatorProtocolMismatch,
    MetricRegistryMismatch,
    DuplicateEventId { event_id: String },
    DuplicateReceiptId { receipt_id: String },
    DuplicateReleaseArtifact { artifact_digest: String },
    DuplicateEventDigest { event_digest: String },
    EventSequenceNotIncreasing,
    SealedEventHasReleaseArtifact,
    SealedEventHasRecipient,
    VisibleEventMissingReleaseArtifact,
    VisibleEventMissingRecipient,
    EventAlreadyExistsWithDifferentContents { event_id: String },
    HistoricalEventExceedsDisclosureTier,
}

impl fmt::Display for FeedbackBudgetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for FeedbackBudgetError {}

impl FeedbackBudgetPolicy {
    fn validate(&self) -> Result<(), FeedbackBudgetError> {
        if self.policy_id.trim().is_empty()
            || self.policy_digest.trim().is_empty()
            || self.challenge_data_digest.trim().is_empty()
            || self.challenge_manifest_digest.trim().is_empty()
            || self.evaluator_protocol_digest.trim().is_empty()
            || self.metric_registry_digest.trim().is_empty()
            || self.preregistration_receipt_id.trim().is_empty()
            || self.freeze_event_digest.trim().is_empty()
        {
            return Err(FeedbackBudgetError::MissingPolicyIdentity);
        }
        // A zero release budget is valid: it permits only sealed evaluation.
        Ok(())
    }
}

fn validate_event(
    policy: &FeedbackBudgetPolicy,
    event: &FeedbackReleaseEvent,
) -> Result<(), FeedbackBudgetError> {
    if event.event_id.trim().is_empty()
        || event.receipt_id.trim().is_empty()
        || event.policy_digest.trim().is_empty()
        || event.model_digest.trim().is_empty()
        || event.model_revision_digest.trim().is_empty()
        || event.event_digest.trim().is_empty()
    {
        return Err(FeedbackBudgetError::MissingEventIdentity);
    }
    if event.policy_digest != policy.policy_digest {
        return Err(FeedbackBudgetError::PolicyDigestMismatch);
    }
    if event.challenge_data_digest != policy.challenge_data_digest
        || event.challenge_manifest_digest != policy.challenge_manifest_digest
    {
        return Err(FeedbackBudgetError::ChallengeIdentityMismatch);
    }
    if event.evaluator_protocol_digest != policy.evaluator_protocol_digest {
        return Err(FeedbackBudgetError::EvaluatorProtocolMismatch);
    }
    if event.metric_registry_digest != policy.metric_registry_digest {
        return Err(FeedbackBudgetError::MetricRegistryMismatch);
    }

    match event.disclosure_tier {
        FeedbackDisclosureTier::SealedNoFeedback => {
            if event.release_artifact_digest.is_some() {
                return Err(FeedbackBudgetError::SealedEventHasReleaseArtifact);
            }
            if event.recipient_id.is_some() {
                return Err(FeedbackBudgetError::SealedEventHasRecipient);
            }
        }
        _ => {
            if event
                .release_artifact_digest
                .as_deref()
                .is_none_or(|digest| digest.trim().is_empty())
            {
                return Err(FeedbackBudgetError::VisibleEventMissingReleaseArtifact);
            }
            if event
                .recipient_id
                .as_deref()
                .is_none_or(|recipient| recipient.trim().is_empty())
            {
                return Err(FeedbackBudgetError::VisibleEventMissingRecipient);
            }
        }
    }
    Ok(())
}

fn report(
    policy: &FeedbackBudgetPolicy,
    historical_nonsealed_releases: usize,
    projected_nonsealed_releases: usize,
    highest_historical_disclosure_tier: FeedbackDisclosureTier,
    requested_disclosure_tier: FeedbackDisclosureTier,
    idempotent_replay: bool,
    status: FeedbackBudgetStatus,
    reason: FeedbackBudgetReason,
) -> FeedbackBudgetReport {
    FeedbackBudgetReport {
        policy_id: policy.policy_id.clone(),
        policy_digest: policy.policy_digest.clone(),
        challenge_data_digest: policy.challenge_data_digest.clone(),
        challenge_manifest_digest: policy.challenge_manifest_digest.clone(),
        historical_nonsealed_releases,
        projected_nonsealed_releases,
        releases_remaining_after_request: policy
            .maximum_nonsealed_releases
            .saturating_sub(projected_nonsealed_releases),
        highest_historical_disclosure_tier,
        requested_disclosure_tier,
        maximum_disclosure_tier: policy.maximum_disclosure_tier,
        idempotent_replay,
        status,
        reason,
        validation_error: None,
    }
}

/// Evaluate whether the next developer-visible release is allowed.
///
/// The historical ledger is authoritative: all counts and disclosure levels
/// are derived from its ordered events, never caller-provided totals. An exact
/// event replay is idempotent and does not consume another release opportunity.
pub fn evaluate_feedback_budget(
    policy: &FeedbackBudgetPolicy,
    historical_events: &[FeedbackReleaseEvent],
    next_request: &FeedbackReleaseEvent,
) -> Result<FeedbackBudgetReport, FeedbackBudgetError> {
    policy.validate()?;

    let mut event_ids = BTreeSet::new();
    let mut receipt_ids = BTreeSet::new();
    let mut artifact_digests = BTreeSet::new();
    let mut event_digests = BTreeSet::new();
    let mut previous_sequence = None;
    let mut historical_nonsealed_releases = 0_usize;
    let mut highest_historical_disclosure_tier = FeedbackDisclosureTier::SealedNoFeedback;
    let mut historical_tier_violation = false;

    for event in historical_events {
        validate_event(policy, event)?;
        if !event_ids.insert(event.event_id.as_str()) {
            return Err(FeedbackBudgetError::DuplicateEventId {
                event_id: event.event_id.clone(),
            });
        }
        if !receipt_ids.insert(event.receipt_id.as_str()) {
            return Err(FeedbackBudgetError::DuplicateReceiptId {
                receipt_id: event.receipt_id.clone(),
            });
        }
        if !event_digests.insert(event.event_digest.as_str()) {
            return Err(FeedbackBudgetError::DuplicateEventDigest {
                event_digest: event.event_digest.clone(),
            });
        }
        if previous_sequence.is_some_and(|previous| event.sequence <= previous) {
            return Err(FeedbackBudgetError::EventSequenceNotIncreasing);
        }
        previous_sequence = Some(event.sequence);

        if let Some(artifact) = event.release_artifact_digest.as_deref() {
            if !artifact_digests.insert(artifact) {
                return Err(FeedbackBudgetError::DuplicateReleaseArtifact {
                    artifact_digest: artifact.to_string(),
                });
            }
        }
        if event.disclosure_tier.is_developer_visible() {
            historical_nonsealed_releases = historical_nonsealed_releases
                .checked_add(1)
                .ok_or(FeedbackBudgetError::InvalidPolicyBudget)?;
            if event.disclosure_tier > highest_historical_disclosure_tier {
                highest_historical_disclosure_tier = event.disclosure_tier;
            }
            historical_tier_violation |= event.disclosure_tier > policy.maximum_disclosure_tier;
        }
    }

    validate_event(policy, next_request)?;

    if let Some(existing) = historical_events
        .iter()
        .find(|event| event.event_id == next_request.event_id)
    {
        if existing != next_request {
            return Err(
                FeedbackBudgetError::EventAlreadyExistsWithDifferentContents {
                    event_id: next_request.event_id.clone(),
                },
            );
        }
        if historical_tier_violation {
            return Ok(report(
                policy,
                historical_nonsealed_releases,
                historical_nonsealed_releases,
                highest_historical_disclosure_tier,
                next_request.disclosure_tier,
                true,
                FeedbackBudgetStatus::DisclosureTierExceeded,
                FeedbackBudgetReason::HistoricalDisclosureTierViolation,
            ));
        }
        if historical_nonsealed_releases > policy.maximum_nonsealed_releases {
            return Ok(report(
                policy,
                historical_nonsealed_releases,
                historical_nonsealed_releases,
                highest_historical_disclosure_tier,
                next_request.disclosure_tier,
                true,
                FeedbackBudgetStatus::FeedbackBudgetExhausted,
                FeedbackBudgetReason::HistoricalCountBudgetViolation,
            ));
        }
        return Ok(report(
            policy,
            historical_nonsealed_releases,
            historical_nonsealed_releases,
            highest_historical_disclosure_tier,
            next_request.disclosure_tier,
            true,
            FeedbackBudgetStatus::EligibleWithinFrozenFeedbackBudget,
            FeedbackBudgetReason::ExactIdempotentReplay,
        ));
    }

    if event_ids.contains(next_request.event_id.as_str()) {
        return Err(FeedbackBudgetError::DuplicateEventId {
            event_id: next_request.event_id.clone(),
        });
    }
    if receipt_ids.contains(next_request.receipt_id.as_str()) {
        return Err(FeedbackBudgetError::DuplicateReceiptId {
            receipt_id: next_request.receipt_id.clone(),
        });
    }
    if event_digests.contains(next_request.event_digest.as_str()) {
        return Err(FeedbackBudgetError::DuplicateEventDigest {
            event_digest: next_request.event_digest.clone(),
        });
    }
    if let Some(artifact) = next_request.release_artifact_digest.as_deref() {
        if artifact_digests.contains(artifact) {
            return Err(FeedbackBudgetError::DuplicateReleaseArtifact {
                artifact_digest: artifact.to_string(),
            });
        }
    }
    if previous_sequence.is_some_and(|previous| next_request.sequence <= previous) {
        return Err(FeedbackBudgetError::EventSequenceNotIncreasing);
    }

    if historical_tier_violation {
        return Ok(report(
            policy,
            historical_nonsealed_releases,
            historical_nonsealed_releases,
            highest_historical_disclosure_tier,
            next_request.disclosure_tier,
            false,
            FeedbackBudgetStatus::DisclosureTierExceeded,
            FeedbackBudgetReason::HistoricalDisclosureTierViolation,
        ));
    }
    if historical_nonsealed_releases > policy.maximum_nonsealed_releases {
        return Ok(report(
            policy,
            historical_nonsealed_releases,
            historical_nonsealed_releases,
            highest_historical_disclosure_tier,
            next_request.disclosure_tier,
            false,
            FeedbackBudgetStatus::FeedbackBudgetExhausted,
            FeedbackBudgetReason::HistoricalCountBudgetViolation,
        ));
    }

    if next_request.disclosure_tier > policy.maximum_disclosure_tier {
        return Ok(report(
            policy,
            historical_nonsealed_releases,
            historical_nonsealed_releases,
            highest_historical_disclosure_tier,
            next_request.disclosure_tier,
            false,
            FeedbackBudgetStatus::DisclosureTierExceeded,
            FeedbackBudgetReason::DisclosureTierExceeded,
        ));
    }

    let consumes_release = next_request.disclosure_tier.is_developer_visible();
    if consumes_release && historical_nonsealed_releases >= policy.maximum_nonsealed_releases {
        return Ok(report(
            policy,
            historical_nonsealed_releases,
            historical_nonsealed_releases,
            highest_historical_disclosure_tier,
            next_request.disclosure_tier,
            false,
            FeedbackBudgetStatus::FeedbackBudgetExhausted,
            FeedbackBudgetReason::CountBudgetExhausted,
        ));
    }

    let projected = historical_nonsealed_releases + usize::from(consumes_release);
    Ok(report(
        policy,
        historical_nonsealed_releases,
        projected,
        highest_historical_disclosure_tier,
        next_request.disclosure_tier,
        false,
        FeedbackBudgetStatus::EligibleWithinFrozenFeedbackBudget,
        FeedbackBudgetReason::WithinFrozenBudget,
    ))
}

/// Fail-closed status-oriented wrapper. Malformed or incomplete ledger evidence
/// returns NotAssessed; callers needing the exact failure can use
/// evaluate_feedback_budget directly.
pub fn assess_feedback_budget(
    policy: &FeedbackBudgetPolicy,
    historical_events: &[FeedbackReleaseEvent],
    next_request: &FeedbackReleaseEvent,
) -> FeedbackBudgetReport {
    match evaluate_feedback_budget(policy, historical_events, next_request) {
        Ok(report) => report,
        Err(error) => FeedbackBudgetReport {
            policy_id: policy.policy_id.clone(),
            policy_digest: policy.policy_digest.clone(),
            challenge_data_digest: policy.challenge_data_digest.clone(),
            challenge_manifest_digest: policy.challenge_manifest_digest.clone(),
            historical_nonsealed_releases: 0,
            projected_nonsealed_releases: 0,
            releases_remaining_after_request: 0,
            highest_historical_disclosure_tier: FeedbackDisclosureTier::SealedNoFeedback,
            requested_disclosure_tier: next_request.disclosure_tier,
            maximum_disclosure_tier: policy.maximum_disclosure_tier,
            idempotent_replay: false,
            status: FeedbackBudgetStatus::NotAssessed,
            reason: FeedbackBudgetReason::InvalidOrIncompleteCustodyEvidence,
            validation_error: Some(error),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> FeedbackBudgetPolicy {
        FeedbackBudgetPolicy {
            policy_id: "benchmark-feedback-v1".to_string(),
            policy_digest: "sha256-feedback-policy-v1".to_string(),
            challenge_data_digest: "sha256-sealed-challenge-data".to_string(),
            challenge_manifest_digest: "sha256-sealed-challenge-manifest".to_string(),
            evaluator_protocol_digest: "sha256-evaluator-protocol-v1".to_string(),
            metric_registry_digest: "sha256-metric-registry-v1".to_string(),
            maximum_nonsealed_releases: 2,
            maximum_disclosure_tier: FeedbackDisclosureTier::AggregateScore,
            preregistration_receipt_id: "feedback-preregistration-001".to_string(),
            freeze_event_digest: "sha256-feedback-freeze-event".to_string(),
        }
    }

    fn event(id: &str, sequence: u64, tier: FeedbackDisclosureTier) -> FeedbackReleaseEvent {
        let visible = tier.is_developer_visible();
        FeedbackReleaseEvent {
            event_id: id.to_string(),
            receipt_id: format!("receipt-{id}"),
            policy_digest: "sha256-feedback-policy-v1".to_string(),
            model_digest: format!("sha256-model-{id}"),
            model_revision_digest: format!("sha256-model-revision-{id}"),
            challenge_data_digest: "sha256-sealed-challenge-data".to_string(),
            challenge_manifest_digest: "sha256-sealed-challenge-manifest".to_string(),
            evaluator_protocol_digest: "sha256-evaluator-protocol-v1".to_string(),
            metric_registry_digest: "sha256-metric-registry-v1".to_string(),
            disclosure_tier: tier,
            release_artifact_digest: visible.then(|| format!("sha256-release-{id}")),
            recipient_id: visible.then(|| "developer-audience".to_string()),
            sequence,
            event_digest: format!("sha256-event-{id}"),
        }
    }

    #[test]
    fn sealed_evaluation_does_not_consume_feedback_budget() {
        let sealed = event("sealed-1", 1, FeedbackDisclosureTier::SealedNoFeedback);
        let next = event("binary-1", 2, FeedbackDisclosureTier::BinaryDecision);
        let report = evaluate_feedback_budget(&policy(), &[sealed], &next)
            .expect("sealed history and binary release");
        assert_eq!(
            report.status,
            FeedbackBudgetStatus::EligibleWithinFrozenFeedbackBudget
        );
        assert_eq!(report.historical_nonsealed_releases, 0);
        assert_eq!(report.projected_nonsealed_releases, 1);
        assert_eq!(report.releases_remaining_after_request, 1);
    }

    #[test]
    fn within_budget_binary_and_aggregate_releases_are_allowed() {
        let history = [event("binary-1", 1, FeedbackDisclosureTier::BinaryDecision)];
        let next = event("aggregate-2", 2, FeedbackDisclosureTier::AggregateScore);
        let report = evaluate_feedback_budget(&policy(), &history, &next)
            .expect("within count and tier budget");
        assert_eq!(
            report.status,
            FeedbackBudgetStatus::EligibleWithinFrozenFeedbackBudget
        );
        assert_eq!(report.historical_nonsealed_releases, 1);
        assert_eq!(report.projected_nonsealed_releases, 2);
        assert_eq!(report.releases_remaining_after_request, 0);
    }

    #[test]
    fn repeated_feedback_exhausts_count_budget_even_when_raw_data_stays_sealed() {
        let history = [
            event("binary-1", 1, FeedbackDisclosureTier::BinaryDecision),
            event("aggregate-2", 2, FeedbackDisclosureTier::AggregateScore),
        ];
        let next = event("binary-3", 3, FeedbackDisclosureTier::BinaryDecision);
        let report = evaluate_feedback_budget(&policy(), &history, &next)
            .expect("exhausted release budget is a reportable denial");
        assert_eq!(report.status, FeedbackBudgetStatus::FeedbackBudgetExhausted);
        assert_eq!(report.historical_nonsealed_releases, 2);
        assert_eq!(report.projected_nonsealed_releases, 2);
        assert_eq!(report.releases_remaining_after_request, 0);
        assert_eq!(report.reason, FeedbackBudgetReason::CountBudgetExhausted);
    }

    #[test]
    fn multi_metric_profile_exceeds_an_aggregate_only_disclosure_budget() {
        let history = [event(
            "aggregate-1",
            1,
            FeedbackDisclosureTier::AggregateScore,
        )];
        let next = event("profile-2", 2, FeedbackDisclosureTier::MultiMetricProfile);
        let report = evaluate_feedback_budget(&policy(), &history, &next)
            .expect("higher disclosure level is a reportable denial");
        assert_eq!(report.status, FeedbackBudgetStatus::DisclosureTierExceeded);
        assert_eq!(report.reason, FeedbackBudgetReason::DisclosureTierExceeded);
        assert_eq!(report.projected_nonsealed_releases, 1);
    }

    #[test]
    fn per_statistic_and_raw_diagnostics_exceed_the_frozen_tier() {
        for (id, tier) in [
            ("per-statistic", FeedbackDisclosureTier::PerStatisticResults),
            ("raw", FeedbackDisclosureTier::RawDiagnostics),
        ] {
            let next = event(id, 1, tier);
            let report =
                evaluate_feedback_budget(&policy(), &[], &next).expect("detail tier denial");
            assert_eq!(report.status, FeedbackBudgetStatus::DisclosureTierExceeded);
        }
    }

    #[test]
    fn exact_event_replay_is_idempotent_and_does_not_spend_another_release() {
        let first = event("binary-1", 1, FeedbackDisclosureTier::BinaryDecision);
        let second = event("aggregate-2", 2, FeedbackDisclosureTier::AggregateScore);
        let history = [first.clone(), second];
        let report = evaluate_feedback_budget(&policy(), &history, &first)
            .expect("exact release replay is idempotent");
        assert!(report.idempotent_replay);
        assert_eq!(
            report.status,
            FeedbackBudgetStatus::EligibleWithinFrozenFeedbackBudget
        );
        assert_eq!(report.historical_nonsealed_releases, 2);
        assert_eq!(report.projected_nonsealed_releases, 2);
    }

    #[test]
    fn policy_substitution_or_post_hoc_budget_loosening_is_rejected() {
        let history = [event("binary-1", 1, FeedbackDisclosureTier::BinaryDecision)];
        let next = event("binary-2", 2, FeedbackDisclosureTier::BinaryDecision);
        let mut changed = policy();
        changed.policy_digest = "sha256-loosened-policy".to_string();
        assert_eq!(
            evaluate_feedback_budget(&changed, &history, &next).unwrap_err(),
            FeedbackBudgetError::PolicyDigestMismatch
        );
    }

    #[test]
    fn mismatched_challenge_protocol_and_metric_registry_are_rejected() {
        let mut next = event("binary-1", 1, FeedbackDisclosureTier::BinaryDecision);
        next.challenge_data_digest = "sha256-other-data".to_string();
        assert_eq!(
            evaluate_feedback_budget(&policy(), &[], &next).unwrap_err(),
            FeedbackBudgetError::ChallengeIdentityMismatch
        );
        next.challenge_data_digest = policy().challenge_data_digest;
        next.evaluator_protocol_digest = "sha256-other-protocol".to_string();
        assert_eq!(
            evaluate_feedback_budget(&policy(), &[], &next).unwrap_err(),
            FeedbackBudgetError::EvaluatorProtocolMismatch
        );
        next.evaluator_protocol_digest = policy().evaluator_protocol_digest;
        next.metric_registry_digest = "sha256-other-metrics".to_string();
        assert_eq!(
            evaluate_feedback_budget(&policy(), &[], &next).unwrap_err(),
            FeedbackBudgetError::MetricRegistryMismatch
        );
    }

    #[test]
    fn sealed_events_must_not_bind_any_developer_visible_artifact_or_recipient() {
        let mut sealed = event("sealed-1", 1, FeedbackDisclosureTier::SealedNoFeedback);
        sealed.release_artifact_digest = Some("sha256-leaked-artifact".to_string());
        assert_eq!(
            evaluate_feedback_budget(&policy(), &[], &sealed).unwrap_err(),
            FeedbackBudgetError::SealedEventHasReleaseArtifact
        );

        sealed.release_artifact_digest = None;
        sealed.recipient_id = Some("developer".to_string());
        assert_eq!(
            evaluate_feedback_budget(&policy(), &[], &sealed).unwrap_err(),
            FeedbackBudgetError::SealedEventHasRecipient
        );
    }

    #[test]
    fn duplicate_event_receipt_artifact_and_digest_identities_are_rejected() {
        let first = event("binary-1", 1, FeedbackDisclosureTier::BinaryDecision);
        let mut duplicate = event("binary-1", 2, FeedbackDisclosureTier::BinaryDecision);
        assert!(matches!(
            evaluate_feedback_budget(
                &policy(),
                &[first.clone(), duplicate.clone()],
                &event("next", 3, FeedbackDisclosureTier::BinaryDecision)
            ),
            Err(FeedbackBudgetError::DuplicateEventId { .. })
        ));
        duplicate.event_id = "binary-2".to_string();
        duplicate.sequence = 2;
        duplicate.receipt_id = first.receipt_id.clone();
        assert!(matches!(
            evaluate_feedback_budget(
                &policy(),
                &[first.clone(), duplicate.clone()],
                &event("next", 3, FeedbackDisclosureTier::BinaryDecision)
            ),
            Err(FeedbackBudgetError::DuplicateReceiptId { .. })
        ));
        duplicate.receipt_id = "receipt-binary-2".to_string();
        duplicate.release_artifact_digest = first.release_artifact_digest.clone();
        assert!(matches!(
            evaluate_feedback_budget(
                &policy(),
                &[first.clone(), duplicate.clone()],
                &event("next", 3, FeedbackDisclosureTier::BinaryDecision)
            ),
            Err(FeedbackBudgetError::DuplicateReleaseArtifact { .. })
        ));
        duplicate.release_artifact_digest = Some("sha256-other-release".to_string());
        duplicate.event_digest = first.event_digest.clone();
        assert!(matches!(
            evaluate_feedback_budget(
                &policy(),
                &[first, duplicate],
                &event("next", 3, FeedbackDisclosureTier::BinaryDecision)
            ),
            Err(FeedbackBudgetError::DuplicateEventDigest { .. })
        ));
    }

    #[test]
    fn event_sequence_must_increase_and_event_id_cannot_be_reused_for_new_contents() {
        let first = event("binary-1", 2, FeedbackDisclosureTier::BinaryDecision);
        let out_of_order = event("binary-2", 1, FeedbackDisclosureTier::BinaryDecision);
        assert_eq!(
            evaluate_feedback_budget(
                &policy(),
                &[first.clone(), out_of_order],
                &event("next", 3, FeedbackDisclosureTier::BinaryDecision)
            )
            .unwrap_err(),
            FeedbackBudgetError::EventSequenceNotIncreasing
        );

        let changed_replay = event("binary-1", 2, FeedbackDisclosureTier::AggregateScore);
        assert_eq!(
            evaluate_feedback_budget(&policy(), &[first], &changed_replay).unwrap_err(),
            FeedbackBudgetError::EventAlreadyExistsWithDifferentContents {
                event_id: "binary-1".to_string()
            }
        );
    }

    #[test]
    fn historical_policy_violations_remain_visible_and_do_not_authorize_more_feedback() {
        let mut restricted = policy();
        restricted.maximum_disclosure_tier = FeedbackDisclosureTier::BinaryDecision;
        let history = [event(
            "profile-1",
            1,
            FeedbackDisclosureTier::MultiMetricProfile,
        )];
        let next = event("binary-2", 2, FeedbackDisclosureTier::BinaryDecision);
        let report = evaluate_feedback_budget(&restricted, &history, &next)
            .expect("historical release violation is reportable");
        assert_eq!(report.status, FeedbackBudgetStatus::DisclosureTierExceeded);
        assert_eq!(
            report.reason,
            FeedbackBudgetReason::HistoricalDisclosureTierViolation
        );
    }

    #[test]
    fn malformed_custody_evidence_is_explicitly_not_assessed() {
        let mut malformed = event("binary-1", 1, FeedbackDisclosureTier::BinaryDecision);
        malformed.recipient_id = None;
        let report = assess_feedback_budget(&policy(), &[], &malformed);
        assert_eq!(report.status, FeedbackBudgetStatus::NotAssessed);
        assert_eq!(
            report.reason,
            FeedbackBudgetReason::InvalidOrIncompleteCustodyEvidence
        );
        assert_eq!(
            report.validation_error,
            Some(FeedbackBudgetError::VisibleEventMissingRecipient)
        );
        assert_eq!(report.releases_remaining_after_request, 0);
    }

    #[test]
    fn visible_event_requires_release_artifact_and_recipient() {
        let mut visible = event("binary-1", 1, FeedbackDisclosureTier::BinaryDecision);
        visible.release_artifact_digest = None;
        assert_eq!(
            evaluate_feedback_budget(&policy(), &[], &visible).unwrap_err(),
            FeedbackBudgetError::VisibleEventMissingReleaseArtifact
        );
        visible.release_artifact_digest = Some("sha256-release".to_string());
        visible.recipient_id = None;
        assert_eq!(
            evaluate_feedback_budget(&policy(), &[], &visible).unwrap_err(),
            FeedbackBudgetError::VisibleEventMissingRecipient
        );
    }
}
