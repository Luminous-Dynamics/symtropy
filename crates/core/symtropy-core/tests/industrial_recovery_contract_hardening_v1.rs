use std::collections::BTreeSet;
use symtropy_core::prelude::*;

fn dependency(
    id: &str,
    production: u64,
    inventory: u64,
) -> IndustrialDependencyState {
    IndustrialDependencyState {
        dependency_id: id.into(),
        governance: IndustrialGovernance::Ordinary,
        demand_units_per_tick: 1,
        local_production_units_per_tick: production,
        recycling_units_per_tick: 0,
        inventory_units: inventory,
    }
}

fn zero_target_spec() -> IndustrialEpochSpec {
    IndustrialEpochSpec {
        epoch_id: "recovery-zero-target-v1".into(),
        evidence_binding: "epoch:recovery-zero-target:v1".into(),
        dependencies: vec![
            dependency("measurement", 0, 2),
            dependency("power", 1, 0),
        ],
        capabilities: vec![IndustrialCapability {
            capability_id: "qualified-output".into(),
            essential: true,
            dependency_ids: BTreeSet::from(["measurement".into(), "power".into()]),
        }],
        // Only the positive power flow needs a prerequisite entry. The zero target
        // is therefore a valid *unmodeled* flow, not a malformed epoch spec.
        flow_model: IndustrialEpochFlowModel::PrerequisiteGated(vec![
            IndustrialFlowPrerequisite {
                dependency_id: "power".into(),
                flow_kind: IndustrialFlowKind::Production,
                prerequisite_dependency_ids: BTreeSet::from(["measurement".into()]),
            },
        ]),
    }
}

fn healthy_spec() -> IndustrialEpochSpec {
    IndustrialEpochSpec {
        epoch_id: "recovery-healthy-v1".into(),
        evidence_binding: "epoch:recovery-healthy:v1".into(),
        dependencies: vec![
            dependency("measurement", 1, 0),
            dependency("power", 1, 0),
        ],
        capabilities: vec![IndustrialCapability {
            capability_id: "qualified-output".into(),
            essential: true,
            dependency_ids: BTreeSet::from(["measurement".into(), "power".into()]),
        }],
        flow_model: IndustrialEpochFlowModel::PrerequisiteGated(vec![
            IndustrialFlowPrerequisite {
                dependency_id: "measurement".into(),
                flow_kind: IndustrialFlowKind::Production,
                prerequisite_dependency_ids: BTreeSet::from(["power".into()]),
            },
            IndustrialFlowPrerequisite {
                dependency_id: "power".into(),
                flow_kind: IndustrialFlowKind::Production,
                prerequisite_dependency_ids: BTreeSet::from(["measurement".into()]),
            },
        ]),
    }
}

#[test]
fn valid_zero_flow_target_cannot_acquire_recovery_authority_after_the_fact() {
    let spec = zero_target_spec();
    IndustrialEpochState::from_spec(spec.clone()).unwrap();
    let reserve = IndustrialRecoveryReserveState::new(
        "external-repair-reserve",
        "recovery-reserve:external:v1",
        1,
    )
    .unwrap();

    assert!(matches!(
        qualify_industrial_recovery_contract(
            &spec,
            &reserve,
            "recover-measurement",
            "recovery:measurement:v1",
            "measurement",
            IndustrialFlowKind::Production,
            1,
        ),
        Err(IndustrialRecoveryError::TargetFlowNotPositiveAtQualification { .. })
    ));
}

#[test]
fn equal_quantity_reserve_with_different_evidence_cannot_spend_qualified_contract() {
    let spec = healthy_spec();
    let qualified_reserve = IndustrialRecoveryReserveState::new(
        "external-repair-reserve",
        "recovery-reserve:qualified:v1",
        1,
    )
    .unwrap();
    let contract = qualify_industrial_recovery_contract(
        &spec,
        &qualified_reserve,
        "recover-measurement",
        "recovery:measurement:v1",
        "measurement",
        IndustrialFlowKind::Production,
        1,
    )
    .unwrap();

    let mut state = IndustrialEpochState::from_spec(spec).unwrap();
    state
        .apply_shock(IndustrialShock::SetLocalProduction {
            dependency_id: "measurement".into(),
            units_per_tick: 0,
        })
        .unwrap();
    let mut substituted_reserve = IndustrialRecoveryReserveState::new(
        "external-repair-reserve",
        "recovery-reserve:substituted:v1",
        1,
    )
    .unwrap();

    assert_eq!(
        execute_industrial_recovery(&mut state, &contract, &mut substituted_reserve),
        Err(IndustrialRecoveryError::RecoveryReserveBindingMismatch)
    );
    assert_eq!(
        state
            .dependency("measurement")
            .unwrap()
            .local_production_units_per_tick,
        0
    );
    assert_eq!(substituted_reserve.available_units(), 1);
}
