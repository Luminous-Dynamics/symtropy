// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Farmer quote decisions and voluntary budget comparisons over canonical service
//! access records. No household income or undisclosed affordability is inferred.

use crate::field_robot_cost::{CostEvidenceDispositionV1, CostEvidenceStateV1};
use crate::service_access::{
    assess_service_access_v1, ServiceAccessCalculationDispositionV1, ServiceAccessEvidenceV1,
    ServiceAccessInputV1, ServiceNoOutcomeReasonV1, TravelChargeTreatmentV1,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const QUOTE_AFFORDABILITY_SCHEMA_VERSION_V1: u16 = 1;

fn serialize_u64<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
where S: serde::Serializer {
    serializer.serialize_str(&value.to_string())
}
fn deserialize_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where D: serde::Deserializer<'de> {
    String::deserialize(deserializer)?.parse::<u64>().map_err(serde::de::Error::custom)
}
mod option_u64_string {
    use super::*;
    pub fn serialize<S>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        match value { Some(v) => serializer.serialize_some(&v.to_string()), None => serializer.serialize_none() }
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
    where D: serde::Deserializer<'de> {
        Option::<String>::deserialize(deserializer)?
            .map(|raw| raw.parse::<u64>().map_err(serde::de::Error::custom)).transpose()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarmerQuoteDecisionV1 { Accepted, Declined, Expired, Pending }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuoteFeeAmountScopeV1 { TotalForRequestedScope, PerServiceUnit }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuoteDeclineReasonV1 {
    PriceAboveBudget,
    TimingNotSuitable,
    TaskScopeMismatch,
    TrustOrSafetyConcern,
    AlternativeProvider,
    Other,
    NotStated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuoteAffordabilityBlockerCodeV1 {
    UnsupportedSchema,
    CanonicalServiceAccessBlocked,
    EmptyIdentity,
    ScenarioMismatch,
    ScopeMismatch,
    PeriodMismatch,
    CurrencyMismatch,
    DuplicateResponse,
    MissingQuoteResponse,
    ResponseForUnknownRequest,
    ResponseForUnquotedRequest,
    InvalidDecisionShape,
    BookingDecisionMismatch,
    DeclineReasonMismatch,
    InvalidDecisionTime,
    InvalidEvidence,
    MissingEvidence,
    StaleEvidence,
    ConflictingEvidence,
    InvalidBudgetConsent,
    ArithmeticOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteAffordabilityBlockerV1 {
    pub code: QuoteAffordabilityBlockerCodeV1,
    pub request_id: Option<String>,
    pub detail: String,
}

/// Exactly one quote-response snapshot per quoted request. Additional mandatory
/// fees are a total for the job. Budgets are retained only with explicit consent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FarmerQuoteResponseV1 {
    pub request_id: String,
    pub decision: FarmerQuoteDecisionV1,
    #[serde(serialize_with = "option_u64_string::serialize", deserialize_with = "option_u64_string::deserialize")]
    pub decision_at_minute: Option<u64>,
    pub fee_amount_scope: QuoteFeeAmountScopeV1,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub other_mandatory_fees_minor: u64,
    #[serde(serialize_with = "option_u64_string::serialize", deserialize_with = "option_u64_string::deserialize")]
    pub farmer_max_total_budget_minor: Option<u64>,
    pub budget_disclosure_consent: bool,
    pub decline_reason: Option<QuoteDeclineReasonV1>,
    pub evidence: ServiceAccessEvidenceV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteAffordabilityInputV1 {
    pub schema_version: u16,
    pub scenario_id: String,
    pub scope_ref: String,
    pub period_ref: String,
    pub currency_code: String,
    pub responses: Vec<FarmerQuoteResponseV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteAffordabilityBreakdownV1 {
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub quote_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub accepted_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub declined_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub expired_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub pending_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub declined_price_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub declined_timing_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub declined_scope_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub declined_trust_or_safety_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub declined_alternative_provider_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub declined_other_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub declined_not_stated_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub budget_disclosed_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub quotes_within_declared_budget_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub quotes_above_declared_budget_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub affordability_unknown_count: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuoteAffordabilityCalculationDispositionV1 { Calculated, Blocked }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteAffordabilityAssessmentV1 {
    pub scenario_id: String,
    pub calculation_disposition: QuoteAffordabilityCalculationDispositionV1,
    pub evidence_disposition: CostEvidenceDispositionV1,
    pub blockers: Vec<QuoteAffordabilityBlockerV1>,
    pub breakdown: Option<QuoteAffordabilityBreakdownV1>,
}

fn count(value: usize) -> u64 { u64::try_from(value).unwrap_or(u64::MAX) }
fn valid_currency_code(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_uppercase())
}
fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn push_blocker(
    blockers: &mut Vec<QuoteAffordabilityBlockerV1>,
    code: QuoteAffordabilityBlockerCodeV1,
    request_id: Option<&str>,
    detail: &str,
) {
    let blocker = QuoteAffordabilityBlockerV1 {
        code, request_id: request_id.map(str::to_owned), detail: detail.to_owned(),
    };
    if !blockers.contains(&blocker) { blockers.push(blocker); }
}
fn validate_evidence(response: &FarmerQuoteResponseV1, blockers: &mut Vec<QuoteAffordabilityBlockerV1>) {
    let id = Some(response.request_id.as_str());
    if response.evidence.artifact_sha256.as_deref().is_some_and(|v| !valid_sha256(v)) {
        push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::InvalidEvidence, id,
            "evidence digest is not 64 hexadecimal characters");
    }
    match response.evidence.state {
        CostEvidenceStateV1::SourceReferenced => {
            if response.evidence.source_ref.as_deref().is_none_or(|v| v.trim().is_empty())
                || response.evidence.artifact_sha256.as_deref().is_none_or(|v| !valid_sha256(v)) {
                push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::InvalidEvidence, id,
                    "source-referenced evidence requires a source reference and SHA-256-shaped digest");
            }
        }
        CostEvidenceStateV1::Estimated => {
            if response.evidence.source_ref.as_deref().is_none_or(|v| v.trim().is_empty()) {
                push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::InvalidEvidence, id,
                    "estimated evidence requires a source reference");
            }
        }
        CostEvidenceStateV1::SyntheticFixture => {
            if !response.evidence.source_ref.as_deref().is_some_and(|v| v.starts_with("synthetic-fixture:")) {
                push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::InvalidEvidence, id,
                    "synthetic evidence must use an explicit synthetic-fixture reference");
            }
        }
        CostEvidenceStateV1::Missing => push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::MissingEvidence, id,
            "quote-response evidence is missing"),
        CostEvidenceStateV1::Stale => push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::StaleEvidence, id,
            "quote-response evidence is stale"),
        CostEvidenceStateV1::Conflicting => push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::ConflictingEvidence, id,
            "quote-response evidence conflicts"),
    }
}

fn calculated_total_minor(
    request: &crate::service_access::ServiceAccessRequestV1,
    response: &FarmerQuoteResponseV1,
) -> Option<u64> {
    let fee = request.quoted_fee_minor?;
    let service_total = match response.fee_amount_scope {
        QuoteFeeAmountScopeV1::TotalForRequestedScope => fee,
        QuoteFeeAmountScopeV1::PerServiceUnit => fee.checked_mul(request.requested_units)?,
    };
    let with_travel = match request.travel_charge_treatment {
        Some(TravelChargeTreatmentV1::SeparateQuotedFee) => service_total.checked_add(request.travel_fee_minor?)?,
        Some(TravelChargeTreatmentV1::IncludedInQuote | TravelChargeTreatmentV1::NoTravelCharge) => service_total,
        Some(TravelChargeTreatmentV1::Unknown) | None => return None,
    };
    with_travel.checked_add(response.other_mandatory_fees_minor)
}

fn validate_response(
    request: &crate::service_access::ServiceAccessRequestV1,
    response: &FarmerQuoteResponseV1,
    blockers: &mut Vec<QuoteAffordabilityBlockerV1>,
) {
    let id = Some(response.request_id.as_str());
    if response.request_id.trim().is_empty() {
        push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::EmptyIdentity, id, "quote response request ID must not be blank");
    }
    if response.budget_disclosure_consent != response.farmer_max_total_budget_minor.is_some() {
        push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::InvalidBudgetConsent, id,
            "a stored farmer budget requires explicit disclosure consent; do not store a budget when consent is absent");
    }
    match response.decision {
        FarmerQuoteDecisionV1::Accepted => {
            if response.decision_at_minute.is_none() || response.decline_reason.is_some() {
                push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::InvalidDecisionShape, id,
                    "accepted quote requires a decision time and no decline reason");
            }
            if request.no_outcome_reason == Some(ServiceNoOutcomeReasonV1::FarmerDeclinedQuote) {
                push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::BookingDecisionMismatch, id, "accepted quote conflicts with farmer-declined outcome");
            }
        }
        FarmerQuoteDecisionV1::Declined => {
            if response.decision_at_minute.is_none() || response.decline_reason.is_none() {
                push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::DeclineReasonMismatch, id,
                    "declined quote requires a decision time and explicit reason category; NotStated is allowed");
            }
            if request.booking_at_minute.is_some() || request.no_outcome_reason != Some(ServiceNoOutcomeReasonV1::FarmerDeclinedQuote) {
                push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::BookingDecisionMismatch, id,
                    "declined quote requires no booking and a matching farmer-declined outcome");
            }
        }
        FarmerQuoteDecisionV1::Expired => {
            if response.decision_at_minute.is_none() || response.decline_reason.is_some()
                || request.booking_at_minute.is_some()
                || request.no_outcome_reason == Some(ServiceNoOutcomeReasonV1::FarmerDeclinedQuote) {
                push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::InvalidDecisionShape, id,
                    "expired quote requires expiry time, no decline reason and no booking");
            }
        }
        FarmerQuoteDecisionV1::Pending => {
            if response.decision_at_minute.is_some() || response.decline_reason.is_some()
                || request.booking_at_minute.is_some()
                || request.no_outcome_reason == Some(ServiceNoOutcomeReasonV1::FarmerDeclinedQuote) {
                push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::InvalidDecisionShape, id,
                    "pending quote must have no decision time, decline reason or booking");
            }
        }
    }
    if let Some(decision_at) = response.decision_at_minute {
        let quote_at = request.quote_at_minute.unwrap_or(u64::MAX);
        let valid_until = request.quote_valid_until_minute.unwrap_or(0);
        match response.decision {
            FarmerQuoteDecisionV1::Accepted | FarmerQuoteDecisionV1::Declined => {
                if decision_at < quote_at || decision_at > valid_until
                    || request.booking_at_minute.is_some_and(|at| at < decision_at) {
                    push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::InvalidDecisionTime, id,
                        "acceptance or decline must be within quote validity and precede any booking");
                }
            }
            FarmerQuoteDecisionV1::Expired => {
                if decision_at < valid_until {
                    push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::InvalidDecisionTime, id,
                        "expiry time must be at or after quote validity ends");
                }
            }
            FarmerQuoteDecisionV1::Pending => push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::InvalidDecisionTime, id,
                "pending quote must not carry a decision time"),
        }
    }
    if response.decline_reason.is_some_and(|_| response.decision != FarmerQuoteDecisionV1::Declined) {
        push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::DeclineReasonMismatch, id,
            "decline reason can only accompany a declined quote");
    }
    if response.decline_reason == Some(QuoteDeclineReasonV1::PriceAboveBudget)
        && let (Some(budget), Some(total)) = (response.farmer_max_total_budget_minor, calculated_total_minor(request, response))
        && total <= budget {
        push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::DeclineReasonMismatch, id,
            "PriceAboveBudget conflicts with a disclosed ceiling at or above the all-in quote");
    }
    if calculated_total_minor(request, response).is_none() {
        push_blocker(blockers, QuoteAffordabilityBlockerCodeV1::ArithmeticOverflow, id,
            "quote total is incomplete or exact integer arithmetic overflows");
    }
    validate_evidence(response, blockers);
}

/// Compare quote decisions and voluntarily disclosed all-in budget ceilings against the
/// canonical request ledger. Missing response rows never become zero-cost or accepted.
pub fn assess_quote_affordability_v1(
    service_access: &ServiceAccessInputV1,
    input: &QuoteAffordabilityInputV1,
) -> QuoteAffordabilityAssessmentV1 {
    let mut blockers = Vec::new();
    let canonical = assess_service_access_v1(service_access);
    if canonical.calculation_disposition != ServiceAccessCalculationDispositionV1::Calculated {
        push_blocker(&mut blockers, QuoteAffordabilityBlockerCodeV1::CanonicalServiceAccessBlocked, None,
            "canonical service-access lifecycle is blocked; quote analysis cannot proceed");
    }
    if input.schema_version != QUOTE_AFFORDABILITY_SCHEMA_VERSION_V1 {
        push_blocker(&mut blockers, QuoteAffordabilityBlockerCodeV1::UnsupportedSchema, None,
            "unsupported quote-affordability schema version");
    }
    if input.scenario_id.trim().is_empty() || input.scope_ref.trim().is_empty() || input.period_ref.trim().is_empty() {
        push_blocker(&mut blockers, QuoteAffordabilityBlockerCodeV1::EmptyIdentity, None,
            "scenario, scope and period identity must be non-empty");
    }
    if input.scenario_id != service_access.scenario_id {
        push_blocker(&mut blockers, QuoteAffordabilityBlockerCodeV1::ScenarioMismatch, None, "quote-analysis scenario must match canonical service access");
    }
    if input.scope_ref != service_access.scope_ref {
        push_blocker(&mut blockers, QuoteAffordabilityBlockerCodeV1::ScopeMismatch, None, "quote-analysis scope must match canonical service access");
    }
    if input.period_ref != service_access.period_ref {
        push_blocker(&mut blockers, QuoteAffordabilityBlockerCodeV1::PeriodMismatch, None, "quote-analysis period must match canonical service access");
    }
    if input.currency_code != service_access.currency_code || !valid_currency_code(&input.currency_code) {
        push_blocker(&mut blockers, QuoteAffordabilityBlockerCodeV1::CurrencyMismatch, None,
            "quote-analysis currency must match canonical service access and be an uppercase three-letter code");
    }

    let requests: BTreeMap<&str, &crate::service_access::ServiceAccessRequestV1> =
        service_access.requests.iter().map(|r| (r.request_id.as_str(), r)).collect();
    let mut response_ids = BTreeSet::new();
    let mut matched_responses = BTreeMap::new();
    for response in &input.responses {
        if !response_ids.insert(response.request_id.as_str()) {
            push_blocker(&mut blockers, QuoteAffordabilityBlockerCodeV1::DuplicateResponse, Some(&response.request_id),
                "a request may have exactly one quote-response snapshot");
        }
        match requests.get(response.request_id.as_str()) {
            None => push_blocker(&mut blockers, QuoteAffordabilityBlockerCodeV1::ResponseForUnknownRequest, Some(&response.request_id), "quote response refers to no canonical request"),
            Some(request) if request.quote_at_minute.is_none() => {
                push_blocker(&mut blockers, QuoteAffordabilityBlockerCodeV1::ResponseForUnquotedRequest, Some(&response.request_id), "quote response cannot refer to a request without a quote");
            }
            Some(request) => {
                matched_responses.insert(response.request_id.as_str(), (request, response));
                validate_response(request, response, &mut blockers);
            }
        }
    }
    let quote_requests: Vec<_> = service_access.requests.iter().filter(|r| r.quote_at_minute.is_some()).collect();
    for request in &quote_requests {
        if !matched_responses.contains_key(request.request_id.as_str()) {
            push_blocker(&mut blockers, QuoteAffordabilityBlockerCodeV1::MissingQuoteResponse, Some(&request.request_id),
                "every quoted request needs an explicit accepted, declined, expired or pending response");
        }
    }
    if !blockers.is_empty() {
        return QuoteAffordabilityAssessmentV1 {
            scenario_id: input.scenario_id.clone(),
            calculation_disposition: QuoteAffordabilityCalculationDispositionV1::Blocked,
            evidence_disposition: CostEvidenceDispositionV1::Blocked,
            blockers,
            breakdown: None,
        };
    }

    let accepted = input.responses.iter().filter(|r| r.decision == FarmerQuoteDecisionV1::Accepted).count();
    let declined = input.responses.iter().filter(|r| r.decision == FarmerQuoteDecisionV1::Declined).count();
    let expired = input.responses.iter().filter(|r| r.decision == FarmerQuoteDecisionV1::Expired).count();
    let pending = input.responses.iter().filter(|r| r.decision == FarmerQuoteDecisionV1::Pending).count();
    let budgets: Vec<_> = input.responses.iter().filter(|r| r.farmer_max_total_budget_minor.is_some()).collect();
    let mut within_budget = 0_u64;
    let mut above_budget = 0_u64;
    for response in &budgets {
        let Some(request) = requests.get(response.request_id.as_str()).copied() else { continue };
        let total = calculated_total_minor(request, response).unwrap_or(u64::MAX);
        if total <= response.farmer_max_total_budget_minor.unwrap_or(0) {
            within_budget = within_budget.saturating_add(1);
        } else {
            above_budget = above_budget.saturating_add(1);
        }
    }
    let source_only = canonical.evidence_disposition == CostEvidenceDispositionV1::SourceReferencedInputs
        && input.responses.iter().all(|r| r.evidence.state == CostEvidenceStateV1::SourceReferenced);
    let breakdown = QuoteAffordabilityBreakdownV1 {
        quote_count: count(quote_requests.len()),
        accepted_count: count(accepted),
        declined_count: count(declined),
        expired_count: count(expired),
        pending_count: count(pending),
        declined_price_count: count(input.responses.iter().filter(|r| r.decline_reason == Some(QuoteDeclineReasonV1::PriceAboveBudget)).count()),
        declined_timing_count: count(input.responses.iter().filter(|r| r.decline_reason == Some(QuoteDeclineReasonV1::TimingNotSuitable)).count()),
        declined_scope_count: count(input.responses.iter().filter(|r| r.decline_reason == Some(QuoteDeclineReasonV1::TaskScopeMismatch)).count()),
        declined_trust_or_safety_count: count(input.responses.iter().filter(|r| r.decline_reason == Some(QuoteDeclineReasonV1::TrustOrSafetyConcern)).count()),
        declined_alternative_provider_count: count(input.responses.iter().filter(|r| r.decline_reason == Some(QuoteDeclineReasonV1::AlternativeProvider)).count()),
        declined_other_count: count(input.responses.iter().filter(|r| r.decline_reason == Some(QuoteDeclineReasonV1::Other)).count()),
        declined_not_stated_count: count(input.responses.iter().filter(|r| r.decline_reason == Some(QuoteDeclineReasonV1::NotStated)).count()),
        budget_disclosed_count: count(budgets.len()),
        quotes_within_declared_budget_count: within_budget,
        quotes_above_declared_budget_count: above_budget,
        affordability_unknown_count: count(input.responses.len().saturating_sub(budgets.len())),
    };
    QuoteAffordabilityAssessmentV1 {
        scenario_id: input.scenario_id.clone(),
        calculation_disposition: QuoteAffordabilityCalculationDispositionV1::Calculated,
        evidence_disposition: if source_only { CostEvidenceDispositionV1::SourceReferencedInputs } else { CostEvidenceDispositionV1::ProvisionalInputs },
        blockers: Vec::new(),
        breakdown: Some(breakdown),
    }
}
