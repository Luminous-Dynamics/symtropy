include!("manta_forge_qualified_recovery_adversary_v1.rs");

#[test]
fn manta_recovery_execution_is_bound_to_exact_observed_disturbance() {
    let case = h4_case();
    let successor_spec = recovery_successor_spec(&case);
    let mut reserve = IndustrialRecoveryReserveState::new(
        "repair-reserve-v4",
        "recovery-reserve:repair-v4:controlled-v1",
        recovery_scalar("successor_repair_reserve_inventory_units"),
    )
    .unwrap();
    let contract = qualify_industrial_recovery_contract(
        &successor_spec,
        &reserve,
        "recover-metrology-v4",
        "recovery:metrology-v4:qualified-v1",
        "metrology-v4",
        IndustrialFlowKind::Production,
        recovery_scalar("recovery_reserve_units_per_recovery"),
    )
    .unwrap();

    let mut recoverable = fresh_recovery_successor(&case);
    recoverable
        .apply_shock(IndustrialShock::SetLocalProduction {
            dependency_id: "metrology-v4".into(),
            units_per_tick: recovery_scalar("shock_units_per_tick"),
        })
        .unwrap();

    let disturbance = observe_industrial_recovery_disturbance(
        &recoverable,
        &contract,
        "disturbance:metrology-production-loss-v4",
        "disturbance:metrology-production-loss-v4:v1",
    )
    .unwrap();
    assert_eq!(
        disturbance.observed_units_per_tick,
        recovery_scalar("shock_units_per_tick")
    );

    let receipt = execute_evidence_bound_industrial_recovery(
        &mut recoverable,
        &contract,
        &mut reserve,
        &disturbance,
    )
    .unwrap();
    assert_eq!(
        receipt.disturbance_id,
        "disturbance:metrology-production-loss-v4"
    );
    assert_eq!(receipt.observed_degraded_units_per_tick, 0);
    assert_eq!(receipt.recovery.restored_units_per_tick, 1);
    assert_eq!(receipt.recovery.reserve_units_before, 2);
    assert_eq!(receipt.recovery.reserve_units_after, 1);

    for expected_tick in 1..=recovery_scalar("maturity_periods") {
        let report = recoverable.step().unwrap();
        assert_eq!(report.tick, expected_tick);
        assert!(report.essential_capabilities_available);
    }
}

#[test]
fn stale_disturbance_observation_cannot_authorize_changed_state() {
    let case = h4_case();
    let successor_spec = recovery_successor_spec(&case);
    let mut reserve = IndustrialRecoveryReserveState::new(
        "repair-reserve-v4",
        "recovery-reserve:repair-v4:controlled-v1",
        2,
    )
    .unwrap();
    let contract = qualify_industrial_recovery_contract(
        &successor_spec,
        &reserve,
        "recover-metrology-v4",
        "recovery:metrology-v4:qualified-v1",
        "metrology-v4",
        IndustrialFlowKind::Production,
        1,
    )
    .unwrap();
    let mut state = fresh_recovery_successor(&case);
    state
        .apply_shock(IndustrialShock::SetLocalProduction {
            dependency_id: "metrology-v4".into(),
            units_per_tick: 0,
        })
        .unwrap();
    let disturbance = observe_industrial_recovery_disturbance(
        &state,
        &contract,
        "disturbance:metrology-production-loss-v4",
        "disturbance:metrology-production-loss-v4:v1",
    )
    .unwrap();

    // State changes after observation but before recovery. The stale observation
    // must not authorize the changed subject.
    state
        .apply_shock(IndustrialShock::SetLocalProduction {
            dependency_id: "metrology-v4".into(),
            units_per_tick: 1,
        })
        .unwrap();
    assert!(matches!(
        execute_evidence_bound_industrial_recovery(
            &mut state,
            &contract,
            &mut reserve,
            &disturbance,
        ),
        Err(IndustrialRecoveryDisturbanceError::ObservedStateDrift { .. })
    ));
    assert_eq!(reserve.available_units(), 2);
}
