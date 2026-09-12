use std::collections::BTreeSet;
use symtropy_core::industrial_ecology::{
    BlockedIndustrialFlow, IndustrialCapability, IndustrialDependencyState, IndustrialFlowKind,
    IndustrialFlowPrerequisite, IndustrialGovernance, IndustrialShock,
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

fn capability(id: &str, essential: bool, dependencies: &[&str]) -> IndustrialCapability {
    IndustrialCapability {
        capability_id: id.into(),
        essential,
        dependency_ids: dependencies.iter().map(|id| (*id).to_owned()).collect(),
    }
}

fn v1_spec() -> IndustrialEpochSpec {
    IndustrialEpochSpec {
        epoch_id: "manta-v1".into(),
        evidence_binding: "epoch:manta-v1-lineage-experiment".into(),
        dependencies: vec![
            dependency(
                "forge-tooling-v1",
                IndustrialGovernance::Ordinary,
                1,
                0,
                0,
                8,
            ),
            dependency(
                "imported-controller-v1",
                IndustrialGovernance::Ordinary,
                1,
                0,
                0,
                2,
            ),
            dependency(
                "metrology-v1",
                IndustrialGovernance::Ordinary,
                1,
                0,
                0,
                6,
            ),
            dependency(
                "reactor-service-v1",
                IndustrialGovernance::SafeguardedExternal,
                1,
                0,
                0,
                10,
            ),
            dependency(
                "structural-stock-v1",
                IndustrialGovernance::Ordinary,
                1,
                0,
                0,
                20,
            ),
        ],
        capabilities: vec![
            capability(
                "operation-v1",
                true,
                &["reactor-service-v1", "structural-stock-v1"],
            ),
            capability(
                "successor-construction-v1",
                false,
                &[
                    "forge-tooling-v1",
                    "imported-controller-v1",
                    "structural-stock-v1",
                ],
            ),
            capability(
                "successor-qualification-v1",
                false,
                &["metrology-v1", "reactor-service-v1"],
            ),
        ],
        flow_model: IndustrialEpochFlowModel::LegacyUngated,
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

fn v2_spec(capabilities: Vec<IndustrialCapability>) -> IndustrialEpochSpec {
    IndustrialEpochSpec {
        epoch_id: "manta-v2".into(),
        evidence_binding: "epoch:manta-v2-qualified-substitution".into(),
        dependencies: vec![
            dependency(
                "forge-tooling-v2",
                IndustrialGovernance::Ordinary,
                1,
                0,
                0,
                0,
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
                0,
            ),
            dependency(
                "reactor-service-v2",
                IndustrialGovernance::SafeguardedExternal,
                1,
                0,
                0,
                0,
            ),
            dependency(
                "structural-stock-v2",
                IndustrialGovernance::Ordinary,
                1,
                0,
                1,
                0,
            ),
        ],
        capabilities,
        flow_model: IndustrialEpochFlowModel::PrerequisiteGated(vec![
            IndustrialFlowPrerequisite {
                dependency_id: "local-controller-v2".into(),
                flow_kind: IndustrialFlowKind::Production,
                prerequisite_dependency_ids: BTreeSet::from(["metrology-v2".into()]),
            },
            IndustrialFlowPrerequisite {
                dependency_id: "metrology-v2".into(),
                flow_kind: IndustrialFlowKind::Production,
                prerequisite_dependency_ids: BTreeSet::from(["reactor-service-v2".into()]),
            },
            IndustrialFlowPrerequisite {
                dependency_id: "structural-stock-v2".into(),
                flow_kind: IndustrialFlowKind::Recycling,
                prerequisite_dependency_ids: BTreeSet::from(["forge-tooling-v2".into()]),
            },
        ]),
    }
}

fn v1_to_v2_handoff() -> IndustrialEpochHandoffPlan {
    IndustrialEpochHandoffPlan {
        handoff_id: "handoff-manta-v1-v2-lineage-experiment".into(),
        evidence_binding: "handoff:manta-v1-v2-lineage-experiment".into(),
        source_epoch_id: "manta-v1".into(),
        source_epoch_evidence_binding: "epoch:manta-v1-lineage-experiment".into(),
        successor_epoch_id: "manta-v2".into(),
        successor_epoch_evidence_binding: "epoch:manta-v2-qualified-substitution".into(),
        source_inventory_dispositions: vec![
            IndustrialSourceInventoryDisposition {
                source_dependency_id: "forge-tooling-v1".into(),
                successor_dependency_id: Some("forge-tooling-v2".into()),
                transferred_units: 6,
                retired_units: 0,
            },
            IndustrialSourceInventoryDisposition {
                source_dependency_id: "imported-controller-v1".into(),
                successor_dependency_id: None,
                transferred_units: 0,
                retired_units: 0,
            },
            IndustrialSourceInventoryDisposition {
                source_dependency_id: "metrology-v1".into(),
                successor_dependency_id: Some("metrology-v2".into()),
                transferred_units: 4,
                retired_units: 0,
            },
            IndustrialSourceInventoryDisposition {
                source_dependency_id: "reactor-service-v1".into(),
                successor_dependency_id: Some("reactor-service-v2".into()),
                transferred_units: 8,
                retired_units: 0,
            },
            IndustrialSourceInventoryDisposition {
                source_dependency_id: "structural-stock-v1".into(),
                successor_dependency_id: Some("structural-stock-v2".into()),
                transferred_units: 18,
                retired_units: 0,
            },
        ],
        external_inventory_admissions: Vec::new(),
    }
}

#[test]
fn evolving_lineage_loses_reproduction_before_current_platform_operation() {
    // MANTA-v1 consumes the last two imported controllers while its other support
    // stockpiles remain healthy. The lineage changes design before the old
    // controller dependency actually becomes a shortage.
    let mut v1 = IndustrialEpochState::from_spec(v1_spec()).unwrap();
    assert_eq!(v1.step().unwrap().tick, 1);
    assert_eq!(v1.step().unwrap().tick, 2);
    assert_eq!(
        v1.dependency("imported-controller-v1")
            .unwrap()
            .inventory_units,
        0
    );
    assert!(v1
        .last_report()
        .unwrap()
        .shortages
        .iter()
        .all(|shortage| shortage.dependency_id != "imported-controller-v1"));

    let known_v2_capabilities = v2_capabilities();
    let (mut v2, handoff_receipt) = v1
        .handoff_to(
            v2_spec(known_v2_capabilities.clone()),
            v1_to_v2_handoff(),
        )
        .unwrap();
    assert_eq!(handoff_receipt.source_final_tick, 2);
    assert_eq!(v2.tick(), 0);
    assert_eq!(
        v2.dependency("forge-tooling-v2").unwrap().inventory_units,
        6
    );
    assert_eq!(v2.dependency("metrology-v2").unwrap().inventory_units, 4);
    assert_eq!(
        v2.dependency("reactor-service-v2")
            .unwrap()
            .inventory_units,
        8
    );
    assert_eq!(
        v2.dependency("structural-stock-v2")
            .unwrap()
            .inventory_units,
        18
    );
    assert_eq!(
        v2.dependency("local-controller-v2")
            .unwrap()
            .inventory_units,
        0
    );

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

    // FORGE degradation: metrology production is initially lost. The flow was
    // modeled at epoch construction, so it may later recover without inventing a
    // new production capability.
    v2.apply_shock(IndustrialShock::SetLocalProduction {
        dependency_id: "metrology-v2".into(),
        units_per_tick: 0,
    })
    .unwrap();

    let mut both_construction_and_qualification_available_after_adaptation = false;
    let mut tick6_blocked_controller = false;
    let mut tick8_blocked_recycling = false;

    for expected_tick in 1..=9 {
        if expected_tick == 6 {
            // Qualified adaptation restores the already-modeled metrology flow.
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
        if expected_tick >= 6 && construction.available && qualification.available {
            both_construction_and_qualification_available_after_adaptation = true;
        }

        if expected_tick == 6 {
            tick6_blocked_controller = report.blocked_flows.contains(&BlockedIndustrialFlow {
                dependency_id: "local-controller-v2".into(),
                flow_kind: IndustrialFlowKind::Production,
                blocking_prerequisite_ids: vec!["metrology-v2".into()],
            });
        }
        if expected_tick == 8 {
            tick8_blocked_recycling = report.blocked_flows.contains(&BlockedIndustrialFlow {
                dependency_id: "structural-stock-v2".into(),
                flow_kind: IndustrialFlowKind::Recycling,
                blocking_prerequisite_ids: vec!["forge-tooling-v2".into()],
            });
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

    // Metrology runs out on tick 5, is restored for tick 6, and stays qualified
    // until safeguarded reactor service itself becomes short on tick 9.
    assert_eq!(qualification.first_unavailable_tick, Some(5));
    assert_eq!(qualification.first_recovery_tick(), Some(6));
    assert_eq!(qualification.currently_available(), Some(false));
    assert!(qualification.transitions.iter().any(|transition| {
        transition.tick == 9
            && !transition.available
            && transition.unavailable_required_capability_ids
                == vec!["successor-qualification-v2"]
    }));

    // The one-tick propagation theorem is visible: prior metrology shortage blocks
    // controller production on tick 6 even though metrology itself recovers then.
    assert!(tick6_blocked_controller);
    assert_eq!(construction.first_unavailable_tick, Some(6));
    assert!(construction.transitions.iter().any(|transition| {
        transition.tick == 6
            && !transition.available
            && transition.unavailable_required_capability_ids
                == vec!["construction-controller-v2"]
    }));

    // Tooling becomes short on tick 7. The controller recovers, but the same role
    // remains unavailable for a *different* reason, which the trace now preserves.
    assert!(construction.transitions.iter().any(|transition| {
        transition.tick == 7
            && !transition.available
            && transition.unavailable_required_capability_ids
                == vec!["construction-tooling-v2"]
    }));
    assert!(tick8_blocked_recycling);
    assert_eq!(construction.first_recovery_tick(), None);
    assert_eq!(construction.currently_available(), Some(false));

    // Current-platform operation outlives lineage reproduction, but safeguarded
    // reactor-service exhaustion eventually ends operation as well.
    assert_eq!(operation.first_unavailable_tick, Some(9));
    assert_eq!(operation.currently_available(), Some(false));

    // After the adaptation begins, there is never another completed tick where the
    // lineage simultaneously has both construction and qualification capability.
    // Therefore a MANTA-v3 successor cannot both be built and qualified before the
    // safeguarded-service window closes in this deterministic scenario.
    assert!(!both_construction_and_qualification_available_after_adaptation);
}
