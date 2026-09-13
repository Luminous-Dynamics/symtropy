use std::collections::BTreeSet;
use symtropy_core::prelude::*;

fn dependency(id: &str, production: u64) -> IndustrialDependencyState {
    IndustrialDependencyState {
        dependency_id: id.into(),
        governance: IndustrialGovernance::Ordinary,
        demand_units_per_tick: 1,
        local_production_units_per_tick: production,
        recycling_units_per_tick: 0,
        inventory_units: 0,
    }
}

fn spec() -> IndustrialEpochSpec {
    IndustrialEpochSpec {
        epoch_id: "linear-recovery-v1".into(),
        evidence_binding: "epoch:linear-recovery:v1".into(),
        dependencies: vec![dependency("measurement", 1), dependency("power", 1)],
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
fn two_recoveries_form_one_monotonic_reserve_sequence() {
    let spec = spec();
    let mut reserve = IndustrialRecoveryReserveState::new(
        "external-repair-reserve",
        "recovery-reserve:linear:v1",
        2,
    )
    .unwrap();
    let contract = qualify_industrial_recovery_contract(
        &spec,
        &reserve,
        "recover-measurement",
        "recovery:measurement:linear-v1",
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
    let first = execute_industrial_recovery(&mut state, &contract, &mut reserve).unwrap();
    assert_eq!((first.reserve_spend_sequence_before, first.reserve_spend_sequence_after), (0, 1));
    assert_eq!((first.reserve_units_before, first.reserve_units_after), (2, 1));
    assert_eq!(first.reserve_initial_units, 2);

    state
        .apply_shock(IndustrialShock::SetLocalProduction {
            dependency_id: "measurement".into(),
            units_per_tick: 0,
        })
        .unwrap();
    let second = execute_industrial_recovery(&mut state, &contract, &mut reserve).unwrap();
    assert_eq!((second.reserve_spend_sequence_before, second.reserve_spend_sequence_after), (1, 2));
    assert_eq!((second.reserve_units_before, second.reserve_units_after), (1, 0));
    assert_eq!(second.reserve_initial_units, 2);
    assert_eq!(reserve.spend_sequence(), 2);
    assert_eq!(reserve.spent_units(), 2);
    assert_eq!(reserve.available_units(), 0);
}

#[test]
fn same_reserve_identity_with_different_genesis_quantity_is_rejected() {
    let spec = spec();
    let qualified_reserve = IndustrialRecoveryReserveState::new(
        "external-repair-reserve",
        "recovery-reserve:linear:v1",
        2,
    )
    .unwrap();
    let contract = qualify_industrial_recovery_contract(
        &spec,
        &qualified_reserve,
        "recover-measurement",
        "recovery:measurement:linear-v1",
        "measurement",
        IndustrialFlowKind::Production,
        1,
    )
    .unwrap();

    let mut reconstructed = IndustrialRecoveryReserveState::new(
        "external-repair-reserve",
        "recovery-reserve:linear:v1",
        3,
    )
    .unwrap();
    let mut state = IndustrialEpochState::from_spec(spec).unwrap();
    state
        .apply_shock(IndustrialShock::SetLocalProduction {
            dependency_id: "measurement".into(),
            units_per_tick: 0,
        })
        .unwrap();

    assert_eq!(
        execute_industrial_recovery(&mut state, &contract, &mut reconstructed),
        Err(IndustrialRecoveryError::RecoveryReserveGenesisMismatch)
    );
    assert_eq!(reconstructed.spend_sequence(), 0);
    assert_eq!(reconstructed.available_units(), 3);
}
