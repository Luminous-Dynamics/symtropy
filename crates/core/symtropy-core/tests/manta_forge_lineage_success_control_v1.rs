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
use symtropy_core::industrial_watch_trace::{
    record_industrial_capability_watch_reports, IndustrialCapabilityWatchTrace,
};

fn dependency(
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

fn capability(
    id: impl Into<String>,
    essential: bool,
    dependencies: impl IntoIterator<Item = String>,
) -> IndustrialCapability {
    IndustrialCapability {
        capability_id: id.into(),
        essential,
        dependency_ids: dependencies.into_iter().collect(),
    }
}

fn capabilities(generation: &str) -> Vec<IndustrialCapability> {
    vec![
        capability(
            format!("operation-{generation}"),
            true,
            vec![
                format!("reactor-service-{generation}"),
                format!("structural-stock-{generation}"),
            ],
        ),
        capability(
            format!("construction-controller-{generation}"),
            false,
            vec![format!("local-controller-{generation}")],
        ),
        capability(
            format!("construction-tooling-{generation}"),
            false,
            vec![
                format!("forge-tooling-{generation}"),
                format!("structural-stock-{generation}"),
            ],
        ),
        capability(
            format!("successor-qualification-{generation}"),
            false,
            vec![
                format!("metrology-{generation}"),
                format!("reactor-service-{generation}"),
            ],
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

fn v2_spec() -> IndustrialEpochSpec {
    IndustrialEpochSpec {
        epoch_id: "manta-v2-control".into(),
        evidence_binding: "epoch:manta-v2-success-control".into(),
        dependencies: vec![
            dependency(
                "forge-tooling-v2",
                IndustrialGovernance::Ordinary,
                1,
                0,
                0,
                8,
            ),
            dependency(
                "local-controller-v2",
                IndustrialGovernance::Ordinary,
                1,
                1,
                0,
                0,
            ),
            dependency(
                "metrology-v2",
                IndustrialGovernance::Ordinary,
                1,
                1,
                0,
                1,
            ),
            dependency(
                "reactor-service-v2",
                IndustrialGovernance::SafeguardedExternal,
                1,
                0,
                0,
                8,
            ),
            dependency(
                "structural-stock-v2",
                IndustrialGovernance::Ordinary,
                1,
                0,
                1,
                18,
            ),
        ],
        capabilities: capabilities("v2"),
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

fn v3_spec() -> IndustrialEpochSpec {
    IndustrialEpochSpec {
        epoch_id: "manta-v3-control".into(),
        evidence_binding: "epoch:manta-v3-success-control".into(),
        dependencies: vec![
            dependency(
                "forge-tooling-v3",
                IndustrialGovernance::Ordinary,
                1,
                0,
                0,
                0,
            ),
            dependency(
                "local-controller-v3",
                IndustrialGovernance::Ordinary,
                1,
                1,
                0,
                0,
            ),
            dependency(
                "metrology-v3",
                IndustrialGovernance::Ordinary,
                1,
                1,
                0,
                0,
            ),
            dependency(
                "reactor-service-v3",
                IndustrialGovernance::SafeguardedExternal,
                1,
                0,
                0,
                0,
            ),
            dependency(
                "structural-stock-v3",
                IndustrialGovernance::Ordinary,
                1,
                0,
                1,
                0,
            ),
        ],
        capabilities: capabilities("v3"),
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

fn v2_to_v3_handoff() -> IndustrialEpochHandoffPlan {
    IndustrialEpochHandoffPlan {
        handoff_id: "handoff-manta-v2-v3-success-control".into(),
        evidence_binding: "handoff:manta-v2-v3-success-control".into(),
        source_epoch_id: "manta-v2-control".into(),
        source_epoch_evidence_binding: "epoch:manta-v2-success-control".into(),
        successor_epoch_id: "manta-v3-control".into(),
        successor_epoch_evidence_binding: "epoch:manta-v3-success-control".into(),
        source_inventory_dispositions: vec![
            IndustrialSourceInventoryDisposition {
                source_dependency_id: "forge-tooling-v2".into(),
                successor_dependency_id: Some("forge-tooling-v3".into()),
                transferred_units: 4,
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
                transferred_units: 4,
                retired_units: 0,
            },
            IndustrialSourceInventoryDisposition {
                source_dependency_id: "structural-stock-v2".into(),
                successor_dependency_id: Some("structural-stock-v3".into()),
                transferred_units: 18,
                retired_units: 0,
            },
        ],
        external_inventory_admissions: Vec::new(),
    }
}

#[test]
fn earlier_adaptation_creates_a_reproduction_window_and_v3_handoff() {
    let known_v2_capabilities = capabilities("v2");
    let mut v2 = IndustrialEpochState::from_spec(v2_spec()).unwrap();
    let watches = vec![
        IndustrialCapabilityWatch::new(
            "operation",
            BTreeSet::from(["operation-v2".into()]),
            &known_v2_capabilities,
        )
        .unwrap(),
        IndustrialCapabilityWatch::new(
            "construction",
            BTreeSet::from([
                "construction-controller-v2".into(),
                "construction-tooling-v2".into(),
            ]),
            &known_v2_capabilities,
        )
        .unwrap(),
        IndustrialCapabilityWatch::new(
            "qualification",
            BTreeSet::from(["successor-qualification-v2".into()]),
            &known_v2_capabilities,
        )
        .unwrap(),
    ];
    let mut traces = vec![
        IndustrialCapabilityWatchTrace::new("operation"),
        IndustrialCapabilityWatchTrace::new("construction"),
        IndustrialCapabilityWatchTrace::new("qualification"),
    ];

    // Same qualitative perturbation as the failed-lineage experiment: metrology
    // production disappears. Here adaptation arrives early enough that tooling and
    // safeguarded support remain available when the one-tick dependency lag clears.
    v2.apply_shock(IndustrialShock::SetLocalProduction {
        dependency_id: "metrology-v2".into(),
        units_per_tick: 0,
    })
    .unwrap();

    let mut reproduction_window_tick = None;
    for expected_tick in 1..=4 {
        if expected_tick == 3 {
            v2.apply_shock(IndustrialShock::SetLocalProduction {
                dependency_id: "metrology-v2".into(),
                units_per_tick: 1,
            })
            .unwrap();
        }

        let report = v2.step().unwrap();
        assert_eq!(report.tick, expected_tick);
        let role_reports = assess_industrial_capability_watches(&report, &watches);
        record_industrial_capability_watch_reports(&mut traces, &role_reports).unwrap();

        let construction = role_reports
            .iter()
            .find(|role| role.watch_id == "construction")
            .unwrap();
        let qualification = role_reports
            .iter()
            .find(|role| role.watch_id == "qualification")
            .unwrap();
        if expected_tick >= 3 && construction.available && qualification.available {
            reproduction_window_tick.get_or_insert(expected_tick);
        }
    }

    let operation = traces
        .iter()
        .find(|trace| trace.watch_id == "operation")
        .unwrap();
    let construction = traces
        .iter()
        .find(|trace| trace.watch_id == "construction")
        .unwrap();
    let qualification = traces
        .iter()
        .find(|trace| trace.watch_id == "qualification")
        .unwrap();

    // Tick 2 exhausts metrology. Restoring its modeled production before tick 3
    // restores qualification immediately, while the one-tick flow dependency rule
    // delays local-controller recovery until tick 4.
    assert_eq!(qualification.first_unavailable_tick, Some(2));
    assert_eq!(qualification.first_recovery_tick(), Some(3));
    assert_eq!(construction.first_unavailable_tick, Some(3));
    assert_eq!(construction.first_recovery_tick(), Some(4));
    assert_eq!(operation.first_unavailable_tick, None);
    assert_eq!(reproduction_window_tick, Some(4));

    // A fresh completed tick describes the exact source state used for handoff.
    assert_eq!(v2.last_report().unwrap().tick, 4);
    assert_eq!(v2.dependency("forge-tooling-v2").unwrap().inventory_units, 4);
    assert_eq!(v2.dependency("local-controller-v2").unwrap().inventory_units, 0);
    assert_eq!(v2.dependency("metrology-v2").unwrap().inventory_units, 0);
    assert_eq!(
        v2.dependency("reactor-service-v2")
            .unwrap()
            .inventory_units,
        4
    );
    assert_eq!(
        v2.dependency("structural-stock-v2")
            .unwrap()
            .inventory_units,
        18
    );

    let (mut v3, receipt) = v2.handoff_to(v3_spec(), v2_to_v3_handoff()).unwrap();
    assert_eq!(receipt.source_final_tick, 4);
    assert_eq!(receipt.source_final_observation_tick, Some(4));
    assert!(receipt.source_final_shortage_ids.is_empty());
    assert!(receipt.source_final_unavailable_capability_ids.is_empty());

    assert_eq!(v3.tick(), 0);
    assert_eq!(v3.dependency("forge-tooling-v3").unwrap().inventory_units, 4);
    assert_eq!(
        v3.dependency("reactor-service-v3")
            .unwrap()
            .inventory_units,
        4
    );
    assert_eq!(
        v3.dependency("structural-stock-v3")
            .unwrap()
            .inventory_units,
        18
    );

    // The successor is independently validated and can execute its first complete
    // tick with operation, construction, and qualification all available.
    let report = v3.step().unwrap();
    assert_eq!(report.tick, 1);
    assert!(report.essential_capabilities_available);
    assert!(report.unavailable_capability_ids.is_empty());
}
