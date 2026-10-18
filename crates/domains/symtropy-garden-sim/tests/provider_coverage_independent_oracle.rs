// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Hand-authored provider-coverage outcomes with an independent scenario fixture.
//! Passing these tests does not establish a real provider's affordability or viability.

use symtropy_garden_sim::field_robot_cost::{
    CostEvidenceDispositionV1, CostEvidenceDomainV1, CostEvidenceRecordV1, CostEvidenceStateV1,
    FIELD_ROBOT_COST_SCHEMA_VERSION, FieldRobotCostInputV1, ServiceUnitV1,
};
use symtropy_garden_sim::provider_coverage::{
    PROVIDER_COVERAGE_SCHEMA_VERSION, ProviderCostCoverageV1, ProviderCoverageBlockerCodeV1,
    ProviderCoverageCalculationDispositionV1, ProviderRevenueEvidenceV1,
    ServiceProviderCoverageInputV1, assess_service_provider_coverage_v1,
};

const DIGEST: &str = "abababababababababababababababababababababababababababababababab";

fn independent_cost_input() -> FieldRobotCostInputV1 {
    let evidence = [
        CostEvidenceDomainV1::SiteGeometry,
        CostEvidenceDomainV1::ServiceOutput,
        CostEvidenceDomainV1::CapitalCost,
        CostEvidenceDomainV1::MachineOperatingCost,
        CostEvidenceDomainV1::TravelLogistics,
        CostEvidenceDomainV1::LabourCost,
        CostEvidenceDomainV1::MaintenanceRecovery,
    ]
    .into_iter()
    .map(|domain| CostEvidenceRecordV1 {
        domain,
        state: CostEvidenceStateV1::SyntheticFixture,
        source_ref: Some(format!("synthetic-fixture:independent:{domain:?}")),
        artifact_sha256: None,
    })
    .collect();

    FieldRobotCostInputV1 {
        schema_version: FIELD_ROBOT_COST_SCHEMA_VERSION,
        scenario_id: "provider-envelope-oracle-v1".into(),
        scope_ref: "provider-envelope-scope-v1".into(),
        period_ref: "provider-envelope-year-v1".into(),
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

fn independent_coverage_input() -> ServiceProviderCoverageInputV1 {
    ServiceProviderCoverageInputV1 {
        schema_version: PROVIDER_COVERAGE_SCHEMA_VERSION,
        scenario_id: "provider-envelope-oracle-v1".into(),
        scope_ref: "provider-envelope-scope-v1".into(),
        period_ref: "provider-envelope-year-v1".into(),
        currency_code: "ZAR".into(),
        primary_service_units_paid: 500,
        fee_minor_per_paid_unit: 500,
        other_service_receipts_minor: 300_000,
        primary_revenue_evidence: ProviderRevenueEvidenceV1 {
            state: CostEvidenceStateV1::SyntheticFixture,
            source_ref: Some("synthetic-fixture:independent-primary-receipts".into()),
            artifact_sha256: None,
        },
        other_service_revenue_evidence: ProviderRevenueEvidenceV1 {
            state: CostEvidenceStateV1::SyntheticFixture,
            source_ref: Some("synthetic-fixture:independent-other-receipts".into()),
            artifact_sha256: None,
        },
        annual_cost_input: independent_cost_input(),
    }
}

fn assert_blocked(input: &ServiceProviderCoverageInputV1, code: ProviderCoverageBlockerCodeV1) {
    let report = assess_service_provider_coverage_v1(input);
    assert_eq!(
        report.calculation_disposition,
        ProviderCoverageCalculationDispositionV1::Blocked
    );
    assert_eq!(
        report.evidence_disposition,
        CostEvidenceDispositionV1::Blocked
    );
    assert!(report.blockers.iter().any(|blocker| blocker.code == code));
    assert!(report.breakdown.is_none());
}

#[test]
fn positive_provider_coverage_matches_hand_calculated_receipt_oracle() {
    let report = assess_service_provider_coverage_v1(&independent_coverage_input());
    assert_eq!(
        report.calculation_disposition,
        ProviderCoverageCalculationDispositionV1::Calculated
    );
    assert_eq!(
        report.evidence_disposition,
        CostEvidenceDispositionV1::ProvisionalInputs
    );
    let breakdown = report.breakdown.unwrap();
    assert_eq!(breakdown.primary_service_receipts_minor, 250_000);
    assert_eq!(breakdown.other_service_receipts_minor, 300_000);
    assert_eq!(breakdown.total_receipts_minor, 550_000);
    assert_eq!(breakdown.modeled_annual_cost_minor, 388_000);
    assert_eq!(breakdown.modeled_surplus_minor, 162_000);
    assert_eq!(
        breakdown.coverage,
        ProviderCostCoverageV1::RevenueCoversModeledAnnualCost
    );
    assert_eq!(breakdown.completed_service_units, 1_000);
    assert_eq!(breakdown.paid_service_units, 500);
    assert_eq!(breakdown.unpaid_completed_service_units, 500);
}

#[test]
fn revenue_shortfall_is_reported_exactly() {
    let mut input = independent_coverage_input();
    input.primary_service_units_paid = 100;
    input.other_service_receipts_minor = 0;
    let report = assess_service_provider_coverage_v1(&input);
    let breakdown = report.breakdown.unwrap();
    assert_eq!(breakdown.total_receipts_minor, 50_000);
    assert_eq!(breakdown.modeled_surplus_minor, -338_000);
    assert_eq!(
        breakdown.coverage,
        ProviderCostCoverageV1::RevenueBelowModeledAnnualCost
    );
}

#[test]
fn other_service_revenue_is_separate_and_cannot_imply_primary_service_affordability() {
    let mut input = independent_coverage_input();
    input.primary_service_units_paid = 0;
    input.other_service_receipts_minor = 400_000;
    let breakdown = assess_service_provider_coverage_v1(&input)
        .breakdown
        .unwrap();
    assert_eq!(breakdown.primary_service_receipts_minor, 0);
    assert_eq!(breakdown.other_service_receipts_minor, 400_000);
    assert_eq!(breakdown.modeled_surplus_minor, 12_000);
    assert_eq!(breakdown.unpaid_completed_service_units, 1_000);
    assert_eq!(
        breakdown.coverage,
        ProviderCostCoverageV1::RevenueCoversModeledAnnualCost
    );
}

#[test]
fn paid_units_above_completed_units_blocks() {
    let mut input = independent_coverage_input();
    input.primary_service_units_paid = input.annual_cost_input.annual_completed_service_units + 1;
    assert_blocked(
        &input,
        ProviderCoverageBlockerCodeV1::PaidUnitsExceedCompletedOutput,
    );
}

#[test]
fn scenario_mismatch_blocks() {
    let mut input = independent_coverage_input();
    input.scenario_id.push_str("-mismatch");
    assert_blocked(&input, ProviderCoverageBlockerCodeV1::ScenarioMismatch);
}

#[test]
fn scope_mismatch_blocks() {
    let mut input = independent_coverage_input();
    input.scope_ref.push_str("-mismatch");
    assert_blocked(&input, ProviderCoverageBlockerCodeV1::ScopeMismatch);
}

#[test]
fn period_mismatch_blocks() {
    let mut input = independent_coverage_input();
    input.period_ref.push_str("-mismatch");
    assert_blocked(&input, ProviderCoverageBlockerCodeV1::PeriodMismatch);
}

#[test]
fn currency_mismatch_blocks() {
    let mut input = independent_coverage_input();
    input.currency_code = "USD".into();
    assert_blocked(&input, ProviderCoverageBlockerCodeV1::CurrencyMismatch);
}

#[test]
fn blocked_cost_input_blocks_provider_coverage() {
    let mut input = independent_coverage_input();
    input.annual_cost_input.annual_completed_service_units = 0;
    assert_blocked(&input, ProviderCoverageBlockerCodeV1::CostInputBlocked);
}

#[test]
fn stale_primary_revenue_evidence_blocks() {
    let mut input = independent_coverage_input();
    input.primary_revenue_evidence.state = CostEvidenceStateV1::Stale;
    assert_blocked(&input, ProviderCoverageBlockerCodeV1::RevenueEvidenceStale);
}

#[test]
fn missing_other_service_evidence_blocks() {
    let mut input = independent_coverage_input();
    input.other_service_revenue_evidence.state = CostEvidenceStateV1::Missing;
    assert_blocked(
        &input,
        ProviderCoverageBlockerCodeV1::RevenueEvidenceMissing,
    );
}

#[test]
fn malformed_source_digest_blocks_revenue() {
    let mut input = independent_coverage_input();
    input.primary_revenue_evidence = ProviderRevenueEvidenceV1::source_referenced(
        "synthetic-source://primary-receipts",
        "not-a-sha256-digest",
    );
    assert_blocked(
        &input,
        ProviderCoverageBlockerCodeV1::InvalidRevenueEvidence,
    );
}

#[test]
fn fee_revenue_overflow_blocks_without_partial_output() {
    let mut input = independent_coverage_input();
    input.primary_service_units_paid = u64::MAX;
    input.annual_cost_input.annual_completed_service_units = u64::MAX;
    input.fee_minor_per_paid_unit = 2;
    assert_blocked(&input, ProviderCoverageBlockerCodeV1::ArithmeticOverflow);
}

#[test]
fn revenue_total_overflow_blocks_without_partial_output() {
    let mut input = independent_coverage_input();
    input.primary_service_units_paid = 1;
    input.fee_minor_per_paid_unit = 1;
    input.other_service_receipts_minor = u64::MAX;
    assert_blocked(&input, ProviderCoverageBlockerCodeV1::ArithmeticOverflow);
}

#[test]
fn source_referenced_inputs_are_declared_but_not_authenticated() {
    let mut input = independent_coverage_input();
    input.annual_cost_input.evidence = [
        CostEvidenceDomainV1::SiteGeometry,
        CostEvidenceDomainV1::ServiceOutput,
        CostEvidenceDomainV1::CapitalCost,
        CostEvidenceDomainV1::MachineOperatingCost,
        CostEvidenceDomainV1::TravelLogistics,
        CostEvidenceDomainV1::LabourCost,
        CostEvidenceDomainV1::MaintenanceRecovery,
    ]
    .into_iter()
    .map(|domain| CostEvidenceRecordV1 {
        domain,
        state: CostEvidenceStateV1::SourceReferenced,
        source_ref: Some(format!("fixture-source://{domain:?}")),
        artifact_sha256: Some(DIGEST.to_owned()),
    })
    .collect();
    input.primary_revenue_evidence =
        ProviderRevenueEvidenceV1::source_referenced("receipt-source://primary", DIGEST);
    input.other_service_revenue_evidence =
        ProviderRevenueEvidenceV1::source_referenced("receipt-source://other", DIGEST);
    let report = assess_service_provider_coverage_v1(&input);
    assert_eq!(
        report.calculation_disposition,
        ProviderCoverageCalculationDispositionV1::Calculated
    );
    assert_eq!(
        report.evidence_disposition,
        CostEvidenceDispositionV1::SourceReferencedInputs
    );
}
