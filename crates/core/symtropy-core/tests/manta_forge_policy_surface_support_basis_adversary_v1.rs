include!("manta_forge_policy_surface_assumption_change_control_v1.rs");

fn basis_dependency(
    id: &str,
    governance: IndustrialGovernance,
    demand: u64,
    production: u64,
    recycling: u64,
    inventory: u64,
) -> IndustrialDependencyState {
    IndustrialDependencyState {
        dependency_id: id.into(),
        governance,
        demand_units_per_tick: demand,
        local_production_units_per_tick: production,
        recycling_units_per_tick: recycling,
        inventory_units: inventory,
    }
}

fn basis_v3_spec(
    case: &FrontierCase,
    tooling_demand: u64,
    tooling_inventory: u64,
    reactor_demand: u64,
    reactor_inventory: u64,
) -> IndustrialEpochSpec {
    IndustrialEpochSpec {
        epoch_id: "manta-v3-frontier".into(),
        evidence_binding: format!(
            "epoch:manta-v3-frontier:r{}-t{}",
            case.recovery_tick, case.tooling_stock
        ),
        dependencies: vec![
            basis_dependency(
                "forge-tooling-v3",
                IndustrialGovernance::Ordinary,
                tooling_demand,
                0,
                0,
                tooling_inventory,
            ),
            basis_dependency(
                "local-controller-v3",
                IndustrialGovernance::Ordinary,
                1,
                1,
                0,
                0,
            ),
            basis_dependency(
                "metrology-v3",
                IndustrialGovernance::Ordinary,
                1,
                1,
                0,
                0,
            ),
            basis_dependency(
                "reactor-service-v3",
                IndustrialGovernance::SafeguardedExternal,
                reactor_demand,
                0,
                0,
                reactor_inventory,
            ),
            basis_dependency(
                "structural-stock-v3",
                IndustrialGovernance::Ordinary,
                1,
                0,
                1,
                18,
            ),
        ],
        capabilities: vec![
            capability(
                "operation-v3",
                true,
                &["reactor-service-v3", "structural-stock-v3"],
            ),
            capability(
                "lineage-reproduction-v3",
                true,
                &[
                    "forge-tooling-v3",
                    "local-controller-v3",
                    "metrology-v3",
                    "reactor-service-v3",
                    "structural-stock-v3",
                ],
            ),
        ],
        flow_model: IndustrialEpochFlowModel::PrerequisiteGated(vec![
            prerequisite(
                "local-controller-v3",
                IndustrialFlowKind::Production,
                "metrology-v3",
            ),
            prerequisite(
                "metrology-v3",
                IndustrialFlowKind::Production,
                "reactor-service-v3",
            ),
            prerequisite(
                "structural-stock-v3",
                IndustrialFlowKind::Recycling,
                "forge-tooling-v3",
            ),
        ]),
    }
}

fn basis_descendant_spec(
    case: &FrontierCase,
    generation: u64,
    tooling_demand: u64,
    reactor_demand: u64,
) -> IndustrialEpochSpec {
    IndustrialEpochSpec {
        epoch_id: format!("manta-v{generation}-frontier"),
        evidence_binding: format!(
            "epoch:manta-v{generation}-frontier:r{}-t{}",
            case.recovery_tick, case.tooling_stock
        ),
        dependencies: vec![
            basis_dependency(
                &format!("forge-tooling-v{generation}"),
                IndustrialGovernance::Ordinary,
                tooling_demand,
                0,
                0,
                0,
            ),
            basis_dependency(
                &format!("local-controller-v{generation}"),
                IndustrialGovernance::Ordinary,
                1,
                1,
                0,
                0,
            ),
            basis_dependency(
                &format!("metrology-v{generation}"),
                IndustrialGovernance::Ordinary,
                1,
                1,
                0,
                0,
            ),
            basis_dependency(
                &format!("reactor-service-v{generation}"),
                IndustrialGovernance::SafeguardedExternal,
                reactor_demand,
                0,
                0,
                0,
            ),
            basis_dependency(
                &format!("structural-stock-v{generation}"),
                IndustrialGovernance::Ordinary,
                1,
                0,
                1,
                0,
            ),
        ],
        capabilities: vec![
            capability(
                &format!("operation-v{generation}"),
                true,
                &[
                    &format!("reactor-service-v{generation}"),
                    &format!("structural-stock-v{generation}"),
                ],
            ),
            capability(
                &format!("lineage-reproduction-v{generation}"),
                true,
                &[
                    &format!("forge-tooling-v{generation}"),
                    &format!("local-controller-v{generation}"),
                    &format!("metrology-v{generation}"),
                    &format!("reactor-service-v{generation}"),
                    &format!("structural-stock-v{generation}"),
                ],
            ),
        ],
        flow_model: IndustrialEpochFlowModel::PrerequisiteGated(vec![
            prerequisite(
                &format!("local-controller-v{generation}"),
                IndustrialFlowKind::Production,
                &format!("metrology-v{generation}"),
            ),
            prerequisite(
                &format!("metrology-v{generation}"),
                IndustrialFlowKind::Production,
                &format!("reactor-service-v{generation}"),
            ),
            prerequisite(
                &format!("structural-stock-v{generation}"),
                IndustrialFlowKind::Recycling,
                &format!("forge-tooling-v{generation}"),
            ),
        ]),
    }
}

fn straight_horizon(mut state: IndustrialEpochState) -> u64 {
    let mut survived = 0u64;
    loop {
        let report = state.step().unwrap();
        if !report.essential_capabilities_available {
            return survived;
        }
        survived += 1;
        assert!(survived <= 64, "adversarial state unexpectedly remained unbounded");
    }
}

fn simulate_basis_surface_point(
    case: &FrontierCase,
    maturity_periods: u64,
    source_tooling_demand: u64,
    source_tooling_inventory: u64,
    source_reactor_demand: u64,
    source_reactor_inventory: u64,
    descendant_tooling_demand: u64,
    descendant_reactor_demand: u64,
) -> DynamicPolicySurfacePoint {
    let mut state = IndustrialEpochState::from_spec(basis_v3_spec(
        case,
        source_tooling_demand,
        source_tooling_inventory,
        source_reactor_demand,
        source_reactor_inventory,
    ))
    .unwrap();
    let mut generation = 3u64;
    let mut founded_descendants = 1u64;
    let mut maturity_completed_descendants = 0u64;
    let mut descendant_reproduction_transitions = 0u64;

    loop {
        let mut terminal_residual_periods = 0u64;
        let mut matured = true;
        for expected_tick in 1..=maturity_periods {
            let report = state.step().unwrap();
            assert_eq!(report.tick, expected_tick);
            if !report.essential_capabilities_available {
                matured = false;
                break;
            }
            terminal_residual_periods += 1;
        }
        if !matured {
            return DynamicPolicySurfacePoint {
                maturity_periods,
                founded_descendants,
                maturity_completed_descendants,
                descendant_reproduction_transitions,
                terminal_residual_periods,
            };
        }
        maturity_completed_descendants += 1;

        let tooling_id = format!("forge-tooling-v{generation}");
        let reactor_id = format!("reactor-service-v{generation}");
        let structural_id = format!("structural-stock-v{generation}");
        let tooling_units = state.dependency(&tooling_id).unwrap().inventory_units;
        let reactor_units = state.dependency(&reactor_id).unwrap().inventory_units;
        let structural_units = state.dependency(&structural_id).unwrap().inventory_units;
        if tooling_units == 0 || reactor_units == 0 {
            return DynamicPolicySurfacePoint {
                maturity_periods,
                founded_descendants,
                maturity_completed_descendants,
                descendant_reproduction_transitions,
                terminal_residual_periods,
            };
        }

        let next_generation = generation + 1;
        let (successor, receipt) = state
            .handoff_to(
                basis_descendant_spec(
                    case,
                    next_generation,
                    descendant_tooling_demand,
                    descendant_reactor_demand,
                ),
                descendant_handoff_plan(
                    case,
                    generation,
                    next_generation,
                    tooling_units,
                    reactor_units,
                    structural_units,
                ),
            )
            .unwrap();
        assert_eq!(receipt.source_final_observation_tick, Some(maturity_periods));
        founded_descendants += 1;
        descendant_reproduction_transitions += 1;
        state = successor;
        generation = next_generation;
    }
}

fn h4_case() -> FrontierCase {
    let cases = cases();
    let depth = depth_cases();
    let index = depth
        .iter()
        .position(|entry| entry.expected_successor_horizon == Some(4))
        .expect("frontier must contain H=4");
    cases[index].clone()
}

#[test]
fn reserve_composition_can_change_without_changing_h4_surface_when_unit_basis_matches() {
    let case = h4_case();
    let expected = expected_surface_points();

    let tooling_limited_horizon = straight_horizon(
        IndustrialEpochState::from_spec(basis_v3_spec(&case, 1, 4, 1, 100)).unwrap(),
    );
    let reactor_limited_horizon = straight_horizon(
        IndustrialEpochState::from_spec(basis_v3_spec(&case, 1, 100, 1, 4)).unwrap(),
    );
    assert_eq!(tooling_limited_horizon, 4);
    assert_eq!(reactor_limited_horizon, 4);

    let tooling_limited: Vec<_> = [1u64, 2, 3, 4]
        .into_iter()
        .map(|maturity| simulate_basis_surface_point(&case, maturity, 1, 4, 1, 100, 1, 1))
        .collect();
    let reactor_limited: Vec<_> = [1u64, 2, 3, 4]
        .into_iter()
        .map(|maturity| simulate_basis_surface_point(&case, maturity, 1, 100, 1, 4, 1, 1))
        .collect();

    assert_eq!(tooling_limited, expected);
    assert_eq!(reactor_limited, expected);
}

#[test]
fn equal_h4_scalar_is_not_sufficient_when_handoff_changes_support_unit_basis() {
    let case = h4_case();
    let expected_h4 = expected_surface_points();

    // Eight tooling units at two units/period still provide exactly four complete
    // source-generation periods. The successor, however, interprets the transferred
    // residual quantity at one unit/period. That conversion changes descendant
    // runway even though the source scalar H is still four.
    let source_horizon = straight_horizon(
        IndustrialEpochState::from_spec(basis_v3_spec(&case, 2, 8, 1, 100)).unwrap(),
    );
    assert_eq!(source_horizon, 4);

    let changed_basis: Vec<_> = [1u64, 2, 3, 4]
        .into_iter()
        .map(|maturity| simulate_basis_surface_point(&case, maturity, 2, 8, 1, 100, 1, 1))
        .collect();

    assert_ne!(changed_basis, expected_h4);
    assert_eq!(
        changed_basis,
        vec![
            DynamicPolicySurfacePoint {
                maturity_periods: 1,
                founded_descendants: 7,
                maturity_completed_descendants: 7,
                descendant_reproduction_transitions: 6,
                terminal_residual_periods: 1,
            },
            DynamicPolicySurfacePoint {
                maturity_periods: 2,
                founded_descendants: 3,
                maturity_completed_descendants: 3,
                descendant_reproduction_transitions: 2,
                terminal_residual_periods: 2,
            },
            DynamicPolicySurfacePoint {
                maturity_periods: 3,
                founded_descendants: 2,
                maturity_completed_descendants: 1,
                descendant_reproduction_transitions: 1,
                terminal_residual_periods: 2,
            },
            DynamicPolicySurfacePoint {
                maturity_periods: 4,
                founded_descendants: 1,
                maturity_completed_descendants: 1,
                descendant_reproduction_transitions: 0,
                terminal_residual_periods: 4,
            },
        ]
    );
}

#[test]
fn preserving_intergenerational_support_basis_restores_h4_policy_projection() {
    let case = h4_case();
    let expected_h4 = expected_surface_points();
    let preserved_basis: Vec<_> = [1u64, 2, 3, 4]
        .into_iter()
        .map(|maturity| simulate_basis_surface_point(&case, maturity, 2, 8, 1, 100, 2, 1))
        .collect();
    assert_eq!(preserved_basis, expected_h4);
}
