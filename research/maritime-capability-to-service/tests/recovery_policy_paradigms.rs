use maritime_capability_to_service::{
    compare_recovery_regret, worst_case_recovery_regret, RecoveryRegretEnvelope,
    RecoveryRegretMetric, RecoveryServiceOutcome,
};

fn outcome(
    policy: &str,
    scenario: &str,
    values: [u64; 13],
) -> RecoveryServiceOutcome {
    RecoveryServiceOutcome {
        instance_id: "MCSC-RECOVERY-PARADIGMS-001".to_owned(),
        instance_version: "v0.1".to_owned(),
        scenario_id: scenario.to_owned(),
        policy_id: policy.to_owned(),
        service_id: "ESSENTIAL-CARGO".to_owned(),
        deadline_day: 10,
        horizon_day: 20,
        expected_units: 100,
        inspection_qualified_units: values[0],
        receiver_accepted_units: values[1],
        on_time_accepted_units: values[2],
        unresolved_units: values[3],
        service_deficit_unit_hours: values[4],
        time_to_minimum_viable_service_day: values[5],
        time_to_full_service_day: values[6],
        recovery_cost_units: values[7],
        recovery_emissions_units: values[8],
        requalification_actions: values[9],
        replan_invocations: values[10],
        plan_churn_changes: values[11],
        decision_latency_millis: values[12],
    }
}

fn envelope_for_policy(
    policy: &str,
    scenario_pairs: &[(&str, [u64; 13], [u64; 13])],
) -> RecoveryRegretEnvelope {
    let vectors = scenario_pairs
        .iter()
        .map(|(scenario, candidate_values, oracle_values)| {
            let candidate = outcome(policy, scenario, *candidate_values);
            let oracle = outcome("CLAIRVOYANT-BOUND", scenario, *oracle_values);
            compare_recovery_regret(&candidate, &oracle).expect("matched scenario context")
        })
        .collect::<Vec<_>>();
    worst_case_recovery_regret(&vectors).expect("unique frozen scenario set")
}

fn bound<'a>(
    envelope: &'a RecoveryRegretEnvelope,
    metric: RecoveryRegretMetric,
) -> &'a maritime_capability_to_service::WorstCaseRecoveryRegret {
    envelope
        .worst_case
        .iter()
        .find(|item| item.metric == metric)
        .expect("all regret dimensions are emitted")
}

#[test]
fn recovery_paradigms_expose_service_stability_and_decision_latency_tradeoffs() {
    // Each tuple is (scenario ID, candidate policy outcome, same-scenario
    // clairvoyant upper-bound outcome). Values are synthetic and unweighted.
    let scenarios = [
        (
            "SHARED-FEEDER-OUTAGE",
            [60, 60, 60, 40, 800, 2, 16, 200, 90, 2, 1, 1, 700],
            [100, 100, 100, 0, 0, 1, 4, 250, 90, 1, 1, 2, 500],
        ),
        (
            "CALIBRATION-EXPIRY",
            [50, 50, 40, 50, 1_200, 3, 18, 220, 100, 3, 1, 2, 650],
            [100, 100, 100, 0, 0, 1, 4, 250, 90, 1, 1, 2, 500],
        ),
        (
            "SEQUENTIAL-DISRUPTION",
            [55, 55, 40, 45, 1_300, 4, 20, 250, 110, 4, 1, 2, 650],
            [100, 100, 100, 0, 0, 1, 4, 250, 90, 1, 1, 2, 500],
        ),
    ];

    let disruption_management = envelope_for_policy("DISRUPTION-MANAGEMENT", &scenarios);
    let full_rescheduling = envelope_for_policy("FULL-RESCHEDULING", &[
        (
            "SHARED-FEEDER-OUTAGE",
            [90, 90, 80, 10, 200, 1, 6, 300, 130, 2, 3, 30, 3_500],
            scenarios[0].2,
        ),
        (
            "CALIBRATION-EXPIRY",
            [85, 80, 70, 20, 300, 2, 7, 320, 140, 3, 4, 35, 4_500],
            scenarios[1].2,
        ),
        (
            "SEQUENTIAL-DISRUPTION",
            [80, 75, 60, 25, 500, 2, 10, 400, 170, 4, 6, 50, 6_000],
            scenarios[2].2,
        ),
    ]);
    let proactive_reactive = envelope_for_policy("PROACTIVE-REACTIVE-HYBRID", &[
        (
            "SHARED-FEEDER-OUTAGE",
            [85, 80, 75, 20, 250, 1, 7, 270, 110, 2, 2, 12, 1_800],
            scenarios[0].2,
        ),
        (
            "CALIBRATION-EXPIRY",
            [80, 75, 65, 25, 450, 2, 9, 290, 120, 3, 3, 15, 2_000],
            scenarios[1].2,
        ),
        (
            "SEQUENTIAL-DISRUPTION",
            [75, 70, 60, 30, 550, 2, 11, 350, 150, 4, 4, 20, 2_500],
            scenarios[2].2,
        ),
    ]);

    for envelope in [
        &disruption_management,
        &full_rescheduling,
        &proactive_reactive,
    ] {
        assert_eq!(envelope.scenario_count, 3);
        assert_eq!(envelope.worst_case.len(), 14);
    }

    // Local disruption management is stable/cheap to replan, but leaves the
    // greatest service and qualification deficit in the frozen hard scenarios.
    assert_eq!(
        bound(
            &disruption_management,
            RecoveryRegretMetric::UnmetUnitsAtDeadline
        )
        .delta,
        60
    );
    assert_eq!(
        bound(
            &disruption_management,
            RecoveryRegretMetric::InspectionQualifiedUnits
        )
        .delta,
        -50
    );
    assert_eq!(
        bound(
            &disruption_management,
            RecoveryRegretMetric::ReplanInvocations
        )
        .delta,
        0
    );

    // Full rescheduling restores more service, but pays for it in churn and latency.
    assert_eq!(
        bound(
            &full_rescheduling,
            RecoveryRegretMetric::UnmetUnitsAtDeadline
        )
        .delta,
        40
    );
    assert_eq!(
        bound(
            &full_rescheduling,
            RecoveryRegretMetric::PlanChurnChanges
        )
        .delta,
        48
    );
    assert_eq!(
        bound(
            &full_rescheduling,
            RecoveryRegretMetric::DecisionLatencyMillis
        )
        .delta,
        5_500
    );

    // The proactive-reactive hybrid is neither declared optimal nor scalarized:
    // its service and operational-burden envelope is separately visible.
    assert_eq!(
        bound(
            &proactive_reactive,
            RecoveryRegretMetric::UnmetUnitsAtDeadline
        )
        .delta,
        40
    );
    assert_eq!(
        bound(
            &proactive_reactive,
            RecoveryRegretMetric::PlanChurnChanges
        )
        .delta,
        18
    );
    assert_eq!(
        bound(
            &proactive_reactive,
            RecoveryRegretMetric::DecisionLatencyMillis
        )
        .delta,
        2_000
    );
}
