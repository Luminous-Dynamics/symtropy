// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent need-observation oracle. Every record is synthetic.

use serde_json::from_str;
use symtropy_garden_sim::need_observation::{
    NeedDispositionV1 as D, NeedEpisodeV1, NeedExclusionCountsV1, NeedObservationBlockerCodeV1 as B,
    NeedObservationDispositionV1 as AssessmentDisposition, NeedObservationInputV1,
    NeedOutcomeSourceV1 as S, NeedReportBasisV1 as Basis, NeedRequestPresenceV1 as RequestPresence,
    NoRequestReasonV1 as Reason, assess_need_observation_v1,
};

const EXAMPLE: &str = include_str!("../fixtures/need_observation_example.v1.json");
const MANIFEST: &str = include_str!("../fixtures/need_observation_cases.v1.tsv");

fn valid_input() -> NeedObservationInputV1 {
    from_str(EXAMPLE).expect("valid synthetic need-observation input")
}

fn has_blocker(input: &NeedObservationInputV1, code: B) -> bool {
    assess_need_observation_v1(input)
        .blockers
        .iter()
        .any(|blocker| blocker.code == code)
}

#[test]
fn valid_mixed_need_and_request_denominators_are_separate() {
    let report = assess_need_observation_v1(&valid_input());
    assert_eq!(report.disposition, AssessmentDisposition::Calculated);
    let summary = report.summary.expect("valid summary");
    assert_eq!(summary.screened_candidate_count, 8);
    assert_eq!(summary.eligible_need_count, 6);
    assert_eq!(summary.protocol_excluded_count, 2);
    assert_eq!(summary.linked_canonical_request_count, 1);
    assert_eq!(summary.farmer_confirms_no_request_count, 2);
    assert_eq!(summary.no_canonical_request_located_count, 1);
    assert_eq!(summary.request_existence_unknown_or_not_assessed_count, 2);
}

#[test]
fn prospective_retrospective_and_relay_basis_remain_distinct() {
    let report = assess_need_observation_v1(&valid_input());
    let summary = report.summary.expect("valid summary");
    assert_eq!(summary.prospective_farmer_report_count, 4);
    assert_eq!(summary.retrospective_farmer_recall_count, 1);
    assert_eq!(summary.authorized_relay_count, 1);
}

#[test]
fn a_farmer_confirmed_no_request_has_no_invented_request_reference() {
    let report = assess_need_observation_v1(&valid_input());
    let summary = report.summary.expect("valid summary");
    assert_eq!(summary.farmer_confirms_no_request_count, 2);
    assert_eq!(
        summary.no_request_reason_counts.iter().map(|entry| entry.count).sum::<u64>(),
        2
    );
    assert_eq!(summary.no_request_reason_not_recorded_count, 0);
    assert_eq!(
        summary.no_request_reason_counts.iter().find(|entry| entry.reason == Reason::ExpectedUnaffordable).map(|entry| entry.count),
        Some(1)
    );
}

#[test]
fn farmer_reported_off_platform_service_never_becomes_canonical_attainment() {
    let report = assess_need_observation_v1(&valid_input());
    let summary = report.summary.expect("valid summary");
    assert_eq!(summary.adequate_service_within_window_count, 2);
    assert_eq!(summary.canonical_service_attainment_count, 1);
    assert_eq!(summary.other_channel_reported_attainment_count, 1);
}

#[test]
fn authorized_relayed_other_channel_attainment_stays_noncanonical() {
    let mut input = valid_input();
    input.episodes[4].request_presence = RequestPresence::NoCanonicalRequestLocated;
    input.episodes[4].disposition = D::AdequateServiceWithinWindow;
    input.episodes[4].outcome_source = S::AuthorizedRelayOtherChannel;
    input.episodes[4].no_request_reason = None;
    let report = assess_need_observation_v1(&input);
    assert_eq!(report.disposition, AssessmentDisposition::Calculated);
    let summary = report.summary.expect("valid summary");
    assert_eq!(summary.adequate_service_within_window_count, 3);
    assert_eq!(summary.canonical_service_attainment_count, 1);
    assert_eq!(summary.other_channel_reported_attainment_count, 2);

    input.episodes[4].report_basis = Basis::FarmerConfirmedProspective;
    assert!(has_blocker(&input, B::OutcomeSourceMismatch));
}

#[test]
fn absent_optional_barrier_answer_is_not_imputed() {
    let mut input = valid_input();
    input.episodes[1].no_request_reason = None;
    let report = assess_need_observation_v1(&input);
    assert_eq!(report.disposition, AssessmentDisposition::Calculated);
    let summary = report.summary.expect("valid summary");
    assert_eq!(summary.no_request_reason_not_recorded_count, 1);
    assert_eq!(
        summary.no_request_reason_counts.iter().map(|entry| entry.count).sum::<u64>(),
        1
    );
}

#[test]
fn open_deferred_and_missing_outcomes_stay_distinct() {
    let report = assess_need_observation_v1(&valid_input());
    let summary = report.summary.expect("valid summary");
    assert_eq!(summary.open_need_without_adequate_service_count, 2);
    assert_eq!(summary.farmer_deferred_count, 1);
    assert_eq!(summary.follow_up_missing_count, 1);
    assert_eq!(summary.outcome_unknown_count, 0);
}

#[test]
fn request_presence_link_requires_real_opaque_request_ref() {
    let mut input = valid_input();
    input.episodes[0].request_ref = None;
    assert!(has_blocker(&input, B::RequestReferenceMismatch));
}

#[test]
fn no_request_state_cannot_carry_fabricated_request_ref() {
    let mut input = valid_input();
    input.episodes[1].request_ref = Some("req-invented-002".into());
    assert!(has_blocker(&input, B::RequestReferenceMismatch));
}

#[test]
fn telemetry_non_use_cannot_create_a_need_episode() {
    let mut input = valid_input();
    input.episodes[1].report_basis = Basis::InferredFromNonUseTelemetry;
    assert!(has_blocker(&input, B::UnqualifiedNeedProvenance));
    assert!(assess_need_observation_v1(&input).summary.is_none());
}

#[test]
fn unverified_intermediary_relay_cannot_count_as_confirmed_need() {
    let mut input = valid_input();
    input.episodes[4].report_basis = Basis::UnverifiedIntermediaryRelay;
    assert!(has_blocker(&input, B::UnqualifiedNeedProvenance));
}

#[test]
fn authorized_relay_requires_an_opaque_authority_reference() {
    let mut input = valid_input();
    input.episodes[4].authority_ref = None;
    assert!(has_blocker(&input, B::AuthorityReferenceMismatch));
    input.episodes[4].authority_ref = Some("https://example.com/mandate".into());
    assert!(has_blocker(&input, B::AuthorityReferenceMismatch));
}

#[test]
fn duplicate_need_references_block_the_report() {
    let mut input = valid_input();
    input.episodes[1].need_ref = input.episodes[0].need_ref.clone();
    input.screened_candidate_count = 9;
    assert!(has_blocker(&input, B::DuplicateNeedReference));
}

#[test]
fn episode_must_match_preregistered_task_scope_period_and_window() {
    let mut input = valid_input();
    input.episodes[1].scope_ref = "scope-other-v1".into();
    assert!(has_blocker(&input, B::ScopeOrTaskMismatch));
}

#[test]
fn screen_counts_and_exclusion_classes_must_reconcile() {
    let mut input = valid_input();
    input.screened_candidate_count = 9;
    assert!(has_blocker(&input, B::ScreenedCountMismatch));

    let mut input = valid_input();
    input.protocol_excluded_count = 3;
    input.screened_candidate_count = 9;
    assert!(has_blocker(&input, B::ExclusionCountMismatch));
}

#[test]
fn no_request_reason_is_not_allowed_on_linked_request() {
    let mut input = valid_input();
    input.episodes[0].no_request_reason = Some(Reason::ExpectedUnaffordable);
    assert!(has_blocker(&input, B::NoRequestReasonMismatch));
}

#[test]
fn canonical_service_attainment_requires_a_linked_canonical_request() {
    let mut input = valid_input();
    input.episodes[2].outcome_source = S::CanonicalServiceLifecycle;
    assert!(has_blocker(&input, B::OutcomeSourceMismatch));
}

#[test]
fn unknown_outcome_cannot_claim_positive_service_evidence() {
    let mut input = valid_input();
    input.episodes[5].outcome_source = S::CanonicalServiceLifecycle;
    assert!(has_blocker(&input, B::OutcomeSourceMismatch));
}

#[test]
fn empty_eligible_need_cohort_is_not_assessable() {
    let mut input = valid_input();
    input.episodes.clear();
    input.screened_candidate_count = 2;
    input.protocol_excluded_count = 2;
    input.exclusions.training_or_test_record_count = 2;
    input.exclusions.consent_not_granted_count = 0;
    let report = assess_need_observation_v1(&input);
    assert_eq!(report.disposition, AssessmentDisposition::NotAssessable);
    assert!(report.summary.is_none());
}

#[test]
fn unsafe_personal_or_url_reference_blocks_public_aggregate() {
    let mut input = valid_input();
    input.episodes[0].need_ref = "https://example.com/farmer-name".into();
    assert!(has_blocker(&input, B::MissingOrUnsafeReference));
}

#[test]
fn unknown_fields_are_rejected() {
    let mut value: serde_json::Value = serde_json::from_str(EXAMPLE).expect("fixture JSON");
    value["episodes"][0]["invented_booking_status"] = serde_json::Value::String("booked".into());
    assert!(serde_json::from_value::<NeedObservationInputV1>(value).is_err());
}

#[test]
fn frozen_manifest_contains_all_expected_case_ids_once() {
    let mut ids = Vec::new();
    let mut outcomes = Vec::new();
    for line in MANIFEST.lines().skip(1).filter(|line| !line.trim().is_empty()) {
        let fields: Vec<&str> = line.split('\\t').collect();
        assert_eq!(fields.len(), 3, "malformed manifest line: {line}");
        assert!(!fields[1].is_empty(), "missing expected result: {line}");
        assert!(!fields[2].is_empty(), "missing rationale: {line}");
        ids.push(fields[0]);
        outcomes.push(fields[1]);
    }
    let expected = [
        "valid_mixed_need_and_request_denominators_are_separate",
        "prospective_retrospective_and_relay_basis_remain_distinct",
        "a_farmer_confirmed_no_request_has_no_invented_request_reference",
        "farmer_reported_off_platform_service_never_becomes_canonical_attainment",
        "authorized_relayed_other_channel_attainment_stays_noncanonical",
        "absent_optional_barrier_answer_is_not_imputed",
        "open_deferred_and_missing_outcomes_stay_distinct",
        "request_presence_link_requires_real_opaque_request_ref",
        "no_request_state_cannot_carry_fabricated_request_ref",
        "telemetry_non_use_cannot_create_a_need_episode",
        "unverified_intermediary_relay_cannot_count_as_confirmed_need",
        "authorized_relay_requires_an_opaque_authority_reference",
        "duplicate_need_references_block_the_report",
        "episode_must_match_preregistered_task_scope_period_and_window",
        "screen_counts_and_exclusion_classes_must_reconcile",
        "no_request_reason_is_not_allowed_on_linked_request",
        "canonical_service_attainment_requires_a_linked_canonical_request",
        "unknown_outcome_cannot_claim_positive_service_evidence",
        "empty_eligible_need_cohort_is_not_assessable",
        "unsafe_personal_or_url_reference_blocks_public_aggregate",
        "unknown_fields_are_rejected",
    ];
    let expected_outcomes = [
        "CALCULATED",
        "CALCULATED",
        "CALCULATED",
        "CALCULATED",
        "CALCULATED",
        "CALCULATED",
        "CALCULATED",
        "BLOCKED",
        "BLOCKED",
        "BLOCKED",
        "BLOCKED",
        "BLOCKED",
        "BLOCKED",
        "BLOCKED",
        "BLOCKED",
        "BLOCKED",
        "BLOCKED",
        "BLOCKED",
        "NOT_ASSESSABLE",
        "BLOCKED",
        "REJECTED",
    ];
    assert_eq!(ids.len(), expected.len());
    assert_eq!(ids, expected);
    assert_eq!(outcomes, expected_outcomes);
    let mut unique = ids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), ids.len(), "duplicate manifest case ID");
}
