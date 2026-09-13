include!("manta_forge_qualified_recovery_adversary_v1.rs");

#[test]
fn manta_recovery_execution_is_bound_to_exact_observed_disturbance() {
    let case = h4_case();
    let successor_spec = recovery_successor_spec(&case);
    let mut reserve = recovery_reserve();
    let contract = qualify_industrial_recovery_contract(
        &successor_spec,
        &reserve,
        "recover-metrology-v4",
        "recovery:metrology-v4:qualified-v1",
        recovery_text("disturbance_target"),
        IndustrialFlowKind::Production,
        recovery_scalar("recovery_reserve_units_per_recovery"),
    )
    .unwrap();

    let mut recoverable = fresh_recovery_successor(&case);
    recoverable
        .apply_shock(IndustrialShock::SetLocalProduction {
            dependency_id: recovery_text("disturbance_target").into(),
            units_per_tick: recovery_scalar("degraded_units_per_tick"),
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
        recovery_scalar("degraded_units_per_tick")
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
    assert_eq!(
        receipt.observed_degraded_units_per_tick,
        recovery_scalar("degraded_units_per_tick")
    );
    assert_eq!(
        receipt.recovery.restored_units_per_tick,
        recovery_scalar("qualified_recovery_units_per_tick")
    );
    assert_eq!(
        receipt.recovery.reserve_initial_units,
        recovery_scalar("external_recovery_reserve_units")
    );
    assert_eq!(receipt.recovery.reserve_spend_sequence_before, 0);
    assert_eq!(receipt.recovery.reserve_spend_sequence_after, 1);
    assert_eq!(
        receipt.recovery.reserve_units_before,
        recovery_scalar("external_recovery_reserve_units")
    );
    assert_eq!(
        receipt.recovery.reserve_units_after,
        recovery_scalar("recoverable_terminal_recovery_reserve_units")
    );

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
    let mut reserve = recovery_reserve();
    let contract = qualify_industrial_recovery_contract(
        &successor_spec,
        &reserve,
        "recover-metrology-v4",
        "recovery:metrology-v4:qualified-v1",
        recovery_text("disturbance_target"),
        IndustrialFlowKind::Production,
        recovery_scalar("recovery_reserve_units_per_recovery"),
    )
    .unwrap();
    let mut state = fresh_recovery_successor(&case);
    state
        .apply_shock(IndustrialShock::SetLocalProduction {
            dependency_id: recovery_text("disturbance_target").into(),
            units_per_tick: recovery_scalar("degraded_units_per_tick"),
        })
        .unwrap();
    let disturbance = observe_industrial_recovery_disturbance(
        &state,
        &contract,
        "disturbance:metrology-production-loss-v4",
        "disturbance:metrology-production-loss-v4:v1",
    )
    .unwrap();

    state
        .apply_shock(IndustrialShock::SetLocalProduction {
            dependency_id: recovery_text("disturbance_target").into(),
            units_per_tick: recovery_scalar("qualified_recovery_units_per_tick"),
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
    assert_eq!(
        reserve.available_units(),
        recovery_scalar("external_recovery_reserve_units")
    );
    assert_eq!(reserve.spend_sequence(), 0);
}
