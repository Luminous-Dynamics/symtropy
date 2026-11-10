// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::challenge_custody::{
    validate_challenge_eligibility, ChallengeEvaluationReceipt, ChallengeRequest,
    ModelRevision,
};

const CSV_COLUMNS: usize = 10;

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
        "challenge custody receipt must match the frozen schema"
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

fn receipt() -> ChallengeEvaluationReceipt {
    ChallengeEvaluationReceipt {
        receipt_id: "receipt-c0-model-0".to_string(),
        model_digest: "sha256-model-0".to_string(),
        challenge_data_digest: "sha256-challenge-c0".to_string(),
        challenge_manifest_digest: "sha256-manifest-c0".to_string(),
        evaluation_protocol_digest: "sha256-evaluator-v1".to_string(),
        result_digest: "sha256-result-c0-model-0".to_string(),
        result_disclosed_to_developers: true,
    }
}

fn request(model: &str, data: &str, manifest: &str) -> ChallengeRequest {
    ChallengeRequest {
        candidate_model_digest: model.to_string(),
        challenge_data_digest: data.to_string(),
        challenge_manifest_digest: manifest.to_string(),
        evaluation_protocol_digest: "sha256-evaluator-v1".to_string(),
    }
}

fn emit_case(
    scenario: &str,
    revisions: &[ModelRevision],
    receipts: &[ChallengeEvaluationReceipt],
    request: &ChallengeRequest,
) {
    match validate_challenge_eligibility(revisions, receipts, request) {
        Ok(report) => emit(vec![
            scenario.to_string(),
            "eligible".to_string(),
            report.decision.name().to_string(),
            String::new(),
            report.candidate_model_digest,
            report.challenge_data_digest,
            report.challenge_manifest_digest,
            report.evaluation_protocol_digest,
            report.revision_count.to_string(),
            report.unique_consumed_evidence_count.to_string(),
        ]),
        Err(error) => emit(vec![
            scenario.to_string(),
            "rejected".to_string(),
            String::new(),
            format!("{error:?}"),
            request.candidate_model_digest.clone(),
            request.challenge_data_digest.clone(),
            request.challenge_manifest_digest.clone(),
            request.evaluation_protocol_digest.clone(),
            revisions.len().to_string(),
            String::new(),
        ]),
    }
}

fn main() {
    println!(
        "scenario,eligibility,decision,rejection_reason,candidate_model_digest,challenge_data_digest,challenge_manifest_digest,evaluation_protocol_digest,revision_count,unique_consumed_evidence_count"
    );

    let model_0 = vec![revision(
        "revision-0",
        "sha256-model-0",
        None,
        &["sha256-development-data-0", "sha256-calibration-data-0"],
    )];
    let receipts = vec![receipt()];

    let model_1 = vec![
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

    emit_case(
        "fresh-first-challenge",
        &model_0,
        &[],
        &request(
            "sha256-model-0",
            "sha256-challenge-c0",
            "sha256-manifest-c0",
        ),
    );
    emit_case(
        "exact-idempotent-replay",
        &model_0,
        &receipts,
        &request(
            "sha256-model-0",
            "sha256-challenge-c0",
            "sha256-manifest-c0",
        ),
    );
    emit_case(
        "retired-disclosed-challenge-after-repair",
        &model_1,
        &receipts,
        &request(
            "sha256-model-1",
            "sha256-challenge-c0",
            "sha256-manifest-c0",
        ),
    );
    emit_case(
        "fresh-post-repair-challenge",
        &model_1,
        &receipts,
        &request(
            "sha256-model-1",
            "sha256-challenge-c1",
            "sha256-manifest-c1",
        ),
    );
    emit_case(
        "challenge-consumed-by-repair",
        &[
            revision(
                "revision-0",
                "sha256-model-0",
                None,
                &["sha256-development-data-0"],
            ),
            revision(
                "revision-1",
                "sha256-model-1",
                Some("sha256-model-0"),
                &["sha256-challenge-c2"],
            ),
        ],
        &[],
        &request(
            "sha256-model-1",
            "sha256-challenge-c2",
            "sha256-manifest-c2",
        ),
    );
}
