// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent request lifecycle oracle cases. All records are synthetic fixtures.

use symtropy_garden_sim::field_robot_cost::{
    CostEvidenceDispositionV1, CostEvidenceStateV1, ServiceUnitV1,
};
use symtropy_garden_sim::service_access::{
    FarmerDeliveryDecisionV1, MINIMUM_COHORT_SIZE_V1, SERVICE_ACCESS_SCHEMA_VERSION_V1,
    ServiceAccessBlockerCodeV1, ServiceAccessCalculationDispositionV1, ServiceAccessEvidenceV1,
    ServiceAccessInputV1, ServiceAccessRequestV1, ServiceNoOutcomeReasonV1,
    TravelChargeTreatmentV1, assess_service_access_v1,
};

fn request(id: &str, at: u64) -> ServiceAccessRequestV1 {
    ServiceAccessRequestV1 {
        request_id: id.into(),
        scenario_id: "access-oracle-v1".into(),
        scope_ref: "cooperative-scope-v1".into(),
        period_ref: "season-period-v1".into(),
        currency_code: "ZAR".into(),
        task_code: "weeding".into(),
        service_unit: ServiceUnitV1::SquareMetresCompleted,
        requested_units: 1_000,
        requested_at_minute: at,
        useful_window_deadline_minute: at + 30,
        quote_at_minute: None,
        quoted_fee_minor: None,
        price_basis_unit: None,
        quote_valid_until_minute: None,
        travel_charge_treatment: None,
        travel_fee_minor: None,
        booking_at_minute: None,
        completion_at_minute: None,
        delivered_units: None,
        actual_travel_minutes: None,
        delivery_decision_at_minute: None,
        delivery_decision: None,
        no_outcome_reason: None,
        cohort_ref: None,
        cohort_consent: false,
        evidence: ServiceAccessEvidenceV1::synthetic_fixture(id),
    }
}

fn accepted_request(id: &str, at: u64, done: u64, deadline: u64) -> ServiceAccessRequestV1 {
    let mut r = request(id, at);
    r.useful_window_deadline_minute = deadline;
    r.quote_at_minute = Some(at + 1);
    r.quoted_fee_minor = Some(10_000);
    r.price_basis_unit = Some(r.service_unit);
    r.quote_valid_until_minute = Some(at + 100);
    r.travel_charge_treatment = Some(TravelChargeTreatmentV1::IncludedInQuote);
    r.booking_at_minute = Some(at + 2);
    r.completion_at_minute = Some(done);
    r.delivered_units = Some(1_000);
    r.delivery_decision_at_minute = Some(done + 1);
    r.delivery_decision = Some(FarmerDeliveryDecisionV1::Accepted);
    r
}

fn input(requests: Vec<ServiceAccessRequestV1>) -> ServiceAccessInputV1 {
    ServiceAccessInputV1 {
        schema_version: SERVICE_ACCESS_SCHEMA_VERSION_V1,
        scenario_id: "access-oracle-v1".into(),
        scope_ref: "cooperative-scope-v1".into(),
        period_ref: "season-period-v1".into(),
        currency_code: "ZAR".into(),
        minimum_cohort_size: MINIMUM_COHORT_SIZE_V1,
        requests,
    }
}

fn assert_blocked(input: &ServiceAccessInputV1, code: ServiceAccessBlockerCodeV1) {
    let result = assess_service_access_v1(input);
    assert_eq!(
        result.calculation_disposition,
        ServiceAccessCalculationDispositionV1::Blocked
    );
    assert_eq!(
        result.evidence_disposition,
        CostEvidenceDispositionV1::Blocked
    );
    assert!(result.blockers.iter().any(|blocker| blocker.code == code));
    assert!(result.breakdown.is_none());
}

#[test]
fn lifecycle_denominators_and_nearest_rank_waits_match_hand_calculation() {
    let mut first = accepted_request("r-1", 0, 10, 20);
    let mut second = accepted_request("r-2", 30, 50, 45);
    first.actual_travel_minutes = Some(3);
    second.actual_travel_minutes = Some(7);
    let mut third = request("r-3", 60);
    third.no_outcome_reason = Some(ServiceNoOutcomeReasonV1::NoProviderAvailable);
    let report = assess_service_access_v1(&input(vec![first, second, third]));
    assert_eq!(
        report.calculation_disposition,
        ServiceAccessCalculationDispositionV1::Calculated
    );
    assert_eq!(
        report.evidence_disposition,
        CostEvidenceDispositionV1::ProvisionalInputs
    );
    let b = report.breakdown.unwrap();
    assert_eq!(b.request_count, 3);
    assert_eq!(b.quote_count, 2);
    assert_eq!(b.booking_count, 2);
    assert_eq!(b.completed_count, 2);
    assert_eq!(b.farmer_accepted_count, 2);
    assert_eq!(b.no_outcome_count, 1);
    assert_eq!(b.unresolved_count, 0);
    assert_eq!(b.accepted_within_useful_window_count, 1);
    assert_eq!(b.accepted_after_useful_window_count, 1);
    assert_eq!(b.completion_wait_p50_minutes, Some(10));
    assert_eq!(b.completion_wait_p90_minutes, Some(20));
}

#[test]
fn completed_pending_delivery_decision_is_not_promoted_to_acceptance() {
    let mut r = accepted_request("pending", 0, 10, 30);
    r.delivery_decision_at_minute = None;
    r.delivery_decision = None;
    let report = assess_service_access_v1(&input(vec![r]));
    assert_eq!(
        report.calculation_disposition,
        ServiceAccessCalculationDispositionV1::Calculated
    );
    let breakdown = report.breakdown.unwrap();
    assert_eq!(breakdown.request_count, 1);
    assert_eq!(breakdown.completed_count, 1);
    assert_eq!(breakdown.farmer_accepted_count, 0);
    assert_eq!(breakdown.completed_pending_decision_count, 1);
}

#[test]
fn duplicate_request_id_blocks_without_partial_aggregate() {
    assert_blocked(
        &input(vec![request("same", 0), request("same", 60)]),
        ServiceAccessBlockerCodeV1::DuplicateRequestId,
    );
}

#[test]
fn scope_mismatch_blocks() {
    let mut r = request("scope", 0);
    r.scope_ref = "other-scope".into();
    assert_blocked(&input(vec![r]), ServiceAccessBlockerCodeV1::ScopeMismatch);
}

#[test]
fn stale_request_evidence_blocks() {
    let mut r = request("stale", 0);
    r.evidence.state = CostEvidenceStateV1::Stale;
    assert_blocked(&input(vec![r]), ServiceAccessBlockerCodeV1::StaleEvidence);
}

#[test]
fn unpriced_quote_blocks() {
    let mut r = request("quote", 0);
    r.quote_at_minute = Some(1);
    assert_blocked(
        &input(vec![r]),
        ServiceAccessBlockerCodeV1::IncompleteQuoteTerms,
    );
}

#[test]
fn booking_without_quote_blocks() {
    let mut r = request("booking", 0);
    r.booking_at_minute = Some(2);
    assert_blocked(
        &input(vec![r]),
        ServiceAccessBlockerCodeV1::BookingWithoutQuote,
    );
}

#[test]
fn completion_without_booking_blocks() {
    let mut r = request("completion", 0);
    r.completion_at_minute = Some(10);
    r.delivered_units = Some(1);
    assert_blocked(
        &input(vec![r]),
        ServiceAccessBlockerCodeV1::CompletionWithoutBooking,
    );
}

#[test]
fn timestamp_reversal_blocks() {
    let mut r = accepted_request("order", 0, 8, 30);
    r.completion_at_minute = Some(1);
    r.delivery_decision_at_minute = Some(2);
    assert_blocked(
        &input(vec![r]),
        ServiceAccessBlockerCodeV1::InvalidLifecycleOrder,
    );
}

#[test]
fn no_show_requires_booking_and_is_not_counted_as_fulfilled() {
    let mut r = request("no-show", 0);
    r.no_outcome_reason = Some(ServiceNoOutcomeReasonV1::ProviderNoShow);
    assert_blocked(
        &input(vec![r]),
        ServiceAccessBlockerCodeV1::NoOutcomeShapeMismatch,
    );
}

#[test]
fn quote_unit_mismatch_blocks() {
    let mut r = accepted_request("unit", 0, 10, 30);
    r.price_basis_unit = Some(ServiceUnitV1::MissionsCompleted);
    assert_blocked(
        &input(vec![r]),
        ServiceAccessBlockerCodeV1::PriceUnitMismatch,
    );
}

#[test]
fn separate_travel_charge_requires_explicit_fee() {
    let mut r = accepted_request("travel-fee", 0, 10, 30);
    r.travel_charge_treatment = Some(TravelChargeTreatmentV1::SeparateQuotedFee);
    assert_blocked(
        &input(vec![r]),
        ServiceAccessBlockerCodeV1::IncompleteQuoteTerms,
    );
}

#[test]
fn malformed_source_digest_blocks() {
    let mut r = request("digest", 0);
    r.evidence = ServiceAccessEvidenceV1::source_referenced("receipt://request", "bad-digest");
    assert_blocked(&input(vec![r]), ServiceAccessBlockerCodeV1::InvalidEvidence);
}

#[test]
fn cohort_below_threshold_is_suppressed() {
    let mut requests = Vec::new();
    for index in 0..9 {
        let mut r = accepted_request(
            &format!("cohort-{index}"),
            index * 100,
            index * 100 + 10,
            index * 100 + 30,
        );
        r.cohort_ref = Some("cohort-a".into());
        r.cohort_consent = true;
        requests.push(r);
    }
    let result = assess_service_access_v1(&input(requests));
    assert_eq!(
        result.calculation_disposition,
        ServiceAccessCalculationDispositionV1::Calculated
    );
    assert!(result.breakdown.unwrap().cohort_summaries.is_empty());
}

#[test]
fn cohort_at_threshold_is_aggregated_after_explicit_consent() {
    let mut requests = Vec::new();
    for index in 0..10 {
        let mut r = accepted_request(
            &format!("cohort-ok-{index}"),
            index * 100,
            index * 100 + 10,
            index * 100 + 30,
        );
        r.cohort_ref = Some("consented-group".into());
        r.cohort_consent = true;
        requests.push(r);
    }
    let result = assess_service_access_v1(&input(requests));
    let summaries = result.breakdown.unwrap().cohort_summaries;
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].request_count, 10);
    assert_eq!(summaries[0].farmer_accepted_count, 10);
    assert_eq!(summaries[0].fulfilled_within_useful_window_count, 10);
}

#[test]
fn cohort_label_without_consent_blocks() {
    let mut r = request("cohort-consent", 0);
    r.cohort_ref = Some("group-a".into());
    assert_blocked(
        &input(vec![r]),
        ServiceAccessBlockerCodeV1::InvalidCohortConsent,
    );
}

#[test]
fn unsupported_small_cohort_threshold_blocks() {
    let mut assessment = input(vec![request("k", 0)]);
    assessment.minimum_cohort_size = 9;
    assert_blocked(
        &assessment,
        ServiceAccessBlockerCodeV1::CohortThresholdTooSmall,
    );
}

#[test]
fn missing_evidence_blocks() {
    let mut r = request("missing", 0);
    r.evidence.state = CostEvidenceStateV1::Missing;
    r.evidence.source_ref = None;
    assert_blocked(&input(vec![r]), ServiceAccessBlockerCodeV1::MissingEvidence);
}

#[test]
fn quote_expired_before_booking_blocks() {
    let mut r = accepted_request("expired", 0, 20, 30);
    r.quote_valid_until_minute = Some(5);
    r.booking_at_minute = Some(10);
    assert_blocked(
        &input(vec![r]),
        ServiceAccessBlockerCodeV1::QuoteExpiredBeforeBooking,
    );
}
