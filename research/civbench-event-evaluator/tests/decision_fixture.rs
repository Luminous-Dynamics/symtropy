// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civbench_event_evaluator::decision_fixture::parse_closure_fixture;

const FIXTURE: &str = include_str!("../../../docs/research/civbench-jhb-water/decision_closure_fixture.v0.1.json");

#[test]
fn checked_in_fixture_parses_and_validates_all_five_matched_cases() {
    let report = parse_closure_fixture(FIXTURE).expect("fixture should validate");
    assert_eq!(report.fixture_id, "CIVBENCH-DECISION-CLOSURE-FIXTURE-001");
    assert_eq!(report.episodes.len(), 5);
    assert_eq!(report.episodes.iter().map(|episode| episode.case_kind.as_str()).collect::<Vec<_>>(), vec![
        "positive_service_restoration",
        "no_value_sensor",
        "false_alarm_overload",
        "resource_exhaustion",
        "unverified_repair",
    ]);
    assert_eq!(report.episodes.len() * 2, 10);
    for episode in &report.episodes {
        episode.control.record.validate().expect("control record");
        episode.candidate.record.validate().expect("candidate record");
        assert_eq!(episode.control.record.paired_block_id, report.common.block_id);
        assert_eq!(episode.candidate.record.paired_block_id, report.common.block_id);
        assert_eq!(episode.control.record.decision_episode_id, episode.decision_episode_id);
        assert_eq!(episode.candidate.record.decision_episode_id, episode.decision_episode_id);
    }
}

#[test]
fn canonical_fixture_receipt_is_deterministic_and_preserves_not_measured() {
    let first = parse_closure_fixture(FIXTURE).expect("fixture should validate");
    let second = parse_closure_fixture(FIXTURE).expect("same fixture should validate");
    assert_eq!(first.canonical_text(), second.canonical_text());
    assert!(first.canonical_text().contains("metric=essential_shortfall_l,not_measured,none"));
    assert!(first.canonical_text().contains("effect=Indeterminate"));
    assert!(!first.canonical_text().contains("metric=essential_shortfall_l,not_measured,0"));
}

#[test]
fn unknown_top_level_field_fails_closed() {
    let mutated = FIXTURE.replacen(
        "{\n  \"fixture_id\":",
        "{\n  \"unrecognized_field\": true,\n  \"fixture_id\":",
        1,
    );
    let error = parse_closure_fixture(&mutated).expect_err("unknown field must fail");
    assert_eq!(error.code, "unknown_manifest_field");
}

#[test]
fn duplicate_json_key_fails_before_typed_conversion() {
    let mutated = FIXTURE.replacen(
        "{\n  \"fixture_id\":",
        "{\n  \"fixture_id\": \"duplicate\",\n  \"fixture_id\":",
        1,
    );
    let error = parse_closure_fixture(&mutated).expect_err("duplicate key must fail");
    assert_eq!(error.code, "duplicate_json_key");
}

#[test]
fn unknown_arm_field_fails_closed() {
    let mutated = FIXTURE.replacen(
        "\"policy_ref\":",
        "\"unknown_arm_field\": \"surprise\",\n        \"policy_ref\":",
        1,
    );
    let error = parse_closure_fixture(&mutated).expect_err("unknown arm field must fail");
    assert_eq!(error.code, "unknown_manifest_field");
}

#[test]
fn receipt_binding_must_match_episode_and_block() {
    let mutated = FIXTURE.replacen(
        "\"paired_block_id\": \"block-closure-001\"",
        "\"paired_block_id\": \"block-other\"",
        1,
    );
    let error = parse_closure_fixture(&mutated).expect_err("wrong block binding must fail");
    assert_eq!(error.code, "receipt_context_mismatch");
}

#[test]
fn zero_digest_fails_closed() {
    let mutated = FIXTURE.replacen(
        "sha256:0000000000000000000000000000000000000000000000000000000000000001",
        "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        1,
    );
    let error = parse_closure_fixture(&mutated).expect_err("zero digest must fail");
    assert_eq!(error.code, "invalid_receipt_digest");
}

#[test]
fn measured_status_with_null_value_fails_closed() {
    let mutated = FIXTURE.replacen(
        "\"value\": 120000,\n            \"status\": \"modelled\"",
        "\"value\": null,\n            \"status\": \"modelled\"",
        1,
    );
    assert_ne!(mutated, FIXTURE, "mutation must target a real fixture value");
    let error = parse_closure_fixture(&mutated).expect_err("modelled metric cannot be null");
    assert_eq!(error.code, "metric_status_value_mismatch");
}

#[test]
fn paired_metrics_with_different_evidence_status_fail_closed() {
    let candidate_start = FIXTURE.find("\"candidate\": {").expect("candidate arm");
    let tail = &FIXTURE[candidate_start..];
    let relative = tail.find("\"status\": \"modelled\"").expect("candidate modelled status");
    let start = candidate_start + relative;
    let end = start + "\"status\": \"modelled\"".len();
    let mut mutated = FIXTURE.to_owned();
    mutated.replace_range(start..end, "\"status\": \"observed\"");
    let error = parse_closure_fixture(&mutated).expect_err("cross-class pair must fail");
    assert!(matches!(
        error.code,
        "mixed_service_evidence_classes" | "cross_evidence_status_comparison"
    ));
}

#[test]
fn changing_candidate_expected_claim_without_updating_record_fails() {
    let mutated = FIXTURE.replacen(
        "\"service_effect_claim\": \"modelled_only\"",
        "\"service_effect_claim\": \"not_measured\"",
        1,
    );
    let error = parse_closure_fixture(&mutated).expect_err("claim/record mismatch must fail");
    assert!(
        matches!(
            error.code,
            "not_measured_claim_has_evidence" | "candidate_effect_does_not_match_expected_claim"
        )
    );
}
