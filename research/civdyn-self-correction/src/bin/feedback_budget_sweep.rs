// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::feedback_budget::{
    evaluate_feedback_budget, FeedbackBudgetPolicy, FeedbackBudgetReport,
    FeedbackDisclosureTier, FeedbackReleaseEvent,
};

const CSV_COLUMNS: usize = 27;

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "feedback budget receipt must match the frozen schema"
    );
    println!(
        "{}",
        fields
            .iter()
            .map(|field| csv_field(field))
            .collect::<Vec<_>>()
            .join(",")
    );
}

fn policy() -> FeedbackBudgetPolicy {
    FeedbackBudgetPolicy {
        policy_id: "civdyn-feedback-budget-v1".to_string(),
        policy_digest: "sha256-feedback-budget-policy-v1".to_string(),
        challenge_data_digest: "sha256-sealed-challenge-data-v1".to_string(),
        challenge_manifest_digest: "sha256-sealed-challenge-manifest-v1".to_string(),
        evaluator_protocol_digest: "sha256-feedback-evaluator-v1".to_string(),
        metric_registry_digest: "sha256-feedback-metrics-v1".to_string(),
        maximum_nonsealed_releases: 2,
        maximum_disclosure_tier: FeedbackDisclosureTier::AggregateScore,
        preregistration_receipt_id: "feedback-preregistration-v1".to_string(),
        freeze_event_digest: "sha256-feedback-freeze-v1".to_string(),
    }
}

fn event(
    id: &str,
    sequence: u64,
    tier: FeedbackDisclosureTier,
) -> FeedbackReleaseEvent {
    let visible = tier != FeedbackDisclosureTier::SealedNoFeedback;
    FeedbackReleaseEvent {
        event_id: id.to_string(),
        receipt_id: format!("receipt-{id}"),
        policy_digest: "sha256-feedback-budget-policy-v1".to_string(),
        model_digest: format!("sha256-model-{id}"),
        model_revision_digest: format!("sha256-model-revision-{id}"),
        challenge_data_digest: "sha256-sealed-challenge-data-v1".to_string(),
        challenge_manifest_digest: "sha256-sealed-challenge-manifest-v1".to_string(),
        evaluator_protocol_digest: "sha256-feedback-evaluator-v1".to_string(),
        metric_registry_digest: "sha256-feedback-metrics-v1".to_string(),
        disclosure_tier: tier,
        release_artifact_digest: visible.then(|| format!("sha256-release-{id}")),
        recipient_id: visible.then(|| "developer-audience".to_string()),
        sequence,
        event_digest: format!("sha256-feedback-event-{id}"),
    }
}

fn emit_case(
    scenario: &str,
    policy: &FeedbackBudgetPolicy,
    history: &[FeedbackReleaseEvent],
    next: &FeedbackReleaseEvent,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = evaluate_feedback_budget(policy, history, next)?;
    emit_result(scenario, policy, history.len(), next, &result);
    Ok(())
}

fn emit_result(
    scenario: &str,
    policy: &FeedbackBudgetPolicy,
    historical_event_count: usize,
    next: &FeedbackReleaseEvent,
    result: &FeedbackBudgetReport,
) {
    emit(vec![
        scenario.to_string(),
        result.status.name().to_string(),
        result.reason.name().to_string(),
        result.historical_nonsealed_releases.to_string(),
        result.projected_nonsealed_releases.to_string(),
        result.releases_remaining_after_request.to_string(),
        result.highest_historical_disclosure_tier.name().to_string(),
        result.requested_disclosure_tier.name().to_string(),
        result.maximum_disclosure_tier.name().to_string(),
        result.idempotent_replay.to_string(),
        policy.policy_id.clone(),
        policy.policy_digest.clone(),
        policy.challenge_data_digest.clone(),
        policy.challenge_manifest_digest.clone(),
        policy.evaluator_protocol_digest.clone(),
        policy.metric_registry_digest.clone(),
        policy.preregistration_receipt_id.clone(),
        policy.freeze_event_digest.clone(),
        historical_event_count.to_string(),
        next.event_id.clone(),
        next.receipt_id.clone(),
        next.model_digest.clone(),
        next.model_revision_digest.clone(),
        next.release_artifact_digest.clone().unwrap_or_default(),
        next.recipient_id.clone().unwrap_or_default(),
        next.sequence.to_string(),
        next.event_digest.clone(),
    ]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let policy = policy();
    println!(
        "scenario,status,reason,historical_nonsealed_releases,projected_nonsealed_releases,releases_remaining_after_request,highest_historical_disclosure_tier,requested_disclosure_tier,maximum_disclosure_tier,idempotent_replay,policy_id,policy_digest,challenge_data_digest,challenge_manifest_digest,evaluator_protocol_digest,metric_registry_digest,preregistration_receipt_id,freeze_event_digest,historical_event_count,next_event_id,next_receipt_id,next_model_digest,next_model_revision_digest,next_release_artifact_digest,next_recipient_id,next_event_sequence,next_event_digest"
    );

    emit_case(
        "sealed_then_binary",
        &policy,
        &[event("sealed-1", 1, FeedbackDisclosureTier::SealedNoFeedback)],
        &event("binary-2", 2, FeedbackDisclosureTier::BinaryDecision),
    )?;
    emit_case(
        "within_budget_aggregate",
        &policy,
        &[event("binary-1", 1, FeedbackDisclosureTier::BinaryDecision)],
        &event("aggregate-2", 2, FeedbackDisclosureTier::AggregateScore),
    )?;
    emit_case(
        "count_budget_exhausted",
        &policy,
        &[
            event("binary-1", 1, FeedbackDisclosureTier::BinaryDecision),
            event("aggregate-2", 2, FeedbackDisclosureTier::AggregateScore),
        ],
        &event("binary-3", 3, FeedbackDisclosureTier::BinaryDecision),
    )?;
    emit_case(
        "multi_metric_tier_exceeded",
        &policy,
        &[event("aggregate-1", 1, FeedbackDisclosureTier::AggregateScore)],
        &event("profile-2", 2, FeedbackDisclosureTier::MultiMetricProfile),
    )?;
    emit_case(
        "per_statistic_tier_exceeded",
        &policy,
        &[],
        &event("per-statistic-1", 1, FeedbackDisclosureTier::PerStatisticResults),
    )?;
    emit_case(
        "raw_diagnostics_tier_exceeded",
        &policy,
        &[],
        &event("raw-1", 1, FeedbackDisclosureTier::RawDiagnostics),
    )?;

    let first = event("binary-replay-1", 1, FeedbackDisclosureTier::BinaryDecision);
    let second = event("aggregate-replay-2", 2, FeedbackDisclosureTier::AggregateScore);
    emit_case(
        "exact_idempotent_replay",
        &policy,
        &[first.clone(), second],
        &first,
    )?;
    Ok(())
}
