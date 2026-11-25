// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_mobility_policy_tournament::{
    MobilityPolicyTournamentInput, evaluate_mobility_policy_tournament,
};

fn fixture() -> MobilityPolicyTournamentInput {
    serde_json::from_str(include_str!("../fixtures/mobility_policy_tournament_v0_1.json"))
        .expect("frozen matched-policy fixture must parse")
}

#[test]
fn frozen_three_policy_four_cluster_tournament_emits_paired_results() {
    let report = evaluate_mobility_policy_tournament(&fixture()).unwrap();
    assert_eq!(report.matched_pair_count, 4);
    assert_eq!(report.independent_cluster_count, 4);
    assert_eq!(report.policies.len(), 3);
    assert_eq!(report.comparisons.len(), 2);
    assert!(report.all_policies_hard_gate_clean);
    assert!(report.all_policies_false_safe_free);
    assert!(report.policies.iter().all(|policy| {
        policy.information_boundary_clean == Some(true)
            && policy.decision_mapping_conflict_count == 0
    }));
    let mpc = report.comparisons.iter().find(|entry| entry.policy_id == "mpc").unwrap();
    assert_eq!(mpc.on_time_service_rate_basis_points.mean_delta, Some(1000.0));
    assert_eq!(mpc.signed_metered_grid_response_wh_per_run.mean_delta, Some(40.0));
    assert!(mpc.on_time_service_rate_basis_points.interval_available);
    assert_eq!(report.claim_ceiling, "Matched outcome evaluation over the supplied finite scenario/replay corpus only. This report does not implement or endorse an optimizer, prove physical performance, authenticate source artifacts, establish causal effects, grant market eligibility, authorize dispatch, or establish AC feasibility/ELCC. Confidence intervals describe deterministic cluster-bootstrap resampling of the supplied independent-cluster labels; they are not distribution-free field guarantees.");
}
