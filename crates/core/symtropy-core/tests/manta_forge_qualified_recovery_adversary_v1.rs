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

fn recovery_text(key: &str) -> &str {
    RECOVERY_FIXTURE
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("missing recovery fixture text {key}"))
}

fn recovery_source_spec(case: &FrontierCase) -> IndustrialEpochSpec {
    // Deliberately exact #840 direct-topology source subject: recovery adds no
    // nominal dependency and therefore cannot change H, basis or support topology.
    topology_v3_spec(case, ControllerSupportTopology::DirectReactor)
}

fn recovery_successor_spec(case: &FrontierCase) -> IndustrialEpochSpec {
    // Deliberately exact #840 direct-topology descendant template.
    topology_descendant_spec(case, 4, ControllerSupportTopology::DirectReactor)
}

fn recovery_handoff_plan(
    case: &FrontierCase,
    state: &IndustrialEpochState,
) -> IndustrialEpochHandoffPlan {
    topology_handoff_plan(
        case,
        3,
        4,
        state.dependency("forge-tooling-v3").unwrap().inventory_units,
        state
            .dependency("reactor-service-v3")
            .unwrap()
            .inventory_units,
        state
            .dependency("structural-stock-v3")
            .unwrap()
            .inventory_units,
        state
            .dependency("controller-support-v3")
            .unwrap()
            .inventory_units,
    )
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

fn recovery_reserve() -> IndustrialRecoveryReserveState {
    IndustrialRecoveryReserveState::new(
        recovery_text("external_recovery_reserve_id"),
        recovery_text("external_recovery_reserve_binding"),
        recovery_scalar("external_recovery_reserve_units"),
    )
    .unwrap()
}

#[test]
fn nominal_h4_model_and_successor_state_remain_exact_without_recovery_metadata() {
    let case = h4_case();
    let source_horizon = straight_horizon(
        IndustrialEpochState::from_spec(recovery_source_spec(&case)).unwrap(),
    );
    assert_eq!(source_horizon, recovery_scalar("source_horizon_periods"));

    // The recovery source/successor specs are exactly the #840 nominal subjects.
    assert_eq!(
        recovery_source_spec(&case),
        topology_v3_spec(&case, ControllerSupportTopology::DirectReactor)
    );
    assert_eq!(
        recovery_successor_spec(&case),
        topology_descendant_spec(&case, 4, ControllerSupportTopology::DirectReactor)
    );

    let recoverable = fresh_recovery_successor(&case);
    let unrecoverable = fresh_recovery_successor(&case);
    assert_eq!(recoverable, unrecoverable);
    assert_eq!(recovery_reserve(), recovery_reserve());
}

#[test]
fn exact_same_nominal_successor_diverges_under_same_disturbance_only_by_recovery_authority() {
    let case = h4_case();
    let successor_spec = recovery_successor_spec(&case);
    let mut recoverable_reserve = recovery_reserve();
    let unrecoverable_reserve = recovery_reserve();
    let contract = qualify_industrial_recovery_contract(
        &successor_spec,
        &recoverable_reserve,
        "recover-metrology-v4",
        "recovery:metrology-v4:qualified-v1",
        recovery_text("disturbance_target"),
        IndustrialFlowKind::Production,
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
    assert_eq!(recoverable_reserve, unrecoverable_reserve);

    let disturbance = IndustrialShock::SetLocalProduction {
        dependency_id: recovery_text("disturbance_target").into(),
        units_per_tick: recovery_scalar("degraded_units_per_tick"),
    };
    recoverable.apply_shock(disturbance.clone()).unwrap();
    unrecoverable.apply_shock(disturbance).unwrap();
    assert_eq!(recoverable, unrecoverable);
    assert_eq!(recoverable_reserve, unrecoverable_reserve);

    let unrecoverable_report = unrecoverable.step().unwrap();
    assert_eq!(
        unrecoverable_report.tick,
        recovery_scalar("unrecoverable_first_unavailable_tick")
    );
    assert!(!unrecoverable_report.essential_capabilities_available);
    assert!(unrecoverable_report
        .shortages
        .iter()
        .any(|shortage| shortage.dependency_id == recovery_text("disturbance_target")));

    let receipt = execute_industrial_recovery(
        &mut recoverable,
        &contract,
        &mut recoverable_reserve,
    )
    .unwrap();
    assert_eq!(receipt.prior_units_per_tick, 0);
    assert_eq!(
        receipt.restored_units_per_tick,
        recovery_scalar("qualified_recovery_units_per_tick")
    );
    assert_eq!(
        receipt.reserve_units_before,
        recovery_scalar("external_recovery_reserve_units")
    );
    assert_eq!(
        receipt.reserve_units_after,
        recovery_scalar("recoverable_terminal_recovery_reserve_units")
    );

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
        recoverable_reserve.available_units(),
        recovery_scalar("recoverable_terminal_recovery_reserve_units")
    );

    // The control had the same reserve quantity but no qualified authority to use it.
    assert_eq!(
        unrecoverable_reserve.available_units(),
        recovery_scalar("external_recovery_reserve_units")
    );
}
