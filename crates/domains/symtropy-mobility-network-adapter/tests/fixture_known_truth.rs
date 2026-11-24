// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_mobility_network_adapter::{
    CompositeDisposition, MobilityNetworkAdapterInput, evaluate_mobility_network_adapter,
};

fn fixture() -> MobilityNetworkAdapterInput {
    serde_json::from_str(include_str!("../fixtures/mobility_network_adapter_v0_1.json"))
        .expect("frozen adapter fixture must parse")
}

#[test]
fn frozen_three_bus_adapter_fixture_preserves_partial_deliverability_and_claim_boundary() {
    let report = evaluate_mobility_network_adapter(&fixture()).unwrap();
    assert_eq!(report.disposition, CompositeDisposition::ScreenedPartialRequest);
    assert_eq!(report.upstream_receipts.len(), 5);
    assert!((report.site_requested_response_energy_mwh - 30.0).abs() < 1.0e-9);
    assert!(
        (report
            .maximum_dc_network_deliverable_response_energy_mwh
            .unwrap()
            - 17.5)
            .abs()
            < 1.0e-8
    );
    assert!(
        (report.dc_undeliverable_or_curtailed_energy_mwh.unwrap() - 12.5).abs() < 1.0e-8
    );
    assert_eq!(report.full_requested_trajectory_dc_deliverable, Some(false));
    assert!(report.dc_network_report.is_some());
    assert_eq!(report.actual_measured_pcc_response_energy_mwh, None);
    assert!(!report.actual_metered_delivery_assessed);
    assert!(!report.adequacy_contribution_computed);
    assert!(!report.ac_feasibility_established);
    assert!(!report.market_eligibility_granted);
    assert!(!report.interconnection_approval_granted);
    assert!(!report.field_qualification_granted);
    assert!(!report.dispatch_authority_granted);
}
