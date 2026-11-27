use maritime_capability_to_service::kite::Gate;
use maritime_capability_to_service::kite_oracle::DecisionContext;
use maritime_capability_to_service::extensive_form::{
    PolicyActionAlternative, PolicyDecisionStage, PolicyScenarioPath, ResourceUseContribution,
    StageOutcomeContribution, solve_extensive_form_r1,
};

fn context() -> DecisionContext {
    DecisionContext {
        instance_id: "R1-INDEPENDENT-REFERENCE".to_owned(),
        instance_version: "v0.1".to_owned(),
        service_id: "RECEIVER-SERVICE".to_owned(),
        service_deadline: 100,
        expected_units: 10,
    }
}

fn action(id: &str, accepted: u64, cost: u64, cranes: u64, power: u64) -> PolicyActionAlternative {
    let unmet = 10 - accepted;
    PolicyActionAlternative {
        action_id: id.to_owned(),
        admissibility: Gate::Yes,
        hard_constraints_passed: true,
        outcome: StageOutcomeContribution {
            receiver_accepted_units: accepted,
            on_time_accepted_units: accepted,
            acceptance_time_unknown_units: 0,
            unmet_units: unmet,
            unresolved_units: 0,
            late_accepted_units: 0,
            protected_floor_breach_unit_hours: 0,
            service_deficit_unit_hours: unmet * 2,
            time_to_minimum_viable_service: Some(10),
            time_to_full_service: if accepted == 10 { Some(20) } else { None },
            recovery_cost_units: cost,
            emissions_units: 1,
            requalification_actions: 0,
            replanning_invocations: 0,
            plan_churn: 0,
            decision_latency_ms: 10,
        },
        resource_use: ResourceUseContribution {
            crane_minutes: cranes,
            shore_power_wh: power,
            yard_slot_hours: 2,
            labor_hours: 1,
            freshwater_litres: 1,
        },
    }
}

fn scenario(id: &str, safe: (u64, u64, u64, u64), fast: (u64, u64, u64, u64)) -> PolicyScenarioPath {
    PolicyScenarioPath {
        scenario_id: id.to_owned(),
        context: context(),
        stages: vec![PolicyDecisionStage {
            decision_epoch: 1,
            decision_cutoff: 50,
            observable_state_digest: "d".repeat(64),
            alternatives: vec![
                action("SAFE", safe.0, safe.1, safe.2, safe.3),
                action("FAST", fast.0, fast.1, fast.2, fast.3),
            ],
        }],
    }
}

/// Separate hand-enumerated reference for two actions and one common information set.
/// It does not call any production solver, grouping, aggregation or dominance helper.
#[test]
fn extensive_form_solver_matches_independent_tiny_reference() {
    // SAFE: minimum service 8, worst cost 8, worst cranes 6, worst power 6.
    // FAST: minimum service 9, worst cost 4, worst cranes 4, worst power 4.
    // FAST is no worse in any dimension and strictly better in several.
    let reference_frontier = vec![("FAST", 9u64, 1u64, 4u64, 4u64, 4u64)];

    let paths = vec![
        scenario("S1", (10, 5, 5, 5), (9, 3, 3, 3)),
        scenario("S2", (8, 8, 6, 6), (9, 4, 4, 4)),
    ];

    let result = solve_extensive_form_r1(&paths, 4).expect("finite R1 solver");
    assert_eq!(result.candidate_policy_count, 2);
    assert_eq!(result.evaluated_policy_count, 2);
    assert_eq!(result.pareto_frontier.len(), reference_frontier.len());

    let policy = &result.pareto_frontier[0];
    let selected_action = policy.choices.values().next().map(String::as_str);
    assert_eq!(selected_action, Some(reference_frontier[0].0));
    assert_eq!(
        policy.service_adverse_envelope.bounds.receiver_accepted_units,
        reference_frontier[0].1
    );
    assert_eq!(
        policy.service_adverse_envelope.bounds.unmet_units,
        reference_frontier[0].2
    );
    assert_eq!(
        policy.service_adverse_envelope.bounds.recovery_cost_units,
        reference_frontier[0].3
    );
    assert_eq!(
        policy.resource_adverse_envelope.bounds.crane_minutes,
        reference_frontier[0].4
    );
    assert_eq!(
        policy.resource_adverse_envelope.bounds.shore_power_wh,
        reference_frontier[0].5
    );
}
