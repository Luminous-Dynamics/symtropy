include!("manta_forge_policy_surface_topology_adversary_v1.rs");

const RECOVERY_FIXTURE: &str =
    include_str!("../fixtures/manta-forge-qualified-recovery-adversary-v1.txt");

fn recovery_scalar(key: &str) -> u64 {
    RECOVERY_FIXTURE
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("missing recovery fixture scalar {key}"))
        .parse()
        .unwrap()
}

fn recovery_source_spec(case: &FrontierCase) -> IndustrialEpochSpec {
    let mut spec = topology_v3_spec(case, ControllerSupportTopology::DirectReactor);
    spec.dependencies.push(basis_dependency(
        "repair-reserve-v3",
        IndustrialGovernance::Ordinary,
        1,
        0,
        0,
        recovery_scalar("source_repair_reserve_inventory_units"),
    ));
    spec
}

fn recovery_successor_spec(case: &FrontierCase) -> IndustrialEpochSpec {
    let mut spec = topology_descendant_spec(
        case,
        4,
        ControllerSupportTopology::DirectReactor,
    );
    spec.dependencies.push(basis_dependency(
        "repair-reserve-v4",
        IndustrialGovernance::Ordinary,
        1,
        0,
        0,
        0,
    ));
    spec
}

fn recovery_handoff_plan(
    case: &FrontierCase,
    state: &IndustrialEpochState,
) -> IndustrialEpochHandoffPlan {
    let tooling_units = state.dependency("forge-tooling-v3").unwrap().inventory_units;
    let reactor_units = state
        .dependency("reactor-service-v3")
        .unwrap()
        .inventory_units;
    let structural_units = state
        .dependency("structural-stock-v3")
        .unwrap()
        .inventory_units;
    let controller_support_units = state
        .dependency("controller-support-v3")
        .unwrap()
        .inventory_units;
    let repair_reserve_units = state
        .dependency("repair-reserve-v3")
        .unwrap()
        .inventory_units;

    let mut plan = topology_handoff_plan(
        case,
        3,
        4,
        tooling_units,
        reactor_units,
        structural_units,
        controller_support_units,
    );
    plan.source_inventory_dispositions
        .push(IndustrialSourceInventoryDisposition {
            source_dependency_id: "repair-reserve-v3".into(),
            successor_dependency_id: Some("repair-reserve-v4".into()),
            transferred_units: repair_reserve_units,
            retired_units: 0,
        });
    plan.source_inventory_dispositions
        .sort_by(|left, right| left.source_dependency_id.cmp(&right.source_dependency_id));
    plan
}

fn fresh_recovery_successor(case: &FrontierCase) -> IndustrialEpochState {
    let mut source = IndustrialEpochState::from_spec(recovery_source_spec(case)).unwrap();
    for expected_tick in 1..=recovery_scalar("source_periods_before_handoff") {
        let report = source.step().unwrap();
        assert_eq!(report.tick, expected_tick);
        assert!(report.essential_capabilities_available);
    }
    assert_eq!(
        source.dependency("forge-tooling-v3").unwrap().inventory_units,
        recovery_scalar("successor_tooling_inventory_units")
    );
    assert_eq!(
        source
            .dependency("reactor-service-v3")
            .unwrap()
            .inventory_units,
        recovery_scalar("successor_reactor_inventory_units")
    );
    assert_eq!(
        source
            .dependency("repair-reserve-v3")
            .unwrap()
            .inventory_units,
        recovery_scalar("successor_repair_reserve_inventory_units")
    );

    let plan = recovery_handoff_plan(case, &source);
    let (successor, receipt) = source
        .handoff_to(recovery_successor_spec(case), plan)
        .unwrap();
    assert_eq!(
        receipt.source_final_observation_tick,
        Some(recovery_scalar("source_periods_before_handoff"))
    );
    successor
}

#[test]
fn nominal_h4_state_and_support_graph_are_identical_before_recovery_contract_use() {
    let case = h4_case();
    let source_horizon = straight_horizon(
        IndustrialEpochState::from_spec(recovery_source_spec(&case)).unwrap(),
    );
    assert_eq!(source_horizon, recovery_scalar("source_horizon_periods"));

    let recoverable = fresh_recovery_successor(&case);
    let unrecoverable = fresh_recovery_successor(&case);
    assert_eq!(recoverable, unrecoverable);
}

#[test]
fn equal_h4_basis_closure_and_state_can_diverge_under_same_disturbance_by_recoverability() {
    let case = h4_case();
    let successor_spec = recovery_successor_spec(&case);
    let contract = qualify_industrial_recovery_contract(
        &successor_spec,
        "recover-metrology-v4",
        "recovery:metrology-v4:qualified-v1",
        "metrology-v4",
        IndustrialFlowKind::Production,
        "repair-reserve-v4",
        recovery_scalar("recovery_reserve_units_per_recovery"),
    )
    .unwrap();
    assert_eq!(
        contract.qualified_units_per_tick(),
        recovery_scalar("qualified_recovery_units_per_tick")
    );

    let mut recoverable = fresh_recovery_successor(&case);
    let mut unrecoverable = fresh_recovery_successor(&case);
    assert_eq!(recoverable, unrecoverable);

    let disturbance = IndustrialShock::SetLocalProduction {
        dependency_id: "metrology-v4".into(),
        units_per_tick: recovery_scalar("shock_units_per_tick"),
    };
    recoverable.apply_shock(disturbance.clone()).unwrap();
    unrecoverable.apply_shock(disturbance).unwrap();
    assert_eq!(recoverable, unrecoverable);

    let unrecoverable_report = unrecoverable.step().unwrap();
    assert_eq!(
        unrecoverable_report.tick,
        recovery_scalar("unrecoverable_first_unavailable_tick")
    );
    assert!(!unrecoverable_report.essential_capabilities_available);
    assert!(unrecoverable_report
        .shortages
        .iter()
        .any(|shortage| shortage.dependency_id == "metrology-v4"));

    let receipt = execute_industrial_recovery(&mut recoverable, &contract).unwrap();
    assert_eq!(receipt.prior_units_per_tick, 0);
    assert_eq!(
        receipt.restored_units_per_tick,
        recovery_scalar("qualified_recovery_units_per_tick")
    );
    assert_eq!(receipt.reserve_units_before, 2);
    assert_eq!(receipt.reserve_units_after, 1);

    for expected_tick in 1..=recovery_scalar("maturity_periods") {
        let report = recoverable.step().unwrap();
        assert_eq!(report.tick, expected_tick);
        assert!(
            report.essential_capabilities_available,
            "qualified recovery should preserve the controlled maturity interval"
        );
    }
    assert_eq!(
        recoverable
            .dependency("forge-tooling-v4")
            .unwrap()
            .inventory_units,
        recovery_scalar("recoverable_terminal_tooling_inventory_units")
    );
    assert_eq!(
        recoverable
            .dependency("repair-reserve-v4")
            .unwrap()
            .inventory_units,
        recovery_scalar("recoverable_terminal_repair_reserve_inventory_units")
    );
}
