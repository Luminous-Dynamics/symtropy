// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent, synthetic quote-decision and farmer-budget oracle cases.

use symtropy_garden_sim::field_robot_cost::{
    CostEvidenceDispositionV1, CostEvidenceStateV1, ServiceUnitV1,
};
use symtropy_garden_sim::quote_affordability::{
    FarmerQuoteDecisionV1, FarmerQuoteResponseV1, QUOTE_AFFORDABILITY_SCHEMA_VERSION_V1,
    QuoteAffordabilityBlockerCodeV1, QuoteAffordabilityCalculationDispositionV1,
    QuoteAffordabilityInputV1, QuoteDeclineReasonV1, QuoteFeeAmountScopeV1,
    assess_quote_affordability_v1,
};
use symtropy_garden_sim::service_access::{
    SERVICE_ACCESS_SCHEMA_VERSION_V1, ServiceAccessEvidenceV1, ServiceAccessInputV1,
    ServiceAccessRequestV1, ServiceNoOutcomeReasonV1, TravelChargeTreatmentV1,
};

fn request(id: &str, decision: FarmerQuoteDecisionV1) -> ServiceAccessRequestV1 {
    let mut r = ServiceAccessRequestV1 {
        request_id: id.into(),
        scenario_id: "quote-oracle-v1".into(),
        scope_ref: "coop-scope-v1".into(),
        period_ref: "season-v1".into(),
        currency_code: "ZAR".into(),
        task_code: "weeding".into(),
        service_unit: ServiceUnitV1::SquareMetresCompleted,
        requested_units: 100,
        requested_at_minute: 0,
        useful_window_deadline_minute: 100,
        quote_at_minute: Some(10),
        quoted_fee_minor: Some(10),
        price_basis_unit: Some(ServiceUnitV1::SquareMetresCompleted),
        quote_valid_until_minute: Some(50),
        travel_charge_treatment: Some(TravelChargeTreatmentV1::SeparateQuotedFee),
        travel_fee_minor: Some(250),
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
    };
    if decision == FarmerQuoteDecisionV1::Accepted {
        r.booking_at_minute = Some(21);
    }
    if decision == FarmerQuoteDecisionV1::Declined {
        r.no_outcome_reason = Some(ServiceNoOutcomeReasonV1::FarmerDeclinedQuote);
    }
    r
}

fn response(id: &str, decision: FarmerQuoteDecisionV1) -> FarmerQuoteResponseV1 {
    let decision_at = match decision {
        FarmerQuoteDecisionV1::Accepted | FarmerQuoteDecisionV1::Declined => Some(20),
        FarmerQuoteDecisionV1::Expired => Some(50),
        FarmerQuoteDecisionV1::Pending => None,
    };
    FarmerQuoteResponseV1 {
        request_id: id.into(),
        decision,
        decision_at_minute: decision_at,
        fee_amount_scope: QuoteFeeAmountScopeV1::PerServiceUnit,
        other_mandatory_fees_minor: 50,
        farmer_max_total_budget_minor: None,
        budget_disclosure_consent: false,
        decline_reason: if decision == FarmerQuoteDecisionV1::Declined {
            Some(QuoteDeclineReasonV1::TimingNotSuitable)
        } else {
            None
        },
        evidence: ServiceAccessEvidenceV1::synthetic_fixture(id),
    }
}

fn service_access(requests: Vec<ServiceAccessRequestV1>) -> ServiceAccessInputV1 {
    ServiceAccessInputV1 {
        schema_version: SERVICE_ACCESS_SCHEMA_VERSION_V1,
        scenario_id: "quote-oracle-v1".into(),
        scope_ref: "coop-scope-v1".into(),
        period_ref: "season-v1".into(),
        currency_code: "ZAR".into(),
        minimum_cohort_size: 10,
        requests,
    }
}

fn input(responses: Vec<FarmerQuoteResponseV1>) -> QuoteAffordabilityInputV1 {
    QuoteAffordabilityInputV1 {
        schema_version: QUOTE_AFFORDABILITY_SCHEMA_VERSION_V1,
        scenario_id: "quote-oracle-v1".into(),
        scope_ref: "coop-scope-v1".into(),
        period_ref: "season-v1".into(),
        currency_code: "ZAR".into(),
        responses,
    }
}

fn assert_blocked(
    access: &ServiceAccessInputV1,
    quote_input: &QuoteAffordabilityInputV1,
    expected: QuoteAffordabilityBlockerCodeV1,
) {
    let result = assess_quote_affordability_v1(access, quote_input);
    assert_eq!(
        result.calculation_disposition,
        QuoteAffordabilityCalculationDispositionV1::Blocked
    );
    assert_eq!(
        result.evidence_disposition,
        CostEvidenceDispositionV1::Blocked
    );
    assert!(result.blockers.iter().any(|b| b.code == expected));
    assert!(result.breakdown.is_none());
}

#[test]
fn mixed_quote_decisions_and_budget_comparisons_match_hand_calculation() {
    let access = service_access(vec![
        request("accept", FarmerQuoteDecisionV1::Accepted),
        request("decline", FarmerQuoteDecisionV1::Declined),
        request("expire", FarmerQuoteDecisionV1::Expired),
        request("pending", FarmerQuoteDecisionV1::Pending),
    ]);
    let mut accepted = response("accept", FarmerQuoteDecisionV1::Accepted);
    accepted.farmer_max_total_budget_minor = Some(1_300);
    accepted.budget_disclosure_consent = true;
    let mut declined = response("decline", FarmerQuoteDecisionV1::Declined);
    declined.decline_reason = Some(QuoteDeclineReasonV1::PriceAboveBudget);
    declined.farmer_max_total_budget_minor = Some(1_200);
    declined.budget_disclosure_consent = true;
    let quote_input = input(vec![
        accepted,
        declined,
        response("expire", FarmerQuoteDecisionV1::Expired),
        response("pending", FarmerQuoteDecisionV1::Pending),
    ]);
    let result = assess_quote_affordability_v1(&access, &quote_input);
    assert_eq!(
        result.calculation_disposition,
        QuoteAffordabilityCalculationDispositionV1::Calculated
    );
    assert_eq!(
        result.evidence_disposition,
        CostEvidenceDispositionV1::ProvisionalInputs
    );
    let b = result.breakdown.unwrap();
    assert_eq!(b.quote_count, 4);
    assert_eq!(b.accepted_count, 1);
    assert_eq!(b.declined_count, 1);
    assert_eq!(b.expired_count, 1);
    assert_eq!(b.pending_count, 1);
    assert_eq!(b.declined_price_count, 1);
    assert_eq!(b.budget_disclosed_count, 2);
    assert_eq!(b.quotes_within_declared_budget_count, 1);
    assert_eq!(b.quotes_above_declared_budget_count, 1);
    assert_eq!(b.affordability_unknown_count, 2);
}

#[test]
fn no_quotes_is_zero_quotes_not_an_affordability_claim() {
    let mut r = request("unquoted", FarmerQuoteDecisionV1::Pending);
    r.quote_at_minute = None;
    r.quoted_fee_minor = None;
    r.price_basis_unit = None;
    r.quote_valid_until_minute = None;
    r.travel_charge_treatment = None;
    r.travel_fee_minor = None;
    let result = assess_quote_affordability_v1(&service_access(vec![r]), &input(vec![]));
    assert_eq!(
        result.calculation_disposition,
        QuoteAffordabilityCalculationDispositionV1::Calculated
    );
    assert_eq!(result.breakdown.unwrap().quote_count, 0);
}

#[test]
fn missing_quote_response_blocks() {
    assert_blocked(
        &service_access(vec![request("missing", FarmerQuoteDecisionV1::Pending)]),
        &input(vec![]),
        QuoteAffordabilityBlockerCodeV1::MissingQuoteResponse,
    );
}

#[test]
fn duplicate_response_blocks() {
    let a = service_access(vec![request("dup", FarmerQuoteDecisionV1::Accepted)]);
    let r = response("dup", FarmerQuoteDecisionV1::Accepted);
    assert_blocked(
        &a,
        &input(vec![r.clone(), r]),
        QuoteAffordabilityBlockerCodeV1::DuplicateResponse,
    );
}

#[test]
fn response_for_unquoted_request_blocks() {
    let mut r = request("plain", FarmerQuoteDecisionV1::Pending);
    r.quote_at_minute = None;
    r.quoted_fee_minor = None;
    r.price_basis_unit = None;
    r.quote_valid_until_minute = None;
    r.travel_charge_treatment = None;
    r.travel_fee_minor = None;
    assert_blocked(
        &service_access(vec![r]),
        &input(vec![response("plain", FarmerQuoteDecisionV1::Pending)]),
        QuoteAffordabilityBlockerCodeV1::ResponseForUnquotedRequest,
    );
}

#[test]
fn response_for_unknown_request_blocks() {
    assert_blocked(
        &service_access(vec![request("known", FarmerQuoteDecisionV1::Pending)]),
        &input(vec![response("ghost", FarmerQuoteDecisionV1::Pending)]),
        QuoteAffordabilityBlockerCodeV1::ResponseForUnknownRequest,
    );
}

#[test]
fn scope_mismatch_blocks() {
    let mut quote_input = input(vec![response("scope", FarmerQuoteDecisionV1::Pending)]);
    quote_input.scope_ref = "other-scope".into();
    assert_blocked(
        &service_access(vec![request("scope", FarmerQuoteDecisionV1::Pending)]),
        &quote_input,
        QuoteAffordabilityBlockerCodeV1::ScopeMismatch,
    );
}

#[test]
fn currency_mismatch_blocks() {
    let mut quote_input = input(vec![response("currency", FarmerQuoteDecisionV1::Pending)]);
    quote_input.currency_code = "USD".into();
    assert_blocked(
        &service_access(vec![request("currency", FarmerQuoteDecisionV1::Pending)]),
        &quote_input,
        QuoteAffordabilityBlockerCodeV1::CurrencyMismatch,
    );
}

#[test]
fn accepted_decision_requires_time() {
    let mut r = response("accepted-time", FarmerQuoteDecisionV1::Accepted);
    r.decision_at_minute = None;
    assert_blocked(
        &service_access(vec![request(
            "accepted-time",
            FarmerQuoteDecisionV1::Accepted,
        )]),
        &input(vec![r]),
        QuoteAffordabilityBlockerCodeV1::InvalidDecisionShape,
    );
}

#[test]
fn booking_cannot_have_pending_quote_decision() {
    assert_blocked(
        &service_access(vec![request("booked", FarmerQuoteDecisionV1::Accepted)]),
        &input(vec![response("booked", FarmerQuoteDecisionV1::Pending)]),
        QuoteAffordabilityBlockerCodeV1::InvalidDecisionShape,
    );
}

#[test]
fn declined_quote_requires_reason() {
    let mut r = response("decline-reason", FarmerQuoteDecisionV1::Declined);
    r.decline_reason = None;
    assert_blocked(
        &service_access(vec![request(
            "decline-reason",
            FarmerQuoteDecisionV1::Declined,
        )]),
        &input(vec![r]),
        QuoteAffordabilityBlockerCodeV1::DeclineReasonMismatch,
    );
}

#[test]
fn pending_quote_cannot_have_decision_timestamp() {
    let mut r = response("pending-time", FarmerQuoteDecisionV1::Pending);
    r.decision_at_minute = Some(20);
    assert_blocked(
        &service_access(vec![request(
            "pending-time",
            FarmerQuoteDecisionV1::Pending,
        )]),
        &input(vec![r]),
        QuoteAffordabilityBlockerCodeV1::InvalidDecisionShape,
    );
}

#[test]
fn expiry_must_not_precede_quote_validity() {
    let mut r = response("early-expiry", FarmerQuoteDecisionV1::Expired);
    r.decision_at_minute = Some(49);
    assert_blocked(
        &service_access(vec![request(
            "early-expiry",
            FarmerQuoteDecisionV1::Expired,
        )]),
        &input(vec![r]),
        QuoteAffordabilityBlockerCodeV1::InvalidDecisionTime,
    );
}

#[test]
fn budget_requires_explicit_disclosure_consent() {
    let mut r = response("no-consent", FarmerQuoteDecisionV1::Pending);
    r.farmer_max_total_budget_minor = Some(1_000);
    assert_blocked(
        &service_access(vec![request("no-consent", FarmerQuoteDecisionV1::Pending)]),
        &input(vec![r]),
        QuoteAffordabilityBlockerCodeV1::InvalidBudgetConsent,
    );
}

#[test]
fn stale_quote_response_evidence_blocks() {
    let mut r = response("stale", FarmerQuoteDecisionV1::Pending);
    r.evidence.state = CostEvidenceStateV1::Stale;
    assert_blocked(
        &service_access(vec![request("stale", FarmerQuoteDecisionV1::Pending)]),
        &input(vec![r]),
        QuoteAffordabilityBlockerCodeV1::StaleEvidence,
    );
}

#[test]
fn malformed_evidence_digest_blocks() {
    let mut r = response("digest", FarmerQuoteDecisionV1::Pending);
    r.evidence = ServiceAccessEvidenceV1::source_referenced("quote://receipt", "not-a-digest");
    assert_blocked(
        &service_access(vec![request("digest", FarmerQuoteDecisionV1::Pending)]),
        &input(vec![r]),
        QuoteAffordabilityBlockerCodeV1::InvalidEvidence,
    );
}

#[test]
fn per_unit_quote_multiplication_overflow_blocks() {
    let mut req = request("overflow-unit", FarmerQuoteDecisionV1::Pending);
    req.requested_units = u64::MAX;
    let r = response("overflow-unit", FarmerQuoteDecisionV1::Pending);
    assert_blocked(
        &service_access(vec![req]),
        &input(vec![r]),
        QuoteAffordabilityBlockerCodeV1::ArithmeticOverflow,
    );
}

#[test]
fn additional_fee_addition_overflow_blocks() {
    let mut r = response("overflow-fee", FarmerQuoteDecisionV1::Pending);
    r.other_mandatory_fees_minor = u64::MAX;
    assert_blocked(
        &service_access(vec![request(
            "overflow-fee",
            FarmerQuoteDecisionV1::Pending,
        )]),
        &input(vec![r]),
        QuoteAffordabilityBlockerCodeV1::ArithmeticOverflow,
    );
}

#[test]
fn price_above_budget_reason_conflicts_with_budget_at_or_above_quote() {
    let mut r = response("bad-reason", FarmerQuoteDecisionV1::Declined);
    r.decline_reason = Some(QuoteDeclineReasonV1::PriceAboveBudget);
    r.farmer_max_total_budget_minor = Some(1_300);
    r.budget_disclosure_consent = true;
    assert_blocked(
        &service_access(vec![request("bad-reason", FarmerQuoteDecisionV1::Declined)]),
        &input(vec![r]),
        QuoteAffordabilityBlockerCodeV1::DeclineReasonMismatch,
    );
}

#[test]
fn total_for_requested_scope_does_not_multiply_by_quantity() {
    let access = service_access(vec![request(
        "scope-total",
        FarmerQuoteDecisionV1::Accepted,
    )]);
    let mut r = response("scope-total", FarmerQuoteDecisionV1::Accepted);
    r.fee_amount_scope = QuoteFeeAmountScopeV1::TotalForRequestedScope;
    r.farmer_max_total_budget_minor = Some(310);
    r.budget_disclosure_consent = true;
    let result = assess_quote_affordability_v1(&access, &input(vec![r]));
    let b = result.breakdown.unwrap();
    assert_eq!(b.quotes_within_declared_budget_count, 1);
    assert_eq!(b.quotes_above_declared_budget_count, 0);
}

#[test]
fn source_referenced_quote_evidence_is_not_called_authenticated() {
    let mut access = service_access(vec![request("source", FarmerQuoteDecisionV1::Pending)]);
    access.requests[0].evidence =
        ServiceAccessEvidenceV1::source_referenced("service://request", "a".repeat(64));
    let mut r = response("source", FarmerQuoteDecisionV1::Pending);
    r.evidence = ServiceAccessEvidenceV1::source_referenced("service://quote", "b".repeat(64));
    let result = assess_quote_affordability_v1(&access, &input(vec![r]));
    assert_eq!(
        result.evidence_disposition,
        CostEvidenceDispositionV1::SourceReferencedInputs
    );
    assert_eq!(
        result.calculation_disposition,
        QuoteAffordabilityCalculationDispositionV1::Calculated
    );
}

#[test]
fn exact_budget_integer_serializes_as_decimal_string() {
    let mut r = response("json-exact", FarmerQuoteDecisionV1::Pending);
    r.farmer_max_total_budget_minor = Some(9_007_199_254_740_993);
    r.budget_disclosure_consent = true;
    let value = serde_json::to_value(r).unwrap();
    assert_eq!(value["farmer_max_total_budget_minor"], "9007199254740993");
}
