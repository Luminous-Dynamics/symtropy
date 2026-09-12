use std::collections::BTreeSet;
use symtropy_core::industrial_ecology::{
    IndustrialCapability, IndustrialDependencyState, IndustrialFlowKind, IndustrialFlowPrerequisite,
    IndustrialGovernance, IndustrialShock,
};
use symtropy_core::industrial_epoch::{
    IndustrialEpochFlowModel, IndustrialEpochHandoffPlan, IndustrialEpochSpec,
    IndustrialEpochState, IndustrialSourceInventoryDisposition,
};
use symtropy_core::industrial_watch::{
    assess_industrial_capability_watches, IndustrialCapabilityWatch,
};

const FIXTURE: &str = include_str!("../fixtures/manta-forge-viability-frontier-v1.txt");

#[derive(Debug, Clone, PartialEq, Eq)]
struct FrontierCase {
    recovery_tick: u64,
    tooling_stock: u64,
    expected_role_window: Option<u64>,
    expected_handoff_window: Option<u64>,
}

fn parse_window(value: &str) -> Option<u64> {
    if value == "none" {
        None
    } else {
        Some(value.parse().unwrap())
    }
}

fn cases() -> Vec<FrontierCase> {
    FIXTURE
        .lines()
        .filter_map(|line| line.strip_prefix("case="))
        .map(|line| {
            let mut recovery_tick = None;
            let mut tooling_stock = None;
            let mut expected_role_window = None;
            let mut expected_handoff_window = None;
            for field in line.split('|') {
                let (key, value) = field.split_once(':').unwrap();
                match key {
                    "recovery_tick" => recovery_tick = Some(value.parse().unwrap()),
                    "tooling_stock" => tooling_stock = Some(value.parse().unwrap()),
                    "expected_role_window" => expected_role_window = Some(parse_window(value)),
                    "expected_handoff_window" => {
                        expected_handoff_window = Some(parse_window(value))
                    }
                    other => panic!("unknown frontier field {other}"),
                }
            }
            FrontierCase {
                recovery_tick: recovery_tick.unwrap(),
                tooling_stock: tooling_stock.unwrap(),
                expected_role_window: expected_role_window.unwrap(),
                expected_handoff_window: expected_handoff_window.unwrap(),
            }
        })
        .collect()
}

fn dependency(
    id: &str,
    governance: IndustrialGovernance,
    production: u64,
    recycling: u64,
    inventory: u64,
) -> IndustrialDependencyState {
    IndustrialDependencyState {
        dependency_id: id.into(),
        governance,
        demand_units_per_tick: 1,
        local_production_units_per_tick: production,
        recycling_units_per_tick: recycling,
        inventory_units: inventory,
    }
}

fn capability(id: &str, essential: bool, dependencies: &[&str]) -> IndustrialCapability {
    IndustrialCapability {
        capability_id: id.into(),
        essential,
        dependency_ids: dependencies.iter().map(|id| (*id).to_owned()).collect(),
    }
}

fn v2_capabilities() -> Vec<IndustrialCapability> {
    vec![
        capability(
            "operation-v2",
            true,
            &["reactor-service-v2", "structural-stock-v2"],
        ),
        capability(
            "construction-controller-v2",
            false,
            &["local-controller-v2"],
        ),
        capability(
            "construction-tooling-v2",
            false,
            &["forge-tooling-v2", "structural-stock-v2"],
        ),
        capability(
            "successor-qualification-v2",
            false,
            &["metrology-v2", "reactor-service-v2"],
        ),
    ]
}

fn prerequisite(
    dependency_id: &str,
    flow_kind: IndustrialFlowKind,
    prerequisite_dependency_id: &str,
) -> IndustrialFlowPrerequisite {
    IndustrialFlowPrerequisite {
        dependency_id: dependency_id.into(),
        flow_kind,
        prerequisite_dependency_ids: BTreeSet::from([prerequisite_dependency_id.into()]),
    }
}

fn v2_spec(tooling_stock: u64) -> IndustrialEpochSpec {
    IndustrialEpochSpec {
        epoch_id: "manta-v2-frontier".into(),
        evidence_binding: format!("epoch:manta-v2-frontier:tooling-{tooling_stock}"),
        dependencies: vec![
            dependency(
                "forge-tooling-v2",
                IndustrialGovernance::Ordinary,
                0,
                0,
                tooling_stock,
            ),
            dependency(
                "local-controller-v2",
                IndustrialGovernance::Ordinary,
                1,
                0,
                0,
            ),
            dependency(
                "metrology-v2",
                IndustrialGovernance::Ordinary,
                1,
                0,
                1,
            ),
            dependency(
                "reactor-service-v2",
                IndustrialGovernance::SafeguardedExternal,
                0,
                0,
                8,
            ),
            dependency(
                "structural-stock-v2",
                IndustrialGovernance::Ordinary,
                0,
                1,
                18,
            ),
        ],
        capabilities: v2_capabilities(),
        flow_model: IndustrialEpochFlowModel::PrerequisiteGated(vec![
            prerequisite(
                "local-controller-v2",
                IndustrialFlowKind::Production,
                "metrology-v2",
            ),
            prerequisite(
                "metrology-v2",
                IndustrialFlowKind::Production,
                "reactor-service-v2",
            ),
            prerequisite(
                "structural-stock-v2",
                IndustrialFlowKind::Recycling,
                "forge-tooling-v2",
            ),
        ]),
    }
}

fn v3_spec(case: &FrontierCase) -> IndustrialEpochSpec {
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
        capabilities: vec![capability(
            "operation-v3",
            true,
            &["reactor-service-v3", "structural-stock-v3"],
        )],
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

fn handoff_plan(
    case: &FrontierCase,
    tooling_units: u64,
    reactor_units: u64,
    structural_units: u64,
) -> IndustrialEpochHandoffPlan {
    IndustrialEpochHandoffPlan {
        handoff_id: format!(
            "handoff:frontier:r{}-t{}",
            case.recovery_tick, case.tooling_stock
        ),
        evidence_binding: format!(
            "handoff-evidence:frontier:r{}-t{}",
            case.recovery_tick, case.tooling_stock
        ),
        source_epoch_id: "manta-v2-frontier".into(),
        source_epoch_evidence_binding: format!(
            "epoch:manta-v2-frontier:tooling-{}",
            case.tooling_stock
        ),
        successor_epoch_id: "manta-v3-frontier".into(),
        successor_epoch_evidence_binding: format!(
            "epoch:manta-v3-frontier:r{}-t{}",
            case.recovery_tick, case.tooling_stock
        ),
        source_inventory_dispositions: vec![
            IndustrialSourceInventoryDisposition {
                source_dependency_id: "forge-tooling-v2".into(),
                successor_dependency_id: Some("forge-tooling-v3".into()),
                transferred_units: tooling_units,
                retired_units: 0,
            },
            IndustrialSourceInventoryDisposition {
                source_dependency_id: "local-controller-v2".into(),
                successor_dependency_id: None,
                transferred_units: 0,
                retired_units: 0,
            },
            IndustrialSourceInventoryDisposition {
                source_dependency_id: "metrology-v2".into(),
                successor_dependency_id: None,
                transferred_units: 0,
                retired_units: 0,
            },
            IndustrialSourceInventoryDisposition {
                source_dependency_id: "reactor-service-v2".into(),
                successor_dependency_id: Some("reactor-service-v3".into()),
                transferred_units: reactor_units,
                retired_units: 0,
            },
            IndustrialSourceInventoryDisposition {
                source_dependency_id: "structural-stock-v2".into(),
                successor_dependency_id: Some("structural-stock-v3".into()),
                transferred_units: structural_units,
                retired_units: 0,
            },
        ],
        external_inventory_admissions: Vec::new(),
    }
}

fn simulate(case: &FrontierCase) -> (Option<u64>, Option<u64>) {
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

    let mut role_window = None;
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
            role_window = Some(expected_tick);

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

            if tooling_units > 0 && reactor_units > 0 {
                assert_eq!(state.last_report().unwrap().tick, expected_tick);
                let (mut successor, receipt) = state
                    .handoff_to(
                        v3_spec(case),
                        handoff_plan(case, tooling_units, reactor_units, structural_units),
                    )
                    .unwrap();
                assert_eq!(receipt.source_final_observation_tick, Some(expected_tick));
                let first = successor.step().unwrap();
                assert!(first.essential_capabilities_available);
                return (role_window, Some(expected_tick));
            }
            break;
        }
    }
    (role_window, None)
}

#[test]
fn explicit_frontier_fixture_matches_all_dynamic_cases() {
    let cases = cases();
    assert_eq!(cases.len(), 42);

    let mut role_successes = 0usize;
    let mut handoff_successes = 0usize;
    for case in &cases {
        let (role_window, handoff_window) = simulate(case);
        assert_eq!(
            role_window, case.expected_role_window,
            "role-window mismatch for {case:?}"
        );
        assert_eq!(
            handoff_window, case.expected_handoff_window,
            "handoff-window mismatch for {case:?}"
        );
        if role_window.is_some() {
            role_successes += 1;
        }
        if handoff_window.is_some() {
            handoff_successes += 1;
        }
    }

    assert_eq!(role_successes, 25);
    assert_eq!(handoff_successes, 18);
    assert!(handoff_successes < role_successes);
}
