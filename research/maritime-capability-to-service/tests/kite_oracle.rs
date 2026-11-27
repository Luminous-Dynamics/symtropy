use maritime_capability_to_service::kite::Gate;
use maritime_capability_to_service::kite::{
    AuthorityState, ClockQuality, EvidenceState, EvidenceVintage, RightsState, select_as_of,
};
use maritime_capability_to_service::kite_oracle::{
    ActionOutcome, DecisionContext, OutcomeVector, ScenarioDecision, solve_r1_finite,
};

fn context() -> DecisionContext {
    DecisionContext {
        instance_id: "MCSC-R1-TINY-001".to_owned(),
        instance_version: "v0.1".to_owned(),
        service_id: "RECEIVER-SERVICE-001".to_owned(),
        service_deadline: 120,
        expected_units: 10,
    }
}

fn snapshot(
    vintage_id: &str,
    first_available_at: u64,
) -> maritime_capability_to_service::kite::AsOfSnapshot {
    let evidence = EvidenceVintage {
        logical_id: "GRID-FEEDER".to_owned(),
        vintage_id: vintage_id.to_owned(),
        revision: 1,
        supersedes_vintage_id: None,
        event_time: 20,
        source_observed_at: first_available_at - 1,
        first_available_at,
        ingested_at: first_available_at + 1,
        validated_at: first_available_at + 2,
        valid_until: None,
        rights: RightsState::Granted,
        authority: AuthorityState::Valid,
        evidence_state: EvidenceState::Observed,
        clock_quality: ClockQuality::Trusted,
        feature_uses_future_data: false,
    };
    select_as_of(&[evidence], 100).expect("valid evidence snapshot")
}

fn outcome(accepted: u64, cost: u64) -> OutcomeVector {
    OutcomeVector {
        receiver_accepted_units: accepted,
        on_time_accepted_units: accepted,
        acceptance_time_unknown_units: 0,
        unmet_units: 10 - accepted,
        late_accepted_units: 0,
        unresolved_units: 0,
        protected_floor_breach_unit_hours: 0,
        service_deficit_unit_hours: 0,
        time_to_minimum_viable_service: 2,
        time_to_full_service: 4,
        recovery_cost_units: cost,
        emissions_units: 5,
        requalification_actions: 0,
        replanning_invocations: 1,
        plan_churn: 1,
        decision_latency_ms: 100,
    }
}

fn action(id: &str, accepted: u64, cost: u64) -> ActionOutcome {
    ActionOutcome {
        action_id: id.to_owned(),
        admissibility: Gate::Yes,
        hard_constraints_passed: true,
        outcome: outcome(accepted, cost),
    }
}

fn decision(
    id: &str,
    as_of: maritime_capability_to_service::kite::AsOfSnapshot,
    actions: Vec<ActionOutcome>,
) -> ScenarioDecision {
    let digest_seed = as_of
        .selected
        .values()
        .map(|item| item.vintage_id.bytes().map(u64::from).sum::<u64>())
        .sum::<u64>();
    ScenarioDecision {
        scenario_id: id.to_owned(),
        decision_epoch: 1,
        context: context(),
        observable_state_digest: format!("{digest_seed:064x}"),
        as_of,
        actions,
    }
}

#[test]
fn public_r1_api_enforces_non_anticipativity_for_identical_information() {
    let shared = snapshot("GRID-V1", 60);
    let result = solve_r1_finite(
        &[
            decision(
                "SCENARIO-A",
                shared.clone(),
                vec![action("LOCAL", 10, 20), action("IMPORT", 9, 10)],
            ),
            decision(
                "SCENARIO-B",
                shared,
                vec![action("LOCAL", 8, 30), action("IMPORT", 9, 15)],
            ),
        ],
        16,
    )
    .expect("R1 finite solve");

    assert_eq!(result.candidate_policy_count, 2);
    assert_eq!(result.pareto_frontier.len(), 1);
    assert_eq!(
        result.pareto_frontier[0]
            .choices
            .values()
            .next()
            .map(String::as_str),
        Some("IMPORT")
    );
    assert_eq!(
        result.pareto_frontier[0]
            .adverse_envelope
            .bounds
            .receiver_accepted_units,
        9
    );
    assert_eq!(
        result.pareto_frontier[0]
            .adverse_envelope
            .bounds
            .recovery_cost_units,
        15
    );
}

#[test]
fn later_observable_histories_may_choose_different_actions() {
    let result = solve_r1_finite(
        &[
            decision(
                "SCENARIO-A",
                snapshot("GRID-V1", 60),
                vec![action("LOCAL", 10, 20), action("IMPORT", 7, 10)],
            ),
            decision(
                "SCENARIO-B",
                snapshot("GRID-V2", 70),
                vec![action("LOCAL", 7, 10), action("IMPORT", 10, 20)],
            ),
        ],
        16,
    ).expect("R1 finite solve");

    assert_eq!(result.candidate_policy_count, 4);
    assert_eq!(result.pareto_frontier.len(), 1);
    let selected: std::collections::BTreeSet<&str> = result.pareto_frontier[0]
        .choices
        .values()
        .map(String::as_str)
        .collect();
    assert_eq!(
        selected,
        std::collections::BTreeSet::from(["LOCAL", "IMPORT"])
    );
}

#[test]
fn distinct_observation_commitments_prevent_same_id_false_grouping() {
    let shared = snapshot("GRID-V1", 60);
    let first = decision(
        "SCENARIO-A",
        shared.clone(),
        vec![action("LOCAL", 10, 20), action("IMPORT", 9, 10)],
    );
    let mut second = decision(
        "SCENARIO-B",
        shared,
        vec![action("LOCAL", 8, 30), action("IMPORT", 9, 15)],
    );
    second.observable_state_digest = "b".repeat(64);

    let result = solve_r1_finite(&[first, second], 16).expect("distinct committed observations");
    assert_eq!(result.candidate_policy_count, 4);
}

#[test]
fn adverse_envelope_names_dimension_witnesses() {
    let result = solve_r1_finite(
        &[
            decision("SCENARIO-A", snapshot("GRID-V1", 60), vec![action("A", 10, 20)]),
            decision("SCENARIO-B", snapshot("GRID-V2", 70), vec![action("A", 8, 10)]),
        ],
        4,
    ).expect("finite oracle");

    let policy = &result.pareto_frontier[0];
    assert_eq!(
        policy.adverse_envelope.worst_scenario_by_dimension.get(
            &maritime_capability_to_service::kite_oracle::OutcomeDimension::ReceiverAcceptedUnits
        ).map(String::as_str),
        Some("SCENARIO-B")
    );
}
