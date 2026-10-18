// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent, synthetic repair-recovery event oracle cases.

use symtropy_garden_sim::field_robot_cost::{CostEvidenceDispositionV1, CostEvidenceStateV1, ServiceUnitV1};
use symtropy_garden_sim::repair_recovery::{
    assess_repair_recovery_v1, InterruptionCauseV1, PartRequirementV1,
    RepairEpisodeV1, RepairRecoveryBlockerCodeV1, RepairRecoveryCalculationDispositionV1,
    RepairRecoveryInputV1, REPAIR_RECOVERY_SCHEMA_VERSION_V1,
};
use symtropy_garden_sim::service_access::{
    ServiceAccessEvidenceV1, ServiceAccessInputV1, ServiceAccessRequestV1,
    TravelChargeTreatmentV1, SERVICE_ACCESS_SCHEMA_VERSION_V1,
};

fn request(id: &str) -> ServiceAccessRequestV1 {
    ServiceAccessRequestV1 {
        request_id: id.into(), scenario_id: "repair-v1".into(), scope_ref: "coop-v1".into(),
        period_ref: "season-v1".into(), currency_code: "ZAR".into(), task_code: "weeding".into(),
        service_unit: ServiceUnitV1::SquareMetresCompleted, requested_units: 100,
        requested_at_minute: 0, useful_window_deadline_minute: 200,
        quote_at_minute: None, quoted_fee_minor: None, price_basis_unit: None,
        quote_valid_until_minute: None, travel_charge_treatment: None, travel_fee_minor: None,
        booking_at_minute: None, completion_at_minute: None, delivered_units: None,
        actual_travel_minutes: None, delivery_decision_at_minute: None, delivery_decision: None,
        no_outcome_reason: None, cohort_ref: None, cohort_consent: false,
        evidence: ServiceAccessEvidenceV1::synthetic_fixture(id),
    }
}
fn service_access(ids: &[&str]) -> ServiceAccessInputV1 {
    ServiceAccessInputV1 {
        schema_version: SERVICE_ACCESS_SCHEMA_VERSION_V1, scenario_id: "repair-v1".into(),
        scope_ref: "coop-v1".into(), period_ref: "season-v1".into(), currency_code: "ZAR".into(),
        minimum_cohort_size: 10, requests: ids.iter().map(|id| request(id)).collect(),
    }
}
fn episode(id: &str, cause: InterruptionCauseV1, affected: Vec<&str>) -> RepairEpisodeV1 {
    let mut e = RepairEpisodeV1 {
        episode_id: id.into(), scenario_id: "repair-v1".into(), scope_ref: "coop-v1".into(),
        period_ref: "season-v1".into(), currency_code: "ZAR".into(), asset_id: "asset-1".into(),
        asset_configuration_ref: "config-v1".into(), cause,
        outage_started_at_minute: 10, failure_detected_at_minute: None, asset_removed_at_minute: None,
        diagnosis_completed_at_minute: None, part_requirement: PartRequirementV1::NotAssessed,
        part_required_at_minute: None, part_ordered_at_minute: None, part_received_at_minute: None,
        repair_started_at_minute: None, repair_completed_at_minute: None, service_restored_at_minute: None,
        asset_verified_available_at_minute: None,
        affected_request_ids: affected.into_iter().map(str::to_owned).collect(),
        evidence: ServiceAccessEvidenceV1::synthetic_fixture(id),
    };
    match cause {
        InterruptionCauseV1::Breakdown => {
            e.failure_detected_at_minute = Some(10);
            e.asset_removed_at_minute = Some(11);
        }
        InterruptionCauseV1::ScheduledMaintenance => {
            e.asset_removed_at_minute = Some(10);
        }
        InterruptionCauseV1::WeatherOrSiteAccess | InterruptionCauseV1::OperatorUnavailable => {}
    }
    e
}
fn complete_breakdown(id: &str, start: u64, part_required: bool, affected: Vec<&str>) -> RepairEpisodeV1 {
    let mut e = episode(id, InterruptionCauseV1::Breakdown, affected);
    e.outage_started_at_minute = start;
    e.failure_detected_at_minute = Some(start);
    e.asset_removed_at_minute = Some(start + 1);
    e.diagnosis_completed_at_minute = Some(start + 5);
    if part_required {
        e.part_requirement = PartRequirementV1::Required;
        e.part_required_at_minute = Some(start + 6);
        e.part_ordered_at_minute = Some(start + 7);
        e.part_received_at_minute = Some(start + 17);
        e.repair_started_at_minute = Some(start + 18);
    } else {
        e.part_requirement = PartRequirementV1::NotRequired;
        e.repair_started_at_minute = Some(start + 10);
    }
    e.repair_completed_at_minute = Some(start + if part_required { 28 } else { 40 });
    e.service_restored_at_minute = e.repair_completed_at_minute.map(|t| t + 1);
    e.asset_verified_available_at_minute = e.service_restored_at_minute.map(|t| t + if part_required { 1 } else { 19 });
    e
}
fn input(episodes: Vec<RepairEpisodeV1>) -> RepairRecoveryInputV1 {
    RepairRecoveryInputV1 {
        schema_version: REPAIR_RECOVERY_SCHEMA_VERSION_V1, scenario_id: "repair-v1".into(),
        scope_ref: "coop-v1".into(), period_ref: "season-v1".into(), currency_code: "ZAR".into(),
        as_of_minute: 1_000, declared_log_complete: true, episodes,
    }
}
fn assert_blocked(access: &ServiceAccessInputV1, report: &RepairRecoveryInputV1, expected: RepairRecoveryBlockerCodeV1) {
    let result = assess_repair_recovery_v1(access, report);
    assert_eq!(result.calculation_disposition, RepairRecoveryCalculationDispositionV1::Blocked);
    assert_eq!(result.evidence_disposition, CostEvidenceDispositionV1::Blocked);
    assert!(result.blockers.iter().any(|b| b.code == expected));
    assert!(result.breakdown.is_none());
}

#[test]
fn repair_restore_and_parts_wait_percentiles_match_oracle() {
    let a = complete_breakdown("repair-a", 10, true, vec!["r1"]);
    let b = complete_breakdown("repair-b", 100, false, vec!["r2"]);
    let result = assess_repair_recovery_v1(&service_access(&["r1", "r2"]), &input(vec![a, b]));
    assert_eq!(result.calculation_disposition, RepairRecoveryCalculationDispositionV1::Calculated);
    let b = result.breakdown.unwrap();
    assert_eq!(b.breakdown_count, 2);
    assert_eq!(b.verified_episode_count, 2);
    assert_eq!(b.open_episode_count, 0);
    assert_eq!(b.parts_required_count, 1);
    assert_eq!(b.parts_received_count, 1);
    assert_eq!(b.breakdown_restore_p50_minutes, Some(30));
    assert_eq!(b.breakdown_restore_p90_minutes, Some(60));
    assert_eq!(b.parts_wait_p50_minutes, Some(10));
    assert_eq!(b.parts_wait_p90_minutes, Some(10));
    assert_eq!(b.affected_request_link_count, 2);
    assert_eq!(b.unique_affected_request_count, 2);
}

#[test]
fn open_breakdown_and_parts_wait_remain_open() {
    let mut e = episode("open", InterruptionCauseV1::Breakdown, vec!["r1"]);
    e.failure_detected_at_minute = Some(10);
    e.asset_removed_at_minute = Some(11);
    e.diagnosis_completed_at_minute = Some(15);
    e.part_requirement = PartRequirementV1::Required;
    e.part_required_at_minute = Some(16);
    e.part_ordered_at_minute = Some(17);
    let b = assess_repair_recovery_v1(&service_access(&["r1"]), &input(vec![e])).breakdown.unwrap();
    assert_eq!(b.open_episode_count, 1);
    assert_eq!(b.open_parts_wait_count, 1);
    assert_eq!(b.breakdown_restore_p50_minutes, None);
    assert_eq!(b.parts_wait_p50_minutes, None);
}

#[test]
fn missing_breakdown_failure_timestamp_blocks() {
    let mut e = complete_breakdown("missing-failure", 10, false, vec![]);
    e.failure_detected_at_minute = None;
    assert_blocked(&service_access(&["request"]), &input(vec![e]), RepairRecoveryBlockerCodeV1::InvalidInterruptionShape);
}

#[test]
fn non_machine_outage_cannot_claim_repair_events() {
    let mut e = episode("weather-repair", InterruptionCauseV1::WeatherOrSiteAccess, vec![]);
    e.repair_started_at_minute = Some(15);
    assert_blocked(&service_access(&["request"]), &input(vec![e]), RepairRecoveryBlockerCodeV1::InvalidInterruptionShape);
}

#[test]
fn part_receipt_requires_order() {
    let mut e = complete_breakdown("part-no-order", 10, true, vec![]);
    e.part_ordered_at_minute = None;
    assert_blocked(&service_access(&["request"]), &input(vec![e]), RepairRecoveryBlockerCodeV1::InvalidPartRequirementShape);
}

#[test]
fn repair_start_requires_diagnosis() {
    let mut e = complete_breakdown("repair-no-diagnosis", 10, false, vec![]);
    e.diagnosis_completed_at_minute = None;
    assert_blocked(&service_access(&["request"]), &input(vec![e]), RepairRecoveryBlockerCodeV1::InvalidEventOrder);
}

#[test]
fn repair_completion_requires_start() {
    let mut e = complete_breakdown("repair-no-start", 10, false, vec![]);
    e.repair_started_at_minute = None;
    assert_blocked(&service_access(&["request"]), &input(vec![e]), RepairRecoveryBlockerCodeV1::InvalidEventOrder);
}

#[test]
fn breakdown_restore_requires_repair_completion() {
    let mut e = complete_breakdown("restore-no-repair", 10, false, vec![]);
    e.repair_completed_at_minute = None;
    assert_blocked(&service_access(&["request"]), &input(vec![e]), RepairRecoveryBlockerCodeV1::InvalidEventOrder);
}

#[test]
fn verification_requires_service_restoration() {
    let mut e = complete_breakdown("verify-no-restore", 10, false, vec![]);
    e.service_restored_at_minute = None;
    assert_blocked(&service_access(&["request"]), &input(vec![e]), RepairRecoveryBlockerCodeV1::InvalidEventOrder);
}

#[test]
fn future_timestamp_blocks() {
    let mut e = complete_breakdown("future", 10, false, vec![]);
    e.asset_verified_available_at_minute = Some(1_001);
    assert_blocked(&service_access(&["request"]), &input(vec![e]), RepairRecoveryBlockerCodeV1::EventAfterAsOf);
}

#[test]
fn non_monotone_event_order_blocks() {
    let mut e = complete_breakdown("time-order", 10, true, vec![]);
    e.part_received_at_minute = Some(15);
    assert_blocked(&service_access(&["request"]), &input(vec![e]), RepairRecoveryBlockerCodeV1::InvalidEventOrder);
}

#[test]
fn duplicate_episode_id_blocks() {
    let a = complete_breakdown("same", 10, false, vec![]);
    let b = complete_breakdown("same", 100, false, vec![]);
    assert_blocked(&service_access(&["request"]), &input(vec![a, b]), RepairRecoveryBlockerCodeV1::DuplicateEpisodeId);
}

#[test]
fn unknown_affected_request_blocks() {
    let e = complete_breakdown("unknown-request", 10, false, vec!["missing"]);
    assert_blocked(&service_access(&["known"]), &input(vec![e]), RepairRecoveryBlockerCodeV1::UnknownAffectedRequest);
}

#[test]
fn duplicate_affected_request_link_within_episode_blocks() {
    let e = complete_breakdown("duplicate-link", 10, false, vec!["r1", "r1"]);
    assert_blocked(&service_access(&["r1"]), &input(vec![e]), RepairRecoveryBlockerCodeV1::DuplicateAffectedRequestLink);
}

#[test]
fn scope_mismatch_blocks() {
    let mut r = input(vec![complete_breakdown("scope", 10, false, vec![])]);
    r.episodes[0].scope_ref = "other-scope".into();
    assert_blocked(&service_access(&["request"]), &r, RepairRecoveryBlockerCodeV1::ScopeMismatch);
}

#[test]
fn period_mismatch_blocks() {
    let mut r = input(vec![complete_breakdown("period", 10, false, vec![])]);
    r.episodes[0].period_ref = "other-period".into();
    assert_blocked(&service_access(&["request"]), &r, RepairRecoveryBlockerCodeV1::PeriodMismatch);
}

#[test]
fn currency_mismatch_blocks() {
    let mut r = input(vec![complete_breakdown("currency", 10, false, vec![])]);
    r.episodes[0].currency_code = "USD".into();
    assert_blocked(&service_access(&["request"]), &r, RepairRecoveryBlockerCodeV1::CurrencyMismatch);
}

#[test]
fn missing_asset_configuration_blocks() {
    let mut e = complete_breakdown("no-config", 10, false, vec![]);
    e.asset_configuration_ref.clear();
    assert_blocked(&service_access(&["request"]), &input(vec![e]), RepairRecoveryBlockerCodeV1::EmptyIdentity);
}

#[test]
fn stale_evidence_blocks() {
    let mut e = complete_breakdown("stale", 10, false, vec![]);
    e.evidence.state = CostEvidenceStateV1::Stale;
    assert_blocked(&service_access(&["request"]), &input(vec![e]), RepairRecoveryBlockerCodeV1::StaleEvidence);
}

#[test]
fn malformed_evidence_digest_blocks() {
    let mut e = complete_breakdown("digest", 10, false, vec![]);
    e.evidence = ServiceAccessEvidenceV1::source_referenced("repair://receipt", "bad-digest");
    assert_blocked(&service_access(&["request"]), &input(vec![e]), RepairRecoveryBlockerCodeV1::MalformedEvidence);
}

#[test]
fn source_referenced_events_are_not_called_authenticated() {
    let mut access = service_access(&["r1"]);
    access.requests[0].evidence = ServiceAccessEvidenceV1::source_referenced("service://request", "a".repeat(64));
    let mut e = complete_breakdown("source", 10, false, vec!["r1"]);
    e.evidence = ServiceAccessEvidenceV1::source_referenced("repair://events", "b".repeat(64));
    let result = assess_repair_recovery_v1(&access, &input(vec![e]));
    assert_eq!(result.calculation_disposition, RepairRecoveryCalculationDispositionV1::Calculated);
    assert_eq!(result.evidence_disposition, CostEvidenceDispositionV1::SourceReferencedInputs);
}

#[test]
fn incomplete_log_keeps_coverage_flag_false() {
    let mut r = input(vec![complete_breakdown("partial", 10, false, vec![])]);
    r.declared_log_complete = false;
    let result = assess_repair_recovery_v1(&service_access(&["request"]), &r);
    assert!(!result.breakdown.unwrap().declared_log_complete);
}

#[test]
fn zero_episodes_does_not_claim_no_incidents_when_log_is_partial() {
    let mut r = input(vec![]);
    r.declared_log_complete = false;
    let result = assess_repair_recovery_v1(&service_access(&["request"]), &r);
    let b = result.breakdown.unwrap();
    assert_eq!(b.episode_count, 0);
    assert!(!b.declared_log_complete);
}

#[test]
fn external_interruption_is_not_classified_as_breakdown() {
    let mut e = episode("weather", InterruptionCauseV1::WeatherOrSiteAccess, vec!["r1"]);
    e.service_restored_at_minute = Some(20);
    e.asset_verified_available_at_minute = Some(21);
    let result = assess_repair_recovery_v1(&service_access(&["r1"]), &input(vec![e]));
    let b = result.breakdown.unwrap();
    assert_eq!(b.breakdown_count, 0);
    assert_eq!(b.external_interruption_count, 1);
    assert_eq!(b.external_interruption_p50_minutes, Some(11));
    assert_eq!(b.breakdown_restore_p50_minutes, None);
}

#[test]
fn scheduled_maintenance_is_reported_separately_from_breakdown() {
    let mut e = episode("maintenance", InterruptionCauseV1::ScheduledMaintenance, vec![]);
    e.diagnosis_completed_at_minute = Some(12);
    e.repair_started_at_minute = Some(13);
    e.repair_completed_at_minute = Some(18);
    e.service_restored_at_minute = Some(19);
    e.asset_verified_available_at_minute = Some(20);
    let b = assess_repair_recovery_v1(&service_access(&["request"]), &input(vec![e])).breakdown.unwrap();
    assert_eq!(b.breakdown_count, 0);
    assert_eq!(b.scheduled_maintenance_count, 1);
}

#[test]
fn required_part_needs_requirement_timestamp() {
    let mut e = episode("part-no-required-time", InterruptionCauseV1::Breakdown, vec![]);
    e.failure_detected_at_minute = Some(10);
    e.asset_removed_at_minute = Some(11);
    e.part_requirement = PartRequirementV1::Required;
    assert_blocked(&service_access(&["request"]), &input(vec![e]), RepairRecoveryBlockerCodeV1::InvalidPartRequirementShape);
}

#[test]
fn repair_cannot_start_before_required_part_arrives() {
    let mut e = complete_breakdown("repair-early", 10, true, vec![]);
    e.part_received_at_minute = None;
    assert_blocked(&service_access(&["request"]), &input(vec![e]), RepairRecoveryBlockerCodeV1::InvalidPartRequirementShape);
}

#[test]
fn exact_timestamps_serialize_as_decimal_strings() {
    let e = complete_breakdown("json", 9_007_199_254_740_993, false, vec![]);
    let value = serde_json::to_value(e).unwrap();
    assert_eq!(value["outage_started_at_minute"], "9007199254740993");
}
