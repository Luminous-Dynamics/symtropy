// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent oracle for the frozen AGRI-ROBOT-SERVICE-008 report fixture.

use serde_json::from_str;
use symtropy_garden_sim::field_eval_report::{
    FieldEvalPreflightStatusV1, FieldStudyManifestV1, preflight_field_study_v1,
};

const FIXTURE: &str = include_str!("../fixtures/field_eval_report.v1.json");

#[test]
fn frozen_field_report_fixture_matches_independent_expected_accounting() {
    let manifest: FieldStudyManifestV1 =
        from_str(FIXTURE).expect("synthetic field-report fixture parses");
    let report = preflight_field_study_v1(&manifest);

    assert_eq!(report.status, FieldEvalPreflightStatusV1::Calculated);
    assert_eq!(report.screened_count, 8);
    assert_eq!(report.eligible_denominator, 6);
    assert_eq!(report.excluded_count, 2);
    assert!(report.blockers.is_empty());

    let counts = report.outcome_counts.as_ref().expect("valid outcome counts");
    assert_eq!(counts.accepted_within_useful_window, 1);
    assert_eq!(counts.accepted_late, 1);
    assert_eq!(counts.known_not_accepted, 1);
    assert_eq!(counts.provider_completion_awaiting_decision, 1);
    assert_eq!(counts.disputed_or_rework, 1);
    assert_eq!(counts.outcome_unknown, 1);
    assert_eq!(counts.follow_up_missing_or_inaccessible, 0);

    let lower = report
        .evidence_confirmed_lower_bound
        .as_ref()
        .expect("lower identification bound");
    let upper = report
        .missingness_upper_identification_bound
        .as_ref()
        .expect("upper identification bound");
    let coverage = report
        .outcome_ascertainment_coverage
        .as_ref()
        .expect("ascertainment coverage");

    assert_eq!((lower.numerator, lower.denominator), (1, 6));
    assert_eq!((upper.numerator, upper.denominator), (4, 6));
    assert_eq!((coverage.numerator, coverage.denominator), (3, 6));
    assert!(lower.numerator <= upper.numerator);
    assert_eq!(lower.denominator, report.eligible_denominator);
    assert_eq!(upper.denominator, report.eligible_denominator);
    assert_eq!(coverage.denominator, report.eligible_denominator);

    let serialized = serde_json::to_value(&report).expect("report is serializable");
    assert_eq!(serialized["schema_version"], 1);
    assert_eq!(serialized["status"], "calculated");
    assert!(
        serialized["evidence_confirmed_lower_bound"]["interpretation"]
            .as_str()
            .expect("lower bound interpretation")
            .contains("not a confidence interval")
    );
    assert!(
        serialized["missingness_upper_identification_bound"]["interpretation"]
            .as_str()
            .expect("upper bound interpretation")
            .contains("not a confidence interval")
    );
}

#[test]
fn fixture_cannot_silently_drop_a_no_provider_case() {
    let mut manifest: FieldStudyManifestV1 =
        from_str(FIXTURE).expect("synthetic field-report fixture parses");
    manifest.outcome_records.retain(|record| {
        record.request_ref != "req-study-003"
    });
    let report = preflight_field_study_v1(&manifest);
    assert_eq!(report.status, FieldEvalPreflightStatusV1::Blocked);
    assert!(report.evidence_confirmed_lower_bound.is_none());
    assert!(report.missingness_upper_identification_bound.is_none());
}
