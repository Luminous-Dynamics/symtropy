// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic bounded-scenario envelope around the field-robot cost model.
//!
//! This module does not invent probability distributions or confidence intervals.
//! It assesses every supplied scenario with the canonical point cost kernel,
//! preserves blocked scenarios, and marks incomplete envelopes as partial.
//! It does not compare different platform candidates or implement a Pareto layer.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeSet;

use super::field_robot_cost::{
    assess_field_robot_cost_v1, CostCalculationDispositionV1, CostEvidenceDispositionV1,
    FieldRobotCostAssessmentV1, FieldRobotCostInputV1, ServiceUnitV1,
};

pub const FIELD_ROBOT_COST_SENSITIVITY_SCHEMA_VERSION: u16 = 1;

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
pub enum CostSensitivityBlockerCodeV1 {
    UnsupportedSchema,
    EmptySensitivityId,
    EmptyScope,
    EmptyPeriod,
    InvalidCurrencyCode,
    EmptyScenarioSet,
    EmptyScenarioId,
    DuplicateScenarioId,
    ScopeMismatch,
    PeriodMismatch,
    CurrencyMismatch,
    ServiceUnitMismatch,
    AreaDenominatorModeMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostSensitivityBlockerV1 {
    pub code: CostSensitivityBlockerCodeV1,
    pub scenario_id: Option<String>,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldRobotCostSensitivityRequestV1 {
    pub schema_version: u16,
    pub sensitivity_id: String,
    pub scope_ref: String,
    pub period_ref: String,
    pub currency_code: String,
    pub service_unit: ServiceUnitV1,
    /// A maximum acceptable cost in the declared currency's minor units per
    /// completed service unit. It is a comparison threshold, not a universal
    /// recommendation or automatic pass/fail authorization.
    #[serde(with = "exact_option_u64")]
    pub cost_per_service_unit_threshold_minor: Option<u64>,
    /// Every input uses the existing exact cost kernel. Scenario IDs must be
    /// unique; scope, period, currency, service unit and area-denominator mode
    /// must match across the request. Input values such as route or utilization
    /// can vary between scenarios.
    pub scenarios: Vec<FieldRobotCostInputV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostSensitivityDispositionV1 {
    /// Every supplied scenario produced a numeric cost result.
    Complete,
    /// At least one scenario produced a result, but one or more are blocked.
    /// The reported ranges cover calculated cases only and are not guaranteed bounds.
    Partial,
    /// The request is invalid or no supplied scenario produced a numeric result.
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedCostRangeV1 {
    #[serde(with = "exact_u64")]
    pub minimum_minor: u64,
    #[serde(with = "exact_u64")]
    pub maximum_minor: u64,
    /// Number of scenario results contributing to this observed range.
    pub calculated_case_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostThresholdObservationV1 {
    #[serde(with = "exact_u64")]
    pub threshold_minor_per_service_unit: u64,
    /// Counts include calculated provisional cases and source-referenced cases;
    /// these are observations over supplied scenarios, not probabilities.
    pub calculated_cases_within_threshold: usize,
    pub calculated_cases_exceeding_threshold: usize,
    /// True only when the currently calculated scenario results fall on both
    /// sides of the threshold. A Partial envelope may still omit worse outcomes.
    pub observed_straddles_threshold: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldRobotCostSensitivityCaseV1 {
    pub scenario_id: String,
    pub assessment: FieldRobotCostAssessmentV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldRobotCostSensitivityReportV1 {
    pub schema_version: u16,
    pub sensitivity_id: String,
    pub disposition: CostSensitivityDispositionV1,
    pub blockers: Vec<CostSensitivityBlockerV1>,
    pub scenario_count: usize,
    pub calculated_case_count: usize,
    pub source_referenced_case_count: usize,
    pub provisional_case_count: usize,
    pub blocked_case_count: usize,
    /// Min/max across calculated scenarios only. Partial reports explicitly
    /// retain blocked cases and cannot be described as guaranteed limits.
    pub observed_annual_total_minor: Option<ObservedCostRangeV1>,
    pub observed_cost_per_service_unit_minor: Option<ObservedCostRangeV1>,
    pub observed_cost_per_hectare_minor: Option<ObservedCostRangeV1>,
    pub threshold_observation: Option<CostThresholdObservationV1>,
    /// Always sorted by scenario ID so input order cannot change report order.
    pub cases: Vec<FieldRobotCostSensitivityCaseV1>,
}

fn valid_currency_code(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_uppercase())
}

fn push_blocker(
    blockers: &mut BTreeSet<(CostSensitivityBlockerCodeV1, Option<String>, String)>,
    code: CostSensitivityBlockerCodeV1,
    scenario_id: Option<String>,
    detail: impl Into<String>,
) {
    blockers.insert((code, scenario_id, detail.into()));
}

fn blockers_to_vec(
    blockers: BTreeSet<(CostSensitivityBlockerCodeV1, Option<String>, String)>,
) -> Vec<CostSensitivityBlockerV1> {
    blockers
        .into_iter()
        .map(|(code, scenario_id, detail)| CostSensitivityBlockerV1 {
            code,
            scenario_id,
            detail,
        })
        .collect()
}

fn blocked_report(
    request: &FieldRobotCostSensitivityRequestV1,
    blockers: BTreeSet<(CostSensitivityBlockerCodeV1, Option<String>, String)>,
) -> FieldRobotCostSensitivityReportV1 {
    FieldRobotCostSensitivityReportV1 {
        schema_version: FIELD_ROBOT_COST_SENSITIVITY_SCHEMA_VERSION,
        sensitivity_id: request.sensitivity_id.clone(),
        disposition: CostSensitivityDispositionV1::Blocked,
        blockers: blockers_to_vec(blockers),
        scenario_count: request.scenarios.len(),
        calculated_case_count: 0,
        source_referenced_case_count: 0,
        provisional_case_count: 0,
        blocked_case_count: request.scenarios.len(),
        observed_annual_total_minor: None,
        observed_cost_per_service_unit_minor: None,
        observed_cost_per_hectare_minor: None,
        threshold_observation: request.cost_per_service_unit_threshold_minor.map(|threshold| {
            CostThresholdObservationV1 {
                threshold_minor_per_service_unit: threshold,
                calculated_cases_within_threshold: 0,
                calculated_cases_exceeding_threshold: 0,
                observed_straddles_threshold: false,
            }
        }),
        cases: Vec::new(),
    }
}

fn observed_range(values: &[u64]) -> Option<ObservedCostRangeV1> {
    Some(ObservedCostRangeV1 {
        minimum_minor: *values.iter().min()?,
        maximum_minor: *values.iter().max()?,
        calculated_case_count: values.len(),
    })
}

/// Assess an explicit set of bounded scenarios with the point cost calculator.
///
/// Complete means every listed scenario calculated. Partial means at least one
/// case calculated and at least one case is blocked. Blocked means the request
/// failed global validation or zero cases calculated. Ranges are descriptive of
/// calculated scenarios only and are never probability intervals or guaranteed
/// bounds when the envelope is partial.
pub fn assess_field_robot_cost_sensitivity_v1(
    request: &FieldRobotCostSensitivityRequestV1,
) -> FieldRobotCostSensitivityReportV1 {
    let mut blockers = BTreeSet::new();

    if request.schema_version != FIELD_ROBOT_COST_SENSITIVITY_SCHEMA_VERSION {
        push_blocker(&mut blockers, CostSensitivityBlockerCodeV1::UnsupportedSchema, None, "unsupported sensitivity schema version");
    }
    if request.sensitivity_id.trim().is_empty() {
        push_blocker(&mut blockers, CostSensitivityBlockerCodeV1::EmptySensitivityId, None, "sensitivity ID is blank");
    }
    if request.scope_ref.trim().is_empty() {
        push_blocker(&mut blockers, CostSensitivityBlockerCodeV1::EmptyScope, None, "request scope is blank");
    }
    if request.period_ref.trim().is_empty() {
        push_blocker(&mut blockers, CostSensitivityBlockerCodeV1::EmptyPeriod, None, "request period is blank");
    }
    if !valid_currency_code(&request.currency_code) {
        push_blocker(&mut blockers, CostSensitivityBlockerCodeV1::InvalidCurrencyCode, None, "currency must be three uppercase ASCII letters");
    }
    if request.scenarios.is_empty() {
        push_blocker(&mut blockers, CostSensitivityBlockerCodeV1::EmptyScenarioSet, None, "at least one scenario is required");
    }

    let mut scenario_ids = BTreeSet::new();
    let area_mode = request
        .scenarios
        .first()
        .map(|scenario| scenario.annual_area_completed_m2.is_some());

    for scenario in &request.scenarios {
        if scenario.scenario_id.trim().is_empty() {
            push_blocker(&mut blockers, CostSensitivityBlockerCodeV1::EmptyScenarioId, Some(scenario.scenario_id.clone()), "scenario ID is blank");
        }
        if !scenario_ids.insert(scenario.scenario_id.clone()) {
            push_blocker(&mut blockers, CostSensitivityBlockerCodeV1::DuplicateScenarioId, Some(scenario.scenario_id.clone()), "scenario ID is duplicated");
        }
        if scenario.scope_ref != request.scope_ref {
            push_blocker(&mut blockers, CostSensitivityBlockerCodeV1::ScopeMismatch, Some(scenario.scenario_id.clone()), "scenario scope differs from sensitivity request");
        }
        if scenario.period_ref != request.period_ref {
            push_blocker(&mut blockers, CostSensitivityBlockerCodeV1::PeriodMismatch, Some(scenario.scenario_id.clone()), "scenario period differs from sensitivity request");
        }
        if scenario.currency_code != request.currency_code {
            push_blocker(&mut blockers, CostSensitivityBlockerCodeV1::CurrencyMismatch, Some(scenario.scenario_id.clone()), "scenario currency differs from sensitivity request");
        }
        if scenario.service_unit != request.service_unit {
            push_blocker(&mut blockers, CostSensitivityBlockerCodeV1::ServiceUnitMismatch, Some(scenario.scenario_id.clone()), "scenario service unit differs from sensitivity request");
        }
        if Some(scenario.annual_area_completed_m2.is_some()) != area_mode {
            push_blocker(&mut blockers, CostSensitivityBlockerCodeV1::AreaDenominatorModeMismatch, Some(scenario.scenario_id.clone()), "cost-per-hectare denominator mode differs across scenarios");
        }
    }

    if !blockers.is_empty() {
        return blocked_report(request, blockers);
    }

    let mut scenarios = request.scenarios.clone();
    scenarios.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));

    let mut cases = Vec::with_capacity(scenarios.len());
    let mut annual_totals = Vec::new();
    let mut per_service_unit = Vec::new();
    let mut per_hectare = Vec::new();
    let mut calculated_case_count = 0_usize;
    let mut source_referenced_case_count = 0_usize;
    let mut provisional_case_count = 0_usize;
    let mut blocked_case_count = 0_usize;
    let mut threshold_within = 0_usize;
    let mut threshold_exceeding = 0_usize;

    for scenario in scenarios {
        let scenario_id = scenario.scenario_id.clone();
        let assessment = assess_field_robot_cost_v1(&scenario);
        match assessment.calculation_disposition {
            CostCalculationDispositionV1::Calculated => {
                calculated_case_count += 1;
                match assessment.evidence_disposition {
                    CostEvidenceDispositionV1::SourceReferencedInputs => source_referenced_case_count += 1,
                    CostEvidenceDispositionV1::ProvisionalInputs => provisional_case_count += 1,
                    CostEvidenceDispositionV1::Blocked => blocked_case_count += 1,
                }
                if let Some(breakdown) = assessment.breakdown.as_ref() {
                    annual_totals.push(breakdown.annual_total_minor);
                    per_service_unit.push(breakdown.cost_per_completed_service_unit_minor);
                    if let Some(cost) = breakdown.cost_per_hectare_minor {
                        per_hectare.push(cost);
                    }
                    if let Some(threshold) = request.cost_per_service_unit_threshold_minor {
                        if breakdown.cost_per_completed_service_unit_minor <= threshold {
                            threshold_within += 1;
                        } else {
                            threshold_exceeding += 1;
                        }
                    }
                }
            }
            CostCalculationDispositionV1::Blocked => blocked_case_count += 1,
        }
        cases.push(FieldRobotCostSensitivityCaseV1 {
            scenario_id,
            assessment,
        });
    }

    let disposition = if calculated_case_count == 0 {
        CostSensitivityDispositionV1::Blocked
    } else if blocked_case_count > 0 {
        CostSensitivityDispositionV1::Partial
    } else {
        CostSensitivityDispositionV1::Complete
    };

    FieldRobotCostSensitivityReportV1 {
        schema_version: FIELD_ROBOT_COST_SENSITIVITY_SCHEMA_VERSION,
        sensitivity_id: request.sensitivity_id.clone(),
        disposition,
        blockers: Vec::new(),
        scenario_count: cases.len(),
        calculated_case_count,
        source_referenced_case_count,
        provisional_case_count,
        blocked_case_count,
        observed_annual_total_minor: observed_range(&annual_totals),
        observed_cost_per_service_unit_minor: observed_range(&per_service_unit),
        observed_cost_per_hectare_minor: if area_mode == Some(true) {
            observed_range(&per_hectare)
        } else {
            None
        },
        threshold_observation: request.cost_per_service_unit_threshold_minor.map(|threshold| {
            CostThresholdObservationV1 {
                threshold_minor_per_service_unit: threshold,
                calculated_cases_within_threshold: threshold_within,
                calculated_cases_exceeding_threshold: threshold_exceeding,
                observed_straddles_threshold: threshold_within > 0 && threshold_exceeding > 0,
            }
        }),
        cases,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_robot_cost::{
        CostEvidenceDomainV1, CostEvidenceStateV1, CostEvidenceRecordV1,
    };

    fn baseline_input(scenario_id: &str) -> FieldRobotCostInputV1 {
        let evidence = CostEvidenceDomainV1::ALL
            .into_iter()
            .map(CostEvidenceRecordV1::synthetic_fixture)
            .collect();

        FieldRobotCostInputV1 {
            schema_version: 1,
            scenario_id: scenario_id.into(),
            scope_ref: "synthetic-site-01".into(),
            period_ref: "synthetic-year-01".into(),
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
            evidence,
        }
    }

    fn request(scenarios: Vec<FieldRobotCostInputV1>) -> FieldRobotCostSensitivityRequestV1 {
        FieldRobotCostSensitivityRequestV1 {
            schema_version: FIELD_ROBOT_COST_SENSITIVITY_SCHEMA_VERSION,
            sensitivity_id: "synthetic-route-distance-sweep".into(),
            scope_ref: "synthetic-site-01".into(),
            period_ref: "synthetic-year-01".into(),
            currency_code: "ZAR".into(),
            service_unit: ServiceUnitV1::KilogramsDelivered,
            cost_per_service_unit_threshold_minor: Some(400),
            scenarios,
        }
    }

    #[test]
    fn hand_calculated_route_sweep_produces_exact_observed_ranges() {
        let mut low = baseline_input("low-route");
        low.route_distance_metres_per_mission = 1_000;
        let base = baseline_input("base-route");
        let mut high = baseline_input("high-route");
        high.route_distance_metres_per_mission = 30_000;

        let report = assess_field_robot_cost_sensitivity_v1(&request(vec![high, base, low]));
        assert_eq!(report.disposition, CostSensitivityDispositionV1::Complete);
        assert_eq!(report.scenario_count, 3);
        assert_eq!(report.calculated_case_count, 3);
        assert_eq!(report.provisional_case_count, 3);
        assert_eq!(report.observed_annual_total_minor.as_ref().unwrap().minimum_minor, 366_000);
        assert_eq!(report.observed_annual_total_minor.as_ref().unwrap().maximum_minor, 424_000);
        assert_eq!(report.observed_cost_per_service_unit_minor.as_ref().unwrap().minimum_minor, 366);
        assert_eq!(report.observed_cost_per_service_unit_minor.as_ref().unwrap().maximum_minor, 424);
        assert_eq!(report.observed_cost_per_hectare_minor.as_ref().unwrap().minimum_minor, 18_300);
        assert_eq!(report.observed_cost_per_hectare_minor.as_ref().unwrap().maximum_minor, 21_200);
        let threshold = report.threshold_observation.unwrap();
        assert_eq!(threshold.calculated_cases_within_threshold, 2);
        assert_eq!(threshold.calculated_cases_exceeding_threshold, 1);
        assert!(threshold.observed_straddles_threshold);
        assert_eq!(
            report.cases.iter().map(|case| case.scenario_id.as_str()).collect::<Vec<_>>(),
            vec!["base-route", "high-route", "low-route"]
        );
    }

    #[test]
    fn stale_case_yields_partial_not_a_complete_bound() {
        let low = baseline_input("low-route");
        let mut stale = baseline_input("stale-route");
        let record = stale.evidence.iter_mut().find(|e| e.domain == CostEvidenceDomainV1::TravelLogistics).unwrap();
        record.state = CostEvidenceStateV1::Stale;
        record.source_ref = Some("source://stale-route-sheet".into());

        let report = assess_field_robot_cost_sensitivity_v1(&request(vec![low, stale]));
        assert_eq!(report.disposition, CostSensitivityDispositionV1::Partial);
        assert_eq!(report.calculated_case_count, 1);
        assert_eq!(report.blocked_case_count, 1);
        assert_eq!(report.observed_annual_total_minor.unwrap().calculated_case_count, 1);
        assert_eq!(report.observed_cost_per_service_unit_minor.unwrap().calculated_case_count, 1);
    }

    #[test]
    fn all_blocked_scenarios_return_no_numeric_range() {
        let mut input = baseline_input("blocked-route");
        input.evidence.retain(|e| e.domain != CostEvidenceDomainV1::TravelLogistics);
        let report = assess_field_robot_cost_sensitivity_v1(&request(vec![input]));
        assert_eq!(report.disposition, CostSensitivityDispositionV1::Blocked);
        assert_eq!(report.blocked_case_count, 1);
        assert!(report.observed_annual_total_minor.is_none());
        assert!(report.threshold_observation.is_some());
    }

    #[test]
    fn input_order_permutation_produces_identical_report() {
        let low = baseline_input("low-route");
        let base = baseline_input("base-route");
        let mut high = baseline_input("high-route");
        high.route_distance_metres_per_mission = 30_000;
        let forward = assess_field_robot_cost_sensitivity_v1(&request(vec![low.clone(), base.clone(), high.clone()]));
        let reverse = assess_field_robot_cost_sensitivity_v1(&request(vec![high, base, low]));
        assert_eq!(forward, reverse);
        assert_eq!(
            serde_json::to_string(&forward).unwrap(),
            serde_json::to_string(&reverse).unwrap()
        );
    }

    #[test]
    fn duplicate_scenario_ids_block_the_whole_request() {
        let one = baseline_input("duplicate");
        let two = baseline_input("duplicate");
        let report = assess_field_robot_cost_sensitivity_v1(&request(vec![one, two]));
        assert_eq!(report.disposition, CostSensitivityDispositionV1::Blocked);
        assert!(report.blockers.iter().any(|b| b.code == CostSensitivityBlockerCodeV1::DuplicateScenarioId));
        assert!(report.cases.is_empty());
    }

    #[test]
    fn scope_period_currency_and_unit_mismatches_block_the_whole_request() {
        let base = baseline_input("base");
        let mut wrong = baseline_input("wrong-scope");
        wrong.scope_ref = "another-site".into();
        let report = assess_field_robot_cost_sensitivity_v1(&request(vec![base.clone(), wrong]));
        assert!(report.blockers.iter().any(|b| b.code == CostSensitivityBlockerCodeV1::ScopeMismatch));
        assert!(report.cases.is_empty());

        let mut wrong = baseline_input("wrong-period");
        wrong.period_ref = "another-year".into();
        let report = assess_field_robot_cost_sensitivity_v1(&request(vec![base.clone(), wrong]));
        assert!(report.blockers.iter().any(|b| b.code == CostSensitivityBlockerCodeV1::PeriodMismatch));

        let mut wrong = baseline_input("wrong-currency");
        wrong.currency_code = "USD".into();
        let report = assess_field_robot_cost_sensitivity_v1(&request(vec![base.clone(), wrong]));
        assert!(report.blockers.iter().any(|b| b.code == CostSensitivityBlockerCodeV1::CurrencyMismatch));

        let mut wrong = baseline_input("wrong-unit");
        wrong.service_unit = ServiceUnitV1::SquareMetresCompleted;
        let report = assess_field_robot_cost_sensitivity_v1(&request(vec![base, wrong]));
        assert!(report.blockers.iter().any(|b| b.code == CostSensitivityBlockerCodeV1::ServiceUnitMismatch));
    }

    #[test]
    fn area_denominator_mode_mismatch_blocks_hectare_range() {
        let with_area = baseline_input("has-area");
        let mut without_area = baseline_input("no-area");
        without_area.annual_area_completed_m2 = None;
        let report = assess_field_robot_cost_sensitivity_v1(&request(vec![with_area, without_area]));
        assert_eq!(report.disposition, CostSensitivityDispositionV1::Blocked);
        assert!(report.blockers.iter().any(|b| b.code == CostSensitivityBlockerCodeV1::AreaDenominatorModeMismatch));
    }

    #[test]
    fn zero_scenarios_blocks_the_request() {
        let report = assess_field_robot_cost_sensitivity_v1(&request(Vec::new()));
        assert_eq!(report.disposition, CostSensitivityDispositionV1::Blocked);
        assert!(report.blockers.iter().any(|b| b.code == CostSensitivityBlockerCodeV1::EmptyScenarioSet));
    }

    #[test]
    fn sensitivity_report_round_trips_exact_integer_ranges() {
        let report = assess_field_robot_cost_sensitivity_v1(&request(vec![baseline_input("base")]));
        let encoded = serde_json::to_string(&report).unwrap();
        assert!(encoded.contains("\"minimum_minor\":\"388000\""));
        let decoded: FieldRobotCostSensitivityReportV1 = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, report);
    }
}
