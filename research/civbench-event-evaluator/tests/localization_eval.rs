// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civbench_event_evaluator::localization_eval::{
    evaluate_localization, AbstentionReason, LocalizationDecision, LocalizationLevel,
    LocalizationPrediction, LocalizationTruth, ObservabilityAssessment, TruthEvidenceClass,
    TruthLabel,
};

fn digest(value: u64) -> String {
    format!("sha256:{value:064x}")
}

fn known(value: &str, evidence: TruthEvidenceClass) -> TruthLabel {
    TruthLabel::Known {
        value: value.to_owned(),
        evidence,
    }
}

fn truth(event_id: &str, zone: TruthLabel, pipe: TruthLabel) -> LocalizationTruth {
    LocalizationTruth {
        event_id: event_id.to_owned(),
        zone,
        pipe,
    }
}

fn assessment(
    event_id: &str,
    maximum_level: LocalizationLevel,
    zones: &[&str],
    pipes: &[&str],
    digest_value: u64,
) -> ObservabilityAssessment {
    ObservabilityAssessment {
        event_id: event_id.to_owned(),
        maximum_level,
        zone_candidate_ids: zones.iter().map(|value| (*value).to_owned()).collect(),
        pipe_candidate_ids: pipes.iter().map(|value| (*value).to_owned()).collect(),
        sensor_layout_ref: "sensor-layout-v1".to_owned(),
        hydraulic_model_ref: "hydraulic-model-v1".to_owned(),
        assessment_digest: digest(digest_value),
    }
}

fn ranked(
    event_id: &str,
    level: LocalizationLevel,
    candidates: &[&str],
    confidence_ppm: u32,
) -> LocalizationPrediction {
    LocalizationPrediction {
        event_id: event_id.to_owned(),
        policy_ref: "localizer-policy-v1".to_owned(),
        decision: LocalizationDecision::Ranked {
            level,
            candidate_ids: candidates.iter().map(|value| (*value).to_owned()).collect(),
            confidence_ppm,
        },
    }
}

fn abstain(event_id: &str) -> LocalizationPrediction {
    LocalizationPrediction {
        event_id: event_id.to_owned(),
        policy_ref: "localizer-policy-v1".to_owned(),
        decision: LocalizationDecision::Abstain {
            reason: AbstentionReason::NotIdentifiable,
        },
    }
}

#[test]
fn zone_top_k_metrics_and_risk_coverage_are_reported() {
    let truths = vec![truth(
        "event-1",
        known("zone-a", TruthEvidenceClass::Observed),
        TruthLabel::NotMeasured,
    )];
    let assessments = vec![assessment(
        "event-1",
        LocalizationLevel::Zone,
        &["zone-a", "zone-b"],
        &[],
        1,
    )];
    let predictions = vec![ranked(
        "event-1",
        LocalizationLevel::Zone,
        &["zone-a", "zone-b"],
        950_000,
    )];

    let report = evaluate_localization(&truths, &assessments, &predictions)
        .expect("valid zone localization");
    assert_eq!(report.observability.zone, 1);
    assert_eq!(report.zone.observed.eligible_events, 1);
    assert_eq!(report.zone.observed.localized_events, 1);
    assert_eq!(report.zone.observed.top1_hits, 1);
    assert_eq!(report.zone.observed.top5_hits, 1);
    assert_eq!(report.zone.observed.coverage_ppm, Some(1_000_000));
    assert_eq!(report.zone.observed.conditional_top1_accuracy_ppm, Some(1_000_000));
    assert_eq!(report.zone.observed.system_top1_accuracy_ppm, Some(1_000_000));
    assert_eq!(report.zone.observed.risk_coverage.len(), 1);
    assert_eq!(report.zone.observed.risk_coverage[0].risk_ppm, Some(0));
    assert_eq!(report.zone.observed.risk_coverage[0].coverage_ppm, Some(1_000_000));
    assert_eq!(report.pipe.not_measured_events, 1);
}

#[test]
fn pipe_top_k_is_scored_only_when_physical_assessment_supports_pipe_level() {
    let truths = vec![truth(
        "event-1",
        known("zone-a", TruthEvidenceClass::Observed),
        known("pipe-17", TruthEvidenceClass::Observed),
    )];
    let assessments = vec![assessment(
        "event-1",
        LocalizationLevel::Pipe,
        &["zone-a"],
        &["pipe-17", "pipe-18", "pipe-19"],
        2,
    )];
    let predictions = vec![ranked(
        "event-1",
        LocalizationLevel::Pipe,
        &["pipe-17", "pipe-18", "pipe-19"],
        990_000,
    )];

    let report = evaluate_localization(&truths, &assessments, &predictions)
        .expect("valid pipe localization");
    assert_eq!(report.pipe.observed.eligible_events, 1);
    assert_eq!(report.pipe.observed.localized_events, 1);
    assert_eq!(report.pipe.observed.top1_hits, 1);
    assert_eq!(report.pipe.observed.top5_hits, 1);
    assert_eq!(report.zone.observed.more_specific_predictions, 1);
}

#[test]
fn pipe_prediction_beyond_observability_ceiling_is_not_credited() {
    let truths = vec![truth(
        "event-1",
        known("zone-a", TruthEvidenceClass::Observed),
        known("pipe-17", TruthEvidenceClass::Observed),
    )];
    let assessments = vec![assessment(
        "event-1",
        LocalizationLevel::Zone,
        &["zone-a", "zone-b"],
        &[],
        3,
    )];
    let predictions = vec![ranked(
        "event-1",
        LocalizationLevel::Pipe,
        &["pipe-17"],
        990_000,
    )];

    let report = evaluate_localization(&truths, &assessments, &predictions)
        .expect("overclaim should be counted, not crash the evaluator");
    assert_eq!(report.unsupported_granularity_predictions, 1);
    assert_eq!(report.zone.observed.unsupported_granularity_predictions, 1);
    assert_eq!(report.zone.observed.localized_events, 0);
    assert_eq!(report.zone.observed.top1_hits, 0);
    assert_eq!(report.pipe.observed.eligible_events, 0);
    assert_eq!(report.pipe.observability_limited_events, 1);
}

#[test]
fn zone_prediction_is_coarse_when_pipe_localization_is_physically_supported() {
    let truths = vec![truth(
        "event-1",
        known("zone-a", TruthEvidenceClass::Observed),
        known("pipe-17", TruthEvidenceClass::Observed),
    )];
    let assessments = vec![assessment(
        "event-1",
        LocalizationLevel::Pipe,
        &["zone-a", "zone-b"],
        &["pipe-17", "pipe-18"],
        4,
    )];
    let predictions = vec![ranked(
        "event-1",
        LocalizationLevel::Zone,
        &["zone-a"],
        900_000,
    )];

    let report = evaluate_localization(&truths, &assessments, &predictions)
        .expect("zone prediction remains a valid coarse output");
    assert_eq!(report.zone.observed.top1_hits, 1);
    assert_eq!(report.pipe.observed.coarse_predictions, 1);
    assert_eq!(report.pipe.observed.localized_events, 0);
}

#[test]
fn abstention_preserves_zero_coverage_and_undefined_conditional_accuracy() {
    let truths = vec![truth(
        "event-1",
        known("zone-a", TruthEvidenceClass::Observed),
        known("pipe-17", TruthEvidenceClass::Observed),
    )];
    let assessments = vec![assessment(
        "event-1",
        LocalizationLevel::Pipe,
        &["zone-a"],
        &["pipe-17"],
        5,
    )];
    let predictions = vec![abstain("event-1")];

    let report = evaluate_localization(&truths, &assessments, &predictions)
        .expect("abstention is a valid decision");
    assert_eq!(report.zone.observed.eligible_events, 1);
    assert_eq!(report.zone.observed.abstained_events, 1);
    assert_eq!(report.zone.observed.coverage_ppm, Some(0));
    assert_eq!(report.zone.observed.conditional_top1_accuracy_ppm, None);
    assert_eq!(report.zone.observed.system_top1_accuracy_ppm, Some(0));
    assert_eq!(report.zone.observed.risk_coverage.len(), 0);
    assert_eq!(report.pipe.observed.abstained_events, 1);
}

#[test]
fn observed_derived_and_modelled_truth_are_never_pooled() {
    let truths = vec![
        truth(
            "observed-event",
            known("zone-a", TruthEvidenceClass::Observed),
            TruthLabel::NotApplicable,
        ),
        truth(
            "derived-event",
            known("zone-a", TruthEvidenceClass::Derived),
            TruthLabel::NotApplicable,
        ),
        truth(
            "modelled-event",
            known("zone-b", TruthEvidenceClass::Modelled),
            TruthLabel::NotApplicable,
        ),
    ];
    let assessments = vec![
        assessment("observed-event", LocalizationLevel::Zone, &["zone-a"], &[], 6),
        assessment("derived-event", LocalizationLevel::Zone, &["zone-a"], &[], 7),
        assessment("modelled-event", LocalizationLevel::Zone, &["zone-b"], &[], 8),
    ];
    let predictions = vec![
        ranked("observed-event", LocalizationLevel::Zone, &["zone-a"], 950_000),
        ranked("derived-event", LocalizationLevel::Zone, &["zone-b"], 900_000),
        ranked("modelled-event", LocalizationLevel::Zone, &["zone-b"], 850_000),
    ];

    let report = evaluate_localization(&truths, &assessments, &predictions)
        .expect("evidence classes stay separate");
    assert_eq!(report.zone.observed.eligible_events, 1);
    assert_eq!(report.zone.observed.top1_hits, 1);
    assert_eq!(report.zone.derived.eligible_events, 1);
    assert_eq!(report.zone.derived.top1_hits, 0);
    assert_eq!(report.zone.modelled.eligible_events, 1);
    assert_eq!(report.zone.modelled.top1_hits, 1);
}

#[test]
fn unmeasured_unsupported_withheld_and_not_applicable_truth_are_distinct() {
    let truths = vec![
        truth(
            "not-measured",
            TruthLabel::NotMeasured,
            TruthLabel::NotMeasured,
        ),
        truth(
            "unsupported",
            TruthLabel::Unsupported,
            TruthLabel::Unsupported,
        ),
        truth("withheld", TruthLabel::Withheld, TruthLabel::Withheld),
        truth(
            "not-applicable",
            TruthLabel::NotApplicable,
            TruthLabel::NotApplicable,
        ),
    ];
    let assessments = vec![
        assessment("not-measured", LocalizationLevel::DetectionOnly, &[], &[], 9),
        assessment("unsupported", LocalizationLevel::DetectionOnly, &[], &[], 10),
        assessment("withheld", LocalizationLevel::DetectionOnly, &[], &[], 11),
        assessment("not-applicable", LocalizationLevel::DetectionOnly, &[], &[], 12),
    ];
    let predictions = vec![
        abstain("not-measured"),
        abstain("unsupported"),
        abstain("withheld"),
        abstain("not-applicable"),
    ];
    let report = evaluate_localization(&truths, &assessments, &predictions)
        .expect("unscorable labels are retained");
    assert_eq!(report.zone.not_measured_events, 1);
    assert_eq!(report.zone.unsupported_truth_events, 1);
    assert_eq!(report.zone.withheld_truth_events, 1);
    assert_eq!(report.zone.not_applicable_events, 1);
    assert_eq!(report.pipe.not_measured_events, 1);
    assert_eq!(report.pipe.unsupported_truth_events, 1);
    assert_eq!(report.pipe.withheld_truth_events, 1);
    assert_eq!(report.pipe.not_applicable_events, 1);
}

#[test]
fn detection_only_ceiling_records_known_locations_as_observability_limited() {
    let truths = vec![truth(
        "event-1",
        known("zone-a", TruthEvidenceClass::Observed),
        known("pipe-17", TruthEvidenceClass::Observed),
    )];
    let assessments = vec![assessment(
        "event-1",
        LocalizationLevel::DetectionOnly,
        &[],
        &[],
        13,
    )];
    let predictions = vec![abstain("event-1")];

    let report = evaluate_localization(&truths, &assessments, &predictions)
        .expect("detection-only is a valid observability result");
    assert_eq!(report.observability.detection_only, 1);
    assert_eq!(report.zone.observability_limited_events, 1);
    assert_eq!(report.pipe.observability_limited_events, 1);
    assert_eq!(report.zone.observed.eligible_events, 0);
    assert_eq!(report.pipe.observed.eligible_events, 0);
}

#[test]
fn risk_coverage_groups_equal_confidence_thresholds() {
    let truths = vec![
        truth(
            "event-a",
            known("zone-a", TruthEvidenceClass::Observed),
            TruthLabel::NotApplicable,
        ),
        truth(
            "event-b",
            known("zone-b", TruthEvidenceClass::Observed),
            TruthLabel::NotApplicable,
        ),
        truth(
            "event-c",
            known("zone-c", TruthEvidenceClass::Observed),
            TruthLabel::NotApplicable,
        ),
        truth(
            "event-d",
            known("zone-d", TruthEvidenceClass::Observed),
            TruthLabel::NotApplicable,
        ),
    ];
    let assessments = vec![
        assessment("event-a", LocalizationLevel::Zone, &["zone-a"], &[], 14),
        assessment("event-b", LocalizationLevel::Zone, &["zone-b"], &[], 15),
        assessment("event-c", LocalizationLevel::Zone, &["zone-c"], &[], 16),
        assessment("event-d", LocalizationLevel::Zone, &["zone-d"], &[], 17),
    ];
    let predictions = vec![
        ranked("event-a", LocalizationLevel::Zone, &["zone-a"], 950_000),
        ranked("event-b", LocalizationLevel::Zone, &["wrong-b"], 900_000),
        ranked("event-c", LocalizationLevel::Zone, &["zone-c"], 900_000),
        ranked("event-d", LocalizationLevel::Zone, &["wrong-d"], 700_000),
    ];
    // Candidate universes include every candidate actually emitted.
    let mut assessments = assessments;
    assessments[1].zone_candidate_ids.push("wrong-b".to_owned());
    assessments[3].zone_candidate_ids.push("wrong-d".to_owned());

    let report = evaluate_localization(&truths, &assessments, &predictions)
        .expect("risk-coverage fixture");
    let curve = &report.zone.observed.risk_coverage;
    assert_eq!(curve.len(), 3);
    assert_eq!(curve[0].minimum_confidence_ppm, 950_000);
    assert_eq!(curve[0].covered_events, 1);
    assert_eq!(curve[0].coverage_ppm, Some(250_000));
    assert_eq!(curve[0].risk_ppm, Some(0));
    assert_eq!(curve[1].minimum_confidence_ppm, 900_000);
    assert_eq!(curve[1].covered_events, 3);
    assert_eq!(curve[1].top1_errors, 1);
    assert_eq!(curve[1].risk_ppm, Some(333_333));
    assert_eq!(curve[2].minimum_confidence_ppm, 700_000);
    assert_eq!(curve[2].covered_events, 4);
    assert_eq!(curve[2].top1_errors, 2);
    assert_eq!(curve[2].risk_ppm, Some(500_000));
}

#[test]
fn truth_outside_physical_candidate_universe_fails_closed() {
    let truths = vec![truth(
        "event-1",
        known("zone-not-in-universe", TruthEvidenceClass::Observed),
        TruthLabel::NotApplicable,
    )];
    let assessments = vec![assessment(
        "event-1",
        LocalizationLevel::Zone,
        &["zone-a"],
        &[],
        18,
    )];
    let predictions = vec![ranked(
        "event-1",
        LocalizationLevel::Zone,
        &["zone-a"],
        900_000,
    )];
    let error = evaluate_localization(&truths, &assessments, &predictions)
        .expect_err("truth/physics mismatch");
    assert_eq!(error.code, "truth_zone_outside_observable_universe");
}

#[test]
fn model_prediction_outside_physical_candidate_universe_fails_closed() {
    let truths = vec![truth(
        "event-1",
        known("zone-a", TruthEvidenceClass::Observed),
        TruthLabel::NotApplicable,
    )];
    let assessments = vec![assessment(
        "event-1",
        LocalizationLevel::Zone,
        &["zone-a"],
        &[],
        19,
    )];
    let predictions = vec![ranked(
        "event-1",
        LocalizationLevel::Zone,
        &["zone-b"],
        900_000,
    )];
    let error = evaluate_localization(&truths, &assessments, &predictions)
        .expect_err("prediction outside the owner-supplied candidate universe");
    assert_eq!(error.code, "prediction_outside_observable_universe");
}

#[test]
fn observability_assessment_requires_candidate_universe_matching_its_ceiling() {
    let truths = vec![truth(
        "event-1",
        known("zone-a", TruthEvidenceClass::Observed),
        TruthLabel::NotApplicable,
    )];
    let assessments = vec![assessment(
        "event-1",
        LocalizationLevel::Zone,
        &[],
        &[],
        20,
    )];
    let predictions = vec![abstain("event-1")];
    let error = evaluate_localization(&truths, &assessments, &predictions)
        .expect_err("zone ceiling needs zone candidates");
    assert_eq!(error.code, "observability_candidate_universe_mismatch");
}

#[test]
fn malformed_observability_digest_is_rejected() {
    let truths = vec![truth(
        "event-1",
        known("zone-a", TruthEvidenceClass::Observed),
        TruthLabel::NotApplicable,
    )];
    let mut invalid = assessment(
        "event-1",
        LocalizationLevel::Zone,
        &["zone-a"],
        &[],
        21,
    );
    invalid.assessment_digest = "sha256:00".to_owned();
    let predictions = vec![ranked(
        "event-1",
        LocalizationLevel::Zone,
        &["zone-a"],
        900_000,
    )];
    let error = evaluate_localization(&truths, &[invalid], &predictions)
        .expect_err("malformed digest");
    assert_eq!(error.code, "invalid_observability_digest");
}

#[test]
fn confidence_and_candidate_rankings_are_validated() {
    let truths = vec![truth(
        "event-1",
        known("zone-a", TruthEvidenceClass::Observed),
        TruthLabel::NotApplicable,
    )];
    let assessments = vec![assessment(
        "event-1",
        LocalizationLevel::Zone,
        &["zone-a"],
        &[],
        22,
    )];
    let invalid_confidence = vec![ranked(
        "event-1",
        LocalizationLevel::Zone,
        &["zone-a"],
        1_000_001,
    )];
    assert_eq!(
        evaluate_localization(&truths, &assessments, &invalid_confidence)
            .expect_err("confidence exceeds one million")
            .code,
        "confidence_out_of_range"
    );

    let empty_candidates = vec![ranked(
        "event-1",
        LocalizationLevel::Zone,
        &[],
        900_000,
    )];
    assert_eq!(
        evaluate_localization(&truths, &assessments, &empty_candidates)
            .expect_err("empty ranking")
            .code,
        "empty_candidate_ranking"
    );
}

#[test]
fn duplicate_candidate_ids_are_rejected() {
    let truths = vec![truth(
        "event-1",
        known("zone-a", TruthEvidenceClass::Observed),
        TruthLabel::NotApplicable,
    )];
    let mut assessments = vec![assessment(
        "event-1",
        LocalizationLevel::Zone,
        &["zone-a"],
        &[],
        23,
    )];
    assessments[0].zone_candidate_ids.push("zone-a".to_owned());
    let predictions = vec![abstain("event-1")];
    let error = evaluate_localization(&truths, &assessments, &predictions)
        .expect_err("duplicate physical candidate");
    assert_eq!(error.code, "duplicate_candidate_id");
}

#[test]
fn duplicate_truth_assessment_or_prediction_event_fails_closed() {
    let event_truth = truth(
        "event-1",
        known("zone-a", TruthEvidenceClass::Observed),
        TruthLabel::NotApplicable,
    );
    let event_assessment = assessment(
        "event-1",
        LocalizationLevel::Zone,
        &["zone-a"],
        &[],
        24,
    );
    let event_prediction = abstain("event-1");

    assert_eq!(
        evaluate_localization(
            &[event_truth.clone(), event_truth.clone()],
            &[event_assessment.clone(), event_assessment.clone()],
            &[event_prediction.clone(), event_prediction.clone()]
        )
        .expect_err("duplicate truth")
        .code,
        "duplicate_truth_event"
    );
    assert_eq!(
        evaluate_localization(
            &[event_truth.clone(), event_truth.clone()],
            &[event_assessment.clone(), assessment(
                "event-2",
                LocalizationLevel::Zone,
                &["zone-a"],
                &[],
                25
            )],
            &[event_prediction.clone(), abstain("event-2")]
        )
        .expect_err("duplicate truth must fail before other references")
        .code,
        "duplicate_truth_event"
    );
}

#[test]
fn input_event_sets_must_match_exactly() {
    let truths = vec![truth(
        "event-1",
        known("zone-a", TruthEvidenceClass::Observed),
        TruthLabel::NotApplicable,
    )];
    let assessments = vec![assessment(
        "event-2",
        LocalizationLevel::Zone,
        &["zone-a"],
        &[],
        26,
    )];
    let predictions = vec![abstain("event-1")];
    let error = evaluate_localization(&truths, &assessments, &predictions)
        .expect_err("assessment event set differs");
    assert_eq!(error.code, "missing_observability_event");
}

#[test]
fn report_is_deterministic_under_event_input_permutations() {
    let truths = vec![
        truth(
            "event-a",
            known("zone-a", TruthEvidenceClass::Observed),
            TruthLabel::NotApplicable,
        ),
        truth(
            "event-b",
            known("zone-b", TruthEvidenceClass::Observed),
            TruthLabel::NotApplicable,
        ),
    ];
    let assessments = vec![
        assessment("event-a", LocalizationLevel::Zone, &["zone-a"], &[], 27),
        assessment("event-b", LocalizationLevel::Zone, &["zone-b"], &[], 28),
    ];
    let predictions = vec![
        ranked("event-a", LocalizationLevel::Zone, &["zone-a"], 950_000),
        ranked("event-b", LocalizationLevel::Zone, &["zone-b"], 900_000),
    ];
    let first = evaluate_localization(&truths, &assessments, &predictions).expect("first");
    let second = evaluate_localization(
        &[truths[1].clone(), truths[0].clone()],
        &[assessments[1].clone(), assessments[0].clone()],
        &[predictions[1].clone(), predictions[0].clone()],
    )
    .expect("permuted");
    assert_eq!(first, second);
    assert_eq!(first.canonical_text(), second.canonical_text());
}


#[test]
fn report_binds_policy_and_physical_observability_receipts() {
    let truths = vec![truth(
        "event-1",
        known("zone-a", TruthEvidenceClass::Observed),
        TruthLabel::NotApplicable,
    )];
    let assessments = vec![assessment(
        "event-1",
        LocalizationLevel::Zone,
        &["zone-a", "zone-b"],
        &[],
        30,
    )];
    let predictions = vec![ranked(
        "event-1",
        LocalizationLevel::Zone,
        &["zone-a"],
        920_000,
    )];
    let report = evaluate_localization(&truths, &assessments, &predictions)
        .expect("bound report");
    let receipt = report.canonical_text();
    assert!(receipt.contains("policy_ref=localizer-policy-v1"));
    assert!(receipt.contains("observability_binding=event-1,zone,2,0,sensor-layout-v1,hydraulic-model-v1,"));
}

#[test]
fn mixed_policy_versions_cannot_be_pooled_in_one_evaluation() {
    let truths = vec![
        truth(
            "event-a",
            known("zone-a", TruthEvidenceClass::Observed),
            TruthLabel::NotApplicable,
        ),
        truth(
            "event-b",
            known("zone-b", TruthEvidenceClass::Observed),
            TruthLabel::NotApplicable,
        ),
    ];
    let assessments = vec![
        assessment("event-a", LocalizationLevel::Zone, &["zone-a"], &[], 31),
        assessment("event-b", LocalizationLevel::Zone, &["zone-b"], &[], 32),
    ];
    let mut predictions = vec![
        ranked("event-a", LocalizationLevel::Zone, &["zone-a"], 900_000),
        ranked("event-b", LocalizationLevel::Zone, &["zone-b"], 900_000),
    ];
    predictions[1].policy_ref = "localizer-policy-v2".to_owned();
    let error = evaluate_localization(&truths, &assessments, &predictions)
        .expect_err("mixed versions must not be aggregated");
    assert_eq!(error.code, "mixed_localization_policy_refs");
}

#[test]
fn unsupported_prediction_is_counted_even_when_truth_is_not_measured() {
    let truths = vec![truth(
        "event-1",
        TruthLabel::NotMeasured,
        TruthLabel::NotMeasured,
    )];
    let assessments = vec![assessment(
        "event-1",
        LocalizationLevel::Zone,
        &["zone-a"],
        &[],
        29,
    )];
    let predictions = vec![ranked(
        "event-1",
        LocalizationLevel::Pipe,
        &["pipe-a"],
        999_000,
    )];
    let report = evaluate_localization(&truths, &assessments, &predictions)
        .expect("unsupported output is auditable");
    assert_eq!(report.unsupported_granularity_predictions, 1);
    assert_eq!(report.zone.not_measured_events, 1);
    assert_eq!(report.pipe.not_measured_events, 1);
}
