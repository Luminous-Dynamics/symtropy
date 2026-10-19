// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde_json::from_str;
use symtropy_garden_sim::field_eval_report::{
    FieldEvalReportBlockerCodeV1, FieldEvalReportDispositionV1, FieldEvalReportInputV1,
    assess_field_eval_report_v1, sha256_hex,
};

const EXAMPLE: &str = include_str!("../fixtures/field_eval_report_example.v1.json");

fn valid_input() -> FieldEvalReportInputV1 {
    from_str(EXAMPLE).expect("valid synthetic report fixture")
}

fn has_blocker(input: &FieldEvalReportInputV1, code: FieldEvalReportBlockerCodeV1) -> bool {
    assess_field_eval_report_v1(input)
        .blockers
        .iter()
        .any(|blocker| blocker.code == code)
}

#[test]
fn primary_bounds_reconcile_without_float_arithmetic() {
    let report = assess_field_eval_report_v1(&valid_input());
    assert_eq!(report.disposition, FieldEvalReportDispositionV1::Calculated);
    assert_eq!(report.eligible_request_count, 10);
    assert_eq!(report.classified_request_count, Some(7));
    let bounds = report.evidence_confirmed_attainment.expect("valid bounds");
    assert_eq!(bounds.lower_numerator, 3);
    assert_eq!(bounds.upper_numerator, 6);
    assert_eq!(bounds.denominator, 10);
}

#[test]
fn screened_count_must_reconcile_to_eligible_plus_exclusions() {
    let mut input = valid_input();
    input.protocol_excluded_count = 3;
    assert!(has_blocker(
        &input,
        FieldEvalReportBlockerCodeV1::ScreenedCountMismatch
    ));
}

#[test]
fn mutually_exclusive_outcomes_must_account_for_all_eligible_requests() {
    let mut input = valid_input();
    input.outcomes.accepted_after_window_count = 0;
    assert!(has_blocker(
        &input,
        FieldEvalReportBlockerCodeV1::OutcomePartitionMismatch
    ));
}

#[test]
fn known_nonacceptance_reasons_must_reconcile_exactly() {
    let mut input = valid_input();
    input.nonacceptance_reasons.provider_no_show_count = 0;
    assert!(has_blocker(
        &input,
        FieldEvalReportBlockerCodeV1::NonacceptanceReasonMismatch
    ));
}

#[test]
fn completed_pending_decision_must_be_a_partition_bucket_not_a_duplicate_field() {
    let mut value: serde_json::Value = serde_json::from_str(EXAMPLE).expect("fixture JSON");
    value["completed_pending_decision_count"] = serde_json::Value::String("1".into());
    assert!(serde_json::from_value::<FieldEvalReportInputV1>(value).is_err());
}

#[test]
fn known_late_or_disputed_outcomes_do_not_inflate_upper_bound() {
    let mut input = valid_input();
    input.outcomes.accepted_after_window_count = 3;
    input.outcomes.disputed_or_rework_count = 1;
    input.outcomes.known_not_accepted_count = 0;
    input.nonacceptance_reasons = Default::default();
    let report = assess_field_eval_report_v1(&input);
    assert_eq!(report.disposition, FieldEvalReportDispositionV1::Calculated);
    let bounds = report.evidence_confirmed_attainment.expect("valid bounds");
    assert_eq!(bounds.lower_numerator, 4);
    assert_eq!(bounds.upper_numerator, 6);
    assert_eq!(bounds.denominator, 10);
}

#[test]
fn blocked_report_does_not_emit_partial_classified_count_or_rate() {
    let mut input = valid_input();
    input.screened_request_count = 99;
    let report = assess_field_eval_report_v1(&input);
    assert_eq!(report.disposition, FieldEvalReportDispositionV1::Blocked);
    assert!(report.classified_request_count.is_none());
    assert!(report.evidence_confirmed_attainment.is_none());
}

#[test]
fn no_eligible_requests_is_not_assessable_not_zero_percent_success() {
    let mut input = valid_input();
    input.screened_request_count = 2;
    input.eligible_request_count = 0;
    input.protocol_excluded_count = 2;
    input.outcomes = Default::default();
    input.nonacceptance_reasons = Default::default();
    let report = assess_field_eval_report_v1(&input);
    assert_eq!(
        report.disposition,
        FieldEvalReportDispositionV1::NotAssessable
    );
    assert!(report.evidence_confirmed_attainment.is_none());
    assert_eq!(report.classified_request_count, Some(0));
}

#[test]
fn unsafe_reference_format_blocks_public_report() {
    let mut input = valid_input();
    input.source_ledger_ref = "https://example.com/farmer-journal".into();
    assert!(has_blocker(
        &input,
        FieldEvalReportBlockerCodeV1::MissingOrUnsafeReference
    ));
}

#[test]
fn unsupported_schema_fails_closed() {
    let mut input = valid_input();
    input.schema_version = 99;
    assert!(has_blocker(
        &input,
        FieldEvalReportBlockerCodeV1::UnsupportedSchema
    ));
}

#[test]
fn overflowing_count_sums_fail_closed() {
    let mut input = valid_input();
    input.screened_request_count = u64::MAX;
    input.eligible_request_count = u64::MAX;
    input.protocol_excluded_count = 1;
    input.outcomes.accepted_within_window_count = u64::MAX;
    input.outcomes.accepted_after_window_count = 1;
    assert!(has_blocker(
        &input,
        FieldEvalReportBlockerCodeV1::CountOverflow
    ));
}

#[test]
fn unknown_report_fields_are_rejected_instead_of_silently_ignored() {
    let mut value: serde_json::Value = serde_json::from_str(EXAMPLE).expect("fixture JSON");
    value["unexpected_success_count"] = serde_json::Value::String("999".into());
    assert!(serde_json::from_value::<FieldEvalReportInputV1>(value).is_err());
}

#[test]
fn input_sha256_uses_a_standard_known_vector() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn report_result_keeps_a_claim_ceiling() {
    let report = assess_field_eval_report_v1(&valid_input());
    assert!(report.claim_ceiling.contains("Does not authenticate"));
    assert!(report.claim_ceiling.contains("causal effects"));
}
