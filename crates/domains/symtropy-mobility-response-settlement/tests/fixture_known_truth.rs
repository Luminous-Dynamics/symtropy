// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_mobility_response_settlement::{
    MobilityResponseSettlementInput, ReconciliationStatus, evaluate_mobility_response_settlement,
};

fn fixture() -> MobilityResponseSettlementInput {
    serde_json::from_str(include_str!(
        "../fixtures/mobility_response_settlement_v0_1.json"
    ))
    .expect("frozen four-report settlement bundle must parse")
}

#[test]
fn frozen_four_report_bundle_reconciles_and_never_grants_settlement_authority() {
    let report = evaluate_mobility_response_settlement(&fixture()).unwrap();
    assert_eq!(report.status, ReconciliationStatus::TechnicallyReconciled);
    assert!(report.technical_reconciliation_passed);
    assert_eq!(report.source_artifacts.len(), 4);
    assert!((report.requested_response_energy_kwh - 0.005555555555555556).abs() < 1.0e-12);
    assert!(
        (report.dispatch_tracking_signed_delivered_energy_kwh - 0.005555555555555556).abs()
            < 1.0e-12
    );
    assert!((report.pcc_point_net_reduction_energy_kwh - 0.005416666666666667).abs() < 1.0e-12);
    assert!(
        (report.counterfactual_net_reduction_energy_kwh - 0.005305555555555556).abs() < 1.0e-12
    );
    assert!(
        (report.conservative_net_response_lower_bound_kwh.unwrap() - 0.005194444444444444).abs()
            < 1.0e-12
    );
    assert!(!report.conservative_lower_bound_is_joint_probability);
    assert!(!report.source_artifact_signatures_verified);
    assert!(!report.settlement_authority_granted);
    assert!(!report.payment_authorized);
    assert!(!report.market_eligibility_granted);
    assert!(!report.dispatch_authority_granted);
    assert!(!report.interconnection_approval_granted);
    assert!(!report.field_qualification_granted);
}
