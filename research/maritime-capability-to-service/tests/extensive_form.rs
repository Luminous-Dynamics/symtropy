use maritime_capability_to_service::kite::Gate;
use maritime_capability_to_service::kite_oracle::DecisionContext;
use maritime_capability_to_service::extensive_form::{
    PolicyActionAlternative, PolicyDecisionStage, PolicyScenarioPath, ResourceUseContribution,
    StageOutcomeContribution, solve_extensive_form_r1,
};

fn context() -> DecisionContext {
    DecisionContext {
        instance_id: "R1-EXTENSIVE-PUBLIC-TINY".to_owned(),
        instance_version: "v0.1".to_owned(),
        service_id: "ESSENTIAL-SERVICE".to_owned(),
        service_deadline: 100,
        expected_units: 10,
    }
}

fn contribution(accepted: u64, unmet: u64, cost: u64) -> StageOutcomeContribution {
    StageOutcomeContribution {
        receiver_accepted_units: accepted,
        on_time_accepted_units: accepted,
        acceptance_time_unknown_units: 0,
        unmet_units: unmet,
        unresolved_units: 0,
        late_accepted_units: 0,
        protected_floor_breach_unit_hours: 0,
        service_deficit_unit_hours: unmet * 2,
        time_to_minimum_viable_service: if accepted >= 5 { Some(60) } else { None },
        time_to_full_service: if accepted == 10 { Some(90) } else { None },
        recovery_cost_units: cost,
        emissions_units: 2,
        requalification_actions: 0,
        replanning_invocations: 1,
        plan_churn: 1,
        decision_latency_ms: 100,
    }
}

fn resources(crane: u64, power: u64, yard: u64) -> ResourceUseContribution {
    ResourceUseContribution {
        crane_minutes: crane,
        shore_power_wh: power,
        yard_slot_hours: yard,
        labor_hours: 1,
        freshwater_litres: 10,
    }
}

fn alternative(
    id: &str,
    accepted: u64,
    unmet: u64,
    cost: u64,
    resource: ResourceUseContribution,
) -> PolicyActionAlternative {
    PolicyActionAlternative {
        action_id: id.to_owned(),
        admissibility: Gate::Yes,
        hard_constraints_passed: true,
        outcome: contribution(accepted, unmet, cost),
        resource_use: resource,
    }
}

fn stage(
    epoch: u64,
    cutoff: u64,
    digest: &str,
    alternatives: Vec<PolicyActionAlternative>,
) -> PolicyDecisionStage {
    PolicyDecisionStage {
        decision_epoch: epoch,
        decision_cutoff: cutoff,
        observable_state_digest: digest.to_owned(),
        alternatives,
    }
}

fn path(id: &str, stages: Vec<PolicyDecisionStage>) -> PolicyScenarioPath {
    PolicyScenarioPath {
        scenario_id: id.to_owned(),
        context: context(),
        stages,
    }
}

#[test]
fn public_solver_enumerates_non_anticipative_multi_epoch_policies() {
    let paths = vec![
        path(
            "SCENARIO-A",
            vec![
                stage(
                    1,
                    10,
                    &"a".repeat(64),
                    vec![
                        alternative("LOCAL", 0, 0, 3, resources(5, 10, 2)),
                        alternative("IMPORT", 0, 0, 5, resources(3, 5, 1)),
                    ],
                ),
                stage(
                    2,
                    20,
                    &"b".repeat(64),
                    vec![
                        alternative("LOCAL", 10, 0, 10, resources(10, 20, 5)),
                        alternative("IMPORT", 8, 2, 4, resources(5, 10, 3)),
                    ],
                ),
            ],
        ),
        path(
            "SCENARIO-B",
            vec![
                stage(
                    1,
                    10,
                    &"a".repeat(64),
                    vec![
                        alternative("LOCAL", 0, 0, 3, resources(5, 10, 2)),
                        alternative("IMPORT", 0, 0, 5, resources(3, 5, 1)),
                    ],
                ),
                stage(
                    2,
                    20,
                    &"c".repeat(64),
                    vec![
                        alternative("LOCAL", 7, 3, 10, resources(12, 25, 6)),
                        alternative("IMPORT", 10, 0, 6, resources(6, 12, 3)),
                    ],
                ),
            ],
        ),
    ];

    let result = solve_extensive_form_r1(&paths, 16).expect("public extensive-form solver");
    assert_eq!(result.scenario_count, 2);
    assert_eq!(result.stage_decision_count, 4);
    assert_eq!(result.information_set_count, 3);
    assert_eq!(result.candidate_policy_count, 8);
    assert_eq!(result.evaluated_policy_count, 8);
    assert!(!result.pareto_frontier.is_empty());

    for policy in &result.pareto_frontier {
        assert_eq!(policy.choices.len(), 3);
        assert_eq!(
            policy
                .choices
                .keys()
                .filter(|key| key.decision_epoch == 1)
                .count(),
            1
        );
    }
}

#[test]
fn public_solver_refuses_a_policy_budget_smaller_than_the_exhaustive_space() {
    let path = path(
        "SINGLE",
        vec![stage(
            1,
            10,
            &"a".repeat(64),
            vec![
                alternative("A", 10, 0, 1, resources(1, 1, 1)),
                alternative("B", 9, 1, 1, resources(1, 1, 1)),
            ],
        )],
    );
    assert!(
        solve_extensive_form_r1(&[path], 1)
            .unwrap_err()
            .contains("exceeding limit")
    );
}
