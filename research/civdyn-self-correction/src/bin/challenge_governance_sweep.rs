// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::challenge_governance::{
    qualify_challenge_use, ChallengeEligibilityReceipt, ChallengeEvaluationEvent,
    ChallengeLedgerSnapshot, ChallengeSetManifest, ChallengeVisibility, FeedbackDisclosure,
    ModelLineage, ModelRevision,
};

const FAMILY: &str = "civdyn-social-model-family";
const CSV_COLUMNS: usize = 17;

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn emit(fields: Vec<String>) {
    assert_eq!(fields.len(), CSV_COLUMNS);
    println!(
        "{}",
        fields
            .iter()
            .map(|field| csv_field(field))
            .collect::<Vec<_>>()
            .join(",")
    );
}

fn manifest(generation: &str, visibility: ChallengeVisibility) -> ChallengeSetManifest {
    ChallengeSetManifest {
        challenge_set_id: "civdyn-network-challenge".to_string(),
        generation_id: generation.to_string(),
        challenge_set_digest: format!("sha256-{generation}-challenge"),
        source_dataset_digest: format!("sha256-{generation}-source"),
        split_manifest_digest: format!("sha256-{generation}-split"),
        statistic_registry_version: "civdyn-registry-v1".to_string(),
        evaluator_digest: "sha256-civdyn-evaluator-v1".to_string(),
        visibility,
    }
}

fn lineage() -> ModelLineage {
    ModelLineage {
        revisions: vec![
            ModelRevision {
                model_family_id: FAMILY.to_string(),
                revision_sequence: 0,
                model_digest: "sha256-model-revision-0".to_string(),
                parent_model_digest: None,
            },
            ModelRevision {
                model_family_id: FAMILY.to_string(),
                revision_sequence: 1,
                model_digest: "sha256-model-revision-1".to_string(),
                parent_model_digest: Some("sha256-model-revision-0".to_string()),
            },
            ModelRevision {
                model_family_id: FAMILY.to_string(),
                revision_sequence: 2,
                model_digest: "sha256-model-revision-2".to_string(),
                parent_model_digest: Some("sha256-model-revision-1".to_string()),
            },
        ],
        candidate_model_digest: "sha256-model-revision-2".to_string(),
    }
}

fn event(
    challenge: &ChallengeSetManifest,
    run_id: &str,
    model_family_id: &str,
    revision_sequence: u64,
    model_digest: &str,
    disclosure: FeedbackDisclosure,
    recipient: Option<&str>,
) -> ChallengeEvaluationEvent {
    ChallengeEvaluationEvent {
        ledger_sequence: 1,
        run_id: run_id.to_string(),
        challenge_set_id: challenge.challenge_set_id.clone(),
        generation_id: challenge.generation_id.clone(),
        challenge_set_digest: challenge.challenge_set_digest.clone(),
        source_dataset_digest: challenge.source_dataset_digest.clone(),
        split_manifest_digest: challenge.split_manifest_digest.clone(),
        statistic_registry_version: challenge.statistic_registry_version.clone(),
        evaluator_digest: challenge.evaluator_digest.clone(),
        model_family_id: model_family_id.to_string(),
        revision_sequence,
        model_digest: model_digest.to_string(),
        feedback_disclosure: disclosure,
        feedback_recipient_family_id: recipient.map(str::to_string),
        receipt_digest: format!("sha256-{run_id}-receipt"),
    }
}

fn snapshot(events: Vec<ChallengeEvaluationEvent>) -> ChallengeLedgerSnapshot {
    ChallengeLedgerSnapshot {
        authority_id: "mycelix-challenge-ledger".to_string(),
        authority_digest: "sha256-ledger-authority".to_string(),
        snapshot_digest: "sha256-ledger-snapshot".to_string(),
        snapshot_complete: true,
        complete_through_sequence: events.len() as u64,
        events,
    }
}

fn emit_result(
    scenario: &str,
    challenge: &ChallengeSetManifest,
    receipt: &ChallengeEligibilityReceipt,
) {
    emit(vec![
        scenario.to_string(),
        challenge.challenge_set_id.clone(),
        challenge.generation_id.clone(),
        challenge.visibility.name().to_string(),
        receipt.model_family_id.clone(),
        receipt.candidate_model_digest.clone(),
        receipt.candidate_revision_sequence.to_string(),
        receipt.eligibility.name().to_string(),
        receipt.confirmatory_claim_permitted.to_string(),
        receipt.evaluation_feedback_permitted.to_string(),
        receipt.evaluated_event_count.to_string(),
        receipt.ledger_authority_id.clone(),
        receipt.ledger_authority_digest.clone(),
        receipt.ledger_snapshot_digest.clone(),
        receipt.challenge_set_digest.clone(),
        receipt.split_manifest_digest.clone(),
        receipt.evaluator_digest.clone(),
    ]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "scenario,challenge_set_id,generation_id,visibility,model_family_id,candidate_model_digest,candidate_revision_sequence,eligibility,confirmatory_claim_permitted,evaluation_feedback_permitted,evaluated_event_count,ledger_authority_id,ledger_authority_digest,ledger_snapshot_digest,challenge_set_digest,split_manifest_digest,evaluator_digest"
    );

    let challenge = manifest("generation-001", ChallengeVisibility::SealedPrivate);
    let candidate_lineage = lineage();

    let fresh = qualify_challenge_use(&challenge, &snapshot(Vec::new()), &candidate_lineage)?;
    emit_result("fresh_empty_ledger", &challenge, &fresh);

    let no_feedback = snapshot(vec![event(
        &challenge,
        "run-no-feedback",
        FAMILY,
        0,
        "sha256-model-revision-0",
        FeedbackDisclosure::None,
        None,
    )]);
    let sealed = qualify_challenge_use(&challenge, &no_feedback, &candidate_lineage)?;
    emit_result("prior_no_feedback", &challenge, &sealed);

    let aggregate_feedback = snapshot(vec![event(
        &challenge,
        "run-aggregate",
        FAMILY,
        0,
        "sha256-model-revision-0",
        FeedbackDisclosure::AggregateStatus,
        Some(FAMILY),
    )]);
    let aggregate = qualify_challenge_use(&challenge, &aggregate_feedback, &candidate_lineage)?;
    emit_result("prior_aggregate_feedback", &challenge, &aggregate);

    let full_feedback = snapshot(vec![event(
        &challenge,
        "run-full",
        FAMILY,
        0,
        "sha256-model-revision-0",
        FeedbackDisclosure::FullMetrics,
        Some(FAMILY),
    )]);
    let full = qualify_challenge_use(&challenge, &full_feedback, &candidate_lineage)?;
    emit_result("prior_full_feedback", &challenge, &full);

    let same_model = snapshot(vec![event(
        &challenge,
        "run-same-model",
        FAMILY,
        2,
        "sha256-model-revision-2",
        FeedbackDisclosure::None,
        None,
    )]);
    let duplicate = qualify_challenge_use(&challenge, &same_model, &candidate_lineage)?;
    emit_result("same_model_duplicate", &challenge, &duplicate);

    let public_disclosure = snapshot(vec![event(
        &challenge,
        "run-public-feedback",
        FAMILY,
        0,
        "sha256-model-revision-0",
        FeedbackDisclosure::FullMetrics,
        None,
    )]);
    let public_feedback =
        qualify_challenge_use(&challenge, &public_disclosure, &candidate_lineage)?;
    emit_result("public_feedback", &challenge, &public_feedback);

    let public_challenge = manifest("public-generation", ChallengeVisibility::PublicExploratory);
    let exploratory = qualify_challenge_use(
        &public_challenge,
        &snapshot(Vec::new()),
        &candidate_lineage,
    )?;
    emit_result("public_benchmark", &public_challenge, &exploratory);

    let mut incomplete_snapshot = snapshot(Vec::new());
    incomplete_snapshot.snapshot_complete = false;
    incomplete_snapshot.complete_through_sequence = 4;
    let incomplete =
        qualify_challenge_use(&challenge, &incomplete_snapshot, &candidate_lineage)?;
    emit_result("incomplete_ledger", &challenge, &incomplete);

    let new_challenge = manifest("generation-002", ChallengeVisibility::SealedPrivate);
    let new_generation =
        qualify_challenge_use(&new_challenge, &snapshot(Vec::new()), &candidate_lineage)?;
    emit_result("fresh_generation", &new_challenge, &new_generation);

    let unrelated_private = snapshot(vec![event(
        &challenge,
        "run-other-team",
        "independent-team",
        0,
        "sha256-independent-model",
        FeedbackDisclosure::FullMetrics,
        Some("independent-team"),
    )]);
    let other_family =
        qualify_challenge_use(&challenge, &unrelated_private, &candidate_lineage)?;
    emit_result("private_other_family", &challenge, &other_family);

    Ok(())
}
