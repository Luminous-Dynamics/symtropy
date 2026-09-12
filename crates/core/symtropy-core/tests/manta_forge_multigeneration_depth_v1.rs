include!("manta_forge_successor_depth_frontier_v1.rs");

fn descendant_spec(case: &FrontierCase, generation: u64) -> IndustrialEpochSpec {
    IndustrialEpochSpec {
        epoch_id: format!("manta-v{generation}-frontier"),
        evidence_binding: format!(
            "epoch:manta-v{generation}-frontier:r{}-t{}",
            case.recovery_tick, case.tooling_stock
        ),
        dependencies: vec![
            dependency(
                &format!("forge-tooling-v{generation}"),
                IndustrialGovernance::Ordinary,
                0,
                0,
                0,
            ),
            dependency(
                &format!("local-controller-v{generation}"),
                IndustrialGovernance::Ordinary,
                1,
                0,
                0,
            ),
            dependency(
                &format!("metrology-v{generation}"),
                IndustrialGovernance::Ordinary,
                1,
                0,
                0,
            ),
            dependency(
                &format!("reactor-service-v{generation}"),
                IndustrialGovernance::SafeguardedExternal,
                0,
                0,
                0,
            ),
            dependency(
                &format!("structural-stock-v{generation}"),
                IndustrialGovernance::Ordinary,
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

fn descendant_handoff_plan(
    case: &FrontierCase,
    source_generation: u64,
    successor_generation: u64,
    tooling_units: u64,
    reactor_units: u64,
    structural_units: u64,
) -> IndustrialEpochHandoffPlan {
    IndustrialEpochHandoffPlan {
        handoff_id: format!(
            "handoff:frontier:r{}-t{}:v{}-v{}",
            case.recovery_tick,
            case.tooling_stock,
            source_generation,
            successor_generation
        ),
        evidence_binding: format!(
            "handoff-evidence:frontier:r{}-t{}:v{}-v{}",
            case.recovery_tick,
            case.tooling_stock,
            source_generation,
            successor_generation
        ),
        source_epoch_id: format!("manta-v{source_generation}-frontier"),
        source_epoch_evidence_binding: format!(
            "epoch:manta-v{source_generation}-frontier:r{}-t{}",
            case.recovery_tick, case.tooling_stock
        ),
        successor_epoch_id: format!("manta-v{successor_generation}-frontier"),
        successor_epoch_evidence_binding: format!(
            "epoch:manta-v{successor_generation}-frontier:r{}-t{}",
            case.recovery_tick, case.tooling_stock
        ),
        source_inventory_dispositions: vec![
            IndustrialSourceInventoryDisposition {
                source_dependency_id: format!("forge-tooling-v{source_generation}"),
                successor_dependency_id: Some(format!("forge-tooling-v{successor_generation}")),
                transferred_units: tooling_units,
                retired_units: 0,
            },
            IndustrialSourceInventoryDisposition {
                source_dependency_id: format!("local-controller-v{source_generation}"),
                successor_dependency_id: None,
                transferred_units: 0,
                retired_units: 0,
            },
            IndustrialSourceInventoryDisposition {
                source_dependency_id: format!("metrology-v{source_generation}"),
                successor_dependency_id: None,
                transferred_units: 0,
                retired_units: 0,
            },
            IndustrialSourceInventoryDisposition {
                source_dependency_id: format!("reactor-service-v{source_generation}"),
                successor_dependency_id: Some(format!("reactor-service-v{successor_generation}")),
                transferred_units: reactor_units,
                retired_units: 0,
            },
            IndustrialSourceInventoryDisposition {
                source_dependency_id: format!("structural-stock-v{source_generation}"),
                successor_dependency_id: Some(format!("structural-stock-v{successor_generation}")),
                transferred_units: structural_units,
                retired_units: 0,
            },
        ],
        external_inventory_admissions: Vec::new(),
    }
}

fn first_successor(case: &FrontierCase) -> Option<IndustrialEpochState> {
    let known = v2_capabilities();
    let watches = vec![
        IndustrialCapabilityWatch::new(
            "construction",
            BTreeSet::from([
                "construction-controller-v2".into(),
                "construction-tooling-v2".into(),
            ]),
            &known,
        )
        .unwrap(),
        IndustrialCapabilityWatch::new(
            "qualification",
            BTreeSet::from(["successor-qualification-v2".into()]),
            &known,
        )
        .unwrap(),
    ];

    let mut state = IndustrialEpochState::from_spec(v2_spec(case.tooling_stock)).unwrap();
    state
        .apply_shock(IndustrialShock::SetLocalProduction {
            dependency_id: "metrology-v2".into(),
            units_per_tick: 0,
        })
        .unwrap();

    for expected_tick in 1..=9 {
        if expected_tick == case.recovery_tick {
            state
                .apply_shock(IndustrialShock::SetLocalProduction {
                    dependency_id: "metrology-v2".into(),
                    units_per_tick: 1,
                })
                .unwrap();
        }

        let report = state.step().unwrap();
        let roles = assess_industrial_capability_watches(&report, &watches);
        let construction = roles
            .iter()
            .find(|role| role.watch_id == "construction")
            .unwrap();
        let qualification = roles
            .iter()
            .find(|role| role.watch_id == "qualification")
            .unwrap();
        if expected_tick >= case.recovery_tick && construction.available && qualification.available {
            let tooling_units = state
                .dependency("forge-tooling-v2")
                .unwrap()
                .inventory_units;
            let reactor_units = state
                .dependency("reactor-service-v2")
                .unwrap()
                .inventory_units;
            let structural_units = state
                .dependency("structural-stock-v2")
                .unwrap()
                .inventory_units;
            if tooling_units == 0 || reactor_units == 0 {
                return None;
            }
            let (successor, receipt) = state
                .handoff_to(
                    v3_lineage_spec(case),
                    handoff_plan(case, tooling_units, reactor_units, structural_units),
                )
                .unwrap();
            assert_eq!(receipt.source_final_observation_tick, Some(expected_tick));
            return Some(successor);
        }
    }
    None
}

fn simulate_generation_depth(case: &FrontierCase) -> (Option<u64>, u64) {
    let Some(mut state) = first_successor(case) else {
        return (None, 0);
    };

    let mut generation = 3u64;
    let mut descendant_generations = 1u64;
    let mut successful_handoffs = 1u64; // v2 -> v3

    loop {
        let report = state.step().unwrap();
        if !report.essential_capabilities_available {
            break;
        }

        let tooling_id = format!("forge-tooling-v{generation}");
        let reactor_id = format!("reactor-service-v{generation}");
        let structural_id = format!("structural-stock-v{generation}");
        let tooling_units = state.dependency(&tooling_id).unwrap().inventory_units;
        let reactor_units = state.dependency(&reactor_id).unwrap().inventory_units;
        let structural_units = state.dependency(&structural_id).unwrap().inventory_units;

        // The current descendant may have completed this tick successfully while
        // still consuming the last reserve required to found its own successor.
        if tooling_units == 0 || reactor_units == 0 {
            break;
        }

        let next_generation = generation + 1;
        let (successor, receipt) = state
            .handoff_to(
                descendant_spec(case, next_generation),
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
        assert_eq!(receipt.source_final_tick, 1);
        assert_eq!(receipt.source_final_observation_tick, Some(1));
        assert!(receipt.source_final_shortage_ids.is_empty());
        assert!(receipt.source_final_unavailable_capability_ids.is_empty());

        state = successor;
        generation = next_generation;
        descendant_generations += 1;
        successful_handoffs += 1;
    }

    (Some(descendant_generations), successful_handoffs)
}

#[test]
fn successor_runway_equals_linear_multigeneration_reproductive_depth() {
    let cases = cases();
    let depth = depth_cases();
    assert_eq!(cases.len(), 42);
    assert_eq!(depth.len(), 42);

    let mut generation_depth_counts = [0usize; 5];
    let mut total_successful_handoffs = 0u64;

    for (case, expected) in cases.iter().zip(&depth) {
        assert_eq!(case.recovery_tick, expected.recovery_tick);
        assert_eq!(case.tooling_stock, expected.tooling_stock);

        let (observed_depth, handoffs) = simulate_generation_depth(case);
        assert_eq!(
            observed_depth, expected.expected_successor_horizon,
            "multigeneration depth mismatch for {case:?}"
        );
        total_successful_handoffs += handoffs;
        if let Some(generation_depth) = observed_depth {
            generation_depth_counts[generation_depth as usize] += 1;
            assert_eq!(handoffs, generation_depth);
        } else {
            assert_eq!(handoffs, 0);
        }
    }

    assert_eq!(generation_depth_counts[1], 6);
    assert_eq!(generation_depth_counts[2], 5);
    assert_eq!(generation_depth_counts[3], 4);
    assert_eq!(generation_depth_counts[4], 3);
    assert_eq!(generation_depth_counts[1..].iter().sum::<usize>(), 18);

    // 18 initial v2->v3 handoffs plus 22 later descendant handoffs. Every one is
    // linear, conservation-checked, and consumes the predecessor epoch state.
    assert_eq!(total_successful_handoffs, 40);
}
