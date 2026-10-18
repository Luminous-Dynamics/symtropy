// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evidence-aware provider revenue coverage over the existing annual cost kernel.
//!
//! This module does not compute costs, allocate shared costs across services,
//! recommend prices, or establish business viability. The embedded cost input
//! must represent the same complete provider/asset annual cost boundary as the
//! revenue streams supplied here.

use crate::field_robot_cost::{
    CostCalculationDispositionV1, CostEvidenceDispositionV1, CostEvidenceStateV1,
    FieldRobotCostInputV1, assess_field_robot_cost_v1,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub const PROVIDER_COVERAGE_SCHEMA_VERSION: u16 = 1;

mod decimal_u64 {
    use super::*;

    pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        raw.parse::<u64>().map_err(serde::de::Error::custom)
    }
}

mod decimal_i128 {
    use super::*;

    pub fn serialize<S>(value: &i128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<i128, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        raw.parse::<i128>().map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderRevenueStreamV1 {
    PrimaryServiceReceipts,
    OtherServiceReceipts,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderRevenueEvidenceV1 {
    pub state: CostEvidenceStateV1,
    pub source_ref: Option<String>,
    pub artifact_sha256: Option<String>,
}

impl ProviderRevenueEvidenceV1 {
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

    pub fn estimated(source_ref: impl Into<String>) -> Self {
        Self {
            state: CostEvidenceStateV1::Estimated,
            source_ref: Some(source_ref.into()),
            artifact_sha256: None,
        }
    }

    pub fn synthetic_fixture(stream: ProviderRevenueStreamV1) -> Self {
        let name = match stream {
            ProviderRevenueStreamV1::PrimaryServiceReceipts => "primary",
            ProviderRevenueStreamV1::OtherServiceReceipts => "other",
        };
        Self {
            state: CostEvidenceStateV1::SyntheticFixture,
            source_ref: Some(format!("synthetic-fixture:provider-coverage:{name}")),
            artifact_sha256: None,
        }
    }
}

/// Provider-wide annual cost coverage, not a per-service task cost model.
///
/// Paid units count completed primary-service units for which receipts were
/// actually received in this scope and period. Other-service receipts must
/// belong to the same annual cost boundary; they must not be added to a
/// primary-task-only cost model as uncosted revenue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceProviderCoverageInputV1 {
    pub schema_version: u16,
    pub scenario_id: String,
    pub scope_ref: String,
    pub period_ref: String,
    pub currency_code: String,
    #[serde(with = "decimal_u64")]
    pub primary_service_units_paid: u64,
    #[serde(with = "decimal_u64")]
    pub fee_minor_per_paid_unit: u64,
    #[serde(with = "decimal_u64")]
    pub other_service_receipts_minor: u64,
    pub primary_revenue_evidence: ProviderRevenueEvidenceV1,
    pub other_service_revenue_evidence: ProviderRevenueEvidenceV1,
    /// Reuses the existing cost kernel; no parallel cost formula is implemented.
    pub annual_cost_input: FieldRobotCostInputV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCoverageBlockerCodeV1 {
    UnsupportedSchema,
    EmptyIdentity,
    InvalidCurrencyCode,
    ScenarioMismatch,
    ScopeMismatch,
    PeriodMismatch,
    CurrencyMismatch,
    PaidUnitsExceedCompletedOutput,
    CostInputBlocked,
    InvalidRevenueEvidence,
    RevenueEvidenceMissing,
    RevenueEvidenceStale,
    RevenueEvidenceConflicting,
    ArithmeticOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCoverageBlockerV1 {
    pub code: ProviderCoverageBlockerCodeV1,
    pub stream: Option<ProviderRevenueStreamV1>,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCoverageCalculationDispositionV1 {
    Calculated,
    Blocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCostCoverageV1 {
    RevenueCoversModeledAnnualCost,
    RevenueBelowModeledAnnualCost,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCoverageBreakdownV1 {
    pub currency_code: String,
    pub coverage: ProviderCostCoverageV1,
    #[serde(with = "decimal_u64")]
    pub completed_service_units: u64,
    #[serde(with = "decimal_u64")]
    pub paid_service_units: u64,
    #[serde(with = "decimal_u64")]
    pub unpaid_completed_service_units: u64,
    #[serde(with = "decimal_u64")]
    pub primary_service_receipts_minor: u64,
    #[serde(with = "decimal_u64")]
    pub other_service_receipts_minor: u64,
    #[serde(with = "decimal_u64")]
    pub total_receipts_minor: u64,
    #[serde(with = "decimal_u64")]
    pub modeled_annual_cost_minor: u64,
    /// Receipts minus the declared annual cost envelope; negative values are valid.
    #[serde(with = "decimal_i128")]
    pub modeled_surplus_minor: i128,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceProviderCoverageAssessmentV1 {
    pub scenario_id: String,
    pub calculation_disposition: ProviderCoverageCalculationDispositionV1,
    pub evidence_disposition: CostEvidenceDispositionV1,
    pub blockers: Vec<ProviderCoverageBlockerV1>,
    pub breakdown: Option<ProviderCoverageBreakdownV1>,
}

fn nonempty(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|raw| !raw.trim().is_empty())
}

fn valid_currency_code(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_uppercase())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn push_blocker(
    blockers: &mut Vec<ProviderCoverageBlockerV1>,
    code: ProviderCoverageBlockerCodeV1,
    stream: Option<ProviderRevenueStreamV1>,
    detail: impl Into<String>,
) {
    let blocker = ProviderCoverageBlockerV1 {
        code,
        stream,
        detail: detail.into(),
    };
    if !blockers.contains(&blocker) {
        blockers.push(blocker);
    }
}

fn validate_revenue_evidence(
    stream: ProviderRevenueStreamV1,
    evidence: &ProviderRevenueEvidenceV1,
    blockers: &mut Vec<ProviderCoverageBlockerV1>,
) -> bool {
    if evidence
        .artifact_sha256
        .as_deref()
        .is_some_and(|digest| !valid_sha256(digest))
    {
        push_blocker(
            blockers,
            ProviderCoverageBlockerCodeV1::InvalidRevenueEvidence,
            Some(stream),
            "revenue evidence digest is not 64 hexadecimal characters",
        );
        return false;
    }

    match evidence.state {
        CostEvidenceStateV1::SourceReferenced => {
            if !nonempty(&evidence.source_ref)
                || evidence
                    .artifact_sha256
                    .as_deref()
                    .is_none_or(|digest| !valid_sha256(digest))
            {
                push_blocker(
                    blockers,
                    ProviderCoverageBlockerCodeV1::InvalidRevenueEvidence,
                    Some(stream),
                    "source-referenced receipts require a source reference and SHA-256-shaped digest",
                );
                false
            } else {
                true
            }
        }
        CostEvidenceStateV1::Estimated => {
            if !nonempty(&evidence.source_ref) {
                push_blocker(
                    blockers,
                    ProviderCoverageBlockerCodeV1::InvalidRevenueEvidence,
                    Some(stream),
                    "estimated receipts require a non-empty source reference",
                );
                false
            } else {
                true
            }
        }
        CostEvidenceStateV1::SyntheticFixture => {
            if !evidence
                .source_ref
                .as_deref()
                .is_some_and(|value| value.starts_with("synthetic-fixture:"))
            {
                push_blocker(
                    blockers,
                    ProviderCoverageBlockerCodeV1::InvalidRevenueEvidence,
                    Some(stream),
                    "synthetic revenue evidence must use an explicit synthetic-fixture reference",
                );
                false
            } else {
                true
            }
        }
        CostEvidenceStateV1::Missing => {
            push_blocker(
                blockers,
                ProviderCoverageBlockerCodeV1::RevenueEvidenceMissing,
                Some(stream),
                "required revenue evidence is missing",
            );
            false
        }
        CostEvidenceStateV1::Stale => {
            push_blocker(
                blockers,
                ProviderCoverageBlockerCodeV1::RevenueEvidenceStale,
                Some(stream),
                "stale revenue evidence cannot support current-period coverage",
            );
            false
        }
        CostEvidenceStateV1::Conflicting => {
            push_blocker(
                blockers,
                ProviderCoverageBlockerCodeV1::RevenueEvidenceConflicting,
                Some(stream),
                "conflicting revenue evidence blocks coverage",
            );
            false
        }
    }
}

fn blocked_assessment(
    scenario_id: &str,
    mut blockers: Vec<ProviderCoverageBlockerV1>,
) -> ServiceProviderCoverageAssessmentV1 {
    blockers.sort_by(|left, right| {
        (left.code, left.stream, left.detail.as_str()).cmp(&(
            right.code,
            right.stream,
            right.detail.as_str(),
        ))
    });
    ServiceProviderCoverageAssessmentV1 {
        scenario_id: scenario_id.to_owned(),
        calculation_disposition: ProviderCoverageCalculationDispositionV1::Blocked,
        evidence_disposition: CostEvidenceDispositionV1::Blocked,
        blockers,
        breakdown: None,
    }
}

/// Compare evidence-bounded provider receipts against the existing cost kernel.
///
/// This only answers whether receipts cover the supplied modeled annual cost
/// boundary. It does not establish profitability, affordability, liquidity,
/// ROI, price fairness, cost allocation among services, or business viability.
pub fn assess_service_provider_coverage_v1(
    input: &ServiceProviderCoverageInputV1,
) -> ServiceProviderCoverageAssessmentV1 {
    let cost_assessment = assess_field_robot_cost_v1(&input.annual_cost_input);
    let mut blockers = Vec::new();

    if input.schema_version != PROVIDER_COVERAGE_SCHEMA_VERSION {
        push_blocker(
            &mut blockers,
            ProviderCoverageBlockerCodeV1::UnsupportedSchema,
            None,
            "unsupported provider coverage schema version",
        );
    }
    if input.scenario_id.trim().is_empty()
        || input.scope_ref.trim().is_empty()
        || input.period_ref.trim().is_empty()
    {
        push_blocker(
            &mut blockers,
            ProviderCoverageBlockerCodeV1::EmptyIdentity,
            None,
            "scenario, scope and period references are required",
        );
    }
    if !valid_currency_code(&input.currency_code) {
        push_blocker(
            &mut blockers,
            ProviderCoverageBlockerCodeV1::InvalidCurrencyCode,
            None,
            "currency code must be three uppercase ASCII letters",
        );
    }
    if input.scenario_id != input.annual_cost_input.scenario_id {
        push_blocker(
            &mut blockers,
            ProviderCoverageBlockerCodeV1::ScenarioMismatch,
            None,
            "provider coverage and cost scenario IDs must match exactly",
        );
    }
    if input.scope_ref != input.annual_cost_input.scope_ref {
        push_blocker(
            &mut blockers,
            ProviderCoverageBlockerCodeV1::ScopeMismatch,
            None,
            "provider coverage and cost scope references must match exactly",
        );
    }
    if input.period_ref != input.annual_cost_input.period_ref {
        push_blocker(
            &mut blockers,
            ProviderCoverageBlockerCodeV1::PeriodMismatch,
            None,
            "provider coverage and cost period references must match exactly",
        );
    }
    if input.currency_code != input.annual_cost_input.currency_code {
        push_blocker(
            &mut blockers,
            ProviderCoverageBlockerCodeV1::CurrencyMismatch,
            None,
            "provider receipts and cost input must use the same declared currency",
        );
    }
    if input.primary_service_units_paid > input.annual_cost_input.annual_completed_service_units {
        push_blocker(
            &mut blockers,
            ProviderCoverageBlockerCodeV1::PaidUnitsExceedCompletedOutput,
            Some(ProviderRevenueStreamV1::PrimaryServiceReceipts),
            "paid units cannot exceed completed service output in the same scope and period",
        );
    }

    validate_revenue_evidence(
        ProviderRevenueStreamV1::PrimaryServiceReceipts,
        &input.primary_revenue_evidence,
        &mut blockers,
    );
    validate_revenue_evidence(
        ProviderRevenueStreamV1::OtherServiceReceipts,
        &input.other_service_revenue_evidence,
        &mut blockers,
    );

    if cost_assessment.calculation_disposition == CostCalculationDispositionV1::Blocked
        || cost_assessment.breakdown.is_none()
    {
        push_blocker(
            &mut blockers,
            ProviderCoverageBlockerCodeV1::CostInputBlocked,
            None,
            "the canonical annual cost kernel did not return a calculated cost breakdown",
        );
    }

    if !blockers.is_empty() {
        return blocked_assessment(&input.scenario_id, blockers);
    }

    let cost = cost_assessment
        .breakdown
        .as_ref()
        .expect("cost breakdown presence checked");
    let Some(primary_receipts) = input
        .primary_service_units_paid
        .checked_mul(input.fee_minor_per_paid_unit)
    else {
        push_blocker(
            &mut blockers,
            ProviderCoverageBlockerCodeV1::ArithmeticOverflow,
            Some(ProviderRevenueStreamV1::PrimaryServiceReceipts),
            "paid service units multiplied by unit fee overflowed",
        );
        return blocked_assessment(&input.scenario_id, blockers);
    };
    let Some(total_receipts) = primary_receipts.checked_add(input.other_service_receipts_minor)
    else {
        push_blocker(
            &mut blockers,
            ProviderCoverageBlockerCodeV1::ArithmeticOverflow,
            None,
            "total provider receipts overflowed",
        );
        return blocked_assessment(&input.scenario_id, blockers);
    };

    let modeled_surplus = i128::from(total_receipts) - i128::from(cost.annual_total_minor);
    let coverage = if modeled_surplus >= 0 {
        ProviderCostCoverageV1::RevenueCoversModeledAnnualCost
    } else {
        ProviderCostCoverageV1::RevenueBelowModeledAnnualCost
    };
    let completed_units = input.annual_cost_input.annual_completed_service_units;
    let revenue_is_source_referenced = input.primary_revenue_evidence.state
        == CostEvidenceStateV1::SourceReferenced
        && input.other_service_revenue_evidence.state == CostEvidenceStateV1::SourceReferenced;
    let evidence_disposition = if cost_assessment.evidence_disposition
        == CostEvidenceDispositionV1::SourceReferencedInputs
        && revenue_is_source_referenced
    {
        CostEvidenceDispositionV1::SourceReferencedInputs
    } else {
        CostEvidenceDispositionV1::ProvisionalInputs
    };

    ServiceProviderCoverageAssessmentV1 {
        scenario_id: input.scenario_id.clone(),
        calculation_disposition: ProviderCoverageCalculationDispositionV1::Calculated,
        evidence_disposition,
        blockers: Vec::new(),
        breakdown: Some(ProviderCoverageBreakdownV1 {
            currency_code: input.currency_code.clone(),
            coverage,
            completed_service_units: completed_units,
            paid_service_units: input.primary_service_units_paid,
            unpaid_completed_service_units: completed_units - input.primary_service_units_paid,
            primary_service_receipts_minor: primary_receipts,
            other_service_receipts_minor: input.other_service_receipts_minor,
            total_receipts_minor: total_receipts,
            modeled_annual_cost_minor: cost.annual_total_minor,
            modeled_surplus_minor: modeled_surplus,
        }),
    }
}
