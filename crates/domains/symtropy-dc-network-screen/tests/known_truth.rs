// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::collections::BTreeMap;

use symtropy_dc_network_screen::{
    DcBranch, DcBus, DcFlexInterval, DcFlexibilityIdentity, DcNetworkContract, DcNetworkError,
    DcNetworkModel, evaluate_dc_network_deliverability,
};

fn network(branches: Vec<DcBranch>) -> DcNetworkModel {
    network_with_base_mva(branches, 1.0)
}

fn network_with_base_mva(branches: Vec<DcBranch>, base_mva: f64) -> DcNetworkModel {
    DcNetworkModel::new(
        "three-bus-v1",
        "sha256:model",
        "topology-a",
        "sha256:topology",
        0,
        base_mva,
        vec![
            DcBus::new("S", true).unwrap(),
            DcBus::new("A", false).unwrap(),
            DcBus::new("B", false).unwrap(),
        ],
        branches,
    )
    .unwrap()
}

fn triangle() -> DcNetworkModel {
    network(vec![
        DcBranch::new("S-A", "S", "A", 0.1, 100.0, 120.0).unwrap(),
        DcBranch::new("A-B", "A", "B", 0.1, 40.0, 60.0).unwrap(),
        DcBranch::new("S-B", "S", "B", 0.1, 45.0, 80.0).unwrap(),
    ])
}

fn identity() -> DcFlexibilityIdentity {
    DcFlexibilityIdentity::new(
        "dc-site",
        "pcc-b",
        "B",
        "sha256:baseline",
        "sha256:flexible",
    )
    .unwrap()
}

fn contract(require_n1: bool) -> DcNetworkContract {
    DcNetworkContract::new(10_000_000_000, 1.0e-9, 1.0e-12, require_n1).unwrap()
}

fn injections(a_mw: f64, b_mw: f64) -> BTreeMap<String, f64> {
    BTreeMap::from([("A".to_owned(), a_mw), ("B".to_owned(), b_mw)])
}

#[test]
fn hand_computed_triangle_flows_and_ptdf_shift_match() {
    let report = evaluate_dc_network_deliverability(
        &identity(),
        &triangle(),
        &contract(false),
        &[DcFlexInterval::new(
            0,
            3_600_000_000_000,
            injections(100.0, 0.0),
            BTreeMap::from([("B".to_owned(), 30.0)]),
        )
        .unwrap()],
        &[],
    )
    .unwrap();

    let flows = &report.intervals[0].branch_flows;
    let get = |id: &str| flows.iter().find(|f| f.branch_id == id).unwrap();
    assert!((get("A-B").baseline_flow_mw - 33.3333333333).abs() < 1e-8);
    assert!((get("A-B").flexible_flow_mw - 23.3333333333).abs() < 1e-8);
    assert!((get("A-B").flow_delta_mw + 10.0).abs() < 1e-8);
    assert!((get("S-A").flow_delta_mw + 10.0).abs() < 1e-8);
    assert!((get("S-B").flow_delta_mw + 20.0).abs() < 1e-8);
}

#[test]
fn flexibility_can_relieve_one_constraint_but_overload_another() {
    let report = evaluate_dc_network_deliverability(
        &identity(),
        &triangle(),
        &contract(false),
        &[DcFlexInterval::new(
            0,
            1_000,
            injections(100.0, 0.0),
            BTreeMap::from([("B".to_owned(), 30.0)]),
        )
        .unwrap()],
        &[],
    )
    .unwrap();
    let flows = &report.intervals[0].branch_flows;
    let ab = flows.iter().find(|f| f.branch_id == "A-B").unwrap();
    let sb = flows.iter().find(|f| f.branch_id == "S-B").unwrap();
    assert!(ab.absolute_flow_relief_mw > 0.0);
    assert!(sb.absolute_flow_relief_mw < 0.0);
    assert!(sb.flexible_over_limit);
    assert!(!report.network_deliverable);
    assert!(
        (report.intervals[0]
            .maximum_network_feasible_action_fraction
            .unwrap()
            - 0.5833333333)
            .abs()
            < 1e-8
    );
    assert!((report.intervals[0].maximum_network_deliverable_reduction_mw - 17.5).abs() < 1e-8);
}

#[test]
fn n_minus_one_outage_is_explicit_and_line_outage_can_island_network() {
    let radial = network(vec![
        DcBranch::new("S-A", "S", "A", 0.1, 100.0, 120.0).unwrap(),
        DcBranch::new("A-B", "A", "B", 0.1, 100.0, 120.0).unwrap(),
    ]);
    let report = evaluate_dc_network_deliverability(
        &identity(),
        &radial,
        &contract(true),
        &[DcFlexInterval::new(
            0,
            1_000,
            injections(0.0, 10.0),
            BTreeMap::from([("B".to_owned(), 5.0)]),
        )
        .unwrap()],
        &["A-B".to_owned()],
    )
    .unwrap();
    let outage = &report.intervals[0].n_minus_one[0];
    assert!(!outage.feasible);
    assert!(outage.islanded_or_singular);
    assert!(!report.network_deliverable);
}

#[test]
fn triangle_single_line_contingencies_are_recomputed_not_assumed() {
    let report = evaluate_dc_network_deliverability(
        &identity(),
        &triangle(),
        &contract(true),
        &[DcFlexInterval::new(
            0,
            1_000,
            injections(20.0, 0.0),
            BTreeMap::from([("B".to_owned(), 5.0)]),
        )
        .unwrap()],
        &["S-A".to_owned(), "A-B".to_owned(), "S-B".to_owned()],
    )
    .unwrap();
    assert_eq!(report.intervals[0].n_minus_one.len(), 3);
    assert!(
        report.intervals[0]
            .n_minus_one
            .iter()
            .all(|c| !c.islanded_or_singular)
    );
}

#[test]
fn stale_topology_is_rejected() {
    let model = DcNetworkModel::new(
        "model",
        "sha256:model",
        "topology",
        "sha256:topology",
        10_000,
        1.0,
        vec![
            DcBus::new("S", true).unwrap(),
            DcBus::new("A", false).unwrap(),
        ],
        vec![DcBranch::new("S-A", "S", "A", 0.1, 100.0, 120.0).unwrap()],
    )
    .unwrap();
    let error = evaluate_dc_network_deliverability(
        &identity(),
        &model,
        &contract(false),
        &[DcFlexInterval::new(0, 1_000, injections(0.0, 0.0), BTreeMap::new()).unwrap()],
        &[],
    )
    .unwrap_err();
    assert!(matches!(error, DcNetworkError::StaleTopology { .. }));
}

#[test]
fn stale_topology_by_age_is_rejected_even_when_interval_is_after_effective_time() {
    let model = DcNetworkModel::new(
        "model",
        "sha256:model",
        "topology",
        "sha256:topology",
        0,
        1.0,
        vec![
            DcBus::new("S", true).unwrap(),
            DcBus::new("A", false).unwrap(),
        ],
        vec![DcBranch::new("S-A", "S", "A", 0.1, 100.0, 120.0).unwrap()],
    )
    .unwrap();
    let error = evaluate_dc_network_deliverability(
        &identity(),
        &model,
        &DcNetworkContract::new(10, 1.0e-9, 1.0e-12, false).unwrap(),
        &[DcFlexInterval::new(100, 1_000, injections(0.0, 0.0), BTreeMap::new()).unwrap()],
        &[],
    )
    .unwrap_err();
    assert!(matches!(error, DcNetworkError::StaleTopology { .. }));
}

#[test]
fn unknown_slack_bus_injection_and_wrong_injection_bus_fail_closed() {
    let model = triangle();
    let mut slack_injection = BTreeMap::new();
    slack_injection.insert("S".to_owned(), -100.0);
    let error = evaluate_dc_network_deliverability(
        &identity(),
        &model,
        &contract(false),
        &[DcFlexInterval::new(0, 1_000, slack_injection, BTreeMap::new()).unwrap()],
        &[],
    )
    .unwrap_err();
    assert!(matches!(
        error,
        DcNetworkError::SlackBusInjectionSupplied { .. }
    ));

    let error = evaluate_dc_network_deliverability(
        &identity(),
        &model,
        &contract(false),
        &[DcFlexInterval::new(
            0,
            1_000,
            BTreeMap::from([("UNKNOWN".to_owned(), 10.0)]),
            BTreeMap::new(),
        )
        .unwrap()],
        &[],
    )
    .unwrap_err();
    assert!(matches!(error, DcNetworkError::UnknownInjectionBus { .. }));
}

#[test]
fn required_n_minus_one_policy_needs_explicit_contingency_set() {
    let error = evaluate_dc_network_deliverability(
        &identity(),
        &triangle(),
        &contract(true),
        &[DcFlexInterval::new(0, 1_000, injections(0.0, 0.0), BTreeMap::new()).unwrap()],
        &[],
    )
    .unwrap_err();
    assert!(matches!(
        error,
        DcNetworkError::MissingRequiredContingencies
    ));
}

#[test]
fn non_contiguous_chronology_and_unknown_outage_are_rejected() {
    let intervals = [
        DcFlexInterval::new(0, 100, injections(0.0, 0.0), BTreeMap::new()).unwrap(),
        DcFlexInterval::new(200, 100, injections(0.0, 0.0), BTreeMap::new()).unwrap(),
    ];
    let error = evaluate_dc_network_deliverability(
        &identity(),
        &triangle(),
        &contract(false),
        &intervals,
        &[],
    )
    .unwrap_err();
    assert!(matches!(
        error,
        DcNetworkError::NonContiguousIntervals { .. }
    ));

    let error = evaluate_dc_network_deliverability(
        &identity(),
        &triangle(),
        &contract(true),
        &[DcFlexInterval::new(0, 100, injections(0.0, 0.0), BTreeMap::new()).unwrap()],
        &["missing-branch".to_owned()],
    )
    .unwrap_err();
    assert!(matches!(
        error,
        DcNetworkError::UnknownContingencyBranch { .. }
    ));
}

#[test]
fn negative_load_reduction_is_rejected_instead_of_changing_sign_silently() {
    let error = DcFlexInterval::new(
        0,
        1_000,
        injections(0.0, 0.0),
        BTreeMap::from([("B".to_owned(), -5.0)]),
    )
    .unwrap_err();
    assert!(matches!(error, DcNetworkError::InvalidLoadReduction { .. }));
}

#[test]
fn maximum_deliverable_fraction_is_one_when_no_branch_binds() {
    let model = network(vec![
        DcBranch::new("S-A", "S", "A", 0.1, 200.0, 250.0).unwrap(),
        DcBranch::new("A-B", "A", "B", 0.1, 200.0, 250.0).unwrap(),
        DcBranch::new("S-B", "S", "B", 0.1, 200.0, 250.0).unwrap(),
    ]);
    let report = evaluate_dc_network_deliverability(
        &identity(),
        &model,
        &contract(false),
        &[DcFlexInterval::new(
            0,
            1_000,
            injections(10.0, 0.0),
            BTreeMap::from([("B".to_owned(), 5.0)]),
        )
        .unwrap()],
        &[],
    )
    .unwrap();
    assert!(report.network_deliverable);
    assert!(
        (report.intervals[0]
            .maximum_network_feasible_action_fraction
            .unwrap()
            - 1.0)
            .abs()
            < 1e-12
    );
    assert!((report.intervals[0].maximum_network_deliverable_reduction_mw - 5.0).abs() < 1e-12);
}

#[test]
fn explicit_mva_base_preserves_mw_flow_units() {
    let model_1mva = triangle();
    let model_1000mva = DcNetworkModel::new(
        "three-bus-1000mva",
        "sha256:model-1000",
        "topology-a",
        "sha256:topology",
        0,
        1_000.0,
        vec![
            DcBus::new("S", true).unwrap(),
            DcBus::new("A", false).unwrap(),
            DcBus::new("B", false).unwrap(),
        ],
        vec![
            DcBranch::new("S-A", "S", "A", 0.1, 100.0, 120.0).unwrap(),
            DcBranch::new("A-B", "A", "B", 0.1, 40.0, 60.0).unwrap(),
            DcBranch::new("S-B", "S", "B", 0.1, 45.0, 80.0).unwrap(),
        ],
    )
    .unwrap();
    let interval = DcFlexInterval::new(
        0,
        1_000,
        injections(100.0, 0.0),
        BTreeMap::from([("B".to_owned(), 30.0)]),
    )
    .unwrap();
    let report_1mva = evaluate_dc_network_deliverability(
        &identity(),
        &model_1mva,
        &contract(false),
        std::slice::from_ref(&interval),
        &[],
    )
    .unwrap();
    let report_1000mva = evaluate_dc_network_deliverability(
        &identity(),
        &model_1000mva,
        &contract(false),
        &[interval],
        &[],
    )
    .unwrap();
    for branch_id in ["S-A", "A-B", "S-B"] {
        let a = report_1mva.intervals[0]
            .branch_flows
            .iter()
            .find(|f| f.branch_id == branch_id)
            .unwrap();
        let b = report_1000mva.intervals[0]
            .branch_flows
            .iter()
            .find(|f| f.branch_id == branch_id)
            .unwrap();
        assert!((a.flexible_flow_mw - b.flexible_flow_mw).abs() < 1e-10);
    }
}

#[test]
fn pcc_load_reduction_must_be_applied_at_the_pinned_bus() {
    let error = evaluate_dc_network_deliverability(
        &identity(),
        &triangle(),
        &contract(false),
        &[DcFlexInterval::new(
            0,
            1_000,
            injections(0.0, 0.0),
            BTreeMap::from([("A".to_owned(), 5.0)]),
        )
        .unwrap()],
        &[],
    )
    .unwrap_err();
    assert!(matches!(error, DcNetworkError::PccBusMismatch { .. }));
}

#[test]
fn pcc_identity_must_resolve_to_a_non_slack_bus_in_the_topology() {
    let unknown_pcc = DcFlexibilityIdentity::new(
        "site",
        "pcc",
        "missing-bus",
        "sha256:baseline",
        "sha256:flex",
    )
    .unwrap();
    let interval = DcFlexInterval::new(0, 1_000, injections(0.0, 0.0), BTreeMap::new()).unwrap();
    let error = evaluate_dc_network_deliverability(
        &unknown_pcc,
        &triangle(),
        &contract(false),
        std::slice::from_ref(&interval),
        &[],
    )
    .unwrap_err();
    assert!(matches!(error, DcNetworkError::UnknownBus { .. }));

    let slack_pcc =
        DcFlexibilityIdentity::new("site", "pcc", "S", "sha256:baseline", "sha256:flex").unwrap();
    let error = evaluate_dc_network_deliverability(
        &slack_pcc,
        &triangle(),
        &contract(false),
        &[interval],
        &[],
    )
    .unwrap_err();
    assert!(matches!(error, DcNetworkError::PccBusMapsToSlack { .. }));
}

#[test]
fn baseline_overload_can_require_a_nonzero_minimum_dispatch_fraction() {
    let report = evaluate_dc_network_deliverability(
        &identity(),
        &triangle(),
        &contract(false),
        &[DcFlexInterval::new(
            0,
            1_000,
            injections(0.0, -100.0),
            BTreeMap::from([("B".to_owned(), 60.0)]),
        )
        .unwrap()],
        &[],
    )
    .unwrap();
    let interval = &report.intervals[0];
    assert!(
        interval
            .branch_flows
            .iter()
            .find(|flow| flow.branch_id == "S-B")
            .unwrap()
            .baseline_over_limit
    );
    assert!(interval.basecase_flexible_feasible);
    assert!(
        (interval.minimum_network_feasible_action_fraction.unwrap() - 0.5416666667).abs() < 1e-8
    );
    assert!((interval.maximum_network_feasible_action_fraction.unwrap() - 1.0).abs() < 1e-12);
    assert!(report.network_deliverable);
}

#[test]
fn per_unit_flow_results_are_invariant_to_declared_mva_base() {
    let branches = || {
        vec![
            DcBranch::new("S-A", "S", "A", 0.1, 100.0, 120.0).unwrap(),
            DcBranch::new("A-B", "A", "B", 0.1, 100.0, 120.0).unwrap(),
            DcBranch::new("S-B", "S", "B", 0.1, 100.0, 120.0).unwrap(),
        ]
    };
    let interval = DcFlexInterval::new(
        0,
        3_600_000_000_000,
        injections(100.0, 0.0),
        BTreeMap::from([("B".to_owned(), 30.0)]),
    )
    .unwrap();
    let one_mva = evaluate_dc_network_deliverability(
        &identity(),
        &network_with_base_mva(branches(), 1.0),
        &contract(false),
        std::slice::from_ref(&interval),
        &[],
    )
    .unwrap();
    let hundred_mva = evaluate_dc_network_deliverability(
        &identity(),
        &network_with_base_mva(branches(), 100.0),
        &contract(false),
        &[interval],
        &[],
    )
    .unwrap();
    for branch_id in ["S-A", "A-B", "S-B"] {
        let a = one_mva.intervals[0]
            .branch_flows
            .iter()
            .find(|x| x.branch_id == branch_id)
            .unwrap();
        let b = hundred_mva.intervals[0]
            .branch_flows
            .iter()
            .find(|x| x.branch_id == branch_id)
            .unwrap();
        assert!((a.baseline_flow_mw - b.baseline_flow_mw).abs() < 1e-9);
        assert!((a.flexible_flow_mw - b.flexible_flow_mw).abs() < 1e-9);
    }
    assert_eq!(hundred_mva.base_mva, 100.0);
}

#[test]
fn energy_summaries_use_mw_times_hours_without_extra_scale_factor() {
    let report = evaluate_dc_network_deliverability(
        &identity(),
        &triangle(),
        &contract(false),
        &[DcFlexInterval::new(
            0,
            3_600_000_000_000,
            injections(100.0, 0.0),
            BTreeMap::from([("B".to_owned(), 30.0)]),
        )
        .unwrap()],
        &[],
    )
    .unwrap();
    assert!((report.requested_load_reduction_energy_mwh - 30.0).abs() < 1e-9);
    assert!((report.maximum_network_deliverable_reduction_energy_mwh - 17.5).abs() < 1e-8);
}

#[test]
fn topology_must_remain_fresh_until_interval_end() {
    let model = DcNetworkModel::new(
        "model",
        "sha256:model",
        "topology",
        "sha256:topology",
        0,
        100.0,
        vec![
            DcBus::new("S", true).unwrap(),
            DcBus::new("A", false).unwrap(),
        ],
        vec![DcBranch::new("S-A", "S", "A", 0.1, 100.0, 120.0).unwrap()],
    )
    .unwrap();
    let policy = DcNetworkContract::new(10_000, 1e-9, 1e-12, false).unwrap();
    let pcc_identity = DcFlexibilityIdentity::new(
        "dc-site",
        "pcc-a",
        "A",
        "sha256:baseline",
        "sha256:flexible",
    )
    .unwrap();
    let error = evaluate_dc_network_deliverability(
        &pcc_identity,
        &model,
        &policy,
        &[DcFlexInterval::new(9_990, 20, BTreeMap::new(), BTreeMap::new()).unwrap()],
        &[],
    )
    .unwrap_err();
    assert!(matches!(
        error,
        DcNetworkError::StaleTopology {
            interval_start_ns: 10_010,
            ..
        }
    ));
}
