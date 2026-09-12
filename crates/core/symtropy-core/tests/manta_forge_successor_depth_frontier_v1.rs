include!("manta_forge_viability_frontier_v1.rs");

const DEPTH_FIXTURE: &str = include_str!("../fixtures/manta-forge-successor-depth-frontier-v1.txt");

#[derive(Debug, Clone, PartialEq, Eq)]
struct SuccessorDepthCase {
    recovery_tick: u64,
    tooling_stock: u64,
    expected_successor_horizon: Option<u64>,
}

fn depth_cases() -> Vec<SuccessorDepthCase> {
    DEPTH_FIXTURE
        .lines()
        .filter_map(|line| line.strip_prefix("case="))
        .map(|line| {
            let mut recovery_tick = None;
            let mut tooling_stock = None;
            let mut expected_successor_horizon = None;
            for field in line.split('|') {
                let (key, value) = field.split_once(':').unwrap();
                match key {
                    "recovery_tick" => recovery_tick = Some(value.parse().unwrap()),
                    "tooling_stock" => tooling_stock = Some(value.parse().unwrap()),
                    "expected_successor_horizon" => {
                        expected_successor_horizon = Some(parse_window(value))
                    }
                    other => panic!("unknown successor-depth field {other}"),
                }
            }
            SuccessorDepthCase {
                recovery_tick: recovery_tick.unwrap(),
                tooling_stock: tooling_stock.unwrap(),
                expected_successor_horizon: expected_successor_horizon.unwrap(),
            }
        })
        .collect()
}

fn v3_lineage_spec(case: &FrontierCase) -> IndustrialEpochSpec {
    IndustrialEpochSpec {
        epoch_id: "manta-v3-frontier".into(),
        evidence_binding: format!(
            "epoch:manta-v3-frontier:r{}-t{}",
            case.recovery_tick, case.tooling_stock
        ),
        dependencies: vec![
            dependency(
                "forge-tooling-v3",
                IndustrialGovernance::Ordinary,
                0,
                0,
                0,
            ),
            dependency(
                "local-controller-v3",
                IndustrialGovernance::Ordinary,
                1,
                0,
                0,
            ),
            dependency(
                "metrology-v3",
                IndustrialGovernance::Ordinary,
                1,
                0,
                0,
            ),
            dependency(
                "reactor-service-v3",
                IndustrialGovernance::SafeguardedExternal,
                0,
                0,
                0,
            ),
            dependency(
                "structural-stock-v3",
                IndustrialGovernance::Ordinary,
                0,
                1,
                0,
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

fn simulate_successor_horizon(case: &FrontierCase) -> Option<u64> {
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

            let (mut successor, receipt) = state
                .handoff_to(
                    v3_lineage_spec(case),
                    handoff_plan(case, tooling_units, reactor_units, structural_units),
                )
                .unwrap();
            assert_eq!(receipt.source_final_observation_tick, Some(expected_tick));

            let mut survived_ticks = 0u64;
            for _ in 0..16 {
                let report = successor.step().unwrap();
                if !report.essential_capabilities_available {
                    return Some(survived_ticks);
                }
                survived_ticks += 1;
            }
            panic!("successor lineage did not reach its finite frontier within 16 ticks");
        }
    }
    None
}

#[test]
fn successor_depth_fixture_matches_dynamic_lineage_horizons() {
    let cases = cases();
    let depth = depth_cases();
    assert_eq!(cases.len(), 42);
    assert_eq!(depth.len(), 42);

    let mut exact_horizon_counts = [0usize; 5];
    for (case, expected) in cases.iter().zip(&depth) {
        assert_eq!(case.recovery_tick, expected.recovery_tick);
        assert_eq!(case.tooling_stock, expected.tooling_stock);
        let observed = simulate_successor_horizon(case);
        assert_eq!(
            observed, expected.expected_successor_horizon,
            "successor-horizon mismatch for {case:?}"
        );
        if let Some(horizon) = observed {
            exact_horizon_counts[horizon as usize] += 1;
        }
    }

    assert_eq!(exact_horizon_counts[1], 6);
    assert_eq!(exact_horizon_counts[2], 5);
    assert_eq!(exact_horizon_counts[3], 4);
    assert_eq!(exact_horizon_counts[4], 3);
    assert_eq!(exact_horizon_counts[1..].iter().sum::<usize>(), 18);

    let at_least_two = exact_horizon_counts[2..].iter().sum::<usize>();
    let at_least_three = exact_horizon_counts[3..].iter().sum::<usize>();
    let at_least_four = exact_horizon_counts[4..].iter().sum::<usize>();
    assert_eq!(at_least_two, 12);
    assert_eq!(at_least_three, 7);
    assert_eq!(at_least_four, 3);
}
