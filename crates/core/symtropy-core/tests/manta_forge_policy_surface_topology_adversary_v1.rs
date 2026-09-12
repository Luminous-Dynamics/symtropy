include!("manta_forge_policy_surface_support_basis_adversary_v1.rs");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControllerSupportTopology {
    DirectReactor,
    FiniteHiddenSupport,
}

fn topology_flow_model(
    generation: u64,
    topology: ControllerSupportTopology,
) -> IndustrialEpochFlowModel {
    let controller_prerequisite = match topology {
        ControllerSupportTopology::DirectReactor => format!("reactor-service-v{generation}"),
        ControllerSupportTopology::FiniteHiddenSupport => {
            format!("controller-support-v{generation}")
        }
    };
    IndustrialEpochFlowModel::PrerequisiteGated(vec![
        prerequisite(
            &format!("local-controller-v{generation}"),
            IndustrialFlowKind::Production,
            &controller_prerequisite,
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
    ])
}

fn topology_v3_spec(
    case: &FrontierCase,
    topology: ControllerSupportTopology,
) -> IndustrialEpochSpec {
    let mut spec = basis_v3_spec(case, 1, 4, 1, 100);
    spec.dependencies.push(basis_dependency(
        "controller-support-v3",
        IndustrialGovernance::Ordinary,
        1,
        0,
        0,
        3,
    ));
    spec.flow_model = topology_flow_model(3, topology);
    spec
}

fn topology_descendant_spec(
    case: &FrontierCase,
    generation: u64,
    topology: ControllerSupportTopology,
) -> IndustrialEpochSpec {
    let mut spec = basis_descendant_spec(case, generation, 1, 1);
    spec.dependencies.push(basis_dependency(
        &format!("controller-support-v{generation}"),
        IndustrialGovernance::Ordinary,
        1,
        0,
        0,
        0,
    ));
    spec.flow_model = topology_flow_model(generation, topology);
    spec
}

fn topology_handoff_plan(
    case: &FrontierCase,
    source_generation: u64,
    successor_generation: u64,
    tooling_units: u64,
    reactor_units: u64,
    structural_units: u64,
    controller_support_units: u64,
) -> IndustrialEpochHandoffPlan {
    let mut plan = descendant_handoff_plan(
        case,
        source_generation,
        successor_generation,
        tooling_units,
        reactor_units,
        structural_units,
    );
    plan.source_inventory_dispositions
        .push(IndustrialSourceInventoryDisposition {
            source_dependency_id: format!("controller-support-v{source_generation}"),
            successor_dependency_id: None,
            transferred_units: 0,
            retired_units: controller_support_units,
        });
    plan.source_inventory_dispositions
        .sort_by(|left, right| left.source_dependency_id.cmp(&right.source_dependency_id));
    plan
}

fn simulate_topology_surface_point(
    case: &FrontierCase,
    maturity_periods: u64,
    topology: ControllerSupportTopology,
) -> DynamicPolicySurfacePoint {
    let mut state = IndustrialEpochState::from_spec(topology_v3_spec(case, topology)).unwrap();
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
        let controller_support_id = format!("controller-support-v{generation}");
        let tooling_units = state.dependency(&tooling_id).unwrap().inventory_units;
        let reactor_units = state.dependency(&reactor_id).unwrap().inventory_units;
        let structural_units = state.dependency(&structural_id).unwrap().inventory_units;
        let controller_support_units = state
            .dependency(&controller_support_id)
            .unwrap()
            .inventory_units;

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
                topology_descendant_spec(case, next_generation, topology),
                topology_handoff_plan(
                    case,
                    generation,
                    next_generation,
                    tooling_units,
                    reactor_units,
                    structural_units,
                    controller_support_units,
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

#[test]
fn fixed_state_vector_and_h4_can_hide_distinct_support_topology() {
    let case = h4_case();
    let direct = topology_v3_spec(&case, ControllerSupportTopology::DirectReactor);
    let hidden = topology_v3_spec(&case, ControllerSupportTopology::FiniteHiddenSupport);

    assert_eq!(direct.dependencies, hidden.dependencies);
    assert_eq!(direct.capabilities, hidden.capabilities);
    assert_ne!(direct.flow_model, hidden.flow_model);

    assert_eq!(
        straight_horizon(IndustrialEpochState::from_spec(direct).unwrap()),
        4
    );
    assert_eq!(
        straight_horizon(IndustrialEpochState::from_spec(hidden).unwrap()),
        4
    );
}

#[test]
fn basis_qualified_h4_is_not_sufficient_when_support_topology_is_not_preserved() {
    let case = h4_case();
    let canonical_h4 = expected_surface_points();
    let direct: Vec<_> = [1u64, 2, 3, 4]
        .into_iter()
        .map(|maturity| {
            simulate_topology_surface_point(
                &case,
                maturity,
                ControllerSupportTopology::DirectReactor,
            )
        })
        .collect();
    let hidden: Vec<_> = [1u64, 2, 3, 4]
        .into_iter()
        .map(|maturity| {
            simulate_topology_surface_point(
                &case,
                maturity,
                ControllerSupportTopology::FiniteHiddenSupport,
            )
        })
        .collect();

    assert_eq!(direct, canonical_h4);
    assert_ne!(hidden, canonical_h4);
    assert_eq!(
        hidden,
        vec![
            DynamicPolicySurfacePoint {
                maturity_periods: 1,
                founded_descendants: 4,
                maturity_completed_descendants: 4,
                descendant_reproduction_transitions: 3,
                terminal_residual_periods: 1,
            },
            DynamicPolicySurfacePoint {
                maturity_periods: 2,
                founded_descendants: 2,
                maturity_completed_descendants: 1,
                descendant_reproduction_transitions: 1,
                terminal_residual_periods: 1,
            },
            DynamicPolicySurfacePoint {
                maturity_periods: 3,
                founded_descendants: 2,
                maturity_completed_descendants: 1,
                descendant_reproduction_transitions: 1,
                terminal_residual_periods: 1,
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
