use maritime_capability_to_service::kite::Gate;
use maritime_capability_to_service::kite::{
    AuthorityState, ClockQuality, EvidenceState, EvidenceVintage, RightsState, select_as_of,
};
use maritime_capability_to_service::kite_oracle::{
    ActionOutcome, DecisionContext, OutcomeVector, ScenarioDecision, solve_r1_finite,
};

fn context() -> DecisionContext {
    DecisionContext {
        instance_id: "R1-CHALLENGER-TINY".to_owned(),
        instance_version: "v0.1".to_owned(),
        service_id: "RECEIVER-SERVICE".to_owned(),
        service_deadline: 100,
        expected_units: 10,
    }
}

fn snapshot() -> maritime_capability_to_service::kite::AsOfSnapshot {
    let item = EvidenceVintage {
        logical_id: "GRID-FEEDER".to_owned(),
        vintage_id: "GRID-FEEDER-V1".to_owned(),
        revision: 1,
        supersedes_vintage_id: None,
        event_time: 20,
        source_observed_at: 40,
        first_available_at: 41,
        ingested_at: 42,
        validated_at: 43,
        valid_until: None,
        rights: RightsState::Granted,
        authority: AuthorityState::Valid,
        evidence_state: EvidenceState::Observed,
        clock_quality: ClockQuality::Trusted,
        feature_uses_future_data: false,
    };
    select_as_of(&[item], 50).expect("as-of snapshot")
}

fn outcome(accepted: u64, cost: u64) -> OutcomeVector {
    let unmet = 10 - accepted;
    OutcomeVector {
        receiver_accepted_units: accepted,
        on_time_accepted_units: accepted,
        acceptance_time_unknown_units: 0,
        unmet_units: unmet,
        late_accepted_units: 0,
        unresolved_units: 0,
        protected_floor_breach_unit_hours: 0,
        service_deficit_unit_hours: unmet * 3,
        time_to_minimum_viable_service: 2,
        time_to_full_service: 4,
        recovery_cost_units: cost,
        emissions_units: 5,
        requalification_actions: 1,
        replanning_invocations: 1,
        plan_churn: 2,
        decision_latency_ms: 250,
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

fn decision(id: &str, actions: Vec<ActionOutcome>) -> ScenarioDecision {
    ScenarioDecision {
        scenario_id: id.to_owned(),
        decision_epoch: 1,
        context: context(),
        // Same frozen observable evidence bundle for both scenarios.
        observable_state_digest: "a".repeat(64),
        as_of: snapshot(),
        actions,
    }
}

/// Intentionally simple challenger for one information set. It neither calls
/// the production grouping/key routine nor the production dominance function.
fn reference_for_one_information_set(
    decisions: &[ScenarioDecision],
) -> Vec<(String, OutcomeVector)> {
    let action_ids = ["LOCAL", "IMPORT"];
    let mut candidates = Vec::new();

    for action_id in action_ids {
        let selected: Vec<&OutcomeVector> = decisions
            .iter()
            .map(|decision| {
                &decision
                    .actions
                    .iter()
                    .find(|action| action.action_id == action_id)
                    .expect("action exists in both scenarios")
                    .outcome
            })
            .collect();

        let accepted = selected
            .iter()
            .map(|row| row.receiver_accepted_units)
            .min()
            .expect("two scenario outcomes");
        let on_time = selected
            .iter()
            .map(|row| row.on_time_accepted_units)
            .min()
            .expect("two scenario outcomes");
        let unmet = selected.iter().map(|row| row.unmet_units).max().unwrap();
        let deficit = selected
            .iter()
            .map(|row| row.service_deficit_unit_hours)
            .max()
            .unwrap();
        let cost = selected
            .iter()
            .map(|row| row.recovery_cost_units)
            .max()
            .unwrap();

        candidates.push((
            action_id.to_owned(),
            OutcomeVector {
                receiver_accepted_units: accepted,
                on_time_accepted_units: on_time,
                acceptance_time_unknown_units: 0,
                unmet_units: unmet,
                late_accepted_units: 0,
                unresolved_units: 0,
                protected_floor_breach_unit_hours: 0,
                service_deficit_unit_hours: deficit,
                time_to_minimum_viable_service: 2,
                time_to_full_service: 4,
                recovery_cost_units: cost,
                emissions_units: 5,
                requalification_actions: 1,
                replanning_invocations: 1,
                plan_churn: 2,
                decision_latency_ms: 250,
            },
        ));
    }

    fn independently_dominates(left: &OutcomeVector, right: &OutcomeVector) -> bool {
        let no_worse = left.receiver_accepted_units >= right.receiver_accepted_units
            && left.on_time_accepted_units >= right.on_time_accepted_units
            && left.acceptance_time_unknown_units <= right.acceptance_time_unknown_units
            && left.unmet_units <= right.unmet_units
            && left.late_accepted_units <= right.late_accepted_units
            && left.unresolved_units <= right.unresolved_units
            && left.protected_floor_breach_unit_hours <= right.protected_floor_breach_unit_hours
            && left.service_deficit_unit_hours <= right.service_deficit_unit_hours
            && left.time_to_minimum_viable_service <= right.time_to_minimum_viable_service
            && left.time_to_full_service <= right.time_to_full_service
            && left.recovery_cost_units <= right.recovery_cost_units
            && left.emissions_units <= right.emissions_units
            && left.requalification_actions <= right.requalification_actions
            && left.replanning_invocations <= right.replanning_invocations
            && left.plan_churn <= right.plan_churn
            && left.decision_latency_ms <= right.decision_latency_ms;
        let strict = left.receiver_accepted_units > right.receiver_accepted_units
            || left.on_time_accepted_units > right.on_time_accepted_units
            || left.acceptance_time_unknown_units < right.acceptance_time_unknown_units
            || left.unmet_units < right.unmet_units
            || left.late_accepted_units < right.late_accepted_units
            || left.unresolved_units < right.unresolved_units
            || left.protected_floor_breach_unit_hours < right.protected_floor_breach_unit_hours
            || left.service_deficit_unit_hours < right.service_deficit_unit_hours
            || left.time_to_minimum_viable_service < right.time_to_minimum_viable_service
            || left.time_to_full_service < right.time_to_full_service
            || left.recovery_cost_units < right.recovery_cost_units
            || left.emissions_units < right.emissions_units
            || left.requalification_actions < right.requalification_actions
            || left.replanning_invocations < right.replanning_invocations
            || left.plan_churn < right.plan_churn
            || left.decision_latency_ms < right.decision_latency_ms;
        no_worse && strict
    }

    candidates
        .iter()
        .filter(|candidate| {
            !candidates.iter().any(|other| {
                other.0 != candidate.0 && independently_dominates(&other.1, &candidate.1)
            })
        })
        .cloned()
        .collect()
}

#[test]
fn production_r1_matches_a_separate_tiny_one_information_set_challenger() {
    let decisions = vec![
        decision(
            "SCENARIO-A",
            vec![action("LOCAL", 10, 20), action("IMPORT", 9, 10)],
        ),
        decision(
            "SCENARIO-B",
            vec![action("LOCAL", 8, 30), action("IMPORT", 9, 15)],
        ),
    ];

    let reference = reference_for_one_information_set(&decisions);
    assert_eq!(reference.len(), 1);
    assert_eq!(reference[0].0, "IMPORT");
    assert_eq!(reference[0].1.receiver_accepted_units, 9);
    assert_eq!(reference[0].1.unmet_units, 1);
    assert_eq!(reference[0].1.service_deficit_unit_hours, 3);
    assert_eq!(reference[0].1.recovery_cost_units, 15);

    let production = solve_r1_finite(&decisions, 4).expect("production R1 oracle");
    assert_eq!(production.candidate_policy_count, 2);
    assert_eq!(production.pareto_frontier.len(), reference.len());
    assert_eq!(
        production.pareto_frontier[0]
            .choices
            .values()
            .next()
            .map(String::as_str),
        Some(reference[0].0.as_str())
    );

    let production_bounds = &production.pareto_frontier[0].adverse_envelope.bounds;
    assert_eq!(
        production_bounds.receiver_accepted_units,
        reference[0].1.receiver_accepted_units
    );
    assert_eq!(production_bounds.unmet_units, reference[0].1.unmet_units);
    assert_eq!(
        production_bounds.service_deficit_unit_hours,
        reference[0].1.service_deficit_unit_hours
    );
    assert_eq!(
        production_bounds.recovery_cost_units,
        reference[0].1.recovery_cost_units
    );
}
