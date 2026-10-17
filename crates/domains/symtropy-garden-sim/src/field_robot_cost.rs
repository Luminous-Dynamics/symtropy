// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact, evidence-aware cost model for a declared farm service.
//!
//! This module computes cost components from declared annual input values. It
//! does not forecast yield, prove task success, authenticate evidence, select a
//! universal winner, calculate ROI, or authorize procurement/physical action.
//! Cross-candidate trade-offs remain with the canonical portfolio comparator.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};

pub const FIELD_ROBOT_COST_SCHEMA_VERSION: u16 = 1;
pub const MAX_ANNUAL_MINUTES: u64 = 8_784 * 60;
const MINUTES_PER_HOUR: u64 = 60;
const MINUTES_PER_100_HOURS: u64 = 6_000;
const METRES_PER_KILOMETRE: u64 = 1_000;
const SQUARE_METRES_PER_HECTARE: u64 = 10_000;

mod exact_u64 {
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
        String::deserialize(deserializer)?
            .parse::<u64>()
            .map_err(serde::de::Error::custom)
    }
}

mod exact_option_u64 {
    use super::*;

    pub fn serialize<S>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => serializer.serialize_some(&value.to_string()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Option::<String>::deserialize(deserializer)?;
        value
            .map(|raw| raw.parse::<u64>().map_err(serde::de::Error::custom))
            .transpose()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceUnitV1 {
    SquareMetresCompleted,
    KilogramsDelivered,
    PlantsHandled,
    AnimalsMonitored,
    MissionsCompleted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostEvidenceDomainV1 {
    SiteGeometry,
    ServiceOutput,
    CapitalCost,
    MachineOperatingCost,
    TravelLogistics,
    LabourCost,
    MaintenanceRecovery,
}

impl CostEvidenceDomainV1 {
    pub const ALL: [Self; 7] = [
        Self::SiteGeometry,
        Self::ServiceOutput,
        Self::CapitalCost,
        Self::MachineOperatingCost,
        Self::TravelLogistics,
        Self::LabourCost,
        Self::MaintenanceRecovery,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostEvidenceStateV1 {
    /// A source reference and artifact digest are declared. This evaluator does
    /// not independently authenticate either one.
    SourceReferenced,
    /// A measured/quoted/model value is estimated and requires sensitivity.
    Estimated,
    /// Test-only deterministic fixture; never promoted to source-qualified.
    SyntheticFixture,
    Missing,
    Stale,
    Conflicting,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostEvidenceRecordV1 {
    pub domain: CostEvidenceDomainV1,
    pub state: CostEvidenceStateV1,
    pub source_ref: Option<String>,
    pub artifact_sha256: Option<String>,
}

impl CostEvidenceRecordV1 {
    pub fn source_referenced(
        domain: CostEvidenceDomainV1,
        source_ref: impl Into<String>,
        artifact_sha256: impl Into<String>,
    ) -> Self {
        Self {
            domain,
            state: CostEvidenceStateV1::SourceReferenced,
            source_ref: Some(source_ref.into()),
            artifact_sha256: Some(artifact_sha256.into()),
        }
    }

    pub fn estimated(
        domain: CostEvidenceDomainV1,
        source_ref: impl Into<String>,
    ) -> Self {
        Self {
            domain,
            state: CostEvidenceStateV1::Estimated,
            source_ref: Some(source_ref.into()),
            artifact_sha256: None,
        }
    }

    pub fn synthetic_fixture(domain: CostEvidenceDomainV1) -> Self {
        Self {
            domain,
            state: CostEvidenceStateV1::SyntheticFixture,
            source_ref: Some("synthetic-fixture:field-robot-cost-v1".into()),
            artifact_sha256: None,
        }
    }
}

/// All money values are non-negative integer minor currency units (for example,
/// South African cents under currency code ZAR). No implicit FX conversion exists.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldRobotCostInputV1 {
    pub schema_version: u16,
    pub scenario_id: String,
    pub scope_ref: String,
    pub period_ref: String,
    pub currency_code: String,
    pub service_unit: ServiceUnitV1,
    #[serde(with = "exact_u64")]
    pub annual_completed_service_units: u64,
    /// Optional denominator for a cost-per-hectare report; unit is square metres.
    #[serde(with = "exact_option_u64")]
    pub annual_area_completed_m2: Option<u64>,
    /// Geometry descriptors are context metadata; cost is driven by the measured
    /// route/time fields below, not by a generic field-count multiplier.
    pub field_count: u32,
    pub access_point_count: u32,
    #[serde(with = "exact_u64")]
    pub annual_productive_machine_minutes: u64,
    #[serde(with = "exact_u64")]
    pub annual_nonproductive_machine_minutes: u64,
    #[serde(with = "exact_u64")]
    pub annual_operator_labor_minutes: u64,
    /// This is a subset of annual_operator_labor_minutes, not an extra cost term.
    #[serde(with = "exact_u64")]
    pub annual_supervision_minutes: u64,
    #[serde(with = "exact_u64")]
    pub route_distance_metres_per_mission: u64,
    #[serde(with = "exact_u64")]
    pub missions_per_year: u64,
    #[serde(with = "exact_u64")]
    pub acquisition_cost_minor: u64,
    #[serde(with = "exact_u64")]
    pub salvage_value_minor: u64,
    pub useful_life_years: u32,
    #[serde(with = "exact_u64")]
    pub machine_cost_minor_per_hour: u64,
    #[serde(with = "exact_u64")]
    pub route_logistics_cost_minor_per_kilometre: u64,
    #[serde(with = "exact_u64")]
    pub labor_cost_minor_per_hour: u64,
    #[serde(with = "exact_u64")]
    pub maintenance_cost_minor_per_100_machine_hours: u64,
    #[serde(with = "exact_u64")]
    pub mobilization_cost_minor_per_mission: u64,
    #[serde(with = "exact_u64")]
    pub recovery_events_per_year: u64,
    #[serde(with = "exact_u64")]
    pub recovery_cost_minor_per_event: u64,
    pub evidence: Vec<CostEvidenceRecordV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostBlockerCodeV1 {
    UnsupportedSchema,
    EmptyScenarioId,
    EmptyScope,
    EmptyPeriod,
    InvalidCurrencyCode,
    ZeroCompletedOutput,
    ZeroCompletedArea,
    ZeroUsefulLife,
    SalvageExceedsAcquisition,
    AnnualTimeBudgetExceeded,
    SupervisionExceedsLabor,
    MalformedEvidenceRecord,
    DuplicateEvidenceDomain,
    MissingEvidenceRecord,
    EvidenceMissing,
    EvidenceStale,
    EvidenceConflicting,
    ArithmeticOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostBlockerV1 {
    pub code: CostBlockerCodeV1,
    pub domain: Option<CostEvidenceDomainV1>,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostCalculationDispositionV1 {
    Calculated,
    Blocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostEvidenceDispositionV1 {
    SourceReferencedInputs,
    ProvisionalInputs,
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnualCostBreakdownV1 {
    pub currency_code: String,
    pub service_unit: ServiceUnitV1,
    #[serde(with = "exact_u64")]
    pub annual_completed_service_units: u64,
    #[serde(with = "exact_option_u64")]
    pub annual_area_completed_m2: Option<u64>,
    pub field_count: u32,
    pub access_point_count: u32,
    #[serde(with = "exact_u64")]
    pub missions_per_year: u64,
    #[serde(with = "exact_u64")]
    pub annualized_capital_minor: u64,
    #[serde(with = "exact_u64")]
    pub productive_machine_time_minor: u64,
    #[serde(with = "exact_u64")]
    pub nonproductive_machine_time_minor: u64,
    #[serde(with = "exact_u64")]
    pub maintenance_minor: u64,
    #[serde(with = "exact_u64")]
    pub operator_labor_minor: u64,
    /// External/field-to-field travel cost. Within-field machine time is charged
    /// separately to avoid silently mixing cost boundaries.
    #[serde(with = "exact_u64")]
    pub route_logistics_minor: u64,
    #[serde(with = "exact_u64")]
    pub mobilization_minor: u64,
    #[serde(with = "exact_u64")]
    pub recovery_minor: u64,
    #[serde(with = "exact_u64")]
    pub annual_total_minor: u64,
    #[serde(with = "exact_u64")]
    pub annual_route_distance_metres: u64,
    #[serde(with = "exact_u64")]
    pub annual_machine_minutes: u64,
    pub productive_time_share_permille: u16,
    #[serde(with = "exact_u64")]
    pub cost_per_completed_service_unit_minor: u64,
    #[serde(with = "exact_option_u64")]
    pub cost_per_hectare_minor: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldRobotCostAssessmentV1 {
    pub scenario_id: String,
    pub calculation_disposition: CostCalculationDispositionV1,
    pub evidence_disposition: CostEvidenceDispositionV1,
    pub uncertain_domains: Vec<CostEvidenceDomainV1>,
    pub blockers: Vec<CostBlockerV1>,
    pub breakdown: Option<AnnualCostBreakdownV1>,
}

fn valid_currency_code(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_uppercase())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn nonempty(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|text| !text.trim().is_empty())
}

fn push_blocker(
    blockers: &mut BTreeSet<(CostBlockerCodeV1, Option<CostEvidenceDomainV1>, String)>,
    code: CostBlockerCodeV1,
    domain: Option<CostEvidenceDomainV1>,
    detail: impl Into<String>,
) {
    blockers.insert((code, domain, detail.into()));
}

fn record_is_well_formed(record: &CostEvidenceRecordV1) -> bool {
    if record
        .artifact_sha256
        .as_deref()
        .is_some_and(|digest| !valid_sha256(digest))
    {
        return false;
    }
    match record.state {
        CostEvidenceStateV1::SourceReferenced => {
            nonempty(&record.source_ref)
                && record.artifact_sha256.as_deref().is_some_and(valid_sha256)
        }
        CostEvidenceStateV1::Estimated | CostEvidenceStateV1::SyntheticFixture => {
            nonempty(&record.source_ref)
        }
        CostEvidenceStateV1::Missing => true,
        CostEvidenceStateV1::Stale | CostEvidenceStateV1::Conflicting => {
            nonempty(&record.source_ref)
        }
    }
}

fn blockers_to_vec(
    blockers: BTreeSet<(CostBlockerCodeV1, Option<CostEvidenceDomainV1>, String)>,
) -> Vec<CostBlockerV1> {
    blockers
        .into_iter()
        .map(|(code, domain, detail)| CostBlockerV1 { code, domain, detail })
        .collect()
}

fn blocked_assessment(
    scenario_id: &str,
    blockers: BTreeSet<(CostBlockerCodeV1, Option<CostEvidenceDomainV1>, String)>,
    uncertain_domains: Vec<CostEvidenceDomainV1>,
) -> FieldRobotCostAssessmentV1 {
    FieldRobotCostAssessmentV1 {
        scenario_id: scenario_id.to_owned(),
        calculation_disposition: CostCalculationDispositionV1::Blocked,
        evidence_disposition: CostEvidenceDispositionV1::Blocked,
        uncertain_domains,
        blockers: blockers_to_vec(blockers),
        breakdown: None,
    }
}

fn checked_add_many(values: &[u64]) -> Option<u64> {
    values
        .iter()
        .try_fold(0_u64, |sum, value| sum.checked_add(*value))
}

fn ceil_div(value: u64, divisor: u64) -> Option<u64> {
    if divisor == 0 {
        return None;
    }
    let quotient = value / divisor;
    if value.is_multiple_of(divisor) {
        Some(quotient)
    } else {
        quotient.checked_add(1)
    }
}

fn ceil_mul_div(value: u64, multiplier: u64, divisor: u64) -> Option<u64> {
    if divisor == 0 {
        return None;
    }
    let numerator = u128::from(value).checked_mul(u128::from(multiplier))?;
    let denominator = u128::from(divisor);
    let quotient = numerator / denominator;
    let rounded = if numerator.is_multiple_of(denominator) {
        quotient
    } else {
        quotient.checked_add(1)?
    };
    u64::try_from(rounded).ok()
}

fn make_breakdown(input: &FieldRobotCostInputV1) -> Option<AnnualCostBreakdownV1> {
    let annualized_capital_minor = ceil_div(
        input.acquisition_cost_minor.checked_sub(input.salvage_value_minor)?,
        u64::from(input.useful_life_years),
    )?;
    let productive_machine_time_minor = ceil_mul_div(
        input.annual_productive_machine_minutes,
        input.machine_cost_minor_per_hour,
        MINUTES_PER_HOUR,
    )?;
    let nonproductive_machine_time_minor = ceil_mul_div(
        input.annual_nonproductive_machine_minutes,
        input.machine_cost_minor_per_hour,
        MINUTES_PER_HOUR,
    )?;
    let annual_machine_minutes = input
        .annual_productive_machine_minutes
        .checked_add(input.annual_nonproductive_machine_minutes)?;
    let maintenance_minor = ceil_mul_div(
        annual_machine_minutes,
        input.maintenance_cost_minor_per_100_machine_hours,
        MINUTES_PER_100_HOURS,
    )?;
    let operator_labor_minor = ceil_mul_div(
        input.annual_operator_labor_minutes,
        input.labor_cost_minor_per_hour,
        MINUTES_PER_HOUR,
    )?;
    let annual_route_distance_metres = input
        .route_distance_metres_per_mission
        .checked_mul(input.missions_per_year)?;
    let route_logistics_minor = ceil_mul_div(
        annual_route_distance_metres,
        input.route_logistics_cost_minor_per_kilometre,
        METRES_PER_KILOMETRE,
    )?;
    let mobilization_minor = input
        .mobilization_cost_minor_per_mission
        .checked_mul(input.missions_per_year)?;
    let recovery_minor = input
        .recovery_cost_minor_per_event
        .checked_mul(input.recovery_events_per_year)?;
    let annual_total_minor = checked_add_many(&[
        annualized_capital_minor,
        productive_machine_time_minor,
        nonproductive_machine_time_minor,
        maintenance_minor,
        operator_labor_minor,
        route_logistics_minor,
        mobilization_minor,
        recovery_minor,
    ])?;
    let cost_per_completed_service_unit_minor =
        ceil_div(annual_total_minor, input.annual_completed_service_units)?;
    let cost_per_hectare_minor = match input.annual_area_completed_m2 {
        Some(area_m2) => Some(ceil_mul_div(
            annual_total_minor,
            SQUARE_METRES_PER_HECTARE,
            area_m2,
        )?),
        None => None,
    };
    let productive_time_share_permille = if annual_machine_minutes == 0 {
        0
    } else {
        let numerator = u128::from(input.annual_productive_machine_minutes)
            .checked_mul(1_000)?;
        u16::try_from(numerator / u128::from(annual_machine_minutes)).ok()?
    };

    Some(AnnualCostBreakdownV1 {
        currency_code: input.currency_code.clone(),
        service_unit: input.service_unit,
        annual_completed_service_units: input.annual_completed_service_units,
        annual_area_completed_m2: input.annual_area_completed_m2,
        field_count: input.field_count,
        access_point_count: input.access_point_count,
        missions_per_year: input.missions_per_year,
        annualized_capital_minor,
        productive_machine_time_minor,
        nonproductive_machine_time_minor,
        maintenance_minor,
        operator_labor_minor,
        route_logistics_minor,
        mobilization_minor,
        recovery_minor,
        annual_total_minor,
        annual_route_distance_metres,
        annual_machine_minutes,
        productive_time_share_permille,
        cost_per_completed_service_unit_minor,
        cost_per_hectare_minor,
    })
}

/// Compute exact cost components for one declared annual service boundary.
///
/// Input references and SHA-256 values are shape-checked but not fetched or
/// cryptographically authenticated here. All minor-unit cost inputs use the
/// single declared currency code. An estimated or synthetic input yields a
/// provisional evidence disposition even when arithmetic is fully computed.
pub fn assess_field_robot_cost_v1(
    input: &FieldRobotCostInputV1,
) -> FieldRobotCostAssessmentV1 {
    let mut blockers = BTreeSet::new();
    let mut uncertain_domains = Vec::new();

    if input.schema_version != FIELD_ROBOT_COST_SCHEMA_VERSION {
        push_blocker(&mut blockers, CostBlockerCodeV1::UnsupportedSchema, None, "unsupported schema version");
    }
    if input.scenario_id.trim().is_empty() {
        push_blocker(&mut blockers, CostBlockerCodeV1::EmptyScenarioId, None, "scenario ID is blank");
    }
    if input.scope_ref.trim().is_empty() {
        push_blocker(&mut blockers, CostBlockerCodeV1::EmptyScope, None, "scope reference is blank");
    }
    if input.period_ref.trim().is_empty() {
        push_blocker(&mut blockers, CostBlockerCodeV1::EmptyPeriod, None, "period reference is blank");
    }
    if !valid_currency_code(&input.currency_code) {
        push_blocker(&mut blockers, CostBlockerCodeV1::InvalidCurrencyCode, None, "currency code must be three uppercase ASCII letters");
    }
    if input.annual_completed_service_units == 0 {
        push_blocker(&mut blockers, CostBlockerCodeV1::ZeroCompletedOutput, None, "annual completed service units must be non-zero");
    }
    if input.annual_area_completed_m2 == Some(0) {
        push_blocker(&mut blockers, CostBlockerCodeV1::ZeroCompletedArea, None, "completed area denominator must be non-zero when supplied");
    }
    if input.useful_life_years == 0 {
        push_blocker(&mut blockers, CostBlockerCodeV1::ZeroUsefulLife, None, "useful life must be non-zero");
    }
    if input.salvage_value_minor > input.acquisition_cost_minor {
        push_blocker(&mut blockers, CostBlockerCodeV1::SalvageExceedsAcquisition, None, "salvage value exceeds acquisition cost");
    }
    let machine_minutes = input
        .annual_productive_machine_minutes
        .checked_add(input.annual_nonproductive_machine_minutes);
    if machine_minutes.is_none_or(|minutes| minutes > MAX_ANNUAL_MINUTES)
        || input.annual_operator_labor_minutes > MAX_ANNUAL_MINUTES
    {
        push_blocker(&mut blockers, CostBlockerCodeV1::AnnualTimeBudgetExceeded, None, "annual machine or operator time exceeds the declared annual time budget");
    }
    if input.annual_supervision_minutes > input.annual_operator_labor_minutes {
        push_blocker(&mut blockers, CostBlockerCodeV1::SupervisionExceedsLabor, None, "supervision time is a subset of operator labor time");
    }

    let mut states = BTreeMap::new();
    for record in &input.evidence {
        if !record_is_well_formed(record) {
            push_blocker(&mut blockers, CostBlockerCodeV1::MalformedEvidenceRecord, Some(record.domain), "evidence record has a malformed source reference or digest");
        }
        if states.insert(record.domain, record.state).is_some() {
            push_blocker(&mut blockers, CostBlockerCodeV1::DuplicateEvidenceDomain, Some(record.domain), "evidence domain appears more than once");
        }
    }
    for domain in CostEvidenceDomainV1::ALL {
        if !states.contains_key(&domain) {
            push_blocker(&mut blockers, CostBlockerCodeV1::MissingEvidenceRecord, Some(domain), "required evidence domain is absent");
        }
    }
    if !blockers.is_empty() {
        return blocked_assessment(&input.scenario_id, blockers, uncertain_domains);
    }

    for domain in CostEvidenceDomainV1::ALL {
        match states.get(&domain).copied() {
            Some(CostEvidenceStateV1::SourceReferenced) => {}
            Some(CostEvidenceStateV1::Estimated | CostEvidenceStateV1::SyntheticFixture) => {
                uncertain_domains.push(domain);
            }
            Some(CostEvidenceStateV1::Missing) => {
                push_blocker(&mut blockers, CostBlockerCodeV1::EvidenceMissing, Some(domain), "required input is marked missing");
            }
            Some(CostEvidenceStateV1::Stale) => {
                push_blocker(&mut blockers, CostBlockerCodeV1::EvidenceStale, Some(domain), "required input is stale");
            }
            Some(CostEvidenceStateV1::Conflicting) => {
                push_blocker(&mut blockers, CostBlockerCodeV1::EvidenceConflicting, Some(domain), "required input has conflicting records");
            }
            None => {
                push_blocker(&mut blockers, CostBlockerCodeV1::MissingEvidenceRecord, Some(domain), "required evidence domain is absent");
            }
        }
    }
    if !blockers.is_empty() {
        return blocked_assessment(&input.scenario_id, blockers, uncertain_domains);
    }

    let Some(breakdown) = make_breakdown(input) else {
        push_blocker(&mut blockers, CostBlockerCodeV1::ArithmeticOverflow, None, "cost arithmetic overflowed or a denominator was invalid");
        return blocked_assessment(&input.scenario_id, blockers, uncertain_domains);
    };

    let evidence_disposition = if uncertain_domains.is_empty() {
        CostEvidenceDispositionV1::SourceReferencedInputs
    } else {
        CostEvidenceDispositionV1::ProvisionalInputs
    };

    FieldRobotCostAssessmentV1 {
        scenario_id: input.scenario_id.clone(),
        calculation_disposition: CostCalculationDispositionV1::Calculated,
        evidence_disposition,
        uncertain_domains,
        blockers: Vec::new(),
        breakdown: Some(breakdown),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_SHA256: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn evidence() -> Vec<CostEvidenceRecordV1> {
        CostEvidenceDomainV1::ALL
            .into_iter()
            .map(CostEvidenceRecordV1::synthetic_fixture)
            .collect()
    }

    fn baseline_input() -> FieldRobotCostInputV1 {
        FieldRobotCostInputV1 {
            schema_version: FIELD_ROBOT_COST_SCHEMA_VERSION,
            scenario_id: "synthetic-fragmented-vegetable-farm".into(),
            scope_ref: "synthetic-site-01".into(),
            period_ref: "synthetic-annual-period-01".into(),
            currency_code: "ZAR".into(),
            service_unit: ServiceUnitV1::KilogramsDelivered,
            annual_completed_service_units: 1_000,
            annual_area_completed_m2: Some(200_000),
            field_count: 4,
            access_point_count: 3,
            annual_productive_machine_minutes: 6_000,
            annual_nonproductive_machine_minutes: 1_200,
            annual_operator_labor_minutes: 1_800,
            annual_supervision_minutes: 900,
            route_distance_metres_per_mission: 12_000,
            missions_per_year: 10,
            acquisition_cost_minor: 1_000_000,
            salvage_value_minor: 0,
            useful_life_years: 5,
            machine_cost_minor_per_hour: 1_000,
            route_logistics_cost_minor_per_kilometre: 200,
            labor_cost_minor_per_hour: 900,
            maintenance_cost_minor_per_100_machine_hours: 5_000,
            mobilization_cost_minor_per_mission: 500,
            recovery_events_per_year: 2,
            recovery_cost_minor_per_event: 3_000,
            evidence: evidence(),
        }
    }

    fn referenced_evidence() -> Vec<CostEvidenceRecordV1> {
        CostEvidenceDomainV1::ALL
            .into_iter()
            .map(|domain| {
                CostEvidenceRecordV1::source_referenced(
                    domain,
                    format!("synthetic-source-ref://{domain:?}"),
                    FIXTURE_SHA256,
                )
            })
            .collect()
    }

    #[test]
    fn baseline_cost_components_match_hand_calculated_oracle() {
        let assessment = assess_field_robot_cost_v1(&baseline_input());
        assert_eq!(assessment.calculation_disposition, CostCalculationDispositionV1::Calculated);
        assert_eq!(assessment.evidence_disposition, CostEvidenceDispositionV1::ProvisionalInputs);
        let cost = assessment.breakdown.unwrap();
        assert_eq!(cost.annualized_capital_minor, 200_000);
        assert_eq!(cost.productive_machine_time_minor, 100_000);
        assert_eq!(cost.nonproductive_machine_time_minor, 20_000);
        assert_eq!(cost.maintenance_minor, 6_000);
        assert_eq!(cost.operator_labor_minor, 27_000);
        assert_eq!(cost.route_logistics_minor, 24_000);
        assert_eq!(cost.mobilization_minor, 5_000);
        assert_eq!(cost.recovery_minor, 6_000);
        assert_eq!(cost.annual_total_minor, 388_000);
        assert_eq!(cost.annual_route_distance_metres, 120_000);
        assert_eq!(cost.annual_machine_minutes, 7_200);
        assert_eq!(cost.productive_time_share_permille, 833);
        assert_eq!(cost.cost_per_completed_service_unit_minor, 388);
        assert_eq!(cost.cost_per_hectare_minor, Some(19_400));
    }

    #[test]
    fn complete_source_references_change_evidence_disposition_not_math() {
        let mut input = baseline_input();
        input.evidence = referenced_evidence();
        let assessment = assess_field_robot_cost_v1(&input);
        assert_eq!(assessment.calculation_disposition, CostCalculationDispositionV1::Calculated);
        assert_eq!(assessment.evidence_disposition, CostEvidenceDispositionV1::SourceReferencedInputs);
        assert_eq!(assessment.breakdown.unwrap().annual_total_minor, 388_000);
    }

    #[test]
    fn estimated_route_data_makes_result_provisional() {
        let mut input = baseline_input();
        let record = input.evidence.iter_mut().find(|r| r.domain == CostEvidenceDomainV1::TravelLogistics).unwrap();
        *record = CostEvidenceRecordV1::estimated(CostEvidenceDomainV1::TravelLogistics, "estimate:route-distance");
        let assessment = assess_field_robot_cost_v1(&input);
        assert_eq!(assessment.calculation_disposition, CostCalculationDispositionV1::Calculated);
        assert_eq!(assessment.evidence_disposition, CostEvidenceDispositionV1::ProvisionalInputs);
        assert!(assessment.uncertain_domains.contains(&CostEvidenceDomainV1::TravelLogistics));
    }

    #[test]
    fn missing_cost_evidence_blocks_calculation() {
        let mut input = baseline_input();
        input.evidence.iter_mut().find(|r| r.domain == CostEvidenceDomainV1::CapitalCost).unwrap().state = CostEvidenceStateV1::Missing;
        input.evidence[2].source_ref = None;
        let assessment = assess_field_robot_cost_v1(&input);
        assert_eq!(assessment.calculation_disposition, CostCalculationDispositionV1::Blocked);
        assert!(assessment.blockers.iter().any(|b| b.code == CostBlockerCodeV1::EvidenceMissing));
        assert!(assessment.breakdown.is_none());
    }

    #[test]
    fn stale_evidence_blocks_even_when_numeric_values_look_favorable() {
        let mut input = baseline_input();
        let record = input.evidence.iter_mut().find(|r| r.domain == CostEvidenceDomainV1::TravelLogistics).unwrap();
        record.state = CostEvidenceStateV1::Stale;
        record.source_ref = Some("stale-route-sheet".into());
        let assessment = assess_field_robot_cost_v1(&input);
        assert_eq!(assessment.calculation_disposition, CostCalculationDispositionV1::Blocked);
        assert!(assessment.blockers.iter().any(|b| b.code == CostBlockerCodeV1::EvidenceStale));
    }

    #[test]
    fn conflicting_repair_evidence_blocks_ranking_inputs() {
        let mut input = baseline_input();
        let record = input.evidence.iter_mut().find(|r| r.domain == CostEvidenceDomainV1::MaintenanceRecovery).unwrap();
        record.state = CostEvidenceStateV1::Conflicting;
        record.source_ref = Some("conflicting-repair-estimates".into());
        let assessment = assess_field_robot_cost_v1(&input);
        assert_eq!(assessment.evidence_disposition, CostEvidenceDispositionV1::Blocked);
    }

    #[test]
    fn duplicate_evidence_domains_block() {
        let mut input = baseline_input();
        input.evidence.push(input.evidence[0].clone());
        let assessment = assess_field_robot_cost_v1(&input);
        assert!(assessment.blockers.iter().any(|b| b.code == CostBlockerCodeV1::DuplicateEvidenceDomain));
    }

    #[test]
    fn missing_evidence_domain_blocks() {
        let mut input = baseline_input();
        input.evidence.retain(|r| r.domain != CostEvidenceDomainV1::SiteGeometry);
        let assessment = assess_field_robot_cost_v1(&input);
        assert!(assessment.blockers.iter().any(|b| b.code == CostBlockerCodeV1::MissingEvidenceRecord));
    }

    #[test]
    fn malformed_source_digest_blocks() {
        let mut input = baseline_input();
        input.evidence = referenced_evidence();
        input.evidence[0].artifact_sha256 = Some("bad-digest".into());
        let assessment = assess_field_robot_cost_v1(&input);
        assert!(assessment.blockers.iter().any(|b| b.code == CostBlockerCodeV1::MalformedEvidenceRecord));
    }

    #[test]
    fn invalid_currency_code_blocks() {
        let mut input = baseline_input();
        input.currency_code = "zaR".into();
        let assessment = assess_field_robot_cost_v1(&input);
        assert!(assessment.blockers.iter().any(|b| b.code == CostBlockerCodeV1::InvalidCurrencyCode));
    }

    #[test]
    fn unsupported_schema_blocks() {
        let mut input = baseline_input();
        input.schema_version = FIELD_ROBOT_COST_SCHEMA_VERSION + 1;
        let assessment = assess_field_robot_cost_v1(&input);
        assert!(assessment.blockers.iter().any(|b| b.code == CostBlockerCodeV1::UnsupportedSchema));
    }

    #[test]
    fn zero_output_blocks_per_unit_cost() {
        let mut input = baseline_input();
        input.annual_completed_service_units = 0;
        let assessment = assess_field_robot_cost_v1(&input);
        assert!(assessment.blockers.iter().any(|b| b.code == CostBlockerCodeV1::ZeroCompletedOutput));
    }

    #[test]
    fn zero_completed_area_blocks_hectare_cost() {
        let mut input = baseline_input();
        input.annual_area_completed_m2 = Some(0);
        let assessment = assess_field_robot_cost_v1(&input);
        assert!(assessment.blockers.iter().any(|b| b.code == CostBlockerCodeV1::ZeroCompletedArea));
    }

    #[test]
    fn zero_useful_life_blocks_capital_annualization() {
        let mut input = baseline_input();
        input.useful_life_years = 0;
        let assessment = assess_field_robot_cost_v1(&input);
        assert!(assessment.blockers.iter().any(|b| b.code == CostBlockerCodeV1::ZeroUsefulLife));
    }

    #[test]
    fn salvage_value_above_acquisition_cost_blocks() {
        let mut input = baseline_input();
        input.salvage_value_minor = input.acquisition_cost_minor + 1;
        let assessment = assess_field_robot_cost_v1(&input);
        assert!(assessment.blockers.iter().any(|b| b.code == CostBlockerCodeV1::SalvageExceedsAcquisition));
    }

    #[test]
    fn supervision_is_subset_of_labor_not_double_counted() {
        let mut input = baseline_input();
        input.annual_supervision_minutes = input.annual_operator_labor_minutes + 1;
        let assessment = assess_field_robot_cost_v1(&input);
        assert!(assessment.blockers.iter().any(|b| b.code == CostBlockerCodeV1::SupervisionExceedsLabor));
    }

    #[test]
    fn zero_route_missions_have_zero_route_and_mobilization_cost() {
        let mut input = baseline_input();
        input.missions_per_year = 0;
        let assessment = assess_field_robot_cost_v1(&input);
        let cost = assessment.breakdown.unwrap();
        assert_eq!(cost.annual_route_distance_metres, 0);
        assert_eq!(cost.route_logistics_minor, 0);
        assert_eq!(cost.mobilization_minor, 0);
    }

    #[test]
    fn route_distance_increases_logistics_cost() {
        let baseline = assess_field_robot_cost_v1(&baseline_input()).breakdown.unwrap();
        let mut input = baseline_input();
        input.route_distance_metres_per_mission *= 2;
        let longer = assess_field_robot_cost_v1(&input).breakdown.unwrap();
        assert!(longer.route_logistics_minor > baseline.route_logistics_minor);
        assert!(longer.annual_total_minor > baseline.annual_total_minor);
    }

    #[test]
    fn no_robot_baseline_is_modelled_without_fake_capital_cost() {
        let mut input = baseline_input();
        input.acquisition_cost_minor = 0;
        input.salvage_value_minor = 0;
        input.useful_life_years = 1;
        input.annual_productive_machine_minutes = 0;
        input.annual_nonproductive_machine_minutes = 0;
        input.machine_cost_minor_per_hour = 0;
        input.maintenance_cost_minor_per_100_machine_hours = 0;
        input.route_distance_metres_per_mission = 0;
        input.missions_per_year = 0;
        input.mobilization_cost_minor_per_mission = 0;
        input.recovery_events_per_year = 0;
        input.recovery_cost_minor_per_event = 0;
        let assessment = assess_field_robot_cost_v1(&input);
        let cost = assessment.breakdown.unwrap();
        assert_eq!(cost.annualized_capital_minor, 0);
        assert_eq!(cost.annual_machine_minutes, 0);
        assert_eq!(cost.productive_time_share_permille, 0);
        assert_eq!(cost.annual_total_minor, cost.operator_labor_minor);
    }

    #[test]
    fn exact_ceiling_arithmetic_never_rounds_cost_down() {
        let mut input = baseline_input();
        input.annual_operator_labor_minutes = 1;
        input.labor_cost_minor_per_hour = 1;
        input.annual_productive_machine_minutes = 1;
        input.annual_nonproductive_machine_minutes = 0;
        input.machine_cost_minor_per_hour = 1;
        input.annual_supervision_minutes = 1;
        input.maintenance_cost_minor_per_100_machine_hours = 1;
        input.route_distance_metres_per_mission = 1;
        input.missions_per_year = 1;
        input.route_logistics_cost_minor_per_kilometre = 1;
        let cost = assess_field_robot_cost_v1(&input).breakdown.unwrap();
        assert_eq!(cost.productive_machine_time_minor, 1);
        assert_eq!(cost.operator_labor_minor, 1);
        assert_eq!(cost.maintenance_minor, 1);
        assert_eq!(cost.route_logistics_minor, 1);
    }

    #[test]
    fn route_distance_overflow_blocks_without_wrapping() {
        let mut input = baseline_input();
        input.route_distance_metres_per_mission = u64::MAX;
        input.missions_per_year = 2;
        let assessment = assess_field_robot_cost_v1(&input);
        assert!(assessment.blockers.iter().any(|b| b.code == CostBlockerCodeV1::ArithmeticOverflow));
    }

    #[test]
    fn annual_time_over_budget_blocks() {
        let mut input = baseline_input();
        input.annual_productive_machine_minutes = MAX_ANNUAL_MINUTES;
        input.annual_nonproductive_machine_minutes = 1;
        let assessment = assess_field_robot_cost_v1(&input);
        assert!(assessment.blockers.iter().any(|b| b.code == CostBlockerCodeV1::AnnualTimeBudgetExceeded));
    }

    #[test]
    fn total_cost_overflow_blocks_without_partial_breakdown() {
        let mut input = baseline_input();
        input.acquisition_cost_minor = u64::MAX;
        input.salvage_value_minor = 0;
        input.useful_life_years = 1;
        input.machine_cost_minor_per_hour = u64::MAX;
        input.annual_productive_machine_minutes = 60;
        input.annual_nonproductive_machine_minutes = 0;
        let assessment = assess_field_robot_cost_v1(&input);
        assert!(assessment.blockers.iter().any(|b| b.code == CostBlockerCodeV1::ArithmeticOverflow));
        assert!(assessment.breakdown.is_none());
    }

    #[test]
    fn exact_integer_cost_values_round_trip_as_decimal_strings() {
        let input = baseline_input();
        let encoded = serde_json::to_string(&input).unwrap();
        assert!(encoded.contains("\"annual_completed_service_units\":\"1000\""));
        let decoded: FieldRobotCostInputV1 = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, input);
    }
}
