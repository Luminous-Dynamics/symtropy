// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_grid_causal_design::{GridCausalDesignInput, audit_causal_design};

fn fixture() -> GridCausalDesignInput {
    serde_json::from_str(include_str!("../fixtures/grid_causal_design_v0_1.json"))
        .expect("frozen cluster-randomization fixture must parse")
}

#[test]
fn frozen_cluster_randomization_plan_is_reproducible_and_never_estimates_an_effect() {
    let report = audit_causal_design(&fixture()).unwrap();
    assert!(report.design_qualified);
    assert_eq!(report.cluster_count, 8);
    assert_eq!(report.site_count, 8);
    assert_eq!(report.asset_count, 8);
    assert_eq!(report.treated_cluster_count, 4);
    assert_eq!(report.control_cluster_count, 4);
    assert_eq!(report.stratum_count, 2);
    assert!(report.randomization_reproducible);
    assert!(report.pre_registration_precedes_randomization);
    assert!(report.public_randomness_published_after_freeze);
    assert!(report.interference_domains_are_exclusive);
    assert!(report.pretrend_windows_predeclared);
    assert!(report.placebo_windows_predeclared);
    assert!(report.negative_control_outcomes_predeclared);
    assert!(report.cluster_level_inference_required);
    assert!(!report.external_randomness_source_authenticated);
    assert!(!report.causal_effect_estimated);
    assert!(!report.treatment_on_treated_identified);
    assert!(!report.settlement_authority_granted);
    assert!(!report.payment_authorized);
    assert!(!report.dispatch_authority_granted);
}
