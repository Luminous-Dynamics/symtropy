// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Farmer-first service access metrics over request-to-delivery records.
//!
//! A booking or machine completion is not the same as farmer-accepted service.
//! This module validates lifecycle and evidence consistency before computing any
//! summary. It does not infer demand, prices, affordability, causality, safety,
//! fairness, or field performance from synthetic or source-referenced records.

use crate::field_robot_cost::{CostEvidenceDispositionV1, CostEvidenceStateV1, ServiceUnitV1};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const SERVICE_ACCESS_SCHEMA_VERSION_V1: u16 = 1;
pub const MINIMUM_COHORT_SIZE_V1: u32 = 10;

fn serialize_u64<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn deserialize_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .parse::<u64>()
        .map_err(serde::de::Error::custom)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TravelChargeTreatmentV1 {
    IncludedInQuote,
    SeparateQuotedFee,
    NoTravelCharge,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceNoOutcomeReasonV1 {
    NoProviderAvailable,
    ProviderDeclined,
    FarmerDeclinedQuote,
    FarmerCancelled,
    ProviderCancelled,
    ProviderNoShow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarmerDeliveryDecisionV1 {
    Accepted,
    ReworkRequested,
    RejectedForQuality,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceAccessEvidenceV1 {
    pub state: CostEvidenceStateV1,
    pub source_ref: Option<String>,
    pub artifact_sha256: Option<String>,
}

impl ServiceAccessEvidenceV1 {
    pub fn synthetic_fixture(request_id: &str) -> Self {
        Self {
            state: CostEvidenceStateV1::SyntheticFixture,
            source_ref: Some(format!("synthetic-fixture:service-access:{request_id}")),
            artifact_sha256: None,
        }
    }

    pub fn source_referenced(
        source_ref: impl Into<String>,
        artifact_sha256: impl Into<String>,
    ) -> Self {
        Self {
            state: CostEvidenceStateV1::SourceReferenced,
            source_ref: Some(source_ref.into()),
            artifact_sha256: Some(artifact_sha256.into()),
        }
    }
}

/// One request lifecycle snapshot. All times are integer minutes from the same
/// declared period origin; they are not wall-clock timestamps. The useful-window
/// deadline is farmer-declared for this task, not inferred by the system.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceAccessRequestV1 {
    pub request_id: String,
    pub scenario_id: String,
    pub scope_ref: String,
    pub period_ref: String,
    pub currency_code: String,
    pub task_code: String,
    pub service_unit: ServiceUnitV1,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub requested_units: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub requested_at_minute: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub useful_window_deadline_minute: u64,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub quote_at_minute: Option<u64>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub quoted_fee_minor: Option<u64>,
    pub price_basis_unit: Option<ServiceUnitV1>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub quote_valid_until_minute: Option<u64>,
    pub travel_charge_treatment: Option<TravelChargeTreatmentV1>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub travel_fee_minor: Option<u64>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub booking_at_minute: Option<u64>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub completion_at_minute: Option<u64>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub delivered_units: Option<u64>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub actual_travel_minutes: Option<u64>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub delivery_decision_at_minute: Option<u64>,
    pub delivery_decision: Option<FarmerDeliveryDecisionV1>,
    pub no_outcome_reason: Option<ServiceNoOutcomeReasonV1>,
    /// Optional coarse cohort label; never place names or direct identifiers here.
    pub cohort_ref: Option<String>,
    pub cohort_consent: bool,
    pub evidence: ServiceAccessEvidenceV1,
}

mod option_u64_string {
    use super::*;
    pub fn serialize<S>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match value {
            Some(value) => serializer.serialize_some(&value.to_string()),
            None => serializer.serialize_none(),
        }
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Option::<String>::deserialize(deserializer)?
            .map(|raw| raw.parse::<u64>().map_err(serde::de::Error::custom))
            .transpose()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceAccessBlockerCodeV1 {
    UnsupportedSchema,
    EmptyIdentity,
    InvalidCurrencyCode,
    EmptyRequestSet,
    DuplicateRequestId,
    ScenarioMismatch,
    ScopeMismatch,
    PeriodMismatch,
    CurrencyMismatch,
    ZeroRequestedUnits,
    UsefulWindowBeforeRequest,
    InvalidLifecycleOrder,
    IncompleteQuoteTerms,
    PriceUnitMismatch,
    QuoteExpiredBeforeBooking,
    BookingWithoutQuote,
    CompletionWithoutBooking,
    InvalidDeliveredUnits,
    DeliveryDecisionWithoutCompletion,
    DeliveryDecisionShapeMismatch,
    NoOutcomeShapeMismatch,
    InvalidTravelEvidence,
    InvalidCohortConsent,
    InvalidEvidence,
    MissingEvidence,
    StaleEvidence,
    ConflictingEvidence,
    CohortThresholdTooSmall,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceAccessBlockerV1 {
    pub code: ServiceAccessBlockerCodeV1,
    pub request_id: Option<String>,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceAccessCalculationDispositionV1 {
    Calculated,
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceAccessInputV1 {
    pub schema_version: u16,
    pub scenario_id: String,
    pub scope_ref: String,
    pub period_ref: String,
    pub currency_code: String,
    /// Must be at least 10. This suppresses small displayed cohorts but is not
    /// differential privacy or a guarantee against all re-identification.
    pub minimum_cohort_size: u32,
    pub requests: Vec<ServiceAccessRequestV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceAccessCohortSummaryV1 {
    pub cohort_ref: String,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub request_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub farmer_accepted_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub fulfilled_within_useful_window_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub no_outcome_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceAccessBreakdownV1 {
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub request_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub quote_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub booking_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub completed_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub farmer_accepted_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub completed_pending_decision_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub rework_requested_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub rejected_for_quality_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub no_outcome_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub unresolved_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub accepted_within_useful_window_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub accepted_after_useful_window_count: u64,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub completion_wait_p50_minutes: Option<u64>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub completion_wait_p90_minutes: Option<u64>,
    /// Small cohorts are omitted, rather than returned as zero-count categories.
    pub cohort_summaries: Vec<ServiceAccessCohortSummaryV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceAccessAssessmentV1 {
    pub scenario_id: String,
    pub calculation_disposition: ServiceAccessCalculationDispositionV1,
    pub evidence_disposition: CostEvidenceDispositionV1,
    pub blockers: Vec<ServiceAccessBlockerV1>,
    pub breakdown: Option<ServiceAccessBreakdownV1>,
}

fn valid_currency_code(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_uppercase())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn nonempty(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|raw| !raw.trim().is_empty())
}

fn push_blocker(
    blockers: &mut Vec<ServiceAccessBlockerV1>,
    code: ServiceAccessBlockerCodeV1,
    request_id: Option<&str>,
    detail: &str,
) {
    let blocker = ServiceAccessBlockerV1 {
        code,
        request_id: request_id.map(str::to_owned),
        detail: detail.to_owned(),
    };
    if !blockers.contains(&blocker) {
        blockers.push(blocker);
    }
}

fn validate_evidence(record: &ServiceAccessRequestV1, blockers: &mut Vec<ServiceAccessBlockerV1>) {
    if record
        .evidence
        .artifact_sha256
        .as_deref()
        .is_some_and(|digest| !valid_sha256(digest))
    {
        push_blocker(
            blockers,
            ServiceAccessBlockerCodeV1::InvalidEvidence,
            Some(&record.request_id),
            "evidence digest is not 64 hexadecimal characters",
        );
    }
    match record.evidence.state {
        CostEvidenceStateV1::SourceReferenced => {
            if !nonempty(&record.evidence.source_ref)
                || record
                    .evidence
                    .artifact_sha256
                    .as_deref()
                    .is_none_or(|digest| !valid_sha256(digest))
            {
                push_blocker(
                    blockers,
                    ServiceAccessBlockerCodeV1::InvalidEvidence,
                    Some(&record.request_id),
                    "source-referenced record requires a source reference and SHA-256-shaped digest",
                );
            }
        }
        CostEvidenceStateV1::Estimated => {
            if !nonempty(&record.evidence.source_ref) {
                push_blocker(
                    blockers,
                    ServiceAccessBlockerCodeV1::InvalidEvidence,
                    Some(&record.request_id),
                    "estimated record requires a source reference",
                );
            }
        }
        CostEvidenceStateV1::SyntheticFixture => {
            if !record
                .evidence
                .source_ref
                .as_deref()
                .is_some_and(|value| value.starts_with("synthetic-fixture:"))
            {
                push_blocker(
                    blockers,
                    ServiceAccessBlockerCodeV1::InvalidEvidence,
                    Some(&record.request_id),
                    "synthetic evidence must use an explicit synthetic-fixture reference",
                );
            }
        }
        CostEvidenceStateV1::Missing => push_blocker(
            blockers,
            ServiceAccessBlockerCodeV1::MissingEvidence,
            Some(&record.request_id),
            "request lifecycle evidence is missing",
        ),
        CostEvidenceStateV1::Stale => push_blocker(
            blockers,
            ServiceAccessBlockerCodeV1::StaleEvidence,
            Some(&record.request_id),
            "request lifecycle evidence is stale",
        ),
        CostEvidenceStateV1::Conflicting => push_blocker(
            blockers,
            ServiceAccessBlockerCodeV1::ConflictingEvidence,
            Some(&record.request_id),
            "request lifecycle evidence conflicts",
        ),
    }
}

fn validate_request(
    input: &ServiceAccessInputV1,
    record: &ServiceAccessRequestV1,
    blockers: &mut Vec<ServiceAccessBlockerV1>,
) {
    let id = Some(record.request_id.as_str());
    if record.request_id.trim().is_empty() || record.task_code.trim().is_empty() {
        push_blocker(
            blockers,
            ServiceAccessBlockerCodeV1::EmptyIdentity,
            id,
            "request ID and task code must be non-empty",
        );
    }
    if record.scenario_id != input.scenario_id {
        push_blocker(
            blockers,
            ServiceAccessBlockerCodeV1::ScenarioMismatch,
            id,
            "request scenario must match the assessment scenario",
        );
    }
    if record.scope_ref != input.scope_ref {
        push_blocker(
            blockers,
            ServiceAccessBlockerCodeV1::ScopeMismatch,
            id,
            "request scope must match the assessment scope",
        );
    }
    if record.period_ref != input.period_ref {
        push_blocker(
            blockers,
            ServiceAccessBlockerCodeV1::PeriodMismatch,
            id,
            "request period must match the assessment period",
        );
    }
    if record.currency_code != input.currency_code {
        push_blocker(
            blockers,
            ServiceAccessBlockerCodeV1::CurrencyMismatch,
            id,
            "request currency must match the assessment currency",
        );
    }
    if record.requested_units == 0 {
        push_blocker(
            blockers,
            ServiceAccessBlockerCodeV1::ZeroRequestedUnits,
            id,
            "requested service units must be positive",
        );
    }
    if record.useful_window_deadline_minute < record.requested_at_minute {
        push_blocker(
            blockers,
            ServiceAccessBlockerCodeV1::UsefulWindowBeforeRequest,
            id,
            "farmer-declared useful-window deadline precedes the request",
        );
    }

    let quote_exists = record.quote_at_minute.is_some();
    let price_fields_complete = record.quoted_fee_minor.is_some()
        && record.price_basis_unit.is_some()
        && record.quote_valid_until_minute.is_some()
        && record
            .travel_charge_treatment
            .is_some_and(|treatment| treatment != TravelChargeTreatmentV1::Unknown);
    let any_price_field = record.quoted_fee_minor.is_some()
        || record.price_basis_unit.is_some()
        || record.quote_valid_until_minute.is_some()
        || record.travel_charge_treatment.is_some()
        || record.travel_fee_minor.is_some();
    if quote_exists != price_fields_complete || (!quote_exists && any_price_field) {
        push_blocker(
            blockers,
            ServiceAccessBlockerCodeV1::IncompleteQuoteTerms,
            id,
            "a quote requires fee, unit basis, validity and known travel treatment; unquoted requests cannot carry quote terms",
        );
    }
    if record
        .price_basis_unit
        .is_some_and(|unit| unit != record.service_unit)
    {
        push_blocker(
            blockers,
            ServiceAccessBlockerCodeV1::PriceUnitMismatch,
            id,
            "quoted price basis must equal the requested service unit",
        );
    }
    if let (Some(quote_at), Some(valid_until)) =
        (record.quote_at_minute, record.quote_valid_until_minute)
    {
        if quote_at < record.requested_at_minute || valid_until < quote_at {
            push_blocker(
                blockers,
                ServiceAccessBlockerCodeV1::InvalidLifecycleOrder,
                id,
                "quote and validity times must be ordered after the request",
            );
        }
    }
    match record.travel_charge_treatment {
        Some(TravelChargeTreatmentV1::SeparateQuotedFee) if record.travel_fee_minor.is_none() => {
            push_blocker(
                blockers,
                ServiceAccessBlockerCodeV1::IncompleteQuoteTerms,
                id,
                "separate travel charging requires an explicit travel fee",
            )
        }
        Some(
            TravelChargeTreatmentV1::IncludedInQuote | TravelChargeTreatmentV1::NoTravelCharge,
        ) if record.travel_fee_minor.is_some() => push_blocker(
            blockers,
            ServiceAccessBlockerCodeV1::IncompleteQuoteTerms,
            id,
            "travel fee must be absent when included in the quote or not charged",
        ),
        _ => {}
    }

    if let Some(booking_at) = record.booking_at_minute {
        match (record.quote_at_minute, record.quote_valid_until_minute) {
            (Some(quote_at), Some(valid_until))
                if booking_at >= quote_at && booking_at <= valid_until => {}
            (Some(_), Some(_)) => push_blocker(
                blockers,
                ServiceAccessBlockerCodeV1::QuoteExpiredBeforeBooking,
                id,
                "booking must be between quote time and quote expiry",
            ),
            _ => push_blocker(
                blockers,
                ServiceAccessBlockerCodeV1::BookingWithoutQuote,
                id,
                "booking requires a complete, valid quote",
            ),
        }
    }
    if let Some(completed_at) = record.completion_at_minute {
        match record.booking_at_minute {
            Some(booked_at) if completed_at >= booked_at => {}
            Some(_) => push_blocker(
                blockers,
                ServiceAccessBlockerCodeV1::InvalidLifecycleOrder,
                id,
                "completion must not precede booking",
            ),
            None => push_blocker(
                blockers,
                ServiceAccessBlockerCodeV1::CompletionWithoutBooking,
                id,
                "completion requires a recorded booking",
            ),
        }
        if record.delivered_units.is_none_or(|units| units == 0) {
            push_blocker(
                blockers,
                ServiceAccessBlockerCodeV1::InvalidDeliveredUnits,
                id,
                "completion requires a positive delivered-unit quantity",
            );
        }
    } else if record.delivered_units.is_some() {
        push_blocker(
            blockers,
            ServiceAccessBlockerCodeV1::InvalidDeliveredUnits,
            id,
            "delivered units cannot be claimed without completion",
        );
    }
    if let Some(travel_minutes) = record.actual_travel_minutes {
        if record.booking_at_minute.is_none() || travel_minutes == 0 {
            push_blocker(
                blockers,
                ServiceAccessBlockerCodeV1::InvalidTravelEvidence,
                id,
                "actual travel minutes must be positive and linked to a booking",
            );
        }
    }

    if record.delivery_decision_at_minute.is_some() != record.delivery_decision.is_some() {
        push_blocker(
            blockers,
            ServiceAccessBlockerCodeV1::DeliveryDecisionShapeMismatch,
            id,
            "delivery decision time and decision must be present together",
        );
    }
    if let Some(decision_at) = record.delivery_decision_at_minute {
        match record.completion_at_minute {
            Some(completed_at) if decision_at >= completed_at => {}
            Some(_) => push_blocker(
                blockers,
                ServiceAccessBlockerCodeV1::InvalidLifecycleOrder,
                id,
                "farmer decision must not precede completion",
            ),
            None => push_blocker(
                blockers,
                ServiceAccessBlockerCodeV1::DeliveryDecisionWithoutCompletion,
                id,
                "farmer decision requires a completed service",
            ),
        }
    }

    if let Some(reason) = record.no_outcome_reason {
        let valid_shape = match reason {
            ServiceNoOutcomeReasonV1::NoProviderAvailable
            | ServiceNoOutcomeReasonV1::ProviderDeclined => {
                record.quote_at_minute.is_none()
                    && record.booking_at_minute.is_none()
                    && record.completion_at_minute.is_none()
            }
            ServiceNoOutcomeReasonV1::FarmerDeclinedQuote => {
                record.quote_at_minute.is_some()
                    && record.booking_at_minute.is_none()
                    && record.completion_at_minute.is_none()
            }
            ServiceNoOutcomeReasonV1::FarmerCancelled
            | ServiceNoOutcomeReasonV1::ProviderCancelled
            | ServiceNoOutcomeReasonV1::ProviderNoShow => {
                record.booking_at_minute.is_some() && record.completion_at_minute.is_none()
            }
        };
        if !valid_shape
            || record.delivery_decision.is_some()
            || record.delivery_decision_at_minute.is_some()
        {
            push_blocker(
                blockers,
                ServiceAccessBlockerCodeV1::NoOutcomeShapeMismatch,
                id,
                "no-outcome reason conflicts with quote, booking, completion or delivery decision history",
            );
        }
    }

    if record.cohort_consent {
        if !nonempty(&record.cohort_ref) {
            push_blocker(
                blockers,
                ServiceAccessBlockerCodeV1::InvalidCohortConsent,
                id,
                "cohort aggregation consent requires a non-empty coarse cohort label",
            );
        }
    } else if record.cohort_ref.is_some() {
        push_blocker(
            blockers,
            ServiceAccessBlockerCodeV1::InvalidCohortConsent,
            id,
            "cohort label must not be retained when aggregation consent is absent",
        );
    }
    validate_evidence(record, blockers);
}

fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn nearest_rank(values: &mut [u64], percentile: u8) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let len = u128::try_from(values.len()).ok()?;
    let rank = (u128::from(percentile) * len + 99) / 100;
    let index = usize::try_from(rank.max(1) - 1).ok()?;
    values.get(index).copied()
}

fn cohort_summaries(input: &ServiceAccessInputV1) -> Vec<ServiceAccessCohortSummaryV1> {
    let mut groups: BTreeMap<&str, Vec<&ServiceAccessRequestV1>> = BTreeMap::new();
    for record in &input.requests {
        if record.cohort_consent {
            if let Some(cohort) = record.cohort_ref.as_deref() {
                groups.entry(cohort).or_default().push(record);
            }
        }
    }
    groups
        .into_iter()
        .filter_map(|(cohort_ref, records)| {
            if records.len() < usize::try_from(input.minimum_cohort_size).unwrap_or(usize::MAX) {
                return None;
            }
            let accepted = records
                .iter()
                .filter(|record| {
                    record.delivery_decision == Some(FarmerDeliveryDecisionV1::Accepted)
                })
                .count();
            let within_window = records
                .iter()
                .filter(|record| {
                    record.delivery_decision == Some(FarmerDeliveryDecisionV1::Accepted)
                        && record
                            .completion_at_minute
                            .is_some_and(|done| done <= record.useful_window_deadline_minute)
                })
                .count();
            let no_outcome = records
                .iter()
                .filter(|record| record.no_outcome_reason.is_some())
                .count();
            Some(ServiceAccessCohortSummaryV1 {
                cohort_ref: cohort_ref.to_owned(),
                request_count: count(records.len()),
                farmer_accepted_count: count(accepted),
                fulfilled_within_useful_window_count: count(within_window),
                no_outcome_count: count(no_outcome),
            })
        })
        .collect()
}

/// Any blocker suppresses the aggregate instead of publishing a partial denominator.
pub fn assess_service_access_v1(input: &ServiceAccessInputV1) -> ServiceAccessAssessmentV1 {
    let mut blockers = Vec::new();
    if input.schema_version != SERVICE_ACCESS_SCHEMA_VERSION_V1 {
        push_blocker(
            &mut blockers,
            ServiceAccessBlockerCodeV1::UnsupportedSchema,
            None,
            "unsupported service-access schema version",
        );
    }
    if input.scenario_id.trim().is_empty()
        || input.scope_ref.trim().is_empty()
        || input.period_ref.trim().is_empty()
    {
        push_blocker(
            &mut blockers,
            ServiceAccessBlockerCodeV1::EmptyIdentity,
            None,
            "scenario, scope and period identity must be non-empty",
        );
    }
    if !valid_currency_code(&input.currency_code) {
        push_blocker(
            &mut blockers,
            ServiceAccessBlockerCodeV1::InvalidCurrencyCode,
            None,
            "currency code must be an uppercase three-letter code",
        );
    }
    if input.requests.is_empty() {
        push_blocker(
            &mut blockers,
            ServiceAccessBlockerCodeV1::EmptyRequestSet,
            None,
            "an empty request set cannot produce an access assessment",
        );
    }
    if input.minimum_cohort_size < MINIMUM_COHORT_SIZE_V1 {
        push_blocker(
            &mut blockers,
            ServiceAccessBlockerCodeV1::CohortThresholdTooSmall,
            None,
            "minimum displayed cohort size must be at least 10",
        );
    }
    let mut ids = BTreeSet::new();
    for record in &input.requests {
        if !ids.insert(record.request_id.as_str()) {
            push_blocker(
                &mut blockers,
                ServiceAccessBlockerCodeV1::DuplicateRequestId,
                Some(&record.request_id),
                "request IDs must be unique within this assessment",
            );
        }
        validate_request(input, record, &mut blockers);
    }
    if !blockers.is_empty() {
        return ServiceAccessAssessmentV1 {
            scenario_id: input.scenario_id.clone(),
            calculation_disposition: ServiceAccessCalculationDispositionV1::Blocked,
            evidence_disposition: CostEvidenceDispositionV1::Blocked,
            blockers,
            breakdown: None,
        };
    }

    let mut waits: Vec<u64> = input
        .requests
        .iter()
        .filter_map(|record| {
            record
                .completion_at_minute
                .map(|done| done - record.requested_at_minute)
        })
        .collect();
    let mut waits_p50 = waits.clone();
    let p50 = nearest_rank(&mut waits_p50, 50);
    let p90 = nearest_rank(&mut waits, 90);
    let within = input
        .requests
        .iter()
        .filter(|record| {
            record.delivery_decision == Some(FarmerDeliveryDecisionV1::Accepted)
                && record
                    .completion_at_minute
                    .is_some_and(|done| done <= record.useful_window_deadline_minute)
        })
        .count();
    let late = input
        .requests
        .iter()
        .filter(|record| {
            record.delivery_decision == Some(FarmerDeliveryDecisionV1::Accepted)
                && record
                    .completion_at_minute
                    .is_some_and(|done| done > record.useful_window_deadline_minute)
        })
        .count();
    let all_source_referenced = input
        .requests
        .iter()
        .all(|record| record.evidence.state == CostEvidenceStateV1::SourceReferenced);
    let breakdown = ServiceAccessBreakdownV1 {
        request_count: count(input.requests.len()),
        quote_count: count(
            input
                .requests
                .iter()
                .filter(|record| record.quote_at_minute.is_some())
                .count(),
        ),
        booking_count: count(
            input
                .requests
                .iter()
                .filter(|record| record.booking_at_minute.is_some())
                .count(),
        ),
        completed_count: count(
            input
                .requests
                .iter()
                .filter(|record| record.completion_at_minute.is_some())
                .count(),
        ),
        farmer_accepted_count: count(
            input
                .requests
                .iter()
                .filter(|record| {
                    record.delivery_decision == Some(FarmerDeliveryDecisionV1::Accepted)
                })
                .count(),
        ),
        completed_pending_decision_count: count(
            input
                .requests
                .iter()
                .filter(|record| {
                    record.completion_at_minute.is_some() && record.delivery_decision.is_none()
                })
                .count(),
        ),
        rework_requested_count: count(
            input
                .requests
                .iter()
                .filter(|record| {
                    record.delivery_decision == Some(FarmerDeliveryDecisionV1::ReworkRequested)
                })
                .count(),
        ),
        rejected_for_quality_count: count(
            input
                .requests
                .iter()
                .filter(|record| {
                    record.delivery_decision == Some(FarmerDeliveryDecisionV1::RejectedForQuality)
                })
                .count(),
        ),
        no_outcome_count: count(
            input
                .requests
                .iter()
                .filter(|record| record.no_outcome_reason.is_some())
                .count(),
        ),
        unresolved_count: count(
            input
                .requests
                .iter()
                .filter(|record| {
                    record.completion_at_minute.is_none() && record.no_outcome_reason.is_none()
                })
                .count(),
        ),
        accepted_within_useful_window_count: count(within),
        accepted_after_useful_window_count: count(late),
        completion_wait_p50_minutes: p50,
        completion_wait_p90_minutes: p90,
        cohort_summaries: cohort_summaries(input),
    };
    ServiceAccessAssessmentV1 {
        scenario_id: input.scenario_id.clone(),
        calculation_disposition: ServiceAccessCalculationDispositionV1::Calculated,
        evidence_disposition: if all_source_referenced {
            CostEvidenceDispositionV1::SourceReferencedInputs
        } else {
            CostEvidenceDispositionV1::ProvisionalInputs
        },
        blockers: Vec::new(),
        breakdown: Some(breakdown),
    }
}
